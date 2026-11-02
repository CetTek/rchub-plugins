"""Repo-local experimental application SDK. No V1 import, real broker or execution.

Only schemas in contracts/plugins/v2 use this small closed JSON Schema vocabulary.
It is deliberately not a general-purpose JSON Schema implementation.
"""
from __future__ import annotations

import copy
import base64
import hashlib
import json
import re
import struct
from urllib.parse import urlsplit
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCHEMAS = ROOT / 'contracts/plugins/v2'
MAX_FRAME = 1048576
MAX_TRANSPORT = MAX_FRAME + 4096
MAX_INVOKE_PAYLOAD = MAX_FRAME - 4096
MAX_ARTIFACT = 16 * 1024 * 1024
MAX_DEPTH = 32
MAX_IDS = 256
MAX_CHUNK = 32768
STORE_VALUE_BYTES = 8192
STORE_KEYS = 128
STORE_TOTAL_BYTES = 262144
PROTOCOL = 'rchub-app-v2-experimental'
FEATURES = frozenset({'native-process-v1', 'events-v1', 'surfaces-v1', 'scoped-store-v1', 'image-v1', 'string-8192-v1', 'list-item-actions-v1', 'surface-metadata-v1'})
RESERVED = {'constructor', 'prototype', '__proto__'}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _json_tree(value: object, depth: int = 0) -> None:
    require(depth <= MAX_DEPTH, 'JSON nesting limit')
    if isinstance(value, dict):
        for key, child in value.items():
            require(type(key) is str and key not in RESERVED, 'Invalid object key')
            _json_tree(key, depth + 1)
            _json_tree(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            _json_tree(child, depth + 1)
    elif type(value) is str:
        require(not any(0xD800 <= ord(c) <= 0xDFFF for c in value), 'Lone surrogate')
    else:
        require(value is None or type(value) is bool or
                (type(value) is int and abs(value) <= 9007199254740991),
                'Only JSON safe integers, strings, booleans, null and containers')


def canonical(value: object) -> bytes:
    """Deterministic integer-only UTF-8 encoding, including one final LF."""
    _json_tree(value)
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')


def strict_json(raw: bytes, max_bytes: int = MAX_FRAME) -> object:
    """Reject duplicate keys, floats, nonfinite numbers, bad Unicode and deep JSON."""
    require(type(raw) is bytes and 0 < len(raw) <= max_bytes, 'JSON byte limit')

    def pairs(entries):
        result = {}
        for key, value in entries:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result

    def invalid_number(_):
        raise ValueError('Only safe integer numbers')

    try:
        value = json.loads(raw.decode('utf-8'), object_pairs_hook=pairs,
                           parse_float=invalid_number, parse_constant=invalid_number)
        _json_tree(value)
        return value
    except (UnicodeError, RecursionError) as error:
        raise ValueError('Invalid UTF-8 or JSON nesting') from error


def _matches(schema: dict, value: object, root: dict) -> bool:
    try:
        _check(schema, value, root)
        return True
    except ValueError:
        return False


def _check(schema: dict, value: object, root: dict) -> None:
    if '$ref' in schema:
        require(schema['$ref'].startswith('#/$defs/'), 'Only local schema references')
        _check(root['$defs'][schema['$ref'].split('/')[-1]], value, root)
        return
    if 'oneOf' in schema:
        require(sum(_matches(s, value, root) for s in schema['oneOf']) == 1, 'Invalid variant')
    if 'const' in schema:
        require(type(value) is type(schema['const']) and value == schema['const'], 'Constant mismatch')
    if 'enum' in schema:
        require(any(type(value) is type(v) and value == v for v in schema['enum']), 'Unknown value')
    types = {'object': dict, 'array': list, 'string': str, 'integer': int, 'boolean': bool}
    if 'type' in schema:
        require(type(value) is types[schema['type']], 'Type mismatch')
    if isinstance(value, dict):
        require(set(schema.get('required', [])) <= value.keys(), 'Missing required field')
        require(schema.get('minProperties', 0) <= len(value) <= schema.get('maxProperties', MAX_FRAME), 'Object size')
        for key, child in value.items():
            if 'propertyNames' in schema:
                _check(schema['propertyNames'], key, root)
            sub = schema.get('properties', {}).get(key, schema.get('additionalProperties', True))
            require(sub is not False, 'Unknown field')
            if isinstance(sub, dict):
                _check(sub, child, root)
    elif isinstance(value, list):
        require(schema.get('minItems', 0) <= len(value) <= schema.get('maxItems', MAX_FRAME), 'Array size')
        if schema.get('uniqueItems'):
            require(len({canonical(x) for x in value}) == len(value), 'Duplicate item')
        for child in value:
            if 'items' in schema:
                _check(schema['items'], child, root)
    elif type(value) is str:
        require(schema.get('minLength', 0) <= len(value) <= schema.get('maxLength', MAX_FRAME), 'String size')
        if 'pattern' in schema:
            require(re.fullmatch(schema['pattern'], value) is not None, 'Invalid identifier/path')
    elif type(value) is int:
        require(schema.get('minimum', -9007199254740991) <= value <= schema.get('maximum', 9007199254740991), 'Integer range')


def _validate_file(name: str, value: object) -> None:
    bound = 65536 if name == 'application.schema.json' else MAX_TRANSPORT if name == 'transport.schema.json' else MAX_FRAME
    require(len(canonical(value)) <= bound, 'Document byte limit')
    schema = json.loads((SCHEMAS / name).read_text(encoding='utf-8'))
    _check(schema, value, schema)


def _record(schema: dict, depth: int = 0) -> None:
    require(depth <= 3, 'Payload schema depth')
    kind = schema['type']
    if kind == 'object':
        require(set(schema['required']) <= schema['properties'].keys(), 'Unknown required payload field')
        for field in schema['properties'].values():
            _record(field, depth + 1)
    elif kind == 'integer':
        require(schema['minimum'] <= schema['maximum'], 'Inverted payload bounds')
    elif kind == 'array':
        _record(schema['items'], depth + 1)


def validate_manifest(manifest: dict, supported=FEATURES) -> None:
    """Validate schema + references/features; no artifact loading or grants issued."""
    _validate_file('application.schema.json', manifest)
    source = urlsplit(manifest['source'])
    require(source.scheme == 'https' and bool(source.hostname) and
            not source.username and not source.password, 'Invalid source disclosure URL')
    _record(manifest['state'])
    features = set(manifest['requiredFeatures'])
    def feature_fields(field):
        if field['type'] == 'image':
            require('image-v1' in features, 'Image feature required')
        if field['type'] == 'string' and field['maxLength'] > 4096:
            require('string-8192-v1' in features, 'Extended string feature required')
        if field['type'] == 'string' and 'enum' in field:
            require('surface-metadata-v1' in features and all(len(v) <= field['maxLength'] for v in field['enum']), 'Invalid enum feature or values')
        for child in field.get('properties', {}).values():
            feature_fields(child)
        if field['type'] == 'array':
            feature_fields(field['items'])
    feature_fields(manifest['state'])
    for action in manifest['actions'].values():
        feature_fields(action['input']); feature_fields(action['output'])
    for event in manifest['events'].values():
        feature_fields(event)
    require(features <= set(supported), 'Unsupported required feature')
    require('native-process-v1' in features, 'Native feature required')
    if manifest['events']:
        require('events-v1' in features, 'Events feature required')
    if manifest['surfaces']:
        require('surfaces-v1' in features, 'Surfaces feature required')
    choices = manifest['storage']['choices']
    require('none' not in choices or choices == ['none'], 'None storage is exclusive')
    if 'host-scoped' in choices:
        require('scoped-store-v1' in features, 'Scoped store feature required')
    entry = manifest['execution']['artifact']['entry']
    reserved_files = {'con', 'prn', 'aux', 'nul'} | {f'{p}{i}' for p in ('com', 'lpt') for i in range(1, 10)}
    for part in entry.split('/'):
        require(part not in ('.', '..') and not part.endswith('.') and
                part.split('.')[0].lower() not in reserved_files, 'Unsafe relative artifact entry')
    for name, action in manifest['actions'].items():
        _name(manifest, name)
        _record(action['input'])
        _record(action['output'])
    for name, record in manifest['events'].items():
        _name(manifest, name)
        _record(record)
    ids = set()
    for surface in manifest['surfaces']:
        if 'icon' in surface or 'color' in surface:
            require('surface-metadata-v1' in features, 'Surface metadata feature required')
        require(surface['id'] not in ids, 'Duplicate surface ID')
        ids.add(surface['id'])
        actions = surface['actions'] if surface['kind'] == 'declarative' else [surface['action']]
        require(set(actions) <= manifest['actions'].keys(), 'Unknown surface action')
        require(set(surface.get('events', [])) <= manifest['events'].keys(), 'Unknown surface event')
        if surface['kind'] == 'declarative':
            _controls(manifest, surface)


def _name(manifest: dict, name: str) -> None:
    require(re.fullmatch(r'[a-z][a-z0-9]*(\.[a-z][a-z0-9_]*){2,10}', name) is not None
            and len(name) <= 160 and name.startswith(manifest['id'] + '.'), 'Name outside plugin namespace')


def _controls(manifest: dict, surface: dict) -> None:
    """Closed V1-style direct fields, no expressions or executable UI."""
    flat = {}

    def walk(nodes, depth=1):
        require(depth <= 6, 'Control nesting limit')
        for control in nodes:
            key, kind = control['id'], control['type']
            require(key not in flat and key not in RESERVED and len(flat) < 128, 'Duplicate/control limit')
            if kind == 'tabs' or any(k in control for k in ('icon', 'tone', 'confirm', 'appearance', 'inputMode', 'placeholder', 'searchable')):
                require('surface-metadata-v1' in manifest['requiredFeatures'], 'Surface metadata feature required')
            flat[key] = control
            if kind in ('section', 'row', 'column', 'tabs'):
                require(set(control) <= {'id', 'type', 'label', 'icon', 'tone', 'children'}, 'Layout metadata')
                walk(control.get('children', []), depth + 1)
            else:
                require('children' not in control, 'Leaf children')
    walk(surface['controls'])
    state = manifest['state']['properties']
    for c in flat.values():
        kind = c['type']
        if kind in ('section', 'row', 'column', 'tabs'):
            continue
        action = manifest['actions'].get(c.get('action'))
        bound = state.get(c.get('bind'))
        arg = action['input']['properties'].get(c.get('argument')) if action else None
        require('action' not in c or c['action'] in surface['actions'], 'Action outside surface')
        require('bind' not in c or bound is not None, 'Unknown state binding')
        require('argument' not in c or arg is not None, 'Unknown input binding')
        if action:
            require(kind in ('button', 'toggle', 'slider', 'input', 'select', 'list'), 'Action on display-only control')
        require('value' not in c or kind == 'button', 'Static value on nonbutton')
        require('appearance' not in c or kind == 'progress', 'Appearance on nonprogress')
        require('searchable' not in c or kind == 'list', 'Search on nonlist')
        require('placeholder' not in c or kind == 'input', 'Placeholder on noninput')
        require('confirm' not in c or action is not None, 'Confirm without action')
        if 'inputMode' in c:
            require(kind == 'input' and arg is not None and (arg['type'] == 'integer' if c['inputMode'] == 'numeric' else arg['type'] == 'string'), 'Input mode mismatch')
        if kind in ('button', 'toggle', 'slider', 'input', 'select'):
            require(action is not None, 'Interactive action required')
        if kind in ('value', 'status', 'toggle', 'slider', 'progress', 'list', 'image'):
            require(bound is not None, 'Observed binding required')
        if kind == 'image':
            require('image-v1' in manifest['requiredFeatures'] and bound['type'] == 'image' and set(c) <= {'id', 'type', 'label', 'bind', 'icon', 'tone'}, 'Invalid image control')
        if kind in ('text', 'value', 'status') and bound:
            require(bound['type'] not in ('object', 'array', 'image'), 'Scalar display required')
        if any(k in c for k in ('min', 'max', 'step')):
            require(kind in ('slider', 'progress'), 'Numeric metadata on nonnumeric control')
        if kind in ('slider', 'progress'):
            require(bound['type'] == 'integer' and {'min', 'max'} <= c.keys() and
                    bound['minimum'] <= c['min'] < c['max'] <= bound['maximum'], 'Numeric binding bounds')
        if kind == 'slider':
            require(arg is not None and arg['type'] == 'integer' and 'step' in c and
                    arg['minimum'] <= c['min'] < c['max'] <= arg['maximum'], 'Slider argument bounds')
        if kind == 'toggle':
            require(arg is not None and arg['type'] == bound['type'] == 'boolean', 'Boolean toggle required')
        if kind in ('input', 'select'):
            require(arg is not None and arg['type'] in ('string', 'integer', 'boolean'), 'Scalar draft required')
        if 'options' in c or kind == 'select':
            require(kind == 'select' and 'options' in c, 'Select options required')
            for option in c['options']:
                _payload(arg, option['value'])
        if 'submit' in c:
            require(kind in ('input', 'select'), 'Submit on nondraft')
        if 'multiline' in c:
            require(kind == 'input' and arg['type'] == 'string', 'Multiline on nontext')
        if 'defaultValue' in c:
            require(kind in ('input', 'select'), 'Default on nondraft')
            _payload(arg, c['defaultValue'])
            if kind == 'select':
                require(any(type(o['value']) is type(c['defaultValue']) and o['value'] == c['defaultValue']
                            for o in c['options']), 'Default outside options')
        if 'value' in c:
            require(arg is not None, 'Static value without argument')
            _payload(arg, c['value'])
        supplied = set(c.get('inputBindings', {}))
        if arg:
            supplied.add(c['argument'])
        if action and c.get('submit') is not False:
            require(set(action['input']['required']) <= supplied, 'Missing required UI input')
        if kind == 'button' and arg:
            require('value' in c, 'Static button value required')
        for target, source in c.get('inputBindings', {}).items():
            draft = flat.get(source, {})
            draft_action = manifest['actions'].get(draft.get('action'), {})
            field = draft_action.get('input', {}).get('properties', {}).get(draft.get('argument'))
            require(action is not None and target != c.get('argument') and
                    draft.get('type') in ('input', 'select') and field is not None and
                    action['input']['properties'].get(target) == field, 'Incompatible draft binding')
        if kind == 'list':
            require(bound['type'] == 'array' and bound['items']['type'] != 'array', 'Bounded list required')
            item = bound['items']
            if item['type'] == 'object':
                require('itemLabel' in c, 'Object list label required')
                for key in ('itemLabel', 'itemValue'):
                    if key in c:
                        require(c[key] in item['properties'] and
                                item['properties'][c[key]]['type'] not in ('array', 'object'), 'Invalid row field')
                if action:
                    require('itemValue' in c and item['properties'][c['itemValue']] == arg, 'List argument mismatch')
            else:
                require(not any(k in c for k in ('itemLabel', 'itemValue')) and
                        (not action or item == arg), 'Scalar list argument mismatch')
            if 'itemActions' in c:
                require('list-item-actions-v1' in manifest['requiredFeatures'] and item['type'] == 'object' and not action, 'Invalid row actions')
                fields = item['properties']
                for row in c['itemActions']:
                    require(row['action'] in surface['actions'], 'Row action outside surface')
                    target = manifest['actions'][row['action']]['input']
                    supplied = set()
                    for name, value in row.get('args', {}).items():
                        require(name in target['properties'], 'Unknown row argument'); _payload(target['properties'][name], value); supplied.add(name)
                    for name, source in row.get('itemArguments', {}).items():
                        require(name not in supplied and source in item['required'] and name in target['properties'] and fields[source] == target['properties'][name], 'Incompatible row field'); supplied.add(name)
                    if 'argument' in row:
                        name = row['argument']
                        require(name not in supplied and name in target['properties'], 'Duplicate row argument')
                        if 'value' in row: _payload(target['properties'][name], row['value'])
                        else: require(c.get('itemValue') in item['required'] and fields[c['itemValue']] == target['properties'][name], 'Incompatible row value')
                        supplied.add(name)
                    else: require('value' not in row, 'Row value without argument')
                    require(set(target['required']) <= supplied, 'Missing row argument')
        else:
            require(not any(k in c for k in ('itemLabel', 'itemValue', 'itemActions')), 'Row metadata on nonlist')
        require('step' not in c or kind == 'slider', 'Step on nonslider')
        require('inputBindings' not in c or action is not None, 'Draft bindings without action')


def identity(manifest: dict) -> dict:
    """Experimental manifest identity; artifact digest is transitively included."""
    validate_manifest(manifest)
    return {'id': manifest['id'], 'version': manifest['version'],
            'sha256': hashlib.sha256(canonical(manifest)).hexdigest()}


def verify_artifact(manifest: dict, target: str, entry: str, content: bytes) -> None:
    """Verify caller-supplied artifact bytes, never open a path or start a process.

    Does not prove installed file containment, absence of links or TOCTOU safety.
    """
    validate_manifest(manifest)
    artifact = manifest['execution']['artifact']
    require(target == artifact['target'] and entry == artifact['entry'], 'Artifact target/entry mismatch')
    require(type(content) is bytes and 0 < len(content) <= MAX_ARTIFACT, 'Artifact byte limit')
    require(hashlib.sha256(content).hexdigest() == artifact['sha256'], 'Artifact digest mismatch')


def encode_frame(message: dict) -> bytes:
    """Encode one strict message with unsigned big-endian 4-byte byte length."""
    _validate_file('protocol.schema.json', message)
    raw = canonical(message)
    return struct.pack('>I', len(raw)) + raw


class FrameDecoder:
    """Incremental, bounded decoder. Protocol error poisons this stream."""

    def __init__(self):
        self.buffer = bytearray()
        self.expected = None
        self.failed = False

    def feed(self, chunk: bytes) -> list[dict]:
        require(not self.failed, 'Decoder closed after error')
        require(type(chunk) is bytes, 'Bytes required')
        result = []
        cursor = 0
        try:
            while cursor < len(chunk):
                bound = 4 if self.expected is None else self.expected
                take = min(bound - len(self.buffer), len(chunk) - cursor)
                self.buffer.extend(chunk[cursor:cursor + take])
                cursor += take
                if len(self.buffer) != bound:
                    continue
                if self.expected is None:
                    self.expected = struct.unpack('>I', self.buffer)[0]
                    self.buffer.clear()
                    require(0 < self.expected <= MAX_FRAME, 'Frame byte limit')
                else:
                    message = strict_json(bytes(self.buffer))
                    _validate_file('protocol.schema.json', message)
                    result.append(message)
                    self.buffer.clear()
                    self.expected = None
            return result
        except ValueError:
            self.failed = True
            self.buffer.clear()
            raise

    def finish(self) -> None:
        require(not self.failed, 'Decoder closed after error')
        if self.expected is not None or self.buffer:
            self.failed = True
            self.buffer.clear()
            raise ValueError('Truncated frame')


def _payload(schema: dict, payload: dict) -> None:
    if schema['type'] == 'object':
        require(type(payload) is dict and set(schema['required']) <= payload.keys()
                and payload.keys() <= schema['properties'].keys(), 'Undeclared/missing payload field')
        for key, value in payload.items():
            _payload(schema['properties'][key], value)
    elif schema['type'] == 'array':
        require(type(payload) is list and len(payload) <= schema['maxItems'], 'Array payload limit')
        for value in payload:
            _payload(schema['items'], value)
    elif schema['type'] == 'image':
        require(type(payload) is str and len(payload) <= schema['maxLength'] and payload.startswith('data:image/jpeg;base64,'), 'Invalid image')
        encoded = payload[len('data:image/jpeg;base64,'):]
        try:
            raw = base64.b64decode(encoded, validate=True)
        except (ValueError, base64.binascii.Error) as error:
            raise ValueError('Invalid image base64') from error
        require(base64.b64encode(raw).decode() == encoded and raw.startswith(b'\xff\xd8') and raw.endswith(b'\xff\xd9') and len(raw) >= 4, 'Invalid JPEG')
    else:
        _check(schema, payload, schema)


class MockSession:
    """Deterministic bidirectional protocol model, NOT authentication or a broker.

    Caller supplies opaque instance/context IDs. No owner checks, installed grant,
    process supervision, persistence, timers, sockets, native calls or effects.
    """

    def __init__(self, manifest: dict, instance_id: str, context_id: str, supported=FEATURES):
        validate_manifest(manifest, supported)
        self.manifest = copy.deepcopy(manifest)
        self.instance_id = instance_id
        self.context_id = context_id
        self.active = False
        self.closed = False
        self.pending = {}
        self.used = set()
        self.subscriptions = set()

    def accept(self, message: dict) -> None:
        """Accept one decoded frame; rejected frames cannot advance model state."""
        _validate_file('protocol.schema.json', message)
        require(not self.closed and message['instanceId'] == self.instance_id and
                message['contextId'] == self.context_id, 'Closed or stale instance/context')
        body = message['body']
        sender = message['sender']
        other = 'plugin' if sender == 'host' else 'host'
        kind = body['kind']
        if kind == 'lifecycle':
            require(sender == 'plugin', 'Only simulated plugin reports lifecycle')
            if body['state'] == 'started':
                require(not self.active, 'Duplicate start')
                self.active = True
            else:
                require(self.active, 'Stop before start')
                self.active = False
                self.closed = True
                self.pending.clear()
                self.subscriptions.clear()
            return
        require(self.active, 'Instance not started')
        if kind == 'event':
            require((other, body['subscriptionId'], body['name']) in self.subscriptions,
                    'Unknown subscription or wrong event direction')
            _payload(self.manifest['events'][body['name']], body['payload'])
            return
        if kind == 'request':
            key = (sender, body['requestId'])
            require(key not in self.used and len(self.used) < MAX_IDS and len(self.pending) < 64,
                    'Duplicate request or model capacity exhausted')
            operation = body['operation']
            declarations = self.manifest['actions'] if operation == 'invoke' else self.manifest['events']
            require(body['name'] in declarations, 'Undeclared action/event')
            require(('subscriptionId' in body) == (operation == 'unsubscribe'), 'Unexpected/missing subscription ID')
            if operation == 'invoke':
                _payload(declarations[body['name']]['input'], body['payload'])
            else:
                require(body['payload'] == {}, 'Subscription payload must be empty')
                if operation == 'unsubscribe':
                    require((sender, body['subscriptionId'], body['name']) in self.subscriptions, 'Unknown subscription')
            self.pending[key] = copy.deepcopy(body)
            self.used.add(key)
            return
        key = (other, body['requestId'])
        request = self.pending.get(key)
        require(request is not None and request['name'] == body['name'] and
                request['operation'] == body['operation'], 'Uncorrelated result')
        if body['status'] == 'ok' and body['operation'] == 'invoke':
            _payload(self.manifest['actions'][body['name']]['output'], body['payload'])
        else:
            require(body['payload'] == {}, 'Unexpected result payload')
        if body['status'] == 'ok':
            if body['operation'] == 'subscribe':
                self.subscriptions.add((other, body['requestId'], body['name']))
            elif body['operation'] == 'unsubscribe':
                self.subscriptions.discard((other, request['subscriptionId'], body['name']))
        del self.pending[key]

    def invalidate(self) -> None:
        """Simulate host context loss/revocation; no native revocation guarantee."""
        self.closed = True
        self.active = False
        self.pending.clear()
        self.subscriptions.clear()


class ProtocolSession(MockSession):
    """Strict negotiated process session reference, not authority or supervision.

    Monotonic sender request IDs (decimal strings without leading zeros) keep
    replay memory constant for long-lived sessions. MockSession's 256-ID limit
    remains only a legacy fixture model.
    """

    def __init__(self, manifest, instance_id, context_id, supported=FEATURES, version=1):
        super().__init__(manifest, instance_id, context_id, supported)
        require(type(version) is int and version in (1, 2), 'Unsupported session revision')
        self.version = version
        self.phase = 'new'
        self.features = set(supported)
        self.offered = set()
        self.highwater = {'host': 0, 'plugin': 0}
        self.cancelled = set()
        self.surfaces = {}

    def accept(self, message):
        _validate_file('protocol.schema.json', message)
        require(not self.closed and message['instanceId'] == self.instance_id and
                message['contextId'] == self.context_id, 'Closed or stale instance/context')
        body, sender = message['body'], message['sender']
        kind = body['kind']
        peer = 'plugin' if sender == 'host' else 'host'
        if kind == 'hello':
            require(body['version'] == self.version, 'Session revision mismatch')
            require(sender == 'host' and self.phase == 'new', 'Unexpected hello')
            offered = set(body['features'])
            require(set(self.manifest['requiredFeatures']) <= offered <= self.features, 'Unsupported negotiation')
            self.offered, self.phase = offered, 'hello'
            return
        if kind == 'welcome':
            require(body['version'] == self.version, 'Session revision mismatch')
            chosen = set(body['features'])
            require(sender == 'plugin' and self.phase == 'hello' and
                    set(self.manifest['requiredFeatures']) <= chosen <= self.offered, 'Unexpected welcome/features')
            self.features, self.phase = chosen, 'welcome'
            return
        if kind == 'lifecycle' and body['state'] == 'started':
            require(self.phase == 'welcome', 'Start before negotiation')
            super().accept(message)
            self.phase = 'active'
            return
        if kind == 'shutdown':
            require(sender == 'host' and self.active and self.phase == 'active', 'Unexpected shutdown')
            self.phase = 'stopping'
            self.pending.clear()
            self.subscriptions.clear()
            self.surfaces.clear()
            return
        if kind == 'lifecycle':
            super().accept(message)
            self.phase = 'closed'
            self.surfaces.clear()
            self.cancelled.clear()
            return
        require(self.active and self.phase == 'active', 'Session not accepting work')
        if kind == 'cancel':
            key = (sender, body['requestId'])
            require(key in self.pending and key not in self.cancelled, 'Unknown/duplicate cancellation')
            self.cancelled.add(key)
            return
        if kind == 'state':
            require(sender == 'plugin' and body['surfaceId'] in self.surfaces, 'State for closed surface')
            previous = self.surfaces[body['surfaceId']]
            require(body['revision'] == previous + 1, 'State revision gap/replay')
            _payload(self.manifest['state'], body['values'])
            self.surfaces[body['surfaceId']] = body['revision']
            return
        if kind == 'request':
            rid = body['requestId']
            require(re.fullmatch(r'[1-9][0-9]{0,15}', rid) is not None and
                    int(rid) <= 9007199254740991 and int(rid) > self.highwater[sender], 'Request sequence replay')
            require(len(self.pending) < 64, 'Pending capacity')
            operation = body['operation']
            if operation.startswith('store.') or operation.startswith('surface.'):
                self._special_request(body, sender)
                self.pending[(sender, rid)] = copy.deepcopy(body)
            else:
                # Mock's small used-ID corpus is not a production lifetime bound.
                if operation == 'subscribe':
                    require(body['name'] in self.manifest['events'], 'Undeclared event')
                if operation == 'invoke':
                    require(body['name'] in self.manifest['actions'], 'Undeclared action')
                if operation == 'subscribe':
                    require(len(self.subscriptions) + sum(r['operation'] == 'subscribe'
                            for r in self.pending.values()) < 64, 'Subscription capacity')
                self.used.clear()
                super().accept(message)
                self.used.clear()
            self.highwater[sender] = int(rid)
            return
        if kind == 'result':
            key = (peer, body['requestId'])
            request = self.pending.get(key)
            require(request is not None and request['operation'] == body['operation'] and
                    request['name'] == body['name'], 'Uncorrelated result')
            require(body['status'] != 'cancelled' or key in self.cancelled, 'Unrequested cancellation')
            operation = body['operation']
            if operation.startswith('store.') or operation.startswith('surface.'):
                self._special_result(request, body)
                del self.pending[key]
            else:
                require(body['status'] != 'conflict', 'Conflict only for store CAS')
                super().accept(message)
            self.cancelled.discard(key)
            return
        super().accept(message)

    def _special_request(self, body, sender):
        op, payload = body['operation'], body['payload']
        require('subscriptionId' not in body, 'Unexpected subscription ID')
        if op.startswith('store.'):
            require(sender == 'plugin' and 'scoped-store-v1' in self.features and
                    'host-scoped' in self.manifest['storage']['choices'] and
                    body['name'] == 'rchub.store.value', 'Store not negotiated/scoped')
            validate_store_request(op, payload)
        else:
            require(sender == 'host' and body['name'] == 'rchub.surface.lifecycle' and
                    set(payload) == {'surfaceId'}, 'Surface request direction/payload')
            surface = next((s for s in self.manifest['surfaces'] if s['id'] == payload['surfaceId']
                            and s['kind'] == 'declarative'), None)
            require(surface is not None, 'Unknown declarative surface')
            require((payload['surfaceId'] in self.surfaces) == (op == 'surface.close'), 'Surface lifecycle conflict')
            require(not any(r['operation'].startswith('surface.') and r['payload'] == payload
                            for r in self.pending.values()), 'Surface already pending')

    def _special_result(self, request, body):
        op, payload, status = body['operation'], body['payload'], body['status']
        if status != 'ok':
            require(payload == {} and (status != 'conflict' or op == 'store.put'), 'Invalid special result')
            return
        if op.startswith('surface.'):
            require(payload == {}, 'Surface result payload')
            sid = request['payload']['surfaceId']
            if op == 'surface.open':
                self.surfaces[sid] = 0
            else:
                self.surfaces.pop(sid)
            return
        if op == 'store.get':
            require(type(payload.get('found')) is bool and type(payload.get('revision')) is int,
                    'Store result fields')
            if payload['found']:
                require(set(payload) == {'found', 'revision', 'value'} and payload['revision'] >= 1,
                        'Store found result')
                _store_value(payload['value'])
            else:
                require(payload == {'found': False, 'revision': 0}, 'Store missing result')
        else:
            require(set(payload) == {'revision'} and type(payload['revision']) is int and
                    payload['revision'] == request['payload']['expectedRevision'] + 1, 'Store put revision')

    def invalidate(self):
        super().invalidate()
        self.phase = 'closed'
        self.surfaces.clear()
        self.cancelled.clear()


def _store_value(value):
    require(type(value) is dict and len(canonical(value)) <= STORE_VALUE_BYTES, 'Store value byte/object limit')


def validate_store_request(operation, payload):
    require(type(payload) is dict and operation in ('store.get', 'store.put'), 'Unknown store operation')
    keys = {'key'} if operation == 'store.get' else {'key', 'expectedRevision', 'value'}
    require(set(payload) == keys and type(payload['key']) is str and
            re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}', payload['key']) is not None and
            payload['key'] not in RESERVED, 'Invalid store key/fields')
    if operation == 'store.put':
        require(type(payload['expectedRevision']) is int and 0 <= payload['expectedRevision'] < 9007199254740991,
                'Invalid store revision')
        _store_value(payload['value'])


