//! SRT, ASS ve WebVTT okuma-yazma gidiş-dönüşü ve bozuk girdi testleri.
//!
//! Kapsam: üç biçim için okuma → yazma → yeniden okuma gidiş-dönüşü, bozuk
//! zaman damgası, eksik `-->` ayracı, boş dosya, BOM'lu giriş, çok satırlı
//! altyazı, stil referansı tanımsız ve biçim tespiti.

use subforge::bicim::bicim_tespit;
use subforge::model::{Bicim, OlayTuru, Replik};
use subforge::zaman::Zaman;
use subforge::{ass, kodlama, srt, vtt};

/// Temiz SRT örneği. Son replikten sonra SRT'nin gerektirdiği boş ayraç satırı
/// bulunur; yazıcı da bu satırı üretir.
const SRT_ORNEK: &str = "1\n00:00:01,000 --> 00:00:03,000\nMerhaba dunya\n\n2\n00:00:04,000 --> 00:00:06,500\nIkinci replik\n\n";

/// Çok satırlı SRT örneği. SRT'de satırlar arasında boş satır **olmaz**;
/// boş satır yeni repliği başlatır.
const SRT_COK_SATIR: &str =
    "1\n00:00:01,000 --> 00:00:04,000\nIlk satir\nIkinci satir\nUcuncu satir\n\n";

/// Temiz ASS örneği (bir stil, bir replik).
const ASS_ORNEK: &str = "\
[Script Info]
Title: Ornek
ScriptType: v4+
PlayResX: 1920
PlayResY: 1080

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,48,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,Merhaba dunya
";

/// Temiz WebVTT örneği. Son cue'dan sonra boş ayraç satırı bulunur.
const VTT_ORNEK: &str =
    "WEBVTT\n\n00:01.000 --> 00:03.000\nMerhaba dunya\n\n00:04.000 --> 00:06.500\nIkinci replik\n\n";

fn bos_kodlama() -> kodlama::KodlamaRaporu {
    kodlama::KodlamaRaporu {
        kodlama: "utf-8".to_string(),
        ..kodlama::KodlamaRaporu::default()
    }
}

// ---------------------------------------------------------------- SRT ----

/// SRT okuma → yazma → okuma gidiş-dönüşü birebir aynı belgeyi verir.
#[test]
fn srt_gidis_donus_birebir_ayni() {
    let ilk = srt::ayikla(SRT_ORNEK, &bos_kodlama());
    let metin = srt::yaz(&ilk);
    let ikinci = srt::ayikla(&metin, &bos_kodlama());
    assert_eq!(ilk.replikler, ikinci.replikler);
    assert_eq!(ilk.replik_sayisi(), 2);
    assert_eq!(metin, SRT_ORNEK);
    assert!(ilk.sozdizimi.is_empty());
}

/// SRT'te nokta ayracı kabul edilir (Windows yazılımlarının çoğu böyle yazar).
#[test]
fn srt_nokta_ayraci_kabul_edilir() {
    let metin = "1\n00:00:01.000 --> 00:00:02.000\nNokta ayraci\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].baslangic, Zaman::milis(1000));
    assert!(belge.sozdizimi.is_empty());
}

/// Eksik `-->` ayracı hata kaydı üretir, dosya düşmez.
#[test]
fn srt_eksik_ayrac_sozdizimi_hatasi_uretir() {
    let metin = "1\n00:00:01,000 00:00:02,000\nAyracsiz\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert!(belge.replikler.is_empty());
    assert!(!belge.sozdizimi.is_empty());
    assert!(belge.sozdizimi[0].mesaj.contains("-->"));
    assert_eq!(belge.sozdizimi[0].satir, 2);
}

/// Bozuk zaman damgası hata kaydı üretir, replik yine de eklenir.
#[test]
fn srt_bozuk_zaman_dammgasi_isaretlenir() {
    let metin = "1\naa:bb:cc --> dd:ee:ff\nBozuk zaman\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 1);
    assert_eq!(belge.replikler[0].baslangic, Zaman::SIFIR);
    assert!(!belge.sozdizimi.is_empty());
}

/// Boş SRT dosyası hata üretmez, boş belge döner.
#[test]
fn srt_bos_dosya_hata_uretmez() {
    let belge = srt::ayikla("", &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 0);
    assert!(belge.sozdizimi.is_empty());
    assert_eq!(srt::yaz(&belge), "");
}

