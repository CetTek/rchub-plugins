# Audio

Open-source RCHub reference plugin under the parent MIT license. It declares its
own card/panel, typed actions and exact audio.read/audio.write permissions.
`manifest.json` and `guest.wat` are readable source; `audio.rchub-plugin.json`
contains the reproducible real Wasm proxy guest and manifest. No host code is
embedded in this package, no arbitrary commands or install hooks.

Build without installing a toolchain:

```powershell
python tools/plugins/rchub_plugin.py build plugins/official/audio --proxy --out plugins/official/audio/audio.rchub-plugin.json
python tools/plugins/rchub_plugin.py inspect plugins/official/audio/audio.rchub-plugin.json
```

The `--proxy` compiler generates the documented action-to-capability guest from
the manifest. Custom guest logic must be compiled to `guest.wasm` and packaged
without `--proxy`; the proxy compiler does not compile arbitrary WAT/Rust source.
The runtime validates installed bytes and the immutable grant on every host call.
Only default Windows output volume/mute is implemented by this plugin.
