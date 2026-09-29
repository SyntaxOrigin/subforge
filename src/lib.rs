//! SubForge çekirdek kitaplığı.
//!
//! Sorumluluğu: SRT, ASS/SSA ve WebVTT altyazılarını **metin düzeyinde** okumak,
//! yazmak, zamanlamayı toplu olarak kaydırmak ve altı güvenilirlik kuralıyla
//! denetlemek. Bu modül bilinçli olarak **bilmez**: video gömme (burn-in),
//! kapsülleme (softsub), stil önizleme ve arayüz. Bunlar `MANIFEST.md` kartında
//! "Ertelenen" listesindedir ve harici C kütüphaneleri (libass, FFmpeg) gerektirir.
//!
//! Katman ayrımı (fikir raporu §06): kural motoru biçim ayrıntılarını bilmez;
//! biçim katmanı kural motorunu bilmez. Bu ayrım, aynı kural kümesinin üç
//! biçimde de aynı çalışmasını sağlar ve kural testlerini modelden bağımsız kılar.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::unwrap_used, clippy::expect_used)]

pub mod ass;
pub mod bicim;
pub mod cli;
pub mod hata;
pub mod islem;
pub mod kodlama;
pub mod kural;
pub mod model;
pub mod rapor;
pub mod srt;
pub mod vtt;
pub mod zaman;

pub use bicim::{bicim_tespit, metne_cevir_kayipli, oku, yaz, Kayip};
pub use hata::SubForgeHata;
pub use islem::{donustur, kaydir, normalize, KaydirmaRaporu, NormalizeRaporu};
pub use kodlama::{Kodlama, KodlamaRaporu};
pub use kural::{denetle, Bulgu, Kural, KuralEsikleri, Seviye};
pub use model::{Belge, Bicim, OlayTuru, Replik, SozdizimiKaydi, Stil};
pub use zaman::{KareHazi, Zaman};

/// Araç sürümü; `clap` komut tanımında ve rapor çıktısında kullanılır.
pub const SURUM: &str = env!("CARGO_PKG_VERSION");
