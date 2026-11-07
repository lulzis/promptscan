//! Hinweisregeln ohne Rechtsfolge (SPEC 5.4): `vermerk` und `fundstellen`.
//!
//! * `vermerk` meldet einen Klassifizierungsvermerk nach Art. 13 ISG, der als Kopf- oder
//!   Einzelzeile steht («VERTRAULICH», «Klassifizierung: intern»).
//! * `fundstellen` meldet eine Bitte um Urteile, Fundstellen oder Literatur («nenne passende
//!   BGE», «cite la jurisprudence», «indica le sentenze pertinenti»).
//!
//! Beide melden [`Art::Hinweis`]; das rechtliche Modell ([`crate::stufen`]) macht daraus
//! HINWEIS mit [`crate::fund::Grund::Regelhinweis`]. Die Wortlisten mit Quellen stehen in
//! `lexika/vermerk.toml` und `lexika/fundstellen.toml`. Die Tests am Ende prüfen auch die
//! Abkürzungsliste `lexika/abkuerzungen.toml`, nach der [`crate::text`] Sätze trennt.

use std::ops::Range;

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, LexTreffer, Lexikon, Sprache};
use crate::normen::{self, Norm};
use crate::text::{Text, ist_wortzeichen};

/// Tag, an dem die Quellen dieses Moduls im Wortlaut gelesen wurden.
const GEPRUEFT: Stand = Stand::Geprueft("2026-11-06");

// ---------------------------------------------------------------------------
// Normen
// ---------------------------------------------------------------------------

/// Art. 13 ISG: die drei Klassifizierungsstufen.
pub const ISG_13: Norm = Norm {
    zitat: "Art. 13 ISG",
    erlass: "ISG, SR 128",
    fassung: "2025-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/232/de#art_13",
    wortlaut: "Als «intern» werden Informationen klassifiziert, deren Kenntnisnahme durch Unberechtigte die Interessen nach Artikel 1 Absatz 2 Buchstaben a–d beeinträchtigen kann. Als «vertraulich» werden Informationen klassifiziert, deren Kenntnisnahme durch Unberechtigte die Interessen nach Artikel 1 Absatz 2 Buchstaben a–d erheblich beeinträchtigen kann. Als «geheim» werden Informationen klassifiziert, deren Kenntnisnahme durch Unberechtigte die Interessen nach Artikel 1 Absatz 2 Buchstaben a–d schwerwiegend beeinträchtigen kann.",
};

/// Art. 2 Abs. 1 und 2 ISG: für wen das Gesetz gilt.
pub const ISG_2: Norm = Norm {
    zitat: "Art. 2 Abs. 1 und 2 ISG",
    erlass: "ISG, SR 128",
    fassung: "2025-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/232/de#art_2",
    wortlaut: "Dieses Gesetz gilt für die nachstehenden Behörden (verpflichtete Behörden): a. die Bundesversammlung; b. den Bundesrat; c. die eidgenössischen Gerichte; d. die Bundesanwaltschaft und die Aufsichtsbehörde über die Bundesanwaltschaft; e. die Schweizerische Nationalbank. Es gilt für die nachstehenden Organisationen (verpflichtete Organisationen): a. die Parlamentsdienste; b. die Bundesverwaltung; c. die Verwaltungen der eidgenössischen Gerichte; d. die Armee; …",
};

/// Art. 57 Abs. 1 BGerR: die Amtliche Sammlung (BGE) und die übrigen Mittel der Information.
pub const BGERR_57: Norm = Norm {
    zitat: "Art. 57 Abs. 1 BGerR",
    erlass: "BGerR, SR 173.110.131",
    fassung: "2026-02-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2006/834/de#art_57",
    wortlaut: "Das Bundesgericht informiert die Öffentlichkeit über seine Rechtsprechung mit folgenden Mitteln: a. Amtliche Sammlung der Entscheidungen des Schweizerischen Bundesgerichts (Amtliche Sammlung, BGE); b. Internet; c. öffentliche Auflage der Urteile; d. Mitteilungen an die Medien.",
};

/// Alle Hinweise dieses Moduls.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let mut treffer = vermerke(text);
    treffer.extend(fundstellen(text));
    treffer
}

// ---------------------------------------------------------------------------
// vermerk
// ---------------------------------------------------------------------------

const VERMERK: &str = "vermerk.toml";

/// Zeichen, die einen Vermerk umrahmen dürfen, ohne selbst Text zu sein
/// («**VERTRAULICH**», «[INTERN]», «— geheim —»).
const RAHMEN: &[char] = &[
    '*', '_', '-', '–', '—', ':', '.', '/', '|', '[', ']', '(', ')', '<', '>', '«', '»', '‹', '›',
    '"', '“', '”', '„', '\'', '‘', '’', '#', '=', '~', '!', '•',
];

/// Zeichen, mit denen in der Kopfzeile nach dem Vermerk der übrige Text beginnt
/// («Vertraulich – Memo», «VERTRAULICH: Entwurf»).
const TRENNER: &[char] = &[
    ':', '–', '—', '-', '/', '|', ',', ';', '(', '[', '*', '.', '!',
];

/// Teil eines Vermerks laut `merkmal` in `vermerk.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Teil {
    /// Stufe nach Art. 13 Abs. 1–3 ISG.
    Stufe(u8),
    /// Einleitendes Wort wie «Klassifizierung».
    Vorsatz,
}

fn teil(lex: &Lexikon, eintrag: usize) -> Option<Teil> {
    match lex.eintraege[eintrag].merkmal.as_deref() {
        Some("stufe-1") => Some(Teil::Stufe(1)),
        Some("stufe-2") => Some(Teil::Stufe(2)),
        Some("stufe-3") => Some(Teil::Stufe(3)),
        Some("vorsatz") => Some(Teil::Vorsatz),
        _ => None,
    }
}

/// Ein gefundener Vermerk.
#[derive(Clone, Copy, Debug)]
struct Vermerk {
    start: usize,
    ende: usize,
    stufe: u8,
    /// Eintrag der Stufe in der Wortliste.
    eintrag: usize,
}

fn nur_rahmen(s: &str) -> bool {
    s.chars().all(|c| c.is_whitespace() || RAHMEN.contains(&c))
}

fn vermerke(text: &Text) -> Vec<Treffer> {
    let lex = lexikon::lexikon(VERMERK);
    let s = text.als_str();
    let alle: Vec<(LexTreffer, Teil)> = lex
        .finde(text)
        .into_iter()
        .filter_map(|t| teil(lex, t.eintrag).map(|p| (t, p)))
        .collect();
    if alle.is_empty() {
        return Vec::new();
    }
    let kopf = (1..=text.zeilen_anzahl()).find(|&nr| !text.zeile(nr).trim().is_empty());
    let mut gefunden: Vec<Vermerk> = Vec::new();
    for nr in 1..=text.zeilen_anzahl() {
        let bereich = text.zeilenbereich(nr);
        let teile: Vec<(LexTreffer, Teil)> = abschnitt(&alle, &bereich)
            .iter()
            .copied()
            .filter(|(t, _)| t.ende <= bereich.end)
            .collect();
        // Eine Zeile mit Fragezeichen fragt nach dem Wort; sie ist kein Vermerk.
        if teile.is_empty() || s[bereich.clone()].contains('?') {
            continue;
        }
        let vermerk = match einzelzeile(s, &bereich, &teile) {
            Some(v) => Some(v),
            None if kopf == Some(nr) => kopfzeile(s, &bereich, &teile),
            None => None,
        };
        gefunden.extend(vermerk);
    }
    // Stehen verschiedene Stufen je auf einer eigenen Zeile, zählt der Text die Stufen
    // auf («- intern», «- vertraulich», «- geheim»); das ist kein Vermerk.
    if gefunden.windows(2).any(|p| p[0].stufe != p[1].stufe) {
        return Vec::new();
    }
    gefunden
        .into_iter()
        .map(|v| {
            Treffer::neu("vermerk", v.start, v.ende, Art::Hinweis)
                .mit_zusatz(
                    &format!("Art. 13 Abs. {} ISG", v.stufe),
                    // Englisch bleibt das amtliche deutsche Zitat (technik-7).
                    &format!("Art. 13 Abs. {} ISG", v.stufe),
                )
                .mit_beleg(lex.beleg(v.eintrag))
        })
        .collect()
}

/// Eine Zeile, die ausser dem Vermerk nur Rahmenzeichen enthält: «VERTRAULICH»,
/// «Klassifizierung: intern», «INTERN / INTERNE / AD USO INTERNO». Mehrere Stufenwörter
/// sind erlaubt, wenn sie dieselbe Stufe nennen (dreisprachiger Vermerk).
fn einzelzeile(s: &str, bereich: &Range<usize>, teile: &[(LexTreffer, Teil)]) -> Option<Vermerk> {
    let mut pos = bereich.start;
    let mut stufe: Option<(u8, usize)> = None;
    let mut vorsatz_offen = false;
    for &(t, p) in teile {
        if !nur_rahmen(&s[pos..t.start]) {
            return None;
        }
        match p {
            Teil::Vorsatz if vorsatz_offen => return None,
            Teil::Vorsatz => vorsatz_offen = true,
            Teil::Stufe(n) => {
                match stufe {
                    Some((m, _)) if m != n => return None,
                    Some(_) => {}
                    None => stufe = Some((n, t.eintrag)),
                }
                vorsatz_offen = false;
            }
        }
        pos = t.ende;
    }
    if vorsatz_offen || !nur_rahmen(&s[pos..bereich.end]) {
        return None;
    }
    let (n, eintrag) = stufe?;
    Some(Vermerk {
        start: teile.first()?.0.start,
        ende: teile.last()?.0.ende,
        stufe: n,
        eintrag,
    })
}

/// Die erste nichtleere Zeile beginnt mit dem Vermerk; danach endet sie oder es folgt
/// ein Trennzeichen: «Vertraulich – Memo an die Geschäftsleitung». Bindestrich und
/// Schrägstrich trennen nur nach einem Leerzeichen («Geheim-Tipp» ist kein Vermerk).
fn kopfzeile(s: &str, bereich: &Range<usize>, teile: &[(LexTreffer, Teil)]) -> Option<Vermerk> {
    let &(erster, p) = teile.first()?;
    if !nur_rahmen(&s[bereich.start..erster.start]) {
        return None;
    }
    let (stufe_t, n) = match p {
        Teil::Stufe(n) => (erster, n),
        Teil::Vorsatz => {
            let &(zweiter, q) = teile.get(1)?;
            let Teil::Stufe(n) = q else { return None };
            if !nur_rahmen(&s[erster.ende..zweiter.start]) {
                return None;
            }
            (zweiter, n)
        }
    };
    let rest = &s[stufe_t.ende..bereich.end];
    let getrennt = match rest.trim_start().chars().next() {
        None => true,
        Some('-' | '/') => rest.starts_with(char::is_whitespace),
        Some(c) => TRENNER.contains(&c),
    };
    getrennt.then_some(Vermerk {
        start: erster.start,
        ende: stufe_t.ende,
        stufe: n,
        eintrag: stufe_t.eintrag,
    })
}

