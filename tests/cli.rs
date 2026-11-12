//! Die Befehlszeile als Ganzes (SPEC 3, docs/AUSGABE.md 8): Eingabewege, Kodierungen,
//! Exit-Codes, Farbe, JSON, Konfiguration, `rules`, `explain` und `config`, Hilfe und
//! Meldungen in beiden Sprachen.
//!
//! Die Tests unter «Mit Erkennern» prüfen die Beispiele aus SPEC 3 und die Schwellen von
//! `--fail-on` mit den eingebauten Erkennern; die übrigen brauchen keinen Fund.
//!
//! Jeder Aufruf setzt `PROMPTSCAN_CONFIG` auf eine Datei, die es nicht gibt, und entfernt
//! die Farbvariablen; so hängt kein Test an der Konfiguration oder Umgebung des Rechners.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::{PredicateBooleanExt, PredicateBoxExt};
use predicates::str::{contains, is_empty};
use promptscan::katalog;
use serde_json::{Value, json};

/// Ein Prompt ohne Personendaten, den kein Erkenner melden soll.
const NEUTRAL: &str = "Fasse den Absatz kürzer: Das Wetter ist schön.";

const KEINE_FUNDE: &str = "Ergebnis  Keine Funde in den geprüften Mustern";

/// Kopfzeile ohne Rolle und mit Ziel Cloud (AUSGABE 1).
const KOPF_OHNE_ROLLE: &str = "Lokal geprüft · Rolle: nicht angegeben · Ziel: Cloud";

/// Was unter «Keine Funde» steht (AUSGABE 1).
const BEACHTEN_OHNE_FUND: &str = "Beachten  Ob der Zusammenhang eine Person erkennbar macht und ob der Text\n          Geschäftsgeheimnisse enthält, prüft promptscan nicht.\n          Was geprüft wird: promptscan rules\n";

const BEISPIEL_1: &str = "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich,\n\
AHV 756.1234.5678.97, ist seit März zu 50 % arbeitsunfähig (Diagnose F32.1).\n\
Formuliere eine Kündigung und nenne passende BGE zum Kündigungsschutz.\n";

const BEISPIEL_2: &str =
    "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will das Erbe ausschlagen.";

/// Ein Ordner nur für einen Test, unter dem temporären Ordner von Cargo; er wird am
/// Ende des Tests gelöscht.
struct Ordner(PathBuf);

impl Ordner {
    fn neu(name: &str) -> Ordner {
        let pfad = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("promptscan-cli-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pfad);
        std::fs::create_dir_all(&pfad).expect("Testordner anlegen");
        Ordner(pfad)
    }

    fn datei(&self, name: &str, inhalt: &[u8]) -> PathBuf {
        let pfad = self.0.join(name);
        std::fs::write(&pfad, inhalt).expect("Testdatei schreiben");
        pfad
    }
}

impl Drop for Ordner {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Eine Konfigurationsdatei, die es nicht gibt.
fn keine_konfiguration() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("promptscan-cli-gibt-es-nicht")
        .join("config.toml")
}

/// Das Programm mit fester Umgebung.
fn promptscan() -> Command {
    let mut c = Command::new(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))));
    c.env("PROMPTSCAN_CONFIG", keine_konfiguration())
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("CLICOLOR");
    c
}

/// Führt aus, verlangt Exit-Code 0 und gibt die Standardausgabe zurück.
fn ausgabe(c: &mut Command) -> String {
    let bytes = c.assert().success().get_output().stdout.clone();
    String::from_utf8(bytes).expect("Ausgabe in UTF-8")
}

/// Der Bericht für einen Prompt ohne Fund.
fn ohne_fund(zeilen: &[&str]) -> String {
    let mut s = format!("{KOPF_OHNE_ROLLE}\n\n");
    for z in zeilen {
        s.push_str(&format!("   │ {z}\n"));
    }
    s.push_str(&format!("\n{KEINE_FUNDE}\n{BEACHTEN_OHNE_FUND}"));
    s
}

fn utf16(text: &str, le: bool) -> Vec<u8> {
    let mut bytes = if le {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for einheit in text.encode_utf16() {
        bytes.extend(if le {
            einheit.to_le_bytes()
        } else {
            einheit.to_be_bytes()
        });
    }
    bytes
}

fn json_von(text: &str) -> Value {
    serde_json::from_str(text).expect("gültiges JSON")
}

/// Fasst Leerraum zusammen, damit Zeilenumbrüche beim Suchen nicht stören.
fn einzeilig(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Die Meldungen zur Eingabe im Ergebnisblock auf einer Zeile; leer, wenn es keine gibt
/// (AUSGABE 1, «Meldungen zur Eingabe»). Sie stehen ohne Etikett eingerückt unter
/// «Ergebnis» und beginnen mit «Eingabe …» (englisch «Input …»).
fn eingabe_abschnitt(aus: &str) -> String {
    let mut teile = Vec::new();
    let mut drin = false;
    for z in aus.lines() {
        if let Some(rest) = z.strip_prefix("          ")
            && (rest.starts_with("Eingabe ") || rest.starts_with("Input "))
        {
            drin = true;
            teile.push(rest);
        } else if drin && z.starts_with("          ") {
            teile.push(z);
        } else {
            drin = false;
        }
    }
    einzeilig(&teile.join(" "))
}

// --- Eingabe ------------------------------------------------------------------------------

#[test]
fn text_mit_t() {
    assert_eq!(
        ausgabe(promptscan().args(["-t", NEUTRAL])),
        ohne_fund(&[NEUTRAL])
    );
    assert_eq!(
        ausgabe(promptscan().args(["--text", NEUTRAL])),
        ohne_fund(&[NEUTRAL])
    );
}

const ZWEI_ZEILEN: [&str; 2] = ["Erste Zeile.", "Zweite Zeile über den Ablauf."];

#[test]
fn datei_in_utf8_mit_und_ohne_bom() {
    let o = Ordner::neu("utf8");
    let text = format!("{}\n{}\n", ZWEI_ZEILEN[0], ZWEI_ZEILEN[1]);
    let ohne_bom = o.datei("ohne-bom.txt", text.as_bytes());
    assert_eq!(
        ausgabe(promptscan().arg(&ohne_bom)),
        ohne_fund(&ZWEI_ZEILEN)
    );
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(text.as_bytes());
    let mit_bom = o.datei("mit-bom.txt", &bytes);
    assert_eq!(ausgabe(promptscan().arg(&mit_bom)), ohne_fund(&ZWEI_ZEILEN));
}

#[test]
fn datei_in_utf16_mit_bom_und_crlf() {
    // So schreibt Windows PowerShell 5.1 mit `>`: UTF-16LE mit BOM und CRLF.
    let o = Ordner::neu("utf16");
    let text = format!("{}\r\n{}\r\n", ZWEI_ZEILEN[0], ZWEI_ZEILEN[1]);
    let le = o.datei("le.txt", &utf16(&text, true));
    assert_eq!(ausgabe(promptscan().arg(&le)), ohne_fund(&ZWEI_ZEILEN));
    let be = o.datei("be.txt", &utf16(&text, false));
    assert_eq!(ausgabe(promptscan().arg(&be)), ohne_fund(&ZWEI_ZEILEN));
}

#[test]
fn datei_in_anderer_kodierung_wird_abgewiesen() {
    // «Kürzer» in Latin-1: das ü ist das einzelne Byte 0xFC, kein gültiges UTF-8.
    let o = Ordner::neu("latin1");
    let datei = o.datei("latin1.txt", b"K\xFCrzer");
    promptscan()
        .arg(&datei)
        .assert()
        .code(2)
        .stdout(is_empty())
        .stderr(contains("weder UTF-8 noch UTF-16 mit BOM"));
}

#[test]
fn eingabe_ueber_stdin() {
    assert_eq!(
        ausgabe(promptscan().write_stdin("Zeile über stdin.\n")),
        ohne_fund(&["Zeile über stdin."])
    );
    let text = format!("{}\r\n{}\r\n", ZWEI_ZEILEN[0], ZWEI_ZEILEN[1]);
    assert_eq!(
        ausgabe(promptscan().write_stdin(utf16(&text, true))),
        ohne_fund(&ZWEI_ZEILEN)
    );
}

#[test]
fn fehlende_datei_gibt_exit_2() {
    let o = Ordner::neu("fehlt");
    promptscan()
        .arg(o.0.join("fehlt.txt"))
        .assert()
        .code(2)
        .stdout(is_empty())
        .stderr(contains("promptscan:").and(contains("fehlt.txt")));
}

// --- Bedienungsfehler ---------------------------------------------------------------------

#[test]
fn unbekannte_regel_gibt_exit_2() {
    let mut erwartet =
        contains("promptscan: Eine Regel oder Stufe «gibt-es-nicht» gibt es nicht.\n").boxed();
    for r in katalog::alle() {
        erwartet = erwartet.and(contains(r.id)).boxed();
    }
    promptscan()
        .args(["explain", "gibt-es-nicht"])
        .assert()
        .code(2)
        .stdout(is_empty())
        .stderr(erwartet);
    promptscan()
        .args(["explain", "gibt-es-nicht", "--lang", "en"])
        .assert()
        .code(2)
        .stderr(contains("There is no rule or level «gibt-es-nicht»."));
    // Ein Vorschlag bei einem Tippfehler.
    promptscan()
        .args(["explain", "ahw"])
        .assert()
        .code(2)
        .stderr(contains("Gemeint ist wohl: promptscan explain ahv"));
}

#[test]
fn falsche_optionen_geben_exit_2() {
    for args in [
        vec!["--role", "pilot", "-t", NEUTRAL],
        vec!["--target", "mond", "-t", NEUTRAL],
        vec!["--format", "xml", "-t", NEUTRAL],
        vec!["--lang", "fr", "-t", NEUTRAL],
        vec!["--fail-on", "gruen", "-t", NEUTRAL],
        vec!["--gibt-es-nicht", "-t", NEUTRAL],
        vec!["-t", NEUTRAL, "datei.txt"],
    ] {
        promptscan().args(&args).assert().code(2).stdout(is_empty());
    }
}

// --- Farbe ------------------------------------------------------------------------------

#[test]
fn ohne_farbe_keine_escape_sequenzen() {
    let regel = katalog::alle()
        .first()
        .map(|r| r.id)
        .expect("mindestens eine Regel");
    for befehl in [vec!["rules"], vec!["explain", regel]] {
        // Gegenprobe: Mit CLICOLOR_FORCE erscheinen Escape-Sequenzen (fette Überschriften),
        // auch wenn die Ausgabe in eine Pipe geht.
        let farbig = ausgabe(promptscan().args(&befehl).env("CLICOLOR_FORCE", "1"));
        assert!(
            farbig.contains("\u{1B}["),
            "{befehl:?}: keine Farbe in {farbig:?}"
        );
        // --no-color schlägt CLICOLOR_FORCE.
        let ohne = ausgabe(
            promptscan()
                .args(&befehl)
                .arg("--no-color")
                .env("CLICOLOR_FORCE", "1"),
        );
        assert!(
            !ohne.contains('\u{1B}'),
            "{befehl:?} mit --no-color: {ohne:?}"
        );
        // NO_COLOR schlägt CLICOLOR_FORCE.
        let ohne_env = ausgabe(
            promptscan()
                .args(&befehl)
                .env("CLICOLOR_FORCE", "1")
                .env("NO_COLOR", "1"),
        );
        assert!(
            !ohne_env.contains('\u{1B}'),
            "{befehl:?} mit NO_COLOR: {ohne_env:?}"
        );
        // In eine Pipe ohne Vorgabe: keine Farbe.
        let pipe = ausgabe(promptscan().args(&befehl));
        assert!(!pipe.contains('\u{1B}'), "{befehl:?} in die Pipe: {pipe:?}");
        assert_eq!(ohne, pipe);
    }
    let bericht = ausgabe(promptscan().args(["-t", NEUTRAL, "--no-color"]));
    assert!(!bericht.contains('\u{1B}'));
}

// --- JSON -------------------------------------------------------------------------------

#[test]
fn bericht_als_json() {
    let v = json_von(&ausgabe(
        promptscan()
            .args(["-t", NEUTRAL, "--format", "json"])
            .env("CLICOLOR_FORCE", "1"),
    ));
    assert_eq!(
        v["schema_version"],
        json!(promptscan::ausgabe::SCHEMA_VERSION)
    );
    assert_eq!(v["dokument"], "bericht");
    assert_eq!(v["programm"], "promptscan");
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        v["optionen"],
        json!({"rolle": null, "rolle_quelle": "keine", "ziel": "cloud"})
    );
    assert_eq!(v["lage"], json!({"herkunft": "keine"}));
    assert_eq!(v["funde"], json!([]));
    assert_eq!(v["anzahl"]["gesamt"], json!(0));
    assert_eq!(v["hoechste_stufe"], Value::Null);

    let v = json_von(&ausgabe(promptscan().args([
        "-t", NEUTRAL, "--format", "json", "--role", "lawyer", "--target", "local",
    ])));
    assert_eq!(
        v["optionen"],
        json!({"rolle": "lawyer", "rolle_quelle": "option", "ziel": "local"})
    );
    assert_eq!(
        v["lage"],
        json!({"herkunft": "angegeben", "rolle": "lawyer"})
    );
}

// --- rules und explain --------------------------------------------------------------------

#[test]
fn rules_deutsch_englisch_json() {
    let regeln = katalog::alle();
    let de = ausgabe(promptscan().arg("rules"));
    assert!(de.starts_with("Regeln nach Stufe"), "{de}");
    let en = ausgabe(promptscan().args(["rules", "--lang", "en"]));
    assert!(en.starts_with("Rules by level"), "{en}");
    for r in &regeln {
        let zeile = format!("\n  {:<14}", r.id);
        assert!(de.contains(&zeile) && en.contains(&zeile), "{} fehlt", r.id);
    }
    let v = json_von(&ausgabe(promptscan().args(["rules", "--format", "json"])));
    assert_eq!(v["dokument"], "regeln");
    assert_eq!(v["regeln"].as_array().map(Vec::len), Some(regeln.len()));
}

#[test]
fn explain_deutsch_englisch_json() {
    for r in katalog::alle() {
        let de = ausgabe(promptscan().args(["explain", r.id]));
        assert!(
            einzeilig(&de).starts_with(&einzeilig(&format!("{} · {}", r.id, r.titel.de))),
            "{de}"
        );
        assert!(de.contains("\nWas promptscan nicht entscheidet\n"), "{de}");
        let en = ausgabe(promptscan().args(["explain", r.id, "--lang", "en"]));
        assert!(en.contains("\nWhat promptscan does not decide\n"), "{en}");
        let v = json_von(&ausgabe(
            promptscan().args(["explain", r.id, "--format", "json"]),
        ));
        assert_eq!(v["dokument"], "erklaerung");
        assert_eq!(v["id"], r.id);
    }
}

#[test]
fn explain_fuer_eine_stufe() {
    let rot = ausgabe(promptscan().args(["explain", "rot"]));
    assert!(
        rot.starts_with("ROT       Strafbewehrte Geheimhaltungspflicht\n"),
        "{rot}"
    );
    assert!(rot.contains("\nRollen (--role)\n"), "{rot}");
    let gelb = ausgabe(promptscan().args(["explain", "yellow", "--lang", "en"]));
    assert!(gelb.starts_with("YELLOW    Personal data\n"), "{gelb}");
    let v = json_von(&ausgabe(
        promptscan().args(["explain", "orange", "--format", "json"]),
    ));
    assert_eq!(v["dokument"], "stufe");
    assert_eq!(v["stufe"], "orange");
}

// --- Exit-Codes und Konfiguration -------------------------------------------------------

#[test]
fn fail_on_ohne_fund_gibt_exit_0() {
    for schwelle in ["red", "orange", "yellow", "never"] {
        promptscan()
            .args(["-t", NEUTRAL, "--fail-on", schwelle])
            .assert()
            .code(0);
    }
}

#[test]
fn konfigurationsdatei() {
    let o = Ordner::neu("konfiguration");
    let englisch = o.datei("en.toml", b"lang = \"en\"\nrole = \"lawyer\"\n");
    let rules = ausgabe(
        promptscan()
            .arg("rules")
            .env("PROMPTSCAN_CONFIG", &englisch),
    );
    assert!(rules.starts_with("Rules by level"), "{rules}");
    // Die Befehlszeile geht der Datei vor.
    let rules = ausgabe(
        promptscan()
            .args(["rules", "--lang", "de"])
            .env("PROMPTSCAN_CONFIG", &englisch),
    );
    assert!(rules.starts_with("Regeln nach Stufe"), "{rules}");
    let v = json_von(&ausgabe(
        promptscan()
            .args(["-t", NEUTRAL, "--format", "json"])
            .env("PROMPTSCAN_CONFIG", &englisch),
    ));
    assert_eq!(v["optionen"]["rolle"], "lawyer");

    // Bei der Prüfung ohne --role bricht ein Fehler der Datei ab: Eine eingestellte Rolle
    // fiele sonst still weg. Die Meldung nennt Pfad, Problem und Abhilfe.
    for (name, inhalt, problem) in [
        (
            "sprache.toml",
            "lang = \"fr\"\n",
            "Die Sprache «fr» gibt es nicht.",
        ),
        (
            "rolle.toml",
            "role = \"pilot\"\n",
            "Die Rolle «pilot» gibt es nicht.",
        ),
        (
            "unbekannt.toml",
            "farbe = true\n",
            "Den Schlüssel «farbe» gibt es nicht.",
        ),
        (
            "kaputt.toml",
            "lang = \n",
            "Zeile 1 ist kein gültiges TOML: «lang =».",
        ),
        (
            "ohne-anfuehrung.toml",
            "role = lawyer\n",
            "ist kein gültiges TOML",
        ),
        (
            "zahl.toml",
            "role = 3\n",
            "gehört in gerade Anführungszeichen",
        ),
    ] {
        let datei = o.datei(name, inhalt.as_bytes());
        let (code, aus, fehler) = lauf(
            promptscan()
                .args(["-t", NEUTRAL])
                .env("PROMPTSCAN_CONFIG", &datei),
        );
        assert_eq!(code, Some(2), "{name}");
        assert!(aus.is_empty(), "{aus}");
        // Problem in der ersten Zeile, der Pfad dahinter, bei Platzmangel auf einer eigenen
        // Zeile (p7).
        assert!(
            fehler.starts_with("promptscan: Konfiguration: "),
            "{fehler}"
        );
        let flach = einzeilig(&fehler);
        assert!(flach.contains(problem), "{fehler}");
        assert!(
            flach.contains(&format!("Datei: {}", datei.display())),
            "{fehler}"
        );
        meldung(&fehler);
        schmal_ausser_pfad(&fehler, &datei);
    }
}

/// Höchstens 80 Zeichen je Zeile, ausser eine Zeile ist nur «Datei: PFAD» und der Pfad
/// allein zu lang (er steht ungebrochen, damit man ihn kopieren kann). Der Pfad darf
/// Leerzeichen enthalten (`C:\Users\Vorname Nachname\…`).
fn schmal_ausser_pfad(text: &str, pfad: &Path) {
    let pfad = pfad.display().to_string();
    for z in text.lines() {
        let n = z.chars().count();
        let nur_pfad = ["Datei: ", "File: "]
            .iter()
            .any(|a| z.trim().strip_prefix(a) == Some(pfad.as_str()));
        assert!(n <= 80 || nur_pfad, "{n} Zeichen: {z}");
    }
}

#[test]
fn version() {
    promptscan()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(format!(
            "promptscan {}",
            env!("CARGO_PKG_VERSION")
        )));
}

