//! Regel `geburtsdatum`: ein Datum, das ein Auslöser als Geburtsdatum einleitet (SPEC 5.2).
//!
//! Auslöser und Anschlusswörter stehen in `lexika/geburtsdatum.toml`, die Monatsnamen in
//! `lexika/geburtsdatum-monate.toml`. Unmittelbar nach dem Auslöser muss ein Datum stehen,
//! das im gregorianischen Kalender besteht. Markiert wird nur das Datum.
//!
//! Datumsformen: CLDR 49 (de-CH, fr-CH, it-CH: «d.M.y», «dd.MM.y», «dd/MM/y», «d MMMM y»,
//! «d. MMMM y»), Schreibweisungen der Bundeskanzlei Rz. 534–536 (Tag, Monat, Jahr; Jahr
//! vierstellig; Festabstand zwischen Tag und Monat) und RFC 3339 Abschnitt 5.6 («full-date»).
//! Monatslängen und Schaltjahre nach RFC 3339 Abschnitt 5.7 und Anhang C.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::{Captures, Regex};

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Lexikon};
use crate::normen;
use crate::text::{Text, ist_unsichtbar, rechts_frei};

/// Kennung der Regel.
pub const REGEL: &str = "geburtsdatum";
/// Wortliste der Auslöser und Anschlusswörter.
pub const LEX_AUSLOESER: &str = "geburtsdatum.toml";
/// Wortliste der Monatsnamen.
pub const LEX_MONATE: &str = "geburtsdatum-monate.toml";

/// Die Merkmale in `geburtsdatum.toml`; jedes wirkt in [`muster`].
pub const MERKMALE: [&str; 7] = [
    "nomen",
    "partizip",
    "abkuerzung",
    "anschluss",
    "ort",
    "ordinal",
    "feldzusatz",
];

/// Leerraum innerhalb einer Zeile, auch der Festabstand (BK-Schreibweisungen Rz. 535).
const LEER: &str = r"[ \t\x{00A0}\x{202F}\x{2009}\x{2007}]";

/// Ein gross beginnendes Wort eines Ortsnamens («Genève», «Chaux-de-Fonds», «St.»).
const ORTSWORT: &str = r"\p{Lu}[\p{L}\p{M}'\x{2019}.\-]*";

/// So viele Bytes nach dem Auslöser werden höchstens gelesen.
const FENSTER: usize = 200;

/// Meldet jedes gültige Datum, das unmittelbar auf einen Auslöser folgt.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let m = muster();
    // Ein weicher Umbruch zählt wie ein Leerzeichen («geb.⏎12.03.1985»).
    let s = text.verbunden();
    let mut gesehen = HashSet::new();
    let mut aus = Vec::new();
    for t in m.ausloeser.finde(text) {
        let regex = match m.ausloeser.eintraege[t.eintrag].merkmal.as_deref() {
            Some("nomen") => &m.nomen,
            Some("abkuerzung") => &m.abkuerzung,
            Some("partizip") if schreibung_passt(text, t.start, t.ende) => &m.partizip,
            _ => continue,
        };
        let lesbar = Lesbar::neu(text, t.ende);
        let Some(c) = regex.captures(&lesbar.text) else {
            continue;
        };
        let Some(d) = c.name("datum") else {
            continue;
        };
        let (start, ende) = lesbar.stelle(d.start(), d.end());
        if !rechts_frei(s, ende) {
            continue;
        }
        let Some((tag, monat, jahr)) = datum_lesen(&c, &m.monat) else {
            continue;
        };
        if gueltig(tag, monat, jahr) && gesehen.insert((start, ende)) {
            aus.push(
                Treffer::neu(REGEL, start, ende, Art::Personenangabe)
                    .mit_beleg(m.ausloeser.beleg(t.eintrag)),
            );
        }
    }
    aus
}

/// Die einmal gebauten Suchmuster.
struct Muster {
    /// Nur die Auslöser (nomen, partizip, abkuerzung).
    ausloeser: Lexikon,
    nomen: Regex,
    abkuerzung: Regex,
    partizip: Regex,
    /// Monatsname (klein, ohne Schlusspunkt) → Monatsnummer.
    monat: HashMap<String, u32>,
}

