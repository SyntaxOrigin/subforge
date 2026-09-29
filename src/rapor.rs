//! Çıktı katmanı: JSON kanıt raporu ve terminal tablosu.
//!
//! İki çıktı biçimi aynı veriden üretilir; böylece `--json` ile makine okuyan
//! çıktı alan kullanıcıyla aynı bulguları paylaşır. Terminal çıktısı ASCII
//! sütunlarla hizalanır ki Windows konsolunda da okunur olsun.

use crate::islem::{DonusturmeRaporu, KaydirmaRaporu, NormalizeRaporu};
use crate::kural::{ozet_satiri, Bulgu, Kural, KuralEsikleri, KuralOzeti, Seviye};
use crate::model::{Belge, SozdizimiKaydi};
use crate::zaman::Zaman;
use serde::Serialize;

/// Denetim çıktısının tamamı; `serde_json` ile serileştirilir.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DenetimRaporu {
    /// Araç sürümü.
    pub surum: &'static str,
    /// Denetlenen dosyanın görüntü metni.
    pub dosya: String,
    /// Kaynak biçim.
    pub bicim: String,
    /// Toplam replik sayısı.
    pub replik_sayisi: usize,
    /// Toplam satır sayısı.
    pub satir_sayisi: usize,
    /// Belgenin toplam süresi (milisaniye).
    pub toplam_sure_ms: i64,
    /// Kodlama denetimi özeti.
    pub kodlama: crate::kodlama::KodlamaRaporu,
    /// Uygulanan eşikler.
    pub esikler: KuralEsikleri,
    /// Kural başına bulgu adedi.
    pub kural_ozeti: Vec<KuralOzeti>,
    /// Bulgu sayısı.
    pub bulgu_sayisi: usize,
    /// Kritik bulgu sayısı.
    pub kritik_sayisi: usize,
    /// Bulgular; dosya sırasıyla.
    pub bulgular: Vec<Bulgu>,
    /// Stil tablosunda tanımsız stil atıfları.
    pub tanimsiz_stiller: Vec<String>,
    /// Okuma sırasında karşılaşılan sözdizimi hataları.
    pub sozdizimi: Vec<SozdizimiKaydi>,
}

impl DenetimRaporu {
    /// Belge, dosya yolu, eşikler ve bulgularla rapor kurar.
    pub fn kur(dosya: &str, belge: &Belge, esikler: &KuralEsikleri, bulgular: Vec<Bulgu>) -> Self {
        let bulgular = sirala_bulgular(bulgular);
        let kritik_sayisi = bulgular
            .iter()
            .filter(|b| b.seviye == Seviye::Kritik)
            .count();
        Self {
            surum: crate::SURUM,
            dosya: dosya.to_string(),
            bicim: belge.bicim.ad().to_string(),
            replik_sayisi: belge.replik_sayisi(),
            satir_sayisi: belge.toplam_satir(),
            toplam_sure_ms: belge.son_zaman().deger(),
            kodlama: belge.kodlama.clone(),
            esikler: esikler.clone(),
            kural_ozeti: Kural::TUMU
                .iter()
                .map(|k| ozet_satiri(*k, bulgular.iter().filter(|b| b.kural == *k).count()))
                .collect(),
            bulgu_sayisi: bulgular.len(),
            kritik_sayisi,
            bulgular,
            tanimsiz_stiller: belge.tanimsiz_stiller.clone(),
            sozdizimi: belge.sozdizimi.clone(),
        }
    }

    /// Denetimden geçti mi (kritik bulgu yoksa ve sözdizimi hatası yoksa).
    pub fn gecti(&self) -> bool {
        self.kritik_sayisi == 0 && self.sozdizimi.is_empty()
    }

