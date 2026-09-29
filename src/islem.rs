//! Zamanlama işlemleri: toplu kaydırma, normalizasyon ve biçim dönüştürme.
//!
//! Fikir raporu §03 onay kriteri 3: "zaman kaydırma sonrası tüm zaman
//! damgaları kare sınırına oturtulur ve dosya içi çakışma oluşmaz". Bu modül
//! o davranışı `kaydir` + `normalize` çiftiyle sağlar.
//!
//! Bu katman **kural motorunu bilmez**; kullanıcı bir işlemi açıkça istediği
//! için yazılır. Denetim (`check`) ise hiçbir zaman dosyayı değiştirmez.

use crate::bicim::{
    metne_cevir_kayipli, varsayilan_ass_olay_format, varsayilan_ass_stil_format, Kayip,
};
use crate::model::{Belge, Bicim, Replik};
use crate::zaman::KareHazi;

/// Toplu kaydırma sonucunu özetleyen kayıt.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct KaydirmaRaporu {
    /// İstenen kaydırma (milisaniye; negatif olabilir).
    pub istenen_ms: i64,
    /// Gerçekte uygulanan kaydırma (milisaniye).
    pub uygulanan_ms: i64,
    /// Sıfıra kırpılan replik adedi (`0.00` giriş noktasına düşenler).
    pub kirpilan_replik: usize,
    /// Kaydırılan replik adedi.
    pub etkilenen_replik: usize,
}

impl KaydirmaRaporu {
    /// Kırpılan replik var mı.
    pub fn kirpma_var(&self) -> bool {
        self.kirpilan_replik > 0
    }
}

/// Tüm repliklerin başlangıç ve bitiş zamanlarını `kaydirma_ms` kadar kaydırır.
///
/// Negatif kaydırmada 0.00 saniyenin altına düşen zamanlar **kırpılır** ve
/// 0'a sabitlenir; rapor bunu `kirpilan_replik` ile bildirir. Böylece negatif
/// süreli replik üretilmez, ama sessizce de kaybolmaz.
pub fn kaydir(belge: &mut Belge, kaydirma_ms: i64) -> KaydirmaRaporu {
    let mut rapor = KaydirmaRaporu {
        istenen_ms: kaydirma_ms,
        ..KaydirmaRaporu::default()
    };
    for replik in &mut belge.replikler {
        let yeni_baslangic = (replik.baslangic.deger() + kaydirma_ms).max(0);
        let yeni_bitis = (replik.bitis.deger() + kaydirma_ms).max(0);
        if yeni_baslangic != replik.baslangic.deger() || yeni_bitis != replik.bitis.deger() {
            rapor.etkilenen_replik += 1;
        }
        if replik.baslangic.deger() + kaydirma_ms < 0 || replik.bitis.deger() + kaydirma_ms < 0 {
            rapor.kirpilan_replik += 1;
        }
        replik.baslangic = crate::zaman::Zaman::milis(yeni_baslangic);
        replik.bitis = crate::zaman::Zaman::milis(yeni_bitis);
    }
    rapor.uygulanan_ms = kaydirma_ms;
    rapor
}

/// Yüzde olarak kaydırma.
///
/// Yüzde, belgenin toplam süresine göredir: `%10` = sürenin onda biri kadar
/// kaydırma. Toplam süre sıfırsa işlem yapılmaz ve `0` döner.
pub fn kaydir_yuzde(belge: &mut Belge, yuzde: f64) -> KaydirmaRaporu {
    let toplam = belge.son_zaman().deger();
    if toplam <= 0 || !yuzde.is_finite() {
        return KaydirmaRaporu {
            istenen_ms: 0,
            uygulanan_ms: 0,
            kirpilan_replik: 0,
            etkilenen_replik: 0,
        };
    }
    let ms = (toplam as f64 * yuzde / 100.0).round() as i64;
    let mut rapor = kaydir(belge, ms);
    rapor.istenen_ms = ms;
    rapor
}

/// Normalizasyon sonucunu özetleyen kayıt.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
pub struct NormalizeRaporu {
    /// Kare hızına oturtulan zaman damgası adedi.
    pub kareye_oturtulan: usize,
    /// Sıralanan replik adedi.
    pub siralanan: usize,
    /// Yeniden numaralandırılan replik adedi.
    pub numaralanan: usize,
    /// Dosyanın toplam süresi (milisaniye), işlem sonrası.
    pub toplam_sure_ms: i64,
}

