# RCHub Plugins

Open-source reference plugins and a dependency-free Python SDK for RCHub, maintained by CetTek. Licensed under MIT. The RCHub host application is maintained separately.

[Developer guide (English)](docs/plugins/README.md) · [Geliştirici rehberi (Türkçe)](docs/plugins/README.tr.md) · [Package contract](contracts/plugins/README.md) · [JSON Schema](contracts/plugins/plugin.schema.json)

## Included plugins

Audio, System, Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech and Effects live in [plugins/official](plugins/official). Each directory includes a manifest, readable Wasm proxy source and a portable `.rchub-plugin.json` package.

These are development reference packages for a compatible RCHub host. Native capabilities currently target Windows; provider availability depends on the host/runtime and hardware. Apps lists/stops processes, Screenshot returns a thumbnail, and Effects plays a beep. They do not reproduce every legacy control. Source availability does not certify safety or production readiness.

## Build

Python 3.9 or newer; no third-party Python dependencies:

```sh
python tools/plugins/build_official.py
python tools/plugins/rchub_plugin.py validate plugins/official/audio/audio.rchub-plugin.json
```

The builder reproduces all twelve packages from the definitions in `build_official.py`. The SDK also supports community manifests and custom compiled Wasm. See the guides for schemas, UI controls, permissions and installation.

## Releases and installation

Release `v0.2.0` publishes twelve source packages whose source links point to the matching tag. Media is now **Media & Display**, combining volume/mute, DDC/CI brightness and playback. Standalone Audio/Display remain available for compatibility; the current DEV catalog recommends the combined package in their place. All plugins define their own control layout, icons and button tones through the shared UI schema. See the guides' Panel customization section.

Download a `.rchub-plugin.json` release asset and import it into a compatible development host. Review the source, requested permissions and installation disclosure first. Installing requires explicit approval; later operations still enforce installed grants and current authorization. Existing installed manifests are not rewritten. Reinstall to obtain a changed layout; Media 0.2.0 explicitly requests audio/read-write, display/read-write and media/write permissions together. Older separate grants are never combined automatically.

Version 0.1.1 introduced public source links; version 0.2.0 adds the shared-control layouts and combined Media package. Package hashes change with each manifest/release. Remove an old installation and approve the new version separately.

## Contributing

Discuss a new capability before proposing host-dependent functionality. Use the documented manifest/UI contracts, include EN/TR labels, declare exact permissions and provide reproducible source. Never include credentials, account data or machine-specific configuration. A package or publisher label is not proof of review or origin.