    /// Raporu JSON metnine çevirir.
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"hata\":\"{e}\"}}"))
    }

    /// Raporu terminal tablosuna çevirir.
    pub fn terminal(&self) -> String {
        let mut cikti = String::new();
        cikti.push_str("=== subforge check ===\n");
        satir_ekle(&mut cikti, "dosya", &self.dosya);
        satir_ekle(&mut cikti, "bicim", &self.bicim);
        satir_ekle(
            &mut cikti,
            "kodlama",
            &format!(
                "{} (BOM: {}, bozuk bayt: {}, karisik: {})",
                self.kodlama.kodlama,
                if self.kodlama.bom_var { "var" } else { "yok" },
                self.kodlama.bozuk_bayt_sayisi,
                if self.kodlama.karisik_kodlama {
                    "var"
                } else {
                    "yok"
                }
            ),
        );
        satir_ekle(
            &mut cikti,
            "replik",
            &format!(
                "{}  ({} satir, {} sure)",
                self.replik_sayisi,
                self.satir_sayisi,
                sure_metni(self.toplam_sure_ms)
            ),
        );
        cikti.push('\n');
        cikti.push_str("kural             adet  aciklama\n");
        cikti.push_str("----------------- ----- ---------------------------------------------\n");
        for ozet in &self.kural_ozeti {
            let kural = Kural::TUMU
                .iter()
                .find(|k| k.ad() == ozet.kural)
                .copied()
                .unwrap_or(Kural::OkumaHizi);
            cikti.push_str(&format!(
                "{:<17} {:>5}  {}\n",
                ozet.kural,
                ozet.adet,
                kural.aciklama()
            ));
        }
        cikti.push('\n');
        if self.bulgular.is_empty() {
            cikti.push_str("bulgu yok: tum kurallar saglandi.\n");
        } else {
            cikti.push_str("  #  replik  seviye  kural               olcum / esik\n");
            cikti.push_str("  -  ------  -------  ------------------  ------------------------\n");
            for (i, b) in self.bulgular.iter().enumerate() {
                cikti.push_str(&format!(
                    "{:>3}  {:<6}  {:<7}  {:<18}  {} / {}\n",
                    i + 1,
                    b.replik_no
                        .map_or_else(|| "-".to_string(), |n| n.to_string()),
                    b.seviye.ad(),
                    b.kural.ad(),
                    b.olcum,
                    b.esik
                ));
            }
        }
        if !self.tanimsiz_stiller.is_empty() {
            cikti.push('\n');
            cikti.push_str(&format!(
                "uyari: stil tablosunda tanimsiz stil referanslari: {}\n",
                self.tanimsiz_stiller.join(", ")
            ));
        }
        if !self.sozdizimi.is_empty() {
            cikti.push('\n');
            cikti.push_str("sozdizimi hatalari:\n");
            for h in &self.sozdizimi {
                cikti.push_str(&format!("  satir {}: {}\n", h.satir, h.mesaj));
            }
        }
        cikti.push('\n');
        cikti.push_str(&format!(
            "sonuc: {} ({} bulgu, {} kritik)\n",
            if self.gecti() { "GECTI" } else { "KALDI" },
            self.bulgu_sayisi,
            self.kritik_sayisi
        ));
        cikti
    }
}

/// Kaydırma işlemi çıktısı.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KaydirmaCiktisi {
    /// Araç sürümü.
    pub surum: &'static str,
    /// Kaynak dosya.
    pub dosya: String,
    /// Çıktı dosyası.
    pub cikti: String,
    /// Kaydırma özeti.
    pub kaydirma: KaydirmaRaporu,
    /// Kaydırma sonrası kalan çakışan replik çiftleri.
    pub kalan_cakisma: usize,
}

/// Kaydırma çıktısını terminal biçimine çevirir.
pub fn kaydirma_terminal(cikti: &KaydirmaCiktisi) -> String {
    let mut s = String::new();
    s.push_str("=== subforge shift ===\n");
    satir_ekle(&mut s, "dosya", &cikti.dosya);
    satir_ekle(&mut s, "cikti", &cikti.cikti);
    satir_ekle(
        &mut s,
        "kaydirma",
        &format!("{} ms", cikti.kaydirma.istenen_ms),
    );
    satir_ekle(
        &mut s,
        "etkilenen",
        &cikti.kaydirma.etkilenen_replik.to_string(),
    );
    if cikti.kaydirma.kirpma_var() {
        satir_ekle(
            &mut s,
            "kirpilan",
            &format!(
                "{} replik 0.00 saniyeye kirpildi",
                cikti.kaydirma.kirpilan_replik
            ),
        );
    }
    satir_ekle(&mut s, "kalan cakisma", &cikti.kalan_cakisma.to_string());
    s
}

/// Normalizasyon çıktısı.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NormalizeCiktisi {
    /// Araç sürümü.
    pub surum: &'static str,
    /// Kaynak dosya.
    pub dosya: String,
    /// Çıktı dosyası.
    pub cikti: String,
    /// Uygulanan kare hızı (metin).
    pub kare_hizi: Option<String>,
    /// Normalizasyon özeti.
    pub islem: NormalizeRaporu,
}

