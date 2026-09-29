//! Güvenilirlik kural motoru: altı kural, eşik profili ve bulgu üretimi.
//!
//! Bu katman **biçim ayrıntılarını bilmez** (fikir raporu §06 "Kural motoru").
//! Yalnızca `Belge` üzerinde çalışır; aynı kural kümesi SRT, ASS ve VTT
//! dosyalarında birebir aynı sonucu verir. Yeni bir biçim eklendiğinde burada
//! hiçbir değişiklik gerekmez — bu, kural testlerini modelden bağımsız kılar.
//!
//! **Hiçbir düzeltme dosyaya yazılmaz.** Motor yalnızca bulgu üretir; düzeltme
//! isteyen kullanıcı `normalize`/`shift` komutlarını açıkça çalıştırır
//! (fikir raporu §03 onay kriteri 2).

use crate::model::{Belge, Bicim, Replik};
use crate::zaman::Zaman;
use std::fmt;

/// Denetlenen altı güvenilirlik kuralı.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Kural {
    /// Saniyedeki karakter sayısı üst sınırı aşıyor.
    OkumaHizi,
    /// Replik, okunabilmesi için gereken en kısa süreden kısa kalıyor.
    AsgariDinlenme,
    /// Satır karakter sayısı ekran sınırını aşıyor veya okunabilir satır sayısı fazla.
    SatirUzunlugu,
    /// İki replik aynı anda ekranda görünebiliyor.
    CakisanAraliklar,
    /// Sıra bozukluğu veya negatif süre.
    SiraBozuklugu,
    /// Karakter kodlaması bozukluğu (BOM, bozuk bayt, karışık kodlama).
    KodlamaBozuklugu,
}

impl Kural {
    /// Altı kuralın tamamı; rapor sırasını da bu vektör belirler.
    pub const TUMU: [Kural; 6] = [
        Kural::OkumaHizi,
        Kural::AsgariDinlenme,
        Kural::SatirUzunlugu,
        Kural::CakisanAraliklar,
        Kural::SiraBozuklugu,
        Kural::KodlamaBozuklugu,
    ];

    /// Kuralın Türkçe adı.
    pub fn ad(&self) -> &'static str {
        match self {
            Self::OkumaHizi => "okuma_hizi",
            Self::AsgariDinlenme => "asgari_dinlenme",
            Self::SatirUzunlugu => "satir_uzunlugu",
            Self::CakisanAraliklar => "cakisan_araliklar",
            Self::SiraBozuklugu => "sira_bozuklugu",
            Self::KodlamaBozuklugu => "kodlama_bozuklugu",
        }
    }

    /// Kuralın tam cümleyle açıklaması.
    pub fn aciklama(&self) -> &'static str {
        match self {
            Self::OkumaHizi => "saniyedeki karakter sayisi okuma hizi ust sinirini asiyor",
            Self::AsgariDinlenme => "replik okunabilmesi icin gereken en kisa sureden kisa",
            Self::SatirUzunlugu => {
                "satir karakter sayisi ekran sinirini asiyor veya satir sayisi fazla"
            }
            Self::CakisanAraliklar => "iki replik ayni anda ekranda gorunebiliyor",
            Self::SiraBozuklugu => "sira bozuk veya sure negatif",
            Self::KodlamaBozuklugu => "karakter kodlamasi bozuk (BOM, bozuk bayt, karisik kodlama)",
        }
    }
}

impl fmt::Display for Kural {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ad())
    }
}

/// Bulgunun ciddiyet derecesi.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Seviye {
    /// Dosya teslim edilemez; düzeltme gerekir.
    Kritik,
    /// Dikkat gerektirir, teslim edilebilir.
    Uyari,
}

impl Seviye {
    /// Seviyenin Türkçe adı.
    pub fn ad(&self) -> &'static str {
        match self {
            Self::Kritik => "kritik",
            Self::Uyari => "uyari",
        }
    }
}

impl fmt::Display for Seviye {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ad())
    }
}

