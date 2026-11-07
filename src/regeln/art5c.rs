//! Besonders schützenswerte Personendaten nach Art. 5 Bst. c DSG (SPEC 5.3).
//!
//! Sechs Regeln, eine je Ziffer: `art5c-1` bis `art5c-6`. Jede Regel sucht die
//! Begriffe ihrer Wortliste `lexika/art5c-N.toml`; jeder Begriff steht wörtlich in
//! einer amtlichen Fundstelle in DE, FR oder IT. Das Modul meldet nur Stellen. Ob
//! daraus ORANGE wird, entscheidet `stufen` (erst mit einem Identifikator im Prompt).
//!
//! Dazu kommt in Ziffer 2 ein einziges Muster: ein ICD-10-Kode (Buchstabe, zwei
//! Ziffern, wahlweise Punkt und eine oder zwei Ziffern) zählt nur, wenn kurz davor
//! im selben Satz ein Auslöser steht («Diagnose», «diagnostic», «diagnosi», «ICD»).
//!
//! Begriffe, die auch Namen sind («Jude», FR «cancer»; Merkmal «name»), zählen nicht,
//! wenn sie in einem Namen liegen, den die Regel `name` erkennt; nur dann wird sie
//! aufgerufen.

use std::sync::LazyLock;

use regex::Regex;

use crate::fund::{Art, Treffer};
use crate::katalog::{Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, LexTreffer, Lexikon};
use crate::normen;
use crate::text::{Text, ist_wortzeichen};

/// Kennung, Ziffer von Art. 5 Bst. c DSG und Wortliste je Regel.
const REGELN: [(&str, u8, &str); 6] = [
    ("art5c-1", 1, "art5c-1.toml"),
    ("art5c-2", 2, "art5c-2.toml"),
    ("art5c-3", 3, "art5c-3.toml"),
    ("art5c-4", 4, "art5c-4.toml"),
    ("art5c-5", 5, "art5c-5.toml"),
    ("art5c-6", 6, "art5c-6.toml"),
];

/// Merkmal eines Eintrags, der als längerer Treffer gewinnt und nichts meldet
/// («DNA-Profil-Gesetz» ist ein Erlass, kein DNA-Profil).
const AUSNAHME: &str = "ausnahme";
/// Merkmal eines Auslösers für ICD-10-Kodes, der selbst gemeldet wird («Diagnose»).
const ICD: &str = "icd";
/// Merkmal eines Auslösers für ICD-10-Kodes, der selbst nichts meldet («ICD»).
const ICD_NUR: &str = "icd-nur";
/// Merkmal eines Eintrags, der nur zählt, wenn eine Dauer in Monaten oder Wochen folgt
/// («enceinte de huit mois»; «l'enceinte de la ville» zählt nicht).
const DAUER: &str = "dauer";
/// Merkmal eines Eintrags, der auch Vor- oder Nachname ist: Er zählt nicht, wenn er in
/// einem Namen liegt, den die Regel `name` erkennt («Jude Bellingham», «Paul Cancer»).
const NAME: &str = "name";
/// Alle Merkmale, die dieses Modul kennt (geprüft im Test `jedes_merkmal_ist_bekannt`).
#[cfg(test)]
const MERKMALE: [&str; 5] = [AUSNAHME, ICD, ICD_NUR, DAUER, NAME];

/// Dauer nach «enceinte de»/«enceinte d'»: Ziffern oder ein französisches Zahlwort
/// (auch zusammengesetzt: «dix-huit», «vingt-et-une»), davor wahlweise «environ»,
/// «près de», «plus de» oder «moins de», danach «mois» oder «semaine(s)».
static DAUER_FOLGT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)^\s*(?:(?:environ|près de|plus de|moins de)\s+)?",
        r"(?:\d+|(?:une|un|deux|trois|quatre|cinq|six|sept|huit|neuf|dix|onze|douze|treize|",
        r"quatorze|quinze|seize|vingt|trente|quarante)",
        r"(?:-(?:et-)?(?:une|un|deux|trois|quatre|cinq|six|sept|huit|neuf))?)",
        r"\s+(?:mois|semaines?)\b",
    ))
    .expect("Dauer-Muster")
});

/// Höchstens so viele Zeichen zwischen dem Ende des Auslösers und dem Kode
/// («Diagnose gemäss ICD-10-GM: F32.1»).
const ICD_ABSTAND: usize = 40;
/// Höchstens so viele Zeichen zwischen zwei Kodes einer Aufzählung («F32.1, F41.0»).
const ICD_KETTE: usize = 12;

/// Aufbau eines ICD-10-Kodes: «a letter in the first position and a number in the
/// second, third and fourth positions. The fourth character follows a decimal point»
/// (WHO, ICD-10 Volume 2, 2019, Abschnitt 2.4); ICD-10-GM kennt eine fünfte Stelle.
static ICD_KODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Z][0-9]{2}(?:\.[0-9]{1,2})?").expect("ICD-10-Muster"));

/// Meldet jede Stelle aus den sechs Wortlisten und, in Ziffer 2, ICD-10-Kodes nach
/// einem Auslöser.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let mut treffer = Vec::new();
    // Erkannte Namen, erst gesucht, wenn ein Eintrag mit Merkmal «name» trifft.
    let mut namen: Option<Vec<Treffer>> = None;
    for (regel, ziffer, datei) in REGELN {
        let lex = lexikon::lexikon(datei);
        let funde = lex.finde(text);
        for f in &funde {
            // Ausnahmen und reine Auslöser melden nichts; «dauer» nur mit folgender Dauer,
            // die dann zur Stelle gehört; «name» nicht innerhalb eines erkannten Namens.
            let ende = match merkmal(lex, f) {
                Some(AUSNAHME | ICD_NUR) => continue,
                Some(DAUER) => match dauer_ende(text.als_str(), f.ende) {
                    Some(ende) => ende,
                    None => continue,
                },
                Some(NAME) => {
                    let namen = namen.get_or_insert_with(|| super::name::erkenne(text));
                    if namen.iter().any(|n| n.start <= f.start && f.ende <= n.ende) {
                        continue;
                    }
                    f.ende
                }
                _ => f.ende,
            };
            treffer.push(
                Treffer::neu(regel, f.start, ende, Art::BesondersSchuetzenswert(ziffer))
                    .mit_beleg(lex.beleg(f.eintrag)),
            );
        }
        if ziffer == 2 {
            treffer.extend(icd_kodes(text, lex, &funde));
        }
    }
    treffer
}

fn merkmal<'a>(lex: &'a Lexikon, f: &LexTreffer) -> Option<&'a str> {
    lex.eintraege[f.eintrag].merkmal.as_deref()
}

/// Ende der Dauer, die unmittelbar auf `ende` folgt («de huit mois»), sonst `None`.
fn dauer_ende(s: &str, ende: usize) -> Option<usize> {
    DAUER_FOLGT.find(&s[ende..]).map(|m| ende + m.end())
}

/// ICD-10-Kodes nach einem Auslöser (Ziffer 2).
///
/// Ein Kode zählt, wenn im selben Satz höchstens [`ICD_ABSTAND`] Zeichen vor ihm ein
/// Auslöser endet oder höchstens [`ICD_KETTE`] Zeichen vor ihm ein gezählter Kode.
/// Belegt wird er mit dem Eintrag des Auslösers.
fn icd_kodes(text: &Text, lex: &Lexikon, funde: &[LexTreffer]) -> Vec<Treffer> {
    let s = text.als_str();
    let ausloeser: Vec<&LexTreffer> = funde
        .iter()
        .filter(|f| matches!(merkmal(lex, f), Some(ICD | ICD_NUR)))
        .collect();
    if ausloeser.is_empty() {
        return Vec::new();
    }
    // Liegt die Stelle von..bis höchstens `hoechstens` Zeichen vor `kode` und im selben
    // Satz?
    let nah = |von: usize, bis: usize, kode: usize, hoechstens: usize| {
        bis <= kode && s[bis..kode].chars().count() <= hoechstens && text.im_selben_satz(von, kode)
    };
    let mut treffer = Vec::new();
    // Start und Ende des zuletzt gezählten Kodes und der Eintrag, der ihn belegt.
    let mut letzter: Option<(usize, usize, usize)> = None;
    for m in ICD_KODE.find_iter(s) {
        let (start, ende) = (m.start(), m.end());
        if !kode_steht_frei(s, start, ende) {
            continue;
        }
        let beleg = letzter
            .filter(|&(von, bis, _)| nah(von, bis, start, ICD_KETTE))
            .map(|(_, _, eintrag)| eintrag)
            .or_else(|| {
                ausloeser
                    .iter()
                    .rev()
                    .find(|f| nah(f.start, f.ende, start, ICD_ABSTAND))
                    .map(|f| f.eintrag)
            });
        if let Some(eintrag) = beleg {
            treffer.push(
                Treffer::neu("art5c-2", start, ende, Art::BesondersSchuetzenswert(2))
                    .mit_zusatz("ICD-10-Kode", "ICD-10 code")
                    .mit_beleg(lex.beleg(eintrag)),
            );
            letzter = Some((start, ende, eintrag));
        }
    }
    treffer
}

/// Steht der Kode für sich? Davor kein Wortzeichen, kein Bindestrich und kein Punkt,
/// danach kein Wortzeichen und kein Bindestrich («Vitamin-B12-Mangel» zählt nicht).
fn kode_steht_frei(s: &str, start: usize, ende: usize) -> bool {
    let davor = s[..start].chars().next_back();
    let danach = s[ende..].chars().next();
    !davor.is_some_and(|c| ist_wortzeichen(c) || c == '-' || c == '.')
        && !danach.is_some_and(|c| ist_wortzeichen(c) || c == '-')
}

// --- Beschreibung für `rules` und `explain` ---------------------------------

const GEPRUEFT: Stand = Stand::Geprueft("2026-11-06");

const fn fedlex(titel: &'static str, url: &'static str) -> Quelle {
    Quelle {
        titel,
        url,
        lizenz: None,
        stand: GEPRUEFT,
    }
}

const DSG: Quelle = fedlex(
    "Bundesgesetz über den Datenschutz (DSG), SR 235.1, Art. 5 Bst. c, Fassung 2025-07-07",
    "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
);
const BOTSCHAFT_1988: Quelle = fedlex(
    "Botschaft zum Bundesgesetz über den Datenschutz, BBl 1988 II 413, S. 446",
    "https://www.fedlex.admin.ch/eli/fga/1988/2_413_421_353/de",
);
const BOTSCHAFT_2017: Quelle = fedlex(
    "Botschaft zur Totalrevision des Bundesgesetzes über den Datenschutz, BBl 2017 6941, S. 7020",
    "https://www.fedlex.admin.ch/eli/fga/2017/2057/de",
);
const BOTSCHAFT_2017_FR: Quelle = fedlex(
    "Message concernant la révision totale de la loi sur la protection des données, FF 2017 6565, p. 6640 et 6641",
    "https://www.fedlex.admin.ch/eli/fga/2017/2057/fr",
);
const BOTSCHAFT_2017_IT: Quelle = fedlex(
    "Messaggio concernente la revisione totale della legge sulla protezione dei dati, FF 2017 5939, p. 6012",
    "https://www.fedlex.admin.ch/eli/fga/2017/2057/it",
);
const OK_DSG_5C: Quelle = Quelle {
    titel: "Onlinekommentar zu Art. 5 Bst. c DSG (OK-Sprecher), Stand 19.01.2026, DOI 10.17176/20260119-171111-0",
    url: "https://onlinekommentar.ch/de/kommentare/dsg5c",
    lizenz: None,
    stand: GEPRUEFT,
};

/// Lizenz der BFS-Tabellen (Nutzungsbedingungen opendata.swiss, Kategorie OPEN-BY).
const BFS_LIZENZ: &str = "OPEN-BY: «Freie Nutzung - Quellenangabe ist Pflicht»";

fn grundlage() -> Vec<&'static normen::Norm> {
    vec![&normen::DSG_5_C, &normen::DSG_30_2_C, &normen::DSG_6_7_A]
}

/// Die gemeinsamen Felder; `quellen` beginnt immer mit dem DSG.
fn regel_info(
    id: &'static str,
    datei: &'static str,
    titel: Txt,
    erkennt: Txt,
    methode: Txt,
    quellen: Vec<Quelle>,
    grenzen: Vec<Txt>,
) -> RegelInfo {
    let mut alle = vec![DSG];
    alle.extend(quellen);
    RegelInfo {
        id,
        klasse: Klasse::C,
        titel,
        erkennt,
        methode,
        hinweis: None,
        grundlage: grundlage(),
        quellen: alle,
        lexika: vec![datei],
        guete: Guete::NichtGemessen,
        grenzen,
    }
}

/// Grenze, die für alle sechs Regeln gilt.
const NENNUNG: Txt = Txt::neu(
    "Ein Begriff ist ein Hinweis, kein Beweis: «Hans Muster fragt nach Krankheit im Arbeitsrecht» meldet ORANGE, obwohl über Hans Muster nichts Gesundheitliches steht. Ohne Identifikator im Prompt bleibt es still oder ein HINWEIS.",
    "A term is an indication, not proof: «Hans Muster asks about illness in employment law» yields ORANGE although nothing about his health is said. Without an identifier in the prompt the rule stays silent or yields a HINWEIS.",
);
/// Grenze, die für alle sechs Regeln gilt.
const LISTE: Txt = Txt::neu(
    "Nur Begriffe mit amtlicher Fundstelle; Umschreibungen, Abkürzungen, Tippfehler und andere Sprachen als DE, FR und IT erkennt die Regel nicht.",
    "Only terms with an official source; paraphrases, abbreviations, typos and languages other than German, French and Italian are not recognised.",
);

