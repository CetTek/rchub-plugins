# RCHub Plugin Developer Guide

RCHub plugins are declarative native-control panels backed by a small WebAssembly guest. The host owns the panel, installation, stored grant and capability checks. A plugin can request only capabilities named in its manifest; it cannot add arbitrary UI code or run JavaScript/HTML.

This guide describes the current development implementation. Marketplace registration and remote execution are DEV-only. Do not treat a catalog entry, `publisher.kind`, source URL or license string as proof of publisher identity, code origin, review or safety. Review the package and requested capabilities before installation.

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

`validate`, `inspect` and `preview` accept a `.rchub-plugin.json` package. `preview` prints the local preview URL; start the mobile development server, open `/plugin-preview`, then paste the package JSON into the fixture preview. The standalone preview uses local fixture state and does not send device operations. SDK commands do not fetch dependencies, run package hooks, or contact an account or server.

## Package format and identity

A distributable package is a UTF-8 JSON envelope:

```json
{"formatVersion":1,"manifest":{},"wasm":[0,97,115,109,1,0,0,0]}
```

The `wasm` value is an array of byte integers. The full canonical package is limited to 131,072 bytes; the Wasm module is limited to 16,384 bytes. The package identity is `{id, version, sha256}`. The lowercase SHA-256 digest covers the exact canonical package bytes: recursively sort object keys lexicographically, retain array order, emit compact JSON with literal UTF-8 non-ASCII, allow finite numbers only, and append exactly one LF (`\n`). The SDK performs this encoding and prints the identity after build.

Objects are strict: unknown keys are rejected. Manifests use schema and host API version 1, localized English and Turkish text, reverse-domain lowercase IDs, numeric `major.minor.patch` versions, Windows/Linux targets, a declared permission list, actions, typed state and UI controls. Numeric fields need integer bounds; strings need a maximum length. Arrays and nested objects have explicit limits. Inputs, outputs and state reject undeclared fields. Keep numeric bounds and UI slider steps as integers for consistent cross-runtime canonicalization.

The runtime accepts only bounded Core Wasm with `memory` (two pages) and `run(i32 entry, i32 inputLen) -> i32 outputLen`. The host writes input JSON at offset 0 (at most 8,192 bytes); the guest writes output JSON at offset 32,768 (at most 32,768 bytes). The only allowed import is `rchub.invoke`, which forwards a typed JSON payload to a native capability after package, operation and installed-grant checks. There is no WASI, start function, other import, arbitrary template, expression evaluation, filesystem path, hook, archive or dependency execution. Each operation creates a fresh guest store under existing fuel, memory and worker limits.

## Permissions and installation

Permissions are the sorted, unique capability IDs in `manifest.permissions`. Every action must use a declared capability. Installation presents the source/risk disclosure and exact requested permissions and requires explicit approval from the phone or PC. The grant persists across reconnect and app restart; use within that grant does not prompt again per action. Each operation still authenticates the current device owner, verifies the installed package and grant, checks capability and platform availability, and validates typed input and output. A missing, invalid or unsupported grant fails closed.

The first installer does not replace an installed release. Remove the installed package, review the new release and install it again. Grants are not migrated or inherited across package identities/releases. Opening a panel may perform a declared, argument-free read action to observe state; the host does not automatically replay a write action.

## Capability availability

A name being accepted by the manifest schema does not mean a provider is implemented. Current Rust native dispatch has the following Windows implementations; platform/package availability is checked at runtime.

