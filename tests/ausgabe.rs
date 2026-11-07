//! Die Ausgabe an den Prüffällen der Gestaltung (Runde 2, Paket A; docs/AUSGABE.md).
//!
//! Die Berichte entstehen mit `promptscan::pruefe` aus den Prüffällen: f03 bis f09 und die
//! Aufrufe aus der Ästhetik-Prüfung, dazu ROT mit Gesundheitsdaten (--role doctor) und
//! --role authority mit «Sozialhilfe». Geprüft werden Breite (80 und 60 Spalten), ASCII,
//! Kopfzeile, Zeilenansicht, Gruppen, Ergebnisblock, Farben, Englisch, JSON, `rules`,
//! `explain`, die Beispiele in SPEC und AUSGABE.md und die Normenliste in SPEC 10.1.
//! Die Personen sind erfunden; es sind die Platzhalternamen der SPEC.

use std::collections::{BTreeMap, BTreeSet};

use promptscan::ausgabe::{self, Darstellung};
use promptscan::katalog;
use promptscan::lexikon;
use promptscan::normen::{self, Norm};
use promptscan::stufen;
use promptscan::{Bericht, Optionen, Rolle, RolleQuelle, Stufe, Ziel};
use serde_json::Value;

const DOKU: &str = include_str!("../docs/AUSGABE.md");
const SPEC: &str = include_str!("../docs/SPEC.md");

// --- Prüffälle --------------------------------------------------------------------------

const P01_KEINE: &str = "Formuliere eine höfliche Absage für die Einladung zum Apéro am Freitag.";
const P02_BGE: &str =
    "Nenne mir passende BGE zum Kündigungsschutz bei Krankheit und fasse sie zusammen.";
const F03_GELB: &str = "Schreibe eine Glückwunschkarte an Hans Muster, Seestrasse 12, 8002 Zürich, zum 60. Geburtstag.\n";
const F04_ORANGE: &str = concat!(
    "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich,\n",
    "AHV 756.1234.5678.97, ist seit März zu 50 % arbeitsunfähig (Diagnose F32.1).\n",
    "Formuliere eine Kündigung und nenne passende BGE zum Kündigungsschutz.\n",
);
const MANDANTIN: &str =
    "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will das Erbe ausschlagen.";
const F07_VIELE: &str = concat!(
    "Erstelle aus dieser Liste eine Lohnabrechnung für November:\n",
    "Hans Muster, geb. 12.03.1985, AHV 756.1234.5678.97, IBAN CH93 0076 2011 6238 5295 7, Tel. 079 123 45 67, hans.muster@example.com\n",
    "Erika Beispiel, geb. 04.07.1990, AHV 756.9876.5432.17, IBAN CH56 0483 5012 3456 7800 9, Tel. 078 765 43 21, erika.beispiel@example.com\n",
    "Peter Probst, geb. 21.11.1978, Bahnhofstrasse 5, 3011 Bern, Tel. 031 222 33 44, peter.probst@example.com\n",
    "Laura Lehmann, geb. 30.01.1995, Rue du Lac 8, 1003 Lausanne, laura.lehmann@example.com\n",
    "Frau Lehmann ist schwanger, Herr Probst ist Mitglied der Gewerkschaft Unia.\n",
);
const F08_LANG: &str = concat!(
    "Bitte überarbeite den folgenden Entwurf eines Schreibens an die Gegenpartei.\n",
    "Achte auf einen sachlichen, aber bestimmten Ton.\n",
    "Die Gliederung soll erhalten bleiben.\n",
    "\n",
    "Sehr geehrte Damen und Herren\n",
    "\n",
    "Wir beziehen uns auf Ihr Schreiben vom 14. Oktober 2026.\n",
    "Unsere Klientin Erika Beispiel bestreitet die geltend gemachte Forderung.\n",
    "Die Rechnung vom 2. September 2026 wurde fristgerecht beanstandet.\n",
    "Die Mängel wurden mit E-Mail vom 5. September 2026 gerügt.\n",
    "\n",
    "1. Sachverhalt\n",
    "\n",
    "Die Parteien schlossen am 1. März 2026 einen Werkvertrag.\n",
    "Die Unternehmerin verpflichtete sich zur Sanierung eines Badezimmers.\n",
    "Der Werklohn wurde pauschal vereinbart.\n",
    "Die Arbeiten sollten bis Ende Juni abgeschlossen sein.\n",
    "Sie wurden erst im August beendet.\n",
    "Bei der Abnahme zeigten sich Risse in den Fliesen.\n",
    "Zudem ist die Dusche undicht.\n",
    "\n",
    "2. Rechtliches\n",
    "\n",
    "Der Besteller hat das Werk nach der Ablieferung zu prüfen.\n",
    "Mängel sind dem Unternehmer sofort anzuzeigen.\n",
    "Die Mängelrüge erfolgte rechtzeitig und substantiiert.\n",
    "Der Besteller kann eine Minderung oder die Nachbesserung verlangen.\n",
    "Unsere Klientin wählt die Nachbesserung.\n",
    "\n",
    "3. Forderung\n",
    "\n",
    "Wir fordern Sie auf, die Mängel bis zum 30. November 2026 zu beheben.\n",
    "Bis dahin wird die Restzahlung zurückbehalten.\n",
    "Eine Zahlung erfolgt nach Abnahme auf das Konto CH93 0076 2011 6238 5295 7.\n",
    "\n",
    "4. Weiteres\n",
    "\n",
    "Für Rückfragen stehen wir gerne zur Verfügung.\n",
    "Eine einvernehmliche Lösung ziehen wir vor.\n",
    "Sollte bis zur genannten Frist nichts geschehen, behalten wir uns weitere Schritte vor.\n",
    "\n",
    "Freundliche Grüsse\n",
    "\n",
    "Kanzlei Muster & Partner\n",
    "Nenne zum Schluss passende Urteile des Bundesgerichts zur Mängelrüge.\n",
);
const F09_VERMERK: &str = "VERTRAULICH\nFasse das Protokoll der Sitzung zusammen.\n";
const F15_ARZT: &str = "Mein Patient Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich, leidet an einer Depression (Diagnose F32.1). Formuliere ein Arztzeugnis.\n";
const F16_BEHOERDE: &str = "Frau Erika Beispiel, Seestrasse 12, 8002 Zürich, bezieht seit Mai Sozialhilfe. Formuliere eine Verfügung zur Rückforderung.\n";
const ANWAELTIN: &str = "Als Anwältin habe ich Hans Muster, Seestrasse 12, 8002 Zürich, beraten. Hans Muster ist krank.";
/// Breite Zeichen und ein kombinierendes Zeichen im Prompt.
const BREIT: &str = "Bitte schreibe an 山田太郎 und an Rene\u{301} Muster, Seestrasse 12, 8002 Zürich, Tel. 079 123 45 67, einen Brief über den Vertrag.";

/// Ein Prüffall: Text, Rolle und Ziel.
struct Fall {
    name: &'static str,
    text: &'static str,
    rolle: Option<Rolle>,
    ziel: Ziel,
}

const fn fall(name: &'static str, text: &'static str, rolle: Option<Rolle>, ziel: Ziel) -> Fall {
    Fall {
        name,
        text,
        rolle,
        ziel,
    }
}

const FAELLE: &[Fall] = &[
    fall("p01-keine", P01_KEINE, None, Ziel::Cloud),
    fall("p02-bge", P02_BGE, None, Ziel::Cloud),
    fall("f03-gelb", F03_GELB, None, Ziel::Cloud),
    fall("f04-orange", F04_ORANGE, None, Ziel::Cloud),
    fall("p05-lawyer", MANDANTIN, Some(Rolle::Lawyer), Ziel::Cloud),
    fall("p06-offen", MANDANTIN, None, Ziel::Cloud),
    fall("f07-viele", F07_VIELE, None, Ziel::Cloud),
    fall("f08-lang", F08_LANG, None, Ziel::Cloud),
    fall("f04-lokal", F04_ORANGE, None, Ziel::Local),
    fall("f09-vermerk", F09_VERMERK, None, Ziel::Cloud),
    fall("f15-arzt", F15_ARZT, Some(Rolle::Doctor), Ziel::Cloud),
    fall(
        "f16-behoerde",
        F16_BEHOERDE,
        Some(Rolle::Authority),
        Ziel::Cloud,
    ),
    fall("anwaeltin", ANWAELTIN, None, Ziel::Cloud),
    fall("anwaeltin-none", ANWAELTIN, Some(Rolle::None), Ziel::Cloud),
    fall("lawyer-lokal", MANDANTIN, Some(Rolle::Lawyer), Ziel::Local),
    fall("breit", BREIT, None, Ziel::Cloud),
];

