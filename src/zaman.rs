//! Zaman modeli: milisaniye tabanlı zaman damgası, biçim bazlı ayrıştırma/yazma
//! ve kare hızı uyumu.
//!
//! Fikir raporu §07'deki formül birebir uygulanır:
//! `kareNo = yuvarla(zaman × kareHizi / 1000)`, `zaman = kareNo / kareHizi × 1000`.
//! Kayan nokta birikmesini önlemek için kare hızı **kesir** olarak saklanır
//! (23.976 → 24000/1001), böylece NTSC oranları da tam sayı aritmetiğiyle yuvarlanır.

use crate::hata::SubForgeHata;
use std::fmt;

/// Altyazı zaman damgası; milisaniye çözünürlüğünde, başlangıç noktasına göre.
///
/// Negatif değerler yalnızca kaydırma sırasında "aşağıya taşındı" bilgisini
/// taşımak için oluşur; hiçbir biçim yazıcısı negatif değeri üretmez.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Zaman(pub i64);

impl Zaman {
    /// Sıfır zaman damgası.
    pub const SIFIR: Zaman = Zaman(0);

    /// Milisaniye değerinden zaman damgası üretir.
    pub fn milis(deger: i64) -> Self {
        Self(deger)
    }

    /// Ham milisaniye değerini döndürür.
    pub fn deger(&self) -> i64 {
        self.0
    }

    /// Zamanı tam saniye + kalan milisaniye olarak ayırır.
    pub fn parcalara(&self) -> (i64, i64) {
        let t = self.0.max(0);
        (t / 1000, t % 1000)
    }

    /// Toplam süreyi saniye cinsinden `f64` olarak verir (kural ölçümleri için).
    pub fn saniye(&self) -> f64 {
        self.0 as f64 / 1000.0
    }

    /// İki zaman damgası arasındaki farkı milisaniye olarak verir.
    pub fn fark(&self, diger: Zaman) -> i64 {
        diger.0 - self.0
    }

    /// Zamanı baştan sona yazdırır: `HH:MM:SS,mmm` (SRT biçimi).
    pub fn srt_yaz(&self) -> String {
        let (saat, dk, sn, ms) = self.parcalar();
        format!("{saat:02}:{dk:02}:{sn:02},{ms:03}")
    }

    /// Zamanı ASS/SSA biçiminde yazdırır: `H:MM:SS.cc` (saniyede yüzde).
    pub fn ass_yaz(&self) -> String {
        let t = self.0.max(0);
        let mut tam = t / 1000;
        // ASS yüzde bölümü iki hanedir; 995 ms ve üzeri 100 cs'ye yuvarlanır ve
        // bu değer bir saniyeye taşınır (taşma yanlış zaman damgası üretir).
        let mut cs = (t % 1000 + 5) / 10;
        if cs >= 100 {
            cs = 0;
            tam += 1;
        }
        let (saat, dk, sn) = (tam / 3600, (tam / 60) % 60, tam % 60);
        format!("{saat}:{dk:02}:{sn:02}.{cs:02}")
    }

    /// Zamanı WebVTT biçiminde yazdırır: saat varsa `HH:MM:SS.mmm`,
    /// saat sıfırsa kısa biçim `MM:SS.mmm` üretilir.
    pub fn vtt_yaz(&self) -> String {
        let (saat, dk, sn, ms) = self.parcalar();
        if saat == 0 {
            format!("{dk:02}:{sn:02}.{ms:03}")
        } else {
            format!("{saat:02}:{dk:02}:{sn:02}.{ms:03}")
        }
    }

    /// Saat / dakika / saniye / milisaniye dörtlüsünü üretir.
    fn parcalar(&self) -> (i64, i64, i64, i64) {
        let (tam, ms) = self.parcalara();
        (tam / 3600, (tam / 60) % 60, tam % 60, ms)
    }

    /// Zamanı en yakın kare sınırına oturtur.
    ///
    /// Rapor §07 formülü: `kareNo = yuvarla(zaman × kareHizi / 1000)`, ardından
    /// `zaman = kareNo / kareHizi × 1000`. İki adım da gereklidir; tek adımda
    /// yalnızca "kare süresine yakınlaştırma" yapılır, kare **numarası**
    /// korunmaz.
    pub fn kareye_yuvarla(&self, hiz: KareHazi) -> Zaman {
        Zaman(hiz.zaman_ms(hiz.kare_no(self.0)))
    }

