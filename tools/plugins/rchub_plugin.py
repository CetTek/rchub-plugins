"""Dependency-free development SDK for RCHub declarative Wasm plugins.

Commands deliberately never fetch dependencies, run package hooks or contact an
account/server. Proxy guests are real generated Core Wasm, not native shortcuts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
MAX_PACKAGE = 131072
MAX_SAFE_INTEGER = 9007199254740991


def utf16_length(value: str) -> int:
    return len(value.encode("utf-16-le", errors="surrogatepass")) // 2


def safe_integer(value: object) -> bool:
    return type(value) is int and -MAX_SAFE_INTEGER <= value <= MAX_SAFE_INTEGER


def accepts(field: dict, value: object) -> bool:
    kind = field["type"]
    if kind == "array":
        return isinstance(value, list) and len(value) <= field["maxItems"] and all(accepts(field["items"], x) for x in value)
    if kind == "object":
        return isinstance(value, dict) and all(k in value for k in field["required"]) and all(k in field["properties"] and accepts(field["properties"][k], v) for k, v in value.items())
    valid = (type(value) is bool if kind == "boolean" else isinstance(value, str) and utf16_length(value) <= field["maxLength"] if kind == "string" else safe_integer(value) and field["minimum"] <= value <= field["maximum"])
    return valid and ("enum" not in field or any(type(value) is type(v) and value == v for v in field["enum"]))
CAPABILITIES = {
    'audio.read', 'audio.write', 'system.read', 'power.write', 'apps.read', 'apps.write',
    'display.read', 'display.write', 'media.write', 'screenshot.read', 'keyboard.write',
    'clipboard.read', 'clipboard.write', 'browser.open', 'speech.write', 'effects.write',
}
ICONS = ['volume', 'power', 'cpu', 'apps', 'camera', 'keyboard', 'clipboard', 'globe',
         'message', 'smile', 'puzzle', 'sun', 'play', 'info']
COLORS = ['blue', 'green', 'orange', 'purple', 'pink', 'teal', 'gray']
CONTROLS = ['section', 'row', 'column', 'button', 'slider', 'toggle', 'select', 'text',
            'value', 'status', 'input', 'list', 'image', 'progress', 'tabs']


def canonical(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'),
                       allow_nan=False) + '\n').encode('utf-8')


def identity(bundle: dict) -> dict:
    return {key: bundle['manifest'][key] for key in ('id', 'version')} | {
        'sha256': hashlib.sha256(canonical(bundle)).hexdigest()}


def uint(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 127
        value >>= 7
        out.append(byte | (128 if value else 0))
        if not value:
            return bytes(out)


def sint(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 127
        value >>= 7
        end = (value == 0 and not byte & 64) or (value == -1 and byte & 64)
        out.append(byte | (0 if end else 128))
        if end:
            return bytes(out)


def string(value: str) -> bytes:
    raw = value.encode()
    return uint(len(raw)) + raw


def section(number: int, raw: bytes) -> bytes:
    return bytes([number]) + uint(len(raw)) + raw


def const(value: int) -> bytes:
    return b'\x41' + sint(value)


def proxy_wasm(manifest: dict) -> bytes:
    """Compile a strict manifest action -> capability proxy, without a toolchain."""
    actions = manifest['actions']
    segments = []
    body = bytearray(b'\x00')  # zero local declarations
    cursor = 8192
    for action in actions:
        cap = action['capability'].encode()
        body += b'\x20\x00' + const(action['entry']) + b'\x46\x04\x7f'
        body += const(cursor) + const(len(cap)) + const(0) + b'\x20\x01'
        body += const(32768) + const(32768) + b'\x10\x00\x05'
        segments.append(b'\x00' + const(cursor) + b'\x0b' + uint(len(cap)) + cap)
        cursor += len(cap)
    body += const(-1) + b'\x0b' * len(actions) + b'\x0b'
    types = b'\x02\x60\x06' + b'\x7f' * 6 + b'\x01\x7f\x60\x02\x7f\x7f\x01\x7f'
    imports = b'\x01' + string('rchub') + string('invoke') + b'\x00\x00'
    exports = b'\x02' + string('memory') + b'\x02\x00' + string('run') + b'\x00\x01'
    return (b'\x00asm\x01\x00\x00\x00' + section(1, types) + section(2, imports)
            + section(3, b'\x01\x01') + section(5, b'\x01\x01\x02\x02')
            + section(7, exports) + section(10, b'\x01' + uint(len(body)) + body)
            + section(11, uint(len(segments)) + b''.join(segments)))


def proxy_wat(manifest: dict) -> str:
    lines = [';; Generated reference proxy: actions remain permission checked by the host.',
             '(module', '  (import "rchub" "invoke" (func $invoke (param i32 i32 i32 i32 i32 i32) (result i32)))',
             '  (memory (export "memory") 2 2)',
             '  (func (export "run") (param $entry i32) (param $len i32) (result i32)']
    cursor = 8192
    for action in manifest['actions']:
        cap = action['capability']
        lines += [f'    local.get $entry i32.const {action["entry"]} i32.eq if (result i32)',
                  f'      i32.const {cursor} i32.const {len(cap.encode())} i32.const 0 local.get $len',
                  '      i32.const 32768 i32.const 32768 call $invoke', '    else']
        cursor += len(cap.encode())
    lines += ['      i32.const -1', '    ' + ' '.join(['end'] * len(manifest['actions'])), '  )']
    cursor = 8192
    for action in manifest['actions']:
        cap = action['capability']
        lines.append(f'  (data (i32.const {cursor}) "{cap}")')
        cursor += len(cap.encode())
    return '\n'.join(lines + [')', ''])


def exact(value: object, required: set, optional: set = frozenset()) -> None:
    if not isinstance(value, dict) or not required <= value.keys() or value.keys() - required - optional:
        raise ValueError(f'Expected keys {sorted(required)}, optional {sorted(optional)}')


def localized(value: object, limit: int) -> None:
    exact(value, {'en', 'tr'})
    if any(not isinstance(v, str) or not v or utf16_length(v) > limit for v in value.values()):
        raise ValueError('Invalid localized text')


def identifier(value: object) -> bool:
    return isinstance(value, str) and re.fullmatch(r'[a-z][a-z0-9_]{0,39}', value) is not None and value not in ('constructor', 'prototype', '__proto__')


def field_schema(value: dict, depth: int = 1) -> None:
    if depth > 3:
        raise ValueError('Schema nesting exceeds limit')
    if isinstance(value, dict) and value.get('type') == 'array':
        exact(value, {'type', 'maxItems', 'items'})
        if type(value['maxItems']) is not int or not 1 <= value['maxItems'] <= 64:
            raise ValueError('Array bound required')
        field_schema(value['items'], depth + 1)
        return
    if isinstance(value, dict) and value.get('type') == 'object':
        object_schema(value, depth)
        return
    exact(value, {'type'}, {'minimum', 'maximum', 'maxLength', 'enum'})
    kind = value['type']
    if kind not in ('integer', 'number', 'boolean', 'string'):
        raise ValueError('Unsupported field type')
    if kind in ('integer', 'number'):
        if any(not safe_integer(value.get(k)) for k in ('minimum', 'maximum')) or value['minimum'] > value['maximum']:
            raise ValueError('Numeric bounds required')
    elif 'minimum' in value or 'maximum' in value:
        raise ValueError('Bounds on nonnumeric field')
    if kind == 'string' and (type(value.get('maxLength')) is not int or not 1 <= value['maxLength'] <= 32768):
        raise ValueError('Bounded string required')
    if kind != 'string' and 'maxLength' in value:
        raise ValueError('Length on nonstring field')
    if 'enum' in value:
        if not isinstance(value['enum'], list) or not 1 <= len(value['enum']) <= 64:
            raise ValueError('Bounded enum required')
        plain = {k: v for k, v in value.items() if k != 'enum'}
        if not all(accepts(plain, v) for v in value['enum']):
            raise ValueError('Enum value outside field schema')


def object_schema(value: dict, depth: int = 0) -> None:
    if depth > 3:
        raise ValueError('Schema nesting exceeds limit')
    exact(value, {'type', 'properties', 'required'})
    if value['type'] != 'object' or not isinstance(value['properties'], dict) or len(value['properties']) > 32:
        raise ValueError('Invalid object schema')
    for key, schema in value['properties'].items():
        if not identifier(key):
            raise ValueError('Invalid field name')
        field_schema(schema, depth + 1)
    required = value['required']
    if not isinstance(required, list) or any(not identifier(k) for k in required) or len(set(required)) != len(required) or any(k not in value['properties'] for k in required):
        raise ValueError('Invalid required fields')


def validate_manifest(m: dict) -> None:
    exact(m, {'schemaVersion', 'id', 'version', 'name', 'description', 'publisher',
              'license', 'source', 'targets', 'hostApi', 'permissions', 'actions', 'state', 'ui'})
    if type(m['schemaVersion']) is not int or m['schemaVersion'] != 1 or type(m['hostApi']) is not int or m['hostApi'] != 1:
        raise ValueError('Unsupported schema/host API')
    if not isinstance(m['id'], str) or len(m['id']) > 120 or not re.fullmatch(r'[a-z][a-z0-9]*(?:\.[a-z][a-z0-9]*){2,8}', m['id']):
        raise ValueError('Invalid plugin id')
    if not isinstance(m['version'], str) or len(m['version']) > 32 or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', m['version']):
        raise ValueError('Invalid version')
    localized(m['name'], 80)
    localized(m['description'], 500)
    exact(m['publisher'], {'name', 'kind'})
    if m['publisher']['kind'] not in ('official', 'community') or not isinstance(m['publisher']['name'], str) or not 1 <= utf16_length(m['publisher']['name']) <= 80:
        raise ValueError('Invalid publisher')
    if not isinstance(m['license'], str) or not 1 <= utf16_length(m['license']) <= 40:
        raise ValueError('Invalid license')
    if not isinstance(m['source'], str) or not m['source'].startswith('https://') or utf16_length(m['source']) > 300:
        raise ValueError('HTTPS source required')
    source = urlsplit(m['source'])
    if not source.hostname or source.username or source.password or any(c.isspace() for c in m['source']):
        raise ValueError('Invalid HTTPS source')
    if not isinstance(m['targets'], list) or not m['targets'] or len(set(m['targets'])) != len(m['targets']) or any(p not in ('windows', 'linux') for p in m['targets']):
        raise ValueError('Invalid target list')
    permissions = m['permissions']
    if not isinstance(permissions, list) or len(permissions) > 16 or permissions != sorted(set(permissions)) or any(p not in CAPABILITIES for p in permissions):
        raise ValueError('Invalid permissions')
    if not isinstance(m['actions'], list) or not 1 <= len(m['actions']) <= 32:
        raise ValueError('Invalid action list')
    actions = {}
    entries = set()
    for action in m['actions']:
        exact(action, {'id', 'entry', 'capability', 'input', 'output'})
        if not identifier(action['id']) or action['id'] in actions or type(action['entry']) is not int or not 0 <= action['entry'] <= 31 or action['entry'] in entries or action['capability'] not in permissions:
            raise ValueError('Invalid/duplicate/undeclared action')
        object_schema(action['input'])
        object_schema(action['output'])
        actions[action['id']] = action
        entries.add(action['entry'])
    if not isinstance(m['state'], dict) or len(m['state']) > 32:
        raise ValueError('Invalid state')
    for key, value in m['state'].items():
        if not identifier(key):
            raise ValueError('Invalid state field')
        field_schema(value)
    for action in actions.values():
        if any(k not in m['state'] or schema != m['state'][k] for k, schema in action['output']['properties'].items()):
            raise ValueError('Action output must match declared state schema')
    exact(m['ui'], {'version', 'icon', 'color', 'controls'})
    if type(m['ui']['version']) is not int or m['ui']['version'] != 1 or m['ui']['icon'] not in ICONS or m['ui']['color'] not in COLORS:
        raise ValueError('Invalid UI version/token')
    count = 0
    ids = set()

    def controls(items: list, depth: int) -> None:
        nonlocal count
        if not isinstance(items, list) or depth > 6:
            raise ValueError('Invalid UI nesting')
        for c in items:
            count += 1
            exact(c, {'id', 'type'}, {'label', 'icon', 'tone', 'children', 'action', 'bind', 'argument', 'value', 'min', 'max', 'step', 'options', 'placeholder', 'itemLabel', 'itemValue'})
            kind = c['type']
            if count > 128 or not identifier(c['id']) or c['id'] in ids or kind not in CONTROLS:
                raise ValueError('Invalid/duplicate UI control')
            ids.add(c['id'])
            for key in ('label', 'placeholder'):
                if key in c:
                    localized(c[key], 160)
            if 'icon' in c and c['icon'] not in ICONS or 'tone' in c and c['tone'] not in ('default', 'primary', 'danger', 'muted'):
                raise ValueError('Invalid UI token')
            for key in ('action', 'bind', 'argument', 'itemLabel', 'itemValue'):
                if key in c and not identifier(c[key]):
                    raise ValueError('Invalid UI reference')
            layout = kind in ('section', 'row', 'column', 'tabs')
            if layout:
                if any(key in c for key in ('action', 'bind', 'argument', 'value', 'min', 'max', 'step', 'options', 'placeholder', 'itemLabel', 'itemValue')):
                    raise ValueError('Interactive field on layout')
                if 'children' in c:
                    controls(c['children'], depth + 1)
                continue
            if 'children' in c:
                raise ValueError('Children on leaf')
            action = actions.get(c.get('action'))
            bound = m['state'].get(c.get('bind'))
            argument = action['input']['properties'].get(c.get('argument')) if action else None
            if 'action' in c and not action or 'bind' in c and not bound or 'argument' in c and not argument:
                raise ValueError('Undeclared UI binding')
            if action and any(k != c.get('argument') for k in action['input']['required']):
                raise ValueError('Control cannot supply required action input')
            if kind in ('button', 'slider', 'toggle', 'select', 'input') and not action:
                raise ValueError('Interactive action required')
            if kind in ('value', 'status', 'progress', 'list', 'image') and not bound:
                raise ValueError('Observed state binding required')
            if kind in ('text', 'value', 'status') and bound and bound['type'] in ('array', 'object'):
                raise ValueError('Scalar state required')
            if kind == 'button' and action['input']['required'] and 'value' not in c:
                raise ValueError('Button argument value required')
            if 'value' in c and (not argument or not accepts(argument, c['value']) or isinstance(c['value'], (dict, list))):
                raise ValueError('Invalid static action argument')
            if any(k in c and not safe_integer(c[k]) for k in ('min', 'max', 'step')) or 'step' in c and c['step'] <= 0:
                raise ValueError('Invalid numeric control bound')
            if any(k in c for k in ('min', 'max', 'step')) and kind not in ('slider', 'progress'):
                raise ValueError('Numeric metadata on nonnumeric control')
            if 'placeholder' in c and kind != 'input' or 'options' in c and kind != 'select':
                raise ValueError('Unexpected leaf metadata')
            if kind in ('slider', 'progress'):
                if not bound or bound['type'] not in ('integer', 'number') or not all(k in c for k in ('min', 'max')) or c['min'] >= c['max'] or c['min'] < bound['minimum'] or c['max'] > bound['maximum']:
                    raise ValueError('Invalid numeric state range')
            if kind == 'slider':
                if not argument or argument['type'] != bound['type'] or not safe_integer(c.get('step')) or c['step'] <= 0 or c['min'] < argument['minimum'] or c['max'] > argument['maximum']:
                    raise ValueError('Invalid slider argument range')
            if kind == 'toggle' and (not bound or bound['type'] != 'boolean' or not argument or argument['type'] != 'boolean'):
                raise ValueError('Toggle requires boolean state and argument')
            if kind == 'input' and (not argument or argument['type'] != 'string'):
                raise ValueError('Input requires string argument')
            if kind == 'select':
                options = c.get('options')
                if not argument or not isinstance(options, list) or not 1 <= len(options) <= 64:
                    raise ValueError('Select requires bounded options')
                for option in options:
                    exact(option, {'label', 'value'})
                    localized(option['label'], 160)
                    if isinstance(option['value'], (dict, list)) or not accepts(argument, option['value']):
                        raise ValueError('Invalid select argument')
            if kind == 'image' and bound['type'] != 'string':
                raise ValueError('Image requires string state')
            if kind == 'list':
                if bound['type'] != 'array' or bound['items']['type'] == 'array':
                    raise ValueError('List requires scalar or flat object array')
                item = bound['items']
                if item['type'] == 'object':
                    if 'itemLabel' not in c:
                        raise ValueError('Object list label required')
                    for key in ('itemLabel', 'itemValue'):
                        if key in c and (c[key] not in item['properties'] or item['properties'][c[key]]['type'] in ('array', 'object')):
                            raise ValueError('Invalid list row field')
                    if action and ('itemValue' not in c or not argument or item['properties'][c['itemValue']]['type'] != argument['type']):
                        raise ValueError('List action requires compatible row value')
                elif 'itemLabel' in c or 'itemValue' in c or action and (not argument or item['type'] != argument['type']):
                    raise ValueError('Invalid scalar list metadata')
            elif 'itemLabel' in c or 'itemValue' in c:
                raise ValueError('List metadata on nonlist')
    controls(m['ui']['controls'], 1)


def validate_bundle(bundle: dict) -> None:
    exact(bundle, {'formatVersion', 'manifest', 'wasm'})
    if type(bundle['formatVersion']) is not int or bundle['formatVersion'] != 1 or not isinstance(bundle['wasm'], list) or not 8 <= len(bundle['wasm']) <= 16384 or any(type(v) is not int or not 0 <= v <= 255 for v in bundle['wasm']):
        raise ValueError('Invalid module bytes')
    if bytes(bundle['wasm'][:8]) != b'\x00asm\x01\x00\x00\x00':
        raise ValueError('Invalid Core Wasm header')
    validate_manifest(bundle['manifest'])
    if len(canonical(bundle)) > MAX_PACKAGE:
        raise ValueError('Package exceeds size limit')


def template(plugin_id: str, name: str, official: bool = False) -> dict:
    numeric = {'type': 'integer', 'minimum': 0, 'maximum': 100}
    boolean = {'type': 'boolean'}
    state = {'volume': numeric, 'muted': boolean}
    obj = lambda props: {'type': 'object', 'properties': props, 'required': list(props)}
    return {'schemaVersion': 1, 'id': plugin_id, 'version': '0.1.0',
            'name': {'en': name, 'tr': 'Ses' if official else name},
            'description': {'en': 'Control default Windows output volume and mute.',
                            'tr': 'Varsayılan Windows çıkış sesini ve sessiz durumunu yönetin.'},
            'publisher': {'name': 'RCHub' if official else 'Your name', 'kind': 'official' if official else 'community'},
            'license': 'MIT', 'source': 'https://github.com/CetTek/rchub-plugins/tree/main/plugins/official/audio' if official else 'https://example.com/replace-with-your-source',
            'targets': ['windows'], 'hostApi': 1, 'permissions': ['audio.read', 'audio.write'],
            'actions': [{'id': 'read', 'entry': 0, 'capability': 'audio.read', 'input': obj({}), 'output': obj(state)},
                        {'id': 'set_volume', 'entry': 1, 'capability': 'audio.write', 'input': obj({'volume': numeric}), 'output': obj(state)},
                        {'id': 'set_mute', 'entry': 2, 'capability': 'audio.write', 'input': obj({'muted': boolean}), 'output': obj(state)}],
            'state': state,
            'ui': {'version': 1, 'icon': 'volume', 'color': 'blue', 'controls': [
                {'id': 'audio', 'type': 'section', 'label': {'en': 'Output', 'tr': 'Çıkış'}, 'children': [
                    {'id': 'read', 'type': 'button', 'label': {'en': 'Refresh', 'tr': 'Yenile'}, 'action': 'read'},
                    {'id': 'volume', 'type': 'slider', 'label': {'en': 'Volume', 'tr': 'Ses düzeyi'}, 'bind': 'volume', 'action': 'set_volume', 'argument': 'volume', 'min': 0, 'max': 100, 'step': 1},
                    {'id': 'mute', 'type': 'toggle', 'label': {'en': 'Mute', 'tr': 'Sessize al'}, 'bind': 'muted', 'action': 'set_mute', 'argument': 'muted'}]}]}}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    create = commands.add_parser('create')
    create.add_argument('directory', type=Path)
    create.add_argument('--id', required=True)
    create.add_argument('--name', required=True)
    build = commands.add_parser('build')
    build.add_argument('directory', type=Path)
    build.add_argument('--out', type=Path)
    build.add_argument('--proxy', action='store_true', help='Compile the documented action-to-capability proxy')
    for command in ('validate', 'inspect', 'preview'):
        p = commands.add_parser(command)
        p.add_argument('package', type=Path)
    args = parser.parse_args()
    if args.command == 'create':
        manifest = template(args.id, args.name)
        validate_manifest(manifest)
        args.directory.mkdir(parents=True, exist_ok=False)
        (args.directory / 'manifest.json').write_bytes(canonical(manifest))
        (args.directory / 'guest.wat').write_text(proxy_wat(manifest), encoding='utf-8', newline='\n')
        print('Created local Audio-capability starter. Replace publisher/source before distribution.')
    elif args.command == 'build':
        manifest = json.loads((args.directory / 'manifest.json').read_text(encoding='utf-8'))
        validate_manifest(manifest)
        module = proxy_wasm(manifest) if args.proxy else (args.directory / 'guest.wasm').read_bytes()
        bundle = {'formatVersion': 1, 'manifest': manifest, 'wasm': list(module)}
        validate_bundle(bundle)
        target = args.out or args.directory / 'plugin.rchub-plugin.json'
        target.write_bytes(canonical(bundle))
        print(json.dumps(identity(bundle)))
    else:
        if args.package.stat().st_size > MAX_PACKAGE:
            raise ValueError('Oversized package')
        bundle = json.loads(args.package.read_text(encoding='utf-8'))
        validate_bundle(bundle)
        print(json.dumps({'package': identity(bundle), 'manifest': bundle['manifest']}, ensure_ascii=False, indent=2))
        if args.command == 'preview':
            print('Open http://localhost:8081/plugin-preview after starting mobile/pnpm dev; paste this package into the standalone preview. No device operation was issued.')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError) as error:
        raise SystemExit(f'Plugin rejected: {error}') from None
