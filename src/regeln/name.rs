//! Regel `name`: Namen natürlicher Personen (SPEC 5.2), dazu der Personenbezug einer
//! Verfahrensrolle ohne Namen (Regel `personenbezug`).
//!
//! Gesucht wird nur nach einem Auslöser, denn ein grossgeschriebenes Wort allein sagt im
//! Deutschen nichts (SPEC 5.2):
//!
//! 1. **Vorname** aus dem BFS-Register, gefolgt von einem grossgeschriebenen Wort:
//!    «Hans Muster». Ist der Vorname zugleich ein Wort der Gesetzestexte oder ein Ortsname
//!    («Mai», «Art», «Uri»), muss ein eindeutiger Nachname aus dem Register folgen.
//! 2. **Anrede oder Titel**: «Monsieur Dupont», «Dr. Muster». Nach einer deutschen Anrede
//!    zählt nur ein Nachname aus dem Register; ist er zugleich ein gewöhnliches Wort
//!    («Angst»), darf kein Artikel vor der Anrede stehen («der Frau Angst machen»).
//! 3. **Verfahrensrolle** aus StPO, ZPO, BGG, VwVG, ZGB, MStP und VStrR: «die beschuldigte
//!    Person Meier», «der Beschwerdeführer A. Meier». Folgt kein Name, meldet die Regel einen
//!    Personenbezug. Nach Initialen zählt nur ein Nachname aus dem Register, der kein
//!    Satzwort der Sprache des Auslösers ist («der Beschwerdeführerin A. Daher ist …»).
//! 4. **Liste**: In einer Zelle einer Liste («Müller Andreas, Lager, 12 Tage») oder über zwei
//!    Zellen («Muster;Hans») ein Paar aus Nachname und Vorname des Registers, in beiden
//!    Reihenfolgen; ein Nachname in Versalien vor einem Vornamen («KELLER Sabine») überall.
//!
//! Ausnahmen nach den Anonymisierungsregeln des Bundesgerichts, Art. 4 Abs. 1: Namen der
//! Rechtsvertreter der Parteien (lit. g) werden nicht markiert, aber nur, wenn der Satz die
//! Person als Vertretung zeigt («Verteidigerin …», «vertreten durch RA …», «Me … plaide
//! pour …»); sonst wirkt ein Anwaltstitel wie ein Titel. Ein Possessivpronomen («mon
//! avocat», «son avocat») zeigt die Vertretung nur, wenn der Satz keine Angabe nach Art. 5
//! lit. c DSG enthält. Behörden (lit. a), Gemeinden
//! (lit. b) und Ortsnamen (lit. e) lösen nichts aus: Gleichnamige Vornamen gelten als
//! mehrdeutig, und ein Heiligenname («St. Moritz») sperrt den folgenden Vornamen.
//!
//! Die Register erzeugt `examples/register_namen.rs` aus den Rohdaten des BFS; ihre Spalten
//! `gesetz` und `ort` belegen, warum ein Name mehrdeutig ist.

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::fund::{Art, Beleg, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Lexikon, Register, Sprache, falten};
use crate::normen;
use crate::text::{Text, ist_unsichtbar, ist_wortzeichen};

/// Anreden, Titel und Heiligennamen.
pub const ANREDEN: &str = "name-anreden.toml";
/// Rechtsvertretung: Ausnahme nach Art. 4 Abs. 1 lit. g der Anonymisierungsregeln.
pub const RECHTSVERTRETUNG: &str = "name-rechtsvertretung.toml";
/// Vertretungsmerkmale: woran der Satz einen Anwalt als Vertretung einer Partei zeigt.
pub const VERTRETUNG: &str = "name-vertretung.toml";
/// Funktionen und Ämter nach einer Anrede («Frau Präsidentin»).
pub const FUNKTIONEN: &str = "name-funktionen.toml";
/// Artikel und Possessivpronomen vor einer deutschen Anrede.
pub const BEGLEITER: &str = "name-begleiter.toml";
/// Verfahrensrollen aus StPO, ZPO, BGG, VwVG, ZGB, MStP und VStrR.
pub const ROLLEN: &str = "verfahrensrollen.toml";
/// Wörter, die das Register auch als Nachnamen führt: Grussformeln, Wochentage und
/// Alltagswörter (in Listen mehrdeutig) und Satzwörter (nach Initialen kein Nachname).
pub const ALLTAG: &str = "name-alltag.toml";
/// Der französische Artikel «Le» (zugleich Vorname im Register) und Amts- und
/// Behördenbezeichnungen danach («Le Grand Conseil», «Le Maire»).
pub const AMTSBEZEICHNUNGEN: &str = "name-amtsbezeichnungen.toml";
/// Vornamen (BFS, STATPOP 2025).
pub const VORNAMEN: &str = "register/vornamen.tsv";
/// Nachnamen (BFS, STATPOP 2025).
pub const NACHNAMEN: &str = "register/nachnamen.tsv";

/// Höchstens so viele Teile hat ein Nachname im Register, getrennt durch Leerzeichen oder
/// Apostroph («Auf der Maur», «Matthey-de-l'Endroit»).
const MAX_TEILE: usize = 3;

/// Meldet Namen (`name`) und Verfahrensrollen ohne Namen (`personenbezug`).
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    Analyse::neu(text).durchlauf()
}

/// Was ein Wort aus einer der Wortlisten ist (Feld `merkmal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wortart {
    /// Deutsche Anrede: danach nur ein Nachname aus dem Register.
    AnredeDe,
    /// Französische oder italienische Anrede: danach ein grossgeschriebenes Wort.
    Anrede,
    /// Titel, auch in Ketten: danach ein grossgeschriebenes Wort.
    Titel,
    /// Heiligenname: sperrt den folgenden Vornamen.
    Heilig,
    /// Funktion oder Amt: kein Name, aber Glied der Kette nach einer Anrede.
    Funktion,
    /// Vertretung einer Partei schon dem Wort nach («Verteidigerin»): der Name daneben wird
    /// nicht markiert.
    Rechtsvertretung,
    /// Französischer oder italienischer Anwaltstitel oder -beruf («Me», «avv.»): wie ein
    /// Titel, ausser ein Vertretungsmerkmal zeigt die Vertretung einer Partei.
    Anwalt,
    /// Deutscher Anwaltstitel oder -beruf («Rechtsanwältin», «RA»): wie eine deutsche Anrede,
    /// ausser ein Vertretungsmerkmal zeigt die Vertretung einer Partei.
    AnwaltDe,
    /// Französischer oder italienischer Anwaltsberuf («avocat», «avvocato»): kein Auslöser,
    /// denn danach folgt oft ein Adjektiv oder ein Fachgebiet («un avocat Genevois»). Mit
    /// einem Vertretungsmerkmal nimmt er den Namen daneben aus wie ein Anwaltstitel.
    AnwaltWort,
    /// Rechtsbeistand oder Berater («conseil»): kein Auslöser. Vertretung nur mit einer
    /// Partei danach («conseil de Marie Dupont») oder einem Partizip der Vertretung davor
    /// («assisté de son conseil»), nicht mit einem Possessivpronomen allein («mon conseil»).
    Beratung,
    /// Beauftragter («mandataire»): kein Auslöser. Vertretung nur mit «partie» oder einer
    /// Verfahrensrolle danach («mandataire de la partie»); die Vorsorgebeauftragte nach
    /// Art. 360 ZGB («son mandataire», «le mandataire de Mme Rochat») bleibt gemeldet.
    Beauftragt,
    /// Artikel oder Possessivpronomen vor einer deutschen Anrede.
    Begleiter,
    /// Verfahrensrolle.
    Rolle,
}

impl Wortart {
    fn aus_merkmal(merkmal: &str) -> Option<Wortart> {
        Some(match merkmal {
            "anrede-de" => Wortart::AnredeDe,
            "anrede" => Wortart::Anrede,
            "titel" => Wortart::Titel,
            "heilig" => Wortart::Heilig,
            "funktion" => Wortart::Funktion,
            "rechtsvertretung" => Wortart::Rechtsvertretung,
            "anwalt" => Wortart::Anwalt,
            "anwalt-de" => Wortart::AnwaltDe,
            "anwalt-wort" => Wortart::AnwaltWort,
            "beratung" => Wortart::Beratung,
            "beauftragt" => Wortart::Beauftragt,
            "begleiter" => Wortart::Begleiter,
            "rolle" => Wortart::Rolle,
            _ => return None,
        })
    }

    /// Ein Wort aus der Liste der Rechtsvertretung.
    fn vertretung(self) -> bool {
        matches!(
            self,
            Wortart::Rechtsvertretung
                | Wortart::Anwalt
                | Wortart::AnwaltDe
                | Wortart::AnwaltWort
                | Wortart::Beratung
                | Wortart::Beauftragt
        )
    }

    /// Ein Anwaltstitel oder -beruf: Er leitet den Namen ein, dem ein Verb der Vertretung
    /// folgen kann («Me Jean Dupont plaide pour …»).
    fn anwalt(self) -> bool {
        matches!(
            self,
            Wortart::Anwalt | Wortart::AnwaltDe | Wortart::AnwaltWort
        )
    }
}

/// Merkmal eines Eintrags in `name-vertretung.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Vertretungsmerkmal {
    /// Partizip mit Präposition vor dem Anwaltswort: «vertreten durch», «représenté par».
    Vertreten,
    /// Verb nach dem Namen des Anwalts, vor einer Partei: «vertritt», «plaide pour»,
    /// «rappresenta».
    Vertritt,
    /// Possessivpronomen vor dem Anwaltswort: «Ihr Anwalt», «mon avocat», «son avocat»; es
    /// zeigt die Vertretung nur ohne Angabe nach Art. 5 lit. c DSG im Satz. Nach dem
    /// Anwaltswort verbindet es ihn mit der Partei («Anwalt meiner Mandantin»).
    Besitz,
    /// Präposition oder Artikel vor der Partei: «von», «de la», «della».
    Bezug,
    /// Die Partei selbst: «Partei», «défense», «parte».
    Partei,
}

impl Vertretungsmerkmal {
    fn aus_merkmal(merkmal: &str) -> Option<Vertretungsmerkmal> {
        Some(match merkmal {
            "vertreten" => Vertretungsmerkmal::Vertreten,
            "vertritt" => Vertretungsmerkmal::Vertritt,
            "besitz" => Vertretungsmerkmal::Besitz,
            "bezug" => Vertretungsmerkmal::Bezug,
            "partei" => Vertretungsmerkmal::Partei,
            _ => return None,
        })
    }
}

/// Fundstelle eines Vertretungsmerkmals im Text (Byte-Bereich).
#[derive(Clone, Copy, Debug)]
struct Merkmalstelle {
    start: usize,
    ende: usize,
    merkmal: Vertretungsmerkmal,
}

/// Ein Name im Register.
#[derive(Clone, Copy, Debug)]
struct Registername {
    /// Schreibweise im Register.
    schreibung: &'static str,
    /// Spalte `gesetz` oder `ort` belegt: zugleich ein Wort der Gesetzestexte oder ein
    /// Ortsname.
    mehrdeutig: bool,
    /// Spalte `ort` belegt: zugleich ein Kantons-, Bezirks- oder Gemeindename.
    ort: bool,
}

/// Register und Wortlisten, einmal geladen.
struct Daten {
    /// Schlüssel: [`schluessel`] des Namens.
    vornamen: HashMap<String, Registername>,
    nachnamen: HashMap<String, Registername>,
    vornamen_register: &'static Register,
    /// In der Reihenfolge des Vorrangs bei Überlappung.
    lexika: Vec<&'static Lexikon>,
    /// Vertretungsmerkmale; sie markieren keine Wörter, sondern werden nur neben einem
    /// Anwaltswort gelesen.
    vertretung: &'static Lexikon,
    /// Grussformeln, Wochentage und Alltagswörter aus [`ALLTAG`] (Schlüssel nach
    /// [`schluessel`]): In einer Liste gelten sie als mehrdeutige Nachnamen.
    alltag: HashSet<String>,
    /// Satzwörter aus [`ALLTAG`] je Sprache: nach Initialen kein Nachname.
    satzwoerter: HashSet<(String, Sprache)>,
    /// Artikel, die zugleich Vornamen des Registers sind («Le»), als [`schluessel`].
    artikel: HashSet<String>,
    /// Amts- und Behördenbezeichnungen (merkmal «amt»); gelesen nur nach einem solchen
    /// Artikel.
    aemter: &'static Lexikon,
}

/// Merkmal eines Eintrags in [`ALLTAG`]: Grussformel, Wochentag oder Alltagswort.
const MERKMAL_ALLTAG: &str = "alltag";
/// Merkmal eines Eintrags in [`ALLTAG`]: Wort, das einen Satz beginnen kann (Adverb,
/// Konjunktion, Pronomen, Artikel, Präposition).
const MERKMAL_SATZWORT: &str = "satzwort";

static DATEN: LazyLock<Daten> = LazyLock::new(|| {
    let vornamen = lexikon::register(VORNAMEN);
    let nachnamen = lexikon::register(NACHNAMEN);
    let mut alltag = HashSet::new();
    let mut satzwoerter = HashSet::new();
    for e in &lexikon::lexikon(ALLTAG).eintraege {
        for form in std::iter::once(&e.begriff).chain(&e.formen) {
            match e.merkmal.as_deref() {
                Some(MERKMAL_ALLTAG) => {
                    alltag.insert(schluessel(form));
                }
                Some(MERKMAL_SATZWORT) => {
                    satzwoerter.insert((schluessel(form), e.sprache));
                }
                _ => {}
            }
        }
    }
    let aemter = lexikon::lexikon(AMTSBEZEICHNUNGEN);
    let artikel = aemter
        .eintraege
        .iter()
        .filter(|e| e.merkmal.as_deref() == Some("artikel"))
        .flat_map(|e| std::iter::once(&e.begriff).chain(&e.formen))
        .map(|f| schluessel(f))
        .collect();
    Daten {
        vornamen: karte(vornamen, "vorname"),
        nachnamen: karte(nachnamen, "nachname"),
        vornamen_register: vornamen,
        // Zuerst die Ausnahme, zuletzt die Begleiter.
        lexika: [RECHTSVERTRETUNG, ANREDEN, ROLLEN, FUNKTIONEN, BEGLEITER]
            .into_iter()
            .map(lexikon::lexikon)
            .collect(),
        vertretung: lexikon::lexikon(VERTRETUNG),
        alltag,
        satzwoerter,
        artikel,
        aemter,
    }
});

/// Vergleichsform: klein, typografische Varianten vereinheitlicht (wie die Wortlisten).
fn schluessel(s: &str) -> String {
    falten(s, true).text
}

/// Namen eines Registers nach [`schluessel`]. Schreibweisen, die nur in Gross- und
/// Kleinschreibung abweichen («d'Amato», «D'Amato»), fallen zusammen; mehrdeutig ist der
/// Name dann, wenn es eine von ihnen ist.
fn karte(register: &'static Register, spalte: &str) -> HashMap<String, Registername> {
    let index = |name: &str| {
        register
            .spalte(name)
            .unwrap_or_else(|| panic!("lexika/{}: Spalte «{name}» fehlt", register.datei))
    };
    let (name, gesetz, ort) = (index(spalte), index("gesetz"), index("ort"));
    let mut karte: HashMap<String, Registername> = HashMap::with_capacity(register.zeilen.len());
    for zeile in &register.zeilen {
        let ist_ort = zeile[ort] != "-";
        let mehrdeutig = zeile[gesetz] != "-" || ist_ort;
        karte
            .entry(schluessel(&zeile[name]))
            .and_modify(|r| {
                r.mehrdeutig |= mehrdeutig;
                r.ort |= ist_ort;
            })
            .or_insert(Registername {
                schreibung: &zeile[name],
                mehrdeutig,
                ort: ist_ort,
            });
    }
    karte
}

/// Steht `wort` im Register? Dann: mehrdeutig oder nicht. Ein Doppelname mit Bindestrich
/// («Anna-Lena», «Muster-Meier») zählt auch, wenn jeder Teil im Register steht.
fn im_register(karte: &HashMap<String, Registername>, wort: &str) -> Option<bool> {
    if let Some(r) = karte.get(&schluessel(wort)) {
        return Some(r.mehrdeutig);
    }
    if !wort.contains(ist_bindestrich) {
        return None;
    }
    let mut mehrdeutig = false;
    for teil in wort.split(ist_bindestrich) {
        mehrdeutig |= karte.get(&schluessel(teil))?.mehrdeutig;
    }
    Some(mehrdeutig)
}

/// Beleg aus dem Kopf eines Registers.
fn register_beleg(register: &Register, begriff: &str) -> Beleg {
    let kopf = |k: &str| register.kopf(k).unwrap_or_default().to_string();
    Beleg {
        datei: register.datei.clone(),
        begriff: begriff.to_string(),
        quelle: format!("{}; {}", kopf("titel"), kopf("herausgeber")),
        fassung: kopf("veroeffentlicht"),
        url: kopf("url"),
    }
}

/// Leerraum innerhalb einer Zeile. Ein Zeilenende trennt Namen, ausser es ist ein weicher
/// Umbruch (siehe [`Analyse::luecke`]).
fn ist_leerraum(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\u{2007}'
    )
}

fn ist_bindestrich(c: char) -> bool {
    matches!(c, '-' | '\u{2010}' | '\u{2011}')
}

fn ist_apostroph(c: char) -> bool {
    matches!(c, '\'' | '\u{2019}' | '\u{02BC}')
}

/// Genau ein Apostroph.
fn ein_apostroph(z: &str) -> bool {
    let mut zeichen = z.chars();
    matches!((zeichen.next(), zeichen.next()), (Some(a), None) if ist_apostroph(a))
}

/// Ein Wort: Buchstaben und Ziffern, auch mit Bindestrich verbunden («Hans-Peter»). Ein
/// Apostroph trennt («d'Angelo» sind zwei Wörter), damit «d'Erika Beispiel» mit dem Vornamen
/// beginnt; mehrteilige Nachnamen setzt [`Analyse::nachname_ab`] wieder zusammen.
/// Unsichtbare Formatzeichen zwischen zwei Wortzeichen gehören zum Wort («Bei\u{AD}spiel»
/// ist ein Wort), wie sie auch [`falten`] übergeht.
#[derive(Clone, Copy, Debug)]
struct Wort {
    start: usize,
    ende: usize,
}

fn woerter(s: &str) -> Vec<Wort> {
    let zeichen: Vec<(usize, char)> = s.char_indices().collect();
    let mut woerter = Vec::new();
    let mut i = 0;
    while i < zeichen.len() {
        if !ist_wortzeichen(zeichen[i].1) {
            i += 1;
            continue;
        }
        let start = zeichen[i].0;
        let mut j = i;
        loop {
            while j < zeichen.len() {
                if ist_wortzeichen(zeichen[j].1) {
                    j += 1;
                    continue;
                }
                // Unsichtbare Formatzeichen gehören zum Wort, wenn danach ein Wortzeichen
                // folgt. Jede Folge wird nur einmal gelesen: Die Zeit bleibt linear.
                let k = j + zeichen[j..]
                    .iter()
                    .take_while(|(_, c)| ist_unsichtbar(*c))
                    .count();
                if k > j && zeichen.get(k).is_some_and(|(_, c)| ist_wortzeichen(*c)) {
                    j = k;
                } else {
                    break;
                }
            }
            let bindestrich = j + 1 < zeichen.len()
                && ist_bindestrich(zeichen[j].1)
                && ist_wortzeichen(zeichen[j + 1].1);
            if !bindestrich {
                break;
            }
            j += 1;
        }
        let ende = zeichen.get(j).map_or(s.len(), |z| z.0);
        woerter.push(Wort { start, ende });
        i = j;
    }
    woerter
}

/// Schreibung eines Wortes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// Grossbuchstabe am Anfang, auch Kleinbuchstaben: «Muster», «McDonald».
    Gross,
    /// Nur Grossbuchstaben, mindestens zwei: «MUSTER».
    Versal,
    /// Nur Kleinbuchstaben, mindestens zwei: «von», «alten».
    Klein,
    /// Ziffern, ein einzelner Buchstabe, Kleinbuchstabe vor Grossbuchstabe.
    Sonst,
}

/// Fundstelle eines Wortlisteneintrags, auf Wörter abgebildet.
#[derive(Clone, Copy)]
struct Marke {
    art: Wortart,
    erstes: usize,
    letztes: usize,
    start: usize,
    /// Byte-Ende, mit Punkt («Dr.») oder Apostroph («sant'»).
    ende: usize,
    lex: &'static Lexikon,
    eintrag: usize,
}

/// Ein gefundener Name.
#[derive(Clone, Debug)]
struct Name {
    start: usize,
    ende: usize,
    /// Index des ersten und des letzten Wortes.
    erstes: usize,
    letztes: usize,
    beleg: Beleg,
}

/// Wo eine Zelle einer Liste beginnt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Zellenanfang {
    /// Am Zeilenanfang. Hier beginnt auch ein Satz («Lieber Peter, ich …»): Die Zeile muss
    /// nach dem Namen mit einem Trenner weitergehen.
    Zeile,
    /// Nach einem Aufzählungszeichen («- », «• », «1. »), einem Trenner «,» «;» Tab «|»
    /// oder einem Doppelpunkt («Patient: …»).
    Liste,
}

/// Trenner zwischen den Zellen einer Liste: «,» «;» Tab «|».
fn ist_trenner(c: char) -> bool {
    matches!(c, ',' | ';' | '\t' | '|')
}

/// Leerzeichen innerhalb einer Zelle; der Tabulator trennt Zellen.
fn ist_zellenleerraum(c: char) -> bool {
    ist_leerraum(c) && c != '\t'
}

fn nur_zellenleerraum(z: &str) -> bool {
    !z.is_empty() && z.chars().all(ist_zellenleerraum)
}

/// Eine Zelle, die ein Satz ist: Sie endet auf «?» oder «!», oder auf «.» nach mindestens
/// drei Wörtern («Ich bin seit Montag arbeitsunfähig.»). «Seestr.» und «12 Tage» sind keine
/// Sätze.
fn ist_satzzelle(zelle: &str) -> bool {
    let zelle = zelle.trim_end_matches(ist_leerraum);
    let woerter = zelle
        .split(ist_leerraum)
        .filter(|w| w.chars().any(char::is_alphabetic))
        .count();
    zelle.ends_with(['?', '!']) || (zelle.ends_with('.') && woerter >= 3)
}

/// Aufzählungszeichen am Zeilenanfang.
fn ist_aufzaehlung(z: &str) -> bool {
    matches!(z, "-" | "*" | "•" | "–" | "·")
        || z.strip_suffix(['.', ')'])
            .is_some_and(|n| !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit()))
}

/// Ergebnis nach einer Anrede.
enum Pfad {
    Name(Name),
    /// Rechtsvertretung in der Kette: hier weitersuchen, nichts melden.
    Weiter(usize),
    Nichts,
}

