//! Das rechtliche Modell (SPEC 4): aus Treffern werden Funde mit Stufe und Grund.
//!
//! Die Regeln in Kurzform:
//!
//! * **GELB** für direkte Identifikatoren (Art. 5 lit. a DSG).
//! * **ORANGE** für besonders schützenswerte Inhalte (Art. 5 lit. c DSG), aber nur, wenn
//!   der Prompt einen direkten Identifikator enthält (4.4). Ohne Identifikator gibt es
//!   einen HINWEIS, wenn derselbe Satz eine Person bezeichnet, sonst nichts.
//! * **ROT** für alles, was GELB oder ORANGE wäre, wenn eine Geheimhaltungspflicht
//!   besteht: angegeben mit `--role` oder aus dem Text erschlossen (4.3). Schützt die
//!   Pflicht «ein Geheimnis» (Art. 320, 321 StGB, Art. 47 BankG, Art. 69 FINIG), wird
//!   auch die juristische Person ROT und zählt als Identifikator; mit `professional`
//!   (Art. 62 DSG, nur Personendaten) und ohne Pflicht zählt sie nicht, weil das DSG nur
//!   natürliche Personen schützt. Normen des DSG nennt ein ROT-Fund nur, wenn er eine
//!   natürliche Person betrifft.
//! * Kollektiv- und Kommanditgesellschaften sind keine juristischen Personen: ohne
//!   Identifikator HINWEIS, mit Identifikator GELB, im Geheimnis-Kontext ROT.
//! * `--target local` macht jeden Fund zum HINWEIS: Ohne Übermittlung kein Bekanntgeben
//!   (Art. 5 lit. e DSG).
//! * Ein Element erhält die höchste zutreffende Stufe (4.2); ein Fund, der ganz in
//!   einem höheren oder gleich hohen, längeren Fund liegt, entfällt.

use serde::Serialize;

use crate::fund::{Art, Fund, Grund, Rolle, Stufe, Treffer, Ziel, Zweisprachig};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::normen::{self, Norm};
use crate::text::Text;

/// Was der Nutzer über sich und das Ziel angibt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Optionen {
    /// `None`: nicht angegeben; dann darf der Text eine Rolle zeigen.
    /// `Some(Rolle::None)`: ausdrücklich keine Geheimhaltungspflicht.
    pub rolle: Option<Rolle>,
    pub ziel: Ziel,
}

impl Default for Optionen {
    fn default() -> Self {
        Optionen {
            rolle: None,
            ziel: Ziel::Cloud,
        }
    }
}

/// Die Geheimhaltungslage des Prompts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "herkunft", rename_all = "kebab-case")]
pub enum Lage {
    /// Keine Geheimhaltungspflicht angegeben oder erkennbar.
    Keine,
    /// Mit `--role` oder in der Konfiguration angegeben.
    Angegeben { rolle: Rolle },
    /// Aus dem Text erschlossen, an dieser Stelle.
    AusText {
        rolle: Rolle,
        stelle: String,
        zeile: usize,
        spalte: usize,
    },
}

impl Lage {
    pub fn rolle(&self) -> Option<Rolle> {
        match self {
            Lage::Keine => None,
            Lage::Angegeben { rolle } | Lage::AusText { rolle, .. } => Some(*rolle),
        }
    }
}

/// Die Normen, aus denen bei dieser Rolle ROT folgt (Tabelle in SPEC 4.3). Bei
/// [`Rolle::Offen`] die Normen, die je nach Beruf in Frage kommen: Art. 321 StGB
/// (z. B. Anwältin, Psychologe), Art. 320 StGB (Behörde), Art. 47 BankG (Bank). Das BGFA
/// nennt nur `lawyer`, wenn die Rolle angegeben oder eindeutig erschlossen ist.
/// Revisorinnen und Revisoren fallen unter Art. 321 StGB, soweit Art. 730b Abs. 2 OR sie
/// zur Verschwiegenheit verpflichtet (Rechtstexte A2 und C2).
pub fn normen_fuer(rolle: Rolle) -> Vec<&'static Norm> {
    match rolle {
        Rolle::Lawyer => vec![&normen::STGB_321_1, &normen::BGFA_13_1],
        Rolle::Auditor => vec![&normen::STGB_321_1, &normen::OR_730B_2],
        Rolle::Clergy
        | Rolle::Notary
        | Rolle::PatentAttorney
        | Rolle::Doctor
        | Rolle::Psychologist => vec![&normen::STGB_321_1],
        Rolle::Bank => vec![&normen::BANKG_47_1_A, &normen::BANKG_47_2],
        Rolle::FinancialInstitution => vec![&normen::FINIG_69_1_A, &normen::FINIG_69_3],
        // Art. 73 Abs. 1 StPO bindet nur Strafbehörden (Art. 12 f. StPO), nicht jedes Gericht.
        Rolle::Authority | Rolle::Court => vec![&normen::STGB_320_1],
        Rolle::CriminalAuthority => vec![&normen::STGB_320_1, &normen::STPO_73_1],
        Rolle::Student => vec![&normen::STGB_321_1_ABS2, &normen::DSG_62_2],
        Rolle::Professional => vec![&normen::DSG_62_1],
        Rolle::Offen => vec![
            &normen::STGB_321_1,
            &normen::STGB_320_1,
            &normen::BANKG_47_1_A,
        ],
        Rolle::None => vec![],
    }
}

/// Die Normen eines ROT-Fundes: die der Rolle, ohne die des DSG, wenn der Fund keine
/// natürliche Person betrifft. Art. 62 DSG schützt geheime Personendaten, und
/// Personendaten beziehen sich nur auf natürliche Personen (Art. 5 lit. a DSG).
pub fn normen_fuer_fund(rolle: Rolle, natuerliche_person: bool) -> Vec<&'static Norm> {
    normen_fuer(rolle)
        .into_iter()
        .filter(|n| natuerliche_person || !n.erlass.starts_with("DSG"))
        .collect()
}

/// Schützt die Pflicht dieser Rolle «ein Geheimnis», gleich wessen (Art. 320 Ziff. 1 und
/// Art. 321 Ziff. 1 StGB, Art. 47 Abs. 1 BankG, Art. 69 Abs. 1 FINIG)? Dann wird auch eine
/// juristische Person ROT und zählt als Identifikator. Art. 62 DSG (`professional`)
/// schützt nur «geheime Personendaten», also Angaben über natürliche Personen.
pub fn schuetzt_geheimnis(rolle: Rolle) -> bool {
    !matches!(rolle, Rolle::None | Rolle::Professional)
}

/// Bezeichnung der Geheimhaltungspflicht einer Rolle; englisch nach der Fedlex-Übersetzung
/// («professional confidentiality», Art. 321 StGB).
pub fn geheimnis_name(rolle: Rolle, englisch: bool) -> &'static str {
    match (rolle, englisch) {
        (Rolle::Authority | Rolle::Court | Rolle::CriminalAuthority, false) => "Amtsgeheimnis",
        (Rolle::Authority | Rolle::Court | Rolle::CriminalAuthority, true) => "official secrecy",
        (Rolle::Bank, false) => "Bankgeheimnis",
        (Rolle::Bank, true) => "banking secrecy",
        (Rolle::Professional, false) => "berufliche Schweigepflicht",
        (Rolle::Professional, true) => "professional duty of confidentiality",
        (Rolle::Offen, false) => "Geheimnis-Kontext, Beruf offen",
        (Rolle::Offen, true) => "secrecy context, profession open",
        (Rolle::None, false) => "keine",
        (Rolle::None, true) => "none",
        (_, false) => "Berufsgeheimnis",
        (_, true) => "professional confidentiality",
    }
}

/// Wofür eine der Normen bei [`Rolle::Offen`] gilt, für die Erklärungszeile.
pub fn offen_beruf(norm: &Norm, englisch: bool) -> &'static str {
    let paar = if norm.zitat == normen::STGB_321_1.zitat {
        ("z. B. Anwalt, Psychologin", "e.g. lawyer, psychologist")
    } else if norm.zitat == normen::STGB_320_1.zitat {
        ("Behörde", "authority")
    } else if norm.zitat == normen::BANKG_47_1_A.zitat {
        ("Bank", "bank")
    } else {
        ("", "")
    };
    if englisch { paar.1 } else { paar.0 }
}

