# RCHub Plugins

Official plugins for [RCHub](https://github.com/emi-ran/RcHub), the remote PC control app. The desktop app downloads them from this repository's releases through a signed catalog, installs the ones that are missing and keeps them up to date.

| Plugin | Id | What it does |
| --- | --- | --- |
| Clipboard | `rchub.clipboard` | Read and set the computer's clipboard text |
| Media Controls | `rchub.media` | Play/pause, track and volume controls |
| Power Options | `rchub.power` | Lock, sleep, restart, shut down |
| Screenshot | `rchub.screenshot` | Capture the screen and view it full-screen |
| System Info | `rchub.system-info` | CPU, memory, disk and uptime |
| Web Browser | `rchub.web-browser` | Open a link in the computer's default browser |

All plugins target Windows x86-64 for now.

## Layout

```
plugins/<name>/        one Rust plugin: Cargo.toml, src/, ui/, package.py, README, CHANGELOG
tools/                 catalog signing and publishing (Node, no dependencies)
catalog/               public key that the desktop app trusts
.github/workflows/     test (every push/PR) and release (main)
```

## How a release works

1. Bump `version` in the plugin's `package.py` (and its CHANGELOG) and merge to `main`.
2. The `release` workflow builds every plugin on `windows-latest` with `package.py`.
3. For each plugin it uses the release tag `<id>-v<version>`. If that release already exists the published zip is reused unchanged (a published version is immutable). Otherwise a new release with the zip is created.
4. It then signs `plugins.json` (id, version, url, size, sha256, minimum app version) with the plugin Ed25519 key and replaces it in the rolling `catalog` release:
   `https://github.com/CetTek/rchub-plugins/releases/download/catalog/plugins.json`

The desktop app verifies the catalog signature with the public key in `catalog/plugin-public-key.b64`, checks each zip's size and SHA-256, and only installs `rchub.*` plugins that come from here.

### Signing key

The private key lives only in the `PLUGIN_SIGNING_KEY` Actions secret (and an offline backup of the owner). It is never committed. To rotate the key run `node tools/keygen.mjs --out-dir <dir>`, replace the secret and `catalog/plugin-public-key.*`, and ship a desktop release that embeds the new public key.

## Local build

```powershell
cd plugins/web-browser
python package.py      # writes web-browser-plugin-windows-x86_64.zip
```

Requires Rust (MSVC toolchain) and Python 3. Dry-run the whole pipeline without publishing:

```powershell
$env:PLUGIN_SIGNING_KEY_FILE = "<path to a test private key>"
node tools/build-catalog.mjs --dist dist
```

## License

MIT, see [LICENSE](LICENSE).
