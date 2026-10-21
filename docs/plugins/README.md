# RCHub Plugin Developer Guide

RCHub plugins are declarative native-control panels backed by a small WebAssembly guest. The host owns the panel, installation, stored grant and capability checks. A plugin can request only capabilities named in its manifest; it cannot add arbitrary UI code or run JavaScript/HTML.

This guide describes the current development implementation. Marketplace registration and remote execution are DEV-only. Do not treat a catalog entry, `publisher.kind`, source URL or license string as proof of publisher identity, code origin, review or safety. Review the package and requested capabilities before installation.

For an AI-oriented documentation index, see the public plugin repository's [llms.txt](https://github.com/CetTek/rchub-plugins/blob/main/llms.txt). The RCHub DEV app also provides an interactive, mock-only preview at `/plugin-preview`.

## Quick start

The SDK is a dependency-free Python script. Run commands from the repository root:

```powershell
python tools/plugins/rchub_plugin.py create examples/volume --id com.example.volume --name "Volume"
python tools/plugins/rchub_plugin.py build examples/volume --proxy --out examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py validate examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py inspect examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py preview examples/volume/volume.rchub-plugin.json
```

`create` writes `manifest.json` and readable `guest.wat` for the Audio starter. Replace the starter publisher and source metadata before distributing it. `build --proxy` generates a real Core Wasm guest that maps each declared action entry to its declared capability. It is a convenience guest for one-action/one-capability forwarding; it does not implement custom logic. Without `--proxy`, `build` reads `guest.wasm` from the project directory. Use a Wasm toolchain to compile custom guest logic.

Source `manifest.json` files use readable two-space JSON indentation. Built `.rchub-plugin.json` packages stay compact and canonical because their SHA-256 identity is calculated from canonical bytes; use `inspect` to print a readable package and manifest view.

`validate`, `inspect` and `preview` accept a `.rchub-plugin.json` package. `preview` prints the local preview URL; start the mobile development server, open `/plugin-preview`, then paste the package JSON into the fixture preview. The standalone preview uses local fixture state and does not send device operations. SDK commands do not fetch dependencies, run package hooks, or contact an account or server.

## Package format and identity

A distributable package is a UTF-8 JSON envelope:

```json
{"formatVersion":1,"manifest":{},"wasm":[0,97,115,109,1,0,0,0]}
```

The `wasm` value is an array of byte integers. The full canonical package is limited to 131,072 bytes; the Wasm module is limited to 16,384 bytes. The package identity is `{id, version, sha256}`. The lowercase SHA-256 digest covers the exact canonical package bytes: recursively sort object keys lexicographically, retain array order, emit compact JSON with literal UTF-8 non-ASCII, allow finite numbers only, and append exactly one LF (`\n`). The SDK performs this encoding and prints the identity after build.

Objects are strict: unknown keys are rejected. Manifests use schema and host API version 1, localized English and Turkish text, reverse-domain lowercase IDs, numeric `major.minor.patch` versions, Windows/Linux targets, a declared permission list, actions, typed state and UI controls. Numeric fields need integer bounds; strings need a maximum length. Arrays and nested objects have explicit limits. Inputs, outputs and state reject undeclared fields. Keep numeric bounds and UI slider steps as integers for consistent cross-runtime canonicalization.

The runtime accepts only bounded Core Wasm with `memory` (5 pages in new proxies; 16 for large declared output; 2 for legacy guests) and `run(i32 entry, i32 inputLen) -> i32 outputLen`. The host writes input JSON at offset 0 (at most 65,536 bytes in new 5/16-page guests, 8,192 in legacy 2-page guests); the guest writes output JSON at offset 32,768 (at most 900,000 bytes). The only allowed import is `rchub.invoke`, which forwards a typed JSON payload to a native capability after package, operation and installed-grant checks. There is no WASI, start function, other import, arbitrary template, expression evaluation, filesystem path, hook, archive or dependency execution. Each operation creates a fresh guest store under existing fuel, memory and worker limits.

## Permissions and installation

Permissions are the sorted, unique capability IDs in `manifest.permissions`. Every action must use a declared capability. Installation presents the source/risk disclosure and exact requested permissions and requires explicit approval from the phone or PC. The grant persists across reconnect and app restart; use within that grant does not prompt again per action. Each operation still authenticates the current device owner, verifies the installed package and grant, checks capability and platform availability, and validates typed input and output. A missing, invalid or unsupported grant fails closed.

