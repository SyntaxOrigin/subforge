//! Altı güvenilirlik kuralı ve eşik sınır durumları.
//!
//! Kural motoru biçimden bağımsızdır; bu dosyadaki testler SRT, ASS ve VTT
//! için **aynı** kural kümesinin aynı sonucu verdiğini de gösterir.

use subforge::kodlama::KodlamaRaporu;
use subforge::kural::{denetle, Kural, KuralEsikleri};
use subforge::model::{Belge, Bicim, Replik};
use subforge::zaman::Zaman;

/// Belgeyi verilen repliklerden kurar (test kısayolu).
fn belge(replikler: Vec<Replik>) -> Belge {
    let mut b = Belge::bos(Bicim::Srt);
    b.replikler = replikler;
    b
}

/// Zamansal replik kurar.
fn replik(no: u32, bas: i64, bit: i64, metin: &str) -> Replik {
    Replik::yeni(no, Zaman::milis(bas), Zaman::milis(bit), metin)
}

/// Belirli bir kuralın bulgu sayısını döndürür.
fn adet(bulgular: &[subforge::Bulgu], kural: Kural) -> usize {
    bulgular.iter().filter(|b| b.kural == kural).count()
}

// ------------------------------------------- Kural 1: okuma hızı ----

/// Okuma hızı eşiğin tam üstünde tetiklenir.
#[test]
fn kural1_okuma_hizi_esigi_ustu_tetiklenir() {
    // 3 saniyede 60 karakter = 20 karakter/sn > 17 eşiği.
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(60))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 1);
}

/// Okuma hızı tam eşikte tetiklenmez (`>` karşılaştırması).
#[test]
fn kural1_okuma_hizi_tam_esikte_tetiklenmez() {
    // 2 saniyede 34 karakter = 17 karakter/sn == eşik.
    let b = belge(vec![replik(1, 0, 2000, &"a".repeat(34))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 0);
}

/// Okuma hızı eşiğin 0,1 altında tetiklenmez (eşik-1 senaryosu).
#[test]
fn kural1_okuma_hizi_esik_bir_altinda_tetiklenmez() {
    // 2 saniyede 32 karakter = 16 karakter/sn.
    let b = belge(vec![replik(1, 0, 2000, &"a".repeat(32))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 0);
}

/// Okuma hızı eşiğin 0,1 üstünde tetiklenir (eşik+1 senaryosu).
#[test]
fn kural1_okuma_hizi_esik_bir_ustunde_tetiklenir() {
    // 2 saniyede 36 karakter = 18 karakter/sn.
    let b = belge(vec![replik(1, 0, 2000, &"a".repeat(36))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 1);
}

/// Negatif süreli replikte okuma hızı ölçülmez (bu kural 5'in işidir).
#[test]
fn kural1_negatif_surede_olculmez() {
    let b = belge(vec![replik(1, 5000, 1000, &"a".repeat(500))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 0);
    assert_eq!(adet(&bulgular, Kural::SiraBozuklugu), 1);
}

// --------------------------------- Kural 2: asgari dinlenme süresi ----

/// Asgari dinlenme süresi (mutlak 1000 ms) işi bağlayıcı olduğunda bulgu üretir.
#[test]
fn kural2_asgari_dinlenme_mutlak_esik_tetiklenir() {
    // 4 karakter: 4/6*1000 = 667 ms < 1000 ms, dolayısıyla mutlak eşik baskın.
    let b = belge(vec![replik(1, 0, 500, "Kisa")]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::AsgariDinlenme), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::AsgariDinlenme)
        .expect("bulgu var");
    assert_eq!(bulgu.esik, "1000 ms");
}

/// Asgari okuma hızından gelen süre (6 karakter/sn) mutlak eşiği aşar.
#[test]
fn kural2_asgari_hiz_esigi_uygulanir() {
    // 30 karakter, 4 saniye = 7.5 karakter/sn; gereken 5000 ms baskın.
    let b = belge(vec![replik(1, 0, 4000, &"a".repeat(30))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::AsgariDinlenme)
        .expect("bulgu var");
    assert_eq!(bulgu.esik, "5000 ms");
    // Okuma hızı üst sınırı (17) aşılmadığı için kural 1 tetiklenmez.
    assert_eq!(adet(&bulgular, Kural::OkumaHizi), 0);
}

/// Gereken süre tam eşikte bulgu üretmez.
#[test]
fn kural2_tam_esikte_bulgu_uretmez() {
    // 30 karakter, 5 saniyede okunur: gereken 5000 ms, gerçek 5000 ms.
    let b = belge(vec![replik(1, 0, 5000, &"a".repeat(30))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::AsgariDinlenme), 0);
}

/// Gereken sürenin 1 ms altı bulgu üretir (eşik-1).
#[test]
fn kural2_esik_bir_ms_altinda_bulgu_uretir() {
    let b = belge(vec![replik(1, 0, 4999, &"a".repeat(30))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::AsgariDinlenme), 1);
}

/// Gereken sürenin 1 ms üstü bulgu üretmez (eşik+1).
#[test]
fn kural2_esik_bir_ms_ustunde_bulgu_uretmez() {
    let b = belge(vec![replik(1, 0, 5001, &"a".repeat(30))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::AsgariDinlenme), 0);
}

// ------------------------------- Kural 3: satır uzunluğu ve satır sayısı ----

/// Eşiğin tam üstündeki satır tetiklenir.
#[test]
fn kural3_satir_uzunlugu_esik_ustu_tetiklenir() {
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(43))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 1);
}

/// Tam eşikte (42 karakter) satır tetiklenmez.
#[test]
fn kural3_satir_uzunlugu_tam_esikte_tetiklenmez() {
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(42))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 0);
}

