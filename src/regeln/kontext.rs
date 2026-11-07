//! Regeln `kontext` und `personenbezug` (SPEC 4.3 und 4.4).
//!
//! * `kontext`: Die schreibende Person zeigt eine Geheimhaltungspflicht, durch eine
//!   Selbstbezeichnung («ich bin Ärztin», «en tant qu'avocat, je», «sono avvocato») oder
//!   durch den Bezug auf die eigene Klientschaft («meine Klientin», «il mio cliente») oder
//!   Patientin («ma patiente»). Treffer: [`Art::Geheimniskontext`] mit der Rolle. Die
//!   Klientschaft und die Vertretung einer Person («Ich vertrete Herrn …», «je représente
//!   M. …») lassen den Beruf offen: Rolle [`Rolle::Offen`].
//! * `personenbezug`: Bezug in der ersten Person auf eine bestimmte Person ohne
//!   Identifikator («mein Arbeitnehmer», «mon employé», «il mio dipendente»). Treffer:
//!   [`Art::Personenbezug`]; er wird nie selbst angezeigt.
//!
//! Die Wörter stehen mit Fundstelle in `lexika/kontext.toml` (Berufe) und
//! `lexika/personenbezug.toml` (Klientschaft, Patienten, Personenbezeichnungen). Die
//! Rahmen der ersten Person sind geschlossene Listen hier im Code. Sie zählen nur im
//! selben Satz und nur an ganzen Wörtern: Zusammensetzungen («Ex-Klientin») zählen nicht,
//! ausser bei Einträgen mit `teilwort = "ende"` («Hausärztin»). Ob nach einem Rahmen eine
//! Person steht, sagt die Regel `name` (Namen und Verfahrensrollen).

use std::cell::OnceCell;
use std::sync::LazyLock;

use crate::fund::{Art, Rolle, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Eintrag, Lexikon, Sprache, Teilwort, falten};
use crate::normen::{self, Norm};
use crate::regeln::{firma, name};
use crate::stufen::normen_fuer;
use crate::text::{Text, ist_unsichtbar, ist_wortzeichen};

const KONTEXT: &str = "kontext";
const PERSONENBEZUG: &str = "personenbezug";
const DATEI_BERUFE: &str = "kontext.toml";
const DATEI_BEZUG: &str = "personenbezug.toml";
const GEPRUEFT: Stand = Stand::Geprueft("2026-11-06");

// --- Rahmen der ersten Person -----------------------------------------------------------

/// Possessivbegleiter der ersten Person.
const BESITZ_DE: &[&str] = &[
    "mein", "meine", "meinem", "meinen", "meiner", "meines", "unser", "unsere", "unserem",
    "unseren", "unserer", "unseres", "unsre", "unsrem", "unsren", "unsrer", "unsres",
];
const BESITZ_FR: &[&str] = &["mon", "ma", "mes", "notre", "nos"];
const BESITZ_IT: &[&str] = &[
    "mio", "mia", "miei", "mie", "nostro", "nostra", "nostri", "nostre",
];

/// Verneinungen heben einen Rahmen auf («Ich bin kein Anwalt», «je ne suis pas avocat»).
const VERNEINUNG_DE: &[&str] = &[
    "kein", "keine", "keinem", "keinen", "keiner", "keines", "nicht", "nie", "niemals", "nimmer",
    "weder",
];
const VERNEINUNG_FR: &[&str] = &["ne", "n'", "pas", "plus", "jamais", "point", "guère"];
const VERNEINUNG_IT: &[&str] = &["non", "mai", "né"];

/// Wörter zwischen «ich bin» oder «wir sind» und dem Beruf («ich bin auch Ärztin»). Ein
/// Unternehmen spricht von sich in der Mehrzahl: «Wir sind ein Wertpapierhaus».
const FUELL_ICH: &[&str] = &["ein", "eine", "selbst", "selber", "auch"];
const FUELL_WIR: &[&str] = &["ein", "eine", "selbst", "selber", "auch", "alle", "beide"];

const ARTIKEL_SG_FR: &[&str] = &["l'", "le", "la", "un", "une"];
/// Nach «nous sommes»: Mehrzahl, für ein Unternehmen auch die Einzahl («nous sommes une
/// direction de fonds»).
const ARTIKEL_WIR_FR: &[&str] = &["les", "des", "un", "une"];
const ARTIKEL_SG_IT: &[&str] = &["l'", "il", "lo", "la", "un", "uno", "una", "un'"];
const ARTIKEL_PL_IT: &[&str] = &["i", "gli", "le"];
/// Vor «sono», «siamo», «faccio»: Verneinung, «ci sono» und «vi sono» (es gibt), Bedingung.
const SPERRE_IT: &[&str] = &["non", "ci", "vi", "se"];
/// Nach «In qualità di medico,» oder «Come medico,»: Pronomen oder Verb der ersten Person.
const ERSTE_PERSON_IT: &[&str] = &[
    "io", "noi", "ho", "abbiamo", "vorrei", "vorremmo", "devo", "dobbiamo", "posso", "possiamo",
    "siamo",
];
/// Präpositionen zwischen dem Beruf und der Person oder Firma, die jemand vertritt oder
/// betreut («En tant qu'avocate de la société Exemple SA, je …», «In qualità di avvocato
/// della Trasporti Esempio SA devo …»).
const PARTEI_FR: &[&str] = &["de", "d'", "du", "des"];
const PARTEI_IT: &[&str] = &[
    "di", "d'", "del", "della", "dello", "dell'", "dei", "degli", "delle",
];

// --- Private Stellung statt Pflicht ------------------------------------------------------

/// Possessiv der ersten Person im Genitiv nach dem Beruf: «Vermögensverwalter meiner
/// Mutter», «… unseres Vaters».
const GENITIV_BESITZ_DE: &[&str] = &["meiner", "meines", "unserer", "unseres", "unsrer", "unsres"];
/// «de» und seine Verbindungen mit dem Artikel.
const DE_FR: &[&str] = &["de", "d'", "du", "des"];
/// «di» und seine Verbindungen mit dem Artikel.
const DI_IT: &[&str] = &[
    "di", "d'", "del", "dello", "della", "dell'", "dei", "degli", "delle",
];
/// Im Bundesrecht leitet der «procuratore generale» die Bundesanwaltschaft (Art. 9 Abs. 1
/// StBOG). Im Privatrecht hat ein «procuratore» eine Vollmacht (Art. 458 Abs. 1 OR:
/// «Procuratore è colui, che …»), und eine Generalvollmacht heisst «procura generale»
/// (Art. 535 Abs. 3 OR). Mit «di …» danach zählt der Eintrag darum nicht, ausser vor
/// einem Wort aus [`AMT_IT`]. Begriffe aus `kontext.toml`.
const VOLLMACHT_IT: &[&str] = &["procuratore generale"];
/// Nach «procuratore generale di …» bleibt das Amt: «della Confederazione», «del Cantone
/// Ticino», «della Repubblica e Cantone Ticino», «del Ministero pubblico».
const AMT_IT: &[&str] = &[
    "confederazione",
    "cantone",
    "canton",
    "repubblica",
    "ministero",
];

// --- Wortlisten -------------------------------------------------------------------------

/// Eine Wortliste, nach Sprache getrennt. [`Lexikon::finde`] behält an derselben Stelle
/// nur einen Eintrag; gleich geschriebene Wörter zweier Sprachen («patient», «partner»)
/// bleiben so je Sprache erhalten.
struct JeSprache(Vec<(Sprache, Lexikon)>);

impl JeSprache {
    fn laden(datei: &str) -> JeSprache {
        let alle = lexikon::lexikon(datei);
        JeSprache(
            [Sprache::De, Sprache::Fr, Sprache::It]
                .into_iter()
                .map(|sprache| {
                    let eintraege = alle
                        .eintraege
                        .iter()
                        .filter(|e| e.sprache == sprache)
                        .cloned()
                        .collect();
                    (sprache, Lexikon::aus_eintraegen(datei, eintraege))
                })
                .collect(),
        )
    }
}

static BERUFE: LazyLock<JeSprache> = LazyLock::new(|| JeSprache::laden(DATEI_BERUFE));
static BEZUG: LazyLock<JeSprache> = LazyLock::new(|| JeSprache::laden(DATEI_BEZUG));

/// Wert von `merkmal` für den Geheimnis-Kontext ohne bestimmten Beruf («meine Klientin»).
const OFFEN: &str = "offen";

/// Die Rolle eines Eintrags (Feld `merkmal`); `none` zählt nicht, `offen` ist
/// [`Rolle::Offen`].
fn rolle(e: &Eintrag) -> Option<Rolle> {
    match e.merkmal.as_deref() {
        Some(OFFEN) => Some(Rolle::Offen),
        m => m.and_then(Rolle::aus_name).filter(|r| *r != Rolle::None),
    }
}

// --- Erkennen ---------------------------------------------------------------------------

/// Die Personen, die die Regel `name` meldet: Namen («Herrn Peter Gisler») und
/// Verfahrensrollen ohne Namen («die beschuldigte Person», «die Klägerin»), dazu die Firmen,
/// die die Regel `firma` meldet («der Muster AG»). Erst berechnet, wenn ein Rahmen danach
/// fragt.
struct Personen<'t> {
    text: &'t Text,
    /// Anfang und Ende jeder Person der Regel `name`, nach Anfang sortiert.
    namen: OnceCell<Vec<(usize, usize)>>,
    /// Anfang und Ende jeder Firma der Regel `firma`, nach Anfang sortiert.
    firmen: OnceCell<Vec<(usize, usize)>>,
}

/// Anfang und Ende der Treffer, nach Anfang sortiert.
fn bereiche(treffer: Vec<Treffer>) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = treffer.iter().map(|t| (t.start, t.ende)).collect();
    v.sort_unstable();
    v
}

/// Das späteste Ende der Bereiche, die bei `start` beginnen.
fn ende_ab(bereiche: &[(usize, usize)], start: usize) -> Option<usize> {
    let i = bereiche.partition_point(|b| b.0 < start);
    bereiche[i..]
        .iter()
        .take_while(|b| b.0 == start)
        .map(|b| b.1)
        .max()
}

impl<'t> Personen<'t> {
    fn neu(text: &'t Text) -> Personen<'t> {
        Personen {
            text,
            namen: OnceCell::new(),
            firmen: OnceCell::new(),
        }
    }

    /// Ende der Person, die bei diesem Wort beginnt.
    fn person(&self, w: &Wort) -> Option<usize> {
        let namen = self
            .namen
            .get_or_init(|| bereiche(name::erkenne(self.text)));
        ende_ab(namen, w.start)
    }

    /// Ende der Person oder Firma, die bei diesem Wort beginnt.
    fn partei(&self, w: &Wort) -> Option<usize> {
        let firmen = self
            .firmen
            .get_or_init(|| bereiche(firma::erkenne(self.text)));
        self.person(w).max(ende_ab(firmen, w.start))
    }
}

/// Die erste Person unter den nächsten drei Wörtern, ohne Satzzeichen oder Verneinung
/// davor: ihre Stelle in `rechts` und ihr Ende. «Herrn Peter Gisler», «die beschuldigte
/// Person», «le prévenu», «il signor Marco …».
fn person_nach(
    rechts: &[Wort],
    personen: &Personen,
    verneinung: &[&str],
) -> Option<(usize, usize)> {
    for (k, w) in rechts.iter().enumerate().take(3) {
        if !w.ist_wort || verneinung.contains(&w.klein.as_str()) {
            return None;
        }
        if let Some(ende) = personen.person(w) {
            return Some((k, ende));
        }
    }
    None
}

/// Wie [`person_nach`], auch mit einer Firma («Muster AG»): Zahl der Wörter von `rechts`
/// bis zum Ende der Person oder Firma.
fn partei_danach(rechts: &[Wort], personen: &Personen, verneinung: &[&str]) -> Option<usize> {
    for w in rechts.iter().take(3) {
        if !w.ist_wort || verneinung.contains(&w.klein.as_str()) {
            return None;
        }
        if let Some(ende) = personen.partei(w) {
            return Some(rechts.partition_point(|v| v.start < ende));
        }
    }
    None
}

/// Die Person oder Firma, die jemand in seinem Beruf vertritt oder betreut, nach dem Beruf
/// und einer Präposition: «(avocate) de la société Exemple SA», «(avvocato) della
/// Trasporti Esempio SA», «(avocate) de Monsieur Jean Exemple». Zahl der Wörter nach dem
/// Beruf bis zu ihrem Ende, sonst 0.
fn vertretene_partei(
    u: &Umgebung,
    praepositionen: &[&str],
    personen: &Personen,
    verneinung: &[&str],
) -> usize {
    if !ist(u.nach(1), praepositionen) {
        return 0;
    }
    partei_danach(&u.rechts[1..], personen, verneinung).map_or(0, |n| n + 1)
}

/// Anfang des Worts, in dem ein Eintrag mit `teilwort = "ende"` endet («Hausärztin»), wenn
/// davor im Wort nur Buchstaben stehen. Mit Bindestrich («Ex-Ärztin») zählt es nicht.
fn wortanfang(s: &str, woerter: &[Wort], start: usize) -> Option<usize> {
    let i = woerter
        .partition_point(|w| w.start <= start)
        .checked_sub(1)?;
    let w = &woerter[i];
    (w.ist_wort && start < w.ende && s[w.start..start].chars().all(char::is_alphabetic))
        .then_some(w.start)
}

/// Geheimnis-Kontexte und Personenbezüge, in Textreihenfolge.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let s = text.als_str();
    let woerter = zerlegen(s);
    let personen = Personen::neu(text);
    let mut treffer = Vec::new();
    for (sprache, lex) in &BERUFE.0 {
        for lt in lex.finde(text) {
            let eintrag = &lex.eintraege[lt.eintrag];
            let start = match eintrag.teilwort {
                Teilwort::Ende => wortanfang(s, &woerter, lt.start),
                _ => Some(lt.start),
            };
            let Some(u) = start.and_then(|a| Umgebung::von(text, &woerter, a, lt.ende)) else {
                continue;
            };
            let beleg = lex.beleg(lt.eintrag);
            let berufsrolle = rolle(eintrag);
            if let (Some(rolle), Some((start, ende))) =
                (berufsrolle, selbstbezeichnung(*sprache, &u, &personen))
            {
                if privat(text, *sprache, eintrag, rolle, &u) {
                    continue;
                }
                treffer.push(
                    Treffer::neu(KONTEXT, start, ende, Art::Geheimniskontext(rolle))
                        .mit_zusatz("Selbstbezeichnung", "self-description")
                        .mit_beleg(beleg),
                );
            } else if let Some(start) = besitz(*sprache, &u) {
                // «mein Anwalt»: die schreibende Person ist Klientin, nicht Geheimnisträgerin.
                treffer.push(
                    Treffer::neu(PERSONENBEZUG, start, lt.ende, Art::Personenbezug)
                        .mit_beleg(beleg),
                );
            }
        }
    }
    for (sprache, lex) in &BEZUG.0 {
        for lt in lex.finde(text) {
            let Some(start) =
                Umgebung::von(text, &woerter, lt.start, lt.ende).and_then(|u| besitz(*sprache, &u))
            else {
                continue;
            };
            let e = &lex.eintraege[lt.eintrag];
            let beleg = lex.beleg(lt.eintrag);
            match (e.regel.as_str(), rolle(e)) {
                (KONTEXT, Some(rolle)) => {
                    let (de, en) = zusatz_bezug(rolle);
                    treffer.push(
                        Treffer::neu(KONTEXT, start, lt.ende, Art::Geheimniskontext(rolle))
                            .mit_zusatz(de, en)
                            .mit_beleg(beleg.clone()),
                    );
                    // «meine Klientin» bezeichnet zugleich eine bestimmte andere Person; eine
                    // Selbstbezeichnung («als Ärztin») tut das nicht und bleibt ohne Bezug.
                    treffer.push(
                        Treffer::neu(PERSONENBEZUG, start, lt.ende, Art::Personenbezug)
                            .mit_beleg(beleg),
                    );
                }
                (PERSONENBEZUG, _) => treffer.push(
                    Treffer::neu(PERSONENBEZUG, start, lt.ende, Art::Personenbezug)
                        .mit_beleg(beleg),
                ),
                _ => continue,
            }
        }
    }
    treffer.extend(vertretung(text, &woerter, &personen));
    treffer.sort_by_key(|t| (t.start, t.ende));
    treffer
}

fn zusatz_bezug(rolle: Rolle) -> (&'static str, &'static str) {
    match rolle {
        Rolle::Doctor | Rolle::Psychologist => {
            ("eigene Patientin oder eigener Patient", "own patient")
        }
        _ => ("eigene Klientschaft", "own client"),
    }
}

/// Die Selbstbezeichnung zeigt eine private Stellung statt der Pflicht ihrer Rolle:
///
/// * Ein Finanzinstitut mit Possessiv der ersten Person im Genitiv danach verwaltet für
///   eine verbundene Person («Vermögensverwalter meiner Mutter», «gestionnaire de fortune
///   de ma mère», «gestore patrimoniale di mia madre»). Wer ausschliesslich Vermögenswerte
///   wirtschaftlich oder familiär verbundener Personen verwaltet, untersteht dem FINIG
///   nicht (Art. 2 Abs. 2 lit. a FINIG). Folgt dem Possessiv ein Wort der Kundschaft
///   ([`kundschaft_bei`]: «Vermögensverwalter meiner Kundin»), bleibt die Rolle.
/// * Ein Eintrag aus [`VOLLMACHT_IT`] mit «di …» danach ist eine Vollmacht des
///   Privatrechts, ausser vor einem Wort aus [`AMT_IT`] («procuratore generale della
///   Confederazione»).
fn privat(text: &Text, sprache: Sprache, e: &Eintrag, rolle: Rolle, u: &Umgebung) -> bool {
    let besitz = match sprache {
        Sprache::De => ist(u.nach(1), GENITIV_BESITZ_DE).then_some(2),
        Sprache::Fr => (ist1(u.nach(1), "de") && ist(u.nach(2), BESITZ_FR)).then_some(3),
        Sprache::It => (ist(u.nach(1), DI_IT) && ist(u.nach(2), BESITZ_IT)).then_some(3),
    };
    let familie = rolle == Rolle::FinancialInstitution
        && besitz.is_some_and(|n| {
            // Im Deutschen darf ein Attribut dazwischen stehen («meiner langjährigen Kundin»).
            let n = if sprache == Sprache::De && u.nach(n).is_some_and(attribut_de) {
                n + 1
            } else {
                n
            };
            !kundschaft_bei(text, u.nach(n))
        });
    let vollmacht = sprache == Sprache::It
        && VOLLMACHT_IT.contains(&e.begriff.as_str())
        && ist(u.nach(1), DI_IT)
        && !ist(u.nach(2), AMT_IT);
    familie || vollmacht
}

/// Begriff in `personenbezug.toml` für die Kundschaft eines Gewerbes.
const KUNDE: &str = "Kunde";

