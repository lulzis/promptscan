//! Klasse A (SPEC 5.1): Kennungen, die sich formal prüfen lassen.
//!
//! * `ahv`: AHV-Nummer mit Kontrollziffer (Art. 133 AHVV, Anhang 7 WL VA/IK).
//! * `iban`: IBAN aus der Schweiz und Liechtenstein mit Prüfziffern nach ISO 13616-1:2020.
//! * `tel`: Schweizer Telefonnummer aus den Bereichen, die der Nummerierungsplan E.164
//!   Festnetz- und Mobilfunkdiensten zuweist (TAV E.164, SR 784.101.113/2.2).
//! * `mail`: E-Mail-Adresse, eine dokumentierte Teilmenge von addr-spec nach RFC 5322.
//!
//! Die Erkenner lesen den normalisierten Text Zeichen für Zeichen. Reguläre Ausdrücke
//! braucht es nicht: Jede Schreibweise hat feste Gruppen, und die Ränder (keine weitere
//! Ziffer davor oder danach) lassen sich so direkt prüfen. Alle Treffer haben die Art
//! [`Art::Identifikator`]; die Stufe setzt [`crate::stufen`].
//!
//! Unsichtbare Formatzeichen ([`crate::text::ist_unsichtbar`], etwa U+00AD oder U+200B)
//! lesen die Erkenner nicht: Sie prüfen den Text ohne diese Zeichen ([`OhneUnsichtbare`]),
//! die Stellen gelten danach wieder im ganzen Text.
//!
//! Alle Quellen wurden am 06.11.2026 gelesen; die Fundstellen stehen bei den Konstanten
//! und in [`info`].

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::normen::{self, Norm};
use crate::text::{OhneUnsichtbare, Text, ist_wortzeichen};

/// Alle Treffer der Klasse A.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let sichtbar = OhneUnsichtbare::neu(text.als_str());
    let s = sichtbar.text.as_ref();
    let mut t = ahv(s);
    t.extend(iban(s));
    t.extend(tel(s));
    t.extend(mail(s));
    for treffer in &mut t {
        treffer.start = sichtbar.anfang(treffer.start);
        treffer.ende = sichtbar.ende(treffer.ende);
    }
    t
}

// --- Gemeinsame Hilfen --------------------------------------------------------------

/// Leerzeichen zwischen Zifferngruppen: das gewöhnliche und dieselben Varianten, die
/// `lexikon::falten` als Leerzeichen liest (geschützt, schmal geschützt, schmal,
/// Ziffernabstand). Textverarbeitungen setzen sie in Nummern oft von selbst.
const LEERZEICHEN: [char; 5] = [' ', '\u{00A0}', '\u{202F}', '\u{2009}', '\u{2007}'];

fn ist_leerzeichen(c: char) -> bool {
    LEERZEICHEN.contains(&c)
}

/// Das Zeichen vor dem Byte-Versatz `i`.
fn davor(s: &str, i: usize) -> Option<char> {
    s[..i].chars().next_back()
}

/// Das Zeichen ab dem Byte-Versatz `i`.
fn danach(s: &str, i: usize) -> Option<char> {
    s[i..].chars().next()
}

/// Das zweite Zeichen vor `i`.
fn zwei_davor(s: &str, i: usize) -> Option<char> {
    s[..i].chars().nth_back(1)
}

/// Das zweite Zeichen ab `i`.
fn zwei_danach(s: &str, i: usize) -> Option<char> {
    s[i..].chars().nth(1)
}

fn ist_ziffer(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_digit())
}

/// Trennt das Zeichen eine Kennung vom Umfeld? Kein Buchstabe, keine Ziffer, kein «@».
fn grenzt_ab(c: Option<char>) -> bool {
    c.is_none_or(|c| !ist_wortzeichen(c) && c != '@')
}

/// Geht vor `start` eine Ziffernfolge weiter: ein Trenner und davor eine Ziffer?
fn kette_davor(s: &str, start: usize, trenner: fn(char) -> bool) -> bool {
    davor(s, start).is_some_and(trenner) && ist_ziffer(zwei_davor(s, start))
}

/// Geht nach `ende` eine Ziffernfolge weiter: ein Trenner und danach eine Ziffer?
fn kette_danach(s: &str, ende: usize, trenner: fn(char) -> bool) -> bool {
    danach(s, ende).is_some_and(trenner) && ist_ziffer(zwei_danach(s, ende))
}

fn ist_punkt(c: char) -> bool {
    c == '.'
}

/// Zeilenumbrüche: `\n` (auch aus CRLF und CR, siehe [`Text::neu`]), vertikaler Tabulator,
/// Seitenvorschub, U+0085, U+2028 und U+2029.
fn ist_zeilenumbruch(c: char) -> bool {
    matches!(
        c,
        '\n' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// Leerraum ab `pos` (Zeichen mit `char::is_whitespace`), höchstens vier Zeichen gelesen:
/// das Ende, die Zahl der Zeichen und wie viele davon Zeilenumbrüche sind.
fn leerraum(s: &str, pos: usize) -> (usize, usize, usize) {
    let mut ende = pos;
    let mut zeichen = 0;
    let mut umbrueche = 0;
    for c in s[pos..].chars().take(4) {
        if !c.is_whitespace() {
            break;
        }
        ende += c.len_utf8();
        zeichen += 1;
        if ist_zeilenumbruch(c) {
            umbrueche += 1;
        }
    }
    (ende, zeichen, umbrueche)
}

// --- ahv ------------------------------------------------------------------------------

/// Die Gruppen der Schreibweisen mit Trennern: 3.4.4.2 mit Punkten zeigen Rz 1202 und
/// Anhang 7 WL VA/IK («756.3047.5009.62»). Ohne Trenner steht die Nummer im Standard
/// eCH-0044 (vnType, 7560000000001 bis 7569999999999).
const AHV_GRUPPEN: [usize; 4] = [3, 4, 4, 2];

/// Wie die Gruppen einer AHV-Nummer getrennt sind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AhvTrenner {
    /// Punkte wie in Rz 1202 WL VA/IK; nach einem Punkt darf die Nummer einmal auf die
    /// nächste Zeile umbrechen.
    Punkt,
    /// Je ein Leerzeichen aus [`LEERZEICHEN`]. Amtlich nicht belegt, in Texten aber üblich.
    Leerzeichen,
}

fn ahv(s: &str) -> Vec<Treffer> {
    let mut treffer = Vec::new();
    for (start, _) in s.match_indices("756") {
        let Some((ende, ziffern, trenner)) = ahv_lesen(s, start) else {
            continue;
        };
        let mit_leerzeichen = trenner == Some(AhvTrenner::Leerzeichen);
        let links = grenzt_ab(davor(s, start))
            && !kette_davor(s, start, ist_punkt)
            && !(mit_leerzeichen && kette_davor(s, start, ist_leerzeichen));
        let rechts = grenzt_ab(danach(s, ende))
            && !kette_danach(s, ende, ist_punkt)
            && !(mit_leerzeichen && kette_danach(s, ende, ist_leerzeichen));
        if links && rechts && ahv_kontrollziffer(&ziffern[..12]) == ziffern[12] {
            treffer.push(
                Treffer::neu("ahv", start, ende, Art::Identifikator)
                    .mit_zusatz("Prüfziffer gültig", "check digit valid"),
            );
        }
    }
    treffer
}

/// Liest ab `start` eine AHV-Nummer ohne Trenner oder mit Trennern nach [`AHV_GRUPPEN`];
/// gibt das Ende, die 13 Ziffern und die Trenner.
fn ahv_lesen(s: &str, start: usize) -> Option<(usize, [u8; 13], Option<AhvTrenner>)> {
    let ohne_trenner = s
        .as_bytes()
        .get(start..start + 13)
        .filter(|teil| teil.iter().all(u8::is_ascii_digit));
    if let Some(teil) = ohne_trenner {
        let mut ziffern = [0u8; 13];
        for (z, &b) in ziffern.iter_mut().zip(teil) {
            *z = b - b'0';
        }
        return Some((start + 13, ziffern, None));
    }
    [AhvTrenner::Punkt, AhvTrenner::Leerzeichen]
        .into_iter()
        .find_map(|trenner| {
            ahv_gruppen(s, start, trenner).map(|(ende, ziffern)| (ende, ziffern, Some(trenner)))
        })
}

/// Liest die Gruppen 3-4-4-2 ab `start` mit den Trennern `trenner`.
fn ahv_gruppen(s: &str, start: usize, trenner: AhvTrenner) -> Option<(usize, [u8; 13])> {
    let b = s.as_bytes();
    let mut ziffern = [0u8; 13];
    let mut n = 0;
    let mut pos = start;
    let mut umbruch_frei = true;
    for (k, &laenge) in AHV_GRUPPEN.iter().enumerate() {
        if k > 0 {
            pos = match trenner {
                AhvTrenner::Punkt => {
                    if b.get(pos) != Some(&b'.') {
                        return None;
                    }
                    // Nach dem Punkt einmal ein Zeilenumbruch, mit Leerraum höchstens drei
                    // Zeichen («756.1234.⏎5678.97»).
                    match leerraum(s, pos + 1) {
                        (_, 0, _) => pos + 1,
                        (ende, 1..=3, 1) if umbruch_frei => {
                            umbruch_frei = false;
                            ende
                        }
                        _ => return None,
                    }
                }
                AhvTrenner::Leerzeichen => {
                    let c = danach(s, pos).filter(|&c| ist_leerzeichen(c))?;
                    pos + c.len_utf8()
                }
            };
        }
        let teil = b.get(pos..pos + laenge)?;
        if !teil.iter().all(u8::is_ascii_digit) {
            return None;
        }
        for &z in teil {
            ziffern[n] = z - b'0';
            n += 1;
        }
        pos += laenge;
    }
    Some((pos, ziffern))
}

/// Kontrollziffer nach Anhang 7 WL VA/IK: Die Ziffern werden von rechts nach links,
/// beginnend mit der vorletzten, abwechselnd mit 3 und 1 multipliziert und addiert; die
/// Ergänzung der Summe zum nächsthöheren Vielfachen von 10 ist die Kontrollziffer (0, wenn
/// die Summe schon ein Vielfaches von 10 ist). Das ist das Verfahren der GS1-Prüfziffer
/// (GS1 General Specifications 7.9.1, GTIN-13), das SPEC 5.1 «EAN-13» nennt.
fn ahv_kontrollziffer(erste_zwoelf: &[u8]) -> u8 {
    let summe: u32 = erste_zwoelf
        .iter()
        .rev()
        .zip([3, 1].into_iter().cycle())
        .map(|(&z, gewicht)| u32::from(z) * gewicht)
        .sum();
    ((10 - summe % 10) % 10) as u8
}

// --- iban -----------------------------------------------------------------------------

/// Länder mit der 21-stelligen Ausprägung: «CH» für die Schweiz, «LI» für Liechtenstein
/// (SIX, Schnittstellenbeschreibung IBAN-Tool, Version 10.0, Ziffer 1.1). Der Ländercode
/// steht in Grossbuchstaben (ISO 13616-1:2020, Ziffern 4 und 5: «2!a»).
const IBAN_LAENDER: [&str; 2] = ["CH", "LI"];

/// Ländercode 2, Prüfziffern 2, IID 5, Kontonummer 12 Stellen (SIX, Ziffer 1.1).
const IBAN_LAENGE: usize = 21;

fn iban(s: &str) -> Vec<Treffer> {
    let mut treffer = Vec::new();
    for land in IBAN_LAENDER {
        for (start, _) in s.match_indices(land) {
            if !grenzt_ab(davor(s, start)) {
                continue;
            }
            let Some((ende, zeichen)) = iban_lesen(s, start) else {
                continue;
            };
            if grenzt_ab(danach(s, ende)) && iban_gueltig(&zeichen) {
                treffer.push(
                    Treffer::neu("iban", start, ende, Art::Identifikator)
                        .mit_zusatz("Prüfziffer gültig", "check digit valid"),
                );
            }
        }
    }
    treffer
}

/// Liest 21 Buchstaben oder Ziffern ab `start`: ohne Trenner (elektronisches Format) oder
/// in Vierergruppen (Papierformat; SIX: «in Viererblöcken …, wie es auf Rechnungen oder
/// Kontoauszügen üblich ist»). Zwischen zwei Gruppen stehen ein bis drei Leerraumzeichen
/// jeder Art (Leerzeichen, geschützte Leerzeichen, Tabulatoren), darunter höchstens ein
/// Zeilenumbruch. Gibt das Ende und die Zeichen ohne Leerraum zurück.
fn iban_lesen(s: &str, start: usize) -> Option<(usize, Vec<u8>)> {
    let b = s.as_bytes();
    let ohne_trenner = b
        .get(start..start + IBAN_LAENGE)
        .filter(|teil| teil.iter().all(u8::is_ascii_alphanumeric));
    if let Some(teil) = ohne_trenner {
        return Some((start + IBAN_LAENGE, teil.to_vec()));
    }
    let mut zeichen = Vec::with_capacity(IBAN_LAENGE);
    let mut pos = start;
    while zeichen.len() < IBAN_LAENGE {
        if !zeichen.is_empty() && zeichen.len() % 4 == 0 {
            let (ende, leer, umbrueche) = leerraum(s, pos);
            if !(1..=3).contains(&leer) || umbrueche > 1 {
                return None;
            }
            pos = ende;
        }
        let z = *b.get(pos).filter(|z| z.is_ascii_alphanumeric())?;
        zeichen.push(z);
        pos += 1;
    }
    Some((pos, zeichen))
}

/// Aufbau und Prüfziffern: Stellen 3 bis 9 sind Ziffern (Prüfziffern und IID), die
/// Prüfziffern liegen zwischen 02 und 98 (ISO 13616-1:2020, Ziffer 6.3.3, Anmerkung:
/// «the check digits can only be in the range [02..98]»), und der Rest nach Ziffer 6.2
/// ist 1.
fn iban_gueltig(z: &[u8]) -> bool {
    if z.len() != IBAN_LAENGE || !z[2..9].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let pruefziffern = (z[2] - b'0') * 10 + (z[3] - b'0');
    (2..=98).contains(&pruefziffern) && iban_rest(z[4..].iter().chain(&z[..4])) == 1
}

/// Rest modulo 97 nach ISO 13616-1:2020, Ziffern 6.2.2 bis 6.2.4: die Zeichen in der
/// gegebenen Reihenfolge, Ziffern als eine Stelle, Buchstaben gross oder klein als
/// A = 10 bis Z = 35 (zwei Stellen). Schrittweise gerechnet, damit keine grosse Zahl
/// entsteht.
fn iban_rest<'a>(zeichen: impl Iterator<Item = &'a u8>) -> u32 {
    zeichen.fold(0, |rest, &c| {
        if c.is_ascii_digit() {
            (rest * 10 + u32::from(c - b'0')) % 97
        } else {
            (rest * 100 + u32::from(c.to_ascii_uppercase() - b'A') + 10) % 97
        }
    })
}

// --- tel ------------------------------------------------------------------------------

/// Bereich einer Kennzahl nach Ziffer 3 und 5 TAV E.164.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bereich {
    /// Festnetzdienste, Ziffer 5.1.
    Festnetz,
    /// Unternehmensweite Fernmeldenetze 051 und 058 (Festnetzdienste), Ziffer 5.2 und 5.3.
    Unternehmensnetz,
    /// Funkrufdienste 074 (Mobile Fernmeldedienste), Ziffer 5.4.
    Funkruf,
    /// Mobile Fernmeldedienste, Ziffer 5.5.
    Mobilnetz,
}

impl Bereich {
    fn zusatz(self) -> (&'static str, &'static str) {
        match self {
            Bereich::Festnetz => ("Bereich Festnetz", "fixed-network range"),
            Bereich::Unternehmensnetz => {
                ("Bereich unternehmensweite Netze", "corporate-network range")
            }
            Bereich::Funkruf => ("Bereich Funkruf", "paging range"),
            Bereich::Mobilnetz => ("Bereich Mobilnetz", "mobile range"),
        }
    }
}

/// yz der Kennzahlen 0yz für Festnetzdienste (Ziffer 5.1.3 TAV E.164).
const FESTNETZ: [u8; 20] = [
    21, 22, 24, 26, 27, 31, 32, 33, 34, 41, 43, 44, 52, 55, 56, 61, 62, 71, 81, 91,
];

/// yz der Kennzahlen 0yz für Mobile Fernmeldedienste (Ziffer 5.5.3 TAV E.164).
const MOBILNETZ: [u8; 9] = [68, 69, 72, 73, 75, 76, 77, 78, 79];