fn muster() -> &'static Muster {
    static ZELLE: OnceLock<Muster> = OnceLock::new();
    ZELLE.get_or_init(|| {
        let lex = lexikon::lexikon(LEX_AUSLOESER);
        let ausloeser = Lexikon::aus_eintraegen(
            LEX_AUSLOESER,
            lex.eintraege
                .iter()
                .filter(|e| {
                    matches!(
                        e.merkmal.as_deref(),
                        Some("nomen" | "partizip" | "abkuerzung")
                    )
                })
                .cloned()
                .collect(),
        );
        // «am », «le », «il », aber «l'» ohne Leerraum danach.
        let anschluss = alternative(&formen(lex, "anschluss"), |f| {
            if f.ends_with('\'') {
                mit_apostrophen(f)
            } else {
                [regex::escape(f).as_str(), LEER, "+"].concat()
            }
        });
        let ort = alternative(&formen(lex, "ort"), regex::escape);
        // «1°» auch mit dem Ordinalzeichen «º».
        let ordinal = alternative(&formen(lex, "ordinal"), |f| {
            regex::escape(f).replace('\u{00B0}', "[\u{00B0}\u{00BA}]")
        });

        let mut monat = HashMap::new();
        let mut monatsformen = Vec::new();
        for e in &lexikon::lexikon(LEX_MONATE).eintraege {
            let nummer: u32 = e
                .merkmal
                .as_deref()
                .and_then(|m| m.parse().ok())
                .unwrap_or(0);
            for f in std::iter::once(&e.begriff).chain(&e.formen) {
                monat.insert(monatsschluessel(f), nummer);
                monatsformen.push(f.clone());
            }
        }
        // Nach einer Form ohne Schlusspunkt ist ein Punkt erlaubt («gen.» neben CLDR «gen»).
        let monate = alternative(&monatsformen, |f| {
            if f.ends_with('.') {
                regex::escape(f)
            } else {
                [regex::escape(f).as_str(), r"\.?"].concat()
            }
        });

        let datum = [
            "(?P<datum>",
            r"(?P<t1>[0-9]{1,2})\.(?P<m1>[0-9]{1,2})\.(?P<j1>[0-9]{4})",
            r"|(?P<t2>[0-9]{1,2})/(?P<m2>[0-9]{1,2})/(?P<j2>[0-9]{4})",
            r"|(?P<j3>[0-9]{4})-(?P<m3>[0-9]{2})-(?P<t3>[0-9]{2})",
            "|(?:(?P<erster>",
            ordinal.as_str(),
            r")|(?P<t4>[0-9]{1,2})\.?)",
            LEER,
            "+(?i:(?P<monat>",
            monate.as_str(),
            "))",
            LEER,
            r"+(?P<j4>[0-9]{4})",
            ")",
        ]
        .concat();
        // «Geburtsdatum/Alter»: ein zweiter Feldname nach «/» (Befund technik-6).
        let feldzusatz = alternative(&formen(lex, "feldzusatz"), regex::escape);
        // Nach dem Nomen wahlweise ein Feldzusatz, eine kurze Klammer («(TT.MM.JJJJ)») und
        // ein Trennzeichen.
        let nomen = [
            "^(?:",
            LEER,
            "*/",
            LEER,
            "*(?i:",
            feldzusatz.as_str(),
            "))?",
            LEER,
            r"*(?:\([^()\n]{1,24}\)",
            LEER,
            "*)?(?:[=:|(\u{2013}-]",
            LEER,
            "*)?(?:",
            anschluss.as_str(),
            ")?",
            datum.as_str(),
        ]
        .concat();
        let abkuerzung = [
            "^:?",
            LEER,
            "*(?:",
            anschluss.as_str(),
            ")?",
            datum.as_str(),
        ]
        .concat();
        // «né(e)», «nato/a»; dann wahlweise ein Geburtsort, dann Anschlusswort und Datum.
        let geburtsort = [
            "(?:,?",
            LEER,
            "+(?:",
            ort.as_str(),
            ")",
            LEER,
            "+",
            ORTSWORT,
            "(?:",
            LEER,
            "+",
            ORTSWORT,
            "){0,3}(?:",
            LEER,
            r"*\(\p{Lu}{2}\))?)?",
        ]
        .concat();
        let partizip = [
            r"^(?:\([aeo]\)|/[aeo])?",
            geburtsort.as_str(),
            ",?",
            LEER,
            "+(?:",
            anschluss.as_str(),
            ")",
            datum.as_str(),
        ]
        .concat();
        let bauen = |m: &str| Regex::new(m).expect("Datumsmuster");
        Muster {
            ausloeser,
            nomen: bauen(&nomen),
            abkuerzung: bauen(&abkuerzung),
            partizip: bauen(&partizip),
            monat,
        }
    })
}

/// Alle Formen der Einträge mit diesem Merkmal.
fn formen(lex: &Lexikon, merkmal: &str) -> Vec<String> {
    lex.eintraege
        .iter()
        .filter(|e| e.merkmal.as_deref() == Some(merkmal))
        .flat_map(|e| std::iter::once(e.begriff.clone()).chain(e.formen.iter().cloned()))
        .collect()
}

/// Regex-Alternative aus Wortformen, längste zuerst.
fn alternative(formen: &[String], muster: impl Fn(&str) -> String) -> String {
    let mut f: Vec<&str> = formen.iter().map(String::as_str).collect();
    f.sort_by_key(|x| std::cmp::Reverse(x.len()));
    f.dedup();
    f.into_iter().map(muster).collect::<Vec<_>>().join("|")
}

/// Ein Apostroph passt auch auf seine typografischen Varianten (wie [`lexikon::falten`]).
fn mit_apostrophen(form: &str) -> String {
    regex::escape(form).replace('\'', "['\u{2019}\u{2018}\u{02BC}]")
}

fn monatsschluessel(form: &str) -> String {
    form.trim_end_matches('.').to_lowercase()
}

/// Der Text ab `ab`, höchstens [`FENSTER`] Bytes, an einer Zeichengrenze gekürzt.
fn fenster(s: &str, ab: usize) -> &str {
    let mut bis = (ab + FENSTER).min(s.len());
    while !s.is_char_boundary(bis) {
        bis -= 1;
    }
    &s[ab..bis]
}

/// Das Fenster nach dem Auslöser, wie die Muster es lesen: Ein weicher Zeilenumbruch
/// ([`Text::weicher_umbruch`]) gilt als Leerzeichen («geboren⏎am 12. März 1985»),
/// unsichtbare Formatzeichen ([`ist_unsichtbar`]) fallen weg («geb.\u{200B} 12.03.1985»).
/// Die Stelle des Datums gilt danach wieder im ganzen Text.
struct Lesbar {
    text: String,
    /// Je Byte von `text` der Versatz seines Zeichens im ganzen Text.
    versatz: Vec<usize>,
}

