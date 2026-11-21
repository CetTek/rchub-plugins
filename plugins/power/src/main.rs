use std::io::{self, BufRead, Read, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

const MAX_LINE: usize = 64 * 1024;
const SHUTDOWN_MAX_DELAY_SECONDS: u64 = 315_360_000;
const LOCK_MAX_DELAY_SECONDS: u64 = 86_400;

fn check_exit_status(status: std::process::ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("Power command failed with {status}"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowerAction {
    Shutdown,
    Restart,
    Lock,
}

impl PowerAction {
    fn as_str(self) -> &'static str {
        match self {
            PowerAction::Shutdown => "shutdown",
            PowerAction::Restart => "restart",
            PowerAction::Lock => "lock",
        }
    }

    fn max_delay_seconds(self) -> u64 {
        match self {
            PowerAction::Lock => LOCK_MAX_DELAY_SECONDS,
            PowerAction::Shutdown | PowerAction::Restart => SHUTDOWN_MAX_DELAY_SECONDS,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CommandSpec {
    program: &'static str,
    args: Vec<String>,
}

/// Pure command construction so tests never touch the operating system.
fn build_power_command(action: PowerAction, delay_seconds: u64, force: bool) -> Result<CommandSpec, String> {
    validate_delay(action, delay_seconds)?;
    match action {
        PowerAction::Shutdown | PowerAction::Restart => {
            let mut args = vec![
                if action == PowerAction::Shutdown { "/s".to_string() } else { "/r".to_string() },
                "/t".to_string(),
                delay_seconds.to_string(),
            ];
            if force {
                args.push("/f".to_string());
            }
            Ok(CommandSpec { program: "shutdown.exe", args })
        }
        PowerAction::Lock if delay_seconds == 0 => Ok(CommandSpec {
            program: "rundll32.exe",
            args: vec!["user32.dll,LockWorkStation".to_string()],
        }),
        PowerAction::Lock => Err("A delayed lock is scheduled by the plugin, not by an immediate command".into()),
    }
}

fn validate_delay(action: PowerAction, delay_seconds: u64) -> Result<(), String> {
    let max = action.max_delay_seconds();
    if delay_seconds > max {
        return Err(format!(
            "delaySeconds for {} must be between 0 and {max}",
            action.as_str()
        ));
    }
    Ok(())
}

fn cancel_command() -> CommandSpec {
    CommandSpec { program: "shutdown.exe", args: vec!["/a".to_string()] }
}

#[cfg(target_os = "windows")]
fn run_command(spec: &CommandSpec) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let system_root = std::env::var_os("SystemRoot").ok_or("SystemRoot is unavailable")?;
    let system32 = std::path::PathBuf::from(system_root).join("System32");
    let status = Command::new(system32.join(spec.program))
        .args(&spec.args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("Could not run power command: {error}"))?;
    check_exit_status(status)
}

#[cfg(not(target_os = "windows"))]
fn run_command(_spec: &CommandSpec) -> Result<(), String> {
    Err("Power operations are supported only on Windows".into())
}

#[derive(Debug)]
enum Pending {
    /// Windows owns the timer; the plugin only remembers its last schedule.
    System { action: PowerAction, deadline: Instant, force: bool },
    /// The plugin owns the timer. `generation` invalidates an older thread.
    Lock { deadline: Instant, generation: u64 },
}

impl Pending {
    fn action(&self) -> PowerAction {
        match self {
            Pending::System { action, .. } => *action,
            Pending::Lock { .. } => PowerAction::Lock,
        }
    }
}

#[derive(Debug, Default)]
struct PendingState {
    pending: Option<Pending>,
    next_generation: u64,
}

fn state_lock(state: &Mutex<PendingState>) -> MutexGuard<'_, PendingState> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The OS executes a scheduled shutdown/restart itself, so once its deadline
/// passes there is nothing left to cancel and it must not be reported.
fn prune_expired_system(state: &mut PendingState) {
    let now = Instant::now();
    if matches!(&state.pending, Some(Pending::System { deadline, .. }) if *deadline <= now) {
        state.pending = None;
    }
}

#[derive(Debug)]
struct RequestError {
    code: &'static str,
    message: String,
}

impl RequestError {
    fn invalid(message: impl Into<String>) -> Self {
        Self { code: "invalid_params", message: message.into() }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self { code: "operation_failed", message: message.into() }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self { code: "operation_conflict", message: message.into() }
    }

    fn into_json(self) -> Value {
        json!({ "code": self.code, "message": self.message })
    }
}

fn remaining_seconds(deadline: Instant) -> u64 {
    let remaining = deadline.saturating_duration_since(Instant::now());
    ((remaining.as_millis() + 999) / 1000) as u64
}

fn parse_execute(params: Option<&Map<String, Value>>) -> Result<(PowerAction, u64, bool), RequestError> {
    let params = params.ok_or_else(|| RequestError::invalid("power.execute requires params"))?;
    if params.len() != 4 {
        return Err(RequestError::invalid(
            "power.execute requires exactly action, confirmed, delaySeconds and force",
        ));
    }
    let action = match params.get("action").and_then(Value::as_str) {
        Some("shutdown") => PowerAction::Shutdown,
        Some("restart") => PowerAction::Restart,
        Some("lock") => PowerAction::Lock,
        _ => return Err(RequestError::invalid("action must be shutdown, restart or lock")),
    };
    if params.get("confirmed").and_then(Value::as_bool) != Some(true) {
        return Err(RequestError::invalid("confirmed must be true"));
    }
    let delay_seconds = params
        .get("delaySeconds")
        .and_then(Value::as_u64)
        .ok_or_else(|| RequestError::invalid("delaySeconds must be a non-negative integer"))?;
    let force = params
        .get("force")
        .and_then(Value::as_bool)
        .ok_or_else(|| RequestError::invalid("force must be a boolean"))?;
    validate_delay(action, delay_seconds).map_err(RequestError::invalid)?;
    Ok((action, delay_seconds, force))
}

fn spawn_lock_timer(state: Arc<Mutex<PendingState>>, delay_seconds: u64, generation: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(delay_seconds));
        {
            let mut guard = state_lock(&state);
            let still_current = matches!(
                &guard.pending,
                Some(Pending::Lock { generation: current, .. }) if *current == generation
            );
            if !still_current {
                return;
            }
            guard.pending = None;
        }
        // Deliberately not fatal: the protocol loop must keep serving requests.
        if let Err(error) = run_command(&CommandSpec {
            program: "rundll32.exe",
            args: vec!["user32.dll,LockWorkStation".to_string()],
        }) {
            eprintln!("power plugin: delayed lock failed: {error}");
        }
    });
}

fn execute_power(
    state: &Arc<Mutex<PendingState>>,
    action: PowerAction,
    delay_seconds: u64,
    force: bool,
) -> Result<Value, RequestError> {
    match action {
        PowerAction::Shutdown | PowerAction::Restart => {
            let mut guard = state_lock(state);
            prune_expired_system(&mut guard);
            let replaced = guard.pending.as_ref().map(|pending| pending.action().as_str());
            if matches!(&guard.pending, Some(Pending::System { .. })) {
                // A new schedule safely replaces the previous one only after the
                // old OS timer is aborted.
                run_command(&cancel_command()).map_err(|error| {
                    RequestError::failed(format!(
                        "Could not cancel the previously scheduled operation: {error}"
                    ))
                })?;
            }
            guard.pending = None;
            run_command(&build_power_command(action, delay_seconds, force).map_err(RequestError::failed)?)
                .map_err(RequestError::failed)?;
            if delay_seconds > 0 {
                guard.pending = Some(Pending::System {
                    action,
                    deadline: Instant::now() + Duration::from_secs(delay_seconds),
                    force,
                });
            }
            Ok(json!({
                "status": if delay_seconds > 0 { "scheduled" } else { "started" },
                "action": action.as_str(),
                "delaySeconds": delay_seconds,
                "force": force,
                "replaced": replaced,
            }))
        }
        PowerAction::Lock => {
            let mut guard = state_lock(state);
            prune_expired_system(&mut guard);
            if matches!(&guard.pending, Some(Pending::System { .. })) {
                return Err(RequestError::conflict(
                    "A shutdown or restart is pending; cancel it before locking",
                ));
            }
            let replaced = guard.pending.as_ref().map(|pending| pending.action().as_str());
            if delay_seconds == 0 {
                guard.pending = None;
                drop(guard);
                run_command(&build_power_command(action, 0, false).map_err(RequestError::failed)?)
                    .map_err(RequestError::failed)?;
                Ok(json!({
                    "status": "started",
                    "action": "lock",
                    "delaySeconds": 0,
                    "force": false,
                    "replaced": replaced,
                }))
            } else {
                guard.next_generation += 1;
                let generation = guard.next_generation;
                guard.pending = Some(Pending::Lock {
                    deadline: Instant::now() + Duration::from_secs(delay_seconds),
                    generation,
                });
                drop(guard);
                spawn_lock_timer(Arc::clone(state), delay_seconds, generation);
                Ok(json!({
                    "status": "scheduled",
                    "action": "lock",
                    "delaySeconds": delay_seconds,
                    "force": false,
                    "replaced": replaced,
                }))
            }
        }
    }
}

fn cancel_pending(state: &Mutex<PendingState>) -> Result<Value, RequestError> {
    let mut guard = state_lock(state);
    prune_expired_system(&mut guard);
    match guard.pending.take() {
        None => Ok(json!({ "cancelled": false })),
        Some(Pending::Lock { .. }) => Ok(json!({ "cancelled": true, "action": "lock" })),
        Some(Pending::System { action, deadline, force }) => {
            // Keep the record until the abort succeeds so status stays truthful
            // if `shutdown /a` reports a failure.
            guard.pending = Some(Pending::System { action, deadline, force });
            run_command(&cancel_command()).map_err(|error| {
                RequestError::failed(format!("Could not cancel the pending operation: {error}"))
            })?;
            guard.pending = None;
            Ok(json!({ "cancelled": true, "action": action.as_str() }))
        }
    }
}

fn pending_status(state: &Mutex<PendingState>) -> Value {
    let mut guard = state_lock(state);
    prune_expired_system(&mut guard);
    match &guard.pending {
        None => json!({ "pending": false }),
        Some(Pending::System { action, deadline, force }) => json!({
            "pending": true,
            "action": action.as_str(),
            "remainingSeconds": remaining_seconds(*deadline),
            "force": force,
        }),
        Some(Pending::Lock { deadline, .. }) => json!({
            "pending": true,
            "action": "lock",
            "remainingSeconds": remaining_seconds(*deadline),
            "force": false,
        }),
    }
}

fn dispatch(
    method: &str,
    params: Option<&Map<String, Value>>,
    state: &Arc<Mutex<PendingState>>,
) -> Result<Value, RequestError> {
    let empty_params = || {
        params
            .filter(|params| params.is_empty())
            .ok_or_else(|| RequestError::invalid("This method requires an empty params object"))
    };
    match method {
        "power.execute" => {
            let (action, delay_seconds, force) = parse_execute(params)?;
            execute_power(state, action, delay_seconds, force)
        }
        "power.cancel" => {
            empty_params()?;
            cancel_pending(state)
        }
        "power.status" => {
            empty_params()?;
            Ok(pending_status(state))
        }
        _ => Err(RequestError { code: "method_not_found", message: "Unknown method".into() }),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut line = Vec::new();
    let mut ready = false;
    let state = Arc::new(Mutex::new(PendingState::default()));
    loop {
        line.clear();
        let bytes = input.by_ref().take((MAX_LINE + 1) as u64).read_until(b'\n', &mut line)?;
        if bytes == 0 {
            return Err("host closed stdin".into());
        }
        if line.len() > MAX_LINE || !line.ends_with(b"\n") {
            return Err("invalid or oversized protocol line".into());
        }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("v").and_then(Value::as_u64) != Some(2) {
            return Err("unsupported protocol version".into());
        }
        match (ready, message.get("type").and_then(Value::as_str)) {
            (false, Some("hello")) => {
                output.write_all(b"{\"v\":2,\"type\":\"ready\"}\n")?;
                output.flush()?;
                ready = true;
            }
            (true, Some("stop")) => return Ok(()),
            (true, Some("request")) => {
                let id = message
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or("request requires a nonempty string id")?;
                let response = match message.get("method").and_then(Value::as_str) {
                    Some(method) => match dispatch(method, message.get("params").and_then(Value::as_object), &state) {
                        Ok(result) => json!({"v":2,"type":"response","id":id,"ok":true,"result":result}),
                        Err(error) => json!({"v":2,"type":"response","id":id,"ok":false,"error":error.into_json()}),
                    },
                    None => json!({"v":2,"type":"response","id":id,"ok":false,"error":{"code":"method_not_found","message":"Unknown method"}}),
                };
                serde_json::to_writer(&mut output, &response)?;
                output.write_all(b"\n")?;
                output.flush()?;
            }
            _ => return Err("unexpected protocol message".into()),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("power plugin: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    fn args(spec: &CommandSpec) -> Vec<&str> {
        spec.args.iter().map(String::as_str).collect()
    }

    #[test]
    fn exit_status_controls_success() {
        assert!(check_exit_status(status(0)).is_ok());
        assert!(check_exit_status(status(1)).unwrap_err().contains('1'));
    }

    #[test]
    fn shutdown_command_contains_delay_and_force() {
        let spec = build_power_command(PowerAction::Shutdown, 300, true).unwrap();
        assert_eq!(spec.program, "shutdown.exe");
        assert_eq!(args(&spec), ["/s", "/t", "300", "/f"]);
    }

    #[test]
    fn restart_command_omits_force_when_not_requested() {
        let spec = build_power_command(PowerAction::Restart, 0, false).unwrap();
        assert_eq!(spec.program, "shutdown.exe");
        assert_eq!(args(&spec), ["/r", "/t", "0"]);
    }

    #[test]
    fn restart_command_includes_force_when_requested() {
        let spec = build_power_command(PowerAction::Restart, 60, true).unwrap();
        assert_eq!(args(&spec), ["/r", "/t", "60", "/f"]);
    }

    #[test]
    fn immediate_lock_uses_rundll32() {
        let spec = build_power_command(PowerAction::Lock, 0, false).unwrap();
        assert_eq!(spec.program, "rundll32.exe");
        assert_eq!(args(&spec), ["user32.dll,LockWorkStation"]);
    }

    #[test]
    fn delayed_lock_has_no_immediate_command() {
        assert!(build_power_command(PowerAction::Lock, 10, false).is_err());
    }

    #[test]
    fn delay_bounds_are_action_specific() {
        assert!(validate_delay(PowerAction::Lock, LOCK_MAX_DELAY_SECONDS).is_ok());
        assert!(validate_delay(PowerAction::Lock, LOCK_MAX_DELAY_SECONDS + 1).is_err());
        assert!(validate_delay(PowerAction::Shutdown, SHUTDOWN_MAX_DELAY_SECONDS).is_ok());
        assert!(validate_delay(PowerAction::Shutdown, SHUTDOWN_MAX_DELAY_SECONDS + 1).is_err());
        assert!(validate_delay(PowerAction::Restart, 315_360_000).is_ok());
        assert!(validate_delay(PowerAction::Restart, 315_360_001).is_err());
    }

    #[test]
    fn cancel_uses_shutdown_abort() {
        let spec = cancel_command();
        assert_eq!(spec.program, "shutdown.exe");
        assert_eq!(args(&spec), ["/a"]);
    }

    #[test]
    fn execute_params_require_exact_confirmed_shape() {
        let valid: Map<String, Value> = serde_json::from_value(json!({
            "action": "shutdown",
            "confirmed": true,
            "delaySeconds": 30,
            "force": false,
        }))
        .unwrap();
        assert_eq!(parse_execute(Some(&valid)).unwrap(), (PowerAction::Shutdown, 30, false));

        let mut extra = valid.clone();
        extra.insert("timeout".into(), json!(1));
        assert!(parse_execute(Some(&extra)).is_err());

        let mut unconfirmed = valid.clone();
        unconfirmed.insert("confirmed".into(), json!(false));
        assert!(parse_execute(Some(&unconfirmed)).is_err());

        let mut negative = valid.clone();
        negative.insert("delaySeconds".into(), json!(-1));
        assert!(parse_execute(Some(&negative)).is_err());

        let mut too_long = valid;
        too_long.insert("action".into(), json!("lock"));
        too_long.insert("delaySeconds".into(), json!(LOCK_MAX_DELAY_SECONDS + 1));
        assert!(parse_execute(Some(&too_long)).is_err());
    }

    #[test]
    fn status_and_cancel_handle_nothing_pending() {
        let state = Arc::new(Mutex::new(PendingState::default()));
        let empty = Map::new();
        assert_eq!(
            dispatch("power.status", Some(&empty), &state).unwrap(),
            json!({ "pending": false })
        );
        assert_eq!(
            dispatch("power.cancel", Some(&empty), &state).unwrap(),
            json!({ "cancelled": false })
        );
    }

    #[test]
    fn status_reports_a_tracked_os_schedule() {
        let state = Arc::new(Mutex::new(PendingState {
            pending: Some(Pending::System {
                action: PowerAction::Restart,
                deadline: Instant::now() + Duration::from_secs(120),
                force: true,
            }),
            next_generation: 0,
        }));
        let value = dispatch("power.status", Some(&Map::new()), &state).unwrap();
        assert_eq!(value["pending"], json!(true));
        assert_eq!(value["action"], json!("restart"));
        assert_eq!(value["force"], json!(true));
        let remaining = value["remainingSeconds"].as_u64().unwrap();
        assert!((119..=120).contains(&remaining), "unexpected remaining: {remaining}");
    }

    #[test]
    fn pending_system_operation_rejects_lock_without_changing_schedule() {
        for action in [PowerAction::Shutdown, PowerAction::Restart] {
            for delay in [0, 20] {
                let deadline = Instant::now() + Duration::from_secs(600);
                let state = Arc::new(Mutex::new(PendingState {
                    pending: Some(Pending::System { action, deadline, force: true }),
                    next_generation: 0,
                }));
                let params = json!({"action": "lock", "confirmed": true, "delaySeconds": delay, "force": false});
                let error = dispatch("power.execute", params.as_object(), &state).unwrap_err();
                assert_eq!(error.code, "operation_conflict");
                assert!(matches!(
                    &state_lock(&state).pending,
                    Some(Pending::System { action: current, deadline: current_deadline, force: true })
                        if *current == action && *current_deadline == deadline
                ));
            }
        }
    }

    #[test]
    fn cancel_clears_a_pending_plugin_lock() {
        let state = Arc::new(Mutex::new(PendingState {
            pending: Some(Pending::Lock {
                deadline: Instant::now() + Duration::from_secs(600),
                generation: 1,
            }),
            next_generation: 1,
        }));
        assert_eq!(
            dispatch("power.cancel", Some(&Map::new()), &state).unwrap(),
            json!({ "cancelled": true, "action": "lock" })
        );
        assert!(state_lock(&state).pending.is_none());
    }

    #[test]
    fn status_rejects_extra_params() {
        let state = Arc::new(Mutex::new(PendingState::default()));
        let mut params = Map::new();
        params.insert("extra".into(), json!(1));
        let error = dispatch("power.status", Some(&params), &state).unwrap_err();
        assert_eq!(error.code, "invalid_params");
    }
}
