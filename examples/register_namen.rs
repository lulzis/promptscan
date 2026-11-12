//! Erzeugt die Namensregister der Regel `name` aus amtlichen Rohdaten, offline.
//!
//! ```text
//! cargo run --example register_namen -- <rohdaten> [<zielordner>]
//! ```
//!
//! `<rohdaten>` enthält die Dateien unverändert, wie sie abgerufen wurden; Herkunft und
//! SHA-256 jeder Datei stehen im Kopf der Register. Ohne `<zielordner>` schreibt das
//! Programm nach `lexika/register/`. Es liest nur lokale Dateien und lädt nichts herunter.
//!
//! | Datei | Herkunft |
//! |---|---|
//! | `vornamen-maennlich-2025.csv` | BFS, Männliche Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 |
//! | `vornamen-weiblich-2025.csv` | BFS, Weibliche Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 |
//! | `nachnamen-sprachregion-2025.csv` | BFS, Nachnamen der ständigen Wohnbevölkerung nach Sprachregion, 2025 |
//! | `gemeinden-2026-11-06.csv` | BFS, Historisiertes Gemeindeverzeichnis, Momentaufnahme vom 06.11.2026 |
//! | `fedlex/fedlex-data-admin-ch-eli-cc-…-{de,fr,it}-html-….html` | Fedlex, Wortlaut von BV, ZGB, OR, StGB, StPO, ZPO, BGG, DSG |
//!
//! **Auswahl.** Je Name wird die Anzahl summiert: Vornamen über alle Jahrgänge und beide
//! Dateien, Nachnamen über alle Sprachregionen. Aufgenommen werden die häufigsten Namen,
//! bis sie zusammen mindestens 95 % (Vornamen) bzw. 90 % (Nachnamen) aller Nennungen
//! abdecken; Namen mit derselben Anzahl wie der letzte aufgenommene kommen mit. Zielwert der
//! Regel ist eine Trefferquote von mindestens 0,90 (SPEC 8). Der Vorname ist ihr
//! Hauptauslöser und erhält darum Spielraum nach oben; der Nachname bestätigt nur.
//!
//! **Mehrdeutigkeit.** Ein Name, der auch ein gewöhnliches Wort oder ein Ort ist, darf
//! nicht allein einen Namen auslösen. Belegt wird das je Name mit einer Fundstelle:
//!
//! * Spalte `gesetz`: erste Stelle im Wortlaut der acht Erlasse (Artikeltext ohne
//!   Fussnoten, Reihenfolge BV, ZGB, OR, StGB, StPO, ZPO, BGG, DSG, je DE, FR, IT). Ein
//!   Vorname zählt, wenn er dort als Wort in beliebiger Schreibung vorkommt; ein Nachname,
//!   wenn er dort grossgeschrieben und nicht am Satzanfang steht.
//! * Spalte `ort`: gleichnamiger Kanton, Bezirk oder gleichnamige Gemeinde; bei Vornamen
//!   zählt auch das erste Wort eines mehrteiligen Ortsnamens.
//!
//! `-` heisst: keine Fundstelle.

#![allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Die eigene SHA-256-Prüfsumme der Rohdaten bleibt, wie sie geprüft ist."
)]

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

/// Tag des Abrufs aller Rohdateien.
const ABGERUFEN: &str = "2026-11-06";
/// Veröffentlichung der drei BFS-Datensätze auf opendata.swiss («issued»).
const VEROEFFENTLICHT: &str = "2026-08-21";
/// Abdeckung in Prozent der Nennungen.
const VORNAMEN_PROZENT: u64 = 95;
const NACHNAMEN_PROZENT: u64 = 90;
/// Grenze zwischen Blöcken des Gesetzestextes (Absatz, Titel, Fussnote).
const ABSATZ: char = '\u{2029}';
const ABSATZ_TEXT: &str = "\u{2029}";

const DATEI_VORNAMEN_M: &str = "vornamen-maennlich-2025.csv";
const DATEI_VORNAMEN_W: &str = "vornamen-weiblich-2025.csv";
const DATEI_NACHNAMEN: &str = "nachnamen-sprachregion-2025.csv";
const DATEI_GEMEINDEN: &str = "gemeinden-2026-11-06.csv";