// --- Mit Erkennern (SPEC 3) -------------------------------------------------------------

const SPEC: &str = include_str!("../docs/SPEC.md");

/// Der erste Codeblock nach `marke`, ohne die Zäune.
fn codeblock<'a>(doku: &'a str, marke: &str) -> &'a str {
    let ab = doku.find(marke).unwrap_or_else(|| panic!("fehlt {marke}"));
    let rest = &doku[ab..];
    let rest = &rest[rest.find("```").expect("Zaun")..];
    let rest = &rest[rest.find('\n').expect("Zeile") + 1..];
    &rest[..rest.find("```").expect("Zaun")]
}

/// Das Beispiel aus SPEC 3 unter `marke`: Befehlszeile und erwartete Ausgabe.
fn spec_beispiel(marke: &str) -> (&'static str, &'static str) {
    codeblock(SPEC, marke).split_once('\n').expect("Befehl")
}

#[test]
fn spec_beispiel_1() {
    let o = Ordner::neu("beispiel1");
    let datei = o.datei("kuendigung.txt", BEISPIEL_1.as_bytes());
    let (befehl, erwartet) = spec_beispiel("### Beispiel 1");
    assert_eq!(befehl, "$ promptscan kuendigung.txt");
    // Das Programm selbst, in die Pipe ohne Farbe: genau der Block aus SPEC 3.
    let aus = ausgabe(promptscan().arg(&datei));
    assert_eq!(aus, erwartet);
    assert!(!aus.contains('\u{1B}'));
}

#[test]
fn spec_beispiel_2() {
    let o = Ordner::neu("beispiel2");
    let datei = o.datei("mandantin.txt", format!("{BEISPIEL_2}\n").as_bytes());
    let (befehl, erwartet) = spec_beispiel("### Beispiel 2");
    assert_eq!(befehl, "$ promptscan --role lawyer mandantin.txt");
    let aus = promptscan()
        .args(["--role", "lawyer"])
        .arg(&datei)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(aus).expect("UTF-8"), erwartet);
    // Mit -t derselbe Bericht.
    let t = promptscan()
        .args(["--role", "lawyer", "-t", BEISPIEL_2])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(t).expect("UTF-8"), erwartet);
}

#[test]
fn fail_on_schwellen() {
    let o = Ordner::neu("schwellen");
    let datei = o.datei("kuendigung.txt", BEISPIEL_1.as_bytes());
    // Beispiel 1: höchste Stufe ORANGE.
    let faelle: [(Option<&str>, i32); 5] = [
        (None, 0),
        (Some("red"), 0),
        (Some("orange"), 1),
        (Some("yellow"), 1),
        (Some("never"), 0),
    ];
    for (schwelle, code) in faelle {
        let mut c = promptscan();
        c.arg(&datei);
        if let Some(s) = schwelle {
            c.args(["--fail-on", s]);
        }
        c.assert().code(code);
    }
    // Beispiel 2: ROT mit --role lawyer, nur GELB mit --role none.
    let faelle: [(&str, &str, i32); 4] = [
        ("lawyer", "red", 1),
        ("lawyer", "never", 0),
        ("none", "red", 0),
        ("none", "yellow", 1),
    ];
    for (rolle, schwelle, code) in faelle {
        promptscan()
            .args(["--role", rolle, "--fail-on", schwelle, "-t", BEISPIEL_2])
            .assert()
            .code(code);
    }
}

#[test]
fn farbe_im_bericht() {
    let farbig = promptscan()
        .args(["-t", BEISPIEL_1])
        .env("CLICOLOR_FORCE", "1")
        .assert()
        .get_output()
        .stdout
        .clone();
    let farbig = String::from_utf8(farbig).expect("UTF-8");
    assert!(
        farbig.contains("48;5;208"),
        "ORANGE als Grund in 256-Farbe 208 (AUSGABE 1)"
    );
    assert!(!farbig.contains("^^^"), "mit Farbe keine Marken");
    let ohne = promptscan()
        .args(["-t", BEISPIEL_1, "--no-color"])
        .env("CLICOLOR_FORCE", "1")
        .assert()
        .get_output()
        .stdout
        .clone();
    let ohne = String::from_utf8(ohne).expect("UTF-8");
    assert!(!ohne.contains('\u{1B}'));
    assert!(ohne.contains("^^^"));
}

