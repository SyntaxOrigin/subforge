//! Bayt düzeyinde okuma, karakter kodlaması tanıma ve bozukluk raporlama.
//!
//! Neden `std::fs::read_to_string` değil: bu modülün var oluş sebebi bozuk baytı
//! **görebilmektir**. `read_to_string` geçersiz UTF-8'da hata döndürür ve bozuk
//! satırın nerede olduğunu söylemez; SubForge ise dosyayı yine de açmalı, bozuk
//! yeri işaretlemeli ve kullanıcıya "bu dosya UTF-8 değil" demelidir (fikir raporu
//! §05 biçim okuma satırı: hatalı dosya açılır ama bozuk satır işaretlenir).
//!
//! Tanınan kodlamalar: UTF-8 (BOM'lu ve BOM'suz), UTF-16 LE, UTF-16 BE ve
//! Windows-1254 (Türkçe tek bayt). Karışık kodlama, yani dosyanın bir kısmının
//! geçerli UTF-8 bir kısmının tek bayt olması, satır düzeyinde ayrıştırılarak
//! yakalanır.

/// Tanınan karakter kodlaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kodlama {
    /// BOM'suz UTF-8.
    Utf8,
    /// UTF-8 Bayt Sırası İşareti ile başlayan UTF-8.
    Utf8Bom,
    /// UTF-16 Little Endian (BOM'lu).
    Utf16Le,
    /// UTF-16 Big Endian (BOM'lu).
    Utf16Be,
    /// Windows-1254; Türkçe için seçilen tek bayt kodlama.
    Cp1254,
}

impl Kodlama {
    /// Kodlamanın kısa adını döndürür.
    pub fn ad(&self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf8Bom => "utf-8-bom",
            Self::Utf16Le => "utf-16-le",
            Self::Utf16Be => "utf-16-be",
            Self::Cp1254 => "windows-1254",
        }
    }
}

/// Kodlama denetiminin sonucu; kurallardan altıncısı (kodlama bozukluğu) ve
/// dönüşüm kayıp raporu bu veriden beslenir.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct KodlamaRaporu {
    /// Tanınan kodlama.
    pub kodlama: String,
    /// Dosyanın başında Bayt Sırası İşareti var mıydı.
    pub bom_var: bool,
    /// UTF-8 olarak çözülemeyen bayt dizilerinin adedi.
    pub bozuk_bayt_sayisi: usize,
    /// Çözülmüş metinde bulunan `U+FFFD` değiştirme işareti adedi.
    pub kontrol_isareti_sayisi: usize,
    /// Dosyada geçerli UTF-8 satır ile geçersiz satır birlikte bulunuyor mu.
    pub karisik_kodlama: bool,
    /// Dosyada geçerli UTF-8 olmayan satır adedi.
    pub tek_bayt_satir_sayisi: usize,
    /// İnsan tarafından okunabilir açıklamalar.
    pub notlar: Vec<String>,
}

impl KodlamaRaporu {
    /// Denetimde herhangi bir kodlama sorunu var mı.
    pub fn saglikli(&self) -> bool {
        !self.bom_var
            && self.bozuk_bayt_sayisi == 0
            && self.kontrol_isareti_sayisi == 0
            && !self.karisik_kodlama
    }
}

/// Dosya baytlarını çözüp `(metin, rapor)` döndürür.
///
/// İçerik hiçbir koşulda reddedilmez: çözülemeyen baytlar `U+FFFD` olarak
/// geçer ve `KodlamaRaporu` bunları sayar. Böylece bozuk dosya da denetlenebilir.
pub fn coz(bayt: &[u8]) -> (String, KodlamaRaporu) {
    let mut rapor = KodlamaRaporu::default();
    let govde = bom_ayir(bayt, &mut rapor);

    if rapor.kodlama == Kodlama::Utf16Le.ad() || rapor.kodlama == Kodlama::Utf16Be.ad() {
        let (metin, bozuk) = utf16_coz(govde, rapor.kodlama == Kodlama::Utf16Be.ad());
        rapor.bozuk_bayt_sayisi = bozuk;
        rapor.kontrol_isareti_sayisi = metin.matches('\u{FFFD}').count();
        return (metin, rapor);
    }

    match std::str::from_utf8(govde) {
        Ok(metin) => {
            if rapor.kodlama.is_empty() {
                rapor.kodlama = Kodlama::Utf8.ad().to_string();
            }
            rapor.kontrol_isareti_sayisi = metin.matches('\u{FFFD}').count();
            if rapor.kontrol_isareti_sayisi > 0 {
                rapor.notlar.push(
                    "kaynak metinde U+FFFD degistirme isareti var; kodlama kaybi olmus".to_string(),
                );
            }
            (metin.to_string(), rapor)
        }
        Err(_hata) => {
            rapor.bozuk_bayt_sayisi = bozuk_dizi_say(govde);
            let (gecerli_satir, tek_bayt_satir) = satir_kodlama_analizi(govde);
            rapor.tek_bayt_satir_sayisi = tek_bayt_satir;
            rapor.karisik_kodlama = gecerli_satir > 0 && tek_bayt_satir > 0;
            rapor.kodlama = if rapor.karisik_kodlama {
                "karisik".to_string()
            } else {
                Kodlama::Cp1254.ad().to_string()
            };
            rapor.kontrol_isareti_sayisi = cp1254_bozuk_nokta(govde);
            let not = if rapor.karisik_kodlama {
                format!(
                    "dosya karisik kodlama: {gecerli_satir} satir UTF-8, {tek_bayt_satir} satir tek bayt"
                )
            } else {
                format!(
                    "dosya UTF-8 degil; windows-1254 (Turkce tek bayt) olarak cozuldu, {} bayt dizisi bozuk",
                    rapor.bozuk_bayt_sayisi
                )
            };
            rapor.notlar.push(not);
            (cp1254_coz(govde), rapor)
        }
    }
}