/// Die Rollen, die die Ausgabe bei [`Rolle::Offen`] zur Auswahl anbietet.
pub const OFFEN_ANGEBOT: [Rolle; 6] = [
    Rolle::Lawyer,
    Rolle::Psychologist,
    Rolle::Authority,
    Rolle::Bank,
    Rolle::Professional,
    Rolle::None,
];

/// Bewertet alle Treffer eines Textes.
pub fn bewerte(text: &Text, treffer: Vec<Treffer>, opt: &Optionen) -> (Vec<Fund>, Lage) {
    let mut treffer = treffer;
    treffer.sort_by(|a, b| (a.start, a.ende, a.regel).cmp(&(b.start, b.ende, b.regel)));
    treffer.dedup_by(|a, b| a.regel == b.regel && a.start == b.start && a.ende == b.ende);
    firmenadressen(text, &mut treffer);

    let lage = lage_bestimmen(text, &treffer, opt);
    let geheim = lage.rolle();
    let aus_text = match &lage {
        Lage::AusText { stelle, .. } => Some(stelle.clone()),
        _ => None,
    };

    // Identifikator einer natürlichen Person; eine Firmenadresse zählt nach
    // `firmenadressen` schon als juristische Person.
    let natuerlich = treffer.iter().any(|t| t.art == Art::Identifikator);
    // Schützt die Pflicht «ein Geheimnis», zählt auch die Gesellschaft.
    let schuetzt = geheim.is_some_and(schuetzt_geheimnis);
    let gesellschaft = treffer
        .iter()
        .any(|t| matches!(t.art, Art::JuristischePerson | Art::Personengesellschaft));
    let identifikator = natuerlich || (schuetzt && gesellschaft);
    // Der Bezug muss eine andere Stelle sein als der Inhalt: In «Die beschuldigte Person
    // schweigt» ist die Verfahrensrolle zugleich der Begriff der Ziff. 5 und bezeichnet für
    // sich noch keinen bestimmten Fall.
    let personenbezug_im_satz = |inhalt: &Treffer| {
        treffer.iter().any(|t| {
            t.art == Art::Personenbezug
                && (t.ende <= inhalt.start || t.start >= inhalt.ende)
                && text.im_selben_satz(t.start, inhalt.start)
        })
    };
    let geheimnis = |rolle: Rolle, ziffer: Option<u8>, natuerliche_person: bool| Grund::Geheimnis {
        rolle,
        aus_text: aus_text.clone(),
        ziffer,
        natuerliche_person,
    };
    let personendaten = || match geheim {
        Some(r) => (Stufe::Rot, geheimnis(r, None, natuerlich)),
        None => (Stufe::Gelb, Grund::Personendaten),
    };
    // Nur eine Pflicht, die «ein Geheimnis» schützt, hebt die Gesellschaft auf ROT.
    let geheimnis_der_gesellschaft = |natuerliche_person: bool| {
        geheim
            .filter(|_| schuetzt)
            .map(|r| (Stufe::Rot, geheimnis(r, None, natuerliche_person)))
    };

    let mut funde = Vec::new();
    for t in &treffer {
        let bewertung = match t.art {
            Art::Identifikator => Some(personendaten()),
            Art::Personenangabe if identifikator => Some(personendaten()),
            Art::Personenangabe => Some((Stufe::Hinweis, Grund::OhneIdentifikator)),
            Art::JuristischePerson => geheimnis_der_gesellschaft(false),
            Art::Personengesellschaft if schuetzt => geheimnis_der_gesellschaft(natuerlich),
            Art::Personengesellschaft if identifikator => Some(personendaten()),
            Art::Personengesellschaft => Some((Stufe::Hinweis, Grund::OhneIdentifikator)),
            // «zugleich besonders schützenswert» nur bei einer natürlichen Person.
            Art::BesondersSchuetzenswert(z) if identifikator => Some(match geheim {
                Some(r) => (
                    Stufe::Rot,
                    geheimnis(r, natuerlich.then_some(z), natuerlich),
                ),
                None => (Stufe::Orange, Grund::BesondersSchuetzenswert { ziffer: z }),
            }),
            Art::BesondersSchuetzenswert(_) if personenbezug_im_satz(t) => {
                Some((Stufe::Hinweis, Grund::OhneIdentifikator))
            }
            Art::BesondersSchuetzenswert(_) => None,
            Art::Geheimniskontext(_) | Art::Personenbezug => None,
            Art::Hinweis => Some((Stufe::Hinweis, Grund::Regelhinweis)),
        };
        if let Some((stufe, grund)) = bewertung {
            funde.push(fund(text, t, stufe, grund));
        }
    }

    // Der erschlossene Geheimnis-Kontext erscheint, wenn er etwas auf ROT gehoben hat.
    if let Lage::AusText { rolle, .. } = &lage {
        let gehoben = funde.iter().any(|f| f.stufe == Stufe::Rot);
        let stelle = treffer
            .iter()
            .find(|t| matches!(t.art, Art::Geheimniskontext(r) if r == *rolle));
        if let (true, Some(t)) = (gehoben, stelle) {
            funde.push(fund(
                text,
                t,
                Stufe::Hinweis,
                Grund::Geheimniskontext { rolle: *rolle },
            ));
        }
    }

    if let Some(f) = ahv_sammlung(text, &funde) {
        funde.push(f);
    }

    if opt.ziel == Ziel::Local {
        for f in &mut funde {
            if f.stufe != Stufe::Hinweis {
                f.grund = Grund::Lokal { sonst: f.stufe };
                f.stufe = Stufe::Hinweis;
            }
        }
    }

    let mut funde = ohne_ueberdeckung(funde);
    funde.sort_by(|a, b| {
        (a.start, std::cmp::Reverse(a.stufe), a.ende, a.regel).cmp(&(
            b.start,
            std::cmp::Reverse(b.stufe),
            b.ende,
            b.regel,
        ))
    });
    (zusammenfassen(text, funde), lage)
}

/// Eine Adresse, die unmittelbar auf eine Firma folgt («Muster AG, Seestrasse 12, 8002
/// Zürich»), ist die Adresse dieser juristischen Person. Art. 5 lit. a DSG schützt nur
/// natürliche Personen; die Adresse zählt darum wie die Firma: ROT nur, wenn die Pflicht
/// der Rolle «ein Geheimnis» schützt (SPEC 4.3), sonst kein Fund, und sie hebt keinen
/// Inhalt auf ORANGE. Dazwischen dürfen nur ein Komma, Leerzeichen oder ein Zeilenwechsel
/// stehen; steht ein Name dazwischen («Muster AG, z. H. Erika Beispiel, …»), bleibt es die
/// Adresse einer Person. Die Adresse einer Kollektiv- oder Kommanditgesellschaft bleibt
/// ein Identifikator: Deren unbeschränkt haftende Gesellschafter sind natürliche Personen
/// (Art. 552 Abs. 1, Art. 594 Abs. 2 OR).
fn firmenadressen(text: &Text, treffer: &mut [Treffer]) {
    let roh = text.als_str();
    let firmen: Vec<usize> = treffer
        .iter()
        .filter(|t| t.art == Art::JuristischePerson)
        .map(|t| t.ende)
        .collect();
    let fuge = |z: &str| {
        z.len() <= 4
            && z.chars()
                .all(|c| matches!(c, ' ' | '\t' | ',' | '\n' | '\r'))
    };
    for t in treffer
        .iter_mut()
        .filter(|t| t.regel == "adresse" && t.art == Art::Identifikator)
    {
        let nach_firma = firmen
            .iter()
            .any(|&ende| ende <= t.start && roh.get(ende..t.start).is_some_and(fuge));
        if nach_firma {
            t.art = Art::JuristischePerson;
            t.zusatz = Some(Zweisprachig::neu("Adresse der Firma", "company address"));
        }
    }
}

