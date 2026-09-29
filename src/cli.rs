//! Komut satırı arayüzü: `clap` türetmeleri ve komutların yürütülmesi.
//!
//! Altı komut vardır: `check` (denetim), `report` (özet istatistik), `shift`
//! (toplu zaman kaydırma), `normalize` (kare hızı + sıralama) ve `convert`
//! (biçim dönüştürme). Her komut `--json` ile makine okuyan çıktı üretir.

use crate::hata::SubForgeHata;
use crate::islem::{kaydir, kaydir_yuzde, normalize};
use crate::kural::{denetle, KuralEsikleri};
use crate::model::Bicim;
use crate::rapor::{
    donusturme_terminal, json_cikti, kaydirma_terminal, normalize_terminal, sure_metni,
    DenetimRaporu, DonusturmeCiktisi, KaydirmaCiktisi, NormalizeCiktisi,
};
use crate::zaman::KareHazi;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

/// SubForge — SRT/ASS/WebVTT altyazı denetleyici ve toplu zaman kaydırıcı.
#[derive(Debug, Parser)]
#[command(
    name = "subforge",
    version,
    about = "SRT, ASS ve WebVTT altyazilarini okur, guvenilirlik kurallariyla denetler, toplu zaman kaydirir ve bicim donusturur.",
    long_about = "SubForge, altyazi dosyalarini bayt duzeyinde okur, alti guvenilirlik kuraliyla \
denetler ve kullaniciya acikca isteyerek zaman kaydirma, normalizasyon ve bicim donusturme yapar. \
Denetim hicbir dosyayi degistirmez."
)]
pub struct KomutSatiri {
    /// Çalıştırılacak alt komut.
    #[command(subcommand)]
    pub komut: AltKomut,
}

/// Alt komutlar.
#[derive(Debug, Subcommand)]
pub enum AltKomut {
    /// Altı güvenilirlik kuralını çalıştırır; dosyayı değiştirmez.
    Check(CheckArgumanlari),
    /// Dosyanın istatistik özetini üretir; dosyayı değiştirmez.
    Report(CheckArgumanlari),
    /// Tüm replikleri sabit miktar kaydırır.
    Shift(ShiftArgumanlari),
    /// Zaman damgalarını kare sınırına oturtur, sıralar ve numaralandırır.
    Normalize(NormalizeArgumanlari),
    /// Altyazıyı başka bir biçime çevirir ve kayıp raporu üretir.
    Convert(ConvertArgumanlari),
}

/// `check` ve `report` komutlarının ortak argümanları.
#[derive(Debug, Args, Clone)]
pub struct CheckArgumanlari {
    /// Denetlenecek altyazı dosyası.
    #[arg(value_name = "DOSYA")]
    pub dosya: PathBuf,

    /// Biçimi dosya içeriğinden tespit etmek yerine zorla.
    #[arg(long = "format", value_name = "BICIM")]
    pub bicim: Option<BicimSecimi>,

    /// Kural eşiklerini JSON dosyasından yükle.
    #[arg(long = "rules", value_name = "DOSYA")]
    pub kurallar: Option<PathBuf>,

    /// Çıktıyı JSON olarak üret (varsayılan: terminal tablosu).
    #[arg(long)]
    pub json: bool,

    /// Denetim bulguları olsa bile çıkış kodunu 0 yap.
    #[arg(long = "tolerant")]
    pub toleransli: bool,
}

/// Komut satırından gelen biçim seçimi.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BicimSecimi {
    /// SubRip.
    Srt,
    /// Advanced SubStation Alpha / SubStation Alpha.
    Ass,
    /// Web Video Text Tracks.
    Vtt,
}

impl From<BicimSecimi> for Bicim {
    fn from(deger: BicimSecimi) -> Self {
        match deger {
            BicimSecimi::Srt => Bicim::Srt,
            BicimSecimi::Ass => Bicim::Ass,
            BicimSecimi::Vtt => Bicim::Vtt,
        }
    }
}

/// `shift` komutunun argümanları.
#[derive(Debug, Args)]
pub struct ShiftArgumanlari {
    /// Kaydırılacak altyazı dosyası.
    #[arg(value_name = "DOSYA")]
    pub dosya: PathBuf,

    /// Yazılacak çıktı dosyası.
    #[arg(long = "out", value_name = "DOSYA")]
    pub cikti: PathBuf,

    /// Kaydırma miktarı (milisaniye; negatif değer geriye kaydırır).
    #[arg(long, value_name = "MS")]
    pub ms: Option<i64>,

    /// Kaydırma miktarı (saniye; negatif değer geriye kaydırır).
    ///
    /// Negatif değerler `=` ile yazılmalıdır: `--seconds=-1.4`.
    #[arg(long = "seconds", value_name = "SANIYE", allow_hyphen_values = true)]
    pub saniye: Option<f64>,

