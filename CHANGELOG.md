# Changelog

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
