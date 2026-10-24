# Native ABI2 example — English / Türkçe

**Experimental local DEV assets, not a production plugin or published release.**
New ID `com.example.switchboard` is illustrative, not a reserved host ID or chat
product. V1 official packages/identities remain unchanged. All code here is MIT.

## English: build and validate (Windows x64)

Prerequisites: installed Python 3.9+ and `rustc` plus its working Windows linker.
No Cargo crates, pip modules, downloads, accounts or host private sources needed.
From this repository root, in a fresh example build directory:

```powershell
New-Item -ItemType Directory -Force plugins/examples/application-v2/build | Out-Null
rustc --edition=2021 plugins/examples/application-v2/native.rs -o plugins/examples/application-v2/build/switchboard.exe
python -B plugins/examples/application-v2/prepare.py --artifact plugins/examples/application-v2/build/switchboard.exe --out plugins/examples/application-v2/build/import
python -B -m unittest discover -s plugins/examples/application-v2 -p "test_example.py" -v
python -B -m unittest discover -s tools/plugins -p "test_rchub_app_v2.py" -v
```

`prepare.py` is a local example helper, **not SDK install CLI**. It reads bounded
artifact bytes, sets SHA-256 and a new namespace throughout the canonical fixture,
validates descriptor/integrity, and writes `manifest.json`, `identity.json` and
`artifact.base64.txt`. It refuses existing outputs; use a fresh `--out` directory
on another run. `--id org.example.different` exercises a different namespace;
neither peer nor host needs a plugin-ID special branch. Windows x64 only in this
helper. Linux build/runtime acceptance intentionally deferred, not implied by the
schema's linux-x86_64 target.

Generated source URL is `https://example.invalid/switchboard/source`: placeholder,
not provenance or a fetch endpoint. Replace with actual reviewable source and
update EN/TR disclosure before distributing, then recompute identity with the SDK.
No generated binary or imported package is committed/released by this workflow.

### Import into a compatible ABI2 DEV host

There is **no public hosted installer or `rchub_app_v2.py install` command**.
The public checkout contains developer assets, not the host app. With a separately
provided compatible authenticated debug desktop (built in its own `desktop`
directory using `cargo run --locked --offline --features plugin-dev-runtime`),
use the **native application** import workspace, not the V1 package importer:

1. Paste generated `manifest.json` and the complete `artifact.base64.txt` into
   its descriptor/base64 fields. The native mobile/web workspace has the same
   two-input review path; compatible backend/current owner connection required.
2. Preview/review host unsandboxed warning, source/license, exact identity/hash,
   target, `native.unsandboxed` and data behavior; approve explicitly on PC/phone.
3. UI stages begin/chunks/commit with exact ack offsets. Observe inventory installed
   status. Installation does not run code. Explicitly Activate, then open Controls.
4. Toggle Enabled to change **in-memory example state only**; watch revisions/events.
   The extension-window action rejects intentionally; no native UI is created.
5. Close only your own surface. Another viewer/shared instance may continue.
   Deactivate explicitly stops the shared instance. Remove does not purge retained
   scoped/native custom data. Refresh inventory on unknown outcome, never replay writes.

These steps describe source-backed UI behavior; **visible/live-auth/socket
acceptance was not executed for this public sync**. An unsupported host must fail
closed, not downgrade ABI revision. Do not install unreviewed binaries to try this.

### Source roles and limits

- `native.rs`: buildable std-only native example derived from the reviewed harmless
  integration peer. No UI/network/custom file writes/store in this public example.
  Long-lived framed stdin/stdout, ABI2 handshake, invoke, subscription, full surface
  snapshots, explicit window rejection, shutdown. Small field reader expects
  **canonical trusted-host JSON**; not a full strict parser or security reference.
  A real author must implement strict framing/parsing/correlation and async cancel
  handling appropriate to their own logic; use SDK contract tests, not this scanner.
- `native.py`: byte-matching standalone stdlib fixture peer. Not an installable
  native binary, no SDK import/dependencies/native effects; speaks revision1/2 only
  for compatibility tests. Do not upload it as an executable. Run only via the
  bounded SDK harness, not terminal JSON (binary framing).