/// Die sechs Regeln für `rules` und `explain`.
pub fn info() -> Vec<RegelInfo> {
    vec![
        regel_info(
            "art5c-1",
            "art5c-1.toml",
            Txt::neu(
                "Religiöse, weltanschauliche, politische oder gewerkschaftliche Ansichten oder Tätigkeiten",
                "Religious, philosophical, political or trade union views or activities",
            ),
            Txt::neu(
                "Angaben über Religion und Konfession (Religionszugehörigkeit, Bezeichnungen wie «katholisch», «Muslim», «Jüdin», «musulman» oder «ebreo»), weltanschauliche und politische Überzeugung, politische Betätigung sowie Gewerkschaften und Arbeitnehmerverbände (Art. 5 Bst. c Ziff. 1 DSG).",
                "Information on religion and denomination (religious affiliation, terms such as «katholisch», «Muslim», «Jüdin», «musulman» or «ebreo»), philosophical and political conviction, political activity, and trade unions and workers' associations (Art. 5(c)(1) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-1.toml ohne Rücksicht auf Gross-/Kleinschreibung und nur als ganze Wörter; «Gewerkschaft», FR «syndical» und IT «sindacali» zählen auch am Wortanfang («Gewerkschaftsmitglied», «syndicaliste», «sindacalista»), «Jude» nur gross geschrieben und nicht in einem Namen, den die Regel «name» erkennt («Jude Bellingham»). Die Begriffe stammen wörtlich aus BV, ZGB, AsylG, NDG, OR, dem IAO-Übereinkommen Nr. 98 und dem UNO-Pakt I (IT «sindacato»), die Konfessionen aus der Religionstabelle und der Seite «Religionen» des BFS. «Jude», «Jüdin», «Konfession» und IT «ebreo» stehen in keinem der durchsuchten Erlasse und sind mit Duden und Treccani belegt. Als Ausnahme eingetragen sind und nichts melden: Gemeindeverbände («syndicats de communes»), Genossenschaften der Güterzusammenlegung («syndicat de remaniement»), die Überprüfung durch ein Gericht («sindacato di legittimità»), Aktionärsbindungsverträge («sindacato azionario») und Verfügungen des Gemeindepräsidenten («ordinanza sindacale»); die italienischen Ausnahmen belegt Treccani. Gemeldet wird die Stelle; die Stufe setzt die Stufenlogik.",
                "Searches the terms of lexika/art5c-1.toml case-insensitively and as whole words only; «Gewerkschaft», French «syndical» and Italian «sindacali» also count at the start of a word («Gewerkschaftsmitglied», «syndicaliste», «sindacalista»), «Jude» only capitalised and not inside a name recognised by the rule «name» («Jude Bellingham»). The terms are taken verbatim from the Federal Constitution, Civil Code, Asylum Act, Intelligence Service Act, Code of Obligations, ILO Convention No. 98 and the UN Covenant I (Italian «sindacato»), the denominations from the FSO religion table and the FSO page «Religionen». «Jude», «Jüdin», «Konfession» and Italian «ebreo» appear in none of the acts searched and are sourced from the Duden and Treccani dictionaries. Listed as exceptions that report nothing: municipal associations («syndicats de communes»), land consolidation cooperatives («syndicat de remaniement»), judicial review («sindacato di legittimità»), shareholder agreements («sindacato azionario») and orders of the mayor («ordinanza sindacale»); Treccani is the source for the Italian exceptions. The rule reports the passage; the stage logic sets the level.",
            ),
            vec![
                fedlex(
                    "Bundesverfassung, SR 101, Art. 8 Abs. 2 und Art. 15 Abs. 2, Fassung 2024-03-03",
                    "https://www.fedlex.admin.ch/eli/cc/1999/404/de#art_15",
                ),
                fedlex(
                    "Zivilgesetzbuch, SR 210, Art. 303, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_303",
                ),
                fedlex(
                    "Asylgesetz, SR 142.31, Art. 3 Abs. 1, Fassung 2026-06-12",
                    "https://www.fedlex.admin.ch/eli/cc/1999/358/de#art_3",
                ),
                fedlex(
                    "Nachrichtendienstgesetz, SR 121, Art. 5 Abs. 5, Fassung 2026-11-01",
                    "https://www.fedlex.admin.ch/eli/cc/2017/494/de#art_5",
                ),
                fedlex(
                    "Obligationenrecht, SR 220, Art. 336 Abs. 2 Bst. a, Fassung 2026-10-01",
                    "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_336",
                ),
                fedlex(
                    "IAO-Übereinkommen Nr. 98, SR 0.822.719.9, Art. 1 Ziff. 2, Fassung 2023-08-31",
                    "https://www.fedlex.admin.ch/eli/cc/2001/188/de#art_1",
                ),
                fedlex(
                    "Patto internazionale relativo ai diritti economici, sociali e culturali (UNO-Pakt I), RS 0.103.1, art. 8 cpv. 1 lett. a (IT «sindacato»), Fassung 2024-11-28",
                    "https://www.fedlex.admin.ch/eli/cc/1993/725_725_725/it#art_8",
                ),
                Quelle {
                    titel: "BFS, Ständige Wohnbevölkerung ab 15 Jahren nach Religionszugehörigkeit, Tabelle su-d-40.02.01.08.26 (Strukturerhebung 2024), SHA-256 b3edbbe2340f0a835a6a7382065e2bb59329bb3b41954dd05263933a30079b49",
                    url: "https://dam-api.bfs.admin.ch/hub/api/dam/assets/36347570/master",
                    lizenz: Some(BFS_LIZENZ),
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "OFS, tableau su-f-40.02.01.08.26 (relevé structurel 2024), SHA-256 96c04bc34f08d147f5cbc2842d14dafe7df961b99548fad1aec55233512b10e7",
                    url: "https://dam-api.bfs.admin.ch/hub/api/dam/assets/36347562/master",
                    lizenz: Some(BFS_LIZENZ),
                    stand: GEPRUEFT,
                },
                Quelle {
                    titel: "UST, tabella su-i-40.02.01.08.26 (rilevazione strutturale 2024), SHA-256 273d50b343f6901ac89755dfda2cba97270d6e918be3ed052becd24f6a67ad97",
                    url: "https://dam-api.bfs.admin.ch/hub/api/dam/assets/36347569/master",
                    lizenz: Some(BFS_LIZENZ),
                    stand: GEPRUEFT,
                },
                fedlex(
                    "BFS, Religionen (Themenseite ohne Datum, abgerufen 2026-11-06)",
                    "https://statistik.admin.ch/de/themen/bevoelkerung/religionen",
                ),
                fedlex(
                    "Constitution du canton de Berne, RS 131.212, art. 110 al. 2 (Ausnahme «syndicats de communes»), Fassung 2026-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/1994/1_401_401_361_fga/fr#art_110",
                ),
                fedlex(
                    "Loi fédérale sur le droit foncier rural, RS 211.412.11, art. 2 al. 4 (Ausnahme «syndicat de remaniement»), Fassung 2014-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/1993/1410_1410_1410/fr#art_2",
                ),
                fedlex(
                    "Duden online, «Jude» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/Jude",
                ),
                fedlex(
                    "Duden online, «Jüdin» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/Juedin",
                ),
                fedlex(
                    "Duden online, «Konfession» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/Konfession",
                ),
                fedlex(
                    "Treccani, Vocabolario, «ebreo» (abgerufen 2026-11-06)",
                    "https://www.treccani.it/vocabolario/ebreo/",
                ),
                fedlex(
                    "Treccani, Vocabolario, «sindacato¹» (Ausnahmen «sindacato di legittimità», «sindacato di merito», «sindacato di costituzionalità», abgerufen 2026-11-06)",
                    "https://www.treccani.it/vocabolario/sindacato1/",
                ),
                fedlex(
                    "Treccani, Vocabolario, «sindacato²» (Ausnahmen «sindacato azionario», «sindacato di blocco», abgerufen 2026-11-06)",
                    "https://www.treccani.it/vocabolario/sindacato2/",
                ),
                fedlex(
                    "Treccani, Vocabolario, «sindacale¹» (Ausnahmen «ordinanza sindacale», «deliberazione sindacale», abgerufen 2026-11-06)",
                    "https://www.treccani.it/vocabolario/sindacale1/",
                ),
                BOTSCHAFT_1988,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "Parteinamen fehlen: Der Name einer Partei sagt allein nichts über eine Person, und eine amtliche Liste mit Mitgliedschaftswendungen gibt es nicht. «Mitglied der SVP» bleibt unerkannt.",
                    "Party names are missing: a party name alone says nothing about a person, and there is no official list of membership phrases. «member of the SVP» is not recognised.",
                ),
                Txt::neu(
                    "«reformiert» allein (auch: umgestaltet), «konfessionslos» (keine amtliche Fundstelle), FR «confession» allein und FR/IT «protestant(e)» (auch Partizip) fehlen bewusst. Abkürzungen fehlen ebenso: In «Konfession: röm.-kath.» wird nur «Konfession» erkannt.",
                    "«reformiert» alone (also: reformed), «konfessionslos» (no official source), French «confession» alone and French/Italian «protestant(e)» (also a participle) are left out on purpose. Abbreviations are missing too: in «Konfession: röm.-kath.» only «Konfession» is recognised.",
                ),
                Txt::neu(
                    "IT «sindacato» ist auch Partizip von «sindacare» (überprüfen): «l'operato è stato sindacato» zählt als Treffer. Ebenso zählen «collegio sindacale» (Organ einer Aktiengesellschaft nach italienischem Recht) und «patto di sindacato» (Aktionärsbindung), weil sich für sie keine Fundstelle fand.",
                    "Italian «sindacato» is also the participle of «sindacare» (to review): «l'operato è stato sindacato» counts as a hit. «collegio sindacale» (a board of an Italian company) and «patto di sindacato» (shareholder agreement) count too, as no source was found for them.",
                ),
                Txt::neu(
                    "«Jude» zählt nicht, wenn die Regel «name» es als Teil eines Namens erkennt. Das trifft auch Wendungen wie «der Jude Nathan», die dann still bleiben; «Herr Muster ist Jude» zählt. Namen, die die Regel «name» nicht erkennt, schützt das nicht.",
                    "«Jude» does not count when the rule «name» recognises it as part of a name. This also hits phrases such as «der Jude Nathan», which then stay silent; «Herr Muster ist Jude» counts. Names that the rule «name» does not recognise are not protected.",
                ),
                Txt::neu(
                    "FR «syndicat» meldet auch Verbände, die keine Gewerkschaften sind, wenn keine Ausnahme eingetragen ist: «syndicat d'améliorations foncières» und «syndicat intercommunal» zählen als Treffer.",
                    "French «syndicat» also fires on associations that are not trade unions unless an exception is listed: «syndicat d'améliorations foncières» and «syndicat intercommunal» count as hits.",
                ),
                Txt::neu(
                    "«Religion», «katholisch», «jüdisch» und Ähnliches melden auch Sachaussagen («jüdisches Museum», «Religion und Staat»), sobald irgendwo im Prompt ein Identifikator steht.",
                    "«Religion», «katholisch», «jüdisch» and similar terms also fire on factual statements («Jewish museum», «religion and state») as soon as the prompt contains an identifier anywhere.",
                ),
            ],
        ),
        regel_info(
            "art5c-2",
            "art5c-2.toml",
            Txt::neu(
                "Gesundheit, Intimsphäre, Rasse oder Ethnie",
                "Health, intimate sphere, race or ethnicity",
            ),
            Txt::neu(
                "Angaben über Krankheiten (auch «krank», «krankgeschrieben», «Krebserkrankung», «HIV», «Diabetes», «Diabetiker», «Depression», «Sucht», «addiction», «Burn-out»), Gesundheitszustand, Arztzeugnisse, Suizid, Arbeits- und Erwerbsunfähigkeit, Invalidität, Invalidenrente und IV-Rente («rente AI», «rendita AI»), Schwangerschaft, psychische Störung, psychische, geistige und körperliche Behinderung, Diagnosen samt ICD-10-Kodes, sexuelle Orientierung und Ethnie (Art. 5 Bst. c Ziff. 2 DSG).",
                "Information on illnesses (also «krank», «krankgeschrieben», «Krebserkrankung», «HIV», «Diabetes», «Diabetiker», «Depression», «Sucht», «addiction», «Burn-out»), state of health, medical certificates, suicide, incapacity for work or gainful employment, invalidity, invalidity pensions and IV pensions («rente AI», «rendita AI»), pregnancy, mental disorder, mental, intellectual and physical disability, diagnoses including ICD-10 codes, sexual orientation and ethnicity (Art. 5(c)(2) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-2.toml als ganze Wörter. Auch am Wortende zählen «Krankheit», «Erkrankung» und «Diagnose» («Lungenkrankheit», «Herzerkrankung», «Verdachtsdiagnose»), auch am Wortanfang «Arbeitsunfähigkeit», «Invalidität», «Schwangerschaft», «Selbstmord» und «Suizid» («Suizidversuch», «suizidal»). «HIV», «VIH», «ICD» und «Sucht» zählen nur gross geschrieben (das Verb «sucht» bleibt still); bei «Ethnie» und «Burn-out» entscheidet die Schreibung, ob der deutsche oder der französische Beleg gilt. IT «rendita AI» zählt nur genau so geschrieben («rendita ai superstiti» bleibt still, am Satzanfang aber auch «Rendita AI»), FR «cancer» nicht in einem Namen, den die Regel «name» erkennt («M. Paul Cancer»). Als Ausnahme eingetragen sind und nichts melden nur diese Namen von Versicherungen, Versicherern und Kassen: «Kranken- und Unfallversicherung», «assurance-maladie», «caisse-maladie», «assureur-maladie», «assicurazione malattie», «assicurazione malattia», «assicuratore malattia», «assicuratore-malattie» und «cassa malati»; dazu die wirtschaftliche Abhängigkeit («forme de dépendance économique», «forma di dipendenza economica»). FR «enceinte» zählt nur in Wendungen («est enceinte», «mise enceinte», «femmes enceintes») oder mit einer Dauer in Monaten oder Wochen («enceinte de huit mois»). Die Begriffe stammen wörtlich aus Erlassen (ATSG, UVG, KVG, KVV, KVAG, KLV, EpG, Krebsregistrierungsgesetz, BetmG, IVG, IVV, ZGB, OR, BV, BehiG, ArG, StGB, MStG, StPO, SVG, AsylG, DSG, Übereinkommen gegen Rassendiskriminierung) und aus der Botschaft zum DSG; «Suizid», «Depression» und ihre Gegenstücke aus Seiten des BAG, «krankgeschrieben», «Burn-out» und FR «enceinte» aus Duden und Larousse, weil sie in keinem der durchsuchten Erlasse stehen. Ein ICD-10-Kode (Buchstabe, zwei Ziffern, wahlweise Punkt und ein bis zwei Ziffern; WHO, ICD-10 Band 2, Abschnitt 2.4) zählt nur, wenn im selben Satz höchstens 40 Zeichen davor «Diagnose», «diagnostic», «diagnosi» oder «ICD» steht; in einer Aufzählung genügen 12 Zeichen zum vorigen Kode. Ein einzelner Zeilenumbruch mitten im Absatz trennt den Satz nicht («Diagnose⏎F32.1»), und unsichtbare Formatzeichen wie das weiche Trennzeichen stören weder im Auslöser noch zwischen Auslöser und Kode.",
                "Searches the terms of lexika/art5c-2.toml as whole words. «Krankheit», «Erkrankung» and «Diagnose» also count at the end of a word («Lungenkrankheit», «Herzerkrankung», «Verdachtsdiagnose»), «Arbeitsunfähigkeit», «Invalidität», «Schwangerschaft», «Selbstmord» and «Suizid» also at the start («Suizidversuch», «suizidal»). «HIV», «VIH», «ICD» and «Sucht» only count capitalised (the verb «sucht» stays silent); for «Ethnie» and «Burn-out» the spelling decides whether the German or the French source applies. Italian «rendita AI» only counts when written exactly so («rendita ai superstiti» stays silent, but so does «Rendita AI» at the start of a sentence), French «cancer» not inside a name recognised by the rule «name» («M. Paul Cancer»). Only these names of insurances, insurers and insurance funds are listed as exceptions that report nothing: «Kranken- und Unfallversicherung», «assurance-maladie», «caisse-maladie», «assureur-maladie», «assicurazione malattie», «assicurazione malattia», «assicuratore malattia», «assicuratore-malattie» and «cassa malati»; also economic dependence («forme de dépendance économique», «forma di dipendenza economica»). French «enceinte» only counts in phrases («est enceinte», «mise enceinte», «femmes enceintes») or followed by a duration in months or weeks («enceinte de huit mois»). The terms are taken verbatim from federal acts and ordinances (social insurance, health insurance and its supervision, epidemics, cancer registration, narcotics, disability insurance, Civil Code, Code of Obligations, Constitution, Disability Discrimination Act, Labour Act, Criminal Code, Military Criminal Code, Criminal Procedure Code, Road Traffic Act, Asylum Act, FADP, Convention on the Elimination of Racial Discrimination) and from the FADP dispatch; «Suizid», «Depression» and their counterparts from pages of the Federal Office of Public Health, «krankgeschrieben», «Burn-out» and French «enceinte» from the Duden and Larousse dictionaries, because none of the acts searched contains them. An ICD-10 code (letter, two digits, optionally a point and one or two digits; WHO ICD-10 Volume 2, section 2.4) only counts if «Diagnose», «diagnostic», «diagnosi» or «ICD» stands at most 40 characters before it in the same sentence; in a list 12 characters to the previous code suffice. A single line break in the middle of a paragraph does not split the sentence («Diagnose⏎F32.1»), and invisible format characters such as the soft hyphen do not interfere, neither in the trigger nor between trigger and code.",
            ),
            vec![
                fedlex(
                    "ATSG, SR 830.1, Art. 3 und 5 bis 8, Fassung 2024-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/2002/510/de#art_3",
                ),
                fedlex(
                    "UVG, SR 832.20, Art. 16 Abs. 1, Fassung 2026-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/1982/1676_1676_1676/de#art_16",
                ),
                fedlex(
                    "KVG, SR 832.10, Art. 1a, 31 und 42 Abs. 3bis, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/1995/1328_1328_1328/de#art_42",
                ),
                fedlex(
                    "Krankenpflege-Leistungsverordnung, SR 832.112.31, Art. 7, 11c, 12d, 12e und 19b, Fassung 2026-08-01",
                    "https://www.fedlex.admin.ch/eli/cc/1995/4964_4964_4964/de#art_12_e",
                ),
                fedlex(
                    "Epidemiengesetz, SR 818.101, Art. 5 und 33, Fassung 2025-08-01",
                    "https://www.fedlex.admin.ch/eli/cc/2015/297/de#art_33",
                ),
                fedlex(
                    "Krebsregistrierungsgesetz, SR 818.33, Art. 3 und 21, Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/2018/289/de#art_3",
                ),
                fedlex(
                    "IVG, SR 831.20, Art. 6, 7a, 28a, 35 und 57, Fassung 2026-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/1959/827_857_845/de#art_35",
                ),
                fedlex(
                    "IVV, SR 831.201, Art. 31 («IV-Rente», «rente AI», «rendita AI»), Fassung 2025-06-01",
                    "https://www.fedlex.admin.ch/eli/cc/1961/29_29_29/de#art_31",
                ),
                fedlex(
                    "Ordonnance sur l'assurance-maladie, RS 832.102, art. 11 al. 2 (Ausnahme «assureur-maladie»), Fassung 2026-10-22",
                    "https://www.fedlex.admin.ch/eli/cc/1995/3867_3867_3867/fr#art_11",
                ),
                fedlex(
                    "Legge sulla vigilanza sull'assicurazione malattie, RS 832.12, art. 7 cpv. 2 lett. p (Ausnahme «assicurazione malattia»), Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/2015/839/it#art_7",
                ),
                fedlex(
                    "Loi sur les stupéfiants, RS 812.121, art. 3b («addiction»), Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/1952/241_241_245/fr#art_3_b",
                ),
                fedlex(
                    "Code des obligations, RS 220, art. 728 al. 2 ch. 5 (Ausnahme «dépendance économique»), Fassung 2026-10-01",
                    "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/fr#art_728",
                ),
                fedlex(
                    "Zivilgesetzbuch, SR 210, Art. 390 Abs. 1, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_390",
                ),
                fedlex(
                    "Bundesverfassung, SR 101, Art. 8 Abs. 2 und Art. 117 Abs. 1, Fassung 2024-03-03",
                    "https://www.fedlex.admin.ch/eli/cc/1999/404/de#art_8",
                ),
                fedlex(
                    "Behindertengleichstellungsgesetz, SR 151.3, Art. 2 Abs. 1, Fassung 2020-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/2003/667/de#art_2",
                ),
                fedlex(
                    "Arbeitsgesetz, SR 822.11, Art. 17c, 35 und 35a, Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/1966/57_57_57/de#art_35",
                ),
                fedlex(
                    "Strafprozessordnung, SR 312.0, Art. 195 Abs. 1, Fassung 2025-04-01",
                    "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_195",
                ),
                fedlex(
                    "Strassenverkehrsgesetz, SR 741.01, Art. 16d Abs. 1, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/1959/679_705_685/de#art_16_d",
                ),
                fedlex(
                    "Code pénal militaire, RS 321.0, art. 109 al. 1 («mise enceinte»), Fassung 2026-06-08",
                    "https://www.fedlex.admin.ch/eli/cc/43/359_375_369/fr#art_109",
                ),
                fedlex(
                    "Loi sur l'asile, RS 142.31, art. 99a al. 3 («ethnie»), Fassung 2026-06-12",
                    "https://www.fedlex.admin.ch/eli/cc/1999/358/fr#art_99_a",
                ),
                fedlex(
                    "Convenzione sull'eliminazione di ogni forma di discriminazione razziale, RS 0.104, art. 4 («origine etnica»), Fassung 2022-04-21",
                    "https://www.fedlex.admin.ch/eli/cc/1995/1164_1164_1164/it#art_4",
                ),
                fedlex(
                    "Strafgesetzbuch, SR 311.0, Art. 115, 118 und 261bis, Fassung 2026-11-01",
                    "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_261_bis",
                ),
                Quelle {
                    titel: "WHO, ICD-10 Volume 2, Instruction manual, Sixth Edition 2019, Abschnitt 2.4, SHA-256 e9157b38cbc2609a4ca02835a3cd43f4e78094ded0e5cec9fd0b28fcf3c2378e",
                    url: "https://icd.who.int/browse10/Content/statichtml/ICD10Volume2_en_2019.pdf",
                    lizenz: Some("CC BY-ND 3.0 IGO"),
                    stand: GEPRUEFT,
                },
                fedlex(
                    "BFS, Instrumente zur medizinischen Kodierung (ICD-10-GM)",
                    "https://www.bfs.admin.ch/bfs/de/home/statistiken/gesundheit/nomenklaturen/medkk/instrumente-medizinische-kodierung.html",
                ),
                fedlex(
                    "BAG, Aktionsplan Suizidprävention (abgerufen 2026-11-06)",
                    "https://www.bag.admin.ch/de/aktionsplan-suizidpraevention",
                ),
                fedlex(
                    "OFSP, Plan d'action pour la prévention du suicide (abgerufen 2026-11-06)",
                    "https://www.bag.admin.ch/fr/plan-daction-pour-la-prevention-du-suicide",
                ),
                fedlex(
                    "UFSP, Piano d'azione per la prevenzione del suicidio (abgerufen 2026-11-06)",
                    "https://www.bag.admin.ch/it/piano-dazione-prevenzione-del-suicidio",
                ),
                fedlex(
                    "Duden online, «krankschreiben» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/krankschreiben",
                ),
                fedlex(
                    "Duden online, «Burn-out» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/Burn_out_Raumfahrt_Syndrom",
                ),
                fedlex(
                    "Larousse, «burn out» (abgerufen 2026-11-06)",
                    "https://www.larousse.fr/dictionnaires/francais/burn_out/10910385",
                ),
                fedlex(
                    "Larousse, «enceinte» (abgerufen 2026-11-06)",
                    "https://www.larousse.fr/dictionnaires/francais/enceinte/29121",
                ),
                BOTSCHAFT_2017,
                BOTSCHAFT_2017_FR,
                BOTSCHAFT_2017_IT,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "«Rasse», «race» und «razza» allein fehlen (auch Tierrassen), ebenso FR «appartenance ethnique» (keine wörtliche Fundstelle). Erkannt werden «Ethnie», «ethnische Herkunft», «ethnie», «origine ethnique», «appartenance raciale», «origine raciale», «etnia», «appartenenza etnica» und «origine etnica».",
                    "«Rasse», «race» and «razza» alone are missing (also animal breeds), as is French «appartenance ethnique» (no verbatim source). Recognised are «Ethnie», «ethnische Herkunft», «ethnie», «origine ethnique», «appartenance raciale», «origine raciale», «etnia», «appartenenza etnica» and «origine etnica».",
                ),
                Txt::neu(
                    "FR «invalidité» und IT «invalidità» fehlen (auch: Ungültigkeit eines Vertrags), ebenso DE «invalid», «Behinderte» (auch Partizip von «behindern») und «Behinderung» allein (auch: Hindernis), FR «enceinte» allein (auch Umfassung, Lautsprecher) und IT «parto» (auch Verbform).",
                    "French «invalidité» and Italian «invalidità» are missing (also: invalidity of a contract), as are German «invalid», «Behinderte» (also a participle of «behindern») and «Behinderung» alone (also: obstruction), French «enceinte» alone (also: enclosure, loudspeaker) and Italian «parto» (also a verb form).",
                ),
                Txt::neu(
                    "Es fehlen DE «Krebs» allein (auch Familienname und Tier), «Unfall» (auch blosser Sachschaden), «Mutterschaft» (nicht notwendig eine Gesundheitsangabe), FR «arrêt de travail» (auch Arbeitsniederlegung), DE «Abhängigkeit», FR «dépendance» und IT «dipendenza» allein (auch wirtschaftliche Abhängigkeit, FR auch Nebengebäude) sowie Wörter, für die keine amtliche Fundstelle gefunden wurde, etwa «depressiv», «alkoholkrank», «Herzinfarkt» und «PTBS». Ganze Klassifikationen (ICD-Titel) sind nicht übernommen.",
                    "Missing are German «Krebs» alone (also a surname and an animal), «Unfall» (also mere property damage), «Mutterschaft» (not necessarily health information), French «arrêt de travail» (also a work stoppage), German «Abhängigkeit», French «dépendance» and Italian «dipendenza» alone (also economic dependence, in French also an outbuilding) and words for which no official source was found, such as «depressiv», «alkoholkrank», «Herzinfarkt» and «PTBS». Whole classifications (ICD titles) are not included.",
                ),
                Txt::neu(
                    "«krank» meldet auch übertragenen Gebrauch («Das ist doch krank.») und das Verb («Die Firmen kranken an hohen Kosten.»), «Krankheit» am Wortende auch Bilder wie «Die Software hat noch Kinderkrankheiten.», «Suizid» und «Selbstmord» am Wortanfang auch Themenwörter wie «Suizidprävention», «Depression», «dépression» und «depressione» auch Wetter und Wirtschaft («Die Depression der Dreissigerjahre»). «Sucht» zählt gross geschrieben, am Satzanfang darum auch als Verb («Sucht er eine Stelle?»).",
                    "«krank» also fires in the figurative sense («Das ist doch krank.») and on the verb («Die Firmen kranken an hohen Kosten.»), «Krankheit» at the end of a word also on images such as «Die Software hat noch Kinderkrankheiten.», «Suizid» and «Selbstmord» at the start of a word also on topic words such as «Suizidprävention», «Depression», «dépression» and «depressione» also on weather and economics («Die Depression der Dreissigerjahre»). «Sucht» counts when capitalised, hence also as a verb at the start of a sentence («Sucht er eine Stelle?»).",
                ),
                Txt::neu(
                    "Übertragener Gebrauch zählt auch sonst: «état de santé financière», «stato di salute dei conti», «Gesundheitszustand der Pensionskasse», «le cancer de la bureaucratie», «l'économie est malade», «è malato di calcio», «addiction au chocolat» und «Sucht nach Anerkennung» melden einen Treffer, ebenso das Sternzeichen («nato sotto il segno del Cancro»). Für diese festen Wendungen fand sich keine Fundstelle, die eine Ausnahme tragen könnte.",
                    "Figurative use counts elsewhere too: «état de santé financière», «stato di salute dei conti», «Gesundheitszustand der Pensionskasse», «le cancer de la bureaucratie», «l'économie est malade», «è malato di calcio», «addiction au chocolat» and «Sucht nach Anerkennung» report a hit, as does the zodiac sign («nato sotto il segno del Cancro»). No source was found for these fixed phrases that could carry an exception.",
                ),
                Txt::neu(
                    "Die Ausnahmen für Versicherungen und Kassen verdecken auch echte Hinweise: «indemnités journalières de l'assurance-maladie» meldet nichts, ebenso DE «Krankentaggeld». Nicht eingetragene Namen zählen: In «assurance perte de gain maladie» meldet «maladie» einen Treffer. FR «enceinte» zählt nur in den eingetragenen Wendungen; «une employée enceinte», «une collaboratrice enceinte», «une travailleuse enceinte» und «elle est tombée enceinte» bleiben unerkannt (die Erlasse schreiben nur «femme enceinte»).",
                    "The exceptions for insurances and insurance funds also hide genuine indications: «indemnités journalières de l'assurance-maladie» reports nothing, nor does German «Krankentaggeld». Names that are not listed count: in «assurance perte de gain maladie», «maladie» reports a hit. French «enceinte» only counts in the listed phrases; «une employée enceinte», «une collaboratrice enceinte», «une travailleuse enceinte» and «elle est tombée enceinte» are not recognised (the acts only write «femme enceinte»).",
                ),
                Txt::neu(
                    "FR «cancer» schweigt nur in Namen, die die Regel «name» erkennt; ein Nachname, den sie nicht erkennt, zählt als Krankheit.",
                    "French «cancer» only stays silent inside names recognised by the rule «name»; a surname it does not recognise counts as an illness.",
                ),
                Txt::neu(
                    "Jede Wortliste wird in jedem Text in allen drei Sprachen gesucht. Treffen gleich geschriebene Einträge mehrerer Sprachen dieselbe Stelle («HIV» deutsch und italienisch), gilt der Beleg in der Sprache des Textes; sie wird an häufigen Funktionswörtern geschätzt («und», «est», «è», «di»), eine geschlossene Liste im Code. Ist sie nicht eindeutig, gewinnt der frühere Eintrag. «burnout» hat keinen italienischen Eintrag und trägt auch in italienischen Texten den französischen Beleg.",
                    "Every word list is searched in every text in all three languages. When entries of several languages with the same spelling match the same passage (German and Italian «HIV»), the source in the language of the text applies; it is estimated from frequent function words («und», «est», «è», «di»), a closed list in the code. If it is not clear, the earlier entry wins. «burnout» has no Italian entry and carries the French source in Italian texts too.",
                ),
                Txt::neu(
                    "«Diagnose», «diagnostic» und «diagnosi» melden auch technische Diagnosen («Fehlerdiagnose»). Nach einem Auslöser zählt jede Folge Buchstabe-Ziffer-Ziffer, auch eine, die kein ICD-Kode ist («Diagnose: Mangel an Vitamin B12.»).",
                    "«Diagnose», «diagnostic» and «diagnosi» also fire on technical diagnoses («Fehlerdiagnose»). After a trigger every letter-digit-digit sequence counts, including one that is not an ICD code («Diagnose: Mangel an Vitamin B12.»).",
                ),
                Txt::neu(
                    "Eine Leerzeile, ein Listenpunkt oder ein Satzzeichen am Zeilenende («.», «!», «?», «:») beginnt einen neuen Satz: In «Diagnose:⏎F32.1» zählt nur «Diagnose», nicht der Kode. Ein Formatzeichen mitten im Kode verhindert seine Erkennung.",
                    "A blank line, a list item or a punctuation mark at the end of a line («.», «!», «?», «:») starts a new sentence: in «Diagnose:⏎F32.1» only «Diagnose» counts, not the code. A format character inside the code prevents its detection.",
                ),
                Txt::neu(
                    "Intimsphäre nur über «sexuelle Orientierung» und ihre Übersetzungen; die übrige Intimsphäre (Sexualleben, Geschlechtsidentität) hat keine eindeutigen amtlichen Begriffe.",
                    "The intimate sphere only via «sexual orientation» and its translations; the rest of it (sex life, gender identity) has no unambiguous official terms.",
                ),
            ],
        ),
        regel_info(
            "art5c-3",
            "art5c-3.toml",
            Txt::neu("Genetische Daten", "Genetic data"),
            Txt::neu(
                "Angaben über genetische Daten, genetische Untersuchungen, das Erbgut und das DNA-Profil einer Person (Art. 5 Bst. c Ziff. 3 DSG).",
                "Information on genetic data, genetic testing, the genome and the DNA profile of a person (Art. 5(c)(3) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-3.toml (GUMG, DNA-Profil-Gesetz, DSG) als ganze Wörter. Das DNA-Profil gehört nach der Botschaft (BBl 2017 7020) zu den genetischen Daten, nicht zu den biometrischen. Der Gesetzesname «DNA-Profil-Gesetz» ist als Ausnahme eingetragen und meldet nichts.",
                "Searches the terms of lexika/art5c-3.toml (Human Genetic Testing Act, DNA Profiles Act, FADP) as whole words. According to the dispatch (BBl 2017 7020) the DNA profile belongs to genetic data, not biometric data. The act's name «DNA-Profil-Gesetz» is listed as an exception and reports nothing.",
            ),
            vec![
                fedlex(
                    "GUMG, SR 810.12, Art. 3, Fassung 2025-08-01",
                    "https://www.fedlex.admin.ch/eli/cc/2022/537/de#art_3",
                ),
                fedlex(
                    "DNA-Profil-Gesetz, SR 363, Art. 2, Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/2004/811/de#art_2",
                ),
                BOTSCHAFT_2017,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "Der Prompt enthält genetische Daten fast nie selbst (Sequenzen, Allele); die Regel meldet ihre Nennung. Rohe Sequenzen erkennt sie nicht.",
                    "A prompt almost never contains genetic data itself (sequences, alleles); the rule reports their mention. Raw sequences are not recognised.",
                ),
                Txt::neu(
                    "Die Erlasstitel in FR und IT («Loi sur les profils d'ADN», «Legge sui profili del DNA») sind nicht als Ausnahme eingetragen und melden «profils d'ADN» bzw. «profili del DNA».",
                    "The French and Italian titles of the DNA Profiles Act are not listed as exceptions and report «profils d'ADN» or «profili del DNA».",
                ),
            ],
        ),
        regel_info(
            "art5c-4",
            "art5c-4.toml",
            Txt::neu(
                "Biometrische Daten, die eine Person eindeutig identifizieren",
                "Biometric data uniquely identifying a person",
            ),
            Txt::neu(
                "Nennungen biometrischer Daten: biometrische Daten, Fingerabdrücke, Gesichtsbild (Art. 5 Bst. c Ziff. 4 DSG).",
                "Mentions of biometric data: biometric data, fingerprints, facial image (Art. 5(c)(4) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-4.toml (DSG, Ausweisgesetz, Botschaft zum DSG) als ganze Wörter. Gewöhnliche Fotografien sind nach der Botschaft (BBl 2017 7020) nicht erfasst und fehlen darum. Gesichtsbilder nennt die Botschaft ausdrücklich; FR «images faciales» stammt aus ihrer französischen Fassung (FF 2017 6641), weil das Ausweisgesetz auf Französisch nur «photographie» schreibt.",
                "Searches the terms of lexika/art5c-4.toml (FADP, Identity Documents Act, FADP dispatch) as whole words. According to the dispatch (BBl 2017 7020) ordinary photographs are not covered and are therefore missing. The dispatch names facial images explicitly; French «images faciales» is taken from its French version (FF 2017 6641), because the French Identity Documents Act only says «photographie».",
            ),
            vec![
                fedlex(
                    "Ausweisgesetz, SR 143.1, Art. 2 Abs. 2bis, Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/2002/441/de#art_2",
                ),
                BOTSCHAFT_2017,
                BOTSCHAFT_2017_FR,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "Ein Text-Prompt enthält biometrische Daten selbst nicht (Bild, Abdruck, Stimme); die Regel meldet nur ihre Nennung. Angehängte Bilder prüft promptscan nicht.",
                    "A text prompt does not contain biometric data itself (image, print, voice); the rule only reports their mention. Attached images are not checked.",
                ),
                Txt::neu(
                    "«Fingerabdruck» meldet auch den übertragenen Sinn («digitaler Fingerabdruck» eines Browsers). Iris und Stimme fehlen («Iris» ist auch ein Vorname).",
                    "«Fingerabdruck» also fires in the figurative sense (a browser's digital fingerprint). Iris and voice are missing («Iris» is also a first name).",
                ),
            ],
        ),
        regel_info(
            "art5c-5",
            "art5c-5.toml",
            Txt::neu(
                "Verwaltungs- und strafrechtliche Verfolgungen oder Sanktionen",
                "Administrative and criminal proceedings or sanctions",
            ),
            Txt::neu(
                "Angaben über Strafverfahren und Strafuntersuchung («enquête pénale», «inchiesta penale»), Strafanzeige und Strafantrag (auch FR «plainte pénale», «porter plainte», IT «querela»), Anklage, Verhaftung und Untersuchungshaft, Strafbefehl, Geld- und Freiheitsstrafe, Ordnungsbusse, Strafregister und Vorstrafen (auch «vorbestraft»), Landesverweisung, Disziplinarverfahren und Ausweisentzug (Art. 5 Bst. c Ziff. 5 DSG).",
                "Information on criminal proceedings and criminal investigations («enquête pénale», «inchiesta penale»), criminal complaints (also French «plainte pénale», «porter plainte», Italian «querela»), indictment, arrest and pre-trial detention, summary penalty orders, monetary and custodial penalties, fixed fines, criminal records and previous convictions (also «vorbestraft»), expulsion, disciplinary proceedings and licence withdrawal (Art. 5(c)(5) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-5.toml (StPO, StGB, OBG, StReG, BGFA, SVG, BPG, URG, DSG) als ganze Wörter; «Strafbefehl» und «Strafregister» zählen auch am Wortanfang, «Ausweisentzug» auch am Wortende. Verwaltungsrechtliche Sanktionen nur, wo die Botschaft sie nennt (Disziplinarverfahren, Führerausweisentzüge; BBl 1988 II 446). Mehrdeutige Wörter zählen nur in Wendungen: FR «le prévenu» und IT «l'imputato» nur mit Artikel, FR «plainte» nur in «plainte pénale», «porter plainte» und «déposer plainte», FR «dénonciation» nur in «dénonciation pénale» (dazu «dénoncé une infraction»), IT «denuncia» nur in «sporto denuncia», FR «retrait du permis» nur mit «de conduire». Als Ausnahme eingetragen sind und nichts melden die Erlasstitel «Code de procédure pénale», «Procédure pénale applicable aux mineurs» und «Procédure pénale militaire» (dieser nur gross geschrieben, denn «une procédure pénale militaire» ist ein Verfahren und zählt) sowie die Beschwerde an die Aufsichtsbehörde nach SchKG und ZGB («porté plainte à l'autorité de surveillance», «déposé plainte auprès de l'autorité de surveillance»). «vorbestraft» steht in keinem der durchsuchten Erlasse und ist mit Duden belegt.",
                "Searches the terms of lexika/art5c-5.toml (Criminal Procedure Code, Criminal Code, Fixed Penalties Act, Criminal Records Act, Lawyers Act, Road Traffic Act, Federal Personnel Act, Copyright Act, FADP) as whole words; «Strafbefehl» and «Strafregister» also count at the start of a word, «Ausweisentzug» also at the end. Administrative sanctions only where the dispatch names them (disciplinary proceedings, withdrawal of driving licences; BBl 1988 II 446). Ambiguous words only count in phrases: French «le prévenu» and Italian «l'imputato» only with the article, French «plainte» only in «plainte pénale», «porter plainte» and «déposer plainte», French «dénonciation» only in «dénonciation pénale» (plus «dénoncé une infraction»), Italian «denuncia» only in «sporto denuncia», French «retrait du permis» only with «de conduire». Listed as exceptions that report nothing are the titles «Code de procédure pénale», «Procédure pénale applicable aux mineurs» and «Procédure pénale militaire» (the latter only capitalised, since «une procédure pénale militaire» is a proceeding and counts) and the complaint to the supervisory authority under the Debt Enforcement Act and the Civil Code («porté plainte à l'autorité de surveillance», «déposé plainte auprès de l'autorité de surveillance»). «vorbestraft» appears in none of the acts searched and is sourced from the Duden dictionary.",
            ),
            vec![
                fedlex(
                    "Strafprozessordnung, SR 312.0, Art. 2, 5, 32, 111, 112, 115, 118, 195, 210, 217, 220, 324, 352, Fassung 2025-04-01",
                    "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_111",
                ),
                fedlex(
                    "Strafgesetzbuch, SR 311.0, Art. 30, 34, 40 und 66a, Fassung 2026-11-01",
                    "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_34",
                ),
                fedlex(
                    "Ordnungsbussengesetz, SR 314.1, Art. 1, Fassung 2025-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/2017/725/de#art_1",
                ),
                fedlex(
                    "Strafregistergesetz, SR 330, Art. 1 und 3, Fassung 2026-10-01",
                    "https://www.fedlex.admin.ch/eli/cc/2022/600/de#art_1",
                ),
                fedlex(
                    "Anwaltsgesetz, SR 935.61, Art. 16, Fassung 2025-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_16",
                ),
                fedlex(
                    "Strassenverkehrsgesetz, SR 741.01, Art. 16, 16a und 16c, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/1959/679_705_685/de#art_16_c",
                ),
                fedlex(
                    "DNA-Profil-Gesetz, SR 363, Art. 2, Fassung 2023-09-01",
                    "https://www.fedlex.admin.ch/eli/cc/2004/811/it#art_2",
                ),
                fedlex(
                    "Loi sur le personnel de la Confédération, RS 172.220.1, art. 34c al. 1 let. a («dénoncé une infraction», «sporto denuncia»), Fassung 2024-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/2001/123/fr#art_34_c",
                ),
                fedlex(
                    "Loi sur le droit d'auteur, RS 231.1, art. 77i al. 1 («dénonciation pénale»), Fassung 2025-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/1993/1798_1798_1798/fr#art_77_i",
                ),
                fedlex(
                    "Procédure pénale applicable aux mineurs, RS 312.1, titre (Ausnahme), Fassung 2025-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/2010/226/fr",
                ),
                fedlex(
                    "Procédure pénale militaire, RS 322.1, titre (Ausnahme), Fassung 2024-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/1979/1059_1059_1059/fr",
                ),
                fedlex(
                    "Loi sur la poursuite pour dettes et la faillite, RS 281.1, art. 17 al. 1 (Ausnahme «plainte à l'autorité de surveillance»), Fassung 2026-01-01",
                    "https://www.fedlex.admin.ch/eli/cc/11/529_488_529/fr#art_17",
                ),
                fedlex(
                    "Code civil, RS 210, art. 84 al. 3 (Ausnahme «plainte auprès de l'autorité de surveillance»), Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/fr#art_84",
                ),
                fedlex(
                    "Duden online, «vorbestraft» (abgerufen 2026-11-06)",
                    "https://www.duden.de/rechtschreibung/vorbestraft",
                ),
                BOTSCHAFT_1988,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "«Busse» allein fehlt (auch Mehrzahl von «Bus»), ebenso FR «amende» allein (auch «il amende» = ändert ab); erkannt werden «Ordnungsbusse» und «amende d'ordre». IT «multa» zählt.",
                    "«Busse» alone is missing (also the plural of «Bus»), as is French «amende» alone (also «il amende» = amends); «Ordnungsbusse» and «amende d'ordre» are recognised. Italian «multa» counts.",
                ),
                Txt::neu(
                    "«beschuldigt», «verurteilt», «condamné» und «condannato» fehlen (auch ausserhalb des Strafrechts), ebenso «Gefängnis» und «Strafvollzug», FR «plainte» allein (auch Reklamation), «dénonciation» allein (auch Kündigung eines Vertrags), «retrait du permis» ohne «de conduire», «expulsion», «dénoncé» ohne «une infraction» («dénoncé pour vol»), IT «denuncia» allein (auch Steuererklärung), «denuncia penale» (keine Fundstelle gefunden), «denunciato», «indagato», «arresto» (auch «arresto cardiaco») und «espulsione».",
                    "«beschuldigt», «verurteilt», «condamné» and «condannato» are missing (also used outside criminal law), as are «Gefängnis» and «Strafvollzug», French «plainte» alone (also a complaint about a product), «dénonciation» alone (also termination of a contract), «retrait du permis» without «de conduire», «expulsion», «dénoncé» without «une infraction» («dénoncé pour vol»), Italian «denuncia» alone (also a tax return), «denuncia penale» (no source found), «denunciato», «indagato», «arresto» (also «arresto cardiaco») and «espulsione».",
                ),
                Txt::neu(
                    "«Strafanzeige», «Strafantrag», «plainte pénale», «porter plainte» und «querela» melden auch, wer Anzeige erstattet: Auch das ist eine Angabe über ein Strafverfahren. «querela» meldet auch die Fälschungsklage des italienischen Zivilprozesses («querela di falso»). «vorbestraft» zählt auch verneint («nicht vorbestraft»): Auch das ist eine Angabe aus dem Strafregister.",
                    "«Strafanzeige», «Strafantrag», «plainte pénale», «porter plainte» and «querela» also fire for the person filing the complaint: that too is information about criminal proceedings. «querela» also fires on the forgery action of Italian civil procedure («querela di falso»). «vorbestraft» also counts when negated («nicht vorbestraft»): that too is information from the criminal record.",
                ),
                Txt::neu(
                    "«retrait de permis» meldet auch andere Bewilligungen («retrait de permis de construire»); «procédure pénale» meldet auch Fragen zum Verfahrensrecht («Comment fonctionne la procédure pénale ?»). Ausgenommen sind nur die drei Erlasstitel. Am Satzanfang lässt sich der gross geschriebene Titel nicht von einem Verfahren unterscheiden: «Procédure pénale militaire contre le soldat Muster» bleibt still.",
                    "«retrait de permis» also fires on other permits («retrait de permis de construire»); «procédure pénale» also fires on questions about procedural law («Comment fonctionne la procédure pénale ?»). Only the three titles are exempt. At the start of a sentence the capitalised title cannot be told apart from a proceeding: «Procédure pénale militaire contre le soldat Muster» stays silent.",
                ),
                Txt::neu(
                    "Die Ausnahme für die Beschwerde an die Aufsichtsbehörde verdeckt auch Anzeigen, die ein Disziplinarverfahren auslösen können: «a porté plainte auprès de l'autorité de surveillance des avocats» bleibt still.",
                    "The exception for complaints to the supervisory authority also hides reports that may start disciplinary proceedings: «a porté plainte auprès de l'autorité de surveillance des avocats» stays silent.",
                ),
            ],
        ),
        regel_info(
            "art5c-6",
            "art5c-6.toml",
            Txt::neu(
                "Massnahmen der sozialen Hilfe",
                "Social assistance measures",
            ),
            Txt::neu(
                "Angaben über Beistandschaft, Vormundschaft, fürsorgerische Unterbringung, Kindesschutzmassnahmen samt Entziehung der elterlichen Sorge («Sorge entzogen», «retrait de l'autorité parentale», «privazione dell'autorità parentale») und Sozialhilfe, auch im Wortlaut des DSG («Massnahmen der sozialen Hilfe», «assistenza sociale») (Art. 5 Bst. c Ziff. 6 DSG).",
                "Information on deputyship, guardianship, involuntary placement, child protection measures including the withdrawal of parental care («Sorge entzogen», «retrait de l'autorité parentale», «privazione dell'autorità parentale») and social assistance, also in the wording of the FADP («Massnahmen der sozialen Hilfe», «assistenza sociale») (Art. 5(c)(6) FADP).",
            ),
            Txt::neu(
                "Sucht die Begriffe aus lexika/art5c-6.toml (ZGB, AIG, ZUG, DSG) als ganze Wörter; «Beistandschaft» zählt auch am Wortende («Vertretungsbeistandschaft»), «Sozialhilfe» und «Vormundschaft» auch am Wortanfang («Sozialhilfeleistungen», «Vormundschaftsbehörde»). FR «tuteur» und IT «tutore» zählen nur in den Wendungen des ZGB («nomme un tuteur», «nomina un tutore»), ebenso «enfant sous tutelle» und «minorenne sotto tutela». Nur eindeutige Begriffe des Kindes- und Erwachsenenschutzes und der Sozialhilfe.",
                "Searches the terms of lexika/art5c-6.toml (Civil Code, Foreign Nationals and Integration Act, Competence Act, FADP) as whole words; «Beistandschaft» also counts at the end of a word («Vertretungsbeistandschaft»), «Sozialhilfe» and «Vormundschaft» also at the start («Sozialhilfeleistungen», «Vormundschaftsbehörde»). French «tuteur» and Italian «tutore» only count in the phrases of the Civil Code («nomme un tuteur», «nomina un tutore»), as do «enfant sous tutelle» and «minorenne sotto tutela». Only unambiguous terms of child and adult protection and social assistance.",
            ),
            vec![
                fedlex(
                    "Zivilgesetzbuch, SR 210, Art. 311, 314b, 327a, 327b, 390, 393 und 394, Fassung 2026-07-01",
                    "https://www.fedlex.admin.ch/eli/cc/24/233_245_233/de#art_390",
                ),
                fedlex(
                    "Ausländer- und Integrationsgesetz, SR 142.20, Art. 62 Abs. 1 Bst. e, Fassung 2026-11-01",
                    "https://www.fedlex.admin.ch/eli/cc/2007/758/de#art_62",
                ),
                fedlex(
                    "Zuständigkeitsgesetz, SR 851.1, Art. 3 Abs. 1 und Abs. 2 Bst. a, Fassung 2017-04-08",
                    "https://www.fedlex.admin.ch/eli/cc/1978/221_221_221/de#art_3",
                ),
                BOTSCHAFT_1988,
                OK_DSG_5C,
            ],
            vec![
                NENNUNG,
                LISTE,
                Txt::neu(
                    "Abgrenzung zu den Sozialversicherungen: Die Botschaft zählt Sozialversicherungsleistungen bei Krankheit und Unfall zur sozialen Hilfe (BBl 1988 II 446), das Zuständigkeitsgesetz nimmt Ergänzungsleistungen von den Unterstützungen aus (SR 851.1 Art. 3 Abs. 2 Bst. a). Leistungen der Sozialversicherungen (Ergänzungsleistungen, Arbeitslosenentschädigung, Prämienverbilligung) meldet diese Regel darum nicht; Krankheit, Invalidität, Invalidenrente («IV-Rente») und Arbeitsunfähigkeit meldet art5c-2.",
                    "Boundary with social insurance: the dispatch counts social insurance benefits for illness and accident as social assistance (BBl 1988 II 446), the Competence Act excludes supplementary benefits from support payments (SR 851.1 Art. 3(2)(a)). Social insurance benefits (supplementary benefits, unemployment benefit, premium reductions) are therefore not reported by this rule; illness, invalidity, invalidity pensions («IV-Rente») and incapacity for work are reported by art5c-2.",
                ),
                Txt::neu(
                    "«Beistand» allein («Beistand leisten»), «Nothilfe», «aide d'urgence», «soccorso d'emergenza» (auch Erste Hilfe), FR «tuteur» und «tutelle» sowie IT «tutore» und «tutela» allein (auch Doktorvater, Pflanzenstütze, Schutz im Allgemeinen) und IT «curatore» (auch Kurator, Konkursverwalter) fehlen bewusst.",
                    "«Beistand» alone («to stand by someone»), «Nothilfe», «aide d'urgence», «soccorso d'emergenza» (also first aid), French «tuteur» and «tutelle» and Italian «tutore» and «tutela» alone (also thesis supervisor, plant stake, protection in general) and Italian «curatore» (also curator, insolvency administrator) are left out on purpose.",
                ),
                Txt::neu(
                    "Behörden und kantonale Bezeichnungen fehlen: «Sozialamt», «KESB» und «wirtschaftliche Hilfe» bleiben unerkannt. Ebenso fehlen «elterliche Sorge» allein (keine Massnahme) und die Wegnahme des Kindes nach Art. 310 ZGB, für die der Artikeltext keinen eindeutigen Begriff hat.",
                    "Authorities and cantonal terms are missing: «Sozialamt», «KESB» and «wirtschaftliche Hilfe» are not recognised. Also missing are «elterliche Sorge» alone (not a measure) and the removal of the child under Art. 310 Civil Code, for which the article text has no unambiguous term.",
                ),
                Txt::neu(
                    "«Vormundschaft» meldet auch übertragenen Gebrauch («Vormundschaft des Staates über die Gemeinden»).",
                    "«Vormundschaft» also fires in the figurative sense («Vormundschaft des Staates über die Gemeinden»).",
                ),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Alle gemeldeten Stellen als (Regel, Text der Stelle).
    fn stellen(roh: &str) -> Vec<(&'static str, String)> {
        let text = Text::neu(roh);
        erkenne(&text)
            .into_iter()
            .map(|t| (t.regel, text.als_str()[t.start..t.ende].to_string()))
            .collect()
    }

    fn hat(roh: &str, regel: &str, stelle: &str) -> bool {
        stellen(roh).iter().any(|(r, s)| *r == regel && s == stelle)
    }

    fn still(roh: &str) {
        let s = stellen(roh);
        assert!(s.is_empty(), "«{roh}» meldet {s:?}");
    }

    // --- Ziffer 1 ---

    #[test]
    fn ziffer_1_deutsch() {
        let roh = "Herr Muster ist römisch-katholisch und Mitglied einer Gewerkschaft.";
        assert!(hat(roh, "art5c-1", "römisch-katholisch"));
        assert!(hat(roh, "art5c-1", "Gewerkschaft"));
        assert!(hat(
            "Sie ist Gewerkschaftsmitglied.",
            "art5c-1",
            "Gewerkschaft"
        ));
        assert!(hat(
            "Er äussert seine politische Überzeugung offen.",
            "art5c-1",
            "politische Überzeugung"
        ));
    }

    #[test]
    fn ziffer_1_franzoesisch() {
        let roh = "Mme Dupont est catholique et exerce une activité syndicale.";
        assert!(hat(roh, "art5c-1", "catholique"));
        assert!(hat(roh, "art5c-1", "activité syndicale"));
    }

    #[test]
    fn ziffer_1_italienisch() {
        let roh = "Il signor Rossi è musulmano e ha opinioni politiche moderate.";
        assert!(hat(roh, "art5c-1", "musulmano"));
        assert!(hat(roh, "art5c-1", "opinioni politiche"));
    }

    #[test]
    fn ziffer_1_gegenbeispiele() {
        // Zusammensetzung ohne Wortanfang-Erlaubnis, Verb «reformiert», Parteiname.
        still("Die Religionsfreiheit gilt.");
        still("Das Gesetz wurde 2020 reformiert.");
        still("Die SVP hat die Initiative lanciert.");
        // Gemeindeverband und Güterzusammenlegung: Ausnahmen mit Fundstelle.
        still("Les syndicats de communes gèrent les déchets.");
        still("Le syndicat de remaniement a été créé en 2019.");
        // «syndication» (Finanzwesen) ist kein «syndicat»: das ganze Wort zählt.
        still("La syndication du crédit est terminée.");
    }

    #[test]
    fn ziffer_1_syndicat_ohne_ausnahme_bleibt_treffer() {
        // Für die Flurgenossenschaft fand sich keine amtliche Fundstelle, die als Ausnahme
        // taugt: Sie zählt als Treffer (Grenze in RegelInfo::grenzen).
        assert!(hat(
            "Le syndicat d'améliorations foncières se réunit.",
            "art5c-1",
            "syndicat"
        ));
    }

    #[test]
    fn ziffer_1_muslime_und_gewerkschaft() {
        // BFS, Religionen: «Musliminnen und Muslime», «muslimischer».
        assert!(hat("Herr Muster ist Muslim.", "art5c-1", "Muslim"));
        assert!(hat(
            "Die Musliminnen der Gemeinde feiern.",
            "art5c-1",
            "Musliminnen"
        ));
        assert!(hat(
            "Sie stammt aus einer muslimischen Familie.",
            "art5c-1",
            "muslimischen"
        ));
        // IAO-Übereinkommen Nr. 98 Art. 1: «syndicat», «syndicale», «sindacale».
        assert!(hat(
            "M. Dupont est membre d'un syndicat.",
            "art5c-1",
            "syndicat"
        ));
        assert!(hat("Elle est syndicaliste.", "art5c-1", "syndical"));
        assert!(hat(
            "Les délégués syndicaux se réunissent.",
            "art5c-1",
            "syndicaux"
        ));
        assert!(hat(
            "Il signor Rossi è un sindacalista.",
            "art5c-1",
            "sindacali"
        ));
        assert!(hat(
            "Rossi è iscritto a una lista sindacale.",
            "art5c-1",
            "sindacale"
        ));
    }

    // --- Ziffer 2 ---

    #[test]
    fn ziffer_2_deutsch() {
        assert!(hat(
            "Frau Muster ist seit März arbeitsunfähig.",
            "art5c-2",
            "arbeitsunfähig"
        ));
        assert!(hat(
            "Er leidet an einer psychischen Störung.",
            "art5c-2",
            "psychischen Störung"
        ));
    }

    #[test]
    fn ziffer_2_teilwort() {
        // Ende: «Krankheit» in «Lungenkrankheit».
        assert!(hat("Sie hat eine Lungenkrankheit.", "art5c-2", "krankheit"));
        // Anfang: «Arbeitsunfähigkeit» in «Arbeitsunfähigkeitszeugnis».
        assert!(hat(
            "Das Arbeitsunfähigkeitszeugnis liegt vor.",
            "art5c-2",
            "Arbeitsunfähigkeit"
        ));
        // Ohne Erlaubnis kein Treffer mitten im Wort: «Krankheit» braucht rechts eine Grenze.
        still("Die Krankheitskosten sind gestiegen.");
    }

    #[test]
    fn ziffer_2_franzoesisch() {
        let roh = "Mme Dupont est en incapacité de travail depuis sa grossesse.";
        assert!(hat(roh, "art5c-2", "incapacité de travail"));
        assert!(hat(roh, "art5c-2", "grossesse"));
    }

    #[test]
    fn ziffer_2_italienisch() {
        assert!(hat("La signora Rossi è incinta.", "art5c-2", "incinta"));
        assert!(hat(
            "Il signor Rossi soffre di una turba psichica.",
            "art5c-2",
            "turba psichica"
        ));
    }

    #[test]
    fn ziffer_2_gegenbeispiele() {
        still("Die Sicht war durch Nebel behindert.");
        still("Der Vertrag ist invalid.");
        still("L'invalidité du contrat est constatée.");
        still("L'invalidità del contratto è accertata.");
        still("Die Rasse des Hundes ist unbekannt.");
        still("Je parto domani per Roma.");
        // «Sucht» nur gross geschrieben: das Verb «suchen» zählt nicht.
        still("Er sucht eine neue Stelle.");
        // «Krebs» allein ist auch ein Familienname.
        still("Herr Krebs fragt nach der Frist.");
        // «krank» nur als ganzes Wort.
        still("Die Krankenkasse zahlt die Rechnung.");
    }

    #[test]
    fn ziffer_2_krank_in_drei_sprachen() {
        // EpG Art. 33: «krank», «malades», «malate».
        assert!(hat(
            "Mein Arbeitnehmer ist seit Mai krank.",
            "art5c-2",
            "krank"
        ));
        assert!(hat(
            "Mon employé est malade depuis mai.",
            "art5c-2",
            "malade"
        ));
        assert!(hat(
            "Il mio dipendente è malato da maggio.",
            "art5c-2",
            "malato"
        ));
        assert!(hat("La signora Rossi è malata.", "art5c-2", "malata"));
    }

    #[test]
    fn ziffer_2_krankenversicherung_ist_ausnahme() {
        // KVG Art. 1a Abs. 1 und Titel, ArG Art. 17c Abs. 3: Versicherung und Kasse.
        still("L'assurance-maladie rembourse le traitement.");
        still("Les primes d'assurance maladie augmentent.");
        still("La caisse-maladie a refusé la facture.");
        still("La legge sull'assicurazione malattie è cambiata.");
        still("L'assicurazione sociale contro le malattie copre le cure.");
        still("La cassa malati non paga.");
        // Ohne Versicherung bleibt die Krankheit ein Treffer.
        assert!(hat(
            "Elle souffre d'une maladie rare.",
            "art5c-2",
            "maladie"
        ));
        assert!(hat("Soffre di una malattia rara.", "art5c-2", "malattia"));
        assert!(hat(
            "Elle est en arrêt maladie depuis lundi.",
            "art5c-2",
            "maladie"
        ));
    }

    #[test]
    fn ziffer_2_suizid_am_wortanfang() {
        assert!(hat(
            "Er hat einen Suizidversuch hinter sich.",
            "art5c-2",
            "Suizid"
        ));
        assert!(hat("Sie sprach über Suizid.", "art5c-2", "Suizid"));
        assert!(hat(
            "Mehrere Suizide wurden gemeldet.",
            "art5c-2",
            "Suizide"
        ));
    }

    #[test]
    fn ziffer_2_koerperliche_behinderung_in_drei_sprachen() {
        assert!(hat(
            "Er lebt mit einer körperlichen Behinderung.",
            "art5c-2",
            "körperlichen Behinderung"
        ));
        assert!(hat(
            "Sie hat eine körperliche Behinderung.",
            "art5c-2",
            "körperliche Behinderung"
        ));
        assert!(hat(
            "Il vit avec une déficience corporelle.",
            "art5c-2",
            "déficience corporelle"
        ));
        assert!(hat(
            "Vive con menomazioni fisiche.",
            "art5c-2",
            "menomazioni fisiche"
        ));
    }

    #[test]
    fn ziffer_2_diagnose_am_wortende() {
        assert!(hat("Die Krebsdiagnose kam im Mai.", "art5c-2", "diagnose"));
        // Der ICD-Auslöser wirkt auch am Wortende.
        let s = stellen("Verdachtsdiagnose F32.1 laut Bericht.");
        assert!(s.contains(&("art5c-2", "diagnose".into())), "{s:?}");
        assert!(s.contains(&("art5c-2", "F32.1".into())), "{s:?}");
    }

    #[test]
    fn ziffer_2_ethnische_herkunft_in_drei_sprachen() {
        assert!(hat(
            "Daten zur ethnischen Herkunft werden erfasst.",
            "art5c-2",
            "ethnischen Herkunft"
        ));
        assert!(hat(
            "Il précise son origine ethnique.",
            "art5c-2",
            "origine ethnique"
        ));
        assert!(hat(
            "Indica la sua appartenenza etnica.",
            "art5c-2",
            "appartenenza etnica"
        ));
        // FR «ethnie» trägt den französischen Beleg, DE «Ethnie» den deutschen.
        for (roh, quelle) in [
            ("Il indique l'ethnie de la personne.", "SR 142.31"),
            ("Er nennt die Ethnie der Person.", "SR 311.0"),
        ] {
            let text = Text::neu(roh);
            let t = erkenne(&text);
            assert!(
                t.iter().any(|t| t.regel == "art5c-2"
                    && t.beleg
                        .as_ref()
                        .is_some_and(|b| b.quelle.starts_with(quelle))),
                "«{roh}»: {t:?}"
            );
        }
    }

    #[test]
    fn ziffer_2_enceinte_nur_als_zustand() {
        assert!(hat("Mme Dupont est enceinte.", "art5c-2", "est enceinte"));
        assert!(hat(
            "Elle, enceinte de huit mois, a été licenciée.",
            "art5c-2",
            "enceinte de huit mois"
        ));
        assert!(hat(
            "Enceinte de 12 semaines, elle a informé son employeur.",
            "art5c-2",
            "Enceinte de 12 semaines"
        ));
        assert!(hat(
            "Elle, enceinte d’environ trois mois, cherche conseil.",
            "art5c-2",
            "enceinte d’environ trois mois"
        ));
        assert!(hat(
            "Elle, enceinte de vingt-deux semaines, cherche conseil.",
            "art5c-2",
            "enceinte de vingt-deux semaines"
        ));
        assert!(hat("Il l'a mise enceinte.", "art5c-2", "mise enceinte"));
        // Umfassung, Lautsprecher, Schutzhülle: kein Treffer.
        still("Il attend dans l'enceinte du tribunal.");
        still("L'enceinte de la ville date du Moyen Âge.");
        still("Une enceinte acoustique est branchée.");
        still("L'enceinte de confinement du réacteur est intacte.");
    }

    #[test]
    fn ziffer_2_haeufige_gesundheitswoerter() {
        for (roh, stelle) in [
            ("Sie hat eine Erkrankung der Lunge.", "Erkrankung"),
            ("Er hat eine Herzerkrankung.", "erkrankung"),
            (
                "Bei ihr wurde eine Krebserkrankung festgestellt.",
                "Krebserkrankung",
            ),
            ("Er ist an Krebs erkrankt.", "an Krebs erkrankt"),
            ("Sie hat Brustkrebs.", "Brustkrebs"),
            (
                "Sie ist seit zwei Wochen krankgeschrieben.",
                "krankgeschrieben",
            ),
            ("Das Arztzeugnis liegt bei.", "Arztzeugnis"),
            ("Er ist HIV-positiv.", "HIV"),
            ("Sie leidet an Depressionen.", "Depressionen"),
            ("Er hat Diabetes.", "Diabetes"),
            ("Sie leidet an einer Sucht.", "Sucht"),
            ("Er hat ein Burn-out.", "Burn-out"),
            ("Er bezieht eine IV-Rente.", "IV-Rente"),
            ("Ihm steht eine Invalidenrente zu.", "Invalidenrente"),
            (
                "Sein Gesundheitszustand ist schlecht.",
                "Gesundheitszustand",
            ),
            ("Il a un cancer.", "cancer"),
            (
                "Elle souffre d'une maladie oncologique.",
                "maladie oncologique",
            ),
            ("Le certificat médical est joint.", "certificat médical"),
            ("Il est séropositif.", "séropositif"),
            ("Il est atteint du VIH.", "VIH"),
            ("Elle souffre de dépression.", "dépression"),
            ("Il a du diabète.", "diabète"),
            ("Il est diabétique.", "diabétique"),
            ("Il est en burn-out.", "burn-out"),
            ("Son état de santé s'est aggravé.", "état de santé"),
            (
                "Il souffre d'une forme de dépendance.",
                "forme de dépendance",
            ),
            ("Ha un tumore.", "tumore"),
            ("Ha il cancro.", "cancro"),
            ("Soffre di una malattia tumorale.", "malattia tumorale"),
            ("Il certificato medico è allegato.", "certificato medico"),
            ("È sieropositivo.", "sieropositivo"),
            ("Soffre di depressione.", "depressione"),
            ("Ha il diabete.", "diabete"),
            ("È diabetico.", "diabetico"),
            ("Il suo stato di salute è peggiorato.", "stato di salute"),
            ("Soffre di una forma di dipendenza.", "forma di dipendenza"),
        ] {
            assert!(hat(roh, "art5c-2", stelle), "«{roh}»: {:?}", stellen(roh));
        }
    }

    #[test]
    fn ziffer_2_beleg_in_eigener_sprache() {
        // Gleiche Schreibung in zwei Sprachen: Gross geschrieben ist das deutsche Nomen,
        // klein geschrieben der französische Eintrag.
        for (roh, datei_quelle) in [
            ("Er hat ein Burn-out.", "Duden"),
            ("Il est en burn-out.", "Larousse"),
        ] {
            let text = Text::neu(roh);
            let t = erkenne(&text);
            assert!(
                t.iter().any(|t| t
                    .beleg
                    .as_ref()
                    .is_some_and(|b| b.quelle.starts_with(datei_quelle))),
                "«{roh}»: {t:?}"
            );
        }
    }

    #[test]
    fn ziffer_2_kinderkrankheiten_ist_grenze() {
        // Bildlich gemeint, aber «Krankheit» am Wortende zählt (RegelInfo::grenzen).
        assert!(hat(
            "Die Software hat noch Kinderkrankheiten.",
            "art5c-2",
            "krankheiten"
        ));
    }

    // --- Ziffer 2: ICD-10-Kodes ---

    #[test]
    fn icd_nach_diagnose_mit_aufzaehlung() {
        let s = stellen("Diagnose: F32.1, F41.0 und Z73.0 gemäss Bericht.");
        assert!(s.contains(&("art5c-2", "Diagnose".into())));
        for kode in ["F32.1", "F41.0", "Z73.0"] {
            assert!(
                s.contains(&("art5c-2", kode.into())),
                "{kode} fehlt in {s:?}"
            );
        }
    }

    #[test]
    fn icd_nur_ausloeser_meldet_sich_nicht() {
        let s = stellen("ICD-10: M54.16");
        assert_eq!(s, vec![("art5c-2", "M54.16".to_string())]);
    }

    #[test]
    fn icd_in_franzoesisch_und_italienisch() {
        assert!(hat("Diagnostic : F32.1", "art5c-2", "F32.1"));
        assert!(hat("Diagnosi: F32.1", "art5c-2", "F32.1"));
    }

    #[test]
    fn icd_kode_braucht_ausloeser() {
        // Ohne Auslöser, Auslöser im anderen Satz, Auslöser zu weit weg, Kode im Wort.
        still("Bitte das Formular F32 bis Montag einreichen.");
        let s = stellen("Die Diagnose folgt. Raum F32 ist frei.");
        assert!(!s.iter().any(|(_, t)| t == "F32"), "{s:?}");
        let weit = "Diagnose laut dem langen Bericht der Klinik vom letzten Jahr: F32.1";
        assert!(!stellen(weit).iter().any(|(_, t)| t == "F32.1"));
        let s = stellen("Diagnose: Vitamin-B12-Mangel");
        assert!(!s.iter().any(|(_, t)| t == "B12"), "{s:?}");
    }

    #[test]
    fn icd_beleg_ist_der_ausloeser() {
        let text = Text::neu("ICD-10: F32.1");
        let t = erkenne(&text);
        assert_eq!(t.len(), 1);
        let beleg = t[0].beleg.as_ref().expect("Beleg");
        assert_eq!(beleg.datei, "art5c-2.toml");
        assert_eq!(beleg.begriff, "ICD");
        assert!(t[0].zusatz.is_some());
    }

    #[test]
    fn icd_ueber_weichen_umbruch_und_formatzeichen() {
        // Methode: Ein einzelner Zeilenumbruch mitten im Absatz trennt den Satz nicht,
        // Formatzeichen stören weder im Auslöser noch zwischen Auslöser und Kode.
        for roh in [
            "Diagnose\nF32.1, kommt morgen.",
            "Diagnose\r\nF32.1, kommt morgen.",
            "diagnostic\nF32.1, arrive demain.",
            "diagnosi\nF32.1, arriva domani.",
            "Dia\u{AD}gnose F32.1, kommt morgen.",
            "Diagnose \u{2060}F32.1, kommt morgen.",
        ] {
            assert!(hat(roh, "art5c-2", "F32.1"), "«{roh}»: {:?}", stellen(roh));
        }
        assert!(hat(
            "Dia\u{AD}gnose F32.1, kommt morgen.",
            "art5c-2",
            "Dia\u{AD}gnose"
        ));
        // Grenzen: Satzzeichen am Zeilenende, Leerzeile, Listenpunkt, Formatzeichen im Kode.
        for roh in [
            "Diagnose:\nF32.1",
            "Diagnose.\nF32.1",
            "Diagnose\n\nF32.1",
            "Diagnose\n- F32.1",
            "Diagnose F3\u{AD}2.1",
        ] {
            let s = stellen(roh);
            assert!(hat(roh, "art5c-2", "Diagnose"), "«{roh}»: {s:?}");
            assert!(
                !s.iter().any(|(_, t)| t.starts_with("F3")),
                "«{roh}»: {s:?}"
            );
        }
        let r = &info()[1];
        assert_eq!(r.id, "art5c-2");
        assert!(r.methode.de.contains("(«Diagnose⏎F32.1»)"));
        assert!(r.methode.en.contains("(«Diagnose⏎F32.1»)"));
        assert!(
            r.grenzen
                .iter()
                .any(|g| g.de.contains("In «Diagnose:⏎F32.1» zählt nur «Diagnose»"))
        );
    }

    // --- Ziffer 3 ---

    #[test]
    fn ziffer_3_drei_sprachen() {
        assert!(hat(
            "Von Herrn Muster liegt ein DNA-Profil vor.",
            "art5c-3",
            "DNA-Profil"
        ));
        // Typografischer Apostroph im Text, gerader in der Wortliste.
        assert!(hat(
            "Le profil d’ADN de M. Dupont est enregistré.",
            "art5c-3",
            "profil d’ADN"
        ));
        assert!(hat(
            "I dati genetici del signor Rossi.",
            "art5c-3",
            "dati genetici"
        ));
    }

    #[test]
    fn ziffer_3_ausnahme_gesetzesname() {
        still("Was regelt das DNA-Profil-Gesetz?");
    }

    // --- Ziffer 4 ---

    #[test]
    fn ziffer_4_drei_sprachen() {
        assert!(hat(
            "Die Fingerabdrücke von Frau Muster wurden gespeichert.",
            "art5c-4",
            "Fingerabdrücke"
        ));
        assert!(hat(
            "Les empreintes digitales de M. Dupont.",
            "art5c-4",
            "empreintes digitales"
        ));
        assert!(hat(
            "L'immagine del viso della signora Rossi.",
            "art5c-4",
            "immagine del viso"
        ));
    }

    #[test]
    fn ziffer_4_gewoehnliches_foto_zaehlt_nicht() {
        still("Das Foto von Frau Muster hängt im Büro.");
    }

    // --- Ziffer 5 ---

    #[test]
    fn ziffer_5_deutsch() {
        let roh = "Gegen Herrn Muster läuft ein Strafverfahren; er sitzt in Untersuchungshaft.";
        assert!(hat(roh, "art5c-5", "Strafverfahren"));
        assert!(hat(roh, "art5c-5", "Untersuchungshaft"));
        // Anfang und Ende.
        assert!(hat(
            "Der Strafbefehlsentwurf liegt vor.",
            "art5c-5",
            "Strafbefehl"
        ));
        assert!(hat(
            "Ihm droht ein Führerausweisentzug.",
            "art5c-5",
            "ausweisentzug"
        ));
    }

    #[test]
    fn ziffer_5_franzoesisch() {
        let roh = "Le prévenu a reçu une ordonnance pénale.";
        assert!(hat(roh, "art5c-5", "Le prévenu"));
        assert!(hat(roh, "art5c-5", "ordonnance pénale"));
    }

    #[test]
    fn ziffer_5_italienisch() {
        let roh = "L’imputato è in carcerazione preventiva.";
        assert!(hat(roh, "art5c-5", "L’imputato"));
        assert!(hat(roh, "art5c-5", "carcerazione preventiva"));
    }

    #[test]
    fn ziffer_5_gegenbeispiele() {
        still("Die Busse fahren um acht Uhr.");
        still("Je l'ai prévenu hier soir.");
        still("Il costo imputato al progetto è alto.");
        still("Er verurteilt das Vorgehen scharf.");
        still("Le Parlement amende le projet.");
        // Gesetzesname, Vertragskündigung und Steuererklärung: kein Treffer.
        still("Le Code de procédure pénale règle la procédure.");
        still("La dénonciation du contrat de bail est valable.");
        still("La denuncia dei redditi scade a marzo.");
    }

    #[test]
    fn ziffer_5_strafantrag_und_vorstrafen() {
        for (roh, stelle) in [
            ("Herr Muster hat Vorstrafen.", "Vorstrafen"),
            (
                "Daten über strafrechtliche Verfolgungen sind heikel.",
                "strafrechtliche Verfolgungen",
            ),
            ("Elle a déposé une plainte pénale.", "plainte pénale"),
            ("Il participe à la procédure pénale.", "procédure pénale"),
            (
                "Il n'a pas d'antécédents judiciaires.",
                "antécédents judiciaires",
            ),
            (
                "Il a fait l'objet d'un retrait de permis.",
                "retrait de permis",
            ),
            (
                "Elle a dénoncé une infraction à son employeur.",
                "dénoncé une infraction",
            ),
            (
                "Il veut déposer une dénonciation pénale.",
                "dénonciation pénale",
            ),
            ("Ha presentato querela.", "querela"),
            ("Non ha precedenti penali.", "precedenti penali"),
            ("Ha sporto denuncia contro il vicino.", "sporto denuncia"),
        ] {
            assert!(hat(roh, "art5c-5", stelle), "«{roh}»: {:?}", stellen(roh));
        }
    }

    // --- Ziffer 6 ---

    #[test]
    fn ziffer_6_drei_sprachen() {
        assert!(hat(
            "Für Frau Muster besteht eine Vertretungsbeistandschaft.",
            "art5c-6",
            "beistandschaft"
        ));
        assert!(hat(
            "Er bezieht Sozialhilfeleistungen.",
            "art5c-6",
            "Sozialhilfe"
        ));
        let fr = "M. Dupont est sous curatelle et dépend de l'aide sociale.";
        assert!(hat(fr, "art5c-6", "curatelle"));
        assert!(hat(fr, "art5c-6", "aide sociale"));
        assert!(hat(
            "La signora Rossi è sotto curatela.",
            "art5c-6",
            "curatela"
        ));
    }

    #[test]
    fn ziffer_6_gegenbeispiele() {
        still("Er leistete ihr in der Not Beistand.");
        still("Der curatore der Ausstellung ist eingetroffen.");
        // «tuteur»/«tutore» allein: Doktorvater, Pflanzenstütze.
        still("Le tuteur de thèse relit le chapitre.");
        still("Il tutore sostiene la pianta di pomodoro.");
    }

    #[test]
    fn ziffer_6_vormund_und_soziale_hilfe() {
        for (roh, stelle) in [
            ("Das Kind hat einen Vormund.", "Vormund"),
            ("Das Kind steht unter Vormundschaft.", "Vormundschaft"),
            (
                "Er ist auf Massnahmen der sozialen Hilfe angewiesen.",
                "sozialen Hilfe",
            ),
            ("L'autorité nomme un tuteur à l'enfant.", "nomme un tuteur"),
            (
                "L'enfant sous tutelle vit chez sa tante.",
                "enfant sous tutelle",
            ),
            ("Riceve l'assistenza sociale.", "assistenza sociale"),
            (
                "Riceve prestazioni assistenziali dal Comune.",
                "prestazioni assistenziali",
            ),
            (
                "L'autorità nomina un tutore al minorenne.",
                "nomina un tutore",
            ),
            (
                "Il minorenne sotto tutela vive con la zia.",
                "minorenne sotto tutela",
            ),
        ] {
            assert!(hat(roh, "art5c-6", stelle), "«{roh}»: {:?}", stellen(roh));
        }
    }

    // --- Weitere Wörter und Wendungen ---

    #[test]
    fn ziffer_1_jude_juedin_konfession_ebreo() {
        // Duden und Treccani: Alltagswörter ohne Fundstelle in einem Erlass.
        for (roh, stelle) in [
            ("Frau Muster ist Jüdin.", "Jüdin"),
            ("Die Jüdinnen der Gemeinde feiern.", "Jüdinnen"),
            ("Herr Muster ist Jude.", "Jude"),
            ("Die Juden der Gemeinde feiern.", "Juden"),
            ("Ihre Konfession ist unbekannt.", "Konfession"),
            ("La signora Rossi è ebrea.", "ebrea"),
            ("Il signor Rossi è ebreo.", "ebreo"),
            ("Gli ebrei della città festeggiano.", "ebrei"),
        ] {
            assert!(hat(roh, "art5c-1", stelle), "«{roh}»: {:?}", stellen(roh));
        }
        // «Jude» nur gross geschrieben; «konfessionslos» ist kein ganzes Wort «Konfession».
        still("hey jude");
        still("Er ist konfessionslos.");
        // Hinter dem Etikett bleibt die Abkürzung unerkannt (RegelInfo::grenzen).
        assert_eq!(
            stellen("Konfession: röm.-kath."),
            vec![("art5c-1", "Konfession".to_string())]
        );
    }

    #[test]
    fn ziffer_1_beleg_in_eigener_sprache() {
        for (roh, url) in [
            ("M. Dupont est membre d'un syndicat.", "/fr#art_1"),
            ("Il signor Rossi è un sindacalista.", "/it#art_1"),
            ("Frau Muster ist Jüdin.", "duden.de"),
            ("La signora Rossi è ebrea.", "treccani.it"),
            ("Herr Muster ist Muslim.", "statistik.admin.ch/de/"),
        ] {
            let text = Text::neu(roh);
            let t = erkenne(&text);
            assert!(
                t.iter().any(|t| t.regel == "art5c-1"
                    && t.beleg.as_ref().is_some_and(|b| b.url.contains(url))),
                "«{roh}»: {t:?}"
            );
        }
    }

    #[test]
    fn ziffer_2_kranken_mit_ergaenzungsstrich_ist_ausnahme() {
        // BV Art. 117 Abs. 1: «Kranken- und die Unfallversicherung».
        still("Die Kranken- und Unfallversicherung zahlt.");
        still("Die Kranken-Taggeldversicherung zahlt.");
        // Ohne Ergänzungsstrich bleibt «kranken» ein Treffer.
        assert!(hat("Er pflegt seinen kranken Vater.", "art5c-2", "kranken"));
    }

    #[test]
    fn ziffer_2_diagnose_am_wortende_loest_icd_aus() {
        // Haupt-, Neben-, Verdachts- und Austrittsdiagnose.
        for (roh, kode) in [
            ("Hauptdiagnose: F32.1", "F32.1"),
            ("Nebendiagnosen: E11.9 und I10", "E11.9"),
            ("Nebendiagnosen: E11.9 und I10", "I10"),
            ("Verdachtsdiagnose: F41.0", "F41.0"),
            ("Austrittsdiagnose: J18.9", "J18.9"),
        ] {
            let s = stellen(roh);
            assert!(s.contains(&("art5c-2", kode.to_string())), "«{roh}»: {s:?}");
        }
        assert!(hat("Hauptdiagnose: F32.1", "art5c-2", "diagnose"));
        assert!(hat("Nebendiagnosen: E11.9 und I10", "art5c-2", "diagnosen"));
    }

    #[test]
    fn ziffer_2_suizid_in_drei_sprachen() {
        for (roh, stelle) in [
            ("Herr Muster ist suizidal.", "suizid"),
            ("Il a des idées suicidaires.", "suicidaires"),
            ("Elle a un comportement suicidaire.", "suicidaire"),
            ("On parle de sa suicidalité.", "suicidalité"),
            ("Si parla della sua suicidalità.", "suicidalità"),
            ("Ha tendenze suicide.", "tendenze suicide"),
            ("Ha tentato il suicidio.", "suicidio"),
        ] {
            assert!(hat(roh, "art5c-2", stelle), "«{roh}»: {:?}", stellen(roh));
        }
        // Die italienische Wendung trägt den italienischen Beleg, nicht FR «suicide».
        let text = Text::neu("Ha tendenze suicide.");
        let t = erkenne(&text);
        assert!(
            t.iter().any(|t| t
                .beleg
                .as_ref()
                .is_some_and(|b| b.url.contains("/it/piano-dazione"))),
            "{t:?}"
        );
    }

    #[test]
    fn ziffer_2_origine_etnica() {
        assert!(hat(
            "Indica la sua origine etnica.",
            "art5c-2",
            "origine etnica"
        ));
    }

    #[test]
    fn ziffer_4_images_faciales() {
        // FF 2017 6641: Gegenstück zu «Gesichtsbild» und «immagine del viso».
        assert!(hat(
            "Les images faciales de M. Dupont sont enregistrées.",
            "art5c-4",
            "images faciales"
        ));
        assert!(hat(
            "Une image faciale est stockée.",
            "art5c-4",
            "image faciale"
        ));
        still("La photographie de M. Dupont est jointe.");
    }

    #[test]
    fn ziffer_5_neue_wendungen() {
        for (roh, stelle) in [
            ("Der Bewerber ist vorbestraft.", "vorbestraft"),
            ("Ein vorbestrafter Bewerber meldet sich.", "vorbestrafter"),
            ("Er ist nicht vorbestraft.", "vorbestraft"),
            ("Elle a porté plainte contre son voisin.", "porté plainte"),
            ("Il veut porter plainte.", "porter plainte"),
            ("Il a déposé plainte hier.", "déposé plainte"),
            ("Il veut déposer plainte pénale.", "déposer plainte"),
            (
                "Il a subi un retrait du permis de conduire.",
                "retrait du permis de conduire",
            ),
            ("Ha presentato una querela.", "querela"),
        ] {
            assert!(hat(roh, "art5c-5", stelle), "«{roh}»: {:?}", stellen(roh));
        }
        // «plainte» allein ist auch eine Reklamation; «dépose une plainte» ist keine Wendung.
        still("La plainte du client concerne la livraison.");
        still("Le client dépose une plainte au magasin.");
        // Andere Bewilligungen: «retrait du permis» nur mit «de conduire».
        still("Le retrait du permis de construire est contesté.");
    }

    #[test]
    fn ziffer_6_entziehung_der_elterlichen_sorge_in_drei_sprachen() {
        // Art. 311 und 312 ZGB: dieselbe Massnahme in DE, FR und IT.
        for (roh, stelle) in [
            (
                "Der Mutter wurde die elterliche Sorge entzogen.",
                "Sorge entzogen",
            ),
            (
                "Die Kindesschutzbehörde entzieht die elterliche Sorge.",
                "entzieht die elterliche Sorge",
            ),
            (
                "Le retrait de l'autorité parentale a été prononcé.",
                "retrait de l'autorité parentale",
            ),
            (
                "La privazione dell'autorità parentale è stata pronunciata.",
                "privazione dell'autorità parentale",
            ),
        ] {
            assert!(hat(roh, "art5c-6", stelle), "«{roh}»: {:?}", stellen(roh));
        }
        // Die elterliche Sorge selbst ist keine Massnahme.
        still("Die Eltern haben die gemeinsame elterliche Sorge.");
    }

    #[test]
    fn ziffer_6_vormundschaft_am_wortanfang() {
        assert!(hat(
            "Die Vormundschaftsbehörde entscheidet.",
            "art5c-6",
            "Vormundschaft"
        ));
        // Die Sozialversicherung meldet art5c-2, nicht art5c-6.
        let s = stellen("Er bezieht eine IV-Rente.");
        assert!(s.contains(&("art5c-2", "IV-Rente".to_string())), "{s:?}");
        assert!(!s.iter().any(|(r, _)| *r == "art5c-6"), "{s:?}");
        still("Er bezieht Ergänzungsleistungen.");
        still("Die Arbeitslosenentschädigung ist ausbezahlt.");
        still("Die Prämienverbilligung wurde gewährt.");
    }

    // --- Wortlaut des DSG ---

    /// Art. 5 Bst. c DSG (SR 235.1, Fassung 2025-07-07), Ziffer für Ziffer in DE, FR und
    /// IT: Der eigene Wortlaut des Gesetzes löst die zugehörige Regel aus.
    #[test]
    fn dsg_wortlaut_jeder_ziffer_in_drei_sprachen() {
        let faelle: [(&str, [&str; 3]); 6] = [
            (
                "art5c-1",
                [
                    "Daten über religiöse, weltanschauliche, politische oder gewerkschaftliche Ansichten oder Tätigkeiten",
                    "les données sur les opinions ou les activités religieuses, philosophiques, politiques ou syndicales",
                    "i dati concernenti le opinioni o attività religiose, filosofiche, politiche o sindacali",
                ],
            ),
            (
                "art5c-2",
                [
                    "Daten über die Gesundheit, die Intimsphäre oder die Zugehörigkeit zu einer Rasse oder Ethnie",
                    "les données sur la santé, la sphère intime ou l’origine raciale ou ethnique",
                    "i dati concernenti la salute, la sfera intima o l’appartenenza a una razza o a un’etnia",
                ],
            ),
            (
                "art5c-3",
                [
                    "genetische Daten",
                    "les données génétiques",
                    "i dati genetici",
                ],
            ),
            (
                "art5c-4",
                [
                    "biometrische Daten, die eine natürliche Person eindeutig identifizieren",
                    "les données biométriques identifiant une personne physique de manière univoque",
                    "i dati biometrici che identificano in modo univoco una persona fisica",
                ],
            ),
            (
                "art5c-5",
                [
                    "Daten über verwaltungs- und strafrechtliche Verfolgungen oder Sanktionen",
                    "les données sur des poursuites ou sanctions pénales et administratives",
                    "i dati concernenti perseguimenti e sanzioni amministrativi e penali",
                ],
            ),
            (
                "art5c-6",
                [
                    "Daten über Massnahmen der sozialen Hilfe",
                    "les données sur des mesures d’aide sociale",
                    "i dati concernenti le misure d’assistenza sociale",
                ],
            ),
        ];
        for (regel, wortlaute) in faelle {
            for roh in wortlaute {
                let s = stellen(roh);
                assert!(
                    s.iter().any(|(r, _)| *r == regel),
                    "{regel}: «{roh}» meldet {s:?}"
                );
            }
        }
    }

    // --- Treffer und Beschreibung ---

    #[test]
    fn treffer_tragen_ziffer_und_beleg() {
        let text = Text::neu("Sie ist schwanger und bezieht Sozialhilfe.");
        let t = erkenne(&text);
        assert_eq!(t.len(), 2, "{t:?}");
        for (treffer, ziffer) in t.iter().zip([2u8, 6]) {
            assert_eq!(treffer.art, Art::BesondersSchuetzenswert(ziffer));
            assert_eq!(treffer.regel, format!("art5c-{ziffer}"));
            let beleg = treffer.beleg.as_ref().expect("Beleg");
            assert_eq!(beleg.datei, format!("art5c-{ziffer}.toml"));
            assert!(beleg.url.starts_with("https://"));
        }
    }

    #[test]
    fn leerer_text_meldet_nichts() {
        still("");
        still("Bitte fasse den Text zusammen.");
    }

    /// Ein vertipptes Merkmal machte aus einer Ausnahme still einen Treffer.
    #[test]
    fn jedes_merkmal_ist_bekannt() {
        for (_, _, datei) in REGELN {
            for e in &lexikon::lexikon(datei).eintraege {
                if let Some(m) = e.merkmal.as_deref() {
                    assert!(
                        MERKMALE.contains(&m),
                        "{datei}: {}: merkmal «{m}»",
                        e.begriff
                    );
                }
            }
        }
    }

    /// «Jude» und FR «cancer» sind auch Namen. In einem erkannten Namen zählen sie nicht,
    /// sonst schon.
    #[test]
    fn name_mit_jude_oder_cancer_zaehlt_nicht() {
        for roh in [
            "Herr Jude Muster bewirbt sich.",
            "Schreibe an Jude Bellingham.",
            "M. Paul Cancer demande un rendez-vous.",
        ] {
            let s = stellen(roh);
            assert!(
                !s.iter().any(|(r, _)| *r == "art5c-1" || *r == "art5c-2"),
                "«{roh}»: {s:?}"
            );
        }
        for (roh, regel, stelle) in [
            ("Herr Muster ist Jude.", "art5c-1", "Jude"),
            ("Die Juden in Zürich", "art5c-1", "Juden"),
            ("M. Muster a un cancer.", "art5c-2", "cancer"),
            ("Paul Muster souffre d'un cancer.", "art5c-2", "cancer"),
        ] {
            assert!(hat(roh, regel, stelle), "«{roh}»: {:?}", stellen(roh));
        }
    }

    #[test]
    fn info_ist_vollstaendig() {
        let infos = info();
        assert_eq!(infos.len(), 6);
        for (i, r) in infos.iter().enumerate() {
            let n = i + 1;
            assert_eq!(r.id, format!("art5c-{n}"));
            assert_eq!(r.klasse, Klasse::C);
            assert_eq!(r.lexika, vec![format!("art5c-{n}.toml")]);
            assert!(r.hinweis.is_none());
            assert_eq!(r.grundlage.len(), 3);
            assert_eq!(r.grundlage[0].zitat, normen::DSG_5_C.zitat);
            assert!(r.quellen.len() >= 3);
            assert!(r.grenzen.len() >= 3);
            for t in [&r.titel, &r.erkennt, &r.methode] {
                assert!(!t.de.is_empty() && !t.en.is_empty());
            }
            for q in &r.quellen {
                assert!(q.url.starts_with("https://"), "{}", q.url);
                assert!(matches!(q.stand, Stand::Geprueft(_)));
            }
            assert!(matches!(r.guete, Guete::NichtGemessen));
            // Jede Regel erkennt etwas aus ihrer eigenen Wortliste.
            let lex = lexikon::lexikon(r.lexika[0]);
            assert!(!lex.eintraege.is_empty());
        }
    }

    /// Steht `«satz»` wörtlich in der deutschen Beschreibung (erkennt, methode, grenzen)?
    fn steht_in_info(regel: &str, satz: &str) -> bool {
        let r = info()
            .into_iter()
            .find(|r| r.id == regel)
            .expect("Regel vorhanden");
        let zitat = format!("«{satz}»");
        [&r.erkennt, &r.methode]
            .into_iter()
            .chain(r.grenzen.iter())
            .any(|t| t.de.contains(zitat.as_str()))
    }

    /// Jedes Beispiel in «» aus `rules`/`explain` tut, was die Beschreibung sagt:
    /// `Some(stelle)` = die Regel meldet genau diese Stelle, `None` = die Regel meldet nichts.
    #[test]
    fn beispiele_aus_der_beschreibung_stimmen() {
        let faelle: &[(&str, &str, Option<&str>)] = &[
            // Gemeinsame Grenze (NENNUNG).
            (
                "art5c-2",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                Some("Krankheit"),
            ),
            // art5c-1, erkennt und methode.
            ("art5c-1", "katholisch", Some("katholisch")),
            ("art5c-1", "Muslim", Some("Muslim")),
            ("art5c-1", "Jüdin", Some("Jüdin")),
            ("art5c-1", "musulman", Some("musulman")),
            ("art5c-1", "ebreo", Some("ebreo")),
            ("art5c-1", "Gewerkschaftsmitglied", Some("Gewerkschaft")),
            ("art5c-1", "syndicaliste", Some("syndical")),
            ("art5c-1", "sindacalista", Some("sindacali")),
            ("art5c-1", "Jude", Some("Jude")),
            ("art5c-1", "Konfession", Some("Konfession")),
            ("art5c-1", "syndicats de communes", None),
            ("art5c-1", "syndicat de remaniement", None),
            ("art5c-1", "Jude Bellingham", None),
            ("art5c-1", "sindacato", Some("sindacato")),
            ("art5c-1", "sindacato di legittimità", None),
            ("art5c-1", "sindacato azionario", None),
            ("art5c-1", "ordinanza sindacale", None),
            // art5c-1, grenzen.
            ("art5c-1", "Mitglied der SVP", None),
            ("art5c-1", "reformiert", None),
            ("art5c-1", "konfessionslos", None),
            ("art5c-1", "confession", None),
            ("art5c-1", "l'operato è stato sindacato", Some("sindacato")),
            ("art5c-1", "collegio sindacale", Some("sindacale")),
            ("art5c-1", "patto di sindacato", Some("sindacato")),
            ("art5c-1", "der Jude Nathan", None),
            ("art5c-1", "Herr Muster ist Jude", Some("Jude")),
            ("art5c-1", "Konfession: röm.-kath.", Some("Konfession")),
            (
                "art5c-1",
                "syndicat d'améliorations foncières",
                Some("syndicat"),
            ),
            ("art5c-1", "syndicat intercommunal", Some("syndicat")),
            ("art5c-1", "Religion", Some("Religion")),
            ("art5c-1", "jüdisch", Some("jüdisch")),
            ("art5c-1", "jüdisches Museum", Some("jüdisches")),
            ("art5c-1", "Religion und Staat", Some("Religion")),
            // art5c-2, erkennt.
            ("art5c-2", "krank", Some("krank")),
            ("art5c-2", "krankgeschrieben", Some("krankgeschrieben")),
            ("art5c-2", "Krebserkrankung", Some("Krebserkrankung")),
            ("art5c-2", "HIV", Some("HIV")),
            ("art5c-2", "Diabetes", Some("Diabetes")),
            ("art5c-2", "Diabetiker", Some("Diabetiker")),
            ("art5c-2", "Depression", Some("Depression")),
            ("art5c-2", "Sucht", Some("Sucht")),
            ("art5c-2", "addiction", Some("addiction")),
            ("art5c-2", "Burn-out", Some("Burn-out")),
            ("art5c-2", "rente AI", Some("rente AI")),
            ("art5c-2", "rendita AI", Some("rendita AI")),
            // art5c-2, methode.
            ("art5c-2", "Lungenkrankheit", Some("krankheit")),
            ("art5c-2", "Herzerkrankung", Some("erkrankung")),
            ("art5c-2", "Verdachtsdiagnose", Some("diagnose")),
            ("art5c-2", "Suizidversuch", Some("Suizid")),
            ("art5c-2", "suizidal", Some("suizid")),
            ("art5c-2", "sucht", None),
            ("art5c-2", "Ethnie", Some("Ethnie")),
            ("art5c-2", "Kranken- und Unfallversicherung", None),
            ("art5c-2", "assurance-maladie", None),
            ("art5c-2", "caisse-maladie", None),
            ("art5c-2", "assicurazione malattie", None),
            ("art5c-2", "cassa malati", None),
            ("art5c-2", "rendita ai superstiti", None),
            ("art5c-2", "Rendita AI", None),
            ("art5c-2", "M. Paul Cancer", None),
            ("art5c-2", "assureur-maladie", None),
            ("art5c-2", "assicurazione malattia", None),
            ("art5c-2", "assicuratore malattia", None),
            ("art5c-2", "assicuratore-malattie", None),
            ("art5c-2", "forme de dépendance économique", None),
            ("art5c-2", "forma di dipendenza economica", None),
            ("art5c-2", "est enceinte", Some("est enceinte")),
            ("art5c-2", "mise enceinte", Some("mise enceinte")),
            ("art5c-2", "femmes enceintes", Some("femmes enceintes")),
            (
                "art5c-2",
                "enceinte de huit mois",
                Some("enceinte de huit mois"),
            ),
            // art5c-2, grenzen.
            ("art5c-2", "Rasse", None),
            ("art5c-2", "race", None),
            ("art5c-2", "razza", None),
            ("art5c-2", "appartenance ethnique", None),
            ("art5c-2", "ethnische Herkunft", Some("ethnische Herkunft")),
            ("art5c-2", "ethnie", Some("ethnie")),
            ("art5c-2", "origine ethnique", Some("origine ethnique")),
            (
                "art5c-2",
                "appartenance raciale",
                Some("appartenance raciale"),
            ),
            ("art5c-2", "origine raciale", Some("origine raciale")),
            ("art5c-2", "etnia", Some("etnia")),
            (
                "art5c-2",
                "appartenenza etnica",
                Some("appartenenza etnica"),
            ),
            ("art5c-2", "origine etnica", Some("origine etnica")),
            ("art5c-2", "invalidité", None),
            ("art5c-2", "invalidità", None),
            ("art5c-2", "invalid", None),
            ("art5c-2", "Behinderte", None),
            ("art5c-2", "behindern", None),
            ("art5c-2", "Behinderung", None),
            ("art5c-2", "enceinte", None),
            ("art5c-2", "parto", None),
            ("art5c-2", "Krebs", None),
            ("art5c-2", "Unfall", None),
            ("art5c-2", "Mutterschaft", None),
            ("art5c-2", "arrêt de travail", None),
            ("art5c-2", "Abhängigkeit", None),
            ("art5c-2", "dépendance", None),
            ("art5c-2", "dipendenza", None),
            ("art5c-2", "depressiv", None),
            ("art5c-2", "alkoholkrank", None),
            ("art5c-2", "Herzinfarkt", None),
            ("art5c-2", "PTBS", None),
            ("art5c-2", "Das ist doch krank.", Some("krank")),
            (
                "art5c-2",
                "Die Firmen kranken an hohen Kosten.",
                Some("kranken"),
            ),
            (
                "art5c-2",
                "Die Software hat noch Kinderkrankheiten.",
                Some("krankheiten"),
            ),
            ("art5c-2", "Suizid", Some("Suizid")),
            ("art5c-2", "Selbstmord", Some("Selbstmord")),
            ("art5c-2", "Suizidprävention", Some("Suizid")),
            ("art5c-2", "dépression", Some("dépression")),
            ("art5c-2", "depressione", Some("depressione")),
            (
                "art5c-2",
                "Die Depression der Dreissigerjahre",
                Some("Depression"),
            ),
            ("art5c-2", "Sucht er eine Stelle?", Some("Sucht")),
            (
                "art5c-2",
                "indemnités journalières de l'assurance-maladie",
                None,
            ),
            ("art5c-2", "Krankentaggeld", None),
            ("art5c-2", "une employée enceinte", None),
            ("art5c-2", "une collaboratrice enceinte", None),
            ("art5c-2", "une travailleuse enceinte", None),
            ("art5c-2", "elle est tombée enceinte", None),
            ("art5c-2", "femme enceinte", Some("femme enceinte")),
            ("art5c-5", "querela di falso", Some("querela")),
            (
                "art5c-2",
                "assurance perte de gain maladie",
                Some("maladie"),
            ),
            ("art5c-2", "état de santé financière", Some("état de santé")),
            (
                "art5c-2",
                "stato di salute dei conti",
                Some("stato di salute"),
            ),
            (
                "art5c-2",
                "Gesundheitszustand der Pensionskasse",
                Some("Gesundheitszustand"),
            ),
            ("art5c-2", "le cancer de la bureaucratie", Some("cancer")),
            ("art5c-2", "l'économie est malade", Some("malade")),
            ("art5c-2", "è malato di calcio", Some("malato")),
            ("art5c-2", "addiction au chocolat", Some("addiction")),
            ("art5c-2", "Sucht nach Anerkennung", Some("Sucht")),
            ("art5c-2", "nato sotto il segno del Cancro", Some("Cancro")),
            ("art5c-2", "burnout", Some("burnout")),
            ("art5c-2", "Fehlerdiagnose", Some("diagnose")),
            ("art5c-2", "Diagnose: Mangel an Vitamin B12.", Some("B12")),
            (
                "art5c-2",
                "sexuelle Orientierung",
                Some("sexuelle Orientierung"),
            ),
            // art5c-4.
            ("art5c-4", "images faciales", Some("images faciales")),
            ("art5c-4", "photographie", None),
            // art5c-5, erkennt und methode.
            ("art5c-5", "plainte pénale", Some("plainte pénale")),
            ("art5c-5", "porter plainte", Some("porter plainte")),
            ("art5c-5", "querela", Some("querela")),
            ("art5c-5", "vorbestraft", Some("vorbestraft")),
            ("art5c-5", "le prévenu", Some("le prévenu")),
            ("art5c-5", "l'imputato", Some("l'imputato")),
            ("art5c-5", "déposer plainte", Some("déposer plainte")),
            (
                "art5c-5",
                "dénonciation pénale",
                Some("dénonciation pénale"),
            ),
            (
                "art5c-5",
                "dénoncé une infraction",
                Some("dénoncé une infraction"),
            ),
            ("art5c-5", "sporto denuncia", Some("sporto denuncia")),
            ("art5c-5", "retrait du permis", None),
            ("art5c-5", "Code de procédure pénale", None),
            ("art5c-5", "enquête pénale", Some("enquête pénale")),
            ("art5c-5", "inchiesta penale", Some("inchiesta penale")),
            ("art5c-5", "Procédure pénale applicable aux mineurs", None),
            ("art5c-5", "Procédure pénale militaire", None),
            (
                "art5c-5",
                "une procédure pénale militaire",
                Some("procédure pénale"),
            ),
            (
                "art5c-5",
                "porté plainte à l'autorité de surveillance",
                None,
            ),
            (
                "art5c-5",
                "déposé plainte auprès de l'autorité de surveillance",
                None,
            ),
            // art5c-5, grenzen.
            ("art5c-5", "Busse", None),
            ("art5c-5", "amende", None),
            ("art5c-5", "il amende", None),
            ("art5c-5", "Ordnungsbusse", Some("Ordnungsbusse")),
            ("art5c-5", "amende d'ordre", Some("amende d'ordre")),
            ("art5c-5", "multa", Some("multa")),
            ("art5c-5", "beschuldigt", None),
            ("art5c-5", "verurteilt", None),
            ("art5c-5", "condamné", None),
            ("art5c-5", "condannato", None),
            ("art5c-5", "Gefängnis", None),
            ("art5c-5", "Strafvollzug", None),
            ("art5c-5", "plainte", None),
            ("art5c-5", "dénonciation", None),
            ("art5c-5", "expulsion", None),
            ("art5c-5", "dénoncé pour vol", None),
            ("art5c-5", "denuncia", None),
            ("art5c-5", "denuncia penale", None),
            ("art5c-5", "denunciato", None),
            ("art5c-5", "indagato", None),
            ("art5c-5", "arresto", None),
            ("art5c-5", "arresto cardiaco", None),
            ("art5c-5", "espulsione", None),
            ("art5c-5", "Strafanzeige", Some("Strafanzeige")),
            ("art5c-5", "Strafantrag", Some("Strafantrag")),
            ("art5c-5", "nicht vorbestraft", Some("vorbestraft")),
            ("art5c-5", "retrait de permis", Some("retrait de permis")),
            (
                "art5c-5",
                "retrait de permis de construire",
                Some("retrait de permis"),
            ),
            ("art5c-5", "procédure pénale", Some("procédure pénale")),
            (
                "art5c-5",
                "Comment fonctionne la procédure pénale ?",
                Some("procédure pénale"),
            ),
            (
                "art5c-5",
                "Procédure pénale militaire contre le soldat Muster",
                None,
            ),
            (
                "art5c-5",
                "a porté plainte auprès de l'autorité de surveillance des avocats",
                None,
            ),
            // art5c-6.
            (
                "art5c-6",
                "Massnahmen der sozialen Hilfe",
                Some("sozialen Hilfe"),
            ),
            ("art5c-6", "assistenza sociale", Some("assistenza sociale")),
            (
                "art5c-6",
                "Vertretungsbeistandschaft",
                Some("beistandschaft"),
            ),
            ("art5c-6", "Sozialhilfeleistungen", Some("Sozialhilfe")),
            ("art5c-6", "Vormundschaftsbehörde", Some("Vormundschaft")),
            (
                "art5c-6",
                "Vormundschaft des Staates über die Gemeinden",
                Some("Vormundschaft"),
            ),
            ("art5c-6", "tuteur", None),
            ("art5c-6", "tutore", None),
            ("art5c-6", "nomme un tuteur", Some("nomme un tuteur")),
            ("art5c-6", "nomina un tutore", Some("nomina un tutore")),
            (
                "art5c-6",
                "enfant sous tutelle",
                Some("enfant sous tutelle"),
            ),
            (
                "art5c-6",
                "minorenne sotto tutela",
                Some("minorenne sotto tutela"),
            ),
            ("art5c-6", "IV-Rente", None),
            ("art5c-6", "Beistand", None),
            ("art5c-6", "Beistand leisten", None),
            ("art5c-6", "Nothilfe", None),
            ("art5c-6", "aide d'urgence", None),
            ("art5c-6", "soccorso d'emergenza", None),
            ("art5c-6", "tutelle", None),
            ("art5c-6", "tutela", None),
            ("art5c-6", "curatore", None),
            ("art5c-6", "Sozialamt", None),
            ("art5c-6", "KESB", None),
            ("art5c-6", "wirtschaftliche Hilfe", None),
            ("art5c-6", "elterliche Sorge", None),
            ("art5c-6", "Sorge entzogen", Some("Sorge entzogen")),
            (
                "art5c-6",
                "retrait de l'autorité parentale",
                Some("retrait de l'autorité parentale"),
            ),
            (
                "art5c-6",
                "privazione dell'autorità parentale",
                Some("privazione dell'autorità parentale"),
            ),
            // Einzelwörter aus Erkennt, Methode und Grenzen, die gemeinsame Grenze NENNUNG je
            // Regel und Regelnamen.
            ("art5c-1", "Gewerkschaft", Some("Gewerkschaft")),
            ("art5c-1", "syndical", Some("syndical")),
            ("art5c-1", "sindacali", Some("sindacali")),
            ("art5c-1", "syndicat", Some("syndicat")),
            ("art5c-1", "Religionen", Some("Religionen")),
            ("art5c-1", "protestant(e)", None),
            ("art5c-1", "sindacare", None),
            ("art5c-1", "name", None),
            (
                "art5c-1",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                None,
            ),
            ("art5c-2", "Krankheit", Some("Krankheit")),
            ("art5c-2", "Erkrankung", Some("Erkrankung")),
            ("art5c-2", "Diagnose", Some("Diagnose")),
            ("art5c-2", "Arbeitsunfähigkeit", Some("Arbeitsunfähigkeit")),
            ("art5c-2", "Invalidität", Some("Invalidität")),
            ("art5c-2", "Schwangerschaft", Some("Schwangerschaft")),
            ("art5c-2", "Selbstmord", Some("Selbstmord")),
            ("art5c-2", "VIH", Some("VIH")),
            ("art5c-2", "ICD", None),
            ("art5c-2", "diagnostic", Some("diagnostic")),
            ("art5c-2", "diagnosi", Some("diagnosi")),
            ("art5c-2", "maladie", Some("maladie")),
            ("art5c-2", "cancer", Some("cancer")),
            ("art5c-2", "name", None),
            (
                "art5c-4",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                None,
            ),
            ("art5c-4", "Fingerabdruck", Some("Fingerabdruck")),
            ("art5c-4", "digitaler Fingerabdruck", Some("Fingerabdruck")),
            ("art5c-4", "Iris", None),
            ("art5c-3", "DNA-Profil-Gesetz", None),
            (
                "art5c-3",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                None,
            ),
            (
                "art5c-3",
                "Loi sur les profils d'ADN",
                Some("profils d'ADN"),
            ),
            (
                "art5c-3",
                "Legge sui profili del DNA",
                Some("profili del DNA"),
            ),
            ("art5c-3", "profils d'ADN", Some("profils d'ADN")),
            ("art5c-3", "profili del DNA", Some("profili del DNA")),
            ("art5c-5", "Strafbefehl", Some("Strafbefehl")),
            ("art5c-5", "Strafregister", Some("Strafregister")),
            ("art5c-5", "Ausweisentzug", Some("Ausweisentzug")),
            ("art5c-5", "de conduire", None),
            ("art5c-5", "Bus", None),
            ("art5c-5", "dénoncé", None),
            ("art5c-5", "une infraction", None),
            (
                "art5c-5",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                None,
            ),
            ("art5c-6", "Beistandschaft", Some("Beistandschaft")),
            ("art5c-6", "Sozialhilfe", Some("Sozialhilfe")),
            ("art5c-6", "Vormundschaft", Some("Vormundschaft")),
            (
                "art5c-6",
                "Hans Muster fragt nach Krankheit im Arbeitsrecht",
                None,
            ),
        ];
        for (regel, satz, stelle) in faelle {
            assert!(
                steht_in_info(regel, satz),
                "{regel}: «{satz}» steht nicht in der Beschreibung"
            );
            let s = stellen(satz);
            match stelle {
                Some(st) => assert!(
                    s.iter().any(|(r, t)| r == regel && t == st),
                    "{regel}: «{satz}» meldet {s:?}, erwartet «{st}»"
                ),
                None => assert!(
                    !s.iter().any(|(r, _)| r == regel),
                    "{regel}: «{satz}» meldet {s:?}, erwartet nichts"
                ),
            }
        }
    }

    /// Gleich geschriebene Einträge mehrerer Sprachen tragen den Beleg in der Sprache des
    /// Textes; «burnout» hat keinen italienischen Eintrag und trägt den französischen
    /// (RegelInfo::grenzen von art5c-2).
    #[test]
    fn hiv_und_burnout_beleg_nach_sprache_des_textes() {
        for (roh, url) in [
            ("È positivo all'HIV.", "/it#art_12_d"),
            (
                "Il paziente Mario Esempio è positivo all'HIV.",
                "/it#art_12_d",
            ),
            ("Er ist HIV-positiv und hat eine Frage.", "/de#art_5"),
            ("HIV", "/de#art_5"),
            ("Soffre di burnout.", "larousse.fr"),
        ] {
            let text = Text::neu(roh);
            let t = erkenne(&text);
            let belege: Vec<&str> = t
                .iter()
                .filter(|t| t.regel == "art5c-2")
                .filter_map(|t| t.beleg.as_ref().map(|b| b.url.as_str()))
                .collect();
            assert_eq!(belege.len(), 1, "«{roh}»: {belege:?}");
            assert!(belege[0].contains(url), "«{roh}»: {belege:?}");
        }
        let grenzen = &info()
            .into_iter()
            .find(|r| r.id == "art5c-2")
            .expect("art5c-2")
            .grenzen;
        assert!(grenzen.iter().any(|g| g.de.contains("«burnout»")
            && g.de.contains("Sprache des Textes")
            && g.en.contains("language of the text")));
    }
}
