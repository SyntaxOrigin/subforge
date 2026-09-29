# SubForge

**SRT, ASS ve WebVTT altyazılarını okuyan, altı güvenilirlik kuralıyla denetleyen,
toplu zaman kaydıran ve biçim dönüştüren metin aracı.**

SubForge'in varlık sebebi şudur: çoğu altyazı hatası çeviri bilgisiyle değil
**zamanlama ve okunabilirlik** ilgilidir. Replik görüntüde görünmeden başlar,
konuşma bitmeden biter, okunacak kadar uzun kalır ama oyuncu zaten atlama
yapmıştır. SubForge bu hataları kural tabanlı bir denetimle yakalar, kullanıcıya
**replik numarası + ölçülen değer + eşik + öneri** olarak gösterir ve hiçbir
düzeltmeyi kullanıcı onayı olmadan dosyaya yazmaz.

Harici kütüphane kullanmaz; tüm biçim ayrıştırma, kodlama çözümleme ve kural
motoru Rust standart kütüphanesiyle yazılmıştır.

---

## Özellikler

- **SRT, ASS/SSA ve WebVTT okuma + yazma.** Üç biçim de kendi ayrıştırıcısıyla
  (hazır kütüphane yok) okunur ve yazılır. ASS'te `[Script Info]`, `[V4+ Styles]`
  ve `[Events]` bölümleri ile `Format:` alan listeleri korunur; SSA'nın eski
  `[V4 Styles]` biçimi de desteklenir.
- **Hata toleranslı ayrıştırma.** Bozuk zaman damgası, eksik `-->` ayracı, boş
  metin veya eksik alan sayısı dosyayı düşürmez; hata **satır numarasıyla**
  kaydedilir, replik yine de eklenir ve denetim devam eder.
- **Altı güvenilirlik kuralı** (aşağıdaki tablo).
- **Karakter kodlaması denetimi.** Dosya bayt düzeyinde okunur; UTF-8, UTF-8 BOM,
  UTF-16 LE/BE ve Windows-1254 tanınır, **karışık kodlama** satır düzeyinde
  yakalanır. `std::fs::read_to_string` yerine `std::fs::read` kullanılır çünkü
  bozuk baytı **görebilmek** bu modülün varlık sebebidir.
- **Toplu zaman kaydırma.** Mutlak (ms/saniye) ve yüzdesel kaydırma; 0'ın altına
  düşen replikler kırpılır ve kırpılan adet raporda bildirilir.
- **Kare hızı uyumu.** 23.976, 25, 29.97 gibi değerler **NTSC kesri** olarak
  (`24000/1001`) saklanır; kare sınırına oturtma tam sayı aritmetiğiyle ve
  yuvarlama hatası birikmeden yapılır.
- **Biçim dönüştürme ve kayıp özellik raporu.** SRT ⇄ ASS ⇄ WebVTT dönüşümünde
  kaybolan her özellik **açıkça listelenir**, sessizce atılmaz.
- **JSON ve terminal tablo çıktısı.** Aynı veriden üretilir; `--json` ile
  makine okuyan kanıt raporu alınır.
- **Yapılandırılabilir eşikler.** JSON profil dosyasıyla değiştirilir; eksik alanlar
  varsayılan değerlerle dolar.

### Güvenilirlik kuralları

| Kural | Tetik koşulu | Varsayılan eşik |
|---|---|---|
| `okuma_hizi` | Saniyedeki karakter sayısı üst sınırı aşıyor | 17 karakter/sn |
| `asgari_dinlenme` | Replik, gereken en kısa süreden kısa kalıyor | 1,0 sn **veya** 6 karakter/sn (büyüğü) |
| `satir_uzunlugu` | Satır ekran sınırını aşıyor veya satır sayısı fazla | 42 karakter, en fazla 2 satır |
| `cakisan_araliklar` | İki replik aynı anda ekranda görünebiliyor | 0 ms boşluk |
| `sira_bozuklugu` | Süre negatif, süre sıfır veya başlangıçlar artan değil | 0 ms |
| `kodlama_bozuklugu` | BOM, bozuk bayt dizisi, karışık kodlama, `U+FFFD` | BOM'suz UTF-8 |

> **Bu eşiklerin hiçbiri ölçülmemiştir.** Fikir raporunun kendisi de
> "eşik değerlerinin çoğu ölçülmemiştir" demektedir. Ölçülmüş tek sayı
> kare hızı formülüdür (bkz. `## Test`).

---

## Kurulum