/// Ein Erlass, dessen Wortlaut die Mehrdeutigkeit eines Namens belegt.
struct Erlass {
    /// Teil des Fedlex-Dateinamens: ELI und Fassung.
    datei: &'static str,
    sr: &'static str,
    kurz: &'static str,
    fassung: &'static str,
}

const ERLASSE: [Erlass; 8] = [
    Erlass {
        datei: "1999-404-20240303",
        sr: "SR 101",
        kurz: "BV",
        fassung: "2024-03-03",
    },
    Erlass {
        datei: "24-233_245_233-20260701",
        sr: "SR 210",
        kurz: "ZGB",
        fassung: "2026-07-01",
    },
    Erlass {
        datei: "27-317_321_377-20261001",
        sr: "SR 220",
        kurz: "OR",
        fassung: "2026-10-01",
    },
    Erlass {
        datei: "54-757_781_799-20261101",
        sr: "SR 311.0",
        kurz: "StGB",
        fassung: "2026-11-01",
    },
    Erlass {
        datei: "2010-267-20250401",
        sr: "SR 312.0",
        kurz: "StPO",
        fassung: "2025-04-01",
    },
    Erlass {
        datei: "2010-262-20260701",
        sr: "SR 272",
        kurz: "ZPO",
        fassung: "2026-07-01",
    },
    Erlass {
        datei: "2006-218-20260401",
        sr: "SR 173.110",
        kurz: "BGG",
        fassung: "2026-04-01",
    },
    Erlass {
        datei: "2022-491-20250707",
        sr: "SR 235.1",
        kurz: "DSG",
        fassung: "2025-07-07",
    },
];

const SPRACHEN: [&str; 3] = ["de", "fr", "it"];

fn main() -> ExitCode {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    let Some(roh) = argumente.first().map(PathBuf::from) else {
        eprintln!("Aufruf: cargo run --example register_namen -- <rohdaten> [<zielordner>]");
        return ExitCode::from(2);
    };
    let ziel = argumente.get(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("lexika")
            .join("register")
    });
    match erzeugen(&roh, &ziel) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fehler: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Eine gelesene Rohdatei mit Prüfsumme.
struct Rohdatei {
    name: String,
    sha256: String,
    text: String,
}

