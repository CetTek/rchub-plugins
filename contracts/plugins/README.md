# Plugin platform v1 implementation contract

Approved plan: user requested a plugin-owned card/panel system, management,
Marketplace and public developer docs, then approved the parallel plan on
2026-10-20. This is the development implementation of that plan; production
remote untrusted execution remains a separate release-readiness gate. No new
per-launch env flags, DB migration or account/auth storage. Preserve old pilot
records without auto-grant/migration. Official new packages use MIT; the host
application's license is unchanged. The original implementation task did not
authorize publication. On 2026-10-21 the user separately authorized publishing the
standalone MIT plugins/SDK/guides under CetTek/rchub-plugins; this does not publish
the private application or enable production execution.

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
id: reverse-domain lowercase [a-z][a-z0-9]*(.[a-z][a-z0-9]*){2,8}, max 120
version: numeric major.minor.patch, max 32
name,description: {en:string,tr:string}, name max 80/description max 500
publisher: {name:string(max 80), kind:"official"|"community"}
license: string(max 40), source: HTTPS source-code URL(max 300)
targets: nonempty unique array of "windows"|"linux"
hostApi:1
permissions: unique sorted string array(max 16), known capability IDs only
actions: array(max 32) of {id:lowercase identifier,entry:int 0..31,
  capability:string, input:InputSchema, output:OutputSchema}