/// Eşiğin bir altında (41 karakter) satır tetiklenmez.
#[test]
fn kural3_satir_uzunlugu_esik_bir_altinda_tetiklenmez() {
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(41))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 0);
}

/// Eşiğin bir üstünde (43 karakter) satır tetiklenir.
#[test]
fn kural3_satir_uzunlugu_esik_bir_ustunde_tetiklenir() {
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(43))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 1);
}

/// İki satır normal kabul edilir, üç satır tetiklenir.
#[test]
fn kural3_okunabilir_satir_sayisi_tetiklenir() {
    let iki = belge(vec![replik(1, 0, 3000, "bir\niki")]);
    let bulgular = denetle(&iki, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 0);

    let uc = belge(vec![replik(1, 0, 3000, "bir\niki\nuc")]);
    let bulgular = denetle(&uc, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 1);
}

/// Uzun satır bulgusu doğru satır numarasını taşır.
#[test]
fn kural3_uzun_satiri_satir_numarasini_tasir() {
    let uzun = "b".repeat(43);
    let b = belge(vec![replik(1, 0, 3000, &format!("kisa\n{uzun}"))]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::SatirUzunlugu)
        .expect("bulgu var");
    assert_eq!(bulgu.satir, 2);
    assert_eq!(bulgu.olcum, "43 karakter");
}

// ------------------------------- Kural 4: çakışan zaman aralıkları ----

/// Çakışan iki replik bulgu üretir.
#[test]
fn kural4_cakisan_aralik_tetiklenir() {
    let b = belge(vec![
        replik(1, 0, 3000, "bir"),
        replik(2, 2000, 4000, "iki"),
    ]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::CakisanAraliklar), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::CakisanAraliklar)
        .expect("bulgu var");
    assert_eq!(bulgu.olcum, "1000 ms cakisma");
    assert_eq!(bulgu.iliskili_replik_no, Some(2));
}

/// Bitiş ile başlangıç tam eşitse çakışma yoktur.
#[test]
fn kural4_temas_eden_aralik_cakisma_degildir() {
    let b = belge(vec![
        replik(1, 0, 3000, "bir"),
        replik(2, 3000, 4000, "iki"),
    ]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::CakisanAraliklar), 0);
}

/// 1 ms çakışma eşiğin tam üstünde bulgu üretir (eşik+1).
#[test]
fn kural4_bir_ms_cakisma_bulgu_uretir() {
    let b = belge(vec![
        replik(1, 0, 3000, "bir"),
        replik(2, 2999, 4000, "iki"),
    ]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::CakisanAraliklar), 1);
}

/// Boşluk eşiği 100 ms'e çıkarılırsa 50 ms çakışma bulgu üretmez.
#[test]
fn kural4_ozel_bosluk_esigi_uygulanir() {
    let b = belge(vec![
        replik(1, 0, 3000, "bir"),
        replik(2, 2950, 4000, "iki"),
    ]);
    let gevsek = KuralEsikleri {
        kabul_edilebilir_bosluk_ms: 100,
        ..KuralEsikleri::default()
    };
    let bulgular = denetle(&b, &gevsek);
    assert_eq!(adet(&bulgular, Kural::CakisanAraliklar), 0);
}

