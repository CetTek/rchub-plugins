# Clipboard example plugin

A Windows x86-64 native plugin with desktop and Android HTML surfaces for explicitly reading and writing the computer's **text** clipboard. It does not synchronize automatically, or transfer images or files. On the Expo mobile/web surface, reading nonempty text also attempts to copy it locally and reveals a Copy button; editing the text or beginning another read hides it. Other surfaces show the read text without attempting local copy. Clipboard contents can include passwords and other sensitive data: an approved remote device can read or replace them while the plugin is running. Native plugins run with the current user's permissions and are **not sandboxed**. Install only trusted packages and stop the plugin when it is not needed.

## Packaging and use

Install Rust/Cargo and Python 3 on Windows x86-64, then run `python package.py` (or `py -3 package.py`) in this directory. The script builds a native binary and creates `clipboard-plugin-windows-x86_64.zip`, containing a protocol 2 manifest, the executable under `bin/`, and the same self-contained HTML under both `ui/desktop/` and `ui/android/`. Install the archive in the desktop app, review the native-code warning, start the plugin, and open its desktop surface or select it on an approved Android device. A running graphical Windows session and available clipboard are required. Linux is not supported by this example; no Linux artifact is advertised.

The 0.2.1 package declares `localizedNames` (`Pano` in Turkish, `Clipboard` in English). The surface reads `window.RCHub.locale`, accepts `tr` or `en`, falls back to English, and sets `<html lang>`; visible text, accessibility labels and known bridge errors are localized. Clipboard data, method names and error codes are never translated. Installing this package requires a host that accepts `localizedNames`; older strict hosts reject the new manifest, while already installed older packages keep their original name. A surface host that does not inject `locale` shows the English UI.

The host rejects installing another package with the same ID. If this example is already installed, uninstall it using the host's normal controls before installing the rebuilt package; preserve any data you need first. Updating an installed package in place is not supported.

## Protocol

The process uses the desktop host's v2 UTF-8 JSON-lines protocol: `hello` / `ready`, then requests, and `stop`. Lines are limited to 64 KiB including newline; stdout is reserved for protocol and diagnostics go to stderr. Requests require a nonempty ID of at most 128 UTF-8 bytes. Malformed framing or messages terminate the process. The surface uses `window.RCHub.request(method, params)` and never polls automatically.

* Text is limited to 256 KiB (262144 UTF-8 bytes), with NUL rejected. `clipboard.read.begin` creates a snapshot and returns a capability and byte length; `clipboard.read.chunk` reads ordered chunks of at most 4096 UTF-8 bytes. `clipboard.write.begin`, `.chunk`, and `.commit` stage and then atomically request a clipboard replacement; `clipboard.cancel` discards a transfer. Chunks preserve Unicode boundaries and fit existing JSON transport limits even for escaped control characters.
* Transfers use random 128-bit capabilities, at most four concurrent transfers, and expire after 60 seconds of inactivity, with expired entries removed on the next request. A failed or incomplete write does not touch the computer clipboard. The host does not bind transfers to a device identity: anyone who obtains a capability can use it. Transport quotas remain in force, and timeouts can leave the result of a commit uncertain.
* Desktop relay responses share a rolling budget across all plugins and connected clients. Concurrent traffic can delay requests or cause timeouts; the budget does not reserve capacity for clipboard transfers and does not change hourly quotas. Each client also remains subject to its own send limits. An interrupted write is never committed automatically.
* The Expo mobile/web surface handles `client.clipboard.begin`, `.chunk`, `.commit`, and `.cancel` locally; the desktop surface does not implement device clipboard copying. The existing `client.clipboard.set` method remains available for small legacy requests.

Invalid parameters return `invalid_params`; unknown methods return `method_not_found`. Errors contain no clipboard text. Requests and responses are transported through RCHub, so they are subject to its connection, approval, timeout and privacy constraints. A timeout does not prove a write failed; do not automatically retry it.

## Verification

Build and test on Windows with a real interactive clipboard, and exercise read, write, empty text, Unicode, oversized text, unavailable clipboard, start/stop and both desktop and approved Android surfaces. Verify Android pairing and transport separately. No cross-platform or end-to-end verification is implied by this source.