Install does not replace an installed release. Compatible hosts support explicitly approved, atomic updates to strictly newer versions of the same ID with identical permissions. Changed permissions require separate removal and installation; no new permission is silently inherited. Opening a panel may perform a declared, argument-free read action to observe state; the host does not automatically replay a write action.

The Rust desktop advertises atomic-update support only after authenticating its device socket and epoch. The server accepts that advertisement only from the current canonical PC socket and matching epoch, and clears it on reconnect or rebind. Mobile shows update availability for eligible same-permission releases; an older desktop or one that has not advertised support returns `unsupported` without dispatching or replaying the update. Restart the updated desktop and explicitly retry. The advertisement indicates operation support only; it grants no permissions or requester authority. If a request ends with `unknown`, refresh installed inventory before deciding whether to retry because the operation outcome was not observed.

## Capability availability

A name being accepted by the manifest schema does not mean a provider is implemented. Current Rust native dispatch has the following Windows implementations; platform/package availability is checked at runtime.

| Capability | Current status | Effect / result |
| --- | --- | --- |
| `audio.read` | Ready on Windows | Read default output volume and mute state. |
| `audio.write` | Ready on Windows | Set bounded volume and/or mute; return observed state. |
| `system.read` | Ready | Read OS, hostname, CPU, memory and uptime. |
| `browser.open` | Ready on Windows | Open an HTTP(S) URL in the default browser. |
| `power.write` | Ready on Windows | Lock/sleep, bounded delayed shutdown/restart with explicit force, or cancel a scheduled shutdown. |
| `keyboard.write` | Ready on Windows | Type bounded text or send a constrained hotkey. |
| `media.write` | Ready on Windows | Play/pause, next or previous media track. |
| `clipboard.read` | Ready on Windows | Read bounded clipboard text. |
| `clipboard.write` | Ready on Windows | Set bounded clipboard text and return the observed text. |
| `apps.read` | Ready on Windows | List up to 64 visible applications or all processes, with typed window/process identities. |
| `apps.write` | Ready on Windows | Bring forward, minimize/maximize/restore, close a window or stop a process; validate observed identity. |
| `display.read`, `display.write` | Implemented on Windows | Primary physical monitor DDC/CI brightness only. Read reports `available:false` without support; writes then fail. |
| `screenshot.read` | Implemented on Windows | Optimized primary-screen JPEG, up to 1920× 1080 at quality 80, reduced as needed to an 800,000-character data URL cap. Tap the image in the client to open it full-screen. |
| `speech.write` | Implemented on Windows | Literal SAPI text, at most 2,000 characters and 10 seconds; expiry cancels speech. |
| `effects.write` | Implemented on Windows | Only `{effect:"beep"}`; plays the Windows system sound. |

Do not advertise or install a package as functional when its required capability or target is unavailable. The host reports unsupported/unavailable instead of treating a no-op as success. Other platforms may have fewer providers; verify the availability shown by the host.

## UI schema

The complete field/limit/ABI reference is the
[implementation contract](../../contracts/plugins/README.md); use the
[JSON Schema](../../contracts/plugins/plugin.schema.json) for editor assistance.
JSON Schema alone cannot validate references between actions, state and controls;
run the SDK validator and let the PC perform full Wasm validation.

| Controls | Required relationship |
| --- | --- |
| section, row, column, tabs | `children` contain bounded control trees; tabs select a child group. |
| button | Declared `action`; supply its required argument via `argument` and static `value`. |
| slider | Numeric `bind` and compatible action `argument`, explicit `min`, `max`, positive `step`. |
| toggle, select | Toggle requires boolean bound state/action argument; select requires a declared argument and bounded localized `options`. |
| input | String action `argument` with optional multiline; typed integer/boolean draft-only fields use `submit:false`, `defaultValue` and action-bound schemas. |
| value, status, text | Display declared `bind` state or localized text. |
| progress | Numeric bound state and explicit `min`/`max`. |
| list | Array state with `itemLabel`/`itemValue`; optional search and selected-row `itemActions` map typed fields/static arguments. |
| image | Bound embedded PNG/JPEG data URL for portable PC/mobile rendering; no remote URL. |

