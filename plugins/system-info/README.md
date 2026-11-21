# System Info example plugin

A standalone native Rust plugin for Windows and Linux x86-64. It implements the RCHub v2 JSONL lifecycle and the `system.info` request. It collects OS name and version, architecture, CPU brand and logical CPU count, and total and available physical memory. It does not request or return hostnames, usernames, network addresses, disk contents, or process lists. OS and CPU details can still identify a device when combined with other information.

## Interface and language

The bundled interface reads the desktop host's document-start locale from `window.RCHub.locale`. It uses Turkish only for `tr` and English for `en` or any missing or unrecognized value, so the same archive works with hosts that do not provide the property. It sets `<html lang>`, localizes static and dynamic text, status messages, error text, document title and accessibility descriptions, and formats numbers, percentages and snapshot times for the selected locale.

The panel leads with a memory-usage gauge, used and available figures, and a used/available bar, followed by compact iconized rows for operating system, architecture, processor and logical cores. All values come from the same `system.info` response. Zero, null or inconsistent values are shown as unavailable instead of being turned into derived claims. In particular, the backend forwards `sysinfo`'s available-memory value without a separate success indicator: zero can mean either no available memory or an unavailable reading, so the UI cannot safely distinguish the two. Snapshots are timestamped manual captures, not live monitoring; refreshing keeps the previous snapshot visible if the next request fails. Loading, unavailable and error states are explicit, feedback is limited to status text and a refresh-icon rotation, and motion is suppressed when the operating system requests reduced motion. The interface title is **System Info** in English and **Sistem Bilgisi** in Turkish.

Memory usage uses the displayed rounded percentage: below 70% is blue, 70–89% is orange, and 90% or more is red. A localized level label also communicates the state without relying on color. These are display thresholds, not performance guarantees.

## Packaging

Install Rust/Cargo and Python 3. On each target operating system, run from this directory:

```sh
python package.py
```

On Windows, `py -3 package.py` is an alternative. The script builds for the current x86-64 Rust host target and produces `system-info-plugin-windows-x86_64.zip` or `system-info-plugin-linux-x86_64.zip`. Each archive contains `manifest.json`, its platform executable under `bin/`, and the same self-contained interface at `ui/desktop/index.html` and `ui/android/index.html`; build separately on each OS. The manifest keeps the original `name` and also emits `localizedNames` with `tr` and `en` titles. Hosts that ignore unknown metadata fall back to the original name; a strict older host that rejects unknown manifest fields may need an update before this package installs. Build separately on each OS: building on one OS does not establish that the other platform works.

Native plugins run with the current user's OS permissions and are **not sandboxed**. Install only packages you trust; a digest alone does not authenticate a publisher.

## Protocol

Stdin and stdout contain UTF-8 JSON objects, one per line, maximum 64 KiB including the newline. Stdout is reserved for protocol messages; diagnostics go to stderr. The host sends `{"v":2,"type":"hello"}`; the plugin replies and flushes `{"v":2,"type":"ready"}`. The host can then send requests and finally `{"v":2,"type":"stop"}`, after which the plugin exits.

The bundled interface uses `window.RCHub.request('system.info', {})`, a host-provided Promise returning the result or rejecting on error. It loads a snapshot on opening and lets the user refresh. The UI does not directly access native APIs or other plugins.

Request:

```json
{"v":2,"type":"request","id":"request-1","method":"system.info","params":{}}
```

Successful response (example values):

```json
{"v":2,"type":"response","id":"request-1","ok":true,"result":{"os":{"name":"Linux","version":"6.0","architecture":"x86_64"},"cpu":{"brand":"Example CPU","logical_cores":8},"memory":{"total_bytes":17179869184,"available_bytes":8589934592}}}
```

`os.name`, `os.version`, and `cpu.brand` can be null if unavailable; `logical_cores` can be zero if unavailable. Memory fields are nonnegative integer bytes and may be zero when unavailable. Available memory is a snapshot, not a guarantee that applications can allocate that amount. Architecture uses Rust's target architecture string. Values reflect what the operating system exposes, including container or virtualization effects.

An unknown method returns `{"v":2,"type":"response","id":"request-1","ok":false,"error":{"code":"method_not_found","message":"Unknown method"}}`. A nonempty or missing `params` object for `system.info` returns `invalid_params`. A malformed protocol message, invalid/missing request ID, unsupported version, oversized line, or premature EOF terminates the process. IDs must be nonempty strings and are echoed unchanged.
