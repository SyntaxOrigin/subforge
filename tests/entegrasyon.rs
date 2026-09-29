//! Uçtan uca entegrasyon testleri: dosya sistemi üzerinden aç → denetle →
//! dönüştür → kaydet akışı.
//!
//! Bu dosya WORKER_CONTRACT.md §5.3'te tarif edilen kendi geçici dizin
//! yardımcısını içerir. `tempfile` crate'i bağımlılık politikasıyla yasaktır
//! (WORKER_CONTRACT.md §3.2), bu yüzden yardımcı kendi kodumuzla yazılmıştır.

use std::path::{Path, PathBuf};

use subforge::kodlama::KodlamaRaporu;
use subforge::kural::{denetle, KuralEsikleri};
use subforge::model::Bicim;
use subforge::rapor::DenetimRaporu;
use subforge::{bicim, islem};

/// Test içinde geçici dosya/dizin üreten, `Drop` ile temizleyen kapsayıcı.
///
/// Neden `tempfile` yok: bağımlılık politikası (WORKER_CONTRACT §3.2)
/// `tempfile`'i hiçbir projede vermez; yardımcı kendi kodumuzla yazılır.
pub struct GeciciDizin {
    yol: PathBuf,
}

impl GeciciDizin {
    /// `std::env::temp_dir()` altında etiketten türetilmiş benzersiz dizin açar.
    pub fn yeni(etiket: &str) -> std::io::Result<Self> {
        let kok = std::env::temp_dir().join(format!("subforge-{etiket}-{}", std::process::id()));
        // Aynı test iki kez çalışırsa eski içerik temizlenir.
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok)?;
        Ok(Self { yol: kok })
    }

    /// Dizin içine göreli yol döndürür.
    pub fn yol(&self) -> &Path {
        &self.yol
    }

    /// Dizin içine yazılmış bir dosyanın tam yolunu döndürür.
    pub fn dosya(&self, ad: &str) -> PathBuf {
        self.yol.join(ad)
    }
}

impl Drop for GeciciDizin {
    fn drop(&mut self) {
        // Temizlik başarısız olsa da testi düşürmemeli; `let _ =` bilinçlidir ve
        // WORKER_CONTRACT §5.3 tarafından bu yol için açıkça kabul edilir:
        // `Drop` içinden hata döndürülemez, hata yutulur.
        let _ = std::fs::remove_dir_all(&self.yol);
    }
}

/// Dizin içine metin dosyası yazar.
fn yaz(dizin: &GeciciDizin, ad: &str, icerik: &str) -> PathBuf {
    let yol = dizin.dosya(ad);
    std::fs::write(&yol, icerik).expect("ornek dosya yazilir");
    yol
}

/// Denetlenmiş bir SRT örneği.
const SRT_ORNEK: &str = "1\n00:00:01,000 --> 00:00:02,000\nMerhaba dunya\n\n2\n00:00:01,500 --> 00:00:04,000\nIkinci replik cok uzun bir metin iceriyor ve okunabilir satir sinirini asiyor\n\n3\n00:00:10,000 --> 00:00:09,000\nTers sureli replik\n\n";

/// UTF-8 BOM'lu SRT örneği.
const SRT_BOMLU: &str = "\u{feff}1\n00:00:01,000 --> 00:00:02,000\nBOMlu dosya\n\n";

/// Dosya okuma → biçim tespiti → denetim zinciri tamamlanır.
#[test]
fn dosya_okunup_denetilir() {
    let d = GeciciDizin::yeni("okuma").expect("gecici dizin");
    let yol = yaz(&d, "ornek.srt", SRT_ORNEK);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    assert_eq!(belge.bicim, Bicim::Srt);
    assert_eq!(belge.replik_sayisi(), 3);
    let bulgular = denetle(&belge, &KuralEsikleri::default());
    assert!(!bulgular.is_empty());
    // Çakışan ilk iki replik bulunmalı.
    assert!(bulgular
        .iter()
        .any(|b| b.kural == subforge::Kural::CakisanAraliklar));
    // Negatif süreli üçüncü replik bulunmalı.
    assert!(bulgular
        .iter()
        .any(|b| b.kural == subforge::Kural::SiraBozuklugu));
}

/// BOM'lu dosya açılır ve kodlama kuralı bulgu üretir.
#[test]
fn bomlu_dosya_acilir_kodlama_kurali_tetiklenir() {
    let d = GeciciDizin::yeni("bom").expect("gecici dizin");
    let yol = yaz(&d, "bomlu.srt", SRT_BOMLU);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    assert_eq!(belge.replik_sayisi(), 1);
    assert!(belge.kodlama.bom_var);
    let bulgular = denetle(&belge, &KuralEsikleri::default());
    assert!(bulgular
        .iter()
        .any(|b| b.kural == subforge::Kural::KodlamaBozuklugu));
}