| Capability | Current status | Effect / result |
| --- | --- | --- |
| `audio.read` | Ready on Windows | Read default output volume and mute state. |
| `audio.write` | Ready on Windows | Set bounded volume and/or mute; return observed state. |
| `system.read` | Ready | Read OS, hostname, CPU, memory and uptime. |
| `browser.open` | Ready on Windows | Open an HTTP(S) URL in the default browser. |
| `power.write` | Ready on Windows | Lock, sleep, shutdown or restart. Treat shutdown/restart as consequential actions. |
| `keyboard.write` | Ready on Windows | Type bounded text or send a constrained hotkey. |
| `media.write` | Ready on Windows | Play/pause, next or previous media track. |
| `clipboard.read` | Ready on Windows | Read bounded clipboard text. |
| `clipboard.write` | Ready on Windows | Set bounded clipboard text and return the observed text. |
| `apps.read` | Ready on Windows | List a bounded set of process names and PIDs. |
| `apps.write` | Ready on Windows | Terminate a process by PID. Review the exact target and risk. |
| `display.read`, `display.write` | Implemented on Windows | Primary physical monitor DDC/CI brightness only. Read reports `available:false` without support; writes then fail. |
| `screenshot.read` | Implemented on Windows | Primary-screen PNG thumbnail, at most 320×180, adaptively reduced to a 30,000-character data URL. |
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
| toggle, select | Compatible bound state/action argument; select has bounded localized `options`. |
| input | String action `argument`; typing is local, explicit submit invokes the action. |
| value, status, text | Display declared `bind` state or localized text. |
| progress | Numeric bound state and explicit `min`/`max`. |
| list | Array state; object rows use `itemLabel` and `itemValue`; an action receives the typed row value. |
| image | Bound embedded PNG/JPEG data URL for portable PC/mobile rendering; no remote URL. |

For a complete working layout, copy the
[Audio manifest](../../plugins/official/audio/manifest.json). It binds the
`volume` slider to observed numeric state and passes a committed value to
`set_volume`; `muted` is a boolean toggle. Names, labels, ranges and action links
come from the manifest, not an Audio-specific mobile modal. The
[Apps manifest](../../plugins/official/apps/manifest.json) demonstrates object rows
and passing the selected PID to a typed action.

The manifest declares UI version 1, a built-in icon and theme color token, and a bounded tree of controls. IDs are unique lowercase identifiers. Layout nodes (`section`, `row`, `column`, `tabs`) can contain children. Bindings refer directly to declared state or action-input fields. Leaf controls can name an action and input argument, a static scalar value, numeric range, localized options or typed list row fields. The host validates control/action/schema compatibility and owns the confirmation, permissions, about and removal UI.

The mobile renderer currently implements `section`, `row`, `column`, `tabs`, `button`, `slider`, `toggle`, `select`, `input`, `progress`, `image`, `list`, `text`, `value` and `status`. Controls need valid schemas and corresponding state/output values; invalid or unavailable actions must not be represented as working controls. Image values are limited to data URLs for PNG, JPEG or WebP. Use bounded typed arrays and objects for lists rather than encoding structured data in strings.

## Importing and marketplace

Community packages can be imported without a catalog entry. On mobile, open the `.rchub-plugin.json` as text and paste its complete contents into the import field; there is no package file picker in this flow. On PC, enter a local package path or paste its JSON contents. Both paths validate the bounded package and show publisher/source disclosure, license, targets and permissions before the installation approval. Source and publisher fields are claims supplied by the package, not verified provenance.

The mobile workspace and its management modal are presentation only: installed cards come from the host inventory, the Marketplace grid uses the real catalog/search/category/pagination endpoints, and installing or removing stays disabled while no authorized PC client is connected. Browsing the Marketplace remains available while the PC is offline; browsing never installs or executes a package.

The current catalog serves repository-owned package artifacts and metadata in DEV. Catalog routes do not fetch arbitrary URLs, certify publishers or provide reviews/download counters. Catalog availability does not install or execute a package; installation and later invocation remain owner-authenticated operations.

## Reference packages

Public sources, SDK and this guide are maintained at
[CetTek/rchub-plugins](https://github.com/CetTek/rchub-plugins).
Official package version 0.1.1 links to the immutable `v0.1.1` source tag.
The source metadata change produces new package hashes. Existing installed
0.1.0 packages/grants are not migrated; remove the old installation and explicitly
approve installation of 0.1.1 to use the public View source code link.

`plugins/official/` contains twelve repository-owned MIT packages: Audio, System,
Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech and
Effects. “Official” is a repository label, not a cryptographic signature. Manifests
and `guest.wat` are reviewable source; reproduce packages using
`python tools/plugins/build_official.py`.

This DEV implementation does not reproduce every legacy option: Apps lists/stops
processes, it does not launch applications; Screenshot is a thumbnail, not the old
full-resolution capture; Effects provides a beep, not fake BSOD or screen shaking.
The Windows providers have compile and fake-provider coverage; hardware, native
writes and physical-phone behavior still need live verification. Do not interpret
an implemented provider as proof that it was exercised on this PC.