    /// Metindeki zaman damgasını ayrıştırır; biçimin kendi kuralı uygulanmaz,
    /// yalnızca sözdizimi denetlenir.
    ///
    /// `00:00:01,000`, `00:00:01.000`, `0:00:01.23` ve `01:02.500` kabul edilir;
    /// ondalık ayracı hem nokta hem virgül olabilir, kesir hane sayısı ne olursa
    /// olsun milisaniyeye ölçeklenir.
    pub fn ayikla(girdi: &str) -> Result<Zaman, String> {
        let temiz = girdi.trim();
        if temiz.is_empty() {
            return Err("zaman damgasi bos".to_string());
        }
        let parcalar: Vec<&str> = temiz.split(':').collect();
        if parcalar.len() > 3 {
            return Err(format!("cok fazla iki nokta: '{girdi}'"));
        }
        let (saat_dk, saniye_kismi) = match parcalar.len() {
            3 => (&parcalar[0..2], parcalar[2]),
            2 => (&parcalar[0..1], parcalar[1]),
            _ => return Err(format!("saat:dakika:saniye eksik: '{girdi}'")),
        };
        let mut toplam_dakika: i64 = 0;
        for parca in saat_dk {
            let d: i64 = parca
                .trim()
                .parse()
                .map_err(|_| format!("sayi olmayan alan: '{parca}'"))?;
            if d < 0 {
                return Err(format!("negatif alan: '{parca}'"));
            }
            toplam_dakika = toplam_dakika
                .checked_mul(60)
                .and_then(|v| v.checked_add(d))
                .ok_or_else(|| format!("zaman tasmasi: '{girdi}'"))?;
        }
        let (sn_metni, kesir_metni) = match saniye_kismi.split_once(['.', ',']) {
            Some((a, b)) => (a, Some(b)),
            None => (saniye_kismi, None),
        };
        let sn: i64 = sn_metni
            .trim()
            .parse()
            .map_err(|_| format!("sayi olmayan saniye: '{sn_metni}'"))?;
        if !(0..60).contains(&sn) {
            return Err(format!("saniye 0-59 araliginda olmali: '{sn}'"));
        }
        let mut ms: i64 = 0;
        if let Some(kesir) = kesir_metni {
            if !kesir.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("kesir sayi degil: '{kesir}'"));
            }
            // Kesir hane sayisi biçime göre değişir (SRT 3, ASS 2). En fazla
            // üç anlamlı hane alınır, eksik haneler 100 çarpanıyla tamamlanır:
            // `.5` -> 500, `.23` -> 230, `.000` -> 0.
            for (indeks, hane) in kesir.chars().take(3).enumerate() {
                ms += i64::from(hane.to_digit(10).unwrap_or(0)) * 10i64.pow(2 - indeks as u32);
            }
        }
        let toplam_sn = toplam_dakika
            .checked_mul(60)
            .and_then(|v| v.checked_add(sn))
            .ok_or_else(|| format!("zaman tasmasi: '{girdi}'"))?;
        let toplam = toplam_sn
            .checked_mul(1000)
            .and_then(|v| v.checked_add(ms))
            .ok_or_else(|| format!("zaman tasmasi: '{girdi}'"))?;
        Ok(Zaman(toplam))
    }
}

impl fmt::Display for Zaman {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.srt_yaz())
    }
}

/// Kesirli kare hızı; `pay / payda` biçiminde saklanır.
///
/// NTSC oranları (`24000/1001`, `30000/1001`, `60000/1001`) kesir olarak
/// tutulur, böylece kare sınırına oturtma işlemi tam sayı aritmetiğiyle ve
/// yuvarlama hatası birikmeden yapılır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KareHazi {
    pay: i64,
    payda: i64,
}

impl KareHazi {
    /// NTSC oranlarının paydası.
    pub const NTS_PAYDA: i64 = 1001;

    /// Tam sayı kare hızından kesirli hız üretir (`25` → `25/1`).
    pub fn sabit(fps: i64) -> Self {
        Self { pay: fps, payda: 1 }
    }