/// UTF-16 LE dosya açılır (BOM tanınır, metin çözülür).
#[test]
fn utf16_dosya_acilir() {
    let d = GeciciDizin::yeni("utf16").expect("gecici dizin");
    let mut bayt = vec![0xFF, 0xFE];
    for b in SRT_ORNEK.encode_utf16() {
        bayt.extend_from_slice(&b.to_le_bytes());
    }
    let yol = d.dosya("utf16.srt");
    std::fs::write(&yol, &bayt).expect("utf16 dosya yazilir");
    let belge = bicim::oku(&yol).expect("dosya okunur");
    assert_eq!(belge.replik_sayisi(), 3);
    assert_eq!(belge.kodlama.kodlama, "utf-16-le");
}

/// Windows-1254 (tek bayt) dosya açılır ve kodlama bozukluğu bildirilir.
#[test]
fn cp1254_dosya_acilir_kodlama_bildirilir() {
    let d = GeciciDizin::yeni("cp1254").expect("gecici dizin");
    let mut bayt: Vec<u8> = Vec::new();
    bayt.extend_from_slice(b"1\n00:00:01,000 --> 00:00:02,000\n");
    // Windows-1254: 0xDD = 'İ', 0xFD = 'ı', 0xFE = 'ş'
    bayt.extend_from_slice(&[
        b'T', 0xDD, b'r', b'k', 0xFD, b'e', b' ', b'y', b'a', b'z', 0xFE, b'\n', b'\n',
    ]);
    let yol = d.dosya("cp1254.srt");
    std::fs::write(&yol, &bayt).expect("cp1254 dosya yazilir");
    let belge = bicim::oku(&yol).expect("dosya okunur");
    assert_eq!(belge.kodlama.kodlama, "windows-1254");
    assert!(belge.replikler[0].metin.contains('ı'));
    assert!(belge.replikler[0].metin.contains('İ'));
    let bulgular = denetle(&belge, &KuralEsikleri::default());
    assert!(bulgular
        .iter()
        .any(|b| b.kural == subforge::Kural::KodlamaBozuklugu));
}

/// Boş dosya açılır, hata vermez, biçim tanımsızdır.
#[test]
fn bos_dosya_hata_verir() {
    let d = GeciciDizin::yeni("bos").expect("gecici dizin");
    let yol = yaz(&d, "bos.srt", "");
    assert!(bicim::oku(&yol).is_err());
}

/// Tanımsız içerikli dosya biçim hatası verir.
#[test]
fn tanimsiz_icerik_bicim_hatasi_verir() {
    let d = GeciciDizin::yeni("tanimsiz").expect("gecici dizin");
    let yol = yaz(&d, "notlar.txt", "bu bir altyazi dosyasi degil\n");
    let hata = bicim::oku(&yol).expect_err("hata beklenir");
    assert!(matches!(
        hata,
        subforge::SubForgeHata::BicimTanimliDegil { .. }
    ));
}

/// Var olmayan dosya okuma hatası verir.
#[test]
fn olmayan_dosya_okuma_hatasi_verir() {
    let d = GeciciDizin::yeni("yok").expect("gecici dizin");
    let yol = d.dosya("boyle_bir_dosya_yok.srt");
    let hata = bicim::oku(&yol).expect_err("hata beklenir");
    assert!(matches!(
        hata,
        subforge::SubForgeHata::DosyaOkunamadi { .. }
    ));
}

/// Kaydırma dosya üzerinde uygulanır ve kaynak dosya korunur.
#[test]
fn kaydirma_dosya_uzerinde_uygulanir() {
    let d = GeciciDizin::yeni("kaydir").expect("gecici dizin");
    let kaynak = yaz(&d, "kaynak.srt", SRT_ORNEK);
    let mut belge = bicim::oku(&kaynak).expect("kaynak okunur");
    islem::kaydir(&mut belge, 2000);
    let cikti = d.dosya("cikti.srt");
    bicim::yaz(&belge, Bicim::Srt, &cikti).expect("cikti yazilir");

    // Kaynak dosya değişmedi.
    let kaynak_tekrar = bicim::oku(&kaynak).expect("kaynak tekrar okunur");
    assert_eq!(kaynak_tekrar.replikler[0].baslangic.deger(), 1000);

    // Çıktı dosyası kaydırılmış.
    let cikti_belge = bicim::oku(&cikti).expect("cikti okunur");
    assert_eq!(cikti_belge.replikler[0].baslangic.deger(), 3000);
    assert_eq!(cikti_belge.replik_sayisi(), 3);
}

