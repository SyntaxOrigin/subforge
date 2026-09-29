//! Advanced SubStation Alpha (ASS) ve SubStation Alpha (SSA) okuma-yazma.
//!
//! Kaynak: Aegisub projesinin biçim belgeleri, <https://aegisub.org/docs/ass/>
//! ve SSA v4 stilleri için
//! <https://wiki.multimedia.cx/index.php/SubStation_Styles>.
//!
//! ASS üç bölümden oluşur: `[Script Info]`, `[V4+ Styles]` (SSA'da `[V4 Styles]`)
//! ve `[Events]`. Her bölüm kendi `Format:` satırıyla alan listesini bildirir.
//! Bu modül alanları **ad-değer** olarak saklar; böylece `[V4 Styles]` ile
//! `[V4+ Styles]` arasındaki fark ve bilinmeyen alanlar kayıpsız korunur.
//!
//! Zaman damgası biçimi `H:MM:SS.cc` (saniyede yüzde) — diğer iki biçimden
//! farklı olarak üç haneli milisaniye **yoktur**.

use crate::bicim::{
    ass_bicim_adi, satir_temizle, varsayilan_ass_olay_format, varsayilan_ass_stil_format,
};
use crate::kodlama::KodlamaRaporu;
use crate::model::{Belge, Bicim, OlayTuru, Replik, SozdizimiKaydi, Stil};
use crate::zaman::Zaman;

const BOLUM_SCRIPT_INFO: &str = "Script Info";
const BOLUM_STILLER: &str = "Styles";
const BOLUM_EVENTS: &str = "Events";

/// ASS/SSA metnini belgeye çevirir.
pub fn ayikla(metin: &str, kodlama: &KodlamaRaporu) -> Belge {
    let mut belge = Belge::bos(Bicim::Ass);
    belge.kodlama = kodlama.clone();
    belge.crlf = metin.contains("\r\n");
    belge.olay_format = varsayilan_ass_olay_format();
    belge.stil_format = varsayilan_ass_stil_format();

    let mut bolum = Bolum::Bilinmiyor;
    let mut stil_format_okur = false;
    let mut olay_format_okur = false;
    let mut olay_sayaci: u32 = 0;

    for (idx, ham) in metin.lines().enumerate() {
        let temiz = satir_temizle(ham);
        let temiz = temiz.as_str();
        if temiz.is_empty() || temiz.starts_with(';') {
            continue;
        }
        if let Some(baslik) = temiz.strip_prefix('[') {
            bolum = bolum_tespit(baslik.trim_end_matches(']').trim());
            stil_format_okur = false;
            olay_format_okur = false;
            continue;
        }
        match bolum {
            Bolum::Bilgi => {
                if let Some((ad, deger)) = temiz.split_once(':') {
                    belge
                        .basliklar
                        .push((ad.trim().to_string(), deger.trim().to_string()));
                } else {
                    belge.sozdizimi.push(SozdizimiKaydi::yeni(
                        ass_bicim_adi(),
                        idx + 1,
                        format!("[Script Info] satirinda ':' ayraci yok: '{temiz}'"),
                    ));
                }
            }
            Bolum::Stiller => {
                if !stil_format_okur {
                    if let Some(alanlar) = format_alanlari(temiz) {
                        belge.stil_format = alanlar;
                        stil_format_okur = true;
                        continue;
                    }
                }
                stil_satirini_ekle(&mut belge, temiz, idx + 1);
            }
            Bolum::Events => {
                if !olay_format_okur {
                    if let Some(alanlar) = format_alanlari(temiz) {
                        belge.olay_format = alanlar;
                        olay_format_okur = true;
                        continue;
                    }
                }
                if let Some(mut replik) =
                    olay_satirini_ayikla(temiz, idx + 1, &belge.olay_format, &mut belge.sozdizimi)
                {
                    olay_sayaci += 1;
                    replik.numara = olay_sayaci;
                    belge.replikler.push(replik);
                }
            }
            Bolum::Bilinmiyor => {
                // Bölüm başlığı olmayan bir `Format:` satırı yine de alan
                // listesini tanımlar (bazı araçlar boş bölüm başlığı bırakır).
                if let Some(alanlar) = format_alanlari(temiz) {
                    if alanlar.iter().any(|a| a.eq_ignore_ascii_case("Start")) {
                        belge.olay_format = alanlar;
                    } else {
                        belge.stil_format = alanlar;
                    }
                    continue;
                }
                belge.sozdizimi.push(SozdizimiKaydi::yeni(
                    ass_bicim_adi(),
                    idx + 1,
                    format!("bolum basligi olmayan satir: '{temiz}'"),
                ));
            }
        }
    }
    belge
}

