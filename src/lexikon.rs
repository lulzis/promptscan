//! Wortlisten und Register.
//!
//! **Wortliste** (`lexika/*.toml`): Jeder Eintrag trägt seine Quelle (SPEC 2, Grundsatz 2).
//!
//! ```toml
//! [[eintrag]]
//! begriff = "Arbeitsunfähigkeit"          # Lemma, wörtlich wie in der Quelle
//! formen = ["arbeitsunfähig"]             # weitere Formen (Flexion), optional
//! sprache = "de"                          # de | fr | it
//! regel = "art5c-2"
//! quelle = "SR 830.1 Art. 6"              # Fundstelle
//! fassung = "2024-01-01"                  # Datum der Fassung, JJJJ-MM-TT
//! url = "https://www.fedlex.admin.ch/eli/cc/2002/510/de#art_6"
//! gross = false                           # optional: Gross-/Kleinschreibung beachten
//! teilwort = "nein"                       # optional: nein | anfang | ende | frei
//! merkmal = "lawyer"                      # optional: regelabhängig, z. B. Rolle
//! notiz = "…"                             # optional: Begründung, Abgrenzung
//! ```
//!
//! **Register** (`lexika/register/*.tsv`): lange amtliche Listen (Vornamen, Gemeinden,
//! Postleitzahlen). Die Quelle steht einmal im Kopf, weil jede Zeile aus demselben
//! Datensatz stammt:
//!
//! ```text
//! # titel: Vornamen der ständigen Wohnbevölkerung
//! # herausgeber: Bundesamt für Statistik (BFS)
//! # url: https://…
//! # lizenz: …
//! # abgerufen: 2026-11-06
//! # rohdatei-sha256: …
//! # filter: …
//! # spalten: vorname<TAB>anzahl
//! Anna<TAB>12345
//! ```
//!
//! Gesucht wird mit Aho-Corasick in einem Durchlauf, Gross-/Kleinschreibung und
//! typografische Varianten (’ und ', geschütztes Leerzeichen, ß und ss) ausgeglichen,
//! an Wortgrenzen.

use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, OnceLock};

use aho_corasick::{AhoCorasick, MatchKind};
use serde::{Deserialize, Serialize};

use crate::fund::Beleg;
use crate::text::{Text, ist_unsichtbar, ist_wortzeichen, links_frei, rechts_frei};

include!(concat!(env!("OUT_DIR"), "/lexika.rs"));

/// Sprache eines Eintrags: die Amtssprachen des Bundes (Art. 70 Abs. 1 BV).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Sprache {
    De,
    Fr,
    It,
}

/// Wie weit ein Eintrag in ein Wort hineinreichen darf.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Teilwort {
    /// Nur ganze Wörter (Vorgabe).
    #[default]
    Nein,
    /// Am Wortanfang, das Wort darf weitergehen («Arbeitsunfähigkeit» in «Arbeitsunfähigkeitszeugnis»).
    Anfang,
    /// Am Wortende, davor darf etwas stehen («krebs» in «Lungenkrebs»).
    Ende,
    /// Überall.
    Frei,
}

/// Ein Eintrag einer Wortliste.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Eintrag {
    pub begriff: String,
    #[serde(default)]
    pub formen: Vec<String>,
    pub sprache: Sprache,
    pub regel: String,
    pub quelle: String,
    pub fassung: String,
    pub url: String,
    #[serde(default)]
    pub gross: bool,
    #[serde(default)]
    pub teilwort: Teilwort,
    #[serde(default)]
    pub merkmal: Option<String>,
    #[serde(default)]
    pub notiz: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Datei {
    #[serde(default)]
    eintrag: Vec<Eintrag>,
}

/// Fundstelle eines Eintrags im Text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LexTreffer {
    /// Byte-Versatz im NFC-Text.
    pub start: usize,
    pub ende: usize,
    /// Index in [`Lexikon::eintraege`].
    pub eintrag: usize,
}

/// Eine geladene Wortliste mit Suchautomaten.
pub struct Lexikon {
    pub datei: String,
    pub eintraege: Vec<Eintrag>,
    /// Automat über gefaltete Muster (ohne Gross-/Kleinschreibung).
    klein: Option<AhoCorasick>,
    klein_eintrag: Vec<usize>,
    /// Automat über Muster mit Gross-/Kleinschreibung (`gross = true`).
    gross: Option<AhoCorasick>,
    gross_eintrag: Vec<usize>,
}

impl Lexikon {
    /// Liest eine Wortliste und prüft jeden Eintrag (siehe [`pruefe_eintrag`]).
    pub fn aus_toml(datei: &str, inhalt: &str) -> Result<Lexikon, String> {
        let gelesen: Datei = toml::from_str(inhalt).map_err(|e| format!("{datei}: {e}"))?;
        for (i, e) in gelesen.eintrag.iter().enumerate() {
            pruefe_eintrag(e)
                .map_err(|m| format!("{datei}, Eintrag {} («{}»): {m}", i + 1, e.begriff))?;
        }
        Ok(Lexikon::aus_eintraegen(datei, gelesen.eintrag))
    }

