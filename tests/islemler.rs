//! Zamanlama işlemleri: toplu kaydırma, kare hızı normalizasyonu, biçim dönüştürme.

use subforge::islem::{cakisan_ciftler, donustur, kaydir, kaydir_yuzde, normalize};
use subforge::model::{Belge, Bicim, Replik};
use subforge::zaman::{KareHazi, Zaman};

/// Belgeyi verilen repliklerden kurar.
fn belge(replikler: Vec<Replik>) -> Belge {
    let mut b = Belge::bos(Bicim::Srt);
    b.replikler = replikler;
    b
}

/// Zamansal replik kurar.
fn replik(no: u32, bas: i64, bit: i64, metin: &str) -> Replik {
    Replik::yeni(no, Zaman::milis(bas), Zaman::milis(bit), metin)
}

// ------------------------------------------------------------- kaydırma ----

/// İleri kaydırma tüm replikleri aynı miktarda kaydırır.
#[test]
fn kaydirma_ileri_tum_replikleri_kaydirir() {
    let mut b = belge(vec![replik(1, 1000, 2000, "a"), replik(2, 3000, 4000, "b")]);
    let rapor = kaydir(&mut b, 1500);
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(2500));
    assert_eq!(b.replikler[0].bitis, Zaman::milis(3500));
    assert_eq!(b.replikler[1].baslangic, Zaman::milis(4500));
    assert_eq!(rapor.etkilenen_replik, 2);
    assert_eq!(rapor.istenen_ms, 1500);
    assert!(!rapor.kirpma_var());
}

/// Geri kaydırma 0'ın altına düşen replikleri kırpır ve bildirir.
#[test]
fn kaydirma_geri_sifir_altini_kirpar() {
    let mut b = belge(vec![replik(1, 500, 2000, "a"), replik(2, 5000, 6000, "b")]);
    let rapor = kaydir(&mut b, -1000);
    assert_eq!(b.replikler[0].baslangic, Zaman::SIFIR);
    assert_eq!(b.replikler[0].bitis, Zaman::milis(1000));
    assert_eq!(b.replikler[1].baslangic, Zaman::milis(4000));
    assert_eq!(rapor.kirpilan_replik, 1);
    assert!(rapor.kirpma_var());
    // Kırpma negatif süre üretmez.
    assert!(b.replikler.iter().all(Replik::zamanlama_gecerli));
}

/// Yüzde kaydırma belgenin toplam süresine göre çalışır.
#[test]
fn yuzde_kaydirma_toplam_sureye_gore_calisir() {
    // Toplam süre 10 000 ms; %10 = 1000 ms.
    let mut b = belge(vec![replik(1, 0, 5000, "a"), replik(2, 5000, 10_000, "b")]);
    let rapor = kaydir_yuzde(&mut b, 10.0);
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(1000));
    assert_eq!(b.replikler[1].bitis, Zaman::milis(11_000));
    assert_eq!(rapor.istenen_ms, 1000);
}

/// Negatif yüzde geriye kaydırır ve sıfırda kırpar.
#[test]
fn negatif_yuzde_kaydirma_kirpar() {
    let mut b = belge(vec![replik(1, 0, 2000, "a"), replik(2, 2000, 4000, "b")]);
    let rapor = kaydir_yuzde(&mut b, -75.0);
    assert_eq!(b.replikler[0].baslangic, Zaman::SIFIR);
    assert_eq!(b.replikler[1].baslangic, Zaman::SIFIR);
    assert_eq!(rapor.kirpilan_replik, 2);
}

/// Boş belgede yüzde kaydırma işlem yapmaz.
#[test]
fn yuzde_kaydirma_bos_belgede_islem_yapmaz() {
    let mut b = Belge::bos(Bicim::Srt);
    let rapor = kaydir_yuzde(&mut b, 50.0);
    assert_eq!(rapor.uygulanan_ms, 0);
    assert_eq!(b.replik_sayisi(), 0);
}

// ----------------------------------------------------------- normalizasyon ----

