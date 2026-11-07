//! Leitet die Register der Regeln `adresse` und `wohnort` aus den amtlichen Rohdaten ab.
//!
//! ```text
//! cargo run --example register_orte -- <ROHDATEN> [<ZIEL>]
//! ```
//!
//! `<ROHDATEN>` enthält die unveränderten Downloads unter den Pfaden der Konstanten
//! unten; `<ZIEL>` ist vorgegeben `lexika/register` im Projektordner. Das Programm
//! braucht kein Netz, prüft die SHA-256-Summe jeder Rohdatei und schreibt aus gleichen
//! Rohdaten byte-gleiche Register (sortiert, Zeilenende LF). Weicht eine Summe ab, bricht
//! es ab: Neue Rohdaten brauchen ein neues Abrufdatum und neue Summen in diesem Programm.
//!
//! * `plz.tsv`: Paare aus Postleitzahl und Ortschaft (amtliches Ortschaftenverzeichnis).
//! * `gemeinden.tsv`: die politischen Gemeinden (amtliches Gemeindeverzeichnis des BFS).
//! * `strassentypen.tsv`: Strassentypen, Vorwörter und Verbindungswörter, abgeleitet aus
//!   dem amtlichen Strassenverzeichnis, mit gemessener Abdeckung.

#![allow(
    clippy::chunks_exact_to_as_chunks,
    reason = "Die eigene SHA-256-Prüfsumme der Rohdaten bleibt, wie sie geprüft ist."
)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use promptscan::lexikon::falten;

/// Tag des Abrufs aller Rohdateien.
const ABGERUFEN: &str = "2026-11-06";

/// Eine Rohdatei unter `<ROHDATEN>` mit ihrer Prüfsumme beim Abruf.
struct Roh {
    pfad: &'static str,
    sha256: &'static str,
}

const OVZ_ARCHIV: Roh = Roh {
    pfad: "ortschaften/ortschaftenverzeichnis_plz_2056.csv.zip",
    sha256: "f7a99833d11e773916a57baebe7374040f4fbde0c8821250c77fa7197a176978",
};
const OVZ: Roh = Roh {
    pfad: "ortschaften/entpackt/AMTOVZ_CSV_LV95/AMTOVZ_CSV_LV95.csv",
    sha256: "0710c74be9b072d813ed294a6d8e1383bf44fd66dc62237e0d0f08b331178fe0",
};
const STR_CH_ARCHIV: Roh = Roh {
    pfad: "strassen/amtliches-strassenverzeichnis_ch_2056.csv.zip",
    sha256: "142e1244ad40a539a3e41a64e9ff6b60e721dfe6b57438b7a333af7fb0ef89bf",
};
const STR_CH: Roh = Roh {
    pfad: "strassen/entpackt/amtliches-strassenverzeichnis_ch_2056.csv",
    sha256: "40fe097c92dd529f4d42a9f8d4331919c457532e1687ff9869e194f579fad52e",
};
const STR_LI_ARCHIV: Roh = Roh {
    pfad: "strassen/amtliches-strassenverzeichnis_li_2056.csv.zip",
    sha256: "4ead44e57ad78653862fc8d9fcd7fa99365f08984721c4486ccf7022158c5659",
};
const STR_LI: Roh = Roh {
    pfad: "strassen/entpackt/amtliches-strassenverzeichnis_li_2056.csv",
    sha256: "adc6d60b09f73d4d0609dd590b42344283f199d3f5f8809f645426db6acab7d7",
};
const AGV: Roh = Roh {
    pfad: "gemeinden/agv-snapshot-06-11-2026.csv",
    sha256: "e0748c06c754dbde20f04425a7f58c2a718e77982ea10fb8f969ac9d21a83b7d",
};

const URL_OVZ: &str = "https://data.geo.admin.ch/ch.swisstopo-vd.ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz_2056.csv.zip";
const URL_STR_CH: &str = "https://data.geo.admin.ch/ch.swisstopo.amtliches-strassenverzeichnis/amtliches-strassenverzeichnis_ch/amtliches-strassenverzeichnis_ch_2056.csv.zip";
const URL_STR_LI: &str = "https://data.geo.admin.ch/ch.swisstopo.amtliches-strassenverzeichnis/amtliches-strassenverzeichnis_li/amtliches-strassenverzeichnis_li_2056.csv.zip";
const URL_AGV: &str = "https://www.agvchapp.bfs.admin.ch/api/communes/snapshot?date=06-11-2026";
const URL_AGV_DATENSATZ: &str =
    "https://opendata.swiss/de/dataset/historisiertes-gemeindeverzeichnis-der-schweiz";

