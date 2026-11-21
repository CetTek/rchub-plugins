# Web Browser example plugin

A standalone native Rust plugin with desktop and Android HTML surfaces. It opens a user-submitted **HTTP(S)** URL in the default browser of the computer running the plugin. It does not render pages inside a controller iframe, fetch or preview URLs, browse in the background, or dispatch anything without a user action.

The interface title is **Web Browser** in English and **Web Tarayıcı** in Turkish. The package advertises `localizedNames` beside the original manifest name. A host that does not understand locale metadata keeps the original name; a strict older host that rejects unknown manifest fields may require an update before this package can be installed.

## Risk and requirements

Native plugins run as independent processes with the current user's permissions and are **not sandboxed**. Install only packages you trust.

- Any approved device or controller that can reach this plugin can make the paired computer open an `http` or `https` URL. Pages can be used for phishing, unwanted downloads, or resource exhaustion through repeated opens. Use only trusted pairings and stop the plugin when it is not needed.
- The URL is passed to the operating system's default HTTP(S) handler. The plugin does not choose a browser, executable, or command-line arguments, does not follow redirects itself, and does not read page content.
- An accepted request means the operating system handed the URL to the default browser. It does not confirm that the browser started, the page loaded, or the URL was safe. A timeout does not prove the request failed; do not retry automatically.
- Windows requires a default browser association. Linux requires `xdg-open` and a desktop session that provides the default handler. If the platform opener is missing, the request fails with `launch_failed`. Other operating systems are not packaged and fail clearly if the binary is run there.
- Building requires Rust/Cargo and Python 3 on Windows or Linux x86-64. Build separately on each target OS; an artifact built on one platform is not evidence that the other platform works.

## Packaging

From this directory:

```sh
python package.py
```

On Windows, `py -3 package.py` is an alternative. The script builds the current x86-64 Rust host target and writes `web-browser-plugin-windows-x86_64.zip` or `web-browser-plugin-linux-x86_64.zip`. Each archive contains `manifest.json`, the platform executable under `bin/`, and the same self-contained interface at `ui/desktop/index.html` and `ui/android/index.html`. Install the archive through the desktop plugin picker, review the native-code warning, start the plugin, then open its desktop surface or select it on an approved Android device.

The manifest uses id `rchub.web-browser`, protocol 2, and platform-specific entrypoints:

```json
{
  "id": "rchub.web-browser",
  "name": "Web Browser",
  "localizedNames": { "tr": "Web Tarayıcı", "en": "Web Browser" },
  "version": "0.1.0",
  "protocol": 2,
  "entrypoints": { "windows-x86_64": "bin/rchub-web-browser-plugin.exe" },
  "ui": { "desktop": "ui/desktop/index.html", "android": "ui/android/index.html" }
}
```

## Protocol

Stdin and stdout carry UTF-8 JSON objects, one per line, limited to 64 KiB including the newline. Stdout is reserved for protocol messages; diagnostics go to stderr. The host sends `{"v":2,"type":"hello"}`; the plugin replies and flushes `{"v":2,"type":"ready"}`. Requests and finally `{"v":2,"type":"stop"}` follow. Request IDs must be nonempty strings of at most 128 UTF-8 bytes and are echoed unchanged. Malformed framing, unsupported versions, missing IDs, or premature EOF terminate the process.

Request:

```json
{"v":2,"type":"request","id":"request-1","method":"browser.open","params":{"url":"https://example.com/"}}
```

`browser.open` requires exactly one field, `url`, whose value is a string. A valid request returns the normalized absolute URL:

```json
{"v":2,"type":"response","id":"request-1","ok":true,"result":{"opened":true,"url":"https://example.com/"}}
```

Errors use `invalid_params`, `invalid_url`, `launch_failed`, or `method_not_found`, for example:

```json
{"v":2,"type":"response","id":"request-1","ok":false,"error":{"code":"invalid_url","message":"Only http and https URLs are supported"}}
```

