# Experimental native applications — Integration ABI revision 2

Local developer snapshot, 2026-10-24. **Not a production release, a public stability
promise or evidence that a deployed host supports this protocol.** This checkout
was prepared locally; its developer-kit commit was explicitly authorized. Push,
tag, release and publication remain pending separate approval and were not performed.

V1 Wasm remains supported. No mandatory migration, package rebuild, ID change,
grant transfer or hash rewrite. Native application descriptors are a separate
format, not `.rchub-plugin.json` bundles and not portable package wire v2.

## Canonical assets and validation

- [application.schema.json](application.schema.json): descriptor `schemaVersion:2`.
- [protocol.schema.json](protocol.schema.json): process protocol
  `rchub-app-v2-experimental`; hello/welcome require exact `version:2` for ABI2.
- [transport.schema.json](transport.schema.json): authenticated Socket.IO wire
  `v:3`, with **`version:2` on every ABI2 envelope**, including results/events.
- [Python SDK](../../../tools/plugins/rchub_app_v2.py): structural **and semantic**
  checks. Schema validation alone is insufficient.
- [SDK tests](../../../tools/plugins/test_rchub_app_v2.py), shared
  [descriptor](../../../tools/plugins/fixtures/v2/vendor.fixture.json),
  [owner/watch corpus](../../../tools/plugins/fixtures/v2/owner-context.corpus.json),
  [negative corpus](../../../tools/plugins/fixtures/v2/negative.corpus.json).
- [Buildable Windows example and EN/TR instructions](../../../plugins/examples/application-v2/README.md).

From repository root (Python 3.9+; stdlib only):

```powershell
python -B -m unittest discover -s tools/plugins -p "test_rchub_app_v2.py" -v
python -B -m unittest discover -s tools/plugins -p "test_rchub_plugin.py" -v
python -B -m unittest discover -s plugins/examples/application-v2 -p "test_example.py" -v
```

The last suite compiles/runs only the harmless example when installed `rustc`
exists; no dependencies downloaded. These checks are not live host acceptance.

SDK is an importable module, **not an install/build CLI**. Add `tools/plugins` to
Python's import path. Use `validate_manifest`, `identity`, `verify_artifact`,
`encode_frame`, `FrameDecoder`, `ProtocolSession(...,version=2)` and
`validate_transport(message,expected_version=2)`. `encode_transport` and
`decode_transport` validate structure but do not pin a connection revision;
explicitly validate the negotiated version too. `validate_bound_transport` adds
manifest/instance/context/payload checks, not auth, pending/watch correlation or
revision pinning. Adapters must enforce those independently.

`MockSession`, default `ProtocolSession(version=1)` and
[protocol.transcript.json](../../../tools/plugins/fixtures/v2/protocol.transcript.json)
remain historical ABI1 compatibility tests. Never copy their unnegotiated mock
start/256-ID cap or use revision1 as an execution downgrade. The new native
example and its test explicitly negotiate 2.

## Execution and trust

Native author code can implement new PC behavior without adding host capability
providers. It is a **long-lived normal-user UNSANDBOXED process**, not Wasm/WASI.
It can access files/network/OS resources available to that user outside broker
calls. Process pipes, hashes, source URLs, quotas and `native.unsandboxed` do not
make arbitrary code safe or certify a publisher. This is not admin/root elevation.
Hosts reject unsupported formats/platforms; no guarantee every binary will run.

`execution:{kind:"installed-native",trust:"unsandboxed-user",artifact:{target,entry,sha256}}`
describes exactly one raw 1-byte–16-MiB executable. Targets are
`windows-x86_64` / `linux-x86_64`; Linux acceptance is deferred. No archive,
interpreter, shell, arguments, dependency resolver, install hook, arbitrary launch
path or URL download exists in this contract. Relative entry is 1..160 ASCII
characters; absolute/drive/UNC/traversal/backslash/ADS/empty segments/trailing dots/
Windows device names reject. Windows host checks x64 PE, Linux code checks x64 ELF;
structural format checks are not code safety or Linux runtime acceptance.

`verify_artifact` verifies supplied bytes/target/entry/digest, not executable format,
disk containment/links/TOCTOU/atomic installation. Those remain host boundaries.
The example's `.py` peer is not an installable artifact. Do not rename scripts to
`.exe`, supply `python.exe` as an interpreter or substitute an arbitrary executable
that does not speak the protocol. stdout is framed IPC only; diagnostics use stderr.

## Descriptor, identity and consent

Closed descriptor requires localized `name`, `description`, HTTPS `source`,
`license`, `permissions:["native.unsandboxed"]`, localized
`disclosure:{nativeRisk,dataBehavior}`, `requiredFeatures`, `execution`, `actions`,
`events`, `state`, `surfaces`, `storage`, `id`, `version`, `schemaVersion:2`.
`source` is a disclosure claim, not fetched or verified. Replace example.invalid
with real reviewable source before distribution; no automatic publication occurs.

