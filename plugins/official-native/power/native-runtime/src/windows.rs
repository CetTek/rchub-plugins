//! Windows providers use fixed native APIs and strictly typed bounded payloads.
use serde_json::{json, Value};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
pub(crate) fn battery(input: &Value, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    if !input.as_object().is_some_and(|o| o.is_empty()) || !current() { return Err(()); }
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 || !current() { return Err(()); }
    battery_status(status.BatteryFlag, status.BatteryLifePercent)
}

fn battery_status(flags: u8, percent: u8) -> Result<Value, ()> {
    if flags == 255 { return Err(()); }
    let has_battery = flags & 128 == 0;
    let battery_percent = if has_battery && percent <= 100 { i32::from(percent) } else { -1 };
    Ok(json!({"has_battery":has_battery,"battery_percent":battery_percent,"charging":has_battery && flags & 8 != 0}))
}

#[cfg(test)]
#[test]
fn battery_status_distinguishes_unknown_and_missing() {
    assert!(battery_status(255, 255).is_err());
    assert_eq!(battery_status(128, 255).unwrap(), json!({"has_battery":false,"battery_percent":-1,"charging":false}));
    assert_eq!(battery_status(8, 42).unwrap(), json!({"has_battery":true,"battery_percent":42,"charging":true}));
    assert_eq!(battery_status(1, 255).unwrap(), json!({"has_battery":true,"battery_percent":-1,"charging":false}));
}