#[test]
fn json_mit_funden() {
    let aus = promptscan()
        .args(["--role", "lawyer", "--format", "json", "-t", BEISPIEL_2])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v = json_von(&String::from_utf8(aus).expect("UTF-8"));
    assert_eq!(v["hoechste_stufe"], "rot");
    assert_eq!(v["funde"][0]["regel"], "name");
    assert_eq!(
        v["funde"][0]["normen"],
        json!(["Art. 321 Ziff. 1 StGB", "Art. 13 Abs. 1 BGFA"])
    );
    assert_eq!(v["funde"][1]["regel"], "iban");
}

// --- Bedienung: Befehle, Optionen, Meldungen, Konfiguration --------------------------------

/// Ein Prompt mit zwei GELB-Funden und ohne Geheimnis-Kontext.
const GELB_OHNE_KONTEXT: &str = "Erika Beispiel wohnt an der Seestrasse 12, 8002 Zürich.";

/// Höchstens 80 Zeichen je Zeile.
fn schmal(text: &str) {
    for z in text.lines() {
        assert!(
            z.chars().count() <= 80,
            "{} Zeichen: {z}",
            z.chars().count()
        );
    }
}

/// Eine Meldung nach docs/AUSGABE.md 8: erste Zeile «promptscan: » und das Problem,
/// danach eingerückt der nächste Schritt; keine englischen Texte von clap, keine
/// Systemcodes.
fn meldung(fehler: &str) {
    let zeilen: Vec<&str> = fehler.lines().collect();
    assert!(zeilen.len() >= 2, "zweizeilig: {fehler}");
    assert!(zeilen[0].starts_with("promptscan: "), "{fehler}");
    assert!(zeilen[1..].iter().all(|z| z.starts_with("  ")), "{fehler}");
    for fremd in [
        "error:",
        "Usage:",
        "For more information",
        "os error",
        "tip:",
    ] {
        assert!(!fehler.contains(fremd), "«{fremd}» in {fehler}");
    }
}

/// Optionen vor dem Befehl; Kennungen und Werte in jeder Schreibweise.
#[test]
fn optionen_vor_dem_befehl() {
    assert!(ausgabe(promptscan().args(["--lang", "en", "rules"])).starts_with("Rules by level"));
    let v = json_von(&ausgabe(
        promptscan().args(["--format", "json", "explain", "ahv"]),
    ));
    assert_eq!(v["id"], "ahv");
    assert_eq!(
        ausgabe(promptscan().args(["--no-color", "explain", "AHV"])),
        ausgabe(promptscan().args(["explain", "ahv"]))
    );
    let v = json_von(&ausgabe(
        promptscan().args(["--lang", "EN", "--format", "JSON", "explain", "Rot"]),
    ));
    assert_eq!(v["dokument"], "stufe");
}

/// Eine Datei, die wie ein Befehl heisst, erreicht man mit «./rules» oder «-- rules».
#[test]
fn datei_die_wie_ein_befehl_heisst() {
    let o = Ordner::neu("rules");
    o.datei("rules", NEUTRAL.as_bytes());
    for args in [
        vec!["./rules"],
        vec!["--", "rules"],
        vec!["--no-color", "--", "rules"],
    ] {
        assert_eq!(
            ausgabe(promptscan().current_dir(&o.0).args(&args)),
            ohne_fund(&[NEUTRAL]),
            "{args:?}"
        );
    }
    assert!(ausgabe(promptscan().current_dir(&o.0).arg("rules")).starts_with("Regeln nach Stufe"));
}

/// -t, eine Datei und ein Befehl schliessen sich aus; die Meldung sagt, was geht.
#[test]
fn eingabe_und_befehl_schliessen_sich_aus() {
    for (args, anfang) in [
        (
            vec!["-t", NEUTRAL, "rules"],
            "promptscan: -t und der Befehl rules gehen nicht zusammen.\n",
        ),
        (
            vec!["explain", "ahv", "-t", NEUTRAL, "--lang", "en"],
            "promptscan: -t and the command explain do not go together.\n",
        ),
        (
            vec!["datei.txt", "config"],
            "promptscan: Eine Datei («datei.txt») und der Befehl config gehen nicht zusammen.\n",
        ),
        (
            vec!["-t", NEUTRAL, "datei.txt"],
            "promptscan: -t und eine Datei («datei.txt») gehen nicht zusammen.\n",
        ),
        (
            vec!["rules", "datei.txt"],
            "promptscan: Der Befehl rules nimmt «datei.txt» nicht an.\n",
        ),
        (
            vec!["explain"],
            "promptscan: explain braucht eine Regel oder Stufe.\n",
        ),
    ] {
        let (code, aus, fehler) = lauf(promptscan().args(&args));
        assert_eq!(code, Some(2), "{args:?}");
        assert!(aus.is_empty(), "{args:?}: {aus}");
        assert!(fehler.starts_with(anfang), "{args:?}: {fehler}");
        meldung(&fehler);
    }
}

/// Ein Text mit «-» am Anfang (Aufzählung, negative Zahl, Option als Wort) ist Text.
#[test]
fn text_mit_bindestrich_am_anfang() {
    for text in [
        "- erster Punkt",
        "-5 Grad am Morgen",
        "--- Abschnitt ---",
        "--help",
    ] {
        assert_eq!(
            ausgabe(promptscan().args(["-t", text])),
            ohne_fund(&[text]),
            "{text:?}"
        );
    }
    // Mit Funden: Exit-Code wie sonst.
    let text = "- Erika Beispiel, AHV 756.1234.5678.97";
    let (code, aus, _) = lauf(promptscan().args(["--role", "lawyer", "-t", text]));
    assert_eq!(code, Some(1));
    assert!(aus.contains(&format!("\n   │ {text}\n")), "{aus}");
}

/// Ein \ vor dem schliessenden " zieht in Windows PowerShell 5.1 und cmd die
/// folgenden Optionen in den Text. promptscan prüft dann nicht still ohne Rolle.
#[test]
fn verschluckte_optionen_geben_exit_2() {
    // So kommt `promptscan -t "Akte unter C:\Akten\" --role lawyer` aus cmd an.
    let (code, aus, fehler) = lauf(promptscan().args([
        "-t",
        "Erika Beispiel, Akte unter C:\\Akten\" --role lawyer --fail-on yellow",
    ]));
    assert_eq!(code, Some(2));
    assert!(aus.is_empty(), "{aus}");
    assert!(
        einzeilig(&fehler).starts_with(
            "promptscan: Im Text nach -t stehen Optionen von promptscan: «\" --role lawyer --fail-on yellow»."
        ),
        "{fehler}"
    );
    schmal(&fehler);
    assert!(
        einzeilig(&fehler).contains("Optionen vor -t schreiben"),
        "{fehler}"
    );
    meldung(&fehler);
    // Gegenprobe: Anführungszeichen im Text, aber keine Option danach.
    ausgabe(promptscan().args(["-t", "Er sagte \"ja\" und ging."]));
}

/// Bei einem Fehler der Befehlszeile mit -t der Hinweis auf gerade Anführungszeichen; fehlt
/// nur der Text, sagt die Meldung das.
#[test]
fn hinweis_auf_gerade_anfuehrungszeichen() {
    // So kann `promptscan -t "Er sagte "ja""` in Windows PowerShell 5.1 ankommen.
    let (code, aus, fehler) = lauf(promptscan().args(["-t", "Er", "sagte", "ja"]));
    assert_eq!(code, Some(2));
    assert!(aus.is_empty());
    assert!(
        fehler.starts_with("promptscan: Das Argument «ja» ist zu viel.\n"),
        "{fehler}"
    );
    let flach = einzeilig(&fehler);
    assert!(
        flach.contains("Gerade Anführungszeichen (\") im Text nach -t?"),
        "{fehler}"
    );
    assert!(
        flach.contains("promptscan --% -t \"... \\\"...\\\" ...\""),
        "{fehler}"
    );
    meldung(&fehler);
    schmal(&fehler);
    let (_, _, fehler) = lauf(promptscan().arg("-t"));
    assert!(
        fehler.starts_with("promptscan: Nach -t fehlt der Text.\n"),
        "{fehler}"
    );
    assert!(!fehler.contains("Anführungszeichen ("), "{fehler}");
    let (_, _, fehler) = lauf(promptscan().args(["--lang", "en", "-t", "a", "b", "c"]));
    assert!(
        einzeilig(&fehler).contains("Straight quotes (\") in the text after -t?"),
        "{fehler}"
    );
    // p9: So kommt `-t 'Er sagte "ja, gerne" und ging'` aus Windows PowerShell 5.1 an.
    // Die erste Zeile nennt den Grund, nicht eine Datei, die es nicht gibt.
    let (code, _, fehler) = lauf(promptscan().args(["-t", "Er sagte ja,", "gerne und ging"]));
    assert_eq!(code, Some(2));
    assert!(
        fehler.starts_with(
            "promptscan: Der Text nach -t ist zerfallen: «gerne und ging» kam einzeln an.\n  Gerade Anführungszeichen"
        ),
        "{fehler}"
    );
    meldung(&fehler);
    schmal(&fehler);
}

/// B9: Fehler der Befehlszeile deutsch oder englisch, zweizeilig, mit Vorschlag.
#[test]
fn fehler_der_befehlszeile_zweisprachig() {
    for (args, anfang) in [
        (
            vec!["--format", "xml"],
            "promptscan: «xml» ist kein gültiger Wert für --format.\n  Möglich: text, json\n",
        ),
        (
            vec!["--lang", "en", "--format", "xml"],
            "promptscan: «xml» is not a valid value for --format.\n  Possible: text, json\n",
        ),
        (
            vec!["--format", "xml", "--lang", "en"],
            "promptscan: «xml» is not a valid value for --format.\n",
        ),
        (
            vec!["--rolle", "lawyer"],
            "promptscan: Die Option «--rolle» gibt es nicht.\n  Gemeint ist wohl: --role\n",
        ),
        // p9: Optionen in Grossbuchstaben, deutsche Stufen, Argumente zu viel.
        (
            vec!["--ROLE", "lawyer", "-t", NEUTRAL],
            "promptscan: Die Option «--ROLE» gibt es nicht.\n  Gemeint ist wohl: --role\n",
        ),
        (
            vec!["--fail-on", "rot", "-t", NEUTRAL],
            "promptscan: «rot» ist kein gültiger Wert für --fail-on.\n  Gemeint ist wohl: --fail-on red\n  Möglich: red, orange, yellow, never\n",
        ),
        (
            vec!["--lang", "en", "--fail-on", "Gelb"],
            "promptscan: «Gelb» is not a valid value for --fail-on.\n  Did you mean: --fail-on yellow\n",
        ),
        (
            vec!["explain", "ahv", "rot"],
            "promptscan: Der Befehl explain nimmt «rot» nicht an.\n  Je Aufruf eine Regel oder Stufe, etwa: promptscan explain ahv\n",
        ),
        (
            vec!["help", "rules"],
            "promptscan: Der Befehl help nimmt «rules» nicht an.\n  Hilfe: promptscan --help;",
        ),
        (
            vec!["config", "extra"],
            "promptscan: Der Befehl config nimmt «extra» nicht an.\n  Rolle oder Sprache speichern: promptscan config --role ROLLE",
        ),
        (
            vec!["--lang"],
            "promptscan: Nach --lang fehlt der Wert.\n  Möglich: de, en\n",
        ),
        (
            vec!["--fail-on", "gruen"],
            "promptscan: «gruen» ist kein gültiger Wert für --fail-on.\n",
        ),
        (
            vec!["--target", "mond"],
            "promptscan: «mond» ist kein gültiger Wert für --target.\n  Möglich: cloud, local\n",
        ),
        (
            vec!["--quiet", "--quiet"],
            "promptscan: Die Option --quiet steht mehr als einmal da.\n",
        ),
        (
            vec!["--no-color=x", "-t", NEUTRAL],
            "promptscan: Die Option --no-color nimmt keinen Wert an («x»).\n",
        ),
        (
            vec!["-t", NEUTRAL, "--output"],
            "promptscan: Nach --output fehlt der Name der Datei.\n",
        ),
        (
            vec!["--role"],
            "promptscan: Nach --role fehlt der Wert.\n  Möglich: none, clergy, lawyer,",
        ),
        // Ein Tippfehler in einem Befehl wird sonst als Datei gelesen.
        (
            vec!["rule"],
            "promptscan: Die Datei gibt es nicht: rule\n  Gemeint ist wohl der Befehl: promptscan rules\n",
        ),
    ] {
        let (code, aus, fehler) = lauf(promptscan().args(&args));
        assert_eq!(code, Some(2), "{args:?}");
        assert!(aus.is_empty());
        assert!(fehler.starts_with(anfang), "{args:?}: {fehler}");
        meldung(&fehler);
        schmal(&fehler);
    }
    // Die Sprache aus der Konfiguration gilt auch für Fehler der Befehlszeile.
    let o = Ordner::neu("fehler-englisch");
    let datei = o.datei("config.toml", b"lang = \"en\"\n");
    let (_, _, fehler) = lauf(
        promptscan()
            .args(["--format", "xml"])
            .env("PROMPTSCAN_CONFIG", &datei),
    );
    assert!(
        fehler.starts_with("promptscan: «xml» is not a valid value"),
        "{fehler}"
    );
}