/// Der Bereich der nationalen Nummer (neun Ziffern ohne die 0). `None` für jede andere
/// Kennzahl: nicht zugewiesene, Mehrwertdienste (0800, 084x, 090x) und Sonderdienste
/// (086x, 097, 098, 099) nach der Tabelle in Ziffer 3, und 051 mit einer anderen
/// Nummernblockidentifikation als a = 2 (Reserve nach Ziffer 5.2.3).
fn bereich(nsn: &[u8]) -> Option<Bereich> {
    let yz = nsn[0] * 10 + nsn[1];
    if FESTNETZ.contains(&yz) {
        Some(Bereich::Festnetz)
    } else if MOBILNETZ.contains(&yz) {
        Some(Bereich::Mobilnetz)
    } else if yz == 58 || (yz == 51 && nsn[2] == 2) {
        Some(Bereich::Unternehmensnetz)
    } else if yz == 74 {
        Some(Bereich::Funkruf)
    } else {
        None
    }
}

/// Gruppierungen der neun Ziffern nach +41 oder 0041: als Block, 2-7, 2-3-4, 2-3-2-2
/// («+41 32 765 43 21», Ziffer 2.2.6 TAV E.164) und 3-3-3 («+41 327 634 321», ebenda).
const NACH_LANDESKENNZAHL: [&[usize]; 5] = [&[9], &[2, 7], &[2, 3, 4], &[2, 3, 2, 2], &[3, 3, 3]];

/// Gruppierungen der zehn Ziffern mit der 0: als Block, 3-7, 3-3-4, 3-3-2-2
/// («032 765 43 21», Ziffer 2.2.6) und 4-3-3 («0327 654 321», ebenda).
const NATIONAL: [&[usize]; 5] = [&[10], &[3, 7], &[3, 3, 4], &[3, 3, 2, 2], &[4, 3, 3]];

/// Gruppierungen der sieben Ziffern nach «(0yz)» oder «0yz-»: als Block, 3-4 und 3-2-2.
const NACH_KENNZAHL: [&[usize]; 3] = [&[7], &[3, 4], &[3, 2, 2]];

fn tel(s: &str) -> Vec<Treffer> {
    let b = s.as_bytes();
    let mut treffer = Vec::new();
    // Enden der Nummernwörter, gesucht erst beim ersten Block aus zehn Ziffern.
    let mut nummernwoerter: Option<Vec<Nummernwort>> = None;
    let mut i = 0;
    while i < b.len() {
        let kandidat = match b[i] {
            b'+' => tel_international(s, i),
            b'0' => tel_international(s, i)
                .or_else(|| tel_kennzahl_bindestrich(s, i))
                .or_else(|| tel_national(s, i)),
            b'(' => tel_kennzahl_klammer(s, i),
            _ => None,
        };
        let fund = match kandidat {
            Some((ende, nsn)) if tel_links_frei(s, i) && tel_rechts_frei(s, ende) => {
                bereich(&nsn).map(|bereich| (ende, bereich))
            }
            _ => None,
        };
        match fund {
            Some((ende, bereich)) => {
                let andere_nummer = ist_zehnerblock(&s[i..ende])
                    && nummernwort_davor(
                        s,
                        i,
                        nummernwoerter.get_or_insert_with(|| nummernwort_enden(s)),
                    );
                if !andere_nummer {
                    let (de, en) = bereich.zusatz();
                    treffer
                        .push(Treffer::neu("tel", i, ende, Art::Identifikator).mit_zusatz(de, en));
                }
                i = ende;
            }
            None => i += 1,
        }
    }
    treffer
}

/// Wortliste der Nummernwörter vor einem Block aus zehn Ziffern.
pub const NUMMERNWOERTER: &str = "tel-nummernwoerter.toml";

/// Kürzel für «Nummer» zwischen Nummernwort und Nummer («Rechnung Nr. 0791234567»,
/// «facture n° …», «commande no …», «fattura n. …», «Police #…»), gross oder klein. Die
/// Form mit Punkt steht vor der ohne, damit der Punkt mitgeht.
const NUMMER_KUERZEL: [&str; 7] = ["Nr.", "Nr", "No.", "No", "n°", "n.", "#"];

/// Zehn Ziffern ohne Trenner: die nationale Nummer als Block.
fn ist_zehnerblock(nummer: &str) -> bool {
    nummer.len() == 10 && nummer.bytes().all(|z| z.is_ascii_digit())
}

/// Ein Nummernwort im Text: Ende, Anfang und ob es ein blosses Wort ohne «Nummer» ist
/// («Rechnung», «facture», «fattura»; nicht «Rechnungsnummer», «Kunden-Nr.»,
/// «numéro de facture»).
#[derive(Clone, Copy, Debug)]
struct Nummernwort {
    ende: usize,
    start: usize,
    bloss: bool,
}

/// Wortteile, an denen ein Nummernwort die Nummer selbst nennt.
const NUMMER_IM_WORT: [&str; 3] = ["nummer", "numéro", "numero"];

/// Wo im Text ein Nummernwort endet, aufsteigend nach dem Ende. Die Verschmelzungen aus
/// Präposition und Artikel (merkmal «praeposition») sind keine Nummernwörter.
fn nummernwort_enden(s: &str) -> Vec<Nummernwort> {
    let lex = crate::lexikon::lexikon(NUMMERNWOERTER);
    let mut aus: Vec<Nummernwort> = lex
        .finde_in(s)
        .into_iter()
        .filter(|t| lex.eintraege[t.eintrag].merkmal.is_none())
        .map(|t| {
            let begriff = lex.eintraege[t.eintrag].begriff.to_lowercase();
            let bloss = s[t.start..t.ende].to_lowercase() == begriff
                && !NUMMER_IM_WORT.iter().any(|n| begriff.contains(n));
            Nummernwort {
                ende: t.ende,
                start: t.start,
                bloss,
            }
        })
        .collect();
    aus.sort_by_key(|w| w.ende);
    aus
}

/// Das Wort aus Buchstaben, das vor `pos` endet, mit mindestens einem Leerzeichen
/// dazwischen (kein Zeilenumbruch): Anfang und Text.
fn wort_davor(s: &str, pos: usize) -> Option<(usize, &str)> {
    let ende = s[..pos].trim_end_matches(ist_leerzeichen).len();
    if ende == pos {
        return None;
    }
    let anfang = s[..ende]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c.is_alphabetic())
        .last()
        .map(|(i, _)| i)?;
    Some((anfang, &s[anfang..ende]))
}

/// Deckt ein Eintrag mit einem der Merkmale das ganze Wort?
fn ganzes_wort(lex: &crate::lexikon::Lexikon, wort: &str, merkmale: &[&str]) -> bool {
    lex.finde_in(wort).iter().any(|t| {
        t.start == 0
            && t.ende == wort.len()
            && lex.eintraege[t.eintrag]
                .merkmal
                .as_deref()
                .is_some_and(|m| merkmale.contains(&m))
    })
}

/// Steht vor dem blossen Nummernwort bei `start` eine Präposition, wahlweise mit einem
/// bestimmten Begleiter dazwischen? Dann nennt das Wort das Thema, nicht die Nummer: «Bei
/// Fragen zur Rechnung 0791234567 anrufen», «sur la facture: …», «sulla fattura: …».
/// Präpositionen: die Einträge mit merkmal «praeposition» in [`NUMMERNWOERTER`]; der
/// Begleiter aus `firma-grenzen.toml`. «für Rechnung 0791234567» bleibt die Nummer der
/// Rechnung.
fn thema_davor(s: &str, start: usize) -> bool {
    let grenzen = crate::lexikon::lexikon(crate::regeln::firma::LEX_GRENZEN);
    let tel = crate::lexikon::lexikon(NUMMERNWOERTER);
    let praeposition = |w: &str| ganzes_wort(tel, w, &["praeposition"]);
    let Some((anfang, wort)) = wort_davor(s, start) else {
        return false;
    };
    praeposition(wort)
        || (ganzes_wort(grenzen, wort, &["bestimmt"])
            && wort_davor(s, anfang).is_some_and(|(_, w)| praeposition(w)))
}

/// Steht unmittelbar vor `start` ein Nummernwort? Dazwischen nur Leerzeichen (kein
/// Zeilenumbruch), ein Doppelpunkt und Kürzel aus [`NUMMER_KUERZEL`]: «Kundennummer
/// 0441234567», «Kunden-Nr.: 0441234567», «Rechnung Nr. 0791234567». Ein blosses
/// Nummernwort ohne Kürzel nach einer Präposition zählt nicht ([`thema_davor`]).
fn nummernwort_davor(s: &str, start: usize, enden: &[Nummernwort]) -> bool {
    let mut p = start;
    let mut kuerzel_dazwischen = false;
    for _ in 0..4 {
        p = s[..p].trim_end_matches(ist_leerzeichen).len();
        if let Ok(k) = enden.binary_search_by_key(&p, |w| w.ende) {
            let w = enden[k];
            return kuerzel_dazwischen || !w.bloss || !thema_davor(s, w.start);
        }
        let vor = &s[..p];
        if let Some(rest) = vor.strip_suffix(':') {
            p = rest.len();
            continue;
        }
        let kuerzel = NUMMER_KUERZEL.iter().find(|k| {
            p.checked_sub(k.len())
                .and_then(|a| vor.get(a..))
                .is_some_and(|ende| ende.eq_ignore_ascii_case(k))
                && grenzt_ab(davor(s, p - k.len()))
        });
        match kuerzel {
            Some(k) => {
                p -= k.len();
                kuerzel_dazwischen = true;
            }
            None => return false,
        }
    }
    false
}

/// Trenner, über die eine Ziffernfolge weitergehen kann.
fn ist_zifferntrenner(c: char) -> bool {
    ist_leerzeichen(c) || ist_bindestrich(c) || matches!(c, '.' | '/')
}

/// Bindestrich: der gewöhnliche und seine typografischen Varianten Bindestrich U+2010 und
/// geschützter Bindestrich U+2011, die Textverarbeitungen in Nummern setzen.
fn ist_bindestrich(c: char) -> bool {
    matches!(c, '-' | '\u{2010}' | '\u{2011}')
}

/// Davor kein Buchstabe, keine Ziffer, kein «@» oder «+» und keine Ziffer vor einem
/// Trenner: Die Nummer ist nicht das Ende einer längeren Ziffernfolge.
fn tel_links_frei(s: &str, start: usize) -> bool {
    let c = davor(s, start);
    grenzt_ab(c) && c != Some('+') && !kette_davor(s, start, ist_zifferntrenner)
}

/// Danach kein Buchstabe, keine Ziffer, kein «@» und keine Ziffer nach einem Trenner.
fn tel_rechts_frei(s: &str, ende: usize) -> bool {
    grenzt_ab(danach(s, ende)) && !kette_danach(s, ende, ist_zifferntrenner)
}

/// «+41» oder «0041», wahlweise ein Leerzeichen, wahlweise «(0)» mit oder ohne
/// Leerzeichen danach, dann die neun Ziffern. E.123 sieht Klammern in der internationalen
/// Schreibweise nicht vor (Ziffer 7.2); «+41 (0)44 …» wird trotzdem erkannt, weil danach
/// dieselbe nationale Nummer steht.
fn tel_international(s: &str, start: usize) -> Option<(usize, Vec<u8>)> {
    let rest = &s[start..];
    let vorwahl = ["+41", "0041"].into_iter().find(|v| rest.starts_with(*v))?;
    let mut pos = ein_leerzeichen(s, start + vorwahl.len());
    if s[pos..].starts_with("(0)") {
        pos = ein_leerzeichen(s, pos + 3);
    }
    eine_gruppierung(s, pos, &NACH_LANDESKENNZAHL)
}

/// «0» und neun Ziffern; die 0 gehört zur ersten Gruppe.
fn tel_national(s: &str, start: usize) -> Option<(usize, Vec<u8>)> {
    let (ende, ziffern) = eine_gruppierung(s, start, &NATIONAL)?;
    match ziffern.split_first() {
        Some((&0, nsn)) => Some((ende, nsn.to_vec())),
        _ => None,
    }
}

/// «(0yz)», wahlweise ein Leerzeichen, dann die sieben Ziffern: Klammern um
/// Verkehrsausscheidungsziffer und Kennzahl nach E.123 Ziffer 7.2.
fn tel_kennzahl_klammer(s: &str, start: usize) -> Option<(usize, Vec<u8>)> {
    let &[b'(', b'0', y, z, b')'] = s.as_bytes().get(start..start + 5)? else {
        return None;
    };
    teilnehmernummer(s, ein_leerzeichen(s, start + 5), y, z)
}

/// «0yz-», dann die sieben Ziffern: Bindestrich anstelle der Klammern nach E.123,
/// Fussnote 3 zu Ziffer 7.2; auch ein typografischer Bindestrich ([`ist_bindestrich`]).
fn tel_kennzahl_bindestrich(s: &str, start: usize) -> Option<(usize, Vec<u8>)> {
    let &[b'0', y, z] = s.as_bytes().get(start..start + 3)? else {
        return None;
    };
    if !y.is_ascii_digit() || !z.is_ascii_digit() {
        return None;
    }
    let strich = danach(s, start + 3).filter(|&c| ist_bindestrich(c))?;
    teilnehmernummer(s, start + 3 + strich.len_utf8(), y, z)
}

/// Kennzahl yz und sieben Ziffern ab `ab`, zusammen die nationale Nummer.
fn teilnehmernummer(s: &str, ab: usize, y: u8, z: u8) -> Option<(usize, Vec<u8>)> {
    if !y.is_ascii_digit() || !z.is_ascii_digit() {
        return None;
    }
    let (ende, rest) = eine_gruppierung(s, ab, &NACH_KENNZAHL)?;
    let mut nsn = vec![y - b'0', z - b'0'];
    nsn.extend(rest);
    Some((ende, nsn))
}

/// Die erste Gruppierung aus `auswahl`, die ab `ab` passt. Es passt höchstens eine,
/// weil sich die Gruppierungen in der Länge einer Gruppe unterscheiden.
fn eine_gruppierung(s: &str, ab: usize, auswahl: &[&[usize]]) -> Option<(usize, Vec<u8>)> {
    auswahl.iter().find_map(|laengen| gruppen(s, ab, laengen))
}

/// Liest ab `ab` Zifferngruppen der Längen `laengen`, getrennt durch je ein Leerzeichen.
/// Jede Gruppe hat genau diese Länge: Nach ihr folgt keine weitere Ziffer.
fn gruppen(s: &str, ab: usize, laengen: &[usize]) -> Option<(usize, Vec<u8>)> {
    let b = s.as_bytes();
    let mut pos = ab;
    let mut ziffern = Vec::with_capacity(10);
    for (k, &laenge) in laengen.iter().enumerate() {
        if k > 0 {
            let c = danach(s, pos).filter(|&c| ist_leerzeichen(c))?;
            pos += c.len_utf8();
        }
        let teil = b.get(pos..pos + laenge)?;
        if !teil.iter().all(u8::is_ascii_digit) {
            return None;
        }
        ziffern.extend(teil.iter().map(|z| z - b'0'));
        pos += laenge;
        if b.get(pos).is_some_and(u8::is_ascii_digit) {
            return None;
        }
    }
    Some((pos, ziffern))
}

/// Ein Leerzeichen bei `pos` überspringen, falls dort eines steht.
fn ein_leerzeichen(s: &str, pos: usize) -> usize {
    match danach(s, pos) {
        Some(c) if ist_leerzeichen(c) => pos + c.len_utf8(),
        _ => pos,
    }
}

// --- mail -----------------------------------------------------------------------------

/// Postfachnamen für Funktionen nach RFC 2142, Abschnitte 3 bis 5. Sie bezeichnen eine
/// Aufgabe in einer Organisation, keine Person. Gross- und Kleinschreibung zählt nicht
/// (RFC 2142, Abschnitt 2).
const ROLLENADRESSEN: [&str; 15] = [
    // Abschnitt 3
    "info",
    "marketing",
    "sales",
    "support",
    // Abschnitt 4
    "abuse",
    "noc",
    "security",
    // Abschnitt 5
    "postmaster",
    "hostmaster",
    "usenet",
    "news",
    "webmaster",
    "www",
    "uucp",
    "ftp",
];

/// Höchstlänge des lokalen Teils (RFC 5321, 4.5.3.1.1: 64 Oktette).
const LOKAL_MAX: usize = 64;

