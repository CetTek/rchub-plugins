# Plugin platform v1 implementation contract

Development contract for declarative RCHub plugins. Production untrusted execution is not enabled by publishing these examples. MIT applies to this standalone SDK and plugin repository; the host application is separate.

## Portable package

Distributable `.rchub-plugin.json`: UTF-8 JSON envelope `{formatVersion:1,
manifest:Manifest,wasm:number[]}`. No paths/URLs/hooks/archives/dependencies are
executed. Limit full package to 131072 bytes; Wasm <=16384 bytes. Identity is
`{id,version,sha256}` where sha256 is lowercase digest of exact file bytes.
Strict objects reject unknown keys. Imports validated separately. Source/license
metadata is disclosure, not verified provenance. Community file installation does
not require a catalog entry. No arbitrary server-side URL fetching.

Manifest fields (all required unless marked optional):

```
schemaVersion:1
id: reverse-domain lowercase [a-z][a-z0-9]*(.[a-z][a-z0-9]*){2,8}, max120
version: numeric major.minor.patch, max32
name,description: {en:string,tr:string}, name max80/description max500
publisher: {name:string(max80), kind:"official"|"community"}
license: string(max40), source: HTTPS source-code URL(max300)
targets: nonempty unique array of "windows"|"linux"
hostApi:1
permissions: unique sorted string array(max16), known capability IDs only
actions: array(max32) of {id:lowercase identifier,entry:int0..31,
  capability:string, input:InputSchema, output:OutputSchema}
state: object field -> FieldSchema (max32 fields)
ui: {version:1,icon:enum name,color:theme color token,controls:Control[]}
```

`InputSchema` / `OutputSchema`: `{type:"object",properties:{field:FieldSchema},
required:string[]}` (all keys strict, additional arguments rejected).
FieldSchema: `{type:"integer"|"number"|"boolean"|"string", minimum?:number,
maximum?:number,maxLength?:int,enum?:scalar[]}`; numeric integer bounds required on
numbers, maxLength on strings <=32768 (total input still <=8192 bytes, output
<=32768). Also `{type:"array",maxItems:int1..64,items:FieldSchema|ObjectSchema}`
and the same strict ObjectSchema form recursively, max schema nesting3. This
supports actual typed lists; do not encode/parse newline or JSON strings as lists.
Output/state objects use declared fields only.
Read-only action outputs merge validated declared state fields; writes also return
observed state. No expressions/eval, prototype paths or arbitrary templates.
IDs are `[a-z][a-z0-9_]{0,39}`; action entries/IDs and control IDs are unique.

Control: discriminated `type` in section,row,column,button,slider,toggle,select,
text,value,status,input,list,image,progress,tabs. Common required id,type;
optional label:{en,tr},icon, tone; children only layout controls; max depth6,
128 total controls. Bindings are direct declared state/input field names.
Leaf options: action?:actionId, bind?:stateField, argument?:inputField,
value?:scalar (static action argument), min?/max?/step?:number,
options?:[{label:{en,tr},value:scalar}], placeholder?:{en,tr},
itemLabel?:fieldName,itemValue?:fieldName (list row fields only).
Exact action/field/type compatibility must be validated, no unbounded values or
unknown control types. Host owns confirmation/permissions/about/remove controls.
Slider requires explicit min, max and positive step; progress requires min/max.
There are no implicit bounds. Reserved constructor/prototype identifiers are
rejected. Localized/string limits count UTF-16 code units across clients.
Initial implementation must support Audio button/slider/toggle/value/section/row;
other controls have real native renderers only when their schema/output is supported.
Do not show a control that silently does nothing.

## Runtime ABI: rchub-json-v1

Wasm exports `memory` (2 pages), `run:(i32 entry,i32 inputLen)->i32 outputLen`.
Host writes strict action input JSON into offset0 (<=8192 bytes); guest writes
UTF-8 JSON object output at32768 (<=32768 bytes) and returns length, negative=fail.
Allowed sole host import `rchub.invoke:(i32 capabilityPtr,i32 capabilityLen,
i32 inputPtr,i32 inputLen,i32 outputPtr,i32 outputCapacity)->i32 outputLen`.
Import payload is typed native capability input JSON; capability name is read
from bounded guest memory, must exist in manifest AND installed grant. Every
native read/effect checks immutable installed package and current operation both
before and after synchronous validation. No WASI/start/other imports. Existing
fuel/memory/worker bounds retained; fresh store per invocation. Validation must
never execute native calls. Never call host provider directly instead of guest.

Initial working native capabilities: audio.read `{}` -> {volume:int0..100,muted:bool},
audio.write `{volume?:int0..100,muted?:bool}` at least one -> observed same state.
Existing typed Core Audio provider reused. Other native capabilities added only
with exact validated input/output/permissions and real implementations. A package
with unsupported capabilities/platform is explicitly unavailable, never successful.
Official reference Audio actions: read(entry0,audio.read,{}), set_volume(entry1,
audio.write,{volume}), set_mute(entry2,audio.write,{muted}). Permissions sorted
audio.read/audio.write. State volume/muted. No automatic write/replay on open.

Current additional native payloads (Windows; no legacy command passthrough):

