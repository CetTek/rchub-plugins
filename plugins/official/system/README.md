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
