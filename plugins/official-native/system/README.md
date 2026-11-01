# System

MIT reference plugin. The manifest renders memory usage as a progress chart from the bounded `ram_usage_percent` observation, alongside CPU, memory totals and uptime. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

CPU/RAM use bounded manifest-driven ring progress controls; used/total memory and uptime remain typed observations.

CPU/RAM sınırlı manifest halka göstergeleridir; kullanılan/toplam bellek ve çalışma süresi tipli gözlemdir.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Review and explicitly approve a same-permission update on compatible Windows hosts; on older hosts, remove and reinstall explicitly. Installed grants never change silently.

## 0.3.1

The RAM ring has a compact label, and computer/OS observations share a row. Memory remains used/total MB and uptime remains seconds; this package does not claim GB/free-memory formatting. Shared renderers determine responsive ring and modal sizing.

RAM halkası kısa bir etiket kullanır; bilgisayar/işletim sistemi gözlemleri aynı satırdadır. Bellek kullanılan/toplam MB, çalışma süresi saniye olarak kalır; GB/boş bellek biçimlendirmesi sunulmaz. Duyarlı halka ve modal boyutunu ortak çiziciler belirler.

The 0.3.1 source and package are included in this snapshot; installation or update must be explicitly approved and observed separately. Use the explicitly approved same-permission Update action on compatible Windows hosts; older hosts require explicit removal/reinstallation.

0.3.1 kaynak ve paketi bu snapshot'a dahildir; kurulum veya güncelleme ayrıca açıkça onaylanıp gözlemlenmelidir. Uyumlu Windows hostlarda aynı izinleri koruyan Güncelle işlemi açık onayla kullanılabilir; eski hostlarda paketi açıkça kaldırıp yeniden kurmak gerekir.

## Native package migration / Yerel paket geçişi
The Windows native package exposes a `battery` action and battery presence, charge percentage and charging state. A percentage of -1 means unavailable or no battery; an unknown Windows battery status fails rather than inventing a reading.

Windows yerel paketi `battery` işlemiyle pil varlığını, şarj yüzdesini ve şarj durumunu sunar. Yüzde -1, bilgi yok veya pil yok anlamına gelir; Windows pil durumu bilinmiyorsa sahte değer yerine hata döner.


**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Reads CPU, memory, OS name, hostname, uptime and battery status and returns them to the requesting client.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package system --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/system/` is one public repository root containing
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

CPU, bellek, işletim sistemi, bilgisayar adı, çalışma süresi ve pil durumunu okur ve isteyen istemciye iletir.
