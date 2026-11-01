# Audio

Open-source RCHub reference plugin under the parent MIT license. It declares its
own card/panel, typed actions and exact audio.read/audio.write permissions.
`manifest.json` and `guest.wat` are readable source; `audio.rchub-plugin.json`
contains the reproducible real Wasm proxy guest and manifest. No host code is
embedded in this package, no arbitrary commands or install hooks.

Build without installing a toolchain:

```powershell
python tools/plugins/rchub_plugin.py build plugins/official/audio --proxy --out plugins/official/audio/audio.rchub-plugin.json
python tools/plugins/rchub_plugin.py inspect plugins/official/audio/audio.rchub-plugin.json
```

The `--proxy` compiler generates the documented action-to-capability guest from
the manifest. Custom guest logic must be compiled to `guest.wasm` and packaged
without `--proxy`; the proxy compiler does not compile arbitrary WAT/Rust source.
The runtime validates installed bytes and the immutable grant on every host call.
Only default Windows output volume/mute is implemented by this plugin.

## Native package migration / Yerel paket geçişi

**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Reads and changes the default Windows output endpoint.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package audio --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/audio/` is one public repository root containing
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

Varsayılan Windows ses çıkışını okur ve değiştirir.
