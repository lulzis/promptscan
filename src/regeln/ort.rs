//! Regeln `adresse` und `wohnort` (SPEC 5.2).
//!
//! **adresse** meldet eine Postadresse als eine Stelle: Strasse mit Hausnummer und, wenn sie
//! folgen, PLZ und Ort, auch über Komma oder Zeilenumbruch («Seestrasse 12, 8002 Zürich»).
//! Die Bestandteile nennt Art. 26b Abs. 1 GeoNV, das Postfach Art. 23 Abs. 1 lit. b VPG.
//! Strassentypen, PLZ und Ortschaften stammen aus den amtlichen Verzeichnissen; die Register
//! unter `lexika/register/` erzeugt `examples/register_orte.rs` aus den Rohdaten.
//!
//! **wohnort** meldet den Namen einer politischen Gemeinde nach einem Auslösewort aus
//! amtlichen Texten («wohnhaft in», «domicilié à», «Wohnsitz:»).
//!
//! Was die Anonymisierungsregeln des Bundesgerichts offen lassen, meldet keine der beiden
//! Regeln: Gemeindenamen und Ortsbezeichnungen (Art. 4 Abs. 1 lit. b und e), also PLZ und
//! Ort ohne Strasse und Gemeinden ohne Auslösewort, sowie Adressen von Behörden (Art. 7
//! Abs. 3 Satz 2). Adressen von Parteien werden dort «in jedem Fall weggestrichen» (Art. 7
//! Abs. 3 Satz 1); darum genügt Strasse mit Hausnummer, wenn der Strassentyp erfasst ist.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Gefaltet, Lexikon, Register, Sprache, falten};
use crate::normen::{self, Norm};
use crate::text::{Text, ist_unsichtbar, ist_wortzeichen, rechts_frei};

const ADRESSE: &str = "adresse";
const WOHNORT: &str = "wohnort";

const POSTFACH: &str = "adresse-postfach.toml";
const BEHOERDEN: &str = "adresse-behoerden.toml";
const AUSLOESER: &str = "wohnort-ausloeser.toml";
const PLZ: &str = "register/plz.tsv";
const GEMEINDEN: &str = "register/gemeinden.tsv";
const STRASSENTYPEN: &str = "register/strassentypen.tsv";

/// Endungen, die auch gewöhnliche Wörter mit einer Nummer danach bilden («Parkplatz 12»,
/// «Bahnsteig 3»; englisch «Monitoring 2»). Mit ihnen ist Strasse und Nummer nur zusammen
/// mit PLZ und Ort eine Adresse.
const ENDUNGEN_NUR_MIT_ORT: &[&str] = &["platz", "ring", "steig"];

/// Wie weit vor der Hausnummer der Typ einer französischen oder italienischen Strasse
/// stehen darf, in Wörtern («Avenue du Tribunal-Fédéral 29»: drei).
const NAMENSLAENGE: usize = 8;

pub const GEONV_3_F: Norm = Norm {
    zitat: "Art. 3 lit. f GeoNV",
    erlass: "GeoNV, SR 510.625",
    fassung: "2024-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2008/393/de#art_3",
    wortlaut: "Strassen: Strassen, Wege, Gassen, Plätze und benannte Gebiete, die als Strassenbezeichnungen für Adressen dienen",
};

pub const GEONV_19_1: Norm = Norm {
    zitat: "Art. 19 Abs. 1 GeoNV",
    erlass: "GeoNV, SR 510.625",
    fassung: "2024-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2008/393/de#art_19",
    wortlaut: "Das Bundesamt für Statistik: a. vergibt für jede Gemeinde eine verbindliche Nummer; b. erstellt, verwaltet und veröffentlicht das amtliche Gemeindeverzeichnis der Schweiz.",
};

pub const GEONV_24: Norm = Norm {
    zitat: "Art. 24 GeoNV",
    erlass: "GeoNV, SR 510.625",
    fassung: "2024-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2008/393/de#art_24",
    wortlaut: "Das Bundesamt für Landestopografie erstellt, verwaltet und veröffentlicht das amtliche Ortschaftenverzeichnis mit Postleitzahl und Perimeter.",
};

pub const GEONV_26B: Norm = Norm {
    zitat: "Art. 26b Abs. 1 und 4 GeoNV",
    erlass: "GeoNV, SR 510.625",
    fassung: "2024-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2008/393/de#art_26_b",
    wortlaut: "Die Gebäudeadresse wird durch die folgenden Daten bestimmt: … c. die Hausnummer (Polizeinummer) gemäss kantonalem Recht; … e. den zugehörigen Strassennamen aus dem amtlichen Verzeichnis (Art. 26a); f. den zugehörigen Ortschaftsnamen und die Postleitzahl aus dem amtlichen Ortschaftsverzeichnis (Art. 24); … Jede Gebäudeadresse ist innerhalb einer Ortschaft eindeutig.",
};

pub const VPG_23_1_B: Norm = Norm {
    zitat: "Art. 23 Abs. 1 lit. b VPG",
    erlass: "Postverordnung (VPG), SR 783.01",
    fassung: "2026-04-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2012/586/de#art_23",
    wortlaut: "Die Datensätze enthalten: … b. Strasse, Hausnummer, Postleitzahl und Ort sowie, falls vorhanden, Standort und Nummer des Postfachs",
};

pub const ZGB_23_1: Norm = Norm {
    zitat: "Art. 23 Abs. 1 ZGB",
    erlass: "ZGB, SR 210",
    fassung: "2026-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_23",
    wortlaut: "Der Wohnsitz einer Person befindet sich an dem Orte, wo sie sich mit der Absicht dauernden Verbleibens aufhält; …",
};

const GEPRUEFT: Stand = Stand::Geprueft("2026-11-06");

const LIZENZ_SWISSTOPO: &str = "Nutzungsbedingungen für kostenlose Geodaten und Geodienste (OGD) von swisstopo: «Die kostenlosen Geodaten und Geodienste von swisstopo dürfen genutzt, verbreitet und zugänglich gemacht werden. Weiter dürfen sie angereichert, bearbeitet sowie auch kommerziell genutzt werden. Eine Quellenangabe ist Pflicht.» Quellenangabe: Bundesamt für Landestopografie swisstopo";
const LIZENZ_BFS: &str = "opendata.swiss, Freie Nutzung: «Sie dürfen diesen Datensatz für nicht kommerzielle Zwecke nutzen. Sie dürfen diesen Datensatz für kommerzielle Zwecke nutzen. Eine Quellenangabe wird empfohlen (Autor, Titel und Link zum Datensatz).»";