    /// Baut die Automaten. Jede Form eines Eintrags wird ein Muster.
    pub fn aus_eintraegen(datei: &str, eintraege: Vec<Eintrag>) -> Lexikon {
        let mut klein_muster = Vec::new();
        let mut klein_eintrag = Vec::new();
        let mut gross_muster = Vec::new();
        let mut gross_eintrag = Vec::new();
        for (i, e) in eintraege.iter().enumerate() {
            for form in std::iter::once(&e.begriff).chain(e.formen.iter()) {
                if e.gross {
                    let f = falten(form, false).text;
                    if !f.is_empty() {
                        gross_muster.push(f);
                        gross_eintrag.push(i);
                    }
                } else {
                    let f = falten(form, true).text;
                    if !f.is_empty() {
                        klein_muster.push(f);
                        klein_eintrag.push(i);
                    }
                }
            }
        }
        let bauen = |muster: &[String]| {
            (!muster.is_empty()).then(|| {
                AhoCorasick::builder()
                    .match_kind(MatchKind::Standard)
                    .build(muster)
                    .expect("Aho-Corasick-Automat")
            })
        };
        Lexikon {
            datei: datei.to_string(),
            klein: bauen(&klein_muster),
            gross: bauen(&gross_muster),
            klein_eintrag,
            gross_eintrag,
            eintraege,
        }
    }

    /// Alle Fundstellen an Wortgrenzen (nach [`Teilwort`]), ohne Überlappung:
    /// bei Überlappung gewinnt die längere, bei gleicher Länge die frühere Stelle.
    /// Ein Begriff aus mehreren Wörtern trifft auch über einen weichen Zeilenumbruch
    /// hinweg (siehe [`falten_mit`] und [`Text::weicher_umbruch`]).
    pub fn finde(&self, text: &Text) -> Vec<LexTreffer> {
        self.finde_mit(text.als_str(), text.weiche_umbrueche())
    }

    /// Wie [`Lexikon::finde`], auf einer beliebigen Zeichenkette. Hier ist jeder
    /// Zeilenumbruch hart: Ein Begriff trifft nicht über das Zeilenende hinweg.
    pub fn finde_in(&self, s: &str) -> Vec<LexTreffer> {
        self.finde_mit(s, &[])
    }

    fn finde_mit(&self, s: &str, weich: &[usize]) -> Vec<LexTreffer> {
        let mut roh = Vec::new();
        for (automat, eintrag_von, klein) in [
            (&self.klein, &self.klein_eintrag, true),
            (&self.gross, &self.gross_eintrag, false),
        ] {
            let Some(automat) = automat else { continue };
            let gefaltet = falten_mit(s, klein, weich);
            for m in automat.find_overlapping_iter(&gefaltet.text) {
                let (start, ende) = gefaltet.zurueck(m.start(), m.end());
                let eintrag = eintrag_von[m.pattern().as_usize()];
                if grenzen_passen(s, start, ende, self.eintraege[eintrag].teilwort) {
                    roh.push(LexTreffer {
                        start,
                        ende,
                        eintrag,
                    });
                }
            }
        }
        ohne_ueberlappung(self.sprache_waehlen(s, roh))
    }

    /// Treffen gleich geschriebene Einträge desselben Begriffs (gleiche Regel, gleiches
    /// Merkmal) in mehreren Sprachen dieselbe Stelle («HIV» deutsch und italienisch), bleibt
    /// der Eintrag in der Sprache des Textes ([`textsprache`]): Der Treffer trägt dann den
    /// Beleg dieser Sprache. Ist die Sprache nicht bestimmbar oder hat sie keinen solchen
    /// Eintrag, bleiben alle, und [`ohne_ueberlappung`] wählt den früheren Eintrag.
    fn sprache_waehlen(&self, s: &str, mut roh: Vec<LexTreffer>) -> Vec<LexTreffer> {
        roh.sort_by_key(|t| (t.start, t.ende, t.eintrag));
        let gleiche_stelle =
            |a: &LexTreffer, b: &LexTreffer| a.start == b.start && a.ende == b.ende;
        if !roh.windows(2).any(|p| gleiche_stelle(&p[0], &p[1])) {
            return roh;
        }
        let mut sprache: Option<Option<Sprache>> = None;
        let mut behalten = Vec::with_capacity(roh.len());
        let mut i = 0;
        while i < roh.len() {
            let n = roh[i..]
                .iter()
                .take_while(|t| gleiche_stelle(t, &roh[i]))
                .count();
            let gruppe = &roh[i..i + n];
            for t in gruppe {
                let e = &self.eintraege[t.eintrag];
                let andere: Vec<&Eintrag> = gruppe
                    .iter()
                    .map(|u| &self.eintraege[u.eintrag])
                    .filter(|f| {
                        f.sprache != e.sprache && f.regel == e.regel && f.merkmal == e.merkmal
                    })
                    .collect();
                let fremd = !andere.is_empty()
                    && sprache
                        .get_or_insert_with(|| textsprache(s))
                        .is_some_and(|ts| {
                            e.sprache != ts && andere.iter().any(|f| f.sprache == ts)
                        });
                if !fremd {
                    behalten.push(*t);
                }
            }
            i += n;
        }
        behalten
    }

    /// Beleg für einen Treffer: Eintrag und Quelle.
    pub fn beleg(&self, eintrag: usize) -> Beleg {
        let e = &self.eintraege[eintrag];
        Beleg {
            datei: self.datei.clone(),
            begriff: e.begriff.clone(),
            quelle: e.quelle.clone(),
            fassung: e.fassung.clone(),
            url: e.url.clone(),
        }
    }
}

