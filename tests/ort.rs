//! Regeln `adresse` und `wohnort` über die öffentliche Schnittstelle `pruefe`: Stelle,
//! Stufe und Grund, wie sie der Nutzer sieht. Alle Beispiele sind erfunden.

use promptscan::katalog::Klasse;
use promptscan::{Grund, Optionen, Stufe, pruefe};

fn funde(roh: &str, regel: &str) -> Vec<(String, Stufe, Grund)> {
    pruefe(roh, &Optionen::default())
        .funde
        .into_iter()
        .filter(|f| f.regel == regel)
        .map(|f| (f.stelle, f.stufe, f.grund))
        .collect()
}

#[test]
fn adresse_ist_gelb() {
    assert_eq!(
        funde("Bitte an Seestrasse 12, 8002 Zürich schicken.", "adresse"),
        [(
            "Seestrasse 12, 8002 Zürich".to_string(),
            Stufe::Gelb,
            Grund::Personendaten
        )]
    );
}

#[test]
fn wohnort_ohne_identifikator_ist_hinweis() {
    assert_eq!(
        funde(
            "Die Klägerin, wohnhaft in Bern, verlangt Schadenersatz.",
            "wohnort"
        ),
        [("Bern".to_string(), Stufe::Hinweis, Grund::OhneIdentifikator)]
    );
}

#[test]
fn wohnort_mit_adresse_ist_gelb() {
    let roh =
        "Die Klägerin, Seestrasse 12, 8002 Zürich, wohnhaft in Zürich, verlangt Schadenersatz.";
    assert_eq!(
        funde(roh, "wohnort"),
        [("Zürich".to_string(), Stufe::Gelb, Grund::Personendaten)]
    );
    assert_eq!(funde(roh, "adresse").len(), 1);
}

#[test]
fn zeilenende_im_windows_format() {
    // Text::neu macht aus CRLF ein \n; die Adresse bleibt eine Stelle.
    let f = funde("Erika Beispiel\r\nMusterweg 5a\r\n3000 Bern\r\n", "adresse");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].0, "Musterweg 5a\n3000 Bern");
}

#[test]
fn ortsnamen_allein_melden_nichts() {
    let b = pruefe(
        "Die Tagung findet in 8002 Zürich statt; Zürich und Lugano sind Gemeinden.",
        &Optionen::default(),
    );
    assert!(
        b.funde
            .iter()
            .all(|f| f.regel != "adresse" && f.regel != "wohnort"),
        "{:?}",
        b.funde
    );
}

#[test]
fn katalog_fuehrt_beide_regeln() {
    let info = promptscan::regeln::info();
    for id in ["adresse", "wohnort"] {
        let r = info.iter().find(|r| r.id == id).expect(id);
        assert_eq!(r.klasse, Klasse::B);
        assert!(!r.grundlage.is_empty() && !r.quellen.is_empty() && !r.grenzen.is_empty());
        for d in &r.lexika {
            assert!(promptscan::lexikon::dateien().any(|x| x == *d), "{d}");
        }
    }
}

#[test]
fn zahlen_neben_ortsnamen_sind_keine_adresse() {
    for roh in [
        "BGE 145 III 72, 8002 Zürich",
        "Tel. 044 123 45 67, 8002 Zürich",
        "geb. 12.03.1985, 8002 Zürich",
        "gemäss Art. 12 Abs. 2, 8002 Zürich",
    ] {
        assert!(funde(roh, "adresse").is_empty(), "{roh:?}");
    }
}

#[test]
fn wohnort_nimmt_den_laengsten_gemeindenamen() {
    assert_eq!(
        funde("wohnhaft in Muri bei Bern", "wohnort")[0].0,
        "Muri bei Bern"
    );
    assert_eq!(funde("domicilié à Fribourg", "wohnort")[0].0, "Fribourg");
}
