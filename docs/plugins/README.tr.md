# RCHub Eklenti Geliştirici Rehberi

## Native uygulama geliştirme ve GitHub'dan kurulum

Hedeflenen yeni model public GitHub depo URL'sinden kurulan native uygulamadır.
On iki resmi Windows x64 native paketi (sürüm 1.0.0) bu depodaki
[`plugins/official-native`](../../plugins/official-native/README.md) altında
yayınlanmıştır ve uyumlu hostun V2 kataloğundan sunulur; production native capability
sözleşmede tanımlı `owner-revocation-ack-v1` kanıtını gerektirir, aksi halde fail-closed
kalır. **Host, eski V1 executable/pilot eklenti yığınını kaldırdı**; V1 Wasm referans
paketleri, SDK ve sözleşme bu depoda kalır ve aşağıdaki V1 bölümleri
tarihsel/uyumluluk bilgisidir; yeni geliştirme önerisi değildir.

1. [ABI2 sözleşmesine](../../contracts/plugins/v2/README.md) göre framed IPC,
   handshake ve bildirilmiş eylem, olay ve kontrolleri uygulayın. Kendinize ait ID,
   sürüm, incelenebilir kaynak, lisans, EN/TR risk ve veri açıklamaları ile
   `native.unsandboxed` izni tanımlayın. Host hedefi için gerçek native executable
   derleyin. [Örnek](../../plugins/examples/application-v2/README.md) yerel açıklayıcı
   koddur; yayınlanmış resmi paket değildir.
2. Executable'ı kendi deponuza normal Git dosyası olarak göreli
   `execution.artifact.entry` yoluna koyun; doğrulanmış bildirimi depo kökünde
   **`rchub-app.json`** olarak kaydedin. Artifact SHA-256 commit edilen baytlarla
   eşleşmeli; bildirim en fazla 64 KiB, artifact en fazla 16 MiB olmalıdır. Artifact
   symlink, submodule ve Git LFS pointer kabul edilmez.
3. Yerel CLI şablonu ve binary'yi bağlar/doğrular; derleme, yayın, kurulum veya
   yetki verme işlemi yapmaz:

   ```powershell
   python tools/plugins/rchub_app_v2_cli.py prepare --template template.json --artifact app.exe --target windows-x86_64 --entry app.exe --output rchub-app.json
   python tools/plugins/rchub_app_v2_cli.py verify --manifest rchub-app.json --artifact app.exe --target windows-x86_64 --entry app.exe
   ```

4. Kaynak, binary, açıklama ve lisansı inceleyin; bildirim ve artifact'i kendi
   deponuzun varsayılan dalına commit edin ve hazır olduğunuzda public yapın.
   Buradaki örnek URL yayınlanmış bir paket değildir.
5. Uyumlu, kimliği doğrulanmış hostun native uygulama alanına public depo
   kökü `https://github.com/<owner>/<repository>` girin (yalnız biçim örneği).
   HTTPS GitHub depo kökü, isteğe bağlı `.git` ve son `/` kabul edilir; dal/dosya
   URL'si kabul edilmez. Resolver varsayılan dal commit'ini sabitler, kök
   `rchub-app.json` ve artifact'i o Git ağacından okur, hash'leri doğrular ve
   indirmeyi commit ile manifest kimliğine bağlar. Dalın değişmesi incelenen
   baytları sessizce değiştirmez. Çözümleme kurulum veya yayıncı doğrulaması değildir.
6. Depo, commit, hedef, kaynak, lisans, kimlik/hash, izin ve veri davranışını
   inceleyip mevcut PC veya telefon arayüzünde kurulumu açıkça onaylayın. Aşamalama
   kodu etkinleştirmez: ayrıca etkinleştirip kontrolleri açın. Güncel kimlik,
   sahip/cihaz ve izin denetimleri gereklidir. Sonuç bilinmiyorsa yeniden denemeden
   envanteri uzlaştırın.

### Yayınlanmış resmi paketler

`plugins/official-native/<name>/` altında on iki tam paket kökü bulunur: kanonik
`rchub-app.json`, hash'e bağlı `bin/rchub-official-native.exe`, `native-source.json`,
pakete ait `native-runtime/` kaynakları ve lock dosyası, `LICENSE` ve README. On iki
paketin tamamı 1.0.0 sürümünde, `windows-x86_64` hedefinde,
`bin/rchub-official-native.exe` girişinde ve `native.unsandboxed` iznindedir;
artifact SHA-256 değerleri [paket dizininde](../../plugins/official-native/README.md)
listelenir.