| Capability | Input | Observed output |
| --- | --- | --- |
| system.read | `{}` | os/hostname strings<=120; cpu_usage0..100; ram_used_mb,ram_total_mb,uptime_seconds0..2147483647 |
| browser.open | `{url:string<=2048}` HTTP(S), host required, no credentials | `{opened:boolean}` |
| power.write | `{action:"lock"|"sleep"|"shutdown"|"restart"}` | `{accepted:boolean}` |
| keyboard.write | Exactly one of `{text:string<=4096}` / `{keys:string<=80}` constrained modifier/key grammar | `{sent:boolean}` |
| media.write | `{action:"play_pause"|"next"|"previous"}` | `{accepted:boolean}` |
| clipboard.read/write | `{}` / `{text:string<=8192}` Unicode text | `{text:string<=8192}` |
| apps.read | `{}` | `{apps:[{pid:int1..2147483647,name:string<=120}],count:int0..64}` bounded64 |
| apps.write | `{pid:int1..2147483647}` | `{accepted:boolean}` terminate only, no launch |
| display.read/write | `{}` / `{brightness:int0..100}` | `{brightness:int0..100,available:boolean}` primary physical DDC/CI monitor; unsupported read available:false, write fails |
| screenshot.read | `{}` | `{image:string<=30000,width:int1..4096,height:int1..4096}` primary PNG thumbnail<=320x180, adaptive downscale |
| speech.write | `{text:string<=2000}` literal non-XML | `{accepted:boolean}` local SAPI completed within10s; cancels on timeout/revocation |
| effects.write | `{effect:"beep"}` | `{accepted:boolean}` MessageBeep only |

Native hardware/effects were not exercised by automated tests. Compile and fake
provider coverage does not establish live device compatibility or full legacy
feature parity. Only system information has a non-Windows provider currently.

## Wire v2 on existing plugin-dev event channels

Retain Socket.IO event names/adapters to reuse current auth lifecycle, canonical
epoch, ingress and ONE existing network-owned worker. New UI only uses v2; v1
samples remain test compatibility, hidden from product UI. Events:
plugin-dev:request / local-request / dispatch / result / local-result.
Existing readiness/context stay v1. Do not create a second runtime/store worker.

Requester body `{v:2,requestId:UUID,deviceId,action,...}`. Actions:
- inventory: no other fields; observe all v2 installed records (max64).
- install: `package:{id,version,sha256}`, `bundle:<envelope>`,
  `permissions:<exact sorted manifest tuple>`,
  `consent:{disclosureVersion:1,accepted:true}`. Verify digest against canonical
  package bytes: recursively sort object keys lexicographically, retain array
  order, compact JSON with literal UTF-8 non-ASCII, no extra spaces, finite
  numbers only, followed by a SINGLE LF. SDK uses this exact encoding. Use integer
  numeric bounds/steps in v1 to avoid cross-runtime float serialization ambiguity.
  Bundle may be sourced from catalog or imported local file.
- remove: exact package identity; only matching owned validated installed record.
- invoke: exact package identity, `actionId`, `input:<typed JSON object>`.
No URLs, filesystem paths, owner claims, functions or undeclared inputs.
Dispatch replaces requestId/deviceId with operationId,epoch,deadline,binding
as in v1, preserving validated fields; binding derived by server. Global and
per-device/requester reservations/dedup/expiry shared across v1/v2. 196608-byte
bounded messages for v2; existing v1 limits remain unchanged.

v2 DeviceResult correlation `{v:2,operationId,epoch,action,status,...}`;
requester terminal substitutes requestId/deviceId. Status enum same as v1.
Successful inventory: `entries:InstalledEntry[]`.
Install/remove success: `entries:InstalledEntry[]` observed AFTER operation.
Invoke success: `packageId,output:<validated JSON object>`.
Failed: `error:invalid_package|grant_invalid|unsupported|runtime_failed|conflict`,
optional entries only install/remove. Unknown/denied/busy/invalid: no observed
output/entries/error. Non-invoke carries no output; invoke no entries.
InstalledEntry `{package:{id,version,sha256},manifest,availability:"ready"|
"runtime_unavailable"|"unsupported",status:"installed"}`. Corrupt/foreign records
are preserved, not exposed as installed. Inventory should include safe issue
counts if needed (not raw paths). Observe current installed grant before invoke;
server verifies output against authenticated pending manifest/action schema.

New store namespace `plugins-v1`, one record per ID includes full immutable
bundle+release identity+exact permissions+consent+binding. No overwrite/update in
first installer: report conflict; remove + fresh install for new release. This
ensures permission expansion cannot inherit old approval. No old-grant migration.

## Catalog and UI

GET /api/plugins/catalog?search=&category=&cursor= -> {success:true,data:{items:
CatalogEntry[],nextCursor:null|string}} with limit<=24. Entry `{package,manifest,
category:"media"|"system"|"tools",downloadPath:"/api/plugins/packages/<id>/<version>"}`.
GET exact downloadPath -> envelope application/json with canonical LF digest.
Serve only repository package artifacts from bounded local catalog (no fetch).
Source metadata is not publisher verification. No fabricated downloads/reviews.
DEV-only registration consistent with existing explicit NODE_ENV=development.
Routes serve metadata only; lifecycle/execution still requires socket owner auth.
Mobile fetch via current configured api base; don't change targets/endpoints.

Device screen: installed plugin cards only (existing layout/theme), management
entry and empty-state CTA. Remove fixed legacy card list, eager modal warmups and
PluginDevPanel from product screen; preserve legacy source until safe reference
cleanup. Management: Installed/Marketplace, detail/source/license/permissions,
install/remove confirmations; errors/offline/unsupported shown honestly. Dynamic
panel renders manifest controls with shared themed modal/native components.
SDK: create/validate/package/inspect/preview, standalone local fixture mode with
no account/backend needed; docs EN/TR examples, UI reference and capability matrix.
Official artifacts source and MIT license in plugins/official. Full old controls
migrate as independently implemented providers/packages; do not relabel the
legacy generic command channel as granted plugin execution.