/// Beginnt bei `w` ein Wort der Klientschaft oder der Patienten (Regel `kontext` in
/// `personenbezug.toml`: «Klientin», «clients», «mandante») oder der Kundschaft
/// ([`KUNDE`]: «Kundin»)?
fn kundschaft_bei(text: &Text, w: Option<&Wort>) -> bool {
    let Some(w) = w else {
        return false;
    };
    BEZUG.0.iter().any(|(_, lex)| {
        lex.finde(text).iter().any(|lt| {
            let e = &lex.eintraege[lt.eintrag];
            lt.start == w.start && (e.regel == KONTEXT || e.begriff == KUNDE)
        })
    })
}

/// Zwischen dem Verb und der Person: «vertreten» meint die Stellvertretung im Beruf («Ich
/// vertrete meine Kollegin Anna Roth», «Je représente ma collègue …», «Rappresento la
/// collega …»).
const KOLLEGEN: &[&str] = &[
    "kollege",
    "kollegin",
    "kollegen",
    "kolleginnen",
    "arbeitskollege",
    "arbeitskollegin",
    "arbeitskollegen",
    "arbeitskolleginnen",
    "collègue",
    "collègues",
    "collega",
    "colleghi",
    "colleghe",
];

/// Unmittelbar nach der Person: eine Stellvertretung auf Zeit («… Anna Roth während ihrer
/// Ferien», «… bis Ende Mai», «… pendant son congé», «… jusqu'au 30 mai», «… durante le
/// sue vacanze», «… fino a maggio»). Ausnahmen in [`BIS_VOR`].
const AUF_ZEIT: &[&str] = &["während", "bis", "pendant", "jusqu'", "durante", "fino"];

/// Nach einem Wort aus [`AUF_ZEIT`] nennt eine Ortspräposition die Instanz, nicht die Zeit:
/// «Ich vertrete Herrn … bis vor Bundesgericht», «Rappresento la signora … fino davanti al
/// Tribunale federale» zählen. Im Französischen heisst es «jusque devant», ohne Elision und
/// darum ohne Wort aus [`AUF_ZEIT`].
const BIS_VOR: &[(&str, &str)] = &[("bis", "vor"), ("fino", "davanti")];

/// «représenter» und «rappresentare» heissen auch darstellen. Ein Bildwort vor dem Verb im
/// selben Satz verwirft die Vertretung («Dans ce tableau, je représente …», «Nel mio quadro
/// rappresento …»), ausser mit «de» oder «di» danach («Dans le tableau de distribution,
/// je représente …», «Nel quadro della procedura rappresento …»).
const BILD_FR: &[&str] = &[
    "tableau",
    "tableaux",
    "peinture",
    "peintures",
    "photo",
    "photos",
    "portrait",
    "portraits",
];
const BILD_IT: &[&str] = &[
    "quadro",
    "quadri",
    "dipinto",
    "dipinti",
    "ritratto",
    "ritratti",
    "foto",
    "fotografia",
    "fotografie",
];

/// Ein Wort aus `bild` unter `woerter`, ohne «de» oder «di» unmittelbar danach.
fn bild_davor(woerter: &[Wort], bild: &[&str]) -> bool {
    woerter.iter().enumerate().any(|(j, w)| {
        bild.contains(&w.klein.as_str())
            && !woerter.get(j + 1).is_some_and(|n| {
                DE_FR.contains(&n.klein.as_str()) || DI_IT.contains(&n.klein.as_str())
            })
    })
}

/// Die Vertretung einer bestimmten Person in der ersten Person: «Ich vertrete Herrn …»,
/// «Wir vertreten die beschuldigte Person», «Unsere Kanzlei vertritt Frau …», «Je
/// représente M. …», «Nous représentons …», «(Io) rappresento il signor …», «Rappresentiamo
/// …». Eine Person muss unmittelbar folgen (höchstens zwei Wörter dazwischen, etwa Artikel
/// und Anrede). Vertreten kann jede dazu ermächtigte Person (Art. 32 Abs. 1 OR), etwa ein
/// Beistand (Art. 394 Abs. 1 ZGB): Der Beruf bleibt offen ([`Rolle::Offen`]). Keine
/// Vertretung im Sinn dieser Regel sind die Stellvertretung ([`KOLLEGEN`], [`AUF_ZEIT`])
/// und das Darstellen ([`BILD_FR`], [`BILD_IT`]).
fn vertretung(text: &Text, woerter: &[Wort], personen: &Personen) -> Vec<Treffer> {
    const VERBEN: &[&str] = &[
        "vertrete",
        "vertreten",
        "vertritt",
        "représente",
        "représentons",
        "rappresento",
        "rappresentiamo",
    ];
    let mut aus = Vec::new();
    for (i, verb) in woerter.iter().enumerate() {
        if !VERBEN.contains(&verb.klein.as_str()) {
            continue;
        }
        let satz = text.satz_bei(verb.start);
        let anfang = woerter.partition_point(|w| w.start < satz.start);
        let schluss = woerter.partition_point(|w| w.start < satz.end);
        let links = &woerter[anfang..i];
        let rechts = &woerter[i + 1..schluss];
        let davor = |n: usize| links.len().checked_sub(n).map(|k| &links[k]);
        let (rahmen_start, ab, verneinung, bild): (usize, usize, &[&str], &[&str]) =
            match verb.klein.as_str() {
                "vertrete" | "vertreten" => {
                    let pronomen = if verb.klein == "vertrete" {
                        "ich"
                    } else {
                        "wir"
                    };
                    if let Some(p) = davor(1).filter(|w| w.klein == pronomen) {
                        (p.start, 0, VERNEINUNG_DE, &[])
                    } else if rechts.first().is_some_and(|w| w.klein == pronomen)
                        && davor(1).is_some_and(|w| w.ist_wort)
                        && !rechts.last().is_some_and(|w| w.klein == "?")
                    {
                        // Umgestellt hinter einem Satzglied: «Seit 2020 vertrete ich Herrn …».
                        (verb.start, 1, VERNEINUNG_DE, &[])
                    } else {
                        continue;
                    }
                }
                "vertritt" => {
                    let kanzlei = davor(1).is_some_and(|w| w.klein == "kanzlei");
                    match davor(2).filter(|w| {
                        kanzlei && matches!(w.klein.as_str(), "unsere" | "unsre" | "meine")
                    }) {
                        Some(b) => (b.start, 0, VERNEINUNG_DE, &[]),
                        None => continue,
                    }
                }
                "représente" | "représentons" => {
                    let pronomen = if verb.klein == "représente" {
                        "je"
                    } else {
                        "nous"
                    };
                    match davor(1).filter(|w| w.klein == pronomen) {
                        Some(p) => (p.start, 0, VERNEINUNG_FR, BILD_FR),
                        None => continue,
                    }
                }
                "rappresento" | "rappresentiamo" => {
                    let pronomen = if verb.klein == "rappresento" {
                        "io"
                    } else {
                        "noi"
                    };
                    if davor(1).is_some_and(|w| SPERRE_IT.contains(&w.klein.as_str())) {
                        continue;
                    }
                    match davor(1).filter(|w| w.klein == pronomen) {
                        Some(p) => (p.start, 0, VERNEINUNG_IT, BILD_IT),
                        None => (verb.start, 0, VERNEINUNG_IT, BILD_IT),
                    }
                }
                _ => continue,
            };
        let Some((k, person_ende)) = person_nach(&rechts[ab..], personen, verneinung) else {
            continue;
        };
        // Stellvertretung im Beruf: «meine Kollegin Anna Roth».
        if rechts[ab..=ab + k]
            .iter()
            .any(|w| KOLLEGEN.contains(&w.klein.as_str()))
        {
            continue;
        }
        // Stellvertretung auf Zeit: «Anna Roth während ihrer Ferien», nicht «bis vor
        // Bundesgericht» ([`BIS_VOR`]).
        let nach_person = rechts.iter().position(|w| w.start >= person_ende);
        let wort_nach = |n: usize| nach_person.and_then(|p| rechts.get(p + n));
        let instanz = BIS_VOR
            .iter()
            .any(|&(zeit, ort)| ist1(wort_nach(0), zeit) && ist1(wort_nach(1), ort));
        if ist(wort_nach(0), AUF_ZEIT) && !instanz {
            continue;
        }
        // Darstellen: «Dans ce tableau, je représente …».
        if bild_davor(links, bild) {
            continue;
        }
        let ende = if ab == 1 { rechts[0].ende } else { verb.ende };
        aus.push(
            Treffer::neu(
                KONTEXT,
                rahmen_start,
                ende,
                Art::Geheimniskontext(Rolle::Offen),
            )
            .mit_zusatz("Vertretung einer Person", "representing a person"),
        );
    }
    aus
}

// --- Wörter und Umgebung ----------------------------------------------------------------

/// Ein Wort oder Satzzeichen des Textes.
#[derive(Debug)]
struct Wort {
    start: usize,
    ende: usize,
    /// Gefaltet und klein: «qu'», «ärztin».
    klein: String,
    /// Buchstaben oder Ziffern; sonst ein Satzzeichen.
    ist_wort: bool,
    /// Beginnt mit einem Grossbuchstaben.
    gross: bool,
}

impl Wort {
    fn neu(s: &str, start: usize, ende: usize, ist_wort: bool) -> Wort {
        let roh = &s[start..ende];
        Wort {
            start,
            ende,
            klein: falten(roh, true).text,
            ist_wort,
            gross: roh.chars().next().is_some_and(char::is_uppercase),
        }
    }
}

fn ist_apostroph(c: char) -> bool {
    matches!(
        c,
        '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}' | '\u{2032}' | '\u{00B4}' | '`'
    )
}

fn ist_bindestrich(c: char) -> bool {
    matches!(c, '-' | '\u{2010}' | '\u{2011}')
}

/// Zerlegt in Wörter und Satzzeichen. Ein Bindestrich zwischen Wortzeichen gehört zum Wort
/// («Ex-Klientin», «sage-femme»). Ein Apostroph vor einem Buchstaben beendet das Wort und
/// gehört dazu (Elision: «qu'», «l'», «un'»). Unsichtbare Formatzeichen
/// ([`ist_unsichtbar`]) trennen nichts: Zwischen Wörtern zählen sie wie Leerraum
/// («Meine\u{200B} Mandantin»), im Wort gehören sie dazu («Mandan\u{AD}tin»).
fn zerlegen(s: &str) -> Vec<Wort> {
    let mut woerter = Vec::new();
    let mut zeichen = s.char_indices().peekable();
    while let Some((start, c)) = zeichen.next() {
        if c.is_whitespace() || ist_unsichtbar(c) {
            continue;
        }
        let mut ende = start + c.len_utf8();
        if !ist_wortzeichen(c) {
            woerter.push(Wort::neu(s, start, ende, false));
            continue;
        }
        while let Some(&(i, d)) = zeichen.peek() {
            let danach = s[i + d.len_utf8()..].chars().find(|&e| !ist_unsichtbar(e));
            let verbindet = ist_bindestrich(d) || ist_unsichtbar(d);
            let weiter = ist_wortzeichen(d) || (verbindet && danach.is_some_and(ist_wortzeichen));
            let elision = ist_apostroph(d) && danach.is_some_and(char::is_alphabetic);
            if !weiter && !elision {
                break;
            }
            ende = i + d.len_utf8();
            zeichen.next();
            if elision {
                break;
            }
        }
        woerter.push(Wort::neu(s, start, ende, true));
    }
    woerter
}

/// Ein Treffer mit seinen Nachbarwörtern im selben Satz.
struct Umgebung<'a> {
    text: &'a Text,
    links: &'a [Wort],
    kern: &'a [Wort],
    rechts: &'a [Wort],
}

impl<'a> Umgebung<'a> {
    /// `None`, wenn der Treffer nicht genau auf Wortgrenzen liegt («Patienten-Dossier»).
    fn von(text: &'a Text, woerter: &'a [Wort], start: usize, ende: usize) -> Option<Umgebung<'a>> {
        let i = woerter.binary_search_by_key(&start, |w| w.start).ok()?;
        let j = i + woerter[i..]
            .iter()
            .take_while(|w| w.ende <= ende)
            .position(|w| w.ende == ende)?;
        let satz = text.satz_bei(start);
        let a = woerter.partition_point(|w| w.start < satz.start);
        let b = woerter.partition_point(|w| w.start < satz.end);
        Some(Umgebung {
            text,
            links: woerter.get(a..i).unwrap_or_default(),
            kern: &woerter[i..=j],
            rechts: woerter.get(j + 1..b).unwrap_or_default(),
        })
    }

    /// Das `n`-te Wort vor dem Treffer, 1 = unmittelbar davor.
    fn vor(&self, n: usize) -> Option<&'a Wort> {
        let links = self.links;
        links.len().checked_sub(n).and_then(|k| links.get(k))
    }

    /// Das `n`-te Wort nach dem Treffer, 1 = unmittelbar danach.
    fn nach(&self, n: usize) -> Option<&'a Wort> {
        let rechts = self.rechts;
        n.checked_sub(1).and_then(|k| rechts.get(k))
    }

    /// Vor dem `n`-ten Wort vor dem Treffer steht im Satz kein Wort, höchstens Satzzeichen.
    /// Nach einem weichen Umbruch beginnt die Zeile wie ein Satz, etwa nach einer Anrede
    /// ohne Satzzeichen: «Guten Tag⏎Als Anwältin …» ([`Text::satzanfang_bei`]).
    fn satzanfang_vor(&self, n: usize) -> bool {
        self.vor(n)
            .is_some_and(|w| self.text.satzanfang_bei(w.start))
    }

    /// Der Satz endet mit einem Fragezeichen, und zwar im Satzstück des Treffers: In
    /// «Seit 2010 bin ich Ärztin⏎Darf ich …?» fragt nur die zweite Zeile
    /// ([`Text::satzstueck_bei`]).
    fn frage(&self) -> bool {
        let Some(erstes) = self.kern.first() else {
            return false;
        };
        let ende = self.text.satzstueck_bei(erstes.start).end;
        self.rechts
            .iter()
            .take_while(|w| w.start < ende)
            .last()
            .is_some_and(|w| w.klein == "?")
    }

    fn ende(&self) -> usize {
        self.kern.last().map_or(0, |w| w.ende)
    }

    /// Italienische Mehrzahl an der Endung -i des ersten Worts («avvocati», «medici»).
    fn mehrzahl_it(&self) -> bool {
        self.kern.first().is_some_and(|w| w.klein.ends_with('i'))
    }
}

fn ist(w: Option<&Wort>, liste: &[&str]) -> bool {
    w.is_some_and(|w| liste.contains(&w.klein.as_str()))
}

fn ist1(w: Option<&Wort>, wort: &str) -> bool {
    w.is_some_and(|w| w.klein == wort)
}

fn ist_ich_wir(w: &Wort) -> bool {
    matches!(w.klein.as_str(), "ich" | "wir")
}

/// Ein Wort, das Verb oder Attribut sein kann, aber keine Verneinung ist.
fn ohne_verneinung(w: &Wort, verneinung: &[&str]) -> bool {
    w.ist_wort && !verneinung.contains(&w.klein.as_str())
}

/// Attributives Adjektiv vor einem Nomen («meine neue Klientin»): klein geschrieben und mit
/// einer Endung der Adjektivdeklination (-e, -en, -er, -es, -em).
fn attribut_de(w: &Wort) -> bool {
    ohne_verneinung(w, VERNEINUNG_DE)
        && !w.gross
        && ["e", "en", "er", "es", "em"]
            .iter()
            .any(|endung| w.klein.ends_with(endung))
}

// --- Rahmen -----------------------------------------------------------------------------

/// Beginn des Possessivbegleiters der ersten Person unmittelbar vor dem Treffer; im
/// Deutschen darf ein Attribut dazwischen stehen.
fn besitz(sprache: Sprache, u: &Umgebung) -> Option<usize> {
    let liste = match sprache {
        Sprache::De => BESITZ_DE,
        Sprache::Fr => BESITZ_FR,
        Sprache::It => BESITZ_IT,
    };
    let ist_besitz = |w: &&Wort| liste.contains(&w.klein.as_str());
    if let Some(w) = u.vor(1).filter(ist_besitz) {
        return Some(w.start);
    }
    if sprache == Sprache::De && u.vor(1).is_some_and(attribut_de) {
        return u.vor(2).filter(ist_besitz).map(|w| w.start);
    }
    None
}

/// Selbstbezeichnung mit dem Beruf des Treffers: Bereich vom Rahmen bis zum Beruf, bei
/// nachgestelltem Pronomen bis zu diesem.
fn selbstbezeichnung(
    sprache: Sprache,
    u: &Umgebung,
    personen: &Personen,
) -> Option<(usize, usize)> {
    match sprache {
        Sprache::De => selbst_de(u, personen),
        Sprache::Fr => selbst_fr(u, personen),
        Sprache::It => selbst_it(u, personen),
    }
}

/// «ich bin (auch) Ärztin», «Seit 2010 bin ich Ärztin», «wir sind Ärzte», «ich als
/// Anwältin», «ich arbeite als Anwältin», «Als Anwältin habe ich …», «Als Rechtsanwältin
/// von Frau … bitte ich …».
fn selbst_de(u: &Umgebung, personen: &Personen) -> Option<(usize, usize)> {
    let ende = u.ende();
    for (pronomen, verb, fuell) in [("ich", "bin", FUELL_ICH), ("wir", "sind", FUELL_WIR)] {
        let k = if ist(u.vor(1), fuell) { 2 } else { 1 };
        if let (Some(a), Some(b)) = (u.vor(k + 1), u.vor(k)) {
            let gerade = a.klein == pronomen && b.klein == verb;
            // Umgestellt nur hinter einem Satzglied und nicht in einer Frage
            // («Bin ich Arzt, wenn …», «Ab wann bin ich Arzt?»).
            let umgestellt = a.klein == verb
                && b.klein == pronomen
                && u.vor(k + 2).is_some_and(|w| w.ist_wort)
                && !u.frage();
            if gerade || umgestellt {
                return Some((a.start, ende));
            }
        }
    }
    let k = if u.vor(1).is_some_and(attribut_de) {
        2
    } else {
        1
    };
    let als = u.vor(k).filter(|w| w.klein == "als")?;
    if let Some(p) = u.vor(k + 1).filter(|w| ist_ich_wir(w)) {
        return Some((p.start, ende));
    }
    let verb = u
        .vor(k + 1)
        .is_some_and(|w| ohne_verneinung(w, VERNEINUNG_DE));
    if let Some(p) = u.vor(k + 2).filter(|w| verb && ist_ich_wir(w)) {
        return Some((p.start, ende));
    }
    if !u.satzanfang_vor(k) {
        return None;
    }
    // «Als Rechtsanwältin von Frau Anna Roth bitte ich …», «Als Anwältin der beschuldigten
    // Person habe ich …», «Als Anwältin der Muster AG habe ich …»: eine Person oder Firma
    // nach «von», «der» oder «des», und «ich» oder «wir» später im selben Satz; «Als Anwalt
    // von Herrn X hat er …» zählt nicht.
    if ist(u.nach(1), &["von", "der", "des"])
        && partei_danach(&u.rechts[1..], personen, VERNEINUNG_DE).is_some()
        && u.rechts.iter().any(ist_ich_wir)
    {
        return Some((als.start, ende));
    }
    let r = if ist1(u.nach(1), ",") { 2 } else { 1 };
    let verb = u.nach(r).is_some_and(|w| ohne_verneinung(w, VERNEINUNG_DE));
    let p = u.nach(r + 1).filter(|w| verb && ist_ich_wir(w))?;
    Some((als.start, p.ende))
}