impl Lesbar {
    fn neu(text: &Text, ab: usize) -> Lesbar {
        let roh = fenster(text.als_str(), ab);
        let mut lesbar = String::with_capacity(roh.len());
        let mut versatz = Vec::with_capacity(roh.len());
        for (i, c) in roh.char_indices() {
            if ist_unsichtbar(c) {
                continue;
            }
            // Ein Zeilenumbruch ist ein Byte, das Leerzeichen auch: die Versätze bleiben.
            let c = if c == '\n' && text.weicher_umbruch(ab + i) {
                ' '
            } else {
                c
            };
            versatz.extend(std::iter::repeat_n(ab + i, c.len_utf8()));
            lesbar.push(c);
        }
        Lesbar {
            text: lesbar,
            versatz,
        }
    }

    /// Anfang und Ende einer nicht leeren Stelle aus `text` im ganzen Text.
    fn stelle(&self, start: usize, ende: usize) -> (usize, usize) {
        let letztes = self.text[start..ende]
            .chars()
            .next_back()
            .map_or(0, char::len_utf8);
        let anfang = self.versatz[start];
        match ende.checked_sub(letztes).filter(|_| letztes > 0) {
            Some(i) => (anfang, self.versatz[i] + letztes),
            None => (anfang, anfang),
        }
    }
}

/// Ein Partizip zählt klein geschrieben überall, mit grossem Anfangsbuchstaben nur am
/// Satzanfang und ganz in Grossbuchstaben nie («la NATO il 4 aprile 1949»). Nach einem
/// weichen Umbruch beginnt die Zeile wie ein Satz: «Erika Beispiel⏎Geboren am …»
/// ([`Text::satzanfang_bei`]).
fn schreibung_passt(text: &Text, start: usize, ende: usize) -> bool {
    let s = text.als_str();
    let mut zeichen = s[start..ende].chars();
    let Some(erstes) = zeichen.next() else {
        return false;
    };
    if zeichen.any(char::is_uppercase) {
        return false;
    }
    !erstes.is_uppercase() || text.satzanfang_bei(start)
}

/// Tag, Monat und Jahr aus einem Treffer des Datumsmusters.
fn datum_lesen(c: &Captures, monate: &HashMap<String, u32>) -> Option<(u32, u32, u32)> {
    let zahl = |gruppe: &str| c.name(gruppe).and_then(|m| m.as_str().parse::<u32>().ok());
    for (t, m, j) in [("t1", "m1", "j1"), ("t2", "m2", "j2"), ("t3", "m3", "j3")] {
        if let (Some(tag), Some(monat), Some(jahr)) = (zahl(t), zahl(m), zahl(j)) {
            return Some((tag, monat, jahr));
        }
    }
    let monat = *monate.get(&monatsschluessel(c.name("monat")?.as_str()))?;
    let tag = if c.name("erster").is_some() {
        1
    } else {
        zahl("t4")?
    };
    Some((tag, monat, zahl("j4")?))
}

/// Schaltjahr im gregorianischen Kalender (RFC 3339, Anhang C).
fn schaltjahr(jahr: u32) -> bool {
    jahr.is_multiple_of(4) && (!jahr.is_multiple_of(100) || jahr.is_multiple_of(400))
}

/// Höchster Tag des Monats (RFC 3339, Abschnitt 5.7); 0 für keinen Monat.
fn tage_im_monat(monat: u32, jahr: u32) -> u32 {
    match monat {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if schaltjahr(jahr) => 29,
        2 => 28,
        _ => 0,
    }
}

fn gueltig(tag: u32, monat: u32, jahr: u32) -> bool {
    tag >= 1 && tag <= tage_im_monat(monat, jahr)
}