Uyumlu host bunları kendi checkout'ından
`GET /api/plugins/v2/catalog` ve `GET /api/plugins/v2/artifacts/<id>/<version>/<sha256>`
yollarıyla sunar. İki yol da yetki ister: mobil/web için hesap oturumu Bearer
belirteci, masaüstü için zaten kullandığı eşleşmiş cihaz `x-device-id` /
`x-device-password` başlıkları. Yetkisiz istek `401` ile reddedilir, yanıtlar
cache'lenmez ve masaüstü katalog eylemi cihaz kimlik bilgilerini gönderemediği sürece
kapalı kalır. Katalog listesi ve hash'e bağlı indirme; kurulum, etkinleştirme, yayıncı
doğrulaması veya güvenlik kanıtı değildir; artifact yolu yalnız boyutu ve SHA-256
değeri katalog bildirimiyle eşleşen baytları sunar. `POST /api/plugins/github/resolve`
ve commit'e bağlı artifact yolu aynı iki yetkiyi kabul eder: mobil/web için hesap
oturumu Bearer belirteci, masaüstü için eşleşmiş cihaz başlıkları. Eşleşmemiş bir
bilgisayar depo çözemez veya artifact indiremez; cihaz parolası yalnız bağlı sunucu
origin'ine istek başlığı olarak gider.

Bu depo dağıtım kataloğudur, paket başına depo kökü değildir. GitHub URL çözümleyicisi
depo kökünde `rchub-app.json` ister; bu monorepo alt dizin URL'leri onunla kurulamaz.
Katalogdan kurun veya yukarıdaki adımlarla tek paketlik kendi deponuzu yayınlayın.
Linux paketleri yayınlanmadı. [`plugins/official`](../../plugins/official) altındaki eski V1
dizinleri değiştirilmedi; ID, hash ve izinleri aynıdır.

**Native kod sandbox dışındadır:** broker dışında normal kullanıcının dosya, ağ ve
OS kaynaklarına erişebilir. Hash, süreç ayrımı, veri beyanı, kaynak URL'si veya
onay güvenlik ya da yayıncı doğrulaması sağlamaz. Yalnız güvenilir binary kurun;
V1 izinleri taşınmaz. Bu DEV açıklaması her istemci/platformun kullanılabilirliğini
veya canlı UI/kimlik doğrulama kabulünü garanti etmez.

Aşağıdaki ömür/protokol notları deneysel uygulamayı; eski V1 talimatları yalnız
tarihsel uyumluluk bağlamını anlatır.

### Host release bağlantısı ve resmi katalog

RcHub host checkout'ında native V2 varsayılan/release desktop derlemesine dahildir;
`plugin-dev-runtime` native V2 için değil eski Wasm geliştirme runtime'ı içindir.
Desktop'un derleme zamanı `RCHUB_SERVER_ORIGIN` değeri sunucudaki
`RCHUB_PLUGIN_SERVER_ORIGIN` ile aynı tam HTTPS origin olmalıdır. Eksik veya geçersiz
production ayarı fail-closed olur; sunucu yalın HTTPS origin veya geliştirmede
loopback HTTP kabul eder, kimlik bilgisi/path/query/fragment kabul etmez. Uyuşmazlık
yetki bağlamını engeller.

