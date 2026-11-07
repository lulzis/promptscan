//! Befehlszeile von promptscan (SPEC 3; docs/AUSGABE.md Abschnitte 6 und 8).
//!
//! Ablauf: Eine Vorprüfung liest Sprache, Hilfe und `-t` aus den Argumenten, bevor clap
//! sie zerlegt. So erscheinen Hilfe und Fehlermeldungen in der Sprache von `--lang` (sonst
//! `lang` aus der Konfiguration, sonst Deutsch), auch wenn die Befehlszeile fehlerhaft ist.
//! Dann folgen Konfiguration, Befehl und Ausgabe. Alle Meldungen gehen nach stderr: die
//! erste Zeile mit «promptscan: » und dem Problem, darunter, um 2 eingerückt, der nächste
//! Schritt.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Parser, Subcommand, ValueEnum};

use promptscan::ausgabe::{self, Darstellung};
use promptscan::fund::EINGABEWOERTER;
use promptscan::{Optionen, Rolle, RolleQuelle, Stufe, Ziel, katalog, pruefe};

/// Breite von Hilfe und Meldungen in Spalten.
const BREITE: usize = 80;

thread_local! {
    /// Nur ASCII ausgeben ([`ascii_gewuenscht`]). Gesetzt in [`lauf`]; gilt für Hilfe,
    /// Meldungen und `config`, und [`abschliessen`] schreibt danach jede Textausgabe. Je
    /// Thread, damit sich die Tests nicht gegenseitig stören.
    static NUR_ASCII: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn nur_ascii() -> bool {
    NUR_ASCII.with(std::cell::Cell::get)
}

// ---------------------------------------------------------------------------------------
// Befehlszeile
// ---------------------------------------------------------------------------------------

/// Die Hilfe (`-h`, `--help`, `promptscan help`) schreibt promptscan selbst, zweisprachig
/// und auf 80 Spalten ([`hilfe_text`]); clap liefert nur die Zerlegung.
#[derive(Parser)]
#[command(
    name = "promptscan",
    version,
    disable_help_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    befehl: Option<Befehl>,

    /// Datei mit dem Prompt. Ohne Datei und ohne -t: Text einfügen.
    datei: Option<PathBuf>,

    /// Text direkt prüfen. Ein Wert mit «-» am Anfang (Aufzählung) ist Text, keine Option.
    #[arg(short = 't', long = "text", allow_hyphen_values = true, global = true)]
    text: Option<String>,

    /// Berufsrolle: Kennung oder deutsches Eingabewort ([`Rolle::aus_eingabe`]).
    #[arg(long, global = true)]
    role: Option<String>,

    #[arg(long, value_enum, ignore_case = true, global = true)]
    target: Option<ZielArg>,

    #[arg(long, value_enum, ignore_case = true, default_value_t = FormatArg::Text, global = true)]
    format: FormatArg,

    #[arg(long, value_enum, ignore_case = true, global = true)]
    lang: Option<SpracheArg>,

    #[arg(long = "fail-on", value_enum, ignore_case = true, default_value_t = SchwelleArg::Red, global = true)]
    fail_on: SchwelleArg,

    #[arg(long = "no-color", global = true)]
    no_color: bool,

    /// Ausgabe in eine Datei: UTF-8 mit BOM, ohne Farbe.
    #[arg(long, global = true)]
    output: Option<PathBuf>,

    /// Nur das Ergebnis, ohne Prompt und Funde.
    #[arg(long, global = true)]
    quiet: bool,

    /// Nur ASCII-Zeichen ausgeben.
    #[arg(long, global = true)]
    ascii: bool,

    #[arg(short = 'h', long = "help", global = true)]
    hilfe: bool,
}

/// Die Befehle. Ein weiterer kommt als Variante hier und als Zweig in [`lauf`] dazu.
#[derive(Subcommand)]
enum Befehl {
    /// Alle Regeln mit Rechtsgrundlage.
    Rules,
    /// Normtext, Methode, Quellen, Güte und Grenzen einer Regel oder Stufe.
    Explain {
        /// Kennung der Regel, z. B. ahv oder art5c-2, oder eine Stufe.
        regel: Option<String>,
    },
    /// Lizenzstand und Vermerke zu fremden Bestandteilen (Crates, Rust, MinGW-w64, Daten)
    Licenses,
    /// Konfigurationsdatei zeigen; mit --role oder --lang schreiben.
    Config,
    /// Wie --help.
    #[command(hide = true)]
    Help,
}

impl Befehl {
    fn name(&self) -> &'static str {
        match self {
            Befehl::Rules => "rules",
            Befehl::Explain { .. } => "explain",
            Befehl::Licenses => "licenses",
            Befehl::Config => "config",
            Befehl::Help => "help",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ZielArg {
    Cloud,
    Local,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SpracheArg {
    De,
    En,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum SchwelleArg {
    Red,
    Orange,
    Yellow,
    Never,
}

// ---------------------------------------------------------------------------------------
// Vorprüfung der Argumente
// ---------------------------------------------------------------------------------------

/// Was vor dem Zerlegen aus den Argumenten hervorgeht.
#[derive(Debug, Default, PartialEq, Eq)]
struct Vorpruefung {
    /// `--lang de|en`; `None`, wenn nicht oder ungültig angegeben.
    englisch: Option<bool>,
    /// `-h` oder `--help` steht da, nicht als Wert einer Option und nicht nach `--`.
    hilfe: bool,
    /// `-t` oder `--text` steht da.
    mit_text: bool,
    /// Das erste freie Argument, wenn es ein Befehl ist.
    befehl: Option<String>,
    /// `--ascii` steht da.
    ascii: bool,
    /// Der Wert von `--output`; gilt auch für die Hilfe.
    ausgabe: Option<PathBuf>,
    /// `--role` bzw. `--lang` steht nach dem Befehl. Nur dann schreibt `config` den Wert;
    /// davor gilt die Option wie bei `rules` nur für diesen Aufruf.
    rolle_nach_befehl: bool,
    sprache_nach_befehl: bool,
}

/// Optionen mit einem Wert. Ihr Wert gilt nie als Option, auch wenn er mit «-» beginnt.
const MIT_WERT: [&str; 8] = [
    "-t",
    "--text",
    "--role",
    "--target",
    "--format",
    "--lang",
    "--fail-on",
    "--output",
];

const BEFEHLE: [&str; 5] = ["rules", "explain", "licenses", "config", "help"];

fn sprache_aus_wert(wert: &str) -> Option<bool> {
    match wert.trim().to_lowercase().as_str() {
        "de" => Some(false),
        "en" => Some(true),
        _ => None,
    }
}

fn vorpruefen(args: &[OsString]) -> Vorpruefung {
    let mut v = Vorpruefung::default();
    let mut frei = 0;
    let mut i = 1;
    while i < args.len() {
        let a = args[i].to_string_lossy();
        i += 1;
        if a == "--" {
            break;
        }
        if a == "-h" || a == "--help" {
            v.hilfe = true;
            continue;
        }
        if a == "--ascii" {
            v.ascii = true;
            continue;
        }
        let nach_befehl = v.befehl.is_some();
        if a == "--role" || a.starts_with("--role=") {
            v.rolle_nach_befehl |= nach_befehl;
        }
        if a == "--lang" || a.starts_with("--lang=") {
            v.sprache_nach_befehl |= nach_befehl;
        }
        if let Some(wert) = a.strip_prefix("--lang=") {
            v.englisch = sprache_aus_wert(wert);
            continue;
        }
        if let Some(wert) = a.strip_prefix("--output=") {
            v.ausgabe = Some(PathBuf::from(wert));
            continue;
        }
        if a == "-t" || a.starts_with("--text") || (a.starts_with("-t") && !a.starts_with("--")) {
            v.mit_text = true;
        }
        if MIT_WERT.contains(&a.as_ref()) {
            if a == "--lang" {
                v.englisch = args
                    .get(i)
                    .and_then(|w| sprache_aus_wert(&w.to_string_lossy()));
            }
            if a == "--output" {
                v.ausgabe = args.get(i).map(PathBuf::from);
            }
            i += 1;
            continue;
        }
        if !a.starts_with('-') {
            if frei == 0 && BEFEHLE.contains(&a.as_ref()) {
                v.befehl = Some(a.into_owned());
            }
            frei += 1;
        }
    }
    v
}

// ---------------------------------------------------------------------------------------
// Meldungen
// ---------------------------------------------------------------------------------------

/// Bricht `text` an Leerzeichen um: die erste Zeile auf `erste` Zeichen, jede weitere auf
/// `folge`. Ein Wort, das allein zu lang ist (Pfad, URL), steht ungebrochen. Ein
/// geschütztes Leerzeichen (U+00A0) hält zusammen; der Aufrufer ersetzt es danach. Mit
/// `--ascii` wird vorher umgeschrieben ([`nach_ascii`]), damit die Breite stimmt.
fn umbrechen(text: &str, erste: usize, folge: usize) -> Vec<String> {
    let text = ascii_wenn_verlangt(text);
    let mut zeilen = Vec::new();
    let mut zeile = String::new();
    for wort in text.split(' ').filter(|w| !w.is_empty()) {
        let platz = if zeilen.is_empty() { erste } else { folge };
        let laenge = zeile.chars().count();
        if laenge > 0 && laenge + 1 + wort.chars().count() > platz {
            zeilen.push(std::mem::take(&mut zeile));
        }
        if !zeile.is_empty() {
            zeile.push(' ');
        }
        zeile.push_str(wort);
    }
    if !zeile.is_empty() || zeilen.is_empty() {
        zeilen.push(zeile);
    }
    zeilen
}

/// Hält einen Befehl oder Wert beim Umbrechen zusammen, etwa «promptscan explain rot»
/// oder «role = "lawyer"»: Die Leerzeichen werden geschützt (U+00A0).
fn nb(s: &str) -> String {
    s.replace(' ', "\u{A0}")
}

/// Hält einen Pfad beim Umbruch zusammen ([`nb`]). Passt er nicht mehr auf die Zeile,
/// steht er ganz auf der nächsten, damit man ihn kopieren kann.
fn pfad_nb(pfad: &Path) -> String {
    nb(&pfad.display().to_string())
}

/// «Datei: PFAD» bzw. «File: PATH» am Ende der ersten Zeile einer Meldung. Angabe und
/// Pfad halten zusammen; passen sie nicht mehr, stehen sie auf einer eigenen Zeile.
fn datei_angabe(pfad: &Path, englisch: bool) -> String {
    format!(
        "{}\u{A0}{}",
        if englisch { "File:" } else { "Datei:" },
        pfad_nb(pfad)
    )
}

/// Eine Meldung für stderr: «promptscan: » und die erste Zeile von `text` (das Problem),
/// darunter jede weitere Zeile (Vorschlag, nächster Schritt) um 2 eingerückt. Alles ist
/// auf 80 Spalten umgebrochen, auch die erste Zeile; ein Wort, das allein zu lang ist
/// (ein Pfad), steht ungebrochen auf einer eigenen Zeile. Eine Meldung aus nur einer
/// Zeile bleibt ungebrochen: So kommen die Meldungen zur Eingabe (`src/ausgabe.rs`,
/// Abschnitt 1) und die Lesefehler genau wie dort beschrieben an, und ein Befehl in ihnen
/// zerfällt nicht.
fn meldung_formatieren(text: &str) -> String {
    const VORSPANN: &str = "promptscan: ";
    if !text.contains('\n') {
        return format!(
            "{VORSPANN}{}\n",
            ascii_wenn_verlangt(text).replace('\u{A0}', " ")
        );
    }
    let mut aus = String::new();
    for (i, zeile) in text.split('\n').enumerate() {
        let erste = if i == 0 {
            BREITE - VORSPANN.len()
        } else {
            BREITE - 2
        };
        for (j, z) in umbrechen(zeile, erste, BREITE - 2).into_iter().enumerate() {
            let vor = if i == 0 && j == 0 { VORSPANN } else { "  " };
            let _ = writeln!(aus, "{vor}{}", z.replace('\u{A0}', " "));
        }
    }
    aus
}

/// Schreibt eine Meldung auf stderr ([`meldung_formatieren`]). Scheitert das Schreiben,
/// bleibt es ohne Folgen: Es gibt keinen anderen Weg mehr, etwas zu melden.
fn melden(text: &str) {
    let _ = io::stderr()
        .lock()
        .write_all(meldung_formatieren(text).as_bytes());
}

/// Meldet einen Bedienungsfehler und gibt Exit-Code 2 zurück.
fn fehler(text: &str) -> ExitCode {
    melden(text);
    ExitCode::from(2)
}

/// Abstand zweier Wörter: Einfügen, Löschen, Ersetzen und das Vertauschen zweier
/// Nachbarzeichen kosten je 1 (eingeschränkter Damerau-Levenshtein-Abstand).
fn abstand(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0_usize; b.len() + 1]; a.len() + 1];
    for (i, zeile) in d.iter_mut().enumerate() {
        zeile[0] = i;
    }
    for (j, feld) in d[0].iter_mut().enumerate() {
        *feld = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let kosten = usize::from(a[i - 1] != b[j - 1]);
            let mut wert = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + kosten);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                wert = wert.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = wert;
        }
    }
    d[a.len()][b.len()]
}

/// Der Kandidat, den `eingabe` wohl meint: der mit dem kleinsten Abstand, höchstens 1 bei
/// Eingaben bis 4 Zeichen, sonst höchstens 2. Ein Anfang zählt wie Abstand 1, wenn er
/// mindestens 4 Zeichen hat («ärztinnen» beginnt mit «ärztin», «anwa» mit «anwalt»).
fn vorschlag<T: Copy>(eingabe: &str, kandidaten: &[(&str, T)]) -> Option<T> {
    let eingabe = eingabe.trim().to_lowercase();
    let grenze = if eingabe.chars().count() <= 4 { 1 } else { 2 };
    let mut bester: Option<(usize, T)> = None;
    for (wort, ziel) in kandidaten {
        let wert = if (eingabe.chars().count() >= 4 && wort.starts_with(eingabe.as_str()))
            || (wort.chars().count() >= 4 && eingabe.starts_with(wort))
        {
            1
        } else {
            abstand(&eingabe, wort)
        };
        if wert <= grenze && bester.is_none_or(|(b, _)| wert < b) {
            bester = Some((wert, *ziel));
        }
    }
    bester.map(|(_, ziel)| ziel)
}

// ---------------------------------------------------------------------------------------
// Rollen
// ---------------------------------------------------------------------------------------

/// Alle Kennungen von `--role`, mit Komma getrennt.
fn rollenliste() -> String {
    Rolle::ALLE
        .iter()
        .map(|r| r.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Kennungen und deutsche Eingabewörter als Kandidaten für [`vorschlag`].
fn rollen_kandidaten() -> Vec<(&'static str, Rolle)> {
    let mut k: Vec<(&str, Rolle)> = Rolle::ALLE.iter().map(|r| (r.name(), *r)).collect();
    k.extend(EINGABEWOERTER.iter().copied());
    k
}

/// Meldung zu einer unbekannten Rolle. `wie` ist die Schreibweise des Vorschlags:
/// «--role lawyer» auf der Befehlszeile, «role = "lawyer"» in der Datei.
fn rolle_unbekannt(wert: &str, englisch: bool, datei: bool) -> String {
    let wie = |r: Rolle| {
        if datei {
            format!("role = \"{}\"", r.name())
        } else {
            format!("--role {}", r.name())
        }
    };
    match (vorschlag(wert, &rollen_kandidaten()), englisch) {
        (Some(r), false) => format!(
            "Die Rolle «{wert}» gibt es nicht.\nGemeint ist wohl: {}\nAlle Rollen mit Pflicht und Normen: {}",
            nb(&wie(r)),
            nb("promptscan explain rot")
        ),
        (Some(r), true) => format!(
            "There is no role «{wert}».\nDid you mean: {}\nAll roles with duty and provisions: {}",
            nb(&wie(r)),
            nb("promptscan explain red")
        ),
        (None, false) => format!(
            "Die Rolle «{wert}» gibt es nicht.\nRollen: {}; auch deutsch, etwa anwalt oder ärztin. Pflicht und Normen je Rolle: {}",
            rollenliste(),
            nb("promptscan explain rot")
        ),
        (None, true) => format!(
            "There is no role «{wert}».\nRoles: {}; German words such as anwalt also work. Duty and provisions per role: {}",
            rollenliste(),
            nb("promptscan explain red")
        ),
    }
}

// ---------------------------------------------------------------------------------------
// Konfiguration
// ---------------------------------------------------------------------------------------

/// Woher der Pfad der Konfigurationsdatei kommt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrtQuelle {
    /// Umgebungsvariable `PROMPTSCAN_CONFIG`.
    Umgebung,
    /// `%APPDATA%\promptscan\config.toml` (Windows).
    AppData,
    /// `$XDG_CONFIG_HOME/promptscan/config.toml`.
    Xdg,
    /// `~/.config/promptscan/config.toml`.
    Home,
}

impl OrtQuelle {
    fn text(self, englisch: bool) -> &'static str {
        match (self, englisch) {
            (OrtQuelle::Umgebung, _) => "PROMPTSCAN_CONFIG",
            (OrtQuelle::AppData, _) => "%APPDATA%",
            (OrtQuelle::Xdg, _) => "$XDG_CONFIG_HOME",
            (OrtQuelle::Home, false) => "~/.config (Benutzerordner)",
            (OrtQuelle::Home, true) => "~/.config (home folder)",
        }
    }
}

/// Pfad der Konfigurationsdatei: `PROMPTSCAN_CONFIG`, sonst `%APPDATA%\promptscan\config.toml`
/// (Windows) bzw. `$XDG_CONFIG_HOME/promptscan/config.toml` oder `~/.config/promptscan/config.toml`.
fn konfigurationsort() -> Option<(PathBuf, OrtQuelle)> {
    if let Some(p) = std::env::var_os("PROMPTSCAN_CONFIG") {
        return Some((PathBuf::from(p), OrtQuelle::Umgebung));
    }
    let unter = |ordner: OsString| PathBuf::from(ordner).join("promptscan").join("config.toml");
    if cfg!(windows) {
        return std::env::var_os("APPDATA")
            .filter(|a| !a.is_empty())
            .map(|a| (unter(a), OrtQuelle::AppData));
    }
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|x| !x.is_empty()) {
        return Some((unter(x), OrtQuelle::Xdg));
    }
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(|h| {
        let mut p = PathBuf::from(h);
        p.push(".config");
        (unter(p.into_os_string()), OrtQuelle::Home)
    })
}

/// Die Werte der Datei, wie sie dort stehen.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Werte {
    role: Option<String>,
    lang: Option<String>,
    /// Wird übergangen: Das Ziel gilt je Aufruf.
    target: Option<String>,
}

