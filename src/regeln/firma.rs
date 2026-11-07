//! Regel `firma`: Firmen mit Rechtsformzusatz (SPEC 5.2).
//!
//! Art. 950 Abs. 1 OR verlangt die Rechtsform in der Firma. Die zulässigen Bezeichnungen
//! und Abkürzungen stehen in Anhang 2 HRegV (Art. 116a HRegV), die der kollektiven
//! Kapitalanlagen im KAG. Die Regel sucht diese Zusätze (`lexika/firma.toml`) und nimmt
//! die gross beginnenden Wörter unmittelbar davor als Firmennamen dazu. Begleiter,
//! Funktionswörter, Behördenwörter und Ortspräpositionen (`lexika/firma-grenzen.toml`)
//! grenzen den Namen ab.
//!
//! Das DSG schützt juristische Personen nicht (Art. 5 lit. a DSG). Die Stufe bestimmt
//! [`crate::stufen`]: ohne Geheimnis-Kontext kein Fund, ROT nur, wenn die Pflicht der Rolle
//! «ein Geheimnis» schützt. Kollektiv- und Kommanditgesellschaften sind keine juristischen
//! Personen; ihre unbeschränkt haftenden Gesellschafter sind natürliche Personen (Art. 552
//! Abs. 1, Art. 594 Abs. 2 OR). Sie werden wie eine Personenangabe gemeldet
//! ([`Art::Personengesellschaft`]), mit einem Zusatz, der das sagt.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Eintrag, Lexikon};
use crate::normen::{self, Norm};
use crate::text::{Text, ist_wortzeichen, links_frei};

/// Kennung der Regel.
pub const REGEL: &str = "firma";
/// Wortliste der Rechtsformzusätze.
pub const LEX_ZUSAETZE: &str = "firma.toml";
/// Wortliste der Abgrenzung.
pub const LEX_GRENZEN: &str = "firma-grenzen.toml";
/// Wortliste der Gemeinden im Kanton Aargau (Befund fehlersuche-1).
pub const LEX_GEMEINDEN_AG: &str = "firma-gemeinden-ag.toml";
/// Ortschaftenverzeichnis von swisstopo: Ortschaften wie «Wohlen AG», «Rieden AG».
const PLZ: &str = "register/plz.tsv";

/// So viele Bytes vor «AG» werden nach dem Namen einer Aargauer Gemeinde durchsucht.
const ORT_FENSTER: usize = 64;

/// Höchstens so viele Wörter vor dem Zusatz zählen zum Namen.
const NAME_HOECHSTENS: usize = 6;

/// Die Merkmale in `firma-grenzen.toml`; jedes wirkt in [`firmenanfang`].
pub const MERKMALE_GRENZEN: [&str; 7] = [
    "gattung",
    "bestimmt",
    "funktionswort",
    "ort",
    "behoerde",
    "kantonskuerzel",
    "homograph",
];

/// Die Rechtsform gehört in die Firma; die Abkürzungen legt der Bundesrat fest.
pub const OR_950: Norm = Norm {
    zitat: "Art. 950 Abs. 1 Satz 2 und Abs. 2 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_950",
    wortlaut: "In der Firma muss die Rechtsform angegeben werden. … Der Bundesrat legt fest, welche Abkürzungen der Rechtsformen zulässig sind.",
};

/// Bezeichnungen und Abkürzungen der Rechtsformen: Anhang 2 HRegV und KAG.
pub const HREGV_116A: Norm = Norm {
    zitat: "Art. 116a HRegV",
    erlass: "HRegV, SR 221.411",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2007/686/de#art_116_a",
    wortlaut: "Die Rechtsform in der Firma einer Handelsgesellschaft oder Genossenschaft (Art. 950 OR) ist mit der zutreffenden Bezeichnung oder deren Abkürzung in einer Landessprache des Bundes anzugeben. Die Bezeichnungen und Abkürzungen sind im Anhang 2 aufgeführt. Für die Rechtsformen nach dem Kollektivanlagengesetz vom 23. Juni 2006 gelten die darin festgelegten Bezeichnungen und Abkürzungen.",
};

/// Wen das DSG schützt. Derselbe Satz steht in explain firma und explain gelb.
pub const DSG_NATUERLICHE_PERSONEN: Txt = Txt::neu(
    "Das DSG schützt nur natürliche Personen. AG, GmbH, Genossenschaft, KmAG, SICAV und SICAF sind juristische Personen; bei Kollektiv- und Kommanditgesellschaften haften natürliche Personen persönlich.",
    "The DSG only protects natural persons. AG, GmbH, cooperative, KmAG, SICAV and SICAF are legal persons; in general and limited partnerships natural persons are personally liable.",
);

/// Merkmal der Kollektivgesellschaft in `firma.toml`, ausgeschrieben und abgekürzt.
pub const KOLLEKTIVGESELLSCHAFT: &str = "kollektivgesellschaft";
/// Merkmal der Kommanditgesellschaft in `firma.toml`, ausgeschrieben und abgekürzt.
pub const KOMMANDITGESELLSCHAFT: &str = "kommanditgesellschaft";

/// Art und Zusatz eines Treffers nach dem Merkmal des Rechtsformzusatzes.
fn art_und_zusatz(zusatz: &Eintrag) -> (Art, Option<(&'static str, &'static str)>) {
    match zusatz.merkmal.as_deref() {
        Some(KOLLEKTIVGESELLSCHAFT) => (
            Art::Personengesellschaft,
            Some((
                "Personengesellschaft: Gesellschafter haften persönlich, Art. 568 Abs. 1 OR",
                "partnership: the partners are personally liable, Art. 568 Abs. 1 OR",
            )),
        ),
        Some(KOMMANDITGESELLSCHAFT) => (
            Art::Personengesellschaft,
            Some((
                "Personengesellschaft: unbeschränkt haftende Gesellschafter sind natürliche Personen, Art. 594 Abs. 2 OR",
                "partnership: partners with unlimited liability are natural persons, Art. 594 Abs. 2 OR",
            )),
        ),
        _ => (Art::JuristischePerson, None),
    }
}

/// Meldet jede Firma mit Rechtsformzusatz; die Stelle reicht vom Namen bis zum Zusatz.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let zusaetze = lexikon::lexikon(LEX_ZUSAETZE);
    let s = text.als_str();
    zusaetze
        .finde(text)
        .into_iter()
        .filter_map(|t| {
            let eintrag = &zusaetze.eintraege[t.eintrag];
            let anfang = firmenanfang(s, t.start, t.ende, eintrag)?;
            let (art, zusatz) = art_und_zusatz(eintrag);
            let treffer =
                Treffer::neu(REGEL, anfang, t.ende, art).mit_beleg(zusaetze.beleg(t.eintrag));
            Some(match zusatz {
                Some((de, en)) => treffer.mit_zusatz(de, en),
                None => treffer,
            })
        })
        .collect()
}

/// Die Abgrenzungslisten, je Aufgabe ein eigener Automat.
struct Grenzen {
    /// Unbestimmte Begleiter: «eine AG» meint keine bestimmte Firma.
    gattung: Lexikon,
    /// Bestimmte Begleiter allein.
    bestimmt: Lexikon,
    /// Was vorne nicht zum Namen gehört: bestimmte Begleiter, Funktionswörter, Ortspräpositionen.
    begleiter: Lexikon,
    /// Präpositionen vor Ortsnamen.
    ort: Lexikon,
    /// Behördenwörter, auch als Wortende.
    behoerde: Lexikon,
    /// Zusätze, die zugleich ein Kantonskürzel sind.
    kantonskuerzel: Lexikon,
    /// Zusätze, die zugleich ein gewöhnliches Nomen sind.
    homograph: Lexikon,
}