fn lesen(ordner: &Path, name: &str) -> Result<Rohdatei, String> {
    let pfad = ordner.join(name);
    let bytes = fs::read(&pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    let sha256 = sha256_hex(&bytes);
    let text = String::from_utf8(bytes).map_err(|e| format!("{name}: kein UTF-8: {e}"))?;
    Ok(Rohdatei {
        name: name.to_string(),
        sha256,
        text,
    })
}

fn erzeugen(roh: &Path, ziel: &Path) -> Result<(), String> {
    let vornamen_m = lesen(roh, DATEI_VORNAMEN_M)?;
    let vornamen_w = lesen(roh, DATEI_VORNAMEN_W)?;
    let nachnamen = lesen(roh, DATEI_NACHNAMEN)?;
    let gemeinden = lesen(roh, DATEI_GEMEINDEN)?;

    let mut vor: HashMap<String, u64> = HashMap::new();
    for datei in [&vornamen_m, &vornamen_w] {
        let kopf = [
            "TIME_PERIOD",
            "firstname",
            "YEAROFBIRTH",
            "VALUE",
            "OBS_STATUS",
        ];
        for z in csv_zeilen(&datei.name, &datei.text, &kopf)? {
            *vor.entry(z[1].clone()).or_default() += zahl(&datei.name, &z[3])?;
        }
    }
    let mut nach: HashMap<String, u64> = HashMap::new();
    let kopf = [
        "TIME_PERIOD",
        "LASTNAME",
        "Sprachregion",
        "RANG_SPRACHREG",
        "VALUE",
        "OBS_STATUS",
    ];
    for z in csv_zeilen(&nachnamen.name, &nachnamen.text, &kopf)? {
        *nach.entry(z[1].clone()).or_default() += zahl(&nachnamen.name, &z[4])?;
    }

    let v = auswaehlen(&vor, VORNAMEN_PROZENT);
    let n = auswaehlen(&nach, NACHNAMEN_PROZENT);
    println!(
        "Vornamen: {} verschieden, {} Nennungen; Schwelle {}, aufgenommen {}, Abdeckung {}",
        v.verschieden,
        v.summe,
        v.schwelle,
        v.namen.len(),
        prozent(v.abgedeckt, v.summe)
    );
    println!(
        "Nachnamen: {} verschieden, {} Personen; Schwelle {}, aufgenommen {}, Abdeckung {}",
        n.verschieden,
        n.summe,
        n.schwelle,
        n.namen.len(),
        prozent(n.abgedeckt, n.summe)
    );

    let wortlaut = wortlaut(roh)?;
    println!(
        "Wortlaut: {} Dateien, {} Artikel, {} Wortformen, {} grossgeschrieben im Satz",
        wortlaut.dateien.len(),
        wortlaut.artikel,
        wortlaut.jede_form.len(),
        wortlaut.gross_mitte.len()
    );
    let orte = orte(&gemeinden)?;
    println!(
        "Gemeindeverzeichnis: {} Kantone, Bezirke und Gemeinden",
        orte.anzahl
    );

    let mut zeilen_v = Vec::new();
    let mut mehrdeutig_v = 0;
    for (name, anzahl) in &v.namen {
        let gesetz = wortlaut.jede_form.get(&name.to_lowercase());
        let ort = orte.ganz.get(name).or_else(|| orte.erstes.get(name));
        if gesetz.is_some() || ort.is_some() {
            mehrdeutig_v += 1;
        }
        zeilen_v.push([
            name.clone(),
            anzahl.to_string(),
            gesetz.cloned().unwrap_or_else(|| "-".into()),
            ort.cloned().unwrap_or_else(|| "-".into()),
        ]);
    }
    let mut zeilen_n = Vec::new();
    let mut mehrdeutig_n = 0;
    for (name, anzahl) in &n.namen {
        let gesetz = wortlaut.gross_mitte.get(name);
        let ort = orte.ganz.get(name);
        if gesetz.is_some() || ort.is_some() {
            mehrdeutig_n += 1;
        }
        zeilen_n.push([
            name.clone(),
            anzahl.to_string(),
            gesetz.cloned().unwrap_or_else(|| "-".into()),
            ort.cloned().unwrap_or_else(|| "-".into()),
        ]);
    }
    println!("Mehrdeutig: {mehrdeutig_v} Vornamen, {mehrdeutig_n} Nachnamen");

    let gesetze = ERLASSE
        .iter()
        .map(|e| format!("{} {} Fassung {}", e.sr, e.kurz, e.fassung))
        .collect::<Vec<_>>()
        .join("; ");
    let gesetze_sha = wortlaut
        .dateien
        .iter()
        .map(|(d, h)| format!("{d}={h}"))
        .collect::<Vec<_>>()
        .join("; ");
    let ortsverzeichnis = format!(
        "BFS, Historisiertes Gemeindeverzeichnis der Schweiz, Momentaufnahme vom 06.11.2026 \
         (https://www.agvchapp.bfs.admin.ch/api/communes/snapshot?date=06-11-2026), Datensatz \
         https://opendata.swiss/de/dataset/historisiertes-gemeindeverzeichnis-der-schweiz, \
         Nutzungsbedingung «Freie Nutzung» (https://opendata.swiss/terms-of-use#terms_open); \
         {}={}",
        gemeinden.name, gemeinden.sha256
    );
    let mehrdeutig_kopf = "Spalte gesetz: erste Stelle, an der das Wort im Wortlaut der \
         Erlasse steht (Artikeltext ohne Fussnoten; Reihenfolge BV, ZGB, OR, StGB, StPO, ZPO, \
         BGG, DSG, je de, fr, it). Spalte ort: gleichnamiger Kanton, Bezirk oder gleichnamige \
         Gemeinde. «-» = keine Fundstelle. Ein Name mit Fundstelle ist mehrdeutig und löst \
         allein keinen Namen aus.";
    let lizenz = "opendata.swiss, Nutzungsbedingung «Freie Nutzung. Quellenangabe ist \
         Pflicht.» (https://opendata.swiss/terms-of-use#terms_by): nicht kommerzielle und \
         kommerzielle Nutzung erlaubt, Quellenangabe Pflicht (Autor, Titel und Link zum \
         Datensatz). Quelle: BFS – STATPOP, © BFS 2026";

    let kopf_v: Vec<(&str, String)> = vec![
        (
            "titel",
            "Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 (männlich und weiblich, \
             über alle Jahrgänge summiert)"
                .into(),
        ),
        (
            "herausgeber",
            "Bundesamt für Statistik (BFS), Statistik der Bevölkerung und der Haushalte \
             (STATPOP)"
                .into(),
        ),
        (
            "url",
            "https://opendata.swiss/de/dataset/mannliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025"
                .into(),
        ),
        (
            "url-2",
            "https://opendata.swiss/de/dataset/weibliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025"
                .into(),
        ),
        (
            "rohdatei-url",
            "https://dam-api.bfs.admin.ch/hub/api/dam/assets/36752513/master (männlich, \
             ts-x-01.04.00.11); https://dam-api.bfs.admin.ch/hub/api/dam/assets/36752506/master \
             (weiblich, ts-x-01.04.00.10)"
                .into(),
        ),
        ("lizenz", lizenz.into()),
        ("veroeffentlicht", VEROEFFENTLICHT.into()),
        ("abgerufen", ABGERUFEN.into()),
        (
            "rohdatei-sha256",
            format!(
                "{}={}; {}={}",
                vornamen_m.name, vornamen_m.sha256, vornamen_w.name, vornamen_w.sha256
            ),
        ),
        (
            "filter",
            format!(
                "VALUE je Vorname über alle Jahrgänge und beide Dateien summiert ({} Nennungen, \
                 {} Vornamen; das BFS schliesst Vornamen mit weniger als 3 Nennungen aus). \
                 Aufgenommen sind die häufigsten Vornamen, bis sie zusammen mindestens {} % \
                 der Nennungen abdecken: alle mit mindestens {} Nennungen, {} Vornamen, \
                 Abdeckung {}.",
                v.summe,
                v.verschieden,
                VORNAMEN_PROZENT,
                v.schwelle,
                v.namen.len(),
                prozent(v.abgedeckt, v.summe)
            ),
        ),
        ("mehrdeutig", mehrdeutig_kopf.into()),
        (
            "gesetz-regel",
            "Vorname: das Wort in beliebiger Gross- und Kleinschreibung.".into(),
        ),
        ("gesetze", gesetze.clone()),
        ("gesetze-sha256", gesetze_sha.clone()),
        ("ortsverzeichnis", ortsverzeichnis.clone()),
        (
            "ort-regel",
            "Vorname: ganzer Ortsname ohne Kantonszusatz in Klammern, oder erstes Wort eines \
             mehrteiligen Ortsnamens."
                .into(),
        ),
        (
            "erzeugt",
            "cargo run --example register_namen -- <rohdaten> (examples/register_namen.rs)".into(),
        ),
        ("spalten", "vorname\tanzahl\tgesetz\tort".into()),
    ];
    let kopf_n: Vec<(&str, String)> = vec![
        (
            "titel",
            "Nachnamen der ständigen Wohnbevölkerung nach Sprachregion, 2025 (über alle \
             Sprachregionen summiert)"
                .into(),
        ),
        (
            "herausgeber",
            "Bundesamt für Statistik (BFS), Statistik der Bevölkerung und der Haushalte \
             (STATPOP)"
                .into(),
        ),
        (
            "url",
            "https://opendata.swiss/de/dataset/nachnamen-der-standigen-wohnbevolkerung-nach-sprachregion3"
                .into(),
        ),
        (
            "rohdatei-url",
            "https://dam-api.bfs.admin.ch/hub/api/dam/assets/36752522/master (ts-x-01.04.00.16)"
                .into(),
        ),
        ("lizenz", lizenz.into()),
        ("veroeffentlicht", VEROEFFENTLICHT.into()),
        ("abgerufen", ABGERUFEN.into()),
        (
            "rohdatei-sha256",
            format!("{}={}", nachnamen.name, nachnamen.sha256),
        ),
        (
            "filter",
            format!(
                "VALUE je Nachname über alle Sprachregionen summiert ({} Personen, {} \
                 Nachnamen; das BFS schliesst Nachnamen mit weniger als 3 Nennungen aus). \
                 Aufgenommen sind die häufigsten Nachnamen, bis sie zusammen mindestens {} % \
                 der Personen abdecken: alle mit mindestens {} Personen, {} Nachnamen, \
                 Abdeckung {}.",
                n.summe,
                n.verschieden,
                NACHNAMEN_PROZENT,
                n.schwelle,
                n.namen.len(),
                prozent(n.abgedeckt, n.summe)
            ),
        ),
        ("mehrdeutig", mehrdeutig_kopf.into()),
        (
            "gesetz-regel",
            "Nachname: genau so geschrieben, mit Grossbuchstaben am Anfang und nicht am \
             Satzanfang (nach . ! ? : ; oder am Blockanfang)."
                .into(),
        ),
        ("gesetze", gesetze),
        ("gesetze-sha256", gesetze_sha),
        ("ortsverzeichnis", ortsverzeichnis),
        (
            "ort-regel",
            "Nachname: ganzer Ortsname ohne Kantonszusatz in Klammern.".into(),
        ),
        (
            "erzeugt",
            "cargo run --example register_namen -- <rohdaten> (examples/register_namen.rs)".into(),
        ),
        ("spalten", "nachname\tanzahl\tgesetz\tort".into()),
    ];

    fs::create_dir_all(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let groesse_v = schreiben(&ziel.join("vornamen.tsv"), &kopf_v, &zeilen_v)?;
    let groesse_n = schreiben(&ziel.join("nachnamen.tsv"), &kopf_n, &zeilen_n)?;
    println!(
        "geschrieben: vornamen.tsv {} Zeilen, {groesse_v} Bytes; nachnamen.tsv {} Zeilen, {groesse_n} Bytes",
        zeilen_v.len(),
        zeilen_n.len()
    );
    for (d, h) in [
        (&vornamen_m.name, &vornamen_m.sha256),
        (&vornamen_w.name, &vornamen_w.sha256),
        (&nachnamen.name, &nachnamen.sha256),
        (&gemeinden.name, &gemeinden.sha256),
    ] {
        println!("sha256 {h}  {d}");
    }
    for (d, h) in &wortlaut.dateien {
        println!("sha256 {h}  {d}");
    }
    Ok(())
}

fn zahl(datei: &str, feld: &str) -> Result<u64, String> {
    feld.trim()
        .parse()
        .map_err(|e| format!("{datei}: «{feld}» ist keine Zahl: {e}"))
}

fn prozent(teil: u64, ganz: u64) -> String {
    // Zwei Nachkommastellen, kaufmännisch gerundet, ohne Gleitkomma.
    let hundertstel = (teil * 10_000 + ganz / 2) / ganz;
    format!("{},{:02} %", hundertstel / 100, hundertstel % 100)
}

/// Zeilen einer CSV-Datei mit Komma als Trenner und Anführungszeichen um Felder. Die
/// Kopfzeile muss genau `kopf` sein.
fn csv_zeilen(datei: &str, inhalt: &str, kopf: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let inhalt = inhalt.strip_prefix('\u{FEFF}').unwrap_or(inhalt);
    let mut zeilen = inhalt.lines().filter(|z| !z.trim().is_empty());
    let erste = zeilen.next().ok_or_else(|| format!("{datei}: leer"))?;
    let spalten = csv_felder(erste);
    if spalten != kopf {
        return Err(format!("{datei}: unerwartete Kopfzeile {spalten:?}"));
    }
    zeilen
        .enumerate()
        .map(|(i, z)| {
            let felder = csv_felder(z);
            if felder.len() == kopf.len() {
                Ok(felder)
            } else {
                Err(format!(
                    "{datei}, Datenzeile {}: {} statt {} Felder",
                    i + 1,
                    felder.len(),
                    kopf.len()
                ))
            }
        })
        .collect()
}

fn csv_felder(zeile: &str) -> Vec<String> {
    let zeile = zeile.trim_end_matches('\r');
    let mut felder = Vec::new();
    let mut feld = String::new();
    let mut in_anfuehrung = false;
    let mut zeichen = zeile.chars().peekable();
    while let Some(c) = zeichen.next() {
        match c {
            '"' if in_anfuehrung && zeichen.peek() == Some(&'"') => {
                feld.push('"');
                zeichen.next();
            }
            '"' => in_anfuehrung = !in_anfuehrung,
            ',' if !in_anfuehrung => felder.push(std::mem::take(&mut feld)),
            _ => feld.push(c),
        }
    }
    felder.push(feld);
    felder
}

/// Ergebnis der Auswahl nach Abdeckung.
struct Auswahl {
    /// Aufgenommene Namen, nach Anzahl absteigend, dann nach Name.
    namen: Vec<(String, u64)>,
    schwelle: u64,
    summe: u64,
    abgedeckt: u64,
    verschieden: usize,
}

fn auswaehlen(karte: &HashMap<String, u64>, prozent: u64) -> Auswahl {
    let mut alle: Vec<(String, u64)> = karte.iter().map(|(k, v)| (k.clone(), *v)).collect();
    alle.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let summe: u64 = alle.iter().map(|x| x.1).sum();
    let mut kumuliert = 0;
    let mut schwelle = 0;
    for (_, anzahl) in &alle {
        kumuliert += anzahl;
        if kumuliert * 100 >= prozent * summe {
            schwelle = *anzahl;
            break;
        }
    }
    let verschieden = alle.len();
    let namen: Vec<(String, u64)> = alle.into_iter().filter(|x| x.1 >= schwelle).collect();
    let abgedeckt = namen.iter().map(|x| x.1).sum();
    Auswahl {
        namen,
        schwelle,
        summe,
        abgedeckt,
        verschieden,
    }
}

/// Wortschatz der Erlasse mit der ersten Fundstelle je Wort.
struct Wortlaut {
    /// Kleingeschriebenes Wort → erste Fundstelle in beliebiger Schreibung.
    jede_form: HashMap<String, String>,
    /// Wort mit Grossbuchstaben am Anfang, nicht am Satzanfang → erste Fundstelle.
    gross_mitte: HashMap<String, String>,
    artikel: usize,
    /// Gelesene Dateien mit SHA-256, in Lesereihenfolge.
    dateien: Vec<(String, String)>,
}

/// Reguläre Ausdrücke für den Artikeltext.
struct Ausdruecke {
    artikel: Regex,
    fussnoten: Regex,
    hochgestellt: Regex,
    block: Regex,
    tag: Regex,
    wort: Regex,
}

fn wortlaut(roh: &Path) -> Result<Wortlaut, String> {
    let ordner = roh.join("fedlex");
    let mut vorhanden: Vec<String> = fs::read_dir(&ordner)
        .map_err(|e| format!("{}: {e}", ordner.display()))?
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    vorhanden.sort();
    let re = Ausdruecke {
        artikel: Regex::new(r#"<article id="(art_[^"]+)""#).expect("Ausdruck"),
        fussnoten: Regex::new(r#"(?s)<div class="footnotes">.*?</div>"#).expect("Ausdruck"),
        hochgestellt: Regex::new(r"(?s)<sup>.*?</sup>").expect("Ausdruck"),
        block: Regex::new(r"</?(?:p|dd|dt|dl|div|h\d|br|li|td|tr|table|article)\b[^>]*>")
            .expect("Ausdruck"),
        tag: Regex::new(r"<[^>]+>").expect("Ausdruck"),
        wort: Regex::new(r"[\p{L}\p{N}]+(?:['\x{2019}]\p{L}+)*").expect("Ausdruck"),
    };
    let mut aus = Wortlaut {
        jede_form: HashMap::new(),
        gross_mitte: HashMap::new(),
        artikel: 0,
        dateien: Vec::new(),
    };
    for erlass in &ERLASSE {
        for sprache in SPRACHEN {
            let praefix = format!(
                "fedlex-data-admin-ch-eli-cc-{}-{sprache}-html-",
                erlass.datei
            );
            let passend: Vec<&String> = vorhanden
                .iter()
                .filter(|d| d.starts_with(&praefix) && d.ends_with(".html"))
                .collect();
            let [datei] = passend.as_slice() else {
                return Err(format!(
                    "{}: erwartet genau eine Datei {praefix}…, gefunden {}",
                    ordner.display(),
                    passend.len()
                ));
            };
            let gelesen = lesen(&ordner, datei)?;
            aus.dateien
                .push((format!("fedlex/{}", gelesen.name), gelesen.sha256.clone()));
            let html = gelesen.text;
            let anfaenge: Vec<(usize, String)> = re
                .artikel
                .captures_iter(&html)
                .filter_map(|c| Some((c.get(0)?.start(), c.get(1)?.as_str().to_string())))
                .collect();
            for (start, id) in anfaenge {
                let ende = html[start..]
                    .find("</article>")
                    .map_or(html.len(), |i| start + i);
                let text = artikeltext(&html[start..ende], &re);
                aus.artikel += 1;
                let fundstelle = format!("{} {} ({sprache})", erlass.sr, zitat(&id));
                for m in re.wort.find_iter(&text) {
                    let w = m.as_str();
                    aus.jede_form
                        .entry(w.to_lowercase())
                        .or_insert_with(|| fundstelle.clone());
                    if w.chars().next().is_some_and(char::is_uppercase)
                        && !satzanfang(&text, m.start())
                    {
                        aus.gross_mitte
                            .entry(w.to_string())
                            .or_insert_with(|| fundstelle.clone());
                    }
                }
            }
        }
    }
    Ok(aus)
}

/// Text eines Artikels: Fussnoten und hochgestellte Zeichen entfernt, Blöcke getrennt,
/// Zeichenreferenzen aufgelöst, Normalform C.
fn artikeltext(html: &str, re: &Ausdruecke) -> String {
    let t = re.fussnoten.replace_all(html, ABSATZ_TEXT);
    let t = re.hochgestellt.replace_all(&t, " ");
    let t = re.block.replace_all(&t, ABSATZ_TEXT);
    let t = re.tag.replace_all(&t, "");
    entitaeten(&t).nfc().collect()
}

/// Löst `&nbsp;`, `&amp;`, `&quot;`, `&lt;`, `&gt;`, `&apos;` und numerische Referenzen auf.
fn entitaeten(s: &str) -> String {
    let mut aus = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        aus.push_str(&rest[..i]);
        let nach = &rest[i + 1..];
        let zeichen = nach.find(';').filter(|j| *j <= 10).and_then(|j| {
            let name = &nach[..j];
            let z = match name {
                "nbsp" => Some(' '),
                "amp" => Some('&'),
                "quot" => Some('"'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "apos" => Some('\''),
                _ => {
                    if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                    } else if let Some(dez) = name.strip_prefix('#') {
                        dez.parse().ok().and_then(char::from_u32)
                    } else {
                        None
                    }
                }
            };
            z.map(|z| (z, j))
        });
        match zeichen {
            Some((z, j)) => {
                aus.push(z);
                rest = &nach[j + 1..];
            }
            None => {
                aus.push('&');
                rest = nach;
            }
        }
    }
    aus.push_str(rest);
    aus
}

/// Steht das Wort ab `start` am Satz- oder Blockanfang?
fn satzanfang(text: &str, start: usize) -> bool {
    for c in text[..start].chars().rev() {
        if c == ABSATZ {
            return true;
        }
        if c.is_whitespace() {
            continue;
        }
        return matches!(c, '.' | '!' | '?' | ':' | ';');
    }
    true
}

/// `art_153_b` → `Art. 153b`.
fn zitat(id: &str) -> String {
    let teile: Vec<&str> = id.trim_start_matches("art_").split('_').collect();
    format!("Art. {}", teile.concat())
}

/// Orte aus dem Gemeindeverzeichnis.
struct Orte {
    /// Ganzer Name ohne Kantonszusatz → Beschreibung, z. B. «Gemeinde 441 Renan (BE)».
    ganz: HashMap<String, String>,
    /// Erstes Wort eines mehrteiligen Namens → Beschreibung.
    erstes: HashMap<String, String>,
    anzahl: usize,
}

fn orte(datei: &Rohdatei) -> Result<Orte, String> {
    let kopf = [
        "HistoricalCode",
        "BfsCode",
        "ValidFrom",
        "ValidTo",
        "Level",
        "Parent",
        "Name",
        "ShortName",
        "Inscription",
        "Radiation",
        "Rec_Type_fr",
        "Rec_Type_de",
    ];
    let mut aus = Orte {
        ganz: HashMap::new(),
        erstes: HashMap::new(),
        anzahl: 0,
    };
    for z in csv_zeilen(&datei.name, &datei.text, &kopf)? {
        let (art, name) = match z[4].as_str() {
            "1" => ("Kanton", &z[6]),
            "2" => ("Bezirk", &z[7]),
            "3" => ("Gemeinde", &z[6]),
            _ => continue,
        };
        aus.anzahl += 1;
        let beschreibung = format!("{art} {} {name}", z[1]);
        let ohne = ohne_kantonszusatz(name);
        if let Some((erstes, _)) = ohne.split_once(' ') {
            aus.erstes
                .entry(erstes.to_string())
                .or_insert_with(|| beschreibung.clone());
        }
        aus.ganz.entry(ohne.to_string()).or_insert(beschreibung);
    }
    Ok(aus)
}

/// «Wald (ZH)» → «Wald».
fn ohne_kantonszusatz(name: &str) -> &str {
    match name.rfind(" (") {
        Some(i) if name.ends_with(')') => &name[..i],
        _ => name,
    }
}

/// Schreibt ein Register; gibt die Grösse in Bytes zurück.
fn schreiben(
    pfad: &Path,
    kopf: &[(&str, String)],
    zeilen: &[[String; 4]],
) -> Result<usize, String> {
    let mut aus = String::new();
    for (k, w) in kopf {
        if w.contains('\n') || (*k != "spalten" && w.contains('\t')) {
            return Err(format!(
                "Kopfzeile {k} enthält Zeilenumbruch oder Tabulator"
            ));
        }
        let _ = writeln!(aus, "# {k}: {w}");
    }
    for z in zeilen {
        if z.iter()
            .any(|f| f.trim().is_empty() || f.contains('\t') || f.contains('\n'))
        {
            return Err(format!("ungültiges Feld in {z:?}"));
        }
        let _ = writeln!(aus, "{}", z.join("\t"));
    }
    fs::write(pfad, &aus).map_err(|e| format!("{}: {e}", pfad.display()))?;
    Ok(aus.len())
}

/// SHA-256 nach FIPS 180-4, ausgeschrieben, damit das Programm keine weitere Bibliothek
/// braucht.
fn sha256_hex(daten: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bits = (daten.len() as u64).wrapping_mul(8);
    let mut nachricht = daten.to_vec();
    nachricht.push(0x80);
    while nachricht.len() % 64 != 56 {
        nachricht.push(0);
    }
    nachricht.extend_from_slice(&bits.to_be_bytes());
    for block in nachricht.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (ziel, wort) in w.iter_mut().zip(block.chunks_exact(4)) {
            *ziel = u32::from_be_bytes([wort[0], wort[1], wort[2], wort[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for (k, wi) in K.iter().zip(w.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*k)
                .wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().fold(String::with_capacity(64), |mut s, x| {
        let _ = write!(s, "{x:08x}");
        s
    })
}