For a complete working layout, copy the
[Audio manifest](../../plugins/official/audio/manifest.json). It binds the
`volume` slider to observed numeric state and passes a committed value to
`set_volume`; `muted` is a boolean toggle. Names, labels, ranges and action links
come from the manifest, not an Audio-specific mobile modal. The
[Apps manifest](../../plugins/official/apps/manifest.json) demonstrates object rows
and passing the selected PID to a typed action.

The [System manifest](../../plugins/official/system/manifest.json) also shows a
manifest-defined memory usage chart: `system.read` returns a bounded
`ram_usage_percent` observation, rendered with the existing `progress` control.

The manifest declares UI version 1, a built-in icon and theme color token, and a bounded tree of controls. IDs are unique lowercase identifiers. Layout nodes (`section`, `row`, `column`, `tabs`) can contain children. Bindings refer directly to declared state or action-input fields. Leaf controls can name an action and input argument, a static scalar value, numeric range, localized options or typed list row fields. The host validates control/action/schema compatibility and owns the confirmation, permissions, about and removal UI.

The mobile renderer currently implements `section`, `row`, `column`, `tabs`, `button`, `slider`, `toggle`, `select`, `input`, `progress`, `image`, `list`, `text`, `value` and `status`. Controls need valid schemas and corresponding state/output values; invalid or unavailable actions must not be represented as working controls. Image values are limited to data URLs for PNG, JPEG or WebP. Use bounded typed arrays and objects for lists rather than encoding structured data in strings.

## Importing and marketplace

Community packages can be imported without a catalog entry. On mobile, open the `.rchub-plugin.json` as text and paste its complete contents into the import field; there is no package file picker in this flow. On PC, enter a local package path or paste its JSON contents. Both paths validate the bounded package and show publisher/source disclosure, license, targets and permissions before the installation approval. Source and publisher fields are claims supplied by the package, not verified provenance.

The mobile workspace and its management modal are presentation only: installed cards come from the host inventory, the Marketplace grid uses the real catalog/search/category/pagination endpoints, and installing or removing stays disabled while no authorized PC client is connected. Browsing the Marketplace remains available while the PC is offline; browsing never installs or executes a package.

The current catalog serves repository-owned package artifacts and metadata in DEV. Catalog routes do not fetch arbitrary URLs, certify publishers or provide reviews/download counters. Catalog availability does not install or execute a package; installation and later invocation remain owner-authenticated operations.

## Reference packages

### Shared layouts introduced in 0.2.0 (retained in 0.3.0)

The shared mobile/web renderer consumes the manifest, without special screens
selected by plugin ID. Plugins choose control order, `row`/`column`/`section`/`tabs`
groups, localized labels, icons, root theme color and each control's `tone`
(`default`, `primary`, `danger`, `muted`). These existing schema-v1 fields are now
rendered consistently: primary buttons use the theme accent, danger buttons have
a filled destructive treatment, and icon buttons inside rows become compact
circular controls with accessible labels. `sun` and `play` icons use the existing
orange/green semantic tokens. An inline icon toggle becomes a compact toggle;
other toggles retain their labeled switch. Sliders show the draft percentage
while dragging and commit only when released. No arbitrary CSS, HTML or script
is accepted. Edit the manifest/builder and rebuild to customize a package.

Panels read no-input `.read` actions sequentially on opening, matching the
single-request transport. Refresh is a compact panel-footer action; it never
invokes a write action. New Audio/Display/System layouts omit redundant refresh
buttons. Closing cancels the waiter and invalidates remaining reads. Writes
retain the last observation while busy, merge successful partial output and
clear observations on failure; no unconfirmed write is displayed as success.
The preview route uses fixture state only and never sends device commands.

Official Media 0.2.0 is **Media & Display**: volume/mute, DDC/CI brightness and
playback share one manifest, using these same controls. It explicitly requests
audio.read/write, display.read/write and media.write at installation. Previous
separate grants are never unioned. Unsupported DDC/CI disables brightness.
When the combined package is present, the DEV catalog omits standalone Audio
and Display from discovery; their sources and existing installations remain.
All twelve source packages retain the shared layouts/icon metadata. These layouts
require an explicitly reviewed update of older versions; stored manifests are not silently rewritten.
Power 0.3.0 adds bounded delay, explicit force and cancel-scheduled-shutdown; see below.

