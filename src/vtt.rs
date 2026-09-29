//! Web Video Text Tracks (WebVTT) okuma ve yazma.
//!
//! Kaynak: W3C WebVTT şekillendirme kaynağı, <https://www.w3.org/TR/webvtt1/>.
//! Zaman çizgisi `--> ` ile ayrılır; damga `HH:MM:SS.mmm` veya saat yoksa
//! `MM:SS.mmm` olabilir. Bir cue'un isteğe bağlı kimlik satırı, ayraçtan sonra
//! gelen ayar satırı ve gövde satırları vardır. `NOTE` ve `STYLE` blokları
//! desteklenir.

use crate::bicim::{satir_temizle, vtt_bicim_adi};
use crate::kodlama::KodlamaRaporu;
use crate::model::{Belge, Bicim, Replik, SozdizimiKaydi};
use crate::zaman::Zaman;

/// Dosya başlığının ilk satırı.
pub const WEBVTT_BASLIK: &str = "WEBVTT";

/// Zaman çizgisi ayracı.
const ZAMAN_AYRACI: &str = "-->";

/// WebVTT metnini belgeye çevirir.
pub fn ayikla(metin: &str, kodlama: &KodlamaRaporu) -> Belge {
    let mut belge = Belge::bos(Bicim::Vtt);
    belge.kodlama = kodlama.clone();
    belge.crlf = metin.contains("\r\n");

    let satirlar: Vec<&str> = metin.lines().collect();
    if satirlar.is_empty() {
        belge
            .sozdizimi
            .push(SozdizimiKaydi::yeni(vtt_bicim_adi(), 0, "dosya bos"));
        return belge;
    }
    let ilk = satir_temizle(satirlar[0]);
    let baslik_var = ilk.starts_with(WEBVTT_BASLIK);
    if !baslik_var {
        belge.sozdizimi.push(SozdizimiKaydi::yeni(
            vtt_bicim_adi(),
            1,
            format!("ilk satir '{WEBVTT_BASLIK}' ile baslamiyor: '{ilk}'"),
        ));
    }
    // Başlık bloğu: `WEBVTT` satırından sonra boş satıra kadar olan `Anahtar: değer`
    // satırları. Başlık satırı hiç yoksa ilk satır da bir cue olabileceğinden
    // tarama 0. indeksten başlar.
    // Başlık bloğu yalnızca **ilk** boş satıra kadar sürer. WebVTT'e göre
    // başlık alanları `WEBVTT` satırından sonra ilk boş satıra kadar gelir;
    // ardından `NOTE` / `STYLE` / `REGION` blokları başlar. Alan adı kuralı
    // (`Anahtar: değer`, anahtarda boşluk yok) yalnızca boş satır bulunmadığı
    // hatalı dosyalarda ayırıcı görevi görür.
    let mut i = usize::from(baslik_var);
    // Başlık alanları, ilk boş satıra ya da ilk içerik satırına (cue / NOTE /
    // STYLE) kadar süren aralıktır. WebVTT bu aralığı boş satırla ayırır;
    // boş satır yoksa içerik satırı kendisi sınır olur.
    while i < satirlar.len() {
        let temiz = satir_temizle(satirlar[i]);
        if temiz.is_empty() {
            i += 1;
            break;
        }
        if temiz.contains(ZAMAN_AYRACI) || temiz.starts_with("NOTE") || temiz.starts_with("STYLE") {
            break;
        }
        if let Some((ad, deger)) = temiz.split_once(':') {
            if !ad.contains(' ') && !ad.is_empty() {
                belge
                    .basliklar
                    .push((ad.trim().to_string(), deger.trim().to_string()));
                i += 1;
                continue;
            }
        }
        break;
    }

    let mut sayac: u32 = 0;
    while i < satirlar.len() {
        let temiz = satir_temizle(satirlar[i]);
        if temiz.is_empty() {
            i += 1;
            continue;
        }
        if temiz.starts_with("NOTE") {
            i += 1;
            while i < satirlar.len() && !satir_temizle(satirlar[i]).is_empty() {
                i += 1;
            }
            continue;
        }
        if temiz.starts_with("STYLE") {
            i += 1;
            let mut govde: Vec<String> = Vec::new();
            while i < satirlar.len() && !satir_temizle(satirlar[i]).is_empty() {
                govde.push(satir_temizle(satirlar[i]));
                i += 1;
            }
            for satir in govde {
                if let Some((ad, deger)) = satir.split_once(':') {
                    belge
                        .basliklar
                        .push((format!("STYLE {}", ad.trim()), deger.trim().to_string()));
                }
            }
            continue;
        }
        if temiz.starts_with("REGION") {
            i += 1;
            while i < satirlar.len() && !satir_temizle(satirlar[i]).is_empty() {
                i += 1;
            }
            continue;
        }

        // Cue: istege bağlı kimlik satırı, sonra zaman çizgisi.
        let zaman_indeks = if temiz.contains(ZAMAN_AYRACI) {
            i
        } else if i + 1 < satirlar.len() && satir_temizle(satirlar[i + 1]).contains(ZAMAN_AYRACI) {
            i + 1
        } else {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                vtt_bicim_adi(),
                i + 1,
                format!("zaman cizgisi bulunamadi: '{temiz}'"),
            ));
            i += 1;
            continue;
        };
        let zaman_satiri = satir_temizle(satirlar[zaman_indeks]);
        let (baslangic, bitis, hata) = zaman_cizgisi_ayikla(&zaman_satiri);
        if let Some(mesaj) = hata {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                vtt_bicim_adi(),
                zaman_indeks + 1,
                mesaj,
            ));
        }
        let mut j = zaman_indeks + 1;
        let mut govde: Vec<&str> = Vec::new();
        while j < satirlar.len() && !satir_temizle(satirlar[j]).is_empty() {
            govde.push(satirlar[j].trim_end_matches('\r'));
            j += 1;
        }
        let birlestirilmis = govde.join("\n");
        if birlestirilmis.trim().is_empty() {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                vtt_bicim_adi(),
                zaman_indeks + 2,
                "altyazi metni bos".to_string(),
            ));
        }
        sayac += 1;
        belge.replikler.push(Replik::yeni(
            sayac,
            baslangic,
            bitis,
            birlestirilmis.trim_end(),
        ));
        i = j;
    }
    belge
}