/// Kare hızına oturtma zaman damgalarını kare sınırına taşır.
#[test]
fn normalize_kare_hizine_oturtur() {
    let mut b = belge(vec![replik(1, 1042, 3280, "a")]);
    let rapor = normalize(&mut b, Some(KareHazi::sabit(30)), false, false);
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(1033));
    assert_eq!(b.replikler[0].bitis, Zaman::milis(3267));
    assert_eq!(rapor.kareye_oturtulan, 1);
}

/// 23.976 -> 24 dönüşümü normalize komutunda uygulanabilir.
#[test]
fn normalize_kare_hizi_donusu_23976_den_24_e() {
    let mut b = belge(vec![replik(1, 1042, 3280, "a")]);
    normalize(
        &mut b,
        Some(KareHazi::ayri(23.976).expect("ntsc")),
        false,
        false,
    );
    // 1042 ms -> 25 kare -> 1043 ms (23.976 fps)
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(1043));
}

/// Zaten kare sınırında olan zamanlar değişmez.
#[test]
fn normalize_kare_sinirindaki_zamanlari_degistirmez() {
    let mut b = belge(vec![replik(1, 1000, 2000, "a")]);
    let rapor = normalize(&mut b, Some(KareHazi::sabit(25)), false, false);
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(1000));
    assert_eq!(b.replikler[0].bitis, Zaman::milis(2000));
    assert_eq!(rapor.kareye_oturtulan, 0);
}

/// Sıralama replikleri başlangıç zamanına göre düzenler.
#[test]
fn normalize_sirala_baslangic_zamanina_gore_siralar() {
    let mut b = belge(vec![
        replik(1, 5000, 6000, "gec"),
        replik(2, 1000, 2000, "erken"),
        replik(3, 3000, 4000, "orta"),
    ]);
    let rapor = normalize(&mut b, None, true, false);
    let metinler: Vec<&str> = b.replikler.iter().map(|r| r.metin.as_str()).collect();
    assert_eq!(metinler, vec!["erken", "orta", "gec"]);
    assert!(rapor.siralanan > 0);
}

/// Numaralandırma 1'den başlayarak yeniden yazar.
#[test]
fn normalize_numarala_birden_baslar() {
    let mut b = belge(vec![replik(7, 1000, 2000, "a"), replik(9, 3000, 4000, "b")]);
    let rapor = normalize(&mut b, None, false, true);
    assert_eq!(b.replikler[0].numara, 1);
    assert_eq!(b.replikler[1].numara, 2);
    assert_eq!(rapor.numaralanan, 2);
}

/// Sıralı ve numaralı belgede normalize değişiklik yapmaz.
#[test]
fn normalize_zaten_duzenli_belgede_degisiklik_yapmaz() {
    let mut b = belge(vec![replik(1, 1000, 2000, "a"), replik(2, 3000, 4000, "b")]);
    let rapor = normalize(&mut b, Some(KareHazi::sabit(25)), true, true);
    assert_eq!(rapor.kareye_oturtulan, 0);
    assert_eq!(rapor.siralanan, 0);
    assert_eq!(rapor.numaralanan, 0);
    assert_eq!(rapor.toplam_sure_ms, 4000);
}

/// Kare sınırına oturtma sonrası bitiş başlangıçtan geriye düşmez.
#[test]
fn normalize_kisa_sureyi_kare_suresine_yukseltir() {
    // 25 fps'te kare süresi 40 ms. Başlangıç 1000, bitiş 1005 -> ikisi de
    // 1000'e oturur; normalize süreyi bir kareye çıkarır.
    let mut b = belge(vec![replik(1, 1000, 1005, "a")]);
    normalize(&mut b, Some(KareHazi::sabit(25)), false, false);
    assert!(b.replikler[0].zamanlama_gecerli());
    assert_eq!(b.replikler[0].sure_ms(), 40);
}

/// Kare hızı verilmezse zaman damgalarına dokunulmaz.
#[test]
fn normalize_kare_hizi_verilmezse_zamanlari_korur() {
    let mut b = belge(vec![replik(1, 1042, 3280, "a")]);
    normalize(&mut b, None, false, false);
    assert_eq!(b.replikler[0].baslangic, Zaman::milis(1042));
    assert_eq!(b.replikler[0].bitis, Zaman::milis(3280));
}