fn pruefe(text: &str, rolle: Option<Rolle>, ziel: Ziel) -> Bericht {
    promptscan::pruefe(text, &Optionen { rolle, ziel })
}

fn bericht(f: &Fall) -> Bericht {
    pruefe(f.text, f.rolle, f.ziel)
}

// --- Darstellungen ----------------------------------------------------------------------

const fn darstellung(breite: usize, farbe: bool, englisch: bool, ascii: bool) -> Darstellung {
    Darstellung {
        englisch,
        farbe,
        nur_16_farben: false,
        breite,
        ascii,
    }
}

const OHNE_FARBE: Darstellung = darstellung(80, false, false, false);
const MIT_FARBE: Darstellung = darstellung(80, true, false, false);
const ENGLISCH: Darstellung = darstellung(80, false, true, false);

/// Die Darstellungen, in denen jeder Prüffall geprüft wird.
fn varianten(breite: usize) -> Vec<(&'static str, Darstellung)> {
    vec![
        ("ohne Farbe", darstellung(breite, false, false, false)),
        ("mit Farbe", darstellung(breite, true, false, false)),
        (
            "16 Farben",
            Darstellung {
                nur_16_farben: true,
                ..darstellung(breite, true, false, false)
            },
        ),
        ("englisch", darstellung(breite, false, true, false)),
        ("englisch mit Farbe", darstellung(breite, true, true, false)),
        ("ascii", darstellung(breite, false, false, true)),
        ("ascii englisch", darstellung(breite, false, true, true)),
    ]
}

fn text(f: &Fall, d: &Darstellung) -> String {
    ausgabe::bericht_text(&bericht(f), d)
}

// --- Hilfen -----------------------------------------------------------------------------

/// Entfernt ANSI-Sequenzen der Form ESC [ … Endzeichen (0x40 bis 0x7E).
fn ohne_escapes(s: &str) -> String {
    let mut aus = String::new();
    let mut zeichen = s.chars().peekable();
    while let Some(c) = zeichen.next() {
        if c == '\u{1B}' && zeichen.peek() == Some(&'[') {
            zeichen.next();
            for d in zeichen.by_ref() {
                if ('\u{40}'..='\u{7E}').contains(&d) {
                    break;
                }
            }
        } else {
            aus.push(c);
        }
    }
    aus
}

/// Spalten im Terminal: kombinierende Zeichen 0, ostasiatisch breite 2, sonst 1 (genug für
/// die Zeichen der Prüffälle).
fn spalten(s: &str) -> usize {
    s.chars()
        .map(|c| match c as u32 {
            0x0300..=0x036F => 0,
            0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFF60
            | 0x1F300..=0x1F64F
            | 0x20000..=0x3FFFD => 2,
            _ => 1,
        })
        .sum()
}

/// Zeilen, die breiter sind als `breite`. Eine Zeile aus einem einzigen Wort (eine lange
/// Adresse) darf überstehen, wenn `wort_erlaubt`.
fn zu_breit(aus: &str, breite: usize, wort_erlaubt: bool) -> Vec<String> {
    ohne_escapes(aus)
        .lines()
        .filter(|z| spalten(z) > breite && !(wort_erlaubt && !z.trim().contains(' ')))
        .map(String::from)
        .collect()
}