/// Ein Text, in Wörter zerlegt und mit den Wortlisten markiert.
struct Analyse<'a> {
    text: &'a Text,
    s: &'a str,
    /// Der Text mit Satzgrenzen und weichen Zeilenumbrüchen.
    volltext: &'a Text,
    w: Vec<Wort>,
    /// Je Wort: Es beginnt einen Satz nach [`Text::saetze`]. Ein weicher Zeilenumbruch
    /// beginnt keinen Satz: «J'ai rencontré hier⏎Le Roux Pierre» (Prüferproblem
    /// c-erkennung-p3).
    satzanfang: Vec<bool>,
    /// Amts- und Behördenbezeichnungen ([`AMTSBEZEICHNUNGEN`]): Byte-Anfang → Byte-Ende und
    /// Eintrag.
    amt_ab: HashMap<usize, (usize, usize)>,
    marken: Vec<Marke>,
    /// Index der Marke, die bei diesem Wort beginnt.
    marke_ab: Vec<Option<usize>>,
    /// Index der Marke, zu der dieses Wort gehört.
    marke_von: Vec<Option<usize>>,
    /// Das Wort gehört zu einer Marke.
    markiert: Vec<bool>,
    /// Das Wort folgt auf einen Heiligennamen und beginnt keinen Vornamen («St. Moritz»).
    gesperrt: Vec<bool>,
    /// Je Marke: Sie bezeichnet die Vertretung einer Partei (Rechtsvertretung oder
    /// Anwaltswort mit Vertretungsmerkmal).
    vertritt: Vec<bool>,
    /// Je Wort: Hier endet eine Bezeichnung der Vertretung («votre avocate», «le conseil de
    /// la partie»); ein Name nach Komma steht in Apposition dazu.
    vertretung_endet: Vec<bool>,
    /// Vertretungsmerkmale nach Anfang und nach Ende (Byte).
    merkmal_ab: HashMap<usize, Merkmalstelle>,
    merkmal_bis: HashMap<usize, Merkmalstelle>,
    /// Anfänge der Angaben nach Art. 5 lit. c DSG (Regeln art5c), erst bei Bedarf gesucht.
    inhalt: OnceCell<Vec<usize>>,
    daten: &'static Daten,
}

impl<'a> Analyse<'a> {
    fn neu(text: &'a Text) -> Analyse<'a> {
        let s = text.als_str();
        let w = woerter(s);
        let n = w.len();
        let daten: &'static Daten = &DATEN;
        let mut satzanfang = vec![false; n];
        for satz in text.saetze() {
            let k = w.partition_point(|x| x.start < satz.start);
            if w.get(k).is_some_and(|x| x.start < satz.end) {
                satzanfang[k] = true;
            }
        }
        let amt_ab = daten
            .aemter
            .finde(text)
            .into_iter()
            .filter(|t| daten.aemter.eintraege[t.eintrag].merkmal.as_deref() == Some("amt"))
            .map(|t| (t.start, (t.ende, t.eintrag)))
            .collect();
        // Über weiche Zeilenumbrüche hinweg («vertreten⏎durch»), siehe `Lexikon::finde`.
        let stellen: Vec<Merkmalstelle> = daten
            .vertretung
            .finde(text)
            .into_iter()
            .filter_map(|t| {
                let merkmal = daten.vertretung.eintraege[t.eintrag]
                    .merkmal
                    .as_deref()
                    .and_then(Vertretungsmerkmal::aus_merkmal)?;
                Some(Merkmalstelle {
                    start: t.start,
                    ende: t.ende,
                    merkmal,
                })
            })
            .collect();
        let mut a = Analyse {
            text,
            s,
            volltext: text,
            w,
            satzanfang,
            amt_ab,
            marken: Vec::new(),
            marke_ab: vec![None; n],
            marke_von: vec![None; n],
            markiert: vec![false; n],
            gesperrt: vec![false; n],
            vertritt: Vec::new(),
            vertretung_endet: vec![false; n],
            merkmal_ab: stellen.iter().map(|m| (m.start, *m)).collect(),
            merkmal_bis: stellen.iter().map(|m| (m.ende, *m)).collect(),
            inhalt: OnceCell::new(),
            daten,
        };
        a.marken_setzen();
        a.vertretung_bestimmen();
        a
    }

    /// Markiert die Fundstellen der Wortlisten. Überlappen sich zwei, gilt die Liste mit
    /// Vorrang (Reihenfolge in [`Daten::lexika`]). Ein Eintrag aus mehreren Wörtern trifft
    /// auch über einen weichen Zeilenumbruch («beschuldigte⏎Person»).
    fn marken_setzen(&mut self) {
        let daten = self.daten;
        for &lex in &daten.lexika {
            for t in lex.finde(self.volltext) {
                let merkmal = lex.eintraege[t.eintrag].merkmal.as_deref();
                let Some(art) = merkmal.and_then(Wortart::aus_merkmal) else {
                    continue;
                };
                let Some((erstes, letztes)) = self.woerter_von(t.start, t.ende) else {
                    continue;
                };
                if self.markiert[erstes..=letztes].contains(&true) {
                    continue;
                }
                self.marke_ab[erstes] = Some(self.marken.len());
                self.marke_von[erstes..=letztes].fill(Some(self.marken.len()));
                self.markiert[erstes..=letztes].fill(true);
                self.marken.push(Marke {
                    art,
                    erstes,
                    letztes,
                    start: t.start,
                    ende: t.ende,
                    lex,
                    eintrag: t.eintrag,
                });
            }
        }
        let gesperrt: Vec<usize> = self
            .marken
            .iter()
            .filter(|m| m.art == Wortart::Heilig)
            .filter_map(|m| self.nach_marke(m))
            .collect();
        for k in gesperrt {
            self.gesperrt[k] = true;
        }
    }

    /// Bestimmt je Wortlisten-Marke, ob sie die Vertretung einer Partei bezeichnet (Art. 4
    /// Abs. 1 lit. g der Anonymisierungsregeln). Eine Rechtsvertretung («Verteidigerin»,
    /// «patrocinatore») tut es dem Wort nach; ein Anwaltswort («Me», «avv.», «RA», «avocat»)
    /// nur mit einem Vertretungsmerkmal:
    ///
    /// 1. ein Partizip der Vertretung davor, auch vor einer Anrede, einem Artikel oder einem
    ///    Possessivpronomen: «vertreten durch RA», «représentée par Me», «rappresentato
    ///    dall'avv.», «vertreten durch meinen Anwalt»;
    /// 2. eine Apposition nach Komma zu einer Vertretung oder ein Doppelpunkt nach ihr:
    ///    «votre avocate, Me …», «Avocate de la partie adverse: Me …»;
    /// 3. eine Partei danach: «avocat de la partie adverse», «Anwalt der Gegenpartei»,
    ///    «avvocato della controparte», «Anwalt von Hans Muster»;
    /// 4. ein Verb der Vertretung danach, gefolgt von einer Partei: «Hans Muster,
    ///    Rechtsanwalt, vertritt die Klägerin»;
    /// 5. ein Possessivpronomen davor («mon avocat», «Ihr Anwalt», «il suo avvocato»), aber
    ///    nur, wenn der Satz keine Angabe nach Art. 5 lit. c DSG enthält: Sonst ist die
    ///    Person eher selbst betroffen («Mon avocat, Me Jean Dupont, est en incapacité de
    ///    travail», «Jean Dupont, son avocat, est en incapacité de travail»).
    ///
    /// «conseil» zählt nur mit 1 oder 3, «mandataire» nur mit «partie» oder einer
    /// Verfahrensrolle danach. In Textreihenfolge, damit eine Apposition die Vertretung
    /// davor schon kennt.
    fn vertretung_bestimmen(&mut self) {
        self.vertritt = vec![false; self.marken.len()];
        let mut reihe: Vec<usize> = (0..self.marken.len())
            .filter(|&q| self.marken[q].art.vertretung())
            .collect();
        reihe.sort_by_key(|&q| self.marken[q].erstes);
        for q in reihe {
            let m = self.marken[q];
            let kette = self.kette_zurueck(m.erstes);
            let partei = self.partei_danach(&m);
            let vertreten = kette.iter().any(|&c| self.vertreten_vor(c));
            let vertritt = match m.art {
                Wortart::Rechtsvertretung => true,
                Wortart::Beauftragt => partei.is_some_and(|(_, rolle)| rolle),
                Wortart::Beratung => partei.is_some() || vertreten,
                _ => {
                    partei.is_some()
                        || vertreten
                        || self.verb_danach(m.ende)
                        || kette.iter().any(|&c| self.apposition_vor(c))
                        || (kette.iter().any(|&c| self.besitz_davor(c))
                            && !self.inhalt_im_satz(m.start))
                }
            };
            if !vertritt {
                continue;
            }
            self.vertritt[q] = true;
            self.vertretung_endet[m.letztes] = true;
            if let Some((e, true)) = partei {
                self.vertretung_endet[e] = true;
            }
        }
    }

    /// Steht im Satz um Byte `byte` eine Angabe nach Art. 5 lit. c DSG (Regeln art5c)?
    fn inhalt_im_satz(&self, byte: usize) -> bool {
        self.inhalt
            .get_or_init(|| {
                crate::regeln::art5c::erkenne(self.text)
                    .into_iter()
                    .map(|t| t.start)
                    .collect()
            })
            .iter()
            .any(|&start| self.text.im_selben_satz(start, byte))
    }

    /// Das Vertretungsmerkmal, das bei Wort `k` beginnt.
    fn merkmal_an(&self, k: usize) -> Option<Merkmalstelle> {
        self.merkmal_ab.get(&self.w[k].start).copied()
    }

    /// Ein Verbindungswort zwischen Anwaltswort und Partei: Präposition oder Artikel, klein
    /// geschrieben («de», «della», nicht «La» in «Me La Rosa»), oder Possessivpronomen.
    fn verbindung_an(&self, k: usize) -> Option<Merkmalstelle> {
        let st = self.merkmal_an(k)?;
        let klein = self.s[st.start..]
            .chars()
            .next()
            .is_some_and(char::is_lowercase);
        match st.merkmal {
            Vertretungsmerkmal::Bezug if klein => Some(st),
            Vertretungsmerkmal::Besitz => Some(st),
            _ => None,
        }
    }

    /// Das Wort nach einem Vertretungsmerkmal.
    fn nach_stelle(&self, st: &Merkmalstelle) -> Option<usize> {
        let (_, letztes) = self.woerter_von(st.start, st.ende)?;
        self.folgt_ab(letztes, st.ende)
    }

    /// Folgt auf die Marke über ein oder zwei Verbindungswörter eine Partei? Siehe
    /// [`Analyse::partei_ab`].
    fn partei_danach(&self, m: &Marke) -> Option<(usize, bool)> {
        self.partei_ab(self.nach_marke(m)?, 1)
    }

    /// Steht ab Wort `x`, nach mindestens `mindestens` und höchstens zwei Verbindungswörtern,
    /// eine Partei? Eine Verfahrensrolle, das Wort «Partei» («partie», «parte»), eine Anrede,
    /// ein Vorname mit Nachnamen oder nach einem Possessivpronomen jedes Wort («Anwalt
    /// meiner Mandantin»). Gibt das letzte Wort der Partei zurück und ob sie eine Rolle oder
    /// «Partei» ist.
    fn partei_ab(&self, x: usize, mindestens: usize) -> Option<(usize, bool)> {
        let mut x = x;
        let mut verbunden = 0;
        let mut besitz = false;
        while verbunden < 2 {
            if let Some(st) = self.verbindung_an(x) {
                besitz |= st.merkmal == Vertretungsmerkmal::Besitz;
                x = self.nach_stelle(&st)?;
            } else if let Some(q) =
                self.marke_ab[x].filter(|&q| self.marken[q].art == Wortart::Begleiter)
            {
                x = self.nach_marke(&self.marken[q])?;
            } else {
                break;
            }
            verbunden += 1;
        }
        if verbunden < mindestens {
            return None;
        }
        if let Some(q) = self.marke_ab[x] {
            let p = &self.marken[q];
            return match p.art {
                Wortart::Rolle => Some((p.letztes, true)),
                Wortart::Anrede | Wortart::AnredeDe | Wortart::Titel => Some((x, false)),
                _ if besitz => Some((p.letztes, false)),
                _ => None,
            };
        }
        if let Some(st) = self
            .merkmal_an(x)
            .filter(|st| st.merkmal == Vertretungsmerkmal::Partei)
        {
            let (_, letztes) = self.woerter_von(st.start, st.ende)?;
            return Some((letztes, true));
        }
        let mit_nachname =
            |y: usize| matches!(self.form(y), Form::Gross | Form::Versal) || self.ist_initiale(y);
        let vorname = !self.markiert[x]
            && self.form(x) == Form::Gross
            && self.vorname(x).is_some()
            && self.folgt(x).is_some_and(mit_nachname);
        (besitz || vorname).then_some((x, false))
    }

    /// Ist `c` bei Byte `i` Leerraum zwischen zwei Wörtern: Leerraum der Zeile, ein weicher
    /// Zeilenumbruch oder ein unsichtbares Formatzeichen?
    fn leer_bei(&self, i: usize, c: char) -> bool {
        ist_leerraum(c) || ist_unsichtbar(c) || (c == '\n' && self.volltext.weicher_umbruch(i))
    }

    /// Steht zwischen den Bytes `start` und `ende` nur Leerraum, darunter mindestens ein
    /// Leerzeichen oder ein weicher Zeilenumbruch? Über einen weichen Umbruch reicht ein Name
    /// («Erika⏎Beispiel»); eine Leerzeile oder ein Listenpunkt trennt (siehe
    /// [`Text::weicher_umbruch`]).
    fn luecke(&self, start: usize, ende: usize) -> bool {
        let z = &self.s[start..ende];
        z.char_indices().all(|(i, c)| self.leer_bei(start + i, c))
            && z.chars().any(|c| !ist_unsichtbar(c))
    }

    /// Erstes Byte ab `ab`, das kein Leerraum nach [`Analyse::leer_bei`] ist.
    fn nach_leerraum(&self, ab: usize) -> usize {
        self.s[ab..]
            .char_indices()
            .find(|&(i, c)| !self.leer_bei(ab + i, c))
            .map_or(self.s.len(), |(i, _)| ab + i)
    }

    /// Byte nach dem letzten Zeichen vor `bis`, das kein Leerraum nach
    /// [`Analyse::leer_bei`] ist.
    fn vor_leerraum(&self, bis: usize) -> usize {
        self.s[..bis]
            .char_indices()
            .rev()
            .find(|&(i, c)| !self.leer_bei(i, c))
            .map_or(0, |(i, c)| i + c.len_utf8())
    }

    /// Was zwischen den Bytes `start` und `ende` steht, ohne Leerraum am Rand.
    fn kern(&self, start: usize, ende: usize) -> &'a str {
        let a = self.nach_leerraum(start).min(ende);
        let e = self.vor_leerraum(ende).max(a);
        &self.s[a..e]
    }

    /// Folgt ab Byte `ende` ein Verb der Vertretung, auch nach Komma oder Klammer, und nach
    /// ihm eine Partei? «Me Jean Dupont plaide pour Marie Dupont», «Hans Muster,
    /// Rechtsanwalt, vertritt die Klägerin». Im übertragenen Gebrauch folgt keine Partei:
    /// «vertritt die Auffassung», «représente un danger», «rappresenta un rischio».
    fn verb_danach(&self, ende: usize) -> bool {
        let a = self.nach_leerraum(ende);
        let b = if self.s[a..].starts_with([',', ')']) {
            a + 1
        } else {
            a
        };
        let c = self.nach_leerraum(b);
        if c == ende {
            return false;
        }
        self.merkmal_ab
            .get(&c)
            .filter(|st| st.merkmal == Vertretungsmerkmal::Vertritt)
            .and_then(|st| self.nach_stelle(st))
            .is_some_and(|x| self.partei_ab(x, 0).is_some())
    }

    /// Anfänge der Kette vor Wort `c`, von `c` an rückwärts: Anreden, Titel, Anwaltswörter
    /// und Abkürzungen («Frau Rechtsanwältin», «Herrn Dr. iur.», «votre avocate Me»), ein
    /// elidierter Artikel («l'avv.»).
    fn kette_zurueck(&self, c: usize) -> Vec<usize> {
        let mut anfaenge = vec![c];
        let mut c = c;
        while anfaenge.len() < 6 {
            let Some(p) = c.checked_sub(1) else { break };
            let neu = if let Some(q) = self.marke_von[p] {
                let k = &self.marken[q];
                let glied = k.art.anwalt()
                    || matches!(k.art, Wortart::Anrede | Wortart::AnredeDe | Wortart::Titel);
                (glied && k.letztes == p && self.nach_marke(k) == Some(c)).then_some(k.erstes)
            } else if self.ist_kleine_abkuerzung(p) {
                (self.folgt_mit_punkt(p) == Some(c)).then_some(p)
            } else if self.ist_elision(p) {
                Some(p)
            } else {
                None
            };
            let Some(neu) = neu else { break };
            anfaenge.push(neu);
            c = neu;
        }
        anfaenge
    }

    /// Steht unmittelbar vor Wort `c` ein Possessivpronomen? «Ihr Anwalt», «mon avocat».
    fn besitz_davor(&self, c: usize) -> bool {
        let Some(p) = c.checked_sub(1) else {
            return false;
        };
        self.luecke(self.w[p].ende, self.w[c].start)
            && self.merkmal_an(p).is_some_and(|st| {
                st.merkmal == Vertretungsmerkmal::Besitz && st.ende == self.w[p].ende
            })
    }

    /// Endet unmittelbar vor Wort `c` ein Partizip der Vertretung? «vertreten durch RA …»,
    /// «rappresentato dall'avv. …».
    fn vertreten_endet_vor(&self, c: usize) -> bool {
        let start = self.w[c].start;
        let vor = self.vor_leerraum(start);
        let vertreten = |e: usize| {
            self.merkmal_bis
                .get(&e)
                .is_some_and(|st| st.merkmal == Vertretungsmerkmal::Vertreten)
        };
        vertreten(start) || (vor < start && vertreten(vor))
    }

    /// Wie [`Analyse::vertreten_endet_vor`], auch über einen Artikel oder ein
    /// Possessivpronomen vor Wort `c`: «vertreten durch meinen Anwalt», «représentée par
    /// l'avocate», «assisté de son conseil».
    fn vertreten_vor(&self, c: usize) -> bool {
        if self.vertreten_endet_vor(c) {
            return true;
        }
        let Some(p) = c.checked_sub(1) else {
            return false;
        };
        let zwischen = &self.s[self.w[p].ende..self.w[c].start];
        let begleiter = self.marke_ab[p].is_some_and(|q| {
            self.marken[q].art == Wortart::Begleiter && self.marken[q].letztes == p
        });
        let verbindung = self
            .verbindung_an(p)
            .is_some_and(|st| st.ende == self.w[c].start || st.ende == self.w[p].ende);
        (begleiter || verbindung)
            && (self.luecke(self.w[p].ende, self.w[c].start) || ein_apostroph(zwischen))
            && self.vertreten_endet_vor(p)
    }

    /// Steht Wort `c` nach einem Komma in Apposition zu einer Bezeichnung der Vertretung oder
    /// nach einem Doppelpunkt hinter ihr? «votre avocate, Me …», «le conseil de la partie,
    /// Jean Dupont», «Avocate de la partie adverse: Me …».
    fn apposition_vor(&self, c: usize) -> bool {
        let Some(p) = c.checked_sub(1) else {
            return false;
        };
        self.vertretung_endet[p] && matches!(self.kern(self.w[p].ende, self.w[c].start), "," | ":")
    }

    /// Die Wörter von `start` bis `ende`. Beginnt oder endet die Fundstelle mitten in einem
    /// Wort («Saint» in «Saint-Maurice»), gilt sie nicht.
    fn woerter_von(&self, start: usize, ende: usize) -> Option<(usize, usize)> {
        let erstes = self.w.binary_search_by_key(&start, |w| w.start).ok()?;
        let letztes = self.w.partition_point(|w| w.start < ende).checked_sub(1)?;
        let rest = self.s.get(self.w[letztes].ende..ende)?;
        let passt = rest.is_empty() || rest == "." || ein_apostroph(rest);
        (letztes >= erstes && passt).then_some((erstes, letztes))
    }