/// Normalizasyon çıktısını terminal biçimine çevirir.
pub fn normalize_terminal(cikti: &NormalizeCiktisi) -> String {
    let mut s = String::new();
    s.push_str("=== subforge normalize ===\n");
    satir_ekle(&mut s, "dosya", &cikti.dosya);
    satir_ekle(&mut s, "cikti", &cikti.cikti);
    satir_ekle(
        &mut s,
        "kare hizi",
        cikti.kare_hizi.as_deref().unwrap_or("uygulanmadi"),
    );
    satir_ekle(
        &mut s,
        "kareye oturtulan",
        &cikti.islem.kareye_oturtulan.to_string(),
    );
    satir_ekle(&mut s, "siralanan", &cikti.islem.siralanan.to_string());
    satir_ekle(&mut s, "numaralanan", &cikti.islem.numaralanan.to_string());
    satir_ekle(
        &mut s,
        "toplam sure",
        &sure_metni(cikti.islem.toplam_sure_ms),
    );
    s
}

/// Dönüştürme çıktısı.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DonusturmeCiktisi {
    /// Araç sürümü.
    pub surum: &'static str,
    /// Kaynak dosya.
    pub dosya: String,
    /// Çıktı dosyası.
    pub cikti: String,
    /// Kaynak biçim.
    pub kaynak_bicim: String,
    /// Dönüşüm özeti.
    pub donusturme: DonusturmeRaporu,
}

/// Dönüştürme çıktısını terminal biçimine çevirir.
pub fn donusturme_terminal(cikti: &DonusturmeCiktisi) -> String {
    let mut s = String::new();
    s.push_str("=== subforge convert ===\n");
    satir_ekle(&mut s, "dosya", &cikti.dosya);
    satir_ekle(&mut s, "cikti", &cikti.cikti);
    satir_ekle(
        &mut s,
        "donusturme",
        &format!("{} -> {}", cikti.kaynak_bicim, cikti.donusturme.hedef),
    );
    satir_ekle(
        &mut s,
        "replik",
        &cikti.donusturme.replik_sayisi.to_string(),
    );
    s.push('\n');
    if cikti.donusturme.kayiplar.is_empty() {
        s.push_str("kayip yok: tum ozellikler korundu.\n");
    } else {
        s.push_str("kaybolan ozellikler:\n");
        for k in &cikti.donusturme.kayiplar {
            s.push_str(&format!("  - {}\n", k.ozellik));
            s.push_str(&format!("      deger : {}\n", k.deger));
            s.push_str(&format!("      neden : {}\n", k.gerekce));
        }
    }
    s
}

/// Milisaniyeyi `HH:MM:SS.mmm` biçimine çevirir.
pub fn sure_metni(ms: i64) -> String {
    Zaman::milis(ms.max(0)).srt_yaz().replacen(',', ".", 1)
}

/// JSON çıktısı üretir; serileştirme hatası kullanıcı hatası olarak döner.
pub fn json_cikti<T: Serialize>(deger: &T) -> Result<String, crate::hata::SubForgeHata> {
    serde_json::to_string_pretty(deger)
        .map_err(|e| crate::hata::SubForgeHata::gecersiz_arguman(format!("JSON uretilemedi: {e}")))
}

/// `anahtar: deger` biçiminde hizalı satır ekler.
fn satir_ekle(cikti: &mut String, anahtar: &str, deger: &str) {
    cikti.push_str(&format!("{:<9}: {deger}\n", anahtar));
}

/// Bulguları dosya sırasına göre kararlı biçimde sıralar.
///
/// Sıralama ölçütleri: replik numarası, sonra bulgunun kaynak satırı, sonra
/// kural. Böylece aynı dosya her zaman aynı sırayı üretir (test edilebilirlik).
fn sirala_bulgular(mut bulgular: Vec<Bulgu>) -> Vec<Bulgu> {
    bulgular.sort_by(|a, b| {
        a.replik_no
            .unwrap_or(0)
            .cmp(&b.replik_no.unwrap_or(0))
            .then(a.satir.cmp(&b.satir))
            .then(a.kural.cmp(&b.kural))
            .then(a.mesaj.cmp(&b.mesaj))
    });
    bulgular
}