    /// Toplam sürenin yüzdesi kadar kaydır (ör. `--percent=-10`).
    ///
    /// Negatif değerler `=` ile yazılmalıdır: `--percent=-10`. Ayrı `--percent -10`
    /// biçimi `clap` tarafından "bilinmeyen bayrak" olarak yorumlanır.
    #[arg(long = "percent", value_name = "YUZDE", allow_hyphen_values = true)]
    pub yuzde: Option<f64>,

    /// Biçimi dosya içeriğinden tespit etmek yerine zorla.
    #[arg(long = "format", value_name = "BICIM")]
    pub bicim: Option<BicimSecimi>,

    /// Kaydırma sonrası kare sınırına oturt.
    #[arg(long, value_name = "FPS")]
    pub fps: Option<f64>,

    /// Kaydırma sonrası replikleri başlangıç zamanına göre sırala.
    #[arg(long = "sort")]
    pub sirala: bool,

    /// Çıktıyı JSON olarak üret.
    #[arg(long)]
    pub json: bool,
}

/// `normalize` komutunun argümanları.
#[derive(Debug, Args)]
pub struct NormalizeArgumanlari {
    /// Normalize edilecek altyazı dosyası.
    #[arg(value_name = "DOSYA")]
    pub dosya: PathBuf,

    /// Yazılacak çıktı dosyası.
    #[arg(long = "out", value_name = "DOSYA")]
    pub cikti: PathBuf,

    /// Kare hızı; `23.976`, `25`, `29.97` gibi ondalık değerler kabul edilir.
    #[arg(long, value_name = "FPS")]
    pub fps: Option<f64>,

    /// Replikleri başlangıç zamanına göre sırala.
    #[arg(long = "sort")]
    pub sirala: bool,

    /// Replik numaralarını 1'den başlayarak yeniden yaz.
    #[arg(long = "number")]
    pub numarala: bool,

    /// Biçimi dosya içeriğinden tespit etmek yerine zorla.
    #[arg(long = "format", value_name = "BICIM")]
    pub bicim: Option<BicimSecimi>,

    /// Çıktıyı JSON olarak üret.
    #[arg(long)]
    pub json: bool,
}

/// `convert` komutunun argümanları.
#[derive(Debug, Args)]
pub struct ConvertArgumanlari {
    /// Dönüştürülecek altyazı dosyası.
    #[arg(value_name = "DOSYA")]
    pub dosya: PathBuf,

    /// Yazılacak çıktı dosyası.
    #[arg(long = "out", value_name = "DOSYA")]
    pub cikti: PathBuf,

    /// Hedef biçim.
    #[arg(long = "to", value_name = "BICIM")]
    pub hedef: BicimSecimi,

    /// Kaynak biçimi dosya içeriğinden tespit etmek yerine zorla.
    #[arg(long = "format", value_name = "BICIM")]
    pub bicim: Option<BicimSecimi>,

    /// Çıktıyı JSON olarak üret.
    #[arg(long)]
    pub json: bool,
}

/// Komut satırını çalıştırır ve süreç çıkış kodunu döndürür.
///
/// Hata durumunda hata metni `stderr`e yazılır ve sıfırdan farklı kod döner;
/// `panic!` üretilmez.
pub fn calistir(komut: KomutSatiri) -> i32 {
    match komut.komut {
        AltKomut::Check(a) => denetle_ve_yaz(&a, true),
        AltKomut::Report(a) => denetle_ve_yaz(&a, false),
        AltKomut::Shift(a) => shift_calistir(&a),
        AltKomut::Normalize(a) => normalize_calistir(&a),
        AltKomut::Convert(a) => convert_calistir(&a),
    }
}

/// `check`/`report` komutlarının ortak gövdesi.
fn denetle_ve_yaz(a: &CheckArgumanlari, ayrinti: bool) -> i32 {
    match denetim_raporu_uret(a) {
        Ok(rapor) => {
            let yazma = if a.json {
                json_cikti(&rapor).map(|metin| println!("{metin}"))
            } else if ayrinti {
                print!("{}", rapor.terminal());
                Ok(())
            } else {
                print!("{}", ozet_terminal(&rapor));
                Ok(())
            };
            if let Err(hata) = yazma {
                return hata_yaz(&hata);
            }
            if ayrinti && !a.toleransli && !rapor.gecti() {
                3
            } else {
                0
            }
        }
        Err(hata) => hata_yaz(&hata),
    }
}

/// Denetim raporunu üretir; `check` ve `report` aynı veriyi kullanır.
fn denetim_raporu_uret(a: &CheckArgumanlari) -> Result<DenetimRaporu, SubForgeHata> {
    let esikler = esikleri_yukle(a.kurallar.as_deref())?;
    let belge = belge_oku(&a.dosya, a.bicim.map(Bicim::from))?;
    let bulgular = denetle(&belge, &esikler);
    let yol = a.dosya.display().to_string();
    Ok(DenetimRaporu::kur(&yol, &belge, &esikler, bulgular))
}