class ScopedStore:
    """In-memory CAS reference for ONE already-authorized host scope; not persistence.

    Host selects (origin, device, current owner, plugin ID) scope, never caller.
    Updates keep scope only after explicit same-ID native update consent.
    """

    def __init__(self):
        self.values = {}

    def execute(self, operation, payload):
        validate_store_request(operation, payload)
        key = payload['key']
        previous = self.values.get(key)
        if operation == 'store.get':
            return {'found': False, 'revision': 0} if previous is None else {
                'found': True, 'revision': previous[0], 'value': copy.deepcopy(previous[1])}
        revision = previous[0] if previous else 0
        require(payload['expectedRevision'] == revision, 'Store revision conflict')
        candidate = copy.deepcopy(self.values)
        candidate[key] = (revision + 1, copy.deepcopy(payload['value']))
        require(len(candidate) <= STORE_KEYS and
                sum(len(canonical(v[1])) for v in candidate.values()) <= STORE_TOTAL_BYTES, 'Store quota exceeded')
        self.values = candidate
        return {'revision': revision + 1}


def validate_transport(message, expected_version=None):
    """Validate application wire v3 including strict per-operation field ownership.

    Does not authenticate a sender. Only a trusted adapter can construct dispatch.
    """
    _validate_file('transport.schema.json', message)
    if expected_version is not None:
        require(type(expected_version) is int and expected_version in (1, 2), 'Unsupported wire revision')
        # Revision 1 envelopes omit version except capabilities. Revision 2
        # requires it on EVERY frame, so stripping a security field is no downgrade.
        require(message.get('version', 1) == expected_version, 'Wire revision mismatch')
    kind = message['kind']
    revision2 = message.get('version') == 2
    if kind == 'owner-context':
        require(revision2 and set(message) == {'v', 'kind', 'version', 'epoch', 'binding',
                'authorityRevision', 'state', 'reason'}, 'Owner context fields/revision')
        require((message['state'] == 'valid') == (message['reason'] == 'owner-validated'), 'Owner context reason')
        return
    if kind == 'capabilities':
        require(set(message) == {'v', 'kind', 'epoch', 'protocol', 'version', 'features'}, 'Capabilities fields')
        require('native-process-v1' in message['features'], 'Native support missing')
        return
    client = {'requestId', 'deviceId'}
    device = {'operationId', 'epoch'}
    if kind == 'event':
        revfields = {'version'} if revision2 else set()
        require(set(message) in ({'v', 'kind', 'package', 'message'} | client | revfields,
                                {'v', 'kind', 'package', 'message'} | device | revfields), 'Event routing fields')
        _validate_file('protocol.schema.json', message['message'])
        require(message['message']['sender'] == 'plugin' and
                message['message']['body']['kind'] in ('event', 'state', 'lifecycle'), 'Invalid pushed observation')
        return
    action = message.get('action')
    require(action is not None, 'Missing action')
    routing = device | {'deadline', 'binding'} if kind == 'dispatch' else client
    if kind == 'result':
        has_client = bool(set(message) & client)
        routing = client if has_client else device
        base = {'v', 'kind', 'action', 'status'} | routing | ({'version'} if revision2 else set())
        require(base <= set(message), 'Result routing fields')
        status = message['status']
        extra = set(message) - base
        if status != 'ok':
            require(extra <= {'error'} and ('error' not in extra or status == 'failed'), 'Failure leaks observations')
            return
        allowed = {
            'inventory': {'entries'}, 'install.begin': {'transferId'}, 'install.chunk': {'transferId', 'offset'},
            'install.commit': {'entries'}, 'install.abort': set(), 'remove': {'entries'},
            'activate': {'package', 'instanceId', 'contextId'}, 'deactivate': set(),
            'invoke': {'package', 'payload'}, 'subscribe': {'package', 'subscriptionId'},
            'unsubscribe': set(), 'cancel': set(), 'surface.open': set(), 'surface.close': set(),
        }[action]
        optional = {'nextCursor'} if action in ('inventory', 'install.commit', 'remove') else set()
        require(allowed <= extra <= allowed | optional, 'Successful result fields')
        if action == 'invoke':
            require(len(canonical(message['payload'])) <= MAX_FRAME, 'Result payload byte limit')
        for entry in message.get('entries', []):
            validate_manifest(entry['manifest'])
            require(entry['package'] == identity(entry['manifest']), 'Inventory identity mismatch')
            require(('instanceId' in entry and 'contextId' in entry) == (entry['status'] == 'active') and
                    ('instanceId' in entry) == ('contextId' in entry), 'Inventory lifecycle identity')
        return
    require(kind in ('request', 'dispatch'), 'Unknown envelope kind')
    base = {'v', 'kind', 'action'} | routing | ({'version'} if revision2 else set())
    if revision2 and kind == 'dispatch':
        base |= {'authorityRevision'}
    fields = {
        'inventory': set(),
        'install.begin': {'package', 'manifest', 'consent', 'size'},
        'install.chunk': {'package', 'transferId', 'offset', 'data'},
        'install.commit': {'package', 'transferId'}, 'install.abort': {'package', 'transferId'},
        'remove': {'package'}, 'activate': {'package'},
        'deactivate': {'package', 'instanceId', 'contextId'},
        'invoke': {'package', 'instanceId', 'contextId', 'name', 'payload'},
        'subscribe': {'package', 'instanceId', 'contextId', 'name'},
        'unsubscribe': {'package', 'instanceId', 'contextId', 'name', 'subscriptionId'},
        'cancel': {'targetOperationId'} if kind == 'dispatch' else {'targetRequestId'},
        'surface.open': {'package', 'instanceId', 'contextId', 'surfaceId'},
        'surface.close': {'package', 'instanceId', 'contextId', 'surfaceId'},
    }[action]
    optional = {'expectedPackage'} if action == 'install.begin' else {'cursor'} if action == 'inventory' else set()
    if revision2 and kind == 'dispatch' and action == 'surface.close':
        fields |= {'targetOperationId'}
    require(base | fields <= set(message) and set(message) <= base | fields | optional, 'Operation fields')
    if action == 'invoke':
        require(len(canonical(message['payload'])) <= MAX_INVOKE_PAYLOAD, 'Invoke payload byte limit')
    if kind == 'dispatch':
        require(message['binding']['deviceId'] != '' and 'requestId' not in message, 'Server-owned dispatch only')
    if action == 'install.begin':
        validate_manifest(message['manifest'])
        require(message['package'] == identity(message['manifest']), 'Manifest identity mismatch')
        require(message['consent']['permissions'] == message['manifest']['permissions'], 'Consent permission mismatch')
        if 'expectedPackage' in message:
            previous, target = message['expectedPackage'], message['package']
            require(previous['id'] == target['id'] and
                    tuple(map(int, target['version'].split('.'))) > tuple(map(int, previous['version'].split('.'))),
                    'Update must be same-ID newer numeric version')
    if action == 'install.chunk':
        decode_chunk(message['data'])


