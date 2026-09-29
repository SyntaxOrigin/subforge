//! Çekirdek veri modeli: biçimden bağımsız `Belge`, `Replik` ve ASS `Stil`.
//!
//! Bu katman (fikir raporu §06 "Çekirdek") biçim ayrıntılarını bilmez. Kural
//! motoru yalnızca bu modeli görür; bir biçim eklendiğinde kural motorunun
//! değişmemesi bu katmanın varlık sebebidir.

use crate::kodlama::KodlamaRaporu;
use crate::zaman::Zaman;

/// Desteklenen altyazı biçimleri.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Bicim {
    /// SubRip (`.srt`).
    #[default]
    Srt,
    /// Advanced SubStation Alpha / SubStation Alpha (`.ass`, `.ssa`).
    Ass,
    /// Web Video Text Tracks (`.vtt`).
    Vtt,
}

impl Bicim {
    /// Biçimin kanonik küçük harfli adı.
    pub fn ad(&self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Ass => "ass",
            Self::Vtt => "vtt",
        }
    }

    /// Komut satırından gelen adı biçime çevirir.
    pub fn addan(ad: &str) -> Option<Self> {
        match ad.trim().to_ascii_lowercase().as_str() {
            "srt" | "subrip" => Some(Self::Srt),
            "ass" | "ssa" | "ssa1" => Some(Self::Ass),
            "vtt" | "webvtt" => Some(Self::Vtt),
            _ => None,
        }
    }

    /// Biçimin varsayılan dosya uzantısı.
    pub fn uzanti(&self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Ass => "ass",
            Self::Vtt => "vtt",
        }
    }
}

impl std::fmt::Display for Bicim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ad())
    }
}

/// ASS `Dialogue` satırının türü; `Comment` satırları da korunur çünkü
/// dönüşümde sessizce düşürülürlerse kayıp raporunda görünmezler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OlayTuru {
    /// `Dialogue:` — ekranda görünen altyazı.
    #[default]
    Diyalog,
    /// `Comment:` — oynatıcıda görünmeyen açıklama satırı.
    Yorum,
}

impl OlayTuru {
    /// Olay türünün ASS karşılığındaki sözcük.
    pub fn ass_etiketi(&self) -> &'static str {
        match self {
            Self::Diyalog => "Dialogue",
            Self::Yorum => "Comment",
        }
    }

    /// ASS etiketinden olay türünü çözer.
    pub fn ass_etiketten(etiket: &str) -> Self {
        if etiket.trim().eq_ignore_ascii_case("comment") {
            Self::Yorum
        } else {
            Self::Diyalog
        }
    }
}

/// Biçimden bağımsız bir altyazı repliği.
///
/// `ad`, `kenar_*`, `efekt` ve `katman` alanları yalnızca ASS'te anlamlıdır;
/// SRT ve VTT yazıcıları bunları yok sayar, okuyucular varsayılan değerle
/// doldurur. Bu sayede ASS gidiş-dönüşü kayıpsızdır.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Replik {
    /// Dosyadaki replik numarası; `normalize` ile yeniden numaralandırılabilir.
    pub numara: u32,
    /// Başlangıç zaman damgası.
    pub baslangic: Zaman,
    /// Bitiş zaman damgası.
    pub bitis: Zaman,
    /// Replik metni; satırlar `\n` ile ayrılmıştır ve sonunda `\n` yoktur.
    pub metin: String,
    /// ASS stil adı; SRT ve VTT için `None`.
    pub stil: Option<String>,
    /// ASS `Name` alanı (konuşmacı etiketi).
    pub ad: Option<String>,
    /// ASS `MarginL` alanı.
    pub kenar_sol: i32,
    /// ASS `MarginR` alanı.
    pub kenar_sag: i32,
    /// ASS `MarginV` alanı.
    pub kenar_ust: i32,
    /// ASS `Effect` alanı.
    pub efekt: String,
    /// ASS `Layer` alanı.
    pub katman: i32,
    /// Olay türü.
    pub tur: OlayTuru,
}