/// Die Rolle als Kennung oder deutsches Wort, in jeder Schreibweise; ein Tippfehler gibt
/// einen Vorschlag.
#[test]
fn rolle_deutsch_und_mit_vorschlag() {
    for wort in [
        "lawyer",
        "Lawyer",
        "anwalt",
        "Anwältin",
        "RECHTSANWALT",
        "Fürsprecher",
    ] {
        let (code, aus, fehler) =
            lauf(promptscan().args(["--role", wort, "--format", "json", "-t", NEUTRAL]));
        assert_eq!(code, Some(0), "{wort}: {fehler}");
        assert_eq!(json_von(&aus)["optionen"]["rolle"], "lawyer", "{wort}");
    }
    let (code, aus, fehler) = lauf(promptscan().args(["--role", "anwält", "-t", NEUTRAL]));
    assert_eq!(code, Some(2));
    assert!(aus.is_empty());
    assert!(
        fehler.starts_with(
            "promptscan: Die Rolle «anwält» gibt es nicht.\n  Gemeint ist wohl: --role lawyer\n"
        ),
        "{fehler}"
    );
    meldung(&fehler);
    let (_, _, fehler) =
        lauf(promptscan().args(["--lang", "en", "--role", "pilot", "-t", NEUTRAL]));
    assert!(
        fehler.starts_with("promptscan: There is no role «pilot».\n"),
        "{fehler}"
    );
    assert!(
        einzeilig(&fehler).contains("financial-institution"),
        "{fehler}"
    );
    schmal(&fehler);
}

/// Die Hilfe auf Deutsch und Englisch, höchstens 80 Spalten, mit dem Hinweis zu -t und der
/// Zeile «Hilfsmittel»; auch bei sonst fehlerhafter Befehlszeile und in der Sprache der
/// Konfiguration.
#[test]
fn hilfe_deutsch_und_englisch() {
    let de = ausgabe(promptscan().arg("--help"));
    schmal(&de);
    let flach = einzeilig(&de);
    for muss in [
        "Prompt einfügen, Enter, dann Ctrl+",
        // Der Hinweis zu -t und die Zeile «Hilfsmittel».
        "promptscan -t \"TEXT\" Text direkt prüfen. Er steht dann in der Befehlszeile und kann in den Shell-Verlauf und in Prozessprotokolle gelangen, ebenso bei «echo … |».",
        "Vertrauliches als Datei oder durch Einfügen prüfen.",
        "Hilfsmittel, keine Rechtsberatung · erkennt Identifikatoren und Wörter aus Listen, nicht den Zusammenhang",
        "promptscan config --role lawyer",
        "financial-institution",
        "Vorgabe cloud",
        "Deutsch, Französisch und Italienisch",
        "nur mit --role oder aus dem Text erschlossen",
        "--output DATEI",
    ] {
        assert!(flach.contains(muss), "«{muss}» fehlt:\n{de}");
    }
    assert_eq!(ausgabe(promptscan().arg("-h")), de);
    assert_eq!(ausgabe(promptscan().arg("help")), de);
    assert_eq!(
        ausgabe(promptscan().args(["--format", "xml", "--help"])),
        de
    );
    let en = ausgabe(promptscan().args(["--help", "--lang", "en"]));
    schmal(&en);
    assert!(
        einzeilig(&en).contains(
            "An aid, not legal advice · detects identifiers and listed words, not the context"
        ),
        "{en}"
    );
    assert!(einzeilig(&en).contains(
        "It then appears in the command line and can end up in the shell history and process logs"
    ));
    assert!(einzeilig(&en).contains("Check confidential text as a file or by pasting."));
    assert_eq!(ausgabe(promptscan().args(["--lang=en", "-h"])), en);
    let o = Ordner::neu("hilfe");
    let datei = o.datei("config.toml", b"lang = \"en\"\n");
    assert_eq!(
        ausgabe(promptscan().arg("--help").env("PROMPTSCAN_CONFIG", &datei)),
        en
    );
    assert_eq!(
        ausgabe(
            promptscan()
                .args(["--help", "--lang", "de"])
                .env("PROMPTSCAN_CONFIG", &datei)
        ),
        de
    );
}

/// Windows PowerShell 5.1 schreibt mit «>» UTF-16LE mit BOM und CRLF; promptscan
/// liest das wie UTF-8.
#[test]
fn konfiguration_in_utf16() {
    let o = Ordner::neu("utf16");
    let datei = o.datei(
        "config.toml",
        &utf16("role = \"lawyer\"\r\nlang = \"en\"\r\n", true),
    );
    let (code, aus, fehler) = lauf(
        promptscan()
            .args(["-t", BEISPIEL_2])
            .env("PROMPTSCAN_CONFIG", &datei),
    );
    assert_eq!(code, Some(1), "{fehler}");
    assert!(
        aus.contains("Result    RED   Duty of confidentiality backed by criminal law · 2 findings"),
        "{aus}"
    );
    assert!(fehler.is_empty(), "{fehler}");
}

/// Bei rules, explain und config ist ein Fehler der Datei nur eine Warnung.
#[test]
fn kaputte_konfiguration_warnt_nur_bei_den_befehlen() {
    let o = Ordner::neu("warnt");
    for inhalt in ["lang = \n", "role = \"pilot\"\n", "farbe = 1\n"] {
        let datei = o.datei("config.toml", inhalt.as_bytes());
        for args in [vec!["rules"], vec!["explain", "ahv"], vec!["config"]] {
            let (code, aus, fehler) =
                lauf(promptscan().args(&args).env("PROMPTSCAN_CONFIG", &datei));
            assert_eq!(code, Some(0), "{args:?} {inhalt:?}: {fehler}");
            assert!(!aus.is_empty());
            assert!(
                fehler.starts_with("promptscan: Konfiguration: "),
                "{args:?} {inhalt:?}: {fehler}"
            );
            assert!(
                einzeilig(&fehler).contains(&format!("Datei: {}", datei.display())),
                "{fehler}"
            );
        }
    }
    // Mit --role läuft auch die Prüfung, mit Warnung.
    let datei = o.datei("config.toml", b"lang = \n");
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["--role", "lawyer", "-t", BEISPIEL_2])
            .env("PROMPTSCAN_CONFIG", &datei),
    );
    assert_eq!(code, Some(1));
    assert!(fehler.contains("ist kein gültiges TOML"), "{fehler}");
}

/// Ein falscher Wert, den die Befehlszeile übersteuert, gibt eine Warnung statt Stille.
#[test]
fn uebersteuerter_falscher_wert_warnt() {
    let o = Ordner::neu("uebersteuert");
    let datei = o.datei("config.toml", b"role = \"pilot\"\nlang = \"fr\"\n");
    let (code, aus, fehler) = lauf(
        promptscan()
            .args(["--role", "lawyer", "--lang", "de", "-t", BEISPIEL_2])
            .env("PROMPTSCAN_CONFIG", &datei),
    );
    assert_eq!(code, Some(1), "{fehler}");
    assert!(
        aus.contains("Ergebnis  ROT   Strafbewehrte Geheimhaltungspflicht · 2 Funde"),
        "{aus}"
    );
    let flach = einzeilig(&fehler);
    assert!(
        flach.contains("Die Rolle «pilot» gibt es nicht; es gilt --role lawyer."),
        "{fehler}"
    );
    assert!(
        flach.contains("Die Sprache «fr» gibt es nicht; es gilt --lang de."),
        "{fehler}"
    );
}

/// target in der Datei wird übergangen, mit Warnung: Das Ziel gilt je Aufruf.
#[test]
fn ziel_in_der_konfiguration_wird_uebergangen() {
    let o = Ordner::neu("ziel");
    for wert in ["local", "mond"] {
        let datei = o.datei("config.toml", format!("target = \"{wert}\"\n").as_bytes());
        let (code, aus, fehler) = lauf(
            promptscan()
                .args(["--format", "json", "-t", NEUTRAL])
                .env("PROMPTSCAN_CONFIG", &datei),
        );
        assert_eq!(code, Some(0), "{fehler}");
        assert_eq!(json_von(&aus)["optionen"]["ziel"], "cloud");
        let flach = einzeilig(&fehler);
        assert!(
            flach.contains(&format!("target = \"{wert}\" wird übergangen.")),
            "{fehler}"
        );
        assert!(
            flach.contains("Das Ziel gilt je Aufruf: --target local angeben"),
            "{fehler}"
        );
        meldung(&fehler);
    }
}