    fn text(&self, k: usize) -> &'a str {
        &self.s[self.w[k].start..self.w[k].ende]
    }

    fn form(&self, k: usize) -> Form {
        let mut buchstaben = 0;
        let mut klein = 0;
        let mut gross_am_anfang = None;
        for c in self.text(k).chars().filter(|&c| !ist_unsichtbar(c)) {
            if c.is_alphabetic() {
                buchstaben += 1;
                klein += usize::from(c.is_lowercase());
                if gross_am_anfang.is_none() {
                    gross_am_anfang = Some(c.is_uppercase());
                }
            } else if !ist_bindestrich(c) {
                return Form::Sonst;
            }
        }
        match gross_am_anfang {
            _ if buchstaben < 2 => Form::Sonst,
            Some(true) if klein == 0 => Form::Versal,
            Some(true) => Form::Gross,
            Some(false) if klein == buchstaben => Form::Klein,
            _ => Form::Sonst,
        }
    }

    /// Initiale: ein Grossbuchstabe mit Punkt («M.»).
    fn ist_initiale(&self, k: usize) -> bool {
        let mut zeichen = self.text(k).chars();
        let ein_grossbuchstabe =
            matches!((zeichen.next(), zeichen.next()), (Some(b), None) if b.is_uppercase());
        ein_grossbuchstabe && self.s[self.w[k].ende..].starts_with('.')
    }

    /// Kleingeschriebene Abkürzung in einem Titel: «Dr. med.», «lic. iur.», «h.c.».
    fn ist_kleine_abkuerzung(&self, k: usize) -> bool {
        let t = self.text(k);
        t.chars().count() <= 5
            && t.chars().all(|c| c.is_alphabetic() && c.is_lowercase())
            && self.s[self.w[k].ende..].starts_with('.')
    }

    /// Elidierter Artikel oder Namenspartikel vor einem Apostroph: «l'», «d'», «dell'», «D'».
    fn ist_elision(&self, k: usize) -> bool {
        let t = self.text(k);
        let anzahl = t.chars().count();
        let form = t.chars().all(char::is_alphabetic)
            && (1..=4).contains(&anzahl)
            && (anzahl == 1 || t.chars().all(char::is_lowercase));
        form && self
            .w
            .get(k + 1)
            .is_some_and(|n| ein_apostroph(&self.s[self.w[k].ende..n.start]))
    }

    /// Name aus Partikel und Apostroph («d'Angelo»), auch wenn er nicht im Register steht:
    /// Index des letzten Wortes.
    fn elisionsname(&self, k: usize) -> Option<usize> {
        let ok = self.ist_elision(k) && self.form(k + 1) == Form::Gross && !self.markiert[k + 1];
        ok.then_some(k + 1)
    }

    /// Das Wort nach Wort `k`, wenn ab Byte `ende` bis dorthin nur Leerraum steht, auch ein
    /// weicher Zeilenumbruch ([`Analyse::luecke`]). Endet `ende` auf Punkt oder Apostroph
    /// («Dr.Muster», «sant'Antonio»), darf nichts dazwischen stehen.
    fn folgt_ab(&self, k: usize, ende: usize) -> Option<usize> {
        let n = self.w.get(k + 1)?;
        let z = self.s.get(ende..n.start)?;
        let davor = self.s[..ende].chars().next_back();
        let angrenzend = self.luecke(ende, n.start)
            || (z.is_empty() && davor.is_some_and(|c| c == '.' || ist_apostroph(c)));
        angrenzend.then_some(k + 1)
    }

    /// Das nächste Wort, wenn nur Leerraum dazwischen steht.
    fn folgt(&self, k: usize) -> Option<usize> {
        self.folgt_ab(k, self.w[k].ende)
    }

    /// Das nächste Wort nach einer Initiale oder Abkürzung mit Punkt.
    fn folgt_mit_punkt(&self, k: usize) -> Option<usize> {
        let ende = self.w[k].ende;
        if self.s[ende..].starts_with('.') {
            self.folgt_ab(k, ende + 1)
        } else {
            None
        }
    }

    fn nach_marke(&self, m: &Marke) -> Option<usize> {
        self.folgt_ab(m.letztes, m.ende)
    }

    /// Darf Wort `k` Teil eines Namens sein? Nicht, wenn es zu einer Wortliste gehört, ausser
    /// als Funktion nach einem Vornamen («Hans Richter»).
    fn frei(&self, k: usize) -> bool {
        !self.markiert[k]
            || self.marke_ab[k].is_some_and(|m| self.marken[m].art == Wortart::Funktion)
    }

    /// Vorname aus dem Register an Wort `k`: mehrdeutig oder nicht.
    fn vorname(&self, k: usize) -> Option<bool> {
        im_register(&self.daten.vornamen, self.text(k))
    }

    /// Längster Nachname aus dem Register ab Wort `k`, auch mehrteilig («da Silva», «von
    /// Arx», «d'Angelo»): Index des letzten Wortes und ob er mehrdeutig ist. Das letzte Wort
    /// beginnt mit einem Grossbuchstaben.
    fn nachname_ab(&self, k: usize) -> Option<(usize, bool)> {
        let mut schl = String::new();
        let mut bester = None;
        let mut j = k;
        loop {
            let t = self.text(j);
            if !t
                .chars()
                .all(|c| c.is_alphabetic() || ist_bindestrich(c) || ist_unsichtbar(c))
            {
                break;
            }
            schl.push_str(&schluessel(t));
            if matches!(self.form(j), Form::Gross | Form::Versal) {
                let gefunden = if j == k {
                    im_register(&self.daten.nachnamen, t)
                } else {
                    self.daten.nachnamen.get(&schl).map(|r| r.mehrdeutig)
                };
                if let Some(mehrdeutig) = gefunden {
                    bester = Some((j, mehrdeutig));
                }
            }
            if j + 1 - k >= MAX_TEILE {
                break;
            }
            let Some(n) = self.w.get(j + 1) else { break };
            let z = &self.s[self.w[j].ende..n.start];
            if self.luecke(self.w[j].ende, n.start) {
                schl.push(' ');
            } else if ein_apostroph(z) {
                schl.push('\'');
            } else {
                break;
            }
            j += 1;
        }
        bester
    }

    /// Nachname aus dem Register ab `k`, dessen Wörter zu keiner Wortliste gehören; im
    /// Inneren darf ein Artikel stehen («Auf der Maur»).
    fn nachname_frei_von_marken(&self, k: usize) -> Option<(usize, bool)> {
        let begleiter =
            |j: usize| self.marke_ab[j].is_some_and(|m| self.marken[m].art == Wortart::Begleiter);
        self.nachname_ab(k).filter(|&(e, _)| {
            self.frei(k) && self.frei(e) && (k + 1..e).all(|j| self.frei(j) || begleiter(j))
        })
    }

    fn name(&self, erstes: usize, letztes: usize, ende: usize, beleg: Beleg) -> Name {
        Name {
            start: self.w[erstes].start,
            ende,
            erstes,
            letztes,
            beleg,
        }
    }

    /// Ist Wort `k` ein Satzwort der Sprache `sprache` («Daher», «Le», «Egli»)?
    fn satzwort(&self, k: usize, sprache: Sprache) -> bool {
        self.daten
            .satzwoerter
            .contains(&(schluessel(self.text(k)), sprache))
    }

    /// Ist der Nachname `k..=e` eine Grussformel, ein Wochentag oder ein Alltagswort aus
    /// [`ALLTAG`] («Lieber», «Montag», «Glück»)?
    fn alltagswort(&self, k: usize, e: usize) -> bool {
        k == e && self.daten.alltag.contains(&schluessel(self.text(k)))
    }

    /// Nach Initialen ab Wort `y`: ein Nachname aus dem Register, der kein Satzwort in der
    /// Sprache des Auslösers ist, denn eine Initiale mit Punkt kann den Satz beenden («der
    /// Beschwerdeführerin A. Daher ist …», «Le recourant A. Le Tribunal …»). Ein Nachname
    /// aus mehreren Wörtern zählt auch mit einem Satzwort vorn («Mme A. Le Coultre»). Ein
    /// mehrdeutiger Nachname zählt nur, wenn `mehrdeutig_erlaubt`.
    fn nachname_nach_initialen(
        &self,
        y: usize,
        sprache: Sprache,
        mehrdeutig_erlaubt: bool,
    ) -> Option<usize> {
        let (e, mehrdeutig) = self.nachname_frei_von_marken(y)?;
        let satzwort = e == y && self.satzwort(y, sprache);
        let passt = (!mehrdeutig || mehrdeutig_erlaubt) && !satzwort;
        passt.then_some(e)
    }

    /// Die Sprache des Wortlisteneintrags einer Marke.
    fn sprache(&self, m: &Marke) -> Sprache {
        m.lex.eintraege[m.eintrag].sprache
    }

    fn vorname_beleg(&self, k: usize) -> Beleg {
        let wort = self.text(k);
        let begriff = self
            .daten
            .vornamen
            .get(&schluessel(wort))
            .map_or(wort, |r| r.schreibung);
        register_beleg(self.daten.vornamen_register, begriff)
    }

    /// Vorname an Wort `i`, gefolgt vom Nachnamen. `nach_anrede`: Davor steht schon ein
    /// Auslöser; dann zählt ein mehrdeutiger Vorname wie ein eindeutiger.
    fn vorname_pfad(&self, i: usize, nach_anrede: bool) -> Option<Name> {
        if self.markiert[i] || self.gesperrt[i] {
            return None;
        }
        let form = self.form(i);
        if !matches!(form, Form::Gross | Form::Versal) {
            return None;
        }
        let mehrdeutig = self.vorname(i)? && !nach_anrede;
        if form == Form::Versal && mehrdeutig {
            return None;
        }
        if !nach_anrede && self.artikel_statt_vorname(i) {
            return None;
        }
        // Initialen: «Hans M. Muster», «Hans M.»
        let mut letztes = i;
        let mut ende = self.w[i].ende;
        let mut naechstes = self.folgt(i);
        let mut initialen = 0;
        while let Some(x) = naechstes {
            if initialen == 2 || !self.ist_initiale(x) {
                break;
            }
            initialen += 1;
            letztes = x;
            ende = self.w[x].ende + 1;
            naechstes = self.folgt_mit_punkt(x);
        }
        if initialen > 0 {
            if mehrdeutig {
                return None;
            }
            if let Some((e, _)) = naechstes.and_then(|x| self.nachname_frei_von_marken(x)) {
                letztes = e;
                ende = self.w[e].ende;
            }
            return Some(self.name(i, letztes, ende, self.vorname_beleg(i)));
        }
        let x = naechstes?;
        if !self.ueber_umbruch_erlaubt(i, x, nach_anrede) {
            return None;
        }
        let e = self.erstes_wort(x, mehrdeutig, form)?;
        let nach_vorname = !mehrdeutig && e == x && self.vorname(x) == Some(false);
        let e = self.zweites_wort(e, nach_vorname).unwrap_or(e);
        Some(self.name(i, e, self.w[e].ende, self.vorname_beleg(i)))
    }

    /// Ist der Vorname an Wort `i` der französische Artikel «Le» (Wortliste
    /// [`AMTSBEZEICHNUNGEN`], merkmal «artikel»), und steht er am Satzanfang oder vor einer
    /// Amts- oder Behördenbezeichnung derselben Liste? «Le Grand Conseil a voté.», «…, Le
    /// Greffier et …». Dann ist er der Artikel, kein Vorname (Befund technik-1). Mitten im
    /// Satz vor einem anderen Wort bleibt er ein Vorname: «Hier, Le Roux a signé.»
    fn artikel_statt_vorname(&self, i: usize) -> bool {
        if !self.daten.artikel.contains(&schluessel(self.text(i))) {
            return false;
        }
        self.satzanfang[i]
            || self
                .folgt(i)
                .is_some_and(|x| self.amt_ab.contains_key(&self.w[x].start))
    }

    /// Steht an Wort `i` der Artikel «Le» vor einer Amtsbezeichnung derselben Zeile, das Wort
    /// danach und der Beleg der Amtsbezeichnung: «Le Greffier Marie Exemple» ergibt «Marie».
    /// Die Amtsbezeichnung wirkt dann wie eine Anrede (Prüferproblem c-erkennung-p4).
    fn nach_amtsbezeichnung(&self, i: usize) -> Option<(usize, Beleg)> {
        if !self.daten.artikel.contains(&schluessel(self.text(i))) {
            return None;
        }
        let x = self.folgt_in_zeile(i)?;
        let &(ende, eintrag) = self.amt_ab.get(&self.w[x].start)?;
        let (_, letztes) = self.woerter_von(self.w[x].start, ende)?;
        let y = self
            .folgt_ab(letztes, ende)
            .filter(|&y| !self.s[ende..self.w[y].start].contains('\n'))?;
        Some((y, self.daten.aemter.beleg(eintrag)))
    }

    /// Steht zwischen Wort `k` und dem Wort `x` danach ein Zeilenumbruch?
    fn umbruch_zwischen(&self, k: usize, x: usize) -> bool {
        self.s[self.w[k].ende..self.w[x].start].contains('\n')
    }

    /// Das nächste Wort, wenn nur Leerraum derselben Zeile dazwischen steht.
    fn folgt_in_zeile(&self, k: usize) -> Option<usize> {
        self.folgt(k).filter(|&x| !self.umbruch_zwischen(k, x))
    }

    /// Steht vor Wort `i` in derselben Zeile ein Possessivpronomen mit einem Wort danach?
    /// «Meine Patientin Erika», «La mia paziente Giulia», «ma cliente Marie».
    fn besitzrahmen_davor(&self, i: usize) -> bool {
        let (Some(p), Some(b)) = (i.checked_sub(1), i.checked_sub(2)) else {
            return false;
        };
        self.folgt_in_zeile(b) == Some(p)
            && self.folgt_in_zeile(p) == Some(i)
            && matches!(self.form(p), Form::Gross | Form::Klein)
            && self.merkmal_an(b).is_some_and(|st| {
                st.merkmal == Vertretungsmerkmal::Besitz && st.ende == self.w[b].ende
            })
    }

    /// Darf der Nachname ab Wort `x` über den weichen Umbruch nach dem Vornamen an Wort `i`
    /// reichen? Am Zeilenanfang kann ein Satz beginnen, und dort ist jedes Wort gross
    /// geschrieben; auch ein Nachname des Registers kann dort ein gewöhnliches Wort sein
    /// («Grüezi Erika⏎Kurze Frage …»). Darum nur nach einem Auslöser (`nach_anrede`; Anrede,
    /// Titel, Anwaltswort oder Verfahrensrolle direkt vor dem Vornamen) oder nach einem
    /// Possessivrahmen («Meine Patientin Erika⏎Beispiel»). Nach einer Grussformel («Liebe
    /// Erika⏎Danke …») gehört das Wort der nächsten Zeile nicht zum Namen (Prüferproblem
    /// c-erkennung-p2).
    fn ueber_umbruch_erlaubt(&self, i: usize, x: usize, nach_anrede: bool) -> bool {
        let ausloeser_davor = i.checked_sub(1).is_some_and(|p| {
            self.marke_von[p].is_some_and(|q| {
                let m = &self.marken[q];
                m.letztes == p
                    && self.nach_marke(m) == Some(i)
                    && matches!(
                        m.art,
                        Wortart::Anrede
                            | Wortart::AnredeDe
                            | Wortart::Titel
                            | Wortart::Anwalt
                            | Wortart::AnwaltDe
                            | Wortart::Rolle
                    )
            })
        });
        !self.umbruch_zwischen(i, x) || nach_anrede || ausloeser_davor || self.besitzrahmen_davor(i)
    }

    /// Der Nachname nach dem Vornamen, ab Wort `x`: Index seines letzten Wortes.
    fn erstes_wort(&self, x: usize, mehrdeutig: bool, form: Form) -> Option<usize> {
        let register = self.nachname_frei_von_marken(x);
        if mehrdeutig {
            // «Marie Dupont» ja, «im Mai Zahlungen leisten» nein.
            return register.filter(|&(_, m)| !m).map(|(e, _)| e);
        }
        if form == Form::Versal {
            return register
                .filter(|&(e, _)| self.form(e) == Form::Versal)
                .map(|(e, _)| e);
        }
        if let Some((e, _)) = register {
            return Some(e);
        }
        if self.form(x) == Form::Gross && self.frei(x) {
            return Some(x);
        }
        self.elisionsname(x)
    }

    /// Ein weiteres Wort nach dem Nachnamen, der bei `e` endet: nach zwei Vornamen jeder
    /// Nachname aus dem Register («Hans Peter Muster»), sonst ein zweiter, eindeutiger
    /// Nachname («Anna Meier Huber»). Beginnt das Wort einen neuen Namen («Anna Meier Peter
    /// Huber»), gehört es nicht dazu. Über einen weichen Zeilenumbruch reicht es nicht: Die
    /// nächste Zeile kann einen Satz beginnen («Erika Beispiel⏎Le Grand Conseil …»,
    /// Prüferproblem c-erkennung-p8).
    fn zweites_wort(&self, e: usize, nach_vorname: bool) -> Option<usize> {
        let y = self.folgt_in_zeile(e)?;
        if self.markiert[y] {
            return None;
        }
        let (e2, mehrdeutig) = self.nachname_frei_von_marken(y)?;
        if nach_vorname {
            return Some(e2);
        }
        if mehrdeutig {
            return None;
        }
        let neuer_name = e2 == y
            && self.vorname(y) == Some(false)
            && self
                .folgt_in_zeile(y)
                .is_some_and(|z| self.form(z) == Form::Gross);
        (!neuer_name).then_some(e2)
    }

    /// Nach Titel oder französischer oder italienischer Anrede: ein Nachname aus dem
    /// Register oder ein grossgeschriebenes Wort.
    fn nachname_frei(&self, x: usize) -> Option<usize> {
        if let Some((e, _)) = self.nachname_frei_von_marken(x) {
            return Some(e);
        }
        if self.markiert[x] {
            return None;
        }
        if self.form(x) == Form::Gross {
            return Some(x);
        }
        self.elisionsname(x)
    }

    /// Nach einer deutschen Anrede bei Wort `anrede`: ein Nachname aus dem Register. Ist er
    /// mehrdeutig («Angst»), darf vor der Anrede kein Artikel oder Possessivpronomen stehen.
    /// Ein eindeutiger Vorname allein («Frau Erika») zählt auch.
    fn nachname_streng(&self, x: usize, anrede: usize) -> Option<usize> {
        if let Some((e, mehrdeutig)) = self.nachname_frei_von_marken(x) {
            return (!mehrdeutig || !self.begleiter_davor(anrede)).then_some(e);
        }
        let vorname = !self.markiert[x]
            && matches!(self.form(x), Form::Gross | Form::Versal)
            && self.vorname(x) == Some(false);
        vorname.then_some(x)
    }

    /// Steht vor Wort `k` ein Artikel oder Possessivpronomen, auch mit bis zu zwei
    /// kleingeschriebenen Wörtern dazwischen («der alten Frau»)?
    fn begleiter_davor(&self, k: usize) -> bool {
        let mut j = k;
        for _ in 0..3 {
            let Some(p) = j.checked_sub(1) else {
                return false;
            };
            if !self.luecke(self.w[p].ende, self.w[j].start) {
                return false;
            }
            if self.marke_ab[p].is_some_and(|m| self.marken[m].art == Wortart::Begleiter) {
                return true;
            }
            if self.markiert[p] || self.form(p) != Form::Klein {
                return false;
            }
            j = p;
        }
        false
    }

    /// Anrede, Titel oder Anwaltswort ohne Vertretungsmerkmal bei Marke `mi`, dann der Name.
    fn anrede_pfad(&self, mi: usize) -> Pfad {
        let ausloeser = &self.marken[mi];
        let mut frei = !matches!(ausloeser.art, Wortart::AnredeDe | Wortart::AnwaltDe);
        let mut naechstes = self.nach_marke(ausloeser);
        // Kette: «Herrn Dr.», «Frau Präsidentin», «Dr. med.», «Prof. Dr.», «Frau
        // Rechtsanwältin». Eine Initiale beendet sie, auch «M.», das sonst für «Monsieur»
        // steht («Herr M. Muster»).
        let x = loop {
            let Some(x) = naechstes else {
                return Pfad::Nichts;
            };
            if self.ist_initiale(x) {
                break x;
            }
            if let Some(k) = self.marke_ab[x] {
                let m = &self.marken[k];
                match m.art {
                    Wortart::Titel | Wortart::Anrede => frei = true,
                    Wortart::AnredeDe | Wortart::Funktion => {}
                    Wortart::Rechtsvertretung => {
                        return Pfad::Weiter(self.vertretung_ueberspringen(m));
                    }
                    _ if m.art.vertretung() && self.vertritt[k] => {
                        return Pfad::Weiter(self.vertretung_ueberspringen(m));
                    }
                    Wortart::Anwalt => frei = true,
                    Wortart::AnwaltDe => {}
                    Wortart::AnwaltWort
                    | Wortart::Beratung
                    | Wortart::Beauftragt
                    | Wortart::Heilig
                    | Wortart::Begleiter
                    | Wortart::Rolle => return Pfad::Nichts,
                }
                naechstes = self.nach_marke(m);
            } else if self.markiert[x] {
                return Pfad::Nichts;
            } else if self.ist_kleine_abkuerzung(x) {
                naechstes = self.folgt_mit_punkt(x);
            } else {
                break x;
            }
        };
        self.name_nach_kette(ausloeser, x, frei)
    }

    /// Der Name ab Wort `x` nach der Kette eines Auslösers. `frei`: Ein grossgeschriebenes
    /// Wort genügt als Nachname.
    fn name_nach_kette(&self, ausloeser: &Marke, x: usize, frei: bool) -> Pfad {
        let beleg = ausloeser.lex.beleg(ausloeser.eintrag);
        // Initialen vor dem Nachnamen: «Herr H. Muster». Initialen allein («Frau A.») sind
        // die Form anonymisierter Urteile und kein Name; danach beginnt oft ein neuer Satz
        // («Frau A. Sie ist …»), darum zählt nur ein Nachname aus dem Register.
        if self.ist_initiale(x) {
            let mut naechstes = self.folgt_mit_punkt(x);
            let mut weitere = 0;
            while let Some(y) = naechstes {
                if weitere == 2 || !self.ist_initiale(y) {
                    break;
                }
                weitere += 1;
                naechstes = self.folgt_mit_punkt(y);
            }
            let mehrdeutig_erlaubt = !self.begleiter_davor(ausloeser.erstes);
            let sprache = self.sprache(ausloeser);
            return match naechstes
                .and_then(|y| self.nachname_nach_initialen(y, sprache, mehrdeutig_erlaubt))
            {
                Some(e) => Pfad::Name(self.name(x, e, self.w[e].ende, beleg)),
                None => Pfad::Nichts,
            };
        }
        if let Some(mut name) = self.vorname_pfad(x, true) {
            name.beleg = beleg;
            return Pfad::Name(name);
        }
        let ende = if frei {
            self.nachname_frei(x)
        } else {
            self.nachname_streng(x, ausloeser.erstes)
        };
        match ende {
            Some(e) => {
                let e = self.zweites_wort(e, false).unwrap_or(e);
                Pfad::Name(self.name(x, e, self.w[e].ende, beleg))
            }
            None => Pfad::Nichts,
        }
    }

    /// Verfahrensrolle bei Marke `mi`: der Name danach, falls einer folgt. Wie nach einer
    /// deutschen Anrede zählt nur ein Vorname oder ein eindeutiger Nachname aus dem Register,
    /// auch nach Initialen («der Beschwerdeführer A. Meier»), dort aber kein Satzwort der
    /// Sprache der Rolle («der Beschwerdeführerin A. Daher ist …»). Initialen allein («die
    /// Beschwerdeführerin A.») sind die Form anonymisierter Urteile und kein Name.
    fn rolle_pfad(&self, mi: usize) -> Option<Name> {
        let rolle = &self.marken[mi];
        let x = self.nach_marke(rolle)?;
        if self.markiert[x] {
            return None;
        }
        let beleg = rolle.lex.beleg(rolle.eintrag);
        if self.ist_initiale(x) {
            let mut naechstes = self.folgt_mit_punkt(x);
            let mut weitere = 0;
            while let Some(y) = naechstes {
                if weitere == 2 || !self.ist_initiale(y) {
                    break;
                }
                weitere += 1;
                naechstes = self.folgt_mit_punkt(y);
            }
            let e = self.nachname_nach_initialen(naechstes?, self.sprache(rolle), false)?;
            return Some(self.name(x, e, self.w[e].ende, beleg));
        }
        if let Some(mut name) = self.vorname_pfad(x, false) {
            name.beleg = beleg;
            return Some(name);
        }
        let (e, mehrdeutig) = self.nachname_frei_von_marken(x)?;
        if mehrdeutig {
            return None;
        }
        let e = self.zweites_wort(e, false).unwrap_or(e);
        Some(self.name(x, e, self.w[e].ende, beleg))
    }

    /// Name ohne Vornamen vorn, wie ihn Listen schreiben: ein Paar aus Nachname und Vorname
    /// des BFS-Registers. Ein Nachname in Versalien vor einem Vornamen zählt überall
    /// («Patient: KELLER Sabine»); sonst nur in einer Zelle («Müller Andreas, Lager») oder
    /// über zwei Zellen in beiden Reihenfolgen («Muster;Hans», «Hans;Muster»).
    fn liste_pfad(&self, i: usize) -> Option<Name> {
        if self.markiert[i] || self.gesperrt[i] {
            return None;
        }
        self.versal_paar(i).or_else(|| self.zellen_paar(i))
    }

    /// «KELLER Sabine»: Nachname in Versalien, dann ein grossgeschriebener Vorname. Das
    /// letzte Wort des Nachnamens hat mindestens drei Buchstaben, denn zwei Versalien sind
    /// meist eine Abkürzung («OK Peter», «GE Anna»), auch wenn das Register «Ok» und «Ge»
    /// als Nachnamen führt.
    fn versal_paar(&self, i: usize) -> Option<Name> {
        if self.form(i) != Form::Versal {
            return None;
        }
        let (e, nachname_mehrdeutig) = self.nachname_frei_von_marken(i)?;
        if self.form(e) != Form::Versal || self.text(e).chars().count() < 3 {
            return None;
        }
        let nachname_mehrdeutig = nachname_mehrdeutig || self.alltagswort(i, e);
        let j = e + 1;
        let zwischen = self.s.get(self.w[e].ende..self.w.get(j)?.start)?;
        if !nur_zellenleerraum(zwischen) {
            return None;
        }
        let vorname_mehrdeutig = self.listenvorname(j)?;
        self.listenpaar(i, e, nachname_mehrdeutig, vorname_mehrdeutig)
            .then(|| self.name(i, j, self.w[j].ende, self.vorname_beleg(j)))
    }

    /// Paar in einer Zelle («Müller Andreas») oder über zwei Zellen («Muster;Hans»,
    /// «Hans;Muster»). In einer Zelle muss der Nachname eindeutig sein, denn dort stehen auch
    /// Traktanden und Grussformeln («2. Wahl Peter», «Liebe Anna, Lieber Peter,»). Über zwei
    /// Zellen muss der Nachname allein in seiner Zelle stehen und darf kein Vorname sein:
    /// «Peter;Anna» bleibt offen. Trennt sie «;», Tab oder «|», ist die Zeile schon dadurch
    /// eine Liste; ein Komma kann auch im Satz stehen («Nein, Peter»). Grussformeln,
    /// Wochentage und Alltagswörter aus [`ALLTAG`] gelten hier als mehrdeutige Nachnamen.
    fn zellen_paar(&self, i: usize) -> Option<Name> {
        if !matches!(self.form(i), Form::Gross | Form::Klein) {
            return None;
        }
        let anfang = self.zellenanfang(i)?;
        // Die Zelle endet nach Wort `k`, und die Zeile passt.
        let endet = |k: usize, anfang: Zellenanfang| {
            self.zellenende(self.w[k].ende)
                .is_some_and(|ende| self.zeile_passt(anfang, ende))
        };
        let ueber_zellen = |trenner: char| {
            if trenner == ',' {
                anfang
            } else {
                Zellenanfang::Liste
            }
        };
        // Der Nachname `k..=e` ist kein Vorname: Er gibt die Reihenfolge vor.
        let nur_nachname = |k: usize, e: usize| e > k || self.vorname(k).is_none();
        let nachname_vorn = |(e, nachname_mehrdeutig): (usize, bool)| {
            let nachname_mehrdeutig = nachname_mehrdeutig || self.alltagswort(i, e);
            // In einer Zelle: Nachname, Leerzeichen, Vorname, Ende der Zelle.
            let j = e + 1;
            let in_zelle = !nachname_mehrdeutig
                && self
                    .w
                    .get(j)
                    .is_some_and(|n| nur_zellenleerraum(&self.s[self.w[e].ende..n.start]));
            let in_zelle = in_zelle
                .then(|| self.listenvorname(j))
                .flatten()
                .filter(|&vm| endet(j, anfang) && self.listenpaar(i, e, nachname_mehrdeutig, vm));
            if in_zelle.is_some() {
                return Some(self.name(i, j, self.w[j].ende, self.vorname_beleg(j)));
            }
            // Über zwei Zellen: «Muster;Hans».
            if !nur_nachname(i, e) {
                return None;
            }
            let (j, trenner) = self.naechste_zelle(e)?;
            let vorname_mehrdeutig = self.listenvorname(j)?;
            (endet(j, ueber_zellen(trenner))
                && self.listenpaar(i, e, nachname_mehrdeutig, vorname_mehrdeutig))
            .then(|| self.name(i, j, self.w[j].ende, self.vorname_beleg(j)))
        };
        if let Some(name) = self.nachname_frei_von_marken(i).and_then(nachname_vorn) {
            return Some(name);
        }
        // Über zwei Zellen: «Hans;Muster».
        let vorname_mehrdeutig = self.listenvorname(i)?;
        let (j, trenner) = self.naechste_zelle(i)?;
        let (e, nachname_mehrdeutig) = self.nachname_frei_von_marken(j)?;
        let nachname_mehrdeutig = nachname_mehrdeutig || self.alltagswort(j, e);
        let passt = nur_nachname(j, e)
            && endet(e, ueber_zellen(trenner))
            && self.listenpaar(j, e, nachname_mehrdeutig, vorname_mehrdeutig);
        passt.then(|| self.name(i, e, self.w[e].ende, self.vorname_beleg(i)))
    }

    /// Vorname an Wort `j` für ein Listenpaar: grossgeschrieben und im Register. Gibt zurück,
    /// ob er mehrdeutig ist.
    fn listenvorname(&self, j: usize) -> Option<bool> {
        if self.markiert[j] || self.gesperrt[j] || self.form(j) != Form::Gross {
            return None;
        }
        self.vorname(j)
    }

    /// Nachname `k..=e` und ein Vorname bilden ein Paar, wenn der Nachname kein Ortsname ist
    /// und nicht beide zugleich Wörter der Gesetzestexte sind: «Muster;Hans» ja, «Baden,
    /// Anna» nein.
    fn listenpaar(
        &self,
        k: usize,
        e: usize,
        nachname_mehrdeutig: bool,
        vorname_mehrdeutig: bool,
    ) -> bool {
        !(nachname_mehrdeutig && vorname_mehrdeutig) && !self.nachname_ort(k, e)
    }

    /// Ist der Nachname `k..=e` zugleich ein Kantons-, Bezirks- oder Gemeindename?
    fn nachname_ort(&self, k: usize, e: usize) -> bool {
        let mut schl = String::new();
        for j in k..=e {
            if j > k {
                let z = &self.s[self.w[j - 1].ende..self.w[j].start];
                schl.push(if ein_apostroph(z) { '\'' } else { ' ' });
            }
            schl.push_str(&schluessel(self.text(j)));
        }
        if let Some(r) = self.daten.nachnamen.get(&schl) {
            return r.ort;
        }
        self.text(k).split(ist_bindestrich).any(|t| {
            self.daten
                .nachnamen
                .get(&schluessel(t))
                .is_some_and(|r| r.ort)
        })
    }

    /// Beginnt an Wort `k` eine Zelle einer Liste? Davor steht in der Zeile nichts, ein
    /// Aufzählungszeichen, ein Trenner oder ein Doppelpunkt.
    fn zellenanfang(&self, k: usize) -> Option<Zellenanfang> {
        let davor = self.s[..self.w[k].start].trim_end_matches(ist_zellenleerraum);
        match davor.chars().next_back() {
            None | Some('\n' | '\r') => return Some(Zellenanfang::Zeile),
            Some(c) if ist_trenner(c) || c == ':' => return Some(Zellenanfang::Liste),
            _ => {}
        }
        // Hinter dem gefundenen Zeichen schneiden, nicht ein Byte weiter: Der Leerraum kann
        // aus mehreren Bytes bestehen (U+00A0, U+202F; tests/leerraum.rs).
        let zeichen = davor
            .char_indices()
            .rev()
            .find(|&(_, c)| ist_leerraum(c) || c == '\n' || c == '\r')
            .map_or(0, |(p, c)| p + c.len_utf8());
        let vorher = davor[..zeichen].trim_end_matches(ist_zellenleerraum);
        let zeilenanfang = vorher.is_empty() || vorher.ends_with(['\n', '\r']);
        (zeilenanfang && ist_aufzaehlung(&davor[zeichen..])).then_some(Zellenanfang::Liste)
    }

    /// Endet ab Byte `ende` eine Zelle? Dann: am Zeilenende `None`, auch nach einem
    /// Schlusspunkt («Anwesend: Brunner Peter, Gashi Arben.»), sonst die Byte-Stelle nach dem
    /// Trenner.
    fn zellenende(&self, ende: usize) -> Option<Option<usize>> {
        let zeilenende = |r: &str| matches!(r.chars().next(), None | Some('\n' | '\r'));
        let rest = self.s[ende..].trim_start_matches(ist_zellenleerraum);
        match rest.chars().next() {
            _ if zeilenende(rest) => Some(None),
            Some('.') if zeilenende(rest[1..].trim_start_matches(ist_zellenleerraum)) => Some(None),
            Some(c) if ist_trenner(c) => Some(Some(self.s.len() - rest.len() + c.len_utf8())),
            _ => None,
        }
    }

    /// Die nächste Zelle nach Wort `e`, wenn sie mit einem Wort beginnt: sein Index und der
    /// Trenner davor.
    fn naechste_zelle(&self, e: usize) -> Option<(usize, char)> {
        let weiter = self.zellenende(self.w[e].ende)??;
        // Die Trenner sind ASCII, also ein Byte.
        let trenner = char::from(self.s.as_bytes()[weiter - 1]);
        let j = self.w.partition_point(|w| w.start < weiter);
        let n = self.w.get(j)?;
        self.s[weiter..n.start]
            .chars()
            .all(ist_zellenleerraum)
            .then_some((j, trenner))
    }

    /// Am Zeilenanfang ohne Aufzählungszeichen beginnt auch ein Satz. Dort muss die Zeile
    /// nach dem Namen mit einem Trenner und einer Zelle weitergehen, die nicht leer ist,
    /// nicht klein beginnt und kein Satz ist: «Müller Andreas, Lager» ja, «Lieber Peter, ich
    /// komme» und «Gentile Mario, Le scrivo per la pratica.» nein.
    fn zeile_passt(&self, anfang: Zellenanfang, ende: Option<usize>) -> bool {
        match (anfang, ende) {
            (Zellenanfang::Liste, _) => true,
            (Zellenanfang::Zeile, None) => false,
            (Zellenanfang::Zeile, Some(weiter)) => {
                let zelle = self.s[weiter..]
                    .split(|c: char| ist_trenner(c) || c == '\n' || c == '\r')
                    .next()
                    .unwrap_or_default()
                    .trim_matches(ist_zellenleerraum);
                zelle.chars().next().is_some_and(|c| !c.is_lowercase()) && !ist_satzzelle(zelle)
            }
        }
    }

    /// Überspringt eine Vertretung samt Titeln und bis zu drei Namenswörtern (Art. 4 Abs. 1
    /// lit. g der Anonymisierungsregeln). Gibt das nächste Wort zurück.
    fn vertretung_ueberspringen(&self, m: &Marke) -> usize {
        let mut weiter = m.letztes + 1;
        let mut naechstes = self.nach_marke(m);
        // Kette: «Rechtsanwältin Dr.», «avocat Me», «Fürsprecher lic. iur.»
        while let Some(x) = naechstes {
            if let Some(k) = self.marke_ab[x].map(|k| &self.marken[k]) {
                let kette = k.art.vertretung()
                    || matches!(
                        k.art,
                        Wortart::Titel | Wortart::Anrede | Wortart::AnredeDe | Wortart::Funktion
                    );
                if !kette {
                    break;
                }
                weiter = k.letztes + 1;
                naechstes = self.nach_marke(k);
            } else if !self.markiert[x] && self.ist_kleine_abkuerzung(x) {
                weiter = x + 1;
                naechstes = self.folgt_mit_punkt(x);
            } else {
                break;
            }
        }
        // Eine Präposition, ein Artikel oder ein Possessivpronomen leitet die Partei ein, nicht
        // den Namen der Vertretung: «le conseil d'Erika Beispiel», «Verteidiger von Hans
        // Muster». Deren Name wird gemeldet.
        if naechstes.is_some_and(|x| self.verbindung_an(x).is_some()) {
            return weiter;
        }
        // Der Name: «Erika Beispiel», «Maria da Silva», «Hans M. Muster»
        let mut namenswoerter = 0;
        while let Some(x) = naechstes {
            if self.ist_initiale(x) {
                weiter = x + 1;
                naechstes = self.folgt_mit_punkt(x);
                continue;
            }
            if namenswoerter == 3 || self.markiert[x] {
                break;
            }
            let ende = self
                .nachname_ab(x)
                .map(|(e, _)| e)
                .or_else(|| matches!(self.form(x), Form::Gross | Form::Versal).then_some(x))
                .or_else(|| self.elisionsname(x));
            let Some(e) = ende else { break };
            namenswoerter += 1;
            weiter = e + 1;
            naechstes = self.folgt(e);
        }
        weiter
    }

    /// Zeigt der Satz den Namen als Vertretung einer Partei (Art. 4 Abs. 1 lit. g)?
    fn als_vertretung(&self, name: &Name) -> bool {
        let kette = self.kette_zurueck(name.erstes);
        // Ein Anwaltswort ohne Vertretungsmerkmal leitet den Namen ein («Me Jean Dupont»,
        // «l'avocat Jean Dupont»): Ein Verb der Vertretung danach nimmt ihn noch aus.
        let nach_anwalt = kette
            .iter()
            .skip(1)
            .any(|&c| self.marke_ab[c].is_some_and(|q| self.marken[q].art.anwalt()));
        self.vertretung_danach(name)
            || (nach_anwalt && self.verb_danach(name.ende))
            || kette.into_iter().any(|c| self.apposition_vor(c))
    }

    /// Folgt auf den Namen mit Komma oder Klammer eine Bezeichnung der Vertretung, auch mit
    /// Artikel oder Possessivpronomen davor? «Hans Muster, Verteidiger», «Hans Muster, Ihr
    /// Anwalt», «Jean Dupont, l'avocat de la défense», «Mario Rossi, il suo avvocato». Ein
    /// Anwaltswort ohne Vertretungsmerkmal genügt, wenn vor dem Namen ein Partizip der
    /// Vertretung steht: «vertreten durch Hans Muster, Rechtsanwalt».
    fn vertretung_danach(&self, name: &Name) -> bool {
        let Some(n) = self.w.get(name.letztes + 1) else {
            return false;
        };
        if name.ende > n.start || !matches!(self.kern(name.ende, n.start), "," | "(") {
            return false;
        }
        let mut k = name.letztes + 1;
        for _ in 0..3 {
            if let Some(q) = self.marke_ab[k] {
                let m = &self.marken[q];
                if m.art.vertretung() {
                    // «vertreten durch Hans Muster, Rechtsanwalt»; bei «conseil» und
                    // «mandataire» genügt das Partizip davor nicht.
                    let vertreten = || {
                        m.art.anwalt()
                            && self
                                .kette_zurueck(name.erstes)
                                .into_iter()
                                .any(|c| self.vertreten_vor(c))
                    };
                    return self.vertritt[q] || vertreten();
                }
                if m.art != Wortart::Begleiter {
                    return false;
                }
                match self.nach_marke(m) {
                    Some(n) => k = n,
                    None => return false,
                }
            } else if let Some(st) = self.verbindung_an(k) {
                match self.nach_stelle(&st) {
                    Some(n) => k = n,
                    None => return false,
                }
            } else if self.ist_elision(k) {
                k += 1;
            } else {
                return false;
            }
        }
        false
    }

    /// Der Durchlauf von links nach rechts.
    fn durchlauf(&self) -> Vec<Treffer> {
        let mut treffer = Vec::new();
        let mut i = 0;
        while i < self.w.len() {
            let Some(mi) = self.marke_ab[i] else {
                let nach_amt = self.nach_amtsbezeichnung(i).and_then(|(y, beleg)| {
                    let mut name = self.vorname_pfad(y, true)?;
                    name.beleg = beleg;
                    Some(name)
                });
                match nach_amt
                    .or_else(|| self.vorname_pfad(i, false))
                    .or_else(|| self.liste_pfad(i))
                {
                    Some(name) => {
                        i = name.letztes + 1;
                        self.melden(name, &mut treffer);
                    }
                    None => i += 1,
                }
                continue;
            };
            let m = &self.marken[mi];
            i = m.letztes + 1;
            match m.art {
                Wortart::Rechtsvertretung => i = self.vertretung_ueberspringen(m),
                _ if m.art.vertretung() && self.vertritt[mi] => {
                    i = self.vertretung_ueberspringen(m);
                }
                Wortart::AnredeDe
                | Wortart::Anrede
                | Wortart::Titel
                | Wortart::Anwalt
                | Wortart::AnwaltDe => match self.anrede_pfad(mi) {
                    Pfad::Name(name) => {
                        i = name.letztes + 1;
                        self.melden(name, &mut treffer);
                    }
                    Pfad::Weiter(k) => i = k,
                    Pfad::Nichts => {}
                },
                Wortart::Rolle => match self.rolle_pfad(mi) {
                    Some(name) => {
                        i = name.letztes + 1;
                        self.melden(name, &mut treffer);
                    }
                    None => treffer.push(
                        Treffer::neu("personenbezug", m.start, m.ende, Art::Personenbezug)
                            .mit_beleg(m.lex.beleg(m.eintrag)),
                    ),
                },
                Wortart::AnwaltWort
                | Wortart::Beratung
                | Wortart::Beauftragt
                | Wortart::Heilig
                | Wortart::Funktion
                | Wortart::Begleiter => {}
            }
        }
        treffer
    }

    fn melden(&self, name: Name, treffer: &mut Vec<Treffer>) {
        if !self.als_vertretung(&name) {
            treffer.push(
                Treffer::neu("name", name.start, name.ende, Art::Identifikator)
                    .mit_beleg(name.beleg),
            );
        }
    }
}