/// Boş metinli replik işaretlenir.
#[test]
fn srt_bos_metin_isaretlenir() {
    let metin = "1\n00:00:01,000 --> 00:00:02,000\n\n2\n00:00:03,000 --> 00:00:04,000\nDolgu\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 2);
    assert_eq!(belge.replikler[0].metin, "");
    assert_eq!(belge.sozdizimi.len(), 1);
}

/// Çok satırlı altyazı metni korunur.
#[test]
fn srt_cok_satirli_altyazi_korunur() {
    let belge = srt::ayikla(SRT_COK_SATIR, &bos_kodlama());
    let r = &belge.replikler[0];
    assert_eq!(r.satir_sayisi(), 3);
    assert_eq!(
        r.satirlar(),
        vec!["Ilk satir", "Ikinci satir", "Ucuncu satir"]
    );
    let metin = srt::yaz(&belge);
    assert_eq!(metin, SRT_COK_SATIR);
}

/// Negatif süreli replik okunur (düzelmesi kural motorunun işidir).
#[test]
fn srt_negatif_sure_okunur() {
    let metin = "1\n00:00:05,000 --> 00:00:02,000\nTers sure\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].sure_ms(), -3000);
    assert!(!belge.replikler[0].zamanlama_gecerli());
}

/// Sıra bozukluğu okunur (dosya sırası korunur).
#[test]
fn srt_sira_bozuklugu_ayiklanir() {
    let metin =
        "1\n00:00:05,000 --> 00:00:06,000\nOnce\n\n2\n00:00:01,000 --> 00:00:02,000\nSonra\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 2);
    assert!(belge.replikler[1].baslangic < belge.replikler[0].baslangic);
    assert_eq!(belge.sirali()[0].metin, "Sonra");
}

/// Numara satırı yoksa dosya sırasından numara üretilir.
#[test]
fn srt_numara_satiri_olmayinca_sira_kullanilir() {
    let metin = "00:00:01,000 --> 00:00:02,000\nBirinci\n\n00:00:03,000 --> 00:00:04,000\nIkinci\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].numara, 1);
    assert_eq!(belge.replikler[1].numara, 2);
}

/// Konum ayarları (`X1:...`) zaman damgasının parçası değildir.
#[test]
fn srt_konum_ayarlari_yoksayilir() {
    let metin = "1\n00:00:01,000 --> 00:00:02,000 X1:10 X2:20\nKonumlu\n";
    let belge = srt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].bitis, Zaman::milis(2000));
    assert!(belge.sozdizimi.is_empty());
}

/// BOM'lu SRT içeriği sorunsuz okunur.
#[test]
fn srt_bomlu_girdi_okunur() {
    let mut bayt = vec![0xEF, 0xBB, 0xBF];
    bayt.extend_from_slice(SRT_ORNEK.as_bytes());
    let (metin, rapor) = kodlama::coz(&bayt);
    assert!(rapor.bom_var);
    let belge = srt::ayikla(&metin, &rapor);
    assert_eq!(belge.replik_sayisi(), 2);
}

// ---------------------------------------------------------------- ASS ----

/// ASS okuma → yazma → okuma gidiş-dönüşü birebir aynı belgeyi verir.
#[test]
fn ass_gidis_donus_birebir_ayni() {
    let ilk = ass::ayikla(ASS_ORNEK, &bos_kodlama());
    let metin = ass::yaz(&ilk);
    let ikinci = ass::ayikla(&metin, &bos_kodlama());
    assert_eq!(ilk.stiller, ikinci.stiller);
    assert_eq!(ilk.replikler, ikinci.replikler);
    assert_eq!(metin, ASS_ORNEK);
    assert!(ilk.sozdizimi.is_empty());
}

/// ASS `[Script Info]` alanları okunur.
#[test]
fn ass_script_info_okunur() {
    let belge = ass::ayikla(ASS_ORNEK, &bos_kodlama());
    assert_eq!(ass::script_turu(&belge), "v4+");
    assert_eq!(belge.basliklar.len(), 4);
    assert_eq!(
        belge.basliklar[0],
        ("Title".to_string(), "Ornek".to_string())
    );
}

/// ASS stilleri okunur.
#[test]
fn ass_stiller_okunur() {
    let belge = ass::ayikla(ASS_ORNEK, &bos_kodlama());
    assert_eq!(belge.stiller.len(), 1);
    assert_eq!(belge.stiller[0].ad, "Default");
    assert_eq!(belge.stiller[0].alan("Fontname"), "Arial");
    assert!(belge.stil_tanimli("Default"));
}