There is no state, history, or automatic retry. Repeated requests open repeated browser tabs; the UI disables its controls while one request is pending, but the protocol itself does not rate-limit callers.

## URL validation

The native process validates every URL before invoking the operating system. The bundled UI mirrors the main rules so it can show localized feedback, but native validation is authoritative.

- Maximum 2048 bytes, ASCII printable characters only. Whitespace, control characters, non-ASCII characters, and backslashes are rejected. Percent-encode other characters and use punycode for internationalized host names.
- Absolute `http` or `https` URL with a nonempty host. Other schemes, including `file`, `javascript`, `data`, `ftp`, and protocol-relative `//host` forms, are rejected.
- A bare host or `host:port` input is upgraded to `https://`. A colon form whose prefix is not `http`, `https`, a dotted name, `localhost`, or a bracketed IPv6 literal is rejected as ambiguous instead of guessed.
- No username or password (`@` in the authority). IPv6 literals must be bracketed. Ports must be numeric from 1 to 65535. Host labels are limited to ASCII letters, digits, and internal hyphens, with no leading or trailing hyphen, no empty labels, and a 253-byte host limit.
- Single-label numeric hosts are rejected as ambiguous; use dotted-decimal notation. Invalid percent escapes and percent-encoded control bytes are rejected.

## Interface and language

The interface reads the desktop host's document-start locale from `window.RCHub.locale`. It uses Turkish only for `tr` and English for `en` or any missing or unrecognized value, so the same archive works with older hosts that do not provide the property. It sets `<html lang>`, localizes the title, helper text, placeholders, accessible names, status messages, and validation text, and formats the 2048-byte limit with `Intl.NumberFormat`.

The surface follows the new RCHub app look (charcoal canvas, cobalt accent, rounded cards, 48px controls, visible focus ring, short color transitions): a header with a globe mark, a card with the URL field, explicit **Open** action (Enter also submits) and an `aria-live` status panel, a **Quick sites** card (Google, YouTube, X, GitHub), and a **Recent** card listing the last five successfully requested URLs with a **Clear** button. Quick-site and recent buttons are explicit user actions that fill and open the selected URL. The recent list is kept only in the surface's browser storage (`localStorage`, best effort; it is simply empty when storage is unavailable) and is never sent to the plugin. Pending, success, and error states are text in the status region; success states that the page cannot be confirmed from the surface. Invalid input never reaches the opener protocol.

At narrow widths the entry row stacks and the Open button shows its label; the quick-site grid becomes one column at very narrow widths. Focus is visible on the input and all buttons, controls have accessible names, and motion is limited to short color transitions with a reduced-motion override. The same HTML is used for desktop and Android surfaces; the Android surface's request is relayed by the host to the plugin process on the paired computer.

## Verification status

Source verification performed on Windows x86-64:

- `cargo test` exercises URL normalization and rejection cases, exact request-shape checks, valid dispatch through an injected no-op opener, invalid requests that must not call the opener, unknown methods, and launch-error propagation. It does not open a browser.
- `cargo build --release` and `package.py` produce the Windows archive; the archive was inspected and contains the manifest, executable, and both UI entrypoints.
- A protocol run of the packaged executable confirmed `hello` / `ready`; requests for `file:///etc/passwd`, a URL with credentials, and an unknown method returned the expected errors without opening a browser. No valid URL was dispatched to the operating system.

Not performed here: launching a real browser, opening any URL through the OS handler, keyboard checks and Turkish/English rendering under a real host (the restyled surface was only rendered in a browser against a mock bridge at 375 px and desktop width), Android relay, Linux build or `xdg-open` behavior, and start/stop against the desktop app. These require manual desktop-session verification. The Windows launch path uses `ShellExecuteW` with the `open` verb; the Linux path uses `xdg-open`. Neither path is covered by an automated operating-system integration test, and an accepted request does not confirm a page load.