/// Warum die Datei ganz oder teilweise nicht gelesen wird.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KonfigFehler {
    /// Der Pfad ist ein Ordner.
    Ordner,
    /// Zugriff verweigert.
    Zugriff,
    /// Anderer Lesefehler; die Meldung des Systems ohne «(os error …)».
    Lesen(String),
    Kodierung(Lesefehler),
    /// Kein gültiges TOML: Zeile (1-basiert) und ihr Inhalt.
    Syntax {
        zeile: usize,
        inhalt: String,
    },
    /// Ein Schlüssel ausser role, lang und target.
    Schluessel(String),
    /// Ein Wert, der kein Text ist, etwa `role = lawyer` ohne Anführungszeichen.
    KeinText(String),
}

/// Die gelesene Konfiguration.
#[derive(Debug, Default)]
struct Konfig {
    ort: Option<(PathBuf, OrtQuelle)>,
    /// Die Datei gibt es.
    vorhanden: bool,
    werte: Werte,
    fehler: Vec<KonfigFehler>,
}

/// Meldung des Systems ohne die Nummer «(os error …)» am Ende.
fn os_meldung(e: &io::Error) -> String {
    let text = e.to_string();
    match text.rfind(" (os error ") {
        Some(i) if text.ends_with(')') => text[..i].trim_end_matches('.').to_string(),
        _ => text,
    }
}

/// Liest die Werte aus den Bytes der Datei: dekodiert wie die Eingabe ([`dekodieren`]:
/// UTF-8 mit oder ohne BOM, UTF-16 mit BOM), dann TOML.
fn konfig_aus_bytes(bytes: &[u8]) -> (Werte, Vec<KonfigFehler>) {
    let text = match dekodieren(bytes) {
        Ok(t) => t,
        Err(e) => return (Werte::default(), vec![KonfigFehler::Kodierung(e)]),
    };
    let tabelle: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            let start = e.span().map_or(0, |s| s.start).min(text.len());
            let zeile = text[..start].matches('\n').count() + 1;
            let inhalt: String = text
                .lines()
                .nth(zeile - 1)
                .unwrap_or("")
                .trim()
                .chars()
                .take(40)
                .collect();
            return (
                Werte::default(),
                vec![KonfigFehler::Syntax { zeile, inhalt }],
            );
        }
    };
    let mut werte = Werte::default();
    let mut fehler = Vec::new();
    for (schluessel, wert) in &tabelle {
        let ziel = match schluessel.as_str() {
            "role" => &mut werte.role,
            "lang" => &mut werte.lang,
            "target" => &mut werte.target,
            _ => {
                fehler.push(KonfigFehler::Schluessel(schluessel.clone()));
                continue;
            }
        };
        match wert.as_str() {
            Some(s) => *ziel = Some(s.to_string()),
            None => fehler.push(KonfigFehler::KeinText(schluessel.clone())),
        }
    }
    (werte, fehler)
}

impl Konfig {
    fn lesen() -> Konfig {
        Konfig::lesen_von(konfigurationsort())
    }

    fn lesen_von(ort: Option<(PathBuf, OrtQuelle)>) -> Konfig {
        let Some((pfad, _)) = &ort else {
            return Konfig::default();
        };
        let mut k = Konfig {
            vorhanden: true,
            ..Konfig::default()
        };
        if pfad.is_dir() {
            k.fehler.push(KonfigFehler::Ordner);
        } else {
            match std::fs::read(pfad) {
                Ok(bytes) => (k.werte, k.fehler) = konfig_aus_bytes(&bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => k.vorhanden = false,
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    k.fehler.push(KonfigFehler::Zugriff);
                }
                Err(e) => k.fehler.push(KonfigFehler::Lesen(os_meldung(&e))),
            }
        }
        k.ort = ort;
        k
    }

    /// Die Sprache aus `lang`, wenn der Wert gültig ist.
    fn englisch(&self) -> Option<bool> {
        self.werte.lang.as_deref().and_then(sprache_aus_wert)
    }
}

/// Eine Meldung zur Konfiguration: «Konfiguration: » vor dem Problem (erste Zeile von
/// `text`), dahinter «Datei: PFAD», darunter der Rest von `text`. Hat `text` nur eine
/// Zeile, steht «Datei: PFAD» auf der zweiten: Eine Meldung aus einer einzigen Zeile
/// bliebe ungebrochen ([`meldung_formatieren`]) und mit dem Pfad weit über 80 Spalten.
fn konfig_meldung(text: &str, pfad: &Path, englisch: bool) -> String {
    let (problem, rest) = text.split_once('\n').unwrap_or((text, ""));
    let vorspann = if englisch {
        "Configuration"
    } else {
        "Konfiguration"
    };
    let datei = datei_angabe(pfad, englisch);
    if rest.is_empty() {
        format!("{vorspann}: {problem}\n{datei}")
    } else {
        format!("{vorspann}: {problem} {datei}\n{rest}")
    }
}

fn konfig_fehler_text(f: &KonfigFehler, pfad: &Path, englisch: bool) -> String {
    let beispiel = |k: &str| match k {
        "lang" => "de",
        "target" => "cloud",
        _ => "lawyer",
    };
    let text = match (f, englisch) {
        (KonfigFehler::Ordner, false) => "Der Pfad ist ein Ordner, keine Datei.\nPROMPTSCAN_CONFIG muss auf eine Datei zeigen, etwa config.toml.".to_string(),
        (KonfigFehler::Ordner, true) => "The path is a folder, not a file.\nPROMPTSCAN_CONFIG must point to a file, such as config.toml.".to_string(),
        (KonfigFehler::Zugriff, false) => {
            "Die Datei lässt sich nicht lesen: Zugriff verweigert.\nDie Rechte der Datei prüfen.".to_string()
        }
        (KonfigFehler::Zugriff, true) => {
            "The file cannot be read: access denied.\nCheck the permissions of the file.".to_string()
        }
        (KonfigFehler::Lesen(grund), false) => format!(
            "Die Datei lässt sich nicht lesen ({grund}).\nPfad und Datei prüfen: {}",
            nb("promptscan config")
        ),
        (KonfigFehler::Lesen(grund), true) => format!(
            "The file cannot be read ({grund}).\nCheck the path and the file: {}",
            nb("promptscan config")
        ),
        (KonfigFehler::Kodierung(lf), false) => format!(
            "{}.\nNeu schreiben, als UTF-8: {}",
            lf.meldung(false),
            nb("promptscan config --role ROLLE")
        ),
        (KonfigFehler::Kodierung(lf), true) => format!(
            "{}.\nRewrite it as UTF-8: {}",
            lf.meldung(true),
            nb("promptscan config --role ROLE")
        ),
        (KonfigFehler::Syntax { zeile, inhalt }, false) => format!(
            "Zeile {zeile} ist kein gültiges TOML: «{inhalt}».\nErwartet je Zeile etwa {} oder {}, mit geraden Anführungszeichen; oder: {}",
            nb("role = \"lawyer\""),
            nb("lang = \"de\""),
            nb("promptscan config --role lawyer")
        ),
        (KonfigFehler::Syntax { zeile, inhalt }, true) => format!(
            "line {zeile} is not valid TOML: «{inhalt}».\nExpected one per line, such as {} or {}, with straight quotes; or: {}",
            nb("role = \"lawyer\""),
            nb("lang = \"en\""),
            nb("promptscan config --role lawyer")
        ),
        (KonfigFehler::Schluessel(k), false) => format!(
            "Den Schlüssel «{k}» gibt es nicht.\nGültig sind role und lang, etwa {}.",
            nb("role = \"lawyer\"")
        ),
        (KonfigFehler::Schluessel(k), true) => format!(
            "there is no key «{k}».\nValid keys are role and lang, such as {}.",
            nb("role = \"lawyer\"")
        ),
        (KonfigFehler::KeinText(k), false) => format!(
            "Der Wert von {k} gehört in gerade Anführungszeichen.\nEtwa: {}",
            nb(&format!("{k} = \"{}\"", beispiel(k)))
        ),
        (KonfigFehler::KeinText(k), true) => format!(
            "the value of {k} belongs in straight quotes.\nFor example: {}",
            nb(&format!("{k} = \"{}\"", beispiel(k)))
        ),
    };
    konfig_meldung(&text, pfad, englisch)
}

/// Was die Konfiguration zu einem Aufruf beiträgt, und was dazu zu melden ist.
#[derive(Debug, PartialEq, Eq)]
struct Angewandt {
    rolle: Option<Rolle>,
    quelle: RolleQuelle,
    /// Meldungen, nach denen der Aufruf weiterläuft.
    warnungen: Vec<String>,
    /// Meldungen, die die Prüfung mit Exit-Code 2 abbrechen.
    fehler: Vec<String>,
}

/// Wendet die Konfiguration an. Die Befehlszeile geht vor. Bei der Prüfung
/// (`prueft`) bricht ein Fehler ab, der die Rolle betreffen könnte, wenn `--role` fehlt:
/// Liefe die Prüfung weiter, fiele eine eingestellte Rolle still weg (ROT würde GELB). Ein
/// falscher Wert, den die Befehlszeile übersteuert, und jeder Fehler bei `rules`,
/// `explain` und `config` ist nur eine Warnung. `target` wird übergangen.
fn konfiguration_anwenden(
    k: &Konfig,
    cli_rolle: Option<Rolle>,
    cli_lang: Option<SpracheArg>,
    prueft: bool,
    englisch: bool,
) -> Angewandt {
    let mut a = Angewandt {
        rolle: cli_rolle,
        quelle: if cli_rolle.is_some() {
            RolleQuelle::Option
        } else {
            RolleQuelle::Keine
        },
        warnungen: Vec::new(),
        fehler: Vec::new(),
    };
    let Some((pfad, _)) = &k.ort else {
        return a;
    };
    let sperrt = prueft && cli_rolle.is_none();
    for f in &k.fehler {
        let text = konfig_fehler_text(f, pfad, englisch);
        if sperrt {
            a.fehler.push(text);
        } else {
            a.warnungen.push(text);
        }
    }
    if let Some(wert) = &k.werte.role {
        match (Rolle::aus_eingabe(wert), cli_rolle) {
            (Some(r), None) => {
                a.rolle = Some(r);
                a.quelle = RolleQuelle::Konfiguration;
            }
            (Some(_), Some(_)) => {}
            (None, Some(c)) => a.warnungen.push(konfig_meldung(
                &if englisch {
                    format!(
                        "there is no role «{wert}»; {} applies.",
                        nb(&format!("--role {}", c.name()))
                    )
                } else {
                    format!(
                        "Die Rolle «{wert}» gibt es nicht; es gilt {}.",
                        nb(&format!("--role {}", c.name()))
                    )
                },
                pfad,
                englisch,
            )),
            (None, None) => {
                let text = konfig_meldung(&rolle_unbekannt(wert, englisch, true), pfad, englisch);
                if prueft {
                    a.fehler.push(text);
                } else {
                    a.warnungen.push(text);
                }
            }
        }
    }
    if let Some(wert) = k
        .werte
        .lang
        .as_ref()
        .filter(|w| sprache_aus_wert(w).is_none())
    {
        match (cli_lang, englisch) {
            (Some(l), false) => a.warnungen.push(konfig_meldung(
                &format!(
                    "Die Sprache «{wert}» gibt es nicht; es gilt {}.",
                    nb(&format!("--lang {}", sprache_name(l)))
                ),
                pfad,
                englisch,
            )),
            (Some(l), true) => a.warnungen.push(konfig_meldung(
                &format!(
                    "there is no language «{wert}»; {} applies.",
                    nb(&format!("--lang {}", sprache_name(l)))
                ),
                pfad,
                englisch,
            )),
            (None, _) => {
                let text = konfig_meldung(
                    &if englisch {
                        format!(
                            "there is no language «{wert}».\nPossible: {} or {}",
                            nb("lang = \"de\""),
                            nb("lang = \"en\"")
                        )
                    } else {
                        format!(
                            "Die Sprache «{wert}» gibt es nicht.\nMöglich: {} oder {}",
                            nb("lang = \"de\""),
                            nb("lang = \"en\"")
                        )
                    },
                    pfad,
                    englisch,
                );
                if prueft {
                    a.fehler.push(text);
                } else {
                    a.warnungen.push(text);
                }
            }
        }
    }
    if let Some(wert) = &k.werte.target {
        a.warnungen.push(target_uebergangen(pfad, wert, englisch));
    }
    a
}

/// Warnung zu `target` in der Datei. Sie sagt, wie sie verschwindet: `config` schreibt die
/// Zeile nicht um, man löscht sie selbst.
fn target_uebergangen(pfad: &Path, wert: &str, englisch: bool) -> String {
    let zeile = nb(&format!("target = \"{wert}\""));
    let text = if englisch {
        format!(
            "{zeile} is ignored.\nThe target applies per call: pass --target\u{A0}local if the model runs on this computer. Delete the line target from the file; then this warning no longer appears."
        )
    } else {
        format!(
            "{zeile} wird übergangen.\nDas Ziel gilt je Aufruf: --target\u{A0}local angeben, wenn das Modell auf diesem Rechner läuft. Die Zeile target in der Datei löschen; dann entfällt diese Warnung."
        )
    };
    konfig_meldung(&text, pfad, englisch)
}

fn sprache_name(s: SpracheArg) -> &'static str {
    match s {
        SpracheArg::De => "de",
        SpracheArg::En => "en",
    }
}

/// Der Schlüssel einer Zeile `schluessel = wert`, ohne Anführungszeichen; `None` bei
/// Kommentaren, Tabellenköpfen und Leerzeilen.
fn schluessel_der_zeile(zeile: &str) -> Option<&str> {
    let zeile = zeile.trim_start();
    if zeile.starts_with(['#', '[']) {
        return None;
    }
    let (schluessel, _) = zeile.split_once('=')?;
    Some(schluessel.trim().trim_matches(['"', '\'']))
}

/// Der Kommentar am Ende einer Zeile `schluessel = wert # Kommentar`, ab dem `#`. Ein `#`
/// in Anführungszeichen beginnt keinen Kommentar.
fn kommentar_am_ende(zeile: &str) -> Option<&str> {
    let ab = zeile.find('=')? + 1;
    let mut in_text: Option<char> = None;
    let mut maskiert = false;
    for (i, c) in zeile[ab..].char_indices() {
        match in_text {
            Some('"') if maskiert => maskiert = false,
            Some('"') if c == '\\' => maskiert = true,
            Some(q) if c == q => in_text = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_text = Some(c),
            None if c == '#' => return Some(&zeile[ab + i..]),
            None => {}
        }
    }
    None
}

/// Setzt `schluessel = "wert"` in den Text einer Konfigurationsdatei: Die erste Zeile
/// mit diesem Schlüssel vor der ersten Tabelle wird ersetzt, ein Kommentar an ihrem Ende
/// bleibt; fehlt der Schlüssel, kommt eine Zeile ans Ende. Alle übrigen Zeilen bleiben, wie
/// sie sind, samt Kommentaren und Zeilenenden (CRLF oder LF).
fn zeile_setzen(text: &str, schluessel: &str, wert: &str) -> String {
    let ende = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let neu = format!("{schluessel} = \"{wert}\"");
    let mut aus = String::with_capacity(text.len() + neu.len() + 2);
    let mut gesetzt = false;
    let mut in_tabelle = false;
    for zeile in text.split_inclusive('\n') {
        let inhalt = zeile.trim_end_matches(['\r', '\n']);
        in_tabelle |= inhalt.trim_start().starts_with('[');
        if !gesetzt && !in_tabelle && schluessel_der_zeile(inhalt) == Some(schluessel) {
            aus.push_str(&neu);
            if let Some(k) = kommentar_am_ende(inhalt) {
                aus.push(' ');
                aus.push_str(k);
            }
            aus.push_str(&zeile[inhalt.len()..]);
            gesetzt = true;
        } else {
            aus.push_str(zeile);
        }
    }
    if !gesetzt {
        if !aus.is_empty() && !aus.ends_with('\n') {
            aus.push_str(ende);
        }
        aus.push_str(&neu);
        aus.push_str(ende);
    }
    aus
}

/// Was beim Schreiben in eine Datei schiefging: eigene Texte für die häufigen Fälle,
/// sonst der Text des Systems in Klammern. Gibt den Grund (mit «: » oder « (» davor) und
/// den nächsten Schritt zurück. `e` ist `None`, wenn der Pfad ein Ordner ist.
fn schreibfehler_grund(e: Option<&io::Error>, englisch: bool) -> (String, String) {
    // ERROR_SHARING_VIOLATION (32), ERROR_LOCK_VIOLATION (33): Ein anderes Programm hält
    // die Datei offen, unter Windows oft Word oder Excel.
    let offen = cfg!(windows) && e.is_some_and(|e| matches!(e.raw_os_error(), Some(32 | 33)));
    let art = e.map(io::Error::kind);
    let (grund, schritt): (&str, &str) = match (e, offen, art, englisch) {
        (None, _, _, false) => (
            ": Der Pfad ist ein Ordner",
            "Einen Dateinamen angeben, etwa --output\u{A0}bericht.txt",
        ),
        (None, _, _, true) => (
            ": the path is a folder",
            "Give a file name, such as --output\u{A0}report.txt",
        ),
        (_, true, _, false) => (
            ": Die Datei ist in einem anderen Programm geöffnet",
            "Die Datei dort schliessen, etwa in Word oder Excel, oder einen anderen Namen wählen.",
        ),
        (_, true, _, true) => (
            ": the file is open in another program",
            "Close it there, for example in Word or Excel, or choose another name.",
        ),
        (_, _, Some(io::ErrorKind::NotFound), false) => (
            ": Den Ordner gibt es nicht",
            "Pfad prüfen; promptscan legt keinen Ordner an.",
        ),
        (_, _, Some(io::ErrorKind::NotFound), true) => (
            ": the folder does not exist",
            "Check the path; promptscan does not create folders.",
        ),
        (_, _, Some(io::ErrorKind::PermissionDenied), false) => (
            ": Zugriff verweigert",
            "Einen Ordner wählen, in dem Schreiben erlaubt ist, etwa Dokumente; oder die Rechte prüfen.",
        ),
        (_, _, Some(io::ErrorKind::PermissionDenied), true) => (
            ": access denied",
            "Choose a folder you may write to, such as Documents; or check the permissions.",
        ),
        (_, _, Some(io::ErrorKind::StorageFull), false) => (
            ": Der Datenträger ist voll",
            "Platz schaffen oder einen anderen Datenträger wählen.",
        ),
        (_, _, Some(io::ErrorKind::StorageFull), true) => (
            ": the disk is full",
            "Free up space or choose another disk.",
        ),
        (_, _, Some(io::ErrorKind::ReadOnlyFilesystem), false) => (
            ": Der Datenträger ist schreibgeschützt",
            "Einen anderen Ordner oder Datenträger wählen.",
        ),
        (_, _, Some(io::ErrorKind::ReadOnlyFilesystem), true) => {
            (": the disk is read-only", "Choose another folder or disk.")
        }
        (Some(e), _, _, false) => {
            return (
                format!(" ({})", os_meldung(e)),
                "Pfad und Rechte prüfen.".to_string(),
            );
        }
        (Some(e), _, _, true) => {
            return (
                format!(" ({})", os_meldung(e)),
                "Check the path and the permissions.".to_string(),
            );
        }
    };
    (grund.to_string(), schritt.to_string())
}