fn grenzen() -> &'static Grenzen {
    static ZELLE: OnceLock<Grenzen> = OnceLock::new();
    ZELLE.get_or_init(|| {
        let lex = lexikon::lexikon(LEX_GRENZEN);
        let teil = |merkmale: &[&str]| {
            let eintraege = lex
                .eintraege
                .iter()
                .filter(|e| e.merkmal.as_deref().is_some_and(|m| merkmale.contains(&m)))
                .cloned()
                .collect();
            Lexikon::aus_eintraegen(LEX_GRENZEN, eintraege)
        };
        Grenzen {
            gattung: teil(&["gattung"]),
            bestimmt: teil(&["bestimmt"]),
            begleiter: teil(&["bestimmt", "funktionswort", "ort"]),
            ort: teil(&["ort"]),
            behoerde: teil(&["behoerde"]),
            kantonskuerzel: teil(&["kantonskuerzel"]),
            homograph: teil(&["homograph"]),
        }
    })
}

/// Deckt ein Eintrag das ganze Wort?
fn ganz(lex: &Lexikon, wort: &str) -> bool {
    lex.finde_in(wort)
        .iter()
        .any(|t| t.start == 0 && t.ende == wort.len())
}

/// Endet das Wort auf einen Eintrag (nach dessen Teilwort-Regel, z. B. «Obergericht»)?
fn endet_auf(lex: &Lexikon, wort: &str) -> bool {
    lex.finde_in(wort).iter().any(|t| t.ende == wort.len())
}

/// Ein Wort vor dem Zusatz, mit Byte-Versatz.
#[derive(Clone, Copy, Debug)]
struct Wort<'a> {
    start: usize,
    text: &'a str,
}

/// Was ein Wort vor dem Zusatz sein kann.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sorte {
    /// Beginnt gross oder mit einer Ziffer: «Muster», «3M», «Co.».
    Name,
    /// «&» oder «+» zwischen zwei Namen.
    Verbinder,
    /// Klammerzusatz wie «(Schweiz)».
    Klammer,
    /// Alles andere, vor allem klein geschriebene Wörter.
    Anderes,
}

fn sorte(wort: &str) -> Sorte {
    if wort == "&" || wort == "+" {
        Sorte::Verbinder
    } else if wort.starts_with('(') {
        Sorte::Klammer
    } else if wort
        .chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
    {
        Sorte::Name
    } else {
        Sorte::Anderes
    }
}

/// Leerraum innerhalb einer Zeile, auch geschützt.
fn ist_leer(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\u{2007}'
    )
}

/// Zeichen eines Namensworts: Wortzeichen und «-», «.», «'», «’», «&», «+».
fn ist_wortteil(c: char) -> bool {
    ist_wortzeichen(c)
        || matches!(
            c,
            '-' | '\u{2010}' | '\u{2011}' | '.' | '\'' | '\u{2019}' | '&' | '+'
        )
}

/// «A.B.»: zwei oder mehr Grossbuchstaben, je mit Punkt.
fn ist_initialen(wort: &str) -> bool {
    let z: Vec<char> = wort.chars().collect();
    z.len() >= 4
        && z.chunks(2)
            .all(|p| p.len() == 2 && p[0].is_uppercase() && p[1] == '.')
}

/// Folgt auf den Zusatz ein Bindestrich mit Wort («AG-Recht», «SA-Block»)?
fn bindestrich_wort(s: &str, ende: usize) -> bool {
    let mut z = s[ende..].chars();
    matches!(z.next(), Some('-' | '\u{2010}' | '\u{2011}')) && z.next().is_some_and(ist_wortzeichen)
}

fn leerraum_links(s: &str, mut pos: usize) -> usize {
    while let Some(c) = s[..pos].chars().next_back().filter(|&c| ist_leer(c)) {
        pos -= c.len_utf8();
    }
    pos
}

/// Das Wort, das bei `ende` endet, oder ein Klammerzusatz derselben Zeile.
fn wort_links(s: &str, ende: usize) -> Option<Wort<'_>> {
    let davor = &s[..ende];
    if let Some(innen) = davor.strip_suffix(')') {
        let auf = innen.rfind(['(', ')', '\n'])?;
        let inhalt = innen[auf..].strip_prefix('(')?;
        if !inhalt.chars().next().is_some_and(char::is_uppercase) {
            return None;
        }
        return Some(Wort {
            start: auf,
            text: &s[auf..ende],
        });
    }
    let start = davor
        .char_indices()
        .rev()
        .take_while(|&(_, c)| ist_wortteil(c))
        .last()
        .map(|(i, _)| i)?;
    Some(Wort {
        start,
        text: &s[start..ende],
    })
}

/// Die Wörter links von `bis`, vom nächsten zum fernsten, höchstens `anzahl`. Zwischen
/// zwei Wörtern steht Leerraum; Satzzeichen, Zeilenende oder Textanfang beenden die Folge.
fn woerter_links(s: &str, bis: usize, anzahl: usize) -> Vec<Wort<'_>> {
    let mut aus = Vec::new();
    let mut pos = bis;
    while aus.len() < anzahl {
        let nach_leer = leerraum_links(s, pos);
        if nach_leer == pos {
            break;
        }
        let Some(w) = wort_links(s, nach_leer) else {
            break;
        };
        pos = w.start;
        aus.push(w);
    }
    aus
}

/// Teilt die Wörter vor dem Zusatz (vom Zusatz weg geordnet) in den Namen, von links nach
/// rechts, und das erste Wort davor, das nicht mehr dazugehört.
fn name_und_davor<'a>(roh: &[Wort<'a>]) -> (Vec<Wort<'a>>, Option<Wort<'a>>) {
    let mut name = Vec::new();
    let mut davor = None;
    for (i, w) in roh.iter().enumerate() {
        let links = roh.get(i + 1).map(|l| l.text);
        let name_links = links.is_some_and(|l| sorte(l) == Sorte::Name && !l.ends_with('.'));
        let gehoert_dazu = match sorte(w.text) {
            // Ein Wort mit Punkt nur nach «&» («& Co.») oder als Initialen; sonst endet
            // davor ein Satz («Vertrag. Muster AG»).
            Sorte::Name if w.text.ends_with('.') => {
                links.is_some_and(|l| sorte(l) == Sorte::Verbinder) || ist_initialen(w.text)
            }
            Sorte::Name => true,
            Sorte::Verbinder | Sorte::Klammer => name_links,
            Sorte::Anderes => false,
        };
        if !gehoert_dazu || name.len() == NAME_HOECHSTENS {
            davor = Some(*w);
            break;
        }
        name.push(*w);
    }
    name.reverse();
    (name, davor)
}