/// Normalizasyon dosya üzerinde uygulanır.
#[test]
fn normalize_dosya_uzerinde_uygulanir() {
    let d = GeciciDizin::yeni("normalize").expect("gecici dizin");
    let kaynak = yaz(&d, "kaynak.srt", SRT_ORNEK);
    let mut belge = bicim::oku(&kaynak).expect("kaynak okunur");
    islem::normalize(&mut belge, Some(subforge::KareHazi::sabit(25)), true, true);
    let cikti = d.dosya("cikti.srt");
    bicim::yaz(&belge, Bicim::Srt, &cikti).expect("cikti yazilir");
    let cikti_belge = bicim::oku(&cikti).expect("cikti okunur");
    // 25 fps'te her zaman 40 ms'in katı olmalı.
    for r in &cikti_belge.replikler {
        assert_eq!(r.baslangic.deger() % 40, 0, "kare siniri disinda: {r:?}");
    }
    // Sıralama ve numaralandırma uygulanmış olmalı.
    let numaralar: Vec<u32> = cikti_belge.replikler.iter().map(|r| r.numara).collect();
    assert_eq!(numaralar, vec![1, 2, 3]);
}

/// Dönüştürme dosya üzerinde uygulanır ve kayıp raporu üretilir.
#[test]
fn convert_dosya_uzerinde_uygulanir() {
    let d = GeciciDizin::yeni("convert").expect("gecici dizin");
    let kaynak = yaz(&d, "kaynak.srt", SRT_ORNEK);
    let belge = bicim::oku(&kaynak).expect("kaynak okunur");
    let (yeni, rapor) = islem::donustur(&belge, Bicim::Ass);
    let cikti = d.dosya("cikti.ass");
    bicim::yaz(&yeni, Bicim::Ass, &cikti).expect("cikti yazilir");

    // Çıktı tekrar okunabilir.
    let cikti_belge = bicim::oku(&cikti).expect("cikti okunur");
    assert_eq!(cikti_belge.bicim, Bicim::Ass);
    assert_eq!(cikti_belge.replik_sayisi(), 3);
    // Kayıp raporu boş değil.
    assert!(!rapor.kayiplar.is_empty());
    assert_eq!(rapor.replik_sayisi, 3);
}

/// Üç biçim arasında dosya üzerinden gidiş-dönüş yapılabilir.
#[test]
fn uc_bicim_dosya_uzerinde_gidis_donus_yapar() {
    let d = GeciciDizin::yeni("ucbicim").expect("gecici dizin");
    let kaynak = yaz(&d, "kaynak.srt", SRT_ORNEK);
    let srt_belge = bicim::oku(&kaynak).expect("srt okunur");

    let (ass_belge, _) = islem::donustur(&srt_belge, Bicim::Ass);
    let ass_yol = d.dosya("ara.ass");
    bicim::yaz(&ass_belge, Bicim::Ass, &ass_yol).expect("ass yazilir");

    let (vtt_belge, _) = islem::donustur(&ass_belge, Bicim::Vtt);
    let vtt_yol = d.dosya("ara.vtt");
    bicim::yaz(&vtt_belge, Bicim::Vtt, &vtt_yol).expect("vtt yazilir");

    let vtt_geri = bicim::oku(&vtt_yol).expect("vtt geri okunur");
    let (srt_son, _) = islem::donustur(&vtt_geri, Bicim::Srt);
    let srt_son_yol = d.dosya("son.srt");
    bicim::yaz(&srt_son, Bicim::Srt, &srt_son_yol).expect("son srt yazilir");

    let son = bicim::oku(&srt_son_yol).expect("son srt okunur");
    assert_eq!(son.bicim, Bicim::Srt);
    assert_eq!(son.replik_sayisi(), 3);
    // Metin korunur.
    assert_eq!(son.replikler[0].metin, "Merhaba dunya");
    // Zamanlama korunur (ASS yüzde bölümü nedeniyle ±10 ms kayma olabilir).
    let fark = (son.replikler[0].baslangic.deger() - 1000).abs();
    assert!(fark <= 10, "zaman kaymasi cok buyuk: {fark} ms");
}

/// Denetim raporu JSON olarak serileştirilebilir ve içerik taşır.
#[test]
fn denetim_raporu_json_uretir() {
    let d = GeciciDizin::yeni("rapor").expect("gecici dizin");
    let yol = yaz(&d, "ornek.srt", SRT_ORNEK);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    let esik = KuralEsikleri::default();
    let bulgular = denetle(&belge, &esik);
    let yol_metni = yol.display().to_string();
    let rapor = DenetimRaporu::kur(&yol_metni, &belge, &esik, bulgular);
    let json = rapor.json();
    assert!(json.contains("\"bulgu_sayisi\""));
    assert!(json.contains("\"okuma_hizi\""));
    assert!(json.contains("\"surum\""));
    // Kritik bulgu var çünkü çakışma ve negatif süre var.
    assert!(!rapor.gecti());
}