/// Wortlaut der Nutzungsbedingungen, Abschnitt 2 (Version 1.3.2021, veröffentlicht am
/// 26.08.2022), gelesen am Abruftag.
const LIZENZ_SWISSTOPO: &str = "Nutzungsbedingungen für kostenlose Geodaten und Geodienste (OGD) von swisstopo, https://www.swisstopo.admin.ch/de/nutzungsbedingungen-kostenlose-geodaten-und-geodienste, Abschnitt 2: «Die kostenlosen Geodaten und Geodienste von swisstopo dürfen genutzt, verbreitet und zugänglich gemacht werden. Weiter dürfen sie angereichert, bearbeitet sowie auch kommerziell genutzt werden. Eine Quellenangabe ist Pflicht.»";
const QUELLENANGABE_SWISSTOPO: &str = "Bundesamt für Landestopografie swisstopo";
/// Wortlaut der Nutzungsbedingung «Freie Nutzung» von opendata.swiss, die der Datensatz nennt.
const LIZENZ_BFS: &str = "opendata.swiss, Freie Nutzung, https://opendata.swiss/terms-of-use#terms_open: «Sie dürfen diesen Datensatz für nicht kommerzielle Zwecke nutzen. Sie dürfen diesen Datensatz für kommerzielle Zwecke nutzen. Eine Quellenangabe wird empfohlen (Autor, Titel und Link zum Datensatz).»";

/// Wo ein Wort im Strassennamen steht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Stellung {
    /// Endung des letzten Worts (deutsch: «Seestrasse»).
    Ende,
    /// Erstes Wort oder letzter Bindestrich-Teil (französisch, italienisch, rätoromanisch).
    Anfang,
    /// Grossgeschriebenes Adjektiv vor einem deutschen Typ («Alte Landstrasse»).
    Vorwort,
    /// Kleingeschriebenes Wort im Namen («Route de la Gare»).
    Verbindung,
}

impl Stellung {
    fn name(self) -> &'static str {
        match self {
            Stellung::Ende => "ende",
            Stellung::Anfang => "anfang",
            Stellung::Vorwort => "vorwort",
            Stellung::Verbindung => "verbindung",
        }
    }
}

/// Worauf die Aufnahme eines Worts beruht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Grundlage {
    /// GeoNV Art. 3 lit. f in der Fassung der jeweiligen Sprache.
    GeoNv,
    /// Empfehlung Gebäudeadressierung 2018, Kap. 3, Kurznamen von Strassen.
    Empfehlung,
    /// Häufigkeit im Verzeichnis selbst, mindestens 0,01 % der Namen.
    Verzeichnis,
}

impl Grundlage {
    fn name(self) -> &'static str {
        match self {
            Grundlage::GeoNv => "geonv",
            Grundlage::Empfehlung => "empfehlung",
            Grundlage::Verzeichnis => "verzeichnis",
        }
    }
}

use Grundlage::{Empfehlung, GeoNv, Verzeichnis};
use Stellung::{Anfang, Ende};