/// Warum `config` den Ordner der Konfiguration nicht anlegen kann. Wie
/// [`schreibfehler_grund`], nur heisst `NotFound` hier: Das Laufwerk oder die Freigabe
/// fehlt (etwa ein getrenntes Netzlaufwerk); fehlende Ordner legt `config` ja an.
fn ordner_anlegen_grund(e: &io::Error, englisch: bool) -> (String, String) {
    if e.kind() != io::ErrorKind::NotFound {
        return schreibfehler_grund(Some(e), englisch);
    }
    let (grund, schritt) = if englisch {
        (
            ": the drive is not available",
            "Check the path, for a network drive the connection; or set PROMPTSCAN_CONFIG to another file.",
        )
    } else {
        (
            ": Das Laufwerk ist nicht erreichbar",
            "Pfad prüfen, bei einem Netzlaufwerk die Verbindung; oder PROMPTSCAN_CONFIG auf eine andere Datei setzen.",
        )
    };
    (grund.to_string(), schritt.to_string())
}

/// Schreibt `bytes` in die Datei; ein Ordner als Ziel und jeder Fehler ergeben die Meldung
/// nach [`schreibfehler_grund`]. `was` steht vor «lässt sich nicht schreiben».
fn datei_schreiben(pfad: &Path, bytes: &[u8], was: &str, englisch: bool) -> Result<(), String> {
    let fehler = if pfad.is_dir() {
        Some(schreibfehler_grund(None, englisch))
    } else {
        std::fs::write(pfad, bytes)
            .err()
            .map(|e| schreibfehler_grund(Some(&e), englisch))
    };
    match fehler {
        None => Ok(()),
        Some((grund, schritt)) => Err(format!(
            "{was} {}{grund}. {}\n{schritt}",
            if englisch {
                "cannot be written"
            } else {
                "lässt sich nicht schreiben"
            },
            datei_angabe(pfad, englisch)
        )),
    }
}

/// Schreibt `role` und `lang` in die Datei: UTF-8 ohne BOM, legt den Ordner an.
/// Nur die Zeilen der beiden Schlüssel ändern sich; Kommentare, Reihenfolge und die übrigen
/// Schlüssel bleiben ([`zeile_setzen`]). Ergibt das nicht genau die erwartete Tabelle, etwa
/// bei einer Tabelle oder einem mehrzeiligen Wert, wird die Datei aus der Tabelle neu
/// geschrieben. Ist die bisherige Datei nicht lesbar, wird sie ersetzt. Beides meldet der
/// Rückgabewert als Warnung.
fn konfiguration_schreiben(
    pfad: &Path,
    rolle: Option<Rolle>,
    sprache: Option<SpracheArg>,
    englisch: bool,
) -> Result<Option<String>, String> {
    let mut alt = String::new();
    let mut tabelle = toml::Table::new();
    let mut warnung = None;
    match std::fs::read(pfad) {
        Ok(bytes) => match dekodieren(&bytes)
            .ok()
            .and_then(|t| t.parse::<toml::Table>().ok().map(|tab| (t, tab)))
        {
            Some((t, tab)) => (alt, tabelle) = (t, tab),
            None => {
                warnung = Some(konfig_meldung(
                    if englisch {
                        "The previous file was not readable; it has been replaced."
                    } else {
                        "Die bisherige Datei war nicht lesbar; sie ist ersetzt."
                    },
                    pfad,
                    englisch,
                ));
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(_) if pfad.is_dir() => {
            return Err(konfig_fehler_text(&KonfigFehler::Ordner, pfad, englisch));
        }
        Err(e) => {
            return Err(konfig_meldung(
                &if englisch {
                    format!(
                        "The file cannot be read ({}).\nCheck the path; PROMPTSCAN_CONFIG must point to a file.",
                        os_meldung(&e)
                    )
                } else {
                    format!(
                        "Die Datei lässt sich nicht lesen ({}).\nPfad prüfen; PROMPTSCAN_CONFIG muss auf eine Datei zeigen.",
                        os_meldung(&e)
                    )
                },
                pfad,
                englisch,
            ));
        }
    }
    let mut text = alt;
    for (schluessel, wert) in [
        ("role", rolle.map(Rolle::name)),
        ("lang", sprache.map(sprache_name)),
    ] {
        if let Some(w) = wert {
            tabelle.insert(schluessel.into(), toml::Value::String(w.into()));
            text = zeile_setzen(&text, schluessel, w);
        }
    }
    if text.parse::<toml::Table>().ok().as_ref() != Some(&tabelle) {
        text = toml::to_string(&tabelle).map_err(|e| e.to_string())?;
        warnung.get_or_insert_with(|| {
            konfig_meldung(
                if englisch {
                    "The file has been rewritten from its values; comments have been dropped."
                } else {
                    "Die Datei ist aus ihren Werten neu geschrieben; Kommentare sind weggefallen."
                },
                pfad,
                englisch,
            )
        });
    }
    if let Some(ordner) = pfad.parent().filter(|o| !o.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(ordner)
    {
        // Ältere Standardbibliotheken melden ein fehlendes Laufwerk nicht als `NotFound`,
        // sondern als «failed to create whole tree» (Rust 1.88).
        let laufwerk_fehlt = ordner
            .ancestors()
            .last()
            .is_some_and(|w| w.has_root() && !w.exists());
        let e = if laufwerk_fehlt {
            io::Error::from(io::ErrorKind::NotFound)
        } else {
            e
        };
        let (grund, schritt) = ordner_anlegen_grund(&e, englisch);
        return Err(konfig_meldung(
            &if englisch {
                format!("The folder cannot be created{grund}.\n{schritt}")
            } else {
                format!("Der Ordner lässt sich nicht anlegen{grund}.\n{schritt}")
            },
            pfad,
            englisch,
        ));
    }
    let was = if englisch {
        "The configuration"
    } else {
        "Die Konfiguration"
    };
    datei_schreiben(pfad, text.as_bytes(), was, englisch)?;
    Ok(warnung)
}

/// Text für `promptscan config`: Pfad und Herkunft, Inhalt, gültige Werte, wie ändern.
fn konfiguration_text(k: &Konfig, englisch: bool) -> String {
    let mut aus = String::new();
    let Some((pfad, quelle)) = &k.ort else {
        return aus;
    };
    let t = |de: &'static str, en: &'static str| if englisch { en } else { de };
    let _ = writeln!(aus, "{}", t("Konfigurationsdatei", "Configuration file"));
    let _ = writeln!(aus, "  {}", pfad.display());
    let _ = writeln!(
        aus,
        "  {} {}{}",
        t("Ort aus", "Location from"),
        quelle.text(englisch),
        if *quelle == OrtQuelle::Umgebung {
            ""
        } else {
            t(
                "; PROMPTSCAN_CONFIG geht vor",
                "; PROMPTSCAN_CONFIG takes precedence",
            )
        }
    );
    if !k.vorhanden {
        let _ = writeln!(
            aus,
            "  {}",
            t("(gibt es noch nicht)", "(does not exist yet)")
        );
    }
    let _ = writeln!(aus, "\n{}", t("Inhalt", "Content"));
    let mut zeilen: Vec<(String, String)> = Vec::new();
    if let Some(w) = &k.werte.role {
        let deutung = match Rolle::aus_eingabe(w) {
            Some(r) if r.name() == w.as_str() => r.anzeige(englisch).to_string(),
            Some(r) => format!("{} ({})", r.anzeige(englisch), r.name()),
            None => t("unbekannt", "unknown").to_string(),
        };
        zeilen.push((format!("role = \"{w}\""), deutung));
    }
    if let Some(w) = &k.werte.lang {
        let deutung = match sprache_aus_wert(w) {
            Some(false) => t("Deutsch", "German"),
            Some(true) => t("Englisch", "English"),
            None => t("unbekannt", "unknown"),
        };
        zeilen.push((format!("lang = \"{w}\""), deutung.to_string()));
    }
    if let Some(w) = &k.werte.target {
        zeilen.push((
            format!("target = \"{w}\""),
            t(
                "wird übergangen; --target je Aufruf",
                "ignored; --target per call",
            )
            .to_string(),
        ));
    }
    if zeilen.is_empty() {
        let _ = writeln!(aus, "  {}", t("(keine Werte)", "(no values)"));
    }
    for (links, rechts) in &zeilen {
        let _ = writeln!(aus, "  {links:<26}{rechts}");
    }
    let _ = writeln!(aus, "\n{}", t("Gültige Werte", "Valid values"));
    let rollen = format!(
        "{}; {}",
        rollenliste(),
        t(
            "auch deutsch, etwa anwalt oder ärztin. Pflicht und Normen je Rolle: promptscan\u{A0}explain\u{A0}rot",
            "German words such as anwalt also work. Duty and provisions per role: promptscan\u{A0}explain\u{A0}red"
        )
    );
    zweispaltig(&mut aus, "role", &rollen, 10);
    zweispaltig(&mut aus, "lang", "de, en", 10);
    let _ = writeln!(aus, "\n{}", t("Ändern", "Change"));
    let _ = writeln!(aus, "  promptscan config --role lawyer");
    let _ = writeln!(aus, "  promptscan config --lang en");
    ascii_wenn_verlangt(&aus.replace('\u{A0}', " ")).into_owned()
}

/// «Gespeichert: role = "lawyer" (bisher "doctor")» für jeden geschriebenen Schlüssel.
fn gespeichert_text(
    alt: &Werte,
    rolle: Option<Rolle>,
    sprache: Option<SpracheArg>,
    englisch: bool,
) -> String {
    let mut teile = Vec::new();
    for (schluessel, neu, vorher) in [
        ("role", rolle.map(Rolle::name), alt.role.as_deref()),
        ("lang", sprache.map(sprache_name), alt.lang.as_deref()),
    ] {
        let Some(neu) = neu else { continue };
        let mut teil = format!("{schluessel} = \"{neu}\"");
        if let Some(v) = vorher.filter(|v| *v != neu) {
            let _ = write!(
                teil,
                " ({} \"{v}\")",
                if englisch { "previously" } else { "bisher" }
            );
        }
        teile.push(teil);
    }
    format!(
        "{}: {}.",
        if englisch { "Saved" } else { "Gespeichert" },
        teile.join(", ")
    )
}

/// `promptscan config`: zeigt die Datei. Mit `--role` oder `--lang` nach «config» schreibt
/// es sie; vor «config» gelten die Optionen wie bei `rules` nur für diesen Aufruf, `--lang`
/// also für die Sprache der Ansicht. Die Ausgabe ist immer Text;
/// `--format json` gilt hier nicht.
fn befehl_config(
    k: &Konfig,
    cli: &Cli,
    vor: &Vorpruefung,
    rolle: Option<Rolle>,
    englisch: bool,
) -> ExitCode {
    let Some((pfad, _)) = &k.ort else {
        return fehler(if englisch {
            "There is no path for the configuration: neither PROMPTSCAN_CONFIG nor %APPDATA% (Windows) or HOME is set.\nSet PROMPTSCAN_CONFIG to a file, such as config.toml."
        } else {
            "Für die Konfiguration gibt es keinen Pfad: Weder PROMPTSCAN_CONFIG noch %APPDATA% (Windows) bzw. HOME ist gesetzt.\nPROMPTSCAN_CONFIG auf eine Datei setzen, etwa config.toml."
        });
    };
    if cli.target.is_some() {
        melden(if englisch {
            "promptscan config does not save the target.\nThe target applies per call: --target\u{A0}local"
        } else {
            "promptscan config speichert kein Ziel.\nDas Ziel gilt je Aufruf: --target\u{A0}local"
        });
    }
    let rolle_schreiben = rolle.filter(|_| vor.rolle_nach_befehl);
    let sprache_schreiben = cli.lang.filter(|_| vor.sprache_nach_befehl);
    if let (Some(r), None) = (rolle, rolle_schreiben) {
        let befehl = nb(&format!("promptscan config --role {}", r.name()));
        melden(&if englisch {
            format!("--role before «config» saves nothing.\nTo save it: {befehl}")
        } else {
            format!("--role vor «config» speichert nichts.\nZum Speichern: {befehl}")
        });
    }
    let schreibt = rolle_schreiben.is_some() || sprache_schreiben.is_some();
    if schreibt {
        match konfiguration_schreiben(pfad, rolle_schreiben, sprache_schreiben, englisch) {
            Ok(Some(warnung)) => melden(&warnung),
            Ok(None) => {}
            Err(m) => return fehler(&m),
        }
    }
    // Der Stand nach dem Schreiben, mit allem, was an ihm falsch ist, als Warnung.
    let jetzt = Konfig::lesen_von(k.ort.clone());
    for w in konfiguration_anwenden(&jetzt, None, None, false, englisch).warnungen {
        melden(&w);
    }
    let mut aus = String::new();
    if schreibt {
        let gespeichert = gespeichert_text(&k.werte, rolle_schreiben, sprache_schreiben, englisch);
        let _ = writeln!(aus, "{}\n", ascii_wenn_verlangt(&gespeichert));
    }
    aus.push_str(&konfiguration_text(&jetzt, englisch));
    abschliessen(&aus, false, 0, englisch, cli.output.as_deref())
}

// ---------------------------------------------------------------------------------------
// Eingabe
// ---------------------------------------------------------------------------------------

/// Warum eine Eingabe nicht gelesen wird (docs/AUSGABE.md, «Kodierung der Eingabe»).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lesefehler {
    /// BOM von UTF-8, danach kein gültiges UTF-8.
    Utf8,
    /// BOM von UTF-16, danach eine ungerade Zahl Bytes.
    Utf16Ungerade,
    /// BOM von UTF-16, danach kein gültiges UTF-16.
    Utf16,
    /// BOM von UTF-32.
    Utf32,
    /// Das Zeichen NUL (U+0000). Ohne BOM ist das meist UTF-16 ohne BOM: In reinem
    /// ASCII-Text ist dann jedes zweite Byte 0, und das wäre gültiges UTF-8.
    Nul { ohne_bom: bool },
    /// Ohne BOM und kein gültiges UTF-8, etwa Windows-1252.
    Unbekannt,
}

impl Lesefehler {
    fn meldung(self, englisch: bool) -> &'static str {
        match (self, englisch) {
            (Lesefehler::Utf8, false) => "ungültiges UTF-8",
            (Lesefehler::Utf8, true) => "invalid UTF-8",
            (Lesefehler::Utf16Ungerade, false) => "UTF-16 mit ungerader Bytezahl",
            (Lesefehler::Utf16Ungerade, true) => "UTF-16 with an odd number of bytes",
            (Lesefehler::Utf16, false) => "ungültiges UTF-16",
            (Lesefehler::Utf16, true) => "invalid UTF-16",
            (Lesefehler::Utf32, false) => "UTF-32 wird nicht gelesen; bitte als UTF-8 speichern",
            (Lesefehler::Utf32, true) => "UTF-32 is not read; please save as UTF-8",
            (Lesefehler::Nul { ohne_bom: true }, false) => {
                "enthält das Zeichen NUL (U+0000), vermutlich UTF-16 ohne BOM; bitte als UTF-8 speichern"
            }
            (Lesefehler::Nul { ohne_bom: true }, true) => {
                "contains the character NUL (U+0000), probably UTF-16 without BOM; please save as UTF-8"
            }
            (Lesefehler::Nul { ohne_bom: false }, false) => "enthält das Zeichen NUL (U+0000)",
            (Lesefehler::Nul { ohne_bom: false }, true) => "contains the character NUL (U+0000)",
            (Lesefehler::Unbekannt, false) => {
                "weder UTF-8 noch UTF-16 mit BOM; bitte als UTF-8 speichern"
            }
            (Lesefehler::Unbekannt, true) => {
                "neither UTF-8 nor UTF-16 with BOM; please save as UTF-8"
            }
        }
    }
}

/// Dekodiert Bytes: UTF-8 (mit oder ohne BOM) oder UTF-16 mit BOM, wie es Windows PowerShell 5.1
/// mit `>` schreibt. Anderes wird abgewiesen statt geraten: UTF-32 (am BOM erkannt) und jede
/// Eingabe mit dem Zeichen NUL, denn in ihr greift keine Regel. Gilt für Prompt und
/// Konfiguration.
fn dekodieren(bytes: &[u8]) -> Result<String, Lesefehler> {
    // Vor UTF-16: Das BOM von UTF-32LE beginnt mit dem von UTF-16LE.
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        return Err(Lesefehler::Utf32);
    }
    let utf16 = |rest: &[u8], le: bool| {
        if !rest.len().is_multiple_of(2) {
            return Err(Lesefehler::Utf16Ungerade);
        }
        let einheiten: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| {
                if le {
                    u16::from_le_bytes(c)
                } else {
                    u16::from_be_bytes(c)
                }
            })
            .collect();
        String::from_utf16(&einheiten).map_err(|_| Lesefehler::Utf16)
    };
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(rest.to_vec()).map_err(|_| Lesefehler::Utf8)?
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        utf16(rest, true)?
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        utf16(rest, false)?
    } else {
        // In UTF-8 ist ein Nullbyte immer das Zeichen NUL, nie Teil eines anderen Zeichens.
        // Geprüft vor der Gültigkeit, damit auch UTF-16 ohne BOM mit Umlauten die Meldung
        // zu UTF-16 erhält.
        if bytes.contains(&0) {
            return Err(Lesefehler::Nul { ohne_bom: true });
        }
        return String::from_utf8(bytes.to_vec()).map_err(|_| Lesefehler::Unbekannt);
    };
    if text.contains('\0') {
        return Err(Lesefehler::Nul { ohne_bom: false });
    }
    Ok(text)
}