/// Çakışan replik çiftleri doğru sırayla listelenir.
#[test]
fn cakisan_ciftler_dogru_listelenir() {
    let b = belge(vec![
        replik(1, 0, 3000, "a"),
        replik(2, 2000, 4000, "b"),
        replik(3, 5000, 6000, "c"),
    ]);
    let ciftler = cakisan_ciftler(&b);
    assert_eq!(ciftler.len(), 1);
    assert_eq!(ciftler[0], (1, 2, 1000));
}

/// Negatif süreli replikler çakışma listesine girmez.
#[test]
fn cakisan_ciftler_negatif_sureyi_atlar() {
    let b = belge(vec![replik(1, 5000, 1000, "a"), replik(2, 2000, 4000, "b")]);
    assert!(cakisan_ciftler(&b).is_empty());
}

// ------------------------------------------------------------ dönüştürme ----

/// SRT'den ASS'e dönüşüm replik sayısını ve zamanlamayı korur.
#[test]
fn donusturme_srt_den_ass_e_zamanlama_korunur() {
    let b = belge(vec![
        replik(1, 1000, 3000, "Merhaba"),
        replik(2, 4000, 6000, "Dunya"),
    ]);
    let (yeni, rapor) = donustur(&b, Bicim::Ass);
    assert_eq!(yeni.bicim, Bicim::Ass);
    assert_eq!(rapor.replik_sayisi, 2);
    assert_eq!(yeni.replikler[0].baslangic, Zaman::milis(1000));
    assert_eq!(yeni.replikler[0].bitis, Zaman::milis(3000));
    assert_eq!(yeni.replikler[0].metin, "Merhaba");
    // Varsayılan stil tablosu üretilir.
    assert!(!yeni.stiller.is_empty());
    assert!(yeni.stil_tanimli("Default"));
    assert_eq!(yeni.replikler[0].stil.as_deref(), Some("Default"));
}

/// SRT'den ASS'e dönüşümde uydurulan varsayılan stil kayıp raporunda listelenir.
#[test]
fn donusturme_kaybi_acikca_listeler() {
    let b = belge(vec![replik(1, 1000, 3000, "Merhaba")]);
    let (_, rapor) = donustur(&b, Bicim::Ass);
    let kayiplar: Vec<&str> = rapor.kayiplar.iter().map(|k| k.ozellik.as_str()).collect();
    assert!(
        kayiplar.contains(&"replik stili"),
        "kayip raporu replik stilini listelemeli, listelenen: {kayiplar:?}"
    );
    assert!(
        kayiplar.contains(&"stil tablosu"),
        "kayip raporu stil tablosunu listelemeli, listelenen: {kayiplar:?}"
    );
}

/// ASS'ten SRT'e dönüşümde ASS stilleri kayıp olarak listelenir.
#[test]
fn donusturme_ass_den_srt_e_stiller_kayip_olur() {
    let mut b = Belge::bos(Bicim::Ass);
    b.replikler = vec![replik(1, 1000, 3000, "Merhaba")];
    b.stiller = subforge::bicim::varsayilan_ass_stilleri();
    b.replikler[0].stil = Some("Default".to_string());
    let (yeni, rapor) = donustur(&b, Bicim::Srt);
    assert_eq!(yeni.bicim, Bicim::Srt);
    let kayiplar: Vec<&str> = rapor.kayiplar.iter().map(|k| k.ozellik.as_str()).collect();
    assert!(kayiplar.contains(&"ASS stiller"));
    assert!(kayiplar.contains(&"replik stili"));
}

/// ASS'ten SRT'e dönüşüm kenarlık ve efekt alanlarını kayıp olarak listeler.
#[test]
fn donusturme_ass_den_srt_e_kenar_efekt_kaybi() {
    let mut b = Belge::bos(Bicim::Ass);
    let mut r = replik(1, 1000, 3000, "Merhaba");
    r.kenar_ust = 40;
    b.replikler = vec![r];
    let (_, rapor) = donustur(&b, Bicim::Srt);
    let kayiplar: Vec<&str> = rapor.kayiplar.iter().map(|k| k.ozellik.as_str()).collect();
    assert!(kayiplar.contains(&"kenar bosluklari ve efekt"));
}