Gereksinim: Rust **1.74** veya üzeri (beyan edilen MSRV; bu depoyu geliştiren ve
test eden ortam **1.98.1**). MSRV'nin kendisi ayrı bir araç zinciriyle
doğrulanmamıştır — bkz. `## Bilinen Sınırlamalar`.

```
$ cargo --version
cargo 1.98.1 (797e8a9bc 2026-08-05)
$ rustc --version
rustc 1.98.1 (48a229cea 2026-09-01)
```

Yerel derleme:

```
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 18.58s
```

Sistem geneline kurulum:

```
$ cargo install --path .
  Installing %USERPROFILE%\.cargo\bin\subforge.exe
   Installed package `subforge v0.1.0 (C:\...\03-subforge)` (executable `subforge.exe`)
```

Kurulum **zorunlu değildir**; `cargo run --release -- <komut>` da çalışır.
Araç tek dosyalık, statik bağlantılı ve çevrimdışıdır; kurulum gerektirmez.

---

## Kullanım

Aşağıdaki **her komut gerçekten çalıştırılmıştır**; çıktılar kopyalanmıştır.
`ornek/` klasöründeki üç örnek dosya SRT, ASS ve WebVTT'yi temsil eder.

### 1. Yardım

```
$ subforge --help
SubForge, altyazi dosyalarini bayt duzeyinde okur, alti guvenilirlik kuraliyla denetler ve kullaniciya acikca isteyerek zaman kaydirma, normalizasyon ve bicim donusturme yapar. Denetim hicbir dosyayi degistirmez.

Usage: subforge.exe <COMMAND>

Commands:
  check      Altı güvenilirlik kuralını çalıştırır; dosyayı değiştirmez
  report     Dosyanın istatistik özetini üretir; dosyayı değiştirmez
  shift      Tüm replikleri sabit miktar kaydırır
  normalize  Zaman damgalarını kare sınırına oturtur, sıralar ve numaralandırır
  convert    Altyazıyı başka bir biçime çevirir ve kayıp raporu üretir
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### 2. Denetim — `check`

```
$ subforge check ornek/ornek.srt
=== subforge check ===
dosya    : ornek/ornek.srt
bicim    : srt
kodlama  : utf-8 (BOM: yok, bozuk bayt: 0, karisik: yok)
replik   : 6  (7 satir, 00:00:14.000 sure)

kural             adet  aciklama
----------------- ----- ---------------------------------------------
okuma_hizi            3  saniyedeki karakter sayisi okuma hizi ust sinirini asiyor
asgari_dinlenme       5  replik okunabilmesi icin gereken en kisa sureden kisa
satir_uzunlugu        1  satir karakter sayisi ekran sinirini asiyor veya satir sayisi fazla
cakisan_araliklar     1  iki replik ayni anda ekranda gorunebiliyor
sira_bozuklugu        3  sira bozuk veya sure negatif
kodlama_bozuklugu     0  karakter kodlamasi bozuk (BOM, bozuk bayt, karisik kodlama)

  #  replik  seviye  kural               olcum / esik
  -  ------  -------  ------------------  ------------------------
  1  1       uyari    okuma_hizi          19.0 karakter/sn / 17.0 karakter/sn
  2  1       uyari    asgari_dinlenme     2000 ms (38 karakter) / 6334 ms
  3  2       uyari    asgari_dinlenme     2240 ms (35 karakter) / 5834 ms
  4  2       kritik   cakisan_araliklar   2620 ms cakisma / > 0 ms
  5  3       uyari    okuma_hizi          46.9 karakter/sn / 17.0 karakter/sn
  6  3       uyari    asgari_dinlenme     1300 ms (61 karakter) / 10167 ms
  7  3       kritik   sira_bozuklugu      00:00:02,900 < 00:00:03,280 / baslangic artan sirada
  8  3       uyari    satir_uzunlugu      61 karakter / 42 karakter
  9  4       uyari    asgari_dinlenme     0 ms (25 karakter) / 4167 ms
 10  4       kritik   sira_bozuklugu      0 ms / > 0 ms
 11  5       kritik   sira_bozuklugu      -2000 ms / 0 ms
 12  6       uyari    okuma_hizi          21.0 karakter/sn / 17.0 karakter/sn
 13  6       uyari    asgari_dinlenme     2000 ms (42 karakter) / 7000 ms

