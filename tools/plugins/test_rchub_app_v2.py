"""Pure simulated endpoints. Never launches a native artifact or contacts a host."""
import copy
import base64
import hashlib
import json
import struct
import os
import subprocess
import sys
import unittest
from pathlib import Path

import rchub_app_v2 as sdk
import rchub_plugin as v1


class ApplicationV2Tests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads((Path(__file__).parent / 'fixtures/v2/vendor.fixture.json').read_text(encoding='utf-8'))
        self.session = sdk.MockSession(self.manifest, 'instance1', 'context1')

    def frame(self, body, sender='host', **changes):
        return {'protocol': sdk.PROTOCOL, 'instanceId': 'instance1', 'contextId': 'context1',
                'sender': sender, 'body': body} | changes

    def request(self, request_id='req1', operation='invoke', name='vendor.fixture.apply', payload=None, **extra):
        if payload is None:
            payload = {'enabled': True} if operation == 'invoke' else {}
        return {'kind': 'request', 'requestId': request_id, 'operation': operation,
                'name': name, 'payload': payload} | extra

    def result(self, request_id='req1', operation='invoke', name='vendor.fixture.apply', status='ok', payload=None):
        if payload is None:
            payload = {'accepted': True} if operation == 'invoke' and status == 'ok' else {}
        return {'kind': 'result', 'requestId': request_id, 'operation': operation,
                'name': name, 'status': status, 'payload': payload}

    def start(self):
        self.session.accept(self.frame({'kind': 'lifecycle', 'state': 'started'}, 'plugin'))

    def test_manifest_fixture_identity_and_namespace_are_generic(self):
        sdk.validate_manifest(self.manifest)
        self.assertEqual(sdk.identity(self.manifest), sdk.identity(copy.deepcopy(self.manifest)))
        self.assertNotEqual(sdk.identity(self.manifest)['sha256'], sdk.identity(self.manifest | {'version': '0.0.2'})['sha256'])
        self.assertNotIn('capability', self.manifest['actions']['vendor.fixture.apply'])
        self.assertEqual(self.manifest['surfaces'][0]['clients'], ['pc', 'mobile', 'web'])

    def test_extended_payload_features_and_jpeg_bounds(self):
        manifest = copy.deepcopy(self.manifest)
        manifest['state']['properties']['image'] = {'type': 'image', 'maxLength': 800000}
        manifest['state']['properties']['clipboard'] = {'type': 'string', 'maxLength': 8192}
        with self.assertRaises(ValueError): sdk.validate_manifest(manifest)
        manifest['requiredFeatures'] += ['image-v1', 'string-8192-v1']
        sdk.validate_manifest(manifest)
        with self.assertRaises(ValueError): sdk.validate_manifest(manifest, supported=sdk.FEATURES - {'image-v1'})
        image = 'data:image/jpeg;base64,' + base64.b64encode(b'\xff\xd8hello\xff\xd9').decode()
        sdk._payload(manifest['state']['properties']['image'], image)
        sdk._payload(manifest['state']['properties']['clipboard'], 'x' * 8192)
        for value in ('data:image/png;base64,/9j/2Q==', 'data:image/jpeg;base64,AAAA', image + '!', image * 40000):
            with self.assertRaises(ValueError): sdk._payload(manifest['state']['properties']['image'], value)
        with self.assertRaises(ValueError): sdk._payload(manifest['state']['properties']['clipboard'], 'x' * 8193)

    def test_manifest_version_features_and_closed_keys_fail_closed(self):
        for change in [{'schemaVersion': 1}, {'schemaVersion': True}, {'schemaVersion': 3},
                       {'unknown': True}, {'requiredFeatures': ['shell-v1']},
                       {'requiredFeatures': []}, {'requiredFeatures': ['native-process-v1']},
                       {'permissions': ['arbitraryFS']}, {'exec': 'shell'}, {'url': 'https://example.invalid'}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                sdk.validate_manifest(self.manifest | change)
        with self.assertRaises(ValueError):
            sdk.validate_manifest(self.manifest, supported={'native-process-v1'})

    def test_artifact_metadata_and_bytes_fail_closed_without_opening_paths(self):
        content = b'not executable: deterministic fixture bytes'
        self.manifest['execution']['artifact']['sha256'] = hashlib.sha256(content).hexdigest()
        sdk.verify_artifact(self.manifest, 'windows-x86_64', 'bin/fixture.exe', content)
        for target, entry, raw in [('linux-x86_64', 'bin/fixture.exe', content),
                                   ('windows-x86_64', 'fixture.exe', content),
                                   ('windows-x86_64', 'bin/fixture.exe', b'altered'),
                                   ('windows-x86_64', 'bin/fixture.exe', b''),
                                   ('windows-x86_64', 'bin/fixture.exe', b'x' * (sdk.MAX_ARTIFACT + 1))]:
            with self.subTest(target=target, entry=entry, size=len(raw)), self.assertRaises(ValueError):
                sdk.verify_artifact(self.manifest, target, entry, raw)
        for key, value in [('target', 'any'), ('sha256', 'A' * 64), ('sha256', '0' * 63),
                           ('entry', '../x'), ('entry', '/x'), ('entry', 'C:/x'), ('entry', 'bin\\x'),
                           ('entry', 'bin/../x'), ('entry', 'bin//x'), ('entry', 'https://x'),
                           ('entry', 'bin/x:stream'), ('entry', 'con.exe'), ('entry', 'bin/NUL'),
                           ('entry', 'x.'), ('entry', 'x' * 161)]:
            bad = copy.deepcopy(self.manifest)
            bad['execution']['artifact'][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                sdk.validate_manifest(bad)

    def test_declarations_and_surface_references_are_bounded(self):
        mutations = [
            lambda m: m['actions'].update({'other.fixture.run': m['actions']['vendor.fixture.open']}),
            lambda m: m['actions']['vendor.fixture.apply'].update(capability='audio.write'),
            lambda m: m['actions']['vendor.fixture.apply']['input'].update(required=['missing']),
            lambda m: m['events']['vendor.fixture.changed']['properties']['revision'].update(minimum=101),
            lambda m: m['surfaces'][0].update(actions=['vendor.fixture.missing']),
            lambda m: m['surfaces'][0].update(events=['vendor.fixture.missing']),
            lambda m: m['surfaces'][0].update(clients=['browser']),
            lambda m: m['surfaces'][0].update(html='<script/>'),
            lambda m: m['surfaces'][1].update(trust='host-approved'),
            lambda m: m['execution'].update(trust='sandboxed'),
            lambda m: m['surfaces'].append(copy.deepcopy(m['surfaces'][0])),
            lambda m: m['actions']['vendor.fixture.apply']['input']['properties'].update(constructor={'type': 'boolean'}),
        ]
        for mutate in mutations:
            bad = copy.deepcopy(self.manifest)
            mutate(bad)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                sdk.validate_manifest(bad)
        schema = self.manifest['actions']['vendor.fixture.apply']['input']
        for _ in range(4):
            schema['properties']['nested'] = {'type': 'object', 'properties': {}, 'required': []}
            schema = schema['properties']['nested']
        with self.assertRaises(ValueError):
            sdk.validate_manifest(self.manifest)

    def test_storage_is_choice_disclosure_not_filesystem_permission(self):
        for choices in [['none'], ['native-filesystem'], ['native-filesystem', 'host-scoped'], ['host-scoped']]:
            m = copy.deepcopy(self.manifest)
            m['storage']['choices'] = choices
            m['requiredFeatures'].append('scoped-store-v1')
            sdk.validate_manifest(m)
        for storage in [{'choices': ['arbitraryFS']}, {'choices': ['host-scoped']},
                        {'choices': ['none', 'native-filesystem']}, {'choices': ['native-filesystem'], 'path': '/tmp/x'},
                        {'choices': ['native-filesystem'], 'retentionDays': 30}]:
            with self.subTest(storage=storage), self.assertRaises(ValueError):
                sdk.validate_manifest(self.manifest | {'storage': storage})

    def test_nested_payload_and_unicode_scalar_bounds(self):
        record = {'type': 'object', 'properties': {
            'nested': {'type': 'object', 'properties': {
                'label': {'type': 'string', 'maxLength': 2}}, 'required': ['label']}},
            'required': ['nested']}
        self.manifest['actions']['vendor.fixture.apply']['input'] = record
        self.manifest['surfaces'][0]['controls'] = []
        session = sdk.MockSession(self.manifest, 'instance1', 'context1')
        session.accept(self.frame({'kind': 'lifecycle', 'state': 'started'}, 'plugin'))
        for value in [{'nested': {'label': '😀ş'}}, {'nested': {'label': ''}}]:
            request = self.request(str(len(session.used)), payload=value)
            session.accept(self.frame(request))
        for value in [{'nested': {'label': '😀şx'}}, {'nested': {'label': None}},
                      {'nested': {'label': 'x', 'extra': False}}, {'nested': {}}, {'nested': []}]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                session.accept(self.frame(self.request('bad', payload=value)))
        self.assertEqual(len(session.pending), 2)

    def test_schema_driven_validation_has_only_supported_keywords(self):
        allowed = {'$schema', '$comment', '$defs', '$ref', 'type', 'additionalProperties', 'required',
                   'properties', 'const', 'enum', 'oneOf', 'minProperties', 'maxProperties',
                   'minItems', 'maxItems', 'uniqueItems', 'items', 'minLength', 'maxLength',
                   'minimum', 'maximum', 'pattern', 'propertyNames'}

        def inspect(schema):
            self.assertLessEqual(schema.keys(), allowed)
            for key in ('properties', '$defs'):
                for child in schema.get(key, {}).values():
                    inspect(child)
            for key in ('items', 'additionalProperties', 'propertyNames'):
                if isinstance(schema.get(key), dict):
                    inspect(schema[key])
            for child in schema.get('oneOf', []):
                inspect(child)

        for path in sdk.SCHEMAS.glob('*.schema.json'):
            inspect(json.loads(path.read_text(encoding='utf-8')))

    def test_fragmented_coalesced_and_exact_limit_frames(self):
        message = self.frame(self.request())
        raw = sdk.encode_frame(message)
        for split in range(len(raw) + 1):
            decoder = sdk.FrameDecoder()
            self.assertEqual(decoder.feed(raw[:split]) + decoder.feed(raw[split:]), [message])
            decoder.finish()
        decoder = sdk.FrameDecoder()
        self.assertEqual(decoder.feed(raw * 3), [message] * 3)
        decoder.finish()
        large = self.frame(self.request(payload={'value': ''}))
        padding = sdk.MAX_FRAME - len(sdk.canonical(large))
        large['body']['payload']['value'] = 'x' * padding
        self.assertEqual(len(sdk.encode_frame(large)), sdk.MAX_FRAME + 4)
        self.assertEqual(sdk.FrameDecoder().feed(sdk.encode_frame(large)), [large])
        large['body']['payload']['value'] += 'x'
        with self.assertRaises(ValueError):
            sdk.encode_frame(large)

    def test_transport_envelope_and_invoke_boundaries(self):
        package = sdk.identity(self.manifest)
        event = {'v': 3, 'version': 2, 'kind': 'event', 'operationId': 'o' * 64,
                 'epoch': 'e' * 64, 'package': package,
                 'message': self.frame({'kind': 'event', 'subscriptionId': 's' * 64,
                                        'name': 'vendor.fixture.changed', 'payload': {'value': ''}}, 'plugin')}
        payload = event['message']['body']['payload']
        payload['value'] = 'x' * (sdk.MAX_FRAME - len(sdk.canonical(event['message'])))
        self.assertEqual(len(sdk.canonical(event['message'])), sdk.MAX_FRAME)
        for route in ({'operationId': 'o' * 64, 'epoch': 'e' * 64},
                      {'requestId': 'r' * 64, 'deviceId': 'd' * 64}):
            routed = {k: v for k, v in event.items() if k not in ('operationId', 'epoch')} | route
            self.assertEqual(sdk.decode_transport(sdk.encode_transport(routed)), routed)
        payload['value'] += 'x'
        with self.assertRaises(ValueError): sdk.validate_transport(event)
        request = {'v': 3, 'version': 2, 'kind': 'request', 'requestId': 'req1',
                   'deviceId': 'device1', 'action': 'invoke', 'package': package,
                   'instanceId': 'i' * 64, 'contextId': 'c' * 64,
                   'name': 'vendor.fixture.apply', 'payload': {'value': ''}}
        request['payload']['value'] = 'x' * (sdk.MAX_INVOKE_PAYLOAD - len(sdk.canonical(request['payload'])))
        sdk.validate_transport(request)
        frame = self.frame(self.request(payload=request['payload']), instanceId='i' * 64, contextId='c' * 64)
        self.assertLessEqual(len(sdk.canonical(frame)), sdk.MAX_FRAME)
        request['payload']['value'] += 'x'
        with self.assertRaises(ValueError): sdk.validate_transport(request)

    def test_strict_json_negative_corpus_and_poisoned_stream(self):
        raw_cases = [b'{} trailing', b'{"a":1,"a":2}', b'{"a":NaN}', b'{"a":Infinity}',
                     b'{"a":1.0}', b'{"a":9007199254740992}', b'{"a":"\\ud800"}',
                     b'{"a":"\xff"}', b'\xef\xbb\xbf{}', b'[]', b'null', b'{"constructor":{}}',
                     b'[' * 1000 + b']' * 1000]
        for raw in raw_cases:
            decoder = sdk.FrameDecoder()
            with self.subTest(raw=raw[:40]), self.assertRaises(ValueError):
                decoder.feed(struct.pack('>I', len(raw)) + raw)
            with self.assertRaises(ValueError):
                decoder.feed(sdk.encode_frame(self.frame(self.request())))
        for value in [float('nan'), 1.5, 9007199254740992, '\ud800', {1: 'bad'}]:
            with self.subTest(value=repr(value)), self.assertRaises(ValueError):
                sdk.encode_frame(self.frame(self.request(payload={'value': value})))

    def test_truncated_oversize_zero_and_version_mismatch(self):
        raw = sdk.encode_frame(self.frame(self.request()))
        for truncated in [raw[:1], raw[:3], raw[:4], raw[:-1]]:
            decoder = sdk.FrameDecoder()
            decoder.feed(truncated)
            with self.assertRaises(ValueError):
                decoder.finish()
        for length in [0, sdk.MAX_FRAME + 1, 0xFFFFFFFF]:
            decoder = sdk.FrameDecoder()
            with self.assertRaises(ValueError):
                decoder.feed(struct.pack('>I', length))
            self.assertEqual(len(decoder.buffer), 0)
        for change in [{'protocol': 'rchub-json-v1'}, {'protocol': 2}, {'extra': True}, {'instanceId': ''}]:
            message = self.frame(self.request(), **change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                sdk.encode_frame(message)
            raw = sdk.canonical(message)
            with self.assertRaises(ValueError):
                sdk.FrameDecoder().feed(struct.pack('>I', len(raw)) + raw)

    def test_bidirectional_request_correlation_no_replay(self):
        self.start()
        for sender, peer in [('host', 'plugin'), ('plugin', 'host')]:
            self.session.accept(self.frame(self.request(), sender))
            for bad in [self.result(request_id='missing'), self.result(name='vendor.fixture.open'),
                        self.result(operation='subscribe'), self.result(payload={'accepted': 1})]:
                with self.subTest(sender=sender, bad=bad), self.assertRaises(ValueError):
                    self.session.accept(self.frame(bad, peer))
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(self.result(), sender))
            self.session.accept(self.frame(self.result(), peer))
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(self.result(), peer))
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(self.request(), sender))
        self.assertEqual(self.session.pending, {})

    def test_unknown_and_bad_payload_requests_leave_state_unchanged(self):
        self.start()
        for request in [self.request(name='vendor.fixture.missing'), self.request(payload={}),
                        self.request(payload={'enabled': True, 'command': 'shell'}),
                        self.request(payload={'enabled': 'true'}), self.request(subscriptionId='extra'),
                        self.request(operation='unsubscribe', name='vendor.fixture.changed')]:
            with self.subTest(request=request), self.assertRaises(ValueError):
                self.session.accept(self.frame(request))
            self.assertFalse(self.session.pending)
            self.assertFalse(self.session.used)
        self.session.accept(self.frame(self.request()))
        with self.assertRaises(ValueError):
            self.session.accept(self.frame(self.result(status='rejected', payload={'accepted': False}), 'plugin'))
        self.session.accept(self.frame(self.result(status='rejected'), 'plugin'))
        self.assertFalse(self.session.pending)

    def test_subscription_events_unsubscribe_in_both_directions(self):
        self.start()
        name = 'vendor.fixture.changed'
        for sender, peer in [('host', 'plugin'), ('plugin', 'host')]:
            event = {'kind': 'event', 'name': name, 'subscriptionId': 'sub1', 'payload': {'revision': 1}}
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(event, peer))
            self.session.accept(self.frame(self.request('sub1', 'subscribe', name), sender))
            self.session.accept(self.frame(self.result('sub1', 'subscribe', name), peer))
            self.session.accept(self.frame(event, peer))
            for bad in [event | {'subscriptionId': 'unknown'}, event | {'name': 'vendor.fixture.missing'},
                        event | {'payload': {'revision': 101}}, event | {'payload': {'revision': True}}]:
                with self.assertRaises(ValueError):
                    self.session.accept(self.frame(bad, peer))
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(event, sender))
            self.session.accept(self.frame(self.request('unsub1', 'unsubscribe', name, subscriptionId='sub1'), sender))
            self.session.accept(self.frame(self.result('unsub1', 'unsubscribe', name), peer))
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(event, peer))
        self.assertFalse(self.session.subscriptions)

    def test_rejected_subscriptions_and_stale_events_do_not_advance_state(self):
        self.start()
        name = 'vendor.fixture.changed'
        self.session.accept(self.frame(self.request('denied', 'subscribe', name)))
        self.session.accept(self.frame(self.result('denied', 'subscribe', name, status='rejected'), 'plugin'))
        self.assertFalse(self.session.subscriptions)
        self.session.accept(self.frame(self.request('sub', 'subscribe', name)))
        self.session.accept(self.frame(self.result('sub', 'subscribe', name), 'plugin'))
        self.session.accept(self.frame(self.request('remove', 'unsubscribe', name, subscriptionId='sub')))
        self.session.accept(self.frame(self.result('remove', 'unsubscribe', name, status='rejected'), 'plugin'))
        event = self.frame({'kind': 'event', 'subscriptionId': 'sub', 'name': name, 'payload': {'revision': 0}}, 'plugin')
        self.session.accept(event)
        for change in [{'instanceId': 'stale'}, {'contextId': 'stale'}]:
            with self.assertRaises(ValueError):
                self.session.accept(event | change)
        self.session.accept(self.frame(self.request('pending')))
        self.session.invalidate()
        self.assertFalse(self.session.pending)
        self.assertFalse(self.session.subscriptions)
        with self.assertRaises(ValueError):
            self.session.accept(event)

    def test_lifecycle_stale_context_and_invalidation_clear_pending(self):
        with self.assertRaises(ValueError):
            self.session.accept(self.frame(self.request()))
        with self.assertRaises(ValueError):
            self.session.accept(self.frame({'kind': 'lifecycle', 'state': 'started'}))
        self.start()
        for changes in [{'instanceId': 'old'}, {'contextId': 'old'}]:
            with self.assertRaises(ValueError):
                self.session.accept(self.frame(self.request(), **changes))
        with self.assertRaises(ValueError):
            self.start()
        self.session.accept(self.frame(self.request()))
        self.session.accept(self.frame({'kind': 'lifecycle', 'state': 'stopped'}, 'plugin'))
        self.assertFalse(self.session.pending)
        with self.assertRaises(ValueError):
            self.start()
        with self.assertRaises(ValueError):
            self.session.accept(self.frame(self.result(), 'plugin'))
        other = sdk.MockSession(self.manifest, 'instance2', 'context2')
        other.invalidate()
        with self.assertRaises(ValueError):
            other.accept(self.frame({'kind': 'lifecycle', 'state': 'started'}, 'plugin', instanceId='instance2', contextId='context2'))

    def test_mock_capacity_is_bounded_without_claiming_durable_dedup(self):
        self.start()
        for i in range(64):
            self.session.accept(self.frame(self.request(f'req{i}')))
        with self.assertRaises(ValueError):
            self.session.accept(self.frame(self.request('overflow')))
        for i in range(64):
            self.session.accept(self.frame(self.result(f'req{i}'), 'plugin'))
        for i in range(64, sdk.MAX_IDS):
            self.session.accept(self.frame(self.request(f'req{i}')))
            self.session.accept(self.frame(self.result(f'req{i}'), 'plugin'))
        with self.assertRaises(ValueError):
            self.session.accept(self.frame(self.request('overflow')))

    def test_simulated_endpoint_roundtrip_including_events_and_lifecycle(self):
        messages = [self.frame({'kind': 'lifecycle', 'state': 'started'}, 'plugin'),
                    self.frame(self.request()), self.frame(self.result(), 'plugin'),
                    self.frame(self.request('sub', 'subscribe', 'vendor.fixture.changed')),
                    self.frame(self.result('sub', 'subscribe', 'vendor.fixture.changed'), 'plugin'),
                    self.frame({'kind': 'event', 'subscriptionId': 'sub', 'name': 'vendor.fixture.changed', 'payload': {'revision': 2}}, 'plugin'),
                    self.frame({'kind': 'lifecycle', 'state': 'stopped'}, 'plugin')]
        raw = b''.join(sdk.encode_frame(m) for m in messages)
        decoder = sdk.FrameDecoder()
        for index in range(0, len(raw), 7):
            for decoded in decoder.feed(raw[index:index + 7]):
                self.session.accept(decoded)
        decoder.finish()
        self.assertTrue(self.session.closed)
        self.assertFalse(self.session.pending)
        self.assertFalse(self.session.subscriptions)

    def test_legacy_packages_reject_native_contract(self):
        packages = sorted((sdk.ROOT / 'plugins/official').glob('*/*.rchub-plugin.json'))
        self.assertTrue(
            {path.parent.name for path in packages}
            >= {
                'apps', 'audio', 'browser', 'clipboard', 'display', 'effects',
                'keyboard', 'media', 'power', 'screenshot', 'speech', 'system',
            },
            'all twelve legacy packages must stay present',
        )
        for path in packages:
            raw = path.read_bytes()
            bundle = json.loads(raw)
            with self.subTest(package=path.name):
                v1.validate_bundle(bundle)
                self.assertEqual(raw, v1.canonical(bundle))
                with self.assertRaises(ValueError):
                    sdk.validate_manifest(bundle['manifest'])
        with self.assertRaises(ValueError):
            v1.validate_manifest(self.manifest)


