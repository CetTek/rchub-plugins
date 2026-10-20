# Changelog

## Unreleased

## 0.3.0

- Extend reusable controls with ring progress, multiline inputs, typed draft forms/bindings, localized action confirmation and searchable selected-row actions.
- Update System, Keyboard, Browser, Apps, Power and Clipboard source examples; retain install-time grants and explicit reinstall semantics.
- Synchronize strict JSON Schema/Python SDK and EN/TR guides/LLM index. New generated guests safely bound escaped output and 64 KiB input; old 2-page guests remain compatible.
- Native provider behavior and UI live verification belong to the compatible host; package tests do not invoke destructive operations.

- Add a manifest-driven System memory usage progress chart backed by the bounded `ram_usage_percent` value from `system.read`; no new UI control or permission is required.
- Add and publish an LLM-friendly `llms.txt` index for the EN/TR guides, package contract, SDK and official examples; link it and the interactive preview from developer documentation. Published in `28a69af`, raw file returned HTTP 200.
- Format all twelve source `manifest.json` files with stable two-space indentation and make both manifest generators preserve it. Keep downloadable `.rchub-plugin.json` bundles canonical and compact for package identity. Rebuilt twelve package hashes match the RcHub checkout; all manifests are multiline and whitespace checks pass.

## 0.2.0

- Combine Media, Audio and Display into the Media & Display reference package with explicit installation-time approval of its five capabilities.
- Define layouts through existing shared schema-v1 rows, sections, icons and tones across all twelve source packages; no plugin-ID-specific mobile screen.
- Add volume/mute row, brightness slider and circular playback buttons; update power ordering/emphasis and all plugin icon metadata.
- Remove redundant Audio/Display/System refresh buttons: current host reads sequentially on open and provides compact manual refresh.
- Preserve standalone Audio/Display sources and all prior release assets for compatibility. Older installed grants/manifests are never migrated automatically.

## 0.1.1

- Publish the twelve MIT reference plugins, Python SDK, schema and EN/TR developer guides under CetTek.
- Point package sources to the public v0.1.1 tag and regenerate canonical package bytes.
- Keep existing 0.1.0 installations and grants unchanged; new versions require separate installation approval.