/// ASS'te stil tablosunda tanımsız stil referansı işaretlenir.
///
/// Tanımsız stil tespiti `bicim::sozden_oku` katmanında yapılır; doğrudan
/// `ass::ayikla` çağıran bir çağıran bu işlemi atlar.
#[test]
fn ass_tanimsiz_stil_referansi_isaretlenir() {
    let metin = ASS_ORNEK.replace(
        "Dialogue: 0,0:00:01.00,0:00:03.00,Default",
        "Dialogue: 0,0:00:01.00,0:00:03.00,YokBoyleStil",
    );
    let belge = subforge::bicim::sozden_oku(Bicim::Ass, &metin, &bos_kodlama())
        .expect("belge ayristirilir");
    assert_eq!(belge.replikler[0].stil.as_deref(), Some("YokBoyleStil"));
    assert_eq!(belge.tanimsiz_stiller, vec!["YokBoyleStil".to_string()]);
}

/// ASS'te tanımlı stil referansı uyarı üretmez.
#[test]
fn ass_tanimli_stil_referansi_uyari_uretmez() {
    let belge = subforge::bicim::sozden_oku(Bicim::Ass, ASS_ORNEK, &bos_kodlama())
        .expect("belge ayristirilir");
    assert!(belge.tanimsiz_stiller.is_empty());
}

/// ASS metin alanında virgül korunur.
#[test]
fn ass_metin_alani_virgulu_korunur() {
    let metin = ASS_ORNEK.replace("Merhaba dunya", "Merhaba, dunya");
    let belge = ass::ayikla(&metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].metin, "Merhaba, dunya");
    assert!(!belge
        .sozdizimi
        .iter()
        .any(|h| h.mesaj.contains("alan bekleniyordu")));
}

/// ASS `Comment:` satırı korunur.
#[test]
fn ass_comment_satiri_korunur() {
    let metin = format!("{ASS_ORNEK}Comment: 0,0:00:04.00,0:00:05.00,Default,,0,0,0,,Yorum\n");
    let belge = ass::ayikla(&metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 2);
    assert_eq!(belge.replikler[1].tur, OlayTuru::Yorum);
    let geri = ass::yaz(&belge);
    assert!(geri.contains("Comment: 0,0:00:04.00,0:00:05.00"));
}

/// SSA'nın eski `[V4 Styles]` biçimi de okunur ve yazılır.
#[test]
fn ass_v4_styles_bicimi_okunur() {
    let metin = "[Script Info]\nScriptType: v4\n\n[V4 Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, TertiaryColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, AlphaLevel, Encoding\nStyle: Default,Tahoma,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,1,2,0,2,10,10,10,0,1\n\n[Events]\nFormat: Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: Marked=0,0:00:01.00,0:00:03.00,Default,,0000,0000,0000,,Merhaba\n";
    let belge = ass::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.stiller.len(), 1);
    assert_eq!(belge.stil_format.len(), 18);
    assert_eq!(belge.replikler[0].metin, "Merhaba");
    assert_eq!(belge.replikler[0].baslangic, Zaman::milis(1000));
    let geri = ass::yaz(&belge);
    let ikinci = ass::ayikla(&geri, &bos_kodlama());
    assert_eq!(belge.replikler, ikinci.replikler);
    assert_eq!(belge.stiller, ikinci.stiller);
}

/// ASS'te eksik alan sayısı hata olarak işaretlenir.
#[test]
fn ass_eksik_alan_sayisi_isaretlenir() {
    let metin = ASS_ORNEK.replace(
        "Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,Merhaba dunya",
        "Dialogue: 0,0:00:01.00",
    );
    let belge = ass::ayikla(&metin, &bos_kodlama());
    assert!(belge
        .sozdizimi
        .iter()
        .any(|h| h.mesaj.contains("alan bekleniyordu")));
}

/// ASS'te boş dosya hata üretmez.
#[test]
fn ass_bos_dosya_hata_uretmez() {
    let belge = ass::ayikla("", &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 0);
    assert!(belge.sozdizimi.is_empty());
}

/// ASS yüzde bölümlü zaman damgası okunur.
#[test]
fn ass_yuzde_bolumlu_zaman_okunur() {
    let belge = ass::ayikla(ASS_ORNEK, &bos_kodlama());
    assert_eq!(belge.replikler[0].bitis, Zaman::milis(3000));
    assert_eq!(Zaman::milis(1234).ass_yaz(), "0:00:01.23");
}