/// promptscan config zeigt die Datei und schreibt Rolle und Sprache: UTF-8 ohne BOM,
/// Ordner angelegt, übrige Schlüssel behalten.
#[test]
fn befehl_config() {
    let o = Ordner::neu("config");
    let pfad = o.0.join("neu").join("unter").join("config.toml");
    let mit = |c: &mut Command| -> (Option<i32>, String, String) {
        lauf(c.env("PROMPTSCAN_CONFIG", &pfad))
    };
    let (code, aus, _) = mit(promptscan().arg("config"));
    assert_eq!(code, Some(0));
    assert!(aus.contains(&pfad.display().to_string()), "{aus}");
    assert!(aus.contains("Ort aus PROMPTSCAN_CONFIG"), "{aus}");
    assert!(aus.contains("(gibt es noch nicht)"), "{aus}");
    assert!(!pfad.exists());

    let (code, aus, fehler) = mit(promptscan().args(["config", "--role", "Anwältin"]));
    assert_eq!(code, Some(0), "{fehler}");
    assert!(
        aus.starts_with("Gespeichert: role = \"lawyer\".\n"),
        "{aus}"
    );
    assert!(aus.contains("role = \"lawyer\""), "{aus}");
    assert_eq!(std::fs::read(&pfad).expect("Datei"), b"role = \"lawyer\"\n");

    // p5: Vor «config» gelten --lang und --role nur für diesen Aufruf; die Ansicht kommt
    // englisch, gespeichert wird nichts, und zu --role sagt eine Warnung, wie es geht.
    let (code, aus, fehler) = mit(promptscan().args(["--lang", "en", "--role", "bank", "config"]));
    assert_eq!(code, Some(0), "{fehler}");
    assert!(aus.starts_with("Configuration file\n"), "{aus}");
    assert!(
        einzeilig(&fehler).contains(
            "--role before «config» saves nothing. To save it: promptscan config --role bank"
        ),
        "{fehler}"
    );
    assert_eq!(std::fs::read(&pfad).expect("Datei"), b"role = \"lawyer\"\n");

    let (code, aus, _) = mit(promptscan().args(["config", "--lang", "en"]));
    assert_eq!(code, Some(0));
    assert!(aus.starts_with("Saved: lang = \"en\".\n"), "{aus}");
    let text = std::fs::read_to_string(&pfad).expect("Datei");
    assert_eq!(text, "role = \"lawyer\"\nlang = \"en\"\n");
    // Der alte Wert steht in der Bestätigung.
    let (_, aus, _) = mit(promptscan().args(["config", "--lang", "de"]));
    assert!(
        aus.starts_with("Gespeichert: lang = \"de\" (bisher \"en\").\n"),
        "{aus}"
    );
    mit(promptscan().args(["config", "--lang", "en"]));
    // Die gespeicherte Rolle gilt bei der Prüfung.
    let (code, aus, _) = mit(promptscan().args(["-t", BEISPIEL_2]));
    assert_eq!(code, Some(1));
    assert!(
        aus.contains("Result    RED   Duty of confidentiality backed by criminal law · 2 findings"),
        "{aus}"
    );

    // Eine Datei aus Windows PowerShell 5.1 (UTF-16LE mit BOM) wird als UTF-8 ohne BOM neu
    // geschrieben, mit ihren Schlüsseln und ihren Zeilenenden (CRLF).
    let ps = o.datei("ps.toml", &utf16("lang = \"de\"\r\n", true));
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["config", "--role", "arzt"])
            .env("PROMPTSCAN_CONFIG", &ps),
    );
    assert_eq!(code, Some(0), "{fehler}");
    let text = String::from_utf8(std::fs::read(&ps).expect("Datei")).expect("UTF-8 ohne BOM");
    assert_eq!(text, "lang = \"de\"\r\nrole = \"doctor\"\r\n");

    // p6: Kommentare, Reihenfolge und target bleiben; nur die Zeile von role ändert sich.
    let kommentiert = o.datei(
        "kommentiert.toml",
        "# Rolle für die Kanzlei\nrole = \"doctor\" # Anwältin\ntarget = \"local\"\n".as_bytes(),
    );
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["config", "--role", "lawyer"])
            .env("PROMPTSCAN_CONFIG", &kommentiert),
    );
    assert_eq!(code, Some(0), "{fehler}");
    assert_eq!(
        std::fs::read_to_string(&kommentiert).expect("Datei"),
        "# Rolle für die Kanzlei\nrole = \"lawyer\" # Anwältin\ntarget = \"local\"\n"
    );
    // Die Warnung zu target sagt, dass man die Zeile löschen muss.
    assert!(
        einzeilig(&fehler).contains("Die Zeile target in der Datei löschen"),
        "{fehler}"
    );

    // Das Ziel speichert config nicht; eine falsche Rolle wird nicht geschrieben.
    let leer = o.0.join("leer.toml");
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["config", "--target", "local"])
            .env("PROMPTSCAN_CONFIG", &leer),
    );
    assert_eq!(code, Some(0));
    assert!(
        fehler.starts_with("promptscan: promptscan config speichert kein Ziel."),
        "{fehler}"
    );
    assert!(!leer.exists());
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["config", "--role", "anwält"])
            .env("PROMPTSCAN_CONFIG", &leer),
    );
    assert_eq!(code, Some(2));
    assert!(
        fehler.contains("Gemeint ist wohl: --role lawyer"),
        "{fehler}"
    );
    assert!(!leer.exists());
}

/// B7: JSON auf stdout in reinem ASCII; geparst gleich wie das JSON der Bibliothek.
#[test]
fn json_in_reinem_ascii() {
    let (code, aus, _) = lauf(promptscan().args(["--format", "json", "-t", BEISPIEL_1]));
    assert_eq!(code, Some(0));
    assert!(aus.is_ascii(), "{aus}");
    assert!(aus.contains("Z\\u00fcrich"), "{aus}");
    let bericht = promptscan::pruefe(
        BEISPIEL_1,
        &promptscan::Optionen {
            rolle: None,
            ziel: promptscan::Ziel::Cloud,
        },
    );
    assert_eq!(
        json_von(&aus),
        json_von(&promptscan::ausgabe::bericht_json(&bericht))
    );
    // Ersatzpaare ausserhalb der BMP prüft json_in_ascii in main.rs. Auch rules und
    // explain gehen in ASCII hinaus.
    for args in [
        vec!["rules"],
        vec!["explain", "art5c-2"],
        vec!["explain", "rot"],
    ] {
        let aus = ausgabe(promptscan().args(&args).args(["--format", "json"]));
        assert!(aus.is_ascii(), "{args:?}");
        json_von(&aus);
    }
}

/// B7: --output schreibt UTF-8 mit BOM, ohne Farbe; der Exit-Code bleibt.
#[test]
fn ausgabe_in_eine_datei() {
    let o = Ordner::neu("output");
    let ziel = o.0.join("bericht.txt");
    let (code, aus, fehler) = lauf(
        promptscan()
            .args(["--role", "lawyer", "--output"])
            .arg(&ziel)
            .args(["-t", BEISPIEL_2])
            .env("CLICOLOR_FORCE", "1"),
    );
    assert_eq!(code, Some(1));
    assert!(aus.is_empty(), "{aus}");
    assert!(fehler.is_empty(), "{fehler}");
    let bytes = std::fs::read(&ziel).expect("Datei");
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    let text = String::from_utf8(bytes[3..].to_vec()).expect("UTF-8");
    assert!(!text.contains('\u{1B}'), "{text}");
    let pipe = lauf(
        promptscan()
            .args(["--role", "lawyer", "-t", BEISPIEL_2])
            .env("LC_ALL", "C.UTF-8"),
    )
    .1;
    assert_eq!(text, pipe);

    let ziel = o.0.join("bericht.json");
    let (code, aus, _) = lauf(
        promptscan()
            .args(["--format", "json", "--output"])
            .arg(&ziel)
            .args(["-t", BEISPIEL_1]),
    );
    assert_eq!(code, Some(0));
    assert!(aus.is_empty());
    let bytes = std::fs::read(&ziel).expect("Datei");
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert!(bytes[3..].is_ascii());
    json_von(std::str::from_utf8(&bytes[3..]).expect("ASCII"));

    // rules in eine Datei.
    let ziel = o.0.join("regeln.txt");
    ausgabe(promptscan().arg("rules").arg("--output").arg(&ziel));
    assert!(
        std::fs::read(&ziel)
            .expect("Datei")
            .starts_with(b"\xEF\xBB\xBFRegeln nach Stufe")
    );

    // Einen Ordner, den es nicht gibt, legt --output nicht an. p8: eigener Grund, auch
    // englisch ohne Text des Systems; der Pfad hinter «Datei:», bei Platzmangel auf einer
    // eigenen Zeile (p7).
    let fehlt = o.0.join("fehlt").join("x.txt");
    for (lang, anfang, angabe) in [
        (
            "de",
            "promptscan: Die Ausgabe lässt sich nicht schreiben: Den Ordner gibt es nicht.",
            "Datei:",
        ),
        (
            "en",
            "promptscan: The output cannot be written: the folder does not exist.",
            "File:",
        ),
    ] {
        let (code, _, fehler) = lauf(
            promptscan()
                .args(["--lang", lang, "--output"])
                .arg(&fehlt)
                .args(["-t", NEUTRAL]),
        );
        assert_eq!(code, Some(2));
        assert!(fehler.starts_with(anfang), "{fehler}");
        assert!(
            einzeilig(&fehler).contains(&format!("{angabe} {}", fehlt.display())),
            "{fehler}"
        );
        meldung(&fehler);
        schmal_ausser_pfad(&fehler, &fehlt);
    }
    // Ein Ordner als Ziel.
    let (code, _, fehler) = lauf(promptscan().arg("--output").arg(&o.0).args(["-t", NEUTRAL]));
    assert_eq!(code, Some(2));
    assert!(
        fehler.starts_with(
            "promptscan: Die Ausgabe lässt sich nicht schreiben: Der Pfad ist ein Ordner."
        ),
        "{fehler}"
    );

    // p14: Auch die Hilfe geht mit --output in die Datei.
    let hilfe = o.0.join("hilfe.txt");
    let (code, aus, _) = lauf(promptscan().arg("--help").arg("--output").arg(&hilfe));
    assert_eq!(code, Some(0));
    assert!(aus.is_empty(), "{aus}");
    let bytes = std::fs::read(&hilfe).expect("Hilfe in der Datei");
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert_eq!(
        std::str::from_utf8(&bytes[3..]).expect("UTF-8"),
        ausgabe(promptscan().arg("--help"))
    );
}

/// B8: --quiet zeigt nur das Ergebnis; der Exit-Code bleibt.
#[test]
fn quiet_nur_das_ergebnis() {
    let (code, aus, _) = lauf(promptscan().args(["--quiet", "--role", "lawyer", "-t", BEISPIEL_2]));
    assert_eq!(code, Some(1));
    assert!(!aus.contains("Erika") && !aus.contains("│"), "{aus}");
    assert!(
        aus.contains("Ergebnis  ROT   Strafbewehrte Geheimhaltungspflicht · 2 Funde"),
        "{aus}"
    );
    let (code, aus, _) = lauf(promptscan().args([
        "--quiet", "--format", "json", "--role", "lawyer", "-t", BEISPIEL_2,
    ]));
    assert_eq!(code, Some(1));
    let v = json_von(&aus);
    assert_eq!(v["dokument"], "bericht-kurz");
    assert_eq!(v["hoechste_stufe"], "rot");
    assert!(v.get("funde").is_none(), "{v}");
    assert_eq!(
        ausgabe(promptscan().args(["--quiet", "-t", NEUTRAL])),
        format!("{KEINE_FUNDE}\n{BEACHTEN_OHNE_FUND}")
    );
}

/// B10, p1: Mit --ascii ist jedes Byte von Bericht, rules, explain, config, Hilfe und
/// Meldungen höchstens 0x7F, und die Marken ^^^ stehen weiter unter ihrer Stelle.
#[test]
fn ascii_ist_ascii() {
    let o = Ordner::neu("ascii");
    let konfig = o.datei("config.toml", "role = \"Anwältin\"\n".as_bytes());
    let prompt = "Zürich «Akte» – Erika Beispiel, AHV 756.1234.5678.97, ist depressiv.";
    let faelle: Vec<Vec<&str>> = vec![
        vec!["--ascii", "--no-color", "--role", "lawyer", "-t", prompt],
        vec!["--ascii", "--lang", "en", "-t", prompt],
        vec!["--ascii", "--quiet", "-t", prompt],
        vec!["--ascii", "rules"],
        vec!["--ascii", "explain", "ahv"],
        vec!["--ascii", "explain", "rot"],
        vec!["--ascii", "config"],
        vec!["--ascii", "--help"],
        vec!["--ascii", "--lang", "en", "--help"],
        vec!["--ascii", "--role", "anwält", "-t", prompt],
        vec!["--ascii", "explain", "ahw"],
        vec!["--ascii", "--fail-on", "rot"],
    ];
    for args in &faelle {
        let a = promptscan()
            .args(args)
            .env("PROMPTSCAN_CONFIG", &konfig)
            .output()
            .expect("Programm starten");
        assert!(
            a.stdout.is_ascii(),
            "{args:?}: {}",
            String::from_utf8_lossy(&a.stdout)
        );
        assert!(
            a.stderr.is_ascii(),
            "{args:?}: {}",
            String::from_utf8_lossy(&a.stderr)
        );
        assert!(!(a.stdout.is_empty() && a.stderr.is_empty()), "{args:?}");
    }
    // Die Marken stehen unter «Erika Beispiel», auch wenn davor Umlaute umgeschrieben sind.
    let (code, aus, _) = lauf(promptscan().args(faelle[0].clone()));
    assert_eq!(code, Some(1));
    let zeilen: Vec<&str> = aus.lines().collect();
    let i = zeilen
        .iter()
        .position(|z| z.contains("Erika Beispiel"))
        .expect("Zeilenansicht");
    let spalte_name = zeilen[i].find("Erika").expect("Name");
    // Die Marke beginnt mit dem Buchstaben der Stufe (R^^^…), bündig unter der Stelle.
    let spalte_marke = zeilen[i + 1].find("R^").expect("Marken");
    assert_eq!(spalte_name, spalte_marke, "{aus}");
    // Mit --output: das BOM von UTF-8 wie immer, danach nur ASCII.
    let datei = o.0.join("bericht.txt");
    let (code, _, _) = lauf(
        promptscan()
            .args(["--ascii", "--role", "lawyer", "--output"])
            .arg(&datei)
            .args(["-t", prompt]),
    );
    assert_eq!(code, Some(1));
    let bytes = std::fs::read(&datei).expect("Datei");
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert!(bytes[3..].is_ascii(), "{}", String::from_utf8_lossy(&bytes));
    // Ohne --ascii bleiben die Umlaute.
    assert!(!ausgabe(promptscan().arg("--help")).is_ascii());
}

