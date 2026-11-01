#!/usr/bin/env python3
"""Prepare and check experimental V2 descriptors against supplied artifact bytes."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

import rchub_app_v2 as sdk


def read_manifest(path):
    return sdk.strict_json(path.read_bytes())


def read_artifact(path):
    # Bound the read even when the input is larger than the V2 artifact limit.
    with path.open('rb') as source:
        content = source.read(sdk.MAX_ARTIFACT + 1)
    sdk.require(0 < len(content) <= sdk.MAX_ARTIFACT, 'Artifact byte limit')
    return content


def prepare(template, artifact, target, entry):
    manifest = copy.deepcopy(template)
    sdk.require(type(manifest) is dict and type(manifest.get('execution')) is dict,
                'Template must contain execution')
    manifest['execution']['artifact'] = {
        'target': target, 'entry': entry, 'sha256': hashlib.sha256(artifact).hexdigest()
    }
    sdk.verify_artifact(manifest, target, entry, artifact)
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    build = commands.add_parser('prepare', help='Bind a template to supplied native artifact bytes')
    build.add_argument('--template', type=Path, required=True)
    build.add_argument('--artifact', type=Path, required=True)
    build.add_argument('--target', required=True, choices=('windows-x86_64', 'linux-x86_64'))
    build.add_argument('--entry', required=True, help='Relative installed artifact entry')
    build.add_argument('--output', type=Path, required=True)
    check = commands.add_parser('verify', help='Validate a descriptor and supplied artifact bytes')
    check.add_argument('--manifest', type=Path, required=True)
    check.add_argument('--artifact', type=Path, required=True)
    check.add_argument('--target', required=True, choices=('windows-x86_64', 'linux-x86_64'))
    check.add_argument('--entry', required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == 'prepare':
            sdk.require(args.output.resolve() not in (args.template.resolve(), args.artifact.resolve()),
                        'Output must differ from template and artifact')
            manifest = prepare(read_manifest(args.template), read_artifact(args.artifact),
                               args.target, args.entry)
            args.output.write_bytes(sdk.canonical(manifest))
        else:
            manifest = read_manifest(args.manifest)
            sdk.verify_artifact(manifest, args.target, args.entry, read_artifact(args.artifact))
        print(json.dumps(sdk.identity(manifest), sort_keys=True))
        return 0
    except (ValueError, KeyError, TypeError, OSError) as error:
        print(f'V2 {args.command} failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
