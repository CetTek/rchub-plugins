# Power Options example

This standalone protocol v2 native plugin provides **lock, restart and shutdown** for Windows x86-64, with opt-in delays, force control, cancellation and pending-status reporting. It has desktop and Android web surfaces. It is not packaged for Linux: Linux desktop environments and session power services differ, and support has not been implemented or verified. Both surfaces are localized in Turkish and English.

## Build and install

On Windows x86-64, install Rust/Cargo and Python 3, then run `python examples/power-plugin/package.py` from `new-app/desktop` (or use `py -3`). The script builds the current matching Rust host target and creates `examples/power-plugin/power-plugin-windows-x86_64.zip`. Install it through the desktop local package picker, review and accept the native-code warning, and start it before opening its desktop surface or selecting it on an approved Android device. Package creation builds executable code; installation and startup do not by themselves perform a power action.

The 0.2.1 package declares `localizedNames` (`Güç Seçenekleri` in Turkish, `Power Options` in English). Both surfaces read `window.RCHub.locale`, accept `tr` or `en`, fall back to English, and set `<html lang>`; visible copy, accessibility labels, dialogs and known bridge errors are localized, while action identifiers and request fields are unchanged. Installing this package requires a host that accepts `localizedNames`; older strict hosts reject the new manifest, and previously installed packages keep their original name. A host that does not inject `locale` shows the English UI.

## Contract

After the v2 hello/ready handshake:

- `power.execute` accepts exactly `{"action":"lock"|"restart"|"shutdown","confirmed":true,"delaySeconds":<integer>,"force":<boolean>}`. Any missing, extra or mistyped parameter is rejected with `invalid_params`; `confirmed` must be `true`.
- `delaySeconds` must be `0..315360000` for shutdown/restart and `0..86400` for lock. Values outside the action's range are rejected.
- `power.cancel` accepts exactly `{}`. It runs `shutdown.exe /a` for a pending shutdown/restart, or cancels a pending plugin lock timer. If nothing is pending it succeeds with `{"cancelled":false}` instead of failing.
- `power.status` accepts exactly `{}` and reports only operations this plugin instance knows about, for example `{"pending":true,"action":"shutdown","remainingSeconds":120,"force":false}` or `{"pending":false}`.

Command construction:

- Shutdown runs `shutdown.exe /s /t <delaySeconds>`; when force is requested it adds `/f`.
- Restart runs `shutdown.exe /r /t <delaySeconds>`; when force is requested it adds `/f`. Windows supports `/r` with `/t`.
- Immediate lock (delay 0) runs `rundll32.exe user32.dll,LockWorkStation`.
- Delayed lock is a background timer inside the plugin; the protocol loop is never blocked. The timer only lives while the plugin runs: if the plugin stops or is disabled before the deadline, the lock is cancelled.

Windows documentation states that `/f` is implied when `/t` is greater than zero. The UI therefore tells the user that delayed shutdowns and restarts force applications to close even when the Force checkbox is unchecked. The plugin still includes `/f` only when requested; the operating system applies its own rule.

Multiple pending operations are handled explicitly. A new shutdown/restart first aborts any shutdown/restart this plugin previously scheduled (`shutdown.exe /a`) and then schedules the replacement; if the abort fails, the new request fails and the old tracked state is retained. A new lock request replaces a pending plugin lock timer. A lock is rejected with `operation_conflict` while a shutdown/restart is pending, and the caller must cancel that operation first. An immediate command (delay 0) is not tracked as pending after the command exits.

## Safety

The surfaces display an action-specific confirmation dialog that includes the delay and, for restart and shutdown, the force choice, with Cancel before making the request; cancellation makes no request. Locking has no force option. The pending panel shows the remaining time and offers "Cancel pending operation". The plugin waits for each OS command to exit: failure to launch or a nonzero exit produces `operation_failed`; a zero exit produces a success result. This reports the command's exit status, not proof that Windows completed the operation. In particular, `rundll32.exe` success does not establish that the intended session was locked. An error, timeout, or lost connection can occur after an operation started; **never automatically retry a power action**.

Confirmation is a UI safeguard, not an authorization or security boundary: any authorized caller able to send plugin requests can set `confirmed` itself. Native plugins are unsandboxed and execute with the current user's permissions. Use only on a trusted computer with approved devices. Windows policy, permissions, sessions and other applications may prevent or delay an operation. Connections may be lost as a consequence. Plugin-side lock timers are lost when the plugin process exits; OS-scheduled shutdown/restart timers continue independently.

The process launches Windows system `shutdown.exe` and `rundll32.exe` with fixed arguments only. No shell or user-provided command arguments are used, and neither the raw delay input nor any other text is passed to a shell. No operation is automatically invoked at startup, during packaging, or on cancellation.

## Verification

Run these from this directory:

```sh
cargo test
cargo build --release
python package.py
```

`cargo test` covers pure command-line construction, parameter/delay validation and the conflict between a pending shutdown/restart and a lock request in `src/main.rs`; it does not run `shutdown.exe`, `rundll32.exe` or any live power operation. `package.py` rebuilds the release executable and rewrites `power-plugin-windows-x86_64.zip` with manifest version 0.2.1.

Packaging, v2 lifecycle, malformed requests, confirmation/cancel, disabled pending state, errors and timeout handling need end-to-end verification on Windows and on both actual desktop and Android surfaces. Exercise actual power operations only with explicit permission, on a suitable disposable machine, after saving work. A build or successful command exit alone cannot establish that an action occurred. Android pairing, delivery, and session behavior require separate end-to-end verification.