/// `report` komutunun kısa özet çıktısı.
fn ozet_terminal(rapor: &DenetimRaporu) -> String {
    let mut s = String::new();
    s.push_str("=== subforge report ===\n");
    s.push_str(&format!("dosya   : {}\n", rapor.dosya));
    s.push_str(&format!("bicim   : {}\n", rapor.bicim));
    s.push_str(&format!("replik  : {}\n", rapor.replik_sayisi));
    s.push_str(&format!("satir   : {}\n", rapor.satir_sayisi));
    s.push_str(&format!("sure    : {}\n", sure_metni(rapor.toplam_sure_ms)));
    s.push_str(&format!("kodlama : {}\n", rapor.kodlama.kodlama));
    s.push_str(&format!("bulgu   : {}\n", rapor.bulgu_sayisi));
    s.push_str(&format!("kritik  : {}\n", rapor.kritik_sayisi));
    s
}

/// `shift` komutunu çalıştırır.
fn shift_calistir(a: &ShiftArgumanlari) -> i32 {
    match shift_uygula(a) {
        Ok(cikti) => {
            if a.json {
                match json_cikti(&cikti) {
                    Ok(metin) => println!("{metin}"),
                    Err(hata) => {
                        hata_yaz(&hata);
                        return 1;
                    }
                }
            } else {
                print!("{}", kaydirma_terminal(&cikti));
            }
            0
        }
        Err(hata) => hata_yaz(&hata),
    }
}

fn shift_uygula(a: &ShiftArgumanlari) -> Result<KaydirmaCiktisi, SubForgeHata> {
    let girdi = kaydirma_hesapla(a)?;
    let mut belge = belge_oku(&a.dosya, a.bicim.map(Bicim::from))?;
    let rapor = match girdi {
        KaydirmaGirdisi::SabitMs(ms) => kaydir(&mut belge, ms),
        KaydirmaGirdisi::Yuzde(yuzde) => kaydir_yuzde(&mut belge, yuzde),
    };
    if let Some(hiz) = a.fps.map(KareHazi::ayri).transpose()? {
        normalize(&mut belge, Some(hiz), a.sirala, false);
    } else if a.sirala {
        normalize(&mut belge, None, true, false);
    }
    let kalan = crate::islem::cakisan_ciftler(&belge).len();
    let cikti_yol = a.cikti.clone();
    let bicim = belge.bicim;
    crate::bicim::yaz(&belge, bicim, &cikti_yol)?;
    Ok(KaydirmaCiktisi {
        surum: crate::SURUM,
        dosya: a.dosya.display().to_string(),
        cikti: cikti_yol.display().to_string(),
        kaydirma: rapor,
        kalan_cakisma: kalan,
    })
}

/// Kaydırma komutunun kabul ettiği üç kayıt biçiminden biri.
#[derive(Debug, Clone, Copy, PartialEq)]
enum KaydirmaGirdisi {
    /// Mutlak milisaniye (`--ms`, `--saniye`).
    SabitMs(i64),
    /// Toplam sürenin yüzdesi (`--yuzde`).
    Yuzde(f64),
}

/// Kaydırma miktarını argümanlardan hesaplar; çakışan seçenekleri reddeder.
fn kaydirma_hesapla(a: &ShiftArgumanlari) -> Result<KaydirmaGirdisi, SubForgeHata> {
    let secenek_sayisi =
        u8::from(a.ms.is_some()) + u8::from(a.saniye.is_some()) + u8::from(a.yuzde.is_some());
    if secenek_sayisi == 0 {
        return Err(SubForgeHata::gecersiz_arguman(
            "bir kaydirma miktari secilmeli: --ms, --saniye veya --yuzde",
        ));
    }
    if secenek_sayisi > 1 {
        return Err(SubForgeHata::gecersiz_arguman(
            "--ms, --saniye ve --yuzde birlikte kullanilamaz",
        ));
    }
    if let Some(ms) = a.ms {
        return Ok(KaydirmaGirdisi::SabitMs(ms));
    }
    if let Some(saniye) = a.saniye {
        if !saniye.is_finite() {
            return Err(SubForgeHata::gecersiz_arguman(
                "saniye degeri sonlu (finite) olmali",
            ));
        }
        let ms = (saniye * 1000.0).round();
        if !(i64::MIN as f64..=i64::MAX as f64).contains(&ms) {
            return Err(SubForgeHata::gecersiz_arguman(
                "saniye degeri i64 araligini asiyor",
            ));
        }
        return Ok(KaydirmaGirdisi::SabitMs(ms as i64));
    }
    let yuzde = a.yuzde.unwrap_or(0.0);
    if !yuzde.is_finite() {
        return Err(SubForgeHata::gecersiz_arguman(
            "yuzde degeri sonlu (finite) olmali",
        ));
    }
    Ok(KaydirmaGirdisi::Yuzde(yuzde))
}