fn keyboard_key(key: u16, scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
fn bounded_text(text: &str, maximum: usize) -> bool {
    !text.contains('\0') && text.chars().count() <= maximum
}

#[cfg(test)]
#[test]
fn text_limits_count_unicode_scalars() {
    assert!(bounded_text(&"😀".repeat(8192), 8192));
    assert!(!bounded_text(&"😀".repeat(8193), 8192));
    assert!(!bounded_text("a\0b", 8192));
}
fn hotkey(s: &str) -> Result<Vec<u16>, ()> {
    if s.is_empty() || s.len() > 80 {
        return Err(());
    }
    let lower = s.to_ascii_lowercase();
    let mut keys = vec![];
    let parts: Vec<_> = lower.split('+').collect();
    if parts.len() > 5 {
        return Err(());
    }
    for (i, part) in parts.iter().enumerate() {
        let key = match *part {
            "ctrl" => VK_CONTROL,
            "alt" => VK_MENU,
            "shift" => VK_SHIFT,
            "win" => VK_LWIN,
            _ => {
                if i != parts.len() - 1 {
                    return Err(());
                }
                match *part {
                    "enter" => VK_RETURN,
                    "escape" | "esc" => VK_ESCAPE,
                    "tab" => VK_TAB,
                    "space" => VK_SPACE,
                    "backspace" => VK_BACK,
                    "delete" => VK_DELETE,
                    "insert" => VK_INSERT,
                    "home" => VK_HOME,
                    "end" => VK_END,
                    "pageup" => VK_PRIOR,
                    "pagedown" => VK_NEXT,
                    "left" => VK_LEFT,
                    "right" => VK_RIGHT,
                    "up" => VK_UP,
                    "down" => VK_DOWN,
                    s if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() => {
                        s.as_bytes()[0].to_ascii_uppercase().into()
                    }
                    s if s.starts_with('f') => {
                        let n = s[1..].parse::<u16>().map_err(|_| ())?;
                        if !(1..=24).contains(&n) {
                            return Err(());
                        }
                        VK_F1 + n - 1
                    }
                    _ => return Err(()),
                }
            }
        };
        if keys.contains(&key)
            || i == parts.len() - 1 && matches!(key, VK_CONTROL | VK_MENU | VK_SHIFT | VK_LWIN)
        {
            return Err(());
        }
        keys.push(key)
    }
    Ok(keys)
}
/// SendInput has no target parameter: it inherits the foreground window. A missing
/// window, the shell/desktop, or our own process cannot be an intentional typing
/// target, so the text path fails closed instead of reporting a delivery.
fn typing_target(foreground: isize, shell: isize, own_pid: u32, foreground_pid: u32) -> bool {
    foreground != 0 && foreground != shell && foreground_pid != 0 && foreground_pid != own_pid
}
fn typing_target_available() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetShellWindow, GetWindowThreadProcessId,
    };
    let foreground = unsafe { GetForegroundWindow() };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(foreground, &mut pid) };
    typing_target(
        foreground as isize,
        unsafe { GetShellWindow() } as isize,
        std::process::id(),
        pid,
    )
}
fn send(events: &[INPUT], release: &[INPUT], current: &dyn Fn() -> bool) -> Result<(), ()> {
    if !current() {
        return Err(());
    }
    let inserted = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if inserted != events.len() as u32 {
        if !release.is_empty() {
            unsafe {
                SendInput(
                    release.len() as u32,
                    release.as_ptr(),
                    std::mem::size_of::<INPUT>() as i32,
                )
            };
        }
        return Err(());
    }
    Ok(())
}
pub(super) fn invoke(cap: &str, input: Value, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    let o = input.as_object().ok_or(())?;
    match cap {
        "keyboard.write" if o.len() == 1 => {
            let mut events = vec![];
            let mut release = vec![];
            if let Some(text) = o.get("text").and_then(Value::as_str) {
                if text.is_empty() || !bounded_text(text, 4096) {
                    return Err(());
                }
                if !typing_target_available() {
                    return Err(());
                }
                // A physical Enter produces one line break. Mobile drafts can carry
                // CRLF from pasted or composed text; normalize before encoding so a
                // CRLF payload does not type two line breaks.
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                for ch in text.encode_utf16() {
                    events.push(keyboard_key(0, ch, KEYEVENTF_UNICODE));
                    let up = keyboard_key(0, ch, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
                    events.push(up);
                    release.push(up);
                }
            } else if let Some(keys) = o.get("keys").and_then(Value::as_str) {
                let keys = hotkey(keys)?;
                for key in &keys {
                    events.push(keyboard_key(*key, 0, 0));
                }
                for key in keys.iter().rev() {
                    release.push(keyboard_key(*key, 0, KEYEVENTF_KEYUP));
                }
                events.extend_from_slice(&release);
            } else {
                return Err(());
            }
            send(&events, &release, current)?;
            Ok(json!({"sent":true}))
        }
        "media.write" if o.len() == 1 => {
            let action = o.get("action").and_then(Value::as_str).ok_or(())?;
            let key = match action {
                "play_pause" => VK_MEDIA_PLAY_PAUSE,
                "next" => VK_MEDIA_NEXT_TRACK,
                "previous" => VK_MEDIA_PREV_TRACK,
                _ => return Err(()),
            };
            let up = keyboard_key(key, 0, KEYEVENTF_KEYUP);
            send(&[keyboard_key(key, 0, 0), up], &[up], current)?;
            Ok(json!({"accepted":true}))
        }
        "clipboard.read" if o.is_empty() => {
            if !current() {
                return Err(());
            }
            let clipboard = Clipboard::open(false, current)?;
            let text = clipboard.read()?;
            if !current() {
                return Err(());
            }
            Ok(json!({"text":text}))
        }
        "clipboard.write" if o.len() == 1 => {
            let text = o
                .get("text")
                .and_then(Value::as_str)
                .filter(|s| bounded_text(s, 8192))
                .ok_or(())?;
            if !current() {
                return Err(());
            }
            let clipboard = Clipboard::open(true, current)?;
            clipboard.write(text, current)?;
            if !current() {
                return Err(());
            }
            let text = clipboard.read()?;
            if !current() {
                return Err(());
            }
            Ok(json!({"text":text}))
        }
        "apps.read" => apps_read(o, current),
        "apps.write" if o.contains_key("name") => apps_launch(o, current),
        "apps.write" => apps_write(o, current),
        _ => Err(()),
    }
}

fn process_stamp(handle: windows_sys::Win32::Foundation::HANDLE) -> Result<u64, ()> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};
    let mut created: FILETIME = unsafe { std::mem::zeroed() };
    let mut exit = created;
    let mut kernel = created;
    let mut user = created;
    if unsafe { GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(());
    }
    Ok(((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime)) / 10000)
}
struct Process(windows_sys::Win32::Foundation::HANDLE);
impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
fn apps_read(o: &serde_json::Map<String, Value>, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    if o.keys().any(|k| k != "filter") {
        return Err(());
    }
    let filter = o
        .get("filter")
        .map_or(Some("foreground"), Value::as_str)
        .ok_or(())?;
    if !matches!(filter, "all" | "foreground") || !current() {
        return Err(());
    }
    // Visible top-level windows are collected without activating or messaging applications.
    unsafe extern "system" fn collect(
        window: windows_sys::Win32::Foundation::HWND,
        data: isize,
    ) -> i32 {
        if IsWindowVisible(window) != 0 && GetWindowTextLengthW(window) > 0 {
            (*(data as *mut Vec<usize>)).push(window as usize);
        }
        1
    }
    let foreground = unsafe { GetForegroundWindow() };
    let mut windows = Vec::<usize>::new();
    if unsafe { EnumWindows(Some(collect), &mut windows as *mut _ as isize) } == 0 {
        return Err(());
    }
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut apps = vec![];
    let mut processes: Vec<_> = system.processes().iter().collect();
    processes.sort_by_key(|(pid, p)| {
        (
            p.name().to_string_lossy().to_ascii_lowercase(),
            pid.as_u32(),
        )
    });
    for (pid, p) in processes {
        if !current() {
            return Err(());
        }
        let pid = pid.as_u32();
        if pid == 0 || pid > i32::MAX as u32 {
            continue;
        }
        let handle = Process(unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(
                windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        });
        if handle.0.is_null() {
            continue;
        }
        let Ok(started_at) = process_stamp(handle.0) else {
            continue;
        };
        let matching: Vec<_> = windows
            .iter()
            .copied()
            .filter(|w| {
                let mut owner = 0;
                unsafe {
                    GetWindowThreadProcessId(*w as _, &mut owner);
                }
                owner == pid
            })
            .collect();
        let window = matching
            .iter()
            .copied()
            .find(|w| *w == foreground as usize)
            .or_else(|| matching.first().copied());
        let is_foreground = window == Some(foreground as usize);
        if filter == "foreground" && window.is_none() {
            continue;
        }
        let mut title = [0u16; 241];
        let length = window.map_or(0, |w| unsafe {
            GetWindowTextW(w as _, title.as_mut_ptr(), title.len() as i32)
        }) as usize;
        apps.push(json!({"pid":pid,"name":p.name().to_string_lossy().chars().take(120).collect::<String>(),"title":String::from_utf16_lossy(&title[..length]),"window_id":window.map(|w| w.to_string()).unwrap_or_default(),"started_at":started_at,"foreground":is_foreground}));
        if apps.len() >= 64 {
            break;
        }
    }
    if !current() {
        return Err(());
    }
    Ok(json!({"count":apps.len(),"apps":apps}))
}
// Only resolve names against Start Menu shortcuts; never interpret remote input as
// a path, command line, URI, or shell expression.
fn launch_shortcut(roots: &[std::path::PathBuf], name: &str, current: &dyn Fn() -> bool) -> Result<std::path::PathBuf, ()> {
    let query = name.trim().to_lowercase();
    if query.is_empty() || !bounded_text(name, 120) { return Err(()); }
    let mut pending = roots.to_vec();
    let mut matches = Vec::new();
    while let Some(dir) = pending.pop() {
        if !current() { return Err(()); }
        let Ok(entries) = std::fs::read_dir(dir) else { continue; };
        for entry in entries.flatten() {
            if !current() { return Err(()); }
            let Ok(kind) = entry.file_type() else { continue; };
            if kind.is_dir() { pending.push(entry.path()); continue; }
            if !kind.is_file() { continue; }
            let path = entry.path();
            if !path.extension().is_some_and(|s| s.eq_ignore_ascii_case("lnk")) { continue; }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue; };
            let lower = stem.to_lowercase();
            if lower.contains("uninstall") || lower.contains("remove") || !lower.contains(&query) { continue; }
            matches.push((lower.chars().count(), lower, path));
        }
    }
    matches.sort();
    matches.into_iter().next().map(|(_, _, path)| path).ok_or(())
}

