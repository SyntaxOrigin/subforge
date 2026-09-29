//! Zaman modeli ve karakter kodlaması için entegrasyon testleri.
//!
//! Kapsam: zaman damgası ayrıştırma/yazma (SRT/ASS/VTT), kare hızı kesir
//! dönüşümü (23.976 -> 24) ve kodlama tespiti (UTF-8, BOM, UTF-16, karışık).

use subforge::kodlama::coz;
use subforge::zaman::{KareHazi, Zaman};

/// SRT zaman damgası (virgül ayracı) doğru ayrıştırılır.
#[test]
fn srt_zaman_dolgusal_ayraci_ayiklanir() {
    let z = Zaman::ayikla("00:01:02,345").expect("gecerli zaman");
    assert_eq!(z.deger(), 62_345);
}

/// SRT zaman damgası (nokta ayracı) da kabul edilir.
#[test]
fn srt_zaman_nokta_ayraci_ayiklanir() {
    let z = Zaman::ayikla("00:00:01.500").expect("gecerli zaman");
    assert_eq!(z.deger(), 1500);
}

/// ASS biçimi yüzde bölümlüdür: `0:00:01.23` = 1230 ms.
#[test]
fn ass_zaman_yuzde_ayiklanir() {
    let z = Zaman::ayikla("0:00:01.23").expect("gecerli zaman");
    assert_eq!(z.deger(), 1230);
}

/// Tek hane kesir `.5` = 500 ms olarak ölçeklenir.
#[test]
fn tek_hane_kesir_yarim_saniye_verir() {
    let z = Zaman::ayikla("00:00:00.5").expect("gecerli zaman");
    assert_eq!(z.deger(), 500);
}

/// WebVTT'nin kısa biçimi `MM:SS.mmm` kabul edilir.
#[test]
fn vtt_kisa_bicim_ayiklanir() {
    let z = Zaman::ayikla("01:02.500").expect("gecerli zaman");
    assert_eq!(z.deger(), 62_500);
}

/// WebVTT uzun biçimi `HH:MM:SS.mmm` kabul edilir.
#[test]
fn vtt_uzun_bicim_ayiklanir() {
    let z = Zaman::ayikla("00:01:02.500").expect("gecerli zaman");
    assert_eq!(z.deger(), 62_500);
}

/// SRT yazımı `HH:MM:SS,mmm` biçimindedir.
#[test]
fn srt_yazim_bicimi_dogru() {
    assert_eq!(Zaman::milis(62_345).srt_yaz(), "00:01:02,345");
}

/// ASS yazımı `H:MM:SS.cc` biçimindedir.
#[test]
fn ass_yazim_bicimi_dogru() {
    assert_eq!(Zaman::milis(1230).ass_yaz(), "0:00:01.23");
}

/// ASS yazımında 995 ms ve üzeri bir saniyeye taşır (taşma olmaz).
#[test]
fn ass_yazim_yuzde_tasma_korumasi() {
    assert_eq!(Zaman::milis(995).ass_yaz(), "0:00:01.00");
    assert_eq!(Zaman::milis(994).ass_yaz(), "0:00:00.99");
}

/// WebVTT saat sıfırken kısa biçimi (`MM:SS.mmm`) üretir.
#[test]
fn vtt_yazim_saat_sifirken_kisa_bicim() {
    assert_eq!(Zaman::milis(62_500).vtt_yaz(), "01:02.500");
    assert_eq!(Zaman::milis(62_500 + 3_600_000).vtt_yaz(), "01:01:02.500");
}

/// Ayrıştırma ve yazma birbirinin tersidir (gidiş-dönüş).
#[test]
fn zaman_gidis_donus_testi() {
    for ms in [0i64, 1, 999, 1000, 59_999, 60_000, 3_723_456] {
        let z = Zaman::milis(ms);
        assert_eq!(
            Zaman::ayikla(&z.srt_yaz()).ok().map(|g| g.deger()),
            Some(ms)
        );
    }
}

/// Bozuk zaman damgası hata döndürür, sessizce 0 olmaz.
#[test]
fn bozuk_zaman_dammgasi_hata_verir() {
    assert!(Zaman::ayikla("aa:bb:cc").is_err());
    assert!(Zaman::ayikla("00:00:60,000").is_err());
    assert!(Zaman::ayikla("00:00:00,00x").is_err());
    assert!(Zaman::ayikla("").is_err());
    assert!(Zaman::ayikla("00:00:00:00").is_err());
}