// ---------------------------------------------------------------- WebVTT ----

/// WebVTT okuma → yazma → okuma gidiş-dönüşü birebir aynı belgeyi verir.
#[test]
fn vtt_gidis_donus_birebir_ayni() {
    let ilk = vtt::ayikla(VTT_ORNEK, &bos_kodlama());
    let metin = vtt::yaz(&ilk);
    let ikinci = vtt::ayikla(&metin, &bos_kodlama());
    assert_eq!(ilk.replikler, ikinci.replikler);
    assert_eq!(metin, VTT_ORNEK);
    assert!(ilk.sozdizimi.is_empty());
}

/// WebVTT başlık bloğu (`WEBVTT - etiket`) okunur.
///
/// WebVTT'e göre başlık alanları `WEBVTT` satırından sonra **ilk boş satıra
/// kadar** gelir; boş satırdan sonra gelen `Anahtar: değer` satırları içeriktir.
#[test]
fn vtt_baslik_bloku_okunur() {
    let metin =
        "WEBVTT - Ornek\nKind: captions\nLanguage: tr\n\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.basliklar.len(), 2);
    assert_eq!(
        belge.basliklar[1],
        ("Language".to_string(), "tr".to_string())
    );
    assert!(belge.sozdizimi.is_empty());
}

/// Boş satır olmadan gelen başlık alanları da tanınır (bozuk dosya kurtarması).
#[test]
fn vtt_bos_satirsiz_baslik_alani_taninir() {
    let metin = "WEBVTT\nLanguage: tr\n\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(
        belge.basliklar,
        vec![("Language".to_string(), "tr".to_string())]
    );
    assert_eq!(belge.replik_sayisi(), 1);
    assert!(belge.sozdizimi.is_empty());
}

/// Boş satırdan **sonra** gelen `Anahtar: değer` satırı içeriktir, başlık değil.
#[test]
fn vtt_bos_satir_sonrasi_alan_baslik_sayilmaz() {
    let metin = "WEBVTT\n\nKind: captions\n\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert!(belge.basliklar.is_empty(), "baslik: {:?}", belge.basliklar);
    // "Kind: captions" bir cue olarak yorumlanır ve hata kaydı üretir.
    assert!(!belge.sozdizimi.is_empty());
}

/// WebVTT `NOTE` bloğu atlanır, replik sayısına girmez.
#[test]
fn vtt_note_bloku_atlanir() {
    let metin = "WEBVTT\n\nNOTE bu bir yorum\nbirinci satir\n\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 1);
    assert_eq!(belge.replikler[0].metin, "Merhaba");
}

/// WebVTT `STYLE` bloğu başlık alanı olarak saklanır.
///
/// CSS kuralı `::cue { ... }` biçiminde olduğundan ilk iki nokta üst üste gelir;
/// ayrıştırıcı ilk noktadan böler ve anahtar `STYLE` olur.
#[test]
fn vtt_style_blogu_okunur() {
    let metin = "WEBVTT\n\nSTYLE\n::cue { color: white }\n\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 1);
    assert!(belge.basliklar.iter().any(|(k, _)| k.starts_with("STYLE")));
}

/// WebVTT cue kimlik satırı desteklenir.
#[test]
fn vtt_cue_kimligi_desteklenir() {
    let metin = "WEBVTT\n\ngiris\n00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 1);
    assert_eq!(belge.replikler[0].metin, "Merhaba");
    assert!(belge.sozdizimi.is_empty());
}

/// WebVTT uzun zaman biçimi `HH:MM:SS.mmm` kabul edilir.
#[test]
fn vtt_uzun_zaman_bicimi_kabul_edilir() {
    let metin = "WEBVTT\n\n01:00:01.000 --> 01:00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].baslangic, Zaman::milis(3_601_000));
    assert!(belge.sozdizimi.is_empty());
}

/// WebVTT eksik `WEBVTT` başlığı hata olarak işaretlenir.
#[test]
fn vtt_eksik_baslik_isaretlenir() {
    let metin = "00:01.000 --> 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert!(belge.sozdizimi.iter().any(|h| h.mesaj.contains("WEBVTT")));
    assert_eq!(belge.replik_sayisi(), 1);
}

