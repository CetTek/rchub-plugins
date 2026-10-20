# Power

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Delay 0..86400 seconds and force drafts are bound together only for shutdown/restart; UI defaults to 30 seconds/false. Cancel scheduled shutdown is separate. Windows InitiateSystemShutdownExW preserves force=false at nonzero delay; no power action is exercised by automated tests.

Gecikme 0..86400 saniye ve zorla kapat taslakları yalnız kapat/yeniden başlat için birlikte gönderilir; arayüz varsayılanı 30 saniye/false. Planlanan kapatmayı iptal etme ayrıdır. Windows API gecikmede force=false değerini korur; otomatik testler güç işlemi çalıştırmaz.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