/// Sadece `SS.mmm` biçimi (saat/dakika olmadan) reddedilir.
#[test]
fn saat_eksik_bicim_reddedilir() {
    assert!(Zaman::ayikla("01.500").is_err());
}

/// 23.976 kare/saniye NTSC kesri olarak tanınır.
#[test]
fn kare_hizi_23976_ntsc_kesrine_cevrilir() {
    let hiz = KareHazi::ayri(23.976).expect("gecerli kare hizi");
    assert_eq!(hiz.pay(), 24_000);
    assert_eq!(hiz.payda(), KareHazi::NTS_PAYDA);
}

/// 29.97 kare/saniye de NTSC kesridir.
#[test]
fn kare_hizi_2997_ntsc_kesrine_cevrilir() {
    let hiz = KareHazi::ayri(29.97).expect("gecerli kare hizi");
    assert_eq!(hiz.pay(), 30_000);
    assert_eq!(hiz.payda(), KareHazi::NTS_PAYDA);
}

/// 25 kare/saniye tam sayı kesridir ve sadeleştirilmiş biçimde saklanır.
#[test]
fn kare_hazi_25_tam_sayi() {
    let hiz = KareHazi::ayri(25.0).expect("gecerli kare hizi");
    assert_eq!(hiz.pay(), 25);
    assert_eq!(hiz.payda(), 1);
    assert_eq!(hiz.deger(), 25.0);
}

/// 23.976 -> 24 dönüşümünde kare sınırına oturtma doğru çalışır.
#[test]
fn kare_hizi_donusu_23976_den_24_e() {
    let kaynak = KareHazi::ayri(23.976).expect("kaynak");
    let hedef = KareHazi::sabit(24);
    // 1000 ms 23.976'da 23.976 kareye denk gelir -> 24 kare sınırı 1000 ms.
    assert_eq!(kaynak.kare_no(1000), 24);
    assert_eq!(hedef.kare_no(1000), 24);
    // 1 saniye 24 karede tam sınırdadır; yuvarlama onu bozmaz.
    assert_eq!(Zaman::milis(1000).kareye_yuvarla(hedef).deger(), 1000);
    // 1042 ms -> 23.976'da 24.987 kare -> 25 kare -> 1043 ms
    let oturtulmus = Zaman::milis(1042).kareye_yuvarla(kaynak);
    assert_eq!(oturtulmus.deger(), 1043);
    // 25.042 ms -> 23.976'da 600.2 kare -> 600 kare -> 25.025 ms
    assert_eq!(Zaman::milis(25_042).kareye_yuvarla(kaynak).deger(), 25_025);
}

/// Kare hızı formülü fikir raporundaki örneği birebir uygular.
///
/// Rapor §07: `3,280 sn x 30 = 98,4 -> kare 98 -> 98/30 = 3,2667 sn -> 00:00:03,267`.
#[test]
fn kare_hizi_rapor_ornegini_uygular() {
    let hiz = KareHazi::sabit(30);
    let zaman = Zaman::milis(3280).kareye_yuvarla(hiz);
    assert_eq!(hiz.kare_no(3280), 98);
    assert_eq!(zaman.deger(), 3267);
}

/// Geçersiz kare hızı reddedilir.
#[test]
fn gecersiz_kare_hizi_reddedilir() {
    assert!(KareHazi::ayri(0.0).is_err());
    assert!(KareHazi::ayri(-25.0).is_err());
    assert!(KareHazi::ayri(f64::NAN).is_err());
    assert!(KareHazi::ayri(5000.0).is_err());
}

/// Temiz UTF-8 dosya BOM'suz olarak tanınır.
#[test]
fn temiz_utf8_taninir() {
    let (metin, rapor) = coz("Merhaba dünya\n".as_bytes());
    assert_eq!(metin, "Merhaba dünya\n");
    assert_eq!(rapor.kodlama, "utf-8");
    assert!(!rapor.bom_var);
    assert!(rapor.saglikli());
}

