# Public-safe asset provenance — local snapshot 2026-10-24

Only explicitly scoped plugin-development assets were copied from the reviewed
RcHub working tree. No private host implementation, history, account state,
credentials, native test reports or user-specific configuration was copied.
Everything here is MIT developer/reference material; the host application's
license and confidentiality are unchanged. This snapshot is included in the
explicitly authorized local developer-kit commit; it has no new release tag.
Push, tag, release and publication remain pending separate approval; none is claimed.

## Byte-matching source assets (same relative paths)

- `contracts/plugins/v2/application.schema.json`
- `contracts/plugins/v2/protocol.schema.json`
- `contracts/plugins/v2/transport.schema.json`
- `tools/plugins/rchub_app_v2.py`
- `tools/plugins/test_rchub_app_v2.py`
- `tools/plugins/fixtures/v2/vendor.fixture.json`
- `tools/plugins/fixtures/v2/protocol.transcript.json`
- `tools/plugins/fixtures/v2/negative.corpus.json`
- `tools/plugins/fixtures/v2/owner-context.corpus.json`
- `plugins/examples/application-v2/native.py`

Copy hashes were compared against source at sync time. Fixtures contain synthetic
tokens/loopback origins and example.invalid, never live authority. SDK test imports
the existing public V1 validator to test mutual rejection/12 unchanged bundles;
it does not require private host code. Public V1 SDK/schema/builder/packages were
deliberately not replaced by the newer private-tree select/Speech changes.

## Adaptations and new assets

- `native.rs` derives only from RcHub's explicitly harmless
  `scripts/plugin-app-e2e/native-peer.rs`, not host code. Removed test-secret
  environment assertion, PID and integration-store branches; negotiated ABI2
  check and caveat added. It remains a deliberately small canonical-host fixture
  parser, not a strict/general JSON library. No UI/network/custom filesystem work.
- `prepare.py` and `test_example.py` are new public-only example tooling. Descriptor
  template is the matching generic fixture, rewritten to a new ID and exact local
  binary digest; native storage choice is `none` disclosure, not isolation.
- ABI2 contract README is a current developer-facing rewrite of the canonical
  schemas/SDK/amendment, excluding private implementation coordination/history.
  Root README, EN/TR guides and llms index distinguish V1 support and ABI2 trust,
  version/installation/storage/lifetime boundaries and publication status.

Validation evidence concerns SDK/package and harmless subprocess checks only.
Host coordinator's separately reported Windows installed61 is explicitly synthetic
DB/injected transport/headless, not this checkout's live host or Linux acceptance.
No existing V1 official source/package/identity was rebuilt or modified.