/// Fasst unmittelbar benachbarte Funde derselben Regel `art5c-*` mit gleicher Stufe und
/// gleichem Grund zu einer Stelle zusammen: «arbeitsunfähig (Diagnose F32.1)» ist eine
/// Angabe, nicht drei (SPEC 3, Beispiel 1). Dazwischen darf nur Leerraum oder eines der
/// Zeichen `( [ , ; : /` stehen, ohne Zeilenwechsel; eine so geöffnete Klammer schliesst
/// die Stelle mit, wenn sie direkt danach folgt. Namen, Nummern und Adressen bleiben
/// einzeln, denn jede bezeichnet eine eigene Person oder Sache.
fn zusammenfassen(text: &Text, funde: Vec<Fund>) -> Vec<Fund> {
    let roh = text.als_str();
    let fuge = |z: &str| {
        z.chars()
            .all(|c| matches!(c, ' ' | '\t' | '(' | '[' | ',' | ';' | ':' | '/'))
    };
    let mut aus: Vec<Fund> = Vec::with_capacity(funde.len());
    for f in funde {
        if let Some(l) = aus.last_mut()
            && l.regel.starts_with("art5c-")
            && l.regel == f.regel
            && l.stufe == f.stufe
            && l.grund == f.grund
            && f.start >= l.ende
            && roh.get(l.ende..f.start).is_some_and(fuge)
        {
            if l.zusatz != f.zusatz {
                l.zusatz = None;
            }
            l.ende = f.ende;
            l.ende_zeichen = f.ende_zeichen;
            l.stelle = text.ausschnitt(l.start..l.ende).to_string();
            continue;
        }
        aus.push(f);
    }
    for f in &mut aus {
        for (auf, zu) in [('(', ')'), ('[', ']')] {
            let offen = f.stelle.matches(auf).count() > f.stelle.matches(zu).count();
            if offen && roh[f.ende..].starts_with(zu) {
                f.ende += zu.len_utf8();
                f.ende_zeichen += 1;
                f.stelle = text.ausschnitt(f.start..f.ende).to_string();
            }
        }
    }
    aus
}

/// Die Lage: angegeben, sonst der erste Geheimnis-Kontext im Text. Ein bestimmter Beruf
/// («Ich bin Anwältin») geht dem offenen Beruf («meine Klientin») vor, auch wenn er
/// später im Text steht.
fn lage_bestimmen(text: &Text, treffer: &[Treffer], opt: &Optionen) -> Lage {
    let kontext = |t: &Treffer| match t.art {
        Art::Geheimniskontext(rolle) if rolle != Rolle::None => Some(rolle),
        _ => None,
    };
    match opt.rolle {
        Some(Rolle::None) => Lage::Keine,
        Some(rolle) => Lage::Angegeben { rolle },
        None => treffer
            .iter()
            .find(|t| kontext(t).is_some_and(|r| r != Rolle::Offen))
            .or_else(|| treffer.iter().find(|t| kontext(t).is_some()))
            .and_then(|t| {
                let rolle = kontext(t)?;
                let (zeile, spalte) = text.position(t.start);
                Some(Lage::AusText {
                    rolle,
                    stelle: text.ausschnitt(t.start..t.ende).to_string(),
                    zeile,
                    spalte,
                })
            })
            .unwrap_or(Lage::Keine),
    }
}

fn fund(text: &Text, t: &Treffer, stufe: Stufe, grund: Grund) -> Fund {
    let (zeile, spalte) = text.position(t.start);
    Fund {
        regel: t.regel,
        stufe,
        start: t.start,
        ende: t.ende,
        zeile,
        spalte,
        start_zeichen: text.zeichenversatz(t.start),
        ende_zeichen: text.zeichenversatz(t.ende),
        stelle: text.ausschnitt(t.start..t.ende).to_string(),
        grund,
        zusatz: t.zusatz.clone(),
        beleg: t.beleg.clone(),
    }
}

/// Art. 153b AHVG: AHV-Nummern mit Personendaten in strukturierter Form. Gezählt werden
/// Zeilen mit einer gültigen AHV-Nummer, die einen Namen enthalten oder Zeilen einer
/// Tabelle sind; ab zwei solchen Zeilen erscheint der Hinweis. Eine Tabellenzeile trennt
/// Zellen mit «,» «;» Tab oder «|», und die AHV-Nummer steht allein in ihrer Zelle, höchstens
/// mit einer Bezeichnung davor («AHV: 756…»). Keine Zelle der Zeile ist ein Satz: Sie endet
/// nicht auf «?» oder «!» und nicht auf «.» nach mindestens drei Wörtern («756…, das ist
/// meine Nummer.», «756…, gültig?»). Sie zählt, wenn eine zweite Tabellenzeile mit
/// AHV-Nummer dieselbe Folge von Trennern hat oder wenn über ihr eine Kopfzeile mit «AHV»
/// oder «AVS» und einem Trenner steht. So zählen auch Listen, deren Namen die Regel name
/// nicht erkennt («Beispiel;Erika;756…»); der Zusatz nennt dann keine Namen.
fn ahv_sammlung(text: &Text, funde: &[Fund]) -> Option<Fund> {
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

    fn ist_trenner(c: char) -> bool {
        matches!(c, ',' | ';' | '\t' | '|')
    }

    fn mit_bezeichnung(z: &str) -> bool {
        let z = z.to_uppercase();
        z.contains("AHV") || z.contains("AVS")
    }

    /// Eine Zelle, die ein Satz ist: Sie endet auf «?» oder «!», oder auf «.» nach
    /// mindestens drei Wörtern. «Seestr.» und «Diagnose F32.1» sind keine Sätze.
    fn ist_satzzelle(zelle: &str) -> bool {
        let zelle = zelle.trim();
        let woerter = zelle
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphabetic))
            .count();
        zelle.ends_with(['?', '!']) || (zelle.ends_with('.') && woerter >= 3)
    }

    /// Die Folge der Trenner einer Zeile, wenn die AHV-Nummer bei `a..e` darin allein in
    /// ihrer Zelle steht und keine Zelle ein Satz ist.
    fn tabellenzeile(zeile: &str, a: usize, e: usize) -> Option<String> {
        let muster: String = zeile.chars().filter(|&c| ist_trenner(c)).collect();
        let vor = zeile.get(..a)?;
        let nach = zeile.get(e..)?;
        let davor = vor[vor.rfind(ist_trenner).map_or(0, |p| p + 1)..].trim();
        let danach = nach[..nach.find(ist_trenner).unwrap_or(nach.len())].trim();
        let allein = (davor.is_empty() || mit_bezeichnung(davor)) && danach.is_empty();
        let satz = zeile.split(ist_trenner).any(ist_satzzelle);
        (!muster.is_empty() && allein && !satz).then_some(muster)
    }

    let mit_name: HashSet<usize> = funde
        .iter()
        .filter(|g| g.regel == "name")
        .map(|g| g.zeile)
        .collect();
    // Je Zeile die erste AHV-Nummer.
    let mut nummern: BTreeMap<usize, &Fund> = BTreeMap::new();
    for f in funde.iter().filter(|f| f.regel == "ahv") {
        nummern.entry(f.zeile).or_insert(f);
    }
    let mut nach_muster: HashMap<String, Vec<usize>> = HashMap::new();
    for (&nr, f) in &nummern {
        let bereich = text.zeilenbereich(nr);
        let (a, e) = (
            f.start - bereich.start,
            f.ende.min(bereich.end) - bereich.start,
        );
        if let Some(muster) = tabellenzeile(text.zeile(nr), a, e) {
            nach_muster.entry(muster).or_default().push(nr);
        }
    }
    let kopf = (1..=text.zeilen_anzahl()).find(|nr| {
        let z = text.zeile(*nr);
        !nummern.contains_key(nr) && z.contains(ist_trenner) && mit_bezeichnung(z)
    });
    // Zeilen mit gemeinsamer Trennerfolge zählen alle, die übrigen unter einer Kopfzeile.
    let tabelle: BTreeSet<usize> = nach_muster
        .values()
        .flat_map(|zeilen| {
            let gemeinsam = zeilen.len() >= 2;
            zeilen
                .iter()
                .copied()
                .filter(move |&nr| gemeinsam || kopf.is_some_and(|k| nr > k))
        })
        .collect();
    let gezaehlt: Vec<usize> = nummern
        .keys()
        .copied()
        .filter(|nr| mit_name.contains(nr) || tabelle.contains(nr))
        .collect();
    if gezaehlt.len() < 2 {
        return None;
    }
    let erste = nummern[&gezaehlt[0]];
    let n = gezaehlt.len();
    let namen = gezaehlt.iter().filter(|nr| mit_name.contains(nr)).count();
    let t = Treffer::neu("ahv-sammlung", erste.start, erste.ende, Art::Hinweis);
    let mut f = fund(text, &t, Stufe::Hinweis, Grund::Regelhinweis);
    f.zusatz = Some(if namen == n {
        Zweisprachig::neu(
            format!("{n} Zeilen mit AHV-Nummer und Name"),
            format!("{n} lines with AHV number and name"),
        )
    } else if namen == 0 {
        Zweisprachig::neu(
            format!("{n} Zeilen mit AHV-Nummer in Tabellenform"),
            format!("{n} lines with AHV number in tabular form"),
        )
    } else {
        Zweisprachig::neu(
            format!("{n} Zeilen mit AHV-Nummer, {namen} davon mit Name"),
            format!("{n} lines with AHV number, {namen} of them with a name"),
        )
    });
    Some(f)
}