/// Die Strassentypen.
///
/// * GeoNV Art. 3 lit. f: «Strassen, Wege, Gassen, Plätze» (de), «rues, routes, chemins,
///   ruelles, places» (fr), «vie, strade, vicoli, piazze» (it).
/// * Empfehlung 2018, S. 9: Typen, die im Kurznamen abgekürzt werden, mit Abkürzung.
///   «V.» für «Via» fehlt, weil es im Text meist anderes bedeutet («v.» für «von»).
/// * Verzeichnis: Gattungswörter von Verkehrswegen und Plätzen, die in mindestens 0,01 %
///   der Namen vorkommen (das Programm prüft die Schwelle). Rätoromanische Typen gibt es
///   nur hier; die Empfehlung gilt für Rumantsch «sinngemäss» und nennt keine Beispiele.
const TYPEN: &[(&str, Stellung, Grundlage)] = &[
    ("strasse", Ende, GeoNv),
    ("weg", Ende, GeoNv),
    ("gasse", Ende, GeoNv),
    ("platz", Ende, GeoNv),
    ("rue", Anfang, GeoNv),
    ("route", Anfang, GeoNv),
    ("chemin", Anfang, GeoNv),
    ("ruelle", Anfang, GeoNv),
    ("place", Anfang, GeoNv),
    ("via", Anfang, GeoNv),
    ("strada", Anfang, GeoNv),
    ("vicolo", Anfang, GeoNv),
    ("piazza", Anfang, GeoNv),
    ("str.", Ende, Empfehlung),
    ("avenue", Anfang, Empfehlung),
    ("av.", Anfang, Empfehlung),
    ("boulevard", Anfang, Empfehlung),
    ("bd", Anfang, Empfehlung),
    ("ch.", Anfang, Empfehlung),
    ("escaliers", Anfang, Empfehlung),
    ("esc.", Anfang, Empfehlung),
    ("impasse", Anfang, Empfehlung),
    ("imp.", Anfang, Empfehlung),
    ("passage", Anfang, Empfehlung),
    ("pass.", Anfang, Empfehlung),
    ("promenade", Anfang, Empfehlung),
    ("prom.", Anfang, Empfehlung),
    ("rte", Anfang, Empfehlung),
    ("rlle", Anfang, Empfehlung),
    ("espace", Anfang, Empfehlung),
    ("esp.", Anfang, Empfehlung),
    ("esplanade", Anfang, Empfehlung),
    ("espl.", Anfang, Empfehlung),
    ("faubourg", Anfang, Empfehlung),
    ("fbg", Anfang, Empfehlung),
    ("galeries", Anfang, Empfehlung),
    ("gal.", Anfang, Empfehlung),
    ("pl.", Anfang, Empfehlung),
    ("sentier", Anfang, Empfehlung),
    ("sent.", Anfang, Empfehlung),
    ("terrasse", Anfang, Empfehlung),
    ("terr.", Anfang, Empfehlung),
    ("p.za", Anfang, Empfehlung),
    ("viale", Anfang, Empfehlung),
    ("v.le", Anfang, Empfehlung),
    ("gässli", Ende, Verzeichnis),
    ("steig", Ende, Verzeichnis),
    ("gass", Ende, Verzeichnis),
    ("ring", Ende, Verzeichnis),
    ("wegli", Ende, Verzeichnis),
    ("stutz", Ende, Verzeichnis),
    ("weglein", Ende, Verzeichnis),
    ("allee", Ende, Verzeichnis),
    ("promenade", Ende, Verzeichnis),
    ("treppe", Ende, Verzeichnis),
    ("gässchen", Ende, Verzeichnis),
    ("quai", Ende, Verzeichnis),
    ("gässlein", Ende, Verzeichnis),
    ("pfad", Ende, Verzeichnis),
    ("allée", Anfang, Verzeichnis),
    ("quai", Anfang, Verzeichnis),
    ("sentée", Anfang, Verzeichnis),
    ("sentee", Anfang, Verzeichnis),
    ("square", Anfang, Verzeichnis),
    ("traverse", Anfang, Verzeichnis),
    ("sentiero", Anfang, Verzeichnis),
    ("salita", Anfang, Verzeichnis),
    ("contrada", Anfang, Verzeichnis),
    ("piazzetta", Anfang, Verzeichnis),
    ("piazzale", Anfang, Verzeichnis),
    ("stráda", Anfang, Verzeichnis),
    ("strecia", Anfang, Verzeichnis),
    ("strécia", Anfang, Verzeichnis),
    ("caraa", Anfang, Verzeichnis),
    ("cará", Anfang, Verzeichnis),
    ("riva", Anfang, Verzeichnis),
    ("vietta", Anfang, Verzeichnis),
    ("scalinata", Anfang, Verzeichnis),
    ("veia", Anfang, Verzeichnis),
    ("senda", Anfang, Verzeichnis),
    ("voia", Anfang, Verzeichnis),
    ("voa", Anfang, Verzeichnis),
    ("gassa", Anfang, Verzeichnis),
];

/// Häufige Wörter, die bewusst kein Typ sind, mit Grund. Ihre Anzahl steht im Kopf.
const AUSGESCHLOSSEN: &[(&str, Stellung, &str)] = &[
    ("hof", Ende, "Flur- oder Gebietsname"),
    ("berg", Ende, "Flur- oder Gebietsname"),
    ("matt", Ende, "Flur- oder Gebietsname"),
    ("rain", Ende, "Flur- oder Gebietsname"),
    ("acker", Ende, "Flur- oder Gebietsname"),
    ("halde", Ende, "Flur- oder Gebietsname"),
    ("feld", Ende, "Flur- oder Gebietsname"),
    ("zelg", Ende, "Flurname"),
    ("winkel", Ende, "Flurname"),
    ("graben", Ende, "auch Geländeform"),
    ("park", Ende, "auch Anlage oder Parkplatz"),
    ("anlage", Ende, "auch Einrichtung"),
    ("brücke", Ende, "Bauwerk"),
    ("steg", Ende, "Bauwerk"),
    ("clos", Anfang, "benanntes Gebiet"),
    ("passerelle", Anfang, "Bauwerk"),
    ("cour", Anfang, "auch «Gericht»"),
    ("v.", Anfang, "mehrdeutig"),
];

