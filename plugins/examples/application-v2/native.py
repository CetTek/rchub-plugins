"""Standalone stdlib protocol peer. Explicit test execution only; no OS effects.

No filesystem writes, UI, network, subprocesses or environment reads. stdin/stdout
are exclusively 4-byte-big-endian framed JSON. Diagnostics never go to stdout.
This source is NOT an installable native executable or a host security boundary.
"""
import json
import struct
import sys

PROTOCOL = 'rchub-app-v2-experimental'
FEATURES = ['native-process-v1', 'events-v1', 'surfaces-v1']


def read_exact(size):
    result = bytearray()
    while len(result) < size:
        chunk = sys.stdin.buffer.read(size - len(result))
        if not chunk:
            if not result:
                return None
            raise ValueError('Truncated frame')
        result.extend(chunk)
    return bytes(result)


def strict_pairs(entries):
    result = {}
    for key, value in entries:
        if key in result or key in ('constructor', 'prototype', '__proto__'):
            raise ValueError('Duplicate/reserved key')
        result[key] = value
    return result


def invalid_number(value):
    raise ValueError('Integer-only JSON')


def main():
    context = None
    phase = 'new'
    enabled, revision = False, 0
    subscriptions, surfaces = set(), {}
    highwater = 0

    def send(body):
        message = {'protocol': PROTOCOL, 'instanceId': context[0], 'contextId': context[1],
                   'sender': 'plugin', 'body': body}
        raw = (json.dumps(message, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode('utf-8')
        if len(raw) > 65536:
            raise ValueError('Frame size')
        sys.stdout.buffer.write(struct.pack('>I', len(raw)) + raw)
        sys.stdout.buffer.flush()

    def state(surface_id):
        surfaces[surface_id] += 1
        send({'kind': 'state', 'surfaceId': surface_id, 'revision': surfaces[surface_id],
              'values': {'enabled': enabled, 'revision': revision, 'items': [{'label': 'Generic item'}]}})

    while True:
        header = read_exact(4)
        if header is None:
            return
        length = struct.unpack('>I', header)[0]
        if not 0 < length <= 65536:
            raise ValueError('Frame size')
        raw = read_exact(length)
        if raw is None:
            raise ValueError('Truncated frame')
        message = json.loads(raw.decode('utf-8'), object_pairs_hook=strict_pairs,
                             parse_float=invalid_number, parse_constant=invalid_number)
        if (type(message) is not dict or set(message) != {'protocol', 'instanceId', 'contextId', 'sender', 'body'}
                or message['protocol'] != PROTOCOL or message['sender'] != 'host'):
            raise ValueError('Envelope')
        current = (message['instanceId'], message['contextId'])
        if context is not None and current != context:
            raise ValueError('Stale context')
        body = message['body']
        kind = body['kind']
        if phase == 'new':
            if kind != 'hello' or type(body['version']) is not int or body['version'] not in (1, 2) or not set(FEATURES) <= set(body['features']):
                raise ValueError('Negotiation')
            context, phase = current, 'active'
            send({'kind': 'welcome', 'version': body['version'], 'features': FEATURES})
            send({'kind': 'lifecycle', 'state': 'started'})
            continue
        if kind == 'shutdown':
            send({'kind': 'lifecycle', 'state': 'stopped'})
            return
        if kind == 'cancel':
            # All example work is synchronous and already terminal. No replay or
            # false claim that a completed effect was undone.
            continue
        if kind != 'request':
            raise ValueError('Unsupported message')
        rid = body['requestId']
        if not rid.isascii() or not rid.isdecimal() or rid.startswith('0') or not highwater < int(rid) <= 9007199254740991:
            raise ValueError('Request replay')
        highwater = int(rid)
        op, name, payload = body['operation'], body['name'], body['payload']
        result = {'kind': 'result', 'requestId': rid, 'operation': op, 'name': name,
                  'status': 'rejected', 'payload': {}}
        changed = False
        open_surface = None
        if op == 'invoke' and name == 'vendor.fixture.apply' and set(payload) == {'enabled'} and type(payload['enabled']) is bool:
            if revision < 100:
                enabled, revision, changed = payload['enabled'], revision + 1, True
                result.update(status='ok', payload={'accepted': True})
        elif op == 'invoke' and name == 'vendor.fixture.open' and payload == {}:
            # Deliberately no native UI in this harmless reference.
            pass
        elif op == 'subscribe' and name == 'vendor.fixture.changed' and payload == {}:
            subscriptions.add(rid)
            result['status'] = 'ok'
        elif op == 'unsubscribe' and name == 'vendor.fixture.changed' and payload == {} and body.get('subscriptionId') in subscriptions:
            subscriptions.remove(body['subscriptionId'])
            result['status'] = 'ok'
        elif op in ('surface.open', 'surface.close') and name == 'rchub.surface.lifecycle' and payload == {'surfaceId': 'controls'}:
            if op == 'surface.open' and 'controls' not in surfaces:
                surfaces['controls'] = 0
                open_surface, result['status'] = 'controls', 'ok'
            elif op == 'surface.close' and 'controls' in surfaces:
                del surfaces['controls']
                result['status'] = 'ok'
        send(result)
        if open_surface:
            state(open_surface)
        if changed:
            for subscription in sorted(subscriptions):
                send({'kind': 'event', 'subscriptionId': subscription, 'name': 'vendor.fixture.changed',
                      'payload': {'revision': revision}})
            for surface_id in sorted(surfaces):
                state(surface_id)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, TypeError, UnicodeError, RecursionError):
        sys.exit(2)
