//! Regel `name` über die öffentliche Schnittstelle: die Beispiele aus SPEC 5.2, die Stufe
//! je Rolle und der Katalogeintrag. Alle Namen sind erfunden.

use std::path::Path;

use assert_cmd::Command;
use promptscan::fund::Art;
use promptscan::katalog::{self, Guete};
use promptscan::text::Text;
use promptscan::{Optionen, Rolle, Stufe, Ziel, pruefe, regeln};
use serde_json::Value;

fn namen(roh: &str, optionen: &Optionen) -> Vec<(String, Stufe)> {
    pruefe(roh, optionen)
        .funde
        .into_iter()
        .filter(|f| f.regel == "name")
        .map(|f| (f.stelle, f.stufe))
        .collect()
}

#[test]
fn beispiele_der_spezifikation() {
    let ohne = Optionen::default();
    assert_eq!(
        namen(
            "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, ist krank.",
            &ohne
        ),
        vec![("Hans Muster".to_string(), Stufe::Gelb)]
    );
    let anwalt = Optionen {
        rolle: Some(Rolle::Lawyer),
        ziel: Ziel::Cloud,
    };
    assert_eq!(
        namen(
            "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will das Erbe ausschlagen.",
            &anwalt
        ),
        vec![("Erika Beispiel".to_string(), Stufe::Rot)]
    );
}

#[test]
fn gegenbeispiele_der_spezifikation() {
    let ohne = Optionen::default();
    for text in [
        "Art. 5 DSG",
        "Wer im Mai Zahlungen leisten muss?",
        "Frau Präsidentin",
        "Herr Rechtsanwalt",
        "BGE 136 II 508",
        "Der Regierungsrat des Kantons Uri und die Gemeinde Jens.",
        "vertreten durch Rechtsanwältin Erika Beispiel",
    ] {
        assert!(namen(text, &ohne).is_empty(), "«{text}»");
    }
}

/// SPEC 5.2, Art. 4 Abs. 1 lit. g: Die Ausnahme hängt an der Vertretung einer Partei, nicht
/// am Beruf. Ohne Vertretungsmerkmal wirkt der Anwaltstitel wie ein Titel.
#[test]
fn rechtsvertretung_nur_mit_vertretungsmerkmal() {
    let ohne = Optionen::default();
    let gelb = |n: &str| vec![(n.to_string(), Stufe::Gelb)];
    for (text, erwartet) in [
        (
            "Rechtsanwältin Erika Beispiel schreibt.",
            gelb("Erika Beispiel"),
        ),
        ("vertreten durch Rechtsanwältin Erika Beispiel", vec![]),
        ("Me Jean Dupont écrit.", gelb("Jean Dupont")),
        ("représentée par Me Jean Dupont", vec![]),
        ("Jean Dupont, avocat de la partie adverse", vec![]),
        ("l'avv. Mario Rossi scrive.", gelb("Mario Rossi")),
        ("rappresentato dall'avv. Mario Rossi", vec![]),
        ("Mario Rossi, avvocato della controparte", vec![]),
    ] {
        assert_eq!(namen(text, &ohne), erwartet, "«{text}»");
    }
}

/// Die eigene Anwältin nach einem Possessivpronomen bleibt als Vertretung unmarkiert
/// (Prüffälle fr-19 und it-19), ausser der Satz enthält eine Angabe nach Art. 5 lit. c DSG
/// (w3-namen-p4, P01–P03): Dann wird der Name gemeldet, und die Angabe steigt auf ORANGE.
#[test]
fn eigene_anwaltschaft_nur_mit_angabe_nach_art5c() {
    let ohne = Optionen::default();
    for text in [
        "Mon avocat, Me Julien Essai, me conseille d'accepter la proposition.",
        "Il mio avvocato, avv. Roberto Fittizio, dice che devo versare il saldo.",
        "Mein Anwalt, RA Hans Muster, schreibt.",
    ] {
        assert!(namen(text, &ohne).is_empty(), "«{text}»");
    }
    for (text, name) in [
        (
            "Mon avocat, Me Jean Dupont, est en incapacité de travail.",
            "Jean Dupont",
        ),
        (
            "Il mio avvocato, l'avv. Mario Rossi, è in malattia.",
            "Mario Rossi",
        ),
        (
            "Mein Anwalt, RA Hans Muster, ist arbeitsunfähig.",
            "Hans Muster",
        ),
    ] {
        assert_eq!(
            namen(text, &ohne),
            vec![(name.to_string(), Stufe::Gelb)],
            "«{text}»"
        );
        assert_eq!(
            pruefe(text, &ohne).hoechste(),
            Some(Stufe::Orange),
            "«{text}»"
        );
    }
}