fn main() -> ExitCode {
    match ausfuehren() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("register_orte: {e}");
            ExitCode::FAILURE
        }
    }
}

fn ausfuehren() -> Result<(), String> {
    let mut argumente = std::env::args_os().skip(1);
    let roh = PathBuf::from(
        argumente
            .next()
            .ok_or("Aufruf: register_orte <ROHDATEN> [<ZIEL>]")?,
    );
    let ziel = argumente.next().map(PathBuf::from).unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("lexika")
            .join("register")
    });
    for archiv in [&OVZ_ARCHIV, &STR_CH_ARCHIV, &STR_LI_ARCHIV] {
        let pfad = roh.join(archiv.pfad);
        let bytes = fs::read(&pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
        pruefsumme(archiv, &bytes)?;
    }
    plz(&roh, &ziel)?;
    gemeinden(&roh, &ziel)?;
    strassentypen(&roh, &ziel)?;
    Ok(())
}

/// `plz.tsv`: jedes Paar aus PLZ4 und Ortschaftsname einmal.
fn plz(roh: &Path, ziel: &Path) -> Result<(), String> {
    let inhalt = lesen(roh, &OVZ)?;
    let zeilen = spalten(
        &inhalt,
        ';',
        OVZ.pfad,
        &["PLZ4", "Ortschaftsname", "Kantonskürzel"],
    )?;
    let mut paare = BTreeSet::new();
    let mut liechtenstein = 0;
    for z in &zeilen {
        let (plz, ort) = (z[0], z[1]);
        if plz.len() != 4 || !plz.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{}: PLZ «{plz}» ist nicht vierstellig", OVZ.pfad));
        }
        if ort.trim().is_empty() {
            return Err(format!("{}: leerer Ortschaftsname", OVZ.pfad));
        }
        if z[2].is_empty() {
            liechtenstein += 1;
        }
        paare.insert((plz, ort));
    }
    let mut text = kopf(&[
        (
            "titel",
            "Amtliches Ortschaftenverzeichnis mit Postleitzahl und Perimeter".into(),
        ),
        ("herausgeber", "Bundesamt für Landestopografie swisstopo".into()),
        ("url", URL_OVZ.into()),
        ("lizenz", LIZENZ_SWISSTOPO.into()),
        ("quellenangabe", QUELLENANGABE_SWISSTOPO.into()),
        ("abgerufen", ABGERUFEN.into()),
        ("archiv-sha256", OVZ_ARCHIV.sha256.into()),
        (
            "rohdatei",
            "AMTOVZ_CSV_LV95/AMTOVZ_CSV_LV95.csv aus dem Archiv".into(),
        ),
        ("rohdatei-sha256", OVZ.sha256.into()),
        (
            "grundlage",
            "GeoNV (SR 510.625) Art. 24: «Das Bundesamt für Landestopografie erstellt, verwaltet und veröffentlicht das amtliche Ortschaftenverzeichnis mit Postleitzahl und Perimeter.»"
                .into(),
        ),
        (
            "filter",
            format!(
                "alle {} Zeilen (davon {liechtenstein} in Liechtenstein, Kantonskürzel leer); je Paar aus PLZ4 und Ortschaftsname eine Zeile; Zusatzziffer, Gemeinde, Koordinaten und Sprache weggelassen",
                zeilen.len()
            ),
        ),
        ("zeilen", paare.len().to_string()),
        ("spalten", "plz\tortschaft".into()),
    ]);
    for &(plz, ort) in &paare {
        zeile(&mut text, &[plz, ort]);
    }
    schreiben(ziel, "plz.tsv", &text)
}