/// Prüft die Pflichtfelder eines Eintrags. Ein Eintrag ohne Quelle kommt nicht hinein.
pub fn pruefe_eintrag(e: &Eintrag) -> Result<(), String> {
    if e.begriff.trim().is_empty() {
        return Err("begriff ist leer".into());
    }
    if e.regel.trim().is_empty() {
        return Err("regel ist leer".into());
    }
    if e.quelle.trim().is_empty() {
        return Err("quelle fehlt".into());
    }
    if !ist_datum(&e.fassung) {
        return Err(format!("fassung «{}» ist kein Datum JJJJ-MM-TT", e.fassung));
    }
    if !e.url.starts_with("https://") {
        return Err(format!("url «{}» beginnt nicht mit https://", e.url));
    }
    if e.formen.iter().any(|f| f.trim().is_empty()) {
        return Err("leere Form".into());
    }
    Ok(())
}

fn ist_datum(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn grenzen_passen(s: &str, start: usize, ende: usize, teilwort: Teilwort) -> bool {
    // Ein Muster, das selbst mit einem Nicht-Wortzeichen beginnt oder endet («Dr.»),
    // braucht dort keine Grenze.
    let erstes = s[start..ende].chars().next();
    let letztes = s[start..ende].chars().next_back();
    let links = !erstes.is_some_and(ist_wortzeichen) || links_frei(s, start);
    let rechts = !letztes.is_some_and(ist_wortzeichen) || rechts_frei(s, ende);
    match teilwort {
        Teilwort::Nein => links && rechts,
        Teilwort::Anfang => links,
        Teilwort::Ende => rechts,
        Teilwort::Frei => true,
    }
}

/// Wählt Treffer ohne Überlappung: die längsten zuerst, bei gleicher Länge die frühere
/// Stelle, dann der frühere Eintrag; zurück in Textreihenfolge.
///
/// Die gewählten Bereiche stehen nach Anfang geordnet in einem Baum. Weil sie sich nicht
/// überlappen, genügt für jeden Kandidaten ein Blick auf den gewählten Bereich mit dem
/// grössten Anfang vor seinem Ende. So kosten `k` Treffer O(k log k) statt O(k²).
fn ohne_ueberlappung(mut roh: Vec<LexTreffer>) -> Vec<LexTreffer> {
    roh.sort_by(|a, b| {
        (b.ende - b.start)
            .cmp(&(a.ende - a.start))
            .then(a.start.cmp(&b.start))
            .then(a.eintrag.cmp(&b.eintrag))
    });
    let mut gewaehlt: BTreeMap<usize, LexTreffer> = BTreeMap::new();
    for t in roh {
        let frei = gewaehlt
            .range(..t.ende)
            .next_back()
            .is_none_or(|(_, g)| g.ende <= t.start);
        if frei {
            gewaehlt.insert(t.start, t);
        }
    }
    gewaehlt.into_values().collect()
}

/// Häufige Funktionswörter je Sprache, eine Heuristik ohne Quelle: eine geschlossene Liste im
/// Code, die nur zwischen gleich geschriebenen Einträgen wählt
/// ([`Lexikon::sprache_waehlen`]). Ausgelassen sind Wörter, die in einer der beiden anderen
/// Sprachen ebenfalls gängig sind: «des», «du», «per», «non», «qui», dazu «ce» (IT «ce
/// l'ha», «ce n'è») und «est» (IT «Osten»; Prüferproblem c-belege-p8).
const FUNKTIONSWOERTER: [(Sprache, &[&str]); 3] = [
    (
        Sprache::De,
        &[
            "der", "die", "das", "und", "ist", "nicht", "ich", "mit", "ein", "eine", "einen",
            "dem", "den", "sie", "wir", "auf", "für", "sich", "auch", "hat", "wird", "dass", "im",
            "zu", "von", "er",
        ],
    ),
    (
        Sprache::Fr,
        &[
            "les", "et", "je", "une", "que", "pas", "elle", "nous", "vous", "pour", "dans", "sur",
            "avec", "au", "aux", "sont", "mais",
        ],
    ),
    (
        Sprache::It,
        &[
            "gli", "lo", "è", "che", "di", "del", "della", "dello", "dei", "delle", "una", "sono",
            "con", "io", "noi", "ha", "ho", "al", "alla", "nel", "nella", "anche",
        ],
    ),
];

/// Die Sprache des Textes nach den [`FUNKTIONSWOERTER`]n: die Sprache mit den meisten
/// Funktionswörtern, wenn sie mehr hat als jede andere; sonst `None`.
pub fn textsprache(s: &str) -> Option<Sprache> {
    let mut zahl = [0usize; 3];
    for wort in s
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
    {
        let klein = wort.to_lowercase();
        for (i, (_, woerter)) in FUNKTIONSWOERTER.iter().enumerate() {
            if woerter.contains(&klein.as_str()) {
                zahl[i] += 1;
            }
        }
    }
    let beste = (0..3).max_by_key(|&i| zahl[i])?;
    let eindeutig = zahl[beste] > 0 && (0..3).all(|i| i == beste || zahl[i] < zahl[beste]);
    eindeutig.then_some(FUNKTIONSWOERTER[beste].0)
}

/// Gefalteter Text mit Rückweg zu den Byte-Versätzen des Originals.
pub struct Gefaltet {
    pub text: String,
    /// Für jedes Byte von `text`: Anfang und Ende des Ursprungszeichens im Original.
    herkunft: Vec<(usize, usize)>,
}

impl Gefaltet {
    /// Byte-Bereich im gefalteten Text → Byte-Bereich im Original.
    pub fn zurueck(&self, start: usize, ende: usize) -> (usize, usize) {
        let a = self.herkunft.get(start).map(|h| h.0).unwrap_or(0);
        let e = if ende == 0 {
            a
        } else {
            self.herkunft.get(ende - 1).map(|h| h.1).unwrap_or(a)
        };
        (a, e)
    }
}

/// Faltet für den Vergleich: typografische Varianten vereinheitlichen und, wenn
/// `klein`, in Kleinbuchstaben. Unsichtbare Formatzeichen ([`ist_unsichtbar`], etwa
/// U+00AD oder U+200B) fallen weg. Jedes Byte merkt sich sein Ursprungszeichen.
pub fn falten(s: &str, klein: bool) -> Gefaltet {
    falten_mit(s, klein, &[])
}

/// Wie [`falten`], dazu die weichen Zeilenumbrüche `weich` (Byte-Versätze der `\n` in `s`,
/// aufsteigend, wie [`Text::weiche_umbrueche`]):
///
/// * Ein weicher Umbruch wird mit dem Leerraum um ihn herum zu einem Leerzeichen:
///   «psychischen⏎Störung» trifft wie «psychischen Störung».
/// * Eine Worttrennung, ein Bindestrich zwischen zwei Kleinbuchstaben vor dem Umbruch
///   («arbeits-⏎unfähig») oder ein weiches Trennzeichen U+00AD davor, wird
///   zusammengezogen: «arbeitsunfähig».
/// * Nach einem anderen Bindestrich im Wort («AHV-⏎Nummer») fällt nur der Umbruch weg:
///   «AHV-Nummer».
///
/// Die Herkunft eines Treffers reicht dann über den Umbruch: von «arbeits» bis «unfähig».
pub fn falten_mit(s: &str, klein: bool, weich: &[usize]) -> Gefaltet {
    let mut text = String::with_capacity(s.len());
    let mut herkunft = Vec::with_capacity(s.len());
    let mut weich = weich.iter().copied().peekable();
    // Nach einem weichen Umbruch: Leerraum am Anfang der nächsten Zeile überspringen.
    let mut einzug = false;
    for (i, c) in s.char_indices() {
        let ende = i + c.len_utf8();
        if einzug {
            if ist_zeilenleerraum(c) || ist_unsichtbar(c) {
                continue;
            }
            einzug = false;
        }
        while weich.next_if(|&p| p < i).is_some() {}
        if c == '\n' && weich.next_if_eq(&i).is_some() {
            umbruch_falten(s, i, &mut text, &mut herkunft);
            einzug = true;
            continue;
        }
        let schieben = |z: char, text: &mut String, herkunft: &mut Vec<(usize, usize)>| {
            text.push(z);
            for _ in 0..z.len_utf8() {
                herkunft.push((i, ende));
            }
        };
        match c {
            // Unsichtbare Formatzeichen trennen für den Vergleich nichts.
            _ if ist_unsichtbar(c) => {}
            // Apostroph-Varianten: ’ ‘ ʼ ′ ´ `
            '\u{2019}' | '\u{2018}' | '\u{02BC}' | '\u{2032}' | '\u{00B4}' | '`' => {
                schieben('\'', &mut text, &mut herkunft)
            }
            // Leerzeichen-Varianten: geschützt, schmal geschützt, schmal, Ziffernabstand
            '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\u{2007}' => {
                schieben(' ', &mut text, &mut herkunft)
            }
            // Bindestrich-Varianten: Bindestrich, geschützter Bindestrich
            '\u{2010}' | '\u{2011}' => schieben('-', &mut text, &mut herkunft),
            // Schweizer Rechtschreibung kennt kein ß; «ß» und «ẞ» zählen als «ss».
            '\u{00DF}' | '\u{1E9E}' => {
                schieben('s', &mut text, &mut herkunft);
                schieben('s', &mut text, &mut herkunft);
            }
            _ if klein => {
                for z in c.to_lowercase() {
                    schieben(z, &mut text, &mut herkunft);
                }
            }
            _ => schieben(c, &mut text, &mut herkunft),
        }
    }
    Gefaltet { text, herkunft }
}

/// Leerraum innerhalb einer Zeile: Leerzeichen, Tabulator und die Leerzeichen-Varianten,
/// die [`falten`] als Leerzeichen liest.
fn ist_zeilenleerraum(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\u{2007}'
    )
}