const Q_BGER: Quelle = Quelle {
    titel: "Bundesgericht: Anonymisierungsregeln (Präsidentenkonferenz 17.12.2019, Verwaltungskommission 13.02.2020), Art. 4 Abs. 1 und Art. 7",
    url: "https://www.bger.ch/home/juridiction/jurisdiction-recht/anonymisierungsregeln.html",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_EMPFEHLUNG: Quelle = Quelle {
    titel: "BFS und swisstopo: Empfehlung Gebäudeadressierung und Schreibweise von Strassennamen, Version 1.0, Neuchâtel 2018 (Kap. 2 Gebäudeadressierung, Kap. 3 Schreibweise von Strassennamen mit Abkürzungen S. 8 und Kurznamen S. 8–9, Kap. 4 Hausnummern, Kap. 5 Postleitzahl und Ortschaften)",
    url: "https://www.cadastre-manual.admin.ch/dam/de/sd-web/0GCx0raEJjas/Empfehlung-Gebaeudeadressierung-de.pdf",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_STRASSEN: Quelle = Quelle {
    titel: "swisstopo: Amtliches Verzeichnis der Strassen, Schweiz und Liechtenstein (abgeleitet: register/strassentypen.tsv)",
    url: "https://data.geo.admin.ch/ch.swisstopo.amtliches-strassenverzeichnis/amtliches-strassenverzeichnis_ch/amtliches-strassenverzeichnis_ch_2056.csv.zip",
    lizenz: Some(LIZENZ_SWISSTOPO),
    stand: GEPRUEFT,
};
const Q_ORTSCHAFTEN: Quelle = Quelle {
    titel: "swisstopo: Amtliches Ortschaftenverzeichnis mit Postleitzahl und Perimeter (abgeleitet: register/plz.tsv)",
    url: "https://data.geo.admin.ch/ch.swisstopo-vd.ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz_2056.csv.zip",
    lizenz: Some(LIZENZ_SWISSTOPO),
    stand: GEPRUEFT,
};
const Q_GEMEINDEN: Quelle = Quelle {
    titel: "BFS: Historisiertes Gemeindeverzeichnis der Schweiz, Snapshot vom 06.11.2026 (abgeleitet: register/gemeinden.tsv)",
    url: "https://opendata.swiss/de/dataset/historisiertes-gemeindeverzeichnis-der-schweiz",
    lizenz: Some(LIZENZ_BFS),
    stand: GEPRUEFT,
};
const Q_GEONV: Quelle = Quelle {
    titel: "Fedlex: Verordnung über die geografischen Namen (GeoNV), SR 510.625, Art. 3, 19, 24, 26a und 26b",
    url: "https://www.fedlex.admin.ch/eli/cc/2008/393/de",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_VPG: Quelle = Quelle {
    titel: "Fedlex: Postverordnung (VPG), SR 783.01, Art. 23 und 29 (adresse-postfach.toml)",
    url: "https://www.fedlex.admin.ch/eli/cc/2012/586/de",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_BEHOERDEN: Quelle = Quelle {
    titel: "Fedlex: Behördenbezeichnungen in DE, FR, IT aus StPO Art. 12 und 13, DSG Art. 5 lit. i, RVOG Art. 47, BGÖ Art. 1, GeoNV Art. 19, StBOG Art. 2 und 33 (adresse-behoerden.toml, Fundstelle je Eintrag)",
    url: "https://www.fedlex.admin.ch/eli/cc/2010/267/de",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_AUSLOESER: Quelle = Quelle {
    titel: "Fedlex: Auslösewörter in DE, FR, IT aus WPEG Art. 4a, IPRG Art. 37 und 43, ZGB Art. 23, BPR Art. 33, SchKG Art. 50, AIG Art. 4 (wohnort-ausloeser.toml, Fundstelle je Eintrag)",
    url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de",
    lizenz: None,
    stand: GEPRUEFT,
};

const METHODE_ADRESSE_DE: &str = "Eine Hausnummer nach einem Strassennamen mit erfasstem Typ (Endung wie «-strasse», «-weg» oder erstes Wort wie «Rue», «Via») ist eine Adresse; folgen PLZ und Ort, reicht die Stelle bis zum Ort. Die Typen sind aus dem amtlichen Strassenverzeichnis abgeleitet: 63,69 % der 225 593 Strassennamen tragen einen erfassten Typ. Ohne erfassten Typ zählt nur «Name Nummer, PLZ Ort», wenn das Paar aus PLZ und Ortschaft im amtlichen Ortschaftenverzeichnis steht; ein Postfach zählt mit Nummer, PLZ und Ort. Unsichtbare Formatzeichen wie U+00AD und U+200B im Strassennamen und neben dem Leerraum werden übergangen. Adressen von Behörden sind ausgenommen (Anonymisierungsregeln des Bundesgerichts, Art. 7 Abs. 3).";
const METHODE_ADRESSE_EN: &str = "A house number after a street name with a covered street type (ending such as «-strasse», «-weg» or first word such as «Rue», «Via») is an address; if postcode and locality follow, the match extends to the locality. The types are derived from the official street directory: 63.69 % of the 225,593 street names carry a covered type. Without a covered type, only «Name Number, Postcode Locality» counts, when the pair of postcode and locality is in the official locality directory; a PO box counts with number, postcode and locality. Invisible format characters such as U+00AD and U+200B in the street name and next to whitespace are skipped. Addresses of authorities are exempt (Federal Supreme Court anonymisation rules, Art. 7 para. 3).";
const METHODE_WOHNORT_DE: &str = "Ein Auslösewort aus amtlichen Texten («wohnhaft», «mit Wohnsitz», «domicilié», «domiciliato», «residente») mit Präposition oder eine Feldbezeichnung mit Doppelpunkt («Wohnsitz:», «domicile:»), danach der Name einer politischen Gemeinde aus dem amtlichen Gemeindeverzeichnis des BFS (2110 Gemeinden). Gemeldet wird der Gemeindename, auch ohne Klammerzusatz («Zell») oder als Teil eines zweisprachigen Namens («Bienne»). Zwischen Auslösewort, Präposition und Gemeinde darf ein weicher Zeilenumbruch stehen, also ein einzelner Umbruch mitten im Absatz («wohnhaft in⏎Winterthur»); unsichtbare Formatzeichen wie U+00AD und U+200B werden übergangen. Ein Gemeindename ohne Auslösewort wird nicht gemeldet, weil die Regel Ort und Person nicht sicher verbinden kann. Nach den Anonymisierungsregeln des Bundesgerichts bleiben Gemeindenamen in der Regel offen (Art. 4 Abs. 1 lit. b); der Wohnort einer Partei ist aber in der Regel zu anonymisieren, auch als Gemeindename (Art. 7 Abs. 1).";
const METHODE_WOHNORT_EN: &str = "A trigger word from official texts («wohnhaft», «mit Wohnsitz», «domicilié», «domiciliato», «residente») with a preposition, or a field label with a colon («Wohnsitz:», «domicile:»), followed by the name of a political municipality from the FSO's official register of municipalities (2110 municipalities). The municipality name is reported, also without the bracketed suffix («Zell») or as part of a bilingual name («Bienne»). A soft line break, a single break within a paragraph, may stand between trigger word, preposition and municipality («wohnhaft in⏎Winterthur»); invisible format characters such as U+00AD and U+200B are skipped. A municipality name without a trigger word is not reported, because the rule cannot reliably link place and person. Under the anonymisation rules of the Federal Supreme Court, municipality names generally stay open (Art. 4 Abs. 1 lit. b); the place of residence of a party, however, is generally to be anonymised, also as a municipality name (Art. 7 Abs. 1).";

/// Gefaltete Namen, nach erstem Zeichen gruppiert, längste zuerst.
struct Namen(HashMap<char, Vec<String>>);

impl Namen {
    fn aus(namen: HashSet<String>) -> Namen {
        let mut karte: HashMap<char, Vec<String>> = HashMap::new();
        for n in namen {
            if let Some(c) = n.chars().next() {
                karte.entry(c).or_default().push(n);
            }
        }
        for liste in karte.values_mut() {
            liste.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        }
        Namen(karte)
    }

    fn kandidaten(&self, c: char) -> &[String] {
        self.0.get(&c).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Die Suchtabellen aus den Registern, einmal je Programmlauf gebaut.
struct Daten {
    /// PLZ → gefaltete Ortschaftsnamen mit ihren Schreibweisen.
    plz: HashMap<&'static str, Vec<String>>,
    /// Alle Ortschafts- und Gemeindenamen mit Schreibweisen.
    orte: Namen,
    /// Die Gemeindenamen mit Schreibweisen.
    gemeinden: Namen,
    /// Typen am Anfang des Namens, gefaltet («rue», «via», «av.»).
    anfang: HashSet<String>,
    /// Typen als Endung, gefaltet und längste zuerst; `true`: nur mit PLZ und Ort.
    ende: Vec<(String, bool)>,
    /// Vorwörter wie im Verzeichnis und im Dativ («Alte», «Alten»).
    vorwoerter: HashSet<String>,
    /// Kleingeschriebene Wörter in Strassennamen, gefaltet («de», «l'», «della»).
    verbindungen: HashSet<String>,
}

static DATEN: LazyLock<Daten> = LazyLock::new(Daten::laden);

impl Daten {
    fn laden() -> Daten {
        let gemeinden = lexikon::register(GEMEINDEN);
        let kantone: HashSet<&str> = gemeinden
            .kopf("kantonskuerzel")
            .unwrap_or_default()
            .split_whitespace()
            .collect();

        let mut plz: HashMap<&'static str, Vec<String>> = HashMap::new();
        let mut orte = HashSet::new();
        let reg = lexikon::register(PLZ);
        let (i_plz, i_ort) = (spalte(reg, "plz"), spalte(reg, "ortschaft"));
        for z in &reg.zeilen {
            let liste = plz.entry(z[i_plz].as_str()).or_default();
            for v in varianten(&z[i_ort], &kantone) {
                if !liste.contains(&v) {
                    liste.push(v.clone());
                }
                orte.insert(v);
            }
        }
        let mut gemeindenamen = HashSet::new();
        for name in gemeinden.werte("name") {
            for v in varianten(name, &kantone) {
                orte.insert(v.clone());
                gemeindenamen.insert(v);
            }
        }

        let typen = lexikon::register(STRASSENTYPEN);
        let (i_typ, i_stellung) = (spalte(typen, "typ"), spalte(typen, "stellung"));
        let mut anfang = HashSet::new();
        let mut ende = Vec::new();
        let mut vorwoerter = HashSet::new();
        let mut verbindungen = HashSet::new();
        for z in &typen.zeilen {
            let typ = z[i_typ].as_str();
            match z[i_stellung].as_str() {
                "anfang" => {
                    anfang.insert(falten(typ, true).text);
                }
                "ende" => ende.push((falten(typ, true).text, ENDUNGEN_NUR_MIT_ORT.contains(&typ))),
                "vorwort" => {
                    vorwoerter.insert(typ.to_string());
                    // Dativ: «an der Alten Landstrasse».
                    if typ.ends_with('e') {
                        vorwoerter.insert(format!("{typ}n"));
                    }
                }
                "verbindung" => {
                    verbindungen.insert(falten(typ, true).text);
                }
                _ => {}
            }
        }
        ende.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));

        Daten {
            plz,
            orte: Namen::aus(orte),
            gemeinden: Namen::aus(gemeindenamen),
            anfang,
            ende,
            vorwoerter,
            verbindungen,
        }
    }
}

fn spalte(reg: &Register, name: &str) -> usize {
    reg.spalte(name)
        .unwrap_or_else(|| panic!("lexika/{}: Spalte {name} fehlt", reg.datei))
}

/// Schreibweisen eines amtlichen Namens, gefaltet: der Name selbst, ohne Nummer der
/// Ortschaft («Lausanne 25»), ohne Kantonskürzel («Kilchberg ZH»), ohne Klammerzusatz
/// («Zell (ZH)», «St-Saphorin (Lavaux)»), mit Kürzel statt Klammer («Zell ZH») und die Teile
/// zweisprachiger Namen («Biel/Bienne»).
fn varianten(name: &str, kantone: &HashSet<&str>) -> Vec<String> {
    let mut v = vec![name.to_string()];
    let mut basis = name;
    if let Some((vorne, _)) = basis
        .rsplit_once(' ')
        .filter(|(_, h)| !h.is_empty() && h.bytes().all(|b| b.is_ascii_digit()))
    {
        basis = vorne;
        v.push(basis.to_string());
    }
    if let Some((vorne, _)) = basis.rsplit_once(' ').filter(|(_, h)| kantone.contains(h)) {
        basis = vorne;
        v.push(basis.to_string());
    }
    if let Some((vorne, innen)) = basis
        .split_once(" (")
        .and_then(|(vorne, rest)| Some((vorne, rest.strip_suffix(')')?)))
    {
        if kantone.contains(innen) {
            v.push(format!("{vorne} {innen}"));
        }
        basis = vorne;
        v.push(basis.to_string());
    }
    if basis.contains('/') {
        v.extend(basis.split('/').map(|t| t.trim().to_string()));
    }
    let mut gefaltet: Vec<String> = Vec::new();
    for n in v {
        let f = falten(&n, true).text;
        if f.chars().filter(|c| c.is_alphabetic()).count() >= 2 && !gefaltet.contains(&f) {
            gefaltet.push(f);
        }
    }
    gefaltet
}

/// Ein Wort des Textes: Buchstaben und Ziffern mit den Zeichen, die in Strassennamen und
/// Hausnummern vorkommen (Punkt, Bindestrich, Apostroph, Schrägstrich).
#[derive(Clone, Copy, Debug)]
struct Wort {
    start: usize,
    ende: usize,
}

fn wortteil(c: char) -> bool {
    ist_wortzeichen(c)
        || matches!(
            c,
            '.' | '-' | '/' | '\'' | '\u{2019}' | '\u{2010}' | '\u{2011}'
        )
}

/// Die Wörter des Textes. Ein unsichtbares Formatzeichen ([`ist_unsichtbar`]) gehört zum
/// Wort, wenn davor und danach ein Wortteil steht («See\u{AD}strasse», «See\u{200B}strasse»);
/// sonst zählt es als Leerraum ([`ist_leer`]).
fn woerter(s: &str) -> Vec<Wort> {
    let mut v = Vec::new();
    let mut start = None;
    let mut zeichen = s.char_indices().peekable();
    while let Some((i, c)) = zeichen.next() {
        let im_wort = start.is_some()
            && ist_unsichtbar(c)
            && zeichen.peek().is_some_and(|&(_, d)| wortteil(d));
        match (wortteil(c) || im_wort, start) {
            (true, None) => start = Some(i),
            (false, Some(a)) => {
                v.push(Wort { start: a, ende: i });
                start = None;
            }
            _ => {}
        }
    }
    if let Some(a) = start {
        v.push(Wort {
            start: a,
            ende: s.len(),
        });
    }
    v
}

fn wort(s: &str, w: Wort) -> &str {
    &s[w.start..w.ende]
}

fn zwischen<'a>(s: &'a str, w: &[Wort], a: usize, b: usize) -> &'a str {
    &s[w[a].ende..w[b].start]
}

/// Leerzeichen, Tabulator, geschützte Leerzeichen und unsichtbare Formatzeichen; kein
/// Zeilenumbruch.
fn ist_leer(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{00A0}' | '\u{202F}') || ist_unsichtbar(c)
}

fn nur_leerraum(s: &str) -> bool {
    !s.is_empty() && s.chars().all(ist_leer)
}

/// Länge des Leerraums am Anfang, in Bytes.
fn leerraum(s: &str) -> usize {
    s.find(|c: char| !ist_leer(c)).unwrap_or(s.len())
}

/// Wie [`leerraum`] ab `pos`, dazu höchstens ein weicher Zeilenumbruch
/// ([`Text::weicher_umbruch`], auch als CRLF): «wohnhaft in⏎Winterthur».
fn leerraum_weich(text: &Text, pos: usize) -> usize {
    let s = text.als_str();
    let mut laenge = leerraum(&s[pos..]);
    let rest = &s[pos + laenge..];
    let cr = usize::from(rest.starts_with("\r\n"));
    if rest[cr..].starts_with('\n') && text.weicher_umbruch(pos + laenge + cr) {
        laenge += cr + 1;
        laenge += leerraum(&s[pos + laenge..]);
    }
    laenge
}

fn gross(t: &str) -> bool {
    t.chars().next().is_some_and(char::is_uppercase)
}

/// Beginnt das Wort oder einer seiner Teile nach Bindestrich oder Apostroph mit einem
/// Grossbuchstaben? («Lausanne», «Tribunal-Fédéral», «d'Ouchy», «1er-Mars»)
fn namenswort(t: &str) -> bool {
    let mut teilanfang = true;
    for c in t.chars() {
        if teilanfang && c.is_uppercase() {
            return true;
        }
        teilanfang = matches!(c, '-' | '\'' | '\u{2019}' | '\u{2010}' | '\u{2011}');
    }
    false
}

/// Abgekürzter Vorname («G.», «J.-J.»), wie ihn die Empfehlung 2018 in Strassennamen zulässt.
fn initiale(t: &str) -> bool {
    t.split('-').all(|teil| {
        let mut c = teil.chars();
        matches!((c.next(), c.next(), c.next()), (Some(a), Some('.'), None) if a.is_uppercase())
    })
}

/// Darf das Wort im Namen einer Strasse stehen? Namenswort, Verbindungswort aus dem
/// Verzeichnis, Zahl («Rue du 31 Décembre»), Initiale oder «St.» (Empfehlung 2018, S. 8).
fn namensteil(t: &str, d: &Daten) -> bool {
    if t.ends_with('.') {
        return initiale(t) || t == "St.";
    }
    namenswort(t) || t.bytes().all(|b| b.is_ascii_digit()) || verbindungswort(t, d)
}

/// Hausnummer nach der Empfehlung 2018, Kap. 4: Zahl ab 1, Zusatz aus höchstens drei
/// Kleinbuchstaben («18a», «2bis»), Zusatznummer nach Punkt («21.1»). Die Empfehlung lässt
/// Bereiche («10-12», «10/12») und Grossbuchstaben nicht zu; im Text kommen beide vor
/// («Seestrasse 10-12», «12A»), darum gehören sie hier zur Stelle.
static HAUSNUMMER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[1-9][0-9]{0,3}(?:[a-z]{1,3}|[A-Z])?(?:\.[1-9][0-9]{0,2})?(?:[-/][1-9][0-9]{0,3}(?:[a-z]{1,3}|[A-Z])?)?$",
    )
    .expect("Muster der Hausnummer")
});