sonuc: KALDI (13 bulgu, 4 kritik)
```

Bu dosya bilerek bozulmuştur: replik 2 ve 3 çakışıyor, replik 4'ün süresi sıfır,
replik 5'in süresi negatif ve replikler 3'ten sonra artan sırada değil.
Denetim **hiçbir değişiklik yapmamıştır** — kaynak dosya olduğu gibi durur.

### 3. Özet rapor — `report`

```
$ subforge report ornek/ornek.srt
=== subforge report ===
dosya   : ornek/ornek.srt
bicim   : srt
replik  : 6
satir   : 7
sure    : 00:00:14.000
kodlama : utf-8
bulgu   : 13
kritik  : 4
```

### 4. Toplu zaman kaydırma — `shift`

```
$ subforge shift ornek/ornek.srt --out ornek/shifted.srt --seconds 1.4 --sort
=== subforge shift ===
dosya    : ornek/ornek.srt
cikti    : ornek/shifted.srt
kaydirma : 1400 ms
etkilenen: 6
kalan cakisma: 2
```

Kaynak dosya **korunur**; sonuç `--out` ile verilen dosyaya yazılır. Yüzdesel
kaydırmada negatif değerler `=` ile yazılmalıdır:

```
$ subforge shift ornek/ornek.srt --out ornek/geri.srt --percent=-10
=== subforge shift ===
dosya    : ornek/ornek.srt
cikti    : ornek/geri.srt
kaydirma : -1400 ms
etkilenen: 6
kirpilan : 1 replik 0.00 saniyeye kirpildi
kalan cakisma: 1
```

### 5. Normalizasyon — `normalize`

```
$ subforge normalize ornek/shifted.srt --out ornek/normalize.srt --fps 25 --sort --number
=== subforge normalize ===
dosya    : ornek/shifted.srt
cikti    : ornek/normalize.srt
kare hizi: 25
kareye oturtulan: 1
siralanan: 0
numaralanan: 2
toplam sure: 00:00:15.400
```

NTSC oranı da kabul edilir; `23.976`, `24000/1001` kesrine çevrilir:

```
$ subforge normalize ornek/shifted.srt --out ornek/n23976.srt --fps 23.976
=== subforge normalize ===
dosya    : ornek/shifted.srt
cikti    : ornek/n23976.srt
kare hizi: 23.976
kareye oturtulan: 6
siralanan: 0
numaralanan: 0
toplam sure: 00:00:15.390
```

### 6. Biçim dönüştürme ve kayıp raporu — `convert`

```
$ subforge convert ornek/ornek.ass --out ornek/donusturulmus.vtt --to vtt
=== subforge convert ===
dosya    : ornek/ornek.ass
cikti    : ornek/donusturulmus.vtt
donusturme: ass -> vtt
replik   : 4

kaybolan ozellikler:
  - ASS stiller
      deger : 2 stil
      neden : WebVTT `STYLE` blogu kullanilir, ASS `Style:` tablosu degil
  - replik stili
      deger : Default
      neden : WebVTT replik ici stil tasimaz; CSS sinifi kullanilir
  - Comment satirlari
      deger : gizli replikler
      neden : WebVTT `NOTE` blogu kullanilir; icerik replik olarak tasindi
```

### 7. Özel kural profili — `--rules`

```
$ subforge check ornek/ornek.srt --rules ornek/kurallar.json
...
sonuc: KALDI (14 bulgu, 4 kritik)
```

### 8. JSON çıktısı

```
$ subforge convert ornek/ornek.ass --out ornek/x.vtt --to vtt --json
{
  "surum": "0.1.0",
  "dosya": "ornek/ornek.ass",
  "cikti": "ornek/x.vtt",
  "kaynak_bicim": "ass",
  "donusturme": {
    "hedef": "vtt",
    "replik_sayisi": 4,
    "kayiplar": [
      {
        "ozellik": "ASS stiller",
        "deger": "2 stil",
        "gerekce": "WebVTT `STYLE` blogu kullanilir, ASS `Style:` tablosu degil"
      },
      {
        "ozellik": "replik stili",
        "deger": "Default",
        "gerekce": "WebVTT replik ici stil tasimaz; CSS sinifi kullanilir"
      },
      {
        "ozellik": "Comment satirlari",
        "deger": "gizli replikler",
        "gerekce": "WebVTT `NOTE` blogu kullanilir; icerik replik olarak tasindi"
      }
    ]
  }
}
```

`check --json` çıktısı ise `kural_ozeti`, `esikler`, `bulgular` (her biri
`olcum`/`esik`/`mesaj`/`oneri` dörtlüsüyle), `tanimsiz_stiller` ve `sozdizimi`
alanlarını içerir.

---

## Test

```
$ cargo test
   Compiling subforge v0.1.0 (%USERPROFILE%\Desktop\Projeler\projects\03-subforge)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.44s
     Running unittests src\lib.rs (target\debug\deps\subforge-9c1f1d5b5a1e0b7c.exe)

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps\subforge-0285ba3a434ddef5.exe)

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\bicimler.rs (target\debug\deps\bicimler-f27cc7727a4a52ee.exe)