// ------------------------------- Kural 5: sıra bozukluğu / negatif süre ----

/// Negatif süre bulgu üretir.
#[test]
fn kural5_negatif_sure_tetiklenir() {
    let b = belge(vec![replik(1, 5000, 2000, "ters")]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SiraBozuklugu), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::SiraBozuklugu)
        .expect("bulgu var");
    assert!(bulgu.olcum.contains("-3000"));
}

/// Sıfır süre bulgu üretir.
#[test]
fn kural5_sifir_sure_tetiklenir() {
    let b = belge(vec![replik(1, 1000, 1000, "anlik")]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SiraBozuklugu), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::SiraBozuklugu)
        .expect("bulgu var");
    assert_eq!(bulgu.olcum, "0 ms");
}

/// Sıra bozukluğu (ikinci replik birinciden önce başlıyor) bulgu üretir.
#[test]
fn kural5_sira_bozuklugu_tetiklenir() {
    let b = belge(vec![
        replik(1, 5000, 6000, "gec"),
        replik(2, 1000, 2000, "erken"),
    ]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert!(adet(&bulgular, Kural::SiraBozuklugu) >= 1);
}

/// Artan sıra bulgu üretmez.
#[test]
fn kural5_artan_sira_bulgu_uretmez() {
    let b = belge(vec![
        replik(1, 0, 2000, "bir"),
        replik(2, 3000, 4000, "iki"),
    ]);
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::SiraBozuklugu), 0);
}

// ------------------------------- Kural 6: karakter kodlaması ----

/// BOM bulgu üretir.
#[test]
fn kural6_bom_bulgu_uretir() {
    let mut b = belge(vec![replik(1, 0, 2000, "bir")]);
    b.kodlama = KodlamaRaporu {
        kodlama: "utf-8-bom".to_string(),
        bom_var: true,
        ..KodlamaRaporu::default()
    };
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::KodlamaBozuklugu), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::KodlamaBozuklugu)
        .expect("bulgu var");
    assert!(bulgu.olcum.contains("BOM"));
}

/// Karışık kodlama bulgu üretir.
#[test]
fn kural6_karisik_kodlama_bulgu_uretir() {
    let mut b = belge(vec![]);
    b.kodlama = KodlamaRaporu {
        kodlama: "karisik".to_string(),
        karisik_kodlama: true,
        tek_bayt_satir_sayisi: 3,
        ..KodlamaRaporu::default()
    };
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::KodlamaBozuklugu), 1);
    let bulgu = bulgular
        .iter()
        .find(|b| b.kural == Kural::KodlamaBozuklugu)
        .expect("bulgu var");
    assert!(bulgu.olcum.contains("3 tek bayt satir"));
}

/// Bozuk bayt bulgu üretir.
#[test]
fn kural6_bozuk_bayt_bulgu_uretir() {
    let mut b = belge(vec![]);
    b.kodlama = KodlamaRaporu {
        kodlama: "windows-1254".to_string(),
        bozuk_bayt_sayisi: 4,
        ..KodlamaRaporu::default()
    };
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::KodlamaBozuklugu), 1);
}

/// Temiz UTF-8 kodlama bulgu üretmez.
#[test]
fn kural6_temiz_kodlama_bulgu_uretmez() {
    let mut b = belge(vec![replik(1, 0, 2000, "bir")]);
    b.kodlama = KodlamaRaporu {
        kodlama: "utf-8".to_string(),
        ..KodlamaRaporu::default()
    };
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&bulgular, Kural::KodlamaBozuklugu), 0);
}

// ------------------------------------------ biçim bağımsızlık ----

/// Aynı replikler ASS belgesinde de aynı kuralları tetikler.
#[test]
fn kurallar_ass_uyumlu_belgede_ayni_sonucu_verir() {
    let replikler = vec![
        replik(1, 0, 500, "kisa"),  // kural 2
        replik(2, 400, 1200, "uc"), // kural 4 (çakışma)
    ];
    let srt_belge = belge(replikler.clone());
    let mut ass_belge = Belge::bos(Bicim::Ass);
    ass_belge.replikler = replikler;

    let esik = KuralEsikleri::default();
    let srt_bulgular = denetle(&srt_belge, &esik);
    let ass_bulgular = denetle(&ass_belge, &esik);
    let srt_turler: Vec<Kural> = srt_bulgular.iter().map(|b| b.kural).collect();
    let ass_turler: Vec<Kural> = ass_bulgular.iter().map(|b| b.kural).collect();
    assert_eq!(srt_turler, ass_turler);
    assert!(srt_bulgular.len() >= 2);
}