// ---------------------------------------------------------------------------
// fundstellen
// ---------------------------------------------------------------------------

const FUNDSTELLEN: &str = "fundstellen.toml";

/// Gewicht eines Substantivs laut `merkmal` in `fundstellen.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gewicht {
    /// Zählt mit jeder Aufforderung: «BGE», «Rechtsprechung», «jurisprudence».
    Stark,
    /// Mehrdeutig; nur in der Mehrzahl oder mit unbestimmtem Artikel: «Urteile».
    Schwach,
    /// Wie `Schwach`, dazu muss der Satz ein Gericht nennen: «Entscheide», «arrêts».
    SchwachGericht,
    /// Wie `Schwach`, ohne Fragewort und nicht «Quellen des …rechts».
    Quelle,
    /// Meldet nie selbst; nennt das Gericht für `SchwachGericht`.
    Gericht,
}

fn gewicht(lex: &Lexikon, eintrag: usize) -> Option<Gewicht> {
    match lex.eintraege[eintrag].merkmal.as_deref() {
        Some("stark") => Some(Gewicht::Stark),
        Some("schwach") => Some(Gewicht::Schwach),
        Some("schwach-gericht") => Some(Gewicht::SchwachGericht),
        Some("quelle") => Some(Gewicht::Quelle),
        Some("gericht") => Some(Gewicht::Gericht),
        _ => None,
    }
}

// Die Aufforderungswörter sind eine feste grammatische Liste (Imperativ, Infinitiv,
// Höflichkeitsform, Fragewort, Präposition, Artikel), keine Wortliste mit Einzelbeleg.

/// Deutsch: Imperativ 2. Person Einzahl; «brauche», «benötige» als 1. Person.
const DE_IMPERATIV: &[&str] = &[
    "nenne",
    "nenn",
    "zitiere",
    "gib",
    "suche",
    "finde",
    "zeige",
    "zeig",
    "belege",
    "recherchiere",
    "verweise",
    "ergänze",
    "erwähne",
    "empfiehl",
    "sammle",
    "liste",
    "führe",
    "füge",
    "brauche",
    "benötige",
];
/// Deutsch: Formen, die auch nach «ich» eine Bitte sind («ich suche Urteile»).
const DE_ICH: &[&str] = &["suche", "brauche", "benötige"];
/// Deutsch: Verben, die nur mit ihrer Ergänzung bitten («liste … auf», «führe … an»).
const DE_PARTIKEL: &[(&str, &[&str])] = &[
    ("liste", &["auf"]),
    ("listen", &["auf"]),
    ("führe", &["auf", "an"]),
    ("führen", &["auf", "an"]),
    ("füge", &["hinzu", "an", "ein", "bei"]),
    ("fügen", &["hinzu", "an", "ein", "bei"]),
    ("verweise", &["auf"]),
    ("verweisen", &["auf"]),
];
/// Deutsch: Höflichkeitsform, wenn «Sie» folgt («Nennen Sie …»).
const DE_SIE: &[&str] = &[
    "nennen",
    "zitieren",
    "geben",
    "suchen",
    "zeigen",
    "belegen",
    "recherchieren",
    "verweisen",
    "ergänzen",
    "erwähnen",
    "empfehlen",
    "sammeln",
    "listen",
    "führen",
    "fügen",
];
/// Deutsch: Infinitive, die mit einem Bittwort bitten («Kannst du … nennen?»).
const DE_INFINITIV: &[&str] = &[
    "nennen",
    "zitieren",
    "angeben",
    "anzugeben",
    "auflisten",
    "aufzulisten",
    "aufführen",
    "aufzuführen",
    "anführen",
    "anzuführen",
    "aufzählen",
    "aufzuzählen",
    "suchen",
    "heraussuchen",
    "herauszusuchen",
    "raussuchen",
    "finden",
    "zeigen",
    "belegen",
    "recherchieren",
    "geben",
    "erwähnen",
    "empfehlen",
    "sammeln",
    "liefern",
    "ergänzen",
    "verweisen",
];
/// Deutsch: Bittwörter.
const DE_BITTE: &[&str] = &[
    "bitte",
    "kannst",
    "könntest",
    "würdest",
    "möchte",
    "möchtest",
    "magst",
    "wäre",
    "wären",
];
/// Deutsch: Bittwörter, die nur vor «Sie» bitten («Können Sie …»).
const DE_BITTE_SIE: &[&str] = &["können", "könnten", "würden"];
/// Deutsch: Begleiter eines Substantivs; danach ist «Suche», «Liste», «Belege» ein Nomen.
const DE_BEGLEITER: &[&str] = &[
    "der", "die", "das", "den", "dem", "des", "ein", "eine", "einer", "einem", "einen", "eines",
    "kein", "keine", "keiner", "keinem", "keinen", "meine", "deine", "seine", "ihre", "unsere",
    "eure", "jede", "jeder", "jedes", "diese", "dieser", "dieses", "zur", "zum", "beim", "im",
    "am", "vom",
];
/// Deutsch: Subjekte, nach denen die Verbform keine Bitte ist («ich finde», «er nenne»).
const DE_SUBJEKT: &[&str] = &["ich", "er", "es", "man", "sie"];
/// Deutsch: Präpositionen unmittelbar vor dem Substantiv («mit Fundstellen»).
const DE_PRAEP: &[&str] = &[
    "mit",
    "samt",
    "inklusive",
    "inkl",
    "einschliesslich",
    "einschließlich",
    "nebst",
];
/// Deutsch: bestimmte Begleiter; zwischen Präposition und Substantiv heben sie die Bitte
/// auf («mit der Rechtsprechung vereinbar»).
const DE_BESTIMMT: &[&str] = &[
    "der", "die", "das", "den", "dem", "des", "diese", "dieser", "diesem", "diesen", "dieses",
    "seine", "seiner", "seinem", "ihre", "ihrer", "ihrem", "deren", "dessen",
];
/// Deutsch: unbestimmte Begleiter, mit denen ein mehrdeutiges Substantiv zählt.
const DE_UNBESTIMMT: &[&str] = &[
    "ein", "eine", "einen", "einem", "einer", "eines", "einige", "einigen", "einiger", "mehrere",
    "mehreren", "weitere", "weiteren", "andere", "anderen",
];
/// Deutsch: Verneinung unmittelbar vor dem Substantiv.
const DE_OHNE: &[&str] = &[
    "kein", "keine", "keinen", "keinem", "keiner", "keines", "ohne",
];
/// Deutsch: Fragewörter unmittelbar vor dem Substantiv.
const DE_AUSWAHL: &[&str] = &["welche", "welcher", "welches", "welchen", "welchem"];

/// Französisch: Imperativ 2. Person Einzahl; gleich lautend mit dem Indikativ, darum nur
/// am Satzanfang, mit Bindestrich-Pronomen oder nach «et», «puis» oder Komma.
const FR_IMPERATIF: &[&str] = &[
    "cite",
    "indique",
    "mentionne",
    "donne",
    "liste",
    "énumère",
    "trouve",
    "cherche",
    "recherche",
    "fournis",
    "signale",
    "recommande",
    "propose",
    "ajoute",
    "précise",
    "rappelle",
    "montre",
    "joins",
];
/// Französisch: Imperativ 2. Person Mehrzahl (nicht nach «vous»).
const FR_IMPERATIF_VOUS: &[&str] = &[
    "citez",
    "indiquez",
    "mentionnez",
    "donnez",
    "listez",
    "énumérez",
    "trouvez",
    "cherchez",
    "recherchez",
    "fournissez",
    "signalez",
    "recommandez",
    "proposez",
    "ajoutez",
    "précisez",
    "rappelez",
    "montrez",
    "joignez",
];
/// Französisch: Infinitive, die mit einem Bittwort bitten («Pourriez-vous indiquer …»).
const FR_INFINITIF: &[&str] = &[
    "citer",
    "indiquer",
    "mentionner",
    "donner",
    "lister",
    "énumérer",
    "trouver",
    "chercher",
    "rechercher",
    "fournir",
    "signaler",
    "recommander",
    "proposer",
    "ajouter",
    "préciser",
    "rappeler",
    "montrer",
    "joindre",
];
/// Französisch: Bittwörter.
const FR_DEMANDE: &[&str] = &[
    "peux",
    "pouvez",
    "pourrais",
    "pourriez",
    "veuillez",
    "merci",
    "voudrais",
    "aimerais",
    "souhaite",
    "souhaiterais",
    "demande",
    "stp",
    "svp",
    "plaît",
];
/// Französisch: Pronomen nach Bindestrich beim Imperativ («donne-moi»).
const FR_ENCLITIQUE: &[&str] = &["moi", "nous", "en", "les", "la", "le", "lui", "leur", "y"];
/// Französisch: Bindewörter vor einem zweiten Imperativ («… et cite …»).
const FR_LIEN: &[&str] = &["et", "puis", "ensuite", "aussi", "également", "enfin"];
/// Französisch: Satzanfänge mit Begleiter oder Pronomen; danach ist «cite» Indikativ.
const FR_DEBUT_NOMINAL: &[&str] = &[
    "le", "la", "les", "l", "un", "une", "des", "du", "ce", "cet", "cette", "ces", "il", "elle",
    "ils", "elles", "on", "je", "j", "nous", "vous", "tu", "mon", "ma", "mes", "son", "sa", "ses",
    "leur", "leurs", "notre", "nos", "votre", "vos", "au", "aux", "qui",
];
/// Französisch: Präposition unmittelbar vor dem Substantiv («avec des références»).
const FR_PRAEP: &[&str] = &["avec"];
/// Französisch: bestimmte Begleiter (heben eine Präposition davor auf).
const FR_DEFINI: &[&str] = &[
    "le", "la", "les", "l", "du", "au", "aux", "ce", "cet", "cette", "ces", "sa", "son", "ses",
    "leur", "leurs",
];
/// Französisch: unbestimmte Begleiter.
const FR_INDEFINI: &[&str] = &["un", "une", "quelque"];
/// Französisch: Verneinung vor dem Substantiv.
const FR_SANS: &[&str] = &["aucun", "aucune", "sans"];
/// Französisch: Fragewörter.
const FR_AUSWAHL: &[&str] = &["quel", "quels", "quelle", "quelles"];