/// `normalize` komutunu çalıştırır.
fn normalize_calistir(a: &NormalizeArgumanlari) -> i32 {
    match normalize_uygula(a) {
        Ok(cikti) => {
            if a.json {
                match json_cikti(&cikti) {
                    Ok(metin) => println!("{metin}"),
                    Err(hata) => {
                        hata_yaz(&hata);
                        return 1;
                    }
                }
            } else {
                print!("{}", normalize_terminal(&cikti));
            }
            0
        }
        Err(hata) => hata_yaz(&hata),
    }
}

fn normalize_uygula(a: &NormalizeArgumanlari) -> Result<NormalizeCiktisi, SubForgeHata> {
    let hiz = a.fps.map(KareHazi::ayri).transpose()?;
    let mut belge = belge_oku(&a.dosya, a.bicim.map(Bicim::from))?;
    let islem = normalize(&mut belge, hiz, a.sirala, a.numarala);
    let cikti_yol = a.cikti.clone();
    let bicim = belge.bicim;
    crate::bicim::yaz(&belge, bicim, &cikti_yol)?;
    Ok(NormalizeCiktisi {
        surum: crate::SURUM,
        dosya: a.dosya.display().to_string(),
        cikti: cikti_yol.display().to_string(),
        kare_hizi: hiz.map(|h| h.to_string()),
        islem,
    })
}

/// `convert` komutunu çalıştırır.
fn convert_calistir(a: &ConvertArgumanlari) -> i32 {
    match convert_uygula(a) {
        Ok(cikti) => {
            if a.json {
                match json_cikti(&cikti) {
                    Ok(metin) => println!("{metin}"),
                    Err(hata) => {
                        hata_yaz(&hata);
                        return 1;
                    }
                }
            } else {
                print!("{}", donusturme_terminal(&cikti));
            }
            0
        }
        Err(hata) => hata_yaz(&hata),
    }
}

fn convert_uygula(a: &ConvertArgumanlari) -> Result<DonusturmeCiktisi, SubForgeHata> {
    let belge = belge_oku(&a.dosya, a.bicim.map(Bicim::from))?;
    let kaynak_bicim = belge.bicim;
    let hedef = Bicim::from(a.hedef);
    let (yeni, donusturme) = crate::islem::donustur(&belge, hedef);
    let cikti_yol = a.cikti.clone();
    crate::bicim::yaz(&yeni, hedef, &cikti_yol)?;
    Ok(DonusturmeCiktisi {
        surum: crate::SURUM,
        dosya: a.dosya.display().to_string(),
        cikti: cikti_yol.display().to_string(),
        kaynak_bicim: kaynak_bicim.ad().to_string(),
        donusturme,
    })
}

/// Dosyayı okur; `zorla` verilmişse biçimi içerikten değil, argümandan alır.
fn belge_oku(yol: &Path, zorla: Option<Bicim>) -> Result<crate::model::Belge, SubForgeHata> {
    let belge = crate::bicim::oku(yol)?;
    match zorla {
        Some(hedef) if hedef != belge.bicim => {
            // İçerik tespiti başka bir biçim gösteriyorsa kullanıcı haklı olabilir
            // (ör. SRT içeriği .txt uzantısıyla saklanmış olabilir); bu durumda
            // metin yeniden ayrıştırılır.
            let bayt = std::fs::read(yol).map_err(|e| SubForgeHata::dosya_okunamadi(yol, e))?;
            let (metin, kodlama) = crate::kodlama::coz(&bayt);
            crate::bicim::sozden_oku(hedef, &metin, &kodlama)
        }
        _ => Ok(belge),
    }
}

/// Kural eşiklerini dosyadan yükler; dosya verilmemişse varsayılanları kullanır.
fn esikleri_yukle(yol: Option<&Path>) -> Result<KuralEsikleri, SubForgeHata> {
    let Some(dosya) = yol else {
        return Ok(KuralEsikleri::default());
    };
    let metin =
        std::fs::read_to_string(dosya).map_err(|e| SubForgeHata::dosya_okunamadi(dosya, e))?;
    serde_json::from_str(&metin).map_err(|e| {
        SubForgeHata::gecersiz_arguman(format!(
            "'{}' kural profili okunamadi: {e}",
            dosya.display()
        ))
    })
}

/// Hata metnini `stderr`e yazar ve çıkış kodunu döndürür.
fn hata_yaz(hata: &SubForgeHata) -> i32 {
    eprintln!("hata: {hata}");
    hata.exit_code()
}
