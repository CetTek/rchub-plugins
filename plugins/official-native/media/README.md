# Media & Display

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

Version 0.2.0 combines default-output volume/mute, primary-monitor brightness
and playback controls. The current Windows host uses DDC/CI first and WMI only
for one uniquely matched internal primary panel. The manifest defines the volume/mute row,
brightness slider and circular playback row using shared UI controls, icons
and tones. This is not a hardcoded mobile screen.

Installation requests audio.read, audio.write, display.read, display.write and
media.write together. Existing Media/Audio/Display grants are not combined or
upgraded automatically; remove the old package and explicitly install 0.2.0.
The mobile/web panel reads audio and display state sequentially on opening;
the desktop debug panel does not auto-read. Brightness stays unavailable when
neither provider applies; failed writes are never retried through another provider.

The host fallback does not change package identity or installed grants. Older
Display package labels may still say DDC/CI; correcting those labels needs an
explicit package update. Public plugin docs/examples/llms synchronization is pending.

Standalone Audio/Display packages remain in source for compatibility. The DEV
catalog recommends the combined Media & Display package instead of those two
when the combined artifact is present.

## Native package migration / Yerel paket geçişi

**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Reads and changes audio and brightness, and sends media playback keys.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package media --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/media/` is one public repository root containing
`rchub-app.json` plus the actual hash-bound executable and sources. Commit those
files at that repository’s root; a monorepo subdirectory URL is not supported.
There is no checked-in fake executable/hash. The builder refuses an incompatible
contract or an existing destination instead of silently dropping behavior.

**TR:** `native-source.json` tüm işlem şemalarını ve kontrolleri korur. Gerçek
çalıştırılabilir dosya `../native-runtime` kaynaklarından derlenir ve OS API’lerini
kendisi çağırır; host WASM vekili kullanmaz ve **sandbox içinde değildir**.
Kurulumda bir kez tam kaynak/risk/veri davranışı açıklanıp onaylanmalıdır; her
işlemde mevcut kimlik ve kurulu yetki kontrolleri host tarafından sürdürülür.
V1 dosyaları yerel derleme ve davranış doğrulaması bitene kadar korunur; eski
izinler yerel pakete otomatik aktarılmaz. Yukarıdaki komutta gerçek public GitHub
repo adresini kullanın; üretilen paket klasörünün içeriğini o reponun köküne koyun.

Ses ve parlaklığı okur ve değiştirir; medya oynatma tuşları gönderir.