/// Faltet den weichen Umbruch bei `p` (siehe [`falten_mit`]). Der gefaltete Text endet
/// mit dem, was vor dem Umbruch stand; Leerraum davor (Leerzeichen, Tabulator und die
/// Varianten, die schon als Leerzeichen gefaltet sind) wird entfernt.
fn umbruch_falten(s: &str, p: usize, text: &mut String, herkunft: &mut Vec<(usize, usize)>) {
    while text.ends_with([' ', '\t']) {
        text.pop();
        herkunft.pop();
    }
    let vor_umbruch = s[..p].trim_end_matches(|c: char| ist_zeilenleerraum(c));
    if vor_umbruch.ends_with('\u{AD}') {
        return;
    }
    if text.ends_with('-') {
        let strich = herkunft.last().map_or(0, |h| h.0);
        let davor = s[..strich].chars().rev().find(|&c| !ist_unsichtbar(c));
        if davor.is_some_and(char::is_alphabetic) {
            let danach = s[p + 1..]
                .chars()
                .find(|&c| !ist_zeilenleerraum(c) && !ist_unsichtbar(c));
            if davor.is_some_and(char::is_lowercase) && danach.is_some_and(char::is_lowercase) {
                text.pop();
                herkunft.pop();
            }
            return;
        }
    }
    text.push(' ');
    herkunft.push((p, p + 1));
}

