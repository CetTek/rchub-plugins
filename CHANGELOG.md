# Changelog

## Unreleased

- Contract sync with the host (2026-11-02): `contracts/plugins/v2/*.schema.json` and `tools/plugins/rchub_app_v2.py` now match the host's canonical files. Wire `v:3` gains the `revocation-ack` kind with `stopped`/`instances`, the host-only capability `owner-revocation-ack-v1` (nine transport features), eight manifest `requiredFeatures` (`image-v1`, `string-8192-v1`, `list-item-actions-v1`, `surface-metadata-v1` added), and the newer declarative surface vocabulary (`tabs`, `image`, `icon`, `tone`, `confirm`, `appearance`, `inputMode`, `placeholder`, `searchable`, `itemActions`) with a 8192-byte string bound and 800000-byte image bound. Docs state that native readiness and dispatch need a matching ack with `stopped:true` for the exact epoch/revision instead of the retired `ownerInvalidationCovered` flag; the host removed its legacy V1 executable/pilot stack while these V1 WASM reference packages stay published here. SDK/package suite: 57 tests pass.

- Publish twelve official Windows x64 native packages at version 1.0.0 under `plugins/official-native/`: `com.rchub.apps`, `com.rchub.audio`, `com.rchub.browser`, `com.rchub.clipboard`, `com.rchub.display`, `com.rchub.effects`, `com.rchub.keyboard`, `com.rchub.media`, `com.rchub.power`, `com.rchub.screenshot`, `com.rchub.speech` and `com.rchub.system`. Each directory is a complete package root with the canonical `rchub-app.json`, the hash-bound `bin/rchub-official-native.exe`, `native-source.json`, `native-runtime/` Cargo sources and lockfile, MIT license and README. Artifact SHA-256 values are recorded in `plugins/official-native/README.md` and match the committed executable bytes. Legacy V1 WASM directories, identities, hashes and grants are unchanged.
- Document how published packages are served: a compatible host reads `plugins/official-native` and exposes `GET /api/plugins/v2/catalog` plus `GET /api/plugins/v2/artifacts/<id>/<version>/<sha256>` only to an account session Bearer token or paired-device `x-device-id`/`x-device-password` headers, rejects other requests with 401, and keeps the desktop catalog action disabled without credentials. The GitHub-URL resolver still requires a descriptor at a repository root, so monorepo subdirectory URLs cannot be installed through it.
- Sync EN/TR guides, `llms.txt`, root README and the ABI2 example with the published package list and remove stale "nothing published" claims; production native capability remains fail-closed and Linux packages are not published.

- Local-only developer sync completed (2026-10-24): distinguish supported V1 Wasm (no mandatory migration) from experimental unsandboxed native Integration ABI2 in README, EN/TR guides and llms index. Document exact schemaVersion2 / wire-v3 version2 / IPC-version2, staged installation/explicit activation, owner/watch lifetime, install consent, extension windows and non-destructive storage retention.
- Add only reviewed public-safe ABI2 schemas, separate stdlib SDK/tests/corpora and harmless peers, with explicit provenance. New Windows x64 `com.example.switchboard` source builds using installed rustc; example helper writes validated descriptor/identity/base64, never installs/connects. Existing 53 protected V1 files (all 12 bundles, sources, SDK/test/builder/schema) byte-match HEAD; no V1 identities/grants changed.
- Validation: 35 ABI2 + 17 existing V1 SDK/package tests and 2 new native-example tests passed (54 total); actual documented rustc/helper commands and second namespace passed, 10 source copies byte-matched, 49 local guide/index links resolved, Python syntax/JSON and whitespace checks passed. No dependencies installed. Harmless compiled subprocess tests are not live host/UI/auth acceptance.
- Local commit explicitly authorized (2026-10-24) for this public developer-kit checkout only; complete 23-file review and fresh 52 SDK/package + 2 native-example tests passed before the local commit. Push, tag, release and publication remain pending separate approval; none was performed. ABI2 remains experimental DEV. Coordinator's separate Windows installed61 uses synthetic DB/injected transport/headless rendering, not live UI/auth/full socket or product completion. Linux acceptance deferred. Unrelated V1 select/Speech parity/release work remains excluded.

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
