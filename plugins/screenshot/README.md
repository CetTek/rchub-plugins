# Screenshot example plugin

A separate native protocol-v2 plugin captures the primary display on demand. The same self-contained HTML surface, localized in Turkish and English, is packaged for desktop and Android. It does not capture automatically or save screenshots to disk. Screenshots can contain passwords, private messages and other sensitive content: anyone authorized to use this plugin can capture the desktop, and images travel through the configured relay. Install and enable it only where that access is acceptable. Native plugins run with the desktop user's permissions and are not sandboxed.

## Package and use

Install Rust/Cargo and Python 3, then from `new-app/desktop` run:

```sh
python examples/screenshot-plugin/package.py
```

On Windows, `py -3 examples/screenshot-plugin/package.py` also works. The script builds for the current Windows or Linux x86-64 Rust host and produces `examples/screenshot-plugin/screenshot-plugin-<platform>.zip`. Package separately on each operating system; cross-compilation is not supported by the script. Linux builds require the screenshot crate's native X11/XCB and Wayland build/runtime dependencies. A graphical session and display access are required. Wayland compositors may deny or not support capture; there is no portal permission workaround in this example. Locked, headless, protected or inaccessible displays may fail or omit content depending on the OS.

Install the ZIP locally in RCHub, review and accept the native-code warning, and start the plugin. Open its desktop UI or select it on an approved Android device and press **Capture**. Failure is reported in the UI. A previous successful image stays visible if a later capture fails; the status identifies the failure. Stop the plugin when it is not needed.

The 0.1.1+ packages declare `localizedNames` (`Ekran Görüntüsü` in Turkish, `Screenshot` in English). The surface reads `window.RCHub.locale`, accepts `tr` or `en`, falls back to English, and sets `<html lang>`; visible copy, dimensions, accessibility labels and known capture errors are localized, while protocol methods and image data are untouched. Installing this package requires a host that accepts `localizedNames`; older strict hosts reject the new manifest, and previously installed packages keep their original name. A host that does not inject `locale` shows the English UI.

## Contract and limits

`window.RCHub.request('screenshot.capture', {})` requests one capture. The v2 subprocess responds with `result` containing `mime` (`image/jpeg`), `base64` (JPEG bytes, not a data URL), `width`, `height`, `original_width`, and `original_height` in pixels. Only an empty params object is accepted. Errors use `invalid_params`, `method_not_found`, or `capture_failed`. The UI displays the returned JPEG and its dimensions. Selecting the image (it is a focusable button) opens a full-screen viewer: pinch, mouse wheel, double-click/double-tap or the + / - buttons zoom (up to 800%), dragging pans when zoomed, Reset restores the fit, and Close, Esc, a backdrop tap or Back dismisses it and returns focus to the image. Animations are skipped under `prefers-reduced-motion`.

The image is initially fitted within 800 × 800 pixels, then JPEG quality and dimensions are reduced as needed to keep encoded JPEG bytes at or below 30 KiB (base64 at most 40 KiB). This leaves room for the 64 KiB subprocess line and relay envelopes; it is a preview, not a full-resolution screenshot. Capture itself requires memory for the original display image, so the payload limit is not a bound on capture memory. The plugin reports an error if it cannot encode within the limit. Multiple queued relay messages may still exceed aggregate transport limits.

## Verification

Build and exercise the package independently on Windows and Linux, including display permissions, real capture, high-detail images, malformed requests, stop/restart and oversized response handling. Verify desktop and a paired Android device end to end through the actual relay, including disconnects and error states. Platform behavior and real-device interoperability have not been verified by this example's implementation alone.