running 44 tests
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s

     Running tests\entegrasyon.rs (target\debug\deps\entegrasyon-d8cf449095861436.exe)

running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s

     Running tests\islemler.rs (target\debug\deps\islemler-8bfa547130e94e94.exe)

running 24 tests
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s

     Running tests\kurallar.rs (target\debug\deps\kurallar-a73712a317e4489c.exe)

running 35 tests
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s

     Running tests\zaman_kodlama.rs (target\debug\deps\zaman_kodlama-4787ea0dfa335a9.exe)

running 28 tests
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s

   Doc-tests subforge

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**test sonucu: okunan 149; geçen 149; başarısız 0**

### Test kapsamı

| Dosya | Test | Kapsam |
|---|---|---|
| `tests/zaman_kodlama.rs` | 28 | Zaman damgası ayrıştırma/yazma, kesir ölçekleme, gidiş-dönüş, kare hızı (23.976→24), UTF-8/BOM/UTF-16 LE+BE/CP1254/karışık/bozuk bayt |
| `tests/bicimler.rs` | 44 | Üç biçimde okuma→yazma→okuma gidiş-dönüşü, bozuk zaman damgası, eksik `-->` ayracı, boş dosya, BOM'lu giriş, çok satırlı altyazı, stil referansı tanımsız, biçim tespiti |
| `tests/kurallar.rs` | 35 | Altı kuralın pozitif/negatif örnekleri, eşik tam/eşik−1/eşik+1 sınırları, biçim bağımsızlığı, özel profil |
| `tests/islemler.rs` | 24 | Kaydırma (ileri/geri/yüzde/kırpma), kare hızı normalizasyonu, sıralama, numaralandırma, dönüşüm ve kayıp raporu |
| `tests/entegrasyon.rs` | 18 | Geçici dosya üzerinden aç→denetle→dönüştür→kaydet, kodlama tespiti, profil yükleme, terminal/JSON çıktı |

Kare hızı formülü fikir raporunun §07'deki örneğiyle birebir doğrulanır:
`3,280 sn × 30 = 98,4 → kare 98 → 98/30 = 3,2667 sn` (`kare_hizi_rapor_ornegini_uygular`).

Diğer kalite kapıları:

```
$ cargo build --release
    Finished `release` profile [optimized] target(s)
$ cargo clippy --all-targets -- -D warnings
    (çıktı yok — uyarı ve hata yok)
$ cargo fmt --check
    (çıktı yok — biçimlendirme doğru)
```

---

## Proje Yapısı

```
03-subforge/
├── Cargo.toml
├── Cargo.lock
├── LICENSE.txt              MIT lisans metni
├── README.md
├── .gitignore
├── ornek/                   README'de kullanılan gerçek örnek dosyalar
│   ├── ornek.srt
│   ├── ornek.ass
│   ├── ornek.vtt
│   └── kurallar.json
├── src/
│   ├── main.rs              CLI kabuğu (yalnızca argüman ayrıştırma ve çıkış kodu)
│   ├── lib.rs               modül yönlendirmesi ve yeniden dışa aktarımlar
│   ├── hata.rs              SubForgeHata (elle Display + Error impl)
│   ├── kodlama.rs           bayt düzeyinde okuma, BOM/UTF-16/CP1254/karışık tespiti
│   ├── zaman.rs             Zaman, KareHazi (NTSC kesirli), ayrıştırma/yazma
│   ├── model.rs             Belge, Replik, Stil, Bicim, SozdizimiKaydi
│   ├── bicim.rs             biçim tespiti, okuma/yazma yüzeyi, dönüşüm kayıpları
│   ├── srt.rs               SubRip ayrıştırıcı/yazıcı
│   ├── ass.rs               ASS/SSA ayrıştırıcı/yazıcı
│   ├── vtt.rs               WebVTT ayrıştırıcı/yazıcı
│   ├── kural.rs             altı kural, eşikler, bulgu üretimi
│   ├── islem.rs             kaydırma, normalizasyon, dönüştürme
│   ├── rapor.rs             JSON ve terminal çıktısı
│   └── cli.rs               clap tanımları ve komut yürütme
└── tests/
    ├── zaman_kodlama.rs
    ├── bicimler.rs
    ├── kurallar.rs
    ├── islemler.rs
    └── entegrasyon.rs
```