/// Italienisch: Imperativ mit angehängtem «mi» oder «-temi»; überall eine Bitte.
const IT_IMPERATIVO_CLITICO: &[&str] = &[
    "citami",
    "indicami",
    "elencami",
    "trovami",
    "cercami",
    "segnalami",
    "suggeriscimi",
    "consigliami",
    "forniscimi",
    "mostrami",
    "riportami",
    "dammi",
    "dimmi",
    "citatemi",
    "indicatemi",
    "elencatemi",
    "trovatemi",
    "cercatemi",
    "segnalatemi",
    "suggeritemi",
    "consigliatemi",
    "fornitemi",
    "mostratemi",
    "riportatemi",
    "datemi",
    "ditemi",
];
/// Italienisch: Imperativ 2. Person Einzahl und Mehrzahl; gleich lautend mit Indikativ
/// und Partizip, darum nur am Satzanfang oder nach «e», «poi» oder Komma.
const IT_IMPERATIVO: &[&str] = &[
    "cita",
    "indica",
    "menziona",
    "elenca",
    "fornisci",
    "trova",
    "cerca",
    "segnala",
    "suggerisci",
    "consiglia",
    "riporta",
    "aggiungi",
    "mostra",
    "citate",
    "indicate",
    "menzionate",
    "elencate",
    "fornite",
    "trovate",
    "cercate",
    "segnalate",
    "suggerite",
    "consigliate",
    "riportate",
    "aggiungete",
    "mostrate",
];
/// Italienisch: Höflichkeitsform (Lei), am Satzanfang oder nach «mi», «ci».
const IT_LEI: &[&str] = &[
    "citi",
    "indichi",
    "menzioni",
    "elenchi",
    "fornisca",
    "trovi",
    "cerchi",
    "segnali",
    "suggerisca",
    "consigli",
    "riporti",
    "aggiunga",
    "mostri",
];
/// Italienisch: Infinitive, die mit einem Bittwort bitten («Puoi indicarmi …»).
const IT_INFINITO: &[&str] = &[
    "citare",
    "indicare",
    "menzionare",
    "elencare",
    "fornire",
    "trovare",
    "cercare",
    "segnalare",
    "suggerire",
    "consigliare",
    "riportare",
    "aggiungere",
    "mostrare",
    "dare",
    "citarmi",
    "indicarmi",
    "elencarmi",
    "fornirmi",
    "trovarmi",
    "cercarmi",
    "segnalarmi",
    "suggerirmi",
    "consigliarmi",
    "mostrarmi",
    "riportarmi",
    "darmi",
    "dirmi",
];
/// Italienisch: Bittwörter.
const IT_RICHIESTA: &[&str] = &[
    "puoi", "potresti", "potete", "potreste", "vorrei", "prego", "favore", "piacere", "chiedo",
];
/// Italienisch: Bittwörter, die nur in einer Frage bitten («Può indicarmi …?»).
const IT_RICHIESTA_FRAGE: &[&str] = &["può", "potrebbe"];
/// Italienisch: Bindewörter vor einem zweiten Imperativ.
const IT_LEGAME: &[&str] = &["e", "poi", "inoltre", "anche", "quindi", "infine"];
/// Italienisch: Satzanfänge mit Begleiter oder Pronomen.
const IT_INIZIO_NOMINALE: &[&str] = &[
    "il", "lo", "la", "i", "gli", "le", "l", "un", "uno", "una", "questo", "questa", "questi",
    "queste", "quel", "quella", "egli", "ella", "esso", "essa", "lui", "lei", "loro", "io", "noi",
    "voi", "tu", "mio", "mia", "suo", "sua", "nostro", "nostra", "che", "chi",
];
/// Italienisch: Präposition unmittelbar vor dem Substantiv («con riferimenti»).
const IT_PRAEP: &[&str] = &["con"];
/// Italienisch: bestimmte Begleiter (heben eine Präposition davor auf).
const IT_DETERMINATIVO: &[&str] = &[
    "il", "lo", "la", "i", "gli", "le", "l", "del", "dello", "della", "dei", "degli", "delle",
    "al", "allo", "alla", "ai", "agli", "alle", "questo", "questa", "questi", "queste", "suo",
    "sua", "suoi", "sue",
];
/// Italienisch: unbestimmte Begleiter.
const IT_INDETERMINATIVO: &[&str] = &["un", "uno", "una", "qualche"];
/// Italienisch: Verneinung vor dem Substantiv.
const IT_SENZA: &[&str] = &["nessun", "nessuno", "nessuna", "senza"];
/// Italienisch: Fragewörter.
const IT_AUSWAHL: &[&str] = &["quale", "quali", "qual"];

/// Wie weit Aufforderung und Substantiv auseinander stehen dürfen, damit die Markierung
/// beide umfasst (in Wörtern).
const NAH: usize = 6;

fn enthaelt(liste: &[&str], wort: &str) -> bool {
    liste.contains(&wort)
}

/// Ein Wort des Satzes: Byte-Bereich im Text und Kleinschreibung.
#[derive(Clone, Debug)]
struct Wort {
    start: usize,
    ende: usize,
    klein: String,
}

/// Eine Aufforderung, die für den ganzen Satz gilt (Verb oder Frage).
#[derive(Clone, Copy, Debug)]
struct Marke {
    /// Index in den Wörtern des Satzes.
    wort: usize,
    sprache: Sprache,
}

/// Ein Satz mit Wörtern und Aufforderungen.
struct Satz<'a> {
    s: &'a str,
    w: Vec<Wort>,
    marken: Vec<Marke>,
    /// Nennt der Satz ein Gericht («Bundesgericht», «tribunal»)?
    gericht: bool,
}

/// Wo im Satz eine Bitte um Fundstellen steht.
#[derive(Clone, Copy, Debug)]
struct Bitte {
    /// Erstes und letztes Wort des Substantivs.
    erstes: usize,
    letztes: usize,
    /// Wort der nächsten Aufforderung.
    marke: usize,
}

/// Die Treffer, die in `bereich` beginnen. `alle` ist nach dem Anfang sortiert, wie
/// [`Lexikon::finde`] liefert; die binäre Suche kostet je Zeile oder Satz O(log k)
/// statt eines Durchlaufs über alle k Treffer.
fn abschnitt<'a, T>(alle: &'a [(LexTreffer, T)], bereich: &Range<usize>) -> &'a [(LexTreffer, T)] {
    let von = alle.partition_point(|(t, _)| t.start < bereich.start);
    let bis = alle.partition_point(|(t, _)| t.start < bereich.end);
    &alle[von..bis]
}

fn fundstellen(text: &Text) -> Vec<Treffer> {
    let lex = lexikon::lexikon(FUNDSTELLEN);
    let s = text.als_str();
    let alle: Vec<(LexTreffer, Gewicht)> = lex
        .finde(text)
        .into_iter()
        .filter_map(|t| gewicht(lex, t.eintrag).map(|g| (t, g)))
        .collect();
    let mut aus = Vec::new();
    for bereich in text.saetze() {
        let im_satz: Vec<(LexTreffer, Gewicht)> = abschnitt(&alle, bereich)
            .iter()
            .copied()
            .filter(|(t, _)| t.ende <= bereich.end)
            .collect();
        if im_satz.iter().all(|(_, g)| *g == Gewicht::Gericht) {
            continue;
        }
        let w = woerter(s, bereich);
        let frage = s[bereich.clone()].contains('?');
        let marken = satzmarken(s, &w, frage);
        let satz = Satz {
            s,
            w,
            marken,
            gericht: im_satz.iter().any(|(_, g)| *g == Gewicht::Gericht),
        };
        // Ein Hinweis je Satz genügt.
        if let Some((t, b)) = im_satz
            .iter()
            .find_map(|&(t, g)| bitte_um(lex, &satz, t, g).map(|b| (t, b)))
        {
            aus.push(stelle(lex, &satz, t, b));
        }
    }
    aus
}

/// Die Wörter eines Bereichs: Folgen von Buchstaben und Ziffern.
fn woerter(s: &str, bereich: &Range<usize>) -> Vec<Wort> {
    let mut aus = Vec::new();
    let mut anfang: Option<usize> = None;
    let schliessen = |a: usize, e: usize, aus: &mut Vec<Wort>| {
        aus.push(Wort {
            start: a,
            ende: e,
            klein: s[a..e].to_lowercase(),
        });
    };
    for (i, c) in s[bereich.clone()].char_indices() {
        let i = bereich.start + i;
        if ist_wortzeichen(c) {
            anfang.get_or_insert(i);
        } else if let Some(a) = anfang.take() {
            schliessen(a, i, &mut aus);
        }
    }
    if let Some(a) = anfang {
        schliessen(a, bereich.end, &mut aus);
    }
    aus
}

/// Ein Aufzählungszeichen wie «1» in «1.» oder «a» in «a)».
fn aufzaehlung(s: &str, x: &Wort) -> bool {
    x.klein.chars().all(|c| c.is_ascii_digit())
        || (x.klein.chars().count() == 1 && matches!(s[x.ende..].chars().next(), Some(')' | '.')))
}

/// Steht das Wort am Satzanfang? Davor höchstens Aufzählungszeichen, im Satz oder seit dem
/// letzten weichen Umbruch: Nach einer Anrede ohne Satzzeichen beginnt die nächste Zeile
/// wie ein Satz («Bonjour⏎Cite les arrêts …», `Text::satzanfang_bei`). Ein `\n` zwischen
/// zwei Wörtern desselben Satzes ist immer ein weicher Umbruch (`Text::saetze`).
fn am_satzanfang(s: &str, w: &[Wort], i: usize) -> bool {
    let ab = (1..=i)
        .rev()
        .find(|&j| s[w[j - 1].ende..w[j].start].contains('\n'))
        .unwrap_or(0);
    w[ab..i].iter().all(|x| aufzaehlung(s, x))
}

/// Steht das Wort nach einem Bindewort oder nach Komma, Strichpunkt oder Doppelpunkt?
fn nach_bindewort(s: &str, w: &[Wort], i: usize, bindewoerter: &[&str]) -> bool {
    i > 0
        && (enthaelt(bindewoerter, &w[i - 1].klein)
            || s[w[i - 1].ende..w[i].start]
                .trim_end()
                .ends_with([',', ';', ':']))
}