/// WebVTT boş dosya hata kaydı üretir.
#[test]
fn vtt_bos_dosya_isaretlenir() {
    let belge = vtt::ayikla("", &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 0);
    assert_eq!(belge.sozdizimi.len(), 1);
    assert_eq!(belge.sozdizimi[0].mesaj, "dosya bos");
}

/// WebVTT eksik `-->` ayracı hata kaydı üretir.
#[test]
fn vtt_eksik_ayrac_isaretlenir() {
    let metin = "WEBVTT\n\n00:01.000 00:02.000\nMerhaba\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replik_sayisi(), 0);
    assert!(!belge.sozdizimi.is_empty());
}

/// Çok satırlı WebVTT altyazısı korunur. Satırlar arasında boş satır olmaz.
#[test]
fn vtt_cok_satirli_altyazi_korunur() {
    let metin = "WEBVTT\n\n00:01.000 --> 00:04.000\nBirinci\nIkinci\nUcuncu\n\n";
    let belge = vtt::ayikla(metin, &bos_kodlama());
    assert_eq!(belge.replikler[0].satir_sayisi(), 3);
    assert_eq!(vtt::yaz(&belge), metin);
}

/// UTF-16 LE WebVTT dosyası okunur.
#[test]
fn vtt_utf16_le_okunur() {
    let mut bayt = vec![0xFF, 0xFE];
    for b in VTT_ORNEK.encode_utf16() {
        bayt.extend_from_slice(&b.to_le_bytes());
    }
    let (metin, rapor) = kodlama::coz(&bayt);
    assert_eq!(rapor.kodlama, "utf-16-le");
    let belge = vtt::ayikla(&metin, &rapor);
    assert_eq!(belge.replik_sayisi(), 2);
}

// ------------------------------------------------------------ biçim tespiti ----

/// SRT içeriği doğru biçim olarak tanınır.
#[test]
fn srt_bicimi_tespit_edilir() {
    assert_eq!(bicim_tespit(SRT_ORNEK), Some(Bicim::Srt));
}

/// ASS içeriği doğru biçim olarak tanınır.
#[test]
fn ass_bicimi_tespit_edilir() {
    assert_eq!(bicim_tespit(ASS_ORNEK), Some(Bicim::Ass));
}

/// WebVTT içeriği doğru biçim olarak tanınır.
#[test]
fn vtt_bicimi_tespit_edilir() {
    assert_eq!(bicim_tespit(VTT_ORNEK), Some(Bicim::Vtt));
}

/// BOM'lu WebVTT içeriği de tanınır.
#[test]
fn bomlu_webvtt_tespit_edilir() {
    assert_eq!(
        bicim_tespit("\u{feff}WEBVTT\n\n00:01.000 --> 00:02.000\nX\n"),
        Some(Bicim::Vtt)
    );
}

/// Tanınmayan içerik `None` döner.
#[test]
fn taninmayan_icerik_yok_doner() {
    assert_eq!(bicim_tespit(""), None);
    assert_eq!(bicim_tespit("bu bir altyazi dosyasi degil\n"), None);
}

/// Biçim adı karşılıkları (komut satırı seçimi) doğrudur.
#[test]
fn bicim_ad_karsiliklari() {
    assert_eq!(Bicim::addan("srt"), Some(Bicim::Srt));
    assert_eq!(Bicim::addan("SSA"), Some(Bicim::Ass));
    assert_eq!(Bicim::addan("webvtt"), Some(Bicim::Vtt));
    assert_eq!(Bicim::addan("xyz"), None);
    assert_eq!(Bicim::Vtt.uzanti(), "vtt");
}

/// Çok satırlı repliğin en uzun satırı ölçülür.
#[test]
fn en_uzun_satir_olculur() {
    let r = Replik::yeni(1, Zaman::milis(0), Zaman::milis(1000), "ab\nabcd\nabc");
    assert_eq!(r.satir_sayisi(), 3);
    assert_eq!(r.en_uzun_satir(), 4);
    assert_eq!(r.karakter_sayisi(), 9);
    assert_eq!(r.satirlar(), vec!["ab", "abcd", "abc"]);
}

/// Boş metinli replikte satır sayısı sıfırdır (tek boş satır sayılmaz).
#[test]
fn bos_metinde_satir_sayisi_sifir() {
    let r = Replik::yeni(1, Zaman::milis(0), Zaman::milis(1000), "");
    assert_eq!(r.satir_sayisi(), 0);
    assert_eq!(r.karakter_sayisi(), 0);
    assert!(r.satirlar().is_empty());
}