/// Liest die Prompt-Datei; Fehler zweizeilig, ohne Systemcode. Der Pfad steht am Ende der
/// ersten Zeile, bei Platzmangel ganz auf der nächsten.
fn datei_lesen(pfad: &Path, englisch: bool) -> Result<String, String> {
    let p = pfad_nb(pfad);
    if pfad.is_dir() {
        return Err(if englisch {
            format!(
                "The path is a folder, not a file: {p}\nGive the path of a file, such as {}",
                nb("promptscan prompt.txt")
            )
        } else {
            format!(
                "Der Pfad ist ein Ordner, keine Datei: {p}\nDen Pfad einer Datei angeben, etwa {}",
                nb("promptscan prompt.txt")
            )
        });
    }
    // Ein blosses Wort nahe an einem Befehl («rule», «expain») ist wohl ein Tippfehler.
    let befehl = pfad
        .to_str()
        .filter(|s| !s.contains(['.', '/', '\\']))
        .and_then(|s| vorschlag(s, &BEFEHLE.map(|b| (b, b))));
    let hilfe = nb("promptscan --help");
    let bytes = std::fs::read(pfad).map_err(|e| match (e.kind(), befehl, englisch) {
        (io::ErrorKind::NotFound, Some(b), false) => format!(
            "Die Datei gibt es nicht: {p}\nGemeint ist wohl der Befehl: {}\nAlle Befehle: {hilfe}",
            nb(&format!("promptscan {b}"))
        ),
        (io::ErrorKind::NotFound, Some(b), true) => format!(
            "The file does not exist: {p}\nDid you mean the command: {}\nAll commands: {hilfe}",
            nb(&format!("promptscan {b}"))
        ),
        (io::ErrorKind::NotFound, None, false) => format!(
            "Die Datei gibt es nicht: {p}\nPfad prüfen; oder promptscan ohne Datei starten und den Prompt einfügen."
        ),
        (io::ErrorKind::NotFound, None, true) => format!(
            "The file does not exist: {p}\nCheck the path; or start promptscan without a file and paste the prompt."
        ),
        (io::ErrorKind::PermissionDenied, _, false) => format!(
            "Die Datei lässt sich nicht lesen, Zugriff verweigert: {p}\nDie Rechte prüfen oder den Prompt einfügen."
        ),
        (io::ErrorKind::PermissionDenied, _, true) => format!(
            "The file cannot be read, access denied: {p}\nCheck the permissions or paste the prompt."
        ),
        (_, _, false) => format!(
            "Die Datei lässt sich nicht lesen ({}): {p}\nPfad prüfen; oder den Prompt einfügen.",
            os_meldung(&e)
        ),
        (_, _, true) => format!(
            "The file cannot be read ({}): {p}\nCheck the path; or paste the prompt.",
            os_meldung(&e)
        ),
    })?;
    dekodieren(&bytes).map_err(|e| format!("{p}: {}", e.meldung(englisch)))
}

/// Hinweis vor dem Einfügen. Unter Windows endet die Eingabe nur, wenn Ctrl+Z am
/// Anfang einer Zeile steht; darum zuerst Enter.
fn einfuege_hinweis(windows: bool, englisch: bool) -> &'static str {
    match (windows, englisch) {
        (true, false) => {
            "promptscan (Terminalprogramm): Prompt einfügen, Enter, dann Ctrl+Z und Enter."
        }
        (true, true) => {
            "promptscan (terminal program): paste the prompt, press Enter, then Ctrl+Z and Enter."
        }
        (false, false) => "promptscan (Terminalprogramm): Prompt einfügen, Enter, dann Ctrl+D.",
        (false, true) => {
            "promptscan (terminal program): paste the prompt, press Enter, then Ctrl+D."
        }
    }
}

fn eingabe_lesen(cli: &Cli, englisch: bool) -> Result<String, String> {
    if let Some(t) = &cli.text {
        return Ok(t.clone());
    }
    if let Some(pfad) = &cli.datei {
        return datei_lesen(pfad, englisch);
    }
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        let _ = writeln!(
            io::stderr(),
            "{}",
            ascii_wenn_verlangt(einfuege_hinweis(cfg!(windows), englisch))
        );
    }
    let mut bytes = Vec::new();
    stdin.read_to_end(&mut bytes).map_err(|e| {
        if englisch {
            format!("The input cannot be read: {}.", os_meldung(&e))
        } else {
            format!("Die Eingabe lässt sich nicht lesen: {}.", os_meldung(&e))
        }
    })?;
    dekodieren(&bytes).map_err(|e| e.meldung(englisch).to_string())
}

/// Optionen von promptscan; stehen sie im Text von `-t` hinter einem geraden
/// Anführungszeichen, hat die Shell sie verschluckt.
const OPTIONEN: [&str; 13] = [
    "--role",
    "--target",
    "--format",
    "--lang",
    "--fail-on",
    "--no-color",
    "--output",
    "--quiet",
    "--ascii",
    "--text",
    "--help",
    "-t",
    "-h",
];

/// Endet der Text von `-t` auf ein gerades Anführungszeichen, gefolgt von einer Option
/// von promptscan, gibt es den Rest ab dem Anführungszeichen zurück. So sieht es aus, wenn
/// ein `\` vor dem schliessenden `"` dieses schützt und die Shell (Windows PowerShell 5.1,
/// cmd) alle folgenden Optionen in den Text zieht.
fn verschluckte_optionen(text: &str) -> Option<&str> {
    for (i, _) in text.match_indices('"') {
        let rest = &text[i + 1..];
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let Some(wort) = rest.split_whitespace().next() else {
            continue;
        };
        let name = wort.split('=').next().unwrap_or(wort);
        if OPTIONEN.contains(&name) {
            return Some(&text[i..]);
        }
    }
    None
}

fn verschluckte_optionen_text(rest: &str, englisch: bool) -> String {
    let mut gezeigt: String = rest.chars().take(40).collect();
    if gezeigt.len() < rest.len() {
        gezeigt.push('…');
    }
    if englisch {
        format!(
            "The text after -t contains options of promptscan: «{gezeigt}».\nA \\ before the closing \" probably made the shell swallow them. Put the options before -t, drop the \\ at the end, or pass the prompt as a file."
        )
    } else {
        format!(
            "Im Text nach -t stehen Optionen von promptscan: «{gezeigt}».\nVermutlich hat ein \\ vor dem schliessenden \" sie in den Text gezogen. Optionen vor -t schreiben, den \\ am Ende weglassen oder den Prompt als Datei übergeben."
        )
    }
}

/// Hinweis bei einem Fehler der Befehlszeile mit `-t`.
fn anfuehrungszeichen_hinweis(englisch: bool) -> &'static str {
    if englisch {
        "Straight quotes (\") in the text after -t? Windows PowerShell 5.1 and cmd then split the text. Pass the prompt as a file or paste it; in PowerShell also: promptscan\u{A0}--%\u{A0}-t\u{A0}\"...\u{A0}\\\"...\\\"\u{A0}...\""
    } else {
        "Gerade Anführungszeichen (\") im Text nach -t? Windows PowerShell 5.1 und cmd zerlegen ihn dann. Den Prompt als Datei übergeben oder einfügen; in PowerShell auch: promptscan\u{A0}--%\u{A0}-t\u{A0}\"...\u{A0}\\\"...\\\"\u{A0}...\""
    }
}