/// ASS belgesini metne çevirir.
///
/// Alan sırası `Format` satırındaki sırayla birebir korunur; böylece üç
/// farklı stil tablosu varyantı da gidiş-dönüşte kayıpsızdır.
pub fn yaz(belge: &Belge) -> String {
    let eol = if belge.crlf { "\r\n" } else { "\n" };
    let mut cikti = String::new();
    cikti.push_str("[Script Info]");
    cikti.push_str(eol);
    for (ad, deger) in &belge.basliklar {
        cikti.push_str(ad);
        cikti.push_str(": ");
        cikti.push_str(deger);
        cikti.push_str(eol);
    }
    cikti.push_str(eol);

    cikti.push_str("[V4+ Styles]");
    cikti.push_str(eol);
    cikti.push_str("Format: ");
    cikti.push_str(&birlestir(&belge.stil_format));
    cikti.push_str(eol);
    for stil in &belge.stiller {
        cikti.push_str("Style: ");
        let degerler: Vec<String> = belge
            .stil_format
            .iter()
            .map(|alan| stil.alan(alan).to_string())
            .collect();
        cikti.push_str(&degerler.join(","));
        cikti.push_str(eol);
    }
    cikti.push_str(eol);

    cikti.push_str("[Events]");
    cikti.push_str(eol);
    cikti.push_str("Format: ");
    cikti.push_str(&birlestir(&belge.olay_format));
    cikti.push_str(eol);
    let alan_sayisi = belge.olay_format.len();
    for replik in &belge.replikler {
        cikti.push_str(replik.tur.ass_etiketi());
        cikti.push_str(": ");
        for (i, alan) in belge.olay_format.iter().enumerate() {
            cikti.push_str(&olay_alani_deger(replik, alan));
            if i + 1 < alan_sayisi {
                cikti.push(',');
            }
        }
        cikti.push_str(eol);
    }
    cikti
}