/// Bulgunun dört metin alanı.
///
/// Bu alanlar ayrı bir yapıda tutulur çünkü hepsi "ne ölçüldü / eşik neydi /
/// ne yapılabilir / neden önemli" sorusunun yanıtıdır ve her bulguda birlikte
/// üretilirler; `Bulgu::yeni`nin sekiz argüman almasını önler.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BulguMetni {
    /// Ölçülen değer.
    pub olcum: String,
    /// Uygulanan eşik.
    pub esik: String,
    /// Kullanıcıya gösterilecek açıklama.
    pub mesaj: String,
    /// Önerilen, kullanıcı onayı gerektiren düzeltme.
    pub oneri: String,
}

impl BulguMetni {
    /// Dört metin alanından açıklama üretir.
    pub fn yeni(
        olcum: impl Into<String>,
        esik: impl Into<String>,
        mesaj: impl Into<String>,
        oneri: impl Into<String>,
    ) -> Self {
        Self {
            olcum: olcum.into(),
            esik: esik.into(),
            mesaj: mesaj.into(),
            oneri: oneri.into(),
        }
    }
}

/// Tek bir kural ihlali.
///
/// Her bulgu "ne ölçüldü / eşik neydi / ne yapılabilir" üçlüsünü taşır; bu
/// üçlü fikir raporu §03 onay kriteri 1'in (replik numarası, kural adı ve
/// önerilen düzeltme) doğrudan karşılığıdır.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bulgu {
    /// İhlal edilen kural.
    pub kural: Kural,
    /// Ciddiyet derecesi.
    pub seviye: Seviye,
    /// İlgili replik numarası; belge düzeyi bulgular için `None`.
    pub replik_no: Option<u32>,
    /// Çakışan kuralda ikinci repliğin numarası.
    pub iliskili_replik_no: Option<u32>,
    /// Repliğin dosyadaki ilk satırı (1 tabanlı).
    pub satir: usize,
    /// Ölçülen değer.
    pub olcum: String,
    /// Uygulanan eşik.
    pub esik: String,
    /// Kullanıcıya gösterilecek açıklama.
    pub mesaj: String,
    /// Önerilen, kullanıcı onayı gerektiren düzeltme.
    pub oneri: String,
}

impl Bulgu {
    /// Yeni bulgu kurar.
    pub fn yeni(
        kural: Kural,
        seviye: Seviye,
        replik_no: Option<u32>,
        satir: usize,
        metin: BulguMetni,
    ) -> Self {
        Self {
            kural,
            seviye,
            replik_no,
            iliskili_replik_no: None,
            satir,
            olcum: metin.olcum,
            esik: metin.esik,
            mesaj: metin.mesaj,
            oneri: metin.oneri,
        }
    }

    /// İkinci replik numarasını atar (çakışma kuralı için).
    pub fn iliskili_ile(mut self, no: u32) -> Self {
        self.iliskili_replik_no = Some(no);
        self
    }
}

/// Kural eşikleri; kullanıcı JSON profil dosyasıyla değiştirebilir.
///
/// Varsayılanlar fikir raporu §05 "Güvenilirlik kuralları" tablosundan alınmıştır
/// ve **ölçülmemiştir**; rapor kendisi de bunu belirtir.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct KuralEsikleri {
    /// Okuma hızı üst sınırı (karakter/saniye).
    pub okuma_hizi: f64,
    /// Asgari dinlenme süresi (milisaniye); okuma hızı alt sınırı da uygulanır.
    pub asgari_dinlenme_ms: i64,
    /// Asgari okuma hızı (karakter/saniye); bu hızın altında kalan repliğin
    /// süresi gereken değere yükseltilmelidir.
    pub asgari_hiz: f64,
    /// Okunabilir en uzun satır karakter sayısı.
    pub en_fazla_karakter: usize,
    /// Ekranda aynı anda okunabilecek en fazla satır sayısı.
    pub en_fazla_satir: usize,
    /// İki replik arasında kabul edilebilir en küçük boşluk (milisaniye).
    pub kabul_edilebilir_bosluk_ms: i64,
}

