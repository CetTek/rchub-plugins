# RCHub Eklenti Geliştirici Rehberi

RCHub eklentileri, küçük bir WebAssembly konuğuyla çalışan bildirimsel yerel kontrol panelleridir. Paneli, kurulumu, saklanan izni ve yetenek denetimlerini ana uygulama yönetir. Eklenti yalnızca bildiriminde belirtilen yetenekleri isteyebilir; keyfi arayüz kodu veya JavaScript/HTML çalıştıramaz.

Bu rehber mevcut geliştirme uygulamasını anlatır. Marketplace kaydı ve uzaktan çalıştırma yalnızca DEV içindir. Katalog kaydını, `publisher.kind` değerini, kaynak URL'sini veya lisans metnini yayıncı kimliği, kod kaynağı, inceleme ya da güvenlik kanıtı olarak görmeyin. Kurmadan önce paketi ve istediği yetenekleri inceleyin.

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

`validate`, `inspect` ve `preview` `.rchub-plugin.json` paketini alır. `preview` yerel önizleme adresini yazar; mobil geliştirme sunucusunu başlatın, `/plugin-preview` yolunu açın ve paket JSON'unu örnek önizlemeye yapıştırın. Bağımsız önizleme yerel örnek durumu kullanır ve cihaza işlem göndermez. SDK komutları bağımlılık indirmez, paket kancaları çalıştırmaz ve hesap ya da sunucuya bağlanmaz.

## Paket biçimi ve kimlik

Dağıtılabilir paket UTF-8 JSON zarfıdır:

```json
{"formatVersion":1,"manifest":{},"wasm":[0,97,115,109,1,0,0,0]}
```

`wasm` bir tamsayı bayt dizisidir. Kanonik paketin tamamı en fazla 131.072 bayt, Wasm modülü en fazla 16.384 bayt olabilir. Paket kimliği `{id, version, sha256}` biçimindedir. Küçük harfli SHA-256 özeti tam kanonik paket baytlarını kapsar: nesne anahtarları iç içe düzeylerde sözlük sırasına dizilir, dizilerin sırası korunur, JSON boşluksuz ve ASCII dışı karakterler doğrudan UTF-8 olacak şekilde yazılır, yalnızca sonlu sayılara izin verilir ve sona tam bir LF (`\n`) eklenir. SDK bu kodlamayı uygular ve derleme sonrası kimliği yazdırır.

Nesneler katıdır; bilinmeyen anahtarlar reddedilir. Bildirimler şema ve host API sürüm 1'i, İngilizce ve Türkçe metni, küçük harfli ters alan adlı kimlikleri, sayısal `major.minor.patch` sürümünü, Windows/Linux hedeflerini, izin listesini, eylemleri, türlenmiş durumu ve UI denetimlerini kullanır. Sayısal alanların tamsayı sınırları, metin alanlarının azami uzunluğu olmalıdır. Diziler ve iç içe nesneler sınırlıdır. Girdi, çıktı ve durum tanımsız alanları reddeder. Çalışma zamanları arasında kanonik yazımı tutarlı tutmak için sayısal sınır ve kaydırıcı adımlarında tamsayı kullanın.

Çalışma zamanı yalnızca `memory` (iki sayfa) ve `run(i32 entry, i32 inputLen) -> i32 outputLen` dışa aktarımına sahip sınırlı Core Wasm kabul eder. Host girdi JSON'unu 0. konuma (en fazla 8.192 bayt), konuk çıktı JSON'unu 32.768. konuma (en fazla 32.768 bayt) yazar. İzin verilen tek içe aktarım `rchub.invoke` olup, paket, işlem ve kurulu izin kontrollerinden sonra türlenmiş JSON yükünü yerel yeteneğe iletir. WASI, başlangıç işlevi, başka içe aktarım, keyfi şablon, ifade değerlendirme, dosya yolu, kanca, arşiv veya bağımlılık çalıştırma yoktur. Her işlem mevcut yakıt, bellek ve worker sınırları altında yeni bir konuk deposu oluşturur.

## İzinler ve kurulum