/// Höchstlänge der Domain (RFC 5321, 4.5.3.1.2: 255 Oktette).
const DOMAIN_MAX: usize = 255;

/// Höchstlänge eines ASCII-Labels (RFC 5890, 2.3.1: «must not be more than 63 octets»).
const LABEL_MAX: usize = 63;

/// So weit wird vor einem «:» oder «/» nach «://» gesucht.
const URL_FENSTER: usize = 512;

/// Gängige Dateiendungen. Als oberstes Label beenden sie keine Domain: Dateinamen mit «@»
/// wie «Scan_Vertrag@2024.pdf» oder «vertrag@final.docx» sind keine Adressen. Gross- und
/// Kleinschreibung zählt nicht. «json», «html», «htm» und «xml» sind keine
/// Top-Level-Domains; «zip» und «mov» sind welche und stehen darum nicht in der Liste.
const DATEIENDUNGEN: [&str; 29] = [
    "pdf", "png", "jpg", "jpeg", "gif", "svg", "docx", "xlsx", "pptx", "msg", "txt", "doc", "xls",
    "ppt", "csv", "rtf", "odt", "ods", "odp", "tif", "tiff", "bmp", "webp", "heic", "eml", "json",
    "html", "htm", "xml",
];

/// Erstes Label, das eine Bildgrösse bezeichnet («logo@2x.png», «icon@3x.zip»): Dateinamen
/// für hochauflösende Bildschirme, keine Domain. Nur bei zwei Labels und einem obersten
/// Label, das keine Länderendung aus zwei Buchstaben ist: «kontakt@2x.ch» bleibt eine
/// Adresse.
const BILDGROESSEN: [&str; 2] = ["2x", "3x"];

fn mail(s: &str) -> Vec<Treffer> {
    let mut treffer = Vec::new();
    for (at, _) in s.match_indices('@') {
        let Some(start) = lokaler_teil(s, at) else {
            continue;
        };
        let Some((ende, _)) = domain(s, at + 1) else {
            continue;
        };
        let keine_adresse = danach(s, ende) == Some('@')
            || in_url_vor_dem_host(s, start)
            || ist_rollenadresse(&s[start..at]);
        if !keine_adresse {
            treffer.push(Treffer::neu("mail", start, ende, Art::Identifikator));
        }
    }
    treffer.extend(mail_verdeckt(s));
    treffer
}

/// Wie ein ausgeschriebenes «@» oder ein ausgeschriebener Punkt geschrieben ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Schreibweise {
    /// In Klammern: «(at)», «[at]», «{at}», «(dot)», «[dot]», «{dot}».
    Klammern,
    /// Als Wort zwischen Leerzeichen: « at », « dot ».
    Wort,
}

/// Adressen, deren «@» gegen Adresssammler ausgeschrieben ist:
/// «marie(at)example.ch», «marie [at] example [dot] ch», «marie{at}example.ch». Lokaler
/// Teil und Domain werden geprüft wie bei «@». Ein « at » ohne Klammern zählt nur, wenn die
/// Domain mindestens einen ausgeschriebenen Punkt hat («marie at example dot ch»): Sonst
/// wäre «reach him at example.com» eine Adresse.
fn mail_verdeckt(s: &str) -> Vec<Treffer> {
    let mut treffer = Vec::new();
    let mut weiter_ab = 0;
    for (p, c) in s.char_indices() {
        if p < weiter_ab || !(matches!(c, '(' | '[' | '{') || ist_leerzeichen(c)) {
            continue;
        }
        let Some((nach_at, art)) = ausgeschrieben(s, p, "at") else {
            continue;
        };
        let Some(start) = lokaler_teil(s, p) else {
            continue;
        };
        let Some((ende, punkt_ausgeschrieben)) = domain(s, nach_at) else {
            continue;
        };
        let keine_adresse = (art == Schreibweise::Wort
            && (!punkt_ausgeschrieben || !wie_eine_adresse(&s[start..p], &s[nach_at..ende])))
            || danach(s, ende) == Some('@')
            || ist_rollenadresse(&s[start..p]);
        if !keine_adresse {
            treffer.push(Treffer::neu("mail", start, ende, Art::Identifikator));
            weiter_ab = ende;
        }
    }
    treffer
}

/// Sieht die Wortform « at » mit « dot » wie eine Adresse aus und nicht wie ein Satz? Die
/// Domain ist ganz klein geschrieben, oder der lokale Teil enthält einen Punkt, eine Ziffer
/// oder einen Unterstrich: «marie at example dot ch» ja, «works at Example dot com» nein.
fn wie_eine_adresse(lokal: &str, domain: &str) -> bool {
    let label_gross = domain
        .split(|c: char| c.is_whitespace() || c == '.')
        .filter(|w| !w.eq_ignore_ascii_case("dot"))
        .any(|w| w.chars().any(char::is_uppercase));
    !label_gross
        || lokal
            .chars()
            .any(|c| c == '.' || c == '_' || c.is_ascii_digit())
}

/// `wort` ausgeschrieben ab `pos`: in runden, eckigen oder geschweiften Klammern, wahlweise
/// mit einem Leerzeichen davor und danach, oder ohne Klammern mit genau einem Leerzeichen
/// davor und danach. Gross oder klein. Das Ende samt Leerzeichen und die Schreibweise.
fn ausgeschrieben(s: &str, pos: usize, wort: &str) -> Option<(usize, Schreibweise)> {
    let nach_leer = ein_leerzeichen(s, pos);
    let gleich = |a: usize| {
        s.get(a..a + wort.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(wort))
    };
    for (auf, zu) in [('(', ')'), ('[', ']'), ('{', '}')] {
        if danach(s, nach_leer) == Some(auf) {
            let innen = nach_leer + 1;
            let hinter = innen + wort.len();
            return (gleich(innen) && danach(s, hinter) == Some(zu))
                .then(|| (ein_leerzeichen(s, hinter + 1), Schreibweise::Klammern));
        }
    }
    if nach_leer == pos || !gleich(nach_leer) {
        return None;
    }
    let hinter = nach_leer + wort.len();
    let leer = danach(s, hinter).filter(|&c| ist_leerzeichen(c))?;
    Some((hinter + leer.len_utf8(), Schreibweise::Wort))
}

/// Ein Punkt zwischen zwei Labels ab `pos`: «.» oder ausgeschrieben wie in
/// [`ausgeschrieben`] («(dot)», «[dot]», «{dot}», « dot »). Das Ende und ob er
/// ausgeschrieben ist.
fn punkt(s: &str, pos: usize) -> Option<(usize, bool)> {
    if s[pos..].starts_with('.') {
        return Some((pos + 1, false));
    }
    ausgeschrieben(s, pos, "dot").map(|(ende, _)| (ende, true))
}

/// Zeichen des lokalen Teils: atext nach RFC 5322, 3.2.3, ohne / = ? & ` { | } (die
/// trennen in Prompts eher Pfade, Abfragen und Code), dazu der Punkt des dot-atom und
/// Buchstaben und Ziffern ausserhalb ASCII (RFC 6532, 3.2: «atext =/ UTF8-non-ascii»).
fn ist_lokalzeichen(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#' | '$' | '%' | '\'' | '*' | '+' | '-' | '^' | '_' | '~' | '.'
        )
        || (!c.is_ascii() && c.is_alphanumeric())
}

/// Anfang des lokalen Teils vor dem «@» bei `at`.
///
/// Die längste Folge erlaubter Zeichen vor dem «@», gekürzt auf das Stück nach dem letzten
/// «..» (dot-atom-text kennt keine zwei Punkte hintereinander, RFC 5322, 3.2.3) und auf den
/// ersten Buchstaben oder die erste Ziffer (davor steht meist ein Satzzeichen). Keine
/// Adresse, wenn der Teil mit einem Punkt endet, länger als 64 Bytes ist oder direkt ein
/// weiteres «@» davor steht.
fn lokaler_teil(s: &str, at: usize) -> Option<usize> {
    let mut start = at;
    for (i, c) in s[..at].char_indices().rev() {
        if !ist_lokalzeichen(c) {
            break;
        }
        start = i;
    }
    if davor(s, start) == Some('@') {
        return None;
    }
    if let Some(p) = s[start..at].rfind("..") {
        start += p + 2;
    }
    start += s[start..at].find(char::is_alphanumeric)?;
    let lokal = &s[start..at];
    (!lokal.ends_with('.') && lokal.len() <= LOKAL_MAX).then_some(start)
}

/// Zeichen eines Labels: Buchstaben, Ziffern und Bindestrich (RFC 5321, 4.1.2), auch
/// ausserhalb ASCII (U-Label, RFC 6531, 3.3: «sub-domain =/ U-label»).
fn ist_labelzeichen(c: char) -> bool {
    c.is_alphanumeric() || c == '-'
}

/// Ende der Domain, die bei `ab` beginnt.
///
/// Labels aus Buchstaben, Ziffern und Bindestrich, nicht mit Bindestrich am Anfang oder
/// Ende (RFC 5321, 4.1.2: «sub-domain = Let-dig [Ldh-str]»), getrennt durch je einen
/// Punkt, auch einem ausgeschriebenen ([`punkt`]). Ein Punkt am Ende gehört zum Satz. Das
/// letzte Label muss alphabetisch sein (RFC 1123, 2.1: «the highest-level component label
/// will be alphabetic») oder ein A-Label mit «xn--» (RFC 5890, 2.3.1) und keine gängige
/// Dateiendung ([`DATEIENDUNGEN`]); sonst fallen Labels hinten weg, bis eines passt (so wird
/// «paket@1.2.3» keine Adresse). Mindestens zwei Labels, höchstens 255 Bytes ohne die
/// ausgeschriebenen Punkte; das erste Label keine Bildgrösse ([`BILDGROESSEN`]).
///
/// Das Ende und ob zwischen den verbleibenden Labels ein Punkt ausgeschrieben ist.
fn domain(s: &str, ab: usize) -> Option<(usize, bool)> {
    // Je Label: Anfang, Ende und ob der Punkt davor ausgeschrieben ist.
    let mut labels: Vec<(usize, usize, bool)> = Vec::new();
    let mut pos = ab;
    let mut punkt_davor = false;
    loop {
        let rest = &s[pos..];
        let lauf = rest
            .find(|c: char| !ist_labelzeichen(c))
            .map_or(rest, |n| &rest[..n]);
        let label = lauf.trim_end_matches('-');
        if label.is_empty()
            || label.starts_with('-')
            || (label.is_ascii() && label.len() > LABEL_MAX)
        {
            break;
        }
        let ende = pos + label.len();
        labels.push((pos, ende, punkt_davor));
        if label.len() != lauf.len() {
            break;
        }
        match punkt(s, ende) {
            Some((weiter, ausgeschrieben))
                if danach(s, weiter).is_some_and(char::is_alphanumeric) =>
            {
                pos = weiter;
                punkt_davor = ausgeschrieben;
            }
            _ => break,
        }
    }
    while let Some(&(a, e, _)) = labels.last() {
        if ist_oberstes_label(&s[a..e]) {
            break;
        }
        labels.pop();
    }
    let &(erster_a, erster_e, _) = labels.first()?;
    let &(letzter_a, letzter_e, _) = labels.last()?;
    let laenderendung = s[letzter_a..letzter_e].len() == 2;
    if labels.len() == 2
        && !laenderendung
        && BILDGROESSEN
            .iter()
            .any(|b| s[erster_a..erster_e].eq_ignore_ascii_case(b))
    {
        return None;
    }
    let &(_, ende, _) = labels.last()?;
    let laenge = labels.iter().map(|&(a, e, _)| e - a + 1).sum::<usize>() - 1;
    let ausgeschrieben = labels.iter().any(|&(_, _, a)| a);
    (labels.len() >= 2 && laenge <= DOMAIN_MAX).then_some((ende, ausgeschrieben))
}

/// Alphabetisch (RFC 1123, 2.1) oder ein A-Label: «xn--», gross oder klein (RFC 5890, 2.3.1);
/// keine gängige Dateiendung.
fn ist_oberstes_label(label: &str) -> bool {
    let dateiendung = DATEIENDUNGEN.iter().any(|d| label.eq_ignore_ascii_case(d));
    !dateiendung
        && (label.chars().all(char::is_alphabetic)
            || (label.len() > 4
                && label
                    .get(..4)
                    .is_some_and(|p| p.eq_ignore_ascii_case("xn--"))))
}

/// Steht der lokale Teil in einer URL zwischen «://» und dem Host, trennt das «@» die
/// Benutzerangaben vom Host («https://name:passwort@host»); das ist keine E-Mail-Adresse.
/// Geprüft wird nur, wenn «:» oder «/» direkt davor steht, und höchstens 512 Zeichen
/// zurück bis zum letzten Leerraum.
fn in_url_vor_dem_host(s: &str, start: usize) -> bool {
    if !matches!(davor(s, start), Some(':' | '/')) {
        return false;
    }
    let anfang = s[..start]
        .char_indices()
        .rev()
        .take(URL_FENSTER)
        .take_while(|&(_, c)| !c.is_whitespace())
        .last()
        .map_or(start, |(i, _)| i);
    let wort = &s[anfang..start];
    wort.rfind("://")
        .is_some_and(|p| !wort[p + 3..].contains('/'))
}

fn ist_rollenadresse(lokal: &str) -> bool {
    ROLLENADRESSEN
        .iter()
        .any(|rolle| lokal.eq_ignore_ascii_case(rolle))
}

// --- Normen ---------------------------------------------------------------------------
//
// Wortlaut von Fedlex, abgerufen am 06.11.2026, geltende Fassung. Die TAV E.164 liegen
// auf Fedlex als PDF ohne Sprungmarken; sie werden nach Ziffer zitiert.

/// Aufbau der AHV-Nummer.
pub const AHVV_133: Norm = Norm {
    zitat: "Art. 133 AHVV",
    erlass: "AHVV, SR 831.101",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/de#art_133",
    wortlaut: "Die AHV-Nummer ist 13-stellig. Sie setzt sich zusammen aus: a. dem dreistelligen Ländercode für die Schweiz (756); b. einer neunstelligen Nummer, welche ausschliesslich für eine bestimmte, im Register der AHV verzeichnete Person verwendet wird, jedoch keinerlei Rückschlüsse auf diese Person zulässt; c. einer Kontrollziffer.",
};

/// Kontrollziffer bei manueller Erfassung.
pub const AHVV_134QUINQUIES_2: Norm = Norm {
    zitat: "Art. 134quinquies Abs. 2 AHVV",
    erlass: "AHVV, SR 831.101",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/de#art_134_quinquies",
    wortlaut: "Manuell kann sie dort erst nach der Prüfung einer Kontrollziffer erfasst werden.",
};

/// Das BAKOM erstellt die Nummerierungspläne.
pub const AEFV_2_1: Norm = Norm {
    zitat: "Art. 2 Abs. 1 AEFV",
    erlass: "AEFV, SR 784.104",
    fassung: "2026-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/1997/2879_2879_2879/de#art_2",
    wortlaut: "Das Bundesamt für Kommunikation (BAKOM) erstellt die Nummerierungspläne und erlässt die Vorschriften über die Verwaltung der Adressierungselemente.",
};

/// Format der Kennzahlen.
pub const AEFV_16: Norm = Norm {
    zitat: "Art. 16 AEFV",
    erlass: "AEFV, SR 784.104",
    fassung: "2026-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/1997/2879_2879_2879/de#art_16",
    wortlaut: "Die Kennzahlen werden grundsätzlich aus zwei Ziffern gebildet, denen die Verkehrsausscheidungsziffer 0 vorangestellt ist (Format=0xx). Das BAKOM kann sie durch Zusatzziffern erweitern.",
};

const TAV_E164: &str = "TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2";
const TAV_E164_FASSUNG: &str = "2024-03-01";
const TAV_E164_URL: &str = "https://www.fedlex.admin.ch/eli/oce/2024/18/de";

/// Länge der nationalen Nummer.
pub const TAV_2_2_4: Norm = Norm {
    zitat: "Ziff. 2.2.4 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Mit Ausnahme der Sonderdienstnummern gemäss Ziffer 2.3.4 umfassen nationale Nummern grundsätzlich 9 Ziffern.",
};

/// Empfohlene Schreibweisen.
pub const TAV_2_2_6: Norm = Norm {
    zitat: "Ziff. 2.2.6 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Für die Publikation einer nationalen Nummer wird eine Darstellung gemäss folgendem Beispiel empfohlen: Nationale Notation: 0327 654 321 oder 032 765 43 21 Internationale Notation: +41 327 634 321 oder +41 32 765 43 21",
};