class IntegratedContractV2Tests(unittest.TestCase):
    setUp = ApplicationV2Tests.setUp
    frame = ApplicationV2Tests.frame
    request = ApplicationV2Tests.request
    result = ApplicationV2Tests.result
    def negotiated(self, manifest=None):
        session = sdk.ProtocolSession(manifest or self.manifest, 'instance1', 'context1')
        session.accept(self.frame({'kind': 'hello', 'version': 1, 'features': list(sdk.FEATURES)}))
        session.accept(self.frame({'kind': 'welcome', 'version': 1,
                                  'features': (manifest or self.manifest)['requiredFeatures']}, 'plugin'))
        session.accept(self.frame({'kind': 'lifecycle', 'state': 'started'}, 'plugin'))
        return session

    def wire(self, action='inventory', **extra):
        return {'v': 3, 'kind': 'request', 'requestId': 'client1', 'deviceId': 'device1',
                'action': action} | extra

    def test_negotiation_fails_closed_direction_version_features_and_replay(self):
        for body, sender in [({'kind': 'lifecycle', 'state': 'started'}, 'plugin'),
                             ({'kind': 'hello', 'version': 2, 'features': []}, 'host'),
                             ({'kind': 'hello', 'version': 1, 'features': []}, 'host'),
                             ({'kind': 'hello', 'version': 1, 'features': list(sdk.FEATURES)}, 'plugin'),
                             ({'kind': 'welcome', 'version': 1, 'features': list(sdk.FEATURES)}, 'plugin')]:
            session = sdk.ProtocolSession(self.manifest, 'instance1', 'context1')
            with self.subTest(body=body), self.assertRaises(ValueError):
                session.accept(self.frame(body, sender))
            self.assertEqual(session.phase, 'new')
        session = self.negotiated()
        with self.assertRaises(ValueError):
            session.accept(self.frame({'kind': 'hello', 'version': 1, 'features': list(sdk.FEATURES)}))

    def test_long_lived_sequence_not_mock_256_capacity_and_cancel_races(self):
        session = self.negotiated()
        for i in range(1, 601):
            session.accept(self.frame(self.request(str(i))))
            session.accept(self.frame(self.result(str(i)), 'plugin'))
        self.assertEqual(session.highwater['host'], 600)
        self.assertEqual(session.used, set())
        for rid in ('600', '1', '0601', '0', 'text', '9007199254740992'):
            with self.subTest(rid=rid), self.assertRaises(ValueError):
                session.accept(self.frame(self.request(rid)))
        session.accept(self.frame(self.request('601')))
        session.accept(self.frame({'kind': 'cancel', 'requestId': '601'}))
        with self.assertRaises(ValueError):
            session.accept(self.frame({'kind': 'cancel', 'requestId': '601'}))
        # Already-completed work may win cancellation, but no invented rollback.
        session.accept(self.frame(self.result('601'), 'plugin'))
        session.accept(self.frame(self.request('602')))
        with self.assertRaises(ValueError):
            session.accept(self.frame(self.result('602', status='cancelled'), 'plugin'))
        session.accept(self.frame({'kind': 'cancel', 'requestId': '602'}))
        session.accept(self.frame(self.result('602', status='cancelled'), 'plugin'))
        self.assertFalse(session.pending)
        self.assertFalse(session.cancelled)

    def test_surface_snapshot_order_full_validation_close_and_shutdown(self):
        session = self.negotiated()
        open_body = self.request('1', 'surface.open', 'rchub.surface.lifecycle', {'surfaceId': 'controls'})
        snapshot = {'kind': 'state', 'surfaceId': 'controls', 'revision': 1,
                    'values': {'enabled': False, 'revision': 0, 'items': [{'label': '😀ş'}]}}
        with self.assertRaises(ValueError):
            session.accept(self.frame(snapshot, 'plugin'))
        session.accept(self.frame(open_body))
        session.accept(self.frame(self.result('1', 'surface.open', 'rchub.surface.lifecycle', payload={}), 'plugin'))
        session.accept(self.frame(snapshot, 'plugin'))
        for bad in [snapshot, snapshot | {'revision': 3}, snapshot | {'revision': 2, 'values': {'enabled': True}},
                    snapshot | {'revision': 2, 'values': snapshot['values'] | {'items': [{'label': 7}]}}]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                session.accept(self.frame(bad, 'plugin'))
        self.assertEqual(session.surfaces['controls'], 1)
        session.accept(self.frame(self.request('2', 'surface.close', 'rchub.surface.lifecycle', {'surfaceId': 'controls'})))
        session.accept(self.frame(self.result('2', 'surface.close', 'rchub.surface.lifecycle', payload={}), 'plugin'))
        with self.assertRaises(ValueError):
            session.accept(self.frame(snapshot | {'revision': 2}, 'plugin'))
        session.accept(self.frame({'kind': 'shutdown', 'reason': 'context-lost'}))
        with self.assertRaises(ValueError):
            session.accept(self.frame(self.request('3')))
        session.accept(self.frame({'kind': 'lifecycle', 'state': 'stopped'}, 'plugin'))
        self.assertTrue(session.closed)

    def test_bounded_array_schema_depth_and_payloads(self):
        schema = {'type': 'array', 'maxItems': 2, 'items': {'type': 'string', 'maxLength': 2}}
        for good in [[], ['😀ş'], ['a', 'b']]:
            sdk._payload(schema, good)
        for bad in [None, {}, ['a', 'b', 'c'], [True], ['abc']]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                sdk._payload(schema, bad)
        for limit in (0, 65, True):
            bad = copy.deepcopy(self.manifest)
            bad['state']['properties']['items']['maxItems'] = limit
            with self.assertRaises(ValueError):
                sdk.validate_manifest(bad)
        bad = copy.deepcopy(self.manifest)
        field = {'type': 'string', 'maxLength': 1}
        for _ in range(4):
            field = {'type': 'array', 'maxItems': 1, 'items': field}
        bad['state']['properties']['deep'] = field
        with self.assertRaises(ValueError):
            sdk.validate_manifest(bad)

    def test_layout_binding_semantic_negative_corpus(self):
        for change in [{'type': 'html'}, {'min': 0}, {'bind': 'missing'}, {'argument': 'missing'},
                       {'action': 'vendor.fixture.open'}, {'children': []}, {'value': 'true'},
                       {'submit': False}, {'inputBindings': {'enabled': 'missing'}}]:
            bad = copy.deepcopy(self.manifest)
            bad['surfaces'][0]['controls'][0]['children'][0].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                sdk.validate_manifest(bad)
        for change in [{'itemLabel': 'missing'}, {'itemValue': 'missing'}, {'bind': 'enabled'}]:
            bad = copy.deepcopy(self.manifest)
            bad['surfaces'][0]['controls'][0]['children'][2].update(change)
            with self.assertRaises(ValueError):
                sdk.validate_manifest(bad)

    def test_transport_request_dispatch_result_and_capability_roundtrip(self):
        package = sdk.identity(self.manifest)
        begin = self.wire('install.begin', package=package, manifest=self.manifest, size=23,
                          consent={'disclosureVersion': 2, 'accepted': True, 'trust': 'unsandboxed-user',
                                   'permissions': ['native.unsandboxed']})
        messages = [self.wire(), begin,
                    self.wire('install.chunk', package=package, transferId='t1', offset=0,
                              data=base64.b64encode(b'abc').decode()),
                    self.wire('install.commit', package=package, transferId='t1'),
                    self.wire('install.abort', package=package, transferId='t1'),
                    self.wire('remove', package=package), self.wire('activate', package=package),
                    self.wire('invoke', package=package, instanceId='i1', contextId='c1', name='vendor.fixture.apply', payload={'enabled': True}),
                    self.wire('subscribe', package=package, instanceId='i1', contextId='c1', name='vendor.fixture.changed'),
                    self.wire('unsubscribe', package=package, instanceId='i1', contextId='c1', name='vendor.fixture.changed', subscriptionId='s1'),
                    self.wire('deactivate', package=package, instanceId='i1', contextId='c1'),
                    self.wire('surface.open', package=package, instanceId='i1', contextId='c1', surfaceId='controls'),
                    self.wire('surface.close', package=package, instanceId='i1', contextId='c1', surfaceId='controls'),
                    self.wire('cancel', targetRequestId='other')]
        for message in messages:
            self.assertEqual(sdk.decode_transport(sdk.encode_transport(message)), message)
            dispatch = {k: v for k, v in message.items() if k not in ('deviceId', 'requestId')}
            if dispatch['action'] == 'cancel':
                dispatch['targetOperationId'] = dispatch.pop('targetRequestId')
            dispatch.update(kind='dispatch', operationId='op1', epoch='epoch1', deadline=123,
                            binding={'serverOrigin': 'http://localhost', 'deviceId': 'device1', 'ownerUserId': 'owner1'})
            sdk.validate_transport(dispatch)
        sdk.validate_transport({'v': 3, 'kind': 'capabilities', 'epoch': 'epoch1', 'protocol': sdk.PROTOCOL,
                                'version': 1, 'features': list(sdk.FEATURES)})
        sdk.validate_transport({'v': 3, 'kind': 'result', 'requestId': 'client1', 'deviceId': 'device1',
                                'action': 'inventory', 'status': 'ok', 'entries': [
                                    {'package': package, 'manifest': self.manifest,
                                     'availability': 'ready', 'status': 'installed'}]})
        for mutation in [lambda m: m.update(url='https://example.invalid'),
                         lambda m: m.update(binding={'ownerUserId': 'spoof'}),
                         lambda m: m.update(expectedPackage=package),
                         lambda m: m.update(package=package | {'sha256': '0' * 64}),
                         lambda m: m['consent'].update(disclosureVersion=1),
                         lambda m: m['consent'].update(permissions=[])]:
            bad = copy.deepcopy(begin)
            mutation(bad)
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                sdk.validate_transport(bad)
        bad = {'v': 3, 'kind': 'result', 'operationId': 'op1', 'epoch': 'e1', 'action': 'inventory',
               'status': 'unknown', 'entries': []}
        with self.assertRaises(ValueError):
            sdk.validate_transport(bad)

    def test_transfer_bounded_identity_offsets_digest_and_stale_authority(self):
        content = b'x' * sdk.MAX_CHUNK + b'final'
        manifest = copy.deepcopy(self.manifest)
        manifest['execution']['artifact']['sha256'] = hashlib.sha256(content).hexdigest()
        package = sdk.identity(manifest)
        transfer = sdk.ArtifactTransfer(manifest, package, len(content), 't1', 'currentEpoch')
        data = base64.b64encode(content[:sdk.MAX_CHUNK]).decode()
        for args in [('other', package, 'currentEpoch', 0, data),
                     ('t1', package, 'oldEpoch', 0, data),
                     ('t1', package | {'sha256': '0' * 64}, 'currentEpoch', 0, data),
                     ('t1', package, 'currentEpoch', 1, data),
                     ('t1', package, 'currentEpoch', 0, 'Zg==='),
                     ('t1', package, 'currentEpoch', 0, base64.b64encode(b'x' * (sdk.MAX_CHUNK + 1)).decode())]:
            with self.subTest(args=args[:4]), self.assertRaises(ValueError):
                transfer.append(*args)
            self.assertEqual(len(transfer.content), 0)
        transfer.append('t1', package, 'currentEpoch', 0, data)
        with self.assertRaises(ValueError):
            transfer.append('t1', package, 'currentEpoch', 0, data)
        with self.assertRaises(ValueError):
            transfer.finish('t1', package, 'currentEpoch')
        transfer.append('t1', package, 'currentEpoch', sdk.MAX_CHUNK, base64.b64encode(b'final').decode())
        self.assertEqual(transfer.finish('t1', package, 'currentEpoch'), content)
        self.assertTrue(transfer.closed)
        with self.assertRaises(ValueError):
            transfer.finish('t1', package, 'currentEpoch')
        transfer = sdk.ArtifactTransfer(manifest, package, 1, 't2', 'epoch')
        transfer.append('t2', package, 'epoch', 0, 'eA==')
        with self.assertRaises(ValueError):
            transfer.finish('t2', package, 'epoch')
        self.assertTrue(transfer.closed)

    def test_optional_store_cas_limits_paths_no_mutation_on_failure(self):
        store = sdk.ScopedStore()
        self.assertEqual(store.execute('store.get', {'key': 'draft'}), {'found': False, 'revision': 0})
        self.assertEqual(store.execute('store.put', {'key': 'draft', 'expectedRevision': 0, 'value': {'label': '😀ş'}}), {'revision': 1})
        for key in ('../x', '/x', 'C:/x', 'x/y', 'https://x', 'constructor'):
            with self.assertRaises(ValueError):
                store.execute('store.get', {'key': key})
        for payload in [{'key': 'draft', 'expectedRevision': 0, 'value': {}},
                        {'key': 'draft', 'expectedRevision': True, 'value': {}},
                        {'key': 'draft', 'expectedRevision': 1, 'value': {'x': 'z' * 8192}}]:
            with self.assertRaises(ValueError):
                store.execute('store.put', payload)
        self.assertEqual(store.execute('store.get', {'key': 'draft'})['revision'], 1)
        for i in range(1, sdk.STORE_KEYS):
            store.execute('store.put', {'key': str(i), 'expectedRevision': 0, 'value': {}})
        with self.assertRaises(ValueError):
            store.execute('store.put', {'key': 'overflow', 'expectedRevision': 0, 'value': {}})
        manifest = copy.deepcopy(self.manifest)
        manifest['requiredFeatures'].append('scoped-store-v1')
        manifest['storage']['choices'] = ['host-scoped']
        session = self.negotiated(manifest)
        request = self.request('1', 'store.put', 'rchub.store.value', {'key': 'draft', 'expectedRevision': 0, 'value': {}})
        with self.assertRaises(ValueError):
            session.accept(self.frame(request))
        session.accept(self.frame(request, 'plugin'))
        session.accept(self.frame(self.result('1', 'store.put', 'rchub.store.value', payload={'revision': 1})))

    def test_standalone_process_framing_action_events_state_without_os_effects(self):
        messages = [self.frame({'kind': 'hello', 'version': 1, 'features': self.manifest['requiredFeatures']}),
                    self.frame(self.request('1', 'surface.open', 'rchub.surface.lifecycle', {'surfaceId': 'controls'})),
                    self.frame(self.request('2', 'subscribe', 'vendor.fixture.changed')),
                    self.frame(self.request('3')),
                    self.frame(self.request('4', 'unsubscribe', 'vendor.fixture.changed', subscriptionId='2')),
                    self.frame({'kind': 'shutdown', 'reason': 'host-exit'})]
        env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR', 'TEMP', 'TMP') if k in os.environ}
        result = subprocess.run([sys.executable, '-I', '-B', str(sdk.ROOT / 'plugins/examples/application-v2/native.py')],
                                input=b''.join(sdk.encode_frame(m) for m in messages),
                                capture_output=True, timeout=5, env=env, cwd=sdk.ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, b'')
        decoder = sdk.FrameDecoder()
        responses = decoder.feed(result.stdout)
        decoder.finish()
        session = sdk.ProtocolSession(self.manifest, 'instance1', 'context1')
        session.accept(messages[0])
        cursor = 0
        for response in responses:
            body = response['body']
            if body['kind'] == 'result':
                cursor += 1
                session.accept(messages[cursor])
            elif body['kind'] == 'lifecycle' and body['state'] == 'stopped':
                session.accept(messages[-1])
            session.accept(response)
        self.assertTrue(session.closed)
        self.assertEqual([r['body']['kind'] for r in responses].count('event'), 1)
        self.assertEqual([r['body']['kind'] for r in responses].count('state'), 2)

    def test_shared_transcript_negative_corpus_and_bound_observations(self):
        fixtures = Path(__file__).parent / 'fixtures/v2'
        transcript = json.loads((fixtures / 'protocol.transcript.json').read_text(encoding='utf-8'))
        session = sdk.ProtocolSession(self.manifest, 'instance1', 'context1')
        for message in transcript:
            decoded = sdk.FrameDecoder().feed(sdk.encode_frame(message))
            self.assertEqual(decoded, [message])
            session.accept(decoded[0])
        self.assertTrue(session.closed)
        corpus = json.loads((fixtures / 'negative.corpus.json').read_text(encoding='utf-8'))
        for raw in corpus['invalidJson']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                sdk.strict_json(raw.encode('utf-8'))
        for entry in corpus['invalidEntries']:
            manifest = copy.deepcopy(self.manifest)
            manifest['execution']['artifact']['entry'] = entry
            with self.subTest(entry=entry), self.assertRaises(ValueError):
                sdk.validate_manifest(manifest)
        for data in corpus['invalidChunks']:
            with self.subTest(data=data), self.assertRaises(ValueError):
                sdk.decode_chunk(data)
        for key in corpus['invalidStoreKeys']:
            with self.subTest(key=key), self.assertRaises(ValueError):
                sdk.validate_store_request('store.get', {'key': key})
        state = next(m for m in transcript if m['body']['kind'] == 'state')
        event = {'v': 3, 'kind': 'event', 'operationId': 'op1', 'epoch': 'epoch1',
                 'package': sdk.identity(self.manifest), 'message': state}
        sdk.validate_bound_transport(event, self.manifest, 'instance1', 'context1')
        with self.assertRaises(ValueError):
            sdk.validate_bound_transport(event, self.manifest, 'instance1', 'old')
        bad = copy.deepcopy(event)
        bad['message']['body']['values']['items'] = [{'label': False}]
        with self.assertRaises(ValueError):
            sdk.validate_bound_transport(bad, self.manifest, 'instance1', 'context1')
        request = self.wire('invoke', package=sdk.identity(self.manifest), instanceId='instance1',
                            contextId='context1', name='vendor.fixture.apply', payload={'enabled': True})
        sdk.validate_bound_transport(request, self.manifest, 'instance1', 'context1')
        with self.assertRaises(ValueError):
            sdk.validate_bound_transport(request | {'payload': {'enabled': 1}}, self.manifest, 'instance1', 'context1')

    def test_inventory_pagination_update_review_and_store_total_quota(self):
        package = sdk.identity(self.manifest)
        entry = {'package': package, 'manifest': self.manifest, 'availability': 'ready', 'status': 'installed'}
        response = {'v': 3, 'kind': 'result', 'requestId': 'r1', 'deviceId': 'd1', 'action': 'inventory',
                    'status': 'ok', 'entries': [entry, entry], 'nextCursor': 'page2'}
        sdk.validate_transport(response)
        with self.assertRaises(ValueError):
            sdk.validate_transport(response | {'entries': [entry] * 3})
        target = self.manifest | {'version': '0.0.2'}
        begin = self.wire('install.begin', package=sdk.identity(target), manifest=target, size=1,
                          expectedPackage=package, consent={'disclosureVersion': 2, 'accepted': True,
                          'trust': 'unsandboxed-user', 'permissions': ['native.unsandboxed']})
        sdk.validate_transport(begin)
        for expected in (package | {'id': 'other.vendor'}, sdk.identity(target)):
            with self.assertRaises(ValueError):
                sdk.validate_transport(begin | {'expectedPackage': expected})
        store = sdk.ScopedStore()
        value = {'v': 'x' * 8000}
        for i in range(32):
            store.execute('store.put', {'key': str(i), 'expectedRevision': 0, 'value': value})
        with self.assertRaises(ValueError):
            store.execute('store.put', {'key': 'overflow', 'expectedRevision': 0, 'value': value})
        self.assertEqual(len(store.values), 32)