fn apps_launch(o: &serde_json::Map<String, Value>, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    if o.len() != 1 { return Err(()); }
    let name = o.get("name").and_then(Value::as_str).ok_or(())?;
    let roots: Vec<_> = ["APPDATA", "PROGRAMDATA"].into_iter().filter_map(std::env::var_os)
        .map(|base| std::path::PathBuf::from(base).join("Microsoft").join("Windows").join("Start Menu").join("Programs")).collect();
    let path = launch_shortcut(&roots, name, current)?;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    if !current() { return Err(()); }
    let result = unsafe { ShellExecuteW(std::ptr::null_mut(), windows_sys::core::w!("open"), path.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
    if result as isize <= 32 { return Err(()); }
    Ok(json!({"accepted":true}))
}

#[cfg(test)]
#[test]
fn launch_rejects_invalid_and_revoked_requests() {
    assert!(launch_shortcut(&[], " ", &|| true).is_err());
    assert!(launch_shortcut(&[], "app\0name", &|| true).is_err());
    assert!(launch_shortcut(&[std::path::PathBuf::from(".")], "app", &|| false).is_err());
    assert!(apps_launch(&serde_json::Map::from_iter([("path".into(), json!("C:\\x.exe"))]), &|| true).is_err());
}

#[cfg(test)]
#[test]
fn launch_search_selects_shortest_shortcut_and_skips_uninstallers() {
    let root = std::env::temp_dir().join(format!("rchub-launch-test-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    for name in ["Example Extended.lnk", "Example.lnk", "Remove Example.lnk", "Example.exe"] {
        std::fs::write(root.join("nested").join(name), b"").unwrap();
    }
    let found = launch_shortcut(&[root.clone()], "EXAMPLE", &|| true).unwrap();
    assert_eq!(found.file_name().unwrap(), "Example.lnk");
    assert!(launch_shortcut(&[root.clone()], "missing", &|| true).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

fn apps_write(o: &serde_json::Map<String, Value>, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    use windows_sys::Win32::{System::Threading::*, UI::WindowsAndMessaging::*};
    if o.keys()
        .any(|k| !matches!(k.as_str(), "pid" | "action" | "window_id" | "started_at"))
    {
        return Err(());
    }
    let pid = o
        .get("pid")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0 && *n <= i32::MAX as u64)
        .ok_or(())? as u32;
    let stamp = o.get("started_at").and_then(Value::as_u64).ok_or(())?;
    let action = o.get("action").and_then(Value::as_str).ok_or(())?;
    if !matches!(
        action,
        "stop" | "close" | "bring_to_front" | "minimize" | "maximize" | "restore"
    ) || !current()
    {
        return Err(());
    }
    let handle = Process(unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION
                | if action == "stop" {
                    PROCESS_TERMINATE
                } else {
                    0
                },
            0,
            pid,
        )
    });
    if handle.0.is_null() || process_stamp(handle.0)? != stamp {
        return Err(());
    }
    if action == "stop" {
        if !current() || unsafe { TerminateProcess(handle.0, 1) } == 0 {
            return Err(());
        }
    } else {
        let id = o
            .get("window_id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 32)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?;
        let window = id as windows_sys::Win32::Foundation::HWND;
        let mut owner = 0;
        if unsafe { IsWindow(window) } == 0 || unsafe { IsWindowVisible(window) } == 0 {
            return Err(());
        }
        unsafe {
            GetWindowThreadProcessId(window, &mut owner);
        }
        if owner != pid || !current() {
            return Err(());
        }
        let success = match action {
            "close" => unsafe { PostMessageW(window, WM_CLOSE, 0, 0) != 0 },
            "bring_to_front" => bring_to_front(window, pid, stamp, &|| {
                let mut owner = 0;
                current()
                    && process_stamp(handle.0) == Ok(stamp)
                    && unsafe { IsWindow(window) } != 0
                    && unsafe { IsWindowVisible(window) } != 0
                    && unsafe { GetWindowThreadProcessId(window, &mut owner) } != 0
                    && owner == pid
            }),
            "minimize" => unsafe { ShowWindowAsync(window, SW_MINIMIZE) != 0 },
            "maximize" => unsafe { ShowWindowAsync(window, SW_MAXIMIZE) != 0 },
            "restore" => unsafe { ShowWindowAsync(window, SW_RESTORE) != 0 },
            _ => false,
        };
        if !success {
            return Err(());
        }
    }
    Ok(json!({"accepted":true}))
}

// Cross-thread restore and activation are asynchronous. Never attach input queues:
// doing so can make activation block forever on another application's UI thread.
fn bring_to_front(
    window: windows_sys::Win32::Foundation::HWND,
    pid: u32,
    stamp: u64,
    current_target: &dyn Fn() -> bool,
) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    if !current_target() {
        return false;
    }
    if unsafe { IsIconic(window) } != 0 {
        if unsafe { ShowWindowAsync(window, SW_RESTORE) } == 0
            || !observe_window_state(current_target, &|| unsafe { IsIconic(window) == 0 })
        {
            return false;
        }
    }
    if !current_target() {
        return false;
    }
    if unsafe { GetForegroundWindow() } != window {
        // Windows can deny foreground activation. Do not change policy, synthesize
        // keyboard input, or report an accepted request as an observed success.
        unsafe { SetForegroundWindow(window) };
    }
    if observe_window_state(current_target, &|| unsafe {
        GetForegroundWindow() == window
    }) {
        return true;
    }
    foreground_via_helper(window, pid, stamp, current_target)
}

fn foreground_via_helper(
    window: windows_sys::Win32::Foundation::HWND,
    pid: u32,
    stamp: u64,
    current_target: &dyn Fn() -> bool,
) -> bool {
    use std::{
        io::Write,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    use windows_sys::Win32::{System::Threading::*, UI::WindowsAndMessaging::*};
    if !current_target() {
        return false;
    }
    let mut owner = 0;
    unsafe { GetWindowThreadProcessId(window, &mut owner) };
    let handle = Process(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
    if owner != pid
        || handle.0.is_null()
        || process_stamp(handle.0) != Ok(stamp)
        || !current_target()
    {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut command = Command::new(exe);
    command.arg("--plugin-foreground-helper");
    let Ok(mut child) = command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let payload =
        json!({"pid":pid,"stamp":stamp,"window":(window as usize).to_string()}).to_string();
    let sent = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(payload.as_bytes()).is_ok());
    wait_for_foreground_helper(&mut child, sent, current_target, &|| unsafe {
        GetForegroundWindow() == window
    })
}

fn wait_for_foreground_helper(
    child: &mut std::process::Child,
    sent: bool,
    current_target: &dyn Fn() -> bool,
    foreground_observed: &dyn Fn() -> bool,
) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(750);
    while sent && current_target() && std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => {
                return status.success() && current_target() && foreground_observed()
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
            Err(_) => break,
        }
    }
    // Only our disposable helper is terminated; never the selected application.
    let _ = child.kill();
    let _ = child.wait();
    false
}

/// Single-purpose package-owned helper, dispatched before protocol startup.
/// Input-queue attachment can block on a foreign UI thread, so the parent owns a
/// short watchdog and terminates/reaps this disposable process on expiry/revocation.
pub(crate) fn foreground_helper() -> bool {
    use std::io::Read;
    use windows_sys::Win32::{System::Threading::*, UI::WindowsAndMessaging::*};
    let mut bytes = Vec::new();
    if std::io::stdin().take(257).read_to_end(&mut bytes).is_err()
        || bytes.len() > 256
    {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    let Some(o) = value.as_object().filter(|o| o.len() == 3) else {
        return false;
    };
    let Some(pid) = o
        .get("pid")
        .and_then(Value::as_u64)
        .filter(|pid| *pid > 0 && *pid <= i32::MAX as u64)
    else {
        return false;
    };
    let Some(stamp) = o.get("stamp").and_then(Value::as_u64) else {
        return false;
    };
    let Some(window) = o
        .get("window")
        .and_then(Value::as_str)
        .filter(|s| s.len() <= 32)
        .and_then(|s| s.parse::<usize>().ok())
    else {
        return false;
    };
    let window = window as windows_sys::Win32::Foundation::HWND;
    let handle = Process(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32) });
    let valid = || {
        let mut owner = 0;
        !handle.0.is_null()
            && process_stamp(handle.0) == Ok(stamp)
            && unsafe { IsWindow(window) } != 0
            && unsafe { IsWindowVisible(window) } != 0
            && unsafe { GetWindowThreadProcessId(window, &mut owner) } != 0
            && owner == pid as u32
    };
    if !valid() {
        return false;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground == window {
        return true;
    }
    let thread = unsafe { GetCurrentThreadId() };
    let foreground_thread = unsafe { GetWindowThreadProcessId(foreground, std::ptr::null_mut()) };
    let target_thread = unsafe { GetWindowThreadProcessId(window, std::ptr::null_mut()) };
    if foreground_thread == 0
        || foreground_thread == thread
        || target_thread == 0
        || target_thread == thread
    {
        return false;
    }
    // Explicitly create this helper thread's message queue before attachment.
    let mut message: MSG = unsafe { std::mem::zeroed() };
    unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE) };
    struct Attachment(u32, u32);
    impl Drop for Attachment {
        fn drop(&mut self) {
            unsafe { AttachThreadInput(self.0, self.1, 0) };
        }
    }
    // Activation needs the input state of the current foreground thread and of the
    // thread that owns the validated target window. Attaching to only one of them is
    // denied. The parent keeps a watchdog and reaps this disposable helper, so an
    // unresponsive foreign UI thread cannot strand the desktop process.
    if unsafe { AttachThreadInput(thread, foreground_thread, 1) } == 0 {
        return false;
    }
    let _foreground = Attachment(thread, foreground_thread);
    let _target = if target_thread == foreground_thread {
        None
    } else {
        if unsafe { AttachThreadInput(thread, target_thread, 1) } == 0 {
            return false;
        }
        Some(Attachment(thread, target_thread))
    };
    if !valid() {
        return false;
    }
    // Activation can be denied transiently while another window holds the
    // foreground lock; retry a bounded number of times while still attached.
    for attempt in 0..3 {
        unsafe { SetForegroundWindow(window) };
        if unsafe { GetForegroundWindow() } == window {
            break;
        }
        if attempt < 2 {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    // Detach immediately: shared input queues must not stay attached while this
    // helper polls for the observed state, or the target thread's input lags.
    drop(_target);
    drop(_foreground);
    observe_window_state(&valid, &|| unsafe { GetForegroundWindow() == window })
}

fn observe_window_state(current: &dyn Fn() -> bool, observed: &dyn Fn() -> bool) -> bool {
    for attempt in 0..26 {
        if !current() {
            return false;
        }
        if observed() {
            return current();
        }
        if attempt < 25 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    false
}

struct Clipboard {
    window: windows_sys::Win32::Foundation::HWND,
}
impl Clipboard {
    fn open(write: bool, current: &dyn Fn() -> bool) -> Result<Self, ()> {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let window = if write {
            unsafe {
                CreateWindowExW(
                    0,
                    windows_sys::core::w!("STATIC"),
                    windows_sys::core::w!(""),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                )
            }
        } else {
            std::ptr::null_mut()
        };
        if write && window.is_null() {
            return Err(());
        }
        let mut opened = false;
        for attempt in 0..10 {
            if !current() {
                break;
            }
            if unsafe { windows_sys::Win32::System::DataExchange::OpenClipboard(window) } != 0 {
                opened = true;
                break;
            }
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        if !opened {
            if !window.is_null() {
                unsafe { DestroyWindow(window) };
            }
            return Err(());
        }
        Ok(Self { window })
    }
    fn read(&self) -> Result<String, ()> {
        use windows_sys::Win32::{
            Foundation::HGLOBAL,
            System::{DataExchange::GetClipboardData, Memory::*},
        };
        if unsafe { windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable(13) } == 0
        {
            return Ok(String::new());
        }
        let handle = unsafe { GetClipboardData(13) };
        if handle.is_null() {
            return Err(());
        }
        let size = unsafe { GlobalSize(handle as HGLOBAL) } / 2;
        if size == 0 {
            return Err(());
        }
        let ptr = unsafe { GlobalLock(handle as HGLOBAL) } as *const u16;
        if ptr.is_null() {
            return Err(());
        }
        struct Locked(HGLOBAL);
        impl Drop for Locked {
            fn drop(&mut self) {
                unsafe { GlobalUnlock(self.0) };
            }
        }
        let _lock = Locked(handle as HGLOBAL);
        // A schema character can occupy two UTF-16 units, plus the terminator.
        let slice = unsafe { std::slice::from_raw_parts(ptr, size.min(16385)) };
        let end = slice.iter().position(|c| *c == 0).ok_or(())?;
        let text = String::from_utf16(&slice[..end]).map_err(|_| ())?;
        if !bounded_text(&text, 8192) {
            return Err(());
        }
        Ok(text)
    }
    fn write(&self, text: &str, current: &dyn Fn() -> bool) -> Result<(), ()> {
        use windows_sys::Win32::{
            Foundation::{GlobalFree, HGLOBAL},
            System::{DataExchange::*, Memory::*},
        };
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2) };
        if handle.is_null() {
            return Err(());
        }
        struct Owned(HGLOBAL);
        impl Drop for Owned {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { GlobalFree(self.0) };
                }
            }
        }
        let mut owned = Owned(handle);
        let ptr = unsafe { GlobalLock(handle) } as *mut u16;
        if ptr.is_null() {
            return Err(());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            GlobalUnlock(handle);
        }
        if !current() {
            return Err(());
        }
        if unsafe { EmptyClipboard() } == 0 {
            return Err(());
        }
        if !current() {
            return Err(());
        }
        if unsafe { SetClipboardData(13, handle) }.is_null() {
            return Err(());
        }
        owned.0 = std::ptr::null_mut();
        Ok(())
    }
}
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::DataExchange::CloseClipboard();
            if !self.window.is_null() {
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.window);
            }
        }
    }
}