/// Listen in der Form «Nachname Vorname» (praxis-de-2): Die Namen werden gemeldet, und mit
/// einer Rolle tragen sie die Stufe der Angaben daneben.
#[test]
fn listen_nach_nachnamen() {
    let ohne = Optionen::default();
    let liste = "Müller Andreas, Lager, 12 Tage, Grippe\nKeller Sabine, Buchhaltung, 31 Tage, Burnout\nGashi Arben, Produktion, 58 Tage, Unfall, SUVA-Fall";
    let gefunden: Vec<String> = namen(liste, &ohne).into_iter().map(|(n, _)| n).collect();
    assert_eq!(gefunden, ["Müller Andreas", "Keller Sabine", "Gashi Arben"]);
    let zimmer = "Zi. 15: Frei Claudia, 45, Diagnose F10.2, Entzug\nZi. 16: Keller Sabine, 38, Diagnose F32.1\nZi. 17: Müller Andreas, 52, Diagnose F41.0";
    let gefunden: Vec<String> = namen(zimmer, &ohne).into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        gefunden,
        ["Frei Claudia", "Keller Sabine", "Müller Andreas"]
    );
    assert_eq!(
        namen(
            "Patient: KELLER Sabine, geb. 12.03.1985, Diagnose F32.1",
            &ohne
        )
        .into_iter()
        .map(|(n, _)| n)
        .collect::<Vec<_>>(),
        ["KELLER Sabine"]
    );
    assert!(namen("Lieber Peter, ich komme morgen.", &ohne).is_empty());
}

/// Die Beispiele aus explain name, die sonst kein Test wörtlich prüft: mehrdeutiger Vorname,
/// Substantiv, Strassen- und Firmenname nach einem Vornamen, ein Wort der Vertretung.
#[test]
fn beispiele_der_grenzen_in_explain() {
    let ohne = Optionen::default();
    let gefunden =
        |text: &str| -> Vec<String> { namen(text, &ohne).into_iter().map(|(n, _)| n).collect() };
    assert!(gefunden("Vera kommt morgen.").is_empty());
    assert_eq!(gefunden("Vera Keller kommt morgen."), ["Vera Keller"]);
    assert_eq!(gefunden("Ich will Peter Geld geben."), ["Peter Geld"]);
    assert_eq!(
        gefunden("Wir treffen uns an der Peter Merian-Strasse."),
        ["Peter Merian-Strasse"]
    );
    assert_eq!(gefunden("Die Hans Muster AG zahlt."), ["Hans Muster"]);
    assert!(gefunden("Le défenseur Jean Dupont plaide.").is_empty());
}

/// Vornamen und Nachnamen, die zugleich Wörter der Listen nach Art. 5 Bst. c DSG sind,
/// bleiben Namen («Jude» als Vorname, «Cancer» als Nachname); der Nachname «Beispiel»
/// steht wie ein seltener Name nicht im Register (Reste Runde 1, w2-art5c).
#[test]
fn vornamen_und_nachnamen_aus_art5c_wortlisten_bleiben_namen() {
    let ohne = Optionen::default();
    for (text, name) in [
        ("Jude Beispiel spielt heute.", "Jude Beispiel"),
        ("Herr Jude Muster ist krank.", "Jude Muster"),
        ("M. Paul Cancer est malade.", "Paul Cancer"),
    ] {
        let gefunden: Vec<String> = namen(text, &ohne).into_iter().map(|(n, _)| n).collect();
        assert_eq!(gefunden, [name], "«{text}»");
    }
}

/// Befund w3-p1: Mehrbyte-Leerraum (U+00A0, U+202F, U+2009, U+2007 …) und Formatzeichen an
/// jeder Stelle von Listen und Sätzen führen nicht zum Absturz; vorher brach die Prüfung des
/// Zellenanfangs mitten in einem solchen Zeichen ab («Anwesend: M\u{A0}uster Hans»).
#[test]
fn mehrbyte_leerraum_an_jeder_stelle_ohne_absturz() {
    let zeichen = [
        '\u{A0}', '\u{202F}', '\u{2009}', '\u{2007}', '\u{2002}', '\u{2003}', '\u{3000}',
        '\u{200B}', '\u{AD}', '\u{2060}',
    ];
    let saetze = [
        "Anwesend: Muster Hans, Beispiel Erika.",
        "- Muster Hans\n- Beispiel Erika",
        "Name;Vorname\nMuster;Hans",
        "MUSTER Hans, 8002 Zürich",
        "Meine Patientin Erika Beispiel ist arbeitsunfähig.",
        "1. Muster Hans\n2. Beispiel Erika",
    ];
    let ohne = Optionen::default();
    for satz in saetze {
        let grenzen: Vec<usize> = satz
            .char_indices()
            .map(|(i, _)| i)
            .chain([satz.len()])
            .collect();
        for &i in &grenzen {
            for z in zeichen {
                let text = format!("{}{z}{}", &satz[..i], &satz[i..]);
                let _ = pruefe(&text, &ohne);
            }
        }
    }
    // Der Name in der Liste bleibt mit geschütztem Leerzeichen erkannt; «Beispiel Erika»
    // nicht, weil «Beispiel» kein Nachname des Registers ist. Nicht «Muster Hans»: «Muster»
    // steht im Wortlaut des OR (Art. 222) und braucht in einer Zelle einen eindeutigen
    // Partner.
    assert_eq!(
        namen("Anwesend: Brunner\u{A0}Peter, Beispiel Erika.", &ohne)
            .into_iter()
            .map(|(n, _)| n)
            .collect::<Vec<_>>(),
        ["Brunner\u{A0}Peter"]
    );
}