/// Länge der Hausnummer im Wort, ohne Satzpunkt am Ende.
fn hausnummer(t: &str) -> Option<usize> {
    let ohne = t.strip_suffix('.').unwrap_or(t);
    HAUSNUMMER.is_match(ohne).then_some(ohne.len())
}

/// Eine Strasse vor der Hausnummer.
struct Strasse {
    start: usize,
    /// Nur zusammen mit PLZ und Ort eine Adresse.
    nur_mit_ort: bool,
}

/// Strasse mit erfasstem Typ unmittelbar vor dem Wort `i`, der Hausnummer.
fn getypte_strasse(s: &str, w: &[Wort], i: usize, d: &Daten) -> Option<Strasse> {
    romanische_strasse(s, w, i, d).or_else(|| deutsche_strasse(s, w, i, d))
}

/// Französisch, italienisch, rätoromanisch: der Typ als erstes Wort, danach der Name mit
/// mindestens einem Eigennamen («Rue de la Gare», «via Nassa»; nicht «via SMS»). Nicht über
/// Satzzeichen oder Zeilenenden hinweg, und der Name endet nicht auf ein Verbindungswort
/// («ch. 2 de l’AF du 11 déc.» ist eine Fundstelle, keine Strasse).
fn romanische_strasse(s: &str, w: &[Wort], i: usize, d: &Daten) -> Option<Strasse> {
    if verbindungswort(wort(s, w[i.checked_sub(1)?]), d) {
        return None;
    }
    let mut beste = None;
    let mut name_gesehen = false;
    let mut k = i;
    while k > 0 && i - k < NAMENSLAENGE {
        let j = k - 1;
        if !nur_leerraum(zwischen(s, w, j, k)) {
            break;
        }
        let t = wort(s, w[j]);
        if name_gesehen && typ_am_anfang(t, d) {
            beste = Some(w[j].start);
        }
        if !namensteil(t, d) {
            break;
        }
        name_gesehen |= eigenname(t);
        k = j;
    }
    beste.map(|start| Strasse {
        start,
        nur_mit_ort: false,
    })
}

/// Typ am Anfang des Namens. Kurzformen mit Punkt nur grossgeschrieben, wie die Empfehlung
/// 2018 sie schreibt («Av.», «Ch.»): kleingeschrieben sind sie in Rechtstexten anderes
/// («ch. 2» für chiffre).
fn typ_am_anfang(t: &str, d: &Daten) -> bool {
    d.anfang.contains(falten(t, true).text.as_str()) && (!t.ends_with('.') || gross(t))
}

/// Ein Grossbuchstabe vor einem Kleinbuchstaben: «Gare», «l'Ouchy», «Tribunal-Fédéral»;
/// nicht «SMS», «AF», «l'O», «II».
fn eigenname(t: &str) -> bool {
    t.chars()
        .zip(t.chars().skip(1))
        .any(|(a, b)| a.is_uppercase() && b.is_lowercase())
}

fn verbindungswort(t: &str, d: &Daten) -> bool {
    d.verbindungen.contains(falten(t, true).text.as_str())
}

/// Deutsch: der Typ als Endung des Worts vor der Nummer («Seestrasse», «Bahnhofstr.»), ein
/// Typwort allein mit dem grossgeschriebenen Wort davor («Graue Gasse») oder ein Anfangstyp
/// als letzter Bindestrich-Teil («Grand-Rue»). Ein Vorwort aus dem Verzeichnis gehört dazu
/// («Alte Landstrasse»).
fn deutsche_strasse(s: &str, w: &[Wort], i: usize, d: &Daten) -> Option<Strasse> {
    let j = i.checked_sub(1)?;
    let t = wort(s, w[j]);
    if !nur_leerraum(zwischen(s, w, j, i)) || !gross(t) {
        return None;
    }
    let f = falten(t, true).text;
    if f.rsplit_once('-')
        .is_some_and(|(_, teil)| d.anfang.contains(teil))
    {
        return Some(Strasse {
            start: w[j].start,
            nur_mit_ort: false,
        });
    }
    let (typ, nur_mit_ort) = d.ende.iter().find(|(e, _)| f.ends_with(e.as_str()))?;
    if f.len() > typ.len() {
        let start = vorwort_davor(s, w, j, d).unwrap_or(w[j].start);
        return Some(Strasse {
            start,
            nur_mit_ort: *nur_mit_ort,
        });
    }
    // Das Typwort allein: das grossgeschriebene Wort davor gehört zum Namen. Ohne PLZ und Ort
    // genügt es nur, wenn es ein Vorwort aus dem Verzeichnis ist («Alte Gasse 3», nicht
    // «Die Gasse 3»).
    let k = j.checked_sub(1)?;
    let davor = wort(s, w[k]);
    if !nur_leerraum(zwischen(s, w, k, j)) || !gross(davor) || davor.ends_with('.') {
        return None;
    }
    Some(Strasse {
        start: w[k].start,
        nur_mit_ort: *nur_mit_ort || !d.vorwoerter.contains(davor),
    })
}

fn vorwort_davor(s: &str, w: &[Wort], j: usize, d: &Daten) -> Option<usize> {
    let k = j.checked_sub(1)?;
    (nur_leerraum(zwischen(s, w, k, j)) && d.vorwoerter.contains(wort(s, w[k])))
        .then_some(w[k].start)
}