/// Anfang der Firma, deren Rechtsformzusatz bei `start..ende` steht, oder `None`, wenn
/// davor kein Firmenname steht.
fn firmenanfang(s: &str, start: usize, ende: usize, zusatz: &Eintrag) -> Option<usize> {
    let g = grenzen();
    if bindestrich_wort(s, ende) {
        return None;
    }
    // Ohne Leerraum davor («Muster-AG», «(AG)») liefert die Suche kein Wort.
    let roh = woerter_links(s, start, NAME_HOECHSTENS + 1);
    let (mut name, mut davor) = name_und_davor(&roh);
    // Vorne gehören Begleiter nicht zum Namen; ein unbestimmter schliesst ihn aus.
    while let Some(erstes) = name.first().copied() {
        if ganz(&g.gattung, erstes.text) {
            return None;
        }
        if !ganz(&g.begleiter, erstes.text) {
            break;
        }
        name.remove(0);
        davor = Some(erstes);
    }
    let erstes = name.first()?;
    if sorte(erstes.text) != Sorte::Name
        || !name.iter().any(|w| w.text.chars().any(char::is_alphabetic))
    {
        return None;
    }
    if davor.is_some_and(|d| ganz(&g.gattung, d.text)) {
        return None;
    }
    let text_zusatz = &s[start..ende];
    // «AG» ist auch das Kantonskürzel des Aargaus: nach Behörde, Ortspräposition oder dem
    // Namen einer Aargauer Gemeinde oder Ortschaft kein Name.
    if ganz(&g.kantonskuerzel, text_zusatz)
        && (name.iter().any(|w| endet_auf(&g.behoerde, w.text))
            || davor.is_some_and(|d| ganz(&g.ort, d.text))
            || aargauer_ort_davor(s, start, &name))
    {
        return None;
    }
    let namenswoerter = name.iter().filter(|w| sorte(w.text) == Sorte::Name).count();
    // «die Schweizer Aktiengesellschaft» meint die Rechtsform, keine Firma. Ausgeschrieben
    // ist jeder Zusatz ohne `gross`, auch bei den Personengesellschaften (Test
    // `zusaetze_haben_bezeichnung_oder_abkuerzung`).
    if !zusatz.gross && namenswoerter == 1 && davor.is_some_and(|d| ganz(&g.bestimmt, d.text)) {
        return None;
    }
    // «das BRCA1 Gen»: ein Wort reicht beim Nomen-Doppelgänger nicht.
    if ganz(&g.homograph, text_zusatz) && namenswoerter < 2 {
        return None;
    }
    Some(erstes.start)
}

/// Ortschaften des Ortschaftenverzeichnisses, deren Name auf « AG» endet, ohne diesen
/// Zusatz: «Wohlen», «Rieden», «Dättwil». Das Verzeichnis selbst nennt «AG» dort als
/// Kantonsangabe.
fn ortschaften_ag() -> &'static HashSet<&'static str> {
    static ZELLE: OnceLock<HashSet<&'static str>> = OnceLock::new();
    ZELLE.get_or_init(|| {
        lexikon::register(PLZ)
            .werte("ortschaft")
            .filter_map(|o| o.strip_suffix(" AG"))
            .collect()
    })
}

/// Eine vierstellige Postleitzahl («5610»).
fn ist_plz(wort: &str) -> bool {
    wort.len() == 4 && wort.bytes().all(|b| b.is_ascii_digit())
}

