# Writing an RCHub plugin

A plugin is a small native program plus an optional web UI. Users install it on their computer (RCHub desktop), and it appears in the RCHub mobile and web apps. This guide covers everything needed to build, package and test one.

> **Native code is not sandboxed.** A plugin runs with the user's operating-system permissions. Only ship code you are willing to be trusted with, and keep requests narrow.

## What you build

| Part | Required | Notes |
| --- | --- | --- |
| `manifest.json` | yes | Identity, description, entrypoints, UI paths |
| Native executable under `bin/` | yes | Any language; talks to the host over stdin/stdout |
| `ui/desktop/…`, `ui/android/…` HTML | no | Surface shown in the desktop app and in the mobile/web clients |
| `ui/icon.svg` | no | Square 24×24 icon shown in lists |

The package is a ZIP with `manifest.json` at its root, executables under `bin/` and UI files under `ui/`. Limits: 110 MiB ZIP, 256 entries, 100 MiB unpacked, 64 KiB manifest.

## Manifest

```json
{
  "id": "acme.hello",
  "name": "Hello",
  "localizedNames": { "tr": "Merhaba", "en": "Hello" },
  "description": "Say hello from your computer.",
  "localizedDescriptions": { "tr": "Bilgisayarından selam ver.", "en": "Say hello from your computer." },
  "version": "0.1.0",
  "protocol": 2,
  "icon": "ui/icon.svg",
  "entrypoints": { "windows-x86_64": "bin/hello.exe", "linux-x86_64": "bin/hello" },
  "ui": { "desktop": "ui/desktop/index.html", "android": "ui/android/index.html" }
}
```

