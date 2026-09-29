//! Biçim katmanı: SRT, ASS/SSA ve WebVTT okuma-yazma ortak yüzeyi.
//!
//! Bu katman çekirdek modeli bilir, kural motorunu bilmez (fikir raporu §06).
//! Ayrıştırıcılar **hataya dayanıklıdır**: tek bir bozuk satır dosyayı düşürmez,
//! `SozdizimiHatasi` kaydı üretilir ve dosyanın geri kalanı okunur. Bu, fikir
//! raporu §05 "biçim okuma ve yazma" kabul kriteridir.

use crate::ass;
use crate::hata::SubForgeHata;
use crate::kodlama::KodlamaRaporu;
use crate::model::{Belge, Bicim, Stil};
use crate::srt;
use crate::vtt;
use std::path::Path;

/// Bir altyazı dosyasının tamamını bayt düzeyinde okuyup belgeye çevirir.
///
/// Okuma zinciri: bayt oku → kodlama çöz → biçim tespit → ayrıştır. Kodlama
/// çözülemezse hata verilmez; bozuk baytlar `U+FFFD` olur ve altıncı kural
/// bunu denetler.
pub fn oku(yol: &Path) -> Result<Belge, SubForgeHata> {
    let bayt = std::fs::read(yol).map_err(|e| SubForgeHata::dosya_okunamadi(yol, e))?;
    let (metin, kodlama) = crate::kodlama::coz(&bayt);
    let bicim = bicim_tespit(metin.as_str()).ok_or_else(|| SubForgeHata::bicim_tanimli(yol))?;
    let belge = sozden_oku(bicim, &metin, &kodlama)?;
    Ok(belge)
}

/// Metin içeriğinden biçimi tespit eder; `None` dönerse tanımlı değildir.
///
/// Tespit sırası: `WEBVTT` başlığı → ASS bölüm başlığı → SRT zaman çizgisi
/// deseni. Dosya uzantısı **kullanılmaz**, çünkü taşınabilir iş akışında
/// uzantı yanlış olabilir.
pub fn bicim_tespit(metin: &str) -> Option<Bicim> {
    for satir in metin.lines().take(20) {
        let s = satir.trim_start_matches('\u{feff}').trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("WEBVTT") {
            return Some(Bicim::Vtt);
        }
        if s.starts_with('[') {
            let baslik = s.trim_start_matches('[').to_ascii_lowercase();
            if baslik.contains("script info") || baslik.contains("v4 styles") {
                return Some(Bicim::Ass);
            }
            if baslik.contains("events") {
                return Some(Bicim::Ass);
            }
        }
        if s.contains("-->") {
            return Some(Bicim::Srt);
        }
        // İlk anlamlı satır sayısal ise ve sonraki satırda `-->` varsa SRT'dir.
        if s.parse::<u32>().is_ok() && metin.contains("-->") {
            return Some(Bicim::Srt);
        }
    }
    None
}

/// Metin içeriğini belirtilen biçimde belgeye çevirir.
pub fn sozden_oku(
    bicim: Bicim,
    metin: &str,
    kodlama: &KodlamaRaporu,
) -> Result<Belge, SubForgeHata> {
    let mut belge = match bicim {
        Bicim::Srt => srt::ayikla(metin, kodlama),
        Bicim::Ass => ass::ayikla(metin, kodlama),
        Bicim::Vtt => vtt::ayikla(metin, kodlama),
    };
    tanimsiz_stilleri_isaretle(&mut belge);
    Ok(belge)
}

/// Belgeyi verilen biçimde diske yazar (UTF-8, BOM'suz).
///
/// Yazma **atomik değildir**: önce hedef dosyaya doğrudan yazılır. Kısmi yazma
/// riski kabul edilmiştir çünkü altyazı dosyaları küçüktür ve kullanıcı her
/// zaman kaydırma/normalizasyonu ayrı bir çıktı dosyasına yapar (`--out`),
/// böylece kaynak dosya korunur.
pub fn yaz(belge: &Belge, bicim: Bicim, yol: &Path) -> Result<(), SubForgeHata> {
    let metin = metne_cevir_yaz(belge, bicim);
    std::fs::write(yol, metin).map_err(|e| SubForgeHata::cikti_yazilamadi(yol, e))
}

/// Belgeyi verilen biçimde metne çevirir; kayıp listesi üretmez.
pub fn metne_cevir_yaz(belge: &Belge, bicim: Bicim) -> String {
    match bicim {
        Bicim::Srt => srt::yaz(belge),
        Bicim::Ass => ass::yaz(belge),
        Bicim::Vtt => vtt::yaz(belge),
    }
}

/// Dönüşümde kaybolan özelliği tanımlayan kayıp kaydı.
///
/// Fikir raporu §05 kabul kriteri: "dönüşüm sonrası kaybolan her özellik raporda
/// listeleniyor". Kayıp bilgisi bilinçlidir, bir hata değildir.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Kayip {
    /// Kaybolan özelliğin adı.
    pub ozellik: String,
    /// Kaynak değeri.
    pub deger: String,
    /// Kaybolma nedeni.
    pub gerekce: String,
}