/// UTF-8 BOM tanınır, gövdeden ayrılır ve bulgu üretir.
#[test]
fn utf8_bom_taninir_ve_soyulur() {
    let mut bayt = vec![0xEF, 0xBB, 0xBF];
    bayt.extend_from_slice("WEBVTT\n".as_bytes());
    let (metin, rapor) = coz(&bayt);
    assert!(metin.starts_with("WEBVTT"));
    assert!(rapor.bom_var);
    assert_eq!(rapor.kodlama, "utf-8-bom");
    assert!(!rapor.saglikli());
}

/// UTF-16 LE BOM tanınır ve metne çevrilir.
#[test]
fn utf16_le_taninir() {
    let mut bayt = vec![0xFF, 0xFE];
    for b in "1\n00:00:01,000 --> 00:00:02,000\nMerhaba".encode_utf16() {
        bayt.extend_from_slice(&b.to_le_bytes());
    }
    let (metin, rapor) = coz(&bayt);
    assert!(metin.contains("Merhaba"));
    assert_eq!(rapor.kodlama, "utf-16-le");
    assert!(rapor.bom_var);
    assert_eq!(rapor.bozuk_bayt_sayisi, 0);
}

/// UTF-16 BE BOM tanınır ve metne çevrilir.
#[test]
fn utf16_be_taninir() {
    let mut bayt = vec![0xFE, 0xFF];
    for b in "Salam".encode_utf16() {
        bayt.extend_from_slice(&b.to_be_bytes());
    }
    let (metin, rapor) = coz(&bayt);
    assert_eq!(metin, "Salam");
    assert_eq!(rapor.kodlama, "utf-16-be");
    assert!(rapor.bom_var);
}

/// Windows-1254 (Türkçe tek bayt) çözülür ve bozuk bayt bildirilir.
#[test]
fn cp1254_cozulur() {
    // 0xFD = 'ı' (noktasız i) Windows-1254'te
    let bayt = [b'M', b'e', b'r', b'h', b'a', b'b', b'a', 0xFD];
    let (metin, rapor) = coz(&bayt);
    assert_eq!(metin, "Merhabaı");
    assert_eq!(rapor.kodlama, "windows-1254");
    assert!(!rapor.karisik_kodlama);
    assert!(!rapor.saglikli());
}

/// Karışık kodlama (UTF-8 + tek bayt satırlar) satır düzeyinde yakalanır.
///
/// Karışıklık için **her iki türden de** satır bulunmalıdır: ASCII satırlar
/// her iki kodlamada da geçerli olduğu için kanıt sayılmaz.
#[test]
fn karisik_kodlama_tespit_edilir() {
    let mut bayt = Vec::new();
    bayt.extend_from_slice("1\n00:00:01,000 --> 00:00:02,000\n".as_bytes());
    bayt.extend_from_slice("Merhaba dünya\n".as_bytes()); // UTF-8 'ü'
    bayt.push(0xFD); // tek bayt 'ı'
    bayt.extend_from_slice(b"\n");
    let (metin, rapor) = coz(&bayt);
    assert!(metin.contains('ı'));
    assert!(rapor.karisik_kodlama);
    assert_eq!(rapor.kodlama, "karisik");
    assert_eq!(rapor.tek_bayt_satir_sayisi, 1);
    assert!(!rapor.saglikli());
}

/// Saf ASCII dosya karışık sayılmaz (ASCII her iki kodlamada da geçerlidir).
#[test]
fn saf_ascii_karisik_sayilmaz() {
    let (_metin, rapor) = coz(b"1\n00:00:01,000 --> 00:00:02,000\nMerhaba\n");
    assert!(!rapor.karisik_kodlama);
    assert!(rapor.saglikli());
}

/// Bozuk UTF-8 bayt dizileri sayılır (dosya reddedilmez).
#[test]
fn bozuk_bayt_dizisi_sayilir() {
    let bayt = [b'a', 0xFF, 0xFE, b'b', 0xC3, 0x28, b'c'];
    let (_metin, rapor) = coz(&bayt);
    assert!(rapor.bozuk_bayt_sayisi > 0);
    assert!(!rapor.saglikli());
}

/// Boş bayt dizisi hata vermez, boş metin ve temiz rapor döner.
#[test]
fn bos_bayt_dizisi_islenir() {
    let (metin, rapor) = coz(&[]);
    assert!(metin.is_empty());
    assert_eq!(rapor.kodlama, "utf-8");
    assert!(rapor.saglikli());
}