impl Default for KuralEsikleri {
    fn default() -> Self {
        Self {
            okuma_hizi: 17.0,
            asgari_dinlenme_ms: 1000,
            asgari_hiz: 6.0,
            en_fazla_karakter: 42,
            en_fazla_satir: 2,
            kabul_edilebilir_bosluk_ms: 0,
        }
    }
}

impl KuralEsikleri {
    /// Bir repliğin okunabilmesi için gereken en kısa süreyi verir.
    ///
    /// İki koşulun büyüğü alınır: mutlak asgari kalış (`asgari_dinlenme_ms`) ve
    /// asgari okuma hızından hesaplanan süre. Bu, fikir raporundaki
    /// "1,0 sn **veya** 6 karakter/sn" ifadesinin matematiksel karşılığıdır.
    pub fn gereken_sure_ms(&self, karakter_sayisi: usize) -> i64 {
        let hizden = if self.asgari_hiz > 0.0 {
            (karakter_sayisi as f64 / self.asgari_hiz * 1000.0).ceil() as i64
        } else {
            0
        };
        hizden.max(self.asgari_dinlenme_ms)
    }
}

/// Belgeyi altı kuralla denetler ve bulguları kural sırasına göre döndürür.
pub fn denetle(belge: &Belge, esikler: &KuralEsikleri) -> Vec<Bulgu> {
    let mut bulgular: Vec<Bulgu> = Vec::new();
    okuma_hizi_denetle(belge, esikler, &mut bulgular);
    asgari_dinlenme_denetle(belge, esikler, &mut bulgular);
    satir_uzunlugu_denetle(belge, esikler, &mut bulgular);
    cakisan_aralik_denetle(belge, esikler, &mut bulgular);
    sira_bozuklugu_denetle(belge, &mut bulgular);
    kodlama_denetle(belge, &mut bulgular);
    bulgular
}

/// Kural 1 — okuma hızı.
fn okuma_hizi_denetle(belge: &Belge, esikler: &KuralEsikleri, bulgular: &mut Vec<Bulgu>) {
    for r in &belge.replikler {
        if r.sure_ms() <= 0 {
            continue;
        }
        let hiz = r.okuma_hizi();
        if hiz > esikler.okuma_hizi {
            bulgular.push(Bulgu::yeni(
                Kural::OkumaHizi,
                Seviye::Uyari,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    format!("{hiz:.1} karakter/sn"),
                    format!("{:.1} karakter/sn", esikler.okuma_hizi),
                    format!(
                        "replik {} icinde {:.1} karakter/saniye okunuyor",
                        r.numara, hiz
                    ),
                    "sureyi uzat veya repligi ikiye bol",
                ),
            ));
        }
    }
}

/// Kural 2 — asgari dinlenme süresi.
fn asgari_dinlenme_denetle(belge: &Belge, esikler: &KuralEsikleri, bulgular: &mut Vec<Bulgu>) {
    for r in &belge.replikler {
        let karakter = r.karakter_sayisi();
        let gereken = esikler.gereken_sure_ms(karakter);
        let gercek = r.sure_ms();
        // Negatif süre bu kuralın değil "sira bozuklugu" kuralının işidir.
        if gercek >= 0 && gercek < gereken {
            bulgular.push(Bulgu::yeni(
                Kural::AsgariDinlenme,
                Seviye::Uyari,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    format!("{gercek} ms ({karakter} karakter)"),
                    format!("{gereken} ms"),
                    format!(
                        "replik {} yalnizca {} ms kaliyor, {karakter} karakter icin gereken {gereken} ms",
                        r.numara, gercek
                    ),
                    "sureyi gereken degere yukselt",
                ),
            ));
        }
    }
}