def encode_transport(message):
    """Canonical Socket.IO JSON payload bytes; NO 4-byte process framing."""
    validate_transport(message)
    return canonical(message)


def decode_transport(raw):
    message = strict_json(raw, MAX_TRANSPORT)
    validate_transport(message)
    return message


def validate_bound_transport(message, manifest, instance_id=None, context_id=None):
    """Additional installed-manifest checks for adapters after authority lookup.

    Caller still owns authentication, current epoch/owner/grant and correlation.
    This helper never derives authority from the supplied manifest or envelope.
    """
    validate_transport(message)
    validate_manifest(manifest)
    require(message.get('package') == identity(manifest), 'Installed package mismatch')
    if message['kind'] == 'event':
        frame = message['message']
        require(frame['instanceId'] == instance_id and frame['contextId'] == context_id,
                'Stale observation instance/context')
        body = frame['body']
        if body['kind'] == 'event':
            require(body['name'] in manifest['events'], 'Undeclared observed event')
            _payload(manifest['events'][body['name']], body['payload'])
        elif body['kind'] == 'state':
            require(any(s['id'] == body['surfaceId'] and s['kind'] == 'declarative'
                        for s in manifest['surfaces']), 'Unknown observed surface')
            _payload(manifest['state'], body['values'])
        return
    action = message.get('action')
    if 'instanceId' in message:
        require(message['instanceId'] == instance_id and message['contextId'] == context_id,
                'Stale request instance/context')
    if action == 'invoke':
        # Results intentionally omit name; exact pending action is an adapter
        # correlation requirement, never inferred from a client result.
        if message['kind'] != 'result':
            require(message['name'] in manifest['actions'], 'Undeclared requested action')
            _payload(manifest['actions'][message['name']]['input'], message['payload'])
    elif action in ('subscribe', 'unsubscribe'):
        if message['kind'] != 'result':
            require(message['name'] in manifest['events'], 'Undeclared requested event')
    elif action in ('surface.open', 'surface.close'):
        if message['kind'] != 'result':
            require(any(s['id'] == message['surfaceId'] and s['kind'] == 'declarative'
                        for s in manifest['surfaces']), 'Unknown requested surface')