/// `gemeinden.tsv`: die politischen Gemeinden (Level 3) mit amtlichem Namen.
fn gemeinden(roh: &Path, ziel: &Path) -> Result<(), String> {
    let inhalt = lesen(roh, &AGV)?;
    let zeilen = spalten(&inhalt, ',', AGV.pfad, &["Level", "Name", "ShortName"])?;
    let mut namen = BTreeSet::new();
    let mut kantone = BTreeSet::new();
    for z in &zeilen {
        if z[0] == "1" {
            let k = z[2];
            if k.len() != 2 || !k.bytes().all(|b| b.is_ascii_uppercase()) {
                return Err(format!("{}: Kantonskürzel «{k}» unerwartet", AGV.pfad));
            }
            kantone.insert(k);
        } else if z[0] == "3" && !namen.insert(z[1]) {
            return Err(format!("{}: Gemeindename «{}» doppelt", AGV.pfad, z[1]));
        }
    }
    let kantone: Vec<&str> = kantone.into_iter().collect();
    let mut text = kopf(&[
        (
            "titel",
            "Historisiertes Gemeindeverzeichnis der Schweiz, Snapshot der Gemeinden".into(),
        ),
        ("herausgeber", "Bundesamt für Statistik BFS".into()),
        ("url", URL_AGV.into()),
        ("datensatz", URL_AGV_DATENSATZ.into()),
        ("lizenz", LIZENZ_BFS.into()),
        ("abgerufen", ABGERUFEN.into()),
        ("stichtag", "06.11.2026".into()),
        ("rohdatei-sha256", AGV.sha256.into()),
        (
            "grundlage",
            "GeoNV (SR 510.625) Art. 19 Abs. 1 lit. b: das Bundesamt für Statistik «erstellt, verwaltet und veröffentlicht das amtliche Gemeindeverzeichnis der Schweiz»; Abs. 3: «Die Gemeindenamen und Gemeindenummern des amtlichen Gemeindeverzeichnisses sind behördenverbindlich.»"
                .into(),
        ),
        (
            "filter",
            format!(
                "Level 3 (Gemeinden), Spalte Name: {} von {} Zeilen; Kantone (Level 1) und Bezirke (Level 2) weggelassen; «kantonskuerzel» ist Spalte ShortName der {} Kantone (Level 1)",
                namen.len(),
                zeilen.len(),
                kantone.len()
            ),
        ),
        ("kantonskuerzel", kantone.join(" ")),
        ("zeilen", namen.len().to_string()),
        ("spalten", "name".into()),
    ]);
    for &name in &namen {
        zeile(&mut text, &[name]);
    }
    schreiben(ziel, "gemeinden.tsv", &text)
}

/// Typ eines Namens: Anfangstyp vor Endung, sonst die längste Endung des letzten Worts.
/// `woerter` sind gefaltet (Kleinbuchstaben).
fn typ_von(
    woerter: &[&str],
    anfang: &HashMap<&str, usize>,
    ende: &[(&str, usize)],
) -> Option<usize> {
    let erstes = woerter.first()?;
    if let Some(&i) = anfang.get(erstes) {
        return Some(i);
    }
    let letztes = woerter.last()?;
    if let Some(&i) = letztes.rsplit_once('-').and_then(|(_, t)| anfang.get(t)) {
        return Some(i);
    }
    ende.iter()
        .find(|(t, _)| letztes.ends_with(t))
        .map(|&(_, i)| i)
}

/// Suchtabellen für eine Auswahl der Typen: Anfangswörter und Endungen, längste zuerst.
fn tabellen(
    auswahl: impl Fn(Grundlage) -> bool,
) -> (HashMap<&'static str, usize>, Vec<(&'static str, usize)>) {
    let mut anfang = HashMap::new();
    let mut ende = Vec::new();
    for (i, &(typ, stellung, grundlage)) in TYPEN.iter().enumerate() {
        if !auswahl(grundlage) {
            continue;
        }
        match stellung {
            Stellung::Anfang => {
                anfang.insert(typ, i);
            }
            _ => ende.push((typ, i)),
        }
    }
    ende.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(b.0)));
    (anfang, ende)
}

fn prozent(teil: usize, ganz: usize) -> f64 {
    if ganz == 0 {
        0.0
    } else {
        100.0 * teil as f64 / ganz as f64
    }
}