/// Strasse ohne erfassten Typ («Im Rank», «Auf der Mauer»): ein bis vier Wörter am Anfang
/// eines Abschnitts (Textanfang, nach Zeilenumbruch, Komma, Semikolon, Doppelpunkt oder
/// Klammer), das erste grossgeschrieben, das letzte mit Kleinbuchstaben («BGE 145» zählt
/// nicht). Ob PLZ und Ort im Verzeichnis folgen, prüft der Aufrufer.
fn ungetypte_strasse(s: &str, w: &[Wort], i: usize, d: &Daten) -> Option<usize> {
    let letztes = wort(s, w[i.checked_sub(1)?]);
    if letztes.chars().filter(|c| c.is_alphabetic()).count() < 2
        || !letztes.chars().any(char::is_lowercase)
        || verbindungswort(letztes, d)
    {
        return None;
    }
    let mut k = i;
    for _ in 0..4 {
        let j = k.checked_sub(1)?;
        if !nur_leerraum(zwischen(s, w, j, k)) {
            return None;
        }
        let t = wort(s, w[j]);
        let erlaubt = if t.ends_with('.') {
            t == "St."
        } else {
            namenswort(t) || verbindungswort(t, d)
        };
        if !erlaubt {
            return None;
        }
        let davor = if j == 0 {
            &s[..w[j].start]
        } else {
            zwischen(s, w, j - 1, j)
        };
        let anfang =
            davor.contains(['\n', ',', ';', ':', '(']) || (j == 0 && davor.trim().is_empty());
        if anfang && gross(t) {
            return Some(w[j].start);
        }
        k = j;
    }
    None
}

/// Zählt die Zahl etwas statt ein Haus zu nummerieren? Das ist so, wenn ihr auf derselben
/// Zeile ein grossgeschriebenes Wort folgt, das kein amtlicher Ortsname ist («via E-Mail
/// 2 Dokumente», «via Olten 2 Stunden»). «Seestrasse 12 Zürich» bleibt eine Adresse.
fn zaehlt_etwas(s: &str, nummer_ende: usize, d: &Daten) -> bool {
    let leer = leerraum(&s[nummer_ende..]);
    if leer == 0 {
        return false;
    }
    let pos = nummer_ende + leer;
    let ende = s[pos..]
        .find(|c: char| !wortteil(c))
        .map_or(s.len(), |e| pos + e);
    let t = &s[pos..ende];
    if !gross(t) || t.chars().filter(|c| c.is_alphabetic()).count() < 2 {
        return false;
    }
    let g = gefaltet_ab(s, pos);
    let Some(erstes) = g.text.chars().next() else {
        return false;
    };
    laengster(s, pos, &g, d.orte.kandidaten(erstes)).is_none()
}

/// Überspringt den Abstand zwischen Teilen einer Adresse: Leerraum mit höchstens einem Komma
/// und höchstens einem Zeilenumbruch. Gibt die neue Stelle zurück und ob ein Komma oder ein
/// Zeilenumbruch dabei war.
fn luecke(s: &str, ab: usize) -> Option<(usize, bool)> {
    let rest = &s[ab..];
    let laenge = rest
        .find(|c: char| !(ist_leer(c) || c == ',' || c == '\n'))
        .unwrap_or(rest.len());
    let abstand = &rest[..laenge];
    let kommas = abstand.matches(',').count();
    let umbrueche = abstand.matches('\n').count();
    (laenge > 0 && kommas <= 1 && umbrueche <= 1).then_some((ab + laenge, kommas + umbrueche > 0))
}

/// Nummer eines Postfachs nach dem Wort: Leerraum und 1 bis 6 Ziffern; Länge in Bytes.
fn postfachnummer(s: &str, ab: usize) -> Option<usize> {
    let leer = leerraum(&s[ab..]);
    let ziffern = s[ab + leer..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    (leer > 0 && (1..=6).contains(&ziffern) && rechts_frei(s, ab + leer + ziffern))
        .then_some(leer + ziffern)
}

/// PLZ und Ort nach einer Hausnummer oder Postfachnummer.
struct Ort {
    ende: usize,
    /// Das Paar aus PLZ und Ortschaft steht im Ortschaftenverzeichnis.
    strikt: bool,
    /// Durch Komma oder Zeilenumbruch von der Nummer abgesetzt.
    abgesetzt: bool,
}

/// PLZ und Ort ab `ab`, davor optional ein Postfach («Seestrasse 12, Postfach, 8002 Zürich»)
/// und das Länderkennzeichen («CH-8002»). Strikt, wenn das Paar im Ortschaftenverzeichnis
/// steht; sonst lose, wenn der Ort irgendein amtlicher Orts- oder Gemeindename ist: Fach- und
/// Postfach-PLZ («8021», «1000 Lausanne 14») stehen nicht im Verzeichnis, sind in einer
/// Zustelladresse aber korrekt (Empfehlung 2018, Kap. 5).
fn ort_nach(s: &str, ab: usize, d: &Daten, postfaecher: &HashMap<usize, usize>) -> Option<Ort> {
    let (mut pos, mut abgesetzt) = luecke(s, ab)?;
    if let Some(&ende) = postfaecher.get(&pos) {
        let nummer = ende + postfachnummer(s, ende).unwrap_or(0);
        let (p, a) = luecke(s, nummer)?;
        pos = p;
        abgesetzt |= a;
    }
    if s[pos..].starts_with("CH-") || s[pos..].starts_with("FL-") {
        pos += 3;
    }
    let plz = s.get(pos..pos + 4)?;
    if !plz.bytes().all(|b| b.is_ascii_digit()) || plz.starts_with('0') {
        return None;
    }
    let leer = leerraum(&s[pos + 4..]);
    if leer == 0 {
        return None;
    }
    let name = pos + 4 + leer;
    let g = gefaltet_ab(s, name);
    if let Some(ende) = d
        .plz
        .get(plz)
        .and_then(|namen| laengster(s, name, &g, namen))
    {
        return Some(Ort {
            ende: mit_ortsnummer(s, ende),
            strikt: true,
            abgesetzt,
        });
    }
    let erstes = g.text.chars().next()?;
    let ende = laengster(s, name, &g, d.orte.kandidaten(erstes))?;
    Some(Ort {
        ende: mit_ortsnummer(s, ende),
        strikt: false,
        abgesetzt,
    })
}

/// Gefaltetes Stück Text ab `start`, lang genug für jeden Ortsnamen.
fn gefaltet_ab(s: &str, start: usize) -> Gefaltet {
    let mut ende = (start + 160).min(s.len());
    while !s.is_char_boundary(ende) {
        ende -= 1;
    }
    falten(&s[start..ende], true)
}

/// Ende des längsten Namens aus `kandidaten`, der bei `start` steht und an einer Wortgrenze
/// endet. `g` ist der gefaltete Text ab `start`.
fn laengster(s: &str, start: usize, g: &Gefaltet, kandidaten: &[String]) -> Option<usize> {
    kandidaten
        .iter()
        .filter(|k| g.text.starts_with(k.as_str()))
        .map(|k| start + g.zurueck(0, k.len()).1)
        .filter(|&e| rechts_frei(s, e))
        .max()
}

/// Nummer des Postamts nach dem Ort («1000 Lausanne 14»), wenn danach die Adresse endet.
fn mit_ortsnummer(s: &str, ende: usize) -> usize {
    let rest = &s[ende..];
    let leer = leerraum(rest);
    let ziffern = rest[leer..].bytes().take_while(u8::is_ascii_digit).count();
    if leer == 0 || !(1..=2).contains(&ziffern) {
        return ende;
    }
    let danach = &rest[leer + ziffern..];
    let danach = &danach[leerraum(danach)..];
    if danach.is_empty() || danach.starts_with(['\n', ',', '.', ';', ':', ')']) {
        ende + leer + ziffern
    } else {
        ende
    }
}

/// Adresse einer Behörde? Geprüft wird der Abschnitt unmittelbar davor, bis zum vorigen
/// Komma, Semikolon, Doppelpunkt, zur Klammer oder zum Zeilenanfang. Beginnt die Adresse am
/// Zeilenanfang, sind es die zwei Zeilen davor, bis zu einer Leerzeile; so zählt auch eine
/// Abteilung oder eine Person zwischen Behörde und Strasse («Bezirksgericht Zürich»,
/// «Einzelgericht», Strasse).
fn behoerde_davor(s: &str, start: usize, d: &Daten) -> bool {
    let lex = lexikon::lexikon(BEHOERDEN);
    let vor = s[..start].trim_end_matches(ist_leer);
    if let Some(zeilen) = vor.strip_suffix('\n') {
        return zeilen
            .rsplit('\n')
            .take(2)
            .take_while(|z| !z.trim().is_empty())
            .any(|z| behoerde_im_abschnitt(z, lex, d));
    }
    let vor = vor.strip_suffix([',', ';', ':', '(']).unwrap_or(vor);
    let anfang = vor
        .rfind(['\n', ',', ';', ':', '(', ')'])
        .map_or(0, |p| p + 1);
    behoerde_im_abschnitt(&vor[anfang..], lex, d)
}

/// Steht ein Behördenwort am Anfang des Abschnitts (auch nach einem Adjektiv:
/// «Eidgenössisches Departement») oder unter seinen letzten drei Wörtern («beim
/// Bezirksgericht», «Bezirksgericht Zürich»), und folgt ihm kein Satz?
fn behoerde_im_abschnitt(a: &str, lex: &Lexikon, d: &Daten) -> bool {
    let w = woerter(a);
    let index = |byte: usize| w.iter().position(|x| x.start <= byte && byte < x.ende);
    lex.finde_in(a).iter().any(|t| {
        let (Some(erstes), Some(letztes)) = (index(t.start), index(t.ende.saturating_sub(1)))
        else {
            return false;
        };
        let stellung =
            erstes == 0 || (erstes == 1 && adjektiv(wort(a, w[0]))) || letztes + 3 >= w.len();
        stellung && !satz_danach(a, &w[letztes + 1..], d)
    })
}

/// Folgt der Behörde ein Satz statt eines Namenszusatzes? Ein kleingeschriebenes Wort mit
/// mindestens vier Buchstaben vor einem grossgeschriebenen ist ein Verb vor seinem Objekt
/// («Das Bezirksgericht verurteilte Meier»). Kürzere Wörter («für», «des», «de») und die
/// Verbindungswörter aus Strassennamen («della») gehören zum Namen der Behörde
/// («Bundesamt für Justiz», «Ministero pubblico della Confederazione»).
fn satz_danach(a: &str, w: &[Wort], d: &Daten) -> bool {
    w.windows(2).any(|paar| {
        let t = wort(a, paar[0]);
        t.chars().next().is_some_and(char::is_lowercase)
            && t.chars().filter(|c| c.is_alphabetic()).count() >= 4
            && !verbindungswort(t, d)
            && gross(wort(a, paar[1]))
    })
}

/// Grossgeschriebenes Adjektiv vor einer Behörde: «Eidgenössisches», «Kantonales», «Zürcher».
fn adjektiv(t: &str) -> bool {
    const ENDUNGEN: [&str; 10] = [
        "isch", "ische", "isches", "ischen", "ischer", "ale", "ales", "aler", "alen", "er",
    ];
    gross(t) && t.chars().count() >= 4 && ENDUNGEN.iter().any(|e| t.ends_with(e))
}

/// Eine gefundene Adresse; `postfach` ist der Eintrag der Wortliste, wenn sie mit einem
/// Postfach beginnt.
#[derive(Clone, Copy)]
struct Stelle {
    start: usize,
    ende: usize,
    postfach: Option<usize>,
}

fn adressen(text: &Text) -> Vec<Treffer> {
    let s = text.als_str();
    let d = &*DATEN;
    let w = woerter(s);
    let postfach = lexikon::lexikon(POSTFACH);
    let postfach_treffer = postfach.finde(text);
    let postfaecher: HashMap<usize, usize> =
        postfach_treffer.iter().map(|t| (t.start, t.ende)).collect();

    let mut stellen = Vec::new();
    for (i, nummer) in w.iter().enumerate() {
        let Some(laenge) = hausnummer(wort(s, *nummer)) else {
            continue;
        };
        let nummer_ende = nummer.start + laenge;
        let ort = ort_nach(s, nummer_ende, d, &postfaecher);
        if let Some(strasse) = getypte_strasse(s, &w, i, d) {
            let ende = match &ort {
                Some(o) => Some(o.ende),
                None => (!strasse.nur_mit_ort && !zaehlt_etwas(s, nummer_ende, d))
                    .then_some(nummer_ende),
            };
            if let Some(ende) = ende {
                stellen.push(Stelle {
                    start: strasse.start,
                    ende,
                    postfach: None,
                });
            }
        } else if let Some(o) = ort.filter(|o| o.strikt && o.abgesetzt) {
            // Ohne Typ höchstens dreistellig: «Jahrgang 1970, 8002 Zürich» ist keine Adresse.
            let ziffern = wort(s, *nummer)
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            if let Some(start) = ungetypte_strasse(s, &w, i, d).filter(|_| ziffern <= 3) {
                stellen.push(Stelle {
                    start,
                    ende: o.ende,
                    postfach: None,
                });
            }
        }
    }
    // Postfach mit Nummer, PLZ und Ort; das Wort allein ist keine Adresse.
    for t in &postfach_treffer {
        let Some(nummer) = postfachnummer(s, t.ende) else {
            continue;
        };
        if let Some(o) = ort_nach(s, t.ende + nummer, d, &HashMap::new()) {
            stellen.push(Stelle {
                start: t.start,
                ende: o.ende,
                postfach: Some(t.eintrag),
            });
        }
    }

    ohne_ueberlappung(stellen)
        .into_iter()
        .filter(|st| !behoerde_davor(s, st.start, d))
        .map(|st| {
            let t = Treffer::neu(ADRESSE, st.start, st.ende, Art::Identifikator);
            match st.postfach {
                Some(i) => t.mit_beleg(postfach.beleg(i)),
                None => t,
            }
        })
        .collect()
}

/// Bei Überlappung bleibt die längere Stelle.
fn ohne_ueberlappung(mut v: Vec<Stelle>) -> Vec<Stelle> {
    v.sort_by(|a, b| {
        (b.ende - b.start)
            .cmp(&(a.ende - a.start))
            .then(a.start.cmp(&b.start))
    });
    let mut gewaehlt: Vec<Stelle> = Vec::new();
    for st in v {
        if gewaehlt
            .iter()
            .all(|g| st.ende <= g.start || st.start >= g.ende)
        {
            gewaehlt.push(st);
        }
    }
    gewaehlt.sort_by_key(|st| st.start);
    gewaehlt
}

/// Eine PLZ vor dem Gemeindenamen («wohnhaft in 8002 Zürich»): Länge mit Leerraum.
fn plz_davor(s: &str, pos: usize) -> Option<usize> {
    let plz = s.get(pos..pos + 4)?;
    let leer = leerraum(&s[pos + 4..]);
    (plz.bytes().all(|b| b.is_ascii_digit()) && leer > 0).then_some(4 + leer)
}

/// Ist der gefundene Name grossgeschrieben? Ein kleingeschriebener französischer Artikel
/// davor zählt nicht («à la Chaux-de-Fonds»). «residente a lungo» ist kein Ort.
fn grossgeschrieben(name: &str) -> bool {
    let ohne_artikel = ["la ", "le ", "les ", "l'", "l\u{2019}"]
        .iter()
        .find_map(|a| name.strip_prefix(a))
        .unwrap_or(name);
    gross(ohne_artikel)
}

fn wohnorte(text: &Text) -> Vec<Treffer> {
    // Ein weicher Umbruch zwischen Auslösewort und Gemeinde zählt wie ein Leerzeichen
    // («wohnhaft in⏎Winterthur»); die Versätze sind dieselben wie im Text.
    let s = text.verbunden();
    let d = &*DATEN;
    let lex = lexikon::lexikon(AUSLOESER);
    let mut v = Vec::new();
    for t in lex.finde(text) {
        let e = &lex.eintraege[t.eintrag];
        let mut pos = t.ende + leerraum_weich(text, t.ende);
        if e.merkmal.as_deref() == Some("doppelpunkt") {
            if !s[pos..].starts_with(':') {
                continue;
            }
            pos += 1;
            pos += leerraum_weich(text, pos);
        } else {
            let praepositionen: &[&str] = match e.sprache {
                Sprache::De => &["in"],
                Sprache::Fr => &["à", "a"],
                Sprache::It => &["a", "in"],
            };
            let Some(p) = praepositionen
                .iter()
                .find(|p| s[pos..].starts_with(**p) && rechts_frei(s, pos + p.len()))
            else {
                continue;
            };
            pos += p.len();
            let leer = leerraum_weich(text, pos);
            if leer == 0 {
                continue;
            }
            pos += leer;
        }
        pos += plz_davor(s, pos).unwrap_or(0);
        let g = gefaltet_ab(s, pos);
        let Some(erstes) = g.text.chars().next() else {
            continue;
        };
        let Some(ende) = laengster(s, pos, &g, d.gemeinden.kandidaten(erstes)) else {
            continue;
        };
        if !grossgeschrieben(&s[pos..ende]) {
            continue;
        }
        v.push(
            Treffer::neu(WOHNORT, pos, ende, Art::Personenangabe).mit_beleg(lex.beleg(t.eintrag)),
        );
    }
    v
}

pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let mut t = adressen(text);
    t.extend(wohnorte(text));
    t
}

