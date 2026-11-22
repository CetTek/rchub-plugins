# Changelog

## 0.2.3

- UI now uses the shared RCHub neutral background palette.

## 0.2.2

- Added a packaged SVG plugin icon; requires an app version with icon support.

## 0.2.1

- Localized both UI surfaces in Turkish and English from `window.RCHub.locale`, falling back to English.
- Added `localizedNames` to the manifest: `Medya Kontrolleri` / `Media Controls`.
- Localized visible copy, accessibility labels, volume and playback status, and known bridge errors; track metadata is unchanged.
- Installing requires a host that accepts `localizedNames`.

## 0.2.0

- Read the Windows system media session directly through the `windows` crate: `media.status` now reports title, artist, album, playback state, position, duration, seek support and an optional inline cover image.
- Added `media.seek` for seeking within the active session.
- Transport commands target the current media session and fall back to media keys when it does not accept them.
- Volume reads and writes use WASAPI in-process instead of launching PowerShell; removed `media.ps1` from the package.
- The UI shows the current track, progress and artwork, polls `media.status` about once per second only while visible, and seeks by dragging the progress bar.
- Volume changes apply automatically after a 500 ms debounce and flush on release; the "Set volume" button is gone.
- Added an explicit "No active media" state and accessible labels, live status regions and keyboard support.

## 0.1.0

- Added a Windows media and default output volume plugin with protocol v2 desktop and Android surfaces.
