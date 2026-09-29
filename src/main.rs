//! SubForge komut satırı giriş noktası.
//!
//! Bu dosya yalnızca argümanları ayrıştırıp `subforge::cli::calistir`
//! fonksiyonuna devreder; tüm iş mantığı `lib.rs` içindeki çekirdektedir.
//! Böylece birim ve entegrasyon testleri ikiliyi derlemeden çalışır.

#![forbid(unsafe_code)]

use clap::Parser;
use subforge::cli::{calistir, KomutSatiri};

fn main() {
    let komut = KomutSatiri::parse();
    std::process::exit(calistir(komut));
}