Katman ayrımı (fikir raporu §06): **kural motoru biçim ayrıntılarını bilmez.**
Aynı kural kümesi SRT, ASS ve VTT dosyalarında birebir aynı çalışır; yeni bir
biçim eklendiğinde `kural.rs` değişmez. Bu ayrımın testi `kurallar.rs` içindeki
`kurallar_ass_uyumlu_belgede_ayni_sonucu_verir` ve
`kurallar_vtt_uyumlu_belgede_ayni_sonucu_verir` testleridir.

---

## Yapılandırma

Yapılandırma dosyası program dizininde değil, komut satırından verilir:
`--rules <dosya>`. Bu, taşınabilir dağıtım ilkesiyle (program dizinine yazma)
tutarlıdır; araç çalışma zamanında hiçbir dosya yazmaz.

`ornek/kurallar.json` örneği:

```json
{
  "okuma_hizi": 15.0,
  "asgari_dinlenme_ms": 1200,
  "asgari_hiz": 5.0,
  "en_fazla_karakter": 38,
  "en_fazla_satir": 2,
  "kabul_edilebilir_bosluk_ms": 0
}
```

| Alan | Varsayılan | Birim | Etkisi |
|---|---|---|---|
| `okuma_hizi` | `17.0` | karakter/sn | Bu değerin üstü `okuma_hizi` bulgusu üretir |
| `asgari_dinlenme_ms` | `1000` | ms | Mutlak asgari kalış süresi |
| `asgari_hiz` | `6.0` | karakter/sn | `asgari_dinlenme` için gereken süre bu hızdan da hesaplanır; iki değerden **büyüğü** kullanılır |
| `en_fazla_karakter` | `42` | karakter | `satir_uzunlugu` eşiği |
| `en_fazla_satir` | `2` | satır | Ekranda aynı anda okunabilen en fazla satır |
| `kabul_edilebilir_bosluk_ms` | `0` | ms | İki replik arasında bu boşluktan fazlası `cakisan_araliklar` üretir |

Dosyada **eksik alanlar varsayılan değerlerle doldurulur**; yalnızca değiştirmek
istediğiniz alanları yazabilirsiniz.

### Komut bayrakları

| Bayrak | Komut | Varsayılan | Etkisi |
|---|---|---|---|
| `--format <srt\|ass\|vtt>` | check, report, shift, normalize, convert | içerikten tespit | Biçimi dosya uzantısına bakmadan zorlar |
| `--rules <DOSYA>` | check, report | — | Eşikleri JSON dosyasından yükler |
| `--json` | tümü | terminal | Çıktıyı JSON olarak üretir |
| `--tolerant` | check | `false` | Bulgu olsa bile çıkış kodu `0` döner |
| `--out <DOSYA>` | shift, normalize, convert | zorunlu | Sonucun yazılacağı dosya |
| `--ms <MS>` | shift | — | Mutlak kaydırma (milisaniye) |
| `--seconds <SANIYE>` | shift | — | Mutlak kaydırma (saniye) |
| `--percent <YUZDE>` | shift | — | Toplam sürenin yüzdesi kadar kaydırma |
| `--fps <FPS>` | shift, normalize | — | Kare hızına oturtma |
| `--sort` | shift, normalize | `false` | Replikleri başlangıç zamanına göre sıralar |
| `--number` | normalize | `false` | Replik numaralarını 1'den başlatır |
| `--to <BICIM>` | convert | zorunlu | Hedef biçim |

`--ms`, `--seconds` ve `--percent` **aynı anda kullanılamaz**; bu durumda araç
hata verir. Negatif ondalık değerler `=` ile yazılmalıdır (`--seconds=-1.4`);
`--seconds -1.4` biçimi `clap` tarafından "bilinmeyen bayrak" sayılır.

### Çıkış kodları

| Kod | Anlamı |
|---|---|
| `0` | Başarılı (veya `check` için "kritik bulgu yok") |
| `1` | Çalışma ortamı hatası: dosya okunamadı/yazılamadı, geçersiz argüman |
| `2` | Sözdizimi hatası veya `clap` ayrıştırma hatası |
| `3` | `check` çalıştı ve kritik bulgu buldu (`--tolerant` ile bastırılır) |

---

## Bilinen Sınırlamalar

Aşağıdakiler **bilinçli** sınırlamalardır; hiçbiri gizlenmiyor.

### Rapor planından sapma (MANIFEST.md kart 03)

- **libass ve FFmpeg kullanılmaz.** Fikir raporu gömme (burn-in) ve kapsülleme
  (softsub) için bu C kütüphanelerini önerir. Bunlar v2 aşaması özellikleridir;
  MVP'nin tamamı (biçim okuma/yazma, zamanlama düzenleme, güvenilirlik denetimi)
  saf metin işlemedir. `MANIFEST.md` bu sapmayı onaylar.