const GEPRUEFT: Stand = Stand::Geprueft("2026-11-06");

const LIZENZ_BFS: &str = "opendata.swiss, Nutzungsbedingung «Freie Nutzung. Quellenangabe ist Pflicht.» (https://opendata.swiss/terms-of-use#terms_by): nicht kommerzielle und kommerzielle Nutzung erlaubt, Quellenangabe Pflicht (Autor, Titel und Link zum Datensatz). Quelle: BFS – STATPOP";

fn quellen() -> Vec<Quelle> {
    let gesetz = |titel, url| Quelle {
        titel,
        url,
        lizenz: None,
        stand: GEPRUEFT,
    };
    vec![
        Quelle {
            titel: "BFS, Männliche Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 (STATPOP), veröffentlicht 21.08.2026: Register der Vornamen",
            url: "https://opendata.swiss/de/dataset/mannliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025",
            lizenz: Some(LIZENZ_BFS),
            stand: GEPRUEFT,
        },
        Quelle {
            titel: "BFS, Weibliche Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 (STATPOP), veröffentlicht 21.08.2026: Register der Vornamen",
            url: "https://opendata.swiss/de/dataset/weibliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025",
            lizenz: Some(LIZENZ_BFS),
            stand: GEPRUEFT,
        },
        Quelle {
            titel: "BFS, Nachnamen der ständigen Wohnbevölkerung nach Sprachregion, 2025 (STATPOP), veröffentlicht 21.08.2026: Register der Nachnamen",
            url: "https://opendata.swiss/de/dataset/nachnamen-der-standigen-wohnbevolkerung-nach-sprachregion3",
            lizenz: Some(LIZENZ_BFS),
            stand: GEPRUEFT,
        },
        Quelle {
            titel: "BFS, Historisiertes Gemeindeverzeichnis der Schweiz, Momentaufnahme vom 06.11.2026: Kantone, Bezirke und Gemeinden für die Spalte «ort» der Register",
            url: "https://opendata.swiss/de/dataset/historisiertes-gemeindeverzeichnis-der-schweiz",
            lizenz: Some(
                "opendata.swiss, Nutzungsbedingung «Freie Nutzung» (https://opendata.swiss/terms-of-use#terms_open)",
            ),
            stand: Stand::ZuPruefen(
                "Die Nutzungsbedingung steht beim Datensatz; die Momentaufnahme stammt aus der Schnittstelle www.agvchapp.bfs.admin.ch, die der Datensatz nicht ausdrücklich als Ressource nennt.",
            ),
        },
        gesetz(
            "StPO (SR 312.0), Fassung vom 01.04.2025, DE, FR, IT: Verfahrensrollen, Rechtsvertretung, Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/2010/267/de",
        ),
        gesetz(
            "ZPO (SR 272), Fassung vom 01.07.2026, DE, FR, IT: Verfahrensrollen, Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/2010/262/de",
        ),
        gesetz(
            "BGG (SR 173.110), Fassung vom 01.04.2026, DE, FR, IT: Verfahrensrollen, Funktionen, Parteivertretung, Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/2006/218/de",
        ),
        gesetz(
            "BV (SR 101), Fassung vom 03.03.2024, DE, FR, IT: Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/1999/404/de",
        ),
        gesetz(
            "ZGB (SR 210), Fassung vom 01.07.2026, DE, FR, IT: Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de",
        ),
        gesetz(
            "OR (SR 220), Fassung vom 01.10.2026, DE, FR, IT: Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de",
        ),
        gesetz(
            "StGB (SR 311.0), Fassung vom 01.11.2026, DE, FR, IT: Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de",
        ),
        gesetz(
            "DSG (SR 235.1), Fassung vom 07.07.2025, DE, FR, IT: Wortlaut für die Spalte «gesetz»",
            "https://www.fedlex.admin.ch/eli/cc/2022/491/de",
        ),
        gesetz(
            "VwVG (SR 172.021), Fassung vom 01.07.2022, DE: Verfahrensrollen «Beschwerdeführer» (Art. 49, 52) und «Gesuchsteller» (Art. 24)",
            "https://www.fedlex.admin.ch/eli/cc/1969/737_757_755/de",
        ),
        gesetz(
            "MStP (SR 322.1), Fassung vom 01.07.2024, DE: Verfahrensrolle «Beschwerdegegner» (Art. 168)",
            "https://www.fedlex.admin.ch/eli/cc/1979/1059_1059_1059/de",
        ),
        gesetz(
            "VStrR (SR 313.0), Fassung vom 01.09.2023, DE: Verfahrensrolle «Beschuldigte» (Art. 32, 74)",
            "https://www.fedlex.admin.ch/eli/cc/1974/1857_1857_1857/de",
        ),
        gesetz(
            "ZV (SR 631.01), Fassung vom 01.10.2024, DE: Art. 209 Bst. a «…, vertreten durch das BAZG, …» für das Vertretungsmerkmal «vertreten durch»",
            "https://www.fedlex.admin.ch/eli/cc/2007/250/de",
        ),
        gesetz(
            "SchKG (SR 281.1), Fassung vom 01.01.2026, IT (LEF): Alltagswort «fortuna» (Art. 265 Abs. 2)",
            "https://www.fedlex.admin.ch/eli/cc/11/529_488_529/it",
        ),
        gesetz(
            "SSV (SR 741.21), Fassung vom 01.10.2026, DE: Satzwort «daher» (Art. 7 Abs. 1)",
            "https://www.fedlex.admin.ch/eli/cc/1979/1961_1961_1961/de",
        ),
        gesetz(
            "VRV (SR 741.11), Fassung vom 01.10.2026, DE: Wochentage «Montag» und «Freitag» (Art. 91 Abs. 1)",
            "https://www.fedlex.admin.ch/eli/cc/1962/1364_1409_1420/de",
        ),
        gesetz(
            "WG (SR 514.54), Fassung vom 01.09.2023, DE: Alltagswort «Messer» (Art. 4 Abs. 1 Bst. c)",
            "https://www.fedlex.admin.ch/eli/cc/1998/2535_2535_2535/de",
        ),
        gesetz(
            "KLV (SR 832.112.31), Fassung vom 01.08.2026, IT (OPre): Alltagswort «caro» (Art. 4a Abs. 1 Bst. c)",
            "https://www.fedlex.admin.ch/eli/cc/1995/4964_4964_4964/it",
        ),
        gesetz(
            "BPV (SR 172.220.111.3), Fassung vom 01.07.2026, FR (OPers): Satzwort «car» (Art. 75d Abs. 3)",
            "https://www.fedlex.admin.ch/eli/cc/2001/319/fr",
        ),
        gesetz(
            "KKV-FINMA (SR 951.312), Fassung vom 01.01.2021, FR (OPC-FINMA): Satzwort «ceci» (Art. 19 Abs. 3)",
            "https://www.fedlex.admin.ch/eli/cc/2014/707/fr",
        ),
        gesetz(
            "BV (SR 101), Fassung vom 03.03.2024, FR: Artikel «le» (art. 1) und Amtsbezeichnung «canton» (art. 37) nach dem Artikel «Le»",
            "https://www.fedlex.admin.ch/eli/cc/1999/404/fr",
        ),
        gesetz(
            "CPP (SR 312.0), Fassung vom 01.04.2025, FR: Amtsbezeichnung «procureur» (art. 14) nach dem Artikel «Le»",
            "https://www.fedlex.admin.ch/eli/cc/2010/267/fr",
        ),
        gesetz(
            "LTF (SR 173.110), Fassung vom 01.04.2026, FR: Amtsbezeichnungen «juge» und «greffier» (art. 35) nach dem Artikel «Le»",
            "https://www.fedlex.admin.ch/eli/cc/2006/218/fr",
        ),
        gesetz(
            "Constitution du Canton de Vaud (SR 131.231), Fassung vom 18.06.2023, FR: Amtsbezeichnungen «Grand Conseil» (art. 91), «Conseil d'État» (art. 112), «syndic» (art. 150), «préfet» (art. 159) nach dem Artikel «Le»",
            "https://www.fedlex.admin.ch/eli/cc/2003/1136_fga/fr",
        ),
        gesetz(
            "Constitution de la République et Canton du Jura (SR 131.235), Fassung vom 01.01.2026, FR: Amtsbezeichnung «maire» (art. 119) nach dem Artikel «Le»",
            "https://www.fedlex.admin.ch/eli/cc/1977/2_264_259_261_fga/fr",
        ),
        gesetz(
            "Zusammenarbeits-Vereinbarung Flughafen Basel-Mülhausen (SR 748.134.4), Fassung vom 14.01.1998, DE: Titel «…, vertreten durch den Schweizerischen Bundesrat, …» für das Vertretungsmerkmal «vertreten durch»",
            "https://www.fedlex.admin.ch/eli/cc/1998/2302_2302_2302/de",
        ),
        gesetz(
            "Bundesgericht, Regeln für die Anonymisierung der Urteile, genehmigt am 13.02.2020, Art. 4 Abs. 1 lit. a, b, e, g und Art. 5: Ausnahmen",
            "https://www.bger.ch/home/juridiction/jurisdiction-recht/anonymisierungsregeln.html",
        ),
        gesetz(
            "Duden online: Herr, Frau, Dr., Prof., Sankt, Rechtsanwalt, Rechtsanwältin, RA, Fürsprecher, Fürsprecherin, der/die/das, ein, mein, sein, ihr, unser, man, du, nein, hernach, lieb, Glück, Pech",
            "https://www.duden.de/rechtschreibung/Herr",
        ),
        gesetz(
            "Larousse en ligne: monsieur, madame, mademoiselle, docteur, Dr, professeur, saint, avocat, maître, plaider, mon, notre, votre, leur, te, tu, bonjour; français-italien: Pr; italien-français: Arch.",
            "https://www.larousse.fr/dictionnaires/francais/monsieur/52460",
        ),
        gesetz(
            "Treccani, Vocabolario e Enciclopedia dell'Italiano: signore, signora, signorina, dottore, dott., professore, prof., ingegnere, santo, avvocato, mio, nostro, vostro, suo, noi, tu, giorno, gentile, abbreviazioni",
            "https://www.treccani.it/vocabolario/signore/",
        ),
    ]
}

