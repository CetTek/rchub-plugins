# Changelog

Repository-level changes. Each plugin keeps its own CHANGELOG in `plugins/<name>/`.

## [Unreleased]
- **Added:** `rchub.messagebox` plugin 0.1.0 (Message Box): shows a dark-themed message window on the PC and/or reads it aloud with the installed Windows voice or, optionally, Google's online voice (needed for Turkish when no Turkish system voice is installed; the text is sent to Google, falls back to the system voice). Keeps the last 10 messages in memory only.
- **Added:** Manifests of all six plugins carry `description` and `localizedDescriptions` (tr/en). They need a desktop app that accepts these fields and a version bump to be released.
- **Changed:** CI builds only plugin versions that are not yet published (id/version read from `package.py`; released zips are downloaded for the catalog) and caches Rust builds with `Swatinem/rust-cache`.
- **Changed:** Plugin UIs share one neutral background palette (web-browser no longer bluish).
- **Added:** Packaged SVG icons for all six plugins; installation requires desktop app icon support.
- **Changed:** The repository now holds the plugins for the current RCHub app (`new-app`) instead of the legacy ABI2 reference plugins and Python SDK. The old content, releases and tags were removed.
- **Added:** Plugins moved here from the app repository with `rchub.*` ids (`rchub.clipboard`, `rchub.media`, `rchub.power`, `rchub.screenshot`, `rchub.system-info`, `rchub.web-browser`).
- **Added:** CI that builds every plugin, publishes new versions as immutable releases and replaces the Ed25519-signed `plugins.json` catalog in the rolling `catalog` release.