#[test]
fn personenbezug_erscheint_nicht_selbst() {
    let text = Text::neu("Die beschuldigte Person schweigt.");
    let treffer = regeln::name::erkenne(&text);
    assert_eq!(treffer.len(), 1);
    assert_eq!(
        (treffer[0].regel, treffer[0].art),
        ("personenbezug", Art::Personenbezug)
    );
    // SPEC 4.4: der Personenbezug selbst wird nie angezeigt.
    let bericht = pruefe("Die beschuldigte Person schweigt.", &Optionen::default());
    assert!(bericht.funde.is_empty());
}

#[test]
fn katalogeintrag() {
    let r = katalog::finde("name").expect("Regel name im Katalog");
    assert_eq!(r.guete, Guete::NichtGemessen);
    assert_eq!(r.lexika.len(), 10);
    assert!(r.lexika.contains(&regeln::name::VERTRETUNG));
    assert!(r.lexika.contains(&regeln::name::ALLTAG));
    // C1: der Artikel «Le» und die Amtsbezeichnungen danach.
    assert!(r.lexika.contains(&regeln::name::AMTSBEZEICHNUNGEN));
    assert!(r.quellen.iter().any(|q| q.url.contains("opendata.swiss")));
    assert!(r.quellen.iter().any(|q| q.url.contains("bger.ch")));
}

/// Eine Grussformel vor einem Satz über den Schreibenden ist kein Name (w3-namen-p5,
/// F01–F04); ein Anwaltsberuf oder «Arch.» ohne Namen auch nicht (w3-namen-p8, H01–H04).
/// Die Angabe nach Art. 5 lit. c DSG bleibt dann unter ORANGE. Gegenprobe: mit einem Namen
/// steigt sie auf ORANGE.
#[test]
fn ohne_namen_keine_orange_stufe() {
    let ohne = Optionen::default();
    for text in [
        "Lieber Peter, Ich bin seit Montag arbeitsunfähig.",
        "Liebe Anna, Meine Mutter bezieht Sozialhilfe.",
        "Gentile Mario, Le scrivo perché sono in malattia.",
        "Bonjour Pierre, Je suis en incapacité de travail depuis lundi.",
        "Je cherche un avocat Spécialiste FSA, car je suis en incapacité de travail.",
        "Cerco un avvocato Penalista perché sono in malattia.",
        "Siehe Arch. Nr. 12: Akten zur Sozialhilfe.",
        "Quel avocat Genevois peut m'aider ? Je touche l'aide sociale.",
    ] {
        let bericht = pruefe(text, &ohne);
        assert!(bericht.funde.iter().all(|f| f.regel != "name"), "«{text}»");
        assert!(
            bericht.hoechste() < Some(Stufe::Orange),
            "«{text}»: {:?}",
            bericht.hoechste()
        );
    }
    for text in [
        "Peter Keller ist seit Montag arbeitsunfähig.",
        "Mme Rochat est en incapacité de travail depuis lundi.",
    ] {
        assert_eq!(
            pruefe(text, &ohne).hoechste(),
            Some(Stufe::Orange),
            "«{text}»"
        );
    }
}

/// Dateien in UTF-8 mit geschützten und schmalen Leerzeichen über die Befehlszeile
/// (w3-namen-p1): kein Absturz, und die Namen daneben werden gefunden.
#[test]
fn datei_mit_geschuetzten_und_schmalen_leerzeichen() {
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "promptscan-name-leerzeichen-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&ordner);
    std::fs::create_dir_all(&ordner).expect("Testordner anlegen");
    let keine_konfiguration = ordner.join("gibt-es-nicht").join("config.toml");
    for (zeichen, kurz) in [
        ('\u{00A0}', "nbsp"),
        ('\u{202F}', "nnbsp"),
        ('\u{2009}', "thin"),
        ('\u{2007}', "figure"),
    ] {
        let text = format!(
            "Mme{zeichen}Rochat est en incapacité de travail depuis mars.\nDie Miete beträgt CHF{zeichen}1200.\nQuestion{zeichen}: est-elle malade{zeichen}?\n-{zeichen}Keller Sabine, Lager\n"
        );
        let datei = ordner.join(format!("{kurz}.txt"));
        std::fs::write(&datei, text.as_bytes()).expect("Testdatei schreiben");
        let ausgabe = Command::new(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))))
            .env("PROMPTSCAN_CONFIG", &keine_konfiguration)
            .env_remove("NO_COLOR")
            .env_remove("CLICOLOR_FORCE")
            .env_remove("CLICOLOR")
            .args(["--no-color", "--format", "json"])
            .arg(&datei)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let v: Value = serde_json::from_slice(&ausgabe).expect("gültiges JSON");
        let namen: Vec<&str> = v["funde"]
            .as_array()
            .expect("funde")
            .iter()
            .filter(|f| f["regel"] == "name")
            .filter_map(|f| f["stelle"].as_str())
            .collect();
        assert_eq!(namen, ["Rochat", "Keller Sabine"], "{kurz}");
    }
    let _ = std::fs::remove_dir_all(&ordner);
}