/// Fasst Leerraum zusammen, damit Zeilenumbrüche beim Suchen nicht stören.
fn einzeilig(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn zeilen(s: &str) -> Vec<&str> {
    s.lines().collect()
}

/// Alle Texte von `rules` und `explain` in einer Darstellung.
fn erklaerungen(d: &Darstellung) -> Vec<(String, String)> {
    let mut v = vec![("rules".to_string(), ausgabe::regeln_text(d))];
    for r in katalog::alle() {
        v.push((format!("explain {}", r.id), ausgabe::erklaerung_text(&r, d)));
    }
    for s in [Stufe::Rot, Stufe::Orange, Stufe::Gelb, Stufe::Hinweis] {
        v.push((
            format!("explain {s:?}"),
            ausgabe::stufen_erklaerung_text(s, d),
        ));
    }
    v
}

// --- A2: Breite -------------------------------------------------------------------------

#[test]
fn jeder_pruefall_passt_in_80_und_60_spalten() {
    for f in FAELLE {
        for breite in [80, 60] {
            for (name, d) in varianten(breite) {
                let aus = text(f, &d);
                let zu_lang = zu_breit(&aus, breite, false);
                assert!(
                    zu_lang.is_empty(),
                    "{} bei {breite} ({name}): {zu_lang:#?}\n{aus}",
                    f.name
                );
                let kurz = ausgabe::bericht_kurz_text(&bericht(f), &d);
                assert!(zu_breit(&kurz, breite, false).is_empty(), "{} kurz", f.name);
            }
        }
    }
}

#[test]
fn explain_und_rules_passen_in_80_und_60_spalten() {
    for breite in [80, 60, 120] {
        for englisch in [false, true] {
            let d = darstellung(breite, false, englisch, false);
            for (name, aus) in erklaerungen(&d) {
                let grenze = breite.min(80);
                let zu_lang = zu_breit(&aus, grenze, true);
                assert!(zu_lang.is_empty(), "{name} bei {breite}: {zu_lang:#?}");
            }
        }
    }
}

#[test]
fn normzitate_und_markierte_stellen_bleiben_zusammen() {
    // Bei 60 Spalten bricht der Gruppenkopf um, aber nie innerhalb eines Zitats.
    let aus = text(&FAELLE[3], &darstellung(60, false, false, false));
    for z in aus.lines() {
        let z = z.trim_end();
        assert!(!z.ends_with("Art."), "Zitat getrennt: {z}");
        assert!(!z.ends_with(" Abs."), "Zitat getrennt: {z}");
        assert!(!z.ends_with(" lit."), "Zitat getrennt: {z}");
    }
    assert!(aus.contains("Art. 30 Abs. 2 lit. c DSG"), "{aus}");
    // Die markierte Stelle «arbeitsunfähig (Diagnose F32.1)» steht auf einer Zeile.
    assert!(aus.contains("│ arbeitsunfähig (Diagnose F32.1)."), "{aus}");
    // «(Zeile 34)» bleibt zusammen.
    let lang = text(&FAELLE[7], &OHNE_FARBE);
    assert!(lang.contains("(Zeile 34)"), "{lang}");
}

// --- A4: ASCII --------------------------------------------------------------------------

#[test]
fn ascii_ausgabe_hat_nur_ascii_bytes() {
    for f in FAELLE {
        for breite in [80, 60] {
            for englisch in [false, true] {
                let d = darstellung(breite, false, englisch, true);
                let aus = text(f, &d);
                let fremd: Vec<char> = aus.chars().filter(|c| !c.is_ascii()).collect();
                assert!(fremd.is_empty(), "{}: {fremd:?}\n{aus}", f.name);
            }
        }
    }
    for englisch in [false, true] {
        let d = darstellung(80, false, englisch, true);
        for (name, aus) in erklaerungen(&d) {
            let fremd: BTreeSet<char> = aus.chars().filter(|c| !c.is_ascii()).collect();
            assert!(fremd.is_empty(), "{name}: {fremd:?}");
        }
    }
}

#[test]
fn ascii_umschrift_im_prompt_und_im_rahmen() {
    let aus = text(&FAELLE[3], &darstellung(80, false, false, true));
    assert!(aus.starts_with("Lokal geprueft - Rolle: nicht angegeben - Ziel: Cloud\n"));
    // «Zuerich» ist länger als «Zürich»: Die Zeile bricht vor der Adresse um, die als
    // markierte Stelle zusammenbleibt.
    assert!(
        aus.contains(concat!(
            "   | Unser Mitarbeiter Hans Muster, geb. 12.03.1985,\n",
            "   |                   G^^^^^^^^^^       G^^^^^^^^^\n",
            "   | Seestrasse 12, 8002 Zuerich,\n",
            "   | G^^^^^^^^^^^^^^^^^^^^^^^^^^\n",
        )),
        "{aus}"
    );
    assert!(aus.contains("\"nenne passende BGE\""), "{aus}");
    assert!(
        aus.contains("     O ORANGE - G GELB - H HINWEIS\n"),
        "{aus}"
    );
}

// --- A1: Kopfzeile ----------------------------------------------------------------------

#[test]
fn kopfzeile_nennt_lage_rolle_und_ziel() {
    let erste = |b: &Bericht, d: &Darstellung| -> String {
        ausgabe::bericht_text(b, d)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    };
    let b = pruefe(F03_GELB, None, Ziel::Cloud);
    assert_eq!(
        erste(&b, &OHNE_FARBE),
        "Lokal geprüft · Rolle: nicht angegeben · Ziel: Cloud"
    );
    assert_eq!(
        erste(&b, &ENGLISCH),
        "Checked locally · Role: not given · Target: cloud"
    );
    let b = pruefe(F03_GELB, Some(Rolle::Lawyer), Ziel::Cloud);
    assert_eq!(
        erste(&b, &OHNE_FARBE),
        "Lokal geprüft · Rolle: Anwältin/Anwalt (--role) · Ziel: Cloud"
    );
    let mut b = pruefe(F03_GELB, Some(Rolle::Lawyer), Ziel::Cloud);
    b.rolle_quelle = RolleQuelle::Konfiguration;
    assert_eq!(
        erste(&b, &OHNE_FARBE),
        "Lokal geprüft · Rolle: Anwältin/Anwalt (Konfiguration) · Ziel: Cloud"
    );
    let b = pruefe(F03_GELB, Some(Rolle::None), Ziel::Cloud);
    assert_eq!(
        erste(&b, &OHNE_FARBE),
        "Lokal geprüft · Rolle: keine (--role none) · Ziel: Cloud"
    );
    let b = pruefe(F03_GELB, None, Ziel::Local);
    assert_eq!(
        erste(&b, &OHNE_FARBE),
        "Lokal geprüft · Rolle: nicht angegeben · Ziel: lokales Modell"
    );
    // Aus dem Text erschlossen (Rechtstext A1): Das Wort steht ganz; passt die Zeile nicht
    // in 70 Zeichen, beginnt «Ziel» eine zweite Zeile.
    let aus = ausgabe::bericht_text(&pruefe(ANWAELTIN, None, Ziel::Cloud), &OHNE_FARBE);
    assert!(
        aus.starts_with(
            "Lokal geprüft · Rolle: Anwältin/Anwalt, aus «Als Anwältin habe ich»\nZiel: Cloud\n\n"
        ),
        "{aus}"
    );
    let aus = ausgabe::bericht_text(&pruefe(ANWAELTIN, None, Ziel::Cloud), &ENGLISCH);
    assert!(
        aus.starts_with(
            "Checked locally · Role: lawyer, from «Als Anwältin habe ich»\nTarget: cloud\n\n"
        ),
        "{aus}"
    );
    let aus = ausgabe::bericht_text(&pruefe(MANDANTIN, None, Ziel::Cloud), &OHNE_FARBE);
    assert!(
        aus.starts_with("Lokal geprüft · Rolle: Beruf offen, aus «Meine Mandantin»\nZiel: Cloud\n"),
        "{aus}"
    );
    // Genau 70 Zeichen passen auf eine Zeile (Rechtstext A1 «Beruf offen»).
    let klientin = "Meine Klientin Erika Beispiel will das Erbe ausschlagen.";
    let aus = ausgabe::bericht_text(&pruefe(klientin, None, Ziel::Cloud), &OHNE_FARBE);
    let erste = aus.lines().next().unwrap_or("");
    assert_eq!(
        erste,
        "Lokal geprüft · Rolle: Beruf offen, aus «Meine Klientin» · Ziel: Cloud"
    );
    assert_eq!(erste.chars().count(), 70);
    let aus = ausgabe::bericht_text(&pruefe(klientin, None, Ziel::Cloud), &ENGLISCH);
    assert!(
        aus.starts_with(
            "Checked locally · Role: profession open, from «Meine Klientin»\nTarget: cloud\n"
        ),
        "{aus}"
    );
    // Nur wenn schon «Lokal geprüft · Rolle: …» überläuft, wird das Wort nach 20 Zeichen
    // gekürzt.
    let lang = "Als Rechtsanwältin habe ich Hans Muster beraten.";
    let aus = ausgabe::bericht_text(&pruefe(lang, None, Ziel::Cloud), &OHNE_FARBE);
    assert!(
        aus.starts_with(
            "Lokal geprüft · Rolle: Anwältin/Anwalt, aus «Als Rechtsanwältin …»\nZiel: Cloud\n\n"
        ),
        "{aus}"
    );
    assert!(
        aus.contains("«Als Rechtsanwältin habe ich»"),
        "ganz in der Gruppe: {aus}"
    );
    // Ein weicher Umbruch im auslösenden Wort steht als Leerzeichen da, in der Kopfzeile
    // und in der Gruppe (Reste Runde 1, w4), nicht als «Meine …».
    let aus = ausgabe::bericht_text(
        &pruefe(&MANDANTIN.replacen(' ', "\n", 1), None, Ziel::Cloud),
        &OHNE_FARBE,
    );
    assert!(
        aus.starts_with("Lokal geprüft · Rolle: Beruf offen, aus «Meine Mandantin»\n"),
        "{aus}"
    );
    assert!(
        aus.contains("«Meine Mandantin» (eigene Klientschaft)"),
        "{aus}"
    );
    assert!(!aus.contains("Meine …"), "{aus}");
    for f in FAELLE {
        let aus = text(f, &OHNE_FARBE);
        assert!(
            !aus.contains("nichts hat den Rechner verlassen"),
            "{}",
            f.name
        );
        assert!(
            !aus.starts_with('\n'),
            "{}: keine Leerzeile am Anfang",
            f.name
        );
    }
}

// --- A1, A3, A5: Zeilenansicht ----------------------------------------------------------

#[test]
fn markierung_mit_initiale_und_einer_legende() {
    let aus = text(&FAELLE[3], &OHNE_FARBE);
    let z = zeilen(&aus);
    assert_eq!(
        z[2],
        "   │ Unser Mitarbeiter Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich,"
    );
    assert_eq!(
        z[3],
        "   │                   G^^^^^^^^^^       G^^^^^^^^^  G^^^^^^^^^^^^^^^^^^^^^^^^^"
    );
    assert_eq!(z[6], "   │ arbeitsunfähig (Diagnose F32.1).");
    assert_eq!(z[7], "   │ O^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^");
    assert_eq!(
        aus.matches("     O ORANGE · G GELB · H HINWEIS\n").count(),
        1,
        "{aus}"
    );
    let en = text(&FAELLE[3], &ENGLISCH);
    assert!(en.contains("     O ORANGE · Y YELLOW · N NOTE\n"), "{en}");
    assert!(en.contains("   │                   Y^^^^^^^^^^"), "{en}");
    // Die Legende nennt nur Stufen, die vorkommen.
    let gelb = text(&FAELLE[2], &OHNE_FARBE);
    assert!(gelb.contains("\n     G GELB\n"), "{gelb}");
    // Mit Farbe: Textmarker statt Markierungszeile und Legende.
    let farbig = text(&FAELLE[3], &MIT_FARBE);
    assert!(!ohne_escapes(&farbig).contains('^'), "{farbig}");
    assert!(
        !ohne_escapes(&farbig).contains("O ORANGE · G GELB"),
        "{farbig}"
    );
    assert!(
        farbig.contains(&format!("{}Hans Muster", ausgabe::stil(Stufe::Gelb))),
        "{farbig}"
    );
}

#[test]
fn lange_prompts_mit_nummern_und_luecken() {
    let aus = text(&FAELLE[7], &OHNE_FARBE);
    for erwartet in [
        "  … │ 7 Zeilen ohne Fund\n",
        "  8 │ Unsere Klientin Erika Beispiel bestreitet die geltend gemachte Forderung.\n",
        "  … │ 25 Zeilen ohne Fund\n",
        " 34 │ Eine Zahlung erfolgt nach Abnahme auf das Konto\n",
        "  … │ 10 Zeilen ohne Fund\n",
        " 45 │ Nenne zum Schluss passende Urteile des Bundesgerichts zur Mängelrüge.\n",
        "Erika Beispiel (Zeile 8)",
        "(Zeile 45)",
    ] {
        assert!(aus.contains(erwartet), "fehlt {erwartet:?}\n{aus}");
    }
    let en = text(&FAELLE[7], &ENGLISCH);
    assert!(en.contains("  … │ 7 lines without findings\n"), "{en}");
    assert!(en.contains("(line 8)"), "{en}");
    // Bis 10 Zeilen keine Nummern.
    let kurz = text(&FAELLE[3], &OHNE_FARBE);
    assert!(!kurz.contains(" 1 │"), "{kurz}");
    assert!(!kurz.contains("(Zeile"), "{kurz}");
}

#[test]
fn steuerzeichen_im_prompt_steuern_das_terminal_nicht() {
    let b = pruefe(
        "Hans Muster, Seestrasse 12, 8002 Zürich \u{1B}[2J\u{202E}gnuthca",
        None,
        Ziel::Cloud,
    );
    let aus = ausgabe::bericht_text(&b, &OHNE_FARBE);
    assert!(!aus.contains('\u{1B}'), "{aus}");
    assert!(!aus.contains('\u{202E}'), "{aus}");
    assert!(aus.contains('\u{FFFD}'), "{aus}");
}

// --- A1, A5: Gruppen --------------------------------------------------------------------

#[test]
fn gruppen_nach_stufe_mit_arten_in_worten() {
    let aus = text(&FAELLE[3], &OHNE_FARBE);
    let gruppen = concat!(
        "ORANGE    Besonders schützenswerte Personendaten, Art. 5 lit. c DSG\n",
        "          Bekanntgabe an Dritte durch Private: Art. 30 Abs. 2 lit. c DSG\n",
        "          Ziff. 2       arbeitsunfähig (Diagnose F32.1)\n",
        "\n",
        "GELB      Personendaten, Art. 5 lit. a DSG\n",
        "          Name          Hans Muster\n",
        "          Geburtsdatum  12.03.1985\n",
        "          Adresse       Seestrasse 12, 8002 Zürich\n",
        "          AHV-Nummer    756.1234.5678.97 (Prüfziffer gültig)\n",
        "\n",
        "HINWEIS   Rechtsfolge nicht beurteilt\n",
        "          Fundstellen   «nenne passende BGE»\n",
        "                        Sprachmodelle erfinden Urteile. Jede Fundstelle prüfen.\n",
    );
    assert!(aus.contains(gruppen), "{aus}");
    // Regelkennungen nur in der Zeile «Mehr».
    let vor_mehr = aus.split("\nMehr ").next().unwrap_or("");
    for id in ["art5c-2", "fundstellen", "geburtsdatum", " ahv "] {
        assert!(!vor_mehr.contains(id), "{id} vor «Mehr»:\n{aus}");
    }
    let en = text(&FAELLE[3], &ENGLISCH);
    assert!(en.contains("          Date of birth 12.03.1985\n"), "{en}");
    assert!(en.contains("          AHV number    756.1234.5678.97 (check digit valid)\n"));
}

#[test]
fn englisch_bleibt_das_zitat_amtlich_deutsch() {
    // technik-7: der Zusatz der Regel vermerk zitiert wie alle Normen deutsch.
    let en = text(&FAELLE[9], &ENGLISCH);
    assert!(en.contains("«VERTRAULICH» (Art. 13 Abs. 2 ISG)"), "{en}");
    for f in FAELLE {
        assert!(!text(f, &ENGLISCH).contains("para."), "{}", f.name);
    }
}

#[test]
fn wert_und_zusatz_bleiben_zusammen() {
    // Bild C (Prüffall f07): «078 765 43 21» stand am Zeilenende, «(Bereich Mobilnetz)»
    // am Anfang der nächsten Zeile.
    for breite in [80, 60] {
        let aus = text(&FAELLE[6], &darstellung(breite, false, false, false));
        for paar in [
            "079 123 45 67 (Bereich Mobilnetz)",
            "078 765 43 21 (Bereich Mobilnetz)",
            "031 222 33 44 (Bereich Festnetz)",
        ] {
            assert!(
                aus.lines().any(|z| z.contains(paar)),
                "{breite}: {paar}\n{aus}"
            );
        }
    }
}

#[test]
fn gleiche_werte_einmal_mit_anzahl() {
    let aus = text(&FAELLE[13], &OHNE_FARBE);
    assert!(
        aus.contains("          Name          Hans Muster (2×)\n"),
        "{aus}"
    );
    let ascii = text(&FAELLE[13], &darstellung(80, false, false, true));
    assert!(ascii.contains("Hans Muster (2x)"), "{ascii}");
}

#[test]
fn rot_mit_rolle_und_besonders_schuetzenswerten_daten() {
    let aus = text(&FAELLE[10], &OHNE_FARBE);
    assert!(
        aus.contains(concat!(
            "ROT       Berufsgeheimnis, Art. 321 Ziff. 1 StGB\n",
            "          Name          Hans Muster\n",
        )),
        "{aus}"
    );
    assert!(
        aus.contains("          zugleich besonders schützenswert, Art. 5 lit. c Ziff. 2 DSG:\n"),
        "{aus}"
    );
    let behoerde = text(&FAELLE[11], &OHNE_FARBE);
    assert!(behoerde.contains("ROT       Amtsgeheimnis, Art. 320 Ziff. 1 StGB\n"));
    assert!(
        behoerde.contains("Art. 5 lit. c Ziff. 6 DSG:"),
        "{behoerde}"
    );
    assert!(behoerde.contains("Sozialhilfe"), "{behoerde}");
}

#[test]
fn beruf_offen_nennt_die_normen_und_die_wahl() {
    let aus = text(&FAELLE[5], &OHNE_FARBE);
    assert!(
        aus.contains(concat!(
            "ROT       Beruf offen: Art. 321 Ziff. 1 StGB (z. B. Anwalt, Psychologin),\n",
            "          Art. 320 Ziff. 1 StGB (Behörde), Art. 47 Abs. 1 lit. a BankG (Bank)\n",
        )),
        "{aus}"
    );
    assert!(einzeilig(&aus).contains(
        "Welche Pflicht gilt? Festlegen mit --role lawyer | psychologist | authority | bank | professional | none"
    ));
}

#[test]
fn lokales_modell_als_hinweis_mit_der_stufe_in_der_cloud() {
    let aus = text(&FAELLE[8], &OHNE_FARBE);
    for erwartet in [
        "HINWEIS   Läuft das Modell lokal: keine Bekanntgabe (Art. 5 lit. e DSG)\n",
        "          sonst ORANGE  arbeitsunfähig (Diagnose F32.1)\n",
        "          sonst GELB    Hans Muster · 12.03.1985 · Seestrasse 12, 8002 Zürich ·\n",
        "Ergebnis  HINWEIS   Läuft das Modell lokal: keine Bekanntgabe\n",
        "          6 Funde: 6 Hinweise; in der Cloud 1 orange, 4 gelb, 1 Hinweis\n",
    ] {
        assert!(aus.contains(erwartet), "fehlt {erwartet:?}\n{aus}");
    }
    assert!(
        !aus.contains("Ausland") && !aus.contains("kein Offenbaren"),
        "lokal keine Frage nach dem Ausland, ohne ROT kein Offenbaren:\n{aus}"
    );
    // Wäre ein Fund sonst ROT, nennt eine zweite Kopfzeile die Normen der Rolle wie der
    // ROT-Kopf, und das Ergebnis sagt «kein Offenbaren» (Rechtstext B3).
    let rot = text(&FAELLE[14], &OHNE_FARBE);
    for erwartet in [
        concat!(
            "HINWEIS   Läuft das Modell lokal: keine Bekanntgabe (Art. 5 lit. e DSG),\n",
            "          kein Offenbaren (Art. 321 Ziff. 1 StGB, Art. 13 Abs. 1 BGFA)\n",
            "          sonst ROT     Erika Beispiel ·\n",
        ),
        concat!(
            "Ergebnis  HINWEIS   Läuft das Modell lokal: keine Bekanntgabe, kein Offenbaren\n",
            "          2 Funde: 2 Hinweise; in der Cloud 2 rot\n",
        ),
    ] {
        assert!(rot.contains(erwartet), "fehlt {erwartet:?}\n{rot}");
    }
    let rot_en = text(&FAELLE[14], &ENGLISCH);
    for erwartet in [
        concat!(
            "NOTE      If the model runs locally: no disclosure (Art. 5 lit. e DSG),\n",
            "          no disclosure of secrets (Art. 321 Ziff. 1 StGB, Art. 13 Abs. 1 BGFA)\n",
        ),
        concat!(
            "Result    NOTE   If the model runs locally: no disclosure of data or secrets\n",
            "          2 findings: 2 notes; in the cloud 2 red\n",
        ),
    ] {
        assert!(rot_en.contains(erwartet), "fehlt {erwartet:?}\n{rot_en}");
    }
    let en = text(&FAELLE[8], &ENGLISCH);
    assert!(en.contains("          else ORANGE   "), "{en}");
    assert!(
        en.contains("Result    NOTE   If the model runs locally: no disclosure\n"),
        "{en}"
    );
}

// --- A1, A5: Ergebnisblock --------------------------------------------------------------

#[test]
fn ergebnis_zaehlt_funde_und_hinweise_getrennt() {
    let aus = text(&FAELLE[3], &OHNE_FARBE);
    assert!(
        aus.contains(concat!(
            "Ergebnis  ORANGE   Besonders schützenswerte Personendaten\n",
            "          6 Funde: 1 orange, 4 gelb, 1 Hinweis\n",
        )),
        "{aus}"
    );
    let en = text(&FAELLE[3], &ENGLISCH);
    assert!(
        en.contains(concat!(
            "Result    ORANGE   Sensitive personal data\n",
            "          6 findings: 1 orange, 4 yellow, 1 note\n",
        )),
        "{en}"
    );
    // Rechtstext A4 «HINWEIS»: die Zahl in derselben Zeile, solange sie 70 Zeichen nicht
    // überschreitet.
    let hinweis = text(&FAELLE[1], &OHNE_FARBE);
    assert!(
        hinweis.contains(
            "Ergebnis  HINWEIS   Hängt von Tatsachen ab, die promptscan nicht kennt · 1 Fund\n"
        ),
        "{hinweis}"
    );
    let hinweis_en = text(&FAELLE[1], &ENGLISCH);
    assert!(
        hinweis_en
            .contains("Result    NOTE   Depends on facts promptscan does not know · 1 finding\n"),
        "{hinweis_en}"
    );
    // Nur GELB: Name und Adresse (FAELLE[13] ist seit der Wortliste art5c-2 «krank» ORANGE).
    let gelb = text(&FAELLE[2], &OHNE_FARBE);
    assert!(
        gelb.contains("Ergebnis  GELB   Personendaten · 2 Funde\n"),
        "{gelb}"
    );
    let rot = text(&FAELLE[4], &OHNE_FARBE);
    assert!(
        rot.contains("Ergebnis  ROT   Strafbewehrte Geheimhaltungspflicht · 2 Funde\n"),
        "{rot}"
    );
}

#[test]
fn klaeren_beachten_und_mehr() {
    let aus = text(&FAELLE[3], &OHNE_FARBE);
    let ab = aus.find("Ergebnis  ").expect("Ergebnisblock");
    let block = &aus[ab..];
    assert!(
        block.contains(concat!(
            "Klären    Braucht die KI diese Angaben? Nutzt der Anbieter sie nur in Ihrem\n",
            "          Auftrag oder auch für eigene Zwecke, etwa zum Training (Art. 9 DSG)?\n",
            "          Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)?\n",
            "          Dann --role angeben (Rollen: promptscan explain rot).\n",
            "          Jede Fundstelle in der Antwort an der Quelle prüfen.\n",
            "          Gehen die Daten ins Ausland? Dann gelten Art. 16 f. DSG, Anhang 1 DSV.\n",
            "Mehr      promptscan explain orange · promptscan explain art5c-2 ·\n",
            "          promptscan explain fundstellen\n",
        )),
        "{block}"
    );
    assert!(aus.ends_with("promptscan explain fundstellen\n"), "{aus}");
    // Englisch nach Rechtstext A5 «ohne Rolle (E10)» und A6.
    let en = text(&FAELLE[3], &ENGLISCH);
    assert!(
        en.contains(concat!(
            "          Are you bound by a duty of confidentiality (profession, office, bank)?\n",
            "          Then set --role (roles: promptscan explain red).\n",
        )),
        "{en}"
    );
    assert!(
        en.contains("Does the data go abroad? Then Art. 16 f. DSG and Anhang 1 DSV apply.\n"),
        "{en}"
    );
    // Behörde: Rechtstext A5 «E3» unter «Beachten», Ausland ohne Art. 61 DSG (A6), keine
    // Frage nach der Geheimhaltungspflicht.
    let behoerde_text = text(&FAELLE[11], &OHNE_FARBE);
    let behoerde = einzeilig(&behoerde_text);
    assert!(behoerde.contains(
        "Beachten Für kantonale und kommunale Organe gilt kantonales Datenschutzrecht, für Bundesorgane das DSG. In Gerichtsverfahren und Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 DSG)."
    ), "{behoerde}");
    assert!(
        behoerde.contains("Gehen die Daten ins Ausland? Für Bundesorgane: Art. 16 f. DSG."),
        "{behoerde}"
    );
    assert!(!behoerde.contains("Gilt für Sie eine Geheimhaltungspflicht"));
    // Nur Hinweise: keine Frage nach dem Ausland, keine nach der Geheimhaltungspflicht (E10).
    let hinweis = text(&FAELLE[1], &OHNE_FARBE);
    assert!(!hinweis.contains("Ausland"), "{hinweis}");
    assert!(!hinweis.contains("Geheimhaltungspflicht"), "{hinweis}");
    // Mit Rolle keine Frage nach der Geheimhaltungspflicht und keine alte Zeile von w1.
    let rot = text(&FAELLE[4], &OHNE_FARBE);
    assert!(
        !rot.contains("Gilt für Sie eine Geheimhaltungspflicht"),
        "{rot}"
    );
    for f in FAELLE {
        assert!(
            !text(f, &OHNE_FARBE).contains("Ohne Berufsrolle geprüft"),
            "{}",
            f.name
        );
    }
}

#[test]
fn keine_funde() {
    let aus = text(&FAELLE[0], &OHNE_FARBE);
    assert!(
        aus.ends_with(concat!(
            "Ergebnis  Keine Funde in den geprüften Mustern\n",
            "Beachten  Ob der Zusammenhang eine Person erkennbar macht und ob der Text\n",
            "          Geschäftsgeheimnisse enthält, prüft promptscan nicht.\n",
            "          Was geprüft wird: promptscan rules\n",
        )),
        "{aus}"
    );
    // Rechtstext A7 englisch.
    let en = text(&FAELLE[0], &ENGLISCH);
    assert!(
        en.ends_with(concat!(
            "Result    No findings in the patterns checked\n",
            "Caveat    promptscan does not check whether the context makes a person\n",
            "          identifiable or whether the text contains trade secrets.\n",
            "          What is checked: promptscan rules\n",
        )),
        "{en}"
    );
    let farbig = text(&FAELLE[0], &MIT_FARBE);
    assert!(
        farbig.contains("Keine Funde in den geprüften Mustern"),
        "{farbig}"
    );
    assert!(farbig.contains("\u{1B}[1m"), "fett: {farbig}");
}

#[test]
fn kurzbericht_ist_der_ergebnisblock() {
    for f in FAELLE {
        for d in [OHNE_FARBE, MIT_FARBE, ENGLISCH] {
            let b = bericht(f);
            let voll = ausgabe::bericht_text(&b, &d);
            let kurz = ausgabe::bericht_kurz_text(&b, &d);
            assert!(voll.ends_with(&kurz), "{}:\n{kurz}", f.name);
            let erste = ohne_escapes(&kurz);
            assert!(
                erste.starts_with("Ergebnis  ") || erste.starts_with("Result    "),
                "{}: {erste}",
                f.name
            );
            assert!(!erste.contains('│'), "{}", f.name);
        }
    }
}

// --- A3: Farben -------------------------------------------------------------------------

#[test]
fn plaketten_in_festen_farbpaaren() {
    let plakette = |s: Stufe, wort: &str, nur16: bool| {
        format!(
            "{} {wort} {:#}",
            ausgabe::stil_mit(s, nur16),
            ausgabe::stil_mit(s, nur16)
        )
    };
    let aus = text(&FAELLE[3], &MIT_FARBE);
    for (s, wort) in [
        (Stufe::Orange, "ORANGE"),
        (Stufe::Gelb, "GELB"),
        (Stufe::Hinweis, "HINWEIS"),
    ] {
        assert!(aus.contains(&plakette(s, wort, false)), "{wort}: {aus:?}");
    }
    assert!(aus.contains("\u{1B}[48;5;208m"), "Grund 208");
    assert!(aus.contains("\u{1B}[48;5;220m"), "Grund 220");
    assert!(aus.contains("\u{1B}[48;5;117m"), "Grund 117");
    let rot = text(&FAELLE[4], &MIT_FARBE);
    assert!(rot.contains(&plakette(Stufe::Rot, "ROT", false)), "{rot:?}");
    assert!(rot.contains("\u{1B}[38;5;231m\u{1B}[48;5;160m"), "{rot:?}");
    let d16 = Darstellung {
        nur_16_farben: true,
        ..MIT_FARBE
    };
    let aus16 = text(&FAELLE[3], &d16);
    assert!(
        aus16.contains(&plakette(Stufe::Orange, "ORANGE", true)),
        "{aus16:?}"
    );
    assert!(
        !aus16.contains("38;5;") && !aus16.contains("48;5;"),
        "{aus16:?}"
    );
    // Kein Grün, kein Grau, kein dim.
    for f in FAELLE {
        for d in [MIT_FARBE, d16] {
            let aus = text(f, &d);
            for verboten in [
                "\u{1B}[2m",
                "\u{1B}[32m",
                "\u{1B}[92m",
                "\u{1B}[90m",
                "\u{1B}[37m",
            ] {
                assert!(!aus.contains(verboten), "{}: {verboten:?}", f.name);
            }
        }
    }
    // Ohne Farbe keine Escape-Sequenz.
    for f in FAELLE {
        assert!(!text(f, &OHNE_FARBE).contains('\u{1B}'), "{}", f.name);
    }
}

// --- A8: JSON ---------------------------------------------------------------------------

#[test]
fn json_schema_2_und_herkunft_der_rolle() {
    assert_eq!(ausgabe::SCHEMA_VERSION, 2);
    let json =
        |b: &Bericht| -> Value { serde_json::from_str(&ausgabe::bericht_json(b)).expect("JSON") };
    let v = json(&pruefe(F04_ORANGE, None, Ziel::Cloud));
    assert_eq!(v["schema_version"], 2);
    assert_eq!(v["optionen"]["rolle_quelle"], "keine");
    assert_eq!(v["optionen"]["rolle"], Value::Null);
    let v = json(&pruefe(F04_ORANGE, Some(Rolle::Lawyer), Ziel::Cloud));
    assert_eq!(v["optionen"]["rolle_quelle"], "option");
    assert_eq!(v["optionen"]["rolle"], "lawyer");
    let mut b = pruefe(F04_ORANGE, Some(Rolle::FinancialInstitution), Ziel::Cloud);
    b.rolle_quelle = RolleQuelle::Konfiguration;
    let v = json(&b);
    assert_eq!(v["optionen"]["rolle_quelle"], "konfiguration");
    assert_eq!(v["optionen"]["rolle"], "financial-institution");
    let kurz: Value = serde_json::from_str(&ausgabe::bericht_kurz_json(&pruefe(
        F04_ORANGE,
        None,
        Ziel::Cloud,
    )))
    .expect("JSON");
    assert_eq!(kurz["schema_version"], 2);
    for text in [
        ausgabe::regeln_json(),
        ausgabe::stufen_erklaerung_json(Stufe::Rot),
        ausgabe::erklaerung_json(&katalog::finde("ahv").expect("ahv")),
    ] {
        let v: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(v["schema_version"], 2);
    }
    // Die Doku beschreibt die Änderungen gegenüber Version 1.
    assert!(
        DOKU.contains("JSON-Schema-Version 2"),
        "Kopf von AUSGABE.md"
    );
    assert!(DOKU.contains("`optionen.rolle_quelle`"), "AUSGABE.md");
}

// --- A7: explain und rules --------------------------------------------------------------

#[test]
fn explain_einer_regel_mit_der_stufe_zuoberst() {
    let ahv = katalog::finde("ahv").expect("ahv");
    let aus = ausgabe::erklaerung_text(&ahv, &OHNE_FARBE);
    assert!(
        aus.starts_with(concat!(
            "ahv · AHV-Nummer\n",
            "\n",
            "Stufe     GELB: Personendaten (Art. 5 lit. a DSG). ROT mit\n",
            "          Geheimhaltungspflicht, HINWEIS mit --target local.\n",
            "Prüfung   Klasse A: formal prüfbar\n",
        )),
        "{aus}"
    );
    assert!(aus.contains(concat!(
        "\nRechtlicher Bezug (amtlicher Wortlaut)\n",
        "  Art. 5 lit. a DSG — DSG, SR 235.1, Fassung 2025-07-07\n",
    )));
    assert!(aus.contains("\nGrundlage der Erkennung (amtlicher Wortlaut)\n  Art. 133 AHVV — "));
    assert!(
        aus.ends_with(concat!(
            "Was promptscan nicht entscheidet\n",
            "  - Ob eine Persönlichkeitsverletzung vorliegt (Art. 30 DSG) und ob sie\n",
            "    gerechtfertigt ist (Art. 31 Abs. 1 DSG).\n",
            "  - Ob eine Ausnahme greift, etwa eine Einwilligung oder Bewilligung.\n",
            "  - Ob die Norm der Rolle auch den KI-Anbieter erfasst, etwa als Hilfsperson\n",
            "    oder Beauftragten.\n",
            "  - Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist (Art. 9 DSG).\n",
            "  - Ob Daten ins Ausland gehen und wohin (Art. 16 f. DSG).\n",
            "  - Welches Datenschutzrecht für Sie gilt (Art. 2 DSG).\n",
            "  - Ob eine Angabe überhaupt geheim ist.\n",
            "  - Geschäfts- und Fabrikationsgeheimnisse und vertragliche Schweigepflichten\n",
            "    (Art. 162 StGB; für Arbeitnehmende Art. 321a Abs. 4 OR).\n",
            "  Wortlaut und Literatur: promptscan explain gelb, promptscan explain rot\n",
        )),
        "{aus}"
    );
    for verboten in ["Bedeutet", "Bedingung: Dritter", "hängt am Vertrag"] {
        assert!(!aus.contains(verboten), "{verboten}");
    }
    // Jede Regel: Stufe zuoberst, der kurze Abschnitt und der Verweis auf die Stufe.
    for r in katalog::alle() {
        for (d, stufe, verweis) in [
            (
                OHNE_FARBE,
                "Stufe     ",
                "Wortlaut und Literatur: promptscan explain ",
            ),
            (
                ENGLISCH,
                "Level     ",
                "Wording and literature: promptscan explain ",
            ),
        ] {
            let aus = ausgabe::erklaerung_text(&r, &d);
            let z = zeilen(&aus);
            // Der Kopf «id · Titel» kann zwei Zeilen brauchen; nach der ersten Leerzeile
            // kommt die Stufe.
            let leer = z.iter().position(|l| l.is_empty()).expect("Leerzeile");
            assert!(leer <= 2, "{}: Kopf zu lang", r.id);
            assert!(z[leer + 1].starts_with(stufe), "{}: {}", r.id, z[leer + 1]);
            assert!(aus.contains(verweis), "{}", r.id);
        }
    }
    // Jede Regel zeigt alle Punkte kurz (B2: «jede Regel»), weil ein Fund mit --role ROT
    // werden kann; der Verweis nennt die Stufe der Regel und ROT.
    let kontext =
        ausgabe::erklaerung_text(&katalog::finde("kontext").expect("kontext"), &OHNE_FARBE);
    assert!(kontext.contains("  - Ob eine Angabe überhaupt geheim ist.\n"));
    assert!(kontext.ends_with("  Wortlaut und Literatur: promptscan explain hinweis\n"));
    let art5c = ausgabe::erklaerung_text(&katalog::finde("art5c-2").expect("art5c-2"), &OHNE_FARBE);
    assert!(art5c.ends_with(
        "  Wortlaut und Literatur: promptscan explain orange, promptscan explain rot\n"
    ));
    // Der volle Text steht nur in explain <STUFE>.
    assert!(!aus.contains("ein überwiegendes privates oder öffentliches Interesse"));
}

#[test]
fn explain_rot_mit_grund_rollen_und_vollen_punkten() {
    let aus = ausgabe::stufen_erklaerung_text(Stufe::Rot, &OHNE_FARBE);
    assert!(
        aus.starts_with(concat!(
            "ROT       Strafbewehrte Geheimhaltungspflicht\n",
            "\n",
            "Grund     Je nach Rolle ist das Offenbaren eines Geheimnisses strafbar (Normen\n",
        )),
        "{aus}"
    );
    assert!(aus.contains(concat!(
        "\nRollen (--role)\n",
        "  none                  keine · keine Geheimhaltungspflicht\n",
        "                        kein ROT, auch nicht aus dem Text\n",
    )));
    for r in Rolle::ALLE.into_iter().filter(|r| *r != Rolle::None) {
        let zeile = format!(
            "  {:<22}{} · {}\n",
            r.name(),
            r.anzeige(false),
            stufen::geheimnis_name(r, false)
        );
        assert!(aus.contains(&zeile), "fehlt {zeile:?}");
        let normen: Vec<&str> = stufen::normen_fuer(r).iter().map(|n| n.zitat).collect();
        assert!(
            aus.contains(&format!("{}{}\n", " ".repeat(24), normen.join(", "))),
            "{}",
            r.name()
        );
    }
    let flach = einzeilig(&aus);
    for erwartet in [
        "Hilfspersonen wählen die Rolle der Person, für die sie arbeiten",
        "Was promptscan nicht entscheidet - Ob eine Ausnahme greift: nach Art. 321 Ziff. 2 StGB",
        "Literatur zur Anwaltschaft, noch nicht gelesen: Chappuis/Alberini, Secret professionnel de l’avocat et solutions cloud, Anwaltsrevue 8/2017, 337 ff. Schwarzenegger/Thouvenin/Stiller/George, Nutzung von Cloud-Diensten durch Anwältinnen und Anwälte, Anwaltsrevue 1/2019, 25 ff.",
        "in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 Abs. 3 DSG)",
        "- Ob eine Angabe überhaupt geheim ist.",
        "Busse bis zu 250 000 Franken bestraft (Art. 61 lit. b DSG)",
    ] {
        assert!(flach.contains(erwartet), "fehlt {erwartet:?}");
    }
    // Der Betrag bricht nicht um.
    assert!(aus.contains("250\u{A0}000 Franken"), "{aus}");
    // Jede Norm steht einmal im Wortlaut.
    for n in [
        "Art. 321 Ziff. 1 StGB — ",
        "Art. 9 Abs. 1 lit. b DSG — ",
        "Art. 2 Abs. 3 DSG — ",
    ] {
        assert_eq!(aus.matches(n).count(), 1, "{n}");
    }
    let en = ausgabe::stufen_erklaerung_text(Stufe::Rot, &ENGLISCH);
    assert!(
        en.starts_with("RED       Duty of confidentiality backed by criminal law\n\nReason    "),
        "{en}"
    );
    assert!(
        en.contains(
            "\nRoles (--role)\n  none                  none · no duty of confidentiality\n"
        )
    );
}

#[test]
fn explain_gelb_orange_hinweis() {
    let gelb = ausgabe::stufen_erklaerung_text(Stufe::Gelb, &OHNE_FARBE);
    assert!(
        gelb.starts_with("GELB      Personendaten\n\nGrund     Private dürfen"),
        "{gelb}"
    );
    let orange = ausgabe::stufen_erklaerung_text(Stufe::Orange, &OHNE_FARBE);
    assert!(
        orange.starts_with("ORANGE    Besonders schützenswerte Personendaten\n\nGrund     "),
        "{orange}"
    );
    for aus in [&gelb, &orange] {
        let flach = einzeilig(aus);
        for punkt in [
            "- Ob eine Persönlichkeitsverletzung vorliegt (Art. 30 DSG) und ob sie gerechtfertigt ist: durch Einwilligung",
            "- Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist.",
            "- Ob Daten ins Ausland gehen und wohin.",
            "- Welches Datenschutzrecht für Sie gilt.",
            "- Geschäfts- und Fabrikationsgeheimnisse",
            "Arbeitgeber dürfen Daten über Arbeitnehmende nur bearbeiten",
        ] {
            assert!(flach.contains(punkt), "fehlt {punkt}");
        }
        assert!(!flach.contains("Hilfsperson ist"), "nur bei ROT");
        assert!(!flach.contains("Bedingung: Dritter"));
        assert!(!flach.contains("Warum diese Stufe"));
    }
    let hinweis = ausgabe::stufen_erklaerung_text(Stufe::Hinweis, &OHNE_FARBE);
    assert!(
        hinweis.starts_with("HINWEIS   Rechtsfolge nicht beurteilt\n\nGrund     Die Bewertung")
    );
    let flach = einzeilig(&hinweis);
    assert!(flach.contains("ohne Synchronisierung in eine Cloud"));
    // HINWEIS zeigt alle Punkte (B2), weil ein Fund mit --role ROT werden kann.
    assert!(flach.contains("Was promptscan nicht entscheidet - Ob eine Persönlichkeitsverletzung"));
    for punkt in [
        "- Ob eine Ausnahme greift: nach Art. 321 Ziff. 2 StGB",
        "- Ob die Norm der Rolle auch den KI-Anbieter erfasst:",
        "- Ob eine Angabe überhaupt geheim ist.",
        "- Geschäfts- und Fabrikationsgeheimnisse",
    ] {
        assert!(flach.contains(punkt), "fehlt {punkt}");
    }
    // Mit Farbe trägt die Plakette die Stufe.
    let farbig = ausgabe::stufen_erklaerung_text(Stufe::Gelb, &MIT_FARBE);
    assert!(farbig.starts_with(&format!(
        "{} GELB {:#}",
        ausgabe::stil(Stufe::Gelb),
        ausgabe::stil(Stufe::Gelb)
    )));
}

/// Befund w1-recht-p5: Art. 2 Abs. 3 DSG mit beiden Sätzen und die gesetzliche Grundlage
/// der Bundesorgane nur «grundsätzlich» (Ausnahmen Art. 34 Abs. 4, Art. 36 Abs. 2 DSG), bei
/// GELB, ORANGE und ROT; Satz 2 und die Ausnahmen stehen im Wortlaut darunter.
#[test]
fn geltungsbereich_mit_beiden_saetzen_von_art_2_abs_3_dsg() {
    let satz_2 = "Auf erstinstanzliche Verwaltungsverfahren sind die Bestimmungen dieses Gesetzes anwendbar.";
    assert!(normen::DSG_2_3.wortlaut.ends_with(satz_2));
    for stufe in [Stufe::Gelb, Stufe::Orange, Stufe::Rot] {
        let flach = einzeilig(&ausgabe::stufen_erklaerung_text(stufe, &OHNE_FARBE));
        for teil in [
            "In Gerichtsverfahren und in Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 Abs. 3 DSG).",
            "grundsätzlich eine gesetzliche Grundlage (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; Ausnahmen: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG)",
            satz_2,
            normen::DSG_34_4.wortlaut,
            normen::DSG_36_2.wortlaut,
        ] {
            assert!(flach.contains(teil), "{stufe:?}: «{teil}» fehlt");
        }
        assert!(!flach.contains("für jede Bearbeitung"), "{stufe:?}");
        let en = einzeilig(&ausgabe::stufen_erklaerung_text(stufe, &ENGLISCH));
        assert!(
            en.contains(
                "in first instance administrative proceedings, the DSG applies (Art. 2 Abs. 3 DSG)."
            ),
            "{stufe:?}"
        );
        assert!(
            en.contains("exceptions: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG)"),
            "{stufe:?}"
        );
        assert!(!en.contains("for every processing"), "{stufe:?}");
    }
}

#[test]
fn rules_nach_stufe_gegliedert() {
    let aus = ausgabe::regeln_text(&OHNE_FARBE);
    assert!(
        aus.starts_with(concat!(
            "Regeln nach Stufe\n",
            "\n",
            "GELB      Personendaten\n",
            "  ahv           AHV-Nummer\n",
            "                Bezug: Art. 5 lit. a DSG\n",
            "                Erkennung: Art. 133 AHVV, Art. 134quinquies Abs. 2 AHVV\n",
        )),
        "{aus}"
    );
    let stellen: Vec<usize> = [
        "\nGELB      Personendaten\n",
        "\nORANGE    Besonders schützenswerte Personendaten\n",
        "\nROT       Strafbewehrte Geheimhaltungspflicht\n",
        "\nHINWEIS   Rechtsfolge nicht beurteilt\n",
    ]
    .iter()
    .map(|k| aus.find(k).unwrap_or_else(|| panic!("fehlt {k}")))
    .collect();
    assert!(stellen.windows(2).all(|w| w[0] < w[1]), "Reihenfolge");
    for r in katalog::alle() {
        let anfang = format!("\n  {:<14}", r.id);
        assert_eq!(aus.matches(&anfang).count(), 1, "{}", r.id);
    }
    assert!(aus.contains("Ziff. 2.2.4, 2.2.6,"), "Ziffern knapp");
    assert!(
        einzeilig(&aus).ends_with(
            "Wortlaut, Quellen und Grenzen: promptscan explain <REGEL> Hilfsmittel, keine Rechtsberatung · erkennt Identifikatoren und Wörter aus Listen, nicht den Zusammenhang · Güte noch nicht gemessen"
        ),
        "{aus}"
    );
    // Rechtsfolge und Grundlage der Erkennung getrennt (recht-normen-8, B5).
    assert!(aus.contains(concat!(
        "  fundstellen   Bitte um Urteile oder Fundstellen\n",
        "                Bezug: Art. 12 lit. a BGFA\n",
        "                Erkennung: Art. 27 Abs. 1 BGG, Art. 57 Abs. 1 BGerR\n",
    )));
    for z in zeilen(&aus) {
        assert!(z.chars().count() <= 80, "zu breit: {z}");
    }
    let en = ausgabe::regeln_text(&ENGLISCH);
    assert!(
        en.starts_with("Rules by level\n\nYELLOW    Personal data\n"),
        "{en}"
    );
    assert!(einzeilig(&en).ends_with("quality not yet measured"));
    assert!(!en.contains("para."));
}

// --- A9: Beispiele in SPEC 3 und AUSGABE 1 ----------------------------------------------

/// Der erste Codeblock nach `marke`, ohne die Zäune.
fn codeblock<'a>(doku: &'a str, marke: &str) -> &'a str {
    let ab = doku.find(marke).unwrap_or_else(|| panic!("fehlt {marke}"));
    let rest = &doku[ab..];
    let start = rest.find("```").expect("Zaun");
    let rest = &rest[start..];
    let zeile = rest.find('\n').expect("Zeile") + 1;
    let rest = &rest[zeile..];
    let ende = rest.find("```").expect("Zaun");
    &rest[..ende]
}