impl Replik {
    /// Varsayılan alanlarla boş bir replik oluşturur.
    pub fn yeni(numara: u32, baslangic: Zaman, bitis: Zaman, metin: &str) -> Self {
        Self {
            numara,
            baslangic,
            bitis,
            metin: metin.to_string(),
            stil: None,
            ad: None,
            kenar_sol: 0,
            kenar_sag: 0,
            kenar_ust: 0,
            efekt: String::new(),
            katman: 0,
            tur: OlayTuru::Diyalog,
        }
    }

    /// Repliğin metnini satırlara böler; boş satırlar korunur.
    pub fn satirlar(&self) -> Vec<&str> {
        if self.metin.is_empty() {
            Vec::new()
        } else {
            self.metin.split('\n').collect()
        }
    }

    /// Metindeki toplam karakter sayısı (satır sonları sayılmaz).
    pub fn karakter_sayisi(&self) -> usize {
        self.metin.chars().filter(|c| *c != '\n').count()
    }

    /// Satır sayısı.
    pub fn satir_sayisi(&self) -> usize {
        if self.metin.is_empty() {
            0
        } else {
            self.metin.split('\n').count()
        }
    }

    /// En uzun satırın karakter sayısı.
    pub fn en_uzun_satir(&self) -> usize {
        self.satirlar()
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0)
    }

    /// Bitiş ile başlangıç arasındaki süre (milisaniye); negatif olabilir.
    ///
    /// `Zaman::fark` soldan sağa farkı verir: `baslangic.fark(bitis) = bitis - baslangic`.
    pub fn sure_ms(&self) -> i64 {
        self.baslangic.fark(self.bitis)
    }

    /// Saniyedeki karakter sayısı; süre sıfır veya negatifse `0.0` döner.
    pub fn okuma_hizi(&self) -> f64 {
        let ms = self.sure_ms();
        if ms <= 0 {
            return 0.0;
        }
        self.karakter_sayisi() as f64 / (ms as f64 / 1000.0)
    }

    /// Zamanlama tutarlı mı (bitiş başlangıçtan önce değil mi).
    pub fn zamanlama_gecerli(&self) -> bool {
        self.bitis.deger() >= self.baslangic.deger()
    }
}

/// ASS stil tanımı; alan adı-değeri çiftleri saklanır.
///
/// Alanlar ham metin olarak saklanır çünkü (a) `[V4 Styles]`, `[V4 Styles+]`
/// ve `[V4+ Styles]` üç farklı alan listesi kullanır ve (b) bilinmeyen alanlar
/// yazılırken kaybolmamalıdır. Tipik alanlara erişim `alan()` yardımcısıyla
/// yapılır.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Stil {
    /// Stilin adı (birincil anahtar).
    pub ad: String,
    /// `(alan adı, değer)` çiftleri; biçim tanımındaki sırayı korur.
    pub alanlar: Vec<(String, String)>,
}

impl Stil {
    /// Adı ve alan listesiyle stil oluşturur.
    pub fn yeni(ad: &str, alanlar: Vec<(String, String)>) -> Self {
        Self {
            ad: ad.to_string(),
            alanlar,
        }
    }

    /// Stil alanının değerini döndürür (bulunamazsa boş metin).
    pub fn alan(&self, ad: &str) -> &str {
        self.alanlar
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(ad))
            .map_or("", |(_, v)| v.as_str())
    }

    /// Stil alanını değiştirir; alan yoksa sona ekler.
    pub fn alan_ata(&mut self, ad: &str, deger: &str) {
        if let Some(slot) = self
            .alanlar
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(ad))
        {
            slot.1 = deger.to_string();
        } else {
            self.alanlar.push((ad.to_string(), deger.to_string()));
        }
    }

    /// Stili yalnızca okunabilir biçimde alan listesiyle birlikte döndürür.
    pub fn alanlarla(&self) -> &[(String, String)] {
        &self.alanlar
    }
}

