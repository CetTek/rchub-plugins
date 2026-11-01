# Speech / Metni Seslendir

MIT reference plugin. See `manifest.json` and `guest.wat` for full source.
Rebuild from repository root with `python tools/plugins/build_official.py`.

## English

Version 0.3.2 restores a Turkish/English language selector and a multiline message
of up to 200 characters. Send shows the message on the Windows PC and speaks it
for up to 10 seconds. Closing the message stops playback. The selected message
text is sent to the Google Translate TTS HTTPS endpoint for Turkish/English speech.
If Google TTS is unavailable, speech falls back to an installed Windows voice for
the selected language. If neither provider is available, the operation reports an error.
The shared host renderer owns the send button, character count and operation feedback.

The `speak` action keeps `text` required and adds optional `language` (`tr` or `en`).
Older installed text-only packages retain their existing default-voice behavior and
2,000-character native limit. This package requests the same `speech.write` permission;
a compatible host may offer its reviewed same-permission update. Upgrading the host
is required for draft selects and the language/message behavior. Source tags in the
manifest identify the intended release; changing files locally does not publish it.

## Türkçe

0.3.2 sürümü Türkçe/İngilizce dil seçimini ve en fazla 200 karakterlik çok satırlı
iletiyi geri getirir. Gönderme işlemi Windows PC’de iletiyi gösterir ve en fazla
10 saniye seslendirir. İletiyi kapatmak sesi durdurur. Seçilen ileti metni Türkçe/İngilizce
seslendirme için Google Translate TTS HTTPS uç noktasına gönderilir. Google TTS
kullanılamıyorsa seçilen dilin kurulu Windows sesine geçilir. İki sağlayıcı da
kullanılamıyorsa işlem hata verir. Gönder düğmesi,
karakter sayacı ve işlem geri bildirimi ortak ana uygulama arayüzüne aittir.

`speak` eyleminde `text` zorunlu kalır; `language` (`tr` veya `en`) isteğe bağlıdır.
Eski yalnızca metin gönderen kurulumların varsayılan ses davranışı ve 2.000 karakterlik
yerel sınırı korunur. Paket aynı `speech.write` iznini ister; uyumlu ana uygulama
incelenmiş aynı izinli güncellemeyi sunabilir. Taslak dil seçimi ve ileti/ses davranışı
için ana uygulamayı da güncelleyin. Manifestteki kaynak etiketi hedeflenen sürümü
belirtir; yerel dosya değişikliği sürümü yayımlamaz.

## Native package migration / Yerel paket geçişi

**EN:** `native-source.json` retains this package’s complete action schemas and
controls. The executable is compiled from `../native-runtime`; it executes OS APIs
itself, not a host WASM proxy. It is **not sandboxed**. Installation must disclose
its exact source, native risk and data behavior once; the host still authenticates
and checks current installed authority for every operation. Old V1 files remain
only until the native build and behavior checks have been completed; their grants
are not copied to a native package.

Shows a closable message and sends its text to Google TTS. Falls back to an installed Windows voice. Temporary audio is deleted by the owned helper.

From the RcHub repository root, build with the real public repository URL:

```text
python tools/plugins/build_official_native.py --package speech --platform windows --repository https://github.com/OWNER/REPOSITORY --output dist/official-native
```

Replace OWNER/REPOSITORY with the actual package repository. The output
`dist/official-native/speech/` is one public repository root containing
`rchub-app.json` plus the actual hash-bound executable and sources. Commit those
files at that repository’s root; a monorepo subdirectory URL is not supported.
There is no checked-in fake executable/hash. The builder refuses an incompatible
contract or an existing destination instead of silently dropping behavior.

**TR:** `native-source.json` tüm işlem şemalarını ve kontrolleri korur. Gerçek
çalıştırılabilir dosya `../native-runtime` kaynaklarından derlenir ve OS API’lerini
kendisi çağırır; host WASM vekili kullanmaz ve **sandbox içinde değildir**.
Kurulumda bir kez tam kaynak/risk/veri davranışı açıklanıp onaylanmalıdır; her
işlemde mevcut kimlik ve kurulu yetki kontrolleri host tarafından sürdürülür.
V1 dosyaları yerel derleme ve davranış doğrulaması bitene kadar korunur; eski
izinler yerel pakete otomatik aktarılmaz. Yukarıdaki komutta gerçek public GitHub
repo adresini kullanın; üretilen paket klasörünün içeriğini o reponun köküne koyun.

Kapatılabilir ileti gösterir ve metni Google TTS’ye gönderir. Kurulu Windows sesiyle yedeklenir. Geçici ses, sahip olunan yardımcı tarafından silinir.