#[test]
fn das_beispiel_in_ausgabe_md_stimmt() {
    let erwartet = ausgabe::bericht_text(&pruefe(F04_ORANGE, None, Ziel::Cloud), &OHNE_FARBE);
    assert_eq!(codeblock(DOKU, "<!-- beispiel-ohne-farbe -->"), erwartet);
}

#[test]
fn die_beispiele_in_spec_3_stimmen() {
    let eins = codeblock(SPEC, "### Beispiel 1");
    let (befehl, rest) = eins.split_once('\n').expect("Befehl");
    assert_eq!(befehl, "$ promptscan kuendigung.txt");
    assert_eq!(
        rest,
        ausgabe::bericht_text(&pruefe(F04_ORANGE, None, Ziel::Cloud), &OHNE_FARBE)
    );
    // Beispiel 2 aus einer Datei, nicht mit -t (der Prompt bliebe im Verlauf der Shell).
    let zwei = codeblock(SPEC, "### Beispiel 2");
    let (befehl, rest) = zwei.split_once('\n').expect("Befehl");
    assert_eq!(befehl, "$ promptscan --role lawyer mandantin.txt");
    assert_eq!(
        rest,
        ausgabe::bericht_text(
            &pruefe(&format!("{MANDANTIN}\n"), Some(Rolle::Lawyer), Ziel::Cloud),
            &OHNE_FARBE
        )
    );
}

