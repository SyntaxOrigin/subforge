//! SubRip (SRT) okuma ve yazma.
//!
//! Kaynak: SubRip biçim açıklaması, <https://en.wikipedia.org/wiki/SubRip> ve
//! Subtitle Edit'in SRT uygulama notları. SRT sözdizimi resmî bir RFC değildir;
//! yaygın kabul gören yapı şudur:
//!
//! ```text
//! 1
//! 00:00:01,000 --> 00:00:03,240
//! Metin satırı
//! ```
//!
//! Zaman damgası biçimi `HH:MM:SS,mmm`; **nokta** ayracı da kabul edilir çünkü
//! Windows yazılımlarının bir kısmı virgül yerine nokta üretir. Numara satırı
//! zorunlu değildir; yoksa dosya sırasından üretilir.

use crate::bicim::{satir_temizle, srt_bicim_adi, Kayip};
use crate::kodlama::KodlamaRaporu;
use crate::model::{Belge, Bicim, OlayTuru, Replik, SozdizimiKaydi};
use crate::zaman::Zaman;

/// Zaman çizgisi ayracı (`-->`); yazarken boşluklarla çevrilidir.
const ZAMAN_AYRACI: &str = "-->";

/// SRT metnini belgeye çevirir.
///
/// Hatalı bloklar atlanmaz; blok içindeki zaman çizgisi okunamazsa replik
/// `00:00:00,000 --> 00:00:00,000` olarak eklenir ve hata `belge.sozdizimi`
/// kanalına yazılır. Böylece "hatalı dosya açılır ama bozuk satır işaretlenir"
/// (fikir raporu §05) kabul kriteri sağlanır.
pub fn ayikla(metin: &str, kodlama: &KodlamaRaporu) -> Belge {
    let mut belge = Belge::bos(Bicim::Srt);
    belge.kodlama = kodlama.clone();
    belge.crlf = metin.contains("\r\n");

    let satirlar: Vec<&str> = metin.lines().collect();
    let mut i = 0usize;
    let mut sira: u32 = 0;
    while i < satirlar.len() {
        if satir_temizle(satirlar[i]).is_empty() {
            i += 1;
            continue;
        }
        // 1) Numara satırı: tam sayı ise numaradır, değilse zaman çizgisidir.
        let ilk = satir_temizle(satirlar[i]);
        let (mut numara, zaman_satiri) = match ilk.parse::<u32>() {
            Ok(n) => {
                if i + 1 >= satirlar.len() {
                    belge.sozdizimi.push(SozdizimiKaydi::yeni(
                        srt_bicim_adi(),
                        i + 1,
                        "numara satiri var ama zaman cizgisi yok",
                    ));
                    break;
                }
                (n, i + 1)
            }
            Err(_) => {
                sira += 1;
                (sira, i)
            }
        };
        if numara == 0 {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                srt_bicim_adi(),
                zaman_satiri + 1,
                "replik numarasi 0 olamaz",
            ));
            numara = 1;
        }
        let zaman_metni = satir_temizle(satirlar[zaman_satiri]);
        if !zaman_metni.contains(ZAMAN_AYRACI) {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                srt_bicim_adi(),
                zaman_satiri + 1,
                format!("zaman cizgisinde '{ZAMAN_AYRACI}' ayraci yok: '{zaman_metni}'"),
            ));
            i = zaman_satiri + 1;
            continue;
        }
        let (baslangic, bitis, hata) = zaman_cizgisi_ayikla(&zaman_metni);
        if let Some(mesaj) = hata {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                srt_bicim_adi(),
                zaman_satiri + 1,
                mesaj,
            ));
        }
        // 2) Metin satırları: boş satıra kadar.
        let metin_baslangic = zaman_satiri + 1;
        let mut j = metin_baslangic;
        let mut govde: Vec<&str> = Vec::new();
        while j < satirlar.len() && !satir_temizle(satirlar[j]).is_empty() {
            govde.push(satirlar[j].trim_end_matches('\r'));
            j += 1;
        }
        let birlestirilmis = govde.join("\n");
        if birlestirilmis.trim().is_empty() {
            belge.sozdizimi.push(SozdizimiKaydi::yeni(
                srt_bicim_adi(),
                metin_baslangic + 1,
                "altyazi metni bos",
            ));
        }
        let mut replik = Replik::yeni(numara, baslangic, bitis, birlestirilmis.trim_end());
        replik.tur = OlayTuru::Diyalog;
        belge.replikler.push(replik);
        sira = sira.max(numara);
        i = j;
    }
    belge
}

/// SRT belgesini metne çevirir.
pub fn yaz(belge: &Belge) -> String {
    let eol = if belge.crlf { "\r\n" } else { "\n" };
    let mut cikti = String::new();
    for replik in &belge.replikler {
        cikti.push_str(&replik.numara.to_string());
        cikti.push_str(eol);
        cikti.push_str(&replik.baslangic.srt_yaz());
        cikti.push(' ');
        cikti.push_str(ZAMAN_AYRACI);
        cikti.push(' ');
        cikti.push_str(&replik.bitis.srt_yaz());
        cikti.push_str(eol);
        cikti.push_str(&replik.metin);
        cikti.push_str(eol);
        cikti.push_str(eol);
    }
    cikti
}

/// `00:00:01,000 --> 00:00:03,240` çizgisini ayrıştırır.
///
/// Ayrac eksikse veya zaman damgaları geçersizse `Zaman::SIFIR` döner ve
/// hata metni `Some` olarak verilir; çağıran bu bilgiyi belgeye yazar.
fn zaman_cizgisi_ayikla(satir: &str) -> (Zaman, Zaman, Option<String>) {
    let parcalar: Vec<&str> = satir.split(ZAMAN_AYRACI).collect();
    if parcalar.len() < 2 {
        return (
            Zaman::SIFIR,
            Zaman::SIFIR,
            Some("zaman cizgisinde ayrac sayisi yanlis".to_string()),
        );
    }
    // Metin tarafında yazma biçimleri (`--> 00:00:04,000 X1:...`) için
    // konum ayarlayıcıları zaman damgasının parçası değildir; atılır.
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

/// SRT'ye özgü kayıp yoktur; dönüşüm kayıpları `bicim` katmanında hesaplanır.
pub fn kayiplar(_belge: &Belge) -> Vec<Kayip> {
    Vec::new()
}
