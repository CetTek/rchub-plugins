# Changelog

## 0.1.3

- Added a packaged SVG plugin icon; requires an app version with icon support.

## 0.1.2

- The captured image is now a tappable button that opens a full-screen viewer (dark backdrop, close button, Esc, backdrop tap and Back close it; focus is trapped and returned).
- Zoom with pinch, mouse wheel, double-click/double-tap or the + / - buttons (up to 800%); pan by dragging when zoomed; Reset restores the fit. Plain JS/CSS, no new dependency; honors `prefers-reduced-motion`. Turkish and English strings added. Package size is still far below the 128 KiB / 128 files limits.

## 0.1.1

- Localized both UI surfaces in Turkish and English from `window.RCHub.locale`, falling back to English.
- Added `localizedNames` to the manifest: `Ekran Görüntüsü` / `Screenshot`.
- Localized visible copy, dimensions, accessibility labels and known capture errors; protocol methods and image data are unchanged.
- Installing requires a host that accepts `localizedNames`.