def decode_chunk(data):
    """Strict canonical base64 of 1..32768 artifact bytes; never path or URL."""
    require(type(data) is str and len(data) <= 43692, 'Chunk text bound')
    try:
        raw = base64.b64decode(data, validate=True)
    except (ValueError, TypeError) as error:
        raise ValueError('Invalid artifact base64') from error
    require(0 < len(raw) <= MAX_CHUNK and base64.b64encode(raw).decode('ascii') == data, 'Noncanonical/oversize chunk')
    return raw


class ArtifactTransfer:
    """Bounded no-IO transfer reference; caller MUST authorize before every method.

    Binding token below is test correlation, not a replacement auth protocol.
    Host owns transfer IDs, staging paths, current-authority validation and commit.
    """

    def __init__(self, manifest, package, size, transfer_id, authority_context):
        validate_manifest(manifest)
        require(package == identity(manifest), 'Transfer package identity')
        require(type(size) is int and 1 <= size <= MAX_ARTIFACT, 'Transfer size')
        _check({'$ref': '#/$defs/token'}, transfer_id,
               json.loads((SCHEMAS / 'transport.schema.json').read_text(encoding='utf-8')))
        self.manifest, self.package = copy.deepcopy(manifest), copy.deepcopy(package)
        self.size, self.transfer_id, self.authority_context = size, transfer_id, authority_context
        self.content = bytearray()
        self.closed = False

    def append(self, transfer_id, package, authority_context, offset, data):
        require(not self.closed and transfer_id == self.transfer_id and package == self.package and
                authority_context == self.authority_context, 'Stale/foreign transfer')
        require(type(offset) is int and offset == len(self.content), 'Chunk offset gap/replay')
        raw = decode_chunk(data)
        require(len(self.content) + len(raw) <= self.size, 'Artifact size overrun')
        self.content.extend(raw)
        return len(self.content)

    def finish(self, transfer_id, package, authority_context):
        require(not self.closed and transfer_id == self.transfer_id and package == self.package and
                authority_context == self.authority_context, 'Stale/foreign commit')
        require(len(self.content) == self.size, 'Incomplete artifact')
        artifact = self.manifest['execution']['artifact']
        content = bytes(self.content)
        try:
            verify_artifact(self.manifest, artifact['target'], artifact['entry'], content)
        finally:
            self.abort()
        return content

    def abort(self):
        self.closed = True
        self.content.clear()