İzinler `manifest.permissions` içindeki sıralı ve tekrarsız yetenek kimlikleridir. Her eylem bildirilmiş bir yeteneği kullanmalıdır. Kurulumda kaynak/risk açıklaması ve istenen izinler gösterilir; telefondan veya PC'den açık onay gerekir. İzin yeniden bağlantı ve uygulama başlatma boyunca saklanır; bu izin dahilindeki kullanım her eylemde tekrar sorulmaz. Her işlem yine de mevcut cihaz sahibini doğrular, kurulu paketi ve izni denetler, yetenek ve platform uygunluğunu kontrol eder, türlenmiş girdi ve çıktıyı doğrular. Eksik, geçersiz veya desteklenmeyen izin güvenli biçimde reddedilir.

İlk kurulum mevcut sürümün üzerine yazmaz. Yeni sürüm için kurulu paketi kaldırın, yeni paketi inceleyin ve tekrar kurun. İzinler paket kimlikleri/sürümleri arasında taşınmaz veya devralınmaz. Panel açılışı, bildirilen ve bağımsız olarak argümansız bir okuma eylemiyle durumu gözlemleyebilir; host yazma eylemini otomatik tekrar oynatmaz.

## Yeteneklerin durumu

Bir adın bildirim şemasında kabul edilmesi, sağlayıcısının uygulandığı anlamına gelmez. Mevcut Rust yerel yönlendiricisi aşağıdaki Windows uygulamalarını içerir; platform ve paket uygunluğu çalışma anında denetlenir.

| Yetenek | Mevcut durum | Etki / sonuç |
| --- | --- | --- |
| `audio.read` | Windows'ta hazır | Varsayılan çıkış ses düzeyi ve sessiz durumunu okur. |
| `audio.write` | Windows'ta hazır | Sınırlı ses düzeyini ve/veya sessiz durumunu ayarlar; gözlemlenen durumu döndürür. |
| `system.read` | Hazır | İşletim sistemi, bilgisayar adı, CPU, bellek ve çalışma süresini okur. |
| `browser.open` | Windows'ta hazır | HTTP(S) adresini varsayılan tarayıcıda açar. |
| `power.write` | Windows'ta hazır | Kilitleme, uyku, kapatma veya yeniden başlatma. Kapatma/yeniden başlatma önemli sonuç doğurur. |
| `keyboard.write` | Windows'ta hazır | Sınırlı metni yazar veya kısıtlı kısayol gönderir. |
| `media.write` | Windows'ta hazır | Oynat/duraklat, sonraki veya önceki parçayı kontrol eder. |
| `clipboard.read` | Windows'ta hazır | Panodaki sınırlı metni okur. |
| `clipboard.write` | Windows'ta hazır | Panodaki sınırlı metni değiştirir ve gözlemlenen metni döndürür. |
| `apps.read` | Windows'ta hazır | Sınırlı sayıda süreç adını ve PID'yi listeler. |
| `apps.write` | Windows'ta hazır | PID ile bir süreci sonlandırır. Hedefi ve riski inceleyin. |
| `display.read`, `display.write` | Windows'ta uygulandı | Birincil fiziksel monitörde yalnız DDC/CI parlaklık. Destek yoksa okuma `available:false` döner, yazma reddedilir. |
| `screenshot.read` | Windows'ta uygulandı | Birincil ekranın en fazla 320×180 PNG önizlemesi; 30.000 karakterlik data URL sınırı için gerekirse küçültülür. |
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
| input | Metin türünde eylem argümanı; yazma yereldir, açık gönderme eylemi çalıştırır. |
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

Bildirim, UI sürüm 1'i, yerleşik simgeyi, tema rengi belirtecini ve sınırlı denetim ağacını tanımlar. Kimlikler benzersiz küçük harfli tanımlayıcılardır. Yerleşim düğümleri (`section`, `row`, `column`, `tabs`) çocuk içerebilir. Bağlar doğrudan tanımlı durum veya eylem girdisi alanlarını gösterir. Yaprak denetimler eylem ve girdi argümanı, sabit skaler değer, sayısal aralık, yerelleştirilmiş seçenek veya türlenmiş liste satırı alanı belirtebilir. Onay, izinler, hakkında ve kaldırma arayüzü hosta aittir.