/// Alan adlarını virgülle birleştirir.
fn birlestir(alanlar: &[String]) -> String {
    alanlar
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Repliğin bir ASS olay alanı için taşıdığı değeri üretir.
fn olay_alani_deger(replik: &Replik, alan: &str) -> String {
    match alan.to_ascii_lowercase().as_str() {
        "layer" => replik.katman.to_string(),
        "start" => replik.baslangic.ass_yaz(),
        "end" => replik.bitis.ass_yaz(),
        "style" => replik.stil.clone().unwrap_or_default(),
        "name" => replik.ad.clone().unwrap_or_default(),
        "marginl" => replik.kenar_sol.to_string(),
        "marginr" => replik.kenar_sag.to_string(),
        "marginv" => replik.kenar_ust.to_string(),
        "effect" => replik.efekt.clone(),
        "text" => replik.metin.clone(),
        _ => String::new(),
    }
}

/// `Style: Default,Arial,48,...` satırını belgeye ekler.
fn stil_satirini_ekle(belge: &mut Belge, satir: &str, satir_no: usize) {
    let degerler: Vec<String> = satir
        .split_once(':')
        .map_or(satir, |(_, sag)| sag)
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    if degerler.len() < belge.stil_format.len() {
        belge.sozdizimi.push(SozdizimiKaydi::yeni(
            ass_bicim_adi(),
            satir_no,
            format!(
                "stil satiri {} alan bekleniyordu, {} bulundu: '{}'",
                belge.stil_format.len(),
                degerler.len(),
                satir
            ),
        ));
    }
    let alanlar: Vec<(String, String)> = belge
        .stil_format
        .iter()
        .enumerate()
        .map(|(i, alan)| (alan.clone(), degerler.get(i).cloned().unwrap_or_default()))
        .collect();
    let ad = alanlar
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Name"))
        .map_or_else(String::new, |(_, v)| v.clone());
    belge.stiller.push(Stil::yeni(&ad, alanlar));
}

/// `Dialogue: 0,0:00:01.00,0:00:03.00,Default,,0,0,0,,Metin` satırını ayrıştırır.
fn olay_satirini_ayikla(
    satir: &str,
    satir_no: usize,
    format: &[String],
    hatalar: &mut Vec<SozdizimiKaydi>,
) -> Option<Replik> {
    let (tur, govde) = satir.split_once(':')?;
    let parcalar: Vec<&str> = govde.split(',').collect();
    if parcalar.len() < format.len() {
        hatalar.push(SozdizimiKaydi::yeni(
            ass_bicim_adi(),
            satir_no,
            format!(
                "olay satiri {} alan bekleniyordu, {} bulundu",
                format.len(),
                parcalar.len()
            ),
        ));
    }
    let alan = |ad: &str| -> String {
        format
            .iter()
            .position(|f| f.eq_ignore_ascii_case(ad))
            .and_then(|i| parcalar.get(i).map(|s| s.trim().to_string()))
            .unwrap_or_default()
    };
    let bas_metin = Zaman::ayikla(&alan("Start"));
    let bit_metin = Zaman::ayikla(&alan("End"));
    if bas_metin.is_err() || bit_metin.is_err() {
        hatalar.push(SozdizimiKaydi::yeni(
            ass_bicim_adi(),
            satir_no,
            format!("zaman damgasi okunamadi: '{satir}'"),
        ));
    }
    // Metin alanı virgül içerebilir; `Text` alanının başladığı yerden sona
    // kadar yeniden birleştirilir, böylece "Merhaba, dünya" bozulmaz.
    let metin = format
        .iter()
        .position(|f| f.eq_ignore_ascii_case("Text"))
        .and_then(|i| parcalar.get(i).map(|_| parcalar[i..].join(",")))
        .map(|m| satir_temizle(&m))
        .unwrap_or_default();
    let stil = alan("Style");
    Some(Replik {
        numara: 0,
        baslangic: bas_metin.unwrap_or(Zaman::SIFIR),
        bitis: bit_metin.unwrap_or(Zaman::SIFIR),
        metin,
        stil: if stil.is_empty() { None } else { Some(stil) },
        ad: Some(alan("Name")).filter(|s| !s.is_empty()),
        kenar_sol: alan("MarginL").parse().unwrap_or(0),
        kenar_sag: alan("MarginR").parse().unwrap_or(0),
        kenar_ust: alan("MarginV").parse().unwrap_or(0),
        efekt: alan("Effect"),
        katman: alan("Layer").parse().unwrap_or(0),
        tur: OlayTuru::ass_etiketten(tur),
    })
}

/// `Format: a, b, c` satırından alan adlarını çıkarır.
fn format_alanlari(satir: &str) -> Option<Vec<String>> {
    let govde = satir
        .strip_prefix("Format:")
        .or_else(|| satir.strip_prefix("format:"))?;
    Some(
        govde
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// Bölüm başlığını sınıflandırır.
fn bolum_tespit(baslik: &str) -> Bolum {
    let k = baslik.to_ascii_lowercase();
    if k == BOLUM_SCRIPT_INFO.to_ascii_lowercase() {
        Bolum::Bilgi
    } else if k.contains(&BOLUM_STILLER.to_ascii_lowercase()) {
        Bolum::Stiller
    } else if k == BOLUM_EVENTS.to_ascii_lowercase() {
        Bolum::Events
    } else {
        Bolum::Bilinmiyor
    }
}

/// ASS dosyasındaki anlık bölüm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bolum {
    /// `[Script Info]`
    Bilgi,
    /// `[V4 Styles]`, `[V4 Styles+]`, `[V4+ Styles]`
    Stiller,
    /// `[Events]`
    Events,
    /// Başlık yok veya tanınmayan bölüm.
    Bilinmiyor,
}

/// `[Script Info]` bölümündeki `ScriptType` değerini döndürür.
pub fn script_turu(belge: &Belge) -> String {
    belge
        .basliklar
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("ScriptType"))
        .map_or_else(|| "v4+ (belirtilmemis)".to_string(), |(_, v)| v.clone())
}