/// `strassentypen.tsv`: Typen, Vorwörter und Verbindungswörter mit Anzahl und Abdeckung.
fn strassentypen(roh: &Path, ziel: &Path) -> Result<(), String> {
    let ch = lesen(roh, &STR_CH)?;
    let li = lesen(roh, &STR_LI)?;
    let z_ch = spalten(&ch, ';', STR_CH.pfad, &["STN_LABEL", "STR_TYPE"])?;
    let z_li = spalten(&li, ';', STR_LI.pfad, &["STN_LABEL", "STR_TYPE"])?;
    let namen: Vec<(&str, &str)> = z_ch
        .iter()
        .chain(z_li.iter())
        .map(|z| (z[0], z[1]))
        .collect();
    let gesamt = namen.len();
    // 0,01 % der Namen, aufgerundet.
    let mindestens = gesamt.div_ceil(10_000);

    let gefaltet: Vec<String> = namen.iter().map(|(n, _)| falten(n, true).text).collect();

    // Abdeckung je Stufe der Grundlage.
    let mut stufen = Vec::new();
    for (bezeichnung, grenze) in [
        ("nur GeoNV", Grundlage::GeoNv),
        ("mit Empfehlung", Grundlage::Empfehlung),
        ("mit Verzeichnis", Grundlage::Verzeichnis),
    ] {
        let (anfang, ende) = tabellen(|g| g <= grenze);
        let erfasst = gefaltet
            .iter()
            .filter(|g| {
                let w: Vec<&str> = g.split_whitespace().collect();
                typ_von(&w, &anfang, &ende).is_some()
            })
            .count();
        stufen.push(format!(
            "{bezeichnung} {:.2} % ({erfasst})",
            prozent(erfasst, gesamt)
        ));
    }

    let (anfang, ende) = tabellen(|_| true);
    let mut je_typ = vec![0usize; TYPEN.len()];
    let mut je_art: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut vorwoerter: HashMap<&str, usize> = HashMap::new();
    let mut verbindungen: HashMap<String, usize> = HashMap::new();
    let mut ohne_typ: HashMap<&str, usize> = HashMap::new();
    let mut ausgeschlossen = vec![0usize; AUSGESCHLOSSEN.len()];
    let mut erfasst = 0usize;
    for ((name, art), g) in namen.iter().zip(&gefaltet) {
        let woerter: Vec<&str> = g.split_whitespace().collect();
        let original: Vec<&str> = name.split_whitespace().collect();
        if woerter.len() != original.len() {
            return Err(format!("Falten ändert die Wortzahl: «{name}»"));
        }
        let typ = typ_von(&woerter, &anfang, &ende);
        let eintrag = je_art.entry(*art).or_default();
        eintrag.0 += 1;
        match typ {
            Some(i) => {
                erfasst += 1;
                eintrag.1 += 1;
                je_typ[i] += 1;
                if TYPEN[i].1 == Stellung::Ende && original.len() >= 2 {
                    *vorwoerter.entry(original[0]).or_default() += 1;
                }
            }
            None => {
                let letztes = woerter.last().copied().unwrap_or("");
                *ohne_typ.entry(letztes).or_default() += 1;
                for (j, &(wort, stellung, _)) in AUSGESCHLOSSEN.iter().enumerate() {
                    let trifft = match stellung {
                        Stellung::Ende => letztes.ends_with(wort),
                        _ => woerter.first() == Some(&wort),
                    };
                    if trifft {
                        ausgeschlossen[j] += 1;
                    }
                }
            }
        }
        // Kleingeschriebene Wörter nach dem ersten; elidierte Artikel («d'», «l'») als solche.
        let mut im_namen = HashSet::new();
        for (wort, &w) in original.iter().zip(&woerter).skip(1) {
            if !wort.chars().next().is_some_and(char::is_lowercase) {
                continue;
            }
            let verbindung: &str = match w.find('\'') {
                Some(p) if p + 1 < w.len() => &w[..=p],
                _ => w,
            };
            im_namen.insert(verbindung.to_string());
        }
        for v in im_namen {
            *verbindungen.entry(v).or_default() += 1;
        }
    }

    for (i, &(typ, _, grundlage)) in TYPEN.iter().enumerate() {
        if grundlage == Grundlage::Verzeichnis && je_typ[i] < mindestens {
            return Err(format!(
                "Typ «{typ}» kommt nur in {} Namen vor, Schwelle {mindestens}",
                je_typ[i]
            ));
        }
    }

    // Zeilen: Typen, dann Vorwörter und Verbindungswörter aus dem Verzeichnis.
    let mut zeilen: Vec<(Stellung, Grundlage, usize, String)> = TYPEN
        .iter()
        .zip(&je_typ)
        .map(|(&(typ, stellung, grundlage), &n)| (stellung, grundlage, n, typ.to_string()))
        .collect();
    let mut vorwort_zeilen: Vec<(&str, usize)> = vorwoerter
        .into_iter()
        .filter(|&(w, n)| {
            n >= mindestens
                && w.chars().next().is_some_and(char::is_uppercase)
                && (w.ends_with('e') || w.ends_with("er") || w.ends_with("es") || w == "St.")
        })
        .collect();
    vorwort_zeilen.sort();
    zeilen.extend(
        vorwort_zeilen
            .iter()
            .map(|&(w, n)| (Stellung::Vorwort, Grundlage::Verzeichnis, n, w.to_string())),
    );
    zeilen.extend(
        verbindungen
            .iter()
            .filter(|&(_, &n)| n >= mindestens)
            .map(|(w, &n)| (Stellung::Verbindung, Grundlage::Verzeichnis, n, w.clone())),
    );
    zeilen.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(b.2.cmp(&a.2))
            .then(a.3.cmp(&b.3))
    });

    let mut je_art_text = Vec::new();
    for (art, &(n, e)) in &je_art {
        je_art_text.push(format!("{art} {:.2} % ({e} von {n})", prozent(e, n)));
    }
    let mut haeufigste: Vec<(&str, usize)> = ohne_typ.into_iter().collect();
    haeufigste.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let haeufigste: Vec<String> = haeufigste
        .iter()
        .take(20)
        .map(|(w, n)| format!("{w} {n}"))
        .collect();
    let ausgeschlossen: Vec<String> = AUSGESCHLOSSEN
        .iter()
        .zip(&ausgeschlossen)
        .map(|(&(wort, stellung, grund), n)| format!("{wort} ({}, {grund}) {n}", stellung.name()))
        .collect();

    let mut text = kopf(&[
        (
            "titel",
            "Amtliches Verzeichnis der Strassen (Schweiz und Liechtenstein): abgeleitete Strassentypen".into(),
        ),
        ("herausgeber", "Bundesamt für Landestopografie swisstopo".into()),
        ("url", URL_STR_CH.into()),
        ("url-li", URL_STR_LI.into()),
        ("lizenz", LIZENZ_SWISSTOPO.into()),
        ("quellenangabe", QUELLENANGABE_SWISSTOPO.into()),
        ("abgerufen", ABGERUFEN.into()),
        (
            "archiv-sha256",
            format!("CH {}, LI {}", STR_CH_ARCHIV.sha256, STR_LI_ARCHIV.sha256),
        ),
        (
            "rohdatei-sha256",
            format!(
                "CH {} (amtliches-strassenverzeichnis_ch_2056.csv), LI {} (amtliches-strassenverzeichnis_li_2056.csv)",
                STR_CH.sha256, STR_LI.sha256
            ),
        ),
        (
            "grundlage",
            "GeoNV (SR 510.625) Art. 3 lit. f und Art. 26a Abs. 1; Empfehlung Gebäudeadressierung und Schreibweise von Strassennamen, BFS und swisstopo 2018, Kap. 3, S. 9 (Kurznamen von Strassen)"
                .into(),
        ),
        (
            "filter",
            format!(
                "Spalte STN_LABEL aller {gesamt} Zeilen (CH {}, LI {}), alle STR_TYPE und STR_STATUS; Vergleich in Kleinbuchstaben nach promptscan::lexikon::falten",
                z_ch.len(),
                z_li.len()
            ),
        ),
        (
            "auswahl",
            format!(
                "ende: Endung des letzten Worts (deutsch); anfang: erstes Wort oder letzter Bindestrich-Teil (französisch, italienisch, rätoromanisch); geonv: Gattungen nach GeoNV Art. 3 lit. f in der Sprachfassung; empfehlung: Typen und Abkürzungen der Kurznamen-Tabelle; verzeichnis: Gattungswort eines Verkehrswegs oder Platzes in mindestens 0,01 % der Namen ({mindestens}); vorwort: grossgeschriebenes erstes Wort vor einem deutschen Typ mit Adjektivendung -e, -er, -es oder «St.» (Empfehlung S. 8: Sankt → St.), mindestens {mindestens} Namen; verbindung: kleingeschriebenes Wort nach dem ersten, elidierter Artikel mit Apostroph, in mindestens {mindestens} Namen; anzahl: Namen mit diesem Typ (Anfang vor Endung, längste Endung), Vorwort oder Verbindungswort"
            ),
        ),
        (
            "abdeckung",
            format!(
                "{:.2} % der Namen ({erfasst} von {gesamt}) tragen einen Typ; {}",
                prozent(erfasst, gesamt),
                stufen.join("; ")
            ),
        ),
        ("abdeckung-je-str_type", je_art_text.join("; ")),
        ("ausgeschlossen", ausgeschlossen.join("; ")),
        ("haeufigste-letzte-woerter-ohne-typ", haeufigste.join(", ")),
        ("zeilen", zeilen.len().to_string()),
        ("spalten", "typ\tstellung\tanzahl\tanteil\tgrundlage".into()),
    ]);
    for (stellung, grundlage, n, typ) in &zeilen {
        let anzahl = n.to_string();
        let anteil = format!("{:.3}", prozent(*n, gesamt));
        zeile(
            &mut text,
            &[
                typ.as_str(),
                stellung.name(),
                anzahl.as_str(),
                anteil.as_str(),
                grundlage.name(),
            ],
        );
    }
    eprintln!(
        "Strassen: {gesamt} Namen, Abdeckung {:.2} %, Schwelle {mindestens}",
        prozent(erfasst, gesamt)
    );
    schreiben(ziel, "strassentypen.tsv", &text)
}

