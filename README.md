# RCHub Plugins

Official plugins for RCHub, the remote PC control app. The desktop app downloads them from this repository's releases through a signed catalog, installs the ones that are missing and keeps them up to date.

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

## Plugin icon

The optional manifest field `"icon": "ui/icon.svg"` refers to an SVG included in the package. Icons require an app version with icon support; older desktop versions reject the unknown manifest field. Release these packages only after a compatible desktop app is available.

Use UTF-8 SVG of at most 4096 bytes with one root `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">`. Allowed elements: `svg`, `g`, `path`, `circle`, `ellipse`, `rect`, `line`, `polyline`, `polygon`. Allowed attributes: `xmlns`, `viewBox`, `d`, `cx`, `cy`, `r`, `rx`, `ry`, `x`, `y`, `x1`, `y1`, `x2`, `y2`, `width`, `height`, `points`, `fill`, `stroke`, `stroke-width`, `stroke-linecap`, `stroke-linejoin`, `fill-rule`, `clip-rule`, `opacity`. Fill and stroke values must be `none` or `currentColor`. Close all tags. Comments, DOCTYPE, XML declarations, styles, scripts, `use`, `image`, `href`, color codes and `url(` are forbidden.

## How a release works

1. Bump `version` in the plugin's `package.py` (and its CHANGELOG) and merge to `main`.
2. The `release` workflow reads each plugin's id and version from `package.py` and builds (with `package.py`, Rust cached) only plugins whose release does not exist yet, on `windows-latest`.
3. For each plugin it uses the release tag `<id>-v<version>`. If that release already exists the published zip is downloaded and reused unchanged, without building (a published version is immutable). Otherwise a new release with the zip is created.
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
