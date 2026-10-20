# Media & Display

MIT reference plugin. See manifest.json and guest.wat for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.
Installed grants are never inherited from another package or release.

Version 0.2.0 combines default-output volume/mute, primary-monitor DDC/CI
brightness and playback controls. The manifest defines the volume/mute row,
brightness slider and circular playback row using shared UI controls, icons
and tones. This is not a hardcoded mobile screen.

Installation requests audio.read, audio.write, display.read, display.write and
media.write together. Existing Media/Audio/Display grants are not combined or
upgraded automatically; remove the old package and explicitly install 0.2.0.
The host reads audio and display state sequentially on opening. Unsupported
DDC/CI brightness stays unavailable; it does not report working brightness.

Standalone Audio/Display packages remain in source for compatibility. The DEV
catalog recommends the combined Media & Display package instead of those two
when the combined artifact is present.
