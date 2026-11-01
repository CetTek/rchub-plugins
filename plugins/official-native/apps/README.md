# Apps

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Read defaults to visible titled application windows; all-process mode is explicit. Search observed rows and select a row before choosing bring-to-front/minimize/maximize/restore/close/stop. Destructive actions confirm. Writes carry the observed process creation timestamp and window handle; the host rechecks ownership. No application launch.

Okuma görünür başlıklı uygulama pencereleriyle başlar; tüm işlemler ayrıca seçilir. Satırı seçip öne getir/küçült/büyüt/geri yükle/kapat/sonlandır işlemini seçin. Tehlikeli işlem onay ister; ana makine işlem başlangıcını ve pencere sahibini yeniden doğrular. Uygulama başlatmaz.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.

## Native package migration / Yerel paket geçişi
The native package also launches an installed Start Menu shortcut by name. It searches the current-user and machine Programs folders, prefers the shortest matching name, excludes uninstall/remove shortcuts, and checks request authority immediately before launching. It does not accept paths or command lines and does not enumerate UWP-only applications; shortcut targets run with the current user's authority.

The process-view selector defaults to Applications (`foreground`), matching the initial list. Choose All processes to include processes without visible windows.

Yerel paket ayrıca Başlat menüsündeki kurulu bir kısayolu adıyla açar. Kullanıcı ve makine Programs klasörlerinde arar, en kısa eşleşmeyi seçer, kaldırma kısayollarını dışlar ve açmadan hemen önce istek yetkisini kontrol eder. Yol veya komut satırı kabul etmez; yalnız UWP olan uygulamaları listelemez. Kısayol hedefleri mevcut kullanıcı yetkisiyle çalışır.

İşlem görünümü seçicisi, ilk listeyle uyumlu olarak Uygulamalar (`foreground`) seçeneğiyle başlar. Görünür penceresi olmayan işlemleri de görmek için Tüm işlemler seçilir.


**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Lists processes and window identities; can change windows, stop the selected process, or launch a Start Menu shortcut by name.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package apps --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/apps/` is one public repository root containing
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

İşlemleri ve pencere kimliklerini listeler; pencereleri değiştirebilir, seçilen işlemi sonlandırabilir veya Başlat menüsü kısayolunu adıyla açabilir.
