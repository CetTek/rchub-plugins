# Screenshot

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Capture uses bounded quality 80 JPEG up to 1920× 1080/800000 data-URL characters; the host continues ownership/deadline checks while capture runs. Tap the client image to view full-screen.

Yakalama en fazla 1920× 1080/800000 veri adresi karakteri olan kalite 80 JPEG kullanır; yakalama sırasında sahiplik/süre sınırı kontrolü sürer. Tam ekran için görüntüye dokunun.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