/// Beschreibung für `rules` und `explain`.
pub fn info() -> Vec<RegelInfo> {
    vec![RegelInfo {
        id: REGEL,
        klasse: Klasse::B,
        titel: Txt::neu("Geburtsdatum", "Date of birth"),
        erkennt: Txt::neu(
            "Ein Datum, das ein Auslöser als Geburtsdatum einleitet: «geb.», «geboren am», «Geburtsdatum», «né le», «née le», «date de naissance», «nato il», «nata il», «data di nascita», dazu die Feldnamen «Geb.-Datum», «Geb. Datum», «Geb.-Dat.», «date de naiss.», «date naiss.» und «data nascita».",
            "A date introduced as a date of birth: «geb.», «geboren am», «Geburtsdatum», «né le», «née le», «date de naissance», «nato il», «nata il», «data di nascita», plus the field names «Geb.-Datum», «Geb. Datum», «Geb.-Dat.», «date de naiss.», «date naiss.» and «data nascita».",
        ),
        methode: Txt::neu(
            "Sucht die Auslöser aus der Wortliste (Quelle je Wort: ZGB, ZStV, Ausweisverordnung, Duden; die Kurzformen der Feldnamen ohne eigene Fundstelle) und verlangt unmittelbar danach ein Datum: beim Nomen wahlweise nach einer kurzen Klammer und «:», «=», «|» oder Gedankenstrich («Geburtsdatum (TT.MM.JJJJ): …»), beim Partizip nach «am», «le» oder «il», wahlweise mit Geburtsort («né à Genève le …»). Datumsformen nach CLDR 49 und den Schreibweisungen der Bundeskanzlei: 12.03.1985, 1.3.1985, 12/03/1985, 1985-03-12 (RFC 3339), 12. März 1985, 1er mars 1985, 1° marzo 1985, das Jahr vierstellig. Das Datum muss im gregorianischen Kalender bestehen (Monatslängen und Schaltjahre nach RFC 3339); markiert wird nur das Datum. Nach dem Nomen darf ein zweiter Feldname nach «/» stehen: «Alter», «âge», «età» («Geburtsdatum/Alter: 12.03.1985 (40)»). Ein einzelner Zeilenumbruch mitten im Absatz gilt zwischen Auslöser und Datum als Leerzeichen («geb.⏎12.03.1985»), unsichtbare Formatzeichen wie U+00AD oder U+200B werden übersprungen.",
            "Looks up the triggers in the word list (source per word: Civil Code, Civil Status Ordinance, Identity Documents Ordinance, Duden; the short forms of the field names without a source of their own) and requires a date right after them: after a noun optionally behind a short parenthesis and «:», «=», «|» or a dash («Geburtsdatum (TT.MM.JJJJ): …»), after a participle following «am», «le» or «il», optionally with a place of birth («né à Genève le …»). Date forms per CLDR 49 and the Federal Chancellery style guide: 12.03.1985, 1.3.1985, 12/03/1985, 1985-03-12 (RFC 3339), 12. März 1985, 1er mars 1985, 1° marzo 1985, with a four-digit year. The date must exist in the Gregorian calendar (month lengths and leap years per RFC 3339); only the date is marked. After the noun a second field name may follow a «/»: «Alter», «âge», «età» («Geburtsdatum/Alter: 12.03.1985 (40)»). A single line break within a paragraph counts as a space between trigger and date («geb.⏎12.03.1985»), and invisible format characters such as U+00AD or U+200B are skipped.",
        ),
        hinweis: None,
        grundlage: vec![&normen::DSG_5_A],
        quellen: vec![
            Quelle {
                titel: "Art. 15a Abs. 4bis ZStV (SR 211.112.2), Fassung vom 1. Juni 2025: «Geburtsdatum», «data di nascita»",
                url: "https://www.fedlex.admin.ch/eli/cc/2004/362/de#art_15_a",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 10 Abs. 5 VAwG (SR 143.11), Fassung vom 1. November 2026: «date de naissance»",
                url: "https://www.fedlex.admin.ch/eli/cc/2002/468/fr#art_10",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 255 Abs. 1 ZGB (SR 210), Fassung vom 1. Juli 2026: «geboren», «né», «nato»",
                url: "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_255",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 3 Abs. 1bis und Art. 29bis Abs. 2 AHVG (SR 831.10), Fassung vom 1. Januar 2026: «il 1° gennaio», «le 1er janvier»; Art. 8 Abs. 1 StBOG (SR 173.71): «in», «à», «a» vor Ortsnamen",
                url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/fr#art_29_bis",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Duden online, Stichwort «geb.»",
                url: "https://www.duden.de/rechtschreibung/geb__geboren",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Art. 8 Abs. 2 BV (SR 101), Fassung vom 3. März 2024: «Alter», «âge», «età» (Feldzusatz)",
                url: "https://www.fedlex.admin.ch/eli/cc/1999/404/de#art_8",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-07"),
            },
            Quelle {
                titel: "Feldnamen «Geb.-Datum», «Geb. Datum», «Geb.-Dat.», «date de naiss.», «date naiss.», «data nascita»: Kurzformen ohne eigene Fundstelle; die Einträge zitieren Duden «geb.», Larousse «naissance» und Treccani «nascita» zum gekürzten Wort",
                url: "https://www.larousse.fr/dictionnaires/francais/naissance",
                lizenz: None,
                stand: Stand::ZuPruefen(
                    "Weder Fedlex noch die Wörterbücher führen die Kurzformen; Entscheid C4 verlangt sie, der Eigentümer entscheidet, ob sie bleiben.",
                ),
            },
            Quelle {
                titel: "Bundeskanzlei, Schreibweisungen, 2. Aufl. 2013 (korrigierte Ausgabe 2015), Rz. 527, 528 (Monatsnamen) und 534–536 (Datum)",
                url: "https://www.bk.admin.ch/dam/de/sd-web/YVHazXZRkqKn/schreibweisungen.pdf",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "Unicode CLDR 49 (cldr-json 49.0.0), de-CH, fr-CH, it-CH: Monatsnamen und Datumsmuster",
                url: "https://github.com/unicode-org/cldr-json/tree/49.0.0",
                lizenz: Some(
                    "Unicode License V3 (SPDX: Unicode-3.0), Wortlaut mit «promptscan licenses» und in lexika/geburtsdatum-monate.toml",
                ),
                stand: Stand::Geprueft("2026-11-06"),
            },
            Quelle {
                titel: "RFC 3339, Abschnitt 5.6 (full-date), 5.7 (Monatslängen) und Anhang C (Schaltjahr)",
                url: "https://www.rfc-editor.org/rfc/rfc3339.txt",
                lizenz: None,
                stand: Stand::Geprueft("2026-11-06"),
            },
        ],
        lexika: vec![LEX_AUSLOESER, LEX_MONATE],
        guete: Guete::NichtGemessen,
        grenzen: vec![
            Txt::neu(
                "Allein ist ein Geburtsdatum ein HINWEIS; GELB oder ROT wird es erst neben einem Identifikator.",
                "On its own a date of birth is a NOTE; it becomes YELLOW or RED only next to an identifier.",
            ),
            Txt::neu(
                "Nur ein Datum unmittelbar nach dem Auslöser: «am 12. März 1985 geboren», «Geburtsdatum der Mutter: …» und Daten ohne Auslöser (Fristen, Inkrafttreten, Urteilsdaten) erkennt die Regel nicht. Von den Feldnamen aus Formularen kennt sie nur die genannten Kurzformen und als Zusatz nach «/» nur «Alter», «âge» und «età»; «GebDat:», «DOB:», «Geburtsdatum/Ort:» und ein Datum mit Leerzeichen («12. 03. 1985») bleiben unerkannt.",
                "Only a date right after the trigger: «am 12. März 1985 geboren», «Geburtsdatum der Mutter: …» and dates without a trigger (deadlines, entry into force, judgment dates) are not detected. Of the field names used in forms it knows only the short forms named and, as an addition after «/», only «Alter», «âge» and «età»; «GebDat:», «DOB:», «Geburtsdatum/Ort:» and a date with spaces («12. 03. 1985») go unrecognised.",
            ),
            Txt::neu(
                "Zweistellige Jahre (12.03.85), das Jahr allein («geboren 1985»), das Zeichen «*» und Wochentage vor dem Datum werden nicht erkannt.",
                "Two-digit years (12.03.85), the year alone («geboren 1985»), the sign «*» and weekdays before the date are not detected.",
            ),
            Txt::neu(
                "Ob das Datum einer natürlichen Person gehört, prüft die Regel nicht («la Nato è nata il 4 aprile 1949»). Gross geschriebene Partizipien zählen nur am Satzanfang oder am Anfang einer Zeile nach einem weichen Umbruch («Erika Beispiel⏎Geboren am 12. März 1985»), ganz gross geschriebene nie («NATO», «GEBOREN AM»).",
                "The rule does not check whether the date belongs to a natural person («la Nato è nata il 4 aprile 1949»). Capitalised participles count only at the start of a sentence or of a line after a soft break («Erika Beispiel⏎Geboren am 12. März 1985»), all-caps ones never («NATO», «GEBOREN AM»).",
            ),
            Txt::neu(
                "Ein Geburtsort zwischen Partizip und Datum zählt nur nach «in», «à» oder «a» und mit höchstens vier gross beginnenden Wörtern; «né au Locle le …» wird nicht erkannt.",
                "A place of birth between participle and date counts only after «in», «à» or «a» and with at most four capitalised words; «né au Locle le …» is not detected.",
            ),
        ],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fund::{Grund, Stufe};
    use crate::stufen::Optionen;

    /// Die markierten Stellen eines Textes.
    fn stellen(roh: &str) -> Vec<String> {
        let t = Text::neu(roh);
        erkenne(&t)
            .iter()
            .map(|x| t.als_str()[x.start..x.ende].to_string())
            .collect()
    }

    #[test]
    fn gegenprobe_die_regel_findet_etwas() {
        assert_eq!(stellen("Geburtsdatum: 12.03.1985"), ["12.03.1985"]);
    }

    #[test]
    fn deutsch() {
        assert_eq!(stellen("geb. 1.3.1985"), ["1.3.1985"]);
        assert_eq!(stellen("geb. am 12. März 1985"), ["12. März 1985"]);
        assert_eq!(
            stellen("Er ist geboren am 12. März 1985 in Zürich."),
            ["12. März 1985"]
        );
        assert_eq!(
            stellen("geboren in Zürich am 2. Sept. 2006"),
            ["2. Sept. 2006"]
        );
        assert_eq!(stellen("Geboren am 4.12.1970, ledig"), ["4.12.1970"]);
        assert_eq!(stellen("| Geburtsdatum | 12.03.1985 |"), ["12.03.1985"]);
        assert_eq!(
            stellen("Geburtsdatum (TT.MM.JJJJ): 12.03.1985"),
            ["12.03.1985"]
        );
        assert_eq!(stellen("Geburtsdatum (12.03.1985)"), ["12.03.1985"]);
        assert_eq!(
            stellen("geboren am 12.\u{00A0}März 1985"),
            ["12.\u{00A0}März 1985"]
        );
    }

    #[test]
    fn franzoesisch() {
        assert_eq!(stellen("Elle est née le 1er mars 1985."), ["1er mars 1985"]);
        assert_eq!(stellen("né à Genève le 12 mars 1985"), ["12 mars 1985"]);
        assert_eq!(
            stellen("Né le 3 févr. 1980, il habite à Sion."),
            ["3 févr. 1980"]
        );
        assert_eq!(stellen("date de naissance : 12/03/1985"), ["12/03/1985"]);
        assert_eq!(
            stellen("date de naissance (jj.mm.aaaa) : 05.06.1975"),
            ["05.06.1975"]
        );
        assert_eq!(
            stellen("né(e) à La Chaux-de-Fonds le 5 juin 1975"),
            ["5 juin 1975"]
        );
    }

    #[test]
    fn italienisch() {
        assert_eq!(stellen("nato il 12 marzo 1985"), ["12 marzo 1985"]);
        assert_eq!(stellen("nata il 1° marzo 1990"), ["1° marzo 1990"]);
        assert_eq!(stellen("nata il 1º marzo 1990"), ["1º marzo 1990"]);
        assert_eq!(stellen("nata l'8 marzo 1990"), ["8 marzo 1990"]);
        assert_eq!(stellen("nata l\u{2019}11 lug. 1990"), ["11 lug. 1990"]);
        assert_eq!(
            stellen("nato a Lugano (TI) il 3 mag. 1980"),
            ["3 mag. 1980"]
        );
        assert_eq!(stellen("data di nascita: 1985-03-12"), ["1985-03-12"]);
    }

    #[test]
    fn beleg_nennt_den_ausloeser() {
        let t = Text::neu("nata il 1° marzo 1990");
        let tr = erkenne(&t);
        assert_eq!(tr[0].art, Art::Personenangabe);
        let beleg = tr[0].beleg.as_ref().expect("Beleg");
        assert_eq!(beleg.begriff, "nato");
        assert_eq!(beleg.quelle, "SR 210 Art. 255 Abs. 1");
    }

    #[test]
    fn ohne_ausloeser_kein_treffer() {
        assert!(stellen("Inkrafttreten am 1. Januar 2023").is_empty());
        assert!(stellen("Die Frist läuft bis 30.06.2024.").is_empty());
        assert!(stellen("Urteil vom 12. März 2020").is_empty());
        assert!(stellen("Le délai expire le 1er mars 2025.").is_empty());
        assert!(stellen("La decisione del 3 maggio 2021").is_empty());
    }

    #[test]
    fn ausloeser_ohne_datum() {
        assert!(stellen("Maria Schmidt, geb. Schulze").is_empty());
        assert!(stellen("Geburtsdatum: unbekannt").is_empty());
        assert!(stellen("Geburtsdatum (siehe Beilage 3)").is_empty());
        assert!(stellen("Geburtsdatum (Ausweis vom 12.03.2020 verloren)").is_empty());
        assert!(stellen("geboren 1985 in Bern").is_empty());
        assert!(stellen("geboren am 12.03.85").is_empty());
        assert!(stellen("geboren am 12.03.19851").is_empty());
    }

    /// Die Grenzen in `info()` beschreiben, was die Regel tut.
    #[test]
    fn grenzen_wie_beschrieben() {
        assert!(stellen("Er ist am 12. März 1985 geboren.").is_empty());
        assert!(stellen("Geburtsdatum der Mutter: 12.03.1985").is_empty());
        assert!(stellen("* 12.03.1985").is_empty());
        assert!(stellen("geboren am Dienstag, 12. März 1985").is_empty());
        assert!(stellen("né au Locle le 12 mars 1985").is_empty());
        assert!(stellen("GEBOREN AM 12.03.1985").is_empty());
        // Ob das Datum einer Person gehört, prüft die Regel nicht.
        assert_eq!(
            stellen("la Nato è nata il 4 aprile 1949"),
            ["4 aprile 1949"]
        );
        // Andere Feldnamen und Feldzusätze stehen nicht in der Liste (Befund technik-6).
        assert!(stellen("GebDat: 12.03.1985").is_empty());
        assert!(stellen("Geburtsdatum/Ort: 12.03.1985").is_empty());
        assert!(stellen("geb. 12. 03. 1985").is_empty());
    }

    /// Befund technik-6, Entscheid C4: Feldnamen aus Formularen und «/Alter».
    #[test]
    fn feldnamen_aus_formularen() {
        for (text, datum) in [
            ("Hans Muster, Geb. Datum: 12.03.1985", "12.03.1985"),
            ("Hans Muster, Geb.-Dat.: 12.03.1985", "12.03.1985"),
            ("Hans Muster, Geb.-Datum: 12.03.1985", "12.03.1985"),
            (
                "Patientin Erika Beispiel, Geb.-Datum 12.03.1985, Diagnose F32.1",
                "12.03.1985",
            ),
            (
                "Hans Muster, Geburtsdatum/Alter: 12.03.1985 (40)",
                "12.03.1985",
            ),
            ("Geburtsdatum / Alter: 12.03.1985", "12.03.1985"),
            ("Jean Exemple, date naiss.: 12.03.1985", "12.03.1985"),
            ("Jean Exemple, date de naiss. : 12.03.1985", "12.03.1985"),
            ("date de naissance/âge : 12.03.1985", "12.03.1985"),
            ("Mario Esempio, data nascita: 12.03.1985", "12.03.1985"),
            ("data di nascita/età: 12.03.1985", "12.03.1985"),
            (
                "Name: Muster, Geb.-Datum: 12.03.1985, Diagnose: Depression",
                "12.03.1985",
            ),
        ] {
            assert_eq!(stellen(text), [datum], "{text}");
        }
        // Der Beleg nennt einen eigenen Eintrag der Kurzform, der sagt, dass sie keine eigene
        // Fundstelle hat (Prüferprobleme c-belege-p1, c-erkennung-p7), nicht die
        // Fedlex-Stelle der Langform, in der die Kurzform nicht steht.
        for (text, begriff) in [
            ("Geb.-Datum: 12.03.1985", "Geb.-Datum"),
            ("Geb.-Dat.: 12.03.1985", "Geb.-Datum"),
            ("date naiss.: 12.03.1985", "date de naiss."),
            ("data nascita: 12.03.1985", "data nascita"),
        ] {
            let t = Text::neu(text);
            let beleg = erkenne(&t)[0].beleg.clone().expect("Beleg");
            assert_eq!(beleg.begriff, begriff, "{text}");
            assert!(beleg.quelle.contains("ohne eigene Fundstelle"), "{text}");
            assert!(!beleg.url.contains("fedlex"), "{text}");
        }
        let t = Text::neu("Geburtsdatum: 12.03.1985");
        let beleg = erkenne(&t)[0].beleg.clone().expect("Beleg");
        assert_eq!(beleg.begriff, "Geburtsdatum");
        // «Alter» ist mit BV Art. 8 Abs. 2 belegt (Lebensalter), wie «âge» und «età»
        // (Prüferproblem c-belege-p4).
        let alter = lexikon::lexikon(LEX_AUSLOESER)
            .eintraege
            .iter()
            .filter(|e| e.merkmal.as_deref() == Some("feldzusatz"))
            .map(|e| e.url.as_str())
            .collect::<Vec<_>>();
        assert_eq!(alter.len(), 3);
        assert!(alter.iter().all(|u| u.ends_with("#art_8")), "{alter:?}");
        // Ohne Auslöser bleibt es kein Geburtsdatum.
        assert!(stellen("Datum: 12.03.1985").is_empty());
        assert!(stellen("Alter: 12.03.1985").is_empty());
    }

    /// Reste Runde 1 (w4): Ein weicher Zeilenumbruch gilt zwischen Auslöser und Datum als
    /// Leerraum, unsichtbare Formatzeichen werden übersprungen.
    #[test]
    fn weicher_umbruch_und_formatzeichen() {
        assert_eq!(
            stellen("Die Klientin ist geboren\nam 12. März 1985 und fragt nach."),
            ["12. März 1985"]
        );
        // Eine Zeile, die mit «12. » beginnt, ist ein Listenpunkt: Der Umbruch davor trennt.
        assert!(stellen("Die Klientin ist geboren am\n12. März 1985 und fragt.").is_empty());
        assert_eq!(
            stellen("Die Klientin, geb.\u{200B} 12.03.1985, fragt nach."),
            ["12.03.1985"]
        );
        // «geb.» ist eine Abkürzung (lexika/abkuerzungen.toml): Der Umbruch danach ist weich.
        assert_eq!(
            stellen("Die Klientin, geb.\n12.03.1985, fragt nach."),
            ["12.03.1985"]
        );
        assert_eq!(
            stellen("Die Klientin, geb. 12.\u{00AD}03.1985, fragt nach."),
            ["12.\u{00AD}03.1985"]
        );
        assert_eq!(stellen("Geburtsdatum:\u{2060} 12.03.1985"), ["12.03.1985"]);
        // Ein harter Umbruch (Leerzeile) trennt weiter.
        assert!(stellen("Die Klientin ist geboren am\n\n12. März 1985.").is_empty());
        // Die Stelle steht im ganzen Text richtig, auch nach einem entfernten Zeichen.
        let roh = "geb.\u{200B}\u{200B} 12.03.1985";
        let t = Text::neu(roh);
        let tr = erkenne(&t);
        assert_eq!(&roh[tr[0].start..tr[0].ende], "12.03.1985");
    }

    /// Befund w4-technik-p3: Ein Name oder eine Überschrift ohne Satzzeichen bildet mit der
    /// nächsten Zeile einen Satz; ein gross geschriebenes Partizip am Anfang der nächsten
    /// Zeile zählt trotzdem.
    #[test]
    fn partizip_am_zeilenanfang_nach_weichem_umbruch() {
        for (roh, datum) in [
            (
                "Erika Beispiel\nGeboren am 12. März 1985 in Zürich\nSeestrasse 12, 8002 Zürich",
                "12. März 1985",
            ),
            (
                "Marie Exemple\nNée le 12 mars 1985 à Genève\nRue du Lac 12, 1201 Genève",
                "12 mars 1985",
            ),
            (
                "Giulia Esempio\nNata il 12 marzo 1985 a Lugano\nVia Lago 12, 6900 Lugano",
                "12 marzo 1985",
            ),
            (
                "Personalien\nGeboren am 12. März 1985, leidet an einer psychischen Störung.",
                "12. März 1985",
            ),
            (
                "Dossier médical\r\n  Née le 12 mars 1985 à Genève.",
                "12 mars 1985",
            ),
            (
                "Cartella clinica\nNata il 12 marzo 1985 a Lugano.",
                "12 marzo 1985",
            ),
        ] {
            assert_eq!(stellen(roh), [datum], "{roh:?}");
        }
        // Mitten in der Zeile bleibt ein gross geschriebenes Partizip still, ganz gross
        // geschrieben auch am Zeilenanfang.
        assert!(stellen("Erika Beispiel ist Geboren am 12. März 1985.").is_empty());
        assert!(stellen("Erika Beispiel\nGEBOREN AM 12. März 1985").is_empty());
    }

    #[test]
    fn kalender_wird_geprueft() {
        assert!(stellen("geboren am 31.02.1985").is_empty());
        assert!(stellen("geboren am 31.04.1985").is_empty());
        assert!(stellen("geboren am 29.02.1900").is_empty());
        assert!(stellen("geboren am 29.02.1997").is_empty());
        assert!(stellen("geboren am 00.03.1985").is_empty());
        assert!(stellen("geboren am 12.13.1985").is_empty());
        assert!(stellen("Geburtsdatum: 1985-02-30").is_empty());
        assert_eq!(stellen("geboren am 29.02.2000"), ["29.02.2000"]);
        assert_eq!(stellen("geboren am 29.02.1996"), ["29.02.1996"]);
        assert_eq!(stellen("geboren am 31. Dez. 1999"), ["31. Dez. 1999"]);
    }

    #[test]
    fn schaltjahre_nach_rfc_3339() {
        assert!(schaltjahr(2000) && schaltjahr(1996) && schaltjahr(2024));
        assert!(!schaltjahr(1900) && !schaltjahr(2023) && !schaltjahr(2100));
        let tage: Vec<u32> = (1..=12).map(|m| tage_im_monat(m, 2023)).collect();
        assert_eq!(tage, [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]);
        assert_eq!(tage_im_monat(2, 2024), 29);
        assert_eq!(tage_im_monat(13, 2024), 0);
    }

    #[test]
    fn nato_ist_kein_partizip() {
        assert!(stellen("la NATO il 4 aprile 1949").is_empty());
        assert!(stellen("la Nato il 4 aprile 1949").is_empty());
        assert_eq!(
            stellen("Nato il 4 aprile 1949 a Lugano."),
            ["4 aprile 1949"]
        );
    }

    /// Nach einem Namen oder einer Überschrift ohne Satzzeichen beginnt die nächste Zeile
    /// wie ein Satz (`Text::satzanfang_bei`): Lebenslauf- und Personalienblöcke.
    #[test]
    fn partizip_nach_weichem_umbruch() {
        for (roh, datum) in [
            (
                "Erika Beispiel\nGeboren am 12. März 1985 in Zürich\nSeestrasse 12, 8002 Zürich",
                "12. März 1985",
            ),
            (
                "Personalien\nGeboren am 12. März 1985, leidet an einer psychischen Störung.",
                "12. März 1985",
            ),
            (
                "Marie Exemple\nNée le 12 mars 1985 à Genève\nRue du Lac 12, 1201 Genève",
                "12 mars 1985",
            ),
            (
                "Dossier médical\nNée le 12 mars 1985 à Genève, la patiente souffre d'une dépression.",
                "12 mars 1985",
            ),
            (
                "Giulia Esempio\nNata il 12 marzo 1985 a Lugano\nVia Lago 12, 6900 Lugano",
                "12 marzo 1985",
            ),
            (
                "Cartella clinica\nNata il 12 marzo 1985 a Lugano, la paziente soffre di disturbi.",
                "12 marzo 1985",
            ),
        ] {
            assert_eq!(stellen(roh), [datum], "{roh:?}");
        }
        // Mitten in der Zeile zählt das gross geschriebene Partizip weiterhin nicht.
        for roh in [
            "Personalien Geboren am 12. März 1985",
            "Notizie\nla Nato il 4 aprile 1949",
        ] {
            assert!(stellen(roh).is_empty(), "{roh:?}");
        }
    }

    /// Zwischen Auslöser und Datum zählt ein weicher Umbruch wie ein Leerzeichen, und
    /// unsichtbare Formatzeichen werden übergangen. Eine Leerzeile trennt.
    #[test]
    fn datum_ueber_weichen_umbruch_und_formatzeichen() {
        for (roh, datum) in [
            ("Die Klientin, geb.\n12.03.1985, fragt nach.", "12.03.1985"),
            (
                "Die Klientin, geboren am\n12.03.1985, fragt nach.",
                "12.03.1985",
            ),
            ("La cliente, née le\n12 mars 1985, demande.", "12 mars 1985"),
            (
                "La cliente, nata il\n12 marzo 1985, chiede.",
                "12 marzo 1985",
            ),
            ("geb.\u{200B} 12.03.1985", "12.03.1985"),
            ("geb. 12.\u{AD}03.1985", "12.\u{AD}03.1985"),
            ("née le\u{2060} 12 mars 1985", "12 mars 1985"),
        ] {
            assert_eq!(stellen(roh), [datum], "{roh:?}");
        }
        // Grenze: «12. März» am Zeilenanfang sieht aus wie der Listenpunkt «12.», der
        // Umbruch davor ist hart.
        for roh in [
            "Die Klientin, geb.\n\n12.03.1985, fragt nach.",
            "Bitte prüfen, geb.\n- 12.03.1985",
            "Die Klientin, geboren am\n12. März 1985, fragt nach.",
        ] {
            assert!(stellen(roh).is_empty(), "{roh:?}");
        }
    }

    #[test]
    fn jeder_monatsname_wird_erkannt() {
        let lex = lexikon::lexikon(LEX_MONATE);
        for e in &lex.eintraege {
            let nummer: u32 = e.merkmal.as_deref().unwrap_or("").parse().unwrap_or(0);
            assert!((1..=12).contains(&nummer), "{}: merkmal", e.begriff);
            let mut proben = vec![e.begriff.clone()];
            if !e.begriff.ends_with('.') {
                proben.push(format!("{}.", e.begriff));
            }
            for p in proben {
                let roh = format!("geboren am 3. {p} 1980");
                assert_eq!(stellen(&roh), [format!("3. {p} 1980")], "{roh}");
            }
        }
    }

    #[test]
    fn monatsformen_eindeutig() {
        let mut gesehen: HashMap<String, String> = HashMap::new();
        for e in &lexikon::lexikon(LEX_MONATE).eintraege {
            let nummer = e.merkmal.clone().unwrap_or_default();
            for f in std::iter::once(&e.begriff).chain(&e.formen) {
                let alt = gesehen.insert(monatsschluessel(f), nummer.clone());
                assert!(
                    alt.is_none_or(|a| a == nummer),
                    "«{f}» steht für zwei Monate"
                );
            }
        }
    }

    #[test]
    fn jedes_merkmal_ist_bekannt() {
        let lex = lexikon::lexikon(LEX_AUSLOESER);
        for e in &lex.eintraege {
            let m = e.merkmal.as_deref().unwrap_or("");
            assert!(MERKMALE.contains(&m), "{}: merkmal «{m}»", e.begriff);
        }
        for m in MERKMALE {
            assert!(
                lex.eintraege
                    .iter()
                    .any(|e| e.merkmal.as_deref() == Some(m)),
                "kein Eintrag mit merkmal «{m}»"
            );
        }
    }

    #[test]
    fn ohne_identifikator_ein_hinweis() {
        let b = crate::pruefe(
            "Mein Mandant, geb. 12.03.1985, fragt.",
            &Optionen::default(),
        );
        let f = b.funde.iter().find(|f| f.regel == REGEL).expect("Fund");
        assert_eq!(f.stelle, "12.03.1985");
        assert_eq!(
            (f.stufe, &f.grund),
            (Stufe::Hinweis, &Grund::OhneIdentifikator)
        );
    }

    #[test]
    fn info_vollstaendig() {
        let i = &info()[0];
        assert_eq!(i.id, REGEL);
        assert_eq!(i.klasse, Klasse::B);
        assert!(i.hinweis.is_none());
        assert_eq!(i.lexika, [LEX_AUSLOESER, LEX_MONATE]);
        assert!(!i.quellen.is_empty() && !i.grenzen.is_empty());
        // Die Lizenz der CLDR-Monatsnamen verweist auf «promptscan licenses» (Reste Runde 1).
        let cldr = i
            .quellen
            .iter()
            .find(|q| q.titel.starts_with("Unicode CLDR 49"))
            .expect("CLDR-Quelle");
        assert!(
            cldr.lizenz
                .is_some_and(|l| l.contains("Unicode-3.0") && l.contains("«promptscan licenses»"))
        );
    }
}