state: object field -> FieldSchema (max 32 fields)
ui: {version:1,icon:enum name,color:theme color token,controls:Control[]}
```

`InputSchema` / `OutputSchema`: `{type:"object",properties:{field:FieldSchema},
required:string[]}` (all keys strict, additional arguments rejected).
FieldSchema: `{type:"integer"|"number"|"boolean"|"string", minimum?:number,
maximum?:number,maxLength?:int,enum?:scalar[]}`; numeric integer bounds required on
numbers, maxLength on strings <=800000 (total input <=65536 bytes on new 5/16-page guests; legacy 2-page guests <=8192 bytes, output
<=900000). Also `{type:"array",maxItems:int 1..64,items:FieldSchema|ObjectSchema}`
and the same strict ObjectSchema form recursively, max schema nesting 3. This
supports actual typed lists; do not encode/parse newline or JSON strings as lists.
Output/state objects use declared fields only.
Read-only action outputs merge validated declared state fields; writes also return
observed state. No expressions/eval, prototype paths or arbitrary templates.
IDs are `[a-z][a-z0-9_]{0,39}`; action entries/IDs and control IDs are unique.

Control: discriminated `type` in section,row,column,button,slider,toggle,select,
text,value,status,input,list,image,progress,tabs. Common required id,type;
optional label:{en,tr},icon, tone; children only layout controls; max depth 6,
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

Wasm exports `memory` (legacy 2 pages; new proxies use 5 pages, or 16 pages for declared large
outputs), `run:(i32 entry,i32 inputLen)->i32 outputLen`.
Host writes strict action input JSON into offset 0 (<=65536 bytes on new guests; <=8192 on legacy 2-page guests); guest writes
UTF-8 JSON object output at 32768 (<=900000 bytes) and returns length, negative=fail.
Allowed sole host import `rchub.invoke:(i32 capabilityPtr,i32 capabilityLen,
i32 inputPtr,i32 inputLen,i32 outputPtr,i32 outputCapacity)->i32 outputLen`.
Import payload is typed native capability input JSON; capability name is read
from bounded guest memory, must exist in manifest AND installed grant. Every
native read/effect checks immutable installed package and current operation both
before and after synchronous validation. No WASI/start/other imports. Existing
fuel/memory/worker bounds retained; fresh store per invocation. Validation must
never execute native calls. Never call host provider directly instead of guest.

Initial working native capabilities: audio.read `{}` -> {volume:int 0..100,muted:bool},
audio.write `{volume?:int 0..100,muted?:bool}` at least one -> observed same state.
Existing typed Core Audio provider reused. Other native capabilities added only
with exact validated input/output/permissions and real implementations. A package
with unsupported capabilities/platform is explicitly unavailable, never successful.
Official reference Audio actions: read(entry 0,audio.read,{}), set_volume(entry 1,
audio.write,{volume}), set_mute(entry 2,audio.write,{muted}). Permissions sorted
audio.read/audio.write. State volume/muted. No automatic write/replay on open.

Current additional native payloads (Windows; no legacy command passthrough):

| Capability | Input | Observed output |
| --- | --- | --- |
| system.read | `{}` | os/hostname strings<=120; cpu_usage 0..100; ram_usage_percent 0..100; ram_used_mb,ram_total_mb,uptime_seconds 0..2147483647 |
| browser.open | `{url:string<=2048}` HTTP(S), host required, no credentials | `{opened:boolean}` |
| power.write | `{action:"lock"|"sleep"|"shutdown"|"restart"|"cancel_shutdown",delay_seconds?:int 0..86400,force?:boolean}`; options only shutdown/restart | `{accepted:boolean}` |
| keyboard.write | Exactly one of `{text:string<=4096}` / `{keys:string<=80}` constrained modifier/key grammar | `{sent:boolean}` |
| media.write | `{action:"play_pause"|"next"|"previous"}` | `{accepted:boolean}` |
| clipboard.read/write | `{}` / `{text:string<=8192}` Unicode text | `{text:string<=8192}` |
| apps.read | `{filter?:"foreground"|"all"}`; default foreground = visible titled application windows | `{apps:[{pid:int 1..2147483647,name:string<=120,title:string<=240,window_id:string<=32,started_at:int 0..9007199254740991,foreground:boolean}],count:int 0..64}` |
| apps.write | `{pid:int 1..2147483647,started_at:int 0..9007199254740991,action:"stop"|"close"|"bring_to_front"|"minimize"|"maximize"|"restore",window_id?:string<=32}`; window actions require window_id | `{accepted:boolean}`; validates process creation time/window ownership, no launch |
| display.read/write | `{}` / `{brightness:int 0..100}` | `{brightness:int 0..100,available:boolean}` primary physical DDC/CI monitor; unsupported read available:false, write fails |
| screenshot.read | `{}` | `{image:string<=800000,width:int 1..4096,height:int 1..4096}` optimized primary JPEG<=1920x1080 at quality 80, adaptive downscale; client tap opens full-screen |
| speech.write | `{text:string<=2000}` literal non-XML | `{accepted:boolean}` local SAPI completed within 10s; cancels on timeout/revocation |
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
- inventory: no other fields; observe all v2 installed records (max 64).
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

## Declarative controls added with official packages 0.3.0

Schema/UI/hostApi remain version 1. These optional fields extend the strict contract;
older hosts reject unknown fields and must be upgraded before installing these packages.
Existing installations keep their exact manifests/grants; remove and explicitly reinstall.

- `progress.appearance`: `linear` (default) or `ring`, using the same required state/min/max.
- `input.multiline`: boolean for string arguments; `inputMode`: text/url for strings or numeric for integers.
- `input.submit:false`: draft-only input with a declared action/argument schema. Integers and booleans are draft-only; booleans render as checkboxes. `defaultValue` must satisfy that argument schema.
- `inputBindings:{argument:controlId}`: merge typed draft values into an action alongside its static argument/value. Source input's action-argument schema must equal the destination schema, independent of object key order. Missing/invalid values fail closed; bindings cannot replace the primary argument.
- `confirm:{title:{en,tr},message:{en,tr}}`: application confirmation for potentially destructive action, separate from installation permission consent.
- `list.searchable`: filter observed rows locally. `itemActions` (1..8) exposes selected-row actions instead of immediately invoking on row selection. Each action has localized label, action, optional argument/value, `args:{argument:scalar}`, `itemArguments:{argument:rowField}`, icon/tone/confirm. All required arguments and schema compatibility are checked. `itemValue` supplies the primary row argument unless static value is supplied.

New generated proxies place capability strings at 65536, outside the bounded input;
output remains at 32768 with 262144-byte capacity (5 pages), or 900000 (16 pages).
The single synchronous import reads and validates the complete input before writing
output, so input/output region overlap is intentional. Escaped clipboard/process
JSON must fit serialized bounds as well as per-field UTF-16 limits.

Power shutdown/restart uses Windows `InitiateSystemShutdownExW` with separate delay
and force parameters, so a delay never silently implies force. Cancel uses
`AbortSystemShutdownW`; lock/sleep/cancel reject scheduling options. Native writes
are not exercised by automated contract tests. Apps stops require the observed
process creation timestamp; window actions also validate the observed handle's PID.