- **Gömme (burn-in) yok.** Altyazı videoya basılmaz.
- **Kapsülleme (softsub) yok.** Altyazı MP4 içine akış olarak eklenmez.
- **Grafik arayüz yok.** Terminal arayüzü + JSON çıktısı vardır. Stil editörü,
  ön izleme kutusu ve video oynatıcı yoktur.
- **Stil editörü yok.** ASS stilleri okunur, korunur ve yazılır ama
  **düzenlenemez**; yalnızca metin düzeyinde korunur.
- **Kelime düzeyi zamanlama yok.**
- **Geri alma yığını yok.** Araç komut satırı aracıdır; oturum, kuyruk ve geri
  alma yığını bir arayüzün işidir. Geri alma yerine `shift`/`normalize` **her
  zaman yeni bir çıktı dosyasına** yazar ve kaynağa dokunmaz.
- **Program dizininde `config/`, `cache/`, `logs/` tutulmaz.** Ayar dosyası
  komut satırından verilir; araç çalışma zamanında hiçbir dosya yazmaz.

### Teknik sınırlamalar

- **MSRV doğrulanmamıştır.** `Cargo.toml` `rust-version = "1.74"` beyan eder ve
  kod bu seviyeye uygun yazılmıştır (ör. `div_ceil`, `let-else`, `split_once`
  yoktur), ancak **1.74 araç zinciriyle derlenmemiştir**; yalnızca 1.98.1 ile
  derlenmiş ve test edilmiştir.
- **UTF-16 yazılmaz.** UTF-16 dosyaları **okunur**, ancak tüm çıktılar
  BOM'suz UTF-8'dir. Yazma sırasında `normalize`/`convert` kodlamayı UTF-8'e
  çevirir; bu bir dönüşüm kaybı olarak raporlanmaz çünkü metin kayıpsızdır.
- **Kodlama tespiti içerikten yapılır, uzantıdan değil.** Yanlış uzantılı bir
  dosya yine de doğru biçimde okunur. Yanlış biçimli içerik `BicimTanimliDegil`
  hatası verir.
- **SRT'te konum ayarları (`X1:`, `Y1:`) okunur, yazılmaz.** Bunlar
  oynatıcıya özgüdür ve yazma sırasında atılır; kayıp listesinde **listelenmez**,
  çünkü SubRip'in tanımı bunları zorunlu kılmaz. Bu, MVP'nin kapsam dışıdır.
- **VTT cue kimlik satırı ve konum ayarları yazılmaz.** WebVTT'te isteğe bağlıdır.
- **Kurallar kaba ölçümlerdir.** "Erken başlangıç (0,2 sn)" ve "geç bitiş
  (0,4 sn)" kuralları **uygulanmaz**: bunlar konuşma başlangıç/bitiş
  noktalarını gerektirir ve bir metin aracı bu bilgiye sahip değildir. Bu
  kural "minimum kalış" ve "çakışma" kurallarıyla kısmen örtüşür.
- **Çift boşluk/noktalama ve kare hızı uyumsuzluğu kuralları ayrı kural değildir.**
  Kare hızı uyumu bir kural değil, `normalize --fps` işlemidir (düzeltme kullanıcı
  isteğidir).
- **`Write` işlemi atomik değildir.** Çıktı doğrudan hedef dosyaya yazılır.
  Kısmi yazma riski kabul edilmiştir; kaynak dosya her zaman korunur.
- **Kod yorumları ve terminal/JSON mesajları ASCII Türkçesidir** (Türkçe
  karakter kullanılmadan yazılmıştır). Bu, Windows konsol kod sayfası
  bağımlılığını ortadan kaldırmak içindir; kaynak koddaki doc comment'ler tam
  Türkçedir.
- **Windows dışı platformlar denenmemiştir.** Kod `std` üzerinde ve taşınabilirdir,
  ancak yalnızca Windows 10/11 x64 üzerinde derlenip test edilmiştir.

### Ölçülmemiş iddialar

- Altı kuralın **hiçbir eşiği ölçülmemiştir**; fikir raporundan alınmıştır ve
  raporun kendisi de bunları tahmin olarak işaretler.
- Fikir raporunun bellek (≤ 180 MB), açılış süresi (≤ 550 ms) ve toplu iş
  performansı iddiaları **bu depoda ölçülmemiştir**; SubForge tek dosya
  çalıştıran bir CLI'dir ve bu bütçeler arayüz ve ön izleme tarafından
  belirlenir.
- Gerçek bir altyazı dosyası üzerinde kural gürültüsü ölçülmemiştir. Bu, fikir
  raporunun ilk hafta aksiyon listesindeki 4–5. gün maddesidir ve yapılmamıştır.