/// Folgt unmittelbar «Sie» (Höflichkeitsform)?
fn folgt_sie(s: &str, w: &[Wort], i: usize) -> bool {
    w.get(i + 1).is_some_and(|x| &s[x.start..x.ende] == "Sie")
}

/// Ist die Aufforderung verneint («nicht zitieren», «ne … pas», «non citare»)?
fn verneint(w: &[Wort], i: usize, sprache: Sprache) -> bool {
    let davor = i.checked_sub(1).map(|j| w[j].klein.as_str());
    match sprache {
        Sprache::De => {
            davor == Some("nicht")
                || (w.get(i + 1).is_some_and(|x| x.klein == "nicht")
                    && w.get(i + 2).is_none_or(|x| x.klein != "nur"))
        }
        Sprache::Fr => matches!(davor, Some("ne" | "n" | "pas" | "sans")),
        Sprache::It => matches!(davor, Some("non" | "senza")),
    }
}

/// Aufforderungen, die für den ganzen Satz gelten: Verben und Fragen nach dem Bestand.
fn satzmarken(s: &str, w: &[Wort], frage: bool) -> Vec<Marke> {
    let mut m = Vec::new();
    verben_de(s, w, &mut m);
    verben_fr(s, w, &mut m);
    verben_it(s, w, frage, &mut m);
    if frage {
        fragen(w, &mut m);
    }
    m
}

/// Braucht das Verb eine Ergänzung («liste … auf»), und steht sie im Satz?
fn partikel_da(s: &str, w: &[Wort], i: usize) -> bool {
    let k = w[i].klein.as_str();
    match DE_PARTIKEL.iter().find(|(verb, _)| *verb == k) {
        None => true,
        Some((_, partikel)) => {
            w[i + 1..].iter().any(|x| enthaelt(partikel, &x.klein))
                || (k == "liste" && am_satzanfang(s, w, i))
        }
    }
}

fn verben_de(s: &str, w: &[Wort], aus: &mut Vec<Marke>) {
    let bitte = w.iter().enumerate().any(|(i, x)| {
        enthaelt(DE_BITTE, &x.klein) || (enthaelt(DE_BITTE_SIE, &x.klein) && folgt_sie(s, w, i))
    });
    for (i, x) in w.iter().enumerate() {
        let k = x.klein.as_str();
        let davor = i.checked_sub(1).map(|j| w[j].klein.as_str());
        let ist = if enthaelt(DE_IMPERATIV, k) {
            let begleiter = davor.is_some_and(|d| enthaelt(DE_BEGLEITER, d));
            let subjekt = davor
                .is_some_and(|d| enthaelt(DE_SUBJEKT, d) && !(d == "ich" && enthaelt(DE_ICH, k)));
            !begleiter && !subjekt && partikel_da(s, w, i)
        } else if enthaelt(DE_SIE, k) && folgt_sie(s, w, i) {
            partikel_da(s, w, i)
        } else {
            // «Bitte finden Sie anbei …» ist keine Bitte um Fundstellen.
            bitte && enthaelt(DE_INFINITIV, k) && !folgt_sie(s, w, i)
        };
        if ist && !verneint(w, i, Sprache::De) {
            aus.push(Marke {
                wort: i,
                sprache: Sprache::De,
            });
        }
    }
}

/// Folgt dem französischen Imperativ ein Pronomen mit Bindestrich («donne-moi»)?
fn mit_bindestrich_pronomen(s: &str, w: &[Wort], i: usize) -> bool {
    let x = &w[i];
    let strich = s[x.ende..].chars().next();
    matches!(strich, Some('-' | '‑'))
        && w.get(i + 1).is_some_and(|n| {
            n.start == x.ende + strich.map_or(0, char::len_utf8)
                && enthaelt(FR_ENCLITIQUE, &n.klein)
        })
}

fn verben_fr(s: &str, w: &[Wort], aus: &mut Vec<Marke>) {
    let demande = w.iter().any(|x| enthaelt(FR_DEMANDE, &x.klein));
    let debut_nominal = w
        .first()
        .is_some_and(|x| enthaelt(FR_DEBUT_NOMINAL, &x.klein));
    for (i, x) in w.iter().enumerate() {
        let k = x.klein.as_str();
        let ist = if enthaelt(FR_IMPERATIF, k) {
            am_satzanfang(s, w, i)
                || mit_bindestrich_pronomen(s, w, i)
                || (!debut_nominal && nach_bindewort(s, w, i, FR_LIEN))
        } else if enthaelt(FR_IMPERATIF_VOUS, k) {
            !(i > 0 && w[i - 1].klein == "vous")
        } else {
            demande && enthaelt(FR_INFINITIF, k)
        };
        if ist && !verneint(w, i, Sprache::Fr) {
            aus.push(Marke {
                wort: i,
                sprache: Sprache::Fr,
            });
        }
    }
}

fn verben_it(s: &str, w: &[Wort], frage: bool, aus: &mut Vec<Marke>) {
    let richiesta = w.iter().any(|x| {
        enthaelt(IT_RICHIESTA, &x.klein) || (frage && enthaelt(IT_RICHIESTA_FRAGE, &x.klein))
    });
    let inizio_nominale = w
        .first()
        .is_some_and(|x| enthaelt(IT_INIZIO_NOMINALE, &x.klein));
    for (i, x) in w.iter().enumerate() {
        let k = x.klein.as_str();
        let davor = i.checked_sub(1).map(|j| w[j].klein.as_str());
        let ist = if enthaelt(IT_IMPERATIVO_CLITICO, k) {
            true
        } else if enthaelt(IT_IMPERATIVO, k) {
            am_satzanfang(s, w, i) || (!inizio_nominale && nach_bindewort(s, w, i, IT_LEGAME))
        } else if enthaelt(IT_LEI, k) {
            am_satzanfang(s, w, i) || matches!(davor, Some("mi" | "ci"))
        } else {
            richiesta && enthaelt(IT_INFINITO, k)
        };
        if ist && !verneint(w, i, Sprache::It) {
            aus.push(Marke {
                wort: i,
                sprache: Sprache::It,
            });
        }
    }
}

/// Fragen nach dem Bestand; nur in einem Satz mit Fragezeichen.
fn fragen(w: &[Wort], aus: &mut Vec<Marke>) {
    let hat_es = w.iter().any(|x| x.klein == "es");
    for (i, x) in w.iter().enumerate() {
        let naechstes = w.get(i + 1).map(|n| n.klein.as_str());
        let sprache = match x.klein.as_str() {
            "gibt" if hat_es => Some(Sprache::De),
            "kennst" | "existiert" | "existieren" => Some(Sprache::De),
            "kennen" if naechstes == Some("sie") => Some(Sprache::De),
            "existe" | "existent" | "connais" | "connaissez" => Some(Sprache::Fr),
            "y" if naechstes == Some("a") => Some(Sprache::Fr),
            "esiste" | "esistono" | "conosci" | "conosce" => Some(Sprache::It),
            "c" if naechstes == Some("è") => Some(Sprache::It),
            "ci" if naechstes == Some("sono") => Some(Sprache::It),
            _ => None,
        };
        if let Some(sprache) = sprache {
            aus.push(Marke { wort: i, sprache });
        }
    }
}

/// Die zwei Wörter vor dem Substantiv.
fn zwei_vor(erstes: usize) -> Range<usize> {
    erstes.saturating_sub(2)..erstes
}

fn liste_vor(w: &[Wort], erstes: usize, liste: &[&str]) -> Option<usize> {
    zwei_vor(erstes).find(|&j| enthaelt(liste, &w[j].klein))
}

/// Fragewort unmittelbar vor dem Substantiv («Welche Leitentscheide …», «Quels arrêts …»).
fn auswahl_vor(w: &[Wort], erstes: usize, sprache: Sprache) -> Option<usize> {
    let liste = match sprache {
        Sprache::De => DE_AUSWAHL,
        Sprache::Fr => FR_AUSWAHL,
        Sprache::It => IT_AUSWAHL,
    };
    liste_vor(w, erstes, liste)
}

/// Präposition unmittelbar vor dem Substantiv, ohne bestimmten Begleiter dazwischen
/// («mit Fundstellen», «avec des références», «con riferimenti»); dazu «unter Angabe
/// der Fundstellen» und «y compris».
fn praeposition_vor(w: &[Wort], erstes: usize, sprache: Sprache) -> Option<usize> {
    let (praep, bestimmt) = match sprache {
        Sprache::De => (DE_PRAEP, DE_BESTIMMT),
        Sprache::Fr => (FR_PRAEP, FR_DEFINI),
        Sprache::It => (IT_PRAEP, IT_DETERMINATIVO),
    };
    zwei_vor(erstes).rev().find_map(|j| {
        let k = w[j].klein.as_str();
        let vorher = j.checked_sub(1).map(|v| w[v].klein.as_str());
        if sprache == Sprache::De && k == "angabe" && vorher == Some("unter") {
            return Some(j - 1);
        }
        let frei = w[j + 1..erstes]
            .iter()
            .all(|x| !enthaelt(bestimmt, &x.klein));
        let ist =
            enthaelt(praep, k) || (sprache == Sprache::Fr && k == "compris" && vorher == Some("y"));
        (frei && ist).then_some(j)
    })
}

/// «Was sagt …», «Que dit …», «Qu’en dit …», «Cosa dice …» bis vier Wörter vor dem
/// Substantiv. Gibt das Fragewort zurück.
fn was_sagt_vor(w: &[Wort], erstes: usize, sprache: Sprache) -> Option<usize> {
    let klein = |j: Option<usize>| j.map(|j| w[j].klein.as_str());
    (erstes.saturating_sub(4)..erstes).find_map(|j| {
        let k = w[j].klein.as_str();
        let v1 = j.checked_sub(1);
        let v2 = j.checked_sub(2);
        match sprache {
            Sprache::De if matches!(k, "sagt" | "sagen") && klein(v1) == Some("was") => v1,
            Sprache::Fr if matches!(k, "dit" | "disent") => {
                if matches!(klein(v1), Some("que" | "qu")) {
                    v1
                } else if klein(v1) == Some("en") && klein(v2) == Some("qu") {
                    v2
                } else {
                    None
                }
            }
            Sprache::It
                if matches!(k, "dice" | "dicono") && matches!(klein(v1), Some("cosa" | "che")) =>
            {
                v1
            }
            _ => None,
        }
    })
}