impl Kayip {
    /// Yeni kayıp oluşturur.
    pub fn yeni(ozellik: &str, deger: &str, gerekce: &str) -> Self {
        Self {
            ozellik: ozellik.to_string(),
            deger: deger.to_string(),
            gerekce: gerekce.to_string(),
        }
    }
}

/// Belgeyi başka bir biçime çevirir ve kaybolan özellikleri listeler.
pub fn metne_cevir_kayipli(belge: &Belge, hedef: Bicim) -> (Belge, Vec<Kayip>) {
    let mut kayiplar: Vec<Kayip> = Vec::new();
    let mut yeni = Belge::bos(hedef);
    yeni.crlf = belge.crlf;
    yeni.kodlama = belge.kodlama.clone();
    yeni.replikler = belge.replikler.clone();

    match hedef {
        Bicim::Srt => {
            if !belge.stiller.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "ASS stiller",
                    &format!("{} stil", belge.stiller.len()),
                    "SRT stil tasimaz; metin ve zamanlama korunur",
                ));
            }
            for r in &belge.replikler {
                if r.stil.is_some() {
                    kayiplar.push(Kayip::yeni(
                        "replik stili",
                        r.stil.as_deref().unwrap_or(""),
                        "SRT stil tasimaz",
                    ));
                    break;
                }
            }
            if belge
                .replikler
                .iter()
                .any(|r| !r.efekt.is_empty() || r.kenar_ust != 0)
            {
                kayiplar.push(Kayip::yeni(
                    "kenar bosluklari ve efekt",
                    "MarginL/MarginR/MarginV/Effect",
                    "SRT bu alanlari icermez",
                ));
            }
            if !belge.basliklar.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "baslik bloklari",
                    &format!("{} alan", belge.basliklar.len()),
                    "SRT baslik bolumu tanimlamaz",
                ));
            }
            if belge
                .replikler
                .iter()
                .any(|r| r.tur == crate::model::OlayTuru::Yorum)
            {
                kayiplar.push(Kayip::yeni(
                    "Comment satirlari",
                    "gizli replikler",
                    "SRT yorum satiri icermez; tumu normal replige donustu",
                ));
            }
        }
        Bicim::Vtt => {
            if !belge.stiller.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "ASS stiller",
                    &format!("{} stil", belge.stiller.len()),
                    "WebVTT `STYLE` blogu kullanilir, ASS `Style:` tablosu degil",
                ));
            }
            for r in &belge.replikler {
                if r.stil.is_some() {
                    kayiplar.push(Kayip::yeni(
                        "replik stili",
                        r.stil.as_deref().unwrap_or(""),
                        "WebVTT replik ici stil tasimaz; CSS sinifi kullanilir",
                    ));
                    break;
                }
            }
            if belge
                .replikler
                .iter()
                .any(|r| !r.efekt.is_empty() || r.kenar_ust != 0)
            {
                kayiplar.push(Kayip::yeni(
                    "kenar bosluklari ve efekt",
                    "MarginL/MarginR/MarginV/Effect",
                    "WebVTT cue ayarlari kenar boslugu icermez",
                ));
            }
            if belge
                .replikler
                .iter()
                .any(|r| r.tur == crate::model::OlayTuru::Yorum)
            {
                kayiplar.push(Kayip::yeni(
                    "Comment satirlari",
                    "gizli replikler",
                    "WebVTT `NOTE` blogu kullanilir; icerik replik olarak tasindi",
                ));
            }
        }
        Bicim::Ass => {
            if !belge.basliklar.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "baslik bloklari",
                    &format!("{} alan", belge.basliklar.len()),
                    "ASS `Script Info` blogu varsayilan degerlerle yeniden uretildi",
                ));
            }
            // ASS her zaman bir stil tablosu ve replik stili bekler. Kaynak
            // belgede ikisi de yoksa (SRT / WebVTT) bunlar **uydurulur**; bu
            // bilgi kaybıdır ve kullanıcıya açıkça bildirilir.
            if belge.stiller.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "stil tablosu",
                    "yok",
                    "kaynak bicim stil tasimiyor; `Default` stili uretildi",
                ));
            }
            if !belge.replikler.iter().any(|r| r.stil.is_some()) {
                kayiplar.push(Kayip::yeni(
                    "replik stili",
                    "yok",
                    "kaynak belgede replik stili yok; tumu `Default` stiline baglandi",
                ));
            } else if belge.stiller.is_empty() {
                kayiplar.push(Kayip::yeni(
                    "replik stili",
                    "stil adlari",
                    "kaynak belgede stil tablosu yok; `Default` stili uretildi",
                ));
            }
            if belge.replikler.iter().any(|r| r.katman != 0) {
                kayiplar.push(Kayip::yeni(
                    "replik katmani",
                    "Layer",
                    "SRT ve WebVTT katman bilgisi tasimaz",
                ));
            }
        }
    }

    // Kayıpları tekrarsızla, kararlı sırada döndür.
    let mut gorulen: Vec<String> = Vec::new();
    kayiplar.retain(|k| {
        if gorulen.contains(&k.ozellik) {
            false
        } else {
            gorulen.push(k.ozellik.clone());
            true
        }
    });
    yeni.tanimsiz_stiller = belge.tanimsiz_stiller.clone();
    (yeni, kayiplar)
}