/// Liest eine Rohdatei, prüft ihre Summe und gibt sie als Text zurück.
fn lesen(wurzel: &Path, roh: &Roh) -> Result<String, String> {
    let pfad = wurzel.join(roh.pfad);
    let bytes = fs::read(&pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    pruefsumme(roh, &bytes)?;
    String::from_utf8(bytes).map_err(|e| format!("{}: kein UTF-8: {e}", roh.pfad))
}

fn pruefsumme(roh: &Roh, bytes: &[u8]) -> Result<(), String> {
    let summe = sha256_hex(bytes);
    if summe != roh.sha256 {
        return Err(format!(
            "{}: SHA-256 {summe} statt {}; neue Rohdaten brauchen ein neues Abrufdatum und neue Summen",
            roh.pfad, roh.sha256
        ));
    }
    Ok(())
}

/// Die verlangten Spalten aller Datenzeilen einer CSV-Datei ohne Anführungszeichen
/// (in allen drei Datensätzen geprüft); ein UTF-8-BOM am Anfang wird übergangen.
fn spalten<'a>(
    inhalt: &'a str,
    trenner: char,
    datei: &str,
    namen: &[&str],
) -> Result<Vec<Vec<&'a str>>, String> {
    let inhalt = inhalt.strip_prefix('\u{FEFF}').unwrap_or(inhalt);
    if inhalt.contains('"') {
        return Err(format!(
            "{datei}: enthält Anführungszeichen, nicht vorgesehen"
        ));
    }
    let mut zeilen = inhalt.lines().filter(|z| !z.is_empty());
    let kopf: Vec<&str> = zeilen
        .next()
        .ok_or_else(|| format!("{datei}: leer"))?
        .split(trenner)
        .collect();
    let index = namen
        .iter()
        .map(|n| {
            kopf.iter()
                .position(|k| k == n)
                .ok_or_else(|| format!("{datei}: Spalte {n} fehlt"))
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let mut daten = Vec::new();
    for (nr, z) in zeilen.enumerate() {
        let felder: Vec<&str> = z.split(trenner).collect();
        if felder.len() != kopf.len() {
            return Err(format!(
                "{datei}, Datenzeile {}: {} statt {} Felder",
                nr + 1,
                felder.len(),
                kopf.len()
            ));
        }
        if felder.iter().any(|f| f.contains('\t')) {
            return Err(format!("{datei}, Datenzeile {}: Tabulator im Feld", nr + 1));
        }
        daten.push(index.iter().map(|&i| felder[i]).collect());
    }
    Ok(daten)
}

fn kopf(felder: &[(&str, String)]) -> String {
    let mut s = String::new();
    for (k, w) in felder {
        s.push_str("# ");
        s.push_str(k);
        s.push_str(": ");
        s.push_str(w);
        s.push('\n');
    }
    s
}

fn zeile(text: &mut String, felder: &[&str]) {
    text.push_str(&felder.join("\t"));
    text.push('\n');
}

fn schreiben(ziel: &Path, datei: &str, inhalt: &str) -> Result<(), String> {
    fs::create_dir_all(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let pfad = ziel.join(datei);
    fs::write(&pfad, inhalt).map_err(|e| format!("{}: {e}", pfad.display()))?;
    eprintln!("{}: {} Bytes", pfad.display(), inhalt.len());
    Ok(())
}

/// SHA-256 nach FIPS 180-4, hexadezimal; nur `std`, damit keine weitere Crate nötig ist.
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
    let mut verarbeiten = |block: &[u8]| {
        let mut w = [0u32; 64];
        for (wi, wort) in w.iter_mut().zip(block.chunks_exact(4)) {
            *wi = u32::from_be_bytes([wort[0], wort[1], wort[2], wort[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for (k, wi) in K.iter().zip(w.iter()) {
            let [a, b, c, d, e, f, g, hh] = v;
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(*k)
                .wrapping_add(*wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            v = [
                t1.wrapping_add(s0.wrapping_add(maj)),
                a,
                b,
                c,
                d.wrapping_add(t1),
                e,
                f,
                g,
            ];
        }
        for (x, y) in h.iter_mut().zip(v) {
            *x = x.wrapping_add(y);
        }
    };
    let mut bloecke = daten.chunks_exact(64);
    for block in &mut bloecke {
        verarbeiten(block);
    }
    let mut rest = bloecke.remainder().to_vec();
    rest.push(0x80);
    while rest.len() % 64 != 56 {
        rest.push(0);
    }
    rest.extend_from_slice(&(daten.len() as u64).wrapping_mul(8).to_be_bytes());
    for block in rest.chunks_exact(64) {
        verarbeiten(block);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}