/// Baştaki Bayt Sırası İşaretini tanır ve gövdeyi döndürür.
fn bom_ayir<'a>(bayt: &'a [u8], rapor: &mut KodlamaRaporu) -> &'a [u8] {
    if bayt.starts_with(&[0xEF, 0xBB, 0xBF]) {
        rapor.kodlama = Kodlama::Utf8Bom.ad().to_string();
        rapor.bom_var = true;
        rapor
            .notlar
            .push("dosya UTF-8 BOM ile basliyor".to_string());
        return &bayt[3..];
    }
    // UTF-32 LE BOM (FF FE 00 00) UTF-16 LE'den ayrılmalıdır; SubForge
    // UTF-32 desteklemez, bu yüzden UTF-16 olarak ele alınır ve not düşülür.
    if bayt.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) {
        rapor.kodlama = Kodlama::Utf16Le.ad().to_string();
        rapor.bom_var = true;
        rapor
            .notlar
            .push("dosya UTF-32 LE BOM ile basliyor; UTF-16 olarak ele alindi".to_string());
        return &bayt[2..];
    }
    if bayt.starts_with(&[0xFF, 0xFE]) {
        rapor.kodlama = Kodlama::Utf16Le.ad().to_string();
        rapor.bom_var = true;
        rapor
            .notlar
            .push("dosya UTF-16 LE BOM ile basliyor".to_string());
        return &bayt[2..];
    }
    if bayt.starts_with(&[0xFE, 0xFF]) {
        rapor.kodlama = Kodlama::Utf16Be.ad().to_string();
        rapor.bom_var = true;
        rapor
            .notlar
            .push("dosya UTF-16 BE BOM ile basliyor".to_string());
        return &bayt[2..];
    }
    bayt
}

/// UTF-16 bayt dizisini metne çevirir; tek bayt çiftleri geçersizse
/// kaç çiftin bozuk olduğunu döndürür.
fn utf16_coz(bayt: &[u8], big_endian: bool) -> (String, usize) {
    let mut birimler: Vec<u16> = Vec::with_capacity(bayt.len() / 2);
    let mut bozuk = 0usize;
    let mut i = 0usize;
    while i + 1 < bayt.len() {
        let birim = if big_endian {
            u16::from_be_bytes([bayt[i], bayt[i + 1]])
        } else {
            u16::from_le_bytes([bayt[i], bayt[i + 1]])
        };
        if (0xD800..0xDC00).contains(&birim) {
            // Yüksek vekil: alçak vekil gelirse geçerli bir çifttir.
            if i + 3 < bayt.len() {
                let alcak = if big_endian {
                    u16::from_be_bytes([bayt[i + 2], bayt[i + 3]])
                } else {
                    u16::from_le_bytes([bayt[i + 2], bayt[i + 3]])
                };
                if (0xDC00..0xE000).contains(&alcak) {
                    birimler.push(birim);
                    birimler.push(alcak);
                    i += 4;
                    continue;
                }
            }
            bozuk += 1;
            birimler.push(0xFFFD);
            i += 2;
            continue;
        }
        if (0xDC00..0xE000).contains(&birim) {
            bozuk += 1;
            birimler.push(0xFFFD);
            i += 2;
            continue;
        }
        birimler.push(birim);
        i += 2;
    }
    if bayt.len() % 2 == 1 {
        bozuk += 1;
    }
    (String::from_utf16_lossy(&birimler), bozuk)
}

