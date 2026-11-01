//! Package-owned OS implementations. Native code runs with the current user account.
use serde_json::{json, Value};
#[cfg(windows)]
#[path = "speech.rs"]
pub(crate) mod speech;
#[cfg(windows)]
#[path = "visual.rs"]
mod visual;
#[cfg(windows)]
#[path = "windows.rs"]
pub(crate) mod windows;
#[cfg(target_os = "linux")]
pub(super) fn linux_battery(input: &Value, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    if !input.as_object().is_some_and(|o| o.is_empty()) { return Err(()); }
    linux_battery_at(std::path::Path::new("/sys/class/power_supply"), current)
}

#[cfg(target_os = "linux")]
fn linux_battery_at(root: &std::path::Path, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    if !current() { return Err(()); }
    let mut entries = std::fs::read_dir(root).map_err(|_| ())?.collect::<Result<Vec<_>, _>>().map_err(|_| ())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if !current() { return Err(()); }
        let path = entry.path();
        if std::fs::read_to_string(path.join("type")).map_err(|_| ())?.trim() != "Battery" { continue; }
        if let Ok(present) = std::fs::read_to_string(path.join("present")) {
            match present.trim() { "0" => continue, "1" => (), _ => return Err(()) }
        }
        let percent = std::fs::read_to_string(path.join("capacity")).ok()
            .and_then(|s| s.trim().parse::<i32>().ok()).filter(|n| (0..=100).contains(n)).unwrap_or(-1);
        let status = std::fs::read_to_string(path.join("status")).map_err(|_| ())?;
        let charging = match status.trim() {
            "Charging" => true,
            "Discharging" | "Not charging" | "Full" => false,
            _ => return Err(()),
        };
        if !current() { return Err(()); }
        return Ok(json!({"has_battery":true,"battery_percent":percent,"charging":charging}));
    }
    if !current() { return Err(()); }
    Ok(json!({"has_battery":false,"battery_percent":-1,"charging":false}))
}

#[cfg(all(test, target_os = "linux"))]
#[test]
fn linux_battery_reads_sysfs_and_fails_closed() {
    let root = std::env::temp_dir().join(format!("rchub-battery-test-{}", std::process::id()));
    let bat = root.join("BAT0");
    std::fs::create_dir_all(&bat).unwrap();
    std::fs::write(bat.join("type"), "Battery\n").unwrap();
    std::fs::write(bat.join("capacity"), "42\n").unwrap();
    std::fs::write(bat.join("status"), "Charging\n").unwrap();
    assert_eq!(linux_battery_at(&root, &|| true).unwrap(), json!({"has_battery":true,"battery_percent":42,"charging":true}));
    assert!(linux_battery_at(&root, &|| false).is_err());
    std::fs::write(bat.join("status"), "Unknown\n").unwrap();
    assert!(linux_battery_at(&root, &|| true).is_err());
    std::fs::write(bat.join("present"), "0\n").unwrap();
    assert_eq!(linux_battery_at(&root, &|| true).unwrap(), json!({"has_battery":false,"battery_percent":-1,"charging":false}));
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(target_os = "linux")]
fn linux_open_url(url: &str, current: &dyn Fn() -> bool) -> Result<(), ()> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    if !current() { return Err(()); }
    let mut child = Command::new("/usr/bin/xdg-open").arg(url)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|_| ())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if !current() || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(());
        }
        match child.try_wait() {
            Ok(Some(status)) => return if status.success() && current() { Ok(()) } else { Err(()) },
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => { let _ = child.kill(); let _ = child.wait(); return Err(()); }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
#[test]
fn linux_browser_rejects_invalid_or_revoked_requests() {
    for input in [json!({"url":"file:///tmp/test"}), json!({"url":"https://user:pass@example.com"}), json!({"url":"javascript:alert(1)"})] {
        assert!(invoke("browser.open", input, &|| true).is_err());
    }
    assert!(invoke("browser.open", json!({"url":"https://example.com"}), &|| false).is_err());
}



pub(super) fn invoke(cap: &str, input: Value, current: &dyn Fn() -> bool) -> Result<Value, ()> {
    let o = input.as_object().ok_or(())?;
    if !current() {
        return Err(());
    }
    match cap {
        "system.read" if o.is_empty() => {
            let mut system = sysinfo::System::new();
            system.refresh_cpu_usage();
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            if !current() {
                return Err(());
            }
            system.refresh_cpu_usage();
            system.refresh_memory();
            let bound = |n: u64| n.min(i32::MAX as u64);
            let short = |s: String| s.chars().take(120).collect::<String>();
            let used_memory = system.used_memory();
            let total_memory = system.total_memory();
            let ram_usage_percent = percent(used_memory, total_memory);
            let output = json!({"os":short(sysinfo::System::name().unwrap_or_else(||std::env::consts::OS.into())),"hostname":short(sysinfo::System::host_name().unwrap_or_default()),"cpu_usage":system.global_cpu_usage().round().clamp(0.0,100.0)as i32,"ram_used_mb":bound(used_memory/1048576),"ram_total_mb":bound(total_memory/1048576),"ram_usage_percent":ram_usage_percent,"uptime_seconds":bound(sysinfo::System::uptime())});
            if !current() {
                return Err(());
            }
            Ok(output)
        }
        "browser.open" if o.len() == 1 => {
            let url = o
                .get("url")
                .and_then(Value::as_str)
                .filter(|s| s.chars().count() <= 2048 && !s.contains('\0'))
                .ok_or(())?;
            let parsed = reqwest::Url::parse(url).map_err(|_| ())?;
            if !matches!(parsed.scheme(), "http" | "https")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err(());
            }
            #[cfg(windows)]
            {
                use windows_sys::Win32::UI::{
                    Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL,
                };
                let url: Vec<u16> = parsed.as_str().encode_utf16().chain(Some(0)).collect();
                if !current() {
                    return Err(());
                }
                let result = unsafe {
                    ShellExecuteW(
                        std::ptr::null_mut(),
                        windows_sys::core::w!("open"),
                        url.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null(),
                        SW_SHOWNORMAL,
                    )
                };
                if result as isize <= 32 {
                    return Err(());
                }
                Ok(json!({"opened":true}))
            }
            #[cfg(target_os = "linux")]
            {
                linux_open_url(parsed.as_str(), current)?;
                Ok(json!({"opened":true}))
            }
            #[cfg(not(any(windows, target_os = "linux")))]
            Err(())
        }
        "power.write" => {
            let action = o
                .get("action")
                .and_then(Value::as_str)
                .filter(|s| {
                    matches!(
                        *s,
                        "lock" | "sleep" | "shutdown" | "restart" | "cancel_shutdown"
                    )
                })
                .ok_or(())?;
            let (delay, force) = power_options(o, action)?;
            #[cfg(windows)]
            {
                power(action, delay, force, current)?;
                Ok(json!({"accepted":true}))
            }
            #[cfg(not(windows))]
            {
                let _ = (action, delay, force);
                Err(())
            }
        }
        #[cfg(windows)]
        "keyboard.write" | "media.write" | "clipboard.read" | "clipboard.write" | "apps.read"
        | "apps.write" => windows::invoke(cap, input, current),
        #[cfg(windows)]
        "display.read" | "display.write" | "screenshot.read" | "effects.write" => {
            visual::invoke(cap, input, current)
        }
        // Speech needs an owned context lifetime and is dispatched by main.rs.
        // The borrowed synchronous provider guard cannot own ongoing playback.
        "speech.write" => Err(()),
        _ => Err(()),
    }
}

