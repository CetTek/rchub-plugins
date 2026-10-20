# System

MIT reference plugin. The manifest renders memory usage as a progress chart from the bounded `ram_usage_percent` observation, alongside CPU, memory totals and uptime. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

## 0.3.0

CPU/RAM use bounded manifest-driven ring progress controls; used/total memory and uptime remain typed observations.

CPU/RAM sınırlı manifest halka göstergeleridir; kullanılan/toplam bellek ve çalışma süresi tipli gözlemdir.

Rebuild: `python tools/plugins/build_official.py`. Source manifest is readable JSON; distributable bytes stay canonical. Remove and explicitly reinstall older versions; installed grants never change silently.