/// Ein Register: lange amtliche Liste mit Quellenkopf.
pub struct Register {
    pub datei: String,
    /// Kopfzeilen `# schluessel: wert`, in Reihenfolge.
    pub kopf: Vec<(String, String)>,
    pub spalten: Vec<String>,
    pub zeilen: Vec<Vec<String>>,
}

/// Pflichtfelder im Kopf eines Registers.
pub const REGISTER_PFLICHT: [&str; 8] = [
    "titel",
    "herausgeber",
    "url",
    "lizenz",
    "abgerufen",
    "rohdatei-sha256",
    "filter",
    "spalten",
];

impl Register {
    pub fn aus_tsv(datei: &str, inhalt: &str) -> Result<Register, String> {
        let mut kopf = Vec::new();
        let mut zeilen = Vec::new();
        for (nr, zeile) in inhalt.lines().enumerate() {
            let zeile = zeile.trim_end_matches('\r');
            if let Some(rest) = zeile.strip_prefix('#') {
                if let Some((k, w)) = rest.split_once(':') {
                    kopf.push((k.trim().to_string(), w.trim().to_string()));
                }
                continue;
            }
            if zeile.trim().is_empty() {
                continue;
            }
            let felder: Vec<String> = zeile.split('\t').map(|f| f.to_string()).collect();
            if felder.iter().any(|f| f.trim().is_empty()) {
                return Err(format!("{datei}, Zeile {}: leeres Feld", nr + 1));
            }
            zeilen.push(felder);
        }
        for pflicht in REGISTER_PFLICHT {
            if !kopf.iter().any(|(k, w)| k == pflicht && !w.is_empty()) {
                return Err(format!("{datei}: Kopfzeile «# {pflicht}: …» fehlt"));
            }
        }
        let spalten: Vec<String> = kopf
            .iter()
            .find(|(k, _)| k == "spalten")
            .map(|(_, w)| w.split('\t').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        if let Some((nr, z)) = zeilen
            .iter()
            .enumerate()
            .find(|(_, z)| z.len() != spalten.len())
        {
            return Err(format!(
                "{datei}: Datenzeile {} hat {} statt {} Spalten",
                nr + 1,
                z.len(),
                spalten.len()
            ));
        }
        Ok(Register {
            datei: datei.to_string(),
            kopf,
            spalten,
            zeilen,
        })
    }

    pub fn kopf(&self, schluessel: &str) -> Option<&str> {
        self.kopf
            .iter()
            .find(|(k, _)| k == schluessel)
            .map(|(_, w)| w.as_str())
    }

    pub fn spalte(&self, name: &str) -> Option<usize> {
        self.spalten.iter().position(|s| s == name)
    }

    /// Alle Werte einer Spalte.
    pub fn werte<'a>(&'a self, spalte: &str) -> impl Iterator<Item = &'a str> + 'a {
        let index = self.spalte(spalte);
        self.zeilen
            .iter()
            .filter_map(move |z| index.map(|i| z[i].as_str()))
    }
}

/// Alle eingebetteten Dateien unter `lexika/` (relativer Pfad mit `/`).
pub fn dateien() -> impl Iterator<Item = &'static str> {
    DATEIEN.iter().map(|(name, _)| *name)
}

/// Inhalt einer eingebetteten Datei.
pub fn inhalt(datei: &str) -> Option<&'static str> {
    DATEIEN
        .iter()
        .find(|(name, _)| *name == datei)
        .map(|(_, inhalt)| *inhalt)
}

static LEXIKA: LazyLock<std::sync::Mutex<HashMap<String, &'static Lexikon>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
static REGISTER: LazyLock<std::sync::Mutex<HashMap<String, &'static Register>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Die Wortliste `datei` (z. B. `"art5c-2.toml"`), einmal geladen und dann behalten.
/// Fehlt sie, gibt es `None`. Ist sie fehlerhaft, bricht das Programm ab; die Tests
/// (`tests/lexika.rs`) prüfen jede Datei, bevor sie ausgeliefert wird.
pub fn lexikon_falls(datei: &str) -> Option<&'static Lexikon> {
    let inhalt = inhalt(datei)?;
    let mut karte = LEXIKA.lock().expect("Lexikon-Sperre");
    if let Some(lex) = karte.get(datei) {
        return Some(lex);
    }
    let lex =
        Lexikon::aus_toml(datei, inhalt).unwrap_or_else(|e| panic!("Wortliste fehlerhaft: {e}"));
    let lex: &'static Lexikon = Box::leak(Box::new(lex));
    karte.insert(datei.to_string(), lex);
    Some(lex)
}

/// Wie [`lexikon_falls`], aber die Datei muss es geben.
pub fn lexikon(datei: &str) -> &'static Lexikon {
    lexikon_falls(datei).unwrap_or_else(|| panic!("Wortliste lexika/{datei} fehlt"))
}

/// Das Register `datei` (z. B. `"register/vornamen.tsv"`), einmal geladen.
pub fn register_falls(datei: &str) -> Option<&'static Register> {
    let inhalt = inhalt(datei)?;
    let mut karte = REGISTER.lock().expect("Register-Sperre");
    if let Some(reg) = karte.get(datei) {
        return Some(reg);
    }
    let reg =
        Register::aus_tsv(datei, inhalt).unwrap_or_else(|e| panic!("Register fehlerhaft: {e}"));
    let reg: &'static Register = Box::leak(Box::new(reg));
    karte.insert(datei.to_string(), reg);
    Some(reg)
}

/// Wie [`register_falls`], aber die Datei muss es geben.
pub fn register(datei: &str) -> &'static Register {
    register_falls(datei).unwrap_or_else(|| panic!("Register lexika/{datei} fehlt"))
}