// --- A11: SPEC 10.1 ---------------------------------------------------------------------

/// Jede Norm, die das Programm mit Wortlaut führt: die Konstanten in `normen.rs`, die
/// Grundlagen aller Regeln und die Normen aller Rollen.
fn alle_normen() -> Vec<&'static Norm> {
    let mut v: Vec<&'static Norm> = Vec::new();
    let mut dazu = |n: &'static Norm| {
        if !v.iter().any(|m| m.zitat == n.zitat && m.erlass == n.erlass) {
            v.push(n);
        }
    };
    for n in normen::ALLE {
        dazu(n);
    }
    for r in katalog::alle() {
        for n in r.grundlage {
            dazu(n);
        }
    }
    for r in Rolle::ALLE {
        for n in stufen::normen_fuer(r) {
            dazu(n);
        }
    }
    v
}

#[test]
fn spec_10_1_nennt_jede_norm_mit_fassung() {
    let ab = SPEC.find("### 10.1").expect("SPEC 10.1");
    let bis = SPEC[ab..].find("### 10.2").expect("SPEC 10.2") + ab;
    let abschnitt = &SPEC[ab..bis];
    let zeilen: Vec<String> = alle_normen()
        .into_iter()
        .map(|n| format!("| {} | {} | {} |", n.zitat, n.erlass, n.fassung))
        .collect();
    let fehlend: Vec<&String> = zeilen
        .iter()
        .filter(|zeile| !abschnitt.contains(zeile.as_str()))
        .collect();
    // Die erste Tabelle ist genau diese Liste: keine veraltete Zeile bleibt stehen.
    let kopf = "| Zitat | Erlass | Fassung |\n|---|---|---|\n";
    let tabelle = format!("{kopf}{}\n\n", zeilen.join("\n"));
    assert!(
        fehlend.is_empty() && abschnitt.contains(&tabelle),
        "In SPEC 10.1 fehlen {} Zeilen:\n{}\n\nErwartete Tabelle:\n{tabelle}",
        fehlend.len(),
        fehlend
            .iter()
            .map(|z| z.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Keine Norm mit Wortlaut steht zugleich unter «Nur zitiert».
    let nur = abschnitt
        .split("**Nur zitiert.**")
        .nth(1)
        .and_then(|r| r.split("**Erlasse der Wortlisten.**").next())
        .expect("Tabelle «Nur zitiert»");
    for n in alle_normen() {
        assert!(
            !nur.contains(&format!("| {} | {} |", n.zitat, n.erlass)),
            "{} steht mit Wortlaut und unter «Nur zitiert»",
            n.zitat
        );
    }
}

/// Rang (0 Artikel, 1 anderes), Artikelnummer und Fundstelle ohne «Art. ».
type Ordnung = (u8, u32, String);

/// Ordnet eine Fundstelle der Wortlisten: Artikel nach Nummer, dann Anhänge und anderes.
fn artikel_schluessel(stelle: &str) -> Ordnung {
    match stelle.strip_prefix("Art. ") {
        Some(rest) => {
            let ziffern: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (0, ziffern.parse().unwrap_or(u32::MAX), rest.to_string())
        }
        None => (1, 0, stelle.to_string()),
    }
}

/// Die Bundeserlasse, aus denen die Wortlisten schöpfen, als Tabelle für SPEC 10.1: je
/// SR-Nummer und Fassung eine Zeile mit den Fundstellen (`quelle = "SR … Art. …"` in
/// `lexika/*.toml`). Die Abkürzung des Erlasses steht da, wo eine Norm des Programms sie
/// nennt.
fn wortlisten_tabelle() -> String {
    let mut abkuerzung: BTreeMap<String, String> = BTreeMap::new();
    for n in alle_normen() {
        if let Some((kurz, sr)) = n.erlass.rsplit_once(", SR ")
            && !kurz.contains(' ')
        {
            abkuerzung.insert(sr.to_string(), kurz.to_string());
        }
    }
    // Schlüssel: die SR-Nummer als Zahlen (für die Reihenfolge) und «SR | Fassung».
    let mut karte: BTreeMap<(Vec<u32>, String), BTreeSet<Ordnung>> = BTreeMap::new();
    for datei in lexikon::dateien().filter(|d| d.ends_with(".toml")) {
        for e in &lexikon::lexikon(datei).eintraege {
            let Some((sr, stelle)) = e
                .quelle
                .strip_prefix("SR ")
                .and_then(|rest| rest.split_once(' '))
            else {
                continue;
            };
            let nummer: Vec<u32> = sr.split('.').map(|t| t.parse().unwrap_or(0)).collect();
            karte
                .entry((nummer, format!("{sr} | {}", e.fassung)))
                .or_default()
                .insert(artikel_schluessel(stelle));
        }
    }
    let mut aus =
        String::from("| Erlass | Fassung | Fundstellen der Wortlisten |\n|---|---|---|\n");
    for ((_, sr_fassung), stellen) in karte {
        let (sr, fassung) = sr_fassung.split_once(" | ").expect("Trenner");
        let erlass = match abkuerzung.get(sr) {
            Some(kurz) => format!("{kurz}, SR {sr}"),
            None => format!("SR {sr}"),
        };
        let stellen: Vec<String> = stellen
            .into_iter()
            .map(|(art, _, rest)| {
                if art == 0 {
                    format!("Art. {rest}")
                } else {
                    rest
                }
            })
            .collect();
        // Eine Fundstelle kann selbst ein Komma enthalten («Art. 1, Fussnote 5»).
        aus.push_str(&format!(
            "| {erlass} | {fassung} | {} |\n",
            stellen.join("; ")
        ));
    }
    aus
}

/// Reste Runde 1 (w1): SPEC 10.1 nennt auch die Erlasse der Wortlisten, erzeugt aus den
/// Wortlisten selbst. Ändert sich eine Wortliste, zeigt der Fehler die neue Tabelle.
#[test]
fn spec_10_1_nennt_die_erlasse_der_wortlisten() {
    let ab = SPEC.find("### 10.1").expect("SPEC 10.1");
    let bis = SPEC[ab..].find("### 10.2").expect("SPEC 10.2") + ab;
    let tabelle = wortlisten_tabelle();
    assert!(
        SPEC[ab..bis].contains(tabelle.as_str()),
        "SPEC 10.1 braucht diese Tabelle:\n{tabelle}"
    );
}
