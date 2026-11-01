# Browser

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

A URL field and manifest-defined quick sites invoke the same bounded HTTP(S) open action.

Adres girdisi ve manifest hızlı siteleri aynı sınırlı HTTP(S) açma eylemini çağırır.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.

## Native package migration / Yerel paket geçişi
The native package also targets Linux. It invokes `/usr/bin/xdg-open` with the validated HTTP(S) URL as one argument, without a shell. The executable and a working graphical desktop/browser association are required. Success means `xdg-open` accepted the request, not that the page loaded; cancellation or a 10-second timeout kills the opener and reports failure.

Yerel paket Linux'u da hedefler. Doğrulanmış HTTP(S) adresini kabuk kullanmadan tek argüman olarak `/usr/bin/xdg-open` ile açar. Bu program ve çalışan grafik masaüstü/tarayıcı ilişkilendirmesi gereklidir. Başarı sayfanın yüklendiğini değil isteğin kabul edildiğini gösterir; iptal veya 10 saniyelik zaman aşımında açıcı sonlandırılır ve hata döner.


**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Opens supplied HTTP(S) URLs in the default browser; the destination receives a normal browser request.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package browser --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/browser/` is one public repository root containing
`rchub-app.json` plus the actual hash-bound executable and sources. Commit those
files at that repository’s root; a monorepo subdirectory URL is not supported.
There is no checked-in fake executable/hash. The builder refuses an incompatible
contract or an existing destination instead of silently dropping behavior.
The build uses the checked-in Cargo lockfile with `--locked`; install the matching
Rust target and dependencies before building. Use `--platform linux` on a Linux
build host for the Linux artifact.

Derleme depodaki Cargo lock dosyasını `--locked` ile kullanır; önce uygun Rust
hedefini ve bağımlılıkları kurun. Linux çıktısı için Linux derleme ortamında
`--platform linux` kullanın.

**TR:** `native-source.json` tüm işlem şemalarını ve kontrolleri korur. Gerçek
çalıştırılabilir dosya `../native-runtime` kaynaklarından derlenir ve OS API’lerini
kendisi çağırır; host WASM vekili kullanmaz ve **sandbox içinde değildir**.
Kurulumda bir kez tam kaynak/risk/veri davranışı açıklanıp onaylanmalıdır; her
işlemde mevcut kimlik ve kurulu yetki kontrolleri host tarafından sürdürülür.
V1 dosyaları yerel derleme ve davranış doğrulaması bitene kadar korunur; eski
izinler yerel pakete otomatik aktarılmaz. Yukarıdaki komutta gerçek public GitHub
repo adresini kullanın; üretilen paket klasörünün içeriğini o reponun köküne koyun.

Verilen HTTP(S) adreslerini varsayılan tarayıcıda açar; hedefe normal tarayıcı isteği gönderilir.
