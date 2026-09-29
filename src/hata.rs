//! Hata tipleri ve bunların kullanıcıya gösterilecek metinleri.
//!
//! Fikir raporu §07 "Hata yönetimi" satırı üç ayrı sınıf ister: sözdizimi hatası,
//! biçim uyumsuzluğu ve dosya erişimi. Bu modül o üç sınıfı `SubForgeHata`
//! enum'unda ayrı varyantlar olarak korur; `thiserror` bağımlılığı yasak olduğu
//! için `Display` ve `Error` impl'leri elle yazılmıştır (WORKER_CONTRACT §4.3).

use std::fmt;
use std::io;
use std::path::Path;

/// Araçın ürettiği tüm hatalar.
///
/// Her varyant kullanıcıdan gelen bir durumu temsil eder ve `Result` ile döner;
/// hiçbir yol `panic!` üretmez.
#[derive(Debug)]
#[non_exhaustive]
pub enum SubForgeHata {
    /// Dosya okunamadı veya okuma sırasında işletim sistemi hatası verdi.
    DosyaOkunamadi {
        /// Okunmak istenen yolun görüntü metni.
        yol: String,
        /// İşletim sisteminin verdiği asıl hata.
        kaynak: io::Error,
    },
    /// Dosya okundu ama içerik geçerli bir karakter kodlamasına çözülemedi.
    KodlamaCozulemedi {
        /// Sorunlu dosyanın görüntü metni.
        yol: String,
        /// Hatanın ayrıntılı açıklaması.
        ayrinti: String,
    },
    /// İçerik geçerli bir altyazı biçimi olarak tanınmadı.
    BicimTanimliDegil {
        /// Sorunlu dosyanın görüntü metni.
        yol: String,
    },
    /// Altyazı sözdizimi hatalı; dosya yine de açılır, bozuk satır işaretlenir.
    SozdizimiHatasi {
        /// Hatanın ait olduğu biçim adı.
        bicim: &'static str,
        /// Hatanın bulunduğu satır numarası (1 tabanlı).
        satir: usize,
        /// Hatanın kullanıcıya gösterilecek açıklaması.
        mesaj: String,
    },
    /// Çıktı dosyası yazılamadı.
    CiktiYazilamadi {
        /// Yazılmak istenen yolun görüntü metni.
        yol: String,
        /// İşletim sisteminin verdiği asıl hata.
        kaynak: io::Error,
    },
    /// Komut satırından gelen argüman geçersiz (çakışan seçenek, boş değer...).
    GecersizArguman {
        /// Kullanıcıya gösterilecek açıklama.
        mesaj: String,
    },
}

impl fmt::Display for SubForgeHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DosyaOkunamadi { yol, kaynak } => {
                write!(f, "'{yol}' okunamadi: {kaynak}")
            }
            Self::KodlamaCozulemedi { yol, ayrinti } => {
                write!(f, "'{yol}' cozulemedi: {ayrinti}")
            }
            Self::BicimTanimliDegil { yol } => write!(
                f,
                "'{yol}' tanimli degil: dosya SRT, ASS/SSA veya WebVTT degil"
            ),
            Self::SozdizimiHatasi {
                bicim,
                satir,
                mesaj,
            } => write!(f, "{bicim} sozdizimi hatasi, satir {satir}: {mesaj}"),
            Self::CiktiYazilamadi { yol, kaynak } => {
                write!(f, "'{yol}' yazilamadi: {kaynak}")
            }
            Self::GecersizArguman { mesaj } => write!(f, "gecersiz arguman: {mesaj}"),
        }
    }
}

impl std::error::Error for SubForgeHata {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DosyaOkunamadi { kaynak, .. } => Some(kaynak),
            Self::CiktiYazilamadi { kaynak, .. } => Some(kaynak),
            _ => None,
        }
    }
}

impl SubForgeHata {
    /// Dosya okuma hatası üretir.
    pub fn dosya_okunamadi(yol: &Path, kaynak: io::Error) -> Self {
        Self::DosyaOkunamadi {
            yol: yol.display().to_string(),
            kaynak,
        }
    }

    /// Çıktı yazma hatası üretir.
    pub fn cikti_yazilamadi(yol: &Path, kaynak: io::Error) -> Self {
        Self::CiktiYazilamadi {
            yol: yol.display().to_string(),
            kaynak,
        }
    }

    /// Kodlama hatası üretir.
    pub fn kodlama(yol: &Path, ayrinti: impl Into<String>) -> Self {
        Self::KodlamaCozulemedi {
            yol: yol.display().to_string(),
            ayrinti: ayrinti.into(),
        }
    }

    /// Biçim tanımlama hatası üretir.
    pub fn bicim_tanimli(yol: &Path) -> Self {
        Self::BicimTanimliDegil {
            yol: yol.display().to_string(),
        }
    }

    /// Sözdizimi hatası üretir.
    pub fn sozdizimi(bicim: &'static str, satir: usize, mesaj: impl Into<String>) -> Self {
        Self::SozdizimiHatasi {
            bicim,
            satir,
            mesaj: mesaj.into(),
        }
    }

    /// Geçersiz argüman hatası üretir.
    pub fn gecersiz_arguman(mesaj: impl Into<String>) -> Self {
        Self::GecersizArguman {
            mesaj: mesaj.into(),
        }
    }

    /// Komut satırı aracının kullanacağı çıkış kodu.
    ///
    /// Sözdizimi hataları kullanıcının dosyasındadır, diğerleri çalışma ortamı
    /// hatasıdır; ikisi de `1` ile ayrılır, ayrım `--json` çıktısında mümkündür.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::SozdizimiHatasi { .. } => 2,
            _ => 1,
        }
    }
}
