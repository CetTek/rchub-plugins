"""MIT. Offline native-peer/descriptor smoke; not host install/auth/UI acceptance."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('prepare_example', HERE / 'prepare.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)
sdk = prepare.sdk


class ExampleTests(unittest.TestCase):
    def test_new_namespace_identity_and_artifact(self):
        manifest = prepare.descriptor(b'fixture bytes', 'org.example.different')
        self.assertEqual(manifest['id'], 'org.example.different')
        self.assertNotIn('vendor.fixture', sdk.canonical(manifest).decode())
        sdk.verify_artifact(manifest, 'windows-x86_64', 'bin/switchboard.exe', b'fixture bytes')
        with self.assertRaises(ValueError):
            sdk.verify_artifact(manifest, 'windows-x86_64', 'bin/switchboard.exe', b'changed')
        with self.assertRaises(ValueError):
            prepare.descriptor(b'x', '../invalid')

    @unittest.skipUnless(shutil.which('rustc'), 'installed rustc required; no downloads')
    def test_compiled_native_abi2_roundtrip(self):
        with tempfile.TemporaryDirectory(prefix='rchub-public-example-') as temp:
            executable = Path(temp) / ('peer.exe' if os.name == 'nt' else 'peer')
            built = subprocess.run(['rustc', '--edition=2021', str(HERE / 'native.rs'),
                                    '-o', str(executable)], capture_output=True, timeout=60)
            self.assertEqual(built.returncode, 0, built.stderr)
            manifest = prepare.descriptor(executable.read_bytes())
            def frame(body):
                return {'protocol': sdk.PROTOCOL, 'instanceId': 'example1',
                        'contextId': 'context1', 'sender': 'host', 'body': body}
            def request(rid, operation, name, payload, **extra):
                return frame({'kind': 'request', 'requestId': str(rid), 'operation': operation,
                              'name': name, 'payload': payload, **extra})
            messages = [frame({'kind': 'hello', 'version': 2, 'features': manifest['requiredFeatures']}),
                        request(1, 'surface.open', 'rchub.surface.lifecycle', {'surfaceId': 'controls'}),
                        request(2, 'subscribe', 'com.example.switchboard.changed', {}),
                        request(3, 'invoke', 'com.example.switchboard.apply', {'enabled': True}),
                        request(4, 'invoke', 'com.example.switchboard.open', {}),
                        request(5, 'unsubscribe', 'com.example.switchboard.changed', {}, subscriptionId='2'),
                        request(6, 'surface.close', 'rchub.surface.lifecycle', {'surfaceId': 'controls'}),
                        frame({'kind': 'shutdown', 'reason': 'host-exit'})]
            env = {k: os.environ[k] for k in ('SystemRoot', 'WINDIR', 'TEMP', 'TMP') if k in os.environ}
            result = subprocess.run([str(executable)], input=b''.join(map(sdk.encode_frame, messages)),
                                    capture_output=True, env=env, cwd=temp, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, b'')
            decoder = sdk.FrameDecoder()
            responses = decoder.feed(result.stdout)
            decoder.finish()
            session = sdk.ProtocolSession(manifest, 'example1', 'context1', version=2)
            session.accept(messages[0])
            cursor = 0
            for response in responses:
                if response['body']['kind'] == 'result':
                    cursor += 1
                    session.accept(messages[cursor])
                elif response['body'].get('state') == 'stopped':
                    session.accept(messages[-1])
                session.accept(response)
            self.assertTrue(session.closed)
            self.assertEqual(sum(m['body']['kind'] == 'event' for m in responses), 1)
            snapshots = [m['body'] for m in responses if m['body']['kind'] == 'state']
            self.assertEqual([m['revision'] for m in snapshots], [1, 2, 3])
            self.assertTrue(snapshots[1]['values']['enabled'])
            self.assertTrue(any(m['body'].get('name') == 'com.example.switchboard.open'
                                and m['body'].get('status') == 'rejected' for m in responses))


if __name__ == '__main__':
    unittest.main()
