# Changelog

## 0.2.4

- Added manifest description and localizedDescriptions (requires desktop app 0.1.2).

## 0.2.3

- UI now uses the shared RCHub neutral background palette.

## 0.2.2

- Added a packaged SVG plugin icon; requires an app version with icon support.

## Unreleased

- Added regression coverage for rejecting lock requests while a shutdown or restart is pending, without running power commands.

## 0.2.1

- Localized both UI surfaces in Turkish and English from `window.RCHub.locale`, falling back to English.
- Added `localizedNames` to the manifest: `Güç Seçenekleri` / `Power Options`.
- Localized visible copy, confirmation dialogs, pending-operation durations, accessibility labels and known bridge errors; action identifiers and request fields are unchanged.
- Installing requires a host that accepts `localizedNames`.