/// Die Rolle financial-institution, die Frage nach einer Geheimhaltungspflicht ohne Rolle und
/// das Datenschutzrecht bei --role court.
#[test]
fn financial_institution_frage_ohne_rolle_und_datenschutzrecht() {
    let (code, aus, _) = lauf(promptscan().args([
        "--role",
        "financial-institution",
        "--format",
        "json",
        "-t",
        NEUTRAL,
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(json_von(&aus)["optionen"]["rolle"], "financial-institution");
    assert!(ausgabe(promptscan().arg("--help")).contains("financial-institution"));

    // Die Frage nach der Geheimhaltungspflicht unter «Klären» ohne Rolle bei GELB; nicht
    // mit role = "none" in der Datei: Die Rolle aus der Konfiguration zählt wie --role.
    const FRAGE_ROLLE: &str = "Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)?\n          Dann --role angeben (Rollen: promptscan explain rot).\n";
    let ohne = ausgabe(promptscan().args(["-t", GELB_OHNE_KONTEXT]));
    assert!(
        ohne.contains("Ergebnis  GELB   Personendaten · 2 Funde"),
        "{ohne}"
    );
    assert!(ohne.contains(FRAGE_ROLLE), "{ohne}");
    let o = Ordner::neu("reste");
    let keine = o.datei("config.toml", b"role = \"none\"\n");
    let mit = ausgabe(
        promptscan()
            .args(["-t", GELB_OHNE_KONTEXT])
            .env("PROMPTSCAN_CONFIG", &keine),
    );
    assert!(
        mit.contains("Ergebnis  GELB   Personendaten · 2 Funde"),
        "{mit}"
    );
    assert!(
        !mit.contains("Geheimhaltungspflicht (Beruf, Amt, Bank)"),
        "{mit}"
    );

    // Mit --role court steht unter «Beachten», welches Datenschutzrecht gilt, mit beiden
    // Sätzen von Art. 2 Abs. 3 DSG.
    let (code, gericht, _) = lauf(promptscan().args(["--role", "court", "-t", GELB_OHNE_KONTEXT]));
    assert_eq!(code, Some(1));
    assert!(
        gericht.contains(
            "\nBeachten  Für kantonale und kommunale Organe gilt kantonales Datenschutzrecht,\n"
        ),
        "{gericht}"
    );
    assert!(
        einzeilig(&gericht).contains("gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 DSG)."),
        "{gericht}"
    );
    assert!(
        !gericht.contains("Gilt für Sie eine Geheimhaltungspflicht"),
        "{gericht}"
    );
}

// --- Technik: Steuerzeichen, Kodierung, Ausgabe -------------------------------------------

/// Führt aus und gibt Exit-Code, Standardausgabe und Fehlerausgabe zurück.
fn lauf(c: &mut Command) -> (Option<i32>, String, String) {
    let aus = c.output().expect("Programm starten");
    (
        aus.status.code(),
        String::from_utf8(aus.stdout).expect("Ausgabe in UTF-8"),
        String::from_utf8(aus.stderr).expect("Fehlerausgabe in UTF-8"),
    )
}

/// Die Erklärungszeile «Beruf offen nach «…»» (wie «Rolle erschlossen aus «…»») zeigt den
/// Ausschnitt aus dem Prompt wie die Stelle in der Liste: Steuer- und Bidi-Zeichen als
/// U+FFFD, den Tabulator als Leerzeichen. Die Regel kontext überspringt VT und FF als
/// Leerraum zwischen Possessiv und Beruf; roh kämen sie beim Terminal an und verschöben
/// die Zeilen.
#[test]
fn erschlossene_rolle_ohne_steuerzeichen() {
    for (trenner, gezeigt) in [("\u{0B}", "\u{FFFD}"), ("\u{0C}", "\u{FFFD}"), ("\t", " ")] {
        let prompt = format!(
            "meine{trenner}Klientin Erika Beispiel, AHV 756.1234.5678.97, will das Erbe ausschlagen."
        );
        for (sprache, zeile) in [
            // Im Bericht steht der Ausschnitt in der Kopfzeile (AUSGABE 1).
            (
                "de",
                format!("Rolle: Beruf offen, aus «meine{gezeigt}Klientin»"),
            ),
            (
                "en",
                format!("Role: profession open, from «meine{gezeigt}Klientin»"),
            ),
        ] {
            // Mit Farbe lässt anstream jedes Steuerzeichen durch, ohne Farbe noch FF und Tab.
            for farbe in [true, false] {
                let mut c = promptscan();
                c.args(["--lang", sprache, "-t", &prompt]);
                if farbe {
                    c.env("CLICOLOR_FORCE", "1");
                }
                let (code, aus, _) = lauf(&mut c);
                assert_eq!(code, Some(1), "{trenner:?} {sprache} {farbe}: {aus}");
                assert!(
                    aus.contains(&zeile),
                    "{trenner:?} {sprache} {farbe}: {aus:?}"
                );
                assert!(
                    !aus.contains(['\u{0B}', '\u{0C}', '\t']),
                    "{trenner:?} {sprache} {farbe}: {aus:?}"
                );
            }
        }
        // Im JSON bleibt grund.aus_text der Ausschnitt, wie er im Prompt steht; die
        // Erklärungszeilen sind dieselben wie im Text.
        let (code, aus, _) = lauf(promptscan().args(["--format", "json", "-t", &prompt]));
        assert_eq!(code, Some(1));
        let v = json_von(&aus);
        let name = v["funde"]
            .as_array()
            .and_then(|f| f.iter().find(|f| f["regel"] == "name"))
            .expect("Fund name");
        assert_eq!(name["grund"]["aus_text"], format!("meine{trenner}Klientin"));
        let erklaerung = name["erklaerung"]["de"].to_string();
        assert!(
            erklaerung.contains(&format!("«meine{gezeigt}Klientin»")),
            "{trenner:?}: {erklaerung}"
        );
    }
}

/// Bytes in UTF-32 mit BOM.
fn utf32(text: &str, le: bool) -> Vec<u8> {
    let mut bytes = if le {
        vec![0xFF, 0xFE, 0x00, 0x00]
    } else {
        vec![0x00, 0x00, 0xFE, 0xFF]
    };
    for c in text.chars() {
        bytes.extend(if le {
            u32::from(c).to_le_bytes()
        } else {
            u32::from(c).to_be_bytes()
        });
    }
    bytes
}

/// Bytes in UTF-16 ohne BOM.
fn utf16_ohne_bom(text: &str, le: bool) -> Vec<u8> {
    utf16(text, le)[2..].to_vec()
}

const NUR_ASCII: &str =
    "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, AHV 756.1234.5678.97.\r\n";
const MIT_UMLAUTEN: &str =
    "Ich bin Anwältin. Herr Hans Müller, AHV 756.1234.5678.97, ist arbeitsunfähig.";

/// UTF-16 ohne BOM ist bei reinem ASCII-Text gültiges UTF-8 mit einem Nullbyte nach jedem
/// Zeichen und ergäbe «Keine Funde» und Exit 0. Darum wird jede Eingabe mit dem Zeichen NUL
/// abgewiesen, mit Exit 2, auch mit `--fail-on never` und auch als JSON.
#[test]
fn eingabe_mit_nul_wird_abgewiesen() {
    let o = Ordner::neu("nul");
    let faelle: [(&str, Vec<u8>); 5] = [
        ("utf16le-ascii.txt", utf16_ohne_bom(NUR_ASCII, true)),
        ("utf16be-ascii.txt", utf16_ohne_bom(NUR_ASCII, false)),
        // Mit Umlauten ist es kein gültiges UTF-8; die Meldung nennt trotzdem UTF-16.
        ("utf16le-umlaute.txt", utf16_ohne_bom(MIT_UMLAUTEN, true)),
        (
            "utf8-mit-nul.txt",
            b"Hans Muster\0, AHV 756.1234.5678.97".to_vec(),
        ),
        ("utf16-bom-mit-nul.txt", utf16("Hans\0Muster", true)),
    ];
    for (name, bytes) in &faelle {
        let datei = o.datei(name, bytes);
        for (sprache, meldung) in [
            ("de", "enthält das Zeichen NUL (U+0000)"),
            ("en", "contains the character NUL (U+0000)"),
        ] {
            let (code, aus, fehler) = lauf(
                promptscan()
                    .args(["--lang", sprache, "--format", "json", "--fail-on", "never"])
                    .arg(&datei),
            );
            assert_eq!(code, Some(2), "{name}: {fehler}");
            assert!(aus.is_empty(), "{name}: {aus}");
            assert!(fehler.contains(meldung), "{name}: {fehler}");
        }
    }
    // Ohne BOM nennt die Meldung die wahrscheinliche Ursache, auch für stdin.
    let (code, aus, fehler) = lauf(promptscan().write_stdin(utf16_ohne_bom(NUR_ASCII, true)));
    assert_eq!(code, Some(2));
    assert!(aus.is_empty());
    assert_eq!(
        fehler,
        "promptscan: enthält das Zeichen NUL (U+0000), vermutlich UTF-16 ohne BOM; bitte als UTF-8 speichern\n"
    );
    let (code, _, fehler) = lauf(
        promptscan()
            .args(["--lang", "en"])
            .write_stdin(utf16_ohne_bom(NUR_ASCII, false)),
    );
    assert_eq!(code, Some(2));
    assert_eq!(
        fehler,
        "promptscan: contains the character NUL (U+0000), probably UTF-16 without BOM; please save as UTF-8\n"
    );
}

/// Das BOM von UTF-32LE (FF FE 00 00) beginnt wie das von UTF-16LE. Beide UTF-32-BOMs werden
/// vorher erkannt und abgewiesen, damit UTF-32 nicht als UTF-16 gelesen wird.
#[test]
fn utf32_wird_abgewiesen() {
    let o = Ordner::neu("utf32");
    for le in [true, false] {
        let datei = o.datei(
            if le { "utf32le.txt" } else { "utf32be.txt" },
            &utf32(MIT_UMLAUTEN, le),
        );
        let (code, aus, fehler) = lauf(promptscan().args(["--fail-on", "never"]).arg(&datei));
        assert_eq!(code, Some(2), "{fehler}");
        assert!(aus.is_empty(), "{aus}");
        assert!(
            fehler.ends_with(": UTF-32 wird nicht gelesen; bitte als UTF-8 speichern\n"),
            "{fehler}"
        );
        let (code, _, fehler) = lauf(
            promptscan()
                .args(["--lang", "en"])
                .write_stdin(utf32(MIT_UMLAUTEN, le)),
        );
        assert_eq!(code, Some(2));
        assert_eq!(
            fehler,
            "promptscan: UTF-32 is not read; please save as UTF-8\n"
        );
    }
}

const PS_RAT_DE: &str = "Windows PowerShell 5.1: vorher $OutputEncoding = [Text.UTF8Encoding]::new($false) setzen oder den Prompt als Datei übergeben";
const PS_RAT_EN: &str = "Windows PowerShell 5.1: first set $OutputEncoding = [Text.UTF8Encoding]::new($false) or pass the prompt as a file";

/// So kommt [`MIT_UMLAUTEN`] aus Windows PowerShell 5.1 an, wenn `$OutputEncoding` auf der
/// Vorgabe ASCII steht.
const AUS_DER_ASCII_PIPE: &str =
    "Ich bin Anw?ltin. Herr Hans M?ller, AHV 756.1234.5678.97, ist arbeitsunf?hig.\r\n";

/// Verdacht auf eine Pipe, die Umlaute zu «?» gemacht hat. Warnung auf stderr, Meldung
/// unter der Zusammenfassung und im JSON; der Exit-Code bleibt der der Funde («Hans» und
/// die AHV-Nummer, beide GELB).
#[test]
fn verdacht_auf_ascii_pipe_wird_gemeldet() {
    let verdacht_de = "Eingabe nur in ASCII, aber mit «?» im Wort («Anw?ltin»): Vermutlich hat eine Pipe Umlaute und Akzente zu «?» gemacht; Funde können fehlen";
    let verdacht_en = "Input is ASCII only but has «?» inside a word («Anw?ltin»): a pipe probably turned umlauts and accents into «?»; findings may be missing";
    for (schwelle, erwartet) in [("red", 0), ("yellow", 1), ("never", 0)] {
        let (code, aus, fehler) = lauf(
            promptscan()
                .args(["--fail-on", schwelle])
                .write_stdin(AUS_DER_ASCII_PIPE),
        );
        assert_eq!(code, Some(erwartet), "{schwelle}");
        assert_eq!(
            eingabe_abschnitt(&aus),
            einzeilig(&format!("{verdacht_de} {PS_RAT_DE}")),
            "{aus}"
        );
        assert_eq!(
            fehler,
            format!("promptscan: {verdacht_de}\npromptscan: {PS_RAT_DE}\n")
        );
    }
    let (code, aus, fehler) = lauf(
        promptscan()
            .args(["--lang", "en"])
            .write_stdin(AUS_DER_ASCII_PIPE),
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        eingabe_abschnitt(&aus),
        einzeilig(&format!("{verdacht_en} {PS_RAT_EN}")),
        "{aus}"
    );
    assert_eq!(
        fehler,
        format!("promptscan: {verdacht_en}\npromptscan: {PS_RAT_EN}\n")
    );
    let (code, aus, _) = lauf(
        promptscan()
            .args(["--format", "json"])
            .write_stdin(AUS_DER_ASCII_PIPE),
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        json_von(&aus)["eingabe"],
        json!({
            "unsichtbare_formatzeichen": 0,
            "ersatzzeichen": 0,
            "ascii_pipe": "Anw?ltin",
            "meldungen": {"de": [verdacht_de, PS_RAT_DE], "en": [verdacht_en, PS_RAT_EN]}
        })
    );
}

#[test]
fn ascii_pipe_gegenbeispiele() {
    for text in [
        "Was gilt? Bitte kurz.",
        "Siehe https://beispiel.ch/suche?q=abc und beispiel.ch/a?b",
        "Ich bin Anwältin? Anw?ltin.",
        NEUTRAL,
    ] {
        let (code, aus, fehler) = lauf(promptscan().args(["-t", text]));
        assert_eq!(code, Some(0), "{text}");
        assert!(fehler.is_empty(), "{text}: {fehler}");
        assert_eq!(eingabe_abschnitt(&aus), "", "{text}: {aus}");
        assert!(aus.contains(&format!("\n{KEINE_FUNDE}\n")), "{text}: {aus}");
        let (_, aus, _) = lauf(promptscan().args(["--format", "json", "-t", text]));
        assert_eq!(
            json_von(&aus)["eingabe"],
            json!({
                "unsichtbare_formatzeichen": 0,
                "ersatzzeichen": 0,
                "ascii_pipe": null,
                "meldungen": {"de": [], "en": []}
            }),
            "{text}"
        );
    }
}

/// U+FFFD steht, wo ein Programm beim Umwandeln ein Zeichen nicht lesen konnte.
#[test]
fn ersatzzeichen_werden_gemeldet() {
    let prompt = "Herr Hans M\u{FFFD}ller, AHV 756.1234.5678.97, ist arbeitsunf\u{FFFD}hig.";
    let de = "Eingabe enthält 2 Ersatzzeichen U+FFFD: Beim Umwandeln der Kodierung gingen vermutlich Zeichen verloren; Funde können fehlen";
    let en = "Input contains 2 replacement characters U+FFFD: characters were probably lost in an encoding conversion; findings may be missing";
    for (schwelle, erwartet) in [("red", 0), ("yellow", 1)] {
        let (code, aus, fehler) = lauf(promptscan().args(["--fail-on", schwelle, "-t", prompt]));
        assert_eq!(code, Some(erwartet));
        assert_eq!(eingabe_abschnitt(&aus), de, "{aus}");
        assert_eq!(fehler, format!("promptscan: {de}\n"));
    }
    let (_, aus, fehler) = lauf(promptscan().args(["--lang", "en", "-t", prompt]));
    assert_eq!(eingabe_abschnitt(&aus), en, "{aus}");
    assert_eq!(fehler, format!("promptscan: {en}\n"));
    let (_, aus, _) = lauf(promptscan().args(["--format", "json", "-t", prompt]));
    let v = json_von(&aus);
    assert_eq!(v["eingabe"]["ersatzzeichen"], json!(2));
    assert_eq!(v["eingabe"]["meldungen"], json!({"de": [de], "en": [en]}));
    // Einzahl
    let (_, aus, _) = lauf(promptscan().args(["-t", "M\u{FFFD}ller", "--lang", "en"]));
    assert_eq!(
        eingabe_abschnitt(&aus),
        "Input contains 1 replacement character U+FFFD: characters were probably lost in an encoding conversion; findings may be missing",
        "{aus}"
    );
}

/// Unsichtbare Formatzeichen, etwa das weiche Trennzeichen aus Word oder ein Leerzeichen
/// ohne Breite aus einem PDF. Wortlisten und Nummern lesen über sie
/// hinweg; die Meldung unter der Zusammenfassung sagt, dass sie da sind. Auf stderr
/// erscheint nichts, denn sie deuten nicht auf eine beschädigte Kodierung.
#[test]
fn unsichtbare_formatzeichen_werden_gemeldet() {
    let prompt = "Meine Pa\u{AD}tientin Erika Beispiel, AHV 756.1234.\u{200B}5678.97, ist arbeits\u{AD}unfähig.";
    let de = "Eingabe enthält 3 unsichtbare Formatzeichen (U+00AD, U+200B)";
    let en = "Input contains 3 invisible format characters (U+00AD, U+200B)";
    let (code, aus, fehler) = lauf(promptscan().args(["-t", prompt]));
    assert_eq!(code, Some(1), "{aus}");
    assert_eq!(eingabe_abschnitt(&aus), de, "{aus}");
    assert!(fehler.is_empty(), "{fehler}");
    let (_, aus, _) = lauf(promptscan().args(["--lang", "en", "-t", prompt]));
    assert_eq!(eingabe_abschnitt(&aus), en, "{aus}");
    let (_, aus, _) = lauf(promptscan().args(["--format", "json", "-t", prompt]));
    let v = json_von(&aus);
    assert_eq!(
        v["eingabe"],
        json!({
            "unsichtbare_formatzeichen": 3,
            "ersatzzeichen": 0,
            "ascii_pipe": null,
            "meldungen": {"de": [de], "en": [en]}
        })
    );
    let regeln: Vec<&str> = v["funde"]
        .as_array()
        .expect("Funde")
        .iter()
        .filter_map(|f| f["regel"].as_str())
        .collect();
    for regel in ["kontext", "ahv", "art5c-2"] {
        assert!(regeln.contains(&regel), "{regel} fehlt: {regeln:?}");
    }
    assert_eq!(v["lage"]["rolle"], "doctor");
    // Einzahl; mehr als vier Arten werden mit «…» abgekürzt.
    for (text, zeile) in [
        (
            "Mus\u{AD}ter",
            "Eingabe enthält 1 unsichtbares Formatzeichen (U+00AD)",
        ),
        (
            "a\u{AD}b\u{200B}c\u{200C}d\u{2060}e\u{FEFF}f\u{AD}g",
            "Eingabe enthält 6 unsichtbare Formatzeichen (U+00AD, U+200B, U+200C, U+2060, …)",
        ),
    ] {
        let (_, aus, _) = lauf(promptscan().args(["-t", text]));
        assert_eq!(eingabe_abschnitt(&aus), zeile, "{aus}");
    }
    let (_, aus, _) = lauf(promptscan().args(["--lang", "en", "-t", "Mus\u{AD}ter"]));
    assert_eq!(
        eingabe_abschnitt(&aus),
        "Input contains 1 invisible format character (U+00AD)",
        "{aus}"
    );
    // Gegenbeispiel: Eine Emoji-Folge mit Verbinder und Variantenwähler meldet nichts.
    let (_, aus, _) =
        lauf(promptscan().args(["-t", "Gruss \u{1F468}\u{200D}\u{1F469} \u{2764}\u{FE0F}"]));
    assert_eq!(eingabe_abschnitt(&aus), "", "{aus}");
    assert!(aus.contains(&format!("\n{KEINE_FUNDE}\n")), "{aus}");
}

/// Schliesst der Empfänger die Pipe vorzeitig (`| head`, `Select-Object -First`), endet
/// promptscan still mit dem Exit-Code der Funde, auch beim JSON-Bericht: keine Panik, kein
/// Exit 101.
#[test]
fn geschlossene_pipe_behaelt_den_exit_code() {
    use std::io::Read;
    use std::process::{Command as Prozess, Stdio};

    let o = Ordner::neu("pipe");
    let datei = o.datei(
        "viele.txt",
        "Hans Muster AHV 756.1234.5678.97\n".repeat(3000).as_bytes(),
    );
    let datei = datei.to_str().expect("Pfad in UTF-8");
    let faelle: [(&[&str], i32); 6] = [
        (&[datei, "--fail-on", "yellow"], 1),
        (&[datei, "--fail-on", "yellow", "--format", "json"], 1),
        (&[datei, "--format", "json"], 0),
        (&["rules"], 0),
        (&["explain", "rot"], 0),
        (&["explain", "ahv", "--format", "json"], 0),
    ];
    for (args, erwartet) in faelle {
        // Die Pipe sofort schliessen oder nach dem ersten Block.
        for sofort in [true, false] {
            let mut kind = Prozess::new(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME"))))
                .args(args)
                .env("PROMPTSCAN_CONFIG", keine_konfiguration())
                .env_remove("NO_COLOR")
                .env_remove("CLICOLOR_FORCE")
                .env_remove("CLICOLOR")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Programm starten");
            let mut aus = kind.stdout.take().expect("stdout");
            if !sofort {
                let mut block = [0_u8; 64];
                let gelesen = aus.read(&mut block).expect("ersten Block lesen");
                assert!(gelesen > 0, "{args:?}");
            }
            drop(aus);
            let ende = kind.wait_with_output().expect("auf das Programm warten");
            let fehler = String::from_utf8_lossy(&ende.stderr);
            assert_eq!(
                ende.status.code(),
                Some(erwartet),
                "{args:?} sofort={sofort}: {fehler}"
            );
            assert!(fehler.is_empty(), "{args:?} sofort={sofort}: {fehler}");
        }
    }
}

// --- Weiche Zeilenumbrüche ----------------------------------------------------------------

/// Prüft als JSON: Exit-Code, Bericht und die Funde kurz als «stufe regel «stelle»».
fn funde_json(prompt: &str) -> (Option<i32>, Value, Vec<String>) {
    let (code, aus, _) = lauf(promptscan().args(["--format", "json", "-t", prompt]));
    let v = json_von(&aus);
    let funde = v["funde"]
        .as_array()
        .expect("Funde")
        .iter()
        .map(|f| {
            format!(
                "{} {} «{}»",
                f["stufe"].as_str().expect("stufe"),
                f["regel"].as_str().expect("regel"),
                f["stelle"].as_str().expect("stelle")
            )
        })
        .collect();
    (code, v, funde)
}

/// Ein Zeilenumbruch mitten im Satz, wie ihn eine hart umbrochene E-Mail oder ein kopiertes
/// PDF enthält, trennt den Geheimnis-Kontext nicht mehr, in allen drei Sprachen und mit
/// CRLF. Bisher fielen Name und AHV-Nummer von ROT auf GELB, Exit 0 statt 1. Gegenstück
/// jedes Falls: Mit einer Leerzeile beginnt ein neuer Absatz, und der Kontext fehlt wie
/// bisher.
#[test]
fn weicher_umbruch_verbindet_den_geheimnis_kontext() {
    let faelle = [
        (
            "Meine\nMandantin Erika Beispiel, AHV 756.1234.5678.97, will das Erbe ausschlagen.",
            "Meine\nMandantin",
        ),
        (
            "Meine\r\nMandantin Erika Beispiel, AHV 756.1234.5678.97, will das Erbe ausschlagen.",
            "Meine\nMandantin",
        ),
        (
            "Ich bin\nAnwältin. Erika Beispiel, AHV 756.1234.5678.97, will das Erbe ausschlagen.",
            "Ich bin\nAnwältin",
        ),
        (
            "Je suis\navocate. Marie Exemple, AVS 756.1234.5678.97, veut répudier la succession.",
            "Je suis\navocate",
        ),
        (
            "Ma\ncliente Marie Exemple, AVS 756.1234.5678.97, veut répudier la succession.",
            "Ma\ncliente",
        ),
        (
            "Sono\navvocata. Giulia Esempio, AVS 756.1234.5678.97, vuole rinunciare all'eredità.",
            "Sono\navvocata",
        ),
        (
            "La mia\ncliente Giulia Esempio, AVS 756.1234.5678.97, vuole rinunciare all'eredità.",
            "mia\ncliente",
        ),
    ];
    for (prompt, kontext) in faelle {
        let (code, v, funde) = funde_json(prompt);
        assert_eq!(code, Some(1), "{prompt:?}: {funde:?}");
        assert_eq!(v["lage"]["herkunft"], "aus-text", "{prompt:?}");
        // Klientschaft und Vertretung ergeben «offen». Der weiche Umbruch
        // ändert die Rolle nicht gegenüber derselben Zeile ohne Umbruch.
        let einzeilig = prompt.lines().collect::<Vec<_>>().join(" ");
        let (_, v_einzeilig, _) = funde_json(&einzeilig);
        assert_eq!(
            v["lage"]["rolle"], v_einzeilig["lage"]["rolle"],
            "{prompt:?}"
        );
        assert!(
            matches!(v["lage"]["rolle"].as_str(), Some("lawyer" | "offen")),
            "{prompt:?}"
        );
        assert_eq!(v["lage"]["stelle"], kontext, "{prompt:?}");
        assert_eq!(v["hoechste_stufe"], "rot", "{prompt:?}");
        for fund in [
            format!("hinweis kontext «{kontext}»"),
            "rot ahv «756.1234.5678.97»".to_string(),
        ] {
            assert!(funde.contains(&fund), "{prompt:?}: {funde:?}");
        }
        let absatz = prompt.replacen('\n', "\n\n", 1);
        let (code, v, funde) = funde_json(&absatz);
        assert_eq!(code, Some(0), "{absatz:?}: {funde:?}");
        assert_eq!(v["lage"]["herkunft"], "keine", "{absatz:?}");
        assert!(
            funde.contains(&"gelb ahv «756.1234.5678.97»".to_string()),
            "{absatz:?}: {funde:?}"
        );
        assert!(
            funde.iter().all(|f| !f.contains(" kontext ")),
            "{absatz:?}: {funde:?}"
        );
    }
}

/// Begriffe aus mehreren Wörtern gelten über einen weichen Umbruch, und «arbeits-⏎unfähig»
/// wird für den Vergleich zusammengezogen; die Stelle umfasst den Umbruch. Steht der
/// Personenbezug in der Zeile davor, liegt er jetzt im selben Satz. Gegenstück jedes
/// Falls: Mit einer Leerzeile dazwischen gilt der Begriff nicht.
#[test]
fn weicher_umbruch_verbindet_begriffe_der_wortliste() {
    let faelle = [
        (
            "Meine Patientin Erika Beispiel leidet an einer psychischen\nStörung.",
            "rot art5c-2 «psychischen\nStörung»",
        ),
        (
            "Ma patiente Marie Exemple souffre de troubles\npsychiques.",
            "hinweis art5c-2 «troubles\npsychiques»",
        ),
        (
            "La mia paziente Giulia Esempio soffre di turbe\npsichiche.",
            "rot art5c-2 «turbe\npsichiche»",
        ),
        (
            "Mon client Jean Exemple est en incapacité de\ntravail depuis mars.",
            "rot art5c-2 «incapacité de\ntravail»",
        ),
        (
            "Il mio cliente Marco Esempio è in incapacità al\nlavoro da marzo.",
            "rot art5c-2 «incapacità al\nlavoro»",
        ),
        (
            "Mein Klient Hans Muster ist seit März arbeits-\nunfähig.",
            "rot art5c-2 «arbeits-\nunfähig»",
        ),
        // Bisher ohne Fund: «Mein Mandant» und «arbeitsunfähig» lagen in zwei Sätzen.
        (
            "Mein Mandant ist seit\nMärz arbeitsunfähig. Was gilt?",
            "hinweis art5c-2 «arbeitsunfähig»",
        ),
    ];
    for (prompt, fund) in faelle {
        let (_, _, funde) = funde_json(prompt);
        assert!(funde.contains(&fund.to_string()), "{prompt:?}: {funde:?}");
        let absatz = prompt.replacen('\n', "\n\n", 1);
        let (_, _, funde) = funde_json(&absatz);
        assert!(
            funde.iter().all(|f| !f.contains(" art5c-2 ")),
            "{absatz:?}: {funde:?}"
        );
    }
    // Der Trennstrich bleibt, wenn danach ein Grossbuchstabe steht.
    let (_, _, funde) = funde_json("Mein Klient Hans Muster ist seit März arbeits-\nUnfähig.");
    assert!(funde.iter().all(|f| !f.contains(" art5c-2 ")), "{funde:?}");
}

/// Gegenbeispiele: Listenpunkte bleiben getrennt, auch wenn der Punkt davor ohne
/// Satzzeichen endet; Adressblock, Signatur und Überschrift ergeben dieselben Funde wie vor
/// den weichen Umbrüchen. Grenze: Zeilen ohne Satzzeichen am Ende werden mit der nächsten
/// verbunden, auch in Adressblöcken, Signaturen und Überschriften. Hier entsteht daraus
/// kein neuer Fund und keine erschlossene Rolle.
#[test]
fn weicher_umbruch_gegenbeispiele() {
    let faelle: [(&str, &[&str]); 9] = [
        (
            "Bitte prüfe folgende Punkte:\n- meine\n- Mandantin Erika Beispiel, AHV 756.1234.5678.97",
            &["gelb name «Erika Beispiel»", "gelb ahv «756.1234.5678.97»"],
        ),
        (
            "Zu prüfen sind\n1. meine\n2. Mandantin Erika Beispiel, AHV 756.1234.5678.97",
            &["gelb name «Erika Beispiel»", "gelb ahv «756.1234.5678.97»"],
        ),
        (
            "Points à vérifier :\n- ma\n- cliente Marie Exemple, AVS 756.1234.5678.97",
            &["gelb ahv «756.1234.5678.97»"],
        ),
        (
            "Punti da verificare:\na) la mia\nb) cliente Giulia Esempio, AVS 756.1234.5678.97",
            &["gelb name «Giulia Esempio»", "gelb ahv «756.1234.5678.97»"],
        ),
        (
            "Erika Beispiel\nSeestrasse 12\n8002 Zürich\n\nBitte formuliere eine Mahnung.",
            &[
                "gelb name «Erika Beispiel»",
                "gelb adresse «Seestrasse 12\n8002 Zürich»",
            ],
        ),
        (
            "Bitte fasse den Entscheid zusammen.\n\nFreundliche Grüsse\nAnna Muster\nRechtsanwältin\nKanzlei Muster AG",
            &["gelb name «Anna Muster»"],
        ),
        (
            "Merci de résumer la décision.\n\nMeilleures salutations\nMarie Exemple\navocate",
            &[],
        ),
        (
            "Riassumi la sentenza.\n\nCordiali saluti\nGiulia Esempio\navvocata",
            &["gelb name «Giulia Esempio»"],
        ),
        (
            "VERTRAULICH\nUnsere Mitarbeiterin Erika Beispiel ist seit März krank.",
            &[
                "hinweis vermerk «VERTRAULICH»",
                "gelb name «Erika Beispiel»",
                // «krank» ist eine Gesundheitsangabe (Art. 5 lit. c).
                "orange art5c-2 «krank»",
            ],
        ),
    ];
    for (prompt, erwartet) in faelle {
        let (code, v, funde) = funde_json(prompt);
        assert_eq!(code, Some(0), "{prompt:?}: {funde:?}");
        assert_eq!(v["lage"]["herkunft"], "keine", "{prompt:?}");
        assert_eq!(funde, erwartet, "{prompt:?}");
    }
}

// --- Lizenzen -----------------------------------------------------------------------------

#[test]
fn licenses_gibt_lizenzstand_und_vermerke_aus() {
    let de = promptscan()
        .arg("licenses")
        .write_stdin(NEUTRAL)
        .assert()
        .success()
        .stderr(is_empty())
        .get_output()
        .stdout
        .clone();
    let de = String::from_utf8(de).expect("UTF-8");
    assert!(de.starts_with("promptscan: Lizenzstand und Vermerke zu fremden Bestandteilen."));
    // LICENSE.md
    assert!(de.contains("Alle Rechte vorbehalten. Für promptscan ist keine Lizenz erteilt."));
    assert!(de.contains("Zustimmung des Rechteinhabers."));
    // THIRD-PARTY-NOTICES.md: Crates, Standardbibliothek, MinGW-w64, Daten
    assert!(de.contains("# Vermerke zu fremden Bestandteilen"));
    assert!(de.contains("| `regex` |"));
    assert!(de.contains("### 8.2 Rust-Standardbibliothek"));
    assert!(de.contains("COPYING.MinGW-w64-runtime.txt"));
    assert!(de.contains("GCC RUNTIME LIBRARY EXCEPTION"));
    assert!(de.contains("Bundesamt für Landestopografie swisstopo"));
    // Der eingegebene Text wird nicht geprüft.
    assert!(!de.contains(KEINE_FUNDE));

    let en = ausgabe(promptscan().args(["licenses", "--lang", "en"]));
    assert!(en.starts_with("promptscan: licence status and third-party notices."));
    assert!(en.contains("All rights reserved. No licence is granted for promptscan."));
    // Nur der Kopf ist übersetzt; die Dateien folgen im Wortlaut.
    let ohne_kopf = |t: &str| t.split_once("\n\n").expect("Kopf").1.to_string();
    assert_eq!(ohne_kopf(&de), ohne_kopf(&en));
}