/// Kennzahlen für Festnetzdienste.
pub const TAV_5_1_3: Norm = Norm {
    zitat: "Ziff. 5.1.3 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Nummern für den Festnetzdienst weisen folgende Struktur auf: 0yz abc xxxx … yz = 21, 22, 24, 26, 27, 31, 32, 33, 34, 41, 43, 44, 52, 55, 56, 61, 62, 71, 81 und 91",
};

/// Bereich 051.
pub const TAV_5_2_3: Norm = Norm {
    zitat: "Ziff. 5.2.3 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Nummern für den Dienst «Unternehmensweite Fernmeldenetze» weisen folgende Struktur auf: 051 abc xxxx … abc Nummernblockidentifikation mit a = 2 Durchgehender Nummernbereich von Kundinnen und Kunden mit Standorten in der gesamten Schweiz mit a = 0, 1, 3 – 9 Reserve",
};

/// Bereich 058.
pub const TAV_5_3_3: Norm = Norm {
    zitat: "Ziff. 5.3.3 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Nummern für den Dienst «Unternehmensweite Fernmeldenetze» weisen folgende Struktur auf: 058 abc xxxx",
};

/// Bereich 074.
pub const TAV_5_4_3: Norm = Norm {
    zitat: "Ziff. 5.4.3 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Nummern für den Funkrufdienst weisen folgende Struktur auf: 074 abc xxxx",
};

/// Kennzahlen für Mobile Fernmeldedienste.
pub const TAV_5_5_3: Norm = Norm {
    zitat: "Ziff. 5.5.3 TAV E.164",
    erlass: TAV_E164,
    fassung: TAV_E164_FASSUNG,
    url: TAV_E164_URL,
    wortlaut: "Nummern für Mobile Fernmeldedienste weisen folgende Struktur auf: 0yz abc xxxx … yz = 68, 69, 72, 73, 75, 76, 77, 78, 79",
};

// --- Katalog --------------------------------------------------------------------------

/// Datum, an dem die Quellen dieses Moduls gelesen und gegen die Regeln geprüft wurden.
const GEPRUEFT: &str = "2026-11-06";

const fn geprueft(titel: &'static str, url: &'static str) -> Quelle {
    Quelle {
        titel,
        url,
        lizenz: None,
        stand: Stand::Geprueft(GEPRUEFT),
    }
}

const fn zu_pruefen(titel: &'static str, url: &'static str, was: &'static str) -> Quelle {
    Quelle {
        titel,
        url,
        lizenz: None,
        stand: Stand::ZuPruefen(was),
    }
}

const ISO_13616_1_VORSCHAU: &str = "https://cdn.standards.iteh.ai/samples/81090/ab77d634b898429f99ee4b13ab8ffe8a/ISO-13616-1-2020.pdf";

/// Die Regeln `ahv`, `iban`, `tel` und `mail`.
pub fn info() -> Vec<RegelInfo> {
    vec![info_ahv(), info_iban(), info_tel(), info_mail()]
}

fn info_ahv() -> RegelInfo {
    RegelInfo {
        id: "ahv",
        klasse: Klasse::A,
        titel: Txt::neu("AHV-Nummer", "AHV number (Swiss social security number)"),
        erkennt: Txt::neu(
            "AHV-Nummer (13 Ziffern, beginnend mit 756) in der Schreibweise 756.1234.5678.97, mit Leerzeichen wie 756 1234 5678 97 oder ohne Trenner, nur mit stimmender Kontrollziffer.",
            "AHV number (13 digits starting with 756), written as 756.1234.5678.97, with spaces as in 756 1234 5678 97 or without separators, only with a matching check digit.",
        ),
        methode: Txt::neu(
            "Sucht «756» mit zehn weiteren Ziffern: gruppiert 3.4.4.2 mit Punkten (Rz 1202 WL VA/IK), wobei die Nummer nach einem Punkt einmal auf die nächste Zeile umbrechen darf, ohne Trenner (eCH-0044) oder, amtlich nicht belegt, gruppiert 3-4-4-2 mit je einem Leerzeichen, auch einem geschützten. Unsichtbare Formatzeichen wie U+00AD oder U+200B werden übergangen. Die Nummer darf nicht Teil einer längeren Folge sein: Davor und danach steht weder Buchstabe noch Ziffer noch «@», und kein Punkt verbindet sie mit weiteren Ziffern, in der Schreibweise mit Leerzeichen auch kein Leerzeichen. Die Kontrollziffer wird nach Anhang 7 WL VA/IK nachgerechnet: von rechts, beginnend mit der vorletzten Ziffer, abwechselnd mal 3 und mal 1 addieren; die Ergänzung zum nächsten Vielfachen von 10 muss die letzte Ziffer sein. Das ist die GS1-Prüfziffer (EAN-13). Gemeldet werden nur Nummern mit stimmender Kontrollziffer.",
            "Looks for «756» followed by ten more digits: grouped 3.4.4.2 with dots (WL VA/IK no. 1202), where the number may wrap to the next line once after a dot, without separators (eCH-0044) or, not officially documented, grouped 3-4-4-2 with one space each, including a no-break space. Invisible format characters such as U+00AD or U+200B are skipped. The number must not be part of a longer sequence: no letter, digit or «@» directly before or after it, and no dot joining it to further digits, in the notation with spaces no space either. The check digit is recomputed per annex 7 WL VA/IK: from the right, starting with the second-to-last digit, add the digits alternately times 3 and times 1; the amount needed to reach the next multiple of 10 must equal the last digit. This is the GS1 check digit (EAN-13). Only numbers with a matching check digit are reported.",
        ),
        hinweis: None,
        grundlage: vec![&normen::DSG_5_A, &AHVV_133, &AHVV_134QUINQUIES_2],
        quellen: vec![
            geprueft(
                "BSV, Wegleitung über Versicherungsausweis und individuelles Konto (WL VA/IK), gültig ab 1.1.2024, Stand 1.10.2025: Rz 1201 und 1202 (Aufbau, Darstellung 756.3047.5009.62) und Anhang 7 (Kontrollzifferprüfung)",
                "https://sozialversicherungen.admin.ch/de/d/6938/download",
            ),
            geprueft(
                "GS1 General Specifications, Release 26.0 (Januar 2026), Ziffer 7.9.1 mit Tabelle 7-8 (Prüfziffer GTIN-13)",
                "https://ref.gs1.org/standards/genspecs/",
            ),
            geprueft(
                "eCH-0044 Personenidentifikation, Version 4.1, XML-Schema: vnType 7560000000001 bis 7569999999999 (Nummer ohne Trenner)",
                "https://www.ech.ch/xmlns/eCH-0044/4/eCH-0044-4-1.xsd",
            ),
            geprueft(
                "BSV, Mitteilungen an die AHV-Ausgleichskassen und EL-Durchführungsstellen Nr. 222 vom 24.4.2008 (Wechsel von der AHVN11 auf die AHVN13 am 1.7.2008)",
                "https://sozialversicherungen.admin.ch/de/d/6692/download",
            ),
            zu_pruefen(
                "ZAS, Berechnung der Kontrollziffer der AHVN13 (in SPEC 10.2 genannt)",
                "https://sozialversicherungen.admin.ch/de/d/6692/download",
                "Unter dieser Adresse liegt am 6.11.2026 die BSV-Mitteilung Nr. 222, nicht das ZAS-Dokument; eine Abschrift auf sz.ch antwortet mit HTTP 404. Das Verfahren ist durch Anhang 7 WL VA/IK belegt.",
            ),
        ],
        lexika: Vec::new(),
        guete: Guete::Nachgewiesen(Txt::neu(
            "Nachgewiesen mit Eigenschaftstests (tests/klasse_a.rs, feste Startwerte): Jede erzeugte gültige AHV-Nummer wird in jeder Schreibweise unter «Methode» in beliebigem Umgebungstext an ihrer Stelle erkannt, sofern sie wie dort abgegrenzt ist. Zufällige Ziffernfolgen mit 756 am Anfang werden in jeder Schreibweise genau dann gemeldet, wenn die Kontrollziffer stimmt. Jede Ersetzung einer einzelnen Ziffer wird abgewiesen, geprüft an jeder Stelle mit jeder anderen Ziffer und für alle Gewichte und Ziffernpaare. Jede Vertauschung zweier verschiedener Nachbarziffern wird abgewiesen, ausser beide stehen nach 756 und unterscheiden sich um 5; genau diese Vertauschungen bleiben unerkannt. Präzision und Trefferquote auf Prompts sind noch nicht gemessen.",
            "Proven by property tests (tests/klasse_a.rs, fixed seeds): Every valid AHV number generated is detected at its position in every notation listed under «Method», in any surrounding text, provided it is delimited as described there. Random digit strings starting with 756 are reported in every notation exactly when the check digit matches. Every replacement of a single digit is rejected, checked at every position with every other digit and for all weights and digit pairs. Every swap of two different neighbouring digits is rejected unless both follow 756 and differ by 5; exactly these swaps go undetected. Precision and recall on prompts have not been measured yet.",
        )),
        grenzen: vec![
            Txt::neu(
                "Erkennt die Schreibweise mit Punkten, die mit Leerzeichen und die ohne Trenner. Die Schreibweise mit Leerzeichen ist in den gelesenen amtlichen Quellen nicht belegt, aber verbreitet. Bindestriche, Schrägstriche, Tabulatoren oder mehrere Leerzeichen als Trenner und ein Zeilenumbruch ohne Punkt davor bleiben unerkannt.",
                "Recognises the dotted notation, the one with spaces and the one without separators. The notation with spaces is not documented in the official sources read but is common. Hyphens, slashes, tabs or several spaces as separators and a line break not preceded by a dot go unrecognised.",
            ),
            Txt::neu(
                "Prüft die Kontrollziffer, nicht, ob die Nummer vergeben ist. Jede zehnte beliebige Ziffernfolge mit 756 am Anfang hat eine stimmende Kontrollziffer und wird gemeldet.",
                "Checks the check digit, not whether the number has been issued. One in ten arbitrary digit strings starting with 756 has a matching check digit and is reported.",
            ),
            Txt::neu(
                "Vertauschte Nachbarziffern, die sich um 5 unterscheiden (16 statt 61), ändern die Kontrollziffer nicht; ein solcher Tippfehler wird als gültige Nummer gemeldet.",
                "Swapped neighbouring digits that differ by 5 (16 instead of 61) leave the check digit unchanged; such a typo is reported as a valid number.",
            ),
            Txt::neu(
                "Die bis 30.6.2008 verwendete 11-stellige AHV-Nummer (AHVN11) wird nicht erkannt.",
                "The 11-digit AHV number used until 30 June 2008 (AHVN11) is not recognised.",
            ),
            Txt::neu(
                "Unmittelbar angrenzende Buchstaben («Nr756…») verhindern die Erkennung.",
                "Directly adjacent letters («Nr756…») prevent detection.",
            ),
        ],
    }
}

fn info_iban() -> RegelInfo {
    RegelInfo {
        id: "iban",
        klasse: Klasse::A,
        titel: Txt::neu(
            "IBAN (Schweiz, Liechtenstein)",
            "IBAN (Switzerland, Liechtenstein)",
        ),
        erkennt: Txt::neu(
            "Internationale Kontonummer (IBAN) aus der Schweiz (CH) und Liechtenstein (LI) mit 21 Zeichen, ohne Trenner oder in Vierergruppen, nur mit stimmenden Prüfziffern.",
            "International bank account number (IBAN) from Switzerland (CH) and Liechtenstein (LI), 21 characters, without separators or in groups of four, only with matching check digits.",
        ),
        methode: Txt::neu(
            "Sucht «CH» oder «LI», zwei Prüfziffern, die fünfstellige Instituts-Identifikation (IID) und die zwölfstellige Kontonummer aus Ziffern und Buchstaben (SIX, Ziffer 1.1), ohne Trenner oder in Vierergruppen. Zwischen zwei Gruppen stehen ein bis drei Leerraumzeichen jeder Art, etwa Leerzeichen, geschützte Leerzeichen oder Tabulatoren, darunter höchstens ein Zeilenumbruch. Unsichtbare Formatzeichen wie U+00AD oder U+200B werden übergangen. Davor und danach steht kein Buchstabe, keine Ziffer und kein «@». Die Prüfziffern werden nach ISO 13616-1:2020 Ziffer 6.2 nachgerechnet: die ersten vier Zeichen ans Ende stellen, Buchstaben gross oder klein als 10 bis 35 lesen, der Rest der Division durch 97 muss 1 sein (MOD 97-10 nach ISO/IEC 7064). Prüfziffern ausserhalb 02 bis 98 entstehen bei der Erzeugung nach Ziffer 6.3 nie und werden abgewiesen.",
            "Looks for «CH» or «LI», two check digits, the five-digit institution identifier (IID) and the twelve-character account number of digits and letters (SIX, section 1.1), without separators or in groups of four. Between two groups there are one to three whitespace characters of any kind, such as spaces, no-break spaces or tabs, with at most one line break among them. Invisible format characters such as U+00AD or U+200B are skipped. No letter, digit or «@» directly before or after it. The check digits are recomputed per ISO 13616-1:2020 clause 6.2: move the first four characters to the end, read letters in either case as 10 to 35, and the remainder of division by 97 must be 1 (MOD 97-10 per ISO/IEC 7064). Check digits outside 02 to 98 never result from generation per clause 6.3 and are rejected.",
        ),
        hinweis: None,
        grundlage: vec![&normen::DSG_5_A],
        quellen: vec![
            geprueft(
                "ISO 13616-1:2020, Ziffern 4 bis 6 (Aufbau 2!a2!n30c, Prüfung und Erzeugung der Prüfziffern), öffentliche Vorschau",
                ISO_13616_1_VORSCHAU,
            ),
            zu_pruefen(
                "ISO 13616-1:2020, Anhang A (Papierformat) und Anhang B",
                ISO_13616_1_VORSCHAU,
                "Nicht in der öffentlichen Vorschau. Die Vierergruppen mit Leerzeichen sind durch SIX belegt.",
            ),
            zu_pruefen(
                "ISO/IEC 7064, Check character systems (MOD 97-10), zitiert in ISO 13616-1:2020, Ziffern 2 und 6.1",
                ISO_13616_1_VORSCHAU,
                "Die Norm selbst ist nicht frei zugänglich und nicht gelesen; umgesetzt ist das Verfahren, wie ISO 13616-1 Ziffer 6.2 es beschreibt.",
            ),
            geprueft(
                "SIX Interbank Clearing, Schnittstellenbeschreibung und Einsatzmöglichkeit des IBAN-Tools, Version 10.0 vom 30.6.2025, Ziffer 1.1 (21-stellige Ausprägung für CH und LI)",
                "https://www.six-group.com/dam/download/banking-services/standardization/iban/tool/guide-banks-softwarecompanies-de.pdf",
            ),
            geprueft(
                "SIX, IBAN, häufig gestellte Fragen (Darstellung in Viererblöcken, Buchstaben in der Kontonummer, QR-IID 30000 bis 31999)",
                "https://www.six-group.com/de/products-services/banking-services/payment-standardization/standards/iban.html",
            ),
            zu_pruefen(
                "SWIFT, IBAN Registry",
                "https://www.swift.com/standards/data-standards/iban-international-bank-account-number",
                "Abruf am 6.11.2026 mit HTTP 403 abgewiesen. Länge und Aufbau für CH und LI sind durch SIX und ISO 13616-1 belegt.",
            ),
        ],
        lexika: Vec::new(),
        guete: Guete::Nachgewiesen(Txt::neu(
            "Nachgewiesen mit Eigenschaftstests (tests/klasse_a.rs, feste Startwerte): Jede erzeugte gültige IBAN aus CH und LI wird in jeder Schreibweise unter «Methode» in beliebigem Umgebungstext an ihrer Stelle erkannt, sofern sie wie dort abgegrenzt ist. Zahlenkolonnen aus «CH» oder «LI» und zufälligen Ziffern in Vierergruppen werden genau dann gemeldet, wenn die Prüfziffern stimmen. Jede Ersetzung einer Ziffer durch eine andere Ziffer, auch in Vierergruppen mit Leerraum und Umbrüchen, und jede Vertauschung zweier verschiedener Nachbarziffern wird abgewiesen, ebenso die Prüfziffern 00, 01 und 99. MOD 97-10 erkennt aber nicht jeden Fehler in einer Kontonummer mit Buchstaben. Unerkannt bleiben genau: ein Buchstabe in der anderen Schreibung (a statt A); eine Ziffer d statt eines Buchstabens mit dem Wert V oder umgekehrt, wenn V - d - 7P durch 97 teilbar ist (P: Rest modulo 97 der Zeichen davor ab der IID); vertauschte Nachbarn 1B, 2M, 3X oder zwei Buchstaben gleichen Werts. Die Tests prüfen genau diese Ausnahmen, an jeder Stelle mit jedem Zeichen. Präzision und Trefferquote auf Prompts sind noch nicht gemessen.",
            "Proven by property tests (tests/klasse_a.rs, fixed seeds): Every valid IBAN from CH and LI generated is detected at its position in every notation listed under «Method», in any surrounding text, provided it is delimited as described there. Columns of figures made of «CH» or «LI» and random digits in groups of four are reported exactly when the check digits match. Every replacement of a digit by another digit, including in groups of four with whitespace and line breaks, and every swap of two different neighbouring digits is rejected, as are check digits 00, 01 and 99. MOD 97-10 does not, however, catch every error in an account number containing letters. Exactly these go undetected: a letter in the other case (a for A); a digit d for a letter of value V or the reverse when V - d - 7P is divisible by 97 (P: remainder modulo 97 of the characters before it, from the IID on); swapped neighbours 1B, 2M, 3X or two letters of equal value. The tests check exactly these exceptions, at every position with every character. Precision and recall on prompts have not been measured yet.",
        )),
        grenzen: vec![
            Txt::neu(
                "Erkennt nur IBAN aus der Schweiz und Liechtenstein; IBAN anderer Länder bleiben unerkannt.",
                "Only IBANs from Switzerland and Liechtenstein are recognised; IBANs from other countries go unrecognised.",
            ),
            Txt::neu(
                "Erkennt nur die Schreibweise ohne Trenner und die Vierergruppen mit ein bis drei Leerraumzeichen dazwischen. Ländercode in Kleinbuchstaben, andere Gruppierungen, Bindestriche, mehr als drei Leerraumzeichen und zwei Zeilenumbrüche zwischen zwei Gruppen bleiben unerkannt.",
                "Only the notation without separators and groups of four with one to three whitespace characters between them are recognised. A lower-case country code, other groupings, hyphens, more than three whitespace characters and two line breaks between two groups go unrecognised.",
            ),
            Txt::neu(
                "Prüft die Prüfziffern, nicht, ob das Konto besteht. Zu Ländercode, IID und Kontonummer passt genau ein Prüfziffernpaar von 02 bis 98; eine zufällige Zeichenfolge im richtigen Aufbau besteht die Prüfung mit einer Wahrscheinlichkeit von 1 zu 100.",
                "Checks the check digits, not whether the account exists. Exactly one pair of check digits from 02 to 98 fits a given country code, IID and account number; a random string with the right structure passes with a probability of 1 in 100.",
            ),
            Txt::neu(
                "Tippfehler in einer Kontonummer mit Buchstaben können die Prüfung bestehen (Ziffer statt Buchstabe, vertauschte Nachbarn 1B, 2M, 3X, Klein- statt Grossbuchstabe); solche IBAN werden als gültig gemeldet.",
                "Typos in an account number containing letters can pass the check (digit for letter, swapped neighbours 1B, 2M, 3X, lower case for upper case); such IBANs are reported as valid.",
            ),
            Txt::neu(
                "Ob das Konto einer natürlichen oder einer juristischen Person gehört, entscheidet die Regel nicht; gemeldet wird jede gültige IBAN, auch eine QR-IBAN (QR-IID 30000 bis 31999).",
                "The rule does not decide whether the account belongs to a natural or a legal person; every valid IBAN is reported, including a QR-IBAN (QR-IID 30000 to 31999).",
            ),
            Txt::neu(
                "Unmittelbar angrenzende Buchstaben («IBANCH93…») verhindern die Erkennung.",
                "Directly adjacent letters («IBANCH93…») prevent detection.",
            ),
        ],
    }
}