/// Repliklerin atıf yaptığı ama stil tablosunda bulunmayan stil adlarını işaretler.
fn tanimsiz_stilleri_isaretle(belge: &mut Belge) {
    if belge.stiller.is_empty() {
        // Stilsiz biçimlerde her replik stilsizdir; yanlış uyarı üretmemek için
        // stil atanmış replikler varsa yalnızca o zaman işaretleme yapılır.
        if belge.replikler.iter().any(|r| r.stil.is_some()) {
            belge.tanimsiz_stiller = belge
                .replikler
                .iter()
                .filter_map(|r| r.stil.clone())
                .collect();
            belge.tanimsiz_stiller.sort();
            belge.tanimsiz_stiller.dedup();
        }
        return;
    }
    let mut eksik: Vec<String> = Vec::new();
    for r in &belge.replikler {
        if let Some(ad) = r.stil.as_ref() {
            if !ad.is_empty() && !belge.stil_tanimli(ad) && !eksik.contains(ad) {
                eksik.push(ad.clone());
            }
        }
    }
    eksik.sort();
    belge.tanimsiz_stiller = eksik;
}

/// SRT ayrıştırıcısından dönen hatalı satır bilgisi için kısa yardımcı.
pub(crate) fn srt_bicim_adi() -> &'static str {
    "SRT"
}

/// ASS ayrıştırıcısından dönen hatalı satır bilgisi için kısa yardımcı.
pub(crate) fn ass_bicim_adi() -> &'static str {
    "ASS"
}

/// VTT ayrıştırıcısından dönen hatalı satır bilgisi için kısa yardımcı.
pub(crate) fn vtt_bicim_adi() -> &'static str {
    "WebVTT"
}

/// Boş bir belge için varsayılan ASS stil tablosu üretir.
///
/// SRT veya WebVTT'den ASS'e dönüşürken kullanılır; kaynak belgede stil
/// olmadığı için tek bir `Default` stili yeterlidir ve bu olgu dönüşüm
/// kayıp raporunda açıkça listelenir.
pub fn varsayilan_ass_stilleri() -> Vec<Stil> {
    let alanlar: Vec<(String, String)> = [
        ("Name", "Default"),
        ("Fontname", "Arial"),
        ("Fontsize", "48"),
        ("PrimaryColour", "&H00FFFFFF"),
        ("SecondaryColour", "&H000000FF"),
        ("OutlineColour", "&H00000000"),
        ("BackColour", "&H00000000"),
        ("Bold", "0"),
        ("Italic", "0"),
        ("Underline", "0"),
        ("StrikeOut", "0"),
        ("ScaleX", "100"),
        ("ScaleY", "100"),
        ("Spacing", "0"),
        ("Angle", "0"),
        ("BorderStyle", "1"),
        ("Outline", "2"),
        ("Shadow", "0"),
        ("Alignment", "2"),
        ("MarginL", "10"),
        ("MarginR", "10"),
        ("MarginV", "10"),
        ("Encoding", "1"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect();
    vec![Stil::yeni("Default", alanlar)]
}

/// Varsayılan ASS olay `Format` alan listesi.
pub fn varsayilan_ass_olay_format() -> Vec<String> {
    [
        "Layer", "Start", "End", "Style", "Name", "MarginL", "MarginR", "MarginV", "Effect", "Text",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// Varsayılan ASS stil `Format` alan listesi.
pub fn varsayilan_ass_stil_format() -> Vec<String> {
    [
        "Name",
        "Fontname",
        "Fontsize",
        "PrimaryColour",
        "SecondaryColour",
        "OutlineColour",
        "BackColour",
        "Bold",
        "Italic",
        "Underline",
        "StrikeOut",
        "ScaleX",
        "ScaleY",
        "Spacing",
        "Angle",
        "BorderStyle",
        "Outline",
        "Shadow",
        "Alignment",
        "MarginL",
        "MarginR",
        "MarginV",
        "Encoding",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// Test ve dönüşüm amaçlı boş bir replik listesi taşıyan belge kurar.
pub fn bos_belge(bicim: Bicim) -> Belge {
    let mut belge = Belge::bos(bicim);
    if bicim == Bicim::Ass {
        belge.stiller = varsayilan_ass_stilleri();
        belge.stil_format = varsayilan_ass_stil_format();
        belge.olay_format = varsayilan_ass_olay_format();
    }
    belge
}

/// Yardımcı: metni `\r\n`'den arındırıp sonundaki boşlukları kaldırır.
pub(crate) fn satir_temizle(satir: &str) -> String {
    satir.trim_end_matches('\r').trim_end().to_string()
}