class OwnerContext:
    """Revision-2 in-memory receiving-side model, NOT a server authority source.

    Call only on the authenticated canonical server control channel. Loss of
    that channel calls disconnect(); no viewer/session credential belongs here.
    """

    def __init__(self, origin, device_id, epoch):
        self.origin, self.device_id, self.epoch = origin, device_id, epoch
        self.revision = 0
        self.binding = None
        self.valid = False
        self.closed = False
        self.instances = set()
        self.stages = set()

    def control(self, message):
        validate_transport(message, expected_version=2)
        require(not self.closed and message['kind'] == 'owner-context' and
                message['epoch'] == self.epoch and
                message['binding']['serverOrigin'] == self.origin and
                message['binding']['deviceId'] == self.device_id, 'Foreign/stale owner control')
        require(message['authorityRevision'] > self.revision, 'Owner authority replay')
        binding = message['binding']
        if message['state'] == 'revoked':
            require(self.binding == binding, 'Revocation for different binding')
        changed = self.binding is not None and self.binding != binding
        stopped = changed or message['state'] == 'revoked'
        if stopped:
            self.instances.clear()
            self.stages.clear()
        self.binding = copy.deepcopy(binding)
        self.revision = message['authorityRevision']
        self.valid = message['state'] == 'valid'
        return stopped

    def dispatch(self, message):
        validate_transport(message, expected_version=2)
        require(not self.closed and self.valid and message['kind'] == 'dispatch' and
                message.get('version') == 2 and message['epoch'] == self.epoch and
                message['binding'] == self.binding and
                message['authorityRevision'] == self.revision, 'Dispatch owner authority mismatch')

    def disconnect(self):
        self.closed, self.valid = True, False
        self.instances.clear()
        self.stages.clear()