fn info_tel() -> RegelInfo {
    RegelInfo {
        id: "tel",
        klasse: Klasse::A,
        titel: Txt::neu("Telefonnummer (Schweiz)", "Telephone number (Switzerland)"),
        erkennt: Txt::neu(
            "Schweizer Telefonnummern aus den Bereichen für Festnetz, unternehmensweite Netze, Funkruf und Mobilnetz, national mit 0 oder international mit +41 oder 0041.",
            "Swiss telephone numbers from the fixed-network, corporate-network, paging and mobile ranges, national with 0 or international with +41 or 0041.",
        ),
        methode: Txt::neu(
            "Erkennt die neunstellige nationale Nummer (Ziffer 2.2.4 TAV E.164) nach «0», «+41» oder «0041», ohne Trenner oder gruppiert mit je einem Leerzeichen: wie in Ziffer 2.2.6 (032 765 43 21, 0327 654 321, +41 32 765 43 21, +41 327 634 321) sowie 3-7 und 3-3-4. Dazu «(032) 765 43 21» und «032-765 43 21» nach E.123 Ziffer 7.2, auch mit dem Bindestrich U+2010 oder dem geschützten Bindestrich U+2011, und «+41 (0)32 765 43 21». Unsichtbare Formatzeichen wie U+00AD oder U+200B werden übergangen. Gemeldet wird nur eine Kennzahl, die der Nummerierungsplan Festnetzdiensten (Ziffern 5.1 bis 5.3) oder Mobilen Fernmeldediensten (Ziffern 5.4 und 5.5) zuweist; der Zusatz nennt den Bereich. Grenzt die Nummer an weitere Ziffern, auch über ein Leerzeichen, einen Punkt, Bindestrich oder Schrägstrich, wird nichts gemeldet. Ein Block aus zehn Ziffern direkt nach einem Nummernwort der Wortliste (Kundennummer, Rechnung, Rechnungsnummer, Referenz, Police, Schadennummer, Auftrag, Patientennummer, Fall-Nr., Dossier; FR etwa numéro de client, facture, police, dossier; IT etwa numero cliente, fattura, polizza, incarto) ist eine Kunden-, Rechnungs-, Policen- oder Fallnummer und wird nicht gemeldet. Dazwischen dürfen nur Leerzeichen, ein Doppelpunkt und «Nr.», «No.», «n°», «n.» oder «#» stehen («Kunden-Nr.: 0441234567», «Rechnung Nr. 0791234567»). Steht vor einem blossen Wort ohne «Nummer» und ohne Kürzel («Rechnung», «Auftrag», «facture», «fattura») «zur», «zum», «sur» oder «sulla» (Duden, Larousse, Treccani), wahlweise mit Artikel, nennt es nur das Thema, und der Block wird gemeldet («Bei Fragen zur Rechnung 0791234567 anrufen», «sur la facture: 0791234567», «sulla fattura: 0791234567»).",
            "Recognises the nine-digit national number (TAV E.164 section 2.2.4) after «0», «+41» or «0041», without separators or grouped with one space: as in section 2.2.6 (032 765 43 21, 0327 654 321, +41 32 765 43 21, +41 327 634 321) and as 3-7 and 3-3-4. Also «(032) 765 43 21» and «032-765 43 21» per E.123 clause 7.2, including with the hyphen U+2010 or the non-breaking hyphen U+2011, and «+41 (0)32 765 43 21». Invisible format characters such as U+00AD or U+200B are skipped. Only area codes the numbering plan assigns to fixed-network services (sections 5.1 to 5.3) or mobile services (sections 5.4 and 5.5) are reported; the note names the range. If the number adjoins further digits, even across a space, dot, hyphen or slash, nothing is reported. A block of ten digits directly after a number word from the word list (Kundennummer, Rechnung, Rechnungsnummer, Referenz, Police, Schadennummer, Auftrag, Patientennummer, Fall-Nr., Dossier; in French e.g. numéro de client, facture, police, dossier; in Italian e.g. numero cliente, fattura, polizza, incarto) is a customer, invoice, policy or case number and is not reported. Only spaces, a colon and «Nr.», «No.», «n°», «n.» or «#» may stand in between («Kunden-Nr.: 0441234567», «Rechnung Nr. 0791234567»). If «zur», «zum», «sur» or «sulla» (Duden, Larousse, Treccani), optionally with an article, precedes a bare word without «Nummer» and without an abbreviation («Rechnung», «Auftrag», «facture», «fattura»), the word only names the topic and the block is reported («Bei Fragen zur Rechnung 0791234567 anrufen», «sur la facture: 0791234567», «sulla fattura: 0791234567»).",
        ),
        hinweis: None,
        grundlage: vec![
            &normen::DSG_5_A,
            &AEFV_2_1,
            &AEFV_16,
            &TAV_2_2_4,
            &TAV_2_2_6,
            &TAV_5_1_3,
            &TAV_5_2_3,
            &TAV_5_3_3,
            &TAV_5_4_3,
            &TAV_5_5_3,
        ],
        quellen: vec![
            geprueft(
                "BAKOM, TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 vom 8.11.2023, in Kraft seit 1.3.2024 (SR 784.101.113/2.2): Ziffern 2.2, 2.3, 3 und 5",
                TAV_E164_URL,
            ),
            geprueft(
                "BAKOM, Number blocks and codes: führt am 6.11.2026 als geltende TAV «CC 784.101.113/2.2 Numbering plan and segmentation of the E.164 numbers Edition 8: 08.11.2023 / Entry into force: 01.03.2024»",
                "https://www.bakom.admin.ch/en/number-blocks-and-codes",
            ),
            geprueft(
                "ITU-T E.123 (02/2001), Notation for national and international telephone numbers: Ziffern 2.5 bis 2.9, 7.1 und 7.2 mit Fussnote 3",
                "https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-E.123-200102-I!!PDF-E&type=items",
            ),
        ],
        lexika: vec![NUMMERNWOERTER],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Ob die Nummer einer Person oder einem Unternehmen gehört und ob sie vergeben ist, entscheidet die Regel nicht; gemeldet wird jede Nummer aus den genannten Bereichen.",
                "The rule does not decide whether the number belongs to a person or a company or whether it has been assigned; every number from the ranges named is reported.",
            ),
            Txt::neu(
                "Gratis-, Gebührenteilungs- und Mehrwertdienstnummern (0800, 084x, 090x) und Sonderdienste (086x, 097 bis 099, Kurznummern 1xx) werden bewusst nicht gemeldet: Ihre Anfangsziffern bezeichnen einen Dienst, keinen Anschluss (Ziffern 2.3.3 und 2.3.4 TAV E.164). Auch der Zugang zur Voice-Mailbox (0860 mit nachfolgender Nummer) wird nicht gemeldet.",
                "Freephone, shared-cost and premium-rate numbers (0800, 084x, 090x) and special services (086x, 097 to 099, short numbers 1xx) are deliberately not reported: their leading digits designate a service, not a line (TAV E.164 sections 2.3.3 and 2.3.4). Voicemail access (0860 followed by a number) is not reported either.",
            ),
            Txt::neu(
                "Nicht erkannt werden Schrägstrich, Punkte oder Bindestriche zwischen den Gruppen (032/765 43 21, 032.765.43.21, 079-123-45-67), «+41 032 …» ohne Klammern, andere Gruppierungen sowie Nummern aus Liechtenstein (+423) und anderen Ländern.",
                "Not recognised: slashes, dots or hyphens between the groups (032/765 43 21, 032.765.43.21, 079-123-45-67), «+41 032 …» without parentheses, other groupings, and numbers from Liechtenstein (+423) and other countries.",
            ),
            Txt::neu(
                "Zwei Nummern nur durch ein Leerzeichen getrennt oder eine Nummer mit Zusatz wie «/22» gelten als längere Ziffernfolge und werden nicht gemeldet.",
                "Two numbers separated only by a space, or a number with an addition such as «/22», count as a longer digit sequence and are not reported.",
            ),
            Txt::neu(
                "Kunden-, Rechnungs-, Policen- oder Patientennummern aus zehn Ziffern, die mit 0 und einer Kennzahl aus den genannten Bereichen beginnen, werden als Telefonnummer gemeldet, wenn kein Nummernwort unmittelbar davor steht («Die Nummer 0441234567 steht auf der Rechnung.»).",
                "Customer, invoice, policy or patient numbers of ten digits that begin with 0 and an area code from the ranges named are reported as telephone numbers unless a number word stands directly before them («Die Nummer 0441234567 steht auf der Rechnung.»).",
            ),
            Txt::neu(
                "Eine Telefonnummer als Block aus zehn Ziffern direkt nach einem Nummernwort bleibt unerkannt, etwa «Referenz: 0791234567» im Lebenslauf oder «Bitte Auftrag 0791234567 anrufen». Gruppiert («079 123 45 67») oder mit +41 wird sie auch dort gemeldet. Umgekehrt gilt eine Rechnungsnummer nach «sur la» oder «zur» ohne Kürzel als Telefonnummer («le montant sur la facture 0791234567»). «Referenz» allein ist als Nummernwort nicht eigens belegt; wörtlich belegt ist «Referenznummern» (AHVV Art. 71 Abs. 3).",
                "A telephone number written as a block of ten digits directly after a number word goes unrecognised, for example «Referenz: 0791234567» in a CV or «Bitte Auftrag 0791234567 anrufen». Grouped («079 123 45 67») or with +41 it is reported there too. Conversely an invoice number after «sur la» or «zur» without an abbreviation counts as a telephone number («le montant sur la facture 0791234567»). «Referenz» alone is not attested as a number word in its own right; «Referenznummern» is attested verbatim (AHVV Art. 71 Abs. 3).",
            ),
        ],
    }
}

