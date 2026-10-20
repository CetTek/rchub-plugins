import copy
import hashlib
import json
from pathlib import Path
import unittest

import rchub_plugin as sdk


class PackageContractTests(unittest.TestCase):
    def setUp(self):
        self.manifest = sdk.template('org.example.audio', 'Example')

    def test_canonical_order_unicode_and_digest_are_deterministic(self):
        self.assertEqual(sdk.canonical({'z': 'Ses düzeyi', 'a': {'y': 2, 'b': 1}}),
                         '{"a":{"b":1,"y":2},"z":"Ses düzeyi"}\n'.encode())
        self.assertNotEqual(hashlib.sha256(sdk.canonical(self.manifest)).digest(),
                            hashlib.sha256(sdk.canonical(self.manifest | {'version': '0.1.1'})).digest())

    def test_official_artifact_reproduces_actual_guest_and_envelope(self):
        root = sdk.ROOT / 'plugins/official/audio'
        raw = (root / 'audio.rchub-plugin.json').read_bytes()
        package = json.loads(raw)
        sdk.validate_bundle(package)
        self.assertEqual(raw, sdk.canonical(package))
        self.assertEqual(bytes(package['wasm']), sdk.proxy_wasm(package['manifest']))
        self.assertEqual((root / 'guest.wat').read_text(encoding='utf-8'), sdk.proxy_wat(package['manifest']))
        self.assertEqual(json.loads((root / 'manifest.json').read_text(encoding='utf-8')), package['manifest'])

    def test_community_identity_needs_no_catalog_entry(self):
        sdk.validate_manifest(self.manifest)
        self.assertEqual(self.manifest['publisher']['kind'], 'community')

    def test_unknown_fields_and_capabilities_are_rejected(self):
        for change in [{'extra': 1}, {'permissions': ['shell.execute']}, {'targets': ['any']},
                       {'source': 'file:///secret'}, {'hostApi': 2}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                sdk.validate_manifest(self.manifest | change)

    def test_bad_action_grants_and_argument_schemas_are_rejected(self):
        for mutate in [lambda m: m['actions'][0].update(capability='power.write'),
                       lambda m: m['actions'][1]['input']['properties']['volume'].update(maximum=-1),
                       lambda m: m['actions'].append(copy.deepcopy(m['actions'][0])),
                       lambda m: m['permissions'].reverse()]:
            m = copy.deepcopy(self.manifest)
            mutate(m)
            with self.assertRaises(ValueError):
                sdk.validate_manifest(m)

    def test_controls_cannot_invent_actions_or_expand_numeric_input(self):
        for mutate in [lambda c: c.update(action='not_declared'), lambda c: c.update(max=101),
                       lambda c: c.update(argument='not_declared'), lambda c: c.update(step=0),
                       lambda c: c.update(type='javascript')]:
            m = copy.deepcopy(self.manifest)
            mutate(m['ui']['controls'][0]['children'][1])
            with self.assertRaises(ValueError):
                sdk.validate_manifest(m)

    def test_ui_depth_and_duplicate_ids_are_bounded(self):
        for nested in [False, True]:
            m = copy.deepcopy(self.manifest)
            if nested:
                node = {'id': 'leaf', 'type': 'text', 'label': {'en': 'A', 'tr': 'A'}}
                for i in range(7):
                    node = {'id': f'node{i}', 'type': 'section', 'children': [node]}
                m['ui']['controls'] = [node]
            else:
                m['ui']['controls'].append(copy.deepcopy(m['ui']['controls'][0]))
            with self.assertRaises(ValueError):
                sdk.validate_manifest(m)

    def test_typed_list_schema_has_real_bounded_object_items(self):
        array = {'type': 'array', 'maxItems': 64, 'items': {'type': 'object',
            'properties': {'name': {'type': 'string', 'maxLength': 120}}, 'required': ['name']}}
        sdk.field_schema(array)
        m = copy.deepcopy(self.manifest)
        m['state']['apps'] = array
        m['ui']['controls'].append({'id': 'apps', 'type': 'list', 'bind': 'apps', 'itemLabel': 'name'})
        sdk.validate_manifest(m)
        m['ui']['controls'][-1]['itemLabel'] = 'missing'
        with self.assertRaises(ValueError):
            sdk.validate_manifest(m)
        with self.assertRaises(ValueError):
            sdk.field_schema(array | {'maxItems': 65})

    def test_python_bool_is_not_a_wire_version_or_numeric_bound(self):
        for key in ('schemaVersion', 'hostApi'):
            with self.subTest(key=key), self.assertRaises(ValueError):
                sdk.validate_manifest(self.manifest | {key: True})
        m = copy.deepcopy(self.manifest)
        m['ui']['version'] = True
        with self.assertRaises(ValueError):
            sdk.validate_manifest(m)
        b = {'formatVersion': True, 'manifest': self.manifest, 'wasm': list(sdk.proxy_wasm(self.manifest))}
        with self.assertRaises(ValueError):
            sdk.validate_bundle(b)
        for bound in (True, 9007199254740992, -9007199254740992, 1.5):
            with self.assertRaises(ValueError):
                sdk.field_schema({'type': 'number', 'minimum': bound, 'maximum': bound})

    def test_identifiers_and_utf16_lengths_match_javascript_and_rust(self):
        for key in ('constructor', 'prototype', '__proto__'):
            self.assertFalse(sdk.identifier(key))
            with self.assertRaises(ValueError):
                sdk.object_schema({'type': 'object', 'properties': {key: {'type': 'boolean'}}, 'required': []})
        sdk.localized({'en': '😀' * 40, 'tr': 'A'}, 80)
        with self.assertRaises(ValueError):
            sdk.localized({'en': '😀' * 41, 'tr': 'A'}, 80)
        with self.assertRaises(ValueError):
            sdk.field_schema({'type': 'string', 'maxLength': 1, 'enum': ['😀']})
        self.assertTrue(sdk.accepts({'type': 'string', 'maxLength': 2}, '😀'))

    def test_output_must_be_declared_and_match_state(self):
        for output in ({'secret': {'type': 'boolean'}}, {'volume': {'type': 'integer', 'minimum': 0, 'maximum': 101}}):
            m = json.loads(json.dumps(self.manifest))
            m['actions'][0]['output']['properties'] = output
            m['actions'][0]['output']['required'] = list(output)
            with self.assertRaises(ValueError):
                sdk.validate_manifest(m)

    def test_controls_reject_cross_type_fields_and_incomplete_actions(self):
        bad = [
            {'id': 'a', 'type': 'button', 'action': 'set_volume', 'argument': 'volume'},
            {'id': 'a', 'type': 'input', 'action': 'set_mute', 'argument': 'muted'},
            {'id': 'a', 'type': 'select', 'action': 'set_volume', 'argument': 'volume', 'options': []},
            {'id': 'a', 'type': 'select', 'action': 'set_volume', 'argument': 'volume', 'options': [{'label': {'en': 'A', 'tr': 'A'}, 'value': 101}]},
            {'id': 'a', 'type': 'value'},
            {'id': 'a', 'type': 'image', 'bind': 'volume'},
            {'id': 'a', 'type': 'progress', 'bind': 'volume'},
            {'id': 'a', 'type': 'progress', 'bind': 'volume', 'min': 0, 'max': 101},
            {'id': 'a', 'type': 'section', 'action': 'read', 'children': []},
            {'id': 'a', 'type': 'button', 'action': 'read', 'children': []},
            {'id': 'a', 'type': 'text', 'placeholder': {'en': 'A', 'tr': 'A'}},
            {'id': 'a', 'type': 'text', 'min': 0},
            {'id': 'a', 'type': 'text', 'options': []},
            {'id': 'a', 'type': 'text', 'itemLabel': 'name'},
        ]
        for control in bad:
            m = copy.deepcopy(self.manifest)
            m['ui']['controls'] = [control]
            with self.subTest(control=control), self.assertRaises(ValueError):
                sdk.validate_manifest(m)
        m = copy.deepcopy(self.manifest)
        m['ui']['controls'] = [{'id': 'a', 'type': 'progress', 'bind': 'volume', 'min': 0, 'max': 100}]
        sdk.validate_manifest(m)

    def test_nested_schema_depth_is_three_and_all_current_official_packages_validate(self):
        nested = {'type': 'array', 'maxItems': 2, 'items': {'type': 'object', 'properties': {'name': {'type': 'string', 'maxLength': 80}}, 'required': ['name']}}
        sdk.field_schema(nested)
        with self.assertRaises(ValueError):
            sdk.field_schema({'type': 'array', 'maxItems': 2, 'items': nested})
        packages = list((sdk.ROOT / 'plugins/official').glob('*/*.rchub-plugin.json'))
        self.assertGreaterEqual(len(packages), 8)
        for path in packages:
            with self.subTest(package=path.name):
                sdk.validate_bundle(json.loads(path.read_text(encoding='utf-8')))

    def test_json_schema_numeric_control_requirements_and_reserved_ids(self):
        schema = json.loads((sdk.ROOT / 'contracts/plugins/plugin.schema.json').read_text(encoding='utf-8'))
        control = schema['$defs']['control']
        conditions = {tuple(x.get('if', {}).get('properties', {}).get('type', {}).get('enum', [])): x['then'] for x in control['allOf']}
        self.assertEqual(set(conditions[('slider',)]['required']), {'bind', 'action', 'argument', 'min', 'max', 'step'})
        self.assertEqual(set(conditions[('progress',)]['required']), {'bind', 'min', 'max'})
        self.assertIn('constructor', schema['$defs']['id']['not']['enum'])

    def test_byte_size_and_module_header_bounds_are_enforced(self):
        b = {'formatVersion': 1, 'manifest': self.manifest, 'wasm': list(sdk.proxy_wasm(self.manifest))}
        sdk.validate_bundle(b)
        for wasm in [[-1] * 8, [256] * 8, [0] * 8, [0] * 16385]:
            with self.assertRaises(ValueError):
                sdk.validate_bundle(b | {'wasm': wasm})


if __name__ == '__main__':
    unittest.main()