### `#[allow]` ve susturma

Kaynak kodda **hiçbir `#[allow(...)]` yoktur**. `clippy::unwrap_used` ve
`clippy::expect_used` uyarıları üretim kodunda etkindir (`#![warn(...)]`); bu
uyarıları yalnızca `#[cfg(test)]` modülleri susturabilir, oysa testler ayrı
`tests/` dosyalarındadır. Üretim kodunda `unwrap()`, `expect()` veya `panic!`
**yoktur**; tüm hatalar `Result` ile döner.

`GeciciDizin` yardımcısının `Drop` impl'i `let _ =` ile temizlik hatasını
susturur. Bu, `Drop` içinden hata döndürülemediği için zorunludur ve
`WORKER_CONTRACT.md` §5.3 tarafından açıkça kabul edilir.

---

## Gelecek Geliştirmeler

Kartın "Ertelenen" listesinden ve doğal sonraki adımlardan:

1. **Stil editörü** — yazı tipi, boyut, kenar, hizalama; metin düzeyinde
   `[V4+ Styles]` alanlarının düzenlenmesi.
2. **Eriken konuşma kuralı** — harici bir konuşma tanıyıcı olmadan
   uygulanamaz; kural motorunun bir "zamanlama sağlayıcı" arayüzüne
   ihtiyacı var.
3. **Geri alma yığını** — değişiklik grubu modeliyle, toplu işlem geri alınabilir.
4. **Dizin düzeyinde toplu iş** — klasördeki tüm altyazılara aynı kaydırmayı
   uygulama ve dosya başına uygulanan kaydırmayı `oturum.json`'a kaydetme.
5. **WebVTT `STYLE` bloklarının CSS olarak korunması** — şu an başlık alanı
   olarak saklanır, yazıldığında kaybolur.
6. **Gömme (burn-in) ve kapsülleme (softsub)** — libass/FFmpeg bağımlılığı
   getirir; bağımlılık politikası izin vermediği için ertelenmiştir.
7. **Kelime düzeyi zamanlama** — yalnızca belirli giriş biçimlerinde mümkündür;
   hangi dosyalarda sunulacağı açıkça belirlenmelidir.

---

## Troubleshooting

### 1. `hata: 'ornek.srt' tanimli degil: dosya SRT, ASS/SSA veya WebVTT degil`

**Belirti** — Dosya açılamıyor, biçim tanınmıyor.
**Neden** — SubForge biçimi **dosya uzantısından değil, içerikten** tespit eder.
İlk anlamlı 20 satırda `WEBVTT`, bir `[Script Info]` / `[V4+ Styles]` / `[Events]`
başlığı veya `-->` içeren bir satır bulunamamıştır.
**Çözüm** — Dosya gerçekten altyazı değilse bu doğru davranıştır. Farklı bir
biçimde kaydedilmiş bir altyazıysa `--format srt|ass|vtt` ile zorlayın.

### 2. `hata: 'x.vtt' okunamadi: Sistem belirtilen dosyayı bulamıyor. (os error 2)`

**Belirti** — Dosya okunamadı.
**Neden** — Yol yanlış yazılmış, dosya taşınmış veya geçici çalışma dizini
temizlenmiş. Komut satırındaki yol, komutun çalıştırıldığı dizine görelidir.
**Çözüm** — `Get-Location` ile bulunduğunuz dizini denetleyin; yolu `.\ornek\`
gibi göreli ya da mutlak yazın.

### 3. `hata: gecersiz arguman: --ms, --saniye ve --yuzde birlikte kullanilamaz`

**Belirti** — `shift` komutu başlamıyor.
**Neden** — Kaydırma miktarı üç farklı biçimde verilmiş. Tek bir kaydırma
miktarı gerekir.
**Çözüm** — Yalnızca birini kullanın: `--ms 1400` **veya** `--seconds 1.4`
**veya** `--percent=-10`.

### 4. `error: unexpected argument '-1' found`

**Belirti** — `shift --percent -10` veya `shift --seconds -1.4` reddediliyor.
**Neden** — `clap`, `-` ile başlayan bir değeri bayrak sanır.
**Çözüm** — Değeri `=` ile bağlayın: `--percent=-10`, `--seconds=-1.4`.

### 5. `error: unexpected argument '--out' found` (eski sürüm)

**Belirti** — `--cikti` bayrağı tanınmıyor.
**Neden** — Çıktı bayrağı adı `--out` olarak değiştirildi.
**Çözüm** — `--cikti` yerine `--out` kullanın. Alan adları Türkçedir, komut
satırı bayrakları İngilizcedir.

### 6. `check` her zaman `sonuc: KALDI` veriyor

**Belirti** — Temiz görünen bir dosyada bile bulgu üretiliyor.
**Neden** — Varsayılan eşiğin ikisi de sıkıdır: asgari kalış **1,0 saniye veya
6 karakter/sn** gerektirir; kısa cümleler bu eşiğin altında kalır. Bu, fikir
raporundaki değerlerin sadeleştirilmiş hâlidir.
**Çözüm** — `--rules` ile kendi profilinizi verin (bkz. `## Yapılandırma`) veya
`asgari_hiz` değerini düşürün. Bulgu sayısını `subforge report` ile görebilirsiniz.