fn info_mail() -> RegelInfo {
    RegelInfo {
        id: "mail",
        klasse: Klasse::A,
        titel: Txt::neu("E-Mail-Adresse", "E-mail address"),
        erkennt: Txt::neu(
            "E-Mail-Adressen der Form name@domain.tld, auch ausgeschrieben als name(at)domain.tld, ohne Rollenadressen nach RFC 2142 wie info@ oder support@.",
            "E-mail addresses of the form name@domain.tld, also written out as name(at)domain.tld, without role addresses under RFC 2142 such as info@ or support@.",
        ),
        methode: Txt::neu(
            "Erkennt eine Teilmenge von addr-spec nach RFC 5322, Abschnitt 3.4.1. Lokaler Teil als dot-atom aus Buchstaben, Ziffern und ! # $ % ' * + - ^ _ ~ (atext ohne / = ? & ` { | }), Punkte nicht am Anfang, nicht am Ende und nicht doppelt, höchstens 64 Bytes (RFC 5321, 4.5.3.1.1). Domain aus mindestens zwei Labels aus Buchstaben, Ziffern und Bindestrich, ohne Bindestrich am Rand (RFC 5321, 4.1.2), letztes Label alphabetisch (RFC 1123, 2.1) oder ein A-Label «xn--» (RFC 5890, 2.3.1), höchstens 255 Bytes (RFC 5321, 4.5.3.1.2). Buchstaben ausserhalb ASCII sind zugelassen (RFC 6532, 3.2; RFC 6531, 3.3). Rollenadressen nach RFC 2142, Abschnitte 3 bis 5, werden in jeder Schreibung übergangen (Abschnitt 2): Sie bezeichnen eine Aufgabe, keine Person. Unsichtbare Formatzeichen wie U+00AD oder U+200B werden übergangen. Erkannt wird auch die Schreibweise gegen Adresssammler: «@» als «(at)», «[at]» oder «{at}», wahlweise mit einem Leerzeichen davor und danach, und Punkte der Domain als «(dot)», «[dot]», «{dot}» oder « dot » («marie(at)example.ch», «marie [at] example [dot] ch»). Ein « at » ohne Klammern zählt nur zusammen mit einem ausgeschriebenen Punkt und nur, wenn die Domain klein geschrieben ist oder der lokale Teil einen Punkt, eine Ziffer oder einen Unterstrich enthält («marie at example dot ch»). Keine Adresse ergeben gängige Dateiendungen als letztes Label (pdf, png, jpg, jpeg, gif, svg, docx, xlsx, pptx, msg, txt, doc, xls, ppt, csv, rtf, odt, ods, odp, tif, tiff, bmp, webp, heic, eml, json, html, htm, xml) und «2x» oder «3x» als erstes von zwei Labels vor einer Endung, die keine Länderendung ist (Bildgrösse wie in «logo@2x.png»).",
            "Recognises a subset of addr-spec per RFC 5322, section 3.4.1. Local part as dot-atom of letters, digits and ! # $ % ' * + - ^ _ ~ (atext without / = ? & ` { | }), no dot at the start, at the end or doubled, at most 64 bytes (RFC 5321, 4.5.3.1.1). Domain of at least two labels of letters, digits and hyphens, no hyphen at the edge (RFC 5321, 4.1.2), last label alphabetic (RFC 1123, 2.1) or an «xn--» A-label (RFC 5890, 2.3.1), at most 255 bytes (RFC 5321, 4.5.3.1.2). Letters outside ASCII are allowed (RFC 6532, 3.2; RFC 6531, 3.3). Role addresses under RFC 2142, sections 3 to 5, are skipped in any case (section 2): they designate a function, not a person. Invisible format characters such as U+00AD or U+200B are skipped. The notation used against address harvesters is recognised too: «@» written as «(at)», «[at]» or «{at}», optionally with one space before and after, and dots of the domain written as «(dot)», «[dot]», «{dot}» or « dot » («marie(at)example.ch», «marie [at] example [dot] ch»). An « at » without brackets counts only together with a written-out dot and only if the domain is in lower case or the local part contains a dot, a digit or an underscore («marie at example dot ch»). Common file extensions as the last label (pdf, png, jpg, jpeg, gif, svg, docx, xlsx, pptx, msg, txt, doc, xls, ppt, csv, rtf, odt, ods, odp, tif, tiff, bmp, webp, heic, eml, json, html, htm, xml) and «2x» or «3x» as the first of two labels before an ending that is not a country code (image size as in «logo@2x.png») give no address.",
        ),
        hinweis: None,
        grundlage: vec![&normen::DSG_5_A],
        quellen: vec![
            geprueft(
                "RFC 5322, Internet Message Format: Abschnitte 3.2.3 (atext, dot-atom) und 3.4.1 (addr-spec)",
                "https://www.rfc-editor.org/rfc/rfc5322#section-3.4.1",
            ),
            geprueft(
                "RFC 5321, Simple Mail Transfer Protocol: Abschnitte 4.1.2 (Domain) und 4.5.3.1.1 bis 4.5.3.1.2 (Längen)",
                "https://www.rfc-editor.org/rfc/rfc5321#section-4.5.3.1",
            ),
            geprueft(
                "RFC 6532, Internationalized Email Headers: Abschnitt 3.2 (atext mit UTF-8)",
                "https://www.rfc-editor.org/rfc/rfc6532#section-3.2",
            ),
            geprueft(
                "RFC 6531, SMTP Extension for Internationalized Email: Abschnitt 3.3 (U-Label in der Domain)",
                "https://www.rfc-editor.org/rfc/rfc6531#section-3.3",
            ),
            geprueft(
                "RFC 1123, Requirements for Internet Hosts: Abschnitt 2.1 (oberstes Label alphabetisch)",
                "https://www.rfc-editor.org/rfc/rfc1123#section-2.1",
            ),
            geprueft(
                "RFC 5890, IDNA Definitions: Abschnitt 2.3.1 (Präfix «xn--», Labels höchstens 63 Oktette)",
                "https://www.rfc-editor.org/rfc/rfc5890#section-2.3.1",
            ),
            geprueft(
                "RFC 3696, Application Techniques for Checking and Transformation of Names: Abschnitt 3 (Punkte im lokalen Teil)",
                "https://www.rfc-editor.org/rfc/rfc3696#section-3",
            ),
            geprueft(
                "RFC 2142, Mailbox Names for Common Services, Roles and Functions: Abschnitte 2 bis 5",
                "https://www.rfc-editor.org/rfc/rfc2142#section-3",
            ),
            geprueft(
                "RFC 2606, Reserved Top Level DNS Names: Abschnitte 2 und 3 (Beispieldomains der Tests)",
                "https://www.rfc-editor.org/rfc/rfc2606#section-3",
            ),
        ],
        lexika: Vec::new(),
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Rollenadressen nach RFC 2142 (info, marketing, sales, support, abuse, noc, security, postmaster, hostmaster, usenet, news, webmaster, www, uucp, ftp) werden nie gemeldet, auch nicht im Geheimnis-Kontext und auch dann nicht, wenn die Domain einen Personennamen enthält.",
                "Role addresses under RFC 2142 (info, marketing, sales, support, abuse, noc, security, postmaster, hostmaster, usenet, news, webmaster, www, uucp, ftp) are never reported, not even in a secrecy context and not even when the domain contains a person's name.",
            ),
            Txt::neu(
                "Andere Funktionsadressen (kontakt@, buchhaltung@) werden gemeldet; ob eine Person dahintersteht, entscheidet die Regel nicht.",
                "Other function addresses (contact@, accounting@) are reported; the rule does not decide whether a person is behind them.",
            ),
            Txt::neu(
                "Nicht erkannt: lokaler Teil in Anführungszeichen, IP-Adresse als Domain (name@[192.0.2.1]), Domain ohne Punkt (name@localhost), Fediverse-Kennungen (@name@server). Steht / = ? & ` { | } im lokalen Teil, beginnt die Adresse erst nach diesem Zeichen.",
                "Not recognised: quoted local parts, an IP address as domain (name@[192.0.2.1]), a domain without a dot (name@localhost), fediverse handles (@name@server). If / = ? & ` { | } appears in the local part, the address starts only after that character.",
            ),
            Txt::neu(
                "In einer URL gilt «name:passwort@host» nicht als Adresse; eine Adresse im Pfad oder in der Abfrage einer URL schon.",
                "In a URL, «name:password@host» does not count as an address; an address in the path or query of a URL does.",
            ),
            Txt::neu(
                "Folgt ohne Leerzeichen ein Punkt und ein Wort (name@example.com.Danke), zählt das Wort zur Domain.",
                "If a dot and a word follow without a space (name@example.com.Thanks), the word counts as part of the domain.",
            ),
            Txt::neu(
                "Andere Umschreibungen («(punkt)», «ät») und « at » ohne ausgeschriebenen Punkt werden nicht gelesen. Einen ausgeschriebenen Punkt im lokalen Teil liest die Regel nicht: In «marie (dot) muster (at) example.ch» beginnt die Stelle erst bei «muster». Ein Kürzel in Klammern vor einer Domain kann als Adresse gelten («Wien (AT) example.at»), ebenso «(at)» im Code («arr(at)idx.length») und ein englischer Satz mit klein geschriebener Domain («I work at example dot com»).",
                "Other paraphrases («(punkt)», «ät») and « at » without a written-out dot are not read. A written-out dot in the local part is not read either: in «marie (dot) muster (at) example.ch» the match starts only at «muster». An abbreviation in brackets before a domain can count as an address («Wien (AT) example.at»), as can «(at)» in code («arr(at)idx.length») and an English sentence with a lower-case domain («I work at example dot com»).",
            ),
            Txt::neu(
                "Dateinamen mit «@» werden nur bei den genannten Endungen und bei «@2x.» oder «@3x.» übergangen; andere Dateinamen mit alphabetischer Endung (bericht@final.zip, clip@2024.mov; «zip» und «mov» sind auch Top-Level-Domains) werden gemeldet. Eine Adresse, deren letztes Label wie eine dieser Dateiendungen heisst, bleibt unerkannt.",
                "File names with «@» are skipped only for the extensions named and for «@2x.» or «@3x.»; other file names with an alphabetic extension (report@final.zip, clip@2024.mov; «zip» and «mov» are also top-level domains) are reported. An address whose last label is named like one of these file extensions goes unrecognised.",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Optionen;
    use crate::fund::{Rolle, Stufe};

    /// Die Stellen, die `regel` im Text meldet.
    fn stellen(roh: &str, regel: &str) -> Vec<String> {
        let text = Text::neu(roh);
        erkenne(&text)
            .into_iter()
            .filter(|t| t.regel == regel)
            .map(|t| text.als_str()[t.start..t.ende].to_string())
            .collect()
    }

    /// Der deutsche Zusatz der Treffer von `regel`.
    fn zusaetze(roh: &str, regel: &str) -> Vec<String> {
        erkenne(&Text::neu(roh))
            .into_iter()
            .filter(|t| t.regel == regel)
            .filter_map(|t| t.zusatz.map(|z| z.de))
            .collect()
    }

    #[track_caller]
    fn genau(roh: &str, regel: &str, erwartet: &[&str]) {
        assert_eq!(stellen(roh, regel), erwartet, "{regel} in {roh:?}");
    }

    #[track_caller]
    fn nichts(roh: &str, regel: &str) {
        genau(roh, regel, &[]);
    }

    // --- ahv ---

    #[test]
    fn ahv_beispiele_der_wegleitung() {
        // Rz 1202 und Anhang 7 WL VA/IK.
        genau("756.3047.5009.62", "ahv", &["756.3047.5009.62"]);
        genau("756.1234.5678.97", "ahv", &["756.1234.5678.97"]);
        assert_eq!(zusaetze("756.1234.5678.97", "ahv"), ["Prüfziffer gültig"]);
    }

    #[test]
    fn ahv_deutsch_franzoesisch_italienisch() {
        genau(
            "Meine AHV-Nummer lautet 756.1234.5678.97.",
            "ahv",
            &["756.1234.5678.97"],
        );
        genau(
            "Mon numéro AVS est le 756.3047.5009.62, merci.",
            "ahv",
            &["756.3047.5009.62"],
        );
        genau(
            "Il mio numero AVS è 7563047500962.",
            "ahv",
            &["7563047500962"],
        );
    }

    #[test]
    fn ahv_falsche_kontrollziffer() {
        nichts("756.1234.5678.98", "ahv");
        nichts("7561234567890", "ahv");
        // Gegenprobe: dieselbe Nummer mit stimmender Kontrollziffer.
        genau("7561234567897", "ahv", &["7561234567897"]);
    }

    #[test]
    fn ahv_nur_mit_756() {
        // 757.1234.5678.96 hat eine stimmende GS1-Prüfziffer, ist aber keine AHV-Nummer.
        nichts("757.1234.5678.96", "ahv");
        nichts("7571234567896", "ahv");
    }

    #[test]
    fn ahv_nicht_teil_einer_laengeren_folge() {
        for text in [
            "1756.1234.5678.97",
            "756.1234.5678.971",
            "17561234567897",
            "75612345678970",
            "X7561234567897",
            "756.1234.5678.97b",
            "1.756.1234.5678.97",
            "756.1234.5678.97.5",
            "name@7561234567897",
        ] {
            nichts(text, "ahv");
        }
        // Satzzeichen, Klammern und ein Punkt nach einer Abkürzung sind Grenzen.
        genau("AHV-Nr.756.1234.5678.97.", "ahv", &["756.1234.5678.97"]);
        genau("(756.1234.5678.97)", "ahv", &["756.1234.5678.97"]);
        genau("«7561234567897»", "ahv", &["7561234567897"]);
    }

    /// Die gemessene Güte gilt für Nummern, die wie unter «Methode» abgegrenzt sind;
    /// angrenzende Buchstaben verhindern die Erkennung, wie die Grenzen sagen.
    #[test]
    fn guete_an_die_methode_gebunden() {
        nichts("Nr756.1234.5678.97", "ahv");
        nichts("Nr7561234567897", "ahv");
        genau("Nr. 756.1234.5678.97", "ahv", &["756.1234.5678.97"]);
        nichts("IBANCH9300762011623852957", "iban");
        genau(
            "IBAN CH93 0076 2011 6238 5295 7",
            "iban",
            &["CH93 0076 2011 6238 5295 7"],
        );
        let infos = info();
        for (id, beispiel, en_satz) in [
            (
                "ahv",
                "«Nr756…»",
                "Every valid AHV number generated is detected",
            ),
            (
                "iban",
                "«IBANCH93…»",
                "Every valid IBAN from CH and LI generated is detected",
            ),
        ] {
            let r = infos.iter().find(|r| r.id == id).expect(id);
            let Guete::Nachgewiesen(t) = r.guete else {
                panic!("{id}: Güte nicht nachgewiesen");
            };
            // Wörtlich, auch der Grossbuchstabe am Satzanfang.
            assert!(t.en.contains(en_satz), "{id}");
            assert!(
                t.de.contains(
                    "in jeder Schreibweise unter «Methode» in beliebigem Umgebungstext an ihrer Stelle erkannt, sofern sie wie dort abgegrenzt ist."
                ),
                "{id}"
            );
            assert!(
                t.en.contains(
                    "in every notation listed under «Method», in any surrounding text, provided it is delimited as described there."
                ),
                "{id}"
            );
            assert!(
                r.grenzen.iter().any(|g| g.de.contains(beispiel)
                    && g.de.contains("verhindern die Erkennung")
                    && g.en.contains(beispiel)),
                "{id}"
            );
        }
    }

    #[test]
    fn ahv_andere_trenner_sind_nicht_belegt() {
        // Die Schreibweise mit je einem Leerzeichen («756 1234 5678 97») prüft
        // `ahv_mit_leerzeichen`.
        for text in [
            "756-1234-5678-97",
            "756/1234/5678/97",
            "756.12345678.97",
            "756\u{2011}1234\u{2011}5678\u{2011}97",
            "756\t1234\t5678\t97",
            "756  1234 5678 97",
            "756 1234\n5678 97",
            "756.1234 5678.97",
            "756 1234.5678 97",
        ] {
            nichts(text, "ahv");
        }
    }

    #[test]
    fn ahv_mit_leerzeichen() {
        // Die Beispiele aus `erkennt`: «756 1234 5678 97».
        genau("756 1234 5678 97", "ahv", &["756 1234 5678 97"]);
        genau(
            "Meine AHV-Nummer ist 756 1234 5678 97.",
            "ahv",
            &["756 1234 5678 97"],
        );
        genau(
            "Mon numéro AVS : 756\u{A0}9876\u{A0}5432\u{A0}17, merci.",
            "ahv",
            &["756\u{A0}9876\u{A0}5432\u{A0}17"],
        );
        genau(
            "Il mio numero AVS è 756\u{202F}1234 5678\u{A0}97.",
            "ahv",
            &["756\u{202F}1234 5678\u{A0}97"],
        );
        // Gegenbeispiele: falsche Kontrollziffer, Teil einer Zahlenkolonne.
        for text in [
            "756 1234 5678 98",
            "756 9876 5432 18",
            "1 756 1234 5678 97",
            "756 1234 5678 97 12",
            "Nr. 3.756 1234 5678 97",
            "756 1234 5678 97.5",
        ] {
            nichts(text, "ahv");
        }
    }

    #[test]
    fn ahv_umbruch_nach_einem_punkt() {
        for (text, stelle) in [
            ("AHV 756.1234.\n5678.97 bitte", "756.1234.\n5678.97"),
            ("AHV 756.\n1234.5678.97", "756.\n1234.5678.97"),
            ("AHV 756.1234.5678.\n97", "756.1234.5678.\n97"),
            ("AVS 756.9876. \n 5432.17", "756.9876. \n 5432.17"),
            ("AVS 756.9876.\r\n5432.17", "756.9876.\n5432.17"),
        ] {
            genau(text, "ahv", &[stelle]);
        }
        for text in [
            "756.1234.\n5678.\n97",
            "756.1234.\n\n5678.97",
            "756.1234.   \n5678.97",
            "756.1234\n.5678.97",
            "756.1234.\n5678.98",
        ] {
            nichts(text, "ahv");
        }
    }

    #[test]
    fn unsichtbare_formatzeichen_in_kennungen() {
        // Die Stelle umfasst die unsichtbaren Zeichen in der Nummer, nicht die davor
        // und danach.
        for (text, regel, stelle) in [
            (
                "AHV \u{200B}756\u{AD}.1234.5678.97\u{200B}.",
                "ahv",
                "756\u{AD}.1234.5678.97",
            ),
            ("AHV 7561234\u{2060}567897", "ahv", "7561234\u{2060}567897"),
            (
                "IBAN CH93\u{200B}00762011623852957",
                "iban",
                "CH93\u{200B}00762011623852957",
            ),
            (
                "IBAN CH93 0076 2011 6238\u{AD} 5295 7",
                "iban",
                "CH93 0076 2011 6238\u{AD} 5295 7",
            ),
            ("Tel. 079 1\u{200B}23 45 67", "tel", "079 1\u{200B}23 45 67"),
            ("Tel. 079 123 45 6\u{2060}7", "tel", "079 123 45 6\u{2060}7"),
            (
                "Mail: marie.mus\u{AD}ter@beispiel.ch",
                "mail",
                "marie.mus\u{AD}ter@beispiel.ch",
            ),
            (
                "Mail: marie@bei\u{200B}spiel.ch",
                "mail",
                "marie@bei\u{200B}spiel.ch",
            ),
        ] {
            genau(text, regel, &[stelle]);
        }
        // Ein Formatzeichen macht keine Grenze: Davor steht weiter eine Ziffer.
        nichts("1\u{200B}7561234567897", "ahv");
        nichts("X\u{AD}CH9300762011623852957", "iban");
    }

    // --- iban ---

    #[test]
    fn iban_beispiele_von_six() {
        for iban in [
            "CH93 0076 2011 6238 5295 7",
            "CH69 0647 0016 0066 7100 2",
            "CH9300762011623852957",
        ] {
            genau(iban, "iban", &[iban]);
        }
        assert_eq!(
            zusaetze("CH9300762011623852957", "iban"),
            ["Prüfziffer gültig"]
        );
    }

    #[test]
    fn iban_deutsch_franzoesisch_italienisch() {
        genau(
            "Bitte auf CH93 0076 2011 6238 5295 7 überweisen.",
            "iban",
            &["CH93 0076 2011 6238 5295 7"],
        );
        genau(
            "Virement sur CH9300762011623852957, merci.",
            "iban",
            &["CH9300762011623852957"],
        );
        genau(
            "Pagamento sul conto LI89 0880 0000 0123 4567 Q, grazie.",
            "iban",
            &["LI89 0880 0000 0123 4567 Q"],
        );
    }

    #[test]
    fn iban_geschuetzte_und_schmale_leerzeichen() {
        for iban in [
            "CH93\u{00A0}0076\u{00A0}2011\u{00A0}6238\u{00A0}5295\u{00A0}7",
            "CH93\u{202F}0076\u{202F}2011\u{202F}6238\u{202F}5295\u{202F}7",
        ] {
            genau(iban, "iban", &[iban]);
        }
    }

    #[test]
    fn iban_buchstabe_in_der_kontonummer() {
        // Das Beispiel der SIX-FAQ «CH35 0023 0230 1234 5601 X» hat nicht stimmende
        // Prüfziffern; zu dieser Kontonummer gehören die Prüfziffern 64.
        nichts("CH35 0023 0230 1234 5601 X", "iban");
        genau(
            "CH64 0023 0230 1234 5601 X",
            "iban",
            &["CH64 0023 0230 1234 5601 X"],
        );
    }

    #[test]
    fn iban_falsche_pruefziffern() {
        nichts("CH94 0076 2011 6238 5295 7", "iban");
        nichts("CH93 0076 2011 6238 5295 8", "iban");
    }

    #[test]
    fn iban_pruefziffern_nur_von_02_bis_98() {
        // Jedes Paar besteht die Rechnung modulo 97; erzeugt werden können aber nur 02 bis
        // 98 (ISO 13616-1:2020, Ziffer 6.3.3).
        for (gut, schlecht) in [
            ("CH97 0076 2000 0000 0008 7", "CH00 0076 2000 0000 0008 7"),
            ("CH98 0076 2000 0000 0006 9", "CH01 0076 2000 0000 0006 9"),
            ("CH02 0076 2000 0000 0005 1", "CH99 0076 2000 0000 0005 1"),
        ] {
            genau(gut, "iban", &[gut]);
            nichts(schlecht, "iban");
        }
    }

    #[test]
    fn iban_andere_laender_und_schreibweisen() {
        // Ein bis drei Leerraumzeichen zwischen den Gruppen («CH93  0076 2011 6238 5295 7»)
        // prüft `iban_leerraum_zwischen_gruppen`.
        for text in [
            "DE89 3704 0044 0532 0130 00",
            "ch93 0076 2011 6238 5295 7",
            "CH9300 7620 1162 3852 957",
            "CH93-0076-2011-6238-5295-7",
            "CH93    0076 2011 6238 5295 7",
            "CH93\n\n0076 2011 6238 5295 7",
            "CH93 \n \n0076 2011 6238 5295 7",
        ] {
            nichts(text, "iban");
        }
    }

    #[test]
    fn iban_leerraum_zwischen_gruppen() {
        for iban in [
            "CH93  0076 2011 6238 5295 7",
            "CH93   0076\t2011 \t6238 5295 7",
            "CH93 0076 2011\n6238 5295 7",
            "CH93 0076 2011 \n 6238 5295 7",
            "CH93\u{2009}0076\u{2007}2011\u{A0}\u{A0}6238\u{202F}5295 7",
            "CH56 0483 5012 3456\u{2028}7800 9",
        ] {
            genau(iban, "iban", &[iban]);
        }
        genau(
            "Bitte auf CH93 0076\n2011 6238 5295 7 überweisen.",
            "iban",
            &["CH93 0076\n2011 6238 5295 7"],
        );
        genau(
            "Virement sur CH56\t0483\t5012\t3456\t7800\t9, merci.",
            "iban",
            &["CH56\t0483\t5012\t3456\t7800\t9"],
        );
        genau(
            "Conto CH56 0483 5012\r\n3456 7800 9, grazie.",
            "iban",
            &["CH56 0483 5012\n3456 7800 9"],
        );
        // Mehrere Umbrüche über verschiedene Lücken: je Lücke höchstens einer.
        genau(
            "CH93\n0076\n2011 6238 5295 7",
            "iban",
            &["CH93\n0076\n2011 6238 5295 7"],
        );
        // Gegenbeispiele: Zahlenkolonnen in Vierergruppen mit falschen Prüfziffern.
        for text in [
            "CH94 0076\n2011 6238 5295 7",
            "CH56  0483  5012  3456  7800  8",
            "CH12 3456\n7890 1234\n5678 9",
        ] {
            nichts(text, "iban");
        }
    }

    #[test]
    fn iban_nicht_teil_einer_laengeren_folge() {
        for text in [
            "CH9300762011623852957X",
            "CH93 0076 2011 6238 5295 71",
            "XCH9300762011623852957",
            "1CH9300762011623852957",
            "name@CH9300762011623852957",
        ] {
            nichts(text, "iban");
        }
        genau(
            "IBAN:CH9300762011623852957.",
            "iban",
            &["CH9300762011623852957"],
        );
    }

    // --- tel ---

    #[test]
    fn tel_schreibweisen_des_bakom() {
        // Ziffer 2.2.6 TAV E.164.
        for nummer in [
            "0327 654 321",
            "032 765 43 21",
            "+41 327 634 321",
            "+41 32 765 43 21",
        ] {
            genau(nummer, "tel", &[nummer]);
        }
    }

    #[test]
    fn tel_deutsch_franzoesisch_italienisch() {
        genau(
            "Rufen Sie mich unter 032 765 43 21 an.",
            "tel",
            &["032 765 43 21"],
        );
        genau(
            "Tél. : +41 32 765 43 21, merci.",
            "tel",
            &["+41 32 765 43 21"],
        );
        genau(
            "Chiamare il numero 0327 654 321 oppure +41 327 634 321.",
            "tel",
            &["0327 654 321", "+41 327 634 321"],
        );
    }

    #[test]
    fn tel_weitere_schreibweisen() {
        for nummer in [
            "+41 (0)32 765 43 21",
            "+41 (0) 32 765 43 21",
            "+41(0)32 765 43 21",
            "0041 32 765 43 21",
            "+41327654321",
            "0041327654321",
            "0327654321",
            "(032) 765 43 21",
            "(032)7654321",
            "032-765 43 21",
            "032-7654321",
            "032 7654321",
            "032 765 4321",
            "+41 32 7654321",
            "+41 32 765 4321",
            "+41 32\u{00A0}765\u{00A0}43\u{00A0}21",
        ] {
            genau(nummer, "tel", &[nummer]);
        }
    }

    #[test]
    fn tel_bereiche() {
        assert_eq!(zusaetze("079 123 45 67", "tel"), ["Bereich Mobilnetz"]);
        assert_eq!(zusaetze("+41 76 123 45 67", "tel"), ["Bereich Mobilnetz"]);
        assert_eq!(zusaetze("044 123 45 67", "tel"), ["Bereich Festnetz"]);
        assert_eq!(
            zusaetze("058 123 45 67", "tel"),
            ["Bereich unternehmensweite Netze"]
        );
        assert_eq!(
            zusaetze("051 223 45 67", "tel"),
            ["Bereich unternehmensweite Netze"]
        );
        assert_eq!(zusaetze("074 123 45 67", "tel"), ["Bereich Funkruf"]);
    }

    #[test]
    fn tel_genau_die_kennzahlen_des_plans() {
        // Alle zweistelligen Kennzahlen 010 bis 099: erkannt werden genau die aus Ziffer
        // 5.1.3 und 5.5.3 sowie 058, 074 und 051 mit a = 2.
        for yz in 10u8..100 {
            let nummer = format!("0{yz} 223 45 67");
            let erwartet =
                FESTNETZ.contains(&yz) || MOBILNETZ.contains(&yz) || matches!(yz, 51 | 58 | 74);
            assert_eq!(!stellen(&nummer, "tel").is_empty(), erwartet, "{nummer}");
        }
        // 051 mit a = 1 ist Reserve (Ziffer 5.2.3).
        nichts("051 123 45 67", "tel");
    }

    #[test]
    fn tel_dienstnummern_werden_nicht_gemeldet() {
        for nummer in [
            "0800 123 456",
            "0840 123 456",
            "0842 123 456",
            "0844 123 456",
            "0848 123 456",
            "0900 123 456",
            "0901 123 456",
            "0906 123 456",
            "+41 800 123 456",
            "+41 848 123 456",
            "0860 32 765 43 21",
            "+41 860 32 765 43 21",
            "0869 12 345 67",
            "097 123 45 67",
            "098 123 45 67",
            "099 123 45 67",
            "117",
            "1818",
            "+423 123 45 67",
            "020 123 45 67",
            "023 123 45 67",
        ] {
            nichts(nummer, "tel");
        }
    }

    #[test]
    fn tel_nicht_teil_einer_laengeren_folge() {
        for text in [
            "032 765 43 21 98",
            "1 032 765 43 21",
            "032 765 43 210",
            "0327 654 3210",
            "032 765 43 21/22",
            "12.032 765 43 21",
            "032 765 43 21.5",
            "x032 765 43 21",
            "032 765 43 21a",
            "0327654321@example.com",
        ] {
            nichts(text, "tel");
        }
        genau("Tel. 032 765 43 21.", "tel", &["032 765 43 21"]);
        genau(
            "032 765 43 21 / 079 123 45 67",
            "tel",
            &["032 765 43 21", "079 123 45 67"],
        );
    }

    #[test]
    fn tel_typografischer_bindestrich() {
        // Wie «032-765 43 21» (Methode): mit U+2010 und dem geschützten U+2011.
        for nummer in [
            "032\u{2010}765 43 21",
            "032\u{2011}765 43 21",
            "079\u{2011}1234567",
        ] {
            genau(nummer, "tel", &[nummer]);
        }
        genau(
            "Rufen Sie 044\u{2011}123 45 67 an.",
            "tel",
            &["044\u{2011}123 45 67"],
        );
        genau(
            "Appelez le 022\u{2011}123 45 67, merci.",
            "tel",
            &["022\u{2011}123 45 67"],
        );
        genau(
            "Chiamare lo 091\u{2010}123 45 67, grazie.",
            "tel",
            &["091\u{2010}123 45 67"],
        );
        // Weiter nicht: Gedankenstrich, Bindestriche zwischen allen Gruppen, Kette.
        for text in [
            "032\u{2013}765 43 21",
            "079\u{2011}123\u{2011}45\u{2011}67",
            "032 765 43 21\u{2011}5",
            "12\u{2011}032 765 43 21",
        ] {
            nichts(text, "tel");
        }
    }

    #[test]
    fn tel_nicht_belegte_schreibweisen() {
        for text in [
            "032/765 43 21",
            "032.765.43.21",
            "079-123-45-67",
            "+41 032 765 43 21",
            "+41 0 32 765 43 21",
            "0 32 765 43 21",
            "032 76 54 321",
        ] {
            nichts(text, "tel");
        }
    }

    #[test]
    fn tel_keine_verwechslung() {
        for text in [
            "756.1234.5678.97",
            "7561234567897",
            "CH93 0076 2011 6238 5295 7",
            "CH9300762011623852957",
            "LI89 0880 0000 0123 4567 Q",
            "01.02.2024",
            "2024-03-01",
            "1.3.2024, 10:30 Uhr",
            "CHF 1'044.50",
            "CHF 0.50",
            "Fr. 044 123.45",
            "Art. 044 Abs. 2",
            "4A_044/2023",
            "1C_327/2023",
        ] {
            nichts(text, "tel");
        }
    }

    // --- mail ---

    #[test]
    fn mail_deutsch_franzoesisch_italienisch() {
        genau(
            "Schreiben Sie an marie.muster@example.com.",
            "mail",
            &["marie.muster@example.com"],
        );
        genau(
            "Écrivez à jean-luc.exemple@example.org !",
            "mail",
            &["jean-luc.exemple@example.org"],
        );
        genau(
            "Scrivere a mario_rossi+fatture@example.net, grazie.",
            "mail",
            &["mario_rossi+fatture@example.net"],
        );
    }

    #[test]
    fn mail_buchstaben_ausserhalb_ascii() {
        for adresse in [
            "jürg.müller@beispiel.example",
            "élodie@exemple.example",
            "marie@example.xn--p1ai",
        ] {
            genau(adresse, "mail", &[adresse]);
        }
    }

    #[test]
    fn mail_rollenadressen_werden_nicht_gemeldet() {
        for text in [
            "info@example.com",
            "Support@example.org",
            "POSTMASTER@example.net",
            "www@example.com",
            "abuse@example.com",
            "noc@example.com",
        ] {
            nichts(text, "mail");
        }
        // Nur der ganze lokale Teil zählt.
        genau(
            "info.marie@example.com",
            "mail",
            &["info.marie@example.com"],
        );
        genau("kontakt@example.com", "mail", &["kontakt@example.com"]);
    }

    #[test]
    fn mail_keine_adresse() {
        for text in [
            "marie@localhost",
            "marie@[192.0.2.1]",
            "react@18.3.1",
            "npm i paket@2.0.1",
            "@marie",
            "marie@",
            "marie.@example.com",
            "marie..@example.com",
            "@marie@social.example",
            "https://nutzer:geheim@example.com/pfad",
            "https://nutzer@example.com",
            "a@b@example.com",
            "marie@-example.com",
            "marie@example.-com",
        ] {
            nichts(text, "mail");
        }
    }

    #[test]
    fn mail_in_urls_und_markdown() {
        genau(
            "https://example.com/?mail=marie@example.com",
            "mail",
            &["marie@example.com"],
        );
        genau(
            "https://example.com/leute/marie@example.com",
            "mail",
            &["marie@example.com"],
        );
        genau(
            "[marie@example.com](mailto:marie@example.com)",
            "mail",
            &["marie@example.com", "marie@example.com"],
        );
        genau("<marie@example.com>", "mail", &["marie@example.com"]);
    }

    #[test]
    fn mail_raender() {
        for (text, adresse) in [
            ("(marie@example.com)", "marie@example.com"),
            ("«marie@example.com»", "marie@example.com"),
            ("Mail: marie@example.com.", "marie@example.com"),
            ("marie@example.com-", "marie@example.com"),
            ("...marie@example.com", "marie@example.com"),
            ("'marie@example.com'", "marie@example.com"),
            ("o'brien@example.com", "o'brien@example.com"),
            ("marie@sub.example.com.2024", "marie@sub.example.com"),
        ] {
            genau(text, "mail", &[adresse]);
        }
    }

    #[test]
    fn mail_laenge_des_lokalen_teils() {
        let lokal = "a".repeat(LOKAL_MAX);
        let adresse = format!("{lokal}@example.com");
        genau(&adresse, "mail", &[adresse.as_str()]);
        nichts(&format!("a{lokal}@example.com"), "mail");
    }

    /// «@» und Punkt ausgeschrieben gegen Adresssammler.
    #[test]
    fn mail_ausgeschrieben() {
        for (text, adresse) in [
            (
                "hans.muster(at)example.ch hat einen Strafbefehl erhalten.",
                "hans.muster(at)example.ch",
            ),
            (
                "hans.muster[at]example.ch hat einen Strafbefehl erhalten.",
                "hans.muster[at]example.ch",
            ),
            ("Mail an hans(at)example.ch", "hans(at)example.ch"),
            ("Mail an hans{at}example.ch", "hans{at}example.ch"),
            ("Mail an hans (at) example.ch.", "hans (at) example.ch"),
            (
                "Mail an hans [at] example [dot] ch",
                "hans [at] example [dot] ch",
            ),
            ("hans(AT)example(DOT)ch", "hans(AT)example(DOT)ch"),
            ("hans{at}example{dot}ch", "hans{at}example{dot}ch"),
            ("hans at example dot ch", "hans at example dot ch"),
            ("hans at example (dot) ch", "hans at example (dot) ch"),
            (
                "hans (at) sub.example dot ch",
                "hans (at) sub.example dot ch",
            ),
            ("marie@example(dot)ch", "marie@example(dot)ch"),
            (
                "Kontakt: erika.beispiel(at)bluewin.ch, sie ist schwanger.",
                "erika.beispiel(at)bluewin.ch",
            ),
            (
                "marie (dot) muster (at) example.ch",
                "muster (at) example.ch",
            ),
        ] {
            genau(text, "mail", &[adresse]);
        }
        for text in [
            // « at » ohne Klammern nur mit ausgeschriebenem Punkt.
            "reach him at example.com",
            "Wir treffen uns at home.",
            "hans at example.ch",
            // Rollenadresse, eine Domain ohne Punkt, kein lokaler Teil.
            "info(at)example.ch",
            "hans(at)localhost",
            "hans(at) example",
            "hans (at)",
            "(at)example.ch",
            // Andere Klammern oder Wörter, zwei Leerzeichen.
            "hans (et) example.ch",
            "hans (at example.ch",
            "hans  (at)  example.ch",
            "hans(at)2x.png",
            "hans(at)example.pdf",
            // Wortform « at » mit gross geschriebener Domain und einem lokalen Teil ohne
            // Punkt, Ziffer oder Unterstrich.
            "My client works at Example dot com.",
            "Wir treffen uns at Home dot com.",
            "I work at Example dot com since 2020.",
        ] {
            nichts(text, "mail");
        }
        for (text, adresse) in [
            ("marie at example dot ch", "marie at example dot ch"),
            (
                "marie.muster at Example dot com",
                "marie.muster at Example dot com",
            ),
            ("marie_m at Example dot com", "marie_m at Example dot com"),
            ("marie at example DOT ch", "marie at example DOT ch"),
            // Grenze in explain mail: klein geschriebene Domain im Satz.
            ("I work at example dot com.", "work at example dot com"),
            ("Setze arr(at)idx.length auf null.", "arr(at)idx.length"),
        ] {
            genau(text, "mail", &[adresse]);
        }
        // Eine Adresse wird nur einmal gemeldet.
        genau(
            "hans (at) example.ch und marie(at)example.org",
            "mail",
            &["hans (at) example.ch", "marie(at)example.org"],
        );
    }

    /// Dateinamen mit «@».
    #[test]
    fn mail_keine_dateinamen() {
        for text in [
            "Das Logo logo@2x.png fehlt, sagt Hans Muster.",
            "Bitte Scan_Vertrag@2024.pdf an Hans Muster senden.",
            "bild@2x.jpg",
            "vertrag@final.docx",
            "icon@3x.zip",
            "Icon@3X.PNG",
            "notiz@entwurf.TXT",
            "tabelle@stand.xlsx",
            "folien@kunde.pptx",
            "nachricht@weitergeleitet.msg",
            "grafik@druck.svg",
            "foto@ferien.jpeg",
            "anim@web.gif",
            "brief@alt.doc",
            // Endungen, die keine Top-Level-Domain sind.
            "Lade daten@2024.json hoch.",
            "Seite index@v2.html prüfen.",
            "alt@v1.htm",
            "export@stand.XML",
        ] {
            nichts(text, "mail");
        }
        // Vor der Endung bleibt eine Adresse eine Adresse; andere Endungen werden gemeldet.
        genau("marie@example.ch.pdf", "mail", &["marie@example.ch"]);
        genau("bericht@final.zip", "mail", &["bericht@final.zip"]);
        genau("Datei clip@2024.mov anhängen.", "mail", &["clip@2024.mov"]);
        genau("marie@example.com", "mail", &["marie@example.com"]);
        // «2x» vor einer Länderendung oder vor weiteren Labels ist eine Domain.
        genau("Mail an kontakt@2x.ch", "mail", &["kontakt@2x.ch"]);
        genau(
            "kontakt@2x.example.com",
            "mail",
            &["kontakt@2x.example.com"],
        );
    }

    /// Ein Block aus zehn Ziffern nach einem Nummernwort ist keine Telefonnummer; sonst geht
    /// keine verloren.
    #[test]
    fn tel_nicht_nach_nummernwort() {
        for text in [
            "Die Kundennummer 0441234567 von Hans Muster.",
            "Bitte CHF 1'044.50 für Rechnung 0791234567.",
            "Fasse die Rechnung 0791234567 zusammen.",
            "Kunden-Nr.: 0441234567",
            "Kunden-Nr 0441234567",
            "Kundennr. 0441234567",
            "Rechnung Nr. 0791234567",
            "Rechnungsnummer: 0791234567",
            "Referenz 0441234567",
            "Police #0441234567",
            "Policennummer: 0581234567",
            "Schadennummer 0441234567",
            "Auftrag Nr 0441234567",
            "Auftrags-Nr. 0441234567",
            "Patientennummer 0791234567",
            "Fall-Nr. 0441234567",
            "Dossier: 0441234567",
            "Référence : 0441234567",
            "Numéro de client 0441234567",
            "facture n° 0791234567",
            "N° de police 0441234567",
            "numéro de sinistre 0441234567",
            "commande no. 0441234567",
            "Numero cliente 0441234567",
            "fattura n. 0791234567",
            "Polizza 0441234567",
            "numero di sinistro 0441234567",
            "incarto 0441234567",
        ] {
            nichts(text, "tel");
        }
        for (text, nummer) in [
            ("Ruf mich an: 0791234567", "0791234567"),
            ("Kundennummer 079 123 45 67", "079 123 45 67"),
            ("Rechnung +41791234567", "+41791234567"),
            (
                "Die Nummer 0441234567 steht auf der Rechnung.",
                "0441234567",
            ),
            ("Rechnung\n0791234567", "0791234567"),
            ("Abrechnung 0791234567", "0791234567"),
            ("Kunde: 0791234567", "0791234567"),
            ("Patientin: 0791234567", "0791234567"),
            ("Rechnung vom Mai, Tel. 0791234567", "0791234567"),
            ("Rechnungen 0791234567", "0791234567"),
            // Nach «zur», «zum», «sur (la)», «sulla» nennt das blosse Wort das Thema, die
            // Nummer bleibt eine Telefonnummer.
            (
                "Meine Nummer für Rückfragen zur Rechnung: 0791234567",
                "0791234567",
            ),
            ("Bei Fragen zum Auftrag 0791234567 anrufen.", "0791234567"),
            ("Bei Fragen zur Rechnung 0791234567 anrufen.", "0791234567"),
            (
                "Pour toute question sur la facture: 0791234567",
                "0791234567",
            ),
            ("Per domande sulla fattura: 0791234567", "0791234567"),
        ] {
            genau(text, "tel", &[nummer]);
        }
        // Mit Kürzel, als Zusammensetzung oder nach anderen Präpositionen bleibt es die
        // Nummer der Sache.
        for text in [
            "Bei Fragen zur Rechnung Nr. 0791234567 anrufen.",
            "Fragen zur Rechnungsnummer 0791234567",
            "sur la facture n° 0791234567",
            "Bitte CHF 1'044.50 für Rechnung 0791234567.",
        ] {
            nichts(text, "tel");
        }
    }

    /// Fundstellen der Nummernwörter.
    #[test]
    fn nummernwoerter_fundstellen() {
        let lex = crate::lexikon::lexikon(NUMMERNWOERTER);
        let eintrag = |begriff: &str, sprache: crate::lexikon::Sprache| {
            lex.eintraege
                .iter()
                .find(|e| e.begriff == begriff && e.sprache == sprache)
                .unwrap_or_else(|| panic!("{begriff}"))
        };
        use crate::lexikon::Sprache::{De, Fr, It};
        // «Referenz» und «riferimento»: AHVV Art. 71 Abs. 3 trägt die Nummer wörtlich.
        let referenz = eintrag("Referenz", De);
        assert!(referenz.url.ends_with("/de#art_71"));
        assert!(referenz.quelle.contains("Referenznummern"));
        let riferimento = eintrag("riferimento", It);
        assert!(riferimento.url.ends_with("/it#art_71"));
        assert!(riferimento.quelle.contains("numeri di riferimento"));
        assert!(eintrag("Schadennummer", De).quelle.contains("Bedeutung 2a"));
        assert!(
            eintrag("numero cliente", It)
                .quelle
                .contains("Thesaurus (2018)")
        );
        // Die Präpositionen vor dem Thema.
        for (begriff, sprache) in [("zur", De), ("zum", De), ("sur", Fr), ("sulla", It)] {
            assert_eq!(
                eintrag(begriff, sprache).merkmal.as_deref(),
                Some("praeposition")
            );
        }
    }

    /// Mit --role lawyer kein ROT ohne Kennung.
    #[test]
    fn nummern_und_dateinamen_ohne_rot() {
        let anwalt = Optionen {
            rolle: Some(Rolle::Lawyer),
            ..Optionen::default()
        };
        for text in [
            "Bitte Scan_Vertrag@2024.pdf zusammenfassen.",
            "Fasse die Rechnung 0791234567 zusammen.",
        ] {
            let b = crate::pruefe(text, &anwalt);
            assert!(
                !b.funde.iter().any(|f| matches!(f.regel, "mail" | "tel")),
                "{text}"
            );
        }
        let b = crate::pruefe("hans.muster(at)example.ch", &anwalt);
        assert!(
            b.funde
                .iter()
                .any(|f| f.regel == "mail" && f.stufe == Stufe::Rot)
        );
        // Telefonnummern nach dem Thema bleiben ROT.
        for text in [
            "Meine Nummer für Rückfragen zur Rechnung: 0791234567",
            "Bei Fragen zum Auftrag 0791234567 anrufen.",
        ] {
            let b = crate::pruefe(text, &anwalt);
            assert!(
                b.funde
                    .iter()
                    .any(|f| f.regel == "tel" && f.stufe == Stufe::Rot),
                "{text}"
            );
        }
        // Kein ROT aus einem englischen Satz.
        for text in [
            "My client works at Example dot com.",
            "Wir treffen uns at Home dot com.",
        ] {
            let b = crate::pruefe(text, &anwalt);
            assert!(!b.funde.iter().any(|f| f.regel == "mail"), "{text}");
        }
    }

    // --- alle Regeln ---

    #[test]
    fn nur_identifikatoren() {
        let text = Text::neu(
            "756.1234.5678.97, CH93 0076 2011 6238 5295 7, 032 765 43 21, marie@example.com",
        );
        let treffer = erkenne(&text);
        let regeln: Vec<_> = treffer.iter().map(|t| t.regel).collect();
        assert_eq!(regeln, ["ahv", "iban", "tel", "mail"]);
        assert!(treffer.iter().all(|t| t.art == Art::Identifikator));
    }

    #[test]
    fn gegenprobe_ueber_pruefe() {
        let ohne = Optionen::default();
        let b = crate::pruefe("Meine AHV-Nummer ist 756.1234.5678.97.", &ohne);
        let f = b.funde.iter().find(|f| f.regel == "ahv").expect("ahv-Fund");
        assert_eq!(f.stufe, Stufe::Gelb);
        assert_eq!(f.stelle, "756.1234.5678.97");
        assert_eq!(
            f.zusatz.as_ref().map(|z| z.de.as_str()),
            Some("Prüfziffer gültig")
        );

        let anwalt = Optionen {
            rolle: Some(Rolle::Lawyer),
            ..Optionen::default()
        };
        let b = crate::pruefe("IBAN CH93 0076 2011 6238 5295 7", &anwalt);
        assert!(
            b.funde
                .iter()
                .any(|f| f.regel == "iban" && f.stufe == Stufe::Rot)
        );

        let b = crate::pruefe("Tel. 032 765 43 21, Mail marie.muster@example.com", &ohne);
        for regel in ["tel", "mail"] {
            assert!(
                b.funde
                    .iter()
                    .any(|f| f.regel == regel && f.stufe == Stufe::Gelb),
                "{regel}"
            );
        }

        let b = crate::pruefe("Fragen an info@example.com oder 0800 123 456.", &ohne);
        assert!(!b.funde.iter().any(|f| matches!(f.regel, "mail" | "tel")));
    }

    #[test]
    fn info_vollstaendig() {
        let infos = info();
        let ids: Vec<_> = infos.iter().map(|r| r.id).collect();
        assert_eq!(ids, ["ahv", "iban", "tel", "mail"]);
        for r in &infos {
            assert_eq!(r.klasse, Klasse::A);
            for t in [r.titel, r.erkennt, r.methode].iter().chain(&r.grenzen) {
                assert!(!t.de.is_empty() && !t.en.is_empty(), "{}", r.id);
            }
            assert!(r.hinweis.is_none(), "{}", r.id);
            assert!(r.grundlage.contains(&&normen::DSG_5_A), "{}", r.id);
            assert!(!r.quellen.is_empty() && !r.grenzen.is_empty(), "{}", r.id);
            for q in &r.quellen {
                assert!(
                    !q.titel.is_empty() && q.url.starts_with("https://"),
                    "{}",
                    r.id
                );
            }
            // Nur tel liest eine Wortliste: die Nummernwörter.
            let lexika: &[&str] = if r.id == "tel" {
                &[NUMMERNWOERTER]
            } else {
                &[]
            };
            assert_eq!(r.lexika, lexika, "{}", r.id);
        }
        for r in &infos[..2] {
            let Guete::Nachgewiesen(t) = r.guete else {
                panic!("{}: Güte nicht nachgewiesen", r.id);
            };
            assert!(t.de.contains("noch nicht gemessen"), "{}", r.id);
            assert!(t.en.contains("not been measured"), "{}", r.id);
        }
        for r in &infos[2..] {
            assert_eq!(r.guete, Guete::NichtGemessen, "{}", r.id);
        }
    }

    /// eCH-0044 belegt nur die Nummer ohne Trenner. Die Schreibweise mit Leerzeichen steht
    /// in keiner gelesenen amtlichen Quelle; die Methode sagt das, statt sie in die
    /// Aufzählung vor «(eCH-0044)» zu stellen.
    #[test]
    fn ahv_methode_nennt_die_quelle_je_schreibweise() {
        let m = info_ahv().methode;
        assert!(m.de.contains("mit Punkten (Rz 1202 WL VA/IK)"));
        assert!(m.de.contains(
            "ohne Trenner (eCH-0044) oder, amtlich nicht belegt, gruppiert 3-4-4-2 mit je einem Leerzeichen"
        ));
        assert!(m.en.contains("with dots (WL VA/IK no. 1202)"));
        assert!(m.en.contains(
            "without separators (eCH-0044) or, not officially documented, grouped 3-4-4-2 with one space each"
        ));
        for t in [m.de, m.en] {
            let quelle = t.find("(eCH-0044)").unwrap();
            assert!(!t[..quelle].contains("3-4-4-2"), "{t}");
        }
    }

    #[test]
    fn normen_vollstaendig() {
        for n in [
            &AHVV_133,
            &AHVV_134QUINQUIES_2,
            &AEFV_2_1,
            &AEFV_16,
            &TAV_2_2_4,
            &TAV_2_2_6,
            &TAV_5_1_3,
            &TAV_5_2_3,
            &TAV_5_3_3,
            &TAV_5_4_3,
            &TAV_5_5_3,
        ] {
            assert_eq!(n.fassung.len(), 10, "{}", n.zitat);
            assert!(
                n.url.starts_with("https://www.fedlex.admin.ch/"),
                "{}",
                n.zitat
            );
            assert!(
                !n.wortlaut.is_empty() && !n.erlass.is_empty(),
                "{}",
                n.zitat
            );
        }
    }

    #[test]
    fn kennzahlen_wie_im_wortlaut() {
        let zahlen = |wortlaut: &str| -> Vec<u8> {
            wortlaut
                .split("yz = ")
                .nth(1)
                .expect("yz = …")
                .split(|c: char| !c.is_ascii_digit())
                .filter(|t| !t.is_empty())
                .map(|t| t.parse().expect("Zahl"))
                .collect()
        };
        assert_eq!(zahlen(TAV_5_1_3.wortlaut), FESTNETZ);
        assert_eq!(zahlen(TAV_5_5_3.wortlaut), MOBILNETZ);
    }
}