ID is a lowercase dotted namespace (2..9 segments, <=120 chars), version numeric
major.minor.patch. Actions 1..32 and events 0..32 use fully qualified names under
`id + '.'`, <=160 chars. No official-ID allowlist or host provider mapping.
Action input/output, event payload and state use strict bounded object schemas:
boolean, safe integer, string <=4096 Unicode scalars, objects <=32 fields, arrays
<=64 items; schema depth <=3 edges including arrays. Undeclared fields, floats,
null payload fields and arbitrary JSON Schema/extensions reject.

Identity `{id,version,sha256}` hashes canonical **descriptor** bytes, transitively
including artifact SHA-256, not a V1 bundle hash. Canonical UTF-8 sorts keys,
retains array order, uses compact integer-only JSON and one final LF.

Installation and every same-ID native update require fresh explicit target review:

```json
{"disclosureVersion":2,"accepted":true,"trust":"unsandboxed-user","permissions":["native.unsandboxed"]}
```

Host adds non-overridable unsandboxed/source/window ownership warnings. Review
source/risk, requested permission and data behavior on phone or PC; no physical
PC approval required. Consent binds exact manifest/identity/current owner/device.
V1 consent never transfers. New native version must be strictly newer with exact
`expectedPackage` compare-and-swap; failure preserves previous install/grant.
Within installed grants, use/reconnect/restart needs no per-operation/session
consent popup. **That never bypasses current authentication/authorization.**

## Management and authority

Wire uses existing authenticated DEV request/dispatch/result channels; observations
use `plugin-dev:event` / `plugin-dev:local-event`. One JSON envelope <=196608
canonical UTF-8 bytes, **no IPC length prefix**. Capabilities explicitly advertise
`{v:3,kind:"capabilities",epoch,protocol:"rchub-app-v2-experimental",version:2,features:[...]}`.
Compatibility advert is not permission. Unsupported/missing/mixed revision fails closed.
ABI2 hosts additionally advertise the host-only capability `owner-revocation-ack-v1`
(not a manifest feature); native readiness and dispatch require it.

Client requests own `requestId,deviceId,action`; server owns
`operationId,epoch,deadline,binding:{serverOrigin,deviceId,ownerUserId}` and
`authorityRevision` on dispatch. Never let callers mint owner controls/bindings.
Phone/web require current JWT/session plus owner checks; a separate self-only PC
viewer uses server-validated canonical current-device/current-owner binding, no
fabricated user JWT/session, new token, scheduler or general remote bypass.
Every management/action/observation checks exact installed grant/hash, current
owner/link/device, canonical socket/epoch, authority revision and runtime context.

Management order (compatible DEV UI performs it, not a Python install command):

1. `inventory` uses pages <=2 descriptors, host-owned snapshot cursor; refresh
   after unknown/stale results. Current host capacity is 64 entries.
2. `install.begin {package,manifest,size,consent,expectedPackage?}` -> `transferId`.
3. `install.chunk {package,transferId,offset,data}` -> exact next offset. Padded
   canonical standard base64, decoded 1..32768 bytes; no gaps/replay/overrun.
4. `install.commit {package,transferId}` verifies full size/digest/target/current
   authority/grant/CAS/contained stage and commits; failed install never activates.
   `install.abort` discards only unfinished owned stage.
5. Explicit `activate {package}` -> fresh `instanceId,contextId`. Install itself
   runs no native code. Update closes old instance only after verified commit;
   explicit activation required again. No automatic start after reconnect.
6. `invoke`, `subscribe`, `unsubscribe`, `surface.open/close` bind exact active
   package/instance/context. `deactivate` explicitly stops the shared instance;
   closing one panel is not deactivation. `remove` removes owned install/artifact,
   not custom native files or retained scoped data.

Server-only `plugin-dev:owner-context` carries exact epoch/binding, increasing
`authorityRevision`, valid/revoked state/reason. It is trusted-channel data, not a
bearer credential. Normal dispatch cannot establish authority. Owner/device/link
revocation or canonical disconnect invalidates owned runtime/stages. Viewer
logout/session expiry/disconnect closes only its own pending work/watches, not
another viewer or shared process. No 30-second lease or viewer-lifetime process.
Current same-process owner-mutation coverage does not imply immediate detection
of direct SQL/admin/other-process changes; no new polling policy is promised.

### Revocation proof: PC -> server

After each accepted owner-context revision the PC stops every bound instance and
returns a dedicated Socket.IO **`plugin-dev:revocation-ack`** message:

```json
{"v":3,"kind":"revocation-ack","version":2,"deviceId":"device1",
 "epoch":"epoch1","authorityRevision":2,"stopped":true,
 "instances":["instance1"]}
```

