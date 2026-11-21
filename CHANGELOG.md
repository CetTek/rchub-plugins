# Changelog

Repository-level changes. Each plugin keeps its own CHANGELOG in `plugins/<name>/`.

## [Unreleased]
- **Changed:** The repository now holds the plugins for the current RCHub app (`new-app`) instead of the legacy ABI2 reference plugins and Python SDK. The old content, releases and tags were removed.
- **Added:** Plugins moved here from the app repository with `rchub.*` ids (`rchub.clipboard`, `rchub.media`, `rchub.power`, `rchub.screenshot`, `rchub.system-info`, `rchub.web-browser`).
- **Added:** CI that builds every plugin, publishes new versions as immutable releases and replaces the Ed25519-signed `plugins.json` catalog in the rolling `catalog` release.