/// -t, eine Datei und ein Befehl schliessen sich aus. Hat die «Datei» Leerzeichen, ist sie
/// wohl ein Stück des Textes von -t, das die Shell an geraden Anführungszeichen abgetrennt
/// hat; dann sagt das die erste Zeile.
fn eingabe_konflikt(cli: &Cli, englisch: bool) -> Option<String> {
    let befehl = cli.befehl.as_ref().map(Befehl::name);
    let datei = cli.datei.as_ref().map(|d| d.display().to_string());
    let zerlegt = datei
        .as_deref()
        .is_some_and(|d| d.contains(char::is_whitespace));
    let hinweis = anfuehrungszeichen_hinweis(englisch);
    match (befehl, cli.text.is_some(), datei, englisch) {
        (Some(b), true, _, false) => Some(format!(
            "-t und der Befehl {b} gehen nicht zusammen.\nText prüfen: promptscan\u{A0}-t\u{A0}\"…\"; Befehl allein: {}",
            nb(&format!("promptscan {b}"))
        )),
        (Some(b), true, _, true) => Some(format!(
            "-t and the command {b} do not go together.\nCheck a text: promptscan\u{A0}-t\u{A0}\"…\"; the command alone: {}",
            nb(&format!("promptscan {b}"))
        )),
        (Some(b), false, Some(d), false) => Some(format!(
            "Eine Datei («{}») und der Befehl {b} gehen nicht zusammen.\nDatei prüfen: {}; Befehl allein: {}",
            nb(&d),
            nb(&format!("promptscan {d}")),
            nb(&format!("promptscan {b}"))
        )),
        (Some(b), false, Some(d), true) => Some(format!(
            "A file («{}») and the command {b} do not go together.\nCheck the file: {}; the command alone: {}",
            nb(&d),
            nb(&format!("promptscan {d}")),
            nb(&format!("promptscan {b}"))
        )),
        (None, true, Some(d), false) if zerlegt => Some(format!(
            "Der Text nach -t ist zerfallen: «{d}» kam einzeln an.\n{hinweis}"
        )),
        (None, true, Some(d), true) if zerlegt => Some(format!(
            "The text after -t has been split: «{d}» arrived separately.\n{hinweis}"
        )),
        (None, true, Some(d), false) => Some(format!(
            "-t und eine Datei («{}») gehen nicht zusammen.\nEntweder den Text mit -t oder eine Datei angeben.\n{hinweis}",
            nb(&d)
        )),
        (None, true, Some(d), true) => Some(format!(
            "-t and a file («{}») do not go together.\nGive either the text with -t or a file.\n{hinweis}",
            nb(&d)
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// Darstellung: Breite, ASCII, Farben, Konsole
// ---------------------------------------------------------------------------------------

/// Breite für die Ausgabe: bei Umleitung oder `--output` 80; sonst die Breite des
/// Konsolenfensters, sonst `COLUMNS`, sonst 80; nie unter 60.
fn breite_bestimmen(umgeleitet: bool, konsole: Option<usize>, columns: Option<&str>) -> usize {
    if umgeleitet {
        return 80;
    }
    konsole
        .or_else(|| columns.and_then(|c| c.trim().parse::<usize>().ok()))
        .filter(|&b| b > 0)
        .unwrap_or(80)
        .max(60)
}

/// ASCII-Rückfall ohne `--ascii`: nur ausserhalb von Windows, wenn die massgebende
/// Locale-Variable (`LC_ALL` vor `LC_CTYPE` vor `LANG`, die erste nicht leere) gesetzt ist
/// und kein UTF-8 nennt, etwa `C` oder `de_CH.ISO-8859-1`.
fn ascii_aus_umgebung(
    windows: bool,
    lc_all: Option<&str>,
    lc_ctype: Option<&str>,
    lang: Option<&str>,
) -> bool {
    if windows {
        return false;
    }
    match [lc_all, lc_ctype, lang]
        .into_iter()
        .flatten()
        .find(|w| !w.is_empty())
    {
        Some(w) => {
            let w = w.to_lowercase();
            !(w.contains("utf-8") || w.contains("utf8"))
        }
        None => false,
    }
}

/// Nur ASCII? Mit `--ascii` immer; sonst nach der Locale ([`ascii_aus_umgebung`]),
/// ausser die Ausgabe geht mit `--output` in eine Datei (UTF-8 mit BOM).
fn ascii_gewuenscht(option: bool, in_datei: bool) -> bool {
    let umgebung = |n: &str| std::env::var(n).ok();
    option
        || (!in_datei
            && ascii_aus_umgebung(
                cfg!(windows),
                umgebung("LC_ALL").as_deref(),
                umgebung("LC_CTYPE").as_deref(),
                umgebung("LANG").as_deref(),
            ))
}

/// Umschrift für `--ascii`, wo die kanonische Zerlegung (NFD: é → e) nicht reicht:
/// Umlaute wie im Deutschen ohne Umlaut, Anführungszeichen gerade, Rahmen und Striche als
/// `|`, `-` und `+`.
fn umschrift(c: char) -> Option<&'static str> {
    Some(match c {
        'ä' | 'æ' => "ae",
        'ö' | 'œ' => "oe",
        'ü' => "ue",
        'Ä' | 'Æ' => "Ae",
        'Ö' | 'Œ' => "Oe",
        'Ü' => "Ue",
        'ß' => "ss",
        'ø' => "o",
        'Ø' => "O",
        'đ' => "d",
        'ł' => "l",
        'Ł' => "L",
        '«' | '»' | '„' | '“' | '”' | '\u{2033}' => "\"",
        '‹' | '›' | '‚' | '‘' | '’' | '\u{2032}' => "'",
        '…' => "...",
        '·' | '•' | '–' | '—' | '\u{2010}' | '\u{2011}' | '\u{2212}' | '─' | '━' => "-",
        '│' | '┃' | '║' => "|",
        '┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' => "+",
        '→' => "->",
        '←' => "<-",
        '×' => "x",
        '€' => "EUR",
        '\u{A0}' | '\u{2007}' | '\u{2009}' | '\u{200A}' | '\u{202F}' => " ",
        _ => return None,
    })
}

/// Belegt das Zeichen im Terminal zwei Spalten? Die grossen Bereiche mit East_Asian_Width
/// W oder F (Hangul, CJK, Vollbreite, Emoji). Nur für das Sicherheitsnetz in
/// [`nach_ascii`]; die Ausgabe (`src/ausgabe.rs`) rechnet mit allen Bereichen.
fn zwei_spalten(c: char) -> bool {
    matches!(
        u32::from(c),
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F680..=0x1F6FF
            | 0x1F900..=0x1F9FF
            | 0x1FA70..=0x1FAFF
            | 0x20000..=0x3FFFD
    )
}

/// Text in ASCII. Zwei Arten:
/// - lesbar (`gleich_breit` falsch), für eigene Texte vor dem Umbruch: ä → ae, « → ",
///   … → ...; das geschützte Leerzeichen U+00A0 bleibt für den Umbruch stehen.
/// - `gleich_breit`, das Sicherheitsnetz für Bericht, `rules` und `explain`: Jedes Zeichen
///   wird zu so vielen Zeichen, wie es Spalten belegt (ä → a, ein breites Zeichen → ??,
///   ein kombinierendes zu nichts). So bleiben die Marken `^^^` unter ihrer Stelle und kein
///   Umbruch verschiebt sich. Ist die Ausgabe schon ASCII, ändert sich nichts.
///
/// Was sich nicht umschreiben lässt, wird zu `?`.
fn nach_ascii(text: &str, gleich_breit: bool) -> String {
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;
    let mut aus = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() || (c == '\u{A0}' && !gleich_breit) {
            aus.push(c);
            continue;
        }
        if is_combining_mark(c)
            || matches!(c, '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{FE00}'..='\u{FE0F}')
        {
            continue;
        }
        let ersatz: String = match umschrift(c) {
            Some(s) => s.to_string(),
            None => {
                let basis: String = c.nfd().filter(char::is_ascii).collect();
                if basis.is_empty() {
                    "?".to_string()
                } else {
                    basis
                }
            }
        };
        if gleich_breit {
            let zeichen = ersatz.chars().next().unwrap_or('?');
            aus.push(zeichen);
            if zwei_spalten(c) {
                aus.push(zeichen);
            }
        } else {
            aus.push_str(&ersatz);
        }
    }
    aus
}

/// Eigene Texte (Hilfe, Meldungen, `config`) lesbar in ASCII, wenn [`nur_ascii`] gilt.
fn ascii_wenn_verlangt(text: &str) -> std::borrow::Cow<'_, str> {
    if nur_ascii() && !text.is_ascii() {
        std::borrow::Cow::Owned(nach_ascii(text, false))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// Kennt das Terminal nur 16 Farben? Ja, wenn `TERM` gesetzt ist, weder «256» noch
/// «direct» enthält und `COLORTERM` nicht «truecolor» oder «24bit» meldet. Ohne `TERM`
/// (Windows-Konsole) stehen 256 Farben zur Verfügung.
fn nur_16_farben() -> bool {
    let Some(term) = std::env::var_os("TERM") else {
        return false;
    };
    let term = term.to_string_lossy().to_lowercase();
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_lowercase();
    !term.contains("256")
        && !term.contains("direct")
        && colorterm != "truecolor"
        && colorterm != "24bit"
}

/// Am Ende auf Enter warten? Nur unter Windows, wenn promptscan allein an seiner
/// Konsole hängt und Ein- und Ausgabe die Konsole sind: So sieht ein Start per
/// Doppelklick aus, und die Konsole schlösse sich mit dem letzten Prozess. Nie bei
/// Umleitung oder Pipe, nie in einer Shell (dann hängt auch die Shell an der Konsole).
fn am_ende_warten(windows: bool, prozesse: Option<u32>, ein_und_ausgabe_konsole: bool) -> bool {
    windows && prozesse == Some(1) && ein_und_ausgabe_konsole
}

/// Wie lange nach «Zum Schliessen Enter drücken.» ein Enter nicht zählt. Ctrl+Z am Anfang
/// einer Zeile beendet das Einfügen sofort, ohne Enter; das Enter aus «Ctrl+Z und Enter»
/// käme sonst hier an und schlösse das Fenster, bevor jemand den Bericht liest.
const SCHONFRIST: std::time::Duration = std::time::Duration::from_millis(1000);

/// Schliesst ein Enter, das `nach` dem Hinweis kommt, das Fenster?
fn schliesst(nach: std::time::Duration) -> bool {
    nach >= SCHONFRIST
}

/// Wartet auf Enter. Ein Enter in der [`SCHONFRIST`] zählt nicht; Ctrl+Z oder ein
/// Lesefehler beenden sofort.
fn auf_enter_warten(englisch: bool) {
    let _ = write!(
        io::stderr(),
        "\n{}",
        ascii_wenn_verlangt(if englisch {
            "Press Enter to close."
        } else {
            "Zum Schliessen Enter drücken."
        })
    );
    let _ = io::stderr().flush();
    let start = std::time::Instant::now();
    loop {
        let mut zeile = String::new();
        match io::stdin().read_line(&mut zeile) {
            Ok(n) if n > 0 && !schliesst(start.elapsed()) => {}
            _ => return,
        }
    }
}

/// Abfragen an die Windows-Konsole über windows-sys.
#[cfg(windows)]
mod konsole {
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{
        CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO, GetConsoleMode, GetConsoleProcessList,
        GetConsoleScreenBufferInfo, GetStdHandle, STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    fn griff(welcher: STD_HANDLE) -> Option<HANDLE> {
        // SAFETY: GetStdHandle hat keine Vorbedingungen und gibt nur einen Wert zurück.
        let h = unsafe { GetStdHandle(welcher) };
        (!h.is_null() && h != INVALID_HANDLE_VALUE).then_some(h)
    }

    fn ist_konsole(welcher: STD_HANDLE) -> bool {
        let Some(h) = griff(welcher) else {
            return false;
        };
        let mut modus: CONSOLE_MODE = 0;
        // SAFETY: `h` ist ein Griff aus GetStdHandle, `modus` eine gültige Stelle zum
        // Schreiben. Ist `h` keine Konsole, gibt die Funktion 0 zurück.
        unsafe { GetConsoleMode(h, &mut modus) != 0 }
    }

    /// Spalten des Konsolenfensters, in das stdout schreibt (aus `srWindow`).
    pub fn breite() -> Option<usize> {
        let h = griff(STD_OUTPUT_HANDLE)?;
        let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
        // SAFETY: `info` ist eine gültige, beschreibbare Struktur der verlangten Art.
        if unsafe { GetConsoleScreenBufferInfo(h, &mut info) } == 0 {
            return None;
        }
        let spalten = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
        usize::try_from(spalten).ok().filter(|&s| s > 0)
    }

    /// Wie viele Prozesse an der Konsole hängen; `None` ohne Konsole.
    pub fn prozesse() -> Option<u32> {
        const PLATZ: u32 = 8;
        let mut liste = [0_u32; PLATZ as usize];
        // SAFETY: Zeiger und Länge beschreiben das Feld `liste`. Ist es zu klein, gibt die
        // Funktion die nötige Zahl zurück, ohne zu schreiben.
        let n = unsafe { GetConsoleProcessList(liste.as_mut_ptr(), PLATZ) };
        (n > 0).then_some(n)
    }

    /// Sind stdin und stdout die Konsole (keine Datei, keine Pipe)?
    pub fn ein_und_ausgabe() -> bool {
        ist_konsole(STD_INPUT_HANDLE) && ist_konsole(STD_OUTPUT_HANDLE)
    }
}

#[cfg(not(windows))]
mod konsole {
    pub fn breite() -> Option<usize> {
        None
    }

    pub fn prozesse() -> Option<u32> {
        None
    }

    pub fn ein_und_ausgabe() -> bool {
        false
    }
}

fn darstellung(cli: &Cli, englisch: bool) -> Darstellung {
    if cli.no_color || cli.output.is_some() {
        anstream::ColorChoice::Never.write_global();
    }
    let farbe = cli.output.is_none()
        && anstream::AutoStream::choice(&io::stdout()) != anstream::ColorChoice::Never;
    let umgeleitet = cli.output.is_some() || !io::stdout().is_terminal();
    Darstellung {
        englisch,
        farbe,
        nur_16_farben: nur_16_farben(),
        breite: breite_bestimmen(
            umgeleitet,
            konsole::breite(),
            std::env::var("COLUMNS").ok().as_deref(),
        ),
        // Wie in lauf() gesetzt: --ascii, sonst die Locale (nicht bei --output).
        ascii: nur_ascii(),
    }
}

// ---------------------------------------------------------------------------------------
// Ausgabe
// ---------------------------------------------------------------------------------------

/// JSON in reinem ASCII: jedes andere Zeichen als `\uXXXX`, ausserhalb der BMP als
/// Ersatzpaar. Gültig, weil Zeichen ausser ASCII in JSON nur in Zeichenketten stehen; das
/// geparste JSON bleibt gleich. So übersteht es jede Codepage und jede Shell.
fn json_ascii(json: &str) -> String {
    let mut aus = String::with_capacity(json.len());
    for c in json.chars() {
        if c.is_ascii() {
            aus.push(c);
        } else {
            let mut einheiten = [0_u16; 2];
            for e in c.encode_utf16(&mut einheiten) {
                let _ = write!(aus, "\\u{e:04x}");
            }
        }
    }
    aus
}

/// Schreibt die ganze Ausgabe auf stdout, gesperrt und in einem Zug, und gibt den Fehler
/// zurück statt in Panik zu geraten. JSON geht als ASCII hinaus; Text über `anstream`,
/// das ohne Farbe Escape-Sequenzen entfernt.
fn ausgeben(aus: &str, json: bool) -> io::Result<()> {
    if json {
        schreiben(&mut io::stdout().lock(), &json_ascii(aus))
    } else {
        schreiben(&mut anstream::stdout().lock(), aus)
    }
}

fn schreiben(ziel: &mut impl Write, aus: &str) -> io::Result<()> {
    ziel.write_all(aus.as_bytes())?;
    ziel.flush()
}

/// Inhalt einer Datei für `--output`: UTF-8 mit BOM, ohne Escape-Sequenzen; JSON wie auf
/// stdout in ASCII.
fn datei_inhalt(aus: &str, json: bool) -> Vec<u8> {
    let text = if json {
        json_ascii(aus)
    } else {
        anstream::adapter::strip_str(aus).to_string()
    };
    let mut bytes = Vec::with_capacity(text.len() + 3);
    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Exit-Code nach dem Schreiben. Hat der Empfänger die Pipe geschlossen (`| head`,
/// `Select-Object -First`), will er nichts mehr lesen: kein Fehler, der Exit-Code bleibt
/// `code`, der Code der Funde. Jeder andere Schreibfehler ergibt 2 und eine zweizeilige
/// Meldung ([`schreibfehler_grund`]).
fn nach_dem_schreiben(ergebnis: io::Result<()>, code: u8, englisch: bool) -> (u8, Option<String>) {
    match ergebnis {
        Ok(()) => (code, None),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => (code, None),
        Err(e) => {
            let (grund, _) = schreibfehler_grund(Some(&e), englisch);
            let text = if englisch {
                format!(
                    "The output cannot be written{grund}.\nCheck where the output is redirected to, or write it to a file: --output\u{A0}FILE"
                )
            } else {
                format!(
                    "Die Ausgabe lässt sich nicht schreiben{grund}.\nDas Ziel der Umleitung prüfen oder in eine Datei schreiben: --output\u{A0}DATEI"
                )
            };
            (2, Some(text))
        }
    }
}

/// Schreibt die Ausgabe auf stdout oder mit `--output` in die Datei und gibt den
/// Exit-Code zurück: `code`, ausser das Schreiben scheitert (dann 2 und eine Meldung).
/// `json` sagt, ob `aus` JSON ist. Mit `--ascii` geht Text nur in ASCII hinaus
/// ([`nach_ascii`], gleich breit): Das ist das Sicherheitsnetz für jede Zeile, die die
/// Ausgabe selbst nicht umschreibt.
fn abschliessen(aus: &str, json: bool, code: u8, englisch: bool, datei: Option<&Path>) -> ExitCode {
    let aus = if !json && nur_ascii() && !aus.is_ascii() {
        std::borrow::Cow::Owned(nach_ascii(aus, true))
    } else {
        std::borrow::Cow::Borrowed(aus)
    };
    if let Some(pfad) = datei {
        let was = if englisch {
            "The output"
        } else {
            "Die Ausgabe"
        };
        return match datei_schreiben(pfad, &datei_inhalt(&aus, json), was, englisch) {
            Ok(()) => {
                if io::stderr().is_terminal() {
                    melden(&if englisch {
                        format!(
                            "Output written, UTF-8 with BOM.\n{}",
                            datei_angabe(pfad, true)
                        )
                    } else {
                        format!(
                            "Ausgabe geschrieben, UTF-8 mit BOM.\n{}",
                            datei_angabe(pfad, false)
                        )
                    });
                }
                ExitCode::from(code)
            }
            Err(m) => fehler(&m),
        };
    }
    let (code, meldung) = nach_dem_schreiben(ausgeben(&aus, json), code, englisch);
    if let Some(m) = meldung {
        melden(&m);
    }
    ExitCode::from(code)
}

// ---------------------------------------------------------------------------------------
// Hilfe
// ---------------------------------------------------------------------------------------

/// Hilfe zu -t. Die geschützten Leerzeichen halten «echo … |» beim Umbruch zusammen.
const HILFE_T_DE: &str = "Text direkt prüfen. Er steht dann in der Befehlszeile und kann in den Shell-Verlauf und in Prozessprotokolle gelangen, ebenso bei «echo\u{A0}…\u{A0}|». Vertrauliches als Datei oder durch Einfügen prüfen.";
const HILFE_T_EN: &str = "Check text directly. It then appears in the command line and can end up in the shell history and process logs, as with «echo\u{A0}…\u{A0}|». Check confidential text as a file or by pasting.";

/// Die Hilfsmittel-Zeile am Ende der Hilfe, mit festem Umbruch.
const VORBEHALT_DE: &str = "Hilfsmittel, keine Rechtsberatung · erkennt Identifikatoren und\nWörter aus Listen, nicht den Zusammenhang · Güte noch nicht gemessen";
const VORBEHALT_EN: &str = "An aid, not legal advice · detects identifiers and listed words,\nnot the context · quality not yet measured";

/// Text zu `--target` in der Hilfe. Ohne den letzten Halbsatz («in der Konfigurationsdatei
/// gesetzt, gilt es für jeden Prompt»): Er gilt nur für ein `target` aus der Datei, und
/// promptscan übergeht `target` in der Datei. Danach Vorgabe und Geltung.
const TARGET_DE: &str = "Wohin der Prompt geht; local nur, wenn das Modell auf diesem Rechner läuft, ohne Synchronisierung in eine Cloud. Vorgabe cloud; gilt je Aufruf.";
const TARGET_EN: &str = "Where the prompt goes; local only if the model runs on this computer, without syncing to a cloud. Default cloud; applies per call.";

/// Zwei Spalten: `links` ab Spalte 2, `text` ab `spalte`, auf 80 umgebrochen. Ist `links`
/// zu breit, beginnt `text` auf der nächsten Zeile.
fn zweispaltig(aus: &mut String, links: &str, text: &str, spalte: usize) {
    let links = format!("  {links}");
    let zeilen = umbrechen(text, BREITE - spalte, BREITE - spalte);
    let mut erste = true;
    if links.chars().count() + 1 > spalte {
        let _ = writeln!(aus, "{links}");
        erste = false;
    }
    for z in zeilen {
        if erste {
            let _ = writeln!(aus, "{links:<spalte$}{z}");
            erste = false;
        } else {
            let _ = writeln!(aus, "{:spalte$}{z}", "");
        }
    }
}

/// Ein Absatz, um `einzug` Spalten eingerückt und auf 80 umgebrochen.
fn absatz(aus: &mut String, text: &str, einzug: usize) {
    for z in umbrechen(text, BREITE - einzug, BREITE - einzug) {
        let _ = writeln!(aus, "{:einzug$}{z}", "");
    }
}

/// Die Hilfe: Einfügen, Datei, -t mit dem Hinweis zum Shell-Verlauf, Befehle, Stufen,
/// Optionen, Konfiguration, Exit-Codes und die Hilfsmittel-Zeile; höchstens 80 Spalten.
fn hilfe_text(englisch: bool) -> String {
    const S: usize = 28;
    let windows = cfg!(windows);
    let mut aus = String::new();
    let rollen = rollenliste();
    if englisch {
        absatz(
            &mut aus,
            "promptscan checks a prompt before it goes to an AI for information that is sensitive under Swiss law. It recognises German, French and Italian. promptscan is a terminal program.",
            0,
        );
        aus.push_str("\nUsage\n");
        zweispaltig(
            &mut aus,
            "promptscan",
            if windows {
                "paste the prompt, press Enter, then Ctrl+Z and Enter"
            } else {
                "paste the prompt, press Enter, then Ctrl+D"
            },
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan FILE",
            "check a file, UTF-8 or UTF-16 with BOM",
            S,
        );
        zweispaltig(&mut aus, "promptscan -t \"TEXT\"", HILFE_T_EN, S);
        zweispaltig(
            &mut aus,
            "promptscan rules",
            "all rules with their legal basis",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan explain RULE",
            "provision, method, sources and limits of a rule or level, e.g. ahv or red",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan licenses",
            "licence status and third-party notices",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan config",
            "show the configuration file; set the role or language permanently, e.g. promptscan\u{A0}config\u{A0}--role\u{A0}lawyer",
            S,
        );
        absatz(
            &mut aus,
            "Options go before -t and FILE; with a command, before or after it. A file named like a command: promptscan\u{A0}./rules",
            2,
        );
        aus.push_str("\nLevels\n");
        zweispaltig(
            &mut aus,
            "RED",
            "duty of confidentiality backed by criminal law; only with --role or inferred from the text",
            12,
        );
        zweispaltig(
            &mut aus,
            "ORANGE",
            "sensitive personal data, Art.\u{A0}5\u{A0}lit.\u{A0}c\u{A0}DSG",
            12,
        );
        zweispaltig(
            &mut aus,
            "YELLOW",
            "personal data, Art.\u{A0}5\u{A0}lit.\u{A0}a\u{A0}DSG",
            12,
        );
        zweispaltig(&mut aus, "NOTE", "legal consequence not assessed", 12);
        aus.push_str("\nOptions\n");
        zweispaltig(
            &mut aus,
            "--role ROLE",
            &format!(
                "professional role with a duty of confidentiality; without it, the text may show the role. {rollen}; German words such as anwalt also work. Permanently: {}. Duty and provisions: {}",
                nb("promptscan config --role lawyer"),
                nb("promptscan explain red")
            ),
            S,
        );
        zweispaltig(&mut aus, "--target cloud|local", TARGET_EN, S);
        zweispaltig(
            &mut aus,
            "--lang de|en",
            "language of the output; default de",
            S,
        );
        zweispaltig(
            &mut aus,
            "--format text|json",
            "json for programs, in plain ASCII",
            S,
        );
        zweispaltig(
            &mut aus,
            "--output FILE",
            "write the output to FILE, UTF-8 with BOM, without colour. In Windows PowerShell 5.1, redirecting with\u{A0}> garbles umlauts.",
            S,
        );
        zweispaltig(
            &mut aus,
            "--quiet",
            "only the result, without prompt and findings",
            S,
        );
        zweispaltig(
            &mut aus,
            "--fail-on LEVEL",
            "exit code 1 from red (default), orange or yellow; never: always 0",
            S,
        );
        zweispaltig(&mut aus, "--no-color", "no colour; also with NO_COLOR", S);
        zweispaltig(&mut aus, "--ascii", "ASCII characters only", S);
        zweispaltig(
            &mut aus,
            "-h, --help",
            "this help; in German: --help --lang de",
            S,
        );
        zweispaltig(&mut aus, "-V, --version", "version", S);
        aus.push('\n');
        absatz(
            &mut aus,
            &format!(
                "Configuration: {}; PROMPTSCAN_CONFIG takes precedence. Keys role and lang.",
                if windows {
                    "%APPDATA%\\promptscan\\config.toml"
                } else {
                    "~/.config/promptscan/config.toml"
                }
            ),
            0,
        );
        absatz(
            &mut aus,
            "Exit code: 0 nothing at the threshold · 1 finding at the threshold · 2\u{A0}error",
            0,
        );
        aus.push('\n');
        aus.push_str(&ascii_wenn_verlangt(VORBEHALT_EN));
    } else {
        absatz(
            &mut aus,
            "promptscan prüft einen Prompt, bevor er an eine KI geht, auf Angaben, die nach Schweizer Recht heikel sind. Erkannt werden Deutsch, Französisch und Italienisch. promptscan ist ein Terminalprogramm.",
            0,
        );
        aus.push_str("\nAufruf\n");
        zweispaltig(
            &mut aus,
            "promptscan",
            if windows {
                "Prompt einfügen, Enter, dann Ctrl+Z und Enter"
            } else {
                "Prompt einfügen, Enter, dann Ctrl+D"
            },
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan DATEI",
            "Datei prüfen, UTF-8 oder UTF-16 mit BOM",
            S,
        );
        zweispaltig(&mut aus, "promptscan -t \"TEXT\"", HILFE_T_DE, S);
        zweispaltig(
            &mut aus,
            "promptscan rules",
            "alle Regeln mit Rechtsgrundlage",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan explain REGEL",
            "Norm, Methode, Quellen und Grenzen einer Regel oder Stufe, z.\u{A0}B. ahv oder rot",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan licenses",
            "Lizenzstand und Vermerke zu fremden Bestandteilen",
            S,
        );
        zweispaltig(
            &mut aus,
            "promptscan config",
            "Konfigurationsdatei zeigen; Rolle oder Sprache dauerhaft setzen, z.\u{A0}B. promptscan\u{A0}config\u{A0}--role\u{A0}lawyer",
            S,
        );
        absatz(
            &mut aus,
            "Optionen stehen vor -t und DATEI, bei einem Befehl davor oder danach. Eine Datei, die wie ein Befehl heisst: promptscan\u{A0}./rules",
            2,
        );
        aus.push_str("\nStufen\n");
        zweispaltig(
            &mut aus,
            "ROT",
            "strafbewehrte Geheimhaltungspflicht; nur mit --role oder aus dem Text erschlossen",
            12,
        );
        zweispaltig(
            &mut aus,
            "ORANGE",
            "besonders schützenswerte Personendaten, Art.\u{A0}5\u{A0}lit.\u{A0}c\u{A0}DSG",
            12,
        );
        zweispaltig(
            &mut aus,
            "GELB",
            "Personendaten, Art.\u{A0}5\u{A0}lit.\u{A0}a\u{A0}DSG",
            12,
        );
        zweispaltig(&mut aus, "HINWEIS", "Rechtsfolge nicht beurteilt", 12);
        aus.push_str("\nOptionen\n");
        zweispaltig(
            &mut aus,
            "--role ROLLE",
            &format!(
                "Berufsrolle mit Geheimhaltungspflicht; ohne Angabe kann der Text die Rolle zeigen. {rollen}; auch deutsch, etwa anwalt oder ärztin. Dauerhaft: {}. Pflicht und Normen: {}",
                nb("promptscan config --role lawyer"),
                nb("promptscan explain rot")
            ),
            S,
        );
        zweispaltig(&mut aus, "--target cloud|local", TARGET_DE, S);
        zweispaltig(
            &mut aus,
            "--lang de|en",
            "Sprache der Ausgabe; Vorgabe de",
            S,
        );
        zweispaltig(
            &mut aus,
            "--format text|json",
            "json für Programme, in reinem ASCII",
            S,
        );
        zweispaltig(
            &mut aus,
            "--output DATEI",
            "Ausgabe in DATEI schreiben, UTF-8 mit BOM, ohne Farbe. In Windows PowerShell 5.1 verstümmelt die Umleitung mit\u{A0}> die Umlaute.",
            S,
        );
        zweispaltig(
            &mut aus,
            "--quiet",
            "nur das Ergebnis, ohne Prompt und Funde",
            S,
        );
        zweispaltig(
            &mut aus,
            "--fail-on STUFE",
            "Exit-Code 1 ab red (Vorgabe), orange oder yellow; never: immer 0",
            S,
        );
        zweispaltig(&mut aus, "--no-color", "ohne Farbe; auch mit NO_COLOR", S);
        zweispaltig(&mut aus, "--ascii", "nur ASCII-Zeichen", S);
        zweispaltig(
            &mut aus,
            "-h, --help",
            "diese Hilfe; englisch: --help --lang en",
            S,
        );
        zweispaltig(&mut aus, "-V, --version", "Version", S);
        aus.push('\n');
        absatz(
            &mut aus,
            &format!(
                "Konfiguration: {}; PROMPTSCAN_CONFIG geht vor. Schlüssel role und lang.",
                if windows {
                    "%APPDATA%\\promptscan\\config.toml"
                } else {
                    "~/.config/promptscan/config.toml"
                }
            ),
            0,
        );
        absatz(
            &mut aus,
            "Exit-Code: 0 nichts ab der Schwelle · 1 Fund ab der Schwelle · 2\u{A0}Fehler",
            0,
        );
        aus.push('\n');
        aus.push_str(&ascii_wenn_verlangt(VORBEHALT_DE));
    }
    aus.push('\n');
    aus.replace('\u{A0}', " ")
}

/// Die Hilfe auf stdout oder mit `--output` in die Datei, Exit-Code 0, auch wenn der
/// Empfänger die Pipe schliesst.
fn hilfe_ausgeben(englisch: bool, datei: Option<&Path>) -> ExitCode {
    abschliessen(&hilfe_text(englisch), false, 0, englisch, datei)
}

// ---------------------------------------------------------------------------------------
// Fehler der Befehlszeile (clap)
// ---------------------------------------------------------------------------------------

fn kontext(e: &clap::Error, art: ContextKind) -> Option<String> {
    match e.get(art)? {
        ContextValue::String(s) => Some(s.clone()),
        ContextValue::Strings(v) => Some(v.join(", ")),
        ContextValue::StyledStr(s) => Some(s.to_string()),
        ContextValue::StyledStrs(v) => Some(
            v.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ContextValue::Number(n) => Some(n.to_string()),
        ContextValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Der nächste Schritt, wenn ein Befehl ein Argument zu viel bekommt. Die Befehle
/// halten beim Umbruch zusammen (U+00A0).
fn befehl_schritt(befehl: &str, englisch: bool) -> &'static str {
    match (befehl, englisch) {
        ("explain", false) => {
            "Je Aufruf eine Regel oder Stufe, etwa: promptscan\u{A0}explain\u{A0}ahv"
        }
        ("explain", true) => {
            "One rule or level per call, such as: promptscan\u{A0}explain\u{A0}ahv"
        }
        ("config", false) => {
            "Rolle oder Sprache speichern: promptscan\u{A0}config\u{A0}--role\u{A0}ROLLE oder promptscan\u{A0}config\u{A0}--lang\u{A0}en"
        }
        ("config", true) => {
            "Save the role or language: promptscan\u{A0}config\u{A0}--role\u{A0}ROLE or promptscan\u{A0}config\u{A0}--lang\u{A0}en"
        }
        ("rules", false) => {
            "Alle Regeln: promptscan\u{A0}rules; eine Regel im Einzelnen: promptscan\u{A0}explain\u{A0}REGEL"
        }
        ("rules", true) => {
            "All rules: promptscan\u{A0}rules; one rule in detail: promptscan\u{A0}explain\u{A0}RULE"
        }
        (_, false) => {
            "Hilfe: promptscan\u{A0}--help; eine Regel im Einzelnen: promptscan\u{A0}explain\u{A0}REGEL"
        }
        (_, true) => {
            "Help: promptscan\u{A0}--help; one rule in detail: promptscan\u{A0}explain\u{A0}RULE"
        }
    }
}

/// Deutsche und andere naheliegende Wörter für die Werte der Optionen: «--fail-on rot»
/// meint «--fail-on red».
const WERTWOERTER: [(&str, &str, &str); 8] = [
    ("--fail-on", "rot", "red"),
    ("--fail-on", "gelb", "yellow"),
    ("--fail-on", "nie", "never"),
    ("--lang", "deutsch", "de"),
    ("--lang", "german", "de"),
    ("--lang", "englisch", "en"),
    ("--lang", "english", "en"),
    ("--target", "lokal", "local"),
];

/// Die Meldung zu einem Fehler von clap, deutsch oder englisch, zweizeilig. Ein Vorschlag
/// steht auf einer eigenen Zeile.
fn clap_fehler_text(e: &clap::Error, englisch: bool, vor: &Vorpruefung) -> String {
    let arg = kontext(e, ContextKind::InvalidArg).unwrap_or_default();
    // «--format <FORMAT>» → «--format»
    let option = arg.split_whitespace().next().unwrap_or("").to_string();
    let wert = kontext(e, ContextKind::InvalidValue).unwrap_or_default();
    // --role nimmt jeden Text an (geprüft wird danach); clap kennt dann keine Liste.
    let gueltig = kontext(e, ContextKind::ValidValue)
        .filter(|g| !g.is_empty())
        .or_else(|| (option == "--role").then(rollenliste));
    let hilfe = if englisch {
        "All options: promptscan\u{A0}--help"
    } else {
        "Alle Optionen: promptscan\u{A0}--help"
    };
    let gemeint = if englisch {
        "Did you mean:"
    } else {
        "Gemeint ist wohl:"
    };
    match e.kind() {
        ErrorKind::UnknownArgument if arg.starts_with('-') => {
            // clap schlägt nur bei gleicher Schreibweise vor; «--ROLE» meint «--role».
            let name = arg.split('=').next().unwrap_or(&arg);
            let vorschlag = kontext(e, ContextKind::SuggestedArg)
                .or_else(|| vorschlag(name, &OPTIONEN.map(|o| (o, o))).map(str::to_string));
            let mut text = if englisch {
                format!("There is no option «{arg}».")
            } else {
                format!("Die Option «{arg}» gibt es nicht.")
            };
            if let Some(v) = vorschlag {
                let _ = write!(text, "\n{gemeint} {v}");
            }
            format!("{text}\n{hilfe}")
        }
        ErrorKind::UnknownArgument => match (&vor.befehl, englisch) {
            (Some(b), false) => format!(
                "Der Befehl {b} nimmt «{arg}» nicht an.\n{}",
                befehl_schritt(b, false)
            ),
            (Some(b), true) => format!(
                "The command {b} does not take «{arg}».\n{}",
                befehl_schritt(b, true)
            ),
            (None, false) => format!(
                "Das Argument «{arg}» ist zu viel.\npromptscan prüft eine Datei oder einen Text: Text mit Leerzeichen in Anführungszeichen setzen oder den Prompt als Datei übergeben."
            ),
            (None, true) => format!(
                "The argument «{arg}» is one too many.\npromptscan checks one file or one text: put text with spaces in quotes, or pass the prompt as a file."
            ),
        },
        ErrorKind::InvalidValue if wert.is_empty() && option == "--text" => {
            if englisch {
                "-t needs the text.\nFor example: promptscan -t \"…\"; check confidential text as a file or by pasting.".to_string()
            } else {
                "Nach -t fehlt der Text.\nEtwa: promptscan -t \"…\"; Vertrauliches als Datei oder durch Einfügen prüfen.".to_string()
            }
        }
        ErrorKind::InvalidValue if wert.is_empty() && option == "--output" => {
            if englisch {
                "--output needs the name of a file.\nFor example: --output report.txt".to_string()
            } else {
                "Nach --output fehlt der Name der Datei.\nEtwa: --output bericht.txt".to_string()
            }
        }
        // Ein Wert für einen Schalter, etwa «--no-color=x».
        ErrorKind::TooManyValues => {
            if englisch {
                format!("The option {option} takes no value («{wert}»).\nGive it alone: {option}")
            } else {
                format!(
                    "Die Option {option} nimmt keinen Wert an («{wert}»).\nNur die Option angeben: {option}"
                )
            }
        }
        ErrorKind::InvalidValue if wert.is_empty() => match (gueltig, englisch) {
            (Some(g), false) => format!("Nach {option} fehlt der Wert.\nMöglich: {g}"),
            (Some(g), true) => format!("{option} needs a value.\nPossible: {g}"),
            (None, false) => format!("Nach {option} fehlt der Wert.\n{hilfe}"),
            (None, true) => format!("{option} needs a value.\n{hilfe}"),
        },
        ErrorKind::InvalidValue => {
            let klein = wert.trim().to_lowercase();
            let vorschlag = kontext(e, ContextKind::SuggestedValue).or_else(|| {
                WERTWOERTER
                    .iter()
                    .find(|(o, w, _)| *o == option && *w == klein)
                    .map(|(_, _, v)| (*v).to_string())
            });
            let mut text = if englisch {
                format!("«{wert}» is not a valid value for {option}.")
            } else {
                format!("«{wert}» ist kein gültiger Wert für {option}.")
            };
            if let Some(v) = vorschlag {
                let _ = write!(text, "\n{gemeint} {}", nb(&format!("{option} {v}")));
            }
            match (gueltig, englisch) {
                (Some(g), false) => format!("{text}\nMöglich: {g}"),
                (Some(g), true) => format!("{text}\nPossible: {g}"),
                (None, _) => format!("{text}\n{hilfe}"),
            }
        }
        ErrorKind::ArgumentConflict => {
            let vorher = kontext(e, ContextKind::PriorArg).unwrap_or_default();
            let vorher_option = vorher.split_whitespace().next().unwrap_or("");
            match (vorher_option == option, englisch) {
                (true, false) => format!(
                    "Die Option {option} steht mehr als einmal da.\nJede Option nur einmal angeben."
                ),
                (true, true) => {
                    format!("The option {option} appears more than once.\nGive each option once.")
                }
                (false, false) => {
                    format!("{option} und {vorher_option} gehen nicht zusammen.\n{hilfe}")
                }
                (false, true) => {
                    format!("{option} and {vorher_option} do not go together.\n{hilfe}")
                }
            }
        }
        ErrorKind::InvalidUtf8 => {
            if englisch {
                "The command line contains characters that are not valid Unicode.\nPass the prompt as a file.".to_string()
            } else {
                "Die Befehlszeile enthält Zeichen, die kein gültiges Unicode sind.\nDen Prompt als Datei übergeben.".to_string()
            }
        }
        _ => {
            let roh = e.to_string();
            let erste = roh
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches("error: ")
                .to_string();
            if englisch {
                format!("The command line is not valid: {erste}\n{hilfe}")
            } else {
                format!("Die Befehlszeile ist fehlerhaft: {erste}\n{hilfe}")
            }
        }
    }
}

/// Antwort auf einen Fehler von clap: Version und Hilfe ausgeben, sonst melden (Exit 2).
fn clap_fehler(e: &clap::Error, englisch: bool, vor: &Vorpruefung) -> ExitCode {
    match e.kind() {
        ErrorKind::DisplayVersion => {
            let _ = e.print();
            return ExitCode::SUCCESS;
        }
        ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            return hilfe_ausgeben(englisch, vor.ausgabe.as_deref());
        }
        _ => {}
    }
    let mut text = clap_fehler_text(e, englisch, vor);
    // Steht -t da, hat die Shell den Text vielleicht an geraden Anführungszeichen zerlegt.
    // Fehlt nur der Wert von -t, sagt die Meldung schon alles.
    let ohne_text = e.kind() == ErrorKind::InvalidValue
        && kontext(e, ContextKind::InvalidValue).is_none_or(|w| w.is_empty())
        && kontext(e, ContextKind::InvalidArg).is_some_and(|a| a.starts_with("--text"));
    if vor.mit_text && !ohne_text {
        text.push('\n');
        text.push_str(anfuehrungszeichen_hinweis(englisch));
    }
    fehler(&text)
}

// ---------------------------------------------------------------------------------------
// Ablauf
// ---------------------------------------------------------------------------------------

/// Rolle aus `--role`: Kennung oder deutsches Eingabewort.
fn rolle_aus_option(cli: &Cli, englisch: bool) -> Result<Option<Rolle>, String> {
    match &cli.role {
        None => Ok(None),
        Some(w) => Rolle::aus_eingabe(w)
            .map(Some)
            .ok_or_else(|| rolle_unbekannt(w, englisch, false)),
    }
}

/// `explain` ohne passende Regel oder Stufe.
/// Die Meldung nennt einen Vorschlag, wenn es einen gibt, und immer alle Kennungen.
fn explain_unbekannt(regel: &str, englisch: bool) -> String {
    let regeln = katalog::alle();
    let mut kandidaten: Vec<(&str, &str)> = regeln.iter().map(|r| (r.id, r.id)).collect();
    for s in ["gelb", "orange", "rot", "hinweis", "yellow", "red", "note"] {
        kandidaten.push((s, s));
    }
    let liste = regeln.iter().map(|r| r.id).collect::<Vec<_>>().join(", ");
    let mut text = if englisch {
        format!("There is no rule or level «{regel}».")
    } else {
        format!("Eine Regel oder Stufe «{regel}» gibt es nicht.")
    };
    if let Some(v) = vorschlag(regel, &kandidaten) {
        let _ = write!(
            text,
            "\n{} {}",
            if englisch {
                "Did you mean:"
            } else {
                "Gemeint ist wohl:"
            },
            nb(&format!("promptscan explain {v}"))
        );
    }
    if englisch {
        format!("{text}\nLevels: yellow, orange, red, note. Rules: {liste}")
    } else {
        format!("{text}\nStufen: gelb, orange, rot, hinweis. Regeln: {liste}")
    }
}

/// Führt einen Aufruf aus; gibt den Exit-Code und die Sprache der Meldungen zurück.
fn lauf(args: &[OsString]) -> (ExitCode, bool) {
    let vor = vorpruefen(args);
    NUR_ASCII.with(|a| a.set(ascii_gewuenscht(vor.ascii, vor.ausgabe.is_some())));
    let konfig = Konfig::lesen();
    let englisch = vor.englisch.or(konfig.englisch()).unwrap_or(false);
    if vor.hilfe {
        return (hilfe_ausgeben(englisch, vor.ausgabe.as_deref()), englisch);
    }
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => return (clap_fehler(&e, englisch, &vor), englisch),
    };
    NUR_ASCII.with(|a| a.set(ascii_gewuenscht(cli.ascii, cli.output.is_some())));
    let englisch = cli.lang.map_or(englisch, |l| l == SpracheArg::En);
    (befehl_ausfuehren(&cli, &vor, &konfig, englisch), englisch)
}

fn befehl_ausfuehren(cli: &Cli, vor: &Vorpruefung, konfig: &Konfig, englisch: bool) -> ExitCode {
    if cli.hilfe || matches!(cli.befehl, Some(Befehl::Help)) {
        return hilfe_ausgeben(englisch, cli.output.as_deref());
    }
    if let Some(text) = eingabe_konflikt(cli, englisch) {
        return fehler(&text);
    }
    let cli_rolle = match rolle_aus_option(cli, englisch) {
        Ok(r) => r,
        Err(m) => return fehler(&m),
    };
    let json = cli.format == FormatArg::Json;
    let prueft = cli.befehl.is_none();
    if let Some(Befehl::Config) = &cli.befehl {
        return befehl_config(konfig, cli, vor, cli_rolle, englisch);
    }
    let angewandt = konfiguration_anwenden(konfig, cli_rolle, cli.lang, prueft, englisch);
    for w in &angewandt.warnungen {
        melden(w);
    }
    if !angewandt.fehler.is_empty() {
        for f in &angewandt.fehler {
            melden(f);
        }
        return ExitCode::from(2);
    }
    let darstellung = darstellung(cli, englisch);
    let datei = cli.output.as_deref();

    // rules und explain: eine Zeile Abschluss, Exit-Code 0, auch wenn der Empfänger die
    // Pipe vorher schliesst.
    let dokument = |aus: String| {
        let aus = format!("{}\n", aus.trim_end());
        abschliessen(&aus, json, 0, englisch, datei)
    };
    match &cli.befehl {
        Some(Befehl::Rules) => {
            return dokument(if json {
                ausgabe::regeln_json()
            } else {
                ausgabe::regeln_text(&darstellung)
            });
        }
        Some(Befehl::Explain { regel }) => {
            let Some(regel) = regel else {
                return fehler(if englisch {
                    "explain needs a rule or level.\nFor example: promptscan\u{A0}explain\u{A0}ahv or promptscan\u{A0}explain\u{A0}red; all rules: promptscan\u{A0}rules"
                } else {
                    "explain braucht eine Regel oder Stufe.\nEtwa: promptscan\u{A0}explain\u{A0}ahv oder promptscan\u{A0}explain\u{A0}rot; alle Regeln: promptscan\u{A0}rules"
                });
            };
            if let Some(stufe) = ausgabe::stufe_aus_name(regel) {
                return dokument(if json {
                    ausgabe::stufen_erklaerung_json(stufe)
                } else {
                    ausgabe::stufen_erklaerung_text(stufe, &darstellung)
                });
            }
            return match katalog::finde(regel) {
                Some(info) => dokument(if json {
                    ausgabe::erklaerung_json(&info)
                } else {
                    ausgabe::erklaerung_text(&info, &darstellung)
                }),
                None => fehler(&explain_unbekannt(regel, englisch)),
            };
        }
        Some(Befehl::Licenses) => {
            let aus = format!("{}\n", promptscan::lizenzen::text(englisch).trim_end());
            return abschliessen(&aus, false, 0, englisch, datei);
        }
        Some(Befehl::Config | Befehl::Help) | None => {}
    }

    // Prüfen.
    if let Some(rest) = cli.text.as_deref().and_then(verschluckte_optionen) {
        return fehler(&verschluckte_optionen_text(rest, englisch));
    }
    let roh = match eingabe_lesen(cli, englisch) {
        Ok(t) => t,
        Err(e) => return fehler(&e),
    };
    let ziel = match cli.target {
        Some(ZielArg::Local) => Ziel::Local,
        Some(ZielArg::Cloud) | None => Ziel::Cloud,
    };
    let mut bericht = pruefe(
        &roh,
        &Optionen {
            rolle: angewandt.rolle,
            ziel,
        },
    );
    bericht.rolle_quelle = angewandt.quelle;
    // Deutet die Eingabe auf eine beschädigte Kodierung, warnt promptscan auch auf
    // stderr: Ein Skript, das nur den Exit-Code auswertet, sieht die Meldung im Bericht
    // nicht. Der Exit-Code bleibt der der Funde.
    for zeile in ausgabe::Eingabe::von(&bericht.text).kodierung_meldungen(englisch) {
        melden(&zeile);
    }

    let schwelle = match cli.fail_on {
        SchwelleArg::Red => Some(Stufe::Rot),
        SchwelleArg::Orange => Some(Stufe::Orange),
        SchwelleArg::Yellow => Some(Stufe::Gelb),
        SchwelleArg::Never => None,
    };
    let code = match (schwelle, bericht.hoechste()) {
        (Some(s), Some(h)) if h >= s => 1,
        _ => 0,
    };
    let aus = match (json, cli.quiet) {
        (true, false) => format!("{}\n", ausgabe::bericht_json(&bericht)),
        (true, true) => format!("{}\n", ausgabe::bericht_kurz_json(&bericht)),
        (false, false) => ausgabe::bericht_text(&bericht, &darstellung),
        (false, true) => ausgabe::bericht_kurz_text(&bericht, &darstellung),
    };
    abschliessen(&aus, json, code, englisch, datei)
}

fn main() -> ExitCode {
    // Beim Start bestimmt, nicht am Ende: Bis dahin kann ein anderer Prozess an der
    // Konsole hängen, und die Frage ist, wie promptscan gestartet wurde.
    let warten = am_ende_warten(
        cfg!(windows),
        konsole::prozesse(),
        konsole::ein_und_ausgabe(),
    );
    let args: Vec<OsString> = std::env::args_os().collect();
    let (code, englisch) = lauf(&args);
    if warten {
        auf_enter_warten(englisch);
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str, le: bool, bom: bool) -> Vec<u8> {
        let mut bytes = match (bom, le) {
            (false, _) => Vec::new(),
            (true, true) => vec![0xFF, 0xFE],
            (true, false) => vec![0xFE, 0xFF],
        };
        for e in text.encode_utf16() {
            bytes.extend(if le { e.to_le_bytes() } else { e.to_be_bytes() });
        }
        bytes
    }

    #[test]
    fn gueltige_kodierungen() {
        let text = "Anwältin 𝄞\r\nZeile";
        let mut mit_bom = vec![0xEF, 0xBB, 0xBF];
        mit_bom.extend(text.as_bytes());
        for bytes in [
            text.as_bytes().to_vec(),
            mit_bom,
            utf16(text, true, true),
            utf16(text, false, true),
        ] {
            assert_eq!(dekodieren(&bytes).as_deref(), Ok(text));
        }
        assert_eq!(dekodieren(b"").as_deref(), Ok(""));
        assert_eq!(dekodieren(&[0xFF, 0xFE]).as_deref(), Ok(""));
    }

    #[test]
    fn abgewiesene_kodierungen() {
        let ascii = "Hans Muster, AHV 756.1234.5678.97";
        let umlaute = "Anwältin";
        let utf32 = |le: bool| {
            let mut b = if le {
                vec![0xFF, 0xFE, 0, 0]
            } else {
                vec![0, 0, 0xFE, 0xFF]
            };
            for c in ascii.chars() {
                b.extend(if le {
                    u32::from(c).to_le_bytes()
                } else {
                    u32::from(c).to_be_bytes()
                });
            }
            b
        };
        let faelle: [(Vec<u8>, Lesefehler); 10] = [
            (utf32(true), Lesefehler::Utf32),
            (utf32(false), Lesefehler::Utf32),
            (
                utf16(ascii, true, false),
                Lesefehler::Nul { ohne_bom: true },
            ),
            (
                utf16(ascii, false, false),
                Lesefehler::Nul { ohne_bom: true },
            ),
            (
                utf16(umlaute, true, false),
                Lesefehler::Nul { ohne_bom: true },
            ),
            (b"Hans\0Muster".to_vec(), Lesefehler::Nul { ohne_bom: true }),
            (
                utf16("Hans\0", true, true),
                Lesefehler::Nul { ohne_bom: false },
            ),
            (
                b"\xEF\xBB\xBFHans\0".to_vec(),
                Lesefehler::Nul { ohne_bom: false },
            ),
            (b"K\xFCrzer".to_vec(), Lesefehler::Unbekannt),
            (vec![0xFE, 0xFF, 0x00], Lesefehler::Utf16Ungerade),
        ];
        for (bytes, fehler) in faelle {
            assert_eq!(dekodieren(&bytes), Err(fehler), "{bytes:02X?}");
            assert!(!fehler.meldung(false).is_empty() && !fehler.meldung(true).is_empty());
        }
        assert_eq!(dekodieren(b"\xEF\xBB\xBFK\xFCrzer"), Err(Lesefehler::Utf8));
        // Eine einzelne Hälfte eines Surrogatpaars (U+D800) ist ungültiges UTF-16.
        assert_eq!(
            dekodieren(&[0xFF, 0xFE, 0x00, 0xD8]),
            Err(Lesefehler::Utf16)
        );
    }

    /// Ein Ziel, das jedes Schreiben mit einem Fehler beantwortet.
    struct Kaputt(io::ErrorKind);

    impl Write for Kaputt {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn geschlossene_pipe_behaelt_den_code() {
        for code in [0, 1] {
            let ergebnis = schreiben(&mut Kaputt(io::ErrorKind::BrokenPipe), "Bericht");
            assert_eq!(nach_dem_schreiben(ergebnis, code, false), (code, None));
        }
    }

    /// Ein Schreibfehler auf stdout gibt 2 und eine zweizeilige Meldung mit eigenem Grund;
    /// nur ein unbekannter Fehler bringt den Text des Systems, in Klammern.
    #[test]
    fn anderer_schreibfehler_gibt_2() {
        for (art, de, en) in [
            (
                io::ErrorKind::PermissionDenied,
                "Die Ausgabe lässt sich nicht schreiben: Zugriff verweigert.\n",
                "The output cannot be written: access denied.\n",
            ),
            (
                io::ErrorKind::StorageFull,
                "Die Ausgabe lässt sich nicht schreiben: Der Datenträger ist voll.\n",
                "The output cannot be written: the disk is full.\n",
            ),
            (
                io::ErrorKind::Other,
                "Die Ausgabe lässt sich nicht schreiben (other error).\n",
                "The output cannot be written (other error).\n",
            ),
        ] {
            for (englisch, anfang) in [(false, de), (true, en)] {
                for code in [0, 1] {
                    let ergebnis = schreiben(&mut Kaputt(art), "Bericht");
                    let (neu, meldung) = nach_dem_schreiben(ergebnis, code, englisch);
                    assert_eq!(neu, 2);
                    let m = meldung_formatieren(&meldung.expect("Meldung"));
                    assert!(m.starts_with(&format!("promptscan: {anfang}")), "{m}");
                    assert!(m.contains("--output"), "{m}");
                    assert_eq!(m.lines().count(), 2, "{m}");
                }
            }
        }
        let mut puffer = Vec::new();
        assert!(schreiben(&mut puffer, "Bericht\n").is_ok());
        assert_eq!(puffer, b"Bericht\n");
        assert_eq!(nach_dem_schreiben(Ok(()), 1, false), (1, None));
    }

    /// Fehler beim Schreiben einer Datei: eigene Gründe, ein Ordner als Ziel, der Pfad am
    /// Ende der ersten Zeile.
    #[test]
    fn schreibfehler_in_eine_datei() {
        let (grund, schritt) =
            schreibfehler_grund(Some(&io::Error::from(io::ErrorKind::NotFound)), false);
        assert_eq!(grund, ": Den Ordner gibt es nicht");
        assert!(schritt.contains("keinen Ordner an"), "{schritt}");
        let (grund, _) = schreibfehler_grund(None, true);
        assert_eq!(grund, ": the path is a folder");
        if cfg!(windows) {
            // ERROR_SHARING_VIOLATION: Die Datei ist in Word oder Excel offen.
            let (grund, schritt) =
                schreibfehler_grund(Some(&io::Error::from_raw_os_error(32)), false);
            assert_eq!(grund, ": Die Datei ist in einem anderen Programm geöffnet");
            assert!(schritt.contains("Excel"), "{schritt}");
        }
        let ordner = std::env::temp_dir();
        let m = datei_schreiben(&ordner, b"x", "Die Ausgabe", false).expect_err("Ordner");
        // Zwischen «Datei:» und dem Pfad steht ein geschütztes Leerzeichen: der Umbruch
        // trennt die Angabe nicht vom Pfad.
        assert!(
            m.starts_with(
                "Die Ausgabe lässt sich nicht schreiben: Der Pfad ist ein Ordner. Datei:\u{A0}"
            ),
            "{m}"
        );
        let fehlt = ordner.join("promptscan-gibt-es-nicht-7f3a").join("x.txt");
        let m = datei_schreiben(&fehlt, b"x", "The output", true).expect_err("fehlt");
        assert!(
            m.starts_with("The output cannot be written: the folder does not exist. File:\u{A0}"),
            "{m}"
        );
        assert!(!m.contains("os error"), "{m}");
    }

    /// `config` legt fehlende Ordner an: Fehlt dabei etwas, ist es das Laufwerk, nicht der
    /// Ordner (vorher hiess es widersprüchlich «Den Ordner gibt es nicht … promptscan legt
    /// keinen Ordner an»).
    #[test]
    fn ordner_der_konfiguration_nicht_anlegbar() {
        let (grund, schritt) =
            ordner_anlegen_grund(&io::Error::from(io::ErrorKind::NotFound), false);
        assert_eq!(grund, ": Das Laufwerk ist nicht erreichbar");
        assert!(schritt.contains("Netzlaufwerk"), "{schritt}");
        let (grund, _) = ordner_anlegen_grund(&io::Error::from(io::ErrorKind::NotFound), true);
        assert_eq!(grund, ": the drive is not available");
        let (grund, _) =
            ordner_anlegen_grund(&io::Error::from(io::ErrorKind::PermissionDenied), false);
        assert_eq!(grund, ": Zugriff verweigert");
        // Von Anfang bis Ende mit einem Laufwerk, das es auf diesem Rechner nicht gibt.
        if cfg!(windows) {
            let laufwerk = ('M'..='Z')
                .rev()
                .map(|b| format!("{b}:\\"))
                .find(|w| !Path::new(w).exists());
            if let Some(w) = laufwerk {
                let pfad = PathBuf::from(format!("{w}promptscan-test\\config.toml"));
                let m = konfiguration_schreiben(&pfad, Some(Rolle::Lawyer), None, false)
                    .expect_err("Laufwerk fehlt");
                assert!(
                    m.starts_with(
                        "Konfiguration: Der Ordner lässt sich nicht anlegen: Das Laufwerk ist nicht erreichbar."
                    ),
                    "{m}"
                );
                assert!(!m.contains("legt keinen Ordner an"), "{m}");
            }
        }
    }

    fn args(a: &[&str]) -> Vec<OsString> {
        std::iter::once("promptscan")
            .chain(a.iter().copied())
            .map(OsString::from)
            .collect()
    }

    /// Sprache, Hilfe und -t werden vor clap erkannt; Werte von Optionen zählen nicht.
    #[test]
    fn vorpruefung() {
        let v = vorpruefen(&args(&["--lang", "en", "--help"]));
        assert_eq!(v.englisch, Some(true));
        assert!(v.hilfe && !v.mit_text);
        assert_eq!(
            vorpruefen(&args(&["--lang=DE", "-h"])).englisch,
            Some(false)
        );
        assert_eq!(vorpruefen(&args(&["--lang", "fr"])).englisch, None);
        // -t nimmt «--help» als Text; nach «--» ist alles Datei.
        let v = vorpruefen(&args(&["-t", "--help", "--lang", "en"]));
        assert!(!v.hilfe && v.mit_text);
        assert_eq!(v.englisch, Some(true));
        assert!(!vorpruefen(&args(&["--", "--help"])).hilfe);
        assert!(vorpruefen(&args(&["--text=x"])).mit_text);
        assert!(vorpruefen(&args(&["-tx"])).mit_text);
        // Der erste freie Wert, wenn er ein Befehl ist.
        assert_eq!(
            vorpruefen(&args(&["--lang", "en", "rules"]))
                .befehl
                .as_deref(),
            Some("rules")
        );
        assert_eq!(vorpruefen(&args(&["--role", "rules"])).befehl, None);
        assert_eq!(vorpruefen(&args(&["datei.txt", "rules"])).befehl, None);
        // --ascii und --output (auch für die Hilfe); der Wert von -t zählt nicht.
        let v = vorpruefen(&args(&["--ascii", "--output", "h.txt", "-h"]));
        assert!(v.ascii && v.hilfe);
        assert_eq!(v.ausgabe, Some(PathBuf::from("h.txt")));
        assert_eq!(
            vorpruefen(&args(&["--output=x.txt"])).ausgabe,
            Some(PathBuf::from("x.txt"))
        );
        assert!(!vorpruefen(&args(&["-t", "--ascii"])).ascii);
        // --role und --lang nach «config» schreiben, davor nicht.
        let v = vorpruefen(&args(&["--lang", "en", "config", "--role", "lawyer"]));
        assert!(v.rolle_nach_befehl && !v.sprache_nach_befehl);
        let v = vorpruefen(&args(&["--role=lawyer", "config", "--lang=en"]));
        assert!(!v.rolle_nach_befehl && v.sprache_nach_befehl);
    }

    /// Auch die erste Zeile wird umgebrochen; ein langer Pfad steht ganz auf einer eigenen
    /// Zeile.
    #[test]
    fn erste_zeile_umgebrochen() {
        let pfad = Path::new(
            "C:\\Users\\Erika Beispiel\\AppData\\Roaming\\promptscan\\unterordner\\config.toml",
        );
        let m = meldung_formatieren(&konfig_meldung(
            "Die Sprache «fr» gibt es nicht.\nMöglich: lang = \"de\" oder lang = \"en\"",
            pfad,
            false,
        ));
        let zeilen: Vec<&str> = m.lines().collect();
        assert_eq!(
            zeilen[0],
            "promptscan: Konfiguration: Die Sprache «fr» gibt es nicht."
        );
        assert_eq!(zeilen[1], format!("  Datei: {}", pfad.display()));
        assert!(zeilen[2].starts_with("  Möglich:"), "{m}");
        // Ein kurzer Pfad bleibt in der ersten Zeile.
        let m = meldung_formatieren(&konfig_meldung(
            "Die Sprache «fr» gibt es nicht.\nMöglich: de, en",
            Path::new("x.toml"),
            false,
        ));
        assert!(
            m.starts_with(
                "promptscan: Konfiguration: Die Sprache «fr» gibt es nicht. Datei: x.toml\n"
            ),
            "{m}"
        );
        // Eine Warnung aus einem Satz: Auch sie wird umgebrochen, der Pfad steht darunter.
        let m = meldung_formatieren(&konfig_meldung(
            "Die Datei ist aus ihren Werten neu geschrieben; Kommentare sind weggefallen.",
            pfad,
            false,
        ));
        assert_eq!(
            m,
            format!(
                "promptscan: Konfiguration: Die Datei ist aus ihren Werten neu geschrieben;\n  Kommentare sind weggefallen.\n  Datei: {}\n",
                pfad.display()
            )
        );
        let m = meldung_formatieren(&rolle_unbekannt("anwält", false, false));
        assert_eq!(
            m,
            "promptscan: Die Rolle «anwält» gibt es nicht.\n  Gemeint ist wohl: --role lawyer\n  Alle Rollen mit Pflicht und Normen: promptscan explain rot\n"
        );
        let lang = format!("{}\nweiter", "wort ".repeat(40));
        for z in meldung_formatieren(&lang).lines() {
            assert!(z.chars().count() <= BREITE, "{z}");
        }
    }

    /// Umschrift in ASCII, lesbar und gleich breit.
    #[test]
    fn umschrift_in_ascii() {
        let text = "Zürich · «Müller» – Straße … Äpfel │ é ç œ 中 a\u{308} €\u{A0}5";
        let lesbar = nach_ascii(text, false);
        assert_eq!(
            lesbar,
            "Zuerich - \"Mueller\" - Strasse ... Aepfel | e c oe ? a EUR\u{A0}5"
        );
        let breit = nach_ascii(text, true);
        assert!(breit.is_ascii(), "{breit}");
        assert_eq!(
            breit,
            "Zurich - \"Muller\" - Strase . Apfel | e c o ?? a E 5"
        );
        // Jede Spalte bleibt, wo sie war: ein Zeichen je Spalte, zwei für 中, keins für das
        // kombinierende Trema.
        assert_eq!(nach_ascii("ä中\u{308}x", true), "a??x");
        assert_eq!(nach_ascii("schon ASCII ^^^", true), "schon ASCII ^^^");
    }

    /// Mit --ascii sind Hilfe, Meldungen und config in ASCII und schmal.
    #[test]
    fn ascii_fuer_eigene_texte() {
        NUR_ASCII.with(|a| a.set(true));
        for englisch in [false, true] {
            let h = hilfe_text(englisch);
            assert!(h.is_ascii(), "{h}");
            for z in h.lines() {
                assert!(z.chars().count() <= BREITE, "{z}");
            }
            assert!(h.contains(if englisch {
                "listed words"
            } else {
                "Woerter aus Listen"
            }));
        }
        let m = meldung_formatieren(&rolle_unbekannt("anwält", false, false));
        assert!(m.is_ascii(), "{m}");
        assert!(
            m.starts_with("promptscan: Die Rolle \"anwaelt\" gibt es nicht."),
            "{m}"
        );
        let k = konfig(
            Werte {
                role: Some("Anwältin".into()),
                ..Werte::default()
            },
            vec![],
        );
        assert!(konfiguration_text(&k, false).is_ascii());
        NUR_ASCII.with(|a| a.set(false));
    }

    /// Nur die Zeile des Schlüssels ändert sich; Kommentare und CRLF bleiben.
    #[test]
    fn zeile_setzen_behaelt_kommentare() {
        let alt = "# Rolle für die Kanzlei\nrole = \"doctor\" # Anwältin\ntarget = \"local\"\n";
        assert_eq!(
            zeile_setzen(alt, "role", "lawyer"),
            "# Rolle für die Kanzlei\nrole = \"lawyer\" # Anwältin\ntarget = \"local\"\n"
        );
        assert_eq!(
            zeile_setzen(alt, "lang", "en"),
            format!("{alt}lang = \"en\"\n")
        );
        assert_eq!(
            zeile_setzen("lang = \"de\"\r\n", "role", "bank"),
            "lang = \"de\"\r\nrole = \"bank\"\r\n"
        );
        assert_eq!(zeile_setzen("", "role", "bank"), "role = \"bank\"\n");
        assert_eq!(
            zeile_setzen("lang = \"de\"", "role", "bank"),
            "lang = \"de\"\nrole = \"bank\"\n"
        );
        // Ein # in Anführungszeichen ist kein Kommentar; ein auskommentierter Schlüssel zählt nicht.
        assert_eq!(kommentar_am_ende("role = \"a#b\" # c"), Some("# c"));
        assert_eq!(kommentar_am_ende("role = \"a#b\""), None);
        assert_eq!(
            zeile_setzen("# role = \"x\"\n", "role", "bank"),
            "# role = \"x\"\nrole = \"bank\"\n"
        );
        assert_eq!(
            gespeichert_text(
                &Werte {
                    role: Some("doctor".into()),
                    ..Werte::default()
                },
                Some(Rolle::Lawyer),
                Some(SpracheArg::En),
                false
            ),
            "Gespeichert: role = \"lawyer\" (bisher \"doctor\"), lang = \"en\"."
        );
    }

    /// Wo das zeilenweise Ersetzen nicht genau die erwarteten Werte ergibt (ein Wert über
    /// mehrere Zeilen) oder die Datei unlesbar ist, wird sie neu geschrieben, mit einer
    /// Warnung, die das sagt.
    #[test]
    fn konfiguration_neu_geschrieben_mit_warnung() {
        let ordner = std::env::temp_dir().join(format!("promptscan-neu-{}", std::process::id()));
        std::fs::create_dir_all(&ordner).expect("Ordner");
        let pfad = ordner.join("config.toml");
        for (vorher, teil) in [
            (
                &b"# Kanzlei\nrole = \"\"\"\nlawyer\"\"\"\n"[..],
                "Kommentare sind weggefallen",
            ),
            (&b"role = \"lawyer\nlang = \"de\"\n"[..], "nicht lesbar"),
        ] {
            std::fs::write(&pfad, vorher).expect("schreiben");
            let warnung = konfiguration_schreiben(&pfad, Some(Rolle::Doctor), None, false)
                .expect("geschrieben")
                .expect("Warnung");
            assert!(warnung.contains(teil), "{warnung}");
            assert!(warnung.contains("\nDatei:\u{A0}"), "{warnung}");
            assert_eq!(
                std::fs::read_to_string(&pfad).expect("lesen"),
                "role = \"doctor\"\n"
            );
        }
        let _ = std::fs::remove_dir_all(&ordner);
    }

    #[test]
    fn umbrechen_haelt_breite_und_geschuetzte_leerzeichen() {
        let text = "eins zwei drei vier fünf sechs sieben acht neun zehn elf zwölf Art.\u{A0}5\u{A0}lit.\u{A0}c\u{A0}DSG";
        for z in umbrechen(text, 20, 20) {
            assert!(z.chars().count() <= 20 || !z.contains(' '), "{z:?}");
        }
        assert!(
            umbrechen(text, 20, 20)
                .iter()
                .any(|z| z.ends_with("Art.\u{A0}5\u{A0}lit.\u{A0}c\u{A0}DSG"))
        );
        assert_eq!(umbrechen("", 10, 10), vec![String::new()]);
        assert_eq!(
            umbrechen("C:\\ein\\sehr\\langer\\pfad", 5, 5),
            vec!["C:\\ein\\sehr\\langer\\pfad"]
        );
    }

    #[test]
    fn meldung_zweizeilig() {
        let m = meldung_formatieren("Die Datei «x» gibt es nicht.\nPfad prüfen.");
        assert_eq!(
            m,
            "promptscan: Die Datei «x» gibt es nicht.\n  Pfad prüfen.\n"
        );
        let lang = meldung_formatieren(&format!("Problem.\n{}", rollenliste()));
        for z in lang.lines() {
            assert!(z.chars().count() <= BREITE, "{z}");
        }
        assert!(lang.lines().skip(1).all(|z| z.starts_with("  ")));
    }

    #[test]
    fn rollen_eingaben() {
        for r in Rolle::ALLE {
            assert_eq!(Rolle::aus_eingabe(r.name()), Some(r));
            assert_eq!(Rolle::aus_eingabe(&r.name().to_uppercase()), Some(r));
        }
        for (wort, rolle) in [
            ("anwalt", Rolle::Lawyer),
            ("Anwältin", Rolle::Lawyer),
            ("ANWÄLTIN", Rolle::Lawyer),
            ("Anwa\u{308}ltin", Rolle::Lawyer),
            (" ärztin ", Rolle::Doctor),
            ("Pflegefachperson", Rolle::Doctor),
            ("Notar", Rolle::Notary),
            ("Behörde", Rolle::Authority),
            ("Staatsanwaltschaft", Rolle::CriminalAuthority),
            ("Vermögensverwalter", Rolle::FinancialInstitution),
            ("Financial-Institution", Rolle::FinancialInstitution),
            ("keine", Rolle::None),
        ] {
            assert_eq!(Rolle::aus_eingabe(wort), Some(rolle), "{wort}");
        }
        for wort in ["offen", "pilot", "jurist", "", "hr"] {
            assert_eq!(Rolle::aus_eingabe(wort), None, "{wort}");
        }
        // Jedes Eingabewort ist klein, eindeutig und keine Kennung.
        let mut gesehen = std::collections::HashSet::new();
        for (wort, _) in EINGABEWOERTER {
            assert_eq!(*wort, wort.to_lowercase());
            assert!(gesehen.insert(*wort), "doppelt: {wort}");
            assert!(Rolle::aus_name(wort).is_none(), "{wort} ist eine Kennung");
        }
    }

    #[test]
    fn vorschlaege() {
        let k = rollen_kandidaten();
        for (eingabe, rolle) in [
            ("anwält", Rolle::Lawyer),
            ("Anwälte", Rolle::Lawyer),
            ("laywer", Rolle::Lawyer),
            ("ärztinnen", Rolle::Doctor),
            ("doktor", Rolle::Doctor),
            ("psychologen", Rolle::Psychologist),
            ("notarn", Rolle::Notary),
        ] {
            assert_eq!(vorschlag(eingabe, &k), Some(rolle), "{eingabe}");
        }
        for eingabe in ["pilot", "xy", "Hausmeister"] {
            assert_eq!(vorschlag(eingabe, &k), None, "{eingabe}");
        }
        assert_eq!(abstand("lawyer", "laywer"), 1);
        assert_eq!(abstand("", "abc"), 3);
        let m = rolle_unbekannt("anwält", false, false).replace('\u{A0}', " ");
        assert!(
            m.starts_with("Die Rolle «anwält» gibt es nicht.\nGemeint ist wohl: --role lawyer\n"),
            "{m}"
        );
        let m = rolle_unbekannt("anwält", true, true).replace('\u{A0}', " ");
        assert!(m.contains("\nDid you mean: role = \"lawyer\""), "{m}");
        // Der Befehl ist gegen den Umbruch geschützt (U+00A0).
        let m = rolle_unbekannt("pilot", false, false);
        assert!(m.contains("financial-institution"), "{m}");
        assert!(m.contains("promptscan\u{A0}explain\u{A0}rot"), "{m}");
    }

    #[test]
    fn json_in_ascii() {
        let roh = serde_json::json!({"stelle": "Hans Müller «x» 𝄞 \u{A0}", "n": 1}).to_string();
        let ascii = json_ascii(&roh);
        assert!(ascii.is_ascii(), "{ascii}");
        assert!(ascii.contains("M\\u00fcller") && ascii.contains("\\ud834\\udd1e"));
        let a: serde_json::Value = serde_json::from_str(&roh).expect("JSON");
        let b: serde_json::Value = serde_json::from_str(&ascii).expect("JSON");
        assert_eq!(a, b);
    }

    #[test]
    fn datei_mit_bom_ohne_farbe() {
        let bytes = datei_inhalt("\u{1b}[1mROT\u{1b}[0m Müller\n", false);
        assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
        assert_eq!(std::str::from_utf8(&bytes[3..]), Ok("ROT Müller\n"));
        let bytes = datei_inhalt("{\"a\": \"ü\"}", true);
        assert_eq!(std::str::from_utf8(&bytes[3..]), Ok("{\"a\": \"\\u00fc\"}"));
    }

    #[test]
    fn breite() {
        assert_eq!(breite_bestimmen(true, Some(200), Some("120")), 80);
        assert_eq!(breite_bestimmen(false, Some(120), Some("100")), 120);
        assert_eq!(breite_bestimmen(false, None, Some("100")), 100);
        assert_eq!(breite_bestimmen(false, None, Some(" 132 ")), 132);
        assert_eq!(breite_bestimmen(false, None, Some("abc")), 80);
        assert_eq!(breite_bestimmen(false, None, None), 80);
        assert_eq!(breite_bestimmen(false, Some(40), None), 60);
        assert_eq!(breite_bestimmen(false, None, Some("0")), 80);
    }

    #[test]
    fn ascii_rueckfall() {
        assert!(!ascii_aus_umgebung(true, Some("C"), None, None));
        assert!(ascii_aus_umgebung(false, Some("C"), None, None));
        assert!(ascii_aus_umgebung(false, None, None, Some("POSIX")));
        assert!(ascii_aus_umgebung(
            false,
            None,
            Some("de_CH.ISO-8859-1"),
            None
        ));
        assert!(!ascii_aus_umgebung(false, None, None, Some("de_CH.UTF-8")));
        assert!(!ascii_aus_umgebung(false, None, None, Some("en_US.utf8")));
        // LC_ALL geht vor; eine leere Variable zählt nicht.
        assert!(!ascii_aus_umgebung(false, Some("C.UTF-8"), Some("C"), None));
        assert!(ascii_aus_umgebung(
            false,
            Some(""),
            Some("C"),
            Some("C.UTF-8")
        ));
        assert!(!ascii_aus_umgebung(false, None, None, None));
    }

    #[test]
    fn warten_nur_beim_doppelklick() {
        assert!(am_ende_warten(true, Some(1), true));
        // In einer Shell hängen mindestens zwei Prozesse an der Konsole.
        assert!(!am_ende_warten(true, Some(2), true));
        // Umleitung oder Pipe.
        assert!(!am_ende_warten(true, Some(1), false));
        // Keine Konsole, kein Windows.
        assert!(!am_ende_warten(true, None, true));
        assert!(!am_ende_warten(false, Some(1), true));
        // Das Enter aus «Ctrl+Z und Enter» kommt sofort und schliesst nicht.
        use std::time::Duration;
        assert!(!schliesst(Duration::ZERO));
        assert!(!schliesst(Duration::from_millis(400)));
        assert!(schliesst(Duration::from_millis(1000)));
        assert!(schliesst(Duration::from_secs(30)));
    }

    #[test]
    fn verschluckte_optionen_erkennen() {
        let t = "Mandant Hans Muster, AHV 756.1234.5678.97, Akte unter C:\\Akten\" --role lawyer --fail-on yellow";
        assert_eq!(
            verschluckte_optionen(t),
            Some("\" --role lawyer --fail-on yellow")
        );
        assert!(verschluckte_optionen("Akte\" --fail-on=yellow").is_some());
        assert!(verschluckte_optionen("x\"  -t y").is_some());
        for gut in [
            "Er sagte \"ja\" und ging.",
            "Er sagte \"ja\" --  nein",
            "C:\\Akten\"",
            "Optionen wie --role lawyer stehen vorn.",
            "\"--role\" ohne Leerzeichen",
        ] {
            assert_eq!(verschluckte_optionen(gut), None, "{gut}");
        }
        let m = verschluckte_optionen_text("\" --role lawyer", false);
        assert!(
            m.contains("«\" --role lawyer»") && m.contains("\nVermutlich"),
            "{m}"
        );
    }

    fn konfig(werte: Werte, fehler: Vec<KonfigFehler>) -> Konfig {
        Konfig {
            ort: Some((PathBuf::from("config.toml"), OrtQuelle::Umgebung)),
            vorhanden: true,
            werte,
            fehler,
        }
    }

    #[test]
    fn konfiguration_aus_bytes() {
        let erwartet = Werte {
            role: Some("lawyer".into()),
            lang: Some("en".into()),
            target: None,
        };
        let text = "role = \"lawyer\"\r\nlang = \"en\"\r\n";
        // So schreibt Windows PowerShell 5.1 mit «>»: UTF-16LE mit BOM, CRLF.
        for bytes in [
            text.as_bytes().to_vec(),
            utf16(text, true, true),
            [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        ] {
            assert_eq!(konfig_aus_bytes(&bytes), (erwartet.clone(), vec![]));
        }
        let (_, f) = konfig_aus_bytes(b"lang = \"de\"\nrole = lawyer\n");
        assert_eq!(
            f,
            vec![KonfigFehler::Syntax {
                zeile: 2,
                inhalt: "role = lawyer".into()
            }]
        );
        let (w, f) = konfig_aus_bytes(b"rolle = \"lawyer\"\nlang = \"de\"\nrole = 3\n");
        assert_eq!(w.lang.as_deref(), Some("de"));
        assert_eq!(
            f,
            vec![
                KonfigFehler::KeinText("role".into()),
                KonfigFehler::Schluessel("rolle".into())
            ]
        );
        assert_eq!(
            konfig_aus_bytes(&[0xFF, 0xFE, 0, 0]).1,
            vec![KonfigFehler::Kodierung(Lesefehler::Utf32)]
        );
    }

    /// Die Herkunft der Rolle; Fehler sperren nur die Prüfung ohne --role.
    #[test]
    fn konfiguration_anwenden_regeln() {
        let mit_rolle = |r: &str| {
            konfig(
                Werte {
                    role: Some(r.into()),
                    ..Werte::default()
                },
                vec![],
            )
        };
        let a = konfiguration_anwenden(&mit_rolle("Anwältin"), None, None, true, false);
        assert_eq!(
            (a.rolle, a.quelle),
            (Some(Rolle::Lawyer), RolleQuelle::Konfiguration)
        );
        assert!(a.warnungen.is_empty() && a.fehler.is_empty());
        let a =
            konfiguration_anwenden(&mit_rolle("lawyer"), Some(Rolle::Doctor), None, true, false);
        assert_eq!(
            (a.rolle, a.quelle),
            (Some(Rolle::Doctor), RolleQuelle::Option)
        );
        let a = konfiguration_anwenden(&Konfig::default(), None, None, true, false);
        assert_eq!((a.rolle, a.quelle), (None, RolleQuelle::Keine));

        // Falsche Rolle: Fehler bei der Prüfung, Warnung bei rules usw. und mit --role.
        let a = konfiguration_anwenden(&mit_rolle("pilot"), None, None, true, false);
        assert_eq!(a.fehler.len(), 1, "{a:?}");
        assert!(a.fehler[0].contains("«pilot»") && a.fehler[0].contains("config.toml"));
        let a = konfiguration_anwenden(&mit_rolle("pilot"), None, None, false, false);
        assert!(a.fehler.is_empty() && a.warnungen.len() == 1);
        let a = konfiguration_anwenden(&mit_rolle("pilot"), Some(Rolle::Bank), None, true, true);
        assert!(a.fehler.is_empty());
        assert_eq!(
            a.warnungen
                .iter()
                .map(|w| w.replace('\u{A0}', " "))
                .collect::<Vec<_>>(),
            vec![
                "Configuration: there is no role «pilot»; --role bank applies.\nFile: config.toml"
            ]
        );

        // Lese- und Syntaxfehler: sperren die Prüfung nur ohne --role.
        let kaputt = konfig(Werte::default(), vec![KonfigFehler::Zugriff]);
        assert_eq!(
            konfiguration_anwenden(&kaputt, None, None, true, false)
                .fehler
                .len(),
            1
        );
        let a = konfiguration_anwenden(&kaputt, Some(Rolle::None), None, true, false);
        assert!(a.fehler.is_empty() && a.warnungen.len() == 1);

        // Sprache: falsch ohne --lang sperrt die Prüfung, mit --lang nur Warnung.
        let sprache = konfig(
            Werte {
                lang: Some("fr".into()),
                ..Werte::default()
            },
            vec![],
        );
        assert_eq!(
            konfiguration_anwenden(&sprache, None, None, true, false)
                .fehler
                .len(),
            1
        );
        let a = konfiguration_anwenden(&sprache, None, Some(SpracheArg::De), true, false);
        assert!(a.fehler.is_empty() && a.warnungen[0].contains("es gilt --lang\u{A0}de"));

        // target wird übergangen, mit Warnung, auch bei rules und explain.
        let ziel = konfig(
            Werte {
                target: Some("local".into()),
                ..Werte::default()
            },
            vec![],
        );
        for prueft in [true, false] {
            let a = konfiguration_anwenden(&ziel, None, None, prueft, false);
            assert!(a.fehler.is_empty());
            let w = a.warnungen[0].replace('\u{A0}', " ");
            assert!(w.contains("target = \"local\" wird übergangen"), "{w}");
            // Die Warnung sagt, wie sie verschwindet.
            assert!(w.contains("Die Zeile target in der Datei löschen"), "{w}");
        }
    }

    /// Hilfe in beiden Sprachen, höchstens 80 Spalten, mit allem, was sie enthalten muss.
    #[test]
    fn hilfe_vollstaendig_und_schmal() {
        for englisch in [false, true] {
            let h = hilfe_text(englisch);
            for z in h.lines() {
                assert!(z.chars().count() <= BREITE, "{}: {z}", z.chars().count());
            }
            assert!(!h.contains('\u{A0}'));
            let einzeilig = h.split_whitespace().collect::<Vec<_>>().join(" ");
            let hilfe_t = if englisch { HILFE_T_EN } else { HILFE_T_DE }.replace('\u{A0}', " ");
            assert!(einzeilig.contains(&hilfe_t), "Hilfe zu -t fehlt");
            let vorbehalt = if englisch { VORBEHALT_EN } else { VORBEHALT_DE };
            assert!(h.contains(vorbehalt), "Hilfsmittel-Zeile fehlt");
            for r in Rolle::ALLE {
                assert!(einzeilig.contains(r.name()), "{}", r.name());
            }
            for muss in [
                "promptscan config --role lawyer",
                "Ctrl+Z",
                "--output",
                "--quiet",
                "--ascii",
                "--target cloud|local",
                "PROMPTSCAN_CONFIG",
                "Windows PowerShell 5.1",
            ] {
                assert!(einzeilig.contains(muss), "{muss} fehlt");
            }
        }
        let de = hilfe_text(false)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        // Der Text zu --target steht wortgleich in der Hilfe, ohne den Halbsatz
        // zur Konfigurationsdatei: target aus der Datei wird übergangen.
        for muss in [
            "Deutsch, Französisch und Italienisch",
            "Vorgabe cloud",
            "nur mit --role oder aus dem Text",
            "Wohin der Prompt geht; local nur, wenn das Modell auf diesem Rechner läuft, ohne Synchronisierung in eine Cloud.",
        ] {
            assert!(de.contains(muss), "{muss} fehlt");
        }
        let en = hilfe_text(true)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for muss in [
            "German, French and Italian",
            "Default cloud",
            "only with --role or inferred from the text",
            "Where the prompt goes; local only if the model runs on this computer, without syncing to a cloud.",
        ] {
            assert!(en.contains(muss), "{muss} fehlt");
        }
        for text in [&de, &en] {
            assert!(!text.contains("gilt es für jeden Prompt"), "{text}");
            assert!(!text.contains("applies to every prompt"), "{text}");
        }
    }

    #[test]
    fn konfiguration_zeigen() {
        let k = konfig(
            Werte {
                role: Some("anwalt".into()),
                lang: Some("en".into()),
                target: Some("local".into()),
            },
            vec![],
        );
        let t = konfiguration_text(&k, false);
        assert!(t.contains("config.toml") && t.contains("Ort aus PROMPTSCAN_CONFIG"));
        assert!(
            t.contains("role = \"anwalt\"") && t.contains("(lawyer)"),
            "{t}"
        );
        assert!(t.contains("wird übergangen"), "{t}");
        for z in t.lines() {
            assert!(z.chars().count() <= BREITE, "{z}");
        }
        let t = konfiguration_text(
            &Konfig {
                vorhanden: false,
                ..konfig(Werte::default(), vec![])
            },
            true,
        );
        assert!(
            t.contains("(does not exist yet)") && t.contains("(no values)"),
            "{t}"
        );
    }

    /// Unter Windows endet die Eingabe nur mit Ctrl+Z am Zeilenanfang.
    #[test]
    fn einfuege_hinweis_je_system() {
        assert_eq!(
            einfuege_hinweis(true, false),
            "promptscan (Terminalprogramm): Prompt einfügen, Enter, dann Ctrl+Z und Enter."
        );
        assert!(einfuege_hinweis(true, true).contains("press Enter, then Ctrl+Z and Enter"));
        assert!(einfuege_hinweis(false, false).ends_with("Enter, dann Ctrl+D."));
        assert!(einfuege_hinweis(false, true).contains("terminal program"));
    }

    #[test]
    fn explain_unbekannt_mit_vorschlag_und_liste() {
        let m = explain_unbekannt("ahw", false).replace('\u{A0}', " ");
        assert!(
            m.starts_with("Eine Regel oder Stufe «ahw» gibt es nicht.\nGemeint ist wohl: promptscan explain ahv\n"),
            "{m}"
        );
        for r in katalog::alle() {
            assert!(m.contains(r.id), "{}", r.id);
        }
        let m = explain_unbekannt("Gelbe", true).replace('\u{A0}', " ");
        assert!(m.contains("\nDid you mean: promptscan explain gelb"), "{m}");
        let m = explain_unbekannt("xyz", true);
        assert!(
            m.starts_with("There is no rule or level «xyz».\nLevels:"),
            "{m}"
        );
    }
}