/// «je suis (l')avocate», «nous sommes médecins», «en ma qualité de notaire»,
/// «je travaille comme infirmière», «En tant qu'avocate, je …», «En qualité de greffière,
/// je …», «En tant qu'avocate de la société Exemple SA, je …».
fn selbst_fr(u: &Umgebung, personen: &Personen) -> Option<(usize, usize)> {
    let ende = u.ende();
    for (pronomen, verb, artikel) in [
        ("je", "suis", ARTIKEL_SG_FR),
        ("nous", "sommes", ARTIKEL_WIR_FR),
    ] {
        let k = if ist(u.vor(1), artikel) { 2 } else { 1 };
        // Nicht nach «si»: «Si je suis avocat, …».
        let rahmen = ist1(u.vor(k), verb) && !ist1(u.vor(k + 2), "si");
        if let Some(p) = u.vor(k + 1).filter(|w| rahmen && w.klein == pronomen) {
            return Some((p.start, ende));
        }
    }
    let qualitaet = ist(u.vor(1), &["de", "d'"])
        && ist1(u.vor(2), "qualité")
        && ist(u.vor(3), &["ma", "notre"]);
    if let Some(en) = u.vor(4).filter(|w| qualitaet && w.klein == "en") {
        return Some((en.start, ende));
    }
    // «en qualité de» ohne Possessiv wie «en tant que»: nur mit «je» oder «nous» davor oder
    // danach («Il est intervenu en qualité d'avocat» zählt nicht).
    let tant_que = ist(u.vor(1), &["que", "qu'"]) && ist1(u.vor(2), "tant");
    let qualite_de = ist(u.vor(1), &["de", "d'"]) && ist1(u.vor(2), "qualité");
    let m = if ist1(u.vor(1), "comme") {
        1
    } else if (tant_que || qualite_de) && ist1(u.vor(3), "en") {
        3
    } else {
        return None;
    };
    if let Some(p) = pronomen_vor(u, m + 1, Sprache::Fr) {
        return Some((p.start, ende));
    }
    if !u.satzanfang_vor(m) {
        return None;
    }
    let o = vertretene_partei(u, PARTEI_FR, personen, VERNEINUNG_FR);
    let r = o + if ist1(u.nach(o + 1), ",") { 2 } else { 1 };
    let p = u
        .nach(r)
        .filter(|w| matches!(w.klein.as_str(), "je" | "j'" | "nous"))?;
    Some((u.vor(m)?.start, p.ende))
}

/// «(io) sono (un')avvocata», «(noi) siamo medici», «siamo una direzione del fondo»,
/// «faccio il notaio», «nella mia qualità di giudice», «io lavoro come infermiera», «In
/// qualità di medico, vorrei …», «In qualità di avvocato della Trasporti Esempio SA devo
/// …».
fn selbst_it(u: &Umgebung, personen: &Personen) -> Option<(usize, usize)> {
    let ende = u.ende();
    let mehrzahl = u.mehrzahl_it();
    let sg = ist(u.vor(1), ARTIKEL_SG_IT);
    let pl = ist(u.vor(1), ARTIKEL_PL_IT);
    let k = if sg || pl { 2 } else { 1 };
    if let Some(verb) = u.vor(k) {
        let (passt, pronomen) = match verb.klein.as_str() {
            // «sono» ist auch 3. Person Mehrzahl («sono avvocati esperti»): nur mit Einzahl.
            "sono" => (!mehrzahl && !pl, "io"),
            // Mehrzahl, für ein Unternehmen auch die Einzahl («siamo una società di
            // intermediazione mobiliare»), wie «wir sind ein» und «nous sommes une».
            "siamo" => (true, "noi"),
            // «faccio il medico»: den Beruf ausüben, nur mit Artikel.
            "faccio" => (sg && !mehrzahl, "io"),
            "facciamo" => (pl, "noi"),
            _ => (false, ""),
        };
        let davor = u.vor(k + 1);
        let mit_pronomen = davor.filter(|w| w.klein == pronomen);
        // «sono» ohne «io» nur am Satz- oder Teilsatzanfang: «Mi chiamo Anna e sono
        // avvocata», nicht «Questi sono l'avvocato e il notaio». Nach einem weichen
        // Umbruch beginnt die Zeile wie ein Satz: «Buongiorno⏎Sono avvocata».
        let anfang = mit_pronomen.is_some()
            || verb.klein != "sono"
            || davor.is_none_or(|w| !w.ist_wort || matches!(w.klein.as_str(), "e" | "ed" | "ma"))
            || u.satzanfang_vor(k);
        if passt && anfang && !ist(davor, SPERRE_IT) {
            return Some((mit_pronomen.map_or(verb.start, |w| w.start), ende));
        }
    }
    let qualita = ist(u.vor(1), &["di", "d'"])
        && ist1(u.vor(2), "qualità")
        && ist(u.vor(3), &["mia", "nostra"]);
    if let Some(nella) = u.vor(4).filter(|w| qualita && w.klein == "nella") {
        return Some((nella.start, ende));
    }
    let m = if ist1(u.vor(1), "come") {
        1
    } else if ist(u.vor(1), &["di", "d'"]) && ist1(u.vor(2), "qualità") && ist1(u.vor(3), "in") {
        3
    } else {
        return None;
    };
    if let Some(p) = pronomen_vor(u, m + 1, Sprache::It) {
        return Some((p.start, ende));
    }
    if !u.satzanfang_vor(m) {
        return None;
    }
    let o = vertretene_partei(u, PARTEI_IT, personen, VERNEINUNG_IT);
    let mut r = o + if ist1(u.nach(o + 1), ",") { 2 } else { 1 };
    if ist1(u.nach(r), "non") {
        r += 1;
    }
    // Italienisch lässt das Pronomen weg; ein rückbezügliches «mi» oder «ci» vor einem
    // Verb der ersten Person trägt die Person: «Come medico mi chiedo», «ci domandiamo».
    // Allein ein Wort auf -o genügt nicht («come medico generico»).
    let reflexiv = |w: &Wort| {
        let endung = match w.klein.as_str() {
            "mi" => "o",
            "ci" => "iamo",
            _ => return None,
        };
        u.nach(r + 1)
            .filter(|v| v.ist_wort && v.klein.len() > endung.len() + 1 && v.klein.ends_with(endung))
    };
    let p = match u.nach(r) {
        Some(w) if ERSTE_PERSON_IT.contains(&w.klein.as_str()) => w,
        Some(w) if w.klein == "sono" && !mehrzahl => w,
        Some(w) => reflexiv(w)?,
        None => return None,
    };
    Some((u.vor(m)?.start, p.ende))
}

/// Subjektpronomen der ersten Person an Stelle `n` vor dem Treffer oder an `n + 1` mit
/// einem Wort oder Komma dazwischen («je travaille comme», «nous, en tant que»). «nous»
/// und «noi» sind auch Objekt («pour nous», «per noi»): Sie zählen nur am Satzanfang oder
/// nach einem Komma, und ein Verb dazwischen muss auf -ons oder -iamo enden.
fn pronomen_vor<'a>(u: &Umgebung<'a>, n: usize, sprache: Sprache) -> Option<&'a Wort> {
    let (ich, wir, endung, verneinung): (&[&str], &str, &str, &[&str]) = match sprache {
        Sprache::Fr => (&["je", "j'"], "nous", "ons", VERNEINUNG_FR),
        Sprache::It => (&["io"], "noi", "iamo", VERNEINUNG_IT),
        Sprache::De => return None,
    };
    let subjekt = |i: usize| {
        u.vor(i).filter(|w| {
            ich.contains(&w.klein.as_str())
                || (w.klein == wir && (u.satzanfang_vor(i) || ist1(u.vor(i + 1), ",")))
        })
    };
    if let Some(p) = subjekt(n) {
        return Some(p);
    }
    let luecke = u.vor(n)?;
    let passt = luecke.klein == "," || ohne_verneinung(luecke, verneinung);
    let p = subjekt(n + 1).filter(|_| passt)?;
    let verb_passt = p.klein != wir || !luecke.ist_wort || luecke.klein.ends_with(endung);
    verb_passt.then_some(p)
}

// --- Beschreibung -----------------------------------------------------------------------

/// Die Rollen, die die Wortlisten zeigen können, in der Reihenfolge von [`Rolle::ALLE`].
fn rollen() -> Vec<Rolle> {
    let gefunden: Vec<Rolle> = [DATEI_BERUFE, DATEI_BEZUG]
        .into_iter()
        .flat_map(|d| lexikon::lexikon(d).eintraege.iter())
        .filter(|e| e.regel == KONTEXT)
        .filter_map(rolle)
        .collect();
    // Der offene Beruf zuletzt; er ist kein Wert von --role.
    Rolle::ALLE
        .into_iter()
        .chain([Rolle::Offen])
        .filter(|r| gefunden.contains(r))
        .collect()
}

/// Die Normen dieser Rollen nach SPEC 4.3 ([`normen_fuer`]), ohne Doppel.
fn grundlage_kontext() -> Vec<&'static Norm> {
    let mut liste: Vec<&'static Norm> = Vec::new();
    for norm in rollen().into_iter().flat_map(normen_fuer) {
        if !liste.contains(&norm) {
            liste.push(norm);
        }
    }
    liste
}

const Q_STGB: Quelle = Quelle {
    titel: "Strafgesetzbuch (StGB), SR 311.0, Art. 320 und 321, Fassungen DE, FR, IT vom 1.11.2026",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_321",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_BGFA: Quelle = Quelle {
    titel: "Anwaltsgesetz (BGFA), SR 935.61, Art. 13 Abs. 1, Fassungen DE, FR, IT vom 1.7.2025",
    url: "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_13",
    lizenz: None,
    stand: GEPRUEFT,
};
const Q_STPO: Quelle = Quelle {
    titel: "Strafprozessordnung (StPO), SR 312.0, Art. 8, 73, 311 und 335, Fassungen DE, FR, IT vom 1.4.2025",
    url: "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_73",
    lizenz: None,
    stand: GEPRUEFT,
};