/// Entfernt Funde, die ganz in einem höheren oder gleich hohen, längeren Fund liegen.
/// Hinweise einer Regel und der Kontext-Hinweis sind eigene Information; sie entfallen
/// nie und verdrängen nichts.
fn ohne_ueberdeckung(funde: Vec<Fund>) -> Vec<Fund> {
    let bereiche: Vec<(usize, usize, Stufe, bool)> = funde
        .iter()
        .map(|f| {
            let eigenstaendig = matches!(
                f.grund,
                Grund::Regelhinweis | Grund::Geheimniskontext { .. }
            );
            (f.start, f.ende, f.stufe, eigenstaendig)
        })
        .collect();
    let behalten = nicht_ueberdeckt(&bereiche);
    funde
        .into_iter()
        .zip(behalten)
        .filter_map(|(f, b)| b.then_some(f))
        .collect()
}

/// Kern von [`ohne_ueberdeckung`] auf (start, ende, stufe, eigenständig), in O(n log n).
///
/// Ein Bereich f entfällt, wenn ein anderer, nicht eigenständiger Bereich g ihn enthält
/// (g.start ≤ f.start, f.ende ≤ g.ende) und g höher ist oder gleich hoch und echt länger.
/// Enthält g den Bereich f, ist g nie kürzer; «echt länger» heisst darum: anderer Bereich.
/// Durchlauf nach Anfang aufsteigend; je Stufe merkt er das grösste Ende der Bereiche mit
/// kleinerem Anfang und, getrennt, der Bereiche mit gleichem Anfang.
fn nicht_ueberdeckt(bereiche: &[(usize, usize, Stufe, bool)]) -> Vec<bool> {
    let rang = |s: Stufe| match s {
        Stufe::Hinweis => 0,
        Stufe::Gelb => 1,
        Stufe::Orange => 2,
        Stufe::Rot => 3,
    };
    let mut reihe: Vec<usize> = (0..bereiche.len()).filter(|&i| !bereiche[i].3).collect();
    // Anfang aufsteigend, Ende absteigend.
    reihe.sort_by(|&a, &b| {
        let (sa, ea, _, _) = bereiche[a];
        let (sb, eb, _, _) = bereiche[b];
        sa.cmp(&sb).then(eb.cmp(&ea))
    });
    let mut behalten = vec![true; bereiche.len()];
    // grösstes Ende je Stufe unter den Bereichen mit kleinerem Anfang
    let mut frueher: [Option<usize>; 4] = [None; 4];
    let mut i = 0;
    while i < reihe.len() {
        let anfang = bereiche[reihe[i]].0;
        let mut j = i;
        while j < reihe.len() && bereiche[reihe[j]].0 == anfang {
            j += 1;
        }
        // Gruppe mit gleichem Anfang. Für jeden Bereich zählen aus der Gruppe nur andere
        // Bereiche, die gleich lang oder länger sind; Stufe und Länge entscheiden.
        let mut gruppe_laenger: [Option<usize>; 4] = [None; 4];
        let mut k = i;
        while k < j {
            // Bereiche mit gleichem Ende innerhalb der Gruppe gemeinsam behandeln.
            let ende = bereiche[reihe[k]].1;
            let mut m = k;
            while m < j && bereiche[reihe[m]].1 == ende {
                m += 1;
            }
            let mut gleich: [bool; 4] = [false; 4];
            for &x in &reihe[k..m] {
                gleich[rang(bereiche[x].2)] = true;
            }
            for &x in &reihe[k..m] {
                let r = rang(bereiche[x].2);
                // anderer Bereich mit kleinerem Anfang, der x enthält: gleich hoch genügt
                let von_frueher = (r..4).any(|s| frueher[s].is_some_and(|e| e >= ende));
                // gleicher Anfang, echt längeres Ende: gleich hoch genügt
                let laenger = (r..4).any(|s| gruppe_laenger[s].is_some());
                // gleicher Bereich: nur eine höhere Stufe verdrängt
                let gleicher_bereich_hoeher = (r + 1..4).any(|s| gleich[s]);
                if von_frueher || laenger || gleicher_bereich_hoeher {
                    behalten[x] = false;
                }
            }
            for s in 0..4 {
                if gleich[s] {
                    gruppe_laenger[s].get_or_insert(ende);
                }
            }
            k = m;
        }
        for s in 0..4 {
            if let Some(e) = gruppe_laenger[s] {
                frueher[s] = Some(frueher[s].map_or(e, |f| f.max(e)));
            }
        }
        i = j;
    }
    behalten
}