| Field | Required | Rules |
| --- | --- | --- |
| `id` | yes | 1–128 chars: lowercase letters, digits, `.`, `-`. The `rchub.` prefix is reserved for the official catalog |
| `name` | yes | Nonblank, at most 128 characters |
| `localizedNames` | no | `tr` and/or `en` display names; clients fall back to `name` |
| `description` | no | One short line (at most 100 characters, no line breaks) shown under the name in the mobile and web clients. Write what it does, not the plugin's name |
| `localizedDescriptions` | no | `tr` and/or `en` versions of `description`; the client's language is used, then `description` |
| `version` | yes | Nonblank string; bump it for every release |
| `protocol` | yes | `1` (lifecycle only) or `2` (requests and UI bridge) |
| `entrypoints` | yes | `windows-x86_64` and/or `linux-x86_64` → forward-slash path under `bin/` |
| `ui` | no | `desktop` and/or `android` HTML file under `ui/desktop/` or `ui/android/` |
| `icon` | no | A restricted SVG, see [Icon](#icon) |

Unknown fields are **rejected** by the host. A host older than the field you use refuses the package, so only use `description`/`localizedDescriptions` once users have an RCHub desktop version that supports them. Plugins without them keep working and simply show no subtitle.

## Talking to the host

The host starts your executable without a shell, with its own folder as the working directory. Messages are UTF-8 JSON, one object per line (64 KiB max). **stdout is for protocol messages only**; log to stderr.

1. Host → `{"v":2,"type":"hello"}`
2. You → `{"v":2,"type":"ready"}` within 5 seconds, flushed, then stay alive.
3. Host → `{"v":2,"type":"request","id":"1","method":"greet","params":{}}`
4. You → `{"v":2,"type":"response","id":"1","ok":true,"result":{"text":"Hi"}}` or `"ok":false,"error":{"code":"…","message":"…"}`
5. Host → `{"v":2,"type":"stop"}`: clean up and exit within 2 seconds, or you are killed.

Use `"v":1` and skip requests if you only need lifecycle. Unexpected exits are reported, not restarted. Reserve at most 64 KiB per relayed response, including any base64 data.

## UI

A v2 surface calls `window.RCHub.request(method, params)`, which returns a Promise bound to your plugin. `window.RCHub.locale` is `tr` or `en`; pick your own strings from it, and tolerate its absence on older hosts. Desktop surfaces run in a restricted child window; Android and web receive your assets through the server and run in a restricted view. Keep one self-contained HTML file per surface, and design for a phone-width screen first.

## What you can build, and the limits

The data path is **client → desktop plugin**: a phone or browser surface sends a request, the plugin process answers. Design around that.

| Idea | Fit | How |
| --- | --- | --- |
| Remote control (power, media, clipboard, open a link) | Good | One request per action |
| Show computer state (system info, a screenshot) | Good | Request on open or on a timer |
| Read text aloud on the computer (text-to-speech) | Good | The surface sends `speak` with the text; the plugin calls the operating system's speech API |
| Chat between phone and computer | Possible, not instant | Phone → computer is a normal request. Computer → phone cannot be pushed: the plugin has no way to send an unsolicited event to a client, so the phone surface has to poll (for example a `messages.since` request every second or two) |
| Voice messages / file transfer | Only very small data | Every relayed message is limited to 64 KiB including base64 and metadata, and there is no bulk or direct transfer. A few seconds of low-bitrate audio fits; anything larger does not |

Limits to design for:

- **No events to clients.** `window.RCHub.onEvent(callback)` exists in the Android bridge, but the desktop host does not forward unsolicited plugin messages, so it will not fire for plugin-originated events yet. Poll instead.
- **Size.** 64 KiB per relayed request or response. Chunk or reduce data, or return only what the screen needs.
- **Concurrency.** The host allows 16 outstanding remote requests in total; excess requests get an error. Keep handlers quick and answer within a few seconds.
- **Storage.** The host gives you no managed data folder, and updating a plugin does not migrate data. Keep state in memory, or store files yourself under a path you choose (for example the user's application-data directory) and treat it as something an update may leave behind.
- **Surfaces.** Android and web surfaces run in a restricted view: JavaScript only, no file access, no navigation away from the page. Microphone and camera access from that view is **not verified**; do not rely on it without testing on a real device. Desktop surfaces run in a separate restricted window.
- **Availability.** A client sees only running protocol-v2 plugins that have the matching UI for that client. A plugin can stop right after the list is produced, so a later request may still fail; handle errors.
- **Trust.** Any signed-in device on the same account can call your plugin. Validate every parameter, and never expose an action you would not let any of the user's devices trigger.

## `window.RCHub` (v2 surfaces)

| Member | Meaning |
| --- | --- |
| `RCHub.request(method, params)` | Returns a Promise with your plugin's `result`, or rejects with the error `code`/`message` you returned. Bound to your plugin; you cannot address another plugin |
| `RCHub.locale` | `"tr"` or `"en"`, frozen. May be missing on older hosts |
| `RCHub.onEvent(callback)` | Android bridge only; currently not triggered by plugin messages (see above) |

## Icon

`"icon": "ui/icon.svg"` names an SVG inside the package: UTF-8, at most 4096 bytes, one root `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">`. Allowed elements: `svg`, `g`, `path`, `circle`, `ellipse`, `rect`, `line`, `polyline`, `polygon`. `fill` and `stroke` must be `none` or `currentColor`. Scripts, styles, links, `use`, `image`, comments, XML declarations, colour codes and `url(` are rejected.

## Package and install

Build your executable for the platform you target, then write the ZIP: `manifest.json` at its root, your executable(s) under `bin/`, UI under `ui/`. Paths must be relative, forward-slash, with no `..`, no duplicates and no symlinks; only `manifest.json`, `bin/` and `ui/` may sit at the root. Any language works as long as the entrypoint runs by itself (the host does not install runtimes). The `plugins/*/package.py` scripts in this repository show a complete Rust build and packaging, in about 30 lines of Python each.

Install the resulting ZIP from the desktop app's local package picker, accept the native-code warning, then start and stop it from the plugin screen. Updating means installing a ZIP with the same `id` and confirming the replacement; plugin data is not migrated between versions.

## Checklist before sharing

- `id` is unique and not under `rchub.`; `version` is bumped.
- `description` is a single line under 100 characters, and `localizedDescriptions` has both `tr` and `en` if you target both.
- stdout carries only protocol lines; secrets never reach stderr (the last 16 KiB is shown in errors).
- Requests validate their `params` and return `ok:false` with a clear `code` instead of crashing.
- The entrypoint finds its dependencies from its own folder; the host never downloads runtimes.
- Test on a clean install, on desktop and from a phone.

The six official plugins in this repository (`plugins/`) are good working references. They are installed through a signed catalog. Third-party plugins are installed manually from a ZIP.
