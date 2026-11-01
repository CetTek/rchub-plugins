# RCHub Plugins

Open-source reference plugins and dependency-free Python developer SDKs for RCHub, maintained by CetTek. Licensed under MIT. The RCHub host application is maintained separately; no private host sources or account data are included.

[Developer guide (English)](docs/plugins/README.md) · [Geliştirici rehberi (Türkçe)](docs/plugins/README.tr.md) · [Package contract](contracts/plugins/README.md) · [JSON Schema](contracts/plugins/plugin.schema.json)

For AI coding tools, see [`llms.txt`](llms.txt), a concise index of the guides, SDK, contracts and official examples.

## Choose a development model — no mandatory migration

| Need | Choose | Boundary |
| --- | --- | --- |
| Existing typed host capabilities and portable manifest panels | Supported **V1 Wasm** (`rchub_plugin.py`, existing official packages) | Bounded guest, declared capabilities; no arbitrary OS/filesystem guest access. |
| New developer-written PC logic, long-lived state/events, native extension windows | Experimental **native application / Integration ABI2** | Normal-user **UNSANDBOXED** executable; source/hash/process is not safety or publisher proof. |
| Production-ready arbitrary native execution, verified live UI/auth/Socket.IO or Linux acceptance | Neither is established by this snapshot | DEV only; those acceptance gates remain open. |

ABI2 uses descriptor **`schemaVersion:2`**, Socket.IO wire **`v:3` with
`version:2` on all envelopes**, and process hello/welcome **`version:2`**.
It is not V1 wire v2. Existing V1 packages, IDs, hashes and grants remain intact;
no automated conversion or transfer of Wasm consent to unsandboxed native.

Start with the [ABI2 contract](contracts/plugins/v2/README.md),
[Python module](tools/plugins/rchub_app_v2.py) and
[buildable new-ID example (EN/TR)](plugins/examples/application-v2/README.md).
Native code implements its own PC behavior without host provider edits; phone/web/PC
surfaces are declarative, while native windows belong to the extension, never host
authentication/consent. Custom/temp storage is developer-owned; optional host-scoped
store has exact scope/CAS/retention rules, not OS isolation.

**Publication (2026-11-02):** twelve official Windows x64 native packages at version
1.0.0 are published in [plugins/official-native](plugins/official-native/README.md)
with their exact artifact SHA-256 values, and a compatible host serves them through
its authenticated V2 catalog. Legacy V1 packages, identities and grants are
unchanged. ABI2 remains experimental; host evidence is bounded
Windows/injected-transport/headless testing, not live UI/auth/socket-loopback
acceptance or product completion. Linux packages are not published.

## Included V1 plugins

Audio, System, Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech and Effects live in [plugins/official](plugins/official). Each directory includes a manifest, readable Wasm proxy source and a portable `.rchub-plugin.json` package.

These are development reference packages for a compatible RCHub host. Native capabilities currently target Windows; provider availability depends on the host/runtime and hardware. Apps controls observed processes/windows, Screenshot returns a bounded optimized image, and Effects plays a beep. They do not reproduce every legacy control. Source availability does not certify safety or production readiness.

## Published native packages (Windows x64)

Twelve MIT native applications at version 1.0.0 live in [plugins/official-native](plugins/official-native/README.md). Each directory is a complete package root with the canonical descriptor, hash-bound executable, authoring source, runtime sources and lockfile; the exact artifact SHA-256 values are listed there. They are distributed as a catalog: a compatible host serves them through the authorized `GET /api/plugins/v2/catalog` and hash-bound artifact routes, and the GitHub-URL resolver does not accept a monorepo subdirectory. Native code is unsandboxed and V1 grants do not transfer.

## Build V1 packages

Python 3.9 or newer; no third-party Python dependencies:

```sh
python tools/plugins/build_official.py
python tools/plugins/rchub_plugin.py validate plugins/official/audio/audio.rchub-plugin.json
```

The builder reproduces all twelve packages from the definitions in `build_official.py`. Source `manifest.json` files use readable two-space JSON indentation. Built `.rchub-plugin.json` files stay compact and canonical because their SHA-256 identity is calculated from canonical bytes; use `inspect` for a readable package view. The SDK also supports community manifests and custom compiled Wasm. See the guides for schemas, UI controls, permissions and installation.

## Existing V1 releases and installation

Release `v0.3.0` contains twelve source packages whose source links point to the matching tag. Media is now **Media & Display**, combining volume/mute, DDC/CI brightness and playback. Standalone Audio/Display remain available for compatibility; the current DEV catalog recommends the combined package in their place. All plugins define their own control layout, icons and button tones through the shared UI schema. See the guides' Panel customization section.

Download a `.rchub-plugin.json` release asset and import it into a compatible development host. Review the source, requested permissions and installation disclosure first. Installing requires explicit approval; later operations still enforce installed grants and current authorization. Existing installed manifests are not rewritten. Reinstall to obtain a changed layout; Media 0.2.0 explicitly requests audio/read-write, display/read-write and media/write permissions together. Older separate grants are never combined automatically.

Version 0.1.1 introduced public source links; version 0.2.0 added shared layouts and combined Media. Version 0.3.0 adds ring gauges, multiline/typed form drafts, selected-row menus/search, bounded Power scheduling and escaping-safe guest buffers. Upgrade the compatible host first; old strict hosts reject the new optional schema fields. Package hashes change with each manifest/release. Remove an old installation and approve the new version separately.

## Contributing

Discuss a new capability before proposing host-dependent functionality. Use the documented manifest/UI contracts, include EN/TR labels, declare exact permissions and provide reproducible source. Never include credentials, account data or machine-specific configuration. A package or publisher label is not proof of review or origin.
