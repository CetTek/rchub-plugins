# Clipboard

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Empty or non-text clipboard reads yield empty text; contention retries are bounded. Text limit 8192 UTF-16 units; generated guest buffers also allow escaped JSON.

Boş/metin dışı pano boş metin döndürür; kilitlenme denemeleri sınırlıdır. Metin 8192 UTF-16 birimiyle sınırlıdır; vekil tamponları kaçışlı JSON için yeterlidir.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.

## Native package migration / Yerel paket geçişi

**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Reads and replaces up to 8192 UTF-16 units of clipboard text. Action results are returned only to the requesting client, never published in shared surface state. A client must display the read action's response to show clipboard contents.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package clipboard --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/clipboard/` is one public repository root containing
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

En fazla 8192 UTF-16 birimi pano metnini okur ve değiştirir; metin isteyen istemciye iletilir.