/// Belgeyi normalize eder.
///
/// - `hiz` verilirse tüm zaman damgaları en yakın kare sınırına oturtulur
///   (fikir raporu §07 formülü).
/// - `sirala` ise replikler başlangıç zamanına göre kararlı biçimde sıralanır.
/// - `numarala` ise replik numaraları 1'den başlayarak yeniden yazılır.
pub fn normalize(
    belge: &mut Belge,
    hiz: Option<KareHazi>,
    sirala: bool,
    numarala: bool,
) -> NormalizeRaporu {
    let mut rapor = NormalizeRaporu::default();
    if let Some(kare_hizi) = hiz {
        for replik in &mut belge.replikler {
            let y_bas = replik.baslangic.kareye_yuvarla(kare_hizi);
            let y_bit = replik.bitis.kareye_yuvarla(kare_hizi);
            if y_bas != replik.baslangic || y_bit != replik.bitis {
                rapor.kareye_oturtulan += 1;
            }
            replik.baslangic = y_bas;
            replik.bitis = y_bit;
        }
    }
    if sirala {
        let sirali = belge.sirali();
        if sirali != belge.replikler {
            rapor.siralanan = belge
                .replikler
                .iter()
                .zip(sirali.iter())
                .filter(|(a, b)| a != b)
                .count();
            belge.replikler = sirali;
        }
    }
    if numarala {
        for (indeks, replik) in belge.replikler.iter_mut().enumerate() {
            let yeni = u32::try_from(indeks + 1).unwrap_or(u32::MAX);
            if replik.numara != yeni {
                rapor.numaralanan += 1;
            }
            replik.numara = yeni;
        }
    }
    // Kare sınırına oturtma sonrası bitiş başlangıçtan geriye düşebilir
    // (1 kareden kısa repliklerde); bu durumda süre en az bir kare tutulur.
    if let Some(kare_hizi) = hiz {
        let kare_suresi = kare_hizi.zaman_ms(1) - kare_hizi.zaman_ms(0);
        for replik in &mut belge.replikler {
            if replik.bitis.deger() <= replik.baslangic.deger() {
                replik.bitis = crate::zaman::Zaman::milis(replik.baslangic.deger() + kare_suresi);
            }
        }
    }
    rapor.toplam_sure_ms = belge.son_zaman().deger();
    rapor
}

/// Dönüşüm sonucunu özetleyen kayıt.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DonusturmeRaporu {
    /// Hedef biçim.
    pub hedef: Bicim,
    /// Aktarılan replik adedi.
    pub replik_sayisi: usize,
    /// Kaybolan özellikler.
    pub kayiplar: Vec<Kayip>,
}

/// Belgeyi hedef biçime çevirir ve kaybolan özellikleri raporlar.
///
/// ASS hedefiyse stil tablosu boşsa varsayılan tablo üretilir; bu üretim
/// kayıp listesinde "kaynak belgede stil tablosu yok" olarak **açıkça** listelenir
/// (fikir raporu §03 onay kriteri 4: kaybolan özellikler sessizce atılmaz).
pub fn donustur(belge: &Belge, hedef: Bicim) -> (Belge, DonusturmeRaporu) {
    let (mut yeni, kayiplar) = metne_cevir_kayipli(belge, hedef);
    if hedef == Bicim::Ass && yeni.stiller.is_empty() {
        yeni.stiller = crate::bicim::varsayilan_ass_stilleri();
        yeni.stil_format = varsayilan_ass_stil_format();
        yeni.olay_format = varsayilan_ass_olay_format();
        yeni.basliklar = varsayilan_script_info(belge);
        for replik in &mut yeni.replikler {
            if replik.stil.is_none() {
                replik.stil = Some("Default".to_string());
            }
        }
    }
    let rapor = DonusturmeRaporu {
        hedef,
        replik_sayisi: yeni.replikler.len(),
        kayiplar,
    };
    (yeni, rapor)
}

/// SRT/WebVTT'den ASS'e geçerken oluşturulan varsayılan `Script Info` alanları.
fn varsayilan_script_info(belge: &Belge) -> Vec<(String, String)> {
    vec![
        ("Title".to_string(), "SubForge".to_string()),
        ("ScriptType".to_string(), "v4+".to_string()),
        ("WrapStyle".to_string(), "0".to_string()),
        (
            "PlayResX".to_string(),
            if belge.bicim == Bicim::Vtt {
                "1920"
            } else {
                "384"
            }
            .to_string(),
        ),
        (
            "PlayResY".to_string(),
            if belge.bicim == Bicim::Vtt {
                "1080"
            } else {
                "288"
            }
            .to_string(),
        ),
        ("ScaledBorderAndShadow".to_string(), "yes".to_string()),
    ]
}

/// Çakışan replikleri, kaydırma sonrası düzeltmek için listeler.
///
/// `resolve` komutundan bağımsız bir yardımcıdır; kullanıcı bu listeyi görüp
/// karar verir (rapor: hiçbir düzeltme onaysız uygulanmaz).
pub fn cakisan_ciftler(belge: &Belge) -> Vec<(u32, u32, i64)> {
    let mut ciftler = Vec::new();
    for pencere in belge.replikler.windows(2) {
        let (a, b) = (&pencere[0], &pencere[1]);
        if !a.zamanlama_gecerli() || !b.zamanlama_gecerli() {
            continue;
        }
        let cakisma = a.bitis.deger() - b.baslangic.deger();
        if cakisma > 0 {
            ciftler.push((a.numara, b.numara, cakisma));
        }
    }
    ciftler
}

/// Belgenin en uzun replik metnini döndürür (istatistik raporu için).
pub fn en_uzun_replik(belge: &Belge) -> Option<&Replik> {
    belge.replikler.iter().max_by_key(|r| r.karakter_sayisi())
}