class SurfaceWatches:
    """Revision-2 desktop fanout reference for ONE authorized active instance.

    Broker owns operation IDs and requester-to-watch lookup. Caller validates
    instance/grant/current owner before every method. No viewer auth is provided.
    IPC retains one logical surface; wire revisions are independently per watch.
    """

    def __init__(self, manifest):
        validate_manifest(manifest)
        self.manifest = copy.deepcopy(manifest)
        self.watches = {}
        self.surfaces = {}
        self.pending_close = {}

    def opened(self, operation_id, surface_id):
        """Call after first IPC-open ack; later opens use the cached snapshot.

        Returns (first_logical_open, initial_watch_snapshot_or_None). A later
        viewer before the first snapshot gets busy without creating a watch.
        """
        token = json.loads((SCHEMAS / 'transport.schema.json').read_text(encoding='utf-8'))
        _check({'$ref': '#/$defs/token'}, operation_id, token)
        require(operation_id not in self.watches and len(self.watches) < 64, 'Duplicate/watch capacity')
        require(any(s['id'] == surface_id and s['kind'] == 'declarative'
                    for s in self.manifest['surfaces']), 'Unknown watch surface')
        require(surface_id not in self.pending_close, 'Surface closing')
        first = surface_id not in self.surfaces
        if not first:
            require(self.surfaces[surface_id]['values'] is not None, 'Surface initial snapshot pending')
        else:
            self.surfaces[surface_id] = {'revision': 0, 'values': None}
        self.watches[operation_id] = {'surfaceId': surface_id, 'revision': 0}
        cached = self.surfaces[surface_id]['values']
        return first, None if cached is None else self._watch_snapshot(operation_id, cached)

    def _watch_snapshot(self, operation_id, values):
        watch = self.watches[operation_id]
        watch['revision'] += 1
        return {'kind': 'state', 'surfaceId': watch['surfaceId'],
                'revision': watch['revision'], 'values': copy.deepcopy(values)}

    def snapshot(self, body):
        require(set(body) == {'kind', 'surfaceId', 'revision', 'values'} and body['kind'] == 'state' and
                type(body['revision']) is int and body['surfaceId'] in self.surfaces, 'Unknown surface snapshot')
        surface = self.surfaces[body['surfaceId']]
        require(body['revision'] == surface['revision'] + 1, 'IPC snapshot gap/replay')
        _payload(self.manifest['state'], body['values'])
        # Validate all data before advancing shared or per-watch state.
        require(len(canonical(body)) <= MAX_FRAME, 'Snapshot byte limit')
        surface['revision'], surface['values'] = body['revision'], copy.deepcopy(body['values'])
        return {operation: self._watch_snapshot(operation, body['values'])
                for operation, watch in self.watches.items() if watch['surfaceId'] == body['surfaceId']}

    def close(self, target_operation_id, surface_id):
        """Remove exactly the broker-selected watch. True means send IPC close.

        Last close reserves the logical surface until closed() confirms peer ack;
        no new open can race an in-flight IPC close. Other watches are unaffected.
        """
        require(target_operation_id in self.watches and
                self.watches[target_operation_id]['surfaceId'] == surface_id, 'Foreign/unknown target watch')
        del self.watches[target_operation_id]
        last = not any(w['surfaceId'] == surface_id for w in self.watches.values())
        if last:
            self.pending_close[surface_id] = target_operation_id
        return last

    def closed(self, surface_id):
        require(surface_id in self.pending_close, 'No logical close pending')
        del self.pending_close[surface_id]
        del self.surfaces[surface_id]

    def invalidate(self):
        self.watches.clear()
        self.surfaces.clear()
        self.pending_close.clear()