### 7. `normalize` çalıştırdıktan sonra çakışan replik sayısı arttı

**Belirti** — Kare sınırına oturtma sonrası `cakisan_araliklar` bulgusu çoğaldı.
**Neden** — Kare hızı uyumu, her repliği **kendi** kare sınırına taşır; iki
replik farklı karelere oturduğunda aralarındaki boşluk değişir. Bu bir hata
değil, beklenen davranıştır — araç çakışmayı **çözmez**, yalnızca bildirir.
**Çözüm** — Kaydırmayı kare hızı uyumundan **önce** uygulayın
(`shift --fps 25 --sort` tek komutta ikisini yapar), sonra `check` ile yeniden
denetleyin.

---

## Atıflar

Bu bölüm MIT lisansının öngördüğü atıf yükümlülüğünü karşılar.

### Biçim belgeleri (birincil kaynaklar)

- **W3C WebVTT** — WebVTT'nin şekillendirme kaynağı.
  <https://www.w3.org/TR/webvtt1/>
- **SubRip** — SRT biçiminin yaygın açıklaması.
  <https://en.wikipedia.org/wiki/SubRip>
- **Aegisub — ASS biçim belgeleri** — `[V4+ Styles]` / `[Events]` alan listeleri.
  <https://aegisub.org/docs/ass/>
- **SubStation Styles (SSA v4)** — `[V4 Styles]` alan listesi.
  <https://wiki.multimedia.cx/index.php/SubStation_Styles>
- **Windows-1254 (Türkçe tek bayt kodlama)** — 0x80–0x9F aralığının farklılıkları
  ve `Ğ İ Ş ğ ı ş Ÿ` kodları.
  <https://en.wikipedia.org/wiki/Windows-1254>

### Karşılaştırma ve arka plan

- **Subtitle Edit** — açık kaynak altyazı düzenleyici.
  <https://www.nikse.dk/subtitleedit>
- **Aegisub** — ASS biçimine odaklı açık kaynak altyazı düzenleyici.
  <https://aegisub.org/>
- **Netflix Partner Help Center** — altyazı teslim kuralları için kamuya açık
  referans (eşiklerin kaynağı değil, bağlam referansıdır).
  <https://partnerhelp.netflixstudios.com/>

### Rust ekosistemi

- Rust standart kütüphane belgeleri — <https://doc.rust-lang.org/std/>
- Rust edition rehberi (2021) — <https://doc.rust-lang.org/edition-guide/edition-2021/>
- `cargo` yerel rehberi — <https://doc.rust-lang.org/cargo/>
- `clap` — <https://docs.rs/clap/>
- `serde` — <https://serde.rs/> · <https://github.com/serde-rs/json>
- `serde_json` — <https://docs.rs/serde_json/>

Doğrudan kopyalanan kod yoktur. Hiçbir harici altyazı ayrıştırma kütüphanesi
kullanılmamıştır; biçim ayrıştırma, kodlama çözümleme ve kural motoru bu depoda
sıfırdan yazılmıştır.

### Tasarım kaynağı

Bu projenin iç tasarımı şu kavramsal rapordan alınmıştır (yerel dosya, URL yok):
`%USERPROFILE%\Desktop\Fikirler\03-altyazi-atolyesi.html` — "AltyazıAtölyesi —
SubForge", rapor no 03/30, tarih 2026-09-29.

Rapordan birebir uygulananlar: güvenilirlik kural seti ve varsayılan eşikleri
(§05), katman ayrımı (§06), kare hızı formülü ve örneği (§07), kayıp özellik
raporu gereksinimi (§03/§05), "hiçbir düzeltme onaysız uygulanmaz" ilkesi (§03).
Rapordan **sapılanlar** `## Bilinen Sınırlamalar` bölümünde listelenmiştir.

---

## Lisans

**MIT** lisansı. Tam metin `LICENSE.txt` dosyasındadır.

Telif: `Copyright (c) 2026 SubForge contributors`
