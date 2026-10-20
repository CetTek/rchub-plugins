# Apps

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Read defaults to visible titled application windows; all-process mode is explicit. Search observed rows and select a row before choosing bring-to-front/minimize/maximize/restore/close/stop. Destructive actions confirm. Writes carry the observed process creation timestamp and window handle; the host rechecks ownership. No application launch.

Okuma görünür başlıklı uygulama pencereleriyle başlar; tüm işlemler ayrıca seçilir. Satırı seçip öne getir/küçült/büyüt/geri yükle/kapat/sonlandır işlemini seçin. Tehlikeli işlem onay ister; ana makine işlem başlangıcını ve pencere sahibini yeniden doğrular. Uygulama başlatmaz.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
