"""Tests for the experimental V2 descriptor preparation CLI."""
import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path

import rchub_app_v2 as sdk
import rchub_app_v2_cli as cli


class V2PreparationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = Path(__file__).parent / 'fixtures/v2/vendor.fixture.json'
        self.manifest = json.loads(self.fixture.read_text(encoding='utf-8'))

    def test_prepare_and_verify_cli(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / 'app'
            output = root / 'manifest.json'
            artifact.write_bytes(b'fixture bytes, not an executable')
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(cli.main(['prepare', '--template', str(self.fixture),
                                           '--artifact', str(artifact), '--target', 'linux-x86_64',
                                           '--entry', 'bin/app', '--output', str(output)]), 0)
                self.assertEqual(cli.main(['verify', '--manifest', str(output), '--artifact', str(artifact),
                                           '--target', 'linux-x86_64', '--entry', 'bin/app']), 0)
            prepared = sdk.strict_json(output.read_bytes())
            self.assertEqual(prepared['id'], self.manifest['id'])
            self.assertEqual(prepared['execution']['artifact']['entry'], 'bin/app')
            artifact.write_bytes(b'changed')
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(cli.main(['verify', '--manifest', str(output), '--artifact', str(artifact),
                                           '--target', 'linux-x86_64', '--entry', 'bin/app']), 1)

    def test_prepare_rejects_invalid_artifacts_and_entries(self):
        for content, entry in [(b'', 'bin/app'), (b'x', '../app'), (b'x', 'NUL')]:
            with self.subTest(entry=entry), self.assertRaises(ValueError):
                cli.prepare(copy.deepcopy(self.manifest), content, 'linux-x86_64', entry)
        with self.assertRaises(ValueError):
            cli.prepare(copy.deepcopy(self.manifest), b'x' * (sdk.MAX_ARTIFACT + 1),
                        'linux-x86_64', 'bin/app')

    def test_prepare_does_not_overwrite_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / 'app'
            artifact.write_bytes(b'content')
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(cli.main(['prepare', '--template', str(self.fixture),
                                           '--artifact', str(artifact), '--target', 'linux-x86_64',
                                           '--entry', 'bin/app', '--output', str(artifact)]), 1)
            self.assertEqual(artifact.read_bytes(), b'content')


if __name__ == '__main__':
    unittest.main()