/// Bir replik numarası + metin bloğudur; ayrıştırıcıların ortak hata kaydı.
///
/// Fikir raporu §05: "sözdizimi hatasında dosya numarası ve satır bildirilir,
/// bozuk satır işaretlenir ve dosya yine açılır". Bu kayıt o "işaretlemenin"
/// taşıyıcısıdır; ayrıştırıcı hiçbir zaman hata döndürmez, bu listeyi doldurur.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SozdizimiKaydi {
    /// Hatanın ait olduğu biçim adı.
    pub bicim: String,
    /// Hatanın bulunduğu satır numarası (1 tabanlı).
    pub satir: usize,
    /// Hatanın kullanıcıya gösterilecek açıklaması.
    pub mesaj: String,
}

impl SozdizimiKaydi {
    /// Yeni sözdizimi hatası kaydı oluşturur.
    pub fn yeni(bicim: &str, satir: usize, mesaj: impl Into<String>) -> Self {
        Self {
            bicim: bicim.to_string(),
            satir,
            mesaj: mesaj.into(),
        }
    }
}

/// Bir altyazı dosyasının tamamı; biçimden bağımsız temsil.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Belge {
    /// Kaynak biçim.
    pub bicim: Bicim,
    /// Replikler, dosyadaki sırayla.
    pub replikler: Vec<Replik>,
    /// ASS stilleri; SRT ve VTT için boş.
    pub stiller: Vec<Stil>,
    /// Başlık alanları: ASS `Script Info`, VTT `STYLE` bloğu, SRT için boş.
    /// Sıra korunur çünkü yazma işlemi alanı yeniden sıralamamalıdır.
    pub basliklar: Vec<(String, String)>,
    /// Stil bölümünün `Format` satırındaki alan adları.
    pub stil_format: Vec<String>,
    /// Olay bölümünün `Format` satırındaki alan adları.
    pub olay_format: Vec<String>,
    /// Okuma sırasında yapılan kodlama denetimi.
    pub kodlama: KodlamaRaporu,
    /// Dosyada bulunan ama stil tablosunda tanımsız olan stil adları.
    pub tanimsiz_stiller: Vec<String>,
    /// Satır sonu stili: dosya `\n` mi `\r\n` mi kullanıyordu.
    pub crlf: bool,
    /// Okuma sırasında karşılaşılan sözdizimi hataları; dosya yine de açılır.
    pub sozdizimi: Vec<SozdizimiKaydi>,
}

impl Belge {
    /// Yeni, boş bir belge oluşturur (belirli bir biçimde).
    pub fn bos(bicim: Bicim) -> Self {
        Self {
            bicim,
            replikler: Vec::new(),
            stiller: Vec::new(),
            basliklar: Vec::new(),
            stil_format: Vec::new(),
            olay_format: Vec::new(),
            kodlama: KodlamaRaporu::default(),
            tanimsiz_stiller: Vec::new(),
            crlf: false,
            sozdizimi: Vec::new(),
        }
    }

    /// Toplam replik sayısı.
    pub fn replik_sayisi(&self) -> usize {
        self.replikler.len()
    }

    /// Toplam satır sayısı (tüm repliklerin satırlarının toplamı).
    pub fn toplam_satir(&self) -> usize {
        self.replikler.iter().map(Replik::satir_sayisi).sum()
    }

    /// En son repliğin bitiş zamanı.
    pub fn son_zaman(&self) -> Zaman {
        self.replikler
            .iter()
            .map(|r| r.bitis)
            .max()
            .unwrap_or(Zaman::SIFIR)
    }

    /// Belgede tanımlı bir stil adı var mı.
    pub fn stil_tanimli(&self, ad: &str) -> bool {
        self.stiller.iter().any(|s| s.ad.eq_ignore_ascii_case(ad))
    }

    /// Repliklerin başlangıç sırasına göre sıralanmış hâli (kararlı sıralama).
    pub fn sirali(&self) -> Vec<Replik> {
        let mut kopya: Vec<(usize, Replik)> = self.replikler.iter().cloned().enumerate().collect();
        kopya.sort_by(|(ia, a), (ib, b)| a.baslangic.cmp(&b.baslangic).then(ia.cmp(ib)));
        kopya.into_iter().map(|(_, r)| r).collect()
    }
}