class OwnerLifetimeRevision2Tests(unittest.TestCase):
    def setUp(self):
        fixtures = Path(__file__).parent / 'fixtures/v2'
        self.manifest = json.loads((fixtures / 'vendor.fixture.json').read_text(encoding='utf-8'))
        self.corpus = json.loads((fixtures / 'owner-context.corpus.json').read_text(encoding='utf-8'))

    def test_owner_context_canonical_binding_replay_revoke_and_disconnect(self):
        valid, revoked = self.corpus['controls']
        for message in (valid, revoked, self.corpus['surfaceCloseDispatch']):
            self.assertEqual(sdk.decode_transport(sdk.encode_transport(message)), message)
        owner = sdk.OwnerContext('http://127.0.0.1:5000', 'device1', 'epoch1')
        self.assertFalse(owner.control(valid))
        owner.instances.add('instance1')
        owner.stages.add('stage1')
        for bad in [valid, revoked | {'epoch': 'old'}, revoked | {'version': 1},
                    revoked | {'binding': valid['binding'] | {'ownerUserId': 'foreign'}},
                    revoked | {'binding': valid['binding'] | {'serverOrigin': 'http://other'}},
                    revoked | {'binding': valid['binding'] | {'deviceId': 'other'}}]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                owner.control(bad)
            self.assertEqual(owner.instances, {'instance1'})
        self.assertTrue(owner.control(revoked))
        self.assertFalse(owner.valid)
        self.assertFalse(owner.instances)
        self.assertFalse(owner.stages)
        with self.assertRaises(ValueError):
            owner.control(valid)
        for change in ({'authorityRevision': True}, {'authorityRevision': 0},
                       {'authorityRevision': 9007199254740992},
                       {'state': 'revoked', 'reason': 'owner-validated'}):
            with self.assertRaises(ValueError):
                sdk.validate_transport(valid | change)
        self.assertFalse(owner.control(valid | {'authorityRevision': 3}))
        owner.instances.add('instance2')
        owner.stages.add('stage2')
        owner.disconnect()
        self.assertFalse(owner.instances)
        self.assertFalse(owner.stages)
        with self.assertRaises(ValueError):
            owner.control(valid | {'authorityRevision': 4})

    def test_revision2_dispatch_requires_control_and_exact_revision_no_spoof_fields(self):
        owner = sdk.OwnerContext('http://127.0.0.1:5000', 'device1', 'epoch1')
        dispatch = self.corpus['surfaceCloseDispatch']
        with self.assertRaises(ValueError):
            owner.dispatch(dispatch)
        owner.control(self.corpus['controls'][0])
        owner.dispatch(dispatch)
        for change in [{'version': 1}, {'authorityRevision': 2}, {'epoch': 'other'},
                       {'binding': dispatch['binding'] | {'ownerUserId': 'other'}}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                owner.dispatch(dispatch | change)
        no_revision = {k: v for k, v in dispatch.items() if k != 'version'}
        with self.assertRaises(ValueError):
            sdk.validate_transport(no_revision, expected_version=2)
        with self.assertRaises(ValueError):
            sdk.validate_transport(dispatch, expected_version=1)
        for omit in ('targetOperationId', 'authorityRevision'):
            with self.assertRaises(ValueError):
                sdk.validate_transport({k: v for k, v in dispatch.items() if k != omit})
        request = {k: v for k, v in dispatch.items() if k not in
                   ('operationId', 'epoch', 'deadline', 'binding', 'authorityRevision', 'targetOperationId')}
        request.update(kind='request', requestId='request1', deviceId='device1')
        sdk.validate_transport(request, expected_version=2)
        for extra in ({'targetOperationId': 'foreignWatch'}, {'authorityRevision': 1}, {'binding': dispatch['binding']}):
            with self.assertRaises(ValueError):
                sdk.validate_transport(request | extra)
        for extra in ({'requestId': 'client'}, {'action': 'deactivate'}, {'package': dispatch['package']}):
            with self.assertRaises(ValueError):
                sdk.validate_transport(self.corpus['controls'][1] | extra)

    def test_multiviewer_surface_refcount_rebased_snapshots_exact_close(self):
        fanout = sdk.SurfaceWatches(self.manifest)
        values = {'enabled': False, 'revision': 0, 'items': [{'label': 'Generic item'}]}
        for step in self.corpus['watchScenario']:
            if step['step'] == 'open':
                first, snapshot = fanout.opened(step['operationId'], step['surfaceId'])
                self.assertEqual(first, step['first'])
                if not first:
                    self.assertEqual(snapshot['revision'], step['initialWireRevision'])
                    self.assertEqual(snapshot['values'], values)
            elif step['step'] == 'snapshot':
                body = {'kind': 'state', 'surfaceId': 'controls', 'revision': step['ipcRevision'], 'values': values}
                result = fanout.snapshot(body)
                self.assertEqual({k: v['revision'] for k, v in result.items()}, step['expectedWireRevisions'])
                with self.assertRaises(ValueError):
                    fanout.snapshot(body)
            else:
                self.assertEqual(fanout.close(step['operationId'], step['surfaceId']), step['last'])
        self.assertFalse(fanout.watches)
        with self.assertRaises(ValueError):
            fanout.opened('openC', 'controls')
        fanout.closed('controls')
        self.assertEqual(fanout.opened('openC', 'controls'), (True, None))
        fanout.invalidate()
        self.assertFalse(fanout.surfaces)

    def test_second_viewer_waits_for_first_snapshot_foreign_close_and_viewer_loss_not_owner_loss(self):
        fanout = sdk.SurfaceWatches(self.manifest)
        fanout.opened('openA', 'controls')
        with self.assertRaises(ValueError):
            fanout.opened('openB', 'controls')
        self.assertEqual(set(fanout.watches), {'openA'})
        values = {'enabled': False, 'revision': 0, 'items': []}
        fanout.snapshot({'kind': 'state', 'surfaceId': 'controls', 'revision': 1, 'values': values})
        fanout.opened('openB', 'controls')
        for target, surface in [('unknown', 'controls'), ('openA', 'foreign')]:
            with self.assertRaises(ValueError):
                fanout.close(target, surface)
        owner = sdk.OwnerContext('http://127.0.0.1:5000', 'device1', 'epoch1')
        owner.control(self.corpus['controls'][0])
        owner.instances.add('instance1')
        # Viewer A logout maps to exact watch cleanup, never owner revocation.
        self.assertFalse(fanout.close('openA', 'controls'))
        self.assertEqual(owner.instances, {'instance1'})
        self.assertTrue(owner.valid)
        self.assertEqual(set(fanout.watches), {'openB'})
        before = copy.deepcopy(fanout.surfaces)
        with self.assertRaises(ValueError):
            fanout.snapshot({'kind': 'state', 'surfaceId': 'controls', 'revision': 2, 'values': {'enabled': True}})
        self.assertEqual(fanout.surfaces, before)
        owner.control(self.corpus['controls'][1])
        fanout.invalidate()
        self.assertFalse(owner.instances)
        self.assertFalse(fanout.watches)

    def test_revision2_process_negotiation_and_harmless_peer(self):
        session = sdk.ProtocolSession(self.manifest, 'instance1', 'context1', version=2)
        frame = lambda body, sender='host': {'protocol': sdk.PROTOCOL, 'instanceId': 'instance1',
                                            'contextId': 'context1', 'sender': sender, 'body': body}
        with self.assertRaises(ValueError):
            session.accept(frame({'kind': 'hello', 'version': 1, 'features': self.manifest['requiredFeatures']}))
        hello = frame({'kind': 'hello', 'version': 2, 'features': self.manifest['requiredFeatures']})
        shutdown = frame({'kind': 'shutdown', 'reason': 'host-exit'})
        env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR', 'TEMP', 'TMP') if k in os.environ}
        result = subprocess.run([sys.executable, '-I', '-B', str(sdk.ROOT / 'plugins/examples/application-v2/native.py')],
                                input=sdk.encode_frame(hello) + sdk.encode_frame(shutdown),
                                capture_output=True, timeout=5, env=env, cwd=sdk.ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)
        session.accept(hello)
        decoder = sdk.FrameDecoder()
        responses = decoder.feed(result.stdout)
        decoder.finish()
        self.assertEqual(responses[0]['body']['version'], 2)
        for response in responses:
            if response['body'].get('state') == 'stopped':
                session.accept(shutdown)
            session.accept(response)
        self.assertTrue(session.closed)

    def test_owner_replacement_and_late_join_uses_watch_revision_not_ipc_revision(self):
        owner = sdk.OwnerContext('http://127.0.0.1:5000', 'device1', 'epoch1')
        valid = self.corpus['controls'][0]
        owner.control(valid)
        owner.instances.add('oldInstance')
        owner.stages.add('oldStage')
        replacement = valid | {'authorityRevision': 2,
                               'binding': valid['binding'] | {'ownerUserId': 'owner2'}}
        self.assertTrue(owner.control(replacement))
        self.assertFalse(owner.instances)
        self.assertFalse(owner.stages)
        self.assertTrue(owner.valid)
        with self.assertRaises(ValueError):
            owner.control(self.corpus['controls'][1] | {'authorityRevision': 3})
        fanout = sdk.SurfaceWatches(self.manifest)
        fanout.opened('openA', 'controls')
        values = {'enabled': False, 'revision': 0, 'items': []}
        for revision in range(1, 4):
            fanout.snapshot({'kind': 'state', 'surfaceId': 'controls', 'revision': revision, 'values': values})
        first, snapshot = fanout.opened('lateB', 'controls')
        self.assertFalse(first)
        self.assertEqual(snapshot['revision'], 1)
        observations = fanout.snapshot({'kind': 'state', 'surfaceId': 'controls', 'revision': 4, 'values': values})
        self.assertEqual(observations['openA']['revision'], 4)
        self.assertEqual(observations['lateB']['revision'], 2)
        with self.assertRaises(ValueError):
            fanout.opened('lateB', 'controls')
        self.assertEqual(set(fanout.watches), {'openA', 'lateB'})


if __name__ == '__main__':
    unittest.main()