Hostun resmi native kaynak tanımları `plugins/official/*/native-source.json`
altındadır; `tools/plugins/build_official_native.py` gerçek hedef baytlarını
varsayılan olarak `plugins/official-native/<name>/rchub-app.json` ve artifact yoluna
derler. Sunucu çalışma dizini veya üstündeki `plugins/official-native` dizinini okur;
`/api/plugins/v2/catalog` ve hash'e bağlı artifact yollarını sunar, eksik/geçersiz
girdileri atlar. İki yol da güncel yetki ister;
[Yayınlanmış resmi paketler](#yayınlanmış-resmi-paketler) bölümüne bakın. Bu kardeş
checkout derlenmiş Windows paketlerini aynı düzende yayınlar; host checkout'ı bu
incelenmiş baytları doğrudan kullanabilir. Sunmak ve indirmek kurulum, yayıncı
kimliği veya güvenlik kanıtı değildir.

Native yetki, süreç-yerel bir coverage bayrağı yerine her owner-control revizyonu için
kanıtlanır: kabul edilen her revizyonda PC bağlı tüm instance'ları durdurur, sahipli
çocuk sürecin gerçekten toplandığını doğrular ve `plugin-dev:revocation-ack` mesajını
`deviceId`, `epoch`, `authorityRevision`, `stopped` ve durdurulan `instances` ile
döndürür. Readiness, dispatch ve watch'lar tam güncel revizyon için `stopped:true`
taşıyan eşleşen ack ister; eksik, bayat veya başarısız ack - ve kendi ack'i olmayan her
yeni revizyon - fail-closed olur ve türetilmiş gözlemleri düşürür. Doğrulanamayan
öldürmeler `stopped:false` bildirir ve host not-live kalır. Doğrudan SQL, admin veya
başka sunucu süreciyle yapılan değişiklikler yalnız sonraki yetki okumasında fark
edilir, polling ile değil. Canlı DB/JWT, görünür UI, tam production dağıtımı ve Linux
kabulü doğrulanmadı; synthetic, injected veya headless bulgular release kabulü değildir.


### Onay, ömür, pencere ve veri

Kurulum ve her yeni native hedef güncellemesinde disclosure v2 açık onayı gerekir;
exact native izin/kimlik, sayısal daha yeni sürüm ve güncellemede expected kurulu
kimliği denetlenir. V1 onayı taşınmaz. Verilmiş izin dahilinde işlem/session/
reconnect/restart başına popup yok; her işlem/gözlem yine current auth/owner/link,
exact grant/hash, canonical socket/epoch, authorityRevision ve aktif context
denetiminden geçer. Telefon JWT/session kuralı değişmez. PC-local viewer ayrı,
self-only ve server-validated'dır; sahte kullanıcı session/JWT'si üretmez.

Uzun ömürlü süreç tek viewer JWT'sine değil yetkili owner context'e bağlıdır.
Viewer kapatma/logout/süre bitimi yalnız kendi pending/watch işini temizler;
diğer yetkili viewer ve ortak süreç devam edebilir. Owner/link/device/grant iptali
ve canonical bağlantı kaybı ilgili runtime/stage'i durdurur. Kapatma hedefi serverın
çözdüğü exact açık watch'tır; yabancı watch kapatılamaz. Devre dışı bırakmak panel
kapatmadan farklıdır. Unknown sonuçta envanteri uzlaştırın; otomatik replay/rollback
iddiası yok.

PC/mobil/web UI sıralı türlenmiş deklaratif kontrol/state kullanır; HTML/JS/keyfi
çizici yoktur. Native pencere eklentiye aittir, host izin/kimlik arayüzü değildir;
ömrü/verisi geliştiricinin sorumluluğudur. Özel/geçici dosya seçimi açıklanır ve
host cleanup garantisi dışındadır. `storage.choices` yalnız beyan; `none` yazmayı
engellemez. İsteğe bağlı `host-scoped` + `scoped-store-v1` hostun origin/device/
current owner/plugin ID ile belirlediği scope'ta kotaya bağlı CAS nesne deposudur.
Kaldırma scoped veriyi silmez; aynı scope'ta onaylı yeniden kurulum/güncelleme erişim
sağlayabilir, yeni sahibe aktarılmaz. Silme/global retention/mesaj ürünü kuralı eklenmedi.

### Kanıt ve yayın sınırı

ABI2 deneysel DEV entegrasyonu olarak kalır; üretim sürümü değildir. On iki resmi
Windows x64 paketi 1.0.0 sürümüyle bu depodaki `plugins/official-native` dizininde
yayınlanmıştır ve hostun yetki isteyen V2 kataloğundan sunulur; süreçler arası
güvenilir owner iptali sağlanana kadar production native capability fail-closed
kalır ve Linux paketleri yayınlanmadı. V1 paketleri clone halini
korur; ayrı V1 select/Speech kaynak/SDK eşleme işi bu ABI2 sync'e gizlice dahil
edilmedi. Ana agentın 61 bağımsız Windows-isolated installed kontrolü gerçek native
child/RustLocal/headless çizim kullandı, ancak **synthetic DB ve injected transport**
ile: `bRustLocalVerified:true`, `headlessRendererVerified:true`,
`socketIoLoopbackVerified:false`, `pcConnectionLocalUiVerified:false`,
`liveDbAuthVerified:false`, `uiVerified:false`, `productComplete:false`.
Host testleri public checkout'ta yeniden çalıştırılmadı; görünür UI/live DB/JWT/
tam Socket.IO kabulü değildir. Linux kabulü ertelendi; üretim/güvenlik garantisi yok.
[Varlık kökeni](../../plugins/examples/application-v2/PROVENANCE.md).

## V1 Wasm geliştirici rehberi

RCHub eklentileri, küçük bir WebAssembly konuğuyla çalışan bildirimsel yerel kontrol panelleridir. Paneli, kurulumu, saklanan izni ve yetenek denetimlerini ana uygulama yönetir. Eklenti yalnızca bildiriminde belirtilen yetenekleri isteyebilir; keyfi arayüz kodu veya JavaScript/HTML çalıştıramaz.

Bu rehber mevcut geliştirme uygulamasını anlatır. Marketplace kaydı ve uzaktan çalıştırma yalnızca DEV içindir. Katalog kaydını, `publisher.kind` değerini, kaynak URL'sini veya lisans metnini yayıncı kimliği, kod kaynağı, inceleme ya da güvenlik kanıtı olarak görmeyin. Kurmadan önce paketi ve istediği yetenekleri inceleyin.

LLM araçları için [llms.txt](https://github.com/CetTek/rchub-plugins/blob/main/llms.txt) dosyasında kısa bir belge ve örnek dizini bulunur. RCHub DEV uygulamasında `/plugin-preview` yolu, yalnızca örnek durum kullanan etkileşimli önizlemeyi açar.

## Hızlı başlangıç

SDK bağımlılık gerektirmeyen bir Python betiğidir. Komutları depo kökünden çalıştırın:

```powershell
python tools/plugins/rchub_plugin.py create examples/volume --id com.example.volume --name "Volume"
python tools/plugins/rchub_plugin.py build examples/volume --proxy --out examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py validate examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py inspect examples/volume/volume.rchub-plugin.json
python tools/plugins/rchub_plugin.py preview examples/volume/volume.rchub-plugin.json
```

`create`, Audio başlangıcı için `manifest.json` ve okunabilir `guest.wat` oluşturur. Dağıtımdan önce örnek yayıncı ve kaynak bilgilerini değiştirin. `build --proxy`, tanımlı her eylem girişini tanımlı yeteneğine ileten gerçek bir Core Wasm konuğu üretir. Bu kolaylık konuğu, eylem başına tek yeteneği iletir; özel mantık uygulamaz. `--proxy` olmadan `build`, proje klasöründeki `guest.wasm` dosyasını kullanır. Özel konuk mantığını derlemek için Wasm araç zinciri kullanın.

Kaynak `manifest.json` dosyaları okunabilir, iki boşluk girintili JSON kullanır. Derlenen `.rchub-plugin.json` paketleri kompakt ve kanonik kalır; SHA-256 kimlikleri kanonik baytlardan hesaplanır. Paketin ve bildirimin okunabilir görünümü için `inspect` komutunu kullanın.

`validate`, `inspect` ve `preview` `.rchub-plugin.json` paketini alır. `preview` yerel önizleme adresini yazar; mobil geliştirme sunucusunu başlatın, `/plugin-preview` yolunu açın ve paket JSON'unu örnek önizlemeye yapıştırın. Bağımsız önizleme yerel örnek durumu kullanır ve cihaza işlem göndermez. SDK komutları bağımlılık indirmez, paket kancaları çalıştırmaz ve hesap ya da sunucuya bağlanmaz.

## Paket biçimi ve kimlik

Dağıtılabilir paket UTF-8 JSON zarfıdır:

```json
{"formatVersion":1,"manifest":{},"wasm":[0,97,115,109,1,0,0,0]}
```

`wasm` bir tamsayı bayt dizisidir. Kanonik paketin tamamı en fazla 131.072 bayt, Wasm modülü en fazla 16.384 bayt olabilir. Paket kimliği `{id, version, sha256}` biçimindedir. Küçük harfli SHA-256 özeti tam kanonik paket baytlarını kapsar: nesne anahtarları iç içe düzeylerde sözlük sırasına dizilir, dizilerin sırası korunur, JSON boşluksuz ve ASCII dışı karakterler doğrudan UTF-8 olacak şekilde yazılır, yalnızca sonlu sayılara izin verilir ve sona tam bir LF (`\n`) eklenir. SDK bu kodlamayı uygular ve derleme sonrası kimliği yazdırır.

Nesneler katıdır; bilinmeyen anahtarlar reddedilir. Bildirimler şema ve host API sürüm 1'i, İngilizce ve Türkçe metni, küçük harfli ters alan adlı kimlikleri, sayısal `major.minor.patch` sürümünü, Windows/Linux hedeflerini, izin listesini, eylemleri, türlenmiş durumu ve UI denetimlerini kullanır. Sayısal alanların tamsayı sınırları, metin alanlarının azami uzunluğu olmalıdır. Diziler ve iç içe nesneler sınırlıdır. Girdi, çıktı ve durum tanımsız alanları reddeder. Çalışma zamanları arasında kanonik yazımı tutarlı tutmak için sayısal sınır ve kaydırıcı adımlarında tamsayı kullanın.

Çalışma zamanı yalnızca `memory` (yeni vekillerde 5 sayfa, büyük çıktıda 16; eski konuklarda 2) ve `run(i32 entry, i32 inputLen) -> i32 outputLen` dışa aktarımına sahip sınırlı Core Wasm kabul eder. Host girdi JSON'unu 0. konuma (yeni 5/16 sayfalı konukta en fazla 65.536 bayt; eski 2 sayfalı konukta 8.192 bayt), konuk çıktı JSON'unu 32.768. konuma (en fazla 900.000 bayt) yazar. İzin verilen tek içe aktarım `rchub.invoke` olup, paket, işlem ve kurulu izin kontrollerinden sonra türlenmiş JSON yükünü yerel yeteneğe iletir. WASI, başlangıç işlevi, başka içe aktarım, keyfi şablon, ifade değerlendirme, dosya yolu, kanca, arşiv veya bağımlılık çalıştırma yoktur. Her işlem mevcut yakıt, bellek ve worker sınırları altında yeni bir konuk deposu oluşturur.

## İzinler ve kurulum

İzinler `manifest.permissions` içindeki sıralı ve tekrarsız yetenek kimlikleridir. Her eylem bildirilmiş bir yeteneği kullanmalıdır. Kurulumda kaynak/risk açıklaması ve istenen izinler gösterilir; telefondan veya PC'den açık onay gerekir. İzin yeniden bağlantı ve uygulama başlatma boyunca saklanır; bu izin dahilindeki kullanım her eylemde tekrar sorulmaz. Her işlem yine de mevcut cihaz sahibini doğrular, kurulu paketi ve izni denetler, yetenek ve platform uygunluğunu kontrol eder, türlenmiş girdi ve çıktıyı doğrular. Eksik, geçersiz veya desteklenmeyen izin güvenli biçimde reddedilir.

Kurulum mevcut sürümün üzerine yazmaz. Uyumlu host, aynı kimliğin kesinlikle daha yeni ve izinleri birebir aynı sürümüne açık onaylı atomik güncelleme sunar. İzin değişikliği ayrı kaldırma ve kurulum gerektirir; yeni izin sessizce devralınmaz. Panel açılışı, bildirilen ve bağımsız olarak argümansız bir okuma eylemiyle durumu gözlemleyebilir; host yazma eylemini otomatik tekrar oynatmaz.

Rust masaüstü istemcisi atomik güncelleme desteğini yalnızca cihaz soketi ve dönemi doğrulandıktan sonra duyurur. Sunucu duyuruyu sadece geçerli kanonik PC soketinden ve eşleşen dönemden kabul eder; yeniden bağlantı veya bağlama değişiminde desteği temizler. Mobil uygulama aynı izinlere sahip uygun sürüm için güncelleme göstergesi sunar. Eski masaüstü ya da desteği henüz duyurmamış host `unsupported` döndürür; güncelleme gönderilmez veya tekrar oynatılmaz. Güncel masaüstünü yeniden başlatıp güncellemeyi açıkça yeniden deneyin. Duyuru yalnız işlem desteğini bildirir; izin veya istek sahibi yetkisi vermez. İstek `unknown` ile biterse, sonucu gözlemleyemediğimiz için yeniden denemeden önce kurulu eklenti envanterini yenileyin.

## Yeteneklerin durumu

Bir adın bildirim şemasında kabul edilmesi, sağlayıcısının uygulandığı anlamına gelmez. Mevcut Rust yerel yönlendiricisi aşağıdaki Windows uygulamalarını içerir; platform ve paket uygunluğu çalışma anında denetlenir.

| Yetenek | Mevcut durum | Etki / sonuç |
| --- | --- | --- |
| `audio.read` | Windows'ta hazır | Varsayılan çıkış ses düzeyi ve sessiz durumunu okur. |
| `audio.write` | Windows'ta hazır | Sınırlı ses düzeyini ve/veya sessiz durumunu ayarlar; gözlemlenen durumu döndürür. |
| `system.read` | Hazır | İşletim sistemi, bilgisayar adı, CPU, bellek ve çalışma süresini okur. |
| `browser.open` | Windows'ta hazır | HTTP(S) adresini varsayılan tarayıcıda açar. |
| `power.write` | Windows'ta hazır | Kilitleme/uyku, açık zorla kapat seçeneğiyle sınırlı gecikmeli kapatma/yeniden başlatma veya planlanan kapatmayı iptal etme. |
| `keyboard.write` | Windows'ta hazır | Sınırlı metni yazar veya kısıtlı kısayol gönderir. |
| `media.write` | Windows'ta hazır | Oynat/duraklat, sonraki veya önceki parçayı kontrol eder. |
| `clipboard.read` | Windows'ta hazır | Panodaki sınırlı metni okur. |
| `clipboard.write` | Windows'ta hazır | Panodaki sınırlı metni değiştirir ve gözlemlenen metni döndürür. |
| `apps.read` | Windows'ta hazır | En fazla 64 görünür uygulamayı veya tüm işlemleri pencere/işlem kimlikleriyle listeler. |
| `apps.write` | Windows'ta hazır | Gözlenen kimliği doğrulayarak öne getir/küçült/büyüt/geri yükle/pencere kapat/işlem sonlandır. |
| `display.read`, `display.write` | Windows'ta uygulandı | Birincil fiziksel monitörde yalnız DDC/CI parlaklık. Destek yoksa okuma `available:false` döner, yazma reddedilir. |
| `screenshot.read` | Windows'ta uygulandı | En fazla 1920× 1080, kalite 80 JPEG; 800.000 karakterlik data URL sınırına sığması için gerekirse küçültülür. İstemcide görüntüye dokunarak tam ekran açın. |
| `speech.write` | Windows'ta uygulandı | SAPI ile en fazla 2.000 karakter düz metin, en fazla 10 saniye; süre/yetki bitiminde iptal edilir. |
| `effects.write` | Windows'ta uygulandı | Yalnız `{effect:"beep"}`; Windows sistem sesini çalar. |

Gerekli yeteneği veya hedef platformu mevcut olmayan bir paketi çalışır diye tanıtmayın ya da kurmayın. Host, etkisiz bir işlemi başarılı saymak yerine desteklenmiyor/kullanılamıyor olarak bildirir. Diğer platformlarda daha az sağlayıcı bulunabilir; hostta gösterilen kullanılabilirliği kontrol edin.

## UI şeması

Alanlar, sınırlar ve ABI için [uygulama sözleşmesine](../../contracts/plugins/README.md),
editör desteği için [JSON Schema dosyasına](../../contracts/plugins/plugin.schema.json)
bakın. JSON Schema eylem, durum ve denetimler arasındaki bütün çapraz bağları
doğrulayamaz; SDK doğrulamasını çalıştırın. Tam Wasm doğrulamasını PC yapar.

| Denetimler | Gerekli ilişki |
| --- | --- |
| section, row, column, tabs | Sınırlı `children` ağacı; tabs bir çocuk grubunu seçer. |
| button | Tanımlı `action`; zorunlu argümanı `argument` ve sabit `value` ile verin. |
| slider | Sayısal `bind`, uyumlu eylem `argument`, açık `min`, `max` ve pozitif `step`. |
| toggle, select | Uyumlu durum/eylem argümanı; select için sınırlı, yerelleştirilmiş `options`. |
| input | Çok satırlı metin argümanı veya action/argument şemasına bağlı `submit:false` sayısal/boolean taslak; `defaultValue` şemaya uyar. |
| value, status, text | Tanımlı `bind` durumunu veya yerelleştirilmiş metni gösterir. |
| progress | Sayısal durum bağı ve açık `min`/`max`. |
| list | Dizi durumu; nesne satırları için `itemLabel` ve `itemValue`; eyleme türlenmiş satır değeri gider. |
| image | PC/mobil ortak kullanım için gömülü PNG/JPEG data URL; uzak adres kullanılmaz. |

Çalışan örnek için [Audio bildirimini](../../plugins/official/audio/manifest.json)
inceleyin. `volume` kaydırıcısı gözlemlenen sayısal duruma bağlanır, bırakılan
değeri `set_volume` eylemine iletir; `muted` boolean anahtarıdır. Adlar, etiketler,
aralıklar ve eylem bağları Audio'ya özel mobil modal yerine bildirimden gelir.
[Apps bildirimi](../../plugins/official/apps/manifest.json), nesne satırlarını ve
seçilen PID'nin türlenmiş eyleme aktarılmasını gösterir.

[System bildirimi](../../plugins/official/system/manifest.json), mevcut `progress`
denetimiyle çizilen bellek kullanım grafiğini de gösterir. `system.read`, sınırlı
`ram_usage_percent` gözlemini üretir; uygulama görünümü manifestte tanımlanır.

Bildirim, UI sürüm 1'i, yerleşik simgeyi, tema rengi belirtecini ve sınırlı denetim ağacını tanımlar. Kimlikler benzersiz küçük harfli tanımlayıcılardır. Yerleşim düğümleri (`section`, `row`, `column`, `tabs`) çocuk içerebilir. Bağlar doğrudan tanımlı durum veya eylem girdisi alanlarını gösterir. Yaprak denetimler eylem ve girdi argümanı, sabit skaler değer, sayısal aralık, yerelleştirilmiş seçenek veya türlenmiş liste satırı alanı belirtebilir. Onay, izinler, hakkında ve kaldırma arayüzü hosta aittir.

Mobil renderer şu denetimleri uygular: `section`, `row`, `column`, `tabs`, `button`, `slider`, `toggle`, `select`, `input`, `progress`, `image`, `list`, `text`, `value`, `status`. Denetimlerin geçerli şemaları ve karşılık gelen durum/çıktı değerleri olmalıdır; geçersiz veya kullanılamayan eylemler çalışır denetim olarak sunulmamalıdır. Görsel değerleri PNG, JPEG veya WebP data URL ile sınırlıdır. Listeler için metin içine yapılandırılmış veri kodlamak yerine sınırlı türlenmiş dizi ve nesneler kullanın.

## İçe aktarma ve Marketplace

Topluluk paketleri katalog kaydı olmadan içe aktarılabilir. Mobilde `.rchub-plugin.json` dosyasını metin olarak açın ve içeriğinin tamamını içe aktarma alanına yapıştırın; bu akışta paket dosyası seçici yoktur. PC'de yerel paket yolunu girebilir veya JSON içeriğini yapıştırabilirsiniz. Her iki yol da sınırlı paketi doğrular ve kurulum onayından önce yayıncı/kaynak açıklamasını, lisansı, hedefleri ve izinleri gösterir. Kaynak ve yayıncı alanları paket içindeki beyanlardır; doğrulanmış kaynak bilgisi değildir.

Mobil çalışma alanı ve yönetim penceresi yalnızca sunum katmanıdır: yüklü kartlar ana makine envanterinden gelir, Mağaza ızgarası gerçek katalog/arama/kategori/sayfalama uçlarını kullanır ve yetkili bir PC istemcisi bağlı değilken yükleme veya kaldırma devre dışı kalır. PC çevrimdışıyken Mağazaya göz atmak mümkündür; göz atmak hiçbir paketi kurmaz veya çalıştırmaz.

Mevcut katalog DEV ortamında depoya ait paket dosyalarını ve metaveriyi sunar. Katalog rotaları keyfi URL'lerden indirme yapmaz, yayıncıları doğrulamaz, inceleme veya indirme sayaçları sağlamaz. Katalogda bulunmak paketi kurmaz veya çalıştırmaz; kurulum ve sonraki çağrılar sahip doğrulamalı işlemlerdir.

## Örnek paketler

### 0.2.0 ile gelen ortak düzenler (0.3.0 sürümünde korunur)

Ortak mobil/web çizici, plugin kimliğine özel ekran seçmeden manifesti kullanır.
Plugin kontrol sırasını, `row`/`column`/`section`/`tabs` gruplarını, EN/TR
etiketleri, ikonları, ana tema rengini ve kontrolün `tone` değerini (`default`,
`primary`, `danger`, `muted`) belirler. Mevcut schema-v1 alanları ortak çizicide
uygulanır: ana buton tema rengini kullanır, tehlikeli buton dolu kırmızıdır;
satırdaki ikonlu butonlar erişilebilir etiketli dairesel kontrollere dönüşür.
`sun` ve `play` ikonları mevcut turuncu/yeşil renklerini kullanır. Satırdaki ikonlu
anahtar kompakt olur, diğer anahtarlar etiketli switch olarak kalır. Slider
sürüklenirken taslak yüzdeyi gösterir, yalnız bırakıldığında işlem gönderir.
Keyfi CSS/HTML/script kabul edilmez; manifesti/builder'ı düzenleyip paketi üretin.

Panel açıldığında girdisiz `.read` işlemleri tek istekli taşıma katmanına uygun
olarak sırayla okunur. Yenileme panel altında kompakt bir eylemdir ve yazma
işlemi çağırmaz. Yeni Ses/Ekran/Sistem düzenlerinde gereksiz yenileme butonları
yoktur. Kapatma bekleyiciyi ve kalan okumaları iptal eder. Yazarken son gözlem
korunur; başarılı kısmi çıktı birleştirilir, hatada gözlemler temizlenir.
Önizleme yalnız örnek veriyi kullanır, cihaza komut göndermez.

Resmi Media 0.2.0 artık **Medya ve Ekran**: ses/sessiz, DDC/CI parlaklık ve oynatma
aynı manifestte bu ortak bileşenlerle tanımlanır. Kurulumda audio.read/write,
display.read/write ve media.write açıkça istenir; eski izinler birleştirilmez.
DDC/CI desteklenmiyorsa parlaklık pasiftir. Birleşik paket varken DEV kataloğu
ayrı Ses/Ekran paketlerini keşif listesinden çıkarır; kaynakları ve eski
kurulumları korunur. On iki kaynak paket de ortak düzen/ikon bilgisini korur; yeni
düzen için eski sürümü açık inceleme ve onayla güncelleyin. Kayıtlı manifestler kendiliğinden değiştirilmez.
Güç 0.3.0 sınırlı gecikme, açık zorla kapat seçeneği ve planlanan kapatmayı iptal etme sunar.

Public kaynaklar, SDK ve bu rehber
[CetTek/rchub-plugins](https://github.com/CetTek/rchub-plugins) deposundadır.
Resmi 0.3.0 paketleri `v0.3.0` kaynak etiketine bağlanır. Kaynak adresi
değişikliği yeni paket hash'leri üretir. Kurulu eski paketler/izinler sessizce taşınmaz; uyumlu hostta aynı izinli güncellemeyi inceleyip açıkça onaylayın. İzin değişikliği ayrı kaldırma ve kurulum gerektirir.

`plugins/official/` altında depoya ait on iki MIT paket bulunur: Audio, System,
Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech ve
Effects. “Official” depo etiketidir; kriptografik imza değildir. Bildirimler ve
`guest.wat` kaynak olarak incelenebilir. Paketleri yeniden üretmek için
`python tools/plugins/build_official.py` kullanın.

Bu DEV dilimi eski menülerin her seçeneğini taşımaz: Apps işlemleri/pencereleri yönetir, uygulama başlatmaz; Screenshot en fazla 1920× 1080 sınırlı optimize görüntü
verir; Effects sahte BSOD veya ekran sallama yerine sistem sesi sunar. Windows
sağlayıcılarının derleme ve sahte sağlayıcı kapsamı vardır; donanım, yerel yazma
işlemleri ve fiziksel telefondaki davranış canlı doğrulama bekler. Uygulanmış olmak,
bu PC'de canlı çalıştırıldığının kanıtı değildir.

### Yeniden kullanılabilir kontroller ve resmi örnekler (0.3.0)

Eklenti paneli tamamen ortak manifest kontrolleriyle tanımlanır; çizici eklenti kimliğine göre özel ekran seçmez. `progress`, `appearance:"ring"` ile halka gösterebilir. Metin girdileri `multiline` ve text/url `inputMode`; sayısal taslaklar numeric modunu destekler. Yalnız taslak tutan girdiler action/argument şemasını, `submit:false` ve şemaya uygun `defaultValue` değerini belirtir; boolean taslak onay kutusudur. Butonun `inputBindings` alanı taslakları tek doğrulanmış çağrıda birleştirir. `confirm`, tehlikeli işlem için EN/TR başlık ve mesaj içerir; kurulum izninden ayrıdır.

Listelerde yerel `searchable` araması ve seçili satırın `itemActions` menüsü bulunur: yerelleştirilmiş label/action, ana argument/value, sabit `args`, satır alanını taşıyan `itemArguments`, isteğe bağlı icon/tone/confirm. Satır seçmek işlem başlatmaz. Tüm zorunlu alanlar ve şema uyumu doğrulanır; [sözleşme](../../contracts/plugins/README.md) ve resmi manifest örneklerine bakın.

System kompakt CPU/RAM halkalarını ve bellek değerlerini gruplar. Keyboard çok satırlı yazı ve hızlı kısayollar içeren sekmelere sahiptir. Browser adres girdisi ve hızlı siteler sunar. Apps varsayılan olarak görünür başlıklı uygulama pencerelerini listeler; tüm işlemlere geçebilir, gözlenen satırlarda arama yapar ve işlem başlangıç kimliğini doğrulayarak pencere/sonlandırma menüsü sunar. Kapat/sonlandır işlem onayı ister. Power gecikme (0..86400 saniye, arayüz varsayılanı 30) ve zorla kapat taslaklarını yalnız kapat/yeniden başlat çağrısına birlikte ekler; planlanan kapatmayı iptal etme ayrıdır. Windows gecikme seçilince force=false değerini korur. Pano boş/metin dışıysa boş metin döner; geçici kilitlenmede sınırlı yeniden deneme yapılır.

Resmi 0.3.0 kaynak adresleri `v0.3.0` etiketini kullanır. Önce ana uygulamayı güncelleyin: schema/UI/hostApi 1 olarak kalır; eski katı doğrulayıcılar yeni isteğe bağlı alanları reddeder. İzinleri aynı olan yeni sürüm için incelemeli Güncelle işlemini kullanın; izin değişikliği ayrı kaldırma ve kurulum gerektirir. Yeni üretilen vekiller 5 sayfa bellek, 262144 bayt çıktı ve 64 KiB JSON girdi; büyük ekran görüntüsü 16 sayfa/900000 bayt çıktı kullanır. Eski 2 sayfalı vekillerde 8 KiB girdi sınırı korunur. Alanların UTF-16 sınırları da geçerlidir (pano 8192, klavye 4096). Örnek önizleme ve testler yerel yazma işlemi göndermez.


### Açık onaylı yerinde güncelleme

Marketplace ve kurulu eklenti yönetimi sayısal major.minor.patch sürümlerini
karşılaştırır. Güncelleme yeni kaynak, lisans ve izinleri indirdikten sonra açık
onay ister. V2 update isteği tam kurulu kimliğini expectedPackage (id/version/sha256),
hedef package/bundle, permissions ve kurulum consent alanlarıyla gönderir.
Yalnız kesinlikle daha yeni ve aynı izin kümesine sahip sürümler kabul edilir.
Ön sürüm/build eki, sürüm düşürme, aynı sürümü değiştirme ve izin değişikliği reddedilir.
Ana uygulama eski kimliği ve güncel sahip bağını denetler, kaydı atomik değiştirir.
Başarısızlıkta kurulu paket ve izinler korunur; eski kimlik uyuşmazsa envanter
yenilenmelidir. Aynı ID mevcutken install çakışma döndürmeye devam eder.
Otomatik güncelleme veya yeni izni sessizce devralma yoktur.

### System 0.3.1 kaynak kontrol noktası

System 0.3.1 RAM etiketini kısaltır; bilgisayar/işletim sistemi gözlemlerini aynı satıra alır. Bellek kullanılan/toplam MB, çalışma süresi saniyedir; bu bildirim yeşil RAM veya kullanılan/boş GB sunmaz. Duyarlı halka/modal boyutunu ortak host çizicisi belirler. 0.3.1 kaynak ve paketi bu snapshot'a dahildir; kurulum veya güncelleme ayrıca açıkça onaylanıp gözlemlenmelidir. Diğer resmi paketler 0.3.0 kalır.