    /// Ondalık kare hızını NTSC veya PAL uyumlu kesre çevirir.
    ///
    /// `23.976` → `24000/1001`, `25` → `25/1`, `29.97` → `30000/1001`,
    /// `23.5` → `47/2`, `60` → `60/1`.
    pub fn ayri(oran: f64) -> Result<Self, SubForgeHata> {
        if !oran.is_finite() || !(1.0..=1000.0).contains(&oran) {
            return Err(SubForgeHata::gecersiz_arguman(format!(
                "kare hizi 1-1000 araliginda olmali, verilen: {oran}"
            )));
        }
        // Once NTSC adayini dene: 23.976 * 1001 / 1000 ~= 24
        let ntsc_temel = (oran * Self::NTS_PAYDA as f64 / 1000.0).round();
        if ntsc_temel >= 1.0 {
            let ntsc_hedef = ntsc_temel * 1000.0 / Self::NTS_PAYDA as f64;
            if (oran - ntsc_hedef).abs() < 0.002 {
                return Ok(Self::kucult(ntsc_temel as i64 * 1000, Self::NTS_PAYDA));
            }
        }
        let yuzde = (oran * 100.0).round();
        if (oran * 100.0 - yuzde).abs() < 1e-6 {
            return Ok(Self::kucult(yuzde as i64, 100));
        }
        Ok(Self {
            pay: oran.round() as i64,
            payda: 1,
        })
    }

    /// Kesri sadeleştirir: `2500/100` -> `25/1`, `24000/1001` -> `24000/1001`.
    ///
    /// Sadeleştirme kare hesabını değiştirmez ama `pay`/`payda` değerlerinin
    /// okunabilir ve test edilebilir olmasını sağlar.
    fn kucult(pay: i64, payda: i64) -> Self {
        if payda == 0 {
            return Self { pay: 1, payda: 1 };
        }
        let bolen = ebob(pay, payda);
        Self {
            pay: pay / bolen,
            payda: payda / bolen,
        }
    }

    /// Karesel pay değeri.
    pub fn pay(&self) -> i64 {
        self.pay
    }

    /// Karesel payda değeri.
    pub fn payda(&self) -> i64 {
        self.payda
    }

    /// Kare hızını ondalık olarak verir.
    pub fn deger(&self) -> f64 {
        self.pay as f64 / self.payda as f64
    }

    /// Verilen milisaniyeye karşılık gelen (yuvarlanmış) kare numarası.
    pub fn kare_no(&self, ms: i64) -> i64 {
        let payda = i128::from(self.pay);
        let bolen = 1000 * i128::from(self.payda);
        let sayi = i128::from(ms) * payda;
        yuvarla_bol(sayi, bolen)
    }

    /// Verilen kare numarasına karşılık gelen (yuvarlanmış) milisaniye.
    pub fn zaman_ms(&self, kare: i64) -> i64 {
        let bolen = i128::from(self.pay);
        let sayi = i128::from(kare) * 1000 * i128::from(self.payda);
        yuvarla_bol(sayi, bolen)
    }
}

impl fmt::Display for KareHazi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let deger = self.deger();
        if (deger - deger.round()).abs() < 1e-9 {
            write!(f, "{}", deger.round() as i64)
        } else {
            write!(f, "{deger:.3}")
        }
    }
}

/// `i128` bölmesinde sıfırdan yuvarlama (yarım yukarı) uygular.
///
/// Sonuç `i64` aralığına taşarsa doygun (saturating) davranılır; bu yol
/// normalde yalnızca çok büyük kare numaralarında oluşur.
fn yuvarla_bol(sayi: i128, bolen: i128) -> i64 {
    debug_assert!(bolen > 0);
    let bolum = sayi / bolen;
    let kalan = sayi % bolen;
    let sonuc = if kalan * 2 >= bolen { bolum + 1 } else { bolum };
    if sonuc > i128::from(i64::MAX) {
        i64::MAX
    } else if sonuc < i128::from(i64::MIN) {
        i64::MIN
    } else {
        sonuc as i64
    }
}

/// İki pozitif tam sayının en büyük ortak bölenini verir (Öklid algoritması).
fn ebob(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let kalan = a % b;
        a = b;
        b = kalan;
    }
    if a == 0 {
        1
    } else {
        a
    }
}