/// Die Regeln dieses Moduls für `rules` und `explain`.
pub fn info() -> Vec<RegelInfo> {
    vec![
        RegelInfo {
            id: KONTEXT,
            klasse: Klasse::Kontext,
            titel: Txt::neu("Geheimnis-Kontext im Text", "Secrecy context in the text"),
            erkennt: Txt::neu(
                "Die schreibende Person gibt sich als Trägerin eines Berufs- oder Amtsgeheimnisses zu erkennen: durch eine Selbstbezeichnung («ich bin Ärztin», «Als Anwältin habe ich», «Als Rechtsanwältin von Frau Anna Roth bitte ich», «en tant qu'avocat, je», «sono avvocato»), durch den Bezug auf die eigene Klientschaft oder Patientin («meine Klientin», «ma patiente», «il mio cliente») oder durch die Vertretung einer Person («Ich vertrete Herrn Hans Muster», «Je représente Monsieur Jean Exemple», «Rappresento il signor Marco Esempio»). Klientschaft und Vertretung zeigen den Beruf nicht: Er bleibt offen.",
                "The writer reveals a professional or official secrecy duty: by a self-description («ich bin Ärztin», «Als Anwältin habe ich», «Als Rechtsanwältin von Frau Anna Roth bitte ich», «en tant qu'avocat, je», «sono avvocato»), by referring to their own client or patient («meine Klientin», «ma patiente», «il mio cliente») or by representing a person («Ich vertrete Herrn Hans Muster», «Je représente Monsieur Jean Exemple», «Rappresento il signor Marco Esempio»). Clients and representation do not show the profession: it stays open.",
            ),
            methode: Txt::neu(
                "Aho-Corasick-Suche, je Sprache getrennt, nach den Berufsbezeichnungen aus Art. 320 und 321 StGB, Art. 13 BGFA, der StPO, Art. 2 FINIG und weiteren Bundeserlassen (StBOG, RVOG, GesBG, KVV, OR), einzelne aus Duden und Treccani (kontext.toml), und nach «Klient», «Klientschaft», «Mandant» und «Patient» samt den amtlichen französischen und italienischen Wörtern aus BGFA, GwG, OR und MedBG (personenbezug.toml). «Arzt» zählt auch als Wortende («Hausärztin»), «Tierarzt» nicht. Ein Beruf zählt nur in einem Rahmen der ersten Person im selben Satz: «ich bin», «wir sind», «ich (arbeite) als», «Als … habe ich», «Als … von, der oder des <Person oder Firma> … ich», «je suis», «nous sommes», «(je travaille) comme», «en tant que», «en (ma) qualité de», «(io) sono», «(noi) siamo», «faccio il», «come», «in qualità di», «nella mia qualità di». Nach «en tant que», «en qualité de», «comme», «in qualità di» und «come» am Satzanfang darf zwischen Beruf und Pronomen oder Verb die Person oder Firma stehen, die jemand vertritt oder betreut, angeschlossen mit «de» oder «di» («En tant qu'avocate de la société Exemple SA, je», «In qualità di avvocato della Trasporti Esempio SA devo»); als Person oder Firma zählt, was die Regeln name und firma melden. Ein weicher Zeilenumbruch, also ein einzelner Umbruch mitten im Absatz, trennt den Satz nicht («Ich bin⏎Ärztin»); ein gross geschriebenes Wort am Anfang der nächsten Zeile zählt trotzdem als Satzanfang, und eine Frage dort gehört nicht zum Rahmen davor («Guten Tag⏎Als Anwältin habe ich», «Seit 2010 bin ich Ärztin⏎Darf ich …?»). Unsichtbare Formatzeichen wie U+00AD und U+200B werden übergangen. Ein Unternehmen spricht von sich in der Mehrzahl, den Beruf in der Einzahl: «Wir sind ein Wertpapierhaus», «Nous sommes une maison de titres», «Siamo una società di intermediazione mobiliare». Klientschaft und Patienten zählen nur mit Possessivbegleiter der ersten Person («meine», «ma», «il mio»). Eine Vertretung zählt mit «ich vertrete», «wir vertreten», «unsere Kanzlei vertritt», «je représente», «nous représentons», «(io) rappresento» und «rappresentiamo», wenn unter den nächsten drei Wörtern eine Person beginnt, die die Regel name meldet (Name oder Verfahrensrolle). Sie zählt nicht mit «Kollegin», «Kollege», «Arbeitskollegin», «Arbeitskollege», «collègue» oder «collega» (auch in der Mehrzahl) vor der Person, nicht mit «während», «bis» (ausser «bis vor»), «pendant», «jusqu'», «durante» oder «fino» (ausser «fino davanti») unmittelbar nach ihr und nicht nach einem Bildwort im selben Satz («tableau», «peinture», «photo», «portrait», «quadro», «dipinto», «ritratto», «foto», «fotografia», auch in der Mehrzahl), ausser vor «de» oder «di». Ein Finanzinstitut zählt nicht vor einem Possessiv der ersten Person im Genitiv («meiner», «de ma», «di mia»), ausser vor einem Wort der Kundschaft («meiner Kundin», «de mes clients», «dei miei clienti»). «procuratore generale» zählt nicht vor «di …», ausser vor «Confederazione», «Cantone», «Canton», «Repubblica» oder «Ministero». Verneinung, Bedingung («si», «se»), «ci sono», Mehrzahl nach «sono» und Rahmen der dritten Person zählen nicht. Die Rolle steht im Feld merkmal des Eintrags; «offen» bei Klientschaft und bei jeder Vertretung.",
                "Aho-Corasick search, per language, for the profession names of Art. 320 and 321 StGB, Art. 13 BGFA, the StPO, Art. 2 FINIG and further federal acts (StBOG, RVOG, GesBG, KVV, OR), a few from Duden and Treccani (kontext.toml), and for «Klient», «Klientschaft», «Mandant» and «Patient» with the official French and Italian words from BGFA, GwG, OR and MedBG (personenbezug.toml). «Arzt» also counts as the end of a word («Hausärztin»), «Tierarzt» does not. A profession counts only inside a first-person frame in the same sentence: «ich bin», «wir sind», «ich (arbeite) als», «Als … habe ich», «Als … von, der or des <person or company> … ich», «je suis», «nous sommes», «(je travaille) comme», «en tant que», «en (ma) qualité de», «(io) sono», «(noi) siamo», «faccio il», «come», «in qualità di», «nella mia qualità di». After «en tant que», «en qualité de», «comme», «in qualità di» and «come» at the start of a sentence, the person or company that someone represents or looks after may stand between the profession and the pronoun or verb, joined with «de» or «di» («En tant qu'avocate de la société Exemple SA, je», «In qualità di avvocato della Trasporti Esempio SA devo»); a person or company is what the rules name and firma report. A soft line break, a single break within a paragraph, does not split the sentence («Ich bin⏎Ärztin»); a capitalised word at the start of the next line still counts as the start of a sentence, and a question there does not belong to the frame before it («Guten Tag⏎Als Anwältin habe ich», «Seit 2010 bin ich Ärztin⏎Darf ich …?»). Invisible format characters such as U+00AD and U+200B are skipped. A company speaks of itself in the plural, with the profession in the singular: «Wir sind ein Wertpapierhaus», «Nous sommes une maison de titres», «Siamo una società di intermediazione mobiliare». Clients and patients count only with a first-person possessive («meine», «ma», «il mio»). Representation counts with «ich vertrete», «wir vertreten», «unsere Kanzlei vertritt», «je représente», «nous représentons», «(io) rappresento» and «rappresentiamo» when a person reported by the rule name (a name or a procedural role) starts within the next three words. It does not count with «Kollegin», «Kollege», «Arbeitskollegin», «Arbeitskollege», «collègue» or «collega» (plurals too) before the person, with «während», «bis» (except «bis vor»), «pendant», «jusqu'», «durante» or «fino» (except «fino davanti») right after the person, or after a picture word in the same sentence («tableau», «peinture», «photo», «portrait», «quadro», «dipinto», «ritratto», «foto», «fotografia», plurals too), unless «de» or «di» follows it. A financial institution does not count before a first-person possessive in the genitive («meiner», «de ma», «di mia»), unless a word for customers follows («meiner Kundin», «de mes clients», «dei miei clienti»). «procuratore generale» does not count before «di …», except before «Confederazione», «Cantone», «Canton», «Repubblica» or «Ministero». Negation, conditions («si», «se»), «ci sono», plural after «sono» and third-person frames do not count. The role is the entry's merkmal field; «offen» for clients and for any representation.",
            ),
            hinweis: None,
            grundlage: grundlage_kontext(),
            quellen: vec![
                Q_STGB,
                Q_BGFA,
                Quelle {
                    titel: "Geldwäschereigesetz (GwG), SR 955.0, Art. 18a Abs. 3, Fassungen DE, FR, IT vom 1.10.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/1998/892_892_892/de#art_18_a",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Medizinalberufegesetz (MedBG), SR 811.11, Art. 2 Abs. 1 lit. e und Art. 40 lit. c und f, Fassungen DE, FR, IT vom 1.7.2025",
                    url: "https://www.fedlex.admin.ch/eli/cc/2007/537/de#art_40",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Gesundheitsberufegesetz (GesBG), SR 811.21, Art. 2 Abs. 1 lit. a, Fassung DE vom 1.7.2025: «Pflegefachfrau», «Pflegefachmann»",
                    url: "https://www.fedlex.admin.ch/eli/cc/2020/16/de#art_2",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Verordnung über die Krankenversicherung (KVV), SR 832.102, Art. 50 und 50c, Fassungen DE, FR, IT vom 22.10.2026: Psychotherapie, «psychiatre»",
                    url: "https://www.fedlex.admin.ch/eli/cc/1995/3867_3867_3867/de#art_50_c",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Finanzinstitutsgesetz (FINIG), SR 954.1, Art. 2 Abs. 1 lit. a–e und Abs. 2 lit. a, d und j, Fassungen DE, FR, IT vom 1.10.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/2018/801/de#art_2",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Strafbehördenorganisationsgesetz (StBOG), SR 173.71, Art. 9 Abs. 1 und Art. 14 lit. a, Fassungen DE und IT vom 1.1.2024",
                    url: "https://www.fedlex.admin.ch/eli/cc/2010/444/de#art_9",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Regierungs- und Verwaltungsorganisationsgesetz (RVOG), SR 172.010, Art. 31 Abs. 2, Fassung IT vom 1.5.2025: «vicecancelliere»",
                    url: "https://www.fedlex.admin.ch/eli/cc/1997/2022_2022_2022/it#art_31",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 635a, Fassung FR vom 1.10.2026: «réviseur»",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/fr#art_635_a",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 458 Abs. 1 und Art. 535 Abs. 3, Fassung IT vom 1.10.2026: «Procuratore è colui, che …», «conferire una procura generale»",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/it#art_458",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Psychologieberufegesetz (PsyG), SR 935.81, Art. 27 lit. c, Fassungen DE, FR, IT vom 1.7.2025",
                    url: "https://www.fedlex.admin.ch/eli/cc/2012/268/de#art_27",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Q_STPO,
                Quelle {
                    titel: "Bankengesetz (BankG), SR 952.0, Art. 47 Abs. 1 lit. a, Fassungen DE, FR, IT vom 1.10.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/51/117_121_129/de#art_47",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Bankengesetz (BankG), SR 952.0, Art. 4bis Abs. 1, Fassungen FR und IT vom 1.10.2026: «une banque accorde à un client», «concessi da una banca a singoli clienti»",
                    url: "https://www.fedlex.admin.ch/eli/cc/51/117_121_129/fr#art_4_bis",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 32 Abs. 1, Fassung DE vom 1.10.2026: «Wenn jemand, der zur Vertretung eines andern ermächtigt ist, in dessen Namen einen Vertrag abschliesst»",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_32",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Zivilgesetzbuch (ZGB), SR 210, Art. 394 Abs. 1, Fassung DE vom 1.7.2026: «Eine Vertretungsbeistandschaft wird errichtet, wenn die hilfsbedürftige Person bestimmte Angelegenheiten nicht erledigen kann und deshalb vertreten werden muss.»",
                    url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_394",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 40b, Fassungen DE und IT vom 1.10.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/it#art_40_b",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 398 Abs. 2, Fassungen FR und IT vom 1.10.2026: «mandant», «mandante»",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/fr#art_398",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Mandant», abgerufen am 6.11.2026: «Klient (besonders einer Rechtsanwältin oder eines Rechtsanwalts), Kunde, Auftraggeber»",
                    url: "https://www.duden.de/rechtschreibung/Mandant",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Krankenschwester», abgerufen am 6.11.2026: «weibliche Fachkraft für Krankenpflege (Berufsbezeichnung)»",
                    url: "https://www.duden.de/rechtschreibung/Krankenschwester",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Psychiater», abgerufen am 6.11.2026: «Facharzt für Psychiatrie»",
                    url: "https://www.duden.de/rechtschreibung/Psychiater",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Fürsprecher», abgerufen am 6.11.2026, Bedeutung 1: «männliche Person, die durch ihre Fürsprache jemandes Interessen, Wünsche vertritt», Bedeutung 2: «Rechtsanwalt», Gebrauch schweizerisch",
                    url: "https://www.duden.de/rechtschreibung/Fuersprecher",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Revisor», abgerufen am 6.11.2026, Bedeutung 2: «Korrektor, dem die Überprüfung der letzten Korrekturen im druckfertigen Bogen obliegt», Gebrauch Druckwesen",
                    url: "https://www.duden.de/rechtschreibung/Revisor",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Larousse, «réviseur, réviseuse», abgerufen am 6.11.2026, Bedeutung 2: «Professionnel chargé de réviser les épreuves typographiques»",
                    url: "https://www.larousse.fr/dictionnaires/francais/r%C3%A9viseur/69134",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Treccani, Vocabolario, «revisore», abgerufen am 6.11.2026: «Chi per ufficio o per incarico è addetto a un’opera di revisione: r. dei conti, delle tasse; r. di bozze di stampa»",
                    url: "https://www.treccani.it/vocabolario/revisore/",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Duden online, «Advokat», abgerufen am 6.11.2026: «[Rechts]anwalt»",
                    url: "https://www.duden.de/rechtschreibung/Advokat",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Treccani, Vocabolario, «psichiatra», abgerufen am 6.11.2026: «Medico specialista in psichiatria»",
                    url: "https://www.treccani.it/vocabolario/psichiatra/",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
            ],
            lexika: vec![DATEI_BERUFE, DATEI_BEZUG],
            guete: Guete::NichtGemessen,
            grenzen: vec![
                Txt::neu(
                    "Massgebend ist --role. Der Text zeigt die Rolle nur, wenn --role fehlt, und dient als Hilfe; --role none schaltet diese Erkennung ab (SPEC 4.3).",
                    "--role takes precedence. The text supplies the role only when --role is missing and serves as an aid; --role none switches this inference off (SPEC 4.3).",
                ),
                Txt::neu(
                    "Ob eine Angabe geheim ist und ob Einwilligung, Entbindung oder Bewilligung vorliegen (Art. 321 Ziff. 2 und Art. 320 Ziff. 2 StGB), entscheidet promptscan nicht.",
                    "promptscan does not decide whether information is secret or whether consent, a release or an authorisation exists (Art. 321 Ziff. 2 and Art. 320 Ziff. 2 StGB).",
                ),
                Txt::neu(
                    "Klientschaft und Vertretung zeigen den Beruf nicht: «Klient», «Mandant», «client», «mandant», «cliente» und «mandante» stehen für Anwältinnen und Anwälte (Art. 13 Abs. 1 BGFA), für die Psychotherapie (Art. 27 lit. c PsyG), für die Kundschaft einer Bank (Art. 4bis Abs. 1 BankG, französisch «client», italienisch «clienti») und für die Kundschaft jedes Gewerbes (Art. 40b OR, italienisch «cliente»); vertreten kann jede dazu ermächtigte Person (Art. 32 Abs. 1 OR), etwa ein Beistand (Art. 394 Abs. 1 ZGB). ROT bleibt die vorsichtige Stufe mit «Beruf offen»: Der Fund nennt Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB und Art. 47 BankG, Art. 13 BGFA erst mit --role lawyer. Den Beruf mit --role festlegen.",
                    "Clients and representation do not show the profession: «Klient», «Mandant», «client», «mandant», «cliente» and «mandante» are used for lawyers (Art. 13 Abs. 1 BGFA), for psychotherapy (Art. 27 lit. c PsyG), for a bank's customers (Art. 4bis Abs. 1 BankG, French «client», Italian «clienti») and for any trade's customers (Art. 40b OR, Italian «cliente»); anyone authorised to do so can represent a person (Art. 32 Abs. 1 OR), for example a deputy (Art. 394 Abs. 1 ZGB). RED stays the cautious level with «profession open»: the finding names Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB and Art. 47 BankG, Art. 13 BGFA only with --role lawyer. Set the profession with --role.",
                ),
                Txt::neu(
                    "Gesundheitsberufe der Berufsliste und «Patient» gelten als doctor, «Psychotherapeutin», «psychothérapeute» und «psicoterapeuta» als psychologist; die Norm ist dieselbe (Art. 321 Ziff. 1 StGB). Staatsanwältinnen und Staatsanwälte, «Bundesanwalt», «procuratore pubblico», «procuratore generale» und «pubblico ministero» gelten als criminal-authority (Art. 320 Ziff. 1 StGB und Art. 73 Abs. 1 StPO); Richterinnen und Richter, Gerichtsschreiberinnen und Gerichtsschreiber, «cancelliere» und «vicecancelliere» als court, weil offen bleibt, ob sie im Strafverfahren urteilen. Im Bundesrecht bezeichnet «vicecancelliere» die Vizekanzler der Bundeskanzlei (Art. 31 Abs. 2 RVOG); die Rolle court folgt «cancelliere», die Norm ist dieselbe (Art. 320 Ziff. 1 StGB).",
                    "Health professions of the list and «patient» map to doctor, «Psychotherapeutin», «psychothérapeute» and «psicoterapeuta» to psychologist; the norm is the same (Art. 321 Ziff. 1 StGB). Prosecutors, «Bundesanwalt», «procuratore pubblico», «procuratore generale» and «pubblico ministero» map to criminal-authority (Art. 320 Ziff. 1 StGB and Art. 73 Abs. 1 StPO); judges, court clerks, «cancelliere» and «vicecancelliere» map to court, since it stays open whether they sit in criminal proceedings. In federal law «vicecancelliere» names the Vice-Chancellors of the Federal Chancellery (Art. 31 Abs. 2 RVOG); the role court follows «cancelliere», the norm is the same (Art. 320 Ziff. 1 StGB).",
                ),
                Txt::neu(
                    "Im Obligationenrecht ist der «procuratore» der Prokurist (Art. 458 Abs. 1 OR: «Procuratore è colui, che …») und die «procura generale» eine Generalvollmacht (Art. 535 Abs. 3 OR). Mit «di …» danach zählt «procuratore generale» darum nicht («Sono il procuratore generale di mio padre»), ausser vor dem Amt («Sono procuratore generale della Confederazione»). Ohne «di» bleibt der Fehltreffer: «Sono procuratore generale per la Rossi SA».",
                    "In the Code of Obligations the «procuratore» is the holder of a commercial power of attorney (Art. 458 Abs. 1 OR: «Procuratore è colui, che …») and the «procura generale» a general power of attorney (Art. 535 Abs. 3 OR). Followed by «di …», «procuratore generale» therefore does not count («Sono il procuratore generale di mio padre»), except before the office («Sono procuratore generale della Confederazione»). Without «di» the false match remains: «Sono procuratore generale per la Rossi SA».",
                ),
                Txt::neu(
                    "Art. 2 Abs. 2 FINIG nimmt vom Gesetz aus, wer ausschliesslich Vermögenswerte wirtschaftlich oder familiär verbundener Personen verwaltet (lit. a), wer im Rahmen eines gesetzlich geregelten Mandats Vermögen verwaltet (lit. d), und die Banken (lit. j). promptscan erkennt davon nur den Genitiv der ersten Person: «Ich bin Vermögensverwalter meiner Mutter», «Je suis gestionnaire de fortune de ma mère» und «Sono gestore patrimoniale di mia madre» zählen nicht; ob jemand ausschliesslich so verwaltet, entscheidet es nicht. Für die Angestellten einer Bank gilt Art. 47 BankG: «Ich bin Vermögensverwalterin bei der Muster Bank AG» ergibt Art. 69 FINIG, die Stufe bleibt ROT; die Bank mit --role bank festlegen.",
                    "Art. 2 Abs. 2 FINIG exempts from the act anyone who manages exclusively the assets of persons with whom they have economic or family ties (lit. a), anyone who manages assets under a mandate governed by law (lit. d), and banks (lit. j). Of these, promptscan recognises only the first-person genitive: «Ich bin Vermögensverwalter meiner Mutter», «Je suis gestionnaire de fortune de ma mère» and «Sono gestore patrimoniale di mia madre» do not count; it does not decide whether someone manages exclusively such assets. Art. 47 BankG applies to a bank's employees: «Ich bin Vermögensverwalterin bei der Muster Bank AG» gives Art. 69 FINIG and the level stays RED; set the bank with --role bank.",
                ),
                Txt::neu(
                    "Keine Rolle aus dem Text für bank, student und professional: hier gilt nur --role. Die Wörter in Art. 47 BankG sind nicht bankspezifisch, und Art. 62 DSG nennt keinen Beruf.",
                    "No role from the text for bank, student and professional: only --role applies. The words in Art. 47 BankG are not bank-specific, and Art. 62 DSG names no profession.",
                ),
                Txt::neu(
                    "Eine Vertretung zählt nur mit einer Person, die die Regel name erkennt, höchstens drei Wörter nach dem Verb: «Ich vertrete Herrn Hans Muster» zählt, «Ich vertrete die Ansicht, dass …» und «Ich vertrete einen Nachbarn» nicht. «vertreten», «représenter» und «rappresentare» meinen auch die Stellvertretung und das Darstellen: «Ich vertrete meine Kollegin Anna Roth», «Ich vertrete Frau Anna Roth während ihrer Ferien», «Dans ce tableau, je représente Monsieur Jean Exemple» und «Nel mio quadro rappresento il signor Mario Rossi» zählen nicht; «Nel quadro della procedura rappresento la signora Anna Rossi», «Ich vertrete Herrn Hans Muster bis vor Bundesgericht» und «Rappresento la signora Anna Rossi fino davanti al Tribunale federale» zählen. Umgekehrt zählt eine Vertretung nicht, wenn eines dieser Wörter keine Stellvertretung einleitet: «Ich vertrete Frau Anna Roth bis zum Abschluss des Verfahrens», «Je représente Madame Claire Dubois jusqu'au Tribunal fédéral». Andere Lesarten bleiben Fehltreffer: «Morgen vertrete ich Herrn Hans Muster im Unterricht», «Je représente Madame Claire Dubois au mariage de mon cousin».",
                    "Representation counts only with a person recognised by the rule name, at most three words after the verb: «Ich vertrete Herrn Hans Muster» counts, «Ich vertrete die Ansicht, dass …» and «Ich vertrete einen Nachbarn» do not. «vertreten», «représenter» and «rappresentare» also mean standing in for someone and depicting: «Ich vertrete meine Kollegin Anna Roth», «Ich vertrete Frau Anna Roth während ihrer Ferien», «Dans ce tableau, je représente Monsieur Jean Exemple» and «Nel mio quadro rappresento il signor Mario Rossi» do not count; «Nel quadro della procedura rappresento la signora Anna Rossi», «Ich vertrete Herrn Hans Muster bis vor Bundesgericht» and «Rappresento la signora Anna Rossi fino davanti al Tribunale federale» count. Conversely, a representation does not count when one of these words introduces something other than standing in: «Ich vertrete Frau Anna Roth bis zum Abschluss des Verfahrens», «Je représente Madame Claire Dubois jusqu'au Tribunal fédéral». Other readings remain false matches: «Morgen vertrete ich Herrn Hans Muster im Unterricht», «Je représente Madame Claire Dubois au mariage de mon cousin».",
                ),
                Txt::neu(
                    "Wörter, die kein Bundeserlass braucht, belegen Duden online und Treccani (abgerufen am 6.11.2026): «Krankenschwester», «Psychiater», «Fürsprecher», «Advokat», «psichiatra» und «Mandant»; FR «mandant» und IT «mandante» stammen aus Art. 398 Abs. 2 OR. «Fürsprecher» bezeichnet auch, wer für jemanden oder für eine Sache eintritt, FR «contrôleur» (Revisoren nach Art. 321 Ziff. 1 CP) auch andere Kontrolleure, und «Revisor», «réviseur» und «revisore» bezeichnen auch, wer Texte oder Druckbogen prüft (Duden, Larousse, Treccani): «Ich bin ein Fürsprecher der Velofahrer», «Je suis contrôleur de gestion», «Ich bin Revisorin in einer Druckerei», «Je suis réviseuse dans une agence de traduction» und «Sono revisore di bozze» ergeben Fehltreffer. Nicht erkannt: Zusammensetzungen ausser auf «-arzt» und «-ärztin» («Ex-Klientin», «Patientenakte», «Kinderpsychologin»), Hilfspersonen, andere Wortstellungen («Ich bin seit 2010 Ärztin», «Ich bin selbständige Anwältin», «ma nouvelle cliente»), Personen und Firmen, die die Regeln name und firma nicht kennen («Als Anwältin der Gegenseite habe ich»), und italienische Sätze ohne Pronomen («Lavoro come medico»).",
                    "Words that no federal act uses are attested by Duden online and Treccani (retrieved on 6.11.2026): «Krankenschwester», «Psychiater», «Fürsprecher», «Advokat», «psichiatra» and «Mandant»; French «mandant» and Italian «mandante» come from Art. 398 Abs. 2 OR. «Fürsprecher» also means an advocate of a person or cause, French «contrôleur» (auditors under Art. 321 Ziff. 1 CP) any controller, and «Revisor», «réviseur» and «revisore» also mean someone who checks texts or printer's proofs (Duden, Larousse, Treccani): «Ich bin ein Fürsprecher der Velofahrer», «Je suis contrôleur de gestion», «Ich bin Revisorin in einer Druckerei», «Je suis réviseuse dans une agence de traduction» and «Sono revisore di bozze» give false matches. Not recognised: compounds other than those ending in «-arzt» and «-ärztin» («Ex-Klientin», «Patientenakte», «Kinderpsychologin»), auxiliaries, other word orders («Ich bin seit 2010 Ärztin», «Ich bin selbständige Anwältin», «ma nouvelle cliente»), persons and companies unknown to the rules name and firma («Als Anwältin der Gegenseite habe ich») and Italian sentences without a pronoun («Lavoro come medico»).",
                ),
                Txt::neu(
                    "Zitierte Rede zählt wie eigene («Er sagte: Ich bin Arzt»), und eine Selbstbezeichnung zählt im Modell zugleich als Personenbezug: Ein Inhalt nach Art. 5 lit. c DSG im selben Satz erhält einen HINWEIS.",
                    "Quoted speech counts as the writer's own («Er sagte: Ich bin Arzt»), and the model also treats a self-description as a reference to a person: content under Art. 5 lit. c DSG in the same sentence gets a NOTE.",
                ),
                Txt::neu(
                    "Überschriften, Anreden, Adressblöcke und Unterschriften ohne Satzzeichen am Zeilenende bilden mit der nächsten Zeile einen Satz. Nach einem weichen Umbruch zählt nur ein gross geschriebenes Wort als Satzanfang: «Mein Bruder arbeitet⏎als Anwalt, habe ich gehört» ergibt keinen Rahmen, eine gross beginnende Fortsetzung einer Frage dagegen einen Fehltreffer («Ab wann bin ich Ärztin⏎Nach dem Staatsexamen?»).",
                    "Headings, salutations, address blocks and signatures without punctuation at the end of the line form one sentence with the next line. After a soft break only a capitalised word counts as the start of a sentence: «Mein Bruder arbeitet⏎als Anwalt, habe ich gehört» gives no frame, while a question continued with a capital letter gives a false match («Ab wann bin ich Ärztin⏎Nach dem Staatsexamen?»).",
                ),
                Txt::neu(
                    "Die Rahmen der ersten Person und die Verben der Vertretung sind eine geschlossene Liste im Code ohne externe Quelle; ihre Präzision ist nicht gemessen. Reicht sie nicht, gilt ROT nur noch über --role (SPEC 4.3).",
                    "The first-person frames and the verbs of representation are a closed list in the code without an external source; their precision is not measured. If it falls short, RED applies only via --role (SPEC 4.3).",
                ),
            ],
        },
        RegelInfo {
            id: PERSONENBEZUG,
            klasse: Klasse::Kontext,
            titel: Txt::neu(
                "Personenbezug ohne Identifikator",
                "Reference to a person without identifier",
            ),
            erkennt: Txt::neu(
                "Bezug auf eine bestimmte Person, ohne sie zu nennen: in der ersten Person («mein Arbeitnehmer», «unsere Mieterin», «mon employé», «il mio dipendente», auch «mein Anwalt») und als Verfahrensrolle ohne Namen («der Beschwerdeführer», «le recourant», «l'imputato»; die Regel name meldet sie). Nie selbst angezeigt; ein besonders schützenswerter Inhalt im selben Satz, an anderer Stelle als der Bezug, erhält dadurch einen HINWEIS (SPEC 4.4).",
                "A reference to a specific person without naming them: in the first person («mein Arbeitnehmer», «unsere Mieterin», «mon employé», «il mio dipendente», also «mein Anwalt») and as a procedural role without a name («der Beschwerdeführer», «le recourant», «l'imputato»; reported by the rule name). Never shown itself; sensitive content elsewhere in the same sentence gets a NOTE (SPEC 4.4).",
            ),
            methode: Txt::neu(
                "Aho-Corasick-Suche, je Sprache getrennt, nach Personenbezeichnungen aus OR, ZGB, StPO und PartG (personenbezug.toml) und nach den Berufsbezeichnungen aus kontext.toml. Ein Treffer zählt nur mit einem Possessivbegleiter der ersten Person unmittelbar davor («mein», «unsere», «mon», «nos», «mio», «nostra»); im Deutschen darf ein dekliniertes Adjektiv dazwischen stehen («meine neue Mieterin»). Treffer in Zusammensetzungen («Mieter-Verband») zählen nicht.",
                "Aho-Corasick search, per language, for designations of persons from OR, ZGB, StPO and PartG (personenbezug.toml) and for the profession names of kontext.toml. A match counts only with a first-person possessive right before it («mein», «unsere», «mon», «nos», «mio», «nostra»); in German an inflected adjective may stand in between («meine neue Mieterin»). Matches inside compounds («Mieter-Verband») do not count.",
            ),
            hinweis: None,
            grundlage: vec![&normen::DSG_5_A],
            quellen: vec![
                Quelle {
                    titel: "Obligationenrecht (OR), SR 220, Art. 40b, 96, 184, 253, 275, 319, 336d und 431, Fassungen DE, FR, IT vom 1.10.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_319",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Zivilgesetzbuch (ZGB), SR 210, Art. 30a, 252, 255, 255a, 400 und 669, Fassungen DE, FR, IT vom 1.7.2026",
                    url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_252",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "Partnerschaftsgesetz (PartG), SR 211.231, Art. 10 Abs. 1, Fassungen DE, FR, IT vom 1.1.2025",
                    url: "https://www.fedlex.admin.ch/eli/cc/2005/782/de#art_10",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
                Q_STPO,
                Q_STGB,
                Q_BGFA,
                Quelle {
                    titel: "Verfahrensrollen ohne Namen (verfahrensrollen.toml, Regel name): StPO (SR 312.0, Fassung 1.4.2025), ZPO (SR 272, Fassung 1.7.2026) und BGG (SR 173.110), je DE, FR, IT, jeder Eintrag mit Artikel",
                    url: "https://www.fedlex.admin.ch/eli/cc/2010/262/de",
                    lizenz: None,
                    stand: GEPRUEFT,
                },
            ],
            lexika: vec![DATEI_BEZUG, DATEI_BERUFE, crate::regeln::name::ROLLEN],
            guete: Guete::NichtGemessen,
            grenzen: vec![
                Txt::neu(
                    "Ob der Kontext eine Person bestimmbar macht, beurteilt promptscan nicht (SPEC 2, Grundsatz 7); der Personenbezug erzeugt selbst keinen Fund.",
                    "promptscan does not assess whether the context makes a person identifiable (SPEC 2, principle 7); the reference itself never creates a finding.",
                ),
                Txt::neu(
                    "Personenbezeichnungen zählen nur mit Possessivbegleiter der ersten Person: «der Arbeitnehmer», «ton employé», «il suo dipendente» zählen nicht. Verfahrensrollen zählen auch in allgemeinen Rechtsfragen («Was darf die beschuldigte Person, wenn sie arbeitsunfähig ist?» ergibt einen HINWEIS). Die Mehrzahl zählt («meine Mitarbeiter»).",
                    "Designations of persons count only with a first-person possessive: «der Arbeitnehmer», «ton employé», «il suo dipendente» do not count. Procedural roles count in general legal questions too («Was darf die beschuldigte Person, wenn sie arbeitsunfähig ist?» gives a NOTE). Plurals count («meine Mitarbeiter»).",
                ),
                Txt::neu(
                    "Die Liste ist kurz und nur mit geprüfter Fundstelle; es fehlen etwa «Chef», «Freundin» oder «Sohn». Im Französischen und Italienischen darf kein Adjektiv dazwischen stehen («ma nouvelle locataire» zählt nicht).",
                    "The list is short and has verified sources only; «Chef», «Freundin» or «Sohn» are missing, for example. In French and Italian no adjective may stand in between («ma nouvelle locataire» does not count).",
                ),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::fund::{Grund, Stufe};
    use crate::stufen::{Lage, Optionen, bewerte};

    /// Geheimnis-Kontexte eines Textes: Rolle und Stelle.
    fn kontext(s: &str) -> Vec<(Rolle, String)> {
        let text = Text::neu(s);
        erkenne(&text)
            .into_iter()
            .filter_map(|t| match t.art {
                Art::Geheimniskontext(r) => Some((r, text.ausschnitt(t.start..t.ende).into())),
                _ => None,
            })
            .collect()
    }

    /// Personenbezüge eines Textes: Stellen.
    fn bezug(s: &str) -> Vec<String> {
        let text = Text::neu(s);
        erkenne(&text)
            .into_iter()
            .filter(|t| t.art == Art::Personenbezug)
            .map(|t| text.ausschnitt(t.start..t.ende).into())
            .collect()
    }

    fn pruefe_kontext(faelle: &[(&str, Rolle, &str)]) {
        for (satz, rolle, stelle) in faelle {
            assert_eq!(kontext(satz), vec![(*rolle, stelle.to_string())], "{satz}");
        }
    }

    #[test]
    fn selbstbezeichnung_deutsch() {
        pruefe_kontext(&[
            (
                "Ich bin Ärztin und habe eine Frage.",
                Rolle::Doctor,
                "Ich bin Ärztin",
            ),
            (
                "Als Patentanwältin frage ich mich das.",
                Rolle::PatentAttorney,
                "Als Patentanwältin frage ich",
            ),
            ("Ich bin Geistlicher.", Rolle::Clergy, "Ich bin Geistlicher"),
            ("Ich bin A\u{0308}rztin.", Rolle::Doctor, "Ich bin Ärztin"),
            (
                "Als Anwältin habe ich eine Frage zur Frist.",
                Rolle::Lawyer,
                "Als Anwältin habe ich",
            ),
            (
                "Ich arbeite als Psychologe in einer Praxis.",
                Rolle::Psychologist,
                "Ich arbeite als Psychologe",
            ),
            (
                "Seit 2010 bin ich Notarin.",
                Rolle::Notary,
                "bin ich Notarin",
            ),
            (
                "Wir sind beide Revisoren.",
                Rolle::Auditor,
                "Wir sind beide Revisoren",
            ),
            (
                "Ich als erfahrene Staatsanwältin frage mich das.",
                Rolle::CriminalAuthority,
                "Ich als erfahrene Staatsanwältin",
            ),
            (
                "Ich bin Mitglied einer Behörde.",
                Rolle::Authority,
                "Ich bin Mitglied einer Behörde",
            ),
            ("ICH BIN RICHTERIN.", Rolle::Court, "ICH BIN RICHTERIN"),
        ]);
    }

    #[test]
    fn selbstbezeichnung_franzoesisch() {
        pruefe_kontext(&[
            (
                "Je suis avocate et j'ai une question.",
                Rolle::Lawyer,
                "Je suis avocate",
            ),
            (
                "Je suis ecclésiastique.",
                Rolle::Clergy,
                "Je suis ecclésiastique",
            ),
            (
                "En tant que conseil en brevet, je dois répondre.",
                Rolle::PatentAttorney,
                "En tant que conseil en brevet, je",
            ),
            (
                "En tant qu'avocat, je représente une société.",
                Rolle::Lawyer,
                "En tant qu'avocat, je",
            ),
            (
                "En tant qu\u{2019}avocate, je dois répondre.",
                Rolle::Lawyer,
                "En tant qu\u{2019}avocate, je",
            ),
            (
                "Je travaille comme infirmière à Genève.",
                Rolle::Doctor,
                "Je travaille comme infirmière",
            ),
            (
                "En ma qualité de notaire, j'ai reçu l'acte.",
                Rolle::Notary,
                "En ma qualité de notaire",
            ),
            (
                "Nous sommes médecins.",
                Rolle::Doctor,
                "Nous sommes médecins",
            ),
            (
                "Nous, en tant que psychologues, devons nous taire.",
                Rolle::Psychologist,
                "Nous, en tant que psychologues",
            ),
            ("Je suis sage-femme.", Rolle::Doctor, "Je suis sage-femme"),
        ]);
    }

    #[test]
    fn selbstbezeichnung_italienisch() {
        pruefe_kontext(&[
            (
                "Sono consulente in brevetti.",
                Rolle::PatentAttorney,
                "Sono consulente in brevetti",
            ),
            (
                "Sono avvocato e ho una domanda.",
                Rolle::Lawyer,
                "Sono avvocato",
            ),
            (
                "Io sono un'infermiera.",
                Rolle::Doctor,
                "Io sono un'infermiera",
            ),
            (
                "Faccio il notaio da vent'anni.",
                Rolle::Notary,
                "Faccio il notaio",
            ),
            ("Siamo psicologi.", Rolle::Psychologist, "Siamo psicologi"),
            (
                "In qualità di medico, vorrei sapere una cosa.",
                Rolle::Doctor,
                "In qualità di medico, vorrei",
            ),
            (
                "Come medico non posso rispondere.",
                Rolle::Doctor,
                "Come medico non posso",
            ),
            (
                "Io lavoro come infermiera a Lugano.",
                Rolle::Doctor,
                "Io lavoro come infermiera",
            ),
            (
                "Noi lavoriamo come avvocati.",
                Rolle::Lawyer,
                "Noi lavoriamo come avvocati",
            ),
            (
                "Nella mia qualità di giudice devo astenermi.",
                Rolle::Court,
                "Nella mia qualità di giudice",
            ),
            (
                "Mi chiamo Anna e sono avvocata.",
                Rolle::Lawyer,
                "sono avvocata",
            ),
            (
                "Come medico mi chiedo cosa valga.",
                Rolle::Doctor,
                "Come medico mi chiedo",
            ),
            (
                "Come avvocati, ci domandiamo se basti.",
                Rolle::Lawyer,
                "Come avvocati, ci domandiamo",
            ),
        ]);
    }

    /// Klientschaft zeigt keinen bestimmten Beruf. Die Wörter bleiben Auslöser, die Rolle
    /// ist offen; Patientinnen und Patienten zeigen den Gesundheitsberuf.
    #[test]
    fn klientschaft_und_patienten_drei_sprachen() {
        pruefe_kontext(&[
            (
                "Meine Klientin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine Klientin",
            ),
            (
                "Unsere Patientin klagt über Schmerzen.",
                Rolle::Doctor,
                "Unsere Patientin",
            ),
            (
                "Eine meiner Klientinnen hat angerufen.",
                Rolle::Offen,
                "meiner Klientinnen",
            ),
            (
                "Unsere Klientschaft erwartet Diskretion.",
                Rolle::Offen,
                "Unsere Klientschaft",
            ),
            (
                "Für meinen langjährigen Klienten schreibe ich.",
                Rolle::Offen,
                "meinen langjährigen Klienten",
            ),
            (
                "Meine Mandantin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine Mandantin",
            ),
            ("Ma cliente a reçu une lettre.", Rolle::Offen, "Ma cliente"),
            (
                "Mon mandant a reçu une lettre.",
                Rolle::Offen,
                "Mon mandant",
            ),
            (
                "Mon patient refuse le traitement.",
                Rolle::Doctor,
                "Mon patient",
            ),
            (
                "Il mio cliente ha ricevuto una lettera.",
                Rolle::Offen,
                "mio cliente",
            ),
            (
                "La mia mandante ha ricevuto una lettera.",
                Rolle::Offen,
                "mia mandante",
            ),
            (
                "La mia paziente soffre di emicrania.",
                Rolle::Doctor,
                "mia paziente",
            ),
        ]);
    }

    /// Kein Wort der Klientschaft verliert den Geheimnis-Kontext; jedes zeigt den offenen
    /// Beruf, in allen Formen.
    #[test]
    fn jedes_wort_der_klientschaft_bleibt_ausloeser_mit_offenem_beruf() {
        let mut zahl = 0;
        for e in &lexikon::lexikon(DATEI_BEZUG).eintraege {
            if e.regel != KONTEXT || matches!(rolle(e), Some(Rolle::Doctor)) {
                continue;
            }
            assert_eq!(rolle(e), Some(Rolle::Offen), "{}", e.begriff);
            let besitz = match e.sprache {
                Sprache::De => "Meine",
                Sprache::Fr => "Mes",
                Sprache::It => "I miei",
            };
            for wort in std::iter::once(&e.begriff).chain(&e.formen) {
                let satz = format!("{besitz} {wort} warten.");
                let gefunden = kontext(&satz);
                assert_eq!(gefunden.len(), 1, "{satz}: {gefunden:?}");
                assert_eq!(gefunden[0].0, Rolle::Offen, "{satz}");
                zahl += 1;
            }
        }
        // Klient, Klientschaft, Mandant, client, mandant, cliente, mandante mit Formen.
        assert!(zahl >= 7 * 2, "{zahl}");
    }

    #[test]
    fn ohne_ich_bezug_kein_kontext() {
        for satz in [
            "Der Anwalt der Gegenpartei hat geschrieben.",
            "Mein Anwalt hat gesagt, ich solle warten.",
            "Ärztinnen und Ärzte dürfen das nicht weitergeben.",
            "Ich bin kein Anwalt.",
            "Ich bin nicht Ärztin.",
            "Ich bin beim Arzt gewesen.",
            "Ich bin vielen Ärzten begegnet.",
            "Er arbeitet als Arzt in Bern.",
            "Ich frage mich, ob er als Anwalt haftet.",
            "Ich sehe ihn als Anwalt.",
            "Ich arbeite nicht als Ärztin.",
            "Bin ich Arzt, wenn ich Medizin studiert habe?",
            "Ab wann bin ich Arzt?",
            "Als Arzt hat er mich beraten.",
            "Ihre Klientin hat angerufen.",
            "Die Klientin hat angerufen.",
            "Mein Patienten-Dossier ist unvollständig.",
            "Wer bin ich? Arzt oder Anwalt?",
            "Je ne suis pas avocat.",
            "Je suis pas médecin.",
            "Si je suis avocat, puis-je refuser?",
            "Il est intervenu en tant qu'avocat, je crois.",
            "Mon avocat m'a dit d'attendre.",
            "Il a travaillé pour nous comme avocat.",
            "Il nous parle en tant que médecin.",
            "Je ne travaille pas comme médecin.",
            "Ton client a appelé.",
            "Non sono avvocato.",
            "Sono avvocati esperti.",
            "Ci sono un avvocato e un notaio.",
            "Se sono medico, posso rifiutare?",
            "Come medico, mi ha detto di riposare.",
            "Come medico mi consigliò di riposare.",
            "Come medico generico lavora a Lugano.",
            "Come medico ci visitò due volte.",
            "Il mio avvocato mi ha detto di aspettare.",
            "Ha lavorato per noi come avvocato.",
            "Il suo cliente ha chiamato.",
            "Questi sono l'avvocato e il notaio.",
            "Le parti sono il medico e il paziente.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    #[test]
    fn personenbezug_drei_sprachen() {
        for (satz, stelle) in [
            ("Mein Arbeitnehmer ist seit Mai krank.", "Mein Arbeitnehmer"),
            (
                "Unsere langjährige Mitarbeiterin kündigt.",
                "Unsere langjährige Mitarbeiterin",
            ),
            ("Was schulde ich meiner Vermieterin?", "meiner Vermieterin"),
            ("Mein Anwalt hat gesagt, ich solle warten.", "Mein Anwalt"),
            ("Mon employé est absent depuis lundi.", "Mon employé"),
            ("Notre locataire ne paie plus.", "Notre locataire"),
            ("Mon avocat m'a dit d'attendre.", "Mon avocat"),
            ("Il mio dipendente è malato.", "mio dipendente"),
            ("Il mio datore di lavoro non paga.", "mio datore di lavoro"),
        ] {
            assert_eq!(bezug(satz), vec![stelle.to_string()], "{satz}");
        }
    }

    #[test]
    fn personenbezug_nur_in_der_ersten_person() {
        for satz in [
            "Der Arbeitnehmer hat Anspruch auf Ferien.",
            "Meiner Meinung nach muss der Mieter zahlen.",
            "Ihr Arbeitgeber zahlt nicht.",
            "Mein Mieter-Verband hilft.",
            "Ton employé est absent.",
            "Ma nouvelle locataire paie.",
            "Il lavoratore ha diritto alle vacanze.",
            "Il suo dipendente è malato.",
        ] {
            assert_eq!(bezug(satz), Vec::<String>::new(), "{satz}");
        }
    }

    #[test]
    fn zerlegen_elision_und_bindestrich() {
        let woerter: Vec<String> = zerlegen("En tant qu\u{2019}avocate, l'Ex-Klientin.")
            .into_iter()
            .map(|w| w.klein)
            .collect();
        assert_eq!(
            woerter,
            [
                "en",
                "tant",
                "qu'",
                "avocate",
                ",",
                "l'",
                "ex-klientin",
                "."
            ]
        );
    }

    fn treffer(text: &Text, regel: &'static str, stelle: &str, art: Art) -> Treffer {
        let start = text.als_str().find(stelle).expect("Stelle im Text");
        Treffer::neu(regel, start, start + stelle.len(), art)
    }

    #[test]
    fn kontext_hebt_identifikator_auf_rot_ausser_mit_role_none() {
        let text = Text::neu("Meine Klientin Erika Beispiel hat die Frist verpasst.");
        let name = treffer(&text, "name", "Erika Beispiel", Art::Identifikator);
        let mut tr = erkenne(&text);
        tr.push(name.clone());

        let (funde, lage) = bewerte(&text, tr.clone(), &Optionen::default());
        assert!(
            matches!(&lage, Lage::AusText { rolle: Rolle::Offen, stelle, .. } if stelle == "Meine Klientin"),
            "{lage:?}"
        );
        let f = funde.iter().find(|f| f.regel == "name").expect("Name");
        assert_eq!(f.stufe, Stufe::Rot);
        assert_eq!(
            f.grund,
            Grund::Geheimnis {
                rolle: Rolle::Offen,
                aus_text: Some("Meine Klientin".into()),
                ziffer: None,
                natuerliche_person: true,
            }
        );
        assert!(funde.iter().any(|f| f.regel == KONTEXT
            && f.grund
                == Grund::Geheimniskontext {
                    rolle: Rolle::Offen
                }));

        // --role none: der Text hebt nichts.
        let keine = Optionen {
            rolle: Some(Rolle::None),
            ..Optionen::default()
        };
        let (funde, lage) = bewerte(&text, tr, &keine);
        assert_eq!(lage, Lage::Keine);
        assert_eq!(funde.len(), 1);
        assert_eq!(funde[0].stufe, Stufe::Gelb);

        // Gegenprobe ohne diese Regel: nur GELB.
        let (funde, lage) = bewerte(&text, vec![name], &Optionen::default());
        assert_eq!(lage, Lage::Keine);
        assert_eq!(funde[0].stufe, Stufe::Gelb);
    }

    #[test]
    fn personenbezug_macht_inhalt_im_satz_zum_hinweis() {
        let text =
            Text::neu("Mein Arbeitnehmer ist arbeitsunfähig. Was gilt bei Arbeitsunfähigkeit?");
        let inhalt = |s| treffer(&text, "art5c-2", s, Art::BesondersSchuetzenswert(2));
        let eigene = erkenne(&text);
        assert!(!eigene.is_empty());
        assert!(eigene.iter().all(|t| t.regel == PERSONENBEZUG));

        let mut tr = eigene;
        tr.extend([inhalt("arbeitsunfähig"), inhalt("Arbeitsunfähigkeit")]);
        let (funde, lage) = bewerte(&text, tr, &Optionen::default());
        assert_eq!(lage, Lage::Keine);
        assert_eq!(funde.len(), 1, "{funde:?}");
        assert_eq!(
            (funde[0].stelle.as_str(), funde[0].stufe, &funde[0].grund),
            ("arbeitsunfähig", Stufe::Hinweis, &Grund::OhneIdentifikator)
        );

        // Gegenprobe ohne diese Regel: kein Fund.
        let (funde, _) = bewerte(&text, vec![inhalt("arbeitsunfähig")], &Optionen::default());
        assert!(funde.is_empty());
    }

    #[test]
    fn pruefe_erschliesst_rolle_aus_dem_text() {
        let keine = Optionen {
            rolle: Some(Rolle::None),
            ..Optionen::default()
        };
        for (satz, rolle) in [
            (
                "Als Ärztin frage ich mich, was ich weitergeben darf.",
                Rolle::Doctor,
            ),
            ("En tant qu'avocate, je dois savoir ceci.", Rolle::Lawyer),
            ("Il mio cliente vuole sapere una cosa.", Rolle::Offen),
            (
                "Ich bin Vermögensverwalterin und habe eine Frage.",
                Rolle::FinancialInstitution,
            ),
        ] {
            let lage = crate::pruefe(satz, &Optionen::default()).lage;
            assert_eq!(lage.rolle(), Some(rolle), "{satz}");
            assert!(matches!(lage, Lage::AusText { .. }), "{satz}");
            assert_eq!(crate::pruefe(satz, &keine).lage, Lage::Keine, "{satz}");
        }
        let lage = crate::pruefe(
            "Mein Anwalt hat mir geraten zu warten.",
            &Optionen::default(),
        )
        .lage;
        assert_eq!(lage, Lage::Keine);
    }

    #[test]
    fn wortlisten_regeln_und_rollen() {
        let berufe = &lexikon::lexikon(DATEI_BERUFE).eintraege;
        for e in berufe {
            assert_eq!(e.regel, KONTEXT, "{}", e.begriff);
            assert_ne!(
                rolle(e),
                Some(Rolle::Offen),
                "{}: offen nur für Klientschaft",
                e.begriff
            );
            if e.merkmal.as_deref() == Some("none") {
                // Sperrt nur eine Zusammensetzung mit einem Wortende-Eintrag («Tierarzt»).
                let sperrt = berufe.iter().any(|f| {
                    f.teilwort == Teilwort::Ende
                        && rolle(f).is_some()
                        && f.sprache == e.sprache
                        && falten(&e.begriff, true)
                            .text
                            .ends_with(&falten(&f.begriff, true).text)
                });
                assert!(sperrt, "{}: merkmal none ohne Wortende-Eintrag", e.begriff);
                continue;
            }
            assert!(rolle(e).is_some(), "{}: merkmal", e.begriff);
        }
        for e in &lexikon::lexikon(DATEI_BEZUG).eintraege {
            match e.regel.as_str() {
                KONTEXT => assert!(rolle(e).is_some(), "{}: merkmal", e.begriff),
                PERSONENBEZUG => assert!(e.merkmal.is_none(), "{}: merkmal", e.begriff),
                andere => panic!("{}: regel {andere}", e.begriff),
            }
        }
    }

    /// «SR 955.0 Art. 18a Abs. 3» → «art_18_a».
    fn anker(quelle: &str) -> String {
        let nummer = quelle
            .split("Art. ")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or_default();
        let ziffern: String = nummer.chars().take_while(char::is_ascii_digit).collect();
        let rest: String = nummer.chars().skip_while(char::is_ascii_digit).collect();
        if rest.is_empty() {
            format!("art_{ziffern}")
        } else {
            format!("art_{ziffern}_{rest}")
        }
    }

    #[test]
    fn fundstellen_passen_zu_erlass_fassung_und_sprache() {
        // ELI, SR-Nummer und gelesene Fassung.
        const ERLASSE: &[(&str, &str, &str)] = &[
            ("eli/cc/54/757_781_799/", "SR 311.0 ", "2026-11-01"),
            ("eli/cc/2010/267/", "SR 312.0 ", "2025-04-01"),
            ("eli/cc/2002/153/", "SR 935.61 ", "2025-07-01"),
            ("eli/cc/1998/892_892_892/", "SR 955.0 ", "2026-10-01"),
            ("eli/cc/2007/537/", "SR 811.11 ", "2025-07-01"),
            ("eli/cc/27/317_321_377/", "SR 220 ", "2026-10-01"),
            ("eli/cc/24/233_245_233/", "SR 210 ", "2026-07-01"),
            ("eli/cc/2005/782/", "SR 211.231 ", "2025-01-01"),
            ("eli/cc/2020/16/", "SR 811.21 ", "2025-07-01"),
            ("eli/cc/1995/3867_3867_3867/", "SR 832.102 ", "2026-10-22"),
            ("eli/cc/2018/801/", "SR 954.1 ", "2026-10-01"),
            ("eli/cc/2010/444/", "SR 173.71 ", "2024-01-01"),
            ("eli/cc/1997/2022_2022_2022/", "SR 172.010 ", "2025-05-01"),
        ];
        // Ausnahme: Wörter, die kein Bundeserlass braucht («Mandant», «Krankenschwester»),
        // belegt das Wörterbuch der eigenen Sprache; die Notiz begründet es.
        const WOERTERBUECHER: &[(&str, &str, Sprache)] = &[
            (
                "https://www.duden.de/rechtschreibung/",
                "Duden online, «",
                Sprache::De,
            ),
            (
                "https://www.treccani.it/vocabolario/",
                "Treccani, Vocabolario, «",
                Sprache::It,
            ),
        ];
        let mut aus_woerterbuch = Vec::new();
        for datei in [DATEI_BERUFE, DATEI_BEZUG] {
            for e in &lexikon::lexikon(datei).eintraege {
                if let Some((_, titel, sprache)) = WOERTERBUECHER
                    .iter()
                    .find(|(url, ..)| e.url.starts_with(url))
                {
                    assert_eq!(e.sprache, *sprache, "{}", e.begriff);
                    let kopf = format!("{titel}{}»", e.begriff);
                    assert!(e.quelle.starts_with(&kopf), "{}: {}", e.begriff, e.quelle);
                    assert_eq!(e.fassung, "2026-11-06", "{}", e.begriff);
                    let notiz = e.notiz.as_deref().unwrap_or_default();
                    assert!(notiz.contains("darum das Wörterbuch"), "{}", e.begriff);
                    aus_woerterbuch.push(e.begriff.as_str());
                    continue;
                }
                let (eli, sr, fassung) = ERLASSE
                    .iter()
                    .find(|(eli, ..)| e.url.contains(eli))
                    .unwrap_or_else(|| panic!("{}: Erlass {}", e.begriff, e.url));
                let sprache = match e.sprache {
                    Sprache::De => "de",
                    Sprache::Fr => "fr",
                    Sprache::It => "it",
                };
                let url = format!(
                    "https://www.fedlex.admin.ch/{eli}{sprache}#{}",
                    anker(&e.quelle)
                );
                assert_eq!(e.url, url, "{}", e.begriff);
                assert!(e.quelle.starts_with(sr), "{}: {}", e.begriff, e.quelle);
                assert_eq!(e.fassung, *fassung, "{}", e.begriff);
            }
        }
        // Die Beschreibung (grenzen) nennt genau diese Wörter.
        assert_eq!(
            aus_woerterbuch,
            [
                "Krankenschwester",
                "Psychiater",
                "Fürsprecher",
                "Advokat",
                "psichiatra",
                "Mandant"
            ]
        );
    }

    #[test]
    fn kein_wort_in_beiden_listen() {
        let formen = |datei: &str| -> HashSet<(String, Sprache)> {
            lexikon::lexikon(datei)
                .eintraege
                .iter()
                .flat_map(|e| {
                    std::iter::once(&e.begriff)
                        .chain(&e.formen)
                        .map(|f| (falten(f, true).text, e.sprache))
                })
                .collect()
        };
        let doppelt: Vec<_> = formen(DATEI_BERUFE)
            .intersection(&formen(DATEI_BEZUG))
            .cloned()
            .collect();
        assert!(doppelt.is_empty(), "{doppelt:?}");
    }

    #[test]
    fn italienische_berufe_in_der_einzahl_ohne_endung_i() {
        // «sono» verlangt die Einzahl; sie wird an der Endung -i des ersten Wortes erkannt
        // (Umgebung::mehrzahl_it): «consulente in brevetti» ist Einzahl.
        for e in &lexikon::lexikon(DATEI_BERUFE).eintraege {
            if e.sprache == Sprache::It {
                let erstes = e.begriff.split_whitespace().next().unwrap_or_default();
                assert!(!erstes.ends_with('i'), "{}", e.begriff);
            }
        }
    }

    #[test]
    fn beschreibung_vollstaendig() {
        let infos = info();
        let ids: Vec<_> = infos.iter().map(|r| r.id).collect();
        assert_eq!(ids, [KONTEXT, PERSONENBEZUG]);
        for r in &infos {
            assert_eq!(r.klasse, Klasse::Kontext);
            assert_eq!(r.hinweis, None);
            assert_eq!(r.guete, Guete::NichtGemessen);
            assert!(!r.grundlage.is_empty() && !r.quellen.is_empty() && !r.grenzen.is_empty());
            for t in [r.titel, r.erkennt, r.methode].iter().chain(&r.grenzen) {
                assert!(!t.de.is_empty() && !t.en.is_empty(), "{}", r.id);
            }
            for q in &r.quellen {
                assert_eq!(q.stand, GEPRUEFT, "{}", q.titel);
                // Ausser den Wörterbüchern (Wörter ohne Bundeserlass, Mehrdeutigkeiten der
                // Grenzen) nur Fedlex.
                assert!(
                    q.url.starts_with("https://www.fedlex.admin.ch/eli/cc/")
                        || q.url.starts_with("https://www.duden.de/rechtschreibung/")
                        || q.url
                            .starts_with("https://www.larousse.fr/dictionnaires/francais/")
                        || q.url.starts_with("https://www.treccani.it/vocabolario/"),
                    "{}",
                    q.url
                );
            }
            for d in [DATEI_BERUFE, DATEI_BEZUG] {
                assert!(r.lexika.contains(&d), "{}: {d}", r.id);
            }
        }
        let erwartet: Vec<&Norm> = vec![
            &normen::STGB_321_1,
            &normen::BGFA_13_1,
            &normen::OR_730B_2,
            &normen::FINIG_69_1_A,
            &normen::FINIG_69_3,
            &normen::STGB_320_1,
            &normen::STPO_73_1,
            &normen::BANKG_47_1_A,
        ];
        assert_eq!(infos[0].grundlage, erwartet);
        assert_eq!(infos[1].grundlage, vec![&normen::DSG_5_A]);
        assert_eq!(
            rollen(),
            [
                Rolle::Clergy,
                Rolle::Lawyer,
                Rolle::Notary,
                Rolle::PatentAttorney,
                Rolle::Auditor,
                Rolle::Doctor,
                Rolle::Psychologist,
                Rolle::FinancialInstitution,
                Rolle::Authority,
                Rolle::Court,
                Rolle::CriminalAuthority,
                Rolle::Offen
            ]
        );
    }

    /// Der Satz der Beschreibung über Rollen ohne Erkennung aus dem Text stimmt mit den
    /// Wortlisten überein, Rolle für Rolle.
    #[test]
    fn rollen_ohne_erkennung_wie_beschrieben() {
        let satz =
            "Keine Rolle aus dem Text für bank, student und professional: hier gilt nur --role.";
        let beschreibung = info();
        let grenze = beschreibung[0]
            .grenzen
            .iter()
            .find(|t| t.de.starts_with("Keine Rolle aus dem Text"))
            .expect("Grenze");
        assert!(grenze.de.starts_with(satz), "{}", grenze.de);
        let genannt = satz
            .trim_start_matches("Keine Rolle aus dem Text für ")
            .split(':')
            .next()
            .unwrap_or_default()
            .replace(" und ", ", ");
        let genannt: Vec<&str> = genannt.split(", ").collect();
        assert_eq!(genannt, ["bank", "student", "professional"]);
        let erkannt = rollen();
        for r in Rolle::ALLE.into_iter().filter(|r| *r != Rolle::None) {
            assert_eq!(
                genannt.contains(&r.name()),
                !erkannt.contains(&r),
                "{}: Beschreibung und Wortlisten widersprechen sich",
                r.name()
            );
        }
        // Gegenprobe am Text: je ein Wort jeder Rolle mit Wortliste ergibt die Rolle.
        for (satz, rolle) in [
            ("Ich bin Geistlicher.", Rolle::Clergy),
            ("Ich bin Patentanwältin.", Rolle::PatentAttorney),
            ("Ich bin Vermögensverwalter.", Rolle::FinancialInstitution),
            ("Je suis réviseuse.", Rolle::Auditor),
        ] {
            assert_eq!(kontext(satz).first().map(|k| k.0), Some(rolle), "{satz}");
        }
        for satz in [
            "Ich bin Bankangestellte.",
            "Ich bin Studentin.",
            "Ich bin Treuhänderin.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Neue Berufe und Selbstbezeichnungen, je Sprache.
    #[test]
    fn neue_berufe_drei_sprachen() {
        pruefe_kontext(&[
            (
                "Ich bin Pflegefachfrau.",
                Rolle::Doctor,
                "Ich bin Pflegefachfrau",
            ),
            (
                "Ich bin Krankenschwester.",
                Rolle::Doctor,
                "Ich bin Krankenschwester",
            ),
            (
                "Ich bin Psychotherapeutin.",
                Rolle::Psychologist,
                "Ich bin Psychotherapeutin",
            ),
            ("Ich bin Psychiater.", Rolle::Doctor, "Ich bin Psychiater"),
            (
                "Ich bin Fürsprecher in Bern.",
                Rolle::Lawyer,
                "Ich bin Fürsprecher",
            ),
            (
                "Als Advokatin frage ich mich das.",
                Rolle::Lawyer,
                "Als Advokatin frage ich",
            ),
            ("Ich bin Hausärztin.", Rolle::Doctor, "Ich bin Hausärztin"),
            (
                "Wir sind Kinderärzte.",
                Rolle::Doctor,
                "Wir sind Kinderärzte",
            ),
            (
                "Seit 2015 bin ich Oberarzt.",
                Rolle::Doctor,
                "bin ich Oberarzt",
            ),
            (
                "Wir sind ein Wertpapierhaus.",
                Rolle::FinancialInstitution,
                "Wir sind ein Wertpapierhaus",
            ),
            (
                "Ich bin Bundesanwältin.",
                Rolle::CriminalAuthority,
                "Ich bin Bundesanwältin",
            ),
            (
                "Je suis psychothérapeute.",
                Rolle::Psychologist,
                "Je suis psychothérapeute",
            ),
            ("Je suis psychiatre.", Rolle::Doctor, "Je suis psychiatre"),
            (
                "Je suis contrôleuse.",
                Rolle::Auditor,
                "Je suis contrôleuse",
            ),
            (
                "Nous sommes une direction de fonds.",
                Rolle::FinancialInstitution,
                "Nous sommes une direction de fonds",
            ),
            (
                "Je suis gestionnaire de fortune.",
                Rolle::FinancialInstitution,
                "Je suis gestionnaire de fortune",
            ),
            (
                "En qualité de greffière, je dois répondre.",
                Rolle::Court,
                "En qualité de greffière, je",
            ),
            (
                "Je travaille en qualité de juge.",
                Rolle::Court,
                "Je travaille en qualité de juge",
            ),
            (
                "Sono psicoterapeuta.",
                Rolle::Psychologist,
                "Sono psicoterapeuta",
            ),
            ("Sono psichiatra.", Rolle::Doctor, "Sono psichiatra"),
            (
                "Sono procuratrice pubblica.",
                Rolle::CriminalAuthority,
                "Sono procuratrice pubblica",
            ),
            (
                "Faccio il pubblico ministero.",
                Rolle::CriminalAuthority,
                "Faccio il pubblico ministero",
            ),
            (
                "Sono procuratore generale.",
                Rolle::CriminalAuthority,
                "Sono procuratore generale",
            ),
            ("Sono cancelliera.", Rolle::Court, "Sono cancelliera"),
            (
                "Sono vicecancelliera.",
                Rolle::Court,
                "Sono vicecancelliera",
            ),
            (
                "Siamo gestori patrimoniali.",
                Rolle::FinancialInstitution,
                "Siamo gestori patrimoniali",
            ),
        ]);
    }

    /// Ein Unternehmen spricht von sich in der Mehrzahl, mit dem Beruf in der Einzahl:
    /// «Wir sind ein Wertpapierhaus», «Nous sommes une maison de titres» und ebenso
    /// «Siamo una società di intermediazione mobiliare» (gleiche Reichweite DE, FR, IT).
    #[test]
    fn unternehmen_in_der_mehrzahl_drei_sprachen() {
        pruefe_kontext(&[
            (
                "Wir sind ein Vermögensverwalter in Zug.",
                Rolle::FinancialInstitution,
                "Wir sind ein Vermögensverwalter",
            ),
            (
                "Nous sommes une maison de titres.",
                Rolle::FinancialInstitution,
                "Nous sommes une maison de titres",
            ),
            (
                "Nous sommes un gestionnaire de fortune.",
                Rolle::FinancialInstitution,
                "Nous sommes un gestionnaire de fortune",
            ),
            (
                "Siamo una società di intermediazione mobiliare.",
                Rolle::FinancialInstitution,
                "Siamo una società di intermediazione mobiliare",
            ),
            (
                "Noi siamo un gestore patrimoniale.",
                Rolle::FinancialInstitution,
                "Noi siamo un gestore patrimoniale",
            ),
            (
                "Siamo la direzione del fondo.",
                Rolle::FinancialInstitution,
                "Siamo la direzione del fondo",
            ),
        ]);
        for satz in [
            "Ci siamo rivolti a un avvocato.",
            "Siamo andati da un medico.",
            "Non siamo un gestore patrimoniale.",
            "Se siamo una società di intermediazione mobiliare, cosa vale?",
            "Wir sind kein Vermögensverwalter.",
            "Nous ne sommes pas une maison de titres.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Gegenbeispiele zu den neuen Wörtern und Rahmen.
    #[test]
    fn neue_berufe_ohne_fehltreffer() {
        for satz in [
            "Ich bin Tierärztin.",
            "Ich bin Kleintierarzt.",
            "Wir sind Tierärzte.",
            "Ich bin Ex-Ärztin.",
            "Ich bin Kinderpsychologin.",
            "Il est intervenu en qualité d'avocat.",
            "En qualité de juge, il a refusé.",
            "Als Anwalt von Herrn Hans Muster hat er geschrieben.",
            "Als Anwältin von Beruf weiss ich das.",
            "Ich vertrete die Ansicht, dass die Frist läuft.",
            "Ich vertrete einen Nachbarn.",
            "Ich vertrete nicht Herrn Hans Muster.",
            "Vertrete ich Herrn Hans Muster richtig?",
            "Er vertritt Herrn Hans Muster.",
            "Die Kanzlei vertritt Herrn Hans Muster.",
            "Je ne représente pas Monsieur Jean Exemple.",
            "Il représente Monsieur Jean Exemple.",
            "Non rappresento il signor Marco Esempio.",
            "Se rappresento il signor Marco Esempio, cosa rischio?",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Vertretung einer Person, Beruf offen.
    #[test]
    fn vertretung_drei_sprachen() {
        pruefe_kontext(&[
            (
                "Ich vertrete Herrn Hans Muster.",
                Rolle::Offen,
                "Ich vertrete",
            ),
            (
                "Wir vertreten Frau Anna Roth im Verfahren.",
                Rolle::Offen,
                "Wir vertreten",
            ),
            (
                "Seit 2020 vertrete ich Herrn Peter Gisler.",
                Rolle::Offen,
                "vertrete ich",
            ),
            (
                "Unsere Kanzlei vertritt Frau Erika Beispiel.",
                Rolle::Offen,
                "Unsere Kanzlei vertritt",
            ),
            (
                "Je représente Monsieur Jean Exemple.",
                Rolle::Offen,
                "Je représente",
            ),
            (
                "Nous représentons Madame Anna Roth.",
                Rolle::Offen,
                "Nous représentons",
            ),
            (
                "Rappresento il signor Marco Esempio.",
                Rolle::Offen,
                "Rappresento",
            ),
            (
                "Io rappresento il signor Marco Esempio.",
                Rolle::Offen,
                "Io rappresento",
            ),
        ]);
    }

    /// «vertreten», «représenter» und «rappresentare» meinen auch die
    /// Stellvertretung und das Darstellen. Minimalpaare DE, FR, IT.
    #[test]
    fn vertretung_nicht_bei_stellvertretung_oder_bild() {
        for satz in [
            // Stellvertretung im Beruf (Belege und Minimalpaare).
            "Ich vertrete meine Kollegin Anna Roth bis Ende Mai. Sie ist krank.",
            "Ich vertrete meine Kollegin Anna Roth.",
            "Wir vertreten unseren Arbeitskollegen Hans Muster.",
            "Je représente ma collègue Claire Dubois pendant son congé.",
            "Je représente ma collègue Claire Dubois.",
            "Rappresento la collega Anna Rossi durante le sue vacanze.",
            "Rappresento la collega Anna Rossi.",
            // Jedes Wort aus KOLLEGEN, auch in der Mehrzahl (explain kontext, Methode).
            "Ich vertrete meinen Kollegen Hans Muster.",
            "Ich vertrete meine Arbeitskollegin Anna Roth.",
            "Wir vertreten unsere Kolleginnen Anna Roth und Eva Muster.",
            "Wir vertreten unsere Arbeitskolleginnen Anna Roth und Eva Muster.",
            "Nous représentons nos collègues Claire Dubois et Jean Exemple.",
            "Rappresentiamo i colleghi Mario Rossi e Marco Esempio.",
            "Rappresentiamo le colleghe Anna Rossi e Maria Esempio.",
            // Stellvertretung auf Zeit.
            "Ich vertrete Frau Anna Roth während ihrer Ferien. Sie ist erreichbar.",
            "Ich vertrete Frau Anna Roth bis Ende Mai.",
            "Je représente Madame Claire Dubois pendant son congé.",
            "Je représente Madame Claire Dubois jusqu'au 30 mai.",
            "Rappresento la signora Anna Rossi durante le sue vacanze.",
            "Rappresento la signora Anna Rossi fino a maggio.",
            // Darstellen.
            "Dans ce tableau, je représente Monsieur Jean Exemple en roi.",
            "Sur cette photo, je représente Monsieur Jean Exemple.",
            "Nel mio quadro rappresento il signor Mario Rossi come un re.",
            "In questo ritratto rappresento il signor Mario Rossi.",
            // Jedes Bildwort, auch in der Mehrzahl (explain kontext, Methode).
            "Dans ces tableaux, je représente Monsieur Jean Exemple.",
            "Dans cette peinture, je représente Madame Claire Dubois.",
            "Dans ces peintures, je représente Monsieur Jean Exemple.",
            "Sur ces photos, je représente Madame Claire Dubois.",
            "Dans ce portrait, je représente Monsieur Jean Exemple.",
            "Dans ces portraits, je représente Madame Claire Dubois.",
            "Nei miei quadri rappresento la signora Anna Rossi.",
            "In questo dipinto rappresento il signor Mario Rossi.",
            "In questi dipinti rappresento la signora Anna Rossi.",
            "Nei ritratti rappresento il signor Mario Rossi.",
            "In questa foto rappresento la signora Anna Rossi.",
            "In questa fotografia rappresento il signor Mario Rossi.",
            "In queste fotografie rappresento la signora Anna Rossi.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
        // Gegenbeispiele: ohne Kollegin, Zeitangabe oder Bild bleibt die Vertretung.
        pruefe_kontext(&[
            (
                "Ich vertrete Frau Anna Roth im Verfahren.",
                Rolle::Offen,
                "Ich vertrete",
            ),
            (
                "Ich vertrete Herrn Hans Muster bis vor Bundesgericht.",
                Rolle::Offen,
                "Ich vertrete",
            ),
            // Die Instanz in allen drei Sprachen (BIS_VOR; FR «jusque devant» ohne Elision).
            (
                "Je représente Madame Claire Dubois jusque devant le Tribunal fédéral.",
                Rolle::Offen,
                "Je représente",
            ),
            (
                "Rappresento la signora Anna Rossi fino davanti al Tribunale federale.",
                Rolle::Offen,
                "Rappresento",
            ),
            (
                "Je représente Madame Claire Dubois devant le tribunal.",
                Rolle::Offen,
                "Je représente",
            ),
            (
                "Rappresento la signora Anna Rossi davanti al tribunale.",
                Rolle::Offen,
                "Rappresento",
            ),
            (
                "Nel quadro della procedura rappresento la signora Anna Rossi.",
                Rolle::Offen,
                "rappresento",
            ),
            (
                "Dans le tableau de distribution, je représente Monsieur Jean Exemple.",
                Rolle::Offen,
                "je représente",
            ),
            (
                "Je représente Monsieur Jean Exemple, qui conteste le tableau.",
                Rolle::Offen,
                "Je représente",
            ),
            (
                "Rappresento il signor Mario Rossi e il quadro giuridico è chiaro.",
                Rolle::Offen,
                "Rappresento",
            ),
        ]);
        // Bleibende Fehltreffer (explain kontext, Grenzen).
        pruefe_kontext(&[
            (
                "Morgen vertrete ich Herrn Hans Muster im Mathematikunterricht.",
                Rolle::Offen,
                "vertrete ich",
            ),
            (
                "Je représente Madame Claire Dubois au mariage de mon cousin.",
                Rolle::Offen,
                "Je représente",
            ),
        ]);
    }

    /// Wer ausschliesslich Vermögenswerte verbundener Personen verwaltet,
    /// untersteht dem FINIG nicht (Art. 2 Abs. 2 lit. a FINIG). Minimalpaare DE, FR, IT.
    #[test]
    fn finanzinstitut_nicht_fuer_verbundene_personen() {
        for satz in [
            "Ich bin Vermögensverwalter meiner Mutter Anna Roth.",
            "Ich bin Vermögensverwalterin meines Vaters.",
            "Ich bin Trustee meiner Familie.",
            "Je suis gestionnaire de fortune de ma mère.",
            "Je suis gestionnaire de fortune de notre famille.",
            "Sono gestore patrimoniale di mia madre.",
            "Sono gestore patrimoniale della mia famiglia.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
        pruefe_kontext(&[
            (
                "Ich bin Vermögensverwalter in Zug.",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalter",
            ),
            (
                "Ich bin Vermögensverwalter meiner Kundin Anna Roth.",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalter",
            ),
            (
                "Ich bin Vermögensverwalterin meiner langjährigen Kunden.",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalterin",
            ),
            (
                "Ich bin Vermögensverwalterin bei der Muster Bank AG.",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalterin",
            ),
            (
                "Je suis gestionnaire de fortune à Genève.",
                Rolle::FinancialInstitution,
                "Je suis gestionnaire de fortune",
            ),
            (
                "Sono gestore patrimoniale a Lugano.",
                Rolle::FinancialInstitution,
                "Sono gestore patrimoniale",
            ),
        ]);
        // Mit der Klientschaft zählt auch diese selbst (Beruf offen).
        for (satz, stelle) in [
            (
                "Je suis gestionnaire de fortune de mes clients.",
                "Je suis gestionnaire de fortune",
            ),
            (
                "Sono gestore patrimoniale dei miei clienti.",
                "Sono gestore patrimoniale",
            ),
        ] {
            let k = kontext(satz);
            assert!(
                k.contains(&(Rolle::FinancialInstitution, stelle.to_string())),
                "{satz}: {k:?}"
            );
        }
        // Ganzer Ablauf: Der Satz wird nicht ROT.
        let satz = "Ich bin Vermögensverwalter meiner Mutter Anna Roth. Ihre IBAN ist CH93 0076 2011 6238 5295 7.";
        let bericht = crate::pruefe(satz, &Optionen::default());
        assert_eq!(bericht.lage, Lage::Keine, "{satz}");
        assert!(
            bericht.funde.iter().all(|f| f.stufe != Stufe::Rot),
            "{satz}"
        );
        // Angestellte einer Bank: Art. 69 FINIG statt Art. 47 BankG, die Stufe bleibt ROT;
        // --role bank nennt die Norm der Bank.
        let satz = "Ich bin Vermögensverwalterin bei der Muster Bank AG.";
        for (optionen, rolle) in [
            (Optionen::default(), Rolle::FinancialInstitution),
            (
                Optionen {
                    rolle: Some(Rolle::Bank),
                    ..Optionen::default()
                },
                Rolle::Bank,
            ),
        ] {
            let bericht = crate::pruefe(satz, &optionen);
            assert!(
                bericht.funde.iter().any(|f| f.stufe == Stufe::Rot
                    && matches!(f.grund, Grund::Geheimnis { rolle: r, .. } if r == rolle)),
                "{satz}: {:?}",
                bericht.funde
            );
        }
    }

    /// «procuratore generale di …» ist eine private Vollmacht (Art. 458 Abs. 1 und
    /// Art. 535 Abs. 3 OR), ausser vor dem Amt.
    #[test]
    fn procuratore_generale_als_vollmacht() {
        for satz in [
            "Sono il procuratore generale di mio padre, Mario Rossi, e devo vendere la sua casa.",
            "Sono procuratore generale della Rossi SA.",
            "Sono procuratrice generale di mia madre.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
            let bericht = crate::pruefe(satz, &Optionen::default());
            assert!(
                bericht.funde.iter().all(|f| f.stufe != Stufe::Rot),
                "{satz}"
            );
        }
        pruefe_kontext(&[
            (
                "Sono procuratore generale della Confederazione.",
                Rolle::CriminalAuthority,
                "Sono procuratore generale",
            ),
            (
                "Sono il procuratore generale del Cantone Ticino.",
                Rolle::CriminalAuthority,
                "Sono il procuratore generale",
            ),
            (
                "Sono procuratrice generale del Canton Ticino.",
                Rolle::CriminalAuthority,
                "Sono procuratrice generale",
            ),
            (
                "Sono procuratore generale della Repubblica e Cantone Ticino.",
                Rolle::CriminalAuthority,
                "Sono procuratore generale",
            ),
            (
                "Sono procuratore generale del Ministero pubblico.",
                Rolle::CriminalAuthority,
                "Sono procuratore generale",
            ),
            // Bleibender Fehltreffer ohne «di» (explain kontext, Grenzen).
            (
                "Sono procuratore generale per la Rossi SA.",
                Rolle::CriminalAuthority,
                "Sono procuratore generale",
            ),
        ]);
        // Die Begriffe der Ausnahme stehen in der Wortliste, mit der Rolle der Strafbehörde.
        for begriff in VOLLMACHT_IT {
            let e = lexikon::lexikon(DATEI_BERUFE)
                .eintraege
                .iter()
                .find(|e| e.begriff == *begriff && e.sprache == Sprache::It)
                .unwrap_or_else(|| panic!("{begriff} fehlt in {DATEI_BERUFE}"));
            assert_eq!(rolle(e), Some(Rolle::CriminalAuthority), "{begriff}");
        }
    }

    /// «Revisor», «réviseur» und «revisore» bezeichnen auch, wer Texte prüft. Die Fehltreffer
    /// stehen in explain kontext; die Rolle bleibt auditor.
    #[test]
    fn revisoren_mehrdeutig_wie_beschrieben() {
        for satz in [
            "Je suis réviseur dans une agence de traduction. Monsieur Jean Exemple m'a envoyé son texte.",
            "En tant que réviseuse, je corrige la thèse de Madame Claire Dubois.",
            "Ich bin Revisorin in einer Druckerei.",
            "Sono revisore di bozze.",
        ] {
            let k = kontext(satz);
            assert_eq!(k.len(), 1, "{satz}: {k:?}");
            assert_eq!(k[0].0, Rolle::Auditor, "{satz}");
        }
    }

    /// «Als Rechtsanwältin von …, … ich»: der genannte Beruf.
    #[test]
    fn als_beruf_einer_person() {
        pruefe_kontext(&[
            (
                "Als Rechtsanwältin von Frau Anna Roth bitte ich um Auskunft.",
                Rolle::Lawyer,
                "Als Rechtsanwältin",
            ),
            (
                "Als Ärztin des Herrn Hans Muster muss ich wissen, was gilt.",
                Rolle::Doctor,
                "Als Ärztin",
            ),
            (
                "Als Anwältin der beschuldigten Person habe ich eine Frage.",
                Rolle::Lawyer,
                "Als Anwältin",
            ),
        ]);
    }

    /// Verfahrensrollen ohne Namen zählen als Person, wenn die Regel name sie kennt.
    #[test]
    fn vertretung_einer_verfahrensrolle() {
        pruefe_kontext(&[
            (
                "Ich vertrete die beschuldigte Person.",
                Rolle::Offen,
                "Ich vertrete",
            ),
            ("Je représente le prévenu.", Rolle::Offen, "Je représente"),
            ("Rappresento l'imputato.", Rolle::Offen, "Rappresento"),
        ]);
    }

    /// Jedes Beispiel der Beschreibung, das als erkannt gilt, wird erkannt; jedes, das als
    /// nicht erkannt oder Fehltreffer gilt, verhält sich so.
    #[test]
    fn beispiele_der_beschreibung() {
        let beschreibung = info();
        let kontext_info = &beschreibung[0];
        let alle: String = [kontext_info.erkennt, kontext_info.methode]
            .iter()
            .chain(&kontext_info.grenzen)
            .map(|t| t.de)
            .collect::<Vec<_>>()
            .join(" ");
        let erkannt: &[(&str, Rolle, &str)] = &[
            ("ich bin Ärztin", Rolle::Doctor, "ich bin Ärztin."),
            (
                "Als Anwältin habe ich",
                Rolle::Lawyer,
                "Als Anwältin habe ich eine Frage.",
            ),
            (
                "Als Rechtsanwältin von Frau Anna Roth bitte ich",
                Rolle::Lawyer,
                "Als Rechtsanwältin von Frau Anna Roth bitte ich um Rat.",
            ),
            (
                "en tant qu'avocat, je",
                Rolle::Lawyer,
                "En tant qu'avocat, je doute.",
            ),
            ("sono avvocato", Rolle::Lawyer, "Sono avvocato."),
            ("meine Klientin", Rolle::Offen, "Meine Klientin wartet."),
            ("ma patiente", Rolle::Doctor, "Ma patiente attend."),
            ("il mio cliente", Rolle::Offen, "Il mio cliente aspetta."),
            (
                "Ich vertrete Herrn Hans Muster",
                Rolle::Offen,
                "Ich vertrete Herrn Hans Muster.",
            ),
            (
                "Je représente Monsieur Jean Exemple",
                Rolle::Offen,
                "Je représente Monsieur Jean Exemple.",
            ),
            (
                "Rappresento il signor Marco Esempio",
                Rolle::Offen,
                "Rappresento il signor Marco Esempio.",
            ),
            ("Hausärztin", Rolle::Doctor, "Ich bin Hausärztin."),
            (
                "Wir sind ein Wertpapierhaus",
                Rolle::FinancialInstitution,
                "Wir sind ein Wertpapierhaus.",
            ),
            (
                "Nous sommes une maison de titres",
                Rolle::FinancialInstitution,
                "Nous sommes une maison de titres.",
            ),
            (
                "Siamo una società di intermediazione mobiliare",
                Rolle::FinancialInstitution,
                "Siamo una società di intermediazione mobiliare.",
            ),
            (
                "Er sagte: Ich bin Arzt",
                Rolle::Doctor,
                "Er sagte: Ich bin Arzt.",
            ),
            (
                "Ich bin ein Fürsprecher der Velofahrer",
                Rolle::Lawyer,
                "Ich bin ein Fürsprecher der Velofahrer.",
            ),
            (
                "Je suis contrôleur de gestion",
                Rolle::Auditor,
                "Je suis contrôleur de gestion.",
            ),
            (
                "Ich bin Revisorin in einer Druckerei",
                Rolle::Auditor,
                "Ich bin Revisorin in einer Druckerei.",
            ),
            (
                "Je suis réviseuse dans une agence de traduction",
                Rolle::Auditor,
                "Je suis réviseuse dans une agence de traduction.",
            ),
            (
                "Sono revisore di bozze",
                Rolle::Auditor,
                "Sono revisore di bozze.",
            ),
            (
                "Nel quadro della procedura rappresento la signora Anna Rossi",
                Rolle::Offen,
                "Nel quadro della procedura rappresento la signora Anna Rossi.",
            ),
            (
                "Ich vertrete Herrn Hans Muster bis vor Bundesgericht",
                Rolle::Offen,
                "Ich vertrete Herrn Hans Muster bis vor Bundesgericht.",
            ),
            (
                "Rappresento la signora Anna Rossi fino davanti al Tribunale federale",
                Rolle::Offen,
                "Rappresento la signora Anna Rossi fino davanti al Tribunale federale.",
            ),
            (
                "Morgen vertrete ich Herrn Hans Muster im Unterricht",
                Rolle::Offen,
                "Morgen vertrete ich Herrn Hans Muster im Unterricht.",
            ),
            (
                "Je représente Madame Claire Dubois au mariage de mon cousin",
                Rolle::Offen,
                "Je représente Madame Claire Dubois au mariage de mon cousin.",
            ),
            (
                "meiner Kundin",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalter meiner Kundin.",
            ),
            (
                "Ich bin Vermögensverwalterin bei der Muster Bank AG",
                Rolle::FinancialInstitution,
                "Ich bin Vermögensverwalterin bei der Muster Bank AG.",
            ),
            (
                "Sono procuratore generale della Confederazione",
                Rolle::CriminalAuthority,
                "Sono procuratore generale della Confederazione.",
            ),
            (
                "Sono procuratore generale per la Rossi SA",
                Rolle::CriminalAuthority,
                "Sono procuratore generale per la Rossi SA.",
            ),
            (
                "En tant qu'avocate de la société Exemple SA, je",
                Rolle::Lawyer,
                "En tant qu'avocate de la société Exemple SA, je dois répondre.",
            ),
            (
                "In qualità di avvocato della Trasporti Esempio SA devo",
                Rolle::Lawyer,
                "In qualità di avvocato della Trasporti Esempio SA devo scrivere.",
            ),
            ("Ich bin⏎Ärztin", Rolle::Doctor, "Ich bin\nÄrztin."),
            (
                "Guten Tag⏎Als Anwältin habe ich",
                Rolle::Lawyer,
                "Guten Tag\nAls Anwältin habe ich eine Frage.",
            ),
            (
                "Seit 2010 bin ich Ärztin⏎Darf ich …?",
                Rolle::Doctor,
                "Seit 2010 bin ich Ärztin\nDarf ich die Diagnose weitergeben?",
            ),
            (
                "Ab wann bin ich Ärztin⏎Nach dem Staatsexamen?",
                Rolle::Doctor,
                "Ab wann bin ich Ärztin\nNach dem Staatsexamen?",
            ),
        ];
        for (beispiel, rolle, satz) in erkannt {
            assert!(alle.contains(&format!("«{beispiel}»")), "{beispiel}");
            let k = kontext(satz);
            assert_eq!(k.len(), 1, "{satz}: {k:?}");
            assert_eq!(k[0].0, *rolle, "{satz}");
        }
        let nicht_erkannt = [
            "Tierarzt",
            "Ich vertrete die Ansicht, dass …",
            "Ich vertrete einen Nachbarn",
            "Ex-Klientin",
            "Patientenakte",
            "Kinderpsychologin",
            "Ich bin seit 2010 Ärztin",
            "Ich bin selbständige Anwältin",
            "ma nouvelle cliente",
            "Als Anwältin der Gegenseite habe ich",
            "Lavoro come medico",
            "Ich vertrete meine Kollegin Anna Roth",
            "Ich vertrete Frau Anna Roth während ihrer Ferien",
            "Ich vertrete Frau Anna Roth bis zum Abschluss des Verfahrens",
            "Je représente Madame Claire Dubois jusqu'au Tribunal fédéral",
            "Dans ce tableau, je représente Monsieur Jean Exemple",
            "Nel mio quadro rappresento il signor Mario Rossi",
            "Sono il procuratore generale di mio padre",
            "Ich bin Vermögensverwalter meiner Mutter",
            "Je suis gestionnaire de fortune de ma mère",
            "Sono gestore patrimoniale di mia madre",
            "Mein Bruder arbeitet⏎als Anwalt, habe ich gehört",
        ];
        // Die Regel name meldet auch «Klägerin» (Verfahrensrolle): Der Rahmen
        // «Als … der <Person> … ich» zählt dann.
        assert_eq!(
            kontext("Als Anwältin der Klägerin habe ich eine Frage."),
            vec![(Rolle::Lawyer, "Als Anwältin".to_string())]
        );
        for beispiel in nicht_erkannt {
            assert!(alle.contains(&format!("«{beispiel}»")), "{beispiel}");
            let satz = match beispiel {
                "Tierarzt" => "Ich bin Tierarzt.".to_string(),
                "Ex-Klientin" | "Patientenakte" => format!("Meine {beispiel} liegt hier."),
                "Kinderpsychologin" => "Ich bin Kinderpsychologin.".to_string(),
                b => format!("{}.", b.trim_end_matches(" …")).replace('⏎', "\n"),
            };
            assert_eq!(kontext(&satz), Vec::new(), "{satz}");
        }
    }

    /// Nach einer Anrede, einem Betreff oder einer Überschrift ohne Satzzeichen beginnt die
    /// nächste Zeile wie ein Satz (`Text::satzanfang_bei`). Beispiele in DE, FR und IT, mit
    /// und ohne Komma und mit CRLF.
    #[test]
    fn selbstbezeichnung_nach_weichem_umbruch() {
        for (satz, rolle, stelle) in [
            (
                "Sehr geehrte Damen und Herren,\nAls Anwältin vertrete ich Frau Erika Beispiel.",
                Rolle::Lawyer,
                "Als Anwältin vertrete ich",
            ),
            (
                "Sehr geehrte Damen und Herren,\r\nAls Anwältin vertrete ich Frau Erika Beispiel.",
                Rolle::Lawyer,
                "Als Anwältin vertrete ich",
            ),
            (
                "Guten Tag\nAls Arzt frage ich mich, ob ich die Diagnose weitergeben darf.",
                Rolle::Doctor,
                "Als Arzt frage ich",
            ),
            (
                "Betreff Erbstreit Beispiel\nAls Anwältin habe ich eine Frage.",
                Rolle::Lawyer,
                "Als Anwältin habe ich",
            ),
            (
                "Bonjour,\nEn tant qu'avocate, je représente Mme Marie Exemple.",
                Rolle::Lawyer,
                "En tant qu'avocate, je",
            ),
            (
                "Bonjour\nEn tant qu'avocate, je représente Mme Marie Exemple.",
                Rolle::Lawyer,
                "En tant qu'avocate, je",
            ),
            (
                "Objet succession Exemple\nComme avocate, je représente Mme Marie Exemple.",
                Rolle::Lawyer,
                "Comme avocate, je",
            ),
            (
                "Madame, Monsieur\nNous, en tant qu'avocats, représentons M. Jean Exemple.",
                Rolle::Lawyer,
                "Nous, en tant qu'avocats",
            ),
            (
                "Buongiorno\nSono avvocata e rappresento la signora Giulia Esempio.",
                Rolle::Lawyer,
                "Sono avvocata",
            ),
            (
                "Oggetto successione Esempio\nIn qualità di avvocata, vorrei sapere come procedere.",
                Rolle::Lawyer,
                "In qualità di avvocata, vorrei",
            ),
            (
                "Oggetto paziente\nCome medico mi chiedo se posso trasmettere i dati.",
                Rolle::Doctor,
                "Come medico mi chiedo",
            ),
        ] {
            let gefunden = kontext(satz);
            assert!(
                gefunden.contains(&(rolle, stelle.to_string())),
                "{satz}: {gefunden:?}"
            );
            // Wie einzeilig: dasselbe wie die zweite Zeile allein, auch wo eine weitere Prüfung
            // der Regel anschlägt (Vertretung: «vertrete ich», «je représente»).
            let zweite = satz.lines().nth(1).unwrap_or(satz);
            assert_eq!(gefunden, kontext(zweite), "{satz}");
        }
        // Mitten in der Zeile bleibt es wie im einzeiligen Satz.
        for satz in [
            "Guten Tag\nEr arbeitet als Arzt in Bern.",
            "Guten Tag\nIl est intervenu en tant qu'avocat, je crois.",
            "Buongiorno\nQuesti sono l'avvocato e il notaio.",
            "Oggetto\nCome medico, mi ha detto di riposare.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Eine Frage gilt nur im Satzstück des Treffers: Die Frage der nächsten Zeile macht
    /// «Seit 2010 bin ich Ärztin» nicht zur Frage. Steht das Fragezeichen in der Zeile des
    /// Treffers, bleibt es eine Frage.
    #[test]
    fn frage_nur_im_satzstueck() {
        pruefe_kontext(&[
            (
                "Seit 2010 bin ich Ärztin\nDarf ich die Diagnose weitergeben?",
                Rolle::Doctor,
                "bin ich Ärztin",
            ),
            (
                "Seit 2010 bin ich Ärztin.\nDarf ich die Diagnose weitergeben?",
                Rolle::Doctor,
                "bin ich Ärztin",
            ),
        ]);
        for satz in [
            "Ab wann bin ich\nArzt?",
            "Guten Tag\nAb wann bin ich Arzt?",
            "Ab wann bin ich Arzt?",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Unsichtbare Formatzeichen zwischen dem Rahmen und dem Beruf trennen nichts.
    #[test]
    fn unsichtbare_formatzeichen_zwischen_den_woertern() {
        pruefe_kontext(&[
            (
                "Meine \u{200B}Mandantin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine \u{200B}Mandantin",
            ),
            (
                "Meine\u{200B} Mandantin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine\u{200B} Mandantin",
            ),
            (
                "Meine\u{2060} Mandantin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine\u{2060} Mandantin",
            ),
            (
                "Meine \u{FEFF}Mandantin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine \u{FEFF}Mandantin",
            ),
            (
                "Ich bin \u{200B}Anwältin.",
                Rolle::Lawyer,
                "Ich bin \u{200B}Anwältin",
            ),
            (
                "Ma \u{200B}cliente veut répudier la succession.",
                Rolle::Offen,
                "Ma \u{200B}cliente",
            ),
            (
                "Je suis\u{2060} avocate.",
                Rolle::Lawyer,
                "Je suis\u{2060} avocate",
            ),
            (
                "La mia \u{FEFF}cliente vuole rinunciare all'eredità.",
                Rolle::Offen,
                "mia \u{FEFF}cliente",
            ),
            (
                "Meine Mandan\u{AD}tin will das Erbe ausschlagen.",
                Rolle::Offen,
                "Meine Mandan\u{AD}tin",
            ),
        ]);
        let w =
            zerlegen("Mus\u{AD}\u{AD}ter \u{200B} Ex-\u{200B}Klientin\u{200B} qu'\u{2060}avocat");
        let woerter: Vec<&str> = w.iter().map(|w| w.klein.as_str()).collect();
        assert_eq!(woerter, ["muster", "ex-klientin", "qu'", "avocat"]);
    }

    /// Das Beispiel der Grenze in explain personenbezug: genau dieser Satz ergibt einen
    /// HINWEIS zu Art. 5 lit. c Ziff. 2 DSG.
    #[test]
    fn beispiel_der_grenze_personenbezug() {
        let satz = "Was darf die beschuldigte Person, wenn sie arbeitsunfähig ist?";
        let beschreibung = info();
        let grenzen = &beschreibung[1].grenzen;
        assert!(
            grenzen
                .iter()
                .any(|g| g.de.contains(&format!("«{satz}» ergibt einen HINWEIS")))
        );
        assert!(
            grenzen
                .iter()
                .any(|g| g.en.contains(&format!("«{satz}» gives a NOTE")))
        );
        let funde = crate::pruefe(satz, &Optionen::default()).funde;
        let gezeigt: Vec<(&str, Stufe, &str)> = funde
            .iter()
            .map(|f| (f.regel, f.stufe, f.stelle.as_str()))
            .collect();
        assert_eq!(gezeigt, vec![("art5c-2", Stufe::Hinweis, "arbeitsunfähig")]);
    }

    /// Unsichtbare Formatzeichen zwischen Possessiv oder «bin» und dem Wort danach zählen wie
    /// Leerraum, im Wort wie nichts.
    #[test]
    fn formatzeichen_zwischen_den_woertern() {
        pruefe_kontext(&[
            (
                "Meine\u{200B} Mandantin wartet.",
                Rolle::Offen,
                "Meine\u{200B} Mandantin",
            ),
            (
                "Meine \u{2060}Mandantin wartet.",
                Rolle::Offen,
                "Meine \u{2060}Mandantin",
            ),
            (
                "Meine \u{FEFF}Mandantin wartet.",
                Rolle::Offen,
                "Meine \u{FEFF}Mandantin",
            ),
            (
                "Meine Mandan\u{AD}tin wartet.",
                Rolle::Offen,
                "Meine Mandan\u{AD}tin",
            ),
            (
                "Ich bin \u{200B}Anwältin.",
                Rolle::Lawyer,
                "Ich bin \u{200B}Anwältin",
            ),
            (
                "Ma \u{200B}cliente attend.",
                Rolle::Offen,
                "Ma \u{200B}cliente",
            ),
            (
                "Il mio\u{2060} cliente aspetta.",
                Rolle::Offen,
                "mio\u{2060} cliente",
            ),
        ]);
        let w = zerlegen("Mandan\u{AD}tin \u{200B} ich");
        let klein: Vec<&str> = w.iter().map(|w| w.klein.as_str()).collect();
        assert_eq!(klein, ["mandantin", "ich"]);
    }

    /// Ein Rahmen mit vertretener Firma ergibt dieselbe Rolle wie ohne Firma, wenn die Regel
    /// firma die Firma erkennt.
    #[test]
    fn rahmen_mit_vertretener_firma() {
        for (mit, ohne, rolle, partei) in [
            (
                "Als Anwältin der Muster AG habe ich Herrn Hans Muster beraten.",
                "Als Anwältin habe ich Herrn Hans Muster beraten.",
                Rolle::Lawyer,
                "Muster AG",
            ),
            (
                "In qualità di avvocato della Trasporti Esempio SA devo scrivere a Mario Esempio.",
                "In qualità di avvocato devo scrivere a Mario Esempio.",
                Rolle::Lawyer,
                "Trasporti Esempio SA",
            ),
            (
                "En tant qu'avocate de la société Exemple SA, j'ai conseillé Monsieur Jean Exemple.",
                "En tant qu'avocate, j'ai conseillé Monsieur Jean Exemple.",
                Rolle::Lawyer,
                "Exemple SA",
            ),
            (
                "En qualité d'avocate de Exemple SA, je conseille Monsieur Jean Exemple.",
                "En qualité d'avocate, je conseille Monsieur Jean Exemple.",
                Rolle::Lawyer,
                "Exemple SA",
            ),
            (
                "Come medico della Clinica Esempio SA, vorrei sapere una cosa.",
                "Come medico, vorrei sapere una cosa.",
                Rolle::Doctor,
                "Clinica Esempio SA",
            ),
            (
                "En tant qu'avocate de Monsieur Jean Exemple, je dois répondre.",
                "En tant qu'avocate, je dois répondre.",
                Rolle::Lawyer,
                "Jean Exemple",
            ),
            // Firma, deren Name auf eine Aargauer Gemeinde endet.
            (
                "Als Anwalt der Garage Wohlen AG habe ich eine Frage zu Hans Muster.",
                "Als Anwalt habe ich eine Frage zu Hans Muster.",
                Rolle::Lawyer,
                "Garage Wohlen AG",
            ),
        ] {
            // Die Partei melden die Regeln firma oder name.
            let text = Text::neu(mit);
            let gemeldet: Vec<&str> = firma::erkenne(&text)
                .into_iter()
                .chain(name::erkenne(&text))
                .map(|t| text.ausschnitt(t.start..t.ende))
                .collect();
            assert!(gemeldet.contains(&partei), "{mit}: {gemeldet:?}");
            let k_mit: Vec<Rolle> = kontext(mit).into_iter().map(|k| k.0).collect();
            let k_ohne: Vec<Rolle> = kontext(ohne).into_iter().map(|k| k.0).collect();
            assert_eq!(k_mit, vec![rolle], "{mit}");
            assert_eq!(k_mit, k_ohne, "{ohne}");
            let lage_mit = crate::pruefe(mit, &Optionen::default()).lage;
            let lage_ohne = crate::pruefe(ohne, &Optionen::default()).lage;
            assert_eq!(lage_mit.rolle(), Some(rolle), "{mit}");
            assert_eq!(lage_ohne.rolle(), Some(rolle), "{ohne}");
            assert!(matches!(lage_mit, Lage::AusText { .. }), "{mit}");
        }
        // Ohne Firma, die die Regel firma meldet, und ohne erste Person: kein Rahmen.
        for satz in [
            "Als Anwältin der Gegenseite habe ich eine Frage.",
            "En tant qu'avocate de la société, je dois répondre.",
            "Als Anwalt der Muster AG hat er geklagt.",
            "In qualità di avvocato della Trasporti Esempio SA ha scritto.",
            "En tant qu'avocate de la société Exemple SA, elle a répondu.",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }

    /// Eine Anrede oder ein Betreff ohne Satzzeichen bildet mit der nächsten Zeile einen Satz;
    /// die Selbstbezeichnung am Anfang der nächsten Zeile zählt trotzdem, und eine Frage dort
    /// gehört nicht zum Rahmen davor.
    #[test]
    fn weicher_umbruch_vor_der_selbstbezeichnung() {
        pruefe_kontext(&[
            (
                "Sehr geehrte Damen und Herren,\nAls Anwältin habe ich eine Frage.",
                Rolle::Lawyer,
                "Als Anwältin habe ich",
            ),
            (
                "Guten Tag\r\nAls Anwältin habe ich eine Frage.",
                Rolle::Lawyer,
                "Als Anwältin habe ich",
            ),
            (
                "Betreff: Erbstreit\nAls Anwältin der Muster AG habe ich eine Frage.",
                Rolle::Lawyer,
                "Als Anwältin",
            ),
            (
                "Bonjour\nEn tant qu'avocate, je dois répondre.",
                Rolle::Lawyer,
                "En tant qu'avocate, je",
            ),
            (
                "Madame, Monsieur\nComme avocate, je dois répondre.",
                Rolle::Lawyer,
                "Comme avocate, je",
            ),
            (
                "Buongiorno\nIn qualità di avvocata, vorrei sapere una cosa.",
                Rolle::Lawyer,
                "In qualità di avvocata, vorrei",
            ),
            (
                "Buongiorno\nSono avvocata e ho una domanda.",
                Rolle::Lawyer,
                "Sono avvocata",
            ),
            (
                "Buongiorno\nCome medico mi chiedo cosa fare.",
                Rolle::Doctor,
                "Come medico mi chiedo",
            ),
            (
                "Seit 2010 bin ich Ärztin\nDarf ich die Diagnose weitergeben?",
                Rolle::Doctor,
                "bin ich Ärztin",
            ),
        ]);
        // Klein geschrieben setzt die nächste Zeile den Satz fort.
        for satz in [
            "Mein Bruder arbeitet\nals Anwalt, habe ich gehört.",
            "Seit wann bin ich Ärztin\nim Sinne des Gesetzes?",
            "Ab wann bin ich Ärztin?",
        ] {
            assert_eq!(kontext(satz), Vec::new(), "{satz}");
        }
    }
}