pub fn info() -> Vec<RegelInfo> {
    vec![
        RegelInfo {
            id: ADRESSE,
            klasse: Klasse::B,
            titel: Txt::neu("Postadresse", "Postal address"),
            erkennt: Txt::neu(
                "Strasse mit Hausnummer, dazu PLZ und Ort, wenn sie folgen; Postfach mit Nummer, PLZ und Ort",
                "Street with house number, with postcode and locality if they follow; PO box with number, postcode and locality",
            ),
            methode: Txt::neu(METHODE_ADRESSE_DE, METHODE_ADRESSE_EN),
            hinweis: None,
            grundlage: vec![
                &normen::DSG_5_A,
                &GEONV_26B,
                &VPG_23_1_B,
                &GEONV_3_F,
                &GEONV_24,
                &normen::OR_552_1,
                &normen::OR_594_2,
            ],
            quellen: vec![
                Q_BGER,
                Q_EMPFEHLUNG,
                Q_STRASSEN,
                Q_ORTSCHAFTEN,
                Q_GEMEINDEN,
                Q_GEONV,
                Q_VPG,
                Q_BEHOERDEN,
            ],
            lexika: vec![POSTFACH, BEHOERDEN, STRASSENTYPEN, PLZ, GEMEINDEN],
            guete: Guete::NichtGemessen,
            grenzen: vec![
                Txt::neu(
                    "Eine Adresse, die unmittelbar auf eine erkannte juristische Person oder KmGK folgt, nur durch Komma, Leerzeichen, Tabulator oder Zeilenwechsel getrennt (höchstens vier Zeichen), gilt als Adresse der Firma: ROT mit dem Zusatz «Adresse der Firma», wenn die Pflicht der Rolle «ein Geheimnis» schützt (alle Rollen ausser professional und none); sonst kein Fund. Andere Adressen meldet die Regel wie jede Adresse, auch nach einer Kollektiv- oder Kommanditgesellschaft: Deren unbeschränkt haftende Gesellschafter sind natürliche Personen (Art. 552 Abs. 1 OR, Art. 594 Abs. 2 OR). Hält die Regel firma einen Ortsnamen mit «AG» für eine Firma (siehe explain firma), gilt so auch die Adresse einer natürlichen Person als Adresse der Firma.",
                    "An address that directly follows a detected legal person or KmGK, separated only by a comma, space, tab or line break (at most four characters), counts as the company\u{2019}s address: RED with the addition «company address» if the role\u{2019}s duty protects «a secret» (all roles except professional and none); otherwise no finding. Other addresses are reported like any address, also after a general or limited partnership: their partners with unlimited liability are natural persons (Art. 552 Abs. 1 OR, Art. 594 Abs. 2 OR). If the rule firma takes a place name with «AG» for a company (see explain firma), the address of a natural person also counts as the company\u{2019}s address.",
                ),
                Txt::neu(
                    "Strassen ohne erfassten Typ («Im Rank 3», Flurnamen) nur mit PLZ und Ort aus dem Verzeichnis, durch Komma oder Zeilenumbruch abgesetzt; die Endungen «-platz», «-ring» und «-steig» nur mit PLZ und Ort, weil sie auch gewöhnliche Wörter mit Nummer bilden («Parkplatz 12», «Bahnsteig 3», «Monitoring 2»).",
                    "Streets without a covered type («Im Rank 3», field names) only with postcode and locality from the directory, separated by comma or line break; the endings «-platz», «-ring» and «-steig» only with postcode and locality, as they also form ordinary words followed by a number («Parkplatz 12», «Bahnsteig 3», «Monitoring 2»).",
                ),
                Txt::neu(
                    "Ohne PLZ und Ort zählt eine Zahl nicht als Hausnummer, wenn ihr ein grossgeschriebenes Wort folgt, das kein Ortsname ist («via E-Mail 2 Dokumente»); folgt ein kleingeschriebenes Wort («via E-Mail 2 weitere Dokumente») oder ist das Wort eine Zusammensetzung auf «-weg» oder «-strasse» («Lösungsweg 2»), bleibt ein Fehlalarm möglich.",
                    "Without postcode and locality a number does not count as a house number when a capitalised word follows that is not a place name («via E-Mail 2 Dokumente»); if a lower-case word follows («via E-Mail 2 weitere Dokumente») or the word is a compound ending in «-weg» or «-strasse» («Lösungsweg 2»), a false alarm remains possible.",
                ),
                Txt::neu(
                    "Nicht erkannt: Nummer vor der Strasse («12, rue du Lac»), kleingeschriebene deutsche Strassennamen, kleingeschriebene Kurzformen («av. de la Gare 3»; «ch.» heisst in Rechtstexten chiffre), Namen ohne Eigennamen («Rue de l’O 3»), Adressen ohne Hausnummer.",
                    "Not detected: number before the street («12, rue du Lac»), lower-case German street names, lower-case short forms («av. de la Gare 3»; «ch.» means chiffre in legal texts), names without a proper name («Rue de l’O 3»), addresses without a house number.",
                ),
                Txt::neu(
                    "Behördenadressen werden nur erkannt, wenn die Behördenbezeichnung unmittelbar davor steht (Abschnitt oder zwei Zeilen davor) und ihr kein Verb vor einem Namen folgt («Das Bezirksgericht verurteilte Meier, Seestrasse 12» bleibt eine Adresse). Eine Person zwischen Behörde und Strasse ohne Leerzeile gilt als zu Händen der Behörde; «Verwaltung» und «police» treffen auch Private («Hausverwaltung», Versicherungspolice).",
                    "Addresses of authorities are only recognised when the authority's name directly precedes them (segment or two lines before) and is not followed by a verb before a name («Das Bezirksgericht verurteilte Meier, Seestrasse 12» stays an address). A person between authority and street without a blank line counts as care of the authority; «Verwaltung» and «police» also match private parties.",
                ),
                Txt::neu(
                    "PLZ und Ort allein («8002 Zürich») meldet die Regel nicht: Nach den Anonymisierungsregeln des Bundesgerichts bleiben Ortsbezeichnungen in der Regel offen, soweit sie für das Verständnis des Entscheids dienlich sind (Art. 4 Abs. 1 lit. e). Ist es der Wohnort einer Person, bleibt er so unerkannt; den Wohnort einer Partei anonymisiert das Bundesgericht in der Regel (Art. 7 Abs. 1).",
                    "Postcode and place alone («8002 Zürich») are not reported: under the anonymisation rules of the Federal Supreme Court, place names generally stay open insofar as they help to understand the judgment (Art. 4 Abs. 1 lit. e). If it is a person\u{2019}s place of residence, it goes undetected; the Court generally anonymises the place of residence of a party (Art. 7 Abs. 1).",
                ),
                Txt::neu(
                    "Die Abdeckung ist an Strassennamen gemessen, nicht an Adressen; der Anteil der Gebäudeadressen mit erfasstem Typ ist nicht gemessen.",
                    "Coverage is measured on street names, not on addresses; the share of building addresses with a covered type is not measured.",
                ),
            ],
        },
        RegelInfo {
            id: WOHNORT,
            klasse: Klasse::B,
            titel: Txt::neu("Wohnort", "Place of residence"),
            erkennt: Txt::neu(
                "Politische Gemeinde nach «wohnhaft in», «mit Wohnsitz in», «domicilié à», «domiciliato a», «residente a» oder nach «Wohnsitz:», «Wohnort:», «domicile:», «domicilio:»",
                "Political municipality after «wohnhaft in», «mit Wohnsitz in», «domicilié à», «domiciliato a», «residente a» or after «Wohnsitz:», «Wohnort:», «domicile:», «domicilio:»",
            ),
            methode: Txt::neu(METHODE_WOHNORT_DE, METHODE_WOHNORT_EN),
            hinweis: None,
            grundlage: vec![&normen::DSG_5_A, &ZGB_23_1, &GEONV_19_1],
            quellen: vec![Q_BGER, Q_GEMEINDEN, Q_GEONV, Q_AUSLOESER],
            lexika: vec![AUSLOESER, GEMEINDEN],
            guete: Guete::NichtGemessen,
            grenzen: vec![
                Txt::neu(
                    "Nur amtliche Gemeindenamen: Exonyme («Genf», «Ginevra», «Zurigo») und Ortsteile ohne eigene Gemeinde werden nicht erkannt; Liechtenstein und das Ausland fehlen.",
                    "Official municipality names only: exonyms («Genf», «Ginevra», «Zurigo») and localities without their own municipality are not detected; Liechtenstein and foreign places are missing.",
                ),
                Txt::neu(
                    "«Wohnsitz in» ohne «mit» davor wird nicht gemeldet, weil es oft den Staat oder einen Rechtsbegriff meint; «domicilié au Locle» (au = à le) wird nicht erkannt.",
                    "«Wohnsitz in» without a preceding «mit» is not reported, as it often refers to a country or a legal concept; «domicilié au Locle» (au = à le) is not detected.",
                ),
                Txt::neu(
                    "Der Name muss grossgeschrieben sein («residente a lungo» ist kein Ort); ob der Wohnort zu einer natürlichen Person gehört, entscheidet die Regel nicht.",
                    "The name must be capitalised («residente a lungo» is no place); the rule does not decide whether the place of residence belongs to a natural person.",
                ),
                Txt::neu(
                    "Ohne Auslösewort bleibt ein Wohnort unerkannt («Die Patientin aus Ossingen»).",
                    "Without a trigger word, a place of residence goes undetected («Die Patientin aus Ossingen»).",
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stellen(roh: &str, regel: &str) -> Vec<String> {
        let text = Text::neu(roh);
        erkenne(&text)
            .into_iter()
            .filter(|t| t.regel == regel)
            .map(|t| text.ausschnitt(t.start..t.ende).to_string())
            .collect()
    }

    fn adr(roh: &str) -> Vec<String> {
        stellen(roh, ADRESSE)
    }

    fn ort(roh: &str) -> Vec<String> {
        stellen(roh, WOHNORT)
    }

    // --- adresse: Positivfälle ---

    #[test]
    fn spec_beispiel_eine_stelle() {
        let roh = "Unser Mitarbeiter Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich,\nAHV 756.1234.5678.97, ist seit März zu 50 % arbeitsunfähig (Diagnose F32.1).";
        assert_eq!(adr(roh), ["Seestrasse 12, 8002 Zürich"]);
        let text = Text::neu(roh);
        let t = erkenne(&text);
        let a = t.iter().find(|t| t.regel == ADRESSE).expect("adresse");
        assert_eq!(text.position(a.start), (1, 49));
        assert_eq!(a.art, Art::Identifikator);
    }

    /// Unsichtbare Formatzeichen im Strassennamen und neben dem Leerraum werden übergangen;
    /// die Stelle steht im ganzen Text.
    #[test]
    fn formatzeichen_in_strassennamen() {
        for roh in [
            "See\u{AD}strasse 12, 8002 Zürich",
            "See\u{200B}strasse 12, 8002 Zürich",
            "Seestras\u{AD}se 12, 8002 Zürich",
            "Seestrasse\u{200B} 12, 8002 Zürich",
            "Seestrasse \u{2060}12, 8002 Zürich",
            "Seestrasse 12,\u{FEFF} 8002 Zürich",
        ] {
            assert_eq!(adr(roh), [roh], "{roh:?}");
        }
        assert_eq!(
            adr("Sie wohnt an der See\u{AD}strasse 12 in Zürich."),
            ["See\u{AD}strasse 12"]
        );
        let roh = "Brief an Rue du Lac\u{200B} 12, 1201 Genève.";
        assert_eq!(adr(roh), ["Rue du Lac\u{200B} 12, 1201 Genève"]);
        let w = woerter("See\u{AD}strasse \u{200B}12");
        assert_eq!(w.len(), 2);
        assert_eq!(wort("See\u{AD}strasse \u{200B}12", w[1]), "12");
    }

    #[test]
    fn ueber_zeilenumbruch_und_lose_plz() {
        // 3000 ist keine Domizil-PLZ; nach erfasstem Typ genügt ein amtlicher Ortsname.
        assert_eq!(
            adr("Erika Beispiel\nMusterweg 5a\n3000 Bern\n"),
            ["Musterweg 5a\n3000 Bern"]
        );
    }

    #[test]
    fn kurzform_und_laenderkennzeichen() {
        assert_eq!(
            adr("Bitte an Bahnhofstr. 1, CH-8001 Zürich senden."),
            ["Bahnhofstr. 1, CH-8001 Zürich"]
        );
    }

    #[test]
    fn franzoesisch_und_italienisch() {
        assert_eq!(
            adr("Elle habite Rue de la Gare 3, 1003 Lausanne, depuis mai."),
            ["Rue de la Gare 3, 1003 Lausanne"]
        );
        assert_eq!(
            adr("Abita in via Nassa 5, 6900 Lugano."),
            ["via Nassa 5, 6900 Lugano"]
        );
        assert_eq!(adr("Il habite Av. de la Gare 3."), ["Av. de la Gare 3"]);
        assert_eq!(
            adr("Grand-Rue 12, 1095 Lutry"),
            ["Grand-Rue 12, 1095 Lutry"]
        );
    }

    #[test]
    fn ohne_plz_mit_erfasstem_typ() {
        assert_eq!(adr("Er wohnt an der Seestrasse 12."), ["Seestrasse 12"]);
        assert_eq!(
            adr("Sie wohnt an der Alten Landstrasse 12 in Kilchberg."),
            ["Alten Landstrasse 12"]
        );
        assert_eq!(adr("Seestrasse 10-12"), ["Seestrasse 10-12"]);
    }

    #[test]
    fn ohne_typ_nur_mit_verzeichnis() {
        assert_eq!(
            adr("Hans Muster, Im Rank 3, 6300 Zug"),
            ["Im Rank 3, 6300 Zug"]
        );
        // Ohne PLZ und Ort keine Adresse.
        assert!(adr("Hans Muster, Im Rank 3").is_empty());
        // Nicht abgesetzt: keine Adresse.
        assert!(adr("Im Rank 3 8002 Zürich").is_empty());
    }

    #[test]
    fn schreibweisen_der_ortschaft() {
        assert_eq!(
            adr("Rebstrasse 44, 8802 Kilchberg ZH"),
            ["Rebstrasse 44, 8802 Kilchberg ZH"]
        );
        assert_eq!(
            adr("Rebstrasse 44, 8802 Kilchberg"),
            ["Rebstrasse 44, 8802 Kilchberg"]
        );
        assert_eq!(
            adr("Zentralstrasse 1, 2502 Biel"),
            ["Zentralstrasse 1, 2502 Biel"]
        );
        assert_eq!(
            adr("Avenue de la Gare 1, 1000 Lausanne 14"),
            ["Avenue de la Gare 1, 1000 Lausanne 14"]
        );
    }

    #[test]
    fn postfach() {
        assert_eq!(
            adr("Postfach 1234, 8021 Zürich"),
            ["Postfach 1234, 8021 Zürich"]
        );
        assert_eq!(
            adr("case postale 12, 1001 Lausanne"),
            ["case postale 12, 1001 Lausanne"]
        );
        assert_eq!(
            adr("casella postale 123, 6901 Lugano"),
            ["casella postale 123, 6901 Lugano"]
        );
        assert_eq!(
            adr("Seestrasse 12, Postfach, 8002 Zürich"),
            ["Seestrasse 12, Postfach, 8002 Zürich"]
        );
        let text = Text::neu("Postfach 1234, 8021 Zürich");
        let t = erkenne(&text);
        let beleg = t[0].beleg.as_ref().expect("Beleg des Postfachs");
        assert_eq!(beleg.quelle, "SR 783.01 Art. 29 Abs. 4bis");
    }

    #[test]
    fn gewoehnliche_woerter_nur_mit_ort() {
        assert!(adr("Wir haben Parkplatz 12 reserviert.").is_empty());
        assert_eq!(
            adr("Bahnhofplatz 3, 3011 Bern"),
            ["Bahnhofplatz 3, 3011 Bern"]
        );
        assert!(adr("Das Monitoring 2 läuft.").is_empty());
        assert!(adr("Treffpunkt am Bahnsteig 3.").is_empty());
    }

    #[test]
    fn zahl_vor_einem_nomen_zaehlt_etwas() {
        assert!(adr("Ich habe Ihnen via E-Mail 2 Dokumente geschickt.").is_empty());
        assert!(adr("Fahrt via Olten 2 Stunden.").is_empty());
        assert!(adr("Wir sprachen via Zoom 3 Mal.").is_empty());
        // Gegenprobe: ein Ortsname, ein kleines Wort oder ein Buchstabe danach.
        assert_eq!(adr("Seestrasse 12 Zürich"), ["Seestrasse 12"]);
        assert_eq!(adr("Abito in via Nassa 5 a Lugano."), ["via Nassa 5"]);
        assert_eq!(adr("Seestrasse 12 A"), ["Seestrasse 12"]);
    }

    // --- adresse: Gegenbeispiele ---

    #[test]
    fn plz_und_ort_allein_sind_ein_ortsname() {
        assert!(adr("Die Sitzung ist in 8002 Zürich.").is_empty());
        assert!(adr("Bundesgericht, 1000 Lausanne 14").is_empty());
    }

    #[test]
    fn strasse_ohne_nummer() {
        assert!(adr("Er wohnt an der Bahnhofstrasse.").is_empty());
        assert!(adr("Elle habite rue de Lausanne.").is_empty());
    }

    #[test]
    fn artikel_ist_keine_strasse() {
        assert!(adr("Art. 12, 8002").is_empty());
        assert!(adr("Gemäss Art. 12, 8002 Zürich gilt").is_empty());
        assert!(adr("Hans Muster, Jahrgang 1970, 8002 Zürich").is_empty());
        assert!(adr("Bestätigung via SMS 2 Mal").is_empty());
    }

    #[test]
    fn fundstellen_in_rechtstexten_sind_keine_strassen() {
        // So in amtlichen Erlasstexten: «ch.» steht dort für chiffre.
        assert!(adr("Nouvelle teneur selon l’art. 3 ch. 2 de l’AF du 11 déc. 2009").is_empty());
        assert!(adr("Nouvelle expression selon le ch. I 18 de l’O du 15 juin 2012").is_empty());
        assert!(adr("Ch. 4 de l’ACF du 25 août 1959").is_empty());
        // Gegenprobe: die Kurzform grossgeschrieben vor einem Namen.
        assert_eq!(adr("Ch. des Fleurs 3"), ["Ch. des Fleurs 3"]);
    }

    #[test]
    fn postfach_ohne_nummer() {
        assert!(adr("Postfach, 8021 Zürich").is_empty());
        assert!(adr("Postfach 1234").is_empty());
    }

    #[test]
    fn behoerdenadressen_sind_vorbehalten() {
        // Ohne die Behörde davor ist jede Stelle eine Adresse, mit ihr nicht.
        let faelle = [
            (
                "Bundesgericht, ",
                "Avenue du Tribunal-Fédéral 29, 1000 Lausanne 14",
            ),
            ("Bundesgericht, ", "Schweizerhofquai 6, 6004 Luzern"),
            (
                "Bundesamt für Landestopografie swisstopo\n",
                "Seftigenstrasse 264\n3084 Wabern",
            ),
            (
                "Bezirksgericht Zürich\nEinzelgericht\n",
                "Wengistrasse 30\n8004 Zürich",
            ),
            (
                "Eingabe beim Bezirksgericht, ",
                "Wengistrasse 30, 8004 Zürich",
            ),
            (
                "Die Eingabe ist an das Amt für Wirtschaft, ",
                "Musterstrasse 20, 8002 Zürich",
            ),
            (
                "Eidgenössisches Departement des Innern, ",
                "Musterstrasse 20, 3011 Bern",
            ),
            ("Gemeindeverwaltung\n", "Dorfstrasse 1\n8802 Kilchberg"),
            (
                "recours au Tribunal cantonal, ",
                "Rue du Lac 3, 1003 Lausanne",
            ),
            ("Ministère public, ", "Rue du Lac 3, 1204 Genève"),
            ("Tribunale cantonale, ", "via Pretorio 16, 6500 Bellinzona"),
            (
                "Ministero pubblico della Confederazione, ",
                "via Nassa 5, 6900 Lugano",
            ),
        ];
        for (behoerde, adresse) in faelle {
            assert_eq!(adr(adresse), [adresse], "ohne Behörde");
            let mit = format!("{behoerde}{adresse}");
            assert!(adr(&mit).is_empty(), "{mit:?}");
        }
    }

    #[test]
    fn behoerde_im_satz_ist_keine_bezeichnung() {
        for roh in [
            "Das Bezirksgericht verurteilte Hans Muster, Seestrasse 12, 8002 Zürich.",
            "Das Bezirksgericht verurteilte Meier, Seestrasse 12, 8002 Zürich.",
            "Die Polizei kontrollierte Meier, Seestrasse 12, 8002 Zürich.",
        ] {
            assert_eq!(adr(roh), ["Seestrasse 12, 8002 Zürich"], "{roh:?}");
        }
        assert_eq!(
            adr("Le Tribunal condamne Dupont, Rue du Lac 3, 1003 Lausanne."),
            ["Rue du Lac 3, 1003 Lausanne"]
        );
        // Leerzeile: die Behörde oben gehört nicht zur Adresse.
        assert_eq!(
            adr("Bezirksgericht Zürich\n\nHans Muster\nSeestrasse 12\n8002 Zürich"),
            ["Seestrasse 12\n8002 Zürich"]
        );
        // Im Rubrum bleibt die Partei, die Behörde fällt weg.
        assert_eq!(
            adr(
                "A. Muster, Seestrasse 12, 8002 Zürich, Beschwerdeführer, gegen Bezirksgericht Zürich, Wengistrasse 30, 8004 Zürich"
            ),
            ["Seestrasse 12, 8002 Zürich"]
        );
    }

    // --- wohnort ---

    #[test]
    fn wohnort_de_fr_it() {
        assert_eq!(ort("Die Klägerin, wohnhaft in Bern, verlangt"), ["Bern"]);
        assert_eq!(
            ort("eine Person mit Wohnsitz in Biel/Bienne"),
            ["Biel/Bienne"]
        );
        assert_eq!(ort("eine Person mit Wohnsitz in Bienne"), ["Bienne"]);
        assert_eq!(ort("La recourante, domiciliée à Lausanne,"), ["Lausanne"]);
        assert_eq!(ort("domicilié a Genève"), ["Genève"]);
        assert_eq!(ort("domiciliée à La Chaux-de-Fonds"), ["La Chaux-de-Fonds"]);
        assert_eq!(ort("Il ricorrente, domiciliato a Lugano,"), ["Lugano"]);
        assert_eq!(ort("residente a Bellinzona"), ["Bellinzona"]);
    }

    #[test]
    fn wohnort_feldbezeichnung() {
        assert_eq!(ort("Name: Erika Beispiel\nWohnsitz: Zug"), ["Zug"]);
        assert_eq!(ort("Wohnort: Bern"), ["Bern"]);
        assert_eq!(ort("domicile : Sion"), ["Sion"]);
        assert_eq!(ort("domicilio: Locarno"), ["Locarno"]);
    }

    /// Die Beispiele der Grenzen und der Methode verhalten sich wie beschrieben.
    #[test]
    fn grenzen_und_methode_wie_beschrieben() {
        use crate::fund::{Rolle, Stufe};
        use crate::stufen::Optionen;
        let infos = info();
        let (adresse, wohnort) = (&infos[0], &infos[1]);
        let mit_rolle = |rolle| Optionen {
            rolle: Some(rolle),
            ..Optionen::default()
        };
        // Unmittelbar nach einer erkannten Firma: Adresse der Firma, ROT nur mit einer
        // Pflicht, die jedes Geheimnis schützt, sonst kein Fund.
        let satz = "Rechnung an Muster AG, Seestrasse 12, 8002 Zürich.";
        let adressfund = |opt: &Optionen| {
            crate::pruefe(satz, opt)
                .funde
                .into_iter()
                .find(|f| f.regel == ADRESSE)
        };
        assert!(adressfund(&Optionen::default()).is_none());
        assert!(adressfund(&mit_rolle(Rolle::Professional)).is_none());
        assert!(adressfund(&mit_rolle(Rolle::None)).is_none());
        let f = adressfund(&mit_rolle(Rolle::Lawyer)).expect("ROT mit Rolle lawyer");
        assert_eq!(f.stufe, Stufe::Rot);
        assert_eq!(
            f.zusatz.as_ref().map(|z| z.de.as_str()),
            Some("Adresse der Firma")
        );
        // Ohne Firma davor und nach einer Kollektivgesellschaft bleibt es eine gemeldete
        // Adresse.
        for satz in [
            "Seestrasse 12, 8002 Zürich",
            "Rechnung an Muster & Co., Seestrasse 12, 8002 Zürich.",
        ] {
            let funde = crate::pruefe(satz, &Optionen::default()).funde;
            assert!(
                funde.iter().any(|f| f.regel == ADRESSE),
                "{satz}: {funde:?}"
            );
        }
        assert!(adresse.grenzen[0].de.starts_with(
            "Eine Adresse, die unmittelbar auf eine erkannte juristische Person oder KmGK folgt"
        ));
        assert!(adresse.grundlage.contains(&&normen::OR_552_1));
        assert!(adresse.grundlage.contains(&&normen::OR_594_2));
        // PLZ und Ort allein und ein Wohnort ohne Auslösewort bleiben unerkannt.
        assert!(adr("8002 Zürich").is_empty());
        assert!(
            adresse
                .grenzen
                .iter()
                .any(|g| g.de.contains("«8002 Zürich»")
                    && g.de.contains("(Art. 7 Abs. 1)")
                    && g.en.contains("(Art. 7 Abs. 1)"))
        );
        assert!(ort("Die Patientin aus Ossingen").is_empty());
        assert!(
            wohnort
                .grenzen
                .iter()
                .any(|g| g.de.contains("«Die Patientin aus Ossingen»"))
        );
        assert!(wohnort.methode.de.ends_with("(Art. 7 Abs. 1)."));
        assert!(wohnort.methode.en.ends_with("(Art. 7 Abs. 1)."));
        assert!(wohnort.methode.de.contains("«wohnhaft in⏎Winterthur»"));
    }

    /// Ein weicher Zeilenumbruch zwischen Auslöser, Präposition und Ort zählt wie Leerraum;
    /// ein harter nicht.
    #[test]
    fn wohnort_ueber_weichen_umbruch() {
        for roh in [
            "Der Klient ist wohnhaft in\nWinterthur und hat Schulden.",
            "Der Klient ist wohnhaft\nin Winterthur und hat Schulden.",
            "Der Klient ist wohnhaft in\r\n  Winterthur und hat Schulden.",
            "Der Klient ist wohnhaft in\u{200B} Winterthur und hat Schulden.",
            "Le client, domicilié à\nWinterthur, a des dettes.",
        ] {
            assert_eq!(ort(roh), ["Winterthur"], "{roh:?}");
        }
        let roh = "Der Klient ist wohnhaft in\nWinterthur.";
        let text = Text::neu(roh);
        let t = erkenne(&text);
        assert_eq!(&roh[t[0].start..t[0].ende], "Winterthur");
        // Leerzeile, Listenpunkt und Doppelpunkt am Zeilenende trennen.
        assert!(ort("Der Klient ist wohnhaft in\n\nWinterthur.").is_empty());
        assert!(ort("Der Klient ist wohnhaft in\n- Winterthur").is_empty());
        assert!(ort("Wohnsitz:\nWinterthur").is_empty());
    }

    #[test]
    fn wohnort_schreibweisen() {
        assert_eq!(ort("wohnhaft in Wald"), ["Wald"]);
        assert_eq!(ort("wohnhaft in Zell ZH"), ["Zell ZH"]);
        assert_eq!(ort("wohnhaft in 8002 Zürich"), ["Zürich"]);
    }

    #[test]
    fn wohnort_gegenbeispiele() {
        assert!(ort("Die Sitzung findet in Bern statt.").is_empty());
        assert!(ort("eine Person mit Wohnsitz in der Schweiz").is_empty());
        assert!(ort("le débiteur domicilié en Suisse").is_empty());
        assert!(ort("se uno degli sposi è domiciliato in Svizzera").is_empty());
        assert!(ort("Wohnsitz in Bern").is_empty());
        assert!(ort("wohnhaft in Genf").is_empty());
        assert!(ort("residente a lungo").is_empty());
        assert!(ort("wohnhaft in bern").is_empty());
    }

    /// Der weiche Umbruch auch nach «domiciliée à» und «domiciliato a» mit CRLF.
    #[test]
    fn wohnort_ueber_weichen_umbruch_fr_it() {
        assert_eq!(
            ort("Der Klient ist wohnhaft in\nWinterthur und fragt nach."),
            ["Winterthur"]
        );
        assert_eq!(
            ort("Der Klient ist wohnhaft\nin Winterthur und fragt nach."),
            ["Winterthur"]
        );
        assert_eq!(
            ort("La recourante, domiciliée à\nLausanne, demande"),
            ["Lausanne"]
        );
        assert_eq!(ort("Il ricorrente, domiciliato a\r\nLugano,"), ["Lugano"]);
        // Eine Leerzeile und ein Listenpunkt trennen weiter.
        assert!(ort("Der Klient ist wohnhaft in\n\nWinterthur.").is_empty());
        assert!(ort("Der Klient ist wohnhaft in\n- Winterthur").is_empty());
    }

    #[test]
    fn wohnort_mit_beleg() {
        let text = Text::neu("wohnhaft in Bern");
        let t = erkenne(&text);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].art, Art::Personenangabe);
        let beleg = t[0].beleg.as_ref().expect("Beleg");
        assert_eq!(beleg.begriff, "wohnhaft");
        assert_eq!(beleg.quelle, "SR 661 Art. 4a Abs. 2");
    }

    // --- Gegenprobe und Katalog ---

    #[test]
    fn gegenprobe_regeln_melden_etwas() {
        // Würde die Regel nichts tun, wäre dieser Test rot.
        let text = Text::neu("Erika Beispiel, Seestrasse 12, 8002 Zürich, wohnhaft in Zürich.");
        let regeln: Vec<&str> = erkenne(&text).iter().map(|t| t.regel).collect();
        assert_eq!(regeln, [ADRESSE, WOHNORT]);
    }

    #[test]
    fn info_zwei_regeln_klasse_b() {
        let i = info();
        assert_eq!(
            i.iter().map(|r| r.id).collect::<Vec<_>>(),
            [ADRESSE, WOHNORT]
        );
        for r in &i {
            assert_eq!(r.klasse, Klasse::B);
            for d in &r.lexika {
                assert!(lexikon::inhalt(d).is_some(), "lexika/{d} fehlt");
            }
        }
    }

    /// Die Zahlen in der Methode stammen aus den Registern, nicht von Hand.
    #[test]
    fn methode_nennt_die_gemessenen_zahlen() {
        let typen = lexikon::register(STRASSENTYPEN);
        let abdeckung = typen.kopf("abdeckung").expect("Kopf abdeckung");
        let prozent = abdeckung.split_whitespace().next().expect("Prozent");
        assert!(METHODE_ADRESSE_EN.contains(&format!("{prozent} %")));
        assert!(METHODE_ADRESSE_DE.contains(&format!("{} %", prozent.replace('.', ","))));
        let gesamt = abdeckung
            .split(" von ")
            .nth(1)
            .and_then(|r| r.split(')').next())
            .expect("Anzahl Namen");
        let (tausend, rest) = gesamt.split_at(gesamt.len() - 3);
        assert!(METHODE_ADRESSE_DE.contains(&format!("{tausend} {rest} Strassennamen")));
        assert!(METHODE_ADRESSE_EN.contains(&format!("{tausend},{rest} street names")));
        let gemeinden = lexikon::register(GEMEINDEN).zeilen.len();
        assert!(METHODE_WOHNORT_DE.contains(&format!("({gemeinden} Gemeinden)")));
        assert!(METHODE_WOHNORT_EN.contains(&format!("({gemeinden} municipalities)")));
    }

    #[test]
    fn register_kopf_stimmt_mit_zeilen() {
        for datei in [PLZ, GEMEINDEN, STRASSENTYPEN] {
            let reg = lexikon::register(datei);
            let zeilen: usize = reg
                .kopf("zeilen")
                .expect("Kopf zeilen")
                .parse()
                .expect("Zahl");
            assert_eq!(zeilen, reg.zeilen.len(), "{datei}");
        }
    }

    #[test]
    fn varianten_amtlicher_namen() {
        let kantone: HashSet<&str> = ["ZH", "GR", "VD"].into_iter().collect();
        assert_eq!(
            varianten("Lausanne 25", &kantone),
            ["lausanne 25", "lausanne"]
        );
        assert_eq!(
            varianten("Kilchberg ZH", &kantone),
            ["kilchberg zh", "kilchberg"]
        );
        assert_eq!(
            varianten("Zell (ZH)", &kantone),
            ["zell (zh)", "zell zh", "zell"]
        );
        assert_eq!(
            varianten("Biel/Bienne", &kantone),
            ["biel/bienne", "biel", "bienne"]
        );
        assert_eq!(
            varianten("Laax GR 2", &kantone),
            ["laax gr 2", "laax gr", "laax"]
        );
    }

    #[test]
    fn hausnummern_nach_empfehlung() {
        for gut in [
            "21", "105", "18a", "2bis", "2ter", "21.1", "18a.1", "10-12", "10/12", "12A", "12.",
        ] {
            assert!(hausnummer(gut).is_some(), "{gut}");
        }
        for schlecht in ["0", "012", "12345", "abc", "1.2.3", "12abcd"] {
            assert!(hausnummer(schlecht).is_none(), "{schlecht}");
        }
        assert_eq!(hausnummer("12."), Some(2));
    }
}
