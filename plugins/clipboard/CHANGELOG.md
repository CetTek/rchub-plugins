# Changelog

## 0.2.3

- Added a packaged SVG plugin icon; requires an app version with icon support.

## 0.2.2

- Support up to 256 KiB of UTF-8 clipboard text using bounded chunk transfers; incomplete writes never reach the computer clipboard.
- Reinstall the package after removing the previous installation; in-place package updates are not supported.

## 0.2.1

- Localized both UI surfaces in Turkish and English from `window.RCHub.locale`, falling back to English.
- Added `localizedNames` to the manifest: `Pano` / `Clipboard`.
- Localized visible copy, accessibility labels, byte counts and known bridge errors; clipboard text and protocol identifiers are unchanged.
- Installing requires a host that accepts `localizedNames`.