/// Kural 3 — satır uzunluğu ve okunabilir satır sayısı.
fn satir_uzunlugu_denetle(belge: &Belge, esikler: &KuralEsikleri, bulgular: &mut Vec<Bulgu>) {
    for r in &belge.replikler {
        let satirlar = r.satirlar();
        for (indeks, satir) in satirlar.iter().enumerate() {
            let uzunluk = satir.chars().count();
            if uzunluk > esikler.en_fazla_karakter {
                bulgular.push(Bulgu::yeni(
                    Kural::SatirUzunlugu,
                    Seviye::Uyari,
                    Some(r.numara),
                    indeks + 1,
                    BulguMetni::yeni(
                        format!("{uzunluk} karakter"),
                        format!("{} karakter", esikler.en_fazla_karakter),
                        format!(
                            "replik {} satir {} cok uzun ({uzunluk} karakter)",
                            r.numara,
                            indeks + 1
                        ),
                        "satiri bol veya yeniden sar",
                    ),
                ));
            }
        }
        if satirlar.len() > esikler.en_fazla_satir {
            bulgular.push(Bulgu::yeni(
                Kural::SatirUzunlugu,
                Seviye::Uyari,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    format!("{} satir", satirlar.len()),
                    format!("{} satir", esikler.en_fazla_satir),
                    format!(
                        "replik {}'da {} satir var, okunabilir en fazla {} satir olabilir",
                        r.numara,
                        satirlar.len(),
                        esikler.en_fazla_satir
                    ),
                    "satirlari birlestir veya repligi ikiye bol",
                ),
            ));
        }
    }
}

/// Kural 4 — çakışan zaman aralıkları.
fn cakisan_aralik_denetle(belge: &Belge, esikler: &KuralEsikleri, bulgular: &mut Vec<Bulgu>) {
    for pencere in belge.replikler.windows(2) {
        let (onceki, sonraki) = (&pencere[0], &pencere[1]);
        if !onceki.zamanlama_gecerli() || !sonraki.zamanlama_gecerli() {
            continue;
        }
        let cakisma = onceki.bitis.deger() - sonraki.baslangic.deger();
        if cakisma > esikler.kabul_edilebilir_bosluk_ms {
            bulgular.push(
                Bulgu::yeni(
                    Kural::CakisanAraliklar,
                    Seviye::Kritik,
                    Some(onceki.numara),
                    0,
                    BulguMetni::yeni(
                        format!("{cakisma} ms cakisma"),
                        format!("> {} ms", esikler.kabul_edilebilir_bosluk_ms),
                        format!(
                            "replik {} ve {} ayni anda ekranda gorunebiliyor ({cakisma} ms)",
                            onceki.numara, sonraki.numara
                        ),
                        "sonraki repligi oncekinin bitisine kaydir",
                    ),
                )
                .iliskili_ile(sonraki.numara),
            );
        }
    }
}

/// Kural 5 — sıra bozukluğu ve negatif süre.
fn sira_bozuklugu_denetle(belge: &Belge, bulgular: &mut Vec<Bulgu>) {
    for (indeks, r) in belge.replikler.iter().enumerate() {
        let sure = r.sure_ms();
        if sure < 0 {
            bulgular.push(Bulgu::yeni(
                Kural::SiraBozuklugu,
                Seviye::Kritik,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    format!("{sure} ms"),
                    "0 ms",
                    format!(
                        "replik {}'da bitis baslangictan once ({sure} ms negatif sure)",
                        r.numara
                    ),
                    "bitis zamanini baslangictan sonra tasi",
                ),
            ));
        } else if sure == 0 {
            bulgular.push(Bulgu::yeni(
                Kural::SiraBozuklugu,
                Seviye::Kritik,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    "0 ms",
                    "> 0 ms",
                    format!("replik {}'da baslangic ve bitis ayni", r.numara),
                    "bitis zamanini ilerlet",
                ),
            ));
        }
        if indeks == 0 {
            continue;
        }
        let onceki = &belge.replikler[indeks - 1];
        if r.baslangic < onceki.baslangic {
            bulgular.push(Bulgu::yeni(
                Kural::SiraBozuklugu,
                Seviye::Kritik,
                Some(r.numara),
                0,
                BulguMetni::yeni(
                    format!("{} < {}", r.baslangic.srt_yaz(), onceki.baslangic.srt_yaz()),
                    "baslangic artan sirada",
                    format!(
                        "replik {} onceki replik {}'den once basliyor; dosya sirasi bozuk",
                        r.numara, onceki.numara
                    ),
                    "normalize --sirala ile siralamayi duzelt",
                ),
            ));
        }
    }
}