fn percent(used: u64, total: u64) -> i32 {
    if total == 0 {
        0
    } else {
        (used as f64 / total as f64 * 100.0)
            .round()
            .clamp(0.0, 100.0) as i32
    }
}

fn power_options(o: &serde_json::Map<String, Value>, action: &str) -> Result<(u32, bool), ()> {
    if o.keys()
        .any(|k| !matches!(k.as_str(), "action" | "delay_seconds" | "force"))
    {
        return Err(());
    }
    if !matches!(action, "shutdown" | "restart") && o.len() != 1 {
        return Err(());
    }
    let delay = match o.get("delay_seconds") {
        None => 0,
        Some(v) => v.as_u64().filter(|n| *n <= 86400).ok_or(())? as u32,
    };
    let force = match o.get("force") {
        None => false,
        Some(v) => v.as_bool().ok_or(())?,
    };
    Ok((delay, force))
}

#[cfg(windows)]
fn power(action: &str, delay: u32, force: bool, current: &dyn Fn() -> bool) -> Result<(), ()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_NOT_ALL_ASSIGNED, HANDLE, LUID},
        Security::*,
        System::{
            Power::SetSuspendState,
            Shutdown::*,
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
    };
    if action == "lock" {
        if !current() {
            return Err(());
        }
        return if unsafe { LockWorkStation() } != 0 {
            Ok(())
        } else {
            Err(())
        };
    }
    struct Privilege {
        token: HANDLE,
        previous: TOKEN_PRIVILEGES,
    }
    impl Drop for Privilege {
        fn drop(&mut self) {
            unsafe {
                AdjustTokenPrivileges(
                    self.token,
                    0,
                    &self.previous,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                CloseHandle(self.token);
            }
        }
    }
    let mut token = std::ptr::null_mut();
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    } == 0
    {
        return Err(());
    }
    let mut guard = Privilege {
        token,
        previous: unsafe { std::mem::zeroed() },
    };
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    if unsafe { LookupPrivilegeValueW(std::ptr::null(), SE_SHUTDOWN_NAME, &mut luid) } == 0 {
        return Err(());
    }
    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let mut needed = 0;
    if unsafe {
        AdjustTokenPrivileges(
            token,
            0,
            &privileges,
            std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
            &mut guard.previous,
            &mut needed,
        )
    } == 0
        || unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED
    {
        return Err(());
    }
    if !current() {
        return Err(());
    }
    let okay = match action {
        "sleep" => unsafe { SetSuspendState(false, false, false) },
        "shutdown" | "restart" => unsafe {
            InitiateSystemShutdownExW(
                std::ptr::null(),
                std::ptr::null(),
                delay,
                i32::from(force),
                i32::from(action == "restart"),
                SHTDN_REASON_MAJOR_APPLICATION
                    | SHTDN_REASON_MINOR_OTHER
                    | SHTDN_REASON_FLAG_PLANNED,
            ) != 0
        },
        "cancel_shutdown" => unsafe { AbortSystemShutdownW(std::ptr::null()) != 0 },
        _ => false,
    };
    if okay {
        Ok(())
    } else {
        Err(())
    }
}