`stopped` is true only when termination was verified (the owned child was reaped),
never merely because a kill/job request was issued. `instances` lists the verified
stopped instance IDs; a failed stop reports `stopped:false` and no success IDs.
The server records an ack only for the current canonical device/epoch and exact
current `authorityRevision`; stale, duplicate or foreign acks are rejected and each
new revision clears the previous proof. Dispatch, readiness and watches require
`stopped:true` for the exact current revision; a missing, late or failed ack fails
closed, and `stopped:false` also drops derived watches/observations. Canonical
transport loss still invalidates local authority. `stopped`/`instances` are additive
fields on the existing envelope, so a peer that never negotiates
`owner-revocation-ack-v1` is unsupported for this owner-scoped path rather than
silently trusted.

ABI2 surface.close caller supplies surface selector, never `targetOperationId`.
Server resolves **that requester's exact successful open operation** and includes
its ID in close dispatch. Desktop reference-counts watches over one logical IPC
surface. Each watch gets contiguous revisions starting at 1, including a later
viewer from cached full state. No cached initial snapshot -> busy/no watch. Last
close sends one IPC close, quarantines until ack; it does not stop other surfaces.

## Process protocol and surfaces

Envelope `{protocol,instanceId,contextId,sender,body}` uses unsigned 4-byte big-endian
length + UTF-8 JSON, body 1..65536 bytes. Strict Unicode/integer JSON depth <=32;
duplicate/reserved keys, malformed UTF-8, bad length, truncated EOF poison stream.
Actual pipe ownership must agree with sender; sender text is not authentication.
Host hello version2 -> plugin welcome version2 (required subset of offered
features) -> plugin started -> work -> host shutdown -> plugin stopped/owned reap.

Request IDs are per-sender monotonically increasing decimal strings
1..9007199254740991, no leading zero/reuse; <=64 pending and subscriptions including
pending. No 256-operation lifetime cap. Results match exact pending ID/op/name and
declared output. Subscribe ack precedes events; unsubscribe ack ends subscription.
Cancel is cooperative, one terminal result; completed effects may win. Unknown/
timeout/disconnect is never permission to replay or claim rollback.

Up to eight surfaces. Declarative clients `pc,mobile,web` share ordered bounded
controls: section,row,column,text,value,status,button,toggle,slider,input,select,
list,progress. <=128 unique controls per surface, depth<=6. Direct `bind` state
and action `argument`, no expressions/JS/HTML/templates/dynamic components. Not
the whole V1 vocabulary: no tabs/image/itemActions/icon/tone in this revision.
`submit:false` input/select drafts plus typed defaultValue/inputBindings dispatch
only on explicit action. Sliders commit on release, not every drag update.
Lists carry typed row values. Absent observation is unknown, never fake zero/false.

Host requests `surface.open/close` with name `rchub.surface.lifecycle`,
payload `{surfaceId}`, ack `{}`. After successful open, plugin sends full
`state {surfaceId,revision:1,values}`; revisions increase exactly one. Full schema
validated before replacement; no gaps/replay/partial merge or late closed state.
No automatic action/history/retention policy. Events are live observations.

`extension-window {id,kind,action,trust:"extension-owned-not-host-ui"}` delegates a
declared action to the extension. Extension owns actual native window/title/lifetime/
data; never impersonate host consent/auth UI. Example deliberately rejects window
action; no real window or visible-click proof supplied.

## Storage: developer choice, not isolation

`storage.choices`: `native-filesystem`, optional `host-scoped`, or exclusive `none`.
These are disclosure, not OS permissions. Native temporary/custom files are author
owned and outside host cleanup/backup guarantees. Even `none` cannot prevent writes.
No arbitraryFS/WASI permission added to V1.

Host-scoped requires choice plus negotiated `scoped-store-v1`. Plugin-only service
`rchub.store.value`: `store.get {key}` -> `{found:false,revision:0}` or
`{found:true,revision:N,value}`; `store.put {key,expectedRevision:N,value}` ->
`{revision:N+1}` or conflict. Opaque key <=64 chars, no paths/reserved names; strict
object value <=8192 canonical bytes, <=128 keys, <=262144 total value bytes/scope.
No list/delete/history API; CAS, no implicit merge. Host derives scope
`(serverOrigin,deviceId,currentOwnerUserId,pluginId)`, never caller. Removal retains
values inaccessible without exact authorized installation/scope; approved same-scope
reinstall/same-ID update may regain them. Never rebind across owners or silently purge.
SDK `ScopedStore` is in-memory reference, not durable storage proof.

## Evidence and limits

Host coordinator independently reported 61 current Windows-isolated installed
checks using a real native child, RustLocal and headless rendering: staged install,
two viewers, exact own-close, revoke/owned reap and private store restore.
**Synthetic DB facts/injected transport/headless frames**, not live production:
`bRustLocalVerified:true`, `headlessRendererVerified:true`,
`socketIoLoopbackVerified:false`, `pcConnectionLocalUiVerified:false`,
`liveDbAuthVerified:false`, `uiVerified:false`, `productComplete:false`.
This public asset test suite is separate evidence, not a rerun of those host tests.
Visible PC/phone UI, live DB/JWT/real socket path, Linux acceptance, full runtime/
security review and production activation remain unverified. Local documentation
sync is not publication and no new package release was made.