/// Folgt dem Substantiv eine Zahl auf derselben Zeile («BGE 136 II 508», «Urteil
/// 4A_123/2020»), zitiert der Prompt selbst.
fn zitat_folgt(s: &str, ende: usize) -> bool {
    s[ende..]
        .chars()
        .find(|&c| !(c.is_whitespace() && c != '\n'))
        .is_some_and(|c| c.is_ascii_digit())
}

/// «Quellen des Völkerrechts», «sources du droit», «fonti del diritto»: Rechtsquellen,
/// keine Fundstellen.
fn recht_folgt(w: &[Wort], letztes: usize, sprache: Sprache) -> bool {
    w.iter().skip(letztes + 1).take(3).any(|x| match sprache {
        Sprache::De => x.klein.ends_with("recht") || x.klein.ends_with("rechts"),
        Sprache::Fr => x.klein == "droit",
        Sprache::It => x.klein == "diritto",
    })
}

fn unbestimmt_vor(w: &[Wort], erstes: usize, sprache: Sprache) -> bool {
    let liste = match sprache {
        Sprache::De => DE_UNBESTIMMT,
        Sprache::Fr => FR_INDEFINI,
        Sprache::It => IT_INDETERMINATIVO,
    };
    liste_vor(w, erstes, liste).is_some()
}

fn verneint_vor(w: &[Wort], erstes: usize, sprache: Sprache) -> bool {
    let liste = match sprache {
        Sprache::De => DE_OHNE,
        Sprache::Fr => FR_SANS,
        Sprache::It => IT_SENZA,
    };
    liste_vor(w, erstes, liste).is_some()
}

/// Zählt das Substantiv als Bitte um Fundstellen?
fn bitte_um(lex: &Lexikon, satz: &Satz, t: LexTreffer, g: Gewicht) -> Option<Bitte> {
    if g == Gewicht::Gericht || zitat_folgt(satz.s, t.ende) {
        return None;
    }
    let e = &lex.eintraege[t.eintrag];
    let sprache = e.sprache;
    let w = &satz.w;
    let erstes = w.iter().position(|x| x.start >= t.start)?;
    let letztes = w.iter().rposition(|x| x.ende <= t.ende)?;
    if letztes < erstes || verneint_vor(w, erstes, sprache) {
        return None;
    }
    if g != Gewicht::Stark {
        let gefunden = &satz.s[t.start..t.ende];
        let mehrzahl =
            lexikon::falten(gefunden, true).text != lexikon::falten(&e.begriff, true).text;
        // Deutsche Substantive sind gross geschrieben; am Satzanfang kann «Entscheide»
        // auch der Imperativ sein.
        let deutsch_unsicher =
            sprache == Sprache::De && (erstes == 0 || !gefunden.starts_with(char::is_uppercase));
        if deutsch_unsicher
            || !(mehrzahl || unbestimmt_vor(w, erstes, sprache))
            || (g == Gewicht::SchwachGericht && !satz.gericht)
            || (g == Gewicht::Quelle && recht_folgt(w, letztes, sprache))
        {
            return None;
        }
    }
    let mut kandidaten: Vec<usize> = satz
        .marken
        .iter()
        .filter(|m| m.sprache == sprache)
        .map(|m| m.wort)
        .collect();
    if g != Gewicht::Quelle {
        kandidaten.extend(auswahl_vor(w, erstes, sprache));
    }
    kandidaten.extend(praeposition_vor(w, erstes, sprache));
    if g == Gewicht::Stark {
        kandidaten.extend(was_sagt_vor(w, erstes, sprache));
    }
    let marke = kandidaten.into_iter().min_by_key(|&m| m.abs_diff(erstes))?;
    Some(Bitte {
        erstes,
        letztes,
        marke,
    })
}

/// Der Treffer: von der Aufforderung bis zum Substantiv, wenn sie nahe beieinander
/// stehen, sonst nur das Substantiv.
fn stelle(lex: &Lexikon, satz: &Satz, t: LexTreffer, b: Bitte) -> Treffer {
    let w = &satz.w;
    let (start, ende) = if b.marke < b.erstes && b.erstes - b.marke <= NAH {
        (w[b.marke].start, t.ende)
    } else if b.marke > b.letztes && b.marke - b.letztes <= NAH {
        (t.start, w[b.marke].ende)
    } else {
        (t.start, t.ende)
    };
    Treffer::neu("fundstellen", start, ende, Art::Hinweis).mit_beleg(lex.beleg(t.eintrag))
}

// ---------------------------------------------------------------------------
// Katalog
// ---------------------------------------------------------------------------

/// Beschreibung der Regeln `vermerk` und `fundstellen`.
pub fn info() -> Vec<RegelInfo> {
    vec![info_vermerk(), info_fundstellen()]
}

fn info_vermerk() -> RegelInfo {
    RegelInfo {
        id: "vermerk",
        klasse: Klasse::Hinweis,
        titel: Txt::neu("Klassifizierungsvermerk", "Classification marking"),
        erkennt: Txt::neu(
            "Einen Klassifizierungsvermerk nach Art. 13 ISG («intern», «vertraulich», «geheim»; französisch «interne», «confidentiel», «secret»; italienisch «ad uso interno», «confidenziale», «segreto») als Kopf- oder Einzelzeile.",
            "A classification marking under Art. 13 ISG («intern», «vertraulich», «geheim»; French «interne», «confidentiel», «secret»; Italian «ad uso interno», «confidenziale», «segreto») as a header or stand-alone line.",
        ),
        methode: Txt::neu(
            "Die Wortliste enthält die drei Stufen aus Art. 13 ISG und die einleitenden Wörter aus Art. 11 ISG und Art. 18–20 ISV, je im amtlichen deutschen, französischen und italienischen Wortlaut. Eine Zeile zählt, wenn sie ausser der Stufe und allenfalls «Klassifizierung:» nur Satz- oder Zierzeichen enthält, oder wenn die erste nichtleere Zeile mit der Stufe beginnt und danach endet oder ein Trennzeichen folgt («Vertraulich – Memo»). Zeilen mit Fragezeichen zählen nicht; nennen solche Zeilen verschiedene Stufen, ist das eine Aufzählung und zählt ebenfalls nicht. Dasselbe Wort mitten im Text («bitte vertraulich behandeln») ist kein Vermerk.",
            "The word list holds the three levels of Art. 13 ISG and the introducing words of Art. 11 ISG and Art. 18–20 ISV, each in the official German, French and Italian wording. A line counts if, apart from the level and possibly «Klassifizierung:», it holds only punctuation or decoration, or if the first non-empty line starts with the level and then ends or continues after a separator («Vertraulich – Memo»). Lines with a question mark do not count; if such lines name different levels, they are an enumeration and do not count either. The same word in running text («bitte vertraulich behandeln») is not a marking.",
        ),
        // Für wen der Vermerk gilt, sagt die Frage unter «Klären» (Rechtstext A5 «vermerk»,
        // mit Art. 3 ISG); der frühere Satz «Das ISG gilt für Bundesstellen; für Private …»
        // liess die Kantone und Private mit Bundesaufgaben aus.
        hinweis: Some(Txt::neu(
            "Klassifizierungsvermerk (Art. 13 ISG).",
            "Classification marking (Art. 13 ISG).",
        )),
        grundlage: vec![&ISG_13, &ISG_2, &normen::ISG_3],
        quellen: vec![
            Quelle {
                titel: "Informationssicherheitsgesetz (ISG, SR 128), Art. 1, 2, 11 und 13, Fassung vom 1. Oktober 2025, deutsch, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2022/232/de",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Informationssicherheitsverordnung (ISV, SR 128.1), Art. 17 Abs. 5 (formelle Kennzeichnung) und Art. 18–20 (Sachüberschriften der Stufen), Fassung vom 1. Mai 2025, deutsch, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2023/735/de",
                lizenz: None,
                stand: GEPRUEFT,
            },
        ],
        lexika: vec![VERMERK],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Mitten im Text zählt das Wort nicht («Bitte vertraulich behandeln»): Art. 13 ISG meint die Stufe, mit der eine Information klassifiziert und ihr Träger formell gekennzeichnet wird (Art. 17 Abs. 5 ISV). Im Prompt ist so ein Satz oft eine Anweisung an das Modell, und ob eine private Abrede besteht, zeigt der Text nicht.",
                "In running text the word does not count («Bitte vertraulich behandeln»): Art. 13 ISG means the level with which information is classified and its carrier formally marked (Art. 17 Abs. 5 ISV). In a prompt such a sentence is often an instruction to the model, and the text does not show whether a private agreement exists.",
            ),
            Txt::neu(
                "Private Vermerke wie «streng vertraulich» oder «persönlich» und Stufen in anderen Sprachen («confidential») erkennt die Regel nicht.",
                "Private markings such as «streng vertraulich» or «persönlich» and levels in other languages («confidential») are not detected.",
            ),
            Txt::neu(
                "Ob die Information tatsächlich klassifiziert ist und wer sie bekanntgeben darf, entscheidet die Regel nicht.",
                "The rule does not decide whether the information is actually classified or who may disclose it.",
            ),
            Txt::neu(
                "Ein eingefügtes Dokument mitten im Prompt zählt nur, wenn sein Vermerk allein auf einer Zeile steht.",
                "A document pasted in the middle of a prompt only counts if its marking stands alone on a line.",
            ),
            Txt::neu(
                "Eine Zeile, die nur aus einem Stufenwort besteht (etwa in einer Liste), meldet die Regel auch, wenn kein Vermerk gemeint ist.",
                "A line consisting only of a level word (for instance in a list) is reported even if no marking is meant.",
            ),
        ],
    }
}