Mobil renderer şu denetimleri uygular: `section`, `row`, `column`, `tabs`, `button`, `slider`, `toggle`, `select`, `input`, `progress`, `image`, `list`, `text`, `value`, `status`. Denetimlerin geçerli şemaları ve karşılık gelen durum/çıktı değerleri olmalıdır; geçersiz veya kullanılamayan eylemler çalışır denetim olarak sunulmamalıdır. Görsel değerleri PNG, JPEG veya WebP data URL ile sınırlıdır. Listeler için metin içine yapılandırılmış veri kodlamak yerine sınırlı türlenmiş dizi ve nesneler kullanın.

## İçe aktarma ve Marketplace

Topluluk paketleri katalog kaydı olmadan içe aktarılabilir. Mobilde `.rchub-plugin.json` dosyasını metin olarak açın ve içeriğinin tamamını içe aktarma alanına yapıştırın; bu akışta paket dosyası seçici yoktur. PC'de yerel paket yolunu girebilir veya JSON içeriğini yapıştırabilirsiniz. Her iki yol da sınırlı paketi doğrular ve kurulum onayından önce yayıncı/kaynak açıklamasını, lisansı, hedefleri ve izinleri gösterir. Kaynak ve yayıncı alanları paket içindeki beyanlardır; doğrulanmış kaynak bilgisi değildir.

Mobil çalışma alanı ve yönetim penceresi yalnızca sunum katmanıdır: yüklü kartlar ana makine envanterinden gelir, Mağaza ızgarası gerçek katalog/arama/kategori/sayfalama uçlarını kullanır ve yetkili bir PC istemcisi bağlı değilken yükleme veya kaldırma devre dışı kalır. PC çevrimdışıyken Mağazaya göz atmak mümkündür; göz atmak hiçbir paketi kurmaz veya çalıştırmaz.

Mevcut katalog DEV ortamında depoya ait paket dosyalarını ve metaveriyi sunar. Katalog rotaları keyfi URL'lerden indirme yapmaz, yayıncıları doğrulamaz, inceleme veya indirme sayaçları sağlamaz. Katalogda bulunmak paketi kurmaz veya çalıştırmaz; kurulum ve sonraki çağrılar sahip doğrulamalı işlemlerdir.

## Örnek paketler

Public kaynaklar, SDK ve bu rehber
[CetTek/rchub-plugins](https://github.com/CetTek/rchub-plugins) deposundadır.
Resmi 0.1.1 paketleri sabit `v0.1.1` kaynak etiketine bağlanır. Kaynak adresi
değişikliği yeni paket hash'leri üretir. Kurulu 0.1.0 paketleri/izinleri taşınmaz;
public kaynak bağlantısını kullanmak için eski kurulumu kaldırıp 0.1.1 kurulumunu
açıkça onaylayın.

`plugins/official/` altında depoya ait on iki MIT paket bulunur: Audio, System,
Browser, Power, Keyboard, Media, Clipboard, Apps, Display, Screenshot, Speech ve
Effects. “Official” depo etiketidir; kriptografik imza değildir. Bildirimler ve
`guest.wat` kaynak olarak incelenebilir. Paketleri yeniden üretmek için
`python tools/plugins/build_official.py` kullanın.

Bu DEV dilimi eski menülerin her seçeneğini taşımaz: Apps işlemleri listeler ve
sonlandırır, uygulama başlatmaz; Screenshot tam çözünürlük yerine küçük önizleme
verir; Effects sahte BSOD veya ekran sallama yerine sistem sesi sunar. Windows
sağlayıcılarının derleme ve sahte sağlayıcı kapsamı vardır; donanım, yerel yazma
işlemleri ve fiziksel telefondaki davranış canlı doğrulama bekler. Uygulanmış olmak,
bu PC'de canlı çalıştırıldığının kanıtı değildir.