/// Beschreibung der Regel `name`. Die Treffer der Regel `personenbezug` erscheinen nie
/// selbst (SPEC 4.4); ihre Wortliste ist die der Verfahrensrollen.
pub fn info() -> Vec<RegelInfo> {
    vec![RegelInfo {
        id: "name",
        klasse: Klasse::B,
        titel: Txt::neu("Name einer natürlichen Person", "Name of a natural person"),
        erkennt: Txt::neu(
            "Vor- und Nachnamen natürlicher Personen nach einem Vornamen, einer Anrede, einem Titel oder einer Verfahrensrolle, dazu in Listen ein Paar aus Nachname und Vorname («Müller Andreas, Lager», «Muster;Hans», «KELLER Sabine»). Eine Verfahrensrolle ohne Namen («die beschuldigte Person», «der Beschwerdeführer») meldet sie als Personenbezug (Regel personenbezug).",
            "First and last names of natural persons after a first name, a form of address, a title or a procedural role, and in lists a pair of last name and first name («Müller Andreas, Lager», «Muster;Hans», «KELLER Sabine»). A procedural role without a name («die beschuldigte Person», «der Beschwerdeführer») is reported as a personal reference (rule personenbezug).",
        ),
        methode: Txt::neu(
            "Auslöser sind Vornamen aus dem BFS-Register (die häufigsten bis 95 % der Nennungen), Anreden und Titel (Duden, Larousse, Treccani) und Verfahrensrollen aus StPO, ZPO, BGG, VwVG, ZGB, MStP und VStrR in DE, FR und IT. Danach gilt ein grossgeschriebenes Wort als Nachname, nach einer deutschen Anrede, einem deutschen Anwaltstitel oder einer Rolle nur ein Nachname aus dem BFS-Register (die häufigsten bis 90 % der Personen). Hinter Initialen («der Beschwerdeführer A. Meier», «Herr H. Muster») zählt nur ein Nachname aus dem Register, der kein Satzwort in der Sprache des Auslösers ist, denn die Initiale kann den Satz beenden: «der Beschwerdeführerin A. Daher ist …» ist kein Name. In einer Liste (Zeilenanfang, Aufzählungszeichen, Trenner «,» «;» Tab «|» oder Doppelpunkt) zählt ein Paar aus Nachname und Vorname des BFS-Registers in beiden Reihenfolgen; ein Nachname in Versalien vor einem Vornamen zählt überall. Vor- und Nachnamen, die im Wortlaut von BV, ZGB, OR, StGB, StPO, ZPO, BGG oder DSG stehen oder Kantons-, Bezirks- oder Gemeindenamen sind, gelten als mehrdeutig und brauchen einen eindeutigen Partner; in einer Liste gelten auch Grussformeln, Wochentage und Alltagswörter als mehrdeutig, die das Register als Nachnamen führt («Liebe», «Montag», «Bonjour», «Gentile»), und der Nachname darf kein Ortsname sein. Namen von Rechtsvertretern bleiben unmarkiert, wenn der Satz die Person als Vertretung einer Partei zeigt (Anonymisierungsregeln des Bundesgerichts, Art. 4 Abs. 1 lit. g): mit einem Wort wie «Verteidigerin», «défenseur» oder «patrocinatore» oder mit einem Anwaltstitel oder -beruf («RA», «Me», «avv.», «avocat») und einem Vertretungsmerkmal («vertreten durch», «représenté par», «rappresentato dall'», «Ihr Anwalt», «avocat de la partie adverse», «plaide pour»). Ein Verb zählt nur mit einer Partei danach («Me Jean Dupont plaide pour Marie Dupont.»), ein Possessivpronomen («mon avocat», «Ihr Anwalt») nur, wenn der Satz keine Angabe nach Art. 5 lit. c DSG enthält. «conseil» zählt nur mit einer Partei danach oder einem Partizip der Vertretung davor, «mandataire» nur mit «partie» oder einer Verfahrensrolle danach. Sonst wirkt der Anwaltstitel wie ein Titel; «avocat» und «avvocato» lösen allein nichts aus. Der französische Artikel «Le», im Register zugleich Vorname, löst am Satzanfang keinen Namen aus; ein weicher Zeilenumbruch beginnt dafür keinen Satz. Vor einer Amts- oder Behördenbezeichnung aus Bundeserlassen und Kantonsverfassungen («Le Grand Conseil», «Le Maire», «Le Greffier») ist «Le» überall der Artikel (Art. 4 Abs. 1 lit. a); ein Vorname mit Nachnamen danach gilt wie nach einer Anrede als Name («Le Greffier Marie Exemple»). Zwischen den Teilen eines Namens darf ein weicher Zeilenumbruch stehen, also ein einzelner Umbruch mitten im Absatz; zwischen Vor- und Nachnamen aber nur nach einer Anrede, einem Titel, einer Verfahrensrolle oder einem Possessivrahmen («Meine Patientin Erika⏎Beispiel», «Herrn⏎Hans Muster»), denn am Zeilenanfang kann ein Satz beginnen. Unsichtbare Formatzeichen wie das weiche Trennzeichen U+00AD mitten im Wort werden übergangen.",
            "Triggers are first names from the BFS register (the most frequent, up to 95 % of mentions), forms of address and titles (Duden, Larousse, Treccani) and procedural roles from the StPO, ZPO, BGG, VwVG, ZGB, MStP and VStrR in German, French and Italian. After a trigger a capitalised word counts as the last name; after a German form of address, a German lawyer's title or a role only a last name from the BFS register (the most frequent, up to 90 % of persons). Behind initials («der Beschwerdeführer A. Meier», «Herr H. Muster») only a last name from the register counts that is not a sentence word in the trigger's language, because the initial can end the sentence: «der Beschwerdeführerin A. Daher ist …» is not a name. In a list (line start, bullet, separator «,» «;» tab «|» or colon) a pair of last name and first name from the BFS register counts in either order; a last name in capitals before a first name counts everywhere. First and last names that occur in the text of the BV, ZGB, OR, StGB, StPO, ZPO, BGG or DSG or are names of cantons, districts or municipalities count as ambiguous and need an unambiguous partner; in a list greetings, weekdays and everyday words that the register lists as last names («Liebe», «Montag», «Bonjour», «Gentile») count as ambiguous too, and the last name must not be a place name. Names of legal representatives stay unmarked when the sentence shows the person as representing a party (Federal Supreme Court anonymisation rules, Art. 4(1)(g)): with a word such as «Verteidigerin», «défenseur» or «patrocinatore», or with a lawyer's title or profession («RA», «Me», «avv.», «avocat») and a marker of representation («vertreten durch», «représenté par», «rappresentato dall'», «Ihr Anwalt», «avocat de la partie adverse», «plaide pour»). A verb counts only with a party after it («Me Jean Dupont plaide pour Marie Dupont.»), a possessive («mon avocat», «Ihr Anwalt») only if the sentence contains no data under Art. 5(c) FADP. «conseil» counts only with a party after it or a participle of representation before it, «mandataire» only with «partie» or a procedural role after it. Otherwise the lawyer's title acts like a title; «avocat» and «avvocato» trigger nothing on their own. The French article «Le», also a first name in the register, triggers no name at the start of a sentence; a soft line break does not start a sentence for this purpose. Before a title of an office or authority from federal acts and cantonal constitutions («Le Grand Conseil», «Le Maire», «Le Greffier») «Le» is the article everywhere (Art. 4(1)(a)); a first name with a last name after it counts as a name, as after a form of address («Le Greffier Marie Exemple»). A soft line break, a single break within a paragraph, may stand between the parts of a name; between first and last name only after a form of address, a title, a procedural role or a possessive frame («Meine Patientin Erika⏎Beispiel», «Herrn⏎Hans Muster»), because a sentence can start at the beginning of a line. Invisible format characters such as the soft hyphen U+00AD within a word are skipped.",
        ),
        hinweis: None,
        grundlage: vec![&normen::DSG_5_A],
        quellen: quellen(),
        lexika: vec![
            VORNAMEN,
            NACHNAMEN,
            ANREDEN,
            ROLLEN,
            RECHTSVERTRETUNG,
            VERTRETUNG,
            FUNKTIONEN,
            BEGLEITER,
            ALLTAG,
            AMTSBEZEICHNUNGEN,
        ],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Ein Nachname ohne Auslöser («Muster sagt») bleibt unerkannt, ebenso ein Nachname vor dem Vornamen ausserhalb einer Liste («Muster Hans hat angerufen») und am Zeilenanfang ohne Trenner danach («Keller Sabine» allein auf der Zeile). Eine Namensliste nur in dieser Form ergibt keinen Fund der Regel name; die Angaben daneben stehen dann ohne Namen.",
                "A last name without a trigger («Muster sagt») is not recognised, nor is a last name before the first name outside a list («Muster Hans hat angerufen») or at the start of a line without a separator after it («Keller Sabine» alone on the line). A list of names only in this form gives no finding of the rule name; the data next to them then stand without a name.",
            ),
            Txt::neu(
                "In einer Liste zählt ein Paar nur, wenn beide Wörter im BFS-Register stehen und der Nachname kein Ortsname ist: «Beispiel;Erika;Lager» und «Baden;Anna;Lager» bleiben unerkannt. In einer Zelle braucht das Paar einen eindeutigen Nachnamen: «2. Wahl Peter», «Liebe Anna, Lieber Peter,» und «Montag Peter, Dienstag Anna» bleiben ohne Fund, ebenso ein echter Name wie «Gentile Mario, Lugano»; über zwei Zellen oder in Versalien zählt er («Gentile;Mario», «GENTILE Mario»). Am Zeilenanfang muss die Zeile nach dem Paar mit einem Trenner und einer Zelle weitergehen, die nicht klein beginnt und kein Satz ist, ausser «;», Tab oder «|» trennen die beiden Namen; «Lieber Peter, ich komme», «Müller Andreas, Ich bin krank.» und «Muster, Hans» allein bleiben so kein Name. Ein Nachname in Versalien braucht mindestens drei Buchstaben («WU Anna» bleibt unerkannt).",
                "In a list a pair counts only if both words are in the BFS register and the last name is not a place name: «Beispiel;Erika;Lager» and «Baden;Anna;Lager» are not recognised. Within one cell the pair needs an unambiguous last name: «2. Wahl Peter», «Liebe Anna, Lieber Peter,» and «Montag Peter, Dienstag Anna» give no finding, nor does a real name such as «Gentile Mario, Lugano»; across two cells or in capitals it counts («Gentile;Mario», «GENTILE Mario»). At the start of a line the line must continue after the pair with a separator and a cell that does not begin in lower case and is not a sentence, unless «;», tab or «|» separates the two names; «Lieber Peter, ich komme», «Müller Andreas, Ich bin krank.» and «Muster, Hans» alone are thus not a name. A last name in capitals needs at least three letters («WU Anna» is not recognised).",
            ),
            Txt::neu(
                "Nach einer deutschen Anrede oder einer Verfahrensrolle zählt nur ein Nachname, den das BFS-Register mit mindestens 11 Personen führt; «Frau Beispiel» bleibt unerkannt. Hinter Initialen gilt ein Satzwort nicht als Nachname, auch wenn das Register es führt: «Frau A. Ganz» bleibt unerkannt.",
                "After a German form of address or a procedural role only a last name borne by at least 11 persons in the BFS register counts; «Frau Beispiel» is not recognised. Behind initials a sentence word does not count as a last name even if the register lists it: «Frau A. Ganz» is not recognised.",
            ),
            Txt::neu(
                "Vornamen mit weniger als 31 Nennungen fehlen im Register. 190 Vornamen sind mehrdeutig («Mai», «Art», «Uri», «Vera») und lösen nur mit einem eindeutigen Nachnamen aus.",
                "First names with fewer than 31 mentions are missing from the register. 190 first names are ambiguous («Mai», «Art», «Uri», «Vera») and trigger only together with an unambiguous last name.",
            ),
            Txt::neu(
                "Nach einem Vornamen gilt jedes grossgeschriebene Wort als Nachname. Im Deutschen kann das ein Substantiv sein («Peter Geld geben»), ebenso der Teil eines Strassen- oder Firmennamens («Peter Merian-Strasse», «Hans Muster AG»).",
                "After a first name any capitalised word counts as the last name. In German this can be a noun («Peter Geld geben») or part of a street or company name («Peter Merian-Strasse», «Hans Muster AG»).",
            ),
            Txt::neu(
                "Namen von Rechtsvertretern bleiben nur mit einem Vertretungsmerkmal unmarkiert (Art. 4 Abs. 1 lit. g der Anonymisierungsregeln); ein Anwaltstitel allein wirkt wie ein Titel («RA Hans Muster», «Me Jean Dupont»). Ein Possessivpronomen vor dem Anwaltswort zeigt die Vertretung nur ohne Angabe nach Art. 5 lit. c DSG im Satz: «Mon avocat, Me Jean Dupont, a écrit.» und «Jean Dupont, son avocat, a écrit.» bleiben unmarkiert, «Mon avocat, Me Jean Dupont, est en incapacité de travail.» und «Jean Dupont, son avocat, est en incapacité de travail.» melden den Namen. Ob die Angabe die Anwältin betrifft, prüft promptscan nicht: «Mon avocat, Me Jean Dupont, dit que je suis en incapacité de travail.» meldet den Namen auch. Ein Wort, das die Vertretung schon bezeichnet, nimmt den Namen immer aus, auch wenn die Person selbst betroffen ist («Le défenseur Jean Dupont est en incapacité de travail.»). «avocat» und «avvocato» lösen allein nichts aus: «l'avocat Dupont» ohne Vornamen bleibt unerkannt. Namen von Mitgliedern einer Behörde werden markiert; ob sie nach Art. 5 auszunehmen sind, entscheidet promptscan nicht.",
                "Names of legal representatives stay unmarked only with a marker of representation (Art. 4(1)(g) of the anonymisation rules); a lawyer's title alone acts like a title («RA Hans Muster», «Me Jean Dupont»). A possessive before the lawyer's word shows representation only without data under Art. 5(c) FADP in the sentence: «Mon avocat, Me Jean Dupont, a écrit.» and «Jean Dupont, son avocat, a écrit.» stay unmarked, «Mon avocat, Me Jean Dupont, est en incapacité de travail.» and «Jean Dupont, son avocat, est en incapacité de travail.» report the name. promptscan does not check whether the data concern the lawyer: «Mon avocat, Me Jean Dupont, dit que je suis en incapacité de travail.» reports the name too. A word that itself denotes representation always exempts the name, even if the person is the data subject («Le défenseur Jean Dupont est en incapacité de travail.»). «avocat» and «avvocato» trigger nothing on their own: «l'avocat Dupont» without a first name is not recognised. Names of members of an authority are marked; promptscan does not decide whether Art. 5 exempts them.",
            ),
            Txt::neu(
                "Der französische Titel «Dre» fehlt, weil keine zugelassene Quelle ihn führt; «La Dre Rochat» bleibt unerkannt.",
                "The French title «Dre» is missing because no admissible source lists it; «La Dre Rochat» is not recognised.",
            ),
            Txt::neu(
                "Ein Nachname mit dem Artikel «Le» bleibt am Satzanfang unerkannt («Le Roux a signé le contrat.»); mitten im Satz wird er erkannt («Hier, Le Roux a signé.»), auch nach einem weichen Umbruch. Darum gilt «Le» nach einer Überschrift ohne Satzzeichen als Vorname («Considérant⏎Le Roux a signé.» ergibt «Le Roux»). Ortschaften mit «Le» mitten im Satz können als Name gelten, wenn der zweite Teil ein Nachname ist («les villages Le Pont et Le Sentier»). «Di» und «Al» bleiben Vornamen: «Di Maio» ist ein Name.",
                "A last name with the article «Le» is not recognised at the start of a sentence («Le Roux a signé le contrat.»); in the middle of a sentence it is («Hier, Le Roux a signé.»), also after a soft break. Hence «Le» after a heading without punctuation counts as a first name («Considérant⏎Le Roux a signé.» gives «Le Roux»). Localities with «Le» in the middle of a sentence can count as a name if the second part is a last name («les villages Le Pont et Le Sentier»). «Di» and «Al» remain first names: «Di Maio» is a name.",
            ),
            Txt::neu(
                "Ein Zeilenumbruch nach einem Satzzeichen, vor oder nach einer Leerzeile und an einem Listenpunkt trennt die Teile eines Namens («- Erika⏎- Beispiel» ergibt keinen Namen). Ohne Anrede, Titel, Verfahrensrolle oder Possessivrahmen davor endet ein Name am Zeilenende: «Liebe Erika⏎Danke …» ergibt keinen Namen, «Monsieur⏎Le Roux Pierre⏎Rue du Lac 12» nur «Le Roux Pierre», aber auch «Hallo Erika⏎Meier hat angerufen» keinen. Ein zweiter Nachname wird nie über einen Umbruch angehängt.",
                "A line break after a punctuation mark, before or after a blank line and at a list item separates the parts of a name («- Erika⏎- Beispiel» gives no name). Without a form of address, title, procedural role or possessive frame before it, a name ends at the end of the line: «Liebe Erika⏎Danke …» gives no name, «Monsieur⏎Le Roux Pierre⏎Rue du Lac 12» only «Le Roux Pierre», but «Hallo Erika⏎Meier hat angerufen» gives none either. A second last name is never added across a break.",
            ),
            Txt::neu(
                "Trefferquote und Präzision sind nicht gemessen; das Ziel nach SPEC 7.5 ist eine Trefferquote von mindestens 0,90 und eine Präzision von mindestens 0,80.",
                "Recall and precision have not been measured; the target under SPEC 7.5 is a recall of at least 0.90 and a precision of at least 0.80.",
            ),
        ],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fund::{Rolle, Stufe, Ziel};
    use crate::stufen::Optionen;

    fn stellen(roh: &str, regel: &str) -> Vec<String> {
        let text = Text::neu(roh);
        erkenne(&text)
            .into_iter()
            .filter(|t| t.regel == regel)
            .map(|t| text.ausschnitt(t.start..t.ende).to_string())
            .collect()
    }

    fn namen(roh: &str) -> Vec<String> {
        stellen(roh, "name")
    }

    fn bezuege(roh: &str) -> Vec<String> {
        stellen(roh, "personenbezug")
    }

    fn pruefe_liste(liste: &[(&str, &[&str])]) {
        for (text, erwartet) in liste {
            assert_eq!(namen(text), *erwartet, "Text: «{text}»");
        }
    }

    /// Alle Namen sind erfunden oder Platzhalter.
    const POSITIV: &[(&str, &[&str])] = &[
        (
            "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, ist seit Mai krank.",
            &["Hans Muster"],
        ),
        (
            "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will das Erbe ausschlagen.",
            &["Erika Beispiel"],
        ),
        ("Sehr geehrter Herr Meier", &["Meier"]),
        ("Frau Angst hat angerufen.", &["Angst"]),
        ("Herrn Dr. Muster zustellen.", &["Muster"]),
        ("Dr. med. Muster stellt das Zeugnis aus.", &["Muster"]),
        (
            "Prof. Dr. Erika Beispiel lehrt in Bern.",
            &["Erika Beispiel"],
        ),
        ("Frau Präsidentin Muster eröffnet die Sitzung.", &["Muster"]),
        ("Herr H. Muster ist umgezogen.", &["H. Muster"]),
        ("Hans M. Muster ist umgezogen.", &["Hans M. Muster"]),
        (
            "Hans-Peter Muster und Hans Peter Muster sind Brüder.",
            &["Hans-Peter Muster", "Hans Peter Muster"],
        ),
        ("HANS MUSTER, Seestrasse 1", &["HANS MUSTER"]),
        (
            "Maria da Silva und Anna Meier Huber erscheinen.",
            &["Maria da Silva", "Anna Meier Huber"],
        ),
        ("Peter auf der Maur kommt.", &["Peter auf der Maur"]),
        ("Hans Richter ist Zeuge.", &["Hans Richter"]),
        ("Die beschuldigte Person Meier schweigt.", &["Meier"]),
        ("Zeuge Hans Muster sagt aus.", &["Hans Muster"]),
        ("Marie Dupont wohnt in Genf.", &["Marie Dupont"]),
        ("Monsieur Dupont a écrit.", &["Dupont"]),
        (
            "Mme Dupont et M. Martin sont présents.",
            &["Dupont", "Martin"],
        ),
        ("Jean Dupont habite à Lausanne.", &["Jean Dupont"]),
        ("Le prévenu Dupont conteste les faits.", &["Dupont"]),
        ("Le témoin Jean Dupont a été entendu.", &["Jean Dupont"]),
        (
            "Le dossier d\u{2019}Erika Beispiel est complet.",
            &["Erika Beispiel"],
        ),
        ("Il signor Mario Rossi è arrivato.", &["Mario Rossi"]),
        ("Alla sig.ra Rossi", &["Rossi"]),
        ("Il dott. Bianchi visita.", &["Bianchi"]),
        (
            "Il ricorrente Rossi chiede il gratuito patrocinio.",
            &["Rossi"],
        ),
        ("Maria d'Angelo abita a Lugano.", &["Maria d'Angelo"]),
    ];

    /// SPEC 5.2 und 7.2: Gegenbeispiele, die kein Name sind.
    const NEGATIV: &[&str] = &[
        "Art. 5 DSG",
        "Wer im Mai Zahlungen leisten muss, prüft Art. 5 DSG.",
        "Am 1. April Zahlungen leisten.",
        "Frau Präsidentin, ich beantrage das Wort.",
        "Herr Rechtsanwalt, bitte.",
        "Herr Richter, bitte.",
        "Siehe BGE 136 II 508 E. 2.",
        "Er hat der Frau Angst gemacht.",
        "Weil die alte Frau Angst hatte.",
        "Fr. 500.– zahlen",
        "Frau A. hat Beschwerde erhoben.",
        "Was gilt, wenn die beschuldigte Person schweigt?",
        "Il ricorrente chiede il gratuito patrocinio.",
    ];

    /// Anonymisierungsregeln des Bundesgerichts, Art. 4 Abs. 1 lit. a, b und e.
    const BEHOERDEN_GEMEINDEN_ORTE: &[&str] = &[
        "Das Bundesgericht und der Regierungsrat des Kantons Uri entscheiden.",
        "Die Gemeinde will in Jens Wohnungen bauen.",
        "Wir fahren nach St. Moritz Ende Juli.",
        "Die Stadt Zürich und die Gemeinde Basel streiten.",
        "Der Kanton Basel-Stadt und Sankt Gallen.",
        "Die Bezirke Moesa und Maloja.",
    ];

    /// Art. 4 Abs. 1 lit. g: Namen der Rechtsvertreter der Parteien werden nicht markiert,
    /// aber nur, wenn der Satz die Person als Vertretung einer Partei zeigt.
    const LIT_G: &[(&str, &[&str])] = &[
        // Vertretung dem Wort nach.
        ("Verteidigerin Erika Beispiel beantragt Freispruch.", &[]),
        ("Hans Muster, Rechtsbeistand, schreibt.", &[]),
        ("Hans Muster (Parteivertreter) schreibt.", &[]),
        (
            "Me Jean Dupont, conseil de Marie Dupont, écrit.",
            &["Marie Dupont"],
        ),
        ("le conseil d'Erika Beispiel", &["Erika Beispiel"]),
        (
            "l'avv. Mario Rossi, patrocinatore di Anna Bianchi",
            &["Anna Bianchi"],
        ),
        // Verb der Vertretung nach dem Namen oder dem Anwaltswort.
        (
            "Rechtsanwältin Erika Beispiel vertritt Hans Muster.",
            &["Hans Muster"],
        ),
        (
            "Me Jean Dupont plaide pour Marie Dupont.",
            &["Marie Dupont"],
        ),
        (
            "L'avv. Mario Rossi rappresenta Anna Bianchi.",
            &["Anna Bianchi"],
        ),
        ("Hans Muster, Rechtsanwalt, vertritt die Klägerin.", &[]),
        // Possessivpronomen vor dem Anwaltswort, ohne Angabe nach Art. 5 lit. c DSG im Satz,
        // auch für die eigene Anwältin (Prüffälle fr-19 und it-19).
        ("Hans Muster, Ihr Anwalt", &[]),
        ("Jean Dupont, son avocat, a écrit.", &[]),
        ("Mario Rossi, il suo avvocato, scrive.", &[]),
        ("Mein Anwalt, RA Hans Muster, schreibt.", &[]),
        ("Hans Muster, unser Anwalt, schreibt.", &[]),
        ("Mon avocate, Me Sophie Rochat, a écrit.", &[]),
        (
            "Mon avocat, Me Julien Essai, me conseille d'accepter la proposition.",
            &[],
        ),
        ("Il mio avvocato, l'avv. Mario Rossi, scrive.", &[]),
        ("Mario Rossi, il mio avvocato, scrive.", &[]),
        (
            "Il mio avvocato, avv. Roberto Fittizio, dice che devo versare il saldo.",
            &[],
        ),
        // Partizip der Vertretung davor.
        ("vertreten durch RA Hans Muster", &[]),
        ("vertreten durch Frau Rechtsanwältin Erika Beispiel", &[]),
        ("vertreten durch Hans Muster, Rechtsanwalt", &[]),
        ("représentée par Me Jean Dupont", &[]),
        ("rappresentato dall'avv. Mario Rossi", &[]),
        // Partei nach dem Anwaltswort.
        ("Jean Dupont, l'avocat de la défense", &[]),
        ("Jean Dupont, avocat de la partie adverse", &[]),
        ("Hans Muster, Rechtsanwalt der Gegenpartei", &[]),
        ("Mario Rossi, avvocato della controparte", &[]),
        ("l'avocat de la partie adverse, Me Jean Dupont", &[]),
        ("le mandataire de la partie, Jean Dupont", &[]),
        // Als Bezeichnung mit Doppelpunkt (Prüffall fr-14).
        (
            "Partie adverse: Sandrine Exemple, Lausanne\nAvocate de la partie adverse: Me Valérie Essai, Genève",
            &["Sandrine Exemple"],
        ),
        ("Verteidiger: Hans Muster", &[]),
        ("Anwalt: RA Hans Muster", &["Hans Muster"]),
        // Ohne Vertretungsmerkmal wirkt das Anwaltswort wie ein Titel (E2).
        ("Hans Muster, Rechtsanwalt, schreibt.", &["Hans Muster"]),
        ("Hans Muster (Fürsprecher) schreibt.", &["Hans Muster"]),
        ("Frau Rechtsanwältin Erika Beispiel", &["Erika Beispiel"]),
        ("RA Hans Muster", &["Hans Muster"]),
        ("Me Jean Dupont", &["Jean Dupont"]),
        ("Rechtsanwalt lic. iur. Hans M. Muster", &["Hans M. Muster"]),
        ("Jean Dupont (avocat) écrit.", &["Jean Dupont"]),
        ("l'avv. Mario Rossi", &["Mario Rossi"]),
        ("Mario Rossi, avvocato", &["Mario Rossi"]),
        // Praxis (praxis-fr-it-1): Die Person ist selbst betroffen, keine Vertretung.
        (
            "Mon patient, Me Laurent Dupont, souffre d'une maladie de Crohn; rédige une lettre à son assurance.",
            &["Laurent Dupont"],
        ),
        (
            "Il mio paziente, l'avv. Marco Rezzonico, soffre di una malattia cronica e chiede un certificato.",
            &["Marco Rezzonico"],
        ),
        (
            "Notre collaboratrice Me Sophie Rochat, avocate-stagiaire, est en incapacité de travail et sous curatelle depuis mars.",
            &["Sophie Rochat"],
        ),
        (
            "M. Paul Rochat, mandataire pour cause d'inaptitude de sa mère, demande l'accès aux comptes; il est lui-même au bénéfice de l'aide sociale.",
            &["Paul Rochat"],
        ),
        (
            "Mein Patient, Rechtsanwalt Hans Muster, ist krank.",
            &["Hans Muster"],
        ),
        // Possessivpronomen mit einer Angabe nach Art. 5 lit. c DSG im Satz: Die Person ist
        // selbst betroffen (E2, w3-namen-p4: P01–P05, V53–V55 und ihre Minimalpaare oben).
        (
            "Mon avocat, Me Jean Dupont, est en incapacité de travail.",
            &["Jean Dupont"],
        ),
        (
            "Il mio avvocato, l'avv. Mario Rossi, è in malattia.",
            &["Mario Rossi"],
        ),
        (
            "Mein Anwalt, RA Hans Muster, ist arbeitsunfähig.",
            &["Hans Muster"],
        ),
        (
            "Notre avocate Me Sophie Rochat est sous curatelle.",
            &["Sophie Rochat"],
        ),
        (
            "Unser Anwalt Hans Muster bezieht Sozialhilfe.",
            &["Hans Muster"],
        ),
        (
            "Jean Dupont, son avocat, est en incapacité de travail.",
            &["Jean Dupont"],
        ),
        (
            "Mario Rossi, il mio avvocato, è in malattia.",
            &["Mario Rossi"],
        ),
        (
            "Mario Rossi, il suo avvocato, è in malattia.",
            &["Mario Rossi"],
        ),
        (
            "Hans Muster, Ihr Anwalt, ist arbeitsunfähig.",
            &["Hans Muster"],
        ),
        // Ob die Angabe die Anwältin betrifft, prüft promptscan nicht (explain grenzen).
        (
            "Mon avocat, Me Jean Dupont, dit que je suis en incapacité de travail.",
            &["Jean Dupont"],
        ),
        // Die Angabe in einem anderen Satz hebt die Vertretung nicht auf.
        (
            "Jean Dupont, son avocat, a écrit. Le prévenu est en incapacité de travail.",
            &[],
        ),
        // Ein Wort, das die Vertretung schon bezeichnet, nimmt immer aus (explain grenzen).
        (
            "Le défenseur Jean Dupont est en incapacité de travail.",
            &[],
        ),
        // Verben im übertragenen Gebrauch: keine Partei danach (w3-namen-p7).
        (
            "Rechtsanwalt Hans Muster vertritt die Auffassung, dass er wegen seiner Depression arbeitsunfähig ist.",
            &["Hans Muster"],
        ),
        (
            "RA Hans Muster vertritt die Ansicht, dass er krank ist.",
            &["Hans Muster"],
        ),
        ("Me Jean Dupont plaide coupable.", &["Jean Dupont"]),
        (
            "Me Jean Dupont plaide coupable; il est en détention provisoire.",
            &["Jean Dupont"],
        ),
        (
            "L'avv. Mario Rossi rappresenta un rischio: soffre di una depressione.",
            &["Mario Rossi"],
        ),
        (
            "Me Jean Dupont représente un danger pour lui-même: il souffre d'une dépression.",
            &["Jean Dupont"],
        ),
        (
            "Rechtsanwalt Hans Muster vertritt Erika Beispiel.",
            &["Erika Beispiel"],
        ),
        ("Me Jean Dupont plaide pour la recourante.", &[]),
        ("Me Jean Dupont représente la recourante.", &[]),
        ("L'avv. Mario Rossi rappresenta la ricorrente.", &[]),
        // «conseil»: auch Berater oder Rat; Vertretung nur mit Partei oder Partizip
        // (w3-namen-p2).
        (
            "Le conseil Jean Dupont est en incapacité de travail.",
            &["Jean Dupont"],
        ),
        (
            "Jean Dupont, conseil fiscal, est en incapacité de travail.",
            &["Jean Dupont"],
        ),
        ("Mon conseil, Me Jean Dupont, est malade.", &["Jean Dupont"]),
        ("assistée de son conseil, Me Jean Dupont", &[]),
        ("Me Jean Dupont, conseil de la recourante, écrit.", &[]),
        // «mandataire»: auch die Vorsorgebeauftragte nach Art. 360 CC; Vertretung nur mit
        // «partie» oder einer Verfahrensrolle danach (w3-namen-p3).
        (
            "Son mandataire, M. Paul Rochat, touche l'aide sociale.",
            &["Paul Rochat"],
        ),
        (
            "Leur mandataire, M. Paul Rochat, est sous curatelle.",
            &["Paul Rochat"],
        ),
        (
            "Notre mandataire, M. Paul Rochat, est en incapacité de travail.",
            &["Paul Rochat"],
        ),
        (
            "Le mandataire Paul Rochat représente sa mère; il touche lui-même l'aide sociale.",
            &["Paul Rochat"],
        ),
        (
            "Son mandataire, M. Paul Rochat, demande l'accès à ses comptes; il touche lui-même l'aide sociale.",
            &["Paul Rochat"],
        ),
        (
            "Son mandataire pour cause d'inaptitude, M. Paul Rochat, touche l'aide sociale.",
            &["Paul Rochat"],
        ),
        (
            "Le mandataire de Mme Rochat, M. Paul Rochat, touche l'aide sociale.",
            &["Rochat", "Paul Rochat"],
        ),
        (
            "Le mandataire de la partie, Jean Dupont, dépose une requête.",
            &[],
        ),
        ("le mandataire du recourant, Jean Dupont", &[]),
        // Die beruflich qualifizierte Vertretung nach Art. 68 Abs. 2 Bst. d ZPO (E2:
        // «mandataire professionnel»).
        (
            "Jean Dupont, mandataire professionnellement qualifié, dépose une requête.",
            &[],
        ),
        (
            "Hans Muster, beruflich qualifizierter Vertreter, reicht ein Gesuch ein.",
            &[],
        ),
        (
            "Mario Rossi, rappresentante professionalmente qualificato, presenta un'istanza.",
            &[],
        ),
        // «avocat» und «avvocato» lösen allein nichts aus (w3-namen-p8).
        (
            "Je cherche un avocat Spécialiste FSA en droit du travail.",
            &[],
        ),
        ("Cerco un avvocato Penalista a Lugano.", &[]),
        ("Cerco un avvocato Svizzero.", &[]),
        ("Quel avocat Genevois recommandez-vous ?", &[]),
        ("Avocat Conseil: Jean Dupont", &["Jean Dupont"]),
        ("l'avocat Dupont a écrit.", &[]),
        ("l'avocat Jean Dupont a écrit.", &["Jean Dupont"]),
        ("rappresentato dall'avvocato Mario Rossi", &[]),
    ];

    #[test]
    fn positive_beispiele() {
        pruefe_liste(POSITIV);
    }

    #[test]
    fn gegenbeispiele() {
        for text in NEGATIV {
            assert!(namen(text).is_empty(), "Text: «{text}»: {:?}", namen(text));
        }
    }

    #[test]
    fn behoerden_gemeinden_und_orte_ausgenommen() {
        for text in BEHOERDEN_GEMEINDEN_ORTE {
            assert!(namen(text).is_empty(), "Text: «{text}»: {:?}", namen(text));
        }
    }

    #[test]
    fn rechtsvertreter_ausgenommen() {
        pruefe_liste(LIT_G);
    }

    /// Ein Possessivpronomen vor dem Anwaltswort zeigt die Vertretung genau dann, wenn die
    /// Regeln art5c im Satz nichts finden (w3-namen-p4). Geprüft gegen das Ergebnis von
    /// art5c, nicht gegen eine feste Erwartung: «malade», «malato» und «krank» erkennt
    /// art5c-2 heute nicht (V37, V40, V42, V46); kommen sie in die Wortliste, wird der Name
    /// gemeldet, ohne dass dieser Test angepasst werden muss.
    #[test]
    fn besitz_und_angabe_nach_art5c_im_satz() {
        let mit_angabe = |s: &str| !crate::regeln::art5c::erkenne(&Text::neu(s)).is_empty();
        for (text, name) in [
            ("Mon avocat, Me Jean Dupont, est malade.", "Jean Dupont"),
            ("Jean Dupont, son avocat, est malade.", "Jean Dupont"),
            ("Mon avocat Jean Dupont est malade.", "Jean Dupont"),
            ("Mario Rossi, il mio avvocato, è malato.", "Mario Rossi"),
            ("Hans Muster, unser Anwalt, ist krank.", "Hans Muster"),
            (
                "Mon avocat, Me Jean Dupont, est en incapacité de travail.",
                "Jean Dupont",
            ),
            (
                "Mein Anwalt, RA Hans Muster, ist arbeitsunfähig.",
                "Hans Muster",
            ),
            (
                "Mario Rossi, il suo avvocato, è in malattia.",
                "Mario Rossi",
            ),
            ("Mon avocat, Me Jean Dupont, a écrit.", "Jean Dupont"),
            ("Mein Anwalt, RA Hans Muster, schreibt.", "Hans Muster"),
            ("Mario Rossi, il suo avvocato, scrive.", "Mario Rossi"),
        ] {
            let erwartet: Vec<String> = if mit_angabe(text) {
                vec![name.to_string()]
            } else {
                Vec::new()
            };
            assert_eq!(namen(text), erwartet, "«{text}»");
        }
        // Gegenprobe: Die Sätze mit fester Angabe sind auch für art5c Angaben.
        for text in [
            "Mon avocat, Me Jean Dupont, est en incapacité de travail.",
            "Mein Anwalt, RA Hans Muster, ist arbeitsunfähig.",
            "Mario Rossi, il suo avvocato, è in malattia.",
        ] {
            assert!(mit_angabe(text), "«{text}»");
        }
    }

    /// Verfahrensrollen mit Namen (E1, praxis-de-3, recht-wortlisten-8).
    const ROLLEN_MIT_NAMEN: &[(&str, &[&str])] = &[
        (
            "Der Beschwerdeführer A. Meier, geboren am 4. April 1983, leidet an einer psychischen Störung.",
            &["A. Meier"],
        ),
        ("Der Beschwerdeführer A. B. Meier rügt.", &["A. B. Meier"]),
        (
            "Der Beschwerdeführer Meier ist seit Mai arbeitsunfähig.",
            &["Meier"],
        ),
        (
            "Der Beschwerdeführer Brunner rügt eine Verletzung des rechtlichen Gehörs.",
            &["Brunner"],
        ),
        (
            "Der Beklagte Brunner bestreitet die Forderung.",
            &["Brunner"],
        ),
        (
            "Die Klägerin Erika Beispiel verlangt Schadenersatz.",
            &["Erika Beispiel"],
        ),
        ("Der Gesuchsteller Hans Muster ersucht.", &["Hans Muster"]),
        ("Die Beschwerdegegnerin Brunner schweigt.", &["Brunner"]),
        ("Der Beschuldigte Brunner schweigt.", &["Brunner"]),
        ("Le recourant A. Brunner conteste.", &["A. Brunner"]),
        ("Il ricorrente A. Rossi contesta.", &["A. Rossi"]),
        // Initialen allein: die Form anonymisierter Urteile.
        ("Die Beschwerdeführerin A. hat Beschwerde erhoben.", &[]),
        ("Der Beschwerdeführer A. Das Gericht entscheidet.", &[]),
        // Mehrdeutiger Nachname nach Initiale: zugleich ein Wort der Gesetzestexte.
        ("Der Beschwerdeführer A. Muster rügt.", &[]),
        // Kleingeschrieben ist «beklagte» ein Verb oder Partizip, keine Rolle.
        ("Sie beklagte Brunner gegenüber den Lärm.", &[]),
    ];

    #[test]
    fn verfahrensrollen_mit_namen_und_initialen() {
        pruefe_liste(ROLLEN_MIT_NAMEN);
        // Ohne Namen: Personenbezug.
        assert_eq!(
            bezuege("Die Beschwerdeführerin A. hat Beschwerde erhoben."),
            vec!["Beschwerdeführerin"]
        );
        assert_eq!(
            bezuege("Der Beschwerdeführer ist seit Mai arbeitsunfähig."),
            vec!["Beschwerdeführer"]
        );
        assert_eq!(
            bezuege("Die Klägerin war bei der Kündigung schwanger."),
            vec!["Klägerin"]
        );
        assert!(bezuege("Sie beklagte den Lärm.").is_empty());
    }

    /// Die Beispiele aus explain personenbezug (Regel kontext) und explain name, «erkennt».
    #[test]
    fn beispiele_aus_explain() {
        assert_eq!(bezuege("der Beschwerdeführer"), vec!["Beschwerdeführer"]);
        assert_eq!(bezuege("le recourant"), vec!["recourant"]);
        assert_eq!(bezuege("l'imputato"), vec!["imputato"]);
        assert_eq!(
            bezuege("die beschuldigte Person"),
            vec!["beschuldigte Person"]
        );
        assert_eq!(namen("Müller Andreas, Lager"), vec!["Müller Andreas"]);
        assert_eq!(namen("Muster;Hans"), vec!["Muster;Hans"]);
        assert_eq!(namen("KELLER Sabine"), vec!["KELLER Sabine"]);
        assert_eq!(namen("der Beschwerdeführer A. Meier"), vec!["A. Meier"]);
        // grenzen
        assert!(namen("Muster sagt").is_empty());
        assert!(namen("Muster Hans hat angerufen").is_empty());
        assert!(namen("Keller Sabine").is_empty());
        assert!(namen("Beispiel;Erika;Lager").is_empty());
        assert!(namen("Baden;Anna;Lager").is_empty());
        assert_eq!(namen("Keller;Anna;Lager"), vec!["Keller;Anna"]);
        assert!(namen("2. Wahl Peter").is_empty());
        assert!(namen("Liebe Anna, Lieber Peter,").is_empty());
        assert!(namen("Montag Peter, Dienstag Anna").is_empty());
        assert!(namen("Gentile Mario, Lugano").is_empty());
        assert_eq!(namen("Gentile;Mario"), vec!["Gentile;Mario"]);
        assert_eq!(namen("GENTILE Mario"), vec!["GENTILE Mario"]);
        assert!(namen("Lieber Peter, ich komme").is_empty());
        assert!(namen("Müller Andreas, Ich bin krank.").is_empty());
        assert!(namen("Muster, Hans").is_empty());
        assert!(namen("WU Anna").is_empty());
        assert!(namen("Frau Beispiel").is_empty());
        assert!(namen("Frau A. Ganz").is_empty());
        assert_eq!(namen("RA Hans Muster"), vec!["Hans Muster"]);
        assert_eq!(namen("Me Jean Dupont"), vec!["Jean Dupont"]);
        assert!(namen("Mon avocat, Me Jean Dupont, a écrit.").is_empty());
        assert!(namen("Jean Dupont, son avocat, a écrit.").is_empty());
        assert_eq!(
            namen("Mon avocat, Me Jean Dupont, est en incapacité de travail."),
            vec!["Jean Dupont"]
        );
        assert_eq!(
            namen("Jean Dupont, son avocat, est en incapacité de travail."),
            vec!["Jean Dupont"]
        );
        assert_eq!(
            namen("Mon avocat, Me Jean Dupont, dit que je suis en incapacité de travail."),
            vec!["Jean Dupont"]
        );
        assert!(namen("Le défenseur Jean Dupont est en incapacité de travail.").is_empty());
        assert!(namen("l'avocat Dupont").is_empty());
        assert!(namen("La Dre Rochat").is_empty());
        // methode
        assert_eq!(namen("Herr H. Muster"), vec!["H. Muster"]);
        assert!(namen("der Beschwerdeführerin A. Daher ist").is_empty());
        for wort in ["Liebe", "Montag", "Bonjour", "Gentile"] {
            assert!(DATEN.alltag.contains(&schluessel(wort)), "{wort}");
            assert!(DATEN.nachnamen.contains_key(&schluessel(wort)), "{wort}");
        }
        assert_eq!(namen("Hans Muster, Ihr Anwalt"), Vec::<String>::new());
        assert!(namen("Jean Dupont, avocat de la partie adverse").is_empty());
        assert_eq!(
            namen("Me Jean Dupont plaide pour Marie Dupont."),
            vec!["Marie Dupont"]
        );
    }

    /// Nach Initialen kein Satzwort der Sprache des Auslösers (w3-namen-p6, J01–J05), mit
    /// Minimalpaaren in DE, FR und IT.
    #[test]
    fn initialen_und_satzwoerter() {
        pruefe_liste(&[
            (
                "Das Obergericht verurteilte den Beschwerdeführer A. Ferner ordnete es eine Massnahme an.",
                &[],
            ),
            (
                "Die IV-Stelle verneinte den Anspruch der Beschwerdeführerin A. Ferner sei sie seit 2020 arbeitsunfähig.",
                &[],
            ),
            (
                "Zu prüfen ist die Rente der Beschwerdeführerin A. Daher ist ihre Arbeitsunfähigkeit massgebend.",
                &[],
            ),
            (
                "Die Kosten trägt der Beschwerdegegner B. Auch er bezieht Sozialhilfe.",
                &[],
            ),
            ("Frau A. Sie hat angerufen.", &[]),
            ("Frau A. Du bist ein Assistent.", &[]),
            ("Le recourant A. Le Tribunal cantonal a jugé.", &[]),
            ("Le recourant A. Car il a recouru.", &[]),
            ("Il ricorrente A. Egli sostiene il contrario.", &[]),
            ("Il ricorrente A. Di regola si decide così.", &[]),
            // Minimalpaare: ein Nachname, der kein Satzwort ist.
            ("Der Beschwerdeführer A. Meier rügt.", &["A. Meier"]),
            ("Frau A. Meier hat angerufen.", &["A. Meier"]),
            ("Le recourant A. Rochat conteste.", &["A. Rochat"]),
            ("Il ricorrente A. Rossi contesta.", &["A. Rossi"]),
            // Ein Satzwort einer anderen Sprache zählt: «Egli» nach deutscher Anrede.
            ("Frau A. Egli hat angerufen.", &["A. Egli"]),
            ("Der Beschwerdeführer A. Egli rügt.", &["A. Egli"]),
            // Ein Nachname aus mehreren Wörtern mit einem Satzwort vorn.
            ("Mme A. Le Coultre a écrit.", &["A. Le Coultre"]),
            ("Il ricorrente A. Di Stefano contesta.", &["A. Di Stefano"]),
            (
                "La recourante A. du Pasquier conteste.",
                &["A. du Pasquier"],
            ),
        ]);
        // Ohne Namen bleibt der Personenbezug der Rolle.
        assert_eq!(
            bezuege(
                "Das Obergericht verurteilte den Beschwerdeführer A. Ferner ordnete es eine Massnahme an."
            ),
            vec!["Beschwerdeführer"]
        );
    }

    /// Jedes Leerzeichen aus `ist_leerraum` vor einem Wort am Zellenanfang (w3-namen-p1):
    /// kein Absturz, und der Name wird gefunden.
    #[test]
    fn geschuetzte_und_schmale_leerzeichen() {
        for zeichen in [' ', '\t', '\u{00A0}', '\u{202F}', '\u{2009}', '\u{2007}'] {
            for (roh, erwartet) in [
                (
                    format!("-{zeichen}Keller Sabine, Lager"),
                    vec!["Keller Sabine"],
                ),
                (
                    format!("Mme{zeichen}Rochat est en incapacité de travail."),
                    vec!["Rochat"],
                ),
                (
                    format!("Die Miete beträgt CHF{zeichen}1200 pro Monat."),
                    vec![],
                ),
                (
                    format!("Gemäss Art.{zeichen}5 lit.{zeichen}c DSG gilt das."),
                    vec![],
                ),
                (
                    format!("Question{zeichen}: ma collaboratrice est-elle malade{zeichen}?"),
                    vec![],
                ),
            ] {
                assert_eq!(namen(&roh), erwartet, "{zeichen:?}: «{roh}»");
            }
        }
    }

    /// Grussformeln, Wochentage und Alltagswörter am Zeilenanfang; eine Zelle, die ein Satz
    /// ist (w3-namen-p5, E4). Minimalpaare in DE, FR und IT.
    #[test]
    fn listen_alltagswoerter_und_saetze() {
        for text in [
            "Lieber Peter, Ich bin seit Montag arbeitsunfähig.",
            "Liebe Anna, Meine Mutter bezieht Sozialhilfe.",
            "Gentile Mario, Le scrivo perché sono in malattia.",
            "Bonjour Pierre, Je suis en incapacité de travail depuis lundi.",
            "Liebe Anna, Vielen Dank für deine Hilfe.",
            "Buongiorno Marco, Le scrivo per la pratica.",
            "Caro Marco, Grazie per l'aiuto.",
            "Liebe Anna, Lieber Peter,\nHerzlichen Dank.",
            "Montag Peter, Dienstag Anna, Mittwoch Hans",
            "Bonjour Pierre, Merci pour ton message.",
            "Traktanden:\n1. Begrüssung\n2. Wahl Peter\n3. Diverses",
            "Messer Hans, Klinge 5 cm",
            "Glück Anna, Pech Peter",
            "Recht Anna, Unrecht Peter",
            "Caro Marco, Caro Paolo,\nGrazie.",
            "Bonjour Anna, Bonjour Pierre,\nMerci.",
            // «Sabato» und «Fortuna» sind auch mehrdeutige Vornamen; mit einem Nachnamen aus
            // dem Register danach («Sabato Marco») findet sie der Vornamenpfad.
            "Fortuna Giulia, Sfortuna Pietro",
            "Sabato Giulia, Domenica Pietro",
            "Müller Andreas, Ich bin krank.",
            "Dupont Jean, Je suis malade.",
            "Rossi Mario, Sono in malattia.",
            "Müller Andreas, Bin ich krank?",
        ] {
            assert!(namen(text).is_empty(), "«{text}»: {:?}", namen(text));
        }
        // Minimalpaare: eine Zelle, die kein Satz ist; ein eindeutiger Nachname; über zwei
        // Zellen oder in Versalien.
        pruefe_liste(&[
            ("Müller Andreas, Lager", &["Müller Andreas"]),
            ("Dupont Jean, Lausanne", &["Dupont Jean"]),
            ("Rossi Mario, Lugano", &["Rossi Mario"]),
            ("Müller Andreas, Seestr. 1", &["Müller Andreas"]),
            ("Müller Andreas, 12 Tage", &["Müller Andreas"]),
            (
                "Keller Anna, Müller Peter",
                &["Keller Anna", "Müller Peter"],
            ),
            ("Bonjour;Pierre;Lausanne", &["Bonjour;Pierre"]),
            ("BONJOUR Pierre, Lausanne", &["BONJOUR Pierre"]),
            ("CARO Marco, Lugano", &["CARO Marco"]),
        ]);
    }

    /// Titel ohne Punkt und weibliche Abkürzungen (E3, praxis-fr-it-2), mit Gegenprobe ohne
    /// Titel.
    #[test]
    fn titel_fr_it() {
        for (titel, satz) in [
            (
                "La dott.ssa",
                "Pedrazzini è incinta e assente per malattia.",
            ),
            ("La dr.ssa", "Pedrazzini è assente."),
            ("La prof.ssa", "Galli è assente per malattia."),
            ("L'ing.", "Moretti è assente per malattia."),
            ("L'arch.", "Moretti è assente per malattia."),
            ("Le Dr", "Rochat est en incapacité de travail."),
            ("Le Pr", "Rochat est en incapacité de travail."),
        ] {
            let nachname = satz.split(' ').next().unwrap_or_default();
            assert_eq!(namen(&format!("{titel} {satz}")), vec![nachname], "{titel}");
            assert!(namen(satz).is_empty(), "{satz}");
        }
        // Wortgrenzen: «Dr» und «Pr» nicht in «Drapeau» und «Prix», «dott.ssa» nicht vor
        // einem Satzende.
        assert!(namen("Le Drapeau Rochat flotte.").is_empty());
        assert!(namen("Le Prix Rochat est décerné.").is_empty());
        assert!(namen("Ho parlato con la dott.ssa. Pedrazzini è assente.").is_empty());
    }

    /// Listen (E4, praxis-de-2, technik-code-4).
    const LISTEN: &[(&str, &[&str])] = &[
        (
            "Müller Andreas, Lager, 12 Tage, Grippe\nKeller Sabine, Buchhaltung, 31 Tage, Burnout\nGashi Arben, Produktion, 58 Tage, Unfall, SUVA-Fall",
            &["Müller Andreas", "Keller Sabine", "Gashi Arben"],
        ),
        (
            "Zi. 15: Frei Claudia, 45, Diagnose F10.2, Entzug\nZi. 16: Keller Sabine, 38, Diagnose F32.1",
            &["Frei Claudia", "Keller Sabine"],
        ),
        (
            "Müller Andreas; 756.1234.5678.97; 6'200",
            &["Müller Andreas"],
        ),
        (
            "Patient: KELLER Sabine, geb. 12.03.1985, Diagnose F32.1",
            &["KELLER Sabine"],
        ),
        ("Gestern rief KELLER Sabine an.", &["KELLER Sabine"]),
        ("Muster;Hans;756.1234.5678.97", &["Muster;Hans"]),
        ("Hans;Muster;756.1234.5678.97", &["Hans;Muster"]),
        ("Muster\tHans\t756.1234.5678.97", &["Muster\tHans"]),
        ("Muster, Hans, 756.1234.5678.97", &["Muster, Hans"]),
        ("| Keller | Sabine | 38 |", &["Keller | Sabine"]),
        (
            "- Keller Sabine\n- Müller Andreas",
            &["Keller Sabine", "Müller Andreas"],
        ),
        ("1. Keller Sabine", &["Keller Sabine"]),
        ("Dupont, Jean, 1985", &["Dupont, Jean"]),
        ("ROSSI Mario, nato il 3.4.1980", &["ROSSI Mario"]),
        (
            "Anwesend: Brunner Peter, Gashi Arben.",
            &["Brunner Peter", "Gashi Arben"],
        ),
    ];

    /// Gegenbeispiele zu Listen: Satzanfänge, Spaltenköpfe, Ortsnamen, Wörter, die zugleich
    /// Nachnamen sind, und Paare aus zwei Vornamen.
    const LISTEN_GEGEN: &[&str] = &[
        "Lieber Peter, ich komme morgen.",
        "Liebe Anna,\nich komme morgen.",
        "Bonjour Pierre, comment vas-tu ?",
        "Gut, Peter, komm.",
        "Nein, Peter",
        "Name;Vorname;AHV-Nr.",
        "Nachname;Vorname;Geburtsdatum",
        "Baden;Anna;12 Tage",
        "Thun, Anna, Lager",
        "Peter;Anna;Hans",
        "Muster Hans hat angerufen.",
        "Gestern hat Muster Hans angerufen.",
        "Keller Sabine kam gestern.",
        "Basel, 3. Mai 2026",
        "Zug, 3. Mai 2026",
        "OK Peter, mach das.",
    ];

    #[test]
    fn listen() {
        pruefe_liste(LISTEN);
        for text in LISTEN_GEGEN {
            assert!(namen(text).is_empty(), "Text: «{text}»: {:?}", namen(text));
        }
        // Minimalpaare: dieselbe Zeile mit Trenner danach, mit Nachname statt Ortsname, mit
        // drei statt zwei Versalien.
        assert!(namen("Lieber Peter").is_empty());
        assert_eq!(namen("Kanton GE Anna Meier"), vec!["Anna Meier"]);
        // «Wu» steht im Register, zwei Versalien zählen trotzdem nicht.
        assert!(namen("WU Anna kommt.").is_empty());
        assert_eq!(namen("KELLER Anna kommt."), vec!["KELLER Anna"]);
        assert_eq!(namen("Keller Sabine, Lager"), vec!["Keller Sabine"]);
        assert_eq!(namen("Keller, Anna, Lager"), vec!["Keller, Anna"]);
        // Die Spannweite des Namens.
        let roh = "Lohn\nMuster;Hans;756.1234.5678.97";
        let text = Text::neu(roh);
        let t = erkenne(&text);
        assert_eq!((t[0].start, t[0].ende), (5, 16));
        assert_eq!(
            t[0].beleg.as_ref().map(|b| b.begriff.as_str()),
            Some("Hans")
        );
    }

    #[test]
    fn gegenprobe_die_regel_findet_etwas() {
        // Eine Regel, die nichts meldet, bestünde die Gegenbeispiele; hier nicht.
        let gefunden = POSITIV
            .iter()
            .filter(|(text, _)| !namen(text).is_empty())
            .count();
        assert_eq!(gefunden, POSITIV.len());
        assert!(!namen("Rechtsanwältin Erika Beispiel vertritt Hans Muster.").is_empty());
    }

    #[test]
    fn spannweite_nur_der_name() {
        let roh = "Unser Mitarbeiter Hans Muster, geb. 12.03.1985";
        let text = Text::neu(roh);
        let t = erkenne(&text);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].start, t[0].ende), (18, 29));
        assert_eq!(t[0].art, Art::Identifikator);
        assert!(t[0].zusatz.is_none());
        let beleg = t[0].beleg.as_ref().expect("Beleg");
        assert_eq!(beleg.datei, VORNAMEN);
        assert_eq!(beleg.begriff, "Hans");
        assert_eq!(beleg.fassung, "2026-08-21");
        assert!(beleg.url.starts_with("https://opendata.swiss/"));
    }

    #[test]
    fn rolle_ohne_namen_ist_personenbezug() {
        for (roh, erwartet) in [
            (
                "Was gilt, wenn die beschuldigte Person schweigt?",
                "beschuldigte Person",
            ),
            (
                "la partie plaignante demande réparation",
                "partie plaignante",
            ),
            ("il ricorrente", "ricorrente"),
        ] {
            assert_eq!(bezuege(roh), vec![erwartet], "Text: «{roh}»");
            assert!(namen(roh).is_empty(), "Text: «{roh}»");
            let text = Text::neu(roh);
            let t = erkenne(&text);
            assert!(t.iter().all(|t| t.art == Art::Personenbezug));
            assert_eq!(t[0].beleg.as_ref().map(|b| b.datei.as_str()), Some(ROLLEN));
        }
        // Mit Namen: kein Personenbezug, sondern der Name.
        assert!(bezuege("Le prévenu Dupont conteste.").is_empty());
    }

    #[test]
    fn jede_verfahrensrolle_mit_und_ohne_namen() {
        let lex = lexikon::lexikon(ROLLEN);
        for e in &lex.eintraege {
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                assert_eq!(namen(&format!("{form} Meier")), vec!["Meier"], "{form}");
                assert_eq!(bezuege(form), vec![form.clone()], "{form}");
            }
        }
    }

    #[test]
    fn jede_anrede_und_jeder_titel() {
        let lex = lexikon::lexikon(ANREDEN);
        for e in &lex.eintraege {
            let art = e.merkmal.as_deref().and_then(Wortart::aus_merkmal);
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                if art == Some(Wortart::Heilig) {
                    // Ohne Sperre wäre «Moritz Meier» ein Name.
                    assert!(namen(&format!("{form} Moritz Meier")).is_empty(), "{form}");
                } else {
                    assert_eq!(namen(&format!("{form} Meier")), vec!["Meier"], "{form}");
                }
            }
        }
    }

    /// Ein Anwaltswort je Sprache, ein Partizip der Vertretung und ein Bezug auf die Partei.
    fn sprachbeispiel(sprache: lexikon::Sprache) -> (&'static str, &'static str, &'static str) {
        match sprache {
            lexikon::Sprache::Fr => ("avocat", "représenté par", "de"),
            lexikon::Sprache::It => ("avvocato", "rappresentato da", "di"),
            _ => ("Anwalt", "vertreten durch", "von"),
        }
    }

    /// Hängt `b` an `a`, ohne Leerzeichen nach einem Apostroph («dall'avv.»).
    fn verbinde(a: &str, b: &str) -> String {
        if a.ends_with(['\'', '\u{2019}']) {
            format!("{a}{b}")
        } else {
            format!("{a} {b}")
        }
    }

    #[test]
    fn jede_rechtsvertretung_und_funktion() {
        for e in &lexikon::lexikon(RECHTSVERTRETUNG).eintraege {
            let art = e.merkmal.as_deref().and_then(Wortart::aus_merkmal);
            let (_, vertreten, _) = sprachbeispiel(e.sprache);
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                match art {
                    Some(Wortart::Rechtsvertretung) => {
                        assert!(namen(&format!("{form} Hans Muster")).is_empty(), "{form}");
                        assert!(namen(&format!("Hans Muster, {form}")).is_empty(), "{form}");
                    }
                    Some(Wortart::Anwalt | Wortart::AnwaltDe) => {
                        // Ohne Vertretungsmerkmal ein Titel, mit einem die Ausnahme.
                        assert_eq!(
                            namen(&format!("{form} Hans Muster")),
                            vec!["Hans Muster"],
                            "{form}"
                        );
                        assert_eq!(
                            namen(&format!("Hans Muster, {form}")),
                            vec!["Hans Muster"],
                            "{form}"
                        );
                        assert!(
                            namen(&format!("{vertreten} {form} Hans Muster")).is_empty(),
                            "{form}"
                        );
                    }
                    Some(Wortart::AnwaltWort) => {
                        // Kein Auslöser; mit einem Vertretungsmerkmal die Ausnahme.
                        assert!(namen(&format!("un {form} Muster")).is_empty(), "{form}");
                        assert_eq!(
                            namen(&format!("Hans Muster, {form}")),
                            vec!["Hans Muster"],
                            "{form}"
                        );
                        assert!(
                            namen(&format!("{vertreten} {form} Hans Muster")).is_empty(),
                            "{form}"
                        );
                    }
                    Some(Wortart::Beratung) => {
                        // Nur eine Partei danach oder ein Partizip davor zeigt die Vertretung.
                        assert_eq!(
                            namen(&format!("Hans Muster, {form}, schreibt.")),
                            vec!["Hans Muster"],
                            "{form}"
                        );
                        assert_eq!(
                            namen(&format!("Hans Muster, son {form}, schreibt.")),
                            vec!["Hans Muster"],
                            "{form}"
                        );
                        assert!(
                            namen(&format!("Hans Muster, {form} de la recourante")).is_empty(),
                            "{form}"
                        );
                        assert!(
                            namen(&format!("{vertreten} son {form} Hans Muster")).is_empty(),
                            "{form}"
                        );
                    }
                    Some(Wortart::Beauftragt) => {
                        // Nur «partie» oder eine Verfahrensrolle danach zeigt die Vertretung,
                        // kein Possessivpronomen und kein Partizip (Art. 360 CC).
                        for satz in [
                            format!("Hans Muster, {form}, schreibt."),
                            format!("Hans Muster, son {form}, schreibt."),
                            format!("{vertreten} {form} Hans Muster"),
                            format!("Son {form}, Hans Muster, représente sa mère."),
                        ] {
                            assert_eq!(namen(&satz), vec!["Hans Muster"], "«{satz}»");
                        }
                        assert!(
                            namen(&format!("Hans Muster, {form} de la partie")).is_empty(),
                            "{form}"
                        );
                        assert!(
                            namen(&format!("le {form} de la recourante, Hans Muster")).is_empty(),
                            "{form}"
                        );
                    }
                    _ => panic!("{form}: Merkmal fehlt"),
                }
            }
        }
        for e in &lexikon::lexikon(FUNKTIONEN).eintraege {
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                assert!(namen(&format!("Frau {form}")).is_empty(), "{form}");
                assert_eq!(
                    namen(&format!("Herr {form} Meier")),
                    vec!["Meier"],
                    "{form}"
                );
            }
        }
        for e in &lexikon::lexikon(BEGLEITER).eintraege {
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                assert!(namen(&format!("{form} Frau Angst")).is_empty(), "{form}");
                assert_eq!(
                    namen(&format!("{form} Frau Meier")),
                    vec!["Meier"],
                    "{form}"
                );
            }
        }
    }

    /// Jedes Vertretungsmerkmal wirkt, und ohne es wird der Name gemeldet (Minimalpaare).
    #[test]
    fn jedes_vertretungsmerkmal() {
        for e in &lexikon::lexikon(VERTRETUNG).eintraege {
            let merkmal = e
                .merkmal
                .as_deref()
                .and_then(Vertretungsmerkmal::aus_merkmal);
            let (anwalt, _, bezug) = sprachbeispiel(e.sprache);
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                let (mit, ohne, partei): (String, String, &[&str]) = match merkmal {
                    Some(Vertretungsmerkmal::Vertreten) => (
                        verbinde(form, "RA Hans Muster"),
                        "RA Hans Muster".to_string(),
                        &[],
                    ),
                    Some(Vertretungsmerkmal::Vertritt) => {
                        // Ohne Partei nach dem Verb keine Vertretung (übertragener Gebrauch).
                        let bild = format!("RA Hans Muster {form} Lager.");
                        assert_eq!(namen(&bild), vec!["Hans Muster"], "«{bild}»");
                        (
                            format!("RA Hans Muster {form} Anna Meier."),
                            "RA Hans Muster sagt Anna Meier.".to_string(),
                            &["Anna Meier"],
                        )
                    }
                    Some(Vertretungsmerkmal::Besitz) => {
                        // Mit einer Angabe nach Art. 5 lit. c DSG im Satz ist die Person selbst
                        // betroffen.
                        let betroffen =
                            format!("Hans Muster, {form} {anwalt}, ist arbeitsunfähig.");
                        assert_eq!(namen(&betroffen), vec!["Hans Muster"], "«{betroffen}»");
                        // Nach dem Anwaltswort verbindet es ihn mit der Partei («Anwalt
                        // meiner Mandantin»).
                        let partei = format!("Hans Muster, {anwalt} {form} Mandantin");
                        assert!(namen(&partei).is_empty(), "«{partei}»");
                        let ohne_partei = format!("Hans Muster, {anwalt} Mandantin");
                        assert_eq!(namen(&ohne_partei), vec!["Hans Muster"], "«{ohne_partei}»");
                        (
                            format!("Hans Muster, {form} {anwalt}"),
                            format!("Hans Muster, {anwalt}"),
                            &[],
                        )
                    }
                    Some(Vertretungsmerkmal::Bezug) => (
                        format!(
                            "Hans Muster, {}",
                            verbinde(&format!("{anwalt} {form}"), "Gegenpartei")
                        ),
                        format!("Hans Muster, {anwalt} Gegenpartei"),
                        &[],
                    ),
                    Some(Vertretungsmerkmal::Partei) => (
                        format!("Hans Muster, {anwalt} {bezug} {form}"),
                        format!("Hans Muster, {anwalt} {bezug} Lager"),
                        &[],
                    ),
                    None => panic!("{form}: Merkmal fehlt"),
                };
                assert_eq!(namen(&mit), partei, "mit «{form}»: «{mit}»");
                assert!(
                    namen(&ohne).iter().any(|n| n == "Hans Muster"),
                    "ohne «{form}»: «{ohne}»"
                );
            }
        }
    }

    #[test]
    fn ueber_pruefe_gelb_und_mit_rolle_rot() {
        let bericht = crate::pruefe(
            "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, fehlt.",
            &Optionen::default(),
        );
        let name = bericht
            .funde
            .iter()
            .find(|f| f.regel == "name")
            .expect("Fund");
        assert_eq!(
            (name.stelle.as_str(), name.stufe),
            ("Hans Muster", Stufe::Gelb)
        );

        let bericht = crate::pruefe(
            "Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will …",
            &Optionen {
                rolle: Some(Rolle::Lawyer),
                ziel: Ziel::Cloud,
            },
        );
        let name = bericht
            .funde
            .iter()
            .find(|f| f.regel == "name")
            .expect("Fund");
        assert_eq!(
            (name.stelle.as_str(), name.stufe),
            ("Erika Beispiel", Stufe::Rot)
        );
    }

    /// Befund technik-1, Entscheid C1: die Sätze l01 bis l28 und a01 bis a18 der Kritik
    /// (kritik/technik/f-le.json, f-le2.json, kritik/technik-gegen/f1-le.json), dazu b06,
    /// b14 und b20 (f1b-le.json).
    const ARTIKEL_LE: &[(&str, &[&str])] = &[
        ("Le Grand Conseil a adopté la loi sur les impôts.", &[]),
        (
            "Le Tribunal fédéral a jugé que le délai était respecté.",
            &[],
        ),
        ("Le Conseil d'État a décidé de recourir.", &[]),
        ("Le Ministère public a ouvert une instruction.", &[]),
        ("Le Locle et Le Landeron ont fusionné leurs services.", &[]),
        ("Le Mont-sur-Lausanne a adopté un règlement.", &[]),
        ("Le Pont et Le Sentier sont des villages.", &[]),
        ("Al Consiglio di Stato spetta la decisione.", &[]),
        ("Le Juge unique a rejeté la requête.", &[]),
        // Am Satzanfang auch ein echter Nachname (Grenze in explain name).
        ("Le Roux a signé le contrat.", &[]),
        ("Le Code civil prévoit un délai.", &[]),
        ("Le Gouvernement jurassien a répondu.", &[]),
        ("Le Bon de commande est signé.", &[]),
        ("Le Président du tribunal a statué.", &[]),
        (
            "Le Brassus et Le Lieu se trouvent dans la Vallée de Joux.",
            &[],
        ),
        ("El Hadj Diop a recouru.", &["El Hadj Diop"]),
        ("Le Maire de la commune a refusé le permis.", &[]),
        ("Le Rapport du Conseil fédéral est clair.", &[]),
        (
            "Le Grand Conseil a adopté la loi. Quel est le délai référendaire?",
            &[],
        ),
        ("Selon la loi, le Grand Conseil fixe le budget.", &[]),
        ("Le Grand-Saconnex a adopté un plan de quartier.", &[]),
        (
            "Le Grand Conseil doit-il approuver le budget avant le 31 décembre?",
            &[],
        ),
        ("Le Maire et le Conseil municipal ont signé.", &[]),
        ("LE GRAND CONSEIL DU CANTON DE VAUD décrète:", &[]),
        ("Le Grand Conseil a voté. Le Grand Conseil a refusé.", &[]),
        ("Le Juge, Le Greffier et Le Bâtonnier sont présents.", &[]),
        ("Le Préfet a convoqué les parties.", &[]),
        ("Le Président du Grand Conseil a ouvert la séance.", &[]),
        (
            "Le Tribunal fédéral a jugé que le recours est irrecevable.",
            &[],
        ),
        (
            "Le Conseil d'État a décidé de ne pas entrer en matière.",
            &[],
        ),
        (
            "Quel est le rôle du Grand Conseil? Le Grand Conseil vote le budget.",
            &[],
        ),
        ("Le Grand Conseil vaudois a refusé l'initiative.", &[]),
        ("Il Gran Consiglio ha approvato la legge.", &[]),
        ("La Grand-Rue est fermée.", &[]),
        ("Le Syndic de Lausanne a parlé.", &[]),
        ("Le Grand Conseil genevois siège demain.", &[]),
        (
            "Le Grand Conseil peut-il modifier la loi sur l'organisation judiciaire?",
            &[],
        ),
        (
            "Le Grand Conseil a adopté la loi sur la santé. Le traitement des données de santé est-il conforme?",
            &[],
        ),
        ("Le Procureur général a fait recours.", &[]),
        ("Le Canton de Vaud est compétent.", &[]),
        ("Di Maio ha firmato.", &["Di Maio"]),
        // Mitten im Satz: vor einer Amtsbezeichnung der Artikel, sonst ein Name.
        ("Hier, Le Grand Conseil a voté.", &[]),
        ("Selon Le Maire, la séance est levée.", &[]),
        ("Pour Le Canton de Vaud, le délai est échu.", &[]),
        ("Ensuite, Le Procureur général a fait recours.", &[]),
        ("Hier, Le Roux a signé.", &["Le Roux"]),
        // Grenze in explain name: Ortschaft mitten im Satz.
        (
            "Il visite les villages Le Pont et Le Sentier.",
            &["Le Pont"],
        ),
        // Nach einem echten Vornamen bleibt die Amtsbezeichnung ein Nachname, nach einer
        // Anrede «Le» ein Teil des Namens.
        ("Pierre Maire a signé.", &["Pierre Maire"]),
        ("Monsieur Le Grand a signé.", &["Le Grand"]),
        // Ein weicher Zeilenumbruch beginnt keinen Satz (Prüferproblem c-erkennung-p3): auch
        // nach einer Überschrift ohne Satzzeichen ist «Le» dann ein Vorname (Grenze in explain
        // name).
        ("Considérant\nLe Roux a signé.", &["Le Roux"]),
        (
            "J'ai rencontré hier\nLe Roux Pierre, mon client, au tribunal.",
            &["Le Roux Pierre"],
        ),
        (
            "Mon client est\nLe Roux Pierre, il a été condamné.",
            &["Le Roux Pierre"],
        ),
        // Vor einer Amtsbezeichnung bleibt «Le» der Artikel, auch nach einem Umbruch.
        (
            "Erika Beispiel\nLe Grand Conseil a voté.",
            &["Erika Beispiel"],
        ),
        // Die Amtsbezeichnung nach «Le» wirkt wie eine Anrede (Prüferproblem c-erkennung-p4).
        (
            "Le Greffier Marie Exemple a signé le procès-verbal.",
            &["Marie Exemple"],
        ),
        (
            "Le Maire Marie Muster a refusé le permis.",
            &["Marie Muster"],
        ),
        (
            "Le Greffier Jean Dupont a signé le procès-verbal.",
            &["Jean Dupont"],
        ),
    ];

    #[test]
    fn artikel_le_kein_vorname() {
        pruefe_liste(ARTIKEL_LE);
        // Mit Rolle wird aus der abstrakten Frage kein ROT (l19, l22, a02, a03, a17).
        for (rolle, satz) in [
            (
                Rolle::Lawyer,
                "Le Grand Conseil a adopté la loi. Quel est le délai référendaire?",
            ),
            (
                Rolle::Authority,
                "Le Grand Conseil doit-il approuver le budget avant le 31 décembre?",
            ),
            (
                Rolle::Court,
                "Le Grand Conseil peut-il modifier la loi sur l'organisation judiciaire?",
            ),
            (
                Rolle::Authority,
                "Le Canton de Vaud est-il compétent pour fixer le délai?",
            ),
        ] {
            let bericht = crate::pruefe(
                satz,
                &Optionen {
                    rolle: Some(rolle),
                    ..Optionen::default()
                },
            );
            assert!(
                bericht.funde.iter().all(|f| f.stufe != Stufe::Rot),
                "{satz}: {:?}",
                bericht.funde
            );
        }
        // Echte Namen bleiben ROT (Prüferprobleme c-erkennung-p3 und p4).
        for (rolle, satz, name) in [
            (
                Rolle::Lawyer,
                "J'ai rencontré hier\nLe Roux Pierre, mon client, au tribunal.",
                "Le Roux Pierre",
            ),
            (
                Rolle::Lawyer,
                "Concerne: mon client\nLe Goff Jean, né le 12.03.1985",
                "Le Goff Jean",
            ),
            (
                Rolle::Court,
                "Le Greffier Marie Exemple a signé le procès-verbal.",
                "Marie Exemple",
            ),
            (
                Rolle::Authority,
                "Le Maire Marie Muster a refusé le permis.",
                "Marie Muster",
            ),
        ] {
            let bericht = crate::pruefe(
                satz,
                &Optionen {
                    rolle: Some(rolle),
                    ..Optionen::default()
                },
            );
            assert!(
                bericht
                    .funde
                    .iter()
                    .any(|f| f.regel == "name" && f.stelle == name && f.stufe == Stufe::Rot),
                "{satz}: {:?}",
                bericht.funde
            );
        }
        // Jede Amtsbezeichnung der Liste hält «Le» frei, auch mitten im Satz.
        for e in &lexikon::lexikon(AMTSBEZEICHNUNGEN).eintraege {
            if e.merkmal.as_deref() != Some("amt") {
                continue;
            }
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                let gross: String = form
                    .split(' ')
                    .map(|w| {
                        let mut z = w.chars();
                        z.next()
                            .map(|c| c.to_uppercase().chain(z).collect::<String>())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                assert!(
                    namen(&format!("Hier, Le {gross} a parlé.")).is_empty(),
                    "{form}"
                );
            }
        }
    }

    /// Reste aus Runde 1 (w4, technik-code-1 und technik-robust-2, -3): Namen über einen
    /// weichen Zeilenumbruch und mit unsichtbaren Formatzeichen, mit Gegenbeispielen.
    #[test]
    fn namen_ueber_weiche_umbrueche() {
        for (text, erwartet) in [
            (
                "Meine Patientin Erika\nBeispiel ist arbeitsunfähig.",
                &["Erika\nBeispiel"][..],
            ),
            ("Sehr geehrte Frau\nMeier, …", &["Meier"]),
            ("Der Beschwerdeführer\nMeier rügt.", &["Meier"]),
            // «A.» am Zeilenanfang sieht aus wie ein Listenpunkt: kein weicher Umbruch.
            ("Der Beschwerdeführer\nA. Meier rügt.", &[]),
            ("Die beschuldigte\nPerson Meier schweigt.", &["Meier"]),
            ("Maria da\nSilva erscheint.", &["Maria da\nSilva"]),
            (
                "Erika Bei\u{AD}spiel ist arbeitsunfähig.",
                &["Erika Bei\u{AD}spiel"],
            ),
            (
                "Erika Bei\u{200B}spiel ist arbeitsunfähig.",
                &["Erika Bei\u{200B}spiel"],
            ),
            ("Hans\u{AD} Muster ist krank.", &["Hans\u{AD} Muster"]),
            // Vertretung über einen weichen Umbruch: kein Name.
            ("vertreten durch\nRA Hans Muster", &[]),
            ("Hans Muster,\nIhr Anwalt", &[]),
            // Gegenbeispiele: Listenpunkte, Leerzeile, Satzende.
            ("- Erika\n- Beispiel", &[]),
            ("Erika\n\nBeispiel", &[]),
            ("Das ist Erika.\nBeispiel folgt.", &[]),
            // Prüferproblem c-erkennung-p2: Nach einer Grussformel beginnt die nächste Zeile
            // einen Satz; ein gewöhnliches gross geschriebenes Wort dort ist kein Nachname.
            (
                "Liebe Erika\nDanke für die Unterlagen zum Mietvertrag.",
                &[],
            ),
            ("Hallo Hans\nAnbei der Vertrag zur Prüfung.", &[]),
            ("Grüezi Erika\nKurze Frage zur Kündigungsfrist.", &[]),
            ("Hallo Erika\nFrage: Gilt die Frist auch im Urlaub?", &[]),
            ("Liebe Erika\nVielen Dank für das Gespräch.", &[]),
            ("Hallo Hans\nGericht und Frist sind unklar.", &[]),
            ("Ciao Giulia\nGrazie per i documenti.", &[]),
            ("Bericht für Andrea\nVerlauf: stabil.", &[]),
            // Grenze in explain name: auch ein Nachname des Registers nicht.
            ("Hallo Erika\nMeier hat angerufen.", &[]),
            // Mit Possessivrahmen oder Anrede reicht der Name über den Umbruch.
            (
                "La mia paziente Giulia\nEsempio è incinta.",
                &["Giulia\nEsempio"],
            ),
            ("Herrn\nHans Muster", &["Hans Muster"]),
            // Adressblöcke: Der Name endet am Zeilenende (Prüferprobleme c-erkennung-p2, p8).
            (
                "Monsieur\nLe Roux Pierre\nRue du Lac 12\n1003 Lausanne",
                &["Le Roux Pierre"],
            ),
            (
                "Patient\nLe Goff Jean\nDiagnostic: dépression",
                &["Le Goff Jean"],
            ),
        ] {
            assert_eq!(namen(text), erwartet, "Text: {text:?}");
        }
        // Mit Rolle kein ROT nach einer Grussformel (vorher ROT «Erika⏎Danke»).
        for satz in [
            "Liebe Erika\nDanke für die Unterlagen zum Mietvertrag.",
            "Hallo Hans\nAnbei der Vertrag zur Prüfung.",
            "Ciao Giulia\nGrazie per i documenti.",
        ] {
            let b = crate::pruefe(
                satz,
                &Optionen {
                    rolle: Some(Rolle::Lawyer),
                    ..Optionen::default()
                },
            );
            assert!(
                b.funde.iter().all(|f| f.stufe != Stufe::Rot),
                "{satz}: {:?}",
                b.funde
            );
        }
        assert_eq!(
            bezuege("Die beschuldigte\nPerson schweigt."),
            vec!["beschuldigte\nPerson"]
        );
        // Dieselbe Zeile ohne Umbruch ergibt dasselbe über die ganze Prüfung (Rolle aus dem
        // Text, ROT).
        let mit = crate::pruefe(
            "Meine Patientin Erika\nBeispiel ist arbeitsunfähig.",
            &Optionen::default(),
        );
        let ohne = crate::pruefe(
            "Meine Patientin Erika Beispiel ist arbeitsunfähig.",
            &Optionen::default(),
        );
        let stufen = |b: &crate::Bericht| {
            b.funde
                .iter()
                .map(|f| (f.regel, f.stufe))
                .collect::<Vec<_>>()
        };
        assert_eq!(stufen(&mit), stufen(&ohne));
        assert!(stufen(&mit).contains(&("name", Stufe::Rot)));
    }

    #[test]
    fn register_umfang_und_mehrdeutigkeit() {
        assert_eq!(lexikon::register(VORNAMEN).zeilen.len(), 13591);
        assert_eq!(lexikon::register(NACHNAMEN).zeilen.len(), 72773);
        let d = &*DATEN;
        let vorname = |n: &str| d.vornamen.get(&schluessel(n)).map(|r| r.mehrdeutig);
        let nachname = |n: &str| d.nachnamen.get(&schluessel(n)).map(|r| r.mehrdeutig);
        for (n, m) in [
            ("Hans", false),
            ("Erika", false),
            ("Jean", false),
            ("Mai", true),
            ("Art", true),
            ("Uri", true),
            ("Jens", true),
        ] {
            assert_eq!(vorname(n), Some(m), "Vorname {n}");
        }
        assert_eq!(vorname("Juni"), None);
        for (n, m) in [
            ("Meier", false),
            ("Dupont", false),
            ("Rossi", false),
            ("da Silva", false),
            ("d\u{2019}Angelo", false),
            ("Muster", true),
            ("Angst", true),
            ("Richter", true),
        ] {
            assert_eq!(nachname(n), Some(m), "Nachname {n}");
        }
        assert_eq!(nachname("Beispiel"), None);
    }

    #[test]
    fn wortlisten_vollstaendig_belegt() {
        for datei in [ANREDEN, RECHTSVERTRETUNG, FUNKTIONEN, BEGLEITER, ROLLEN] {
            let lex = lexikon::lexikon(datei);
            assert!(!lex.eintraege.is_empty(), "{datei}");
            for e in &lex.eintraege {
                assert_eq!(e.regel, "name", "{datei}: {}", e.begriff);
                let art = e.merkmal.as_deref().and_then(Wortart::aus_merkmal);
                assert!(
                    art.is_some(),
                    "{datei}: {} ohne gültiges Merkmal",
                    e.begriff
                );
            }
        }
        // Vertretungsmerkmale: jedes Merkmal in jeder Sprache, Fundstelle mit Wortlaut.
        let vertretung = lexikon::lexikon(VERTRETUNG);
        for e in &vertretung.eintraege {
            assert_eq!(e.regel, "name", "{VERTRETUNG}: {}", e.begriff);
            assert!(
                e.merkmal
                    .as_deref()
                    .and_then(Vertretungsmerkmal::aus_merkmal)
                    .is_some(),
                "{VERTRETUNG}: {} ohne gültiges Merkmal",
                e.begriff
            );
            assert!(e.quelle.contains('«'), "{}: Wortlaut fehlt", e.begriff);
        }
        for merkmal in ["vertreten", "vertritt", "besitz", "bezug", "partei"] {
            for sprache in [
                lexikon::Sprache::De,
                lexikon::Sprache::Fr,
                lexikon::Sprache::It,
            ] {
                assert!(
                    vertretung
                        .eintraege
                        .iter()
                        .any(|e| e.sprache == sprache && e.merkmal.as_deref() == Some(merkmal)),
                    "{merkmal} fehlt in {sprache:?}"
                );
            }
        }
        // Alltags- und Satzwörter: jedes steht im Register der Nachnamen, hat eine Fundstelle
        // mit Wortlaut, und beide Merkmale gibt es in jeder Sprache.
        let alltag = lexikon::lexikon(ALLTAG);
        for e in &alltag.eintraege {
            assert_eq!(e.regel, "name", "{ALLTAG}: {}", e.begriff);
            assert!(
                matches!(
                    e.merkmal.as_deref(),
                    Some(MERKMAL_ALLTAG | MERKMAL_SATZWORT)
                ),
                "{ALLTAG}: {} ohne gültiges Merkmal",
                e.begriff
            );
            assert!(e.quelle.contains('«'), "{}: Wortlaut fehlt", e.begriff);
            for form in std::iter::once(&e.begriff).chain(&e.formen) {
                assert!(
                    DATEN.nachnamen.contains_key(&schluessel(form)),
                    "{ALLTAG}: {form} steht nicht im Register der Nachnamen"
                );
            }
        }
        for merkmal in [MERKMAL_ALLTAG, MERKMAL_SATZWORT] {
            for sprache in [
                lexikon::Sprache::De,
                lexikon::Sprache::Fr,
                lexikon::Sprache::It,
            ] {
                assert!(
                    alltag
                        .eintraege
                        .iter()
                        .any(|e| e.sprache == sprache && e.merkmal.as_deref() == Some(merkmal)),
                    "{merkmal} fehlt in {sprache:?}"
                );
            }
        }
        // Artikel und Amtsbezeichnungen (C1): Merkmal, Fedlex-Fundstelle mit Artikel.
        let aemter = lexikon::lexikon(AMTSBEZEICHNUNGEN);
        for e in &aemter.eintraege {
            assert_eq!(e.regel, "name", "{AMTSBEZEICHNUNGEN}: {}", e.begriff);
            assert!(
                matches!(e.merkmal.as_deref(), Some("artikel" | "amt")),
                "{AMTSBEZEICHNUNGEN}: {} ohne gültiges Merkmal",
                e.begriff
            );
            assert!(
                e.url.starts_with("https://www.fedlex.admin.ch/eli/cc/"),
                "{}",
                e.begriff
            );
        }
        assert!(DATEN.artikel.contains("le"));
        let rollen = lexikon::lexikon(ROLLEN);
        for sprache in [
            lexikon::Sprache::De,
            lexikon::Sprache::Fr,
            lexikon::Sprache::It,
        ] {
            assert!(rollen.eintraege.iter().any(|e| e.sprache == sprache));
        }
        for e in &rollen.eintraege {
            assert!(
                e.url.starts_with("https://www.fedlex.admin.ch/eli/cc/"),
                "{}",
                e.begriff
            );
            assert!(
                e.quelle.contains("Art.") || e.quelle.contains("art."),
                "{}: Artikel fehlt",
                e.begriff
            );
        }
    }

    #[test]
    fn info_vollstaendig() {
        let info = info();
        assert_eq!(info.len(), 1);
        let r = &info[0];
        assert_eq!(r.id, "name");
        assert_eq!(r.klasse, Klasse::B);
        assert_eq!(r.guete, Guete::NichtGemessen);
        assert_eq!(r.grundlage, vec![&normen::DSG_5_A]);
        for t in [r.titel, r.erkennt, r.methode]
            .iter()
            .chain(r.grenzen.iter())
        {
            assert!(!t.de.is_empty() && !t.en.is_empty());
        }
        assert!(!r.grenzen.is_empty());
        for d in [
            VORNAMEN,
            NACHNAMEN,
            ANREDEN,
            ROLLEN,
            RECHTSVERTRETUNG,
            VERTRETUNG,
            FUNKTIONEN,
            BEGLEITER,
            ALLTAG,
            AMTSBEZEICHNUNGEN,
        ] {
            assert!(r.lexika.contains(&d), "{d} fehlt in lexika");
            assert!(lexikon::inhalt(d).is_some(), "{d} nicht eingebettet");
        }
        assert!(r.quellen.len() >= 10);
        for q in &r.quellen {
            assert!(q.url.starts_with("https://"), "{}", q.titel);
            assert!(!q.titel.is_empty());
        }
        assert!(
            r.quellen
                .iter()
                .filter(|q| q.url.contains("opendata.swiss"))
                .all(|q| q.lizenz.is_some())
        );
    }
}