Public sources, SDK and this guide are maintained at
[CetTek/rchub-plugins](https://github.com/CetTek/rchub-plugins).
Official package version 0.3.0 links to the matching `v0.3.0` source tag.
The source metadata change produces new package hashes. Existing installed
older packages/grants are not silently migrated; review and explicitly approve a same-permission update on a compatible host. Permission changes require separate removal and installation.

`plugins/official/` contains twelve repository-owned MIT packages: Audio, System,
Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech and
Effects. “Official” is a repository label, not a cryptographic signature. Manifests
and `guest.wat` are reviewable source; reproduce packages using
`python tools/plugins/build_official.py`.

This DEV implementation does not reproduce every legacy option: Apps lists and controls windows/processes, it does not launch applications; Screenshot provides a bounded,
optimized image up to 1920× 1080 rather than an unbounded raw capture; Effects
provides a beep, not fake BSOD or screen shaking.
The Windows providers have compile and fake-provider coverage; hardware, native
writes and physical-phone behavior still need live verification. Do not interpret
an implemented provider as proof that it was exercised on this PC.

### Reusable controls and official examples (0.3.0)

Plugins define their complete panel through shared controls; the renderer never selects a custom screen by plugin ID. `progress` now accepts `appearance:"ring"`. String inputs support `multiline` and text/url `inputMode`; integer draft inputs use numeric mode. Draft-only inputs declare their action/argument schema plus `submit:false` and a typed `defaultValue`; boolean drafts render as checkboxes. A button's `inputBindings` combines these drafts into one validated invocation. `confirm` contains localized title/message for destructive actions; installation consent remains separate.

Lists support local `searchable` filtering and selected-row `itemActions`: localized label/action, primary argument/value, static `args`, row-field `itemArguments`, optional icon/tone/confirm. Selecting a row opens its action menu and does not perform a destructive operation. Every argument and required field is schema checked; see the [contract](../../contracts/plugins/README.md) and official source manifests.

System groups compact CPU/RAM rings and memory figures. Keyboard has multiline typing and shortcut tabs with quick hotkeys. Browser has a URL field and quick sites. Apps defaults to visible titled application windows, can read all processes, searches observed rows and exposes window controls/stop with process-creation identity checks; close/stop ask for action confirmation. Power delay (0..86400 seconds, UI default 30) and force drafts apply together only to shutdown/restart, with a separate cancel-scheduled-shutdown action. Windows scheduling preserves force=false even with a delay. Clipboard empty/non-text reads return an empty string; bounded retry handles temporary clipboard contention.

Official 0.3.0 source links use tag `v0.3.0`. Upgrade the host first: schema/UI/hostApi remain 1 but older strict validators reject the new optional fields. Use the reviewed Update action for a newer package with identical permissions; changes to permissions require separate removal and installation. New generated proxies use 5-page memory, 262144-byte output and 64 KiB serialized input; large screenshot output uses 16 pages/900000 bytes. Old 2-page guests retain 8 KiB input. Field UTF-16 limits still apply (clipboard 8192, keyboard 4096). Mock previews and tests do not send native writes.


### Explicit in-place updates

Marketplace and installed management compare numeric major.minor.patch versions.
Update downloads and reviews the new source, license and permissions before explicit
approval. The v2 update request includes expectedPackage (the exact installed
id/version/sha256), target package/bundle, permissions and the installation consent.
Only strictly newer versions with an identical permission set are accepted; no
prerelease/build versions, downgrades, same-version replacements or permission changes.
The host checks the old identity and current owner binding, then atomically replaces
the record. Failed replacement preserves the installed package and grants; a stale
identity is rejected and requires refreshed inventory. Installation still conflicts
when that ID is already installed. No automatic update or inherited new permission.

### System 0.3.1 source checkpoint

System 0.3.1 shortens the RAM label and places computer/OS observations together. Memory is still used/total MB and uptime is seconds; green RAM and used/free GB are not implemented by this manifest. The shared host renderer owns responsive ring/modal sizing. The 0.3.1 source and package are included in this snapshot; installation or update must be explicitly approved and observed separately. Other official packages remain 0.3.0.