/// Aynı replikler VTT belgesinde de aynı kuralları tetikler.
#[test]
fn kurallar_vtt_uyumlu_belgede_ayni_sonucu_verir() {
    let replikler = vec![replik(1, 0, 500, "kisa")];
    let mut vtt_belge = Belge::bos(Bicim::Vtt);
    vtt_belge.replikler = replikler.clone();
    let srt_belge = belge(replikler);
    let esik = KuralEsikleri::default();
    let vtt_bulgular = denetle(&vtt_belge, &esik);
    let srt_bulgular = denetle(&srt_belge, &esik);
    assert_eq!(vtt_bulgular.len(), srt_bulgular.len());
    assert_eq!(vtt_bulgular[0].kural, srt_bulgular[0].kural);
}

// ------------------------------------------ özel eşik profilleri ----

/// Özel eşik profili okuma hızı kuralını gevşetir.
#[test]
fn ozel_esik_profili_okuma_hizini_gevsetir() {
    let b = belge(vec![replik(1, 0, 2000, &"a".repeat(36))]); // 18 karakter/sn
    let siki = denetle(&b, &KuralEsikleri::default());
    assert_eq!(adet(&siki, Kural::OkumaHizi), 1);

    let gevsek = KuralEsikleri {
        okuma_hizi: 25.0,
        ..KuralEsikleri::default()
    };
    let sonuc = denetle(&b, &gevsek);
    assert_eq!(adet(&sonuc, Kural::OkumaHizi), 0);
}

/// Özel eşik profili satır uzunluğu kuralını sıkılaştırır.
#[test]
fn ozel_esik_profili_satir_uzunlugunu_sikilastirir() {
    let b = belge(vec![replik(1, 0, 3000, &"a".repeat(30))]);
    let siki = KuralEsikleri {
        en_fazla_karakter: 20,
        ..KuralEsikleri::default()
    };
    let bulgular = denetle(&b, &siki);
    assert_eq!(adet(&bulgular, Kural::SatirUzunlugu), 1);
}

/// Gereken süre hesabı her iki koşulun büyüğünü verir.
#[test]
fn gereken_sure_iki_kosulun_buyugunu_alir() {
    let esik = KuralEsikleri::default();
    // 3 karakter: 1000 ms (mutlak) baskın.
    assert_eq!(esik.gereken_sure_ms(3), 1000);
    // 60 karakter: 10000 ms (6 karakter/sn) baskın.
    assert_eq!(esik.gereken_sure_ms(60), 10_000);
}

/// Altı kuralın altısı da adlandırılabilir ve sıralanabilir.
#[test]
fn kurallarin_altisi_cesitlidir() {
    assert_eq!(Kural::TUMU.len(), 6);
    let adlar: Vec<&str> = Kural::TUMU.iter().map(|k| k.ad()).collect();
    assert_eq!(
        adlar,
        vec![
            "okuma_hizi",
            "asgari_dinlenme",
            "satir_uzunlugu",
            "cakisan_araliklar",
            "sira_bozuklugu",
            "kodlama_bozuklugu"
        ]
    );
    for k in Kural::TUMU {
        assert!(!k.aciklama().is_empty());
    }
}

/// Temiz, kurallara uyan belgede hiç bulgu üretilmez.
///
/// Metinler kısa tutulmuştur çünkü varsayılan asgari okuma hızı (6 karakter/sn)
/// mutlak 1,0 saniyelik eşiği çoğu gerçek replikte aşar; "temiz" belgenin de
/// bu ikisini birlikte sağlaması gerekir.
#[test]
fn temiz_belgede_bulgu_uretmez() {
    let mut b = belge(vec![
        replik(1, 0, 3000, "Merhaba dunya"),
        replik(2, 4000, 7000, "Ikinci replik"),
    ]);
    b.kodlama = KodlamaRaporu {
        kodlama: "utf-8".to_string(),
        ..KodlamaRaporu::default()
    };
    let bulgular = denetle(&b, &KuralEsikleri::default());
    assert!(
        bulgular.is_empty(),
        "temiz belgede bulgu olmamali: {bulgular:?}"
    );
}