/// Einmal gebaute Wortliste aus Code (für Tests und Regeln mit fester Liste).
pub fn einmal<F: FnOnce() -> Lexikon>(
    zelle: &'static OnceLock<Lexikon>,
    bauen: F,
) -> &'static Lexikon {
    zelle.get_or_init(bauen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eintrag(begriff: &str, formen: &[&str], teilwort: Teilwort, gross: bool) -> Eintrag {
        Eintrag {
            begriff: begriff.into(),
            formen: formen.iter().map(|s| s.to_string()).collect(),
            sprache: Sprache::De,
            regel: "test".into(),
            quelle: "Test".into(),
            fassung: "2026-01-01".into(),
            url: "https://example.org".into(),
            gross,
            teilwort,
            merkmal: None,
            notiz: None,
        }
    }

    /// Reste Runde 1 (w2-art5c): Gleich geschriebene Einträge desselben Begriffs in mehreren
    /// Sprachen; der Treffer trägt den Eintrag in der Sprache des Textes.
    #[test]
    fn gleiche_schreibung_beleg_nach_sprache_des_textes() {
        let mit = |sprache, regel: &str, merkmal: Option<&str>| Eintrag {
            sprache,
            regel: regel.into(),
            merkmal: merkmal.map(Into::into),
            url: format!("https://example.org/{sprache:?}"),
            ..eintrag("HIV", &[], Teilwort::Nein, true)
        };
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                mit(Sprache::De, "test", None),
                mit(Sprache::It, "test", None),
            ],
        );
        let sprache_bei = |s: &str| {
            let t = lex.finde_in(s);
            assert_eq!(t.len(), 1, "{s}");
            lex.eintraege[t[0].eintrag].sprache
        };
        assert_eq!(sprache_bei("È positivo all'HIV."), Sprache::It);
        // Prüferproblem c-belege-p8: «ce» und «est» zählen nicht als Französisch.
        assert_eq!(
            textsprache("La mia paziente Giulia Muster ce l'ha, l'HIV."),
            Some(Sprache::It)
        );
        assert_eq!(textsprache("ce est"), None);
        assert_eq!(
            sprache_bei("Er ist HIV-positiv, und das ist neu."),
            Sprache::De
        );
        // Nicht bestimmbar: der frühere Eintrag.
        assert_eq!(sprache_bei("HIV"), Sprache::De);
        // Französischer Text ohne französischen Eintrag: der frühere Eintrag.
        assert_eq!(
            sprache_bei("Il est séropositif (HIV) et je suis inquiet."),
            Sprache::De
        );
        // Verschiedene Regeln oder Merkmale: keine Wahl nach Sprache, wie bisher.
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![mit(Sprache::De, "a", None), mit(Sprache::It, "b", None)],
        );
        let t = lex.finde_in("È positivo all'HIV.");
        assert_eq!(lex.eintraege[t[0].eintrag].regel, "a");

        assert_eq!(
            textsprache("Der Klient ist krank und hat Fieber."),
            Some(Sprache::De)
        );
        assert_eq!(
            textsprache("Je suis avocate et elle est malade."),
            Some(Sprache::Fr)
        );
        assert_eq!(
            textsprache("Il paziente è malato e non lavora."),
            Some(Sprache::It)
        );
        assert_eq!(textsprache("HIV"), None);
        assert_eq!(textsprache("die è"), None);
    }

    #[test]
    fn ganze_woerter_ohne_gross_klein() {
        let lex =
            Lexikon::aus_eintraegen("t", vec![eintrag("Diagnose", &[], Teilwort::Nein, false)]);
        let t = Text::neu("DIAGNOSE: Krebsdiagnose, diagnosen");
        let treffer = lex.finde(&t);
        assert_eq!(treffer.len(), 1);
        assert_eq!(&t.als_str()[treffer[0].start..treffer[0].ende], "DIAGNOSE");
    }

    #[test]
    fn teilwort_ende() {
        let lex = Lexikon::aus_eintraegen("t", vec![eintrag("krebs", &[], Teilwort::Ende, false)]);
        let t = Text::neu("Lungenkrebs und Krebsrisiko");
        let treffer = lex.finde(&t);
        assert_eq!(treffer.len(), 1);
        assert_eq!(&t.als_str()[treffer[0].start..treffer[0].ende], "krebs");
    }

    #[test]
    fn apostroph_und_eszett_gefaltet() {
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag("l'avocat", &[], Teilwort::Nein, false),
                eintrag("Strasse", &[], Teilwort::Nein, false),
            ],
        );
        let t = Text::neu("L\u{2019}avocat wohnt an der Hauptstraße");
        let treffer = lex.finde(&t);
        assert_eq!(
            treffer.len(),
            1,
            "Strasse ist Teil von Hauptstraße, kein ganzes Wort"
        );
        assert_eq!(
            &t.als_str()[treffer[0].start..treffer[0].ende],
            "L\u{2019}avocat"
        );
        let lex2 =
            Lexikon::aus_eintraegen("t", vec![eintrag("strasse", &[], Teilwort::Ende, false)]);
        let t2 = Text::neu("Hauptstraße 5");
        let tr2 = lex2.finde(&t2);
        assert_eq!(&t2.als_str()[tr2[0].start..tr2[0].ende], "straße");
    }

    #[test]
    fn laengster_gewinnt() {
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag("Störung", &[], Teilwort::Nein, false),
                eintrag("psychische Störung", &[], Teilwort::Nein, false),
            ],
        );
        let t = Text::neu("eine psychische Störung");
        let treffer = lex.finde(&t);
        assert_eq!(treffer.len(), 1);
        assert_eq!(
            &t.als_str()[treffer[0].start..treffer[0].ende],
            "psychische Störung"
        );
    }

    fn gefunden(lex: &Lexikon, s: &str) -> Vec<String> {
        let t = Text::neu(s);
        lex.finde(&t)
            .iter()
            .map(|tr| t.als_str()[tr.start..tr.ende].to_string())
            .collect()
    }

    #[test]
    fn begriff_ueber_weichen_umbruch() {
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag(
                    "psychische Störung",
                    &["psychischen Störung"],
                    Teilwort::Nein,
                    false,
                ),
                eintrag("troubles psychiques", &[], Teilwort::Nein, false),
                eintrag("disturbi psichici", &[], Teilwort::Nein, false),
            ],
        );
        for (s, stelle) in [
            ("Er hat eine psychischen\nStörung.", "psychischen\nStörung"),
            (
                "Er hat eine psychischen \n  Störung.",
                "psychischen \n  Störung",
            ),
            (
                "Il souffre de troubles\npsychiques.",
                "troubles\npsychiques",
            ),
            ("Soffre di disturbi\npsichici.", "disturbi\npsichici"),
            (
                "Er hat eine psychischen\r\nStörung.",
                "psychischen\nStörung",
            ),
            // Tabulatoren und geschützte Leerzeichen am Zeilenende und am Zeilenanfang.
            (
                "Er hat eine psychischen\t\nStörung.",
                "psychischen\t\nStörung",
            ),
            (
                "Er hat eine psychischen \t\u{A0}\n\t Störung.",
                "psychischen \t\u{A0}\n\t Störung",
            ),
        ] {
            assert_eq!(gefunden(&lex, s), [stelle], "{s:?}");
        }
        // Gegenbeispiele: Leerzeile, Satzzeichen, Listenpunkte trennen.
        for s in [
            "Er hat eine psychischen\n\nStörung.",
            "Das ist psychischen.\nStörung folgt.",
            "Liste:\n- psychischen\n- Störung",
            "Il souffre de troubles\n\npsychiques.",
            "Soffre di disturbi.\nPsichici sono altri.",
        ] {
            assert!(gefunden(&lex, s).is_empty(), "{s:?}");
        }
        // `finde_in` kennt keine weichen Umbrüche.
        assert!(lex.finde_in("Er hat eine psychischen\nStörung.").is_empty());
    }

    #[test]
    fn worttrennung_am_zeilenende() {
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag("arbeitsunfähig", &[], Teilwort::Nein, false),
                eintrag("AHV-Nummer", &[], Teilwort::Nein, false),
                eintrag("incapacité", &[], Teilwort::Nein, false),
                eintrag("inabilità", &[], Teilwort::Nein, false),
            ],
        );
        for (s, stelle) in [
            ("Sie ist arbeits-\nunfähig.", "arbeits-\nunfähig"),
            (
                "Sie ist arbeits\u{2010}\n  unfähig.",
                "arbeits\u{2010}\n  unfähig",
            ),
            ("Sie ist arbeits\u{AD}\nunfähig.", "arbeits\u{AD}\nunfähig"),
            ("Die AHV-\nNummer fehlt.", "AHV-\nNummer"),
            ("Une incapa-\ncité de travail.", "incapa-\ncité"),
            ("Una inabi-\nlità al lavoro.", "inabi-\nlità"),
        ] {
            assert_eq!(gefunden(&lex, s), [stelle], "{s:?}");
        }
        // Gegenbeispiele: harter Umbruch, Gedankenstrich, Strich nach Leerzeichen.
        for s in [
            "Sie ist arbeits-\n\nunfähig.",
            "Sie ist arbeits –\nunfähig.",
            "Sie ist arbeits -\nunfähig.",
        ] {
            assert!(gefunden(&lex, s).is_empty(), "{s:?}");
        }
    }

    #[test]
    fn unsichtbare_formatzeichen_im_wort() {
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag("arbeitsunfähig", &[], Teilwort::Nein, false),
                eintrag("ter", &[], Teilwort::Nein, false),
            ],
        );
        for (s, stelle) in [
            ("Sie ist arbeits\u{AD}unfähig.", "arbeits\u{AD}unfähig"),
            ("Sie ist arbeits\u{200B}unfähig.", "arbeits\u{200B}unfähig"),
            (
                "Sie ist ar\u{2060}beits\u{FEFF}unfähig.",
                "ar\u{2060}beits\u{FEFF}unfähig",
            ),
            (
                "Sie ist arbeits\u{200D}unf\u{200C}ähig.",
                "arbeits\u{200D}unf\u{200C}ähig",
            ),
        ] {
            assert_eq!(gefunden(&lex, s), [stelle], "{s:?}");
        }
        // Ein Formatzeichen macht keine Wortgrenze: «ter» in «Mus\u{AD}ter» trifft nicht.
        assert!(gefunden(&lex, "Herr Mus\u{AD}ter").is_empty());
        assert!(gefunden(&lex, "Herr Mus\u{200B}ter").is_empty());
    }

    #[test]
    fn ueberlappung_wie_bisher() {
        // Vergleich mit der früheren, quadratischen Auswahl auf zufälligen Treffern.
        fn bisher(mut roh: Vec<LexTreffer>) -> Vec<LexTreffer> {
            roh.sort_by(|a, b| {
                a.start
                    .cmp(&b.start)
                    .then((b.ende - b.start).cmp(&(a.ende - a.start)))
                    .then(a.eintrag.cmp(&b.eintrag))
            });
            let mut nach_laenge = roh.clone();
            nach_laenge.sort_by(|a, b| {
                (b.ende - b.start)
                    .cmp(&(a.ende - a.start))
                    .then(a.start.cmp(&b.start))
            });
            let mut gewaehlt: Vec<LexTreffer> = Vec::new();
            for t in nach_laenge {
                if gewaehlt
                    .iter()
                    .all(|g| t.ende <= g.start || t.start >= g.ende)
                {
                    gewaehlt.push(t);
                }
            }
            gewaehlt.sort_by_key(|t| (t.start, t.ende));
            gewaehlt
        }
        let mut zustand: u64 = 0x2545_F491_4F6C_DD1D;
        let mut zufall = |n: u64| {
            zustand ^= zustand << 13;
            zustand ^= zustand >> 7;
            zustand ^= zustand << 17;
            (zustand % n) as usize
        };
        for _ in 0..300 {
            let k = zufall(40);
            let roh: Vec<LexTreffer> = (0..k)
                .map(|_| {
                    let start = zufall(60);
                    LexTreffer {
                        start,
                        ende: start + 1 + zufall(8),
                        eintrag: zufall(4),
                    }
                })
                .collect();
            assert_eq!(ohne_ueberlappung(roh.clone()), bisher(roh));
        }
    }

    /// Kleinste Laufzeit aus fünf Läufen, damit ein kurzer Stau der Maschine nicht zählt.
    fn kleinste_zeit(f: impl Fn()) -> std::time::Duration {
        (0..5)
            .map(|_| {
                let t0 = std::time::Instant::now();
                f();
                t0.elapsed()
            })
            .min()
            .unwrap()
    }

    #[test]
    fn viele_treffer_in_fast_linearer_zeit() {
        // Achtmal so viele Treffer dürfen höchstens zwanzigmal so lange brauchen: O(k log k)
        // braucht gut achtmal so lange, die frühere Auswahl quadratisch 64-mal. An jeder
        // Stelle ein Treffer, der gewählt wird, und ein kürzerer, der ihn überlappt.
        let zeit = |k: usize| {
            let roh: Vec<LexTreffer> = (0..k)
                .flat_map(|i| {
                    [
                        LexTreffer {
                            start: 4 * i,
                            ende: 4 * i + 3,
                            eintrag: 0,
                        },
                        LexTreffer {
                            start: 4 * i + 1,
                            ende: 4 * i + 3,
                            eintrag: 1,
                        },
                    ]
                })
                .collect();
            let gewaehlt = ohne_ueberlappung(roh.clone());
            assert_eq!(gewaehlt.len(), k);
            assert!(gewaehlt.iter().all(|t| t.eintrag == 0));
            kleinste_zeit(|| {
                std::hint::black_box(ohne_ueberlappung(roh.clone()));
            })
        };
        zeit(1_000);
        let klein = zeit(2_000);
        let gross = zeit(16_000);
        assert!(
            gross < klein * 20 + std::time::Duration::from_millis(20),
            "{klein:?} für 2000, {gross:?} für 16000"
        );
    }

    #[test]
    fn finden_in_fast_linearer_zeit() {
        // Der ganze Weg (Falten, Automat, Wortgrenzen, Auswahl) mit überlappenden
        // Treffern: achtmal so viel Text höchstens zwanzigmal so lange.
        let lex = Lexikon::aus_eintraegen(
            "t",
            vec![
                eintrag("die", &[], Teilwort::Frei, false),
                eintrag("die die", &[], Teilwort::Frei, false),
                eintrag("ie d", &[], Teilwort::Frei, false),
            ],
        );
        let zeit = |n: usize| {
            let t = Text::neu(&"die ".repeat(n));
            assert_eq!(lex.finde(&t).len(), n / 2);
            kleinste_zeit(|| {
                std::hint::black_box(lex.finde(&t));
            })
        };
        zeit(500);
        let klein = zeit(1_000);
        let gross = zeit(8_000);
        assert!(
            gross < klein * 20 + std::time::Duration::from_millis(20),
            "{klein:?} für 1000, {gross:?} für 8000"
        );
    }

    #[test]
    fn gross_beachtet() {
        let lex = Lexikon::aus_eintraegen("t", vec![eintrag("AG", &[], Teilwort::Nein, true)]);
        assert_eq!(lex.finde(&Text::neu("Muster AG")).len(), 1);
        assert_eq!(lex.finde(&Text::neu("ag")).len(), 0);
    }

    #[test]
    fn pflichtfelder() {
        let mut e = eintrag("x", &[], Teilwort::Nein, false);
        assert!(pruefe_eintrag(&e).is_ok());
        e.fassung = "2026-1-1".into();
        assert!(pruefe_eintrag(&e).is_err());
        e.fassung = "2026-01-01".into();
        e.url = "http://x".into();
        assert!(pruefe_eintrag(&e).is_err());
    }

    #[test]
    fn register_kopf() {
        let tsv = "# titel: T\n# herausgeber: H\n# url: https://x\n# lizenz: L\n# abgerufen: 2026-11-06\n# rohdatei-sha256: ab\n# filter: alle\n# spalten: name\tanzahl\nAnna\t5\n";
        let r = Register::aus_tsv("t.tsv", tsv).unwrap();
        assert_eq!(r.werte("name").collect::<Vec<_>>(), vec!["Anna"]);
        assert!(Register::aus_tsv("t.tsv", "Anna\t5\n").is_err());
    }
}
