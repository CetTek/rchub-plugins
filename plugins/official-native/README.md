# Official native packages (Windows x64, 1.0.0)

Twelve MIT-licensed native applications built from the reviewed
`plugins/official/*/native-source.json` descriptors. Each directory is a complete
package root: `rchub-app.json` (canonical descriptor, `schemaVersion: 2`), the
hash-bound executable at `bin/rchub-official-native.exe`, the authoring
`native-source.json`, the package-owned `native-runtime/` Cargo sources and lockfile,
`LICENSE` and the package README.

| Directory | ID | Version | Artifact SHA-256 |
| --- | --- | --- | --- |
| `apps` | `com.rchub.apps` | 1.0.0 | `ec04c89844a79e25aff04655ddf6e681a3e6abd093afa82e3b0cc206e639c8f1` |
| `audio` | `com.rchub.audio` | 1.0.0 | `db6d24b13cfec4fb3555e110634d298f04dc65540398219606d039cadea5d49a` |
| `browser` | `com.rchub.browser` | 1.0.0 | `4a114f702da7ac1c994554739d8a25b3056d03b95d6fa311bd54f08fd7247810` |
| `clipboard` | `com.rchub.clipboard` | 1.0.0 | `2137e5fb94be53a6dc2baeaeede7c37b2b93113b0bcb57665aec87be3ce3234a` |
| `display` | `com.rchub.display` | 1.0.0 | `05f4590cd76206a95ce83b33cfc6127af528ed0ba4bf249df58de1556a42f60d` |
| `effects` | `com.rchub.effects` | 1.0.0 | `ad639893671e4f230f37e07919115a30bed503a54c4cfcbabb8dfff984b54b7a` |
| `keyboard` | `com.rchub.keyboard` | 1.0.0 | `bc326151fd2088cae2e2351372fdc42ac702744806e0572d9580f68b00858fa5` |
| `media` | `com.rchub.media` | 1.0.0 | `48af5ddb78f5a347f1330ce1cd91e820257896f7c834f99ef77294180dec50e6` |
| `power` | `com.rchub.power` | 1.0.0 | `680ba0972fa3ec08753abd9cdfe9475e00589af95740af28816adc5bfb37d0fe` |
| `screenshot` | `com.rchub.screenshot` | 1.0.0 | `39f1bea777bb5f2051dd701ed146a79dc7236da3d19ceb3a7c70e0b1330b8b31` |
| `speech` | `com.rchub.speech` | 1.0.0 | `4de8591d6a86b92614b35339fe2d9173109f1aa93286dc7e1ea5f4d50556afe3` |
| `system` | `com.rchub.system` | 1.0.0 | `7fc767a4cc8fccd1c50f63f4510c295f1d1b34bcba3b1caeb3c910e8de465585` |

Every package declares target `windows-x86_64`, entry
`bin/rchub-official-native.exe` and permission `native.unsandboxed`. The artifact
digests above match the committed executable bytes; the descriptor identity is the
SHA-256 of its canonical bytes and is shown at installation.

## How these packages are served

The compatible RcHub host reads this directory layout from its own checkout
(`plugins/official-native/<name>/rchub-app.json`) and serves the packages through
`GET /api/plugins/v2/catalog` and the hash-bound
`GET /api/plugins/v2/artifacts/<id>/<version>/<sha256>` route. Both routes require
authorization: an account session Bearer token (mobile/web) or the paired-device
`x-device-id`/`x-device-password` headers (desktop). Requests without valid
authority are rejected, and the desktop keeps its catalog action disabled until it
can send its paired-device credentials.

This repository is a distribution catalog, not a per-package repository root. The
GitHub-URL resolver requires `rchub-app.json` at a repository root, so a monorepo
subdirectory URL is not accepted; install from the catalog above or publish your own
single-package repository as described in the [developer guide](../../docs/plugins/README.md).

Native code runs unsandboxed as the normal OS user. Published bytes, hashes and
reviewable sources do not by themselves make the code safe or its publisher
verified, and no V1 Wasm grant transfers to these packages. Installation and every
update still require explicit consent plus current owner/device authorization.
Linux packages are not published here.
