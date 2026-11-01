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
The general-purpose [ABI2 CLI](../../../tools/plugins/rchub_app_v2_cli.py) can
bind any valid descriptor template to an artifact and verify the resulting bytes:

```powershell
python -B tools/plugins/rchub_app_v2_cli.py prepare --template plugins/examples/application-v2/build/import/manifest.json --artifact plugins/examples/application-v2/build/switchboard.exe --target windows-x86_64 --entry switchboard.exe --output plugins/examples/application-v2/build/prepared.json
python -B tools/plugins/rchub_app_v2_cli.py verify --manifest plugins/examples/application-v2/build/prepared.json --artifact plugins/examples/application-v2/build/switchboard.exe --target windows-x86_64 --entry switchboard.exe
```

`prepare` writes a canonical descriptor and prints its identity; `verify` checks
the supplied target, relative entry and artifact bytes against the descriptor and
prints the identity. Choose an output distinct from the template and artifact;
an existing output file may be overwritten. Unlike `prepare.py`, this CLI does
not generate base64 import data or change the plugin ID. Neither command installs
or runs the application, and successful verification is not a safety guarantee.


Generated source URL is `https://example.invalid/switchboard/source`: placeholder,
not provenance or a fetch endpoint. Replace with actual reviewable source and
update EN/TR disclosure before distributing, then recompute identity with the SDK.
No generated binary or imported package is committed/released by this workflow.

### Install from your own public repository in a compatible DEV host

There is no hosted installer or SDK install command in this checkout. To adapt this
local example, place your reviewed executable at `switchboard.exe` in your own
repository and use the CLI above with `--output rchub-app.json` at that repository's
root. Replace the placeholder source/disclosures before preparing the descriptor;
ensure the root manifest's entry and SHA-256 match the committed binary. Commit both
regular files to your own default branch and make the repository public when ready.
This example and its `example.invalid` source remain unpublished illustrative assets;
the separately built official Windows packages are published in
[`plugins/official-native`](../../../plugins/official-native/README.md).

In a separately supplied compatible authenticated DEV host, enter your public GitHub
repository root URL in the native workspace (`https://github.com/<owner>/<repository>`
is syntax only). The resolver pins the default-branch commit, verifies the root
manifest and executable hash, and binds the download to that commit and manifest
identity. Review repository, commit, source, license, target, hash, native permission
and data behavior, then explicitly approve on PC or phone. Installation stages bytes
without running code; activate separately, open Controls, and observe inventory.
The toggle changes only in-memory state; the window action rejects intentionally.
On an unknown result reconcile inventory before retrying. No live UI/authentication
acceptance or production release is implied. Manual import and V1 support have not
yet been technically removed; they are not the new recommended authoring path.

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
V1 desteği henüz teknik olarak kaldırılmadı; bu yeni geliştirme önerisi değildir.
Yeni kimlik `com.example.switchboard`; `--id org.example.different` farklı ad alanı üretir.
Hostta kimliğe özel dal eklenmez. Kod MIT; özel host kaynağı/kullanıcı verisi içermez.

Windows x64 üzerinde kurulu Python 3.9+, `rustc` ve çalışan linker gerekir.
Yukarıdaki PowerShell komutlarını depo kökünden çalıştırın. Cargo/pip bağımlılığı
indirilmez. `prepare.py`, yeni `--out` klasörüne doğrulanmış `manifest.json`,
`identity.json`, `artifact.base64.txt` yazar; mevcut çıktının üzerine yazmaz.
Helper yalnız Windows x64 hedefler; Linux kabul testi ertelenmiştir.
`example.invalid` gerçek kaynak değildir; dağıtmadan önce gerçek kaynak adresi ve
EN/TR açıklamayı değiştirip kimliği SDK ile yeniden hesaplayın. Bu işlem yayın yapmaz.
[ABI2 CLI](../../../tools/plugins/rchub_app_v2_cli.py) için yukarıdaki `prepare`
ve `verify` komutları depo kökünden çalıştırılır: `prepare` şablonu hedef, göreli
giriş yolu ve artifact baytlarına bağlayıp canonical manifest yazar; `verify` aynı
hedef, yol ve baytları manifest ile karşılaştırır. İkisi de kimliği yazdırır.
Çıktı şablon ve artifact dosyalarından farklı olmalıdır; mevcut çıktı dosyasının
üzerine yazılabilir. Bu CLI, `prepare.py` gibi base64 üretmez veya kimliği
değiştirmez; uygulamayı kurmaz ya da çalıştırmaz. Doğrulama güvenlik garantisi değildir.


Bu checkout host uygulaması veya kurulum CLI'sı sağlamaz. Örneği uyarlarken kendi
deponuza `switchboard.exe` dosyasını koyup yukarıdaki CLI ile depo köküne
`rchub-app.json` üretin. Önce placeholder kaynak/açıklamaları değiştirin; entry ve
SHA-256 commit edilen binary ile eşleşsin. İki normal dosyayı kendi varsayılan
dalınıza commit edip hazır olduğunuzda depoyu public yapın. Bu örnek yayınlanmış
depo veya resmi native paket iddiası değildir.

Ayrı sağlanan uyumlu, kimliği doğrulanmış DEV hostunun native alanına public GitHub
depo kökü URL'sini girin (`https://github.com/<owner>/<repository>` yalnız biçimdir).
Resolver varsayılan dal commit'ini sabitler, kök bildirim ve artifact hash'ini
doğrular, indirmeyi commit ve manifest kimliğine bağlar. Depo, commit, kaynak,
lisans, hedef, hash, native izin ve veri davranışını inceleyip PC veya telefondan
açıkça onaylayın. Kurulum kodu başlatmaz; ayrıca etkinleştirip kontrolleri açın,
envanteri gözleyin. Bilinmeyen sonuçta tekrar denemeden envanteri uzlaştırın.
Canlı UI/auth kabulü veya üretim yayını iddia edilmez; elle içe aktarım ve V1
desteği henüz teknik olarak kaldırılmamıştır.

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
