# Changelog

## Unreleased

- Added regression coverage for rejecting lock requests while a shutdown or restart is pending, without running power commands.

## 0.2.1

- Localized both UI surfaces in Turkish and English from `window.RCHub.locale`, falling back to English.
- Added `localizedNames` to the manifest: `Güç Seçenekleri` / `Power Options`.
- Localized visible copy, confirmation dialogs, pending-operation durations, accessibility labels and known bridge errors; action identifiers and request fields are unchanged.
- Installing requires a host that accepts `localizedNames`.
