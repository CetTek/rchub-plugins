"""MIT. Prepare local ABI2 import inputs; never install, connect or execute code."""
import argparse
import base64
import hashlib
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/plugins'))
import rchub_app_v2 as sdk


def descriptor(content, plugin_id='com.example.switchboard'):
    manifest = sdk.strict_json((ROOT / 'tools/plugins/fixtures/v2/vendor.fixture.json').read_bytes())
    old = manifest['id']
    manifest['id'] = plugin_id
    manifest['name'] = {'en': 'Switchboard example', 'tr': 'Anahtar örneği'}
    manifest['source'] = 'https://example.invalid/switchboard/source'
    manifest['storage'] = {'choices': ['none']}
    artifact = manifest['execution']['artifact']
    artifact.update(entry='bin/switchboard.exe', sha256=hashlib.sha256(content).hexdigest())
    manifest['actions'] = {k.replace(old + '.', plugin_id + '.', 1): v
                           for k, v in manifest['actions'].items()}
    manifest['events'] = {k.replace(old + '.', plugin_id + '.', 1): v
                          for k, v in manifest['events'].items()}
    for surface in manifest['surfaces']:
        if surface['kind'] == 'extension-window':
            surface['action'] = surface['action'].replace(old + '.', plugin_id + '.', 1)
            continue
        for key in ('actions', 'events'):
            surface[key] = [s.replace(old + '.', plugin_id + '.', 1) for s in surface[key]]
        def rewrite(controls):
            for control in controls:
                if 'action' in control:
                    control['action'] = control['action'].replace(old + '.', plugin_id + '.', 1)
                rewrite(control.get('children', []))
        rewrite(surface['controls'])
    sdk.verify_artifact(manifest, 'windows-x86_64', artifact['entry'], content)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--id', default='com.example.switchboard')
    args = parser.parse_args()
    with args.artifact.open('rb') as stream:
        content = stream.read(sdk.MAX_ARTIFACT + 1)
    manifest = descriptor(content, args.id)
    # Byte integrity only. The compatible host additionally validates PE/target,
    # containment, grants and current authority. Source placeholder is not fetched.
    args.out.mkdir(parents=True, exist_ok=True)
    outputs = {
        'manifest.json': sdk.canonical(manifest),
        'artifact.base64.txt': base64.b64encode(content),
        'identity.json': sdk.canonical(sdk.identity(manifest)),
    }
    if any((args.out / name).exists() for name in outputs):
        raise ValueError('Output exists; use a fresh output directory')
    for name, raw in outputs.items():
        (args.out / name).write_bytes(raw)
    print(sdk.canonical(sdk.identity(manifest)).decode('utf-8'), end='')


if __name__ == '__main__':
    main()