fn info_fundstellen() -> RegelInfo {
    RegelInfo {
        id: "fundstellen",
        klasse: Klasse::Hinweis,
        titel: Txt::neu(
            "Bitte um Urteile oder Fundstellen",
            "Request for case law or citations",
        ),
        erkennt: Txt::neu(
            "Eine Bitte um Urteile, Rechtsprechung, Fundstellen oder Literatur auf Deutsch, Französisch oder Italienisch («nenne passende BGE», «mit Fundstellen», «cite la jurisprudence», «indica le sentenze pertinenti»).",
            "A request for judgments, case law, citations or literature in German, French or Italian («nenne passende BGE», «mit Fundstellen», «cite la jurisprudence», «indica le sentenze pertinenti»).",
        ),
        methode: Txt::neu(
            "Die Wortliste enthält Substantive aus amtlichen Texten (BGG, Reglement für das Bundesgericht, PublG, URG, ZGB, BV, VAM) und von der Website des Bundesgerichts. Ein Substantiv zählt nur mit einer Aufforderung derselben Sprache im selben Satz: Imperativ, Bitte mit Infinitiv, Frage nach dem Bestand («Gibt es …?»), Fragewort oder Präposition unmittelbar davor («mit Fundstellen»); verneinte Aufforderungen zählen nicht. Mehrdeutige Substantive («Urteil», «référence», «Quelle») zählen nur in der Mehrzahl oder mit unbestimmtem Artikel, «Entscheid» und «arrêt» nur, wenn der Satz ein Gericht nennt. Folgt dem Substantiv eine Zahl («BGE 136 II 508»), zitiert der Prompt selbst; das zählt nicht.",
            "The word list holds nouns from official texts (BGG, Regulations of the Federal Supreme Court, PublG, URG, ZGB, BV, VAM) and from the Federal Supreme Court's website. A noun only counts together with a request in the same language in the same sentence: imperative, request with infinitive, question about existence («Gibt es …?»), interrogative or preposition right before it («mit Fundstellen»); negated requests do not count. Ambiguous nouns («Urteil», «référence», «Quelle») only count in the plural or with an indefinite article, «Entscheid» and «arrêt» only if the sentence names a court. If a number follows the noun («BGE 136 II 508»), the prompt cites by itself; that does not count.",
        ),
        hinweis: Some(Txt::neu(
            "Sprachmodelle erfinden Urteile. Jede Fundstelle prüfen.",
            "Language models invent judgments. Check every citation.",
        )),
        // Rechtstext B5: rechtlicher Bezug Art. 12 lit. a BGFA, Grundlage der Erkennung
        // Art. 27 Abs. 1 BGG (Abs. 2 regelt die Anonymisierung) und Art. 57 Abs. 1 BGerR.
        grundlage: vec![&normen::BGFA_12_A, &normen::BGG_27_1, &BGERR_57],
        quellen: vec![
            Quelle {
                titel: "Dahl, Magesh, Suzgun, Ho: Large Legal Fictions: Profiling Legal Hallucinations in Large Language Models, Journal of Legal Analysis 16 (2024) 64–93, doi:10.1093/jla/laae003 (Zusammenfassung und Einleitung der Verlagsfassung gelesen)",
                url: "https://doi.org/10.1093/jla/laae003",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Dieselbe Arbeit, arXiv:2401.01301v2 vom 21. Juni 2024: Abbildung 1 (Anteil Halluzinationen über alle referenzbasierten Aufgaben: GPT 4 0,58, GPT 3.5 0,69, PaLM 2 0,72, Llama 2 0,88), Modelle, Stichprobe und Bedingungen in Abschnitt 4",
                url: "https://arxiv.org/abs/2401.01301v2",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Bundesgerichtsgesetz (BGG, SR 173.110), Art. 16, 23 und 27, Fassung vom 1. April 2026, deutsch, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2006/218/de",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Reglement für das Bundesgericht (BGerR, SR 173.110.131), Art. 57 und 58 sowie Art. 4 (Sachüberschrift, italienisch, für lexika/abkuerzungen.toml), Fassung vom 1. Februar 2026, deutsch, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/2006/834/de",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Schweizerisches Bundesgericht, Seite «Leitentscheide (BGE) und EGMR-Entscheide» (deutsch und französisch), abgerufen am 6. November 2026",
                url: "https://www.bger.ch/de/index/juridiction/jurisdiction-inherit-template/jurisdiction-recht/jurisdiction-recht-leitentscheide1954.htm",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Publikationsgesetz (PublG, SR 170.512), Art. 12 Abs. 2 (deutsch und französisch) sowie Art. 5 Fussnote 13 (italienisch, für lexika/abkuerzungen.toml), Fassung vom 1. September 2023",
                url: "https://www.fedlex.admin.ch/eli/cc/2004/745/de#art_12",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Urheberrechtsgesetz (URG, SR 231.1), Art. 25, Fassung vom 1. Juli 2025, deutsch, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/1993/1798_1798_1798/de#art_25",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Zivilgesetzbuch (ZGB, SR 210), Art. 1 Abs. 3, Fassung vom 1. Juli 2026, französisch und italienisch",
                url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/fr#art_1",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Bundesverfassung (BV, SR 101), Art. 30 Abs. 1, Fassung vom 3. März 2024, deutsch und französisch",
                url: "https://www.fedlex.admin.ch/eli/cc/1999/404/de#art_30",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Arzneimittelverordnung (VAM, SR 812.212.21), Art. 17 Abs. 2 und Art. 33 Abs. 1, Fassung vom 1. Januar 2026, deutsch",
                url: "https://www.fedlex.admin.ch/eli/cc/2018/588/de#art_33",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Satzgrenzen (lexika/abkuerzungen.toml): Gesetzestechnische Richtlinien des Bundes (GTR), Stand 5. Juni 2026",
                url: "https://www.bk.admin.ch/dam/de/sd-web/q96zQo4nmf7w/GTR_DE_20260605.pdf",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Satzgrenzen (lexika/abkuerzungen.toml): Cancelleria federale, Direttive di tecnica legislativa (DTL), Prontuario per i traduttori e redattori di lingua italiana, 1° ottobre 2024",
                url: "https://www.bk.admin.ch/dam/it/sd-web/JUhV4BnOp3l1/Corso%20DTL%202021%20Dispensa%20per%20partecipanti.pdf",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Satzgrenzen (lexika/abkuerzungen.toml): Strafgesetzbuch (StGB, SR 311.0), Art. 201–212, Fussnote 308, Fassung vom 1. November 2026, französisch",
                url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/fr#art_201_212",
                lizenz: None,
                stand: GEPRUEFT,
            },
            Quelle {
                titel: "Satzgrenzen (lexika/abkuerzungen.toml): Unicode CLDR 49, common/segments/de.xml, fr.xml und it.xml (Ausnahmen der Satzsegmentierung)",
                url: "https://github.com/unicode-org/cldr/tree/release-49/common/segments",
                lizenz: Some(
                    "Unicode License v3 (SPDX: Unicode-3.0), Copyright © 2001–2026 Unicode, Inc.; Lizenztext: promptscan licenses, Abschnitt 8.4",
                ),
                stand: GEPRUEFT,
            },
        ],
        lexika: vec![FUNDSTELLEN],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Die Belege für das Erfinden stammen aus den USA: Dahl, Magesh, Suzgun und Ho (2024) fragten vier Modelle von 2023 nach zufällig gezogenen Urteilen US-amerikanischer Bundesgerichte; über alle referenzbasierten Aufgaben halluzinierte GPT 4 in 58 % der Antworten, Llama 2 in 88 %, bei neueren, bekannteren Urteilen und oberen Gerichten seltener. Für Schweizer Urteile und neuere Modelle ist das nicht gemessen.",
                "The evidence comes from the US: Dahl, Magesh, Suzgun and Ho (2024) asked four 2023 models about randomly drawn judgments of US federal courts; across all reference-based tasks GPT 4 hallucinated in 58 % of answers, Llama 2 in 88 %, less often for newer, more prominent judgments and higher courts. Swiss judgments and newer models were not measured.",
            ),
            Txt::neu(
                "Fragen zu einem Urteil, das der Prompt selbst zitiert («Was sagt BGE 136 II 508?»), meldet die Regel nicht, obwohl Dahl et al. genau solche Fragen zu einem genannten Urteil gestellt haben.",
                "Questions about a judgment the prompt itself cites («Was sagt BGE 136 II 508?») are not reported, although Dahl et al. asked exactly such questions about a named judgment.",
            ),
            Txt::neu(
                "Bitten ohne Aufforderungswort («Rechtsprechung zu Art. 337 OR»), Umschreibungen («Fasse die Rechtsprechung zusammen»), Zusammensetzungen («Bundesgerichtsurteile») und klein geschriebene mehrdeutige Substantive erkennt die Regel nicht. «Kommentar» und «Lehre» fehlen, weil sie im Alltag oft anderes bedeuten.",
                "Requests without a request word («Rechtsprechung zu Art. 337 OR»), paraphrases («Fasse die Rechtsprechung zusammen»), compounds («Bundesgerichtsurteile») and ambiguous nouns written in lower case are not detected. «Kommentar» and «Lehre» are missing because in everyday use they often mean something else.",
            ),
            Txt::neu(
                "Die Aufforderungswörter (Imperative, Infinitive, Bittwörter, Fragewörter, Präpositionen, Artikel) sind eine feste grammatische Liste im Code ohne Einzelbeleg.",
                "The request words (imperatives, infinitives, politeness words, interrogatives, prepositions, articles) are a fixed grammatical list in the code without an individual source.",
            ),
            Txt::neu(
                "«Quellen», «sources» und «fonti» können Alltägliches meinen; «Nenne mir drei Quellen für erneuerbare Energie» oder «Elenca le fonti rinnovabili» melden einen Hinweis. Die Regel nimmt nur «Quellen des …rechts» aus.",
                "«Quellen», «sources» and «fonti» can mean everyday things; «Nenne mir drei Quellen für erneuerbare Energie» or «Elenca le fonti rinnovabili» produce a note. The rule only excludes «Quellen des …rechts».",
            ),
            Txt::neu(
                "Die Regel prüft keine Zitate; ob eine Fundstelle stimmt, zeigt nur die Quelle selbst.",
                "The rule does not check citations; only the source itself shows whether a citation is correct.",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fund::{Grund, Stufe};
    use crate::stufen::Optionen;
    use unicode_segmentation::UnicodeSegmentation;

    /// Die markierten Stellen einer Regel.
    fn stellen(regel: &str, roh: &str) -> Vec<String> {
        let t = Text::neu(roh);
        erkenne(&t)
            .into_iter()
            .filter(|x| x.regel == regel)
            .map(|x| t.als_str()[x.start..x.ende].to_string())
            .collect()
    }

    fn vermerk(roh: &str) -> Vec<String> {
        stellen("vermerk", roh)
    }

    fn fundstelle(roh: &str) -> Vec<String> {
        stellen("fundstellen", roh)
    }

    fn saetze(roh: &str) -> Vec<String> {
        let t = Text::neu(roh);
        t.saetze()
            .iter()
            .map(|b| t.ausschnitt(b.clone()).to_string())
            .collect()
    }

    fn keine(regel: &str, faelle: &[&str]) {
        for f in faelle {
            let s = stellen(regel, f);
            assert!(s.is_empty(), "«{f}» meldet {s:?}");
        }
    }

    // --- vermerk -----------------------------------------------------------

    #[test]
    fn vermerk_einzelzeile_deutsch() {
        assert_eq!(
            vermerk("VERTRAULICH\n\nBericht über die Sitzung vom Montag."),
            ["VERTRAULICH"]
        );
        assert_eq!(
            vermerk("Fasse zusammen:\n\n**Geheim**\nDer Plan sieht vor, …"),
            ["Geheim"]
        );
        assert_eq!(
            vermerk("Klassifizierung: intern\nProtokoll der Sitzung"),
            ["Klassifizierung: intern"]
        );
        assert_eq!(
            vermerk("Bitte prüfen:\n\nKlassifizierungsstufe «vertraulich»\n\nDer Text folgt."),
            ["Klassifizierungsstufe «vertraulich"]
        );
    }

    #[test]
    fn vermerk_kopfzeile() {
        assert_eq!(
            vermerk("Vertraulich – Memo an die Geschäftsleitung\nWir planen den Umzug."),
            ["Vertraulich"]
        );
        assert_eq!(
            vermerk("\n  VERTRAULICH: Entwurf\nDer Vertrag lautet …"),
            ["VERTRAULICH"]
        );
        // Nur die erste nichtleere Zeile ist Kopfzeile.
        keine("vermerk", &["Memo\nVertraulich – bitte lesen"]);
    }

    #[test]
    fn vermerk_franzoesisch_und_italienisch() {
        assert_eq!(
            vermerk("CONFIDENTIEL\nNote à la direction."),
            ["CONFIDENTIEL"]
        );
        assert_eq!(
            vermerk("Classification : secret\nRapport"),
            ["Classification : secret"]
        );
        assert_eq!(
            vermerk("Échelon de classification : interne\nNote"),
            ["Échelon de classification : interne"]
        );
        assert_eq!(
            vermerk("AD USO INTERNO\nVerbale della riunione."),
            ["AD USO INTERNO"]
        );
        assert_eq!(
            vermerk("Classificazione: segreta\nVerbale"),
            ["Classificazione: segreta"]
        );
        assert_eq!(
            vermerk("Confidenziale – Promemoria\nTesto"),
            ["Confidenziale"]
        );
    }

    #[test]
    fn vermerk_dreisprachig_ein_treffer() {
        assert_eq!(
            vermerk("INTERN / INTERNE / AD USO INTERNO\nText"),
            ["INTERN / INTERNE / AD USO INTERNO"]
        );
    }

    #[test]
    fn vermerk_zusatz_und_beleg() {
        let t = Text::neu("GEHEIM\nText");
        let treffer: Vec<Treffer> = erkenne(&t)
            .into_iter()
            .filter(|x| x.regel == "vermerk")
            .collect();
        assert_eq!(treffer.len(), 1);
        let z = treffer[0].zusatz.clone().expect("Zusatz");
        assert_eq!(z.de, "Art. 13 Abs. 3 ISG");
        assert_eq!(z.en, "Art. 13 Abs. 3 ISG", "englisch das amtliche Zitat");
        let b = treffer[0].beleg.clone().expect("Beleg");
        assert_eq!(b.datei, "vermerk.toml");
        assert_eq!(b.begriff, "geheim");
        assert_eq!(b.quelle, "SR 128 Art. 13 Abs. 3");
        assert_eq!(treffer[0].art, Art::Hinweis);
    }

    #[test]
    fn vermerk_mitten_im_text_nicht() {
        keine(
            "vermerk",
            &[
                "Bitte vertraulich behandeln: Der Mitarbeiter kündigt.",
                "Hallo\nIch erzähle dir das vertraulich.\nBitte vertraulich behandeln.",
                "Der Bericht ist geheim.",
                "Le rapport est confidentiel.",
                "Il verbale è segreto.",
            ],
        );
    }

    #[test]
    fn vermerk_ohne_trenner_nicht() {
        keine(
            "vermerk",
            &[
                "Geheim ist, was niemand weiss.",
                "Vertraulich behandeln!",
                "Interne Mitteilung an alle",
                "Geheim-Tipp: Kaffee",
                "Intern/extern: Unterschiede",
            ],
        );
    }

    #[test]
    fn vermerk_frage_und_aufzaehlung_nicht() {
        keine(
            "vermerk",
            &[
                "Vertraulich: Was heisst das?",
                "Was bedeutet\nvertraulich?",
                "Erkläre die Stufen:\n- intern\n- vertraulich\n- geheim",
                "Klassifizierung\nWelche Stufe passt?",
            ],
        );
    }

    #[test]
    fn vermerk_einzelne_stufe_in_liste_wird_gemeldet() {
        // Dokumentierte Grenze: eine Zeile nur mit einem Stufenwort zählt.
        assert_eq!(vermerk("Die Stufe lautet:\n- vertraulich"), ["vertraulich"]);
    }

    #[test]
    fn vermerk_wird_hinweis() {
        let b = crate::pruefe("VERTRAULICH\nAnfrage zum Vertrag.", &Optionen::default());
        let f = b
            .funde
            .iter()
            .find(|f| f.regel == "vermerk")
            .expect("Fund vermerk");
        assert_eq!(f.stufe, Stufe::Hinweis);
        assert_eq!(f.grund, Grund::Regelhinweis);
        assert_eq!(f.stelle, "VERTRAULICH");
    }

    // --- fundstellen -------------------------------------------------------

    #[test]
    fn fundstellen_deutsch() {
        let faelle: &[(&str, &str)] = &[
            (
                "Formuliere eine Kündigung und nenne passende BGE zum Kündigungsschutz.",
                "nenne passende BGE",
            ),
            ("Erkläre die Rechtslage mit Fundstellen.", "mit Fundstellen"),
            (
                "Gibt es Rechtsprechung zu Art. 337 OR?",
                "Gibt es Rechtsprechung",
            ),
            (
                "Kannst du mir Urteile zum Mietrecht nennen?",
                "Urteile zum Mietrecht nennen",
            ),
            (
                "Welche Leitentscheide sind massgebend?",
                "Welche Leitentscheide",
            ),
            (
                "Nennen Sie Entscheide des Bundesgerichts zur Haftung.",
                "Nennen Sie Entscheide",
            ),
            ("Ich suche Literatur zum Datenschutz.", "suche Literatur"),
            (
                "Was sagt die Rechtsprechung dazu?",
                "Was sagt die Rechtsprechung",
            ),
            ("Gib mir ein Urteil zu dieser Frage.", "Gib mir ein Urteil"),
            ("Belege deine Antwort mit Präjudizien.", "mit Präjudizien"),
            (
                "Liste die einschlägigen BGE auf.",
                "Liste die einschlägigen BGE",
            ),
            (
                "Antworte unter Angabe der Fundstellen.",
                "unter Angabe der Fundstellen",
            ),
            (
                "Nenne nicht nur BGE, sondern auch Literatur.",
                "Nenne nicht nur BGE",
            ),
        ];
        for (prompt, stelle) in faelle {
            assert_eq!(fundstelle(prompt), [*stelle], "«{prompt}»");
        }
    }

    #[test]
    fn fundstellen_franzoesisch() {
        let faelle: &[(&str, &str)] = &[
            ("Cite la jurisprudence pertinente.", "Cite la jurisprudence"),
            (
                "Quels arrêts du Tribunal fédéral traitent de la résiliation ?",
                "Quels arrêts",
            ),
            (
                "Donne-moi des références sur ce point.",
                "Donne-moi des références",
            ),
            (
                "Existe-t-il une jurisprudence sur ce point ?",
                "Existe-t-il une jurisprudence",
            ),
            (
                "Pourriez-vous indiquer les ATF pertinents ?",
                "indiquer les ATF",
            ),
            ("Que dit la doctrine ?", "Que dit la doctrine"),
            (
                "Rédige un contrat et cite la jurisprudence.",
                "cite la jurisprudence",
            ),
        ];
        for (prompt, stelle) in faelle {
            assert_eq!(fundstelle(prompt), [*stelle], "«{prompt}»");
        }
    }

    #[test]
    fn fundstellen_italienisch() {
        let faelle: &[(&str, &str)] = &[
            ("Indica le sentenze pertinenti.", "Indica le sentenze"),
            (
                "Cosa dice la giurisprudenza in materia?",
                "Cosa dice la giurisprudenza",
            ),
            (
                "Ci sono sentenze del Tribunale federale sul tema?",
                "Ci sono sentenze",
            ),
            ("Citami una DTF sul tema.", "Citami una DTF"),
            ("Puoi indicarmi la dottrina?", "indicarmi la dottrina"),
            ("Mi indichi una sentenza recente?", "indichi una sentenza"),
        ];
        for (prompt, stelle) in faelle {
            assert_eq!(fundstelle(prompt), [*stelle], "«{prompt}»");
        }
    }

    #[test]
    fn fundstellen_zitat_ist_keine_bitte() {
        keine(
            "fundstellen",
            &[
                "Gemäss BGE 136 II 508 gilt die Regel weiterhin.",
                "Was sagt BGE 136 II 508 zur Kündigung?",
                "Cite l’ATF 140 III 86 dans la lettre.",
                "Indica la DTF 145 I 1 nella lettera.",
            ],
        );
        // Ein zitiertes BGE hebt eine echte Bitte im selben Satz nicht auf; zu weit
        // auseinander, wird nur das Substantiv markiert.
        assert_eq!(
            fundstelle("Nenne BGE 136 II 508 und weitere Urteile."),
            ["Urteile"]
        );
    }

    #[test]
    fn fundstellen_ohne_aufforderung_nicht() {
        keine(
            "fundstellen",
            &[
                "Wie lange dauert es, bis ein Urteil rechtskräftig ist?",
                "Gemäss der Rechtsprechung ist die Kündigung missbräuchlich.",
                "Le Tribunal fédéral cite sa jurisprudence.",
                "La Corte cita la giurisprudenza costante.",
                "Ich finde die Rechtsprechung zu streng.",
                "Bitte finden Sie anbei die Rechtsprechung.",
                "Bei der Suche nach Rechtsprechung half niemand.",
                "Nenne la jurisprudence.",
            ],
        );
    }

    #[test]
    fn fundstellen_mehrdeutige_substantive() {
        keine(
            "fundstellen",
            &[
                "Entscheide, ob die Kündigung gültig ist, und nenne die Gründe.",
                "Nenne mir Entscheide, die ein Startup treffen muss.",
                "Indique les arrêts de bus à Lausanne.",
                "Nenne die Quellen des Völkerrechts.",
                "Elenca le fonti del diritto svizzero.",
                "Scrivi una lettera con riferimento al contratto.",
                "Nenne das Urteil.",
                "nenne mir urteile zum mietrecht",
                "Welche Quellen nutzt die Zeitung?",
            ],
        );
        // Mit Gericht im Satz zählen «Entscheide» und «arrêts».
        assert_eq!(
            fundstelle("Nenne mir Entscheide des BGer zur Kündigung."),
            ["Nenne mir Entscheide"]
        );
        assert_eq!(
            fundstelle("Cite des arrêts du TF sur ce point."),
            ["Cite des arrêts"]
        );
        assert_eq!(fundstelle("Nenne Quellen dafür."), ["Nenne Quellen"]);
    }

    #[test]
    fn fundstellen_bestimmter_artikel_und_verneinung_nicht() {
        keine(
            "fundstellen",
            &[
                "Ist das mit der Rechtsprechung vereinbar?",
                "Est-ce compatible avec la jurisprudence ?",
                "Zitiere keine Urteile.",
                "Zitiere nicht die Rechtsprechung.",
                "Bitte die Rechtsprechung nicht zitieren.",
                "Per favore non citare la giurisprudenza.",
                "Merci de rédiger la lettre sans citer de jurisprudence.",
            ],
        );
    }

    #[test]
    fn fundstellen_im_selben_satz() {
        keine(
            "fundstellen",
            &["Nenne drei Beispiele. Die Rechtsprechung ist klar."],
        );
        assert_eq!(
            fundstelle("Die Rechtsprechung ist klar. Nenne trotzdem BGE dazu."),
            ["Nenne trotzdem BGE"]
        );
    }

    /// Nach einer Anrede ohne Satzzeichen beginnt die nächste Zeile wie ein Satz
    /// (`Text::satzanfang_bei`); die Aufforderung am Satzanfang zählt dort wie einzeilig.
    #[test]
    fn aufforderung_nach_weichem_umbruch() {
        for (roh, stelle) in [
            (
                "Bonjour\nCite les arrêts du Tribunal fédéral sur l'art. 336 CO.",
                "Cite les arrêts",
            ),
            (
                "Bonjour\nDonne les références des arrêts sur l'art. 336 CO.",
                "Donne les références",
            ),
            (
                "Buongiorno\nCita le sentenze del Tribunale federale sull'art. 336 CO.",
                "Cita le sentenze",
            ),
            (
                "Buongiorno\nCiti le sentenze del Tribunale federale sull'art. 336 CO.",
                "Citi le sentenze",
            ),
            (
                "Hallo zusammen\nListe die einschlägigen BGE zur Kündigung.",
                "Liste die einschlägigen BGE",
            ),
        ] {
            assert_eq!(fundstelle(roh), [stelle], "{roh:?}");
        }
        // Mitten in der Zeile wie im einzeiligen Satz.
        keine(
            "fundstellen",
            &[
                "Bonjour\nIl cite les arrêts du Tribunal fédéral.",
                "Hallo zusammen\nDie Liste der BGE zur Kündigung ist lang.",
            ],
        );
    }

    /// Die binäre Suche wählt dieselben Treffer wie der volle Durchlauf, auch an den
    /// Rändern: Ein Treffer am Ende des Bereichs gehört nicht dazu, einer über das Ende
    /// hinaus auch nicht.
    #[test]
    fn abschnitt_wie_der_volle_durchlauf() {
        let alle: Vec<(LexTreffer, ())> = [(0, 3), (4, 8), (8, 12), (15, 20)]
            .into_iter()
            .map(|(start, ende)| {
                let t = LexTreffer {
                    start,
                    ende,
                    eintrag: 0,
                };
                (t, ())
            })
            .collect();
        for a in 0..=21 {
            for b in a..=21 {
                let bereich = a..b;
                let schnell: Vec<usize> = abschnitt(&alle, &bereich)
                    .iter()
                    .filter(|(t, _)| t.ende <= bereich.end)
                    .map(|(t, _)| t.start)
                    .collect();
                let voll: Vec<usize> = alle
                    .iter()
                    .filter(|(t, _)| bereich.contains(&t.start) && t.ende <= bereich.end)
                    .map(|(t, _)| t.start)
                    .collect();
                assert_eq!(schnell, voll, "{bereich:?}");
            }
        }
    }

    #[test]
    fn fundstellen_quellen_im_alltag_werden_gemeldet() {
        // Dokumentierte Grenze: «Quellen» im Alltagssinn lösen den Hinweis aus.
        assert_eq!(
            fundstelle("Nenne mir drei Quellen für erneuerbare Energie."),
            ["Nenne mir drei Quellen"]
        );
    }

    #[test]
    fn fundstellen_ein_treffer_je_satz_mit_beleg() {
        let t = Text::neu("Nenne BGE und Literatur.");
        let treffer: Vec<Treffer> = erkenne(&t)
            .into_iter()
            .filter(|x| x.regel == "fundstellen")
            .collect();
        assert_eq!(treffer.len(), 1);
        assert_eq!(&t.als_str()[treffer[0].start..treffer[0].ende], "Nenne BGE");
        let b = treffer[0].beleg.clone().expect("Beleg");
        assert_eq!(b.datei, "fundstellen.toml");
        assert_eq!(b.begriff, "BGE");
        assert_eq!(b.quelle, "SR 173.110.131 Art. 57 Abs. 1 Bst. a");
        assert!(treffer[0].zusatz.is_none());
    }

    #[test]
    fn fundstellen_wird_hinweis_mit_text() {
        let b = crate::pruefe("Nenne passende BGE zum Mietrecht.", &Optionen::default());
        let f = b
            .funde
            .iter()
            .find(|f| f.regel == "fundstellen")
            .expect("Fund fundstellen");
        assert_eq!(f.stufe, Stufe::Hinweis);
        assert_eq!(f.grund, Grund::Regelhinweis);
        let info = crate::katalog::finde("fundstellen").expect("Regel im Katalog");
        assert_eq!(
            info.hinweis.map(|h| h.de),
            Some("Sprachmodelle erfinden Urteile. Jede Fundstelle prüfen.")
        );
    }

    #[test]
    fn gegenprobe_beide_regeln_melden() {
        // Würde erkenne() nichts tun, schlügen diese Prüfungen fehl.
        let t = Text::neu("VERTRAULICH\nNenne passende BGE zum Mietrecht.");
        let regeln: Vec<&str> = erkenne(&t).iter().map(|x| x.regel).collect();
        assert_eq!(regeln, ["vermerk", "fundstellen"]);
    }

    #[test]
    fn info_vollstaendig() {
        let v = info();
        assert_eq!(
            v.iter().map(|r| r.id).collect::<Vec<_>>(),
            ["vermerk", "fundstellen"]
        );
        for r in &v {
            assert_eq!(r.klasse, Klasse::Hinweis);
            assert!(r.hinweis.is_some(), "{}", r.id);
            assert!(!r.grundlage.is_empty(), "{}", r.id);
            assert!(!r.quellen.is_empty(), "{}", r.id);
            assert!(!r.grenzen.is_empty(), "{}", r.id);
            for d in &r.lexika {
                assert!(lexikon::inhalt(d).is_some(), "lexika/{d} fehlt");
            }
            // SPEC: zwei bis vier Sätze; gezählt mit den eigenen Satzgrenzen.
            for methode in [r.methode.de, r.methode.en] {
                let n = Text::neu(methode).saetze().len();
                assert!((2..=4).contains(&n), "{}: {n} Sätze", r.id);
            }
        }
    }

    // --- Satzgrenzen (lexika/abkuerzungen.toml) ----------------------------

    #[test]
    fn satzgrenze_art_abs_zwei_saetze() {
        assert_eq!(
            saetze("Art. 5 Abs. 2 DSG gilt. Dann anderes."),
            ["Art. 5 Abs. 2 DSG gilt. ", "Dann anderes."]
        );
    }

    #[test]
    fn satzgrenze_gegenprobe_ohne_liste_getrennt() {
        // UAX #29 allein trennt nach «Art.» und «Abs.»; erst die Liste verbindet.
        let roh = "Art. 5 Abs. 2 DSG gilt. Dann anderes.";
        assert_eq!(roh.split_sentence_bounds().count(), 4);
    }

    #[test]
    fn satzgrenze_titel_orte_und_beispiele() {
        assert_eq!(
            saetze("Er traf z. B. Dr. Muster in St. Gallen. Dann ging er.").len(),
            2
        );
        assert_eq!(saetze("Er nannte z.B. Hans. Dann ging er.").len(), 2);
        assert_eq!(
            saetze("Das ist, d. h. Ihr Fall, klar. Dann anderes.").len(),
            2
        );
    }

    #[test]
    fn satzgrenze_franzoesisch_und_italienisch() {
        assert_eq!(
            saetze(
                "Selon l’art. 5 al. 2 LTF, cf. Message du Conseil fédéral. Ensuite autre chose."
            )
            .len(),
            2
        );
        assert_eq!(
            saetze(
                "Ai sensi dell’art. 5 cpv. 2 LTF, cfr. Messaggio del Consiglio federale. Poi altro."
            )
            .len(),
            2
        );
    }

    #[test]
    fn satzgrenze_lit_braucht_keinen_eintrag() {
        let lex = lexikon::lexikon("abkuerzungen.toml");
        assert!(!lex.eintraege.iter().any(|e| e.begriff == "lit."));
        assert_eq!(saetze("Art. 5 lit. a DSG gilt. Dann anderes.").len(), 2);
    }

    #[test]
    fn satzgrenze_jede_abkuerzung_wirkt() {
        let lex = lexikon::lexikon("abkuerzungen.toml");
        assert!(lex.eintraege.len() >= 30);
        for e in &lex.eintraege {
            assert_eq!(e.regel, "satzgrenze");
            for form in std::iter::once(&e.begriff).chain(e.formen.iter()) {
                let roh = format!("Vorher {form} Xaver folgt. Neuer Satz.");
                let s = saetze(&roh);
                assert_eq!(s.len(), 2, "{form}: {s:?}");
                assert!(s[0].ends_with("folgt. "), "{form}: {s:?}");
            }
        }
    }

    #[test]
    fn satzgrenze_echte_grenzen_bleiben() {
        assert_eq!(saetze("Erster Satz. Zweiter Satz.").len(), 2);
        // «Fr.» steht bewusst nicht in der Liste: Franken am Satzende.
        assert_eq!(saetze("Das kostet 5 Fr. Dann kam er.").len(), 2);
    }
}