/// Temiz belgede denetim geçer.
#[test]
fn temiz_belgede_denetim_gecer() {
    let temiz = "1\n00:00:01,000 --> 00:00:03,000\nMerhaba dunya\n\n";
    let d = GeciciDizin::yeni("temiz").expect("gecici dizin");
    let yol = yaz(&d, "temiz.srt", temiz);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    let esik = KuralEsikleri::default();
    let bulgular = denetle(&belge, &esik);
    let yol_metni = yol.display().to_string();
    let rapor = DenetimRaporu::kur(&yol_metni, &belge, &esik, bulgular);
    assert!(rapor.gecti());
    assert_eq!(rapor.kritik_sayisi, 0);
}

/// Terminal çıktısı üretilir ve sonuç satırı içerir.
#[test]
fn terminal_raporu_uretir() {
    let d = GeciciDizin::yeni("terminal").expect("gecici dizin");
    let yol = yaz(&d, "ornek.srt", SRT_ORNEK);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    let esik = KuralEsikleri::default();
    let bulgular = denetle(&belge, &esik);
    let yol_metni = yol.display().to_string();
    let rapor = DenetimRaporu::kur(&yol_metni, &belge, &esik, bulgular);
    let metin = rapor.terminal();
    assert!(metin.contains("subforge check"));
    assert!(metin.contains("sonuc:"));
    assert!(metin.contains("kural             adet"));
}

/// Özel eşik profili dosyadan yüklenir ve uygulanır.
#[test]
fn ozel_esik_profili_dosyadan_uygulanir() {
    let d = GeciciDizin::yeni("esik").expect("gecici dizin");
    let profil = r#"{
  "okuma_hizi": 30.0,
  "asgari_dinlenme_ms": 500,
  "asgari_hiz": 12.0,
  "en_fazla_karakter": 60,
  "en_fazla_satir": 3,
  "kabul_edilebilir_bosluk_ms": 2000
}"#;
    let profil_yol = yaz(&d, "profil.json", profil);
    let metin = std::fs::read_to_string(&profil_yol).expect("profil okunur");
    let esik: KuralEsikleri = serde_json::from_str(&metin).expect("profil ayristirilir");
    assert_eq!(esik.okuma_hizi, 30.0);
    assert_eq!(esik.asgari_dinlenme_ms, 500);
    assert_eq!(esik.en_fazla_karakter, 60);
    assert_eq!(esik.kabul_edilebilir_bosluk_ms, 2000);

    // Aynı belge varsayılan ve özel profille farklı sonuç verir: örnekteki
    // 1000 ms'lik çakışma varsayılan profilde hata, özel profilde kabul edilir.
    let yol = yaz(&d, "ornek.srt", SRT_ORNEK);
    let belge = bicim::oku(&yol).expect("dosya okunur");
    let siki = denetle(&belge, &KuralEsikleri::default());
    let gevsek = denetle(&belge, &esik);
    let cakisma = |bulgular: &[subforge::Bulgu]| {
        bulgular
            .iter()
            .filter(|b| b.kural == subforge::Kural::CakisanAraliklar)
            .count()
    };
    assert_eq!(cakisma(&siki), 1);
    assert_eq!(cakisma(&gevsek), 0);
}

/// Bozuk profille yükleme hata verir.
#[test]
fn bozuk_esik_profili_hata_verir() {
    let d = GeciciDizin::yeni("bozukesik").expect("gecici dizin");
    let yol = yaz(&d, "bozuk.json", "{ okuma_hizi: 17 }");
    let metin = std::fs::read_to_string(&yol).expect("profil okunur");
    assert!(serde_json::from_str::<KuralEsikleri>(&metin).is_err());
}

/// Eksik alanlar varsayılan değerlerle doldurulur (serde `default`).
#[test]
fn eksik_esik_alanlari_varsayilan_olur() {
    let esik: KuralEsikleri =
        serde_json::from_str(r#"{ "okuma_hizi": 20.0 }"#).expect("eksik alanlar varsayilan dolar");
    assert_eq!(esik.okuma_hizi, 20.0);
    assert_eq!(esik.asgari_dinlenme_ms, 1000);
    assert_eq!(esik.en_fazla_karakter, 42);
    assert_eq!(esik.en_fazla_satir, 2);
}

/// Varsayılan kodlama raporu sağlıklıdır.
#[test]
fn varsayilan_kodlama_raporu_sagliklidir() {
    let rapor = KodlamaRaporu::default();
    assert!(rapor.saglikli());
    assert!(rapor.notlar.is_empty());
}
