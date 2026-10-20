# Clipboard

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Empty or non-text clipboard reads yield empty text; contention retries are bounded. Text limit 8192 UTF-16 units; generated guest buffers also allow escaped JSON.

Boş/metin dışı pano boş metin döndürür; kilitlenme denemeleri sınırlıdır. Metin 8192 UTF-16 birimiyle sınırlıdır; vekil tamponları kaçışlı JSON için yeterlidir.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