/// Ist der ganze Name vor «AG» bei `start` der Name einer Aargauer Gemeinde
/// ([`LEX_GEMEINDEN_AG`]) oder einer Ortschaft «… AG» des Ortschaftenverzeichnisses? Dann
/// ist «AG» das Kantonskürzel: «von Muri AG», «originaire de Muri AG», «5610 Wohlen AG».
/// Eine Postleitzahl davor ändert nichts; «von», «aus» oder «bei» allein genügen nicht
/// («bei Muster AG» bleibt eine Firma). Der Ort muss das erste Namenswort mit umfassen:
/// Steht davor ein weiteres gross geschriebenes Namenswort, ist es eine Firma, deren Name
/// auf einen Ort endet («Garage Wohlen AG», «Elektro Baden AG»; Prüferproblem
/// c-erkennung-p1).
fn aargauer_ort_davor(s: &str, start: usize, name: &[Wort<'_>]) -> bool {
    let vor = leerraum_links(s, start);
    if vor == start {
        return false;
    }
    let Some(erstes) = name.iter().find(|w| !ist_plz(w.text)).map(|w| w.start) else {
        return false;
    };
    let mut anfang = vor.saturating_sub(ORT_FENSTER).min(erstes);
    while !s.is_char_boundary(anfang) {
        anfang += 1;
    }
    let anfang = s[anfang..erstes]
        .rfind('\n')
        .map_or(anfang, |i| anfang + i + 1);
    let stueck = &s[anfang..vor];
    let gemeinden = lexikon::lexikon(LEX_GEMEINDEN_AG);
    // Mehrteilige Gemeinden («Beinwil am See») dürfen vor dem ersten Namenswort beginnen,
    // weil der Name an einem klein geschriebenen Wort endet.
    gemeinden.finde_in(stueck).iter().any(|t| {
        t.ende == stueck.len() && anfang + t.start <= erstes && links_frei(s, anfang + t.start)
    }) || ortschaften_ag().contains(&s[erstes..vor])
}

/// Beschreibung für `rules` und `explain`.
pub fn info() -> Vec<RegelInfo> {
    vec![RegelInfo {
        id: REGEL,
        klasse: Klasse::B,
        titel: Txt::neu("Firma mit Rechtsform", "Company name with legal form"),
        erkennt: Txt::neu(
            "Firmen mit Rechtsformzusatz nach Anhang 2 HRegV oder KAG, z. B. «Muster Treuhand AG», «Exemple Sàrl», «Rossi Sagl»; Kollektiv- und Kommanditgesellschaften («Muster & Meier KlG») mit dem Zusatz «Personengesellschaft».",
            "Company names with a legal-form suffix under Annex 2 of the Commercial Register Ordinance or the CISA, e.g. «Muster Treuhand AG», «Exemple Sàrl», «Rossi Sagl»; general and limited partnerships («Muster & Meier KlG») with the note «partnership».",
        ),
        methode: Txt::neu(
            "Sucht die Bezeichnungen und Abkürzungen der Rechtsformen aus Anhang 2 HRegV und dem KAG in den drei Amtssprachen, Abkürzungen nur in amtlicher Schreibung. Davor müssen bis zu sechs gross beginnende Wörter derselben Zeile stehen, auch mit «&», «+» und Klammerzusatz; sie bilden mit dem Zusatz die Stelle. Ein unbestimmter Begleiter davor («eine AG») schliesst den Treffer aus, bestimmte Begleiter und Funktionswörter gehören nicht zum Namen. «AG» nach Behördenwort oder Ortspräposition gilt als Kantonskürzel, ebenso wenn der ganze Name davor eine Gemeinde des Kantons Aargau (BFS-Gemeindeverzeichnis) oder eine Ortschaft «… AG» des Ortschaftenverzeichnisses ist, mit oder ohne Postleitzahl davor («von Muri AG», «originaire de Muri AG», «5610 Wohlen AG»). Steht vor dem Ort ein weiteres gross geschriebenes Namenswort, bleibt es eine Firma («Garage Wohlen AG»). «Gen» braucht zwei Namenswörter.",
            "Looks up the full names and abbreviations of the legal forms in Annex 2 of the Commercial Register Ordinance and in the CISA in the three official languages; abbreviations only in their official spelling. Up to six words starting with a capital letter on the same line must precede the suffix, also with «&», «+» and a parenthesised addition; together with the suffix they form the match. An indefinite determiner before them («eine AG») excludes the match; definite determiners and function words are not part of the name. «AG» after an authority word or a place preposition counts as the canton abbreviation, as it does when the whole name before it is a municipality of the canton of Aargau (FSO register of municipalities) or a locality «… AG» in the register of localities, with or without a postcode before it («von Muri AG», «originaire de Muri AG», «5610 Wohlen AG»). If another capitalised name word precedes the place, it remains a firm («Garage Wohlen AG»). «Gen» needs two name words.",
        ),
        hinweis: None,
        grundlage: vec![
            &OR_950,
            &HREGV_116A,
            &normen::DSG_5_A,
            &normen::OR_552_1,
            &normen::OR_568_1,
            &normen::OR_594_2,
            &normen::KAG_98_2,
            &normen::STGB_321_1,
        ],
        quellen: vec![
            Quelle {
                titel: "Art. 950 OR (SR 220), Fassung vom 1. Oktober 2026",
                url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_950",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 116a und Anhang 2 HRegV (SR 221.411), Fassung vom 1. Oktober 2026, deutsch, französisch, italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2007/686/de#annex_2",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 12 Abs. 2, 38 Abs. 1, 101 und 111 Abs. 1 KAG (SR 951.31), Fassung vom 1. Oktober 2026, deutsch, französisch, italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2006/822/de#art_12",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Begleiter, Funktionswörter und Behördenwörter: BV (SR 101, Fassung vom 3. März 2024), OR Art. 50, 239, 266f, ZStV Art. 71, StPO Art. 12, StBOG Art. 8; Fundstelle je Eintrag in firma-grenzen.toml",
                url: "https://www.fedlex.admin.ch/eli/cc/1999/404/de",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Bundeskanzlei, Schreibweisungen, 2. Aufl. 2013 (korrigierte Ausgabe 2015), Rz. 347, 348 und 355: Kantonskürzel und Ortsnamen mit Kantonsangabe",
                url: "https://www.bk.admin.ch/dam/de/sd-web/YVHazXZRkqKn/schreibweisungen.pdf",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Duden online, Stichwörter «Gen», «mein», «unser»",
                url: "https://www.duden.de/rechtschreibung/Gen",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "BFS: Historisiertes Gemeindeverzeichnis der Schweiz, Snapshot vom 06.11.2026, die 196 Gemeinden des Kantons Aargau (abgeleitet: firma-gemeinden-ag.toml)",
                url: "https://www.agvchapp.bfs.admin.ch/api/communes/snapshot?date=06-11-2026",
                lizenz: Some(
                    "opendata.swiss, Freie Nutzung, Quellenangabe empfohlen (https://opendata.swiss/terms-of-use#terms_open)",
                ),
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "swisstopo: Amtliches Ortschaftenverzeichnis mit Postleitzahl und Perimeter, Ortschaften mit «AG» im Namen (register/plz.tsv)",
                url: "https://data.geo.admin.ch/ch.swisstopo-vd.ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz_2056.csv.zip",
                lizenz: Some(
                    "Nutzungsbedingungen für kostenlose Geodaten und Geodienste (OGD) von swisstopo; Quellenangabe Pflicht: Bundesamt für Landestopografie swisstopo",
                ),
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Bundesgericht, Regeln für die Anonymisierung der Urteile (Präsidentenkonferenz 17.12.2019, Verwaltungskommission 13.02.2020), Art. 1 Abs. 1 und Art. 4 Abs. 1 lit. a, b, e, g",
                url: "https://www.bger.ch/home/juridiction/jurisdiction-recht/anonymisierungsregeln.html",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
        ],
        lexika: vec![LEX_ZUSAETZE, LEX_GRENZEN, LEX_GEMEINDEN_AG],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            DSG_NATUERLICHE_PERSONEN,
            Txt::neu(
                "Eine juristische Person ergibt ohne Geheimnis-Kontext keinen Fund. ROT wird sie nur, wenn die Pflicht der Rolle «ein Geheimnis» schützt (Art. 320 und 321 StGB, Art. 47 BankG, Art. 69 FINIG); --role professional (Art. 62 DSG, «geheime Personendaten») hebt sie nicht. Die KmGK zählt hier mit, weil ihr unbeschränkt haftender Gesellschafter eine Aktiengesellschaft sein muss (Art. 98 Abs. 2 KAG).",
                "A legal person yields no finding without a secrecy context. It becomes RED only if the role's duty protects «a secret» (Art. 320 and 321 StGB, Art. 47 BankG, Art. 69 FINIG); --role professional (Art. 62 DSG, «secret personal data») does not raise it. The KmGK counts here because its partner with unlimited liability must be a company limited by shares (Art. 98 Abs. 2 KAG).",
            ),
            Txt::neu(
                "Kollektiv- und Kommanditgesellschaften (KlG, KmG, SNC, SCm, SAc) zählen wie eine Personenangabe: ohne direkten Identifikator im Prompt HINWEIS, mit Identifikator GELB, und eine Adresse direkt danach bleibt die Adresse einer Person. Schützt die Pflicht der Rolle «ein Geheimnis», werden sie ROT; mit --role professional nur zusammen mit einem Identifikator.",
                "General and limited partnerships (KlG, KmG, SNC, SCm, SAc) count like an attribute of a person: without a direct identifier in the prompt a NOTE, with an identifier YELLOW, and an address right after them stays the address of a person. If the role's duty protects «a secret», they become RED; with --role professional only together with an identifier.",
            ),
            Txt::neu(
                "Ob eine Firma ein Fabrikations- oder Geschäftsgeheimnis hat (Art. 162 StGB), entscheidet promptscan nicht.",
                "promptscan does not decide whether a company has a manufacturing or trade secret (Art. 162 StGB).",
            ),
            Txt::neu(
                "Firmen ohne Rechtsformzusatz (Einzelunternehmen, Vereine, Stiftungen) erkennt die Regel nicht, ebenso ausländische Rechtsformen (Ltd, Inc., S.p.A.), Schreibungen mit Punkten («S.A.», «S.à r.l.»), «Sarl» ohne Gravis und die rätoromanischen Abkürzungen aus Anhang 2 HRegV.",
                "Firms without a legal-form suffix (sole proprietorships, associations, foundations) are not detected, nor are foreign legal forms (Ltd, Inc., S.p.A.), spellings with dots («S.A.», «S.à r.l.»), «Sarl» without the grave accent, or the Romansh abbreviations of Annex 2.",
            ),
            Txt::neu(
                "Der Name muss vor dem Zusatz in derselben Zeile stehen und gross beginnen: «la société anonyme Exemple», «Muster-AG» und ««Muster» AG» zählen nicht. Ein klein geschriebener Namensteil beendet den Namen: bei «Muster und Partner AG» ist nur «Partner AG» markiert. Ein Wort mit Schlusspunkt gilt als Satzende, ausser nach «&» («& Co.») und bei Initialen wie «A.B.»: «A. AG» ergibt keinen Treffer.",
                "The name must precede the suffix on the same line and start with a capital letter: «la société anonyme Exemple», «Muster-AG» and ««Muster» AG» do not count. A lower-case part ends the name: in «Muster und Partner AG» only «Partner AG» is marked. A word ending in a full stop counts as the end of a sentence, except after «&» («& Co.») and in initials such as «A.B.»: «A. AG» yields no match.",
            ),
            Txt::neu(
                "Die Regel kennt keine Wortarten: ein gross geschriebenes Adjektiv am Satzanfang («Kotierte AG müssen …») oder ein Nomen vor dem Namen («Klientin Muster AG») gerät in die Stelle.",
                "The rule knows no parts of speech: a capitalised adjective at the start of a sentence («Kotierte AG müssen …») or a noun before the name («Klientin Muster AG») becomes part of the match.",
            ),
            Txt::neu(
                "«AG» als Kantonskürzel ist nur nach Behördenwort (Kanton, Gericht, Amt, Behörde, Gemeinde, Polizei, Staatsanwaltschaft), nach «in», «à», «a», in Klammern und nach dem Namen einer Aargauer Gemeinde oder Ortschaft ausgeschlossen; Gerichtskürzel wie «OGer AG», Behörden ausserhalb der Liste wie «Regierungsrat AG» und Orte anderer Kantone mit falschem Kürzel («Zürich AG») bleiben Treffer. Heisst eine Firma genau wie eine Aargauer Gemeinde («Frick AG», «Stein AG»), wird sie nicht erkannt; endet ihr Name nur auf eine Gemeinde («Garage Wohlen AG», «Elektro Baden AG»), bleibt sie ein Treffer. Bleibt ein solches Kürzel ein Treffer, gilt eine Adresse direkt danach als Adresse der Firma (explain adresse).",
                "«AG» as the canton abbreviation is excluded only after an authority word, after «in», «à», «a», in parentheses and after the name of a municipality or locality of Aargau; court abbreviations such as «OGer AG», authorities outside the list such as «Regierungsrat AG» and places of other cantons with the wrong abbreviation («Zürich AG») remain matches. A firm named exactly like a municipality of Aargau («Frick AG», «Stein AG») is not detected; if its name only ends in a municipality («Garage Wohlen AG», «Elektro Baden AG»), it remains a match. If such an abbreviation remains a match, an address right after it counts as the firm's address (explain adresse).",
            ),
            Txt::neu(
                "Öffentlich-rechtliche Unternehmen mit Rechtsformzusatz und die Kanzlei des Rechtsvertreters anonymisiert das Bundesgericht nicht (Anonymisierungsregeln Art. 4 Abs. 1 lit. a und g); die Regel unterscheidet sie nicht von privaten Firmen.",
                "The Federal Supreme Court does not anonymise public-law companies with a legal-form suffix or the law firm of a party's counsel (anonymisation rules Art. 4(1)(a) and (g)); the rule does not tell them apart from private firms.",
            ),
        ],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fund::{Rolle, Stufe, Ziel};
    use crate::lexikon::Sprache;
    use crate::stufen::Optionen;

    /// Die markierten Stellen eines Textes.
    fn stellen(roh: &str) -> Vec<String> {
        let t = Text::neu(roh);
        erkenne(&t)
            .iter()
            .map(|x| t.als_str()[x.start..x.ende].to_string())
            .collect()
    }

    fn mit_rolle(rolle: Option<Rolle>) -> Optionen {
        Optionen {
            rolle,
            ziel: Ziel::Cloud,
        }
    }

    #[test]
    fn gegenprobe_die_regel_findet_etwas() {
        assert_eq!(
            stellen("Die Muster Treuhand AG hat gekündigt."),
            ["Muster Treuhand AG"]
        );
    }

    #[test]
    fn deutsch_franzoesisch_italienisch() {
        assert_eq!(
            stellen("Exemple Sàrl a résilié le contrat."),
            ["Exemple Sàrl"]
        );
        assert_eq!(
            stellen("La società Rossi Sagl ha disdetto il contratto."),
            ["Rossi Sagl"]
        );
        assert_eq!(stellen("Rossi SA, Lugano"), ["Rossi SA"]);
        assert_eq!(
            stellen("Wir vertreten die Muster Holding GmbH."),
            ["Muster Holding GmbH"]
        );
        assert_eq!(
            stellen("Le fonds Exemple SICAV investit."),
            ["Exemple SICAV"]
        );
    }

    #[test]
    fn ausgeschriebene_bezeichnung() {
        assert_eq!(
            stellen("Vertrag mit Muster Aktiengesellschaft, Bern."),
            ["Muster Aktiengesellschaft"]
        );
        assert_eq!(
            stellen("Exemple société anonyme a signé."),
            ["Exemple société anonyme"]
        );
        // Die längere KAG-Bezeichnung gewinnt gegen «Kommanditgesellschaft».
        assert_eq!(
            stellen("Muster Kommanditgesellschaft für kollektive Kapitalanlagen"),
            ["Muster Kommanditgesellschaft für kollektive Kapitalanlagen"]
        );
    }

    #[test]
    fn verbinder_klammer_und_geschuetztes_leerzeichen() {
        assert_eq!(stellen("Offerte der Muster & Co. AG"), ["Muster & Co. AG"]);
        assert_eq!(stellen("Muster + Partner GmbH"), ["Muster + Partner GmbH"]);
        assert_eq!(
            stellen("Konto bei Muster Bank (Schweiz) AG"),
            ["Muster Bank (Schweiz) AG"]
        );
        assert_eq!(stellen("Muster\u{00A0}AG"), ["Muster\u{00A0}AG"]);
    }

    #[test]
    fn beleg_nennt_anhang_2_hregv() {
        let t = Text::neu("Die Muster AG zahlt.");
        let tr = erkenne(&t);
        let beleg = tr[0].beleg.as_ref().expect("Beleg");
        assert_eq!(beleg.quelle, "SR 221.411 Anhang 2");
        assert_eq!(beleg.datei, LEX_ZUSAETZE);
        assert_eq!(tr[0].art, Art::JuristischePerson);
    }

    #[test]
    fn jeder_zusatz_der_wortliste_wird_erkannt() {
        let lex = lexikon::lexikon(LEX_ZUSAETZE);
        for e in &lex.eintraege {
            let roh = format!("Brief an Muster Beispiel {}.", e.begriff);
            assert_eq!(
                stellen(&roh),
                [format!("Muster Beispiel {}", e.begriff)],
                "{roh}"
            );
        }
    }

    #[test]
    fn unbestimmter_begleiter_schliesst_aus() {
        assert!(stellen("Eine AG kann Aktien ausgeben.").is_empty());
        assert!(stellen("Ob eine AG haftet, ist offen.").is_empty());
        assert!(stellen("Jede Schweizer AG muss Buch führen.").is_empty());
        assert!(stellen("Er gründet eine Schweizer AG.").is_empty());
        assert!(stellen("une SA peut émettre des actions").is_empty());
        assert!(stellen("ogni Sagl deve tenere la contabilità").is_empty());
    }

    #[test]
    fn bestimmter_begleiter_gehoert_nicht_zum_namen() {
        assert!(stellen("Die AG haftet mit ihrem Vermögen.").is_empty());
        assert!(stellen("La SA doit tenir une comptabilité.").is_empty());
        assert!(stellen("Les SA sont soumises à la révision.").is_empty());
        assert_eq!(stellen("La Exemple SA a payé."), ["Exemple SA"]);
        // Ausgeschrieben mit einem Wort und Begleiter: die Rechtsform, keine Firma.
        assert!(stellen("die Schweizer Aktiengesellschaft").is_empty());
        assert_eq!(
            stellen("die Muster Holding Aktiengesellschaft"),
            ["Muster Holding Aktiengesellschaft"]
        );
    }

    #[test]
    fn funktionswort_am_satzanfang() {
        assert!(stellen("Bei AG und GmbH gilt dasselbe.").is_empty());
        assert!(stellen("Pour SA et Sàrl, la règle est la même.").is_empty());
        assert_eq!(stellen("Für Muster AG gilt das nicht."), ["Muster AG"]);
    }

    #[test]
    fn kantonskuerzel_nach_behoerde_ort_und_in_klammern() {
        assert!(stellen("Der Kanton AG hat entschieden.").is_empty());
        assert!(stellen("Das Obergericht AG wies die Klage ab.").is_empty());
        assert!(stellen("Die Kantonspolizei AG ermittelt.").is_empty());
        assert!(stellen("Die Staatsanwaltschaft AG erhob Anklage.").is_empty());
        assert!(stellen("Er wohnt in Wohlen AG.").is_empty());
        assert!(stellen("In Wohlen AG gibt es einen Markt.").is_empty());
        assert!(stellen("Il habite à Wohlen AG.").is_empty());
        assert!(stellen("Er wohnt in Baden (AG).").is_empty());
        // Die Ortspräposition schliesst nur das Kantonskürzel aus.
        assert_eq!(stellen("Er arbeitet in Muster GmbH."), ["Muster GmbH"]);
    }

    /// Befund fehlersuche-1, Entscheid C5: «AG» nach einer Aargauer Gemeinde oder
    /// Ortschaft ist das Kantonskürzel, mit oder ohne Postleitzahl, in jeder Sprache.
    #[test]
    fn kantonskuerzel_nach_aargauer_gemeinde() {
        for text in [
            "Hans Muster, von Wohlen AG, Seestrasse 12, 8002 Zürich",
            "Hans Muster, von Muri AG, Seestrasse 12, 8002 Zürich",
            "Hans Muster, von Buchs AG, Seestrasse 12, 8002 Zürich",
            "Hans Muster, von Aarau AG, Seestrasse 12, 8002 Zürich",
            "Erika Beispiel, Baden AG, Bahnhofstrasse 1, 5400 Baden",
            "Die Patientin stammt aus Wohlen AG, Bahnhofstrasse 1, 5610 Wohlen.",
            "Erika Beispiel, Bahnhofstrasse 1, 5610 Wohlen AG\nSeestrasse 12, 8002 Zürich",
            "Bahnhofstrasse 1, 5630 Muri AG\nSeestrasse 12, 8002 Zürich",
            "Jean Exemple, originaire de Muri AG, Rue du Lac 12, 1003 Lausanne",
            "Mario Esempio, attinente di Muri AG, Via Lago 12, 6900 Lugano",
            "Ihr Heimatort ist Beinwil am See AG.",
            "Er wohnt in 5405 Dättwil AG.",
            "Rieden AG liegt bei Baden.",
            "Er ist von Möriken-Wildegg AG.",
        ] {
            assert!(stellen(text).is_empty(), "{text}");
        }
        // Nicht pauschal nach «von», «aus», «bei»: Firmen bleiben Firmen.
        assert_eq!(stellen("Sie ist bei Muster AG angestellt."), ["Muster AG"]);
        assert_eq!(
            stellen("Offerte von Muster AG, Seestrasse 12, 8002 Zürich."),
            ["Muster AG"]
        );
        assert_eq!(stellen("Hans Muster, von Wohler AG"), ["Wohler AG"]);
        // Nur der Name direkt vor «AG» zählt.
        assert_eq!(stellen("Die Muri Treuhand AG zahlt."), ["Muri Treuhand AG"]);
    }

    /// Prüferproblem c-erkennung-p1: Endet der Name einer Firma auf eine Aargauer Gemeinde,
    /// bleibt «AG» die Rechtsform; Kantonskürzel ist es nur, wenn der Ort den ganzen Namen
    /// vor «AG» bildet (eine Postleitzahl davor ausgenommen).
    #[test]
    fn firma_endet_auf_aargauer_gemeinde() {
        for (text, firma) in [
            (
                "Meine Klientin ist die Garage Wohlen AG.",
                "Garage Wohlen AG",
            ),
            (
                "Meine Klientin ist die Elektro Baden AG.",
                "Elektro Baden AG",
            ),
            ("Die Bäckerei Muri AG hat gekündigt.", "Bäckerei Muri AG"),
            (
                "Sie arbeitet bei der Druckerei Brugg AG.",
                "Druckerei Brugg AG",
            ),
            ("Die Treuhand Zofingen AG zahlt.", "Treuhand Zofingen AG"),
            ("Die Möbel Spreitenbach AG zahlt.", "Möbel Spreitenbach AG"),
            (
                "Ma cliente, la Garage Wohlen AG, a été condamnée.",
                "Garage Wohlen AG",
            ),
            (
                "Offerte der Garage Wohlen AG, Bahnhofstrasse 1, 5610 Wohlen.",
                "Garage Wohlen AG",
            ),
        ] {
            assert_eq!(stellen(text), [firma], "{text}");
        }
        // Mit Anwaltsrolle ROT firma, wie in der Ausgangslage.
        for text in [
            "Meine Klientin ist die Elektro Baden AG.",
            "Meine Klientin ist die Garage Wohlen AG.",
            "Meine Klientin ist die Möbel Spreitenbach AG.",
        ] {
            let b = crate::pruefe(text, &mit_rolle(Some(Rolle::Lawyer)));
            assert!(
                b.funde
                    .iter()
                    .any(|f| f.regel == REGEL && f.stufe == Stufe::Rot),
                "{text}: {:?}",
                b.funde
            );
        }
        // Der Ort allein bleibt das Kantonskürzel.
        for text in [
            "Hans Muster, von Muri AG, Seestrasse 12, 8002 Zürich",
            "Erika Beispiel, Bahnhofstrasse 1, 5610 Wohlen AG\nSeestrasse 12, 8002 Zürich",
            "Ihr Heimatort ist Beinwil am See AG.",
            "Er ist von Beinwil (Freiamt) AG.",
        ] {
            assert!(stellen(text).is_empty(), "{text}");
        }
    }

    /// Befund fehlersuche-1: Die Adresse nach dem Heimatort bleibt die Adresse einer
    /// Person, ohne Rolle GELB, mit Rolle ROT ohne den Zusatz «Adresse der Firma».
    #[test]
    fn adresse_nach_heimatort_bleibt() {
        let funde = |roh: &str, rolle: Option<Rolle>| -> Vec<(String, String, Stufe, bool)> {
            crate::pruefe(roh, &mit_rolle(rolle))
                .funde
                .into_iter()
                .map(|f| {
                    let firma = f.zusatz.as_ref().is_some_and(|z| z.de.contains("Firma"));
                    (f.regel.to_string(), f.stelle, f.stufe, firma)
                })
                .collect()
        };
        let adresse = |roh: &str, rolle: Option<Rolle>| -> Vec<(String, Stufe, bool)> {
            funde(roh, rolle)
                .into_iter()
                .filter(|(r, ..)| r == "adresse")
                .map(|(_, s, st, firma)| (s, st, firma))
                .collect()
        };
        let seestrasse = "Seestrasse 12, 8002 Zürich".to_string();
        assert_eq!(
            adresse(
                "Hans Muster, von Wohlen AG, Seestrasse 12, 8002 Zürich",
                None
            ),
            [(seestrasse.clone(), Stufe::Gelb, false)]
        );
        let f = funde(
            "Die Patientin, von Wohlen AG, Seestrasse 12, 8002 Zürich, ist schwanger.",
            None,
        );
        assert!(
            f.iter()
                .any(|(r, _, st, _)| r == "adresse" && *st == Stufe::Gelb)
        );
        assert!(
            f.iter()
                .any(|(r, _, st, _)| r == "art5c-2" && *st == Stufe::Orange)
        );
        assert_eq!(
            adresse(
                "Erika Beispiel, Bahnhofstrasse 1, 5610 Wohlen AG\nSeestrasse 12, 8002 Zürich",
                None
            ),
            [
                (
                    "Bahnhofstrasse 1, 5610 Wohlen AG".to_string(),
                    Stufe::Gelb,
                    false
                ),
                (seestrasse.clone(), Stufe::Gelb, false)
            ]
        );
        let f = funde(
            "Der Beschuldigte, von Wohlen AG, Seestrasse 12, 8002 Zürich, wurde verurteilt.",
            Some(Rolle::CriminalAuthority),
        );
        assert!(!f.iter().any(|(r, ..)| r == "firma"));
        assert!(f.iter().any(|(r, s, st, firma)| r == "adresse"
            && *s == seestrasse
            && *st == Stufe::Rot
            && !firma));
        for text in [
            "Jean Exemple, originaire de Muri AG, Rue du Lac 12, 1003 Lausanne",
            "Mario Esempio, attinente di Muri AG, Via Lago 12, 6900 Lugano",
            "Erika Beispiel, Baden AG, Bahnhofstrasse 1, 5400 Baden",
        ] {
            assert_eq!(adresse(text, None).len(), 1, "{text}");
        }
        // Gegenprobe: Eine echte Firma behält ihre Adresse, mit Rolle ROT.
        let f = funde("Sie ist bei Muster AG angestellt.", Some(Rolle::Lawyer));
        assert!(
            f.iter()
                .any(|(r, _, st, _)| r == "firma" && *st == Stufe::Rot)
        );
        let f = funde(
            "Offerte von Muster AG, Seestrasse 12, 8002 Zürich, für die Klientin.",
            Some(Rolle::Lawyer),
        );
        assert!(
            f.iter()
                .any(|(r, _, st, _)| r == "firma" && *st == Stufe::Rot)
        );
        assert!(f.iter().any(|(r, _, _, firma)| r == "adresse" && *firma));
    }

    #[test]
    fn gen_braucht_zwei_namenswoerter() {
        assert!(stellen("Das BRCA1 Gen ist verändert.").is_empty());
        assert_eq!(
            stellen("Die Wohnbau Muster Gen baut."),
            ["Wohnbau Muster Gen"]
        );
    }

    #[test]
    fn keine_firma() {
        assert!(stellen("Gemäss BGE 140 III 86 ist das so.").is_empty());
        assert!(stellen("Das AG-Recht gilt seit 1992.").is_empty());
        assert!(stellen("Die Muster-AG zahlt.").is_empty());
        assert!(stellen("Art. 620 AG regelt das nicht.").is_empty());
        assert!(stellen("Muster\nAG").is_empty());
        assert!(stellen("die SA-Truppe").is_empty());
        assert!(stellen("Muster ag").is_empty());
    }

    #[test]
    fn satzende_trennt() {
        assert_eq!(stellen("Der Vertrag. Muster AG zahlt."), ["Muster AG"]);
        // Ein einzelner Buchstabe mit Punkt beendet den Namen, Initialen gehören dazu.
        assert!(stellen("Die A. AG klagt.").is_empty());
        assert_eq!(stellen("Die Muster A.B. AG klagt."), ["Muster A.B. AG"]);
    }

    /// Die Grenzen in `info()` beschreiben, was die Regel tut.
    #[test]
    fn grenzen_wie_beschrieben() {
        // Name nach dem Zusatz oder in Anführungszeichen: kein Treffer.
        assert!(stellen("la société anonyme Exemple").is_empty());
        assert!(stellen("die «Muster» AG").is_empty());
        // Ein klein geschriebener Namensteil beendet den Namen.
        assert_eq!(stellen("Die Muster und Partner AG"), ["Partner AG"]);
        // Keine Wortarten.
        assert_eq!(stellen("Kotierte AG müssen berichten."), ["Kotierte AG"]);
        assert_eq!(stellen("die Klientin Muster AG"), ["Klientin Muster AG"]);
        // Kantonskürzel nach einer Behörde ausserhalb der Liste oder nach einem Ort eines
        // anderen Kantons; eine Firma, die wie eine Aargauer Gemeinde heisst.
        assert!(stellen("Baden AG liegt an der Limmat.").is_empty());
        assert_eq!(stellen("Zürich AG liegt an der Limmat."), ["Zürich AG"]);
        assert!(stellen("Die Frick AG baut.").is_empty());
        assert_eq!(stellen("Das OGer AG entschied."), ["OGer AG"]);
        assert_eq!(
            stellen("Der Regierungsrat AG beschliesst."),
            ["Regierungsrat AG"]
        );
        // Schreibungen ausserhalb von Anhang 2 HRegV.
        assert!(stellen("Exemple S.A. à Lausanne").is_empty());
        assert!(stellen("Exemple Sarl").is_empty());
    }

    #[test]
    fn stufe_nur_im_geheimnis_kontext() {
        let roh = "Die Muster Treuhand AG schuldet Fr. 10 000.";
        let ohne = crate::pruefe(roh, &mit_rolle(None));
        assert!(ohne.funde.iter().all(|f| f.regel != REGEL));
        let mit = crate::pruefe(roh, &mit_rolle(Some(Rolle::Lawyer)));
        let f = mit
            .funde
            .iter()
            .find(|f| f.regel == REGEL)
            .expect("Fund im Geheimnis-Kontext");
        assert_eq!(f.stufe, Stufe::Rot);
        assert_eq!(f.stelle, "Muster Treuhand AG");
    }

    #[test]
    fn jedes_merkmal_der_grenzen_ist_bekannt() {
        let lex = lexikon::lexikon(LEX_GRENZEN);
        for e in &lex.eintraege {
            let m = e.merkmal.as_deref().unwrap_or("");
            assert!(
                MERKMALE_GRENZEN.contains(&m),
                "{}: merkmal «{m}»",
                e.begriff
            );
        }
        for m in MERKMALE_GRENZEN {
            assert!(
                lex.eintraege
                    .iter()
                    .any(|e| e.merkmal.as_deref() == Some(m)),
                "kein Eintrag mit merkmal «{m}»"
            );
        }
    }

    /// Ausgeschrieben ohne `gross`, abgekürzt mit `gross`: Darauf stützt sich die Prüfung des
    /// Firmenanfangs («die Schweizer Aktiengesellschaft»). Bei den Personengesellschaften
    /// zeigt die Schreibung die Abkürzung: ein Grossbuchstabe nach dem ersten Zeichen
    /// («KlG», «KmG», «SNC», «SCm», «SAc»), ausgeschrieben keiner («Kollektivgesellschaft»,
    /// «Société en commandite», «Società in nome collettivo»).
    fn gross_passt(e: &Eintrag) -> bool {
        let m = e.merkmal.as_deref();
        let personengesellschaft =
            m == Some(KOLLEKTIVGESELLSCHAFT) || m == Some(KOMMANDITGESELLSCHAFT);
        let abgekuerzt = e.begriff.chars().skip(1).any(char::is_uppercase);
        (m == Some("abkuerzung") && e.gross)
            || (m == Some("bezeichnung") && !e.gross)
            || (personengesellschaft && e.gross == abgekuerzt)
    }

    /// Befund w1-recht-p6: auch die Personengesellschaften prüfen `gross`.
    #[test]
    fn zusaetze_haben_bezeichnung_oder_abkuerzung() {
        let eintraege = &lexikon::lexikon(LEX_ZUSAETZE).eintraege;
        for e in eintraege {
            assert!(gross_passt(e), "{}: gross = {}", e.begriff, e.gross);
        }
        // Gegenprobe: Ein falsches `gross` bei KlG, SNC, KmG, SCm, SAc oder ihrer
        // ausgeschriebenen Form fiele auf.
        for merkmal in [KOLLEKTIVGESELLSCHAFT, KOMMANDITGESELLSCHAFT] {
            for sprache in [Sprache::De, Sprache::Fr, Sprache::It] {
                let je: Vec<&Eintrag> = eintraege
                    .iter()
                    .filter(|e| e.merkmal.as_deref() == Some(merkmal) && e.sprache == sprache)
                    .collect();
                assert!(
                    je.iter().any(|e| e.gross),
                    "{merkmal} {sprache:?}: abgekürzt"
                );
                assert!(
                    je.iter().any(|e| !e.gross),
                    "{merkmal} {sprache:?}: ausgeschrieben"
                );
                for e in je {
                    let mut falsch = e.clone();
                    falsch.gross = !falsch.gross;
                    assert!(!gross_passt(&falsch), "{}", e.begriff);
                }
            }
        }
    }

    /// Art und Zusatz je Zusatz der Wortliste, ausgeschrieben und abgekürzt.
    fn art_von(roh: &str) -> Vec<(String, Art, Option<String>)> {
        let t = Text::neu(roh);
        erkenne(&t)
            .into_iter()
            .map(|x| {
                (
                    t.als_str()[x.start..x.ende].to_string(),
                    x.art,
                    x.zusatz.map(|z| z.de),
                )
            })
            .collect()
    }

    /// E4 (recht-stufen-3): Kollektiv- und Kommanditgesellschaften sind Personengesellschaften
    /// mit Zusatz, in allen drei Sprachen und ausgeschrieben; die KmGK bleibt bei den
    /// juristischen Personen (Art. 98 Abs. 2 KAG).
    #[test]
    fn personengesellschaften_mit_zusatz() {
        const KLG: &str =
            "Personengesellschaft: Gesellschafter haften persönlich, Art. 568 Abs. 1 OR";
        const KMG: &str = "Personengesellschaft: unbeschränkt haftende Gesellschafter sind natürliche Personen, Art. 594 Abs. 2 OR";
        for (zusatz, erwartet) in [
            ("KlG", KLG),
            ("Kollektivgesellschaft", KLG),
            ("SNC", KLG),
            ("Société en nom collectif", KLG),
            ("Società in nome collettivo", KLG),
            ("KmG", KMG),
            ("Kommanditgesellschaft", KMG),
            ("SCm", KMG),
            ("Société en commandite", KMG),
            ("SAc", KMG),
            ("Società in accomandita", KMG),
        ] {
            let roh = format!("Brief an Muster Beispiel {zusatz}.");
            assert_eq!(
                art_von(&roh),
                [(
                    format!("Muster Beispiel {zusatz}"),
                    Art::Personengesellschaft,
                    Some(erwartet.to_string())
                )],
                "{roh}"
            );
        }
        for zusatz in [
            "AG",
            "GmbH",
            "Genossenschaft",
            "KmAG",
            "SICAV",
            "SICAF",
            "KmGK",
            "Kommanditgesellschaft für kollektive Kapitalanlagen",
            "SCmPC",
            "SAcCol",
            "Société en commandite par actions",
            "Società in accomandita per azioni",
        ] {
            let roh = format!("Brief an Muster Beispiel {zusatz}.");
            assert_eq!(
                art_von(&roh),
                [(
                    format!("Muster Beispiel {zusatz}"),
                    Art::JuristischePerson,
                    None
                )],
                "{roh}"
            );
        }
    }

    /// Die Stufen der Kollektivgesellschaft, wie die Grenzen in `info()` sie nennen.
    #[test]
    fn personengesellschaft_wie_eine_personenangabe() {
        let stufen = |roh: &str, rolle: Option<Rolle>| -> Vec<(String, Stufe)> {
            crate::pruefe(roh, &mit_rolle(rolle))
                .funde
                .into_iter()
                .map(|f| (f.stelle, f.stufe))
                .collect()
        };
        let allein = "Die Muster & Meier KlG ist überschuldet.";
        let klg = || "Muster & Meier KlG".to_string();
        assert_eq!(stufen(allein, None), [(klg(), Stufe::Hinweis)]);
        assert_eq!(
            stufen(allein, Some(Rolle::Professional)),
            [(klg(), Stufe::Hinweis)]
        );
        assert_eq!(stufen(allein, Some(Rolle::Lawyer)), [(klg(), Stufe::Rot)]);
        let mit_adresse = "Die Muster & Meier KlG, Seestrasse 12, 8002 Zürich, ist überschuldet.";
        let adresse = || "Seestrasse 12, 8002 Zürich".to_string();
        assert_eq!(
            stufen(mit_adresse, None),
            [(klg(), Stufe::Gelb), (adresse(), Stufe::Gelb)]
        );
        assert_eq!(
            stufen(mit_adresse, Some(Rolle::Professional)),
            [(klg(), Stufe::Rot), (adresse(), Stufe::Rot)]
        );
        // Gegenprobe AG: die Adresse ist die der Firma, ohne Rolle kein Fund.
        assert!(stufen("Die Muster AG, Seestrasse 12, 8002 Zürich, zahlt.", None).is_empty());
    }

    /// E1 (recht-stufen-1): Nur eine Pflicht, die «ein Geheimnis» schützt, hebt die Firma;
    /// Art. 62 DSG (`professional`) schützt nur geheime Personendaten.
    #[test]
    fn professional_hebt_die_firma_nicht() {
        let roh = "Die Muster Treuhand AG schuldet Fr. 10 000.";
        let p = crate::pruefe(roh, &mit_rolle(Some(Rolle::Professional)));
        assert!(p.funde.iter().all(|f| f.regel != REGEL), "{:?}", p.funde);
        for rolle in [
            Rolle::Clergy,
            Rolle::Lawyer,
            Rolle::Notary,
            Rolle::PatentAttorney,
            Rolle::Auditor,
            Rolle::Doctor,
            Rolle::Psychologist,
            Rolle::Bank,
            Rolle::FinancialInstitution,
            Rolle::Authority,
            Rolle::Court,
            Rolle::CriminalAuthority,
            Rolle::Student,
        ] {
            let p = crate::pruefe(roh, &mit_rolle(Some(rolle)));
            assert!(
                p.funde
                    .iter()
                    .any(|f| f.regel == REGEL && f.stufe == Stufe::Rot),
                "{rolle:?}"
            );
        }
        // Die KmGK zählt wie die juristische Person.
        let kmgk = "Die Muster KmGK investiert.";
        assert!(crate::pruefe(kmgk, &mit_rolle(None)).funde.is_empty());
        assert!(
            crate::pruefe(kmgk, &mit_rolle(Some(Rolle::Professional)))
                .funde
                .is_empty()
        );
        assert_eq!(
            crate::pruefe(kmgk, &mit_rolle(Some(Rolle::Lawyer))).funde[0].stufe,
            Stufe::Rot
        );
    }

    #[test]
    fn info_vollstaendig() {
        let i = &info()[0];
        assert_eq!(i.id, REGEL);
        assert_eq!(i.klasse, Klasse::B);
        assert!(i.hinweis.is_none());
        assert!(!i.grundlage.is_empty() && !i.quellen.is_empty() && !i.grenzen.is_empty());
        assert_eq!(i.lexika, [LEX_ZUSAETZE, LEX_GRENZEN, LEX_GEMEINDEN_AG]);
        for n in [&OR_950, &HREGV_116A] {
            assert!(n.url.starts_with("https://www.fedlex.admin.ch/eli/"));
            assert_eq!(n.fassung.len(), 10);
        }
    }
}
