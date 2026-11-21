# Media Controls example

A protocol v2 native plugin with desktop and Android HTML surfaces. It targets **Windows x86-64 only**; no Linux implementation or package is claimed. It reads the Windows system media session (title, artist, album, playback state, position, duration and, when available, cover art), sends transport commands to the active session, seeks within the current track and reads or changes the default multimedia playback endpoint's master volume and mute state. A transport success means the command was accepted or the media key was sent, not that a particular player acted on it. Audio device changes, missing output devices, locked sessions, live streams and player behavior can affect results.

## Package and install

On Windows x86-64 install Rust/Cargo and Python 3, then run `py -3 examples/media-plugin/package.py` from `new-app/desktop`. The script builds the native executable and creates `media-plugin-windows-x86_64.zip` here, including both UI surfaces. Install using the desktop package picker, accept the native-code trust warning, start the plugin and open its desktop surface or select it on an approved Android device. The plugin talks to Windows through the `windows` crate (WinRT media session APIs and WASAPI), so it no longer compiles or launches PowerShell. Native plugins run with the current user's privileges and are not sandboxed.

The 0.2.1 package declares `localizedNames` (`Medya Kontrolleri` in Turkish, `Media Controls` in English). Both surfaces read `window.RCHub.locale`, accept `tr` or `en`, fall back to English, and set `<html lang>`; visible copy, accessibility labels and known bridge errors are localized while media metadata, method names and error codes are passed through unchanged. Installing this package requires a host that accepts `localizedNames`; older strict hosts reject the new manifest, and previously installed packages keep their original name. Without an injected `locale`, the UI remains English.

The system media session API requires Windows 10 version 1809 or newer. Without a media session, `media.status` reports `available: false` and the UI shows an empty state instead of failing.

## Request contract

All methods require a params object. Each volume response is `{ "level": 0..100, "muted": boolean }`; `level` is the rounded master-volume scalar percentage. The default multimedia output endpoint is resolved on every request, so a device may change between requests. `volume.set` changes volume without unmuting.

| Method | Params | Result |
| --- | --- | --- |
| `media.play_pause` | `{}` | `{ "sent": true }` |
| `media.next` | `{}` | `{ "sent": true }` |
| `media.previous` | `{}` | `{ "sent": true }` |
| `media.status` | `{}` | status response (below) |
| `media.seek` | `{ "positionMs": integer }` | `{ "positionMs": integer }` |
| `volume.get` | `{}` | volume response |
| `volume.set` | `{ "level": integer }` in 0–100 | volume response |
| `volume.toggle_mute` | `{}` | volume response |

A `media.status` result contains:

| Field | Type | Meaning |
| --- | --- | --- |
| `available` | boolean | `false` when there is no active media session; all other fields are then empty/zero |
| `title` | string | Track or media title, empty when the session exposes none |
| `artist` | string | Artist, falling back to album artist |
| `album` | string | Album title |
| `playing` | boolean | `true` for the WinRT `Playing` state; everything else is reported as not playing |
| `positionMs` | integer | Current position in milliseconds, estimated from the last timeline update while playing |
| `durationMs` | integer | Track duration in milliseconds; `0` when unknown (for example a live stream) |
| `canSeek` | boolean | Whether the session reports playback-position control and a known duration |
| `artworkDataUrl` | string, optional | Small `data:image/...;base64,...` thumbnail; omitted when absent or larger than 32 KiB |
| `sourceApp` | string, optional | Source application user model ID reported by Windows |

`media.seek` returns `unsupported` when the session has no position control or rejects the request, `platform_error` when there is no session or the platform call fails, and `invalid_params` for negative or non-integer positions. Unknown methods return `method_not_found`, invalid arguments `invalid_params`, and other OS/backend failures `platform_error`. Protocol framing, lifecycle and response correlation follow the desktop README. Requests are executed serially; there are no unsolicited events.

## UI behavior

The desktop and Android surfaces poll `media.status` about once per second only while the page is visible, and never overlap requests: the next poll is scheduled after the previous response. While the user drags the seek slider, incoming status updates keep the metadata current but do not overwrite the slider; releasing it sends one `media.seek` and refreshes the status.

When artwork is unavailable, the UI shows a music-note placeholder. Playback status and operation errors are announced when they occur; no persistent loading message is shown.

The volume slider applies changes automatically with a 500 ms debounce and always flushes the final value when the slider is released or loses focus. There is no separate "Set volume" button. If a send is already in flight, the newest value is queued and applied immediately afterwards. Progress and failures are announced in the volume panel's `aria-live` status line.

A timeout or connection loss does not prove that an action failed: avoid automatic retries, especially for toggle operations.

## Verification required

Package and install on Windows x86-64, then verify lifecycle, each request and error case with an actual output device and player; test desktop UI and an approved Android WebView separately. Confirm the no-session empty state, artwork limits, seeking in a stream that reports no position control, and missing-device behavior. `cargo test` covers request validation and the base64 encoder only. Neither real-device behavior nor Linux support is established by the source alone.