/// Regeln, die im Modell selbst entstehen.
pub fn info() -> Vec<RegelInfo> {
    vec![RegelInfo {
        id: "ahv-sammlung",
        klasse: Klasse::Hinweis,
        titel: Txt::neu("AHV-Nummern in Tabellenform", "AHV numbers in tabular form"),
        erkennt: Txt::neu(
            "Mindestens zwei Zeilen mit je einer gültigen AHV-Nummer und einem Namen.",
            "At least two lines, each with a valid AHV number and a name.",
        ),
        methode: Txt::neu(
            "Zählt Zeilen, in denen die Regeln ahv und name beide anschlagen.",
            "Counts lines on which both the ahv and the name rule match.",
        ),
        // Rechtstext B4 «Hinweis im Bericht» und «explain, Grenzen» (Endfassung).
        hinweis: Some(Txt::neu(
            "Systematisch verwenden darf die AHV-Nummer ausserhalb der AHV nur, wer nach Art. 153c AHVG berechtigt ist.",
            "Outside the AHV, only persons authorised under Art. 153c AHVG may use the AHV number systematically.",
        )),
        grundlage: vec![&normen::AHVG_153B, &normen::AHVG_153C_1],
        quellen: vec![Quelle {
            titel: "AHVG Art. 49, 51, 153b, 153c und 153i, Fassung vom 01.01.2026",
            url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_153_b",
            lizenz: None,
            stand: Stand::Geprueft("2026-11-06"),
        }],
        lexika: vec![],
        guete: Guete::NichtGemessen,
        grenzen: vec![Txt::neu(
            "Ob die Verwendung zur AHV selbst gehört, etwa die Abrechnung des Arbeitgebers mit der Ausgleichskasse (Art. 49 AHVG, Art. 51 Abs. 3 AHVG), oder ob Sie zur systematischen Verwendung berechtigt sind (Art. 153c Abs. 1 AHVG), entscheidet promptscan nicht. Wer die AHV-Nummer ohne Berechtigung systematisch verwendet, wird mit Geldstrafe bestraft (Art. 153i Abs. 1 AHVG).",
            "Whether the use belongs to the AHV itself, such as the employer settling with the compensation fund (Art. 49 AHVG, Art. 51 Abs. 3 AHVG), or whether you are authorised to use the number systematically (Art. 153c Abs. 1 AHVG), is not decided by promptscan. Systematic use of the AHV number without authorisation is punishable by a monetary penalty (Art. 153i Abs. 1 AHVG).",
        )],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &Text, regel: &'static str, stelle: &str, art: Art) -> Treffer {
        let start = text.als_str().find(stelle).expect("Stelle im Text");
        Treffer::neu(regel, start, start + stelle.len(), art)
    }

    fn cloud(rolle: Option<Rolle>) -> Optionen {
        Optionen {
            rolle,
            ziel: Ziel::Cloud,
        }
    }

    #[test]
    fn identifikator_ohne_rolle_ist_gelb() {
        let text = Text::neu("Hans Muster ist krank.");
        let (f, lage) = bewerte(
            &text,
            vec![t(&text, "name", "Hans Muster", Art::Identifikator)],
            &cloud(None),
        );
        assert_eq!(lage, Lage::Keine);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].stufe, Stufe::Gelb);
        assert_eq!(f[0].grund, Grund::Personendaten);
        assert_eq!((f[0].zeile, f[0].spalte), (1, 1));
    }

    #[test]
    fn inhalt_mit_identifikator_ist_orange() {
        let text = Text::neu("Hans Muster ist arbeitsunfähig.");
        let tr = vec![
            t(&text, "name", "Hans Muster", Art::Identifikator),
            t(
                &text,
                "art5c-2",
                "arbeitsunfähig",
                Art::BesondersSchuetzenswert(2),
            ),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        let inhalt = f.iter().find(|f| f.regel == "art5c-2").unwrap();
        assert_eq!(inhalt.stufe, Stufe::Orange);
        assert_eq!(inhalt.grund, Grund::BesondersSchuetzenswert { ziffer: 2 });
    }

    #[test]
    fn inhalt_ohne_identifikator_und_ohne_personenbezug_entfaellt() {
        let text = Text::neu("Was gilt bei Arbeitsunfähigkeit?");
        let tr = vec![t(
            &text,
            "art5c-2",
            "Arbeitsunfähigkeit",
            Art::BesondersSchuetzenswert(2),
        )];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert!(f.is_empty());
    }

    #[test]
    fn inhalt_mit_personenbezug_im_satz_ist_hinweis() {
        let text = Text::neu("Die Beschuldigte ist arbeitsunfähig. Was nun?");
        let tr = vec![
            t(
                &text,
                "personenbezug",
                "Die Beschuldigte",
                Art::Personenbezug,
            ),
            t(
                &text,
                "art5c-2",
                "arbeitsunfähig",
                Art::BesondersSchuetzenswert(2),
            ),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].stufe, &f[0].grund),
            (Stufe::Hinweis, &Grund::OhneIdentifikator)
        );
    }

    #[test]
    fn personenbezug_im_anderen_satz_reicht_nicht() {
        let text = Text::neu("Die Beschuldigte schweigt. Was gilt bei Arbeitsunfähigkeit?");
        let tr = vec![
            t(
                &text,
                "personenbezug",
                "Die Beschuldigte",
                Art::Personenbezug,
            ),
            t(
                &text,
                "art5c-2",
                "Arbeitsunfähigkeit",
                Art::BesondersSchuetzenswert(2),
            ),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert!(f.is_empty());
    }

    #[test]
    fn adresse_nach_firma_zaehlt_wie_die_firma() {
        let text = Text::neu(
            "Muster AG, Seestrasse 12, 8002 Zürich. Beispiel GmbH, z. H. Erika Beispiel, Bahnhofstrasse 1, 3000 Bern. Krankheit.",
        );
        let tr = || {
            vec![
                t(&text, "firma", "Muster AG", Art::JuristischePerson),
                t(
                    &text,
                    "adresse",
                    "Seestrasse 12, 8002 Zürich",
                    Art::Identifikator,
                ),
                t(&text, "firma", "Beispiel GmbH", Art::JuristischePerson),
                t(&text, "name", "Erika Beispiel", Art::Identifikator),
                t(
                    &text,
                    "adresse",
                    "Bahnhofstrasse 1, 3000 Bern",
                    Art::Identifikator,
                ),
                t(
                    &text,
                    "art5c-2",
                    "Krankheit",
                    Art::BesondersSchuetzenswert(2),
                ),
            ]
        };
        let stufen = |f: &[Fund]| -> Vec<(String, Stufe)> {
            f.iter().map(|f| (f.stelle.clone(), f.stufe)).collect()
        };
        // Ohne Rolle: die Firmenadresse ergibt keinen Fund; die Adresse hinter dem Namen
        // bleibt GELB, und erst der Name hebt den Inhalt auf ORANGE.
        let (f, _) = bewerte(&text, tr(), &cloud(None));
        assert_eq!(
            stufen(&f),
            [
                ("Erika Beispiel".to_string(), Stufe::Gelb),
                ("Bahnhofstrasse 1, 3000 Bern".to_string(), Stufe::Gelb),
                ("Krankheit".to_string(), Stufe::Orange),
            ]
        );
        // Ohne den Namen hebt die Firmenadresse nichts auf ORANGE.
        let ohne_name: Vec<Treffer> = tr()
            .into_iter()
            .filter(|t| {
                t.regel != "name" && !t.start.eq(&text.als_str().find("Bahnhofstrasse").unwrap())
            })
            .collect();
        let (f, _) = bewerte(&text, ohne_name, &cloud(None));
        assert!(f.is_empty(), "{f:?}");
        // Mit Rolle: ROT wie die Firma, mit Zusatz.
        let (f, _) = bewerte(&text, tr(), &cloud(Some(Rolle::Lawyer)));
        let adresse = f
            .iter()
            .find(|f| f.stelle == "Seestrasse 12, 8002 Zürich")
            .unwrap();
        assert_eq!(adresse.stufe, Stufe::Rot);
        assert_eq!(
            adresse.zusatz.as_ref().map(|z| z.de.as_str()),
            Some("Adresse der Firma")
        );
    }

    #[test]
    fn benachbarte_inhalte_einer_ziffer_werden_eine_stelle() {
        let text = Text::neu(
            "Hans Muster, Erika Beispiel: arbeitsunfähig (Diagnose F32.1), vorbestraft\nDiagnose",
        );
        let c = |s: &str, z| {
            let regel = if z == 2 { "art5c-2" } else { "art5c-5" };
            t(&text, regel, s, Art::BesondersSchuetzenswert(z))
        };
        let zweite = text.als_str().rfind("Diagnose").unwrap();
        let tr = vec![
            t(&text, "name", "Hans Muster", Art::Identifikator),
            t(&text, "name", "Erika Beispiel", Art::Identifikator),
            c("arbeitsunfähig", 2),
            c("Diagnose", 2),
            c("F32.1", 2),
            c("vorbestraft", 5),
            Treffer::neu(
                "art5c-2",
                zweite,
                zweite + 8,
                Art::BesondersSchuetzenswert(2),
            ),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        let stellen: Vec<_> = f.iter().map(|f| f.stelle.as_str()).collect();
        // Zwei Namen bleiben zwei Funde, eine andere Ziffer und eine andere Zeile ebenso.
        assert_eq!(
            stellen,
            [
                "Hans Muster",
                "Erika Beispiel",
                "arbeitsunfähig (Diagnose F32.1)",
                "vorbestraft",
                "Diagnose"
            ]
        );
        let ganz = &f[2];
        assert_eq!(
            ganz.ende_zeichen - ganz.start_zeichen,
            ganz.stelle.chars().count()
        );
        assert_eq!(&text.als_str()[ganz.start..ganz.ende], ganz.stelle);
    }

    #[test]
    fn bezug_und_inhalt_an_derselben_stelle_reichen_nicht() {
        // «beschuldigte Person» ist Verfahrensrolle und Begriff der Ziff. 5 zugleich.
        let text =
            Text::neu("Die beschuldigte Person schweigt. Die beschuldigte Person ist vorbestraft.");
        let rolle = "beschuldigte Person";
        let zweite = text.als_str().rfind(rolle).unwrap();
        let tr = vec![
            t(&text, "personenbezug", rolle, Art::Personenbezug),
            t(&text, "art5c-5", rolle, Art::BesondersSchuetzenswert(5)),
            Treffer::neu(
                "personenbezug",
                zweite,
                zweite + rolle.len(),
                Art::Personenbezug,
            ),
            Treffer::neu(
                "art5c-5",
                zweite,
                zweite + rolle.len(),
                Art::BesondersSchuetzenswert(5),
            ),
            t(
                &text,
                "art5c-5",
                "vorbestraft",
                Art::BesondersSchuetzenswert(5),
            ),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        // Nur «vorbestraft» hat einen Bezug an anderer Stelle im selben Satz.
        let stellen: Vec<_> = f.iter().map(|f| (f.stelle.as_str(), f.stufe)).collect();
        assert_eq!(stellen, vec![("vorbestraft", Stufe::Hinweis)]);
    }

    #[test]
    fn rolle_hebt_auf_rot() {
        let text = Text::neu("Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7");
        let tr = vec![
            t(&text, "name", "Erika Beispiel", Art::Identifikator),
            t(
                &text,
                "iban",
                "CH93 0076 2011 6238 5295 7",
                Art::Identifikator,
            ),
        ];
        let (f, lage) = bewerte(&text, tr, &cloud(Some(Rolle::Lawyer)));
        assert_eq!(
            lage,
            Lage::Angegeben {
                rolle: Rolle::Lawyer
            }
        );
        assert!(f.iter().all(|f| f.stufe == Stufe::Rot));
        assert_eq!(
            f[0].grund,
            Grund::Geheimnis {
                rolle: Rolle::Lawyer,
                aus_text: None,
                ziffer: None,
                natuerliche_person: true,
            }
        );
    }

    /// E1 (recht-stufen-1): Die Firma wird nur ROT und zählt nur als Identifikator, wenn
    /// die Pflicht der Rolle «ein Geheimnis» schützt. Art. 62 DSG (professional) schützt
    /// nur geheime Personendaten.
    #[test]
    fn firma_nur_bei_einer_pflicht_fuer_ein_geheimnis() {
        let text = Text::neu("Die Muster AG ist im Strafverfahren angeklagt.");
        let tr = || {
            vec![
                t(&text, "firma", "Muster AG", Art::JuristischePerson),
                t(
                    &text,
                    "art5c-5",
                    "Strafverfahren",
                    Art::BesondersSchuetzenswert(5),
                ),
            ]
        };
        // Regressionsfall aus dem Befund: --role professional mit «Muster AG» gibt kein ROT.
        let (f, lage) = bewerte(&text, tr(), &cloud(Some(Rolle::Professional)));
        assert_eq!(
            lage,
            Lage::Angegeben {
                rolle: Rolle::Professional
            }
        );
        assert!(f.iter().all(|f| f.stufe != Stufe::Rot), "{f:?}");
        assert!(f.is_empty(), "{f:?}");
        for r in Rolle::ALLE {
            let (f, _) = bewerte(&text, tr(), &cloud(Some(r)));
            let rot = f.iter().filter(|f| f.stufe == Stufe::Rot).count();
            assert_eq!(rot, if schuetzt_geheimnis(r) { 2 } else { 0 }, "{r:?}");
        }
        assert!(!schuetzt_geheimnis(Rolle::Professional));
        assert!(!schuetzt_geheimnis(Rolle::None));
        for r in [
            Rolle::Lawyer,
            Rolle::Student,
            Rolle::Bank,
            Rolle::FinancialInstitution,
            Rolle::Authority,
            Rolle::Offen,
        ] {
            assert!(schuetzt_geheimnis(r), "{r:?}");
        }
    }

    /// E1: Art. 62 DSG nennt ein Fund nur bei einer natürlichen Person; bei student nennt
    /// die Firma nur Art. 321 Ziff. 1 Abs. 2 StGB.
    #[test]
    fn dsg_normen_nur_bei_natuerlichen_personen() {
        let text = Text::neu("Die Muster AG und Erika Beispiel.");
        let tr = vec![
            t(&text, "firma", "Muster AG", Art::JuristischePerson),
            t(&text, "name", "Erika Beispiel", Art::Identifikator),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(Some(Rolle::Student)));
        let grund = |stelle: &str| f.iter().find(|f| f.stelle == stelle).unwrap().grund.clone();
        assert_eq!(
            grund("Muster AG"),
            Grund::Geheimnis {
                rolle: Rolle::Student,
                aus_text: None,
                ziffer: None,
                natuerliche_person: false,
            }
        );
        assert_eq!(
            grund("Erika Beispiel"),
            Grund::Geheimnis {
                rolle: Rolle::Student,
                aus_text: None,
                ziffer: None,
                natuerliche_person: true,
            }
        );
        assert_eq!(
            normen_fuer_fund(Rolle::Student, false),
            vec![&normen::STGB_321_1_ABS2]
        );
        assert_eq!(
            normen_fuer_fund(Rolle::Student, true),
            vec![&normen::STGB_321_1_ABS2, &normen::DSG_62_2]
        );
        // Keine Rolle verliert ausser dem DSG eine Norm.
        for r in Rolle::ALLE {
            let ohne: Vec<_> = normen_fuer(r)
                .into_iter()
                .filter(|n| !n.erlass.starts_with("DSG"))
                .collect();
            assert_eq!(normen_fuer_fund(r, false), ohne, "{r:?}");
            assert_eq!(normen_fuer_fund(r, true), normen_fuer(r), "{r:?}");
        }
    }

    /// E1: «zugleich besonders schützenswert» nur, wenn der Prompt einen Identifikator einer
    /// natürlichen Person enthält.
    #[test]
    fn zugleich_besonders_schuetzenswert_nur_mit_natuerlicher_person() {
        let text = Text::neu("Gegen die Muster AG läuft ein Strafverfahren. Erika Beispiel.");
        let firma = t(&text, "firma", "Muster AG", Art::JuristischePerson);
        let inhalt = t(
            &text,
            "art5c-5",
            "Strafverfahren",
            Art::BesondersSchuetzenswert(5),
        );
        let name = t(&text, "name", "Erika Beispiel", Art::Identifikator);
        let grund_inhalt = |tr: Vec<Treffer>| {
            let (f, _) = bewerte(&text, tr, &cloud(Some(Rolle::Lawyer)));
            f.into_iter()
                .find(|f| f.regel == "art5c-5")
                .map(|f| (f.stufe, f.grund))
                .unwrap()
        };
        assert_eq!(
            grund_inhalt(vec![firma.clone(), inhalt.clone()]),
            (
                Stufe::Rot,
                Grund::Geheimnis {
                    rolle: Rolle::Lawyer,
                    aus_text: None,
                    ziffer: None,
                    natuerliche_person: false,
                }
            )
        );
        assert_eq!(
            grund_inhalt(vec![firma, inhalt, name]),
            (
                Stufe::Rot,
                Grund::Geheimnis {
                    rolle: Rolle::Lawyer,
                    aus_text: None,
                    ziffer: Some(5),
                    natuerliche_person: true,
                }
            )
        );
    }

    /// E2 (recht-stufen-6): Finanzinstitute nach Art. 2 Abs. 1 FINIG, Art. 69 FINIG; die
    /// Bank bleibt bei Art. 47 BankG.
    #[test]
    fn finanzinstitut_nach_art_69_finig() {
        assert_eq!(
            normen_fuer(Rolle::FinancialInstitution),
            vec![&normen::FINIG_69_1_A, &normen::FINIG_69_3]
        );
        assert_eq!(
            normen_fuer(Rolle::Bank),
            vec![&normen::BANKG_47_1_A, &normen::BANKG_47_2]
        );
        assert_eq!(
            Rolle::aus_name("financial-institution"),
            Some(Rolle::FinancialInstitution)
        );
        assert_eq!(
            geheimnis_name(Rolle::FinancialInstitution, false),
            "Berufsgeheimnis"
        );
        let text = Text::neu("Die Muster Vermögensverwaltung AG zahlt nicht.");
        let tr = vec![t(
            &text,
            "firma",
            "Muster Vermögensverwaltung AG",
            Art::JuristischePerson,
        )];
        let (f, _) = bewerte(&text, tr, &cloud(Some(Rolle::FinancialInstitution)));
        assert_eq!(f[0].stufe, Stufe::Rot);
    }

    /// E4 (recht-stufen-3): Kollektiv- und Kommanditgesellschaft wie eine Personenangabe;
    /// die Adresse danach bleibt die Adresse einer Person.
    #[test]
    fn personengesellschaft_wie_eine_personenangabe() {
        let text =
            Text::neu("Die Muster & Meier KlG, Seestrasse 12, 8002 Zürich, ist überschuldet.");
        let klg = t(
            &text,
            "firma",
            "Muster & Meier KlG",
            Art::Personengesellschaft,
        );
        let adresse = t(
            &text,
            "adresse",
            "Seestrasse 12, 8002 Zürich",
            Art::Identifikator,
        );
        let stufen = |tr: Vec<Treffer>, rolle| -> Vec<(String, Stufe)> {
            let (f, _) = bewerte(&text, tr, &cloud(rolle));
            f.into_iter().map(|f| (f.stelle, f.stufe)).collect()
        };
        // Ohne Identifikator: HINWEIS, auch mit professional.
        for rolle in [None, Some(Rolle::Professional)] {
            assert_eq!(
                stufen(vec![klg.clone()], rolle),
                [("Muster & Meier KlG".to_string(), Stufe::Hinweis)]
            );
        }
        // Mit Identifikator: GELB; die Adresse wird nicht zur Firmenadresse.
        assert_eq!(
            stufen(vec![klg.clone(), adresse.clone()], None),
            [
                ("Muster & Meier KlG".to_string(), Stufe::Gelb),
                ("Seestrasse 12, 8002 Zürich".to_string(), Stufe::Gelb),
            ]
        );
        // Im Geheimnis-Kontext ROT, auch ohne Identifikator.
        assert_eq!(
            stufen(vec![klg], Some(Rolle::Lawyer)),
            [("Muster & Meier KlG".to_string(), Stufe::Rot)]
        );
    }

    /// E5 (recht-stufen-4): «meine Klientin» lässt den Beruf offen; ein bestimmter Beruf im
    /// selben Text geht vor, auch wenn er später steht.
    #[test]
    fn bestimmter_beruf_geht_dem_offenen_vor() {
        let text = Text::neu("Meine Klientin Erika Beispiel klagt. Ich bin Anwältin.");
        let offen = t(
            &text,
            "kontext",
            "Meine Klientin",
            Art::Geheimniskontext(Rolle::Offen),
        );
        let anwaeltin = t(
            &text,
            "kontext",
            "Ich bin Anwältin",
            Art::Geheimniskontext(Rolle::Lawyer),
        );
        let name = t(&text, "name", "Erika Beispiel", Art::Identifikator);
        let (f, lage) = bewerte(
            &text,
            vec![offen.clone(), name.clone(), anwaeltin],
            &cloud(None),
        );
        assert!(
            matches!(&lage, Lage::AusText { rolle: Rolle::Lawyer, stelle, .. } if stelle == "Ich bin Anwältin"),
            "{lage:?}"
        );
        assert!(f.iter().any(|f| f.grund
            == Grund::Geheimniskontext {
                rolle: Rolle::Lawyer
            }));
        // Nur der offene Beruf: ROT bleibt, die Rolle ist «offen».
        let (f, lage) = bewerte(&text, vec![offen, name], &cloud(None));
        assert_eq!(lage.rolle(), Some(Rolle::Offen));
        let n = f.iter().find(|f| f.regel == "name").unwrap();
        assert_eq!(n.stufe, Stufe::Rot);
        assert_eq!(
            normen_fuer(Rolle::Offen),
            vec![
                &normen::STGB_321_1,
                &normen::STGB_320_1,
                &normen::BANKG_47_1_A
            ]
        );
        assert!(!normen_fuer(Rolle::Offen).contains(&&normen::BGFA_13_1));
        // «offen» ist kein Wert von --role.
        assert_eq!(Rolle::aus_name("offen"), None);
        assert!(!Rolle::ALLE.contains(&Rolle::Offen));
    }

    #[test]
    fn kontext_aus_text_hebt_und_erscheint() {
        let text = Text::neu("Meine Mandantin Erika Beispiel will das Erbe ausschlagen.");
        let tr = vec![
            t(
                &text,
                "kontext",
                "Meine Mandantin",
                Art::Geheimniskontext(Rolle::Lawyer),
            ),
            t(&text, "name", "Erika Beispiel", Art::Identifikator),
        ];
        let (f, lage) = bewerte(&text, tr.clone(), &cloud(None));
        assert!(matches!(
            lage,
            Lage::AusText {
                rolle: Rolle::Lawyer,
                ..
            }
        ));
        let name = f.iter().find(|f| f.regel == "name").unwrap();
        assert_eq!(name.stufe, Stufe::Rot);
        assert!(f.iter().any(|f| f.grund
            == Grund::Geheimniskontext {
                rolle: Rolle::Lawyer
            }));

        // Ausdrücklich keine Rolle: der Text hebt nichts.
        let (f, lage) = bewerte(&text, tr, &cloud(Some(Rolle::None)));
        assert_eq!(lage, Lage::Keine);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].stufe, Stufe::Gelb);
    }

    #[test]
    fn firma_nur_im_geheimnis_kontext() {
        let text = Text::neu("Die Muster AG zahlt nicht.");
        let tr = vec![t(&text, "firma", "Muster AG", Art::JuristischePerson)];
        let (f, _) = bewerte(&text, tr.clone(), &cloud(None));
        assert!(f.is_empty());
        let (f, _) = bewerte(&text, tr, &cloud(Some(Rolle::Bank)));
        assert_eq!(f[0].stufe, Stufe::Rot);
    }

    #[test]
    fn personenangabe_ohne_identifikator_ist_hinweis() {
        let text = Text::neu("Mein Mandant, geb. 12.03.1985, fragt.");
        let tr = vec![t(&text, "geburtsdatum", "12.03.1985", Art::Personenangabe)];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert_eq!(
            (f[0].stufe, &f[0].grund),
            (Stufe::Hinweis, &Grund::OhneIdentifikator)
        );
    }

    #[test]
    fn lokal_macht_alles_zum_hinweis() {
        let text = Text::neu("Hans Muster ist arbeitsunfähig.");
        let tr = vec![
            t(&text, "name", "Hans Muster", Art::Identifikator),
            t(
                &text,
                "art5c-2",
                "arbeitsunfähig",
                Art::BesondersSchuetzenswert(2),
            ),
        ];
        let (f, _) = bewerte(
            &text,
            tr,
            &Optionen {
                rolle: None,
                ziel: Ziel::Local,
            },
        );
        assert!(f.iter().all(|f| f.stufe == Stufe::Hinweis));
        assert!(f.iter().any(|f| f.grund
            == Grund::Lokal {
                sonst: Stufe::Orange
            }));
    }

    #[test]
    fn ueberdeckter_fund_entfaellt() {
        let text = Text::neu("Seestrasse 12, 8002 Zürich");
        let tr = vec![
            t(
                &text,
                "adresse",
                "Seestrasse 12, 8002 Zürich",
                Art::Identifikator,
            ),
            t(&text, "wohnort", "Zürich", Art::Personenangabe),
        ];
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].regel, "adresse");
    }

    #[test]
    fn ahv_tabelle_gibt_hinweis() {
        let text = Text::neu("Anna Muster 756.1234.5678.97\nBeat Beispiel 756.9217.0769.85");
        let s = text.als_str();
        let mut tr = Vec::new();
        for (name, ahv) in [
            ("Anna Muster", "756.1234.5678.97"),
            ("Beat Beispiel", "756.9217.0769.85"),
        ] {
            let a = s.find(name).unwrap();
            tr.push(Treffer::neu("name", a, a + name.len(), Art::Identifikator));
            let b = s.find(ahv).unwrap();
            tr.push(Treffer::neu("ahv", b, b + ahv.len(), Art::Identifikator));
        }
        let (f, _) = bewerte(&text, tr, &cloud(None));
        assert!(
            f.iter()
                .any(|f| f.regel == "ahv-sammlung" && f.stufe == Stufe::Hinweis)
        );
        let hinweis = f
            .iter()
            .find(|f| f.regel == "ahv-sammlung")
            .expect("Hinweis");
        assert_eq!(
            hinweis.zusatz.as_ref().map(|z| z.de.as_str()),
            Some("2 Zeilen mit AHV-Nummer und Name")
        );
        assert_eq!(hinweis.stelle, "756.1234.5678.97");
    }

    /// Treffer der Regel ahv für jede der beiden erfundenen Nummern, dazu Namen an den
    /// angegebenen Stellen.
    fn ahv_und_namen(roh: &str, namen: &[&str]) -> Vec<Treffer> {
        let mut tr = Vec::new();
        for n in ["756.1234.5678.97", "756.9876.5432.17"] {
            for (a, n) in roh.match_indices(n) {
                tr.push(Treffer::neu("ahv", a, a + n.len(), Art::Identifikator));
            }
        }
        for n in namen {
            for (a, n) in roh.match_indices(n) {
                tr.push(Treffer::neu("name", a, a + n.len(), Art::Identifikator));
            }
        }
        tr
    }

    /// Der Zusatz des Hinweises ahv-sammlung, falls er erscheint.
    fn sammlung(roh: &str, namen: &[&str]) -> Option<String> {
        let text = Text::neu(roh);
        let (f, _) = bewerte(&text, ahv_und_namen(roh, namen), &cloud(None));
        f.into_iter()
            .find(|f| f.regel == "ahv-sammlung")
            .map(|f| f.zusatz.map(|z| z.de).unwrap_or_default())
    }

    /// technik-code-4, E5: Tabellen zählen auch ohne erkannten Namen; der Zusatz behauptet
    /// dann keine Namen.
    #[test]
    fn ahv_tabelle_ohne_namen() {
        let tabellenform = Some("2 Zeilen mit AHV-Nummer in Tabellenform".to_string());
        for roh in [
            "Beispiel;Erika;756.1234.5678.97\nExempel;Eva;756.9876.5432.17\n",
            "Beispiel,Erika,756.1234.5678.97\nExempel,Eva,756.9876.5432.17\n",
            "Beispiel\tErika\t756.1234.5678.97\nExempel\tEva\t756.9876.5432.17\n",
            "Beispiel, Erika, 756.1234.5678.97\nExempel, Eva, 756.9876.5432.17\n",
            "| Beispiel | Erika | 756.1234.5678.97 |\n| Exempel | Eva | 756.9876.5432.17 |",
            "Beispiel; AHV: 756.1234.5678.97\nExempel; AHV: 756.9876.5432.17",
            // Kopfzeile mit «AHV»: die Zeilen darunter zählen auch mit verschiedenen Trennern.
            "Name;Vorname;AHV-Nr.\nBeispiel;Erika;756.1234.5678.97\nExempel;756.9876.5432.17",
            "Nom;Prénom;N° AVS\nExemple;Eva;756.1234.5678.97\nExempel;756.9876.5432.17",
        ] {
            assert_eq!(sammlung(roh, &[]), tabellenform, "«{roh}»");
        }
        // Ein Teil der Zeilen mit Namen.
        assert_eq!(
            sammlung(
                "Muster;Hans;756.1234.5678.97\nBeispiel;Erika;756.9876.5432.17",
                &["Muster;Hans"]
            ),
            Some("2 Zeilen mit AHV-Nummer, 1 davon mit Name".to_string())
        );
    }

    /// Gegenbeispiele: Fliesstext, eine einzelne Tabellenzeile, eine Nummer, die nicht allein
    /// in ihrer Zelle steht, verschiedene Trenner ohne Kopfzeile und Zeilen, in denen eine
    /// Zelle ein Satz ist (w3-namen-p11: A01, A03).
    #[test]
    fn ahv_ohne_tabelle_kein_hinweis() {
        for roh in [
            "Die AHV-Nummer 756.1234.5678.97 ist gültig.\nAuch 756.9876.5432.17 ist gültig.",
            "756.1234.5678.97 und 756.9876.5432.17",
            "Beispiel;Erika;756.1234.5678.97\nDie Nummer 756.9876.5432.17 ist gültig.",
            "Bitte prüfen, 756.1234.5678.97 stimmt, danke.\nAuch hier, 756.9876.5432.17 stimmt, danke.",
            "Beispiel;Erika;756.1234.5678.97\nExempel,Eva,756.9876.5432.17",
            "756.1234.5678.97, das ist meine Nummer.\n756.9876.5432.17, das ist die meiner Frau.",
            "Bitte prüfe die AHV-Nummern, danke.\n756.1234.5678.97, gültig?\n756.9876.5432.17, gültig?",
            "756.1234.5678.97; stimmt das?\n756.9876.5432.17; bitte prüfen!",
            "Numéro AVS; Remarque\n756.1234.5678.97; c'est mon numéro.\n756.9876.5432.17; et celui de ma femme.",
        ] {
            assert_eq!(sammlung(roh, &[]), None, "«{roh}»");
        }
        // Minimalpaare: dieselben Zeilen mit Zellen, die keine Sätze sind.
        for roh in [
            "756.1234.5678.97, meine Nummer\n756.9876.5432.17, meine Frau",
            "756.1234.5678.97, gültig\n756.9876.5432.17, gültig",
            "Beispiel;Erika;Seestr. 1;756.1234.5678.97\nExempel;Eva;Diagnose F32.1;756.9876.5432.17",
        ] {
            assert_eq!(
                sammlung(roh, &[]),
                Some("2 Zeilen mit AHV-Nummer in Tabellenform".to_string()),
                "«{roh}»"
            );
        }
    }

    /// Über die ganze Prüfung: Listen aus den Befunden praxis-de-2 und technik-code-4.
    #[test]
    fn ahv_sammlung_ueber_pruefe() {
        let zusatz = |roh: &str| {
            crate::pruefe(roh, &Optionen::default())
                .funde
                .into_iter()
                .find(|f| f.regel == "ahv-sammlung")
                .and_then(|f| f.zusatz.map(|z| z.de))
        };
        assert_eq!(
            zusatz(
                "Müller Andreas; 756.1234.5678.97; 6'200\nKeller Sabine; 756.9876.5432.17; 5'800"
            ),
            Some("2 Zeilen mit AHV-Nummer und Name".to_string())
        );
        assert_eq!(
            zusatz(
                "Vorname;Name;AHV-Nr.\nHans;Muster;756.1234.5678.97\nErika;Beispiel;756.9876.5432.17\n"
            ),
            Some("2 Zeilen mit AHV-Nummer, 1 davon mit Name".to_string())
        );
        assert_eq!(
            zusatz("Muster\tHans\t756.1234.5678.97\nBeispiel\tErika\t756.9876.5432.17\n"),
            Some("2 Zeilen mit AHV-Nummer, 1 davon mit Name".to_string())
        );
        assert_eq!(
            zusatz("Muster, Hans, 756.1234.5678.97\nBeispiel, Erika, 756.9876.5432.17\n"),
            Some("2 Zeilen mit AHV-Nummer, 1 davon mit Name".to_string())
        );
    }

    #[test]
    fn jede_rolle_ausser_none_hat_normen() {
        for r in Rolle::ALLE {
            assert_eq!(normen_fuer(r).is_empty(), r == Rolle::None, "{r:?}");
        }
    }

    /// Rechtstexte A2 und C2: Revisorinnen und Revisoren nur, soweit Art. 730b Abs. 2 OR
    /// sie zur Verschwiegenheit verpflichtet; die Norm gilt auch bei einer Firma.
    #[test]
    fn revisionsstelle_mit_art_730b_or() {
        let zitate = |v: Vec<&'static Norm>| v.iter().map(|n| n.zitat).collect::<Vec<_>>();
        assert_eq!(
            zitate(normen_fuer(Rolle::Auditor)),
            ["Art. 321 Ziff. 1 StGB", "Art. 730b Abs. 2 OR"]
        );
        assert_eq!(
            zitate(normen_fuer_fund(Rolle::Auditor, false)),
            ["Art. 321 Ziff. 1 StGB", "Art. 730b Abs. 2 OR"]
        );
        assert!(!zitate(normen_fuer(Rolle::Doctor)).contains(&"Art. 730b Abs. 2 OR"));
    }

    /// Die frühere, quadratische Fassung von `ohne_ueberdeckung` als Massstab.
    fn nicht_ueberdeckt_naiv(b: &[(usize, usize, Stufe, bool)]) -> Vec<bool> {
        b.iter()
            .enumerate()
            .map(|(i, f)| {
                f.3 || !b.iter().enumerate().any(|(j, g)| {
                    i != j
                        && !g.3
                        && g.0 <= f.0
                        && f.1 <= g.1
                        && (g.2 > f.2 || (g.2 == f.2 && g.1 - g.0 > f.1 - f.0))
                })
            })
            .collect()
    }

    #[test]
    fn ueberdeckung_wie_die_quadratische_fassung() {
        use proptest::prelude::*;
        use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
        let config = Config {
            cases: 5000,
            failure_persistence: None,
            ..Config::default()
        };
        let mut saat = [0u8; 32];
        saat[..8].copy_from_slice(&20261106u64.to_le_bytes());
        let mut laeufer =
            TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &saat));
        let stufen = [Stufe::Hinweis, Stufe::Gelb, Stufe::Orange, Stufe::Rot];
        // kleine Wertebereiche, damit gleiche Anfänge, Enden und Bereiche oft vorkommen
        let erzeuger = proptest::collection::vec(
            (
                0usize..10,
                1usize..6,
                0usize..4,
                proptest::bool::weighted(0.15),
            ),
            0..16,
        );
        laeufer
            .run(&erzeuger, |roh| {
                let b: Vec<_> = roh
                    .iter()
                    .map(|&(s, l, t, e)| (s, s + l, stufen[t], e))
                    .collect();
                prop_assert_eq!(nicht_ueberdeckt(&b), nicht_ueberdeckt_naiv(&b));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn viele_funde_in_kurzer_zeit() {
        // 20 000 gültige AHV-Nummern auf einer Zeile: früher quadratisch (Sekunden bis
        // Minuten), jetzt linear bis n log n. Die Grenze ist grosszügig für Debug-Bauten.
        let zeile = "756.1234.5678.97 ".repeat(20_000);
        let start = std::time::Instant::now();
        let b = crate::pruefe(&zeile, &Optionen::default());
        assert_eq!(b.funde.iter().filter(|f| f.regel == "ahv").count(), 20_000);
        assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
    }
}