/// WebVTT belgesini metne çevirir.
///
/// Cue kimlik satırı ve konum ayarı yazılmaz; bunlar WebVTT'de isteğe bağlıdır
/// ve oynatıcı davranışını değiştirmez. Kayıp listesi bu bilgiyi taşır.
pub fn yaz(belge: &Belge) -> String {
    let eol = if belge.crlf { "\r\n" } else { "\n" };
    let mut cikti = String::new();
    cikti.push_str(WEBVTT_BASLIK);
    cikti.push_str(eol);
    for (ad, deger) in &belge.basliklar {
        cikti.push_str(ad);
        cikti.push_str(": ");
        cikti.push_str(deger);
        cikti.push_str(eol);
    }
    cikti.push_str(eol);
    for replik in &belge.replikler {
        cikti.push_str(&replik.baslangic.vtt_yaz());
        cikti.push(' ');
        cikti.push_str(ZAMAN_AYRACI);
        cikti.push(' ');
        cikti.push_str(&replik.bitis.vtt_yaz());
        cikti.push_str(eol);
        cikti.push_str(&replik.metin);
        cikti.push_str(eol);
        cikti.push_str(eol);
    }
    cikti
}

/// `00:01.000 --> 00:03.000 align:start position:10%` çizgisini ayrıştırır.
fn zaman_cizgisi_ayikla(satir: &str) -> (Zaman, Zaman, Option<String>) {
    let parcalar: Vec<&str> = satir.split(ZAMAN_AYRACI).collect();
    if parcalar.len() < 2 {
        return (
            Zaman::SIFIR,
            Zaman::SIFIR,
            Some(format!(
                "zaman cizgisinde '{ZAMAN_AYRACI}' ayraci yok: '{satir}'"
            )),
        );
    }
    let bas_sol: Vec<&str> = parcalar[0].split_whitespace().collect();
    let sag_taraf: Vec<&str> = parcalar[1].split_whitespace().collect();
    let (Some(bas), Some(bit)) = (bas_sol.first(), sag_taraf.first()) else {
        return (
            Zaman::SIFIR,
            Zaman::SIFIR,
            Some("zaman cizgisinin iki tarafi da dolu olmali".to_string()),
        );
    };
    let baslangic = match Zaman::ayikla(bas) {
        Ok(z) => z,
        Err(e) => {
            return (
                Zaman::SIFIR,
                Zaman::SIFIR,
                Some(format!("baslangic zamani okunamadi: {e}")),
            )
        }
    };
    let bitis = match Zaman::ayikla(bit) {
        Ok(z) => z,
        Err(e) => {
            return (
                baslangic,
                Zaman::SIFIR,
                Some(format!("bitis zamani okunamadi: {e}")),
            )
        }
    };
    (baslangic, bitis, None)
}