/// ASS'ten WebVTT'ye dönüşüm stilleri kayıp olarak listeler.
#[test]
fn donusturme_ass_den_vtt_e_stil_kaybi() {
    let mut b = Belge::bos(Bicim::Ass);
    b.replikler = vec![replik(1, 1000, 3000, "Merhaba")];
    b.stiller = subforge::bicim::varsayilan_ass_stilleri();
    let (yeni, rapor) = donustur(&b, Bicim::Vtt);
    assert_eq!(yeni.bicim, Bicim::Vtt);
    let kayiplar: Vec<&str> = rapor.kayiplar.iter().map(|k| k.ozellik.as_str()).collect();
    assert!(kayiplar.contains(&"ASS stiller"));
}

/// WebVTT'den ASS'e dönüşüm varsayılan `Script Info` üretir.
#[test]
fn donusturme_vtt_den_ass_e_script_info_uretilir() {
    let mut b = Belge::bos(Bicim::Vtt);
    b.replikler = vec![replik(1, 1000, 3000, "Merhaba")];
    let (yeni, _) = donustur(&b, Bicim::Ass);
    assert!(yeni
        .basliklar
        .iter()
        .any(|(k, v)| k == "ScriptType" && v == "v4+"));
    assert!(yeni
        .basliklar
        .iter()
        .any(|(k, v)| k == "PlayResX" && v == "1920"));
}

/// Aynı biçime dönüşüm kayıp üretmez (metin ve zamanlama aynen korunur).
#[test]
fn donusturme_ayni_bicim_kayip_uretmez() {
    let b = belge(vec![replik(1, 1000, 3000, "Merhaba")]);
    let (yeni, rapor) = donustur(&b, Bicim::Srt);
    assert!(rapor.kayiplar.is_empty(), "kayip: {:?}", rapor.kayiplar);
    assert_eq!(yeni.replikler, b.replikler);
}

/// Dönüşüm ASS `Comment` satırlarını SRT'de kayıp olarak listeler.
#[test]
fn donusturme_comment_satiri_kaybi() {
    let mut b = Belge::bos(Bicim::Ass);
    let mut r = replik(1, 1000, 3000, "Yorum");
    r.tur = subforge::model::OlayTuru::Yorum;
    b.replikler = vec![r];
    let (_, rapor) = donustur(&b, Bicim::Srt);
    let kayiplar: Vec<&str> = rapor.kayiplar.iter().map(|k| k.ozellik.as_str()).collect();
    assert!(kayiplar.contains(&"Comment satirlari"));
}

/// Üç biçim arasındaki gidiş-dönüş zinciri metni korur.
#[test]
fn uc_bicim_gidis_donus_zinciri_metni_korur() {
    let ornek = "1\n00:00:01,000 --> 00:00:03,000\nMerhaba dunya\n\n";
    let (srt_metin, _) = (ornek, ());
    let srt_belge = subforge::srt::ayikla(srt_metin, &subforge::KodlamaRaporu::default());
    let (ass_belge, _) = donustur(&srt_belge, Bicim::Ass);
    let ass_metin = subforge::ass::yaz(&ass_belge);
    let ass_geri = subforge::ass::ayikla(&ass_metin, &subforge::KodlamaRaporu::default());
    let (vtt_belge, _) = donustur(&ass_geri, Bicim::Vtt);
    let vtt_metin = subforge::vtt::yaz(&vtt_belge);
    let vtt_geri = subforge::vtt::ayikla(&vtt_metin, &subforge::KodlamaRaporu::default());
    let (srt_son, _) = donustur(&vtt_geri, Bicim::Srt);
    assert_eq!(srt_son.replikler[0].metin, "Merhaba dunya");
    assert_eq!(srt_son.replikler[0].baslangic, Zaman::milis(1000));
    assert_eq!(srt_son.replikler[0].bitis, Zaman::milis(3000));
}
