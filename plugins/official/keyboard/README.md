# Keyboard

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

Multiline typing, shortcut tabs and quick hotkeys use generic declarative controls. Text is limited to 4096 UTF-16 code units.

Çok satırlı yazı, kısayol sekmeleri ve hızlı tuşlar ortak kontrolleri kullanır. Metin 4096 UTF-16 kod birimiyle sınırlıdır.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