/// Windows-1254 (Türkçe tek bayt) çözümleyicisi; tanımsız kod noktaları
/// `U+FFFD` olarak çözülür ve kaynakta zaten bir bilgi kaybı işaretidir.
fn cp1254_coz(bayt: &[u8]) -> String {
    let mut metin = String::with_capacity(bayt.len());
    for &b in bayt {
        if b < 0x80 {
            metin.push(b as char);
        } else {
            metin.push(cp1254_kod_noktasi(b));
        }
    }
    metin
}

/// Windows-1254'te tanımsız olan kod noktalarının adedi.
fn cp1254_bozuk_nokta(bayt: &[u8]) -> usize {
    bayt.iter()
        .filter(|&&b| cp1254_kod_noktasi(b) == '\u{FFFD}')
        .count()
}

/// Bayt dizisindeki UTF-8'i geçersiz kılan dizi adedini sayar.
///
/// `str::from_utf8` yalnızca ilk hatalı konumu bildirir; dosyadaki tüm bozuk
/// dizileri saymak için hata zinciri `error_len()` ile adım adım tüketilir.
fn bozuk_dizi_say(bayt: &[u8]) -> usize {
    let mut sayac = 0usize;
    let mut kalan = bayt;
    loop {
        match std::str::from_utf8(kalan) {
            Ok(_) => return sayac,
            Err(hata) => {
                sayac += 1;
                match hata.error_len() {
                    Some(boy) if boy > 0 => {
                        let atla = hata.valid_up_to() + boy;
                        if atla >= kalan.len() {
                            return sayac;
                        }
                        kalan = &kalan[atla..];
                    }
                    // Kesilmiş dizi: dosyanın sonunda yarım UTF-8 dizisi var.
                    _ => return sayac,
                }
            }
        }
    }
}

/// Satır düzeyinde kodlama analizi yapar.
///
/// Dönen çift: `(geçerli UTF-8 satır sayısı, tek bayt satır sayısı)`. Yalnızca
/// ASCII satırlar sayılmaz, çünkü ASCII hem UTF-8 hem Windows-1254'te aynıdır
/// ve karışıklık göstergesi değildir. Dosyanın tamamı tek bayt ise karışık
/// sayılmaz; karışıklık için **her iki türden de** satır bulunmalıdır.
fn satir_kodlama_analizi(bayt: &[u8]) -> (usize, usize) {
    let mut gecerli = 0usize;
    let mut tek_bayt = 0usize;
    for satir in bayt.split(|&b| b == b'\n') {
        let s = match satir.strip_suffix(b"\r") {
            Some(kisa) => kisa,
            None => satir,
        };
        if s.iter().all(|&b| b < 0x80) {
            continue;
        }
        match std::str::from_utf8(s) {
            Ok(_) => gecerli += 1,
            Err(_) => tek_bayt += 1,
        }
    }
    (gecerli, tek_bayt)
}

/// Windows-1254 tek baytını Unicode kod noktasına çevirir.
///
/// Windows-1254, 0x80-0x9F aralığında ISO-8859-1'den ayrılır (Türkçe para
/// işareti, tırnak) ve 0xA0-0xFF aralığında **yedi** kod noktası daha farklıdır:
/// `Ğ İ Ş ğ ı ş Ÿ`. Bu yedi fark `UST_KISIM_UST`te tanımlıdır; geri kalan
/// 0xA0-0xFF Latin-1 ile aynıdır.
fn cp1254_kod_noktasi(b: u8) -> char {
    /// 0x80-0x9F aralığı; tanımsız kodlar `U+FFFD` üretir.
    const UST_KISIM_UST: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}',
        '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
    ];
    /// 0xA0-0xFF aralığında Latin-1'den ayrılan yedi Türkçe kod noktası:
    /// (0xD0, 0xDD, 0xDE, 0xF0, 0xFD, 0xFE, 0xFF) -> (Ğ, İ, Ş, ğ, ı, ş, Ÿ)
    const UST_KISIM_ALT: [(u8, char); 7] = [
        (0xD0, '\u{011E}'),
        (0xDD, '\u{0130}'),
        (0xDE, '\u{015E}'),
        (0xF0, '\u{011F}'),
        (0xFD, '\u{0131}'),
        (0xFE, '\u{015F}'),
        (0xFF, '\u{0178}'),
    ];
    if (0x80..=0x9F).contains(&b) {
        UST_KISIM_UST[usize::from(b - 0x80)]
    } else if let Some((_, kp)) = UST_KISIM_ALT.iter().find(|(kod, _)| *kod == b) {
        *kp
    } else {
        // 0xA0-0xFF aralığının geri kalanı Latin-1 ile aynıdır.
        char::from(b)
    }
}