/// Kural 6 — karakter kodlaması bozukluğu.
fn kodlama_denetle(belge: &Belge, bulgular: &mut Vec<Bulgu>) {
    let k = &belge.kodlama;
    if k.bom_var {
        bulgular.push(Bulgu::yeni(
            Kural::KodlamaBozuklugu,
            Seviye::Uyari,
            None,
            0,
            BulguMetni::yeni(
                format!("{} (BOM)", k.kodlama),
                "BOM'suz UTF-8",
                "dosya bayt sirasi isareti (BOM) ile basliyor; bazi oynaticilar ilk satiri bos sayar",
                "dosyayi BOM'suz UTF-8 olarak yeniden yaz",
            ),
        ));
    }
    if k.karisik_kodlama {
        bulgular.push(Bulgu::yeni(
            Kural::KodlamaBozuklugu,
            Seviye::Kritik,
            None,
            0,
            BulguMetni::yeni(
                format!("{} tek bayt satir", k.tek_bayt_satir_sayisi),
                "tek kodlama",
                "dosyada hem UTF-8 hem tek bayt satirlar var; kodlama karisik",
                "dosyayi tek kodlamaya (UTF-8) cevir",
            ),
        ));
    }
    if k.bozuk_bayt_sayisi > 0 {
        bulgular.push(Bulgu::yeni(
            Kural::KodlamaBozuklugu,
            Seviye::Kritik,
            None,
            0,
            BulguMetni::yeni(
                format!("{} bozuk bayt dizisi", k.bozuk_bayt_sayisi),
                "0",
                "dosyada UTF-8 olarak cozulemeyen bayt dizileri var",
                "dosyayi yeniden kodla",
            ),
        ));
    }
    if k.kontrol_isareti_sayisi > 0 && !k.karisik_kodlama {
        bulgular.push(Bulgu::yeni(
            Kural::KodlamaBozuklugu,
            Seviye::Uyari,
            None,
            0,
            BulguMetni::yeni(
                format!("{} U+FFFD", k.kontrol_isareti_sayisi),
                "0",
                "kaynakta degistirme isareti (U+FFFD) var; karakterler kaybolmus olabilir",
                "kaynak dosyayi kontrol et",
            ),
        ));
    }
}

/// Bir bulgu listesini kural bazında özetler.
pub fn kural_ozeti(bulgular: &[Bulgu]) -> Vec<(Kural, usize)> {
    Kural::TUMU
        .iter()
        .map(|k| {
            let adet = bulgular.iter().filter(|b| b.kural == *k).count();
            (*k, adet)
        })
        .collect()
}

/// Kural listesini biçim bağımsız bir özet nesnesiyle döndürür (JSON çıktısı).
pub fn ozet_satiri(kural: Kural, adet: usize) -> KuralOzeti {
    KuralOzeti {
        kural: kural.ad().to_string(),
        adet,
    }
}

/// JSON raporunda kullanılan kural özeti.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct KuralOzeti {
    /// Kuralın kanonik adı.
    pub kural: String,
    /// Bu kuralın ürettiği bulgu adedi.
    pub adet: usize,
}

/// Denetimde kullanılan biçimi doğrulayan yardımcı (test ve rapor için).
pub fn bicim_uyumlu(belge: &Belge) -> bool {
    belge.bicim == Bicim::Srt || belge.bicim == Bicim::Ass || belge.bicim == Bicim::Vtt
}

/// Bir repliğin okunabilir süre gereksinimini metin olarak döndürür (rapor için).
pub fn sure_gerekcesi(r: &Replik, esikler: &KuralEsikleri) -> String {
    let gereken = esikler.gereken_sure_ms(r.karakter_sayisi());
    format!("{} ms ({} karakter)", gereken, r.karakter_sayisi())
}

/// Bir zaman damgasını insan tarafından okunur biçimde döndürür.
pub fn zaman_metni(z: Zaman) -> String {
    z.srt_yaz()
}