- `test_example.py`: offline compile/subprocess roundtrip, sanitized environment,
  bounded timeouts, artifact/namespace checks; not installation/auth/window proof.
- `prepare.py`: local descriptor/import-input generation; byte hash validation is
  not executable safety, containment or current-owner authorization.

Implement new OS behavior in your native application, declared names/schemas and
declarative controls without editing host providers. Arbitrary normal-user code
is **unsandboxed**: files/network can be accessed outside IPC. Source/hash/process
boundary do not make it safe. `storage:none` is disclosure only. Native windows
belong to your extension, never host consent/authentication. See the complete
[ABI2 contract](../../../contracts/plugins/v2/README.md).

## Türkçe: geliştirme, derleme ve kurulum

Bu örnek yerel DEV içindir; üretim sürümü/yayın veya chat ürünü değildir.
V1 Wasm desteklenir; taşıma zorunluluğu yoktur. Yeni kimlik
`com.example.switchboard`; `--id org.example.different` farklı ad alanı üretir.
Hostta kimliğe özel dal eklenmez. Kod MIT; özel host kaynağı/kullanıcı verisi içermez.

Windows x64 üzerinde kurulu Python 3.9+, `rustc` ve çalışan linker gerekir.
Yukarıdaki PowerShell komutlarını depo kökünden çalıştırın. Cargo/pip bağımlılığı
indirilmez. `prepare.py`, yeni `--out` klasörüne doğrulanmış `manifest.json`,
`identity.json`, `artifact.base64.txt` yazar; mevcut çıktının üzerine yazmaz.
Helper yalnız Windows x64 hedefler; Linux kabul testi ertelenmiştir.
`example.invalid` gerçek kaynak değildir; dağıtmadan önce gerçek kaynak adresi ve
EN/TR açıklamayı değiştirip kimliği SDK ile yeniden hesaplayın. Bu işlem yayın yapmaz.

Public depo host uygulamasını veya kurulum CLI'sını sağlamaz. Ayrı sağlanan uyumlu
ABI2 debug hostu, kendi `desktop` klasöründe
`cargo run --locked --offline --features plugin-dev-runtime` ile başlatılabilir.
Yetkili güncel cihaz/sahip bağlantısı gerekir. **Yerel uygulama** içe aktarma
alanına manifest ve base64 baytları yapıştırın; V1 `.rchub-plugin.json` alanına değil.
Uyumlu mobil/web paneli de iki girdili inceleme akışını kullanır. Önizlemede kaynak,
lisans, hash/hedef, `native.unsandboxed`, veri davranışı ve host risk uyarısını
inceleyip PC veya telefondan açıkça onaylayın. Kurulum begin/chunk/commit yapar,
kod başlatmaz; envanteri gözleyip ayrıca Etkinleştir ve Kontrolleri açın.
Anahtar yalnız bellek içi örnek durumu değiştirir; pencere eylemi bilinçli reddedilir.
Yüzeyi kapatmak yalnız kendi izleyicinizi kapatır; ortak süreç diğer yetkili
izleyiciler için devam edebilir. Devre dışı bırak ortak süreci durdurur. Bilinmeyen
sonuçta envanteri yenileyin, yazmayı otomatik tekrarlamayın.

`native.rs` sınırlı canonical host girdisi için küçük alan okuyucu kullanır;
genel JSON güvenlik doğrulayıcısı değildir. Gerçek uygulamada katı parser/çerçeve,
korelasyon ve kendi asenkron iptal davranışını uygulayın. `native.py` yalnız
framed-IPC test betiğidir, kurulabilir `.exe` değildir. Testler yerel derleme ve
alt süreç protokolünü doğrular; canlı host kurulum/auth/soket/görünür UI kanıtı
değildir. Normal kullanıcı native kodu **yalıtılmamıştır**; dosya/ağa erişebilir.
Kaynak/hash/süreç güvenlik garantisi vermez. Özel/geçici veri geliştiriciye aittir;
`none` beyanı dosya yazmayı engellemez. Kaldırma retained host verisini/özel dosyaları
silmez. Eklenti penceresi hostun izin/kimlik UI'sı değildir. Tam sınırlar:
[ABI2 sözleşmesi](../../../contracts/plugins/v2/README.md).
