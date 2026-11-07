//! Darstellung (SPEC 3): der Bericht als Text für das Terminal und als JSON,
//! `promptscan rules` und `promptscan explain`.
//!
//! Aufbau der Textausgabe, Breitenmodell der Zeilenansicht und JSON-Schema stehen in
//! `docs/AUSGABE.md`. Die Tests am Ende dieses Moduls gleichen die dort dokumentierten
//! JSON-Felder mit der tatsächlichen Ausgabe ab, in beide Richtungen.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::LazyLock;

use anstyle::{Ansi256Color, AnsiColor, Style};
use regex::Regex;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::Bericht;
use crate::fund::{Fund, Grund, Rolle, RolleQuelle, Stufe, Ziel};
use crate::katalog::{self, Guete, Klasse, Quelle, RegelInfo, Stand, Txt};
use crate::lexikon::{self, Lexikon, Register, Sprache};
use crate::normen::{self, Norm};
use crate::regeln::firma;
use crate::stufen::{self, Lage};
use crate::text::Text;

/// Version des JSON-Schemas aller Ausgaben (docs/AUSGABE.md). Ein neues Feld lässt sie
/// unverändert; ein entferntes, umbenanntes oder umgedeutetes Feld oder ein neuer Wert
/// eines Feldes erhöht sie. Version 2: neue Rollen «offen» und «financial-institution»,
/// neues Feld `optionen.rolle_quelle`.
pub const SCHEMA_VERSION: u32 = 2;

/// Bis zu so vielen Zeilen zeigt die Zeilenansicht den ganzen Text. Darüber zeigt sie
/// nur die Zeilen mit Funden und für jede Lücke eine Zeile «…».
pub const ZEILEN_GANZ: usize = 40;

/// Abstand der Tabulatorstopps in der Zeilenansicht, gezählt ab dem Zeilenanfang.
pub const TAB_BREITE: usize = 8;

/// Breite, auf die `explain` und `rules` Fliesstext umbrechen.
const ZEILENBREITE: usize = 80;

/// Ersatz für Steuerzeichen in der Zeilenansicht und in der Liste.
const ERSATZ: char = '\u{FFFD}';

/// Die Stufen in der Reihenfolge der Ausgabe: die schwerste zuerst.
const STUFEN: [Stufe; 4] = [Stufe::Rot, Stufe::Orange, Stufe::Gelb, Stufe::Hinweis];

/// Wie ausgegeben wird.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Darstellung {
    pub englisch: bool,
    /// Ob Farben tatsächlich erscheinen. Ohne Farbe werden Stellen mit `^^^` markiert.
    pub farbe: bool,
    /// Das Terminal kennt nur 16 Farben: ORANGE erscheint dann gelb, das Wort ORANGE
    /// bleibt (SPEC 6). Entscheidet `main.rs` aus `TERM` und `COLORTERM`.
    pub nur_16_farben: bool,
    /// Spalten des Terminals; die Ausgabe wird darauf umgebrochen (Untergrenze 60).
    /// Setzt `main.rs`; ist die Breite unbekannt oder die Ausgabe umgeleitet: 80.
    pub breite: usize,
    /// Nur ASCII ausgeben (`--ascii`): Rahmen, Anführungszeichen und Umlaute umschrieben.
    pub ascii: bool,
}

/// Überschriften in `rules` und `explain`: fett, wenn Farbe erscheint.
fn fett(text: &str, d: &Darstellung) -> String {
    if d.farbe {
        let s = Style::new().bold();
        format!("{s}{text}{s:#}")
    } else {
        text.to_string()
    }
}

/// Die Stufe zu einem Namen für `explain`: gelb, orange, rot, hinweis oder englisch
/// yellow, red, note; ohne Rücksicht auf Gross- und Kleinschreibung.
pub fn stufe_aus_name(name: &str) -> Option<Stufe> {
    match name.trim().to_lowercase().as_str() {
        "gelb" | "yellow" => Some(Stufe::Gelb),
        "orange" => Some(Stufe::Orange),
        "rot" | "red" => Some(Stufe::Rot),
        "hinweis" | "note" => Some(Stufe::Hinweis),
        _ => None,
    }
}

fn zitate(liste: &[&'static Norm]) -> String {
    liste.iter().map(|n| n.zitat).collect::<Vec<_>>().join(", ")
}

/// Zitate für `rules`, knapp: Ziffern desselben Erlasses, die aufeinander folgen, stehen
/// zusammen («Ziff. 2.2.4, 2.2.6 TAV E.164»).
fn zitate_kurz(liste: &[&'static Norm]) -> String {
    fn schliessen(teile: &mut Vec<String>, ziffern: &mut Option<(&str, Vec<&str>)>) {
        if let Some((erlass, nummern)) = ziffern.take() {
            teile.push(format!("Ziff. {} {erlass}", nummern.join(", ")));
        }
    }
    let mut teile: Vec<String> = Vec::new();
    let mut ziffern: Option<(&str, Vec<&str>)> = None;
    for n in liste {
        match n
            .zitat
            .strip_prefix("Ziff. ")
            .and_then(|rest| rest.split_once(' '))
        {
            Some((nummer, erlass)) => {
                if ziffern.as_ref().is_some_and(|(e, _)| *e != erlass) {
                    schliessen(&mut teile, &mut ziffern);
                }
                ziffern
                    .get_or_insert_with(|| (erlass, Vec::new()))
                    .1
                    .push(nummer);
            }
            None => {
                schliessen(&mut teile, &mut ziffern);
                teile.push(n.zitat.to_string());
            }
        }
    }
    schliessen(&mut teile, &mut ziffern);
    teile.join(", ")
}

/// Die Rollen, die bei offenem Beruf zur Auswahl stehen: «lawyer | psychologist | …».
fn offen_angebot() -> String {
    stufen::OFFEN_ANGEBOT
        .iter()
        .map(|r| r.name())
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Pflicht und Normen eines ROT-Fundes. Bei offenem Beruf steht hinter jeder Norm, für
/// wen sie gilt; die Zeile wird dann nach der ersten Norm umbrochen.
fn pflicht_zeilen(rolle: Rolle, liste: &[&'static Norm], englisch: bool) -> Vec<String> {
    let name = stufen::geheimnis_name(rolle, englisch);
    if rolle != Rolle::Offen {
        return vec![format!("{name}: {}", zitate(liste))];
    }
    let teile: Vec<String> = liste
        .iter()
        .map(|n| format!("{} ({})", n.zitat, stufen::offen_beruf(n, englisch)))
        .collect();
    match teile.split_first() {
        Some((erste, rest)) if !rest.is_empty() => {
            vec![format!("{name}: {erste},"), rest.join(", ")]
        }
        _ => vec![format!("{name}: {}", teile.join(", "))],
    }
}

/// Erklärungszeilen eines Fundes (ohne Einzug). Leere Zeilen gibt es nicht.
pub fn grund_zeilen(f: &Fund, englisch: bool) -> Vec<String> {
    let zeilen = match &f.grund {
        Grund::Personendaten => vec![if englisch {
            format!("Personal data, {}", normen::DSG_5_A.zitat)
        } else {
            format!("Personendaten, {}", normen::DSG_5_A.zitat)
        }],
        // Art. 30 DSG steht im 5. Kapitel: Datenbearbeitung durch private Personen.
        Grund::BesondersSchuetzenswert { ziffer } => vec![
            format!(
                "{}, {};",
                normen::art5c_kategorie(*ziffer, englisch),
                normen::art5c_zitat(*ziffer)
            ),
            if englisch {
                format!(
                    "disclosure to third parties by private persons: {}",
                    normen::DSG_30_2_C.zitat
                )
            } else {
                format!(
                    "Bekanntgabe an Dritte durch Private: {}",
                    normen::DSG_30_2_C.zitat
                )
            },
        ],
        Grund::Geheimnis {
            rolle,
            aus_text,
            ziffer,
            natuerliche_person,
        } => {
            let mut v = pflicht_zeilen(
                *rolle,
                &stufen::normen_fuer_fund(*rolle, *natuerliche_person),
                englisch,
            );
            if let Some(z) = ziffer {
                v.push(if englisch {
                    format!("also sensitive: {}", normen::art5c_zitat(*z))
                } else {
                    format!(
                        "zugleich besonders schützenswert: {}",
                        normen::art5c_zitat(*z)
                    )
                });
            }
            if let Some(stelle) = aus_text {
                // Der Ausschnitt kommt aus dem Prompt: bereinigt wie die Stelle der Liste.
                let stelle = stelle_einzeilig(stelle);
                v.push(match (*rolle == Rolle::Offen, englisch) {
                    (true, true) => {
                        format!("profession open after «{stelle}»; set it with --role")
                    }
                    (true, false) => format!("Beruf offen nach «{stelle}»; mit --role festlegen"),
                    (false, true) => format!("role inferred from «{stelle}»; set it with --role"),
                    (false, false) => {
                        format!("Rolle erschlossen aus «{stelle}»; mit --role festlegen")
                    }
                });
            }
            v
        }
        Grund::OhneIdentifikator => vec![if englisch {
            "Reference to a person, but no direct identifier in the prompt".to_string()
        } else {
            "Personenbezug, aber kein direkter Identifikator im Prompt".to_string()
        }],
        // Offener Beruf: die möglichen Normen und die Rollen zur Auswahl.
        Grund::Geheimniskontext {
            rolle: Rolle::Offen,
        } => {
            let normen = zitate(&stufen::normen_fuer(Rolle::Offen));
            if englisch {
                vec![
                    format!("Secrecy context, profession open: {normen};"),
                    format!("set it with --role {}", offen_angebot()),
                ]
            } else {
                vec![
                    format!("Geheimnis-Kontext, Beruf offen: {normen};"),
                    format!("festlegen mit --role {}", offen_angebot()),
                ]
            }
        }
        Grund::Geheimniskontext { rolle } => vec![if englisch {
            format!(
                "Secrecy context ({}): {}; confirm with --role {} or --role none",
                rolle.name(),
                zitate(&stufen::normen_fuer(*rolle)),
                rolle.name()
            )
        } else {
            format!(
                "Geheimnis-Kontext ({}): {}; mit --role {} bestätigen oder --role none",
                rolle.name(),
                zitate(&stufen::normen_fuer(*rolle)),
                rolle.name()
            )
        }],
        Grund::Lokal { sonst } => vec![if englisch {
            format!(
                "Local model: no disclosure ({}); otherwise {}",
                normen::DSG_5_E.zitat,
                sonst.wort(true)
            )
        } else {
            format!(
                "Modell lokal: keine Bekanntgabe ({}); sonst {}",
                normen::DSG_5_E.zitat,
                sonst.wort(false)
            )
        }],
        Grund::Regelhinweis => katalog::finde(f.regel)
            .and_then(|r| r.hinweis)
            .map(|h| h.in_sprache(englisch).to_string())
            .into_iter()
            .collect(),
    };
    zeilen.into_iter().filter(|z| !z.is_empty()).collect()
}

/// Die Normzitate eines Fundes, in der Reihenfolge der Erklärungszeilen.
pub fn normzitate(f: &Fund) -> Vec<String> {
    let rollennormen = |rolle: Rolle| {
        stufen::normen_fuer(rolle)
            .iter()
            .map(|n| n.zitat.to_string())
            .collect::<Vec<_>>()
    };
    match &f.grund {
        Grund::Personendaten => vec![normen::DSG_5_A.zitat.to_string()],
        Grund::BesondersSchuetzenswert { ziffer } => vec![
            normen::art5c_zitat(*ziffer),
            normen::DSG_30_2_C.zitat.to_string(),
        ],
        Grund::Geheimnis {
            rolle,
            ziffer,
            natuerliche_person,
            ..
        } => {
            let mut v: Vec<String> = stufen::normen_fuer_fund(*rolle, *natuerliche_person)
                .iter()
                .map(|n| n.zitat.to_string())
                .collect();
            v.extend(ziffer.map(normen::art5c_zitat));
            v
        }
        Grund::OhneIdentifikator => Vec::new(),
        Grund::Geheimniskontext { rolle } => rollennormen(*rolle),
        Grund::Lokal { .. } => vec![normen::DSG_5_E.zitat.to_string()],
        Grund::Regelhinweis => katalog::finde(f.regel)
            .map(|r| r.grundlage.iter().map(|n| n.zitat.to_string()).collect())
            .unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------------------
// Bericht als Text
// ---------------------------------------------------------------------------------------

/// Breite der Beschriftungsspalte (Stufe, «Ergebnis», «Klären» …): Text ab Spalte 11.
const SP_TEXT: usize = 10;

/// Werte einer Gruppe stehen ab Spalte 25, davor die Art (Name, Adresse …).
const SP_WERT: usize = 24;

/// Untergrenze der Breite: Schmalere Terminals bekommen 60 Spalten.
pub const MIN_BREITE: usize = 60;

/// Breite, wenn [`Darstellung::breite`] nicht gesetzt ist (0).
const VORGABE_BREITE: usize = 80;

/// Bis zu so vielen Zeilen zeigt die Zeilenansicht keine Zeilennummern, und die Gruppen
/// nennen keine Zeilen.
pub const ZEILEN_OHNE_NUMMER: usize = 10;

/// Die Kopfzeile beginnt eine zweite Zeile, wenn sie länger würde.
const KOPF_BREITE: usize = 70;

/// Länger zeigt die Kopfzeile ein auslösendes Wort nicht; danach «…».
const KOPF_WORT: usize = 20;

/// So viele Regeln nennt die Zeile «Mehr» höchstens, nach dem Befehl für die Stufe.
const MEHR_REGELN: usize = 3;

// Wortlaute des Berichts. Sie stehen beisammen, damit sie sich Zeile für Zeile ersetzen
// lassen.

const LOKAL_GEPRUEFT: Txt = Txt::neu("Lokal geprüft", "Checked locally");
const ROLLE: Txt = Txt::neu("Rolle", "Role");
const ZIEL: Txt = Txt::neu("Ziel", "Target");
const NICHT_ANGEGEBEN: Txt = Txt::neu("nicht angegeben", "not given");
const AUS: Txt = Txt::neu("aus", "from");
const KONFIGURATION: Txt = Txt::neu("Konfiguration", "configuration");
const CLOUD: Txt = Txt::neu("Cloud", "cloud");
const LOKALES_MODELL: Txt = Txt::neu("lokales Modell", "local model");

const PERSONENDATEN: Txt = Txt::neu("Personendaten", "Personal data");
const BESONDERS: Txt = Txt::neu(
    "Besonders schützenswerte Personendaten",
    "Sensitive personal data",
);
const DURCH_PRIVATE: Txt = Txt::neu(
    "Bekanntgabe an Dritte durch Private:",
    "Private persons disclosing to third parties:",
);
const NICHT_BEURTEILT: Txt = Txt::neu(
    "Rechtsfolge nicht beurteilt",
    "Legal consequence not assessed",
);
const BERUF_OFFEN: Txt = Txt::neu("Beruf offen", "Profession open");
const ZUGLEICH: Txt = Txt::neu(
    "zugleich besonders schützenswert",
    "also sensitive personal data",
);
const KEIN_OFFENBAREN: Txt = Txt::neu("kein Offenbaren", "no disclosure of secrets");
const SONST: Txt = Txt::neu("sonst", "else");
const ERSCHLOSSEN: Txt = Txt::neu("Rolle daraus erschlossen:", "Role inferred from it:");
const OHNE_IDENTIFIKATOR: Txt = Txt::neu(
    "Personenbezug, aber kein direkter Identifikator im Prompt:",
    "Reference to a person, but no direct identifier in the prompt:",
);
/// Zwischenzeile vor den Kategorien ohne Identifikator; darunter «Ziff. N» wie bei ORANGE,
/// damit die Ziffer nie ohne ihr Zitat steht.
const KATEGORIE_NACH: Txt = Txt::neu("Kategorie nach", "Category under");
/// Der Hinweis der Regel fundstellen in der Gruppe; die Handlung steht unter «Klären».
const FUNDSTELLEN_GRUND: Txt = Txt::neu(
    "Sprachmodelle erfinden Urteile.",
    "Language models invent judgments.",
);

const TITEL_ROT: Txt = Txt::neu(
    "Strafbewehrte Geheimhaltungspflicht",
    "Duty of confidentiality backed by criminal law",
);
const TITEL_HINWEIS: Txt = Txt::neu(
    "Hängt von Tatsachen ab, die promptscan nicht kennt",
    "Depends on facts promptscan does not know",
);
const TITEL_LOKAL: Txt = Txt::neu(
    "Läuft das Modell lokal: keine Bekanntgabe",
    "If the model runs locally: no disclosure",
);
/// Ergebnis, wenn in der Cloud ein Fund ROT wäre.
const TITEL_LOKAL_ROT: Txt = Txt::neu(
    "Läuft das Modell lokal: keine Bekanntgabe, kein Offenbaren",
    "If the model runs locally: no disclosure of data or secrets",
);
const IN_DER_CLOUD: Txt = Txt::neu("in der Cloud", "in the cloud");
const KEINE_FUNDE: Txt = Txt::neu(
    "Keine Funde in den geprüften Mustern",
    "No findings in the patterns checked",
);
/// Zwei Absätze unter «Beachten».
const BLINDER_FLECK: Txt = Txt::neu(
    "Ob der Zusammenhang eine Person erkennbar macht und ob der Text Geschäftsgeheimnisse enthält, prüft promptscan nicht.",
    "promptscan does not check whether the context makes a person identifiable or whether the text contains trade secrets.",
);
const WAS_GEPRUEFT: Txt = Txt::neu(
    "Was geprüft wird: promptscan rules",
    "What is checked: promptscan rules",
);

const ERGEBNIS: Txt = Txt::neu("Ergebnis", "Result");
/// Die Meldungen zur Eingabe stehen ohne Etikett direkt unter «Ergebnis»: Sie beginnen
/// selbst mit «Eingabe …».
const OHNE_ETIKETT: Txt = Txt::neu("", "");
const KLAEREN: Txt = Txt::neu("Klären", "Clarify");
const BEACHTEN: Txt = Txt::neu("Beachten", "Caveat");
const MEHR: Txt = Txt::neu("Mehr", "More");

const KLAEREN_GELB: Txt = Txt::neu(
    "Braucht die KI diese Angaben (Art. 6 Abs. 2 DSG)?",
    "Does the AI need these details (Art. 6 Abs. 2 DSG)?",
);
const KLAEREN_ORANGE: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Nutzt der Anbieter sie nur in Ihrem Auftrag oder auch für eigene Zwecke, etwa zum Training (Art. 9 DSG)?",
    "Does the AI need these details? Does the provider use them only on your behalf or also for its own purposes, e.g. training (Art. 9 DSG)?",
);
/// Art. 321 StGB, Rollen nach Ziff. 1 Abs. 1.
const KLAEREN_321: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Liegt eine Einwilligung oder schriftliche Bewilligung vor (Art. 321 Ziff. 2 StGB)? Ist der Anbieter Ihre Hilfsperson (Art. 321 Ziff. 1 StGB)?",
    "Does the AI need these details? If so: is there consent or written authorisation (Art. 321 Ziff. 2 StGB)? Is the provider your assistant (Art. 321 Ziff. 1 StGB)?",
);
/// Studierende: ohne die Frage nach der Hilfsperson, die Art. 321 Ziff. 1
/// Abs. 2 StGB nicht kennt.
const KLAEREN_321_STUDIERENDE: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Liegt eine Einwilligung oder schriftliche Bewilligung vor (Art. 321 Ziff. 2 StGB)?",
    "Does the AI need these details? If so: is there consent or written authorisation (Art. 321 Ziff. 2 StGB)?",
);
const KLAEREN_320: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Liegt die schriftliche Einwilligung der vorgesetzten Behörde vor (Art. 320 Ziff. 2 StGB)? Ist der Anbieter Hilfsperson der Behörde (Art. 320 Ziff. 1 StGB)?",
    "Does the AI need these details? If so: has the superior authority given written consent (Art. 320 Ziff. 2 StGB)? Is the provider an auxiliary to the authority (Art. 320 Ziff. 1 StGB)?",
);
const KLAEREN_BANK: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Hat die Kundin oder der Kunde eingewilligt? Ist der Anbieter Beauftragter der Bank (Art. 47 Abs. 1 lit. a BankG)?",
    "Does the AI need these details? If so: has the client consented? Is the provider an agent of the bank (Art. 47 Abs. 1 lit. a BankG)?",
);
const KLAEREN_FINIG: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Hat die Kundin oder der Kunde eingewilligt? Ist der Anbieter Beauftragter des Finanzinstituts (Art. 69 Abs. 1 lit. a FINIG)?",
    "Does the AI need these details? If so: has the client consented? Is the provider an agent of the financial institution (Art. 69 Abs. 1 lit. a FINIG)?",
);
const KLAEREN_62: Txt = Txt::neu(
    "Braucht die KI diese Angaben? Wenn ja: Hat die betroffene Person eingewilligt? Ist der Anbieter für Sie tätig (Art. 62 Abs. 2 DSG)?",
    "Does the AI need these details? If so: has the data subject consented? Does the provider carry on an activity for you (Art. 62 Abs. 2 DSG)?",
);
const BRAUCHT_DIE_KI: Txt = Txt::neu(
    "Braucht die KI diese Angaben?",
    "Does the AI need these details?",
);
const WELCHE_PFLICHT: Txt = Txt::neu(
    "Welche Pflicht gilt? Festlegen mit",
    "Which duty applies? Set it with",
);
const KLAEREN_LOKAL: Txt = Txt::neu(
    "Läuft das Modell wirklich hier, ohne Synchronisierung in eine Cloud? Braucht es diese Angaben? Art. 6 Abs. 2 DSG gilt auch lokal.",
    "Does the model really run here, without syncing to a cloud? Does it need these details? Art. 6 Abs. 2 DSG applies locally too.",
);
const AUSLAND: Txt = Txt::neu(
    "Gehen die Daten ins Ausland? Dann gelten Art. 16 f. DSG, Anhang 1 DSV.",
    "Does the data go abroad? Then Art. 16 f. DSG and Anhang 1 DSV apply.",
);
const AUSLAND_BEHOERDE: Txt = Txt::neu(
    "Gehen die Daten ins Ausland? Für Bundesorgane: Art. 16 f. DSG.",
    "Does the data go abroad? For federal bodies: Art. 16 f. DSG.",
);
const KLAEREN_FUNDSTELLEN: Txt = Txt::neu(
    "Jede Fundstelle in der Antwort an der Quelle prüfen.",
    "Check every citation in the answer against its source.",
);
const KLAEREN_KONTEXT: Txt = Txt::neu(
    "Stimmt die erschlossene Rolle? Sonst mit --role festlegen oder mit --role none ausschliessen (Rollen: promptscan explain rot).",
    "Is the inferred role right? Otherwise set it with --role or rule it out with --role none (roles: promptscan explain red).",
);
const KLAEREN_PERSONENBEZUG: Txt = Txt::neu(
    "Lässt der Zusammenhang erkennen, wer gemeint ist (Art. 5 lit. a DSG)?",
    "Does the context show who is meant (Art. 5 lit. a DSG)?",
);
const KLAEREN_VERMERK: Txt = Txt::neu(
    "Gilt der Vermerk für Sie? Das ISG gilt für Behörden und Organisationen des Bundes (Art. 2 ISG), für Kantone bei klassifizierten Informationen des Bundes (Art. 3 ISG); sonst zählt, was vereinbart ist.",
    "Does the marking apply to you? The ISG applies to federal authorities and organisations (Art. 2 ISG), to cantons for classified federal information (Art. 3 ISG); otherwise, what was agreed counts.",
);
const KLAEREN_AHV_SAMMLUNG: Txt = Txt::neu(
    "Dient die Liste der AHV, etwa der Abrechnung mit der Ausgleichskasse? Sonst: Dürfen Sie die AHV-Nummer systematisch verwenden (Art. 153c Abs. 1 AHVG)?",
    "Does the list serve the AHV itself, e.g. settling with the compensation fund? If not: may you use the AHV number systematically (Art. 153c Abs. 1 AHVG)?",
);
/// Nur ohne angegebene und ohne erschlossene Rolle und mit Funden GELB oder ORANGE.
const KLAEREN_OHNE_ROLLE: Txt = Txt::neu(
    "Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)? Dann --role angeben (Rollen: promptscan explain rot).",
    "Are you bound by a duty of confidentiality (profession, office, bank)? Then set --role (roles: promptscan explain red).",
);
/// Anwendbares Datenschutzrecht, unter «Beachten».
const DATENSCHUTZ_BEHOERDE: Txt = Txt::neu(
    "Für kantonale und kommunale Organe gilt kantonales Datenschutzrecht, für Bundesorgane das DSG. In Gerichtsverfahren und Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 DSG).",
    "Cantonal data protection law applies to cantonal and communal bodies, the DSG to federal bodies. In court proceedings and proceedings governed by federal procedural regulations, procedural law applies; in first instance administrative proceedings, the DSG (Art. 2 DSG).",
);

/// Der Bericht als Text für das Terminal (SPEC 3, docs/AUSGABE.md): Kopfzeile,
/// Zeilenansicht mit Markierungen, Gruppen nach Stufe, Ergebnisblock. Alles ist auf
/// [`Darstellung::breite`] umgebrochen.
pub fn bericht_text(b: &Bericht, d: &Darstellung) -> String {
    let mut aus = String::new();
    kopfzeile(&mut aus, b, d);
    aus.push('\n');
    zeilenansicht(&mut aus, b, d);
    aus.push('\n');
    gruppen(&mut aus, b, d);
    ergebnisblock(&mut aus, b, d);
    aus
}

/// Kurzfassung für `--quiet`: nur der Ergebnisblock.
pub fn bericht_kurz_text(b: &Bericht, d: &Darstellung) -> String {
    let mut aus = String::new();
    ergebnisblock(&mut aus, b, d);
    aus
}

/// Die Breite, auf die der Bericht umbricht.
fn breite_von(d: &Darstellung) -> usize {
    if d.breite == 0 {
        VORGABE_BREITE
    } else {
        d.breite.max(MIN_BREITE)
    }
}

/// Text für die Ausgabe; mit `--ascii` umschrieben.
fn ausgeben(s: &str, d: &Darstellung) -> String {
    if !d.ascii {
        return s.to_string();
    }
    let mut aus = String::with_capacity(s.len());
    for c in s.chars() {
        ascii_zeichen(c, &mut aus);
    }
    aus
}

/// Schreibt ein Zeichen für `--ascii`: Umlaute umschrieben, Akzente weggelassen,
/// Anführungszeichen gerade, Rahmen und Mittelpunkt als `|` und `-`. Was keinen lateinischen
/// Grundbuchstaben hat, wird `?`, so oft, wie das Zeichen Spalten belegt.
fn ascii_zeichen(c: char, aus: &mut String) {
    if c.is_ascii() {
        aus.push(c);
        return;
    }
    let ersatz = match c {
        'ä' | 'æ' => "ae",
        'ö' | 'œ' => "oe",
        'ü' => "ue",
        'Ä' | 'Æ' => "Ae",
        'Ö' | 'Œ' => "Oe",
        'Ü' => "Ue",
        'ß' => "ss",
        'ø' => "o",
        'Ø' => "O",
        'đ' => "d",
        'Đ' => "D",
        'ł' => "l",
        'Ł' => "L",
        '«' | '»' | '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => "\"",
        '‹' | '›' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => "'",
        '…' => "...",
        '│' => "|",
        '·' | '\u{AD}' | '\u{2010}'..='\u{2015}' | '\u{2212}' => "-",
        '×' => "x",
        '\u{A0}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' => " ",
        ERSATZ => "?",
        _ => "",
    };
    if !ersatz.is_empty() {
        aus.push_str(ersatz);
        return;
    }
    let breite = zeichenbreite(c);
    if breite == 0 {
        return;
    }
    let vorher = aus.len();
    aus.extend(std::iter::once(c).nfd().filter(char::is_ascii));
    if aus.len() == vorher {
        aus.extend(std::iter::repeat_n('?', breite));
    }
}

/// Spalten, die ein Text ohne Steuerfolgen im Terminal belegt.
fn spalten(s: &str) -> usize {
    s.chars().map(zeichenbreite).sum()
}

/// Erster Buchstabe gross.
fn gross(s: &str) -> String {
    let mut zeichen = s.chars();
    match zeichen.next() {
        Some(erstes) => erstes.to_uppercase().chain(zeichen).collect(),
        None => String::new(),
    }
}

/// Paar aus Schrift und Grund einer Stufe für Plaketten und Textmarker (Gestaltung
/// «farben»): feste 256 Farben, oder die 16 Grundfarben, wenn das Terminal nur diese kennt.
pub fn stil_mit(stufe: Stufe, nur_16_farben: bool) -> Style {
    let (schrift, grund): (anstyle::Color, anstyle::Color) = if nur_16_farben {
        match stufe {
            Stufe::Rot => (AnsiColor::BrightWhite.into(), AnsiColor::Red.into()),
            Stufe::Orange => (AnsiColor::Black.into(), AnsiColor::Yellow.into()),
            Stufe::Gelb => (AnsiColor::Black.into(), AnsiColor::BrightYellow.into()),
            Stufe::Hinweis => (AnsiColor::BrightWhite.into(), AnsiColor::Blue.into()),
        }
    } else {
        let (s, g) = match stufe {
            Stufe::Rot => (231, 160),
            Stufe::Orange => (16, 208),
            Stufe::Gelb => (16, 220),
            Stufe::Hinweis => (16, 117),
        };
        (Ansi256Color(s).into(), Ansi256Color(g).into())
    };
    Style::new().fg_color(Some(schrift)).bg_color(Some(grund))
}

/// Stil einer Stufe mit 256 Farben, siehe [`stil_mit`].
pub fn stil(stufe: Stufe) -> Style {
    stil_mit(stufe, false)
}

/// Die Plakette einer Stufe: mit Farbe das Wort mit je einem Leerzeichen Innenabstand auf
/// dem Grund der Stufe, ohne Farbe das Wort allein. Gibt den Text und seine Spalten zurück.
fn plakette(stufe: Stufe, d: &Darstellung) -> (String, usize) {
    let wort = stufe.wort(d.englisch);
    if d.farbe {
        let s = stil_mit(stufe, d.nur_16_farben);
        (format!("{s} {wort} {s:#}"), wort.len() + 2)
    } else {
        (wort.to_string(), wort.len())
    }
}

/// Die Initiale einer Stufe in der Markierungszeile: R, O, G, H; englisch R, O, Y, N.
fn initiale(stufe: Stufe, englisch: bool) -> char {
    stufe.wort(englisch).chars().next().unwrap_or('^')
}

/// Der Name einer Stufe für `promptscan explain`.
fn stufe_name(stufe: Stufe, englisch: bool) -> &'static str {
    match (stufe, englisch) {
        (Stufe::Rot, false) => "rot",
        (Stufe::Rot, true) => "red",
        (Stufe::Orange, _) => "orange",
        (Stufe::Gelb, false) => "gelb",
        (Stufe::Gelb, true) => "yellow",
        (Stufe::Hinweis, false) => "hinweis",
        (Stufe::Hinweis, true) => "note",
    }
}

// --- Umbruch ---------------------------------------------------------------------------

/// Ein Stück Fliesstext für den Umbruch: `wort` wird nicht getrennt, ausser es passt allein
/// nicht in eine Zeile; `leer` sagt, ob davor ein Leerzeichen steht; `satz`, ob hier ein
/// Satz beginnt, der nicht in den letzten [`SATZ_REST`] Spalten einer Zeile anfangen soll.
struct Stueck {
    wort: String,
    leer: bool,
    satz: bool,
}

/// Ein Satz im Schlussblock beginnt nicht in den letzten zwölf Spalten einer Zeile, sondern
/// auf der nächsten.
const SATZ_REST: usize = 12;

/// Abkürzungen, nach denen kein neuer Satz beginnt.
const ABKUERZUNGEN: &[&str] = &[
    "bzw", "etc", "vgl", "geb", "Tel", "Abs", "Art", "lit", "Ziff",
];

/// Markiert die Stücke, mit denen ein Satz beginnt: nach «.», «?», «!» oder «;», nicht nach
/// einer Abkürzung («z. B.», «e.g.», «geb.»).
fn satzanfaenge(mut v: Vec<Stueck>) -> Vec<Stueck> {
    for i in 1..v.len() {
        let vorher = &v[i - 1].wort;
        let letztes = vorher.rsplit(' ').next().unwrap_or("");
        let ende = match letztes.chars().last() {
            Some('?' | '!' | ';') => true,
            Some('.') => {
                let kern = letztes.trim_end_matches('.');
                kern.chars().count() > 2 && !kern.contains('.') && !ABKUERZUNGEN.contains(&kern)
            }
            _ => false,
        };
        v[i].satz = ende && v[i].leer;
    }
    v
}

/// Was beim Umbruch zusammenbleibt: Normzitate, Ausschnitte in «…» und Befehle.
static ZUSAMMEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"Art\. \d+[a-z]*(?: und \d+[a-z]*)?(?: (?:und )?(?:Abs\.|Ziff\.|lit\.|Satz) [0-9a-z]+(?: und [0-9a-z]+)?| f\.| ff\.)* [A-Z][A-Za-z]*",
        r"|Anhang 1(?: Ziff\. \d+)? DSV",
        r"|\d+(?:\.\d+)+ TAV E\.164",
        r"|«[^»]*»",
        r"|promptscan (?:explain|rules)(?: [^\s·]+)?",
        r"|--role [^\s]+",
        // Der Befehl für Windows PowerShell 5.1 in den Meldungen zur Eingabe.
        r"|\$[A-Za-z]+ = [^\s]+",
    ))
    .expect("Muster für den Umbruch")
});

/// Teile eines Normzitats, nach denen nie umbrochen wird, wenn eine Nummer folgt:
/// «Art. 5», «Abs. 1», «lit. a», «Ziff. 2», «SR 235.1», «Nr. 98».
const ZITAT_VOR_NUMMER: &[&str] = &[
    "Art.", "Abs.", "lit.", "Bst.", "Ziff.", "let.", "para.", "Satz", "Nr.", "SR", "Anhang",
];

/// Teile eines Normzitats, vor denen nach einer Nummer nie umbrochen wird: «Art. 4 Abs. 1
/// lit. a», «Art. 5 f.».
const ZITAT_NACH_NUMMER: &[&str] = &[
    "Abs.", "lit.", "Bst.", "Ziff.", "let.", "para.", "Satz", "f.", "ff.",
];

/// Monate eines Datums, das zusammenbleibt («1. Januar 2026»).
const MONATE: &[&str] = &[
    "Januar",
    "Februar",
    "März",
    "Maerz",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// Ein Wort ohne öffnende und schliessende Satzzeichen: «(DSG).» wird «DSG».
fn wortkern(w: &str) -> &str {
    w.trim_start_matches(['(', '[', '«', '"'])
        .trim_end_matches([',', ')', ']', '.', ';', ':', '»', '"'])
}

/// Eine Nummer in einem Normzitat: «1», «2.2.4», «1bis», «4(1)(g)», «1–6» oder ein
/// einzelner Kleinbuchstabe («a» nach «lit.»).
fn ist_zitatnummer(w: &str) -> bool {
    let k = wortkern(w);
    k.starts_with(|c: char| c.is_ascii_digit())
        || (k.chars().count() == 1 && k.chars().all(|c| c.is_ascii_lowercase()))
}

/// Ein Erlasskürzel: zwei bis acht Buchstaben, davon mindestens zwei gross («DSG», «OR»,
/// «StGB», «BankG», «BGÖ»). Ein falscher Treffer bindet nur zwei Wörter mehr aneinander.
fn ist_erlasskuerzel(w: &str) -> bool {
    let k = wortkern(w);
    let n = k.chars().count();
    (2..=8).contains(&n)
        && k.chars().all(char::is_alphabetic)
        && k.starts_with(char::is_uppercase)
        && k.chars().filter(|c| c.is_uppercase()).count() >= 2
}

/// Ob zwischen `vorher` und `nachher` (zwei Stücke ohne Leerzeichen am Rand) nicht
/// umbrochen werden darf, weil sie zu einem Normzitat oder einem Datum gehören.
fn bleibt_zusammen(vorher: &str, nachher: &str) -> bool {
    let (Some(ende), Some(anfang)) = (vorher.rsplit(' ').next(), nachher.split(' ').next()) else {
        return false;
    };
    let ende_offen = ende.trim_start_matches(['(', '[', '«', '"']);
    let schliesst = ende.ends_with([',', ';', ':', ')', '.', '»', '"']);
    // «Art.» / «5», «Abs.» / «1», «lit.» / «a», «SR» / «235.1»
    (ZITAT_VOR_NUMMER.contains(&ende_offen) && ist_zitatnummer(anfang))
        // «Art. 4» / «Abs. 1», «Art. 5» / «f.»
        || (ZITAT_NACH_NUMMER.contains(&anfang) && ist_zitatnummer(ende) && !schliesst)
        // «Ziff. 2» / «DSG)», «Art. 5 f.» / «DSG»
        || (ist_erlasskuerzel(anfang)
            && ((ist_zitatnummer(ende) && !schliesst) || ende_offen == "f." || ende_offen == "ff."))
        // «DSG» / «Art. 5»
        || (ist_erlasskuerzel(ende) && !schliesst && anfang == "Art.")
        // «1.» / «Januar», «Januar» / «2026»
        || (MONATE.contains(&anfang)
            && ende_offen.len() <= 3
            && ende_offen.ends_with('.')
            && ende_offen[..ende_offen.len() - 1].chars().all(|c| c.is_ascii_digit()))
        || (MONATE.contains(&ende_offen)
            && wortkern(anfang).len() == 4
            && wortkern(anfang).chars().all(|c| c.is_ascii_digit()))
}

/// Fügt Stücke, zwischen denen nicht umbrochen wird ([`bleibt_zusammen`]), mit einem
/// Leerzeichen aneinander.
fn binden(teile: Vec<String>) -> Vec<String> {
    let mut v: Vec<String> = Vec::with_capacity(teile.len());
    for t in teile {
        match v.last_mut() {
            Some(vorher) if bleibt_zusammen(vorher, &t) => {
                vorher.push(' ');
                vorher.push_str(&t);
            }
            _ => v.push(t),
        }
    }
    v
}

/// Zerlegt Fliesstext in Stücke. Getrennt wird an Leerzeichen, ausser in einem Normzitat,
/// einem Datum, einem Ausschnitt in «…» oder einem Befehl.
fn stuecke(text: &str, d: &Darstellung) -> Vec<Stueck> {
    let zusammen: Vec<std::ops::Range<usize>> =
        ZUSAMMEN.find_iter(text).map(|m| m.range()).collect();
    let mut teile = Vec::new();
    let mut wort = String::new();
    for (i, c) in text.char_indices() {
        let geschuetzt = zusammen.iter().any(|r| r.start < i && i < r.end);
        if c == ' ' && !geschuetzt {
            if !wort.is_empty() {
                teile.push(std::mem::take(&mut wort));
            }
        } else {
            wort.push(c);
        }
    }
    if !wort.is_empty() {
        teile.push(wort);
    }
    binden(teile)
        .into_iter()
        .map(|w| Stueck {
            wort: ausgeben(&w, d),
            leer: true,
            satz: false,
        })
        .collect()
}

/// Ein Stück, das nicht getrennt wird, etwa ein Wert aus dem Prompt.
fn ganz(text: &str, d: &Darstellung) -> Stueck {
    Stueck {
        wort: ausgeben(text, d),
        leer: true,
        satz: false,
    }
}

/// Teilt ein Stück, das breiter als `breite` ist: an seinen Leerzeichen, wobei Normzitate
/// darin zusammenbleiben ([`binden`]), sofern sie allein passen; dann zeichenweise.
fn zerteilen(s: Stueck, breite: usize) -> Vec<Stueck> {
    if spalten(&s.wort) <= breite {
        return vec![s];
    }
    let woerter = |t: &str| -> Vec<String> {
        t.split(' ')
            .filter(|w| !w.is_empty())
            .map(String::from)
            .collect()
    };
    let teile: Vec<String> = binden(woerter(&s.wort))
        .into_iter()
        .flat_map(|g| {
            if spalten(&g) <= breite {
                vec![g]
            } else {
                woerter(&g)
            }
        })
        .collect();
    let mut v = Vec::new();
    let mut leer = s.leer;
    let mut satz = s.satz;
    for teil in &teile {
        if spalten(teil) <= breite {
            v.push(Stueck {
                wort: teil.to_string(),
                leer,
                satz,
            });
            leer = true;
            satz = false;
            continue;
        }
        let mut stueck = String::new();
        let mut belegt = 0;
        for c in teil.chars() {
            let w = zeichenbreite(c);
            if belegt + w > breite && !stueck.is_empty() {
                v.push(Stueck {
                    wort: std::mem::take(&mut stueck),
                    leer,
                    satz,
                });
                leer = false;
                satz = false;
                belegt = 0;
            }
            stueck.push(c);
            belegt += w;
        }
        if !stueck.is_empty() {
            v.push(Stueck {
                wort: stueck,
                leer,
                satz,
            });
        }
        leer = true;
        satz = false;
    }
    v
}

/// Bricht Stücke um: Die erste Zeile hat `erste` Spalten Platz, jede weitere `folge`.
/// Passt ein Stück nicht mehr in die erste Zeile, bleibt sie leer. Ein markierter
/// Satzanfang ([`satzanfaenge`]) beginnt nicht in den letzten [`SATZ_REST`] Spalten.
fn umbrechen(stuecke: Vec<Stueck>, erste: usize, folge: usize) -> Vec<String> {
    let folge = folge.max(1);
    let mut zeilen = vec![String::new()];
    let mut belegt = 0;
    let mut platz = erste;
    for s in stuecke.into_iter().flat_map(|s| zerteilen(s, folge)) {
        let w = spalten(&s.wort);
        let abstand = usize::from(belegt > 0 && s.leer);
        let satz_zu_spaet = s.satz && belegt > 0 && platz.saturating_sub(belegt) < SATZ_REST;
        if (belegt + abstand + w > platz || satz_zu_spaet) && (belegt > 0 || platz < folge) {
            zeilen.push(String::new());
            belegt = 0;
            platz = folge;
        }
        let zeile = zeilen.last_mut().expect("mindestens eine Zeile");
        if belegt > 0 && s.leer {
            zeile.push(' ');
            belegt += 1;
        }
        zeile.push_str(&s.wort);
        belegt += w;
    }
    zeilen
}

/// Schreibt Stücke umbrochen: in der ersten Zeile hinter `anfang` (`anfang_breite`
/// Spalten, danach mindestens `abstand` Leerzeichen, frühestens ab Spalte `einzug` + 1),
/// in den weiteren hängend ab Spalte `einzug` + 1.
fn zeilen_schreiben(
    aus: &mut String,
    anfang: &str,
    anfang_breite: usize,
    abstand: usize,
    einzug: usize,
    stuecke: Vec<Stueck>,
    breite: usize,
) {
    let spalte = einzug.max(anfang_breite + abstand);
    let zeilen = umbrechen(
        stuecke,
        breite.saturating_sub(spalte),
        breite.saturating_sub(einzug),
    );
    for (i, z) in zeilen.iter().enumerate() {
        if i > 0 {
            let _ = writeln!(aus, "{:einzug$}{z}", "");
        } else if z.is_empty() {
            let _ = writeln!(aus, "{}", anfang.trim_end());
        } else {
            let _ = writeln!(aus, "{anfang}{:pad$}{z}", "", pad = spalte - anfang_breite);
        }
    }
}

/// Absätze mit Beschriftung in den ersten zehn Spalten («Klären», «Mehr» …); jeder Absatz
/// beginnt eine neue Zeile, die Beschriftung steht nur beim ersten.
fn beschriftet(aus: &mut String, beschriftung: Txt, absaetze: &[String], d: &Darstellung) {
    let breite = breite_von(d);
    for (i, a) in absaetze.iter().enumerate() {
        let anfang = if i == 0 {
            ausgeben(beschriftung.in_sprache(d.englisch), d)
        } else {
            String::new()
        };
        let anfang_breite = spalten(&anfang);
        zeilen_schreiben(
            aus,
            &anfang,
            anfang_breite,
            1,
            SP_TEXT,
            satzanfaenge(stuecke(a, d)),
            breite,
        );
    }
}

// --- Kopfzeile -------------------------------------------------------------------------

/// Kopfzeile: «Lokal geprüft · Rolle: … · Ziel: …». Passt sie nicht in
/// [`KOPF_BREITE`] Spalten, beginnt der nächste Teil eine neue Zeile. Das auslösende Wort
/// einer erschlossenen Rolle steht ganz; nur wenn schon «Lokal geprüft · Rolle: …»
/// überläuft, wird es nach [`KOPF_WORT`] Zeichen mit «…» gekürzt.
fn kopfzeile(aus: &mut String, b: &Bericht, d: &Darstellung) {
    let e = d.englisch;
    let ziel = match b.optionen.ziel {
        Ziel::Cloud => CLOUD,
        Ziel::Local => LOKALES_MODELL,
    };
    let breite = breite_von(d);
    let grenze = KOPF_BREITE.min(breite);
    let rolle_teil = |kurz: bool| format!("{}: {}", ROLLE.in_sprache(e), rolle_text(b, e, kurz));
    let mut rolle = rolle_teil(false);
    let erste = format!("{} · {rolle}", LOKAL_GEPRUEFT.in_sprache(e));
    if spalten(&ausgeben(&erste, d)) > grenze {
        rolle = rolle_teil(true);
    }
    let teile = [
        LOKAL_GEPRUEFT.in_sprache(e).to_string(),
        rolle,
        format!("{}: {}", ZIEL.in_sprache(e), ziel.in_sprache(e)),
    ];
    let mut zeilen: Vec<String> = Vec::new();
    for teil in teile {
        match zeilen.last_mut() {
            Some(z) if spalten(&ausgeben(z, d)) + 3 + spalten(&ausgeben(&teil, d)) <= grenze => {
                z.push_str(" · ");
                z.push_str(&teil);
            }
            _ => zeilen.push(teil),
        }
    }
    for z in zeilen {
        zeilen_schreiben(aus, "", 0, 0, 0, stuecke(&z, d), breite);
    }
}

/// Die Rolle für die Kopfzeile, mit Herkunft: «(--role)», «(Konfiguration)» oder
/// «aus «Wort»»; mit `kurz` das Wort gekürzt ([`kopf_wort`]).
fn rolle_text(b: &Bericht, e: bool, kurz: bool) -> String {
    let herkunft = |rolle: Rolle| match b.rolle_quelle {
        RolleQuelle::Konfiguration => KONFIGURATION.in_sprache(e).to_string(),
        _ if rolle == Rolle::None => "--role none".to_string(),
        _ => "--role".to_string(),
    };
    match &b.lage {
        Lage::AusText { rolle, stelle, .. } => format!(
            "{}, {} «{}»",
            rolle.anzeige(e),
            AUS.in_sprache(e),
            if kurz {
                kopf_wort(stelle)
            } else {
                stelle_einzeilig(stelle)
            }
        ),
        Lage::Angegeben { rolle } => format!("{} ({})", rolle.anzeige(e), herkunft(*rolle)),
        Lage::Keine => match b.optionen.rolle {
            Some(rolle) => format!("{} ({})", rolle.anzeige(e), herkunft(rolle)),
            None => NICHT_ANGEGEBEN.in_sprache(e).to_string(),
        },
    }
}

/// Das auslösende Wort für die Kopfzeile, höchstens [`KOPF_WORT`] Zeichen; gekürzt wird
/// nach einem ganzen Wort («Als Anwältin habe …»), ein einzelnes langes Wort mitten drin.
fn kopf_wort(stelle: &str) -> String {
    let stelle = stelle_einzeilig(stelle);
    if stelle.chars().count() <= KOPF_WORT {
        return stelle;
    }
    let anfang: String = stelle.chars().take(KOPF_WORT - 1).collect();
    let mut kurz = match anfang.rfind(' ') {
        Some(i) if i > 0 => anfang[..=i].to_string(),
        _ => anfang,
    };
    kurz.push('…');
    kurz
}

// --- Zeilenansicht ---------------------------------------------------------------------

/// Anzahl der gezeigten Zeilen: Die leere Zeile nach dem letzten Zeilenende zählt nicht.
fn sichtbare_zeilen(text: &Text) -> usize {
    let n = text.zeilen_anzahl();
    if n > 1 && text.zeile(n).is_empty() {
        n - 1
    } else {
        n
    }
}

/// Für jedes Byte des Textes die höchste Stufe der Funde, die es abdecken.
fn abdeckung(b: &Bericht) -> Vec<Option<Stufe>> {
    let laenge = b.text.len();
    let mut v = vec![None; laenge];
    for f in &b.funde {
        let ende = f.ende.min(laenge);
        let start = f.start.min(ende);
        for platz in &mut v[start..ende] {
            *platz = (*platz).max(Some(f.stufe));
        }
    }
    v
}

/// Eine Zelle der Zeilenansicht: ein Zeichen der Eingabe, wie es erscheint.
struct Zelle {
    text: String,
    breite: usize,
    stufe: Option<Stufe>,
    /// Leerzeichen oder Tabulator: Hier darf die Zeile umbrechen.
    leer: bool,
}

/// Der Rand der Zeilenansicht und was ihn bestimmt.
struct Rand {
    nummern: bool,
    nummernbreite: usize,
    strich: char,
    auslassung: String,
}

impl Rand {
    fn zeile(&self, nr: usize) -> String {
        let nummer = if self.nummern {
            nr.to_string()
        } else {
            String::new()
        };
        format!(" {nummer:>w$} {} ", self.strich, w = self.nummernbreite)
    }

    fn folge(&self) -> String {
        format!(" {:w$} {} ", "", self.strich, w = self.nummernbreite)
    }
}

/// Die Zeilenansicht. Bis [`ZEILEN_GANZ`] Zeilen alle, darüber nur
/// die Zeilen mit Funden und für jede Lücke «… │ N Zeilen ohne Fund». Zeilennummern erst
/// über [`ZEILEN_OHNE_NUMMER`] Zeilen. Lange Zeilen brechen um und behalten den Rand. Mit
/// Farbe sind die Fundstellen in der Farbe ihrer Stufe hinterlegt; ohne Farbe steht unter
/// jeder Teilzeile mit Fund eine Markierungszeile und am Ende eine Legende.
fn zeilenansicht(aus: &mut String, b: &Bericht, d: &Darstellung) {
    let abdeckung = abdeckung(b);
    let anzahl = sichtbare_zeilen(&b.text);
    let alle = anzahl <= ZEILEN_GANZ;
    let auslassung = ausgeben("…", d);
    let nummern = anzahl > ZEILEN_OHNE_NUMMER;
    let rand = Rand {
        nummern,
        nummernbreite: if nummern {
            anzahl.to_string().len().max(spalten(&auslassung))
        } else {
            1
        },
        strich: if d.ascii { '|' } else { '│' },
        auslassung,
    };
    let mut markiert: BTreeSet<Stufe> = BTreeSet::new();
    let mut vorige = 0;
    for nr in 1..=anzahl {
        let bereich = b.text.zeilenbereich(nr);
        let hat_fund = abdeckung[bereich].iter().any(Option::is_some);
        if !alle && !hat_fund {
            continue;
        }
        if nr > vorige + 1 {
            luecke(aus, nr - vorige - 1, &rand, d);
        }
        zeile_ausgeben(aus, b, nr, &rand, &abdeckung, d, &mut markiert);
        vorige = nr;
    }
    if vorige < anzahl {
        luecke(aus, anzahl - vorige, &rand, d);
    }
    if !d.farbe && !markiert.is_empty() {
        let legende: Vec<String> = STUFEN
            .iter()
            .filter(|s| markiert.contains(*s))
            .map(|&s| format!("{} {}", initiale(s, d.englisch), s.wort(d.englisch)))
            .collect();
        let einzug = spalten(&rand.folge());
        let _ = writeln!(aus, "{:einzug$}{}", "", ausgeben(&legende.join(" · "), d));
    }
}

fn luecke(aus: &mut String, n: usize, rand: &Rand, d: &Darstellung) {
    let text = match (n, d.englisch) {
        (1, false) => "1 Zeile ohne Fund".to_string(),
        (n, false) => format!("{n} Zeilen ohne Fund"),
        (1, true) => "1 line without findings".to_string(),
        (n, true) => format!("{n} lines without findings"),
    };
    let pad = rand.nummernbreite.saturating_sub(spalten(&rand.auslassung));
    let _ = writeln!(
        aus,
        " {:pad$}{} {} {text}",
        "", rand.auslassung, rand.strich
    );
}

/// Die Zellen einer Zeile: Tabulatoren bis zum nächsten Stopp, Steuerzeichen sichtbar
/// ersetzt, mit `--ascii` umschrieben.
fn zellen(zeile: &str, start: usize, abdeckung: &[Option<Stufe>], d: &Darstellung) -> Vec<Zelle> {
    let mut v = Vec::with_capacity(zeile.len());
    let mut spalte = 0;
    for (i, c) in zeile.char_indices() {
        let mut text = String::new();
        let mut breite = zeichen_anzeigen(c, spalte, &mut text);
        if d.ascii {
            text = ausgeben(&text, d);
            breite = spalten(&text);
        }
        spalte += breite;
        v.push(Zelle {
            text,
            breite,
            stufe: abdeckung[start + i],
            leer: c == ' ' || c == '\t',
        });
    }
    v
}

/// Teilt eine Zeile in Teilzeilen von höchstens `frei` Spalten. Umbrochen wird an
/// Leerzeichen ausserhalb einer Markierung; passt eine Markierung allein nicht in eine
/// Zeile, auch an Leerzeichen in ihr und zuletzt zwischen zwei Zeichen. Leerzeichen am
/// Anfang einer Folgezeile entfallen.
fn teilzeilen(zellen: &[Zelle], frei: usize) -> Vec<std::ops::Range<usize>> {
    let n = zellen.len();
    let mut v = Vec::new();
    let mut start = 0;
    loop {
        let mut belegt = 0;
        let mut ende = start;
        let mut bruch_frei = None;
        let mut bruch_markiert = None;
        while ende < n && belegt + zellen[ende].breite <= frei {
            if zellen[ende].leer && ende > start {
                if zellen[ende].stufe.is_none() {
                    bruch_frei = Some(ende);
                } else {
                    bruch_markiert = Some(ende);
                }
            }
            belegt += zellen[ende].breite;
            ende += 1;
        }
        if ende >= n {
            v.push(start..n);
            return v;
        }
        let naechste = &zellen[ende];
        let bruch = if naechste.leer && naechste.stufe.is_none() {
            ende
        } else if let Some(b) = bruch_frei {
            b
        } else if naechste.leer {
            ende
        } else if let Some(b) = bruch_markiert {
            b
        } else {
            ende.max(start + 1)
        };
        v.push(start..bruch);
        start = bruch;
        while start < n && zellen[start].leer {
            start += 1;
        }
        if start >= n {
            return v;
        }
    }
}

/// Eine Zeile des Prompts, umbrochen; jede Teilzeile mit eigener Markierungszeile.
fn zeile_ausgeben(
    aus: &mut String,
    b: &Bericht,
    nr: usize,
    rand: &Rand,
    abdeckung: &[Option<Stufe>],
    d: &Darstellung,
    markiert: &mut BTreeSet<Stufe>,
) {
    let bereich = b.text.zeilenbereich(nr);
    let zellen = zellen(
        &b.text.als_str()[bereich.clone()],
        bereich.start,
        abdeckung,
        d,
    );
    let folge = rand.folge();
    let frei = breite_von(d).saturating_sub(spalten(&folge)).max(1);
    for (i, teil) in teilzeilen(&zellen, frei).into_iter().enumerate() {
        let erster = if i == 0 {
            rand.zeile(nr)
        } else {
            folge.clone()
        };
        teilzeile_ausgeben(aus, &erster, &folge, &zellen[teil], d, markiert);
    }
}

fn teilzeile_ausgeben(
    aus: &mut String,
    rand: &str,
    rand_folge: &str,
    zellen: &[Zelle],
    d: &Darstellung,
    markiert: &mut BTreeSet<Stufe>,
) {
    let ende = zellen
        .iter()
        .rposition(|z| !(z.leer && z.stufe.is_none()))
        .map_or(0, |i| i + 1);
    let mut inhalt = String::new();
    let mut marken = String::new();
    let mut spalte = 0;
    let mut offen: Option<Stufe> = None;
    let mut lauf: Option<(Stufe, usize, usize)> = None;
    for z in &zellen[..ende] {
        if d.farbe && z.stufe != offen {
            if let Some(s) = offen {
                let _ = write!(inhalt, "{:#}", stil_mit(s, d.nur_16_farben));
            }
            if let Some(s) = z.stufe {
                let _ = write!(inhalt, "{}", stil_mit(s, d.nur_16_farben));
            }
            offen = z.stufe;
        }
        inhalt.push_str(&z.text);
        lauf = match (lauf, z.stufe) {
            (Some((s, von, w)), Some(t)) if s == t => Some((s, von, w + z.breite)),
            (alt, neu) => {
                if let Some(l) = alt {
                    marke(&mut marken, l, d.englisch, markiert);
                }
                neu.map(|t| (t, spalte, z.breite))
            }
        };
        spalte += z.breite;
    }
    if let Some(s) = offen {
        let _ = write!(inhalt, "{:#}", stil_mit(s, d.nur_16_farben));
    }
    if let Some(l) = lauf {
        marke(&mut marken, l, d.englisch, markiert);
    }
    if inhalt.is_empty() {
        let _ = writeln!(aus, "{}", rand.trim_end());
    } else {
        let _ = writeln!(aus, "{rand}{inhalt}");
    }
    if !d.farbe && !marken.is_empty() {
        let _ = writeln!(aus, "{rand_folge}{marken}");
    }
}

/// Setzt die Marke eines Laufs gleicher Stufe: die Initiale, dann `^`.
fn marke(
    marken: &mut String,
    (stufe, von, breite): (Stufe, usize, usize),
    englisch: bool,
    markiert: &mut BTreeSet<Stufe>,
) {
    if breite == 0 {
        return;
    }
    let jetzt = marken.chars().count();
    marken.extend(std::iter::repeat_n(' ', von.saturating_sub(jetzt)));
    marken.push(initiale(stufe, englisch));
    marken.extend(std::iter::repeat_n('^', breite - 1));
    markiert.insert(stufe);
}

// --- Gruppen ---------------------------------------------------------------------------

/// Eine Zeile einer Gruppe: die Art in Worten und die Werte; gleiche Werte einmal mit
/// Anzahl.
struct Reihe {
    art: String,
    /// Die Art steht auf eigener Zeile ab Spalte 11, die Werte darunter ab Spalte 25.
    eigene_zeile: bool,
    /// Werte in «…» setzen (Ausschnitte bei HINWEIS).
    zitiert: bool,
    werte: Vec<Wert>,
    /// Text unter den Werten (HINWEIS).
    erklaerung: Option<String>,
}

struct Wert {
    text: String,
    zusatz: Option<String>,
    anzahl: usize,
    zeilen: Vec<usize>,
}

impl Reihe {
    fn neu(art: String) -> Reihe {
        Reihe {
            art,
            eigene_zeile: false,
            zitiert: false,
            werte: Vec::new(),
            erklaerung: None,
        }
    }

    fn dazu(&mut self, text: String, zusatz: Option<String>, zeile: usize) {
        if let Some(w) = self
            .werte
            .iter_mut()
            .find(|w| w.text == text && w.zusatz == zusatz)
        {
            w.anzahl += 1;
            if !w.zeilen.contains(&zeile) {
                w.zeilen.push(zeile);
            }
        } else {
            self.werte.push(Wert {
                text,
                zusatz,
                anzahl: 1,
                zeilen: vec![zeile],
            });
        }
    }
}

/// Die Reihe mit dieser Art, sonst eine neue am Ende.
fn reihe(reihen: &mut Vec<Reihe>, art: String) -> &mut Reihe {
    let i = match reihen.iter().position(|r| r.art == art) {
        Some(i) => i,
        None => {
            reihen.push(Reihe::neu(art));
            reihen.len() - 1
        }
    };
    &mut reihen[i]
}

/// Der Wert eines Fundes und sein Zusatz in der Sprache der Ausgabe.
fn wert_von(f: &Fund, englisch: bool) -> (String, Option<String>) {
    let zusatz = f
        .zusatz
        .as_ref()
        .map(|z| z.in_sprache(englisch).to_string());
    if f.regel == "ahv-sammlung"
        && let Some(z) = zusatz
    {
        return (z, None);
    }
    (stelle_einzeilig(&f.stelle), zusatz)
}

/// Die Ziffer von Art. 5 lit. c DSG einer Regel `art5c-N`.
fn art5c_ziffer(regel: &str) -> Option<u8> {
    regel.strip_prefix("art5c-")?.parse().ok()
}

/// Die Art eines Fundes in Worten; die Kennung steht nur in «Mehr».
fn art_name(f: &Fund, englisch: bool) -> String {
    if let Grund::BesondersSchuetzenswert { ziffer } = f.grund {
        return format!("Ziff. {ziffer}");
    }
    if let Some(z) = art5c_ziffer(f.regel) {
        return gross(normen::art5c_kategorie(z, englisch));
    }
    let paar = match f.regel {
        "ahv" => ("AHV-Nummer", "AHV number"),
        "iban" => ("IBAN", "IBAN"),
        "tel" => ("Telefon", "Telephone"),
        "mail" => ("E-Mail", "E-mail"),
        "name" => ("Name", "Name"),
        "firma" => ("Firma", "Company"),
        "adresse" => ("Adresse", "Address"),
        "wohnort" => ("Wohnort", "Residence"),
        "geburtsdatum" => ("Geburtsdatum", "Date of birth"),
        "kontext" => ("Kontext", "Context"),
        "personenbezug" => ("Personenbezug", "Reference"),
        "vermerk" => ("Vermerk", "Marking"),
        "fundstellen" => ("Fundstellen", "Citations"),
        "ahv-sammlung" => ("AHV-Liste", "AHV list"),
        andere => {
            return katalog::finde(andere).map_or_else(
                || andere.to_string(),
                |r| r.titel.in_sprache(englisch).to_string(),
            );
        }
    };
    (if englisch { paar.1 } else { paar.0 }).to_string()
}

/// Kopf einer ROT-Gruppe: Pflicht und Normen.
fn pflicht_kopf(rolle: Rolle, normen: &[&'static Norm], englisch: bool) -> String {
    if rolle == Rolle::Offen {
        let liste: Vec<String> = normen
            .iter()
            .map(|n| format!("{} ({})", n.zitat, stufen::offen_beruf(n, englisch)))
            .collect();
        format!("{}: {}", BERUF_OFFEN.in_sprache(englisch), liste.join(", "))
    } else {
        format!(
            "{}, {}",
            gross(stufen::geheimnis_name(rolle, englisch)),
            zitate(normen)
        )
    }
}

/// Die Gruppen ROT, ORANGE, GELB und HINWEIS, je mit Kopf und einer Zeile je Art, innerhalb
/// nach Position; nach jeder Gruppe eine Leerzeile.
fn gruppen(aus: &mut String, b: &Bericht, d: &Darstellung) {
    let e = d.englisch;
    let mit_zeilen = sichtbare_zeilen(&b.text) > ZEILEN_OHNE_NUMMER;
    let von = |s: Stufe| b.funde.iter().filter(move |f| f.stufe == s);

    // ROT: je Pflicht ein Kopf.
    let mut pflichten: Vec<(Rolle, Vec<&'static Norm>)> = Vec::new();
    for f in von(Stufe::Rot) {
        let (rolle, normen) = rot_pflicht(f, b);
        if !pflichten
            .iter()
            .any(|(r, n)| *r == rolle && zitate(n) == zitate(&normen))
        {
            pflichten.push((rolle, normen));
        }
    }
    for (rolle, normen) in &pflichten {
        let mut reihen = Vec::new();
        let mut besonders = Vec::new();
        for f in von(Stufe::Rot) {
            let (r, n) = rot_pflicht(f, b);
            if r != *rolle || zitate(&n) != zitate(normen) {
                continue;
            }
            let (text, zusatz) = wert_von(f, e);
            match f.grund {
                Grund::Geheimnis {
                    ziffer: Some(z), ..
                } => {
                    let art = format!("{}, {}:", ZUGLEICH.in_sprache(e), normen::art5c_zitat(z));
                    let r = reihe(&mut besonders, art);
                    r.eigene_zeile = true;
                    r.dazu(text, zusatz, f.zeile);
                }
                _ => reihe(&mut reihen, art_name(f, e)).dazu(text, zusatz, f.zeile),
            }
        }
        reihen.extend(besonders);
        gruppe_schreiben(
            aus,
            Stufe::Rot,
            &[pflicht_kopf(*rolle, normen, e)],
            &reihen,
            mit_zeilen,
            d,
        );
    }

    // ORANGE und GELB.
    for (stufe, kopf) in [
        (
            Stufe::Orange,
            vec![
                format!("{}, {}", BESONDERS.in_sprache(e), normen::DSG_5_C.zitat),
                format!(
                    "{} {}",
                    DURCH_PRIVATE.in_sprache(e),
                    normen::DSG_30_2_C.zitat
                ),
            ],
        ),
        (
            Stufe::Gelb,
            vec![format!(
                "{}, {}",
                PERSONENDATEN.in_sprache(e),
                normen::DSG_5_A.zitat
            )],
        ),
    ] {
        let mut reihen = Vec::new();
        for f in von(stufe) {
            let (text, zusatz) = wert_von(f, e);
            reihe(&mut reihen, art_name(f, e)).dazu(text, zusatz, f.zeile);
        }
        if !reihen.is_empty() {
            gruppe_schreiben(aus, stufe, &kopf, &reihen, mit_zeilen, d);
        }
    }

    // HINWEIS bei --target local: was sonst eine höhere Stufe hätte.
    let mut lokal = Vec::new();
    for sonst in [Stufe::Rot, Stufe::Orange, Stufe::Gelb] {
        for f in von(Stufe::Hinweis) {
            if f.grund == (Grund::Lokal { sonst }) {
                let (text, zusatz) = wert_von(f, e);
                let art = format!("{} {}", SONST.in_sprache(e), sonst.wort(e));
                reihe(&mut lokal, art).dazu(text, zusatz, f.zeile);
            }
        }
    }
    if !lokal.is_empty() {
        // Zwei Zeilen, wenn in der Cloud ein Fund ROT wäre: die Normen der Rolle wie im
        // ROT-Kopf.
        let mut kopf = vec![format!(
            "{} ({})",
            TITEL_LOKAL.in_sprache(e),
            normen::DSG_5_E.zitat
        )];
        let rot = von(Stufe::Hinweis).any(|f| f.grund == Grund::Lokal { sonst: Stufe::Rot });
        if rot {
            let normen = stufen::normen_fuer(b.lage.rolle().unwrap_or(Rolle::Offen));
            kopf[0].push(',');
            kopf.push(format!(
                "{} ({})",
                KEIN_OFFENBAREN.in_sprache(e),
                zitate(&normen)
            ));
        }
        gruppe_schreiben(aus, Stufe::Hinweis, &kopf, &lokal, mit_zeilen, d);
    }

    // HINWEIS: Hinweise der Regeln und der Kontext, dann was ohne Identifikator blieb:
    // Geburtsdatum und Wohnort mit ihrer Art, die Kategorien von Art. 5 lit. c DSG wie bei
    // ORANGE als «Ziff. N» unter einer Zwischenzeile mit dem Zitat.
    let mut reihen = Vec::new();
    let mut ohne = Vec::new();
    let mut ohne_c = Vec::new();
    for f in von(Stufe::Hinweis) {
        let (text, zusatz) = wert_von(f, e);
        let erklaerung = match &f.grund {
            Grund::Lokal { .. } => continue,
            Grund::OhneIdentifikator => {
                match art5c_ziffer(f.regel) {
                    Some(z) => reihe(&mut ohne_c, format!("Ziff. {z}")),
                    None => reihe(&mut ohne, art_name(f, e)),
                }
                .dazu(text, zusatz, f.zeile);
                continue;
            }
            Grund::Regelhinweis if f.regel == "fundstellen" => {
                // Die Handlung «Jede Fundstelle … prüfen» steht unter «Klären».
                Some(FUNDSTELLEN_GRUND.in_sprache(e).to_string())
            }
            Grund::Geheimniskontext {
                rolle: Rolle::Offen,
            } => Some(BERUF_OFFEN.in_sprache(e).to_string()),
            Grund::Geheimniskontext { rolle } => Some(format!(
                "{} {}",
                ERSCHLOSSEN.in_sprache(e),
                rolle.anzeige(e)
            )),
            _ => katalog::finde(f.regel)
                .and_then(|r| r.hinweis)
                .map(|h| h.in_sprache(e).to_string())
                .filter(|h| !h.is_empty()),
        };
        let r = reihe(&mut reihen, art_name(f, e));
        r.zitiert = f.regel != "ahv-sammlung";
        r.erklaerung = r.erklaerung.take().or(erklaerung);
        r.dazu(text, zusatz, f.zeile);
    }
    if !ohne.is_empty() || !ohne_c.is_empty() {
        let mut zwischen = Reihe::neu(OHNE_IDENTIFIKATOR.in_sprache(e).to_string());
        zwischen.eigene_zeile = true;
        reihen.push(zwischen);
        reihen.extend(ohne);
        if !ohne_c.is_empty() {
            let mut kategorie = Reihe::neu(format!(
                "{} {}:",
                KATEGORIE_NACH.in_sprache(e),
                normen::DSG_5_C.zitat
            ));
            kategorie.eigene_zeile = true;
            reihen.push(kategorie);
            reihen.extend(ohne_c);
        }
    }
    if !reihen.is_empty() {
        gruppe_schreiben(
            aus,
            Stufe::Hinweis,
            &[NICHT_BEURTEILT.in_sprache(e).to_string()],
            &reihen,
            mit_zeilen,
            d,
        );
    }
}

/// Rolle und Normen eines ROT-Fundes.
fn rot_pflicht(f: &Fund, b: &Bericht) -> (Rolle, Vec<&'static Norm>) {
    match f.grund {
        Grund::Geheimnis {
            rolle,
            natuerliche_person,
            ..
        } => (rolle, stufen::normen_fuer_fund(rolle, natuerliche_person)),
        _ => {
            let rolle = b.lage.rolle().unwrap_or(Rolle::Offen);
            (rolle, stufen::normen_fuer(rolle))
        }
    }
}

/// Eine Gruppe: die Plakette, der Kopf (jeder Satz auf eigener Zeile ab Spalte 11), die
/// Reihen; danach eine Leerzeile.
fn gruppe_schreiben(
    aus: &mut String,
    stufe: Stufe,
    kopf: &[String],
    reihen: &[Reihe],
    mit_zeilen: bool,
    d: &Darstellung,
) {
    let breite = breite_von(d);
    let (p, pb) = plakette(stufe, d);
    for (i, satz) in kopf.iter().enumerate() {
        let (anfang, anfang_breite) = if i == 0 { (p.as_str(), pb) } else { ("", 0) };
        zeilen_schreiben(
            aus,
            anfang,
            anfang_breite,
            1,
            SP_TEXT,
            stuecke(satz, d),
            breite,
        );
    }
    for r in reihen {
        reihe_schreiben(aus, r, mit_zeilen, d);
    }
    aus.push('\n');
}

/// Eine Reihe: die Art ab Spalte 11, die Werte ab Spalte 25 mit « · » getrennt, die
/// Erklärung darunter.
fn reihe_schreiben(aus: &mut String, r: &Reihe, mit_zeilen: bool, d: &Darstellung) {
    let e = d.englisch;
    let breite = breite_von(d);
    let art = ausgeben(&r.art, d);
    let art_breite = spalten(&art);
    let mut werte = Vec::new();
    // Ein Zusatz, den alle von mehreren Werten tragen (etwa «Prüfziffer gültig»), entfällt.
    let gemeinsam = r.werte.len() > 1
        && r.werte
            .iter()
            .all(|w| w.zusatz.is_some() && w.zusatz == r.werte[0].zusatz);
    for (i, w) in r.werte.iter().enumerate() {
        let text = if r.zitiert {
            format!("«{}»", w.text)
        } else {
            w.text.clone()
        };
        let mut teile = vec![ganz(&text, d)];
        if let Some(z) = w.zusatz.as_ref().filter(|_| !gemeinsam) {
            // Wert und Zusatz bleiben zusammen («078 765 43 21 (Bereich Mobilnetz)»),
            // solange sie zusammen in eine Zeile passen.
            let zusammen = ganz(&format!("{text} ({z})"), d);
            if spalten(&zusammen.wort) <= breite.saturating_sub(SP_WERT) {
                teile = vec![zusammen];
            } else {
                teile.extend(stuecke(&format!("({z})"), d));
            }
        }
        if w.anzahl > 1 {
            teile.push(ganz(&format!("({}×)", w.anzahl), d));
        }
        if mit_zeilen {
            let liste: Vec<String> = w.zeilen.iter().map(ToString::to_string).collect();
            let wort = match (w.zeilen.len(), e) {
                (1, false) => "Zeile",
                (_, false) => "Zeilen",
                (1, true) => "line",
                (_, true) => "lines",
            };
            // «(Zeile 8)» bleibt zusammen; zwischen weiteren Nummern darf umgebrochen werden.
            for (k, nummer) in liste.iter().enumerate() {
                let vor = if k == 0 {
                    format!("({wort} ")
                } else {
                    String::new()
                };
                let nach = if k + 1 < liste.len() { "," } else { ")" };
                teile.push(ganz(&format!("{vor}{nummer}{nach}"), d));
            }
        }
        if i + 1 < r.werte.len()
            && let Some(letztes) = teile.last_mut()
        {
            letztes.wort.push_str(&ausgeben(" ·", d));
        }
        werte.extend(teile);
    }
    let art_passt = !r.eigene_zeile && art_breite < SP_WERT - SP_TEXT;
    if art_passt {
        let anfang = format!("{:SP_TEXT$}{art}", "");
        zeilen_schreiben(
            aus,
            &anfang,
            SP_TEXT + art_breite,
            1,
            SP_WERT,
            werte,
            breite,
        );
    } else {
        zeilen_schreiben(aus, "", 0, 0, SP_TEXT, stuecke(&r.art, d), breite);
        if !werte.is_empty() {
            zeilen_schreiben(aus, "", 0, 0, SP_WERT, werte, breite);
        }
    }
    if let Some(erklaerung) = &r.erklaerung {
        zeilen_schreiben(aus, "", 0, 0, SP_WERT, stuecke(erklaerung, d), breite);
    }
}

// --- Ergebnisblock ---------------------------------------------------------------------

fn anzahl_wort(s: Stufe, n: usize, englisch: bool) -> &'static str {
    match (s, englisch) {
        (Stufe::Rot, false) => "rot",
        (Stufe::Rot, true) => "red",
        (Stufe::Orange, _) => "orange",
        (Stufe::Gelb, false) => "gelb",
        (Stufe::Gelb, true) => "yellow",
        (Stufe::Hinweis, false) if n == 1 => "Hinweis",
        (Stufe::Hinweis, false) => "Hinweise",
        (Stufe::Hinweis, true) if n == 1 => "note",
        (Stufe::Hinweis, true) => "notes",
    }
}

/// «1 Fund», «3 Funde» («1 finding», «3 findings»).
fn funde_wort(n: usize, e: bool) -> String {
    match (n, e) {
        (1, false) => "1 Fund".to_string(),
        (n, false) => format!("{n} Funde"),
        (1, true) => "1 finding".to_string(),
        (n, true) => format!("{n} findings"),
    }
}

/// Die Zahlen je Stufe, nur die vorhandenen: «2 rot, 1 Hinweis».
fn je_stufe(anzahlen: [(Stufe, usize); 4], e: bool) -> Vec<String> {
    anzahlen
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(s, n)| format!("{n} {}", anzahl_wort(s, n, e)))
        .collect()
}

/// Zählung für den Ergebnisblock: alle Funde mit den Hinweisen, «2 Funde»; bei mehreren
/// Stufen «3 Funde: 2 rot, 1 Hinweis». Gibt zurück, ob mehrere Stufen gezählt sind (dann
/// steht die Zählung in Zeile 2).
fn zaehlung(rot: usize, orange: usize, gelb: usize, hinweis: usize, e: bool) -> (String, bool) {
    let mut s = funde_wort(rot + orange + gelb + hinweis, e);
    let arten = je_stufe(
        [
            (Stufe::Rot, rot),
            (Stufe::Orange, orange),
            (Stufe::Gelb, gelb),
            (Stufe::Hinweis, hinweis),
        ],
        e,
    );
    let mehrere = arten.len() > 1;
    if mehrere {
        let _ = write!(s, ": {}", arten.join(", "));
    }
    (s, mehrere)
}

/// Titel und Zählung der Zeile «Ergebnis»; `true`, wenn die Zählung auf eine eigene Zeile
/// gehört.
fn ergebnis_texte(b: &Bericht, hoechste: Stufe, e: bool) -> (String, String, bool) {
    let lokal: Vec<Stufe> = b
        .funde
        .iter()
        .filter_map(|f| match f.grund {
            Grund::Lokal { sonst } => Some(sonst),
            _ => None,
        })
        .collect();
    let hinweis = b.anzahl(Stufe::Hinweis);
    if hoechste == Stufe::Hinweis && !lokal.is_empty() {
        // «4 Funde: 4 Hinweise; in der Cloud 3 rot, 1 Hinweis».
        let titel = if lokal.contains(&Stufe::Rot) {
            TITEL_LOKAL_ROT
        } else {
            TITEL_LOKAL
        };
        let wie_viele = |s: Stufe| lokal.iter().filter(|&&x| x == s).count();
        let wolke = je_stufe(
            [
                (Stufe::Rot, wie_viele(Stufe::Rot)),
                (Stufe::Orange, wie_viele(Stufe::Orange)),
                (Stufe::Gelb, wie_viele(Stufe::Gelb)),
                (Stufe::Hinweis, hinweis - lokal.len()),
            ],
            e,
        );
        let zaehlung = format!(
            "{}: {hinweis} {}; {} {}",
            funde_wort(hinweis, e),
            anzahl_wort(Stufe::Hinweis, hinweis, e),
            IN_DER_CLOUD.in_sprache(e),
            wolke.join(", ")
        );
        return (titel.in_sprache(e).to_string(), zaehlung, true);
    }
    let mut titel = match hoechste {
        Stufe::Rot => TITEL_ROT,
        Stufe::Orange => BESONDERS,
        Stufe::Gelb => PERSONENDATEN,
        Stufe::Hinweis => TITEL_HINWEIS,
    }
    .in_sprache(e)
    .to_string();
    if hoechste == Stufe::Rot && rot_rolle(b) == Rolle::Offen {
        // «Beruf offen» wie in Kopfzeile und Gruppenkopf.
        let _ = write!(titel, " ({})", Rolle::Offen.anzeige(e));
    }
    let (z, mehrere) = zaehlung(
        b.anzahl(Stufe::Rot),
        b.anzahl(Stufe::Orange),
        b.anzahl(Stufe::Gelb),
        hinweis,
        e,
    );
    (titel, z, mehrere)
}

/// Die Rolle der ROT-Funde: aus dem ersten Fund, sonst aus der Lage, sonst «Beruf offen».
fn rot_rolle(b: &Bericht) -> Rolle {
    b.funde
        .iter()
        .find_map(|f| match f.grund {
            Grund::Geheimnis { rolle, .. } if f.stufe == Stufe::Rot => Some(rolle),
            _ => None,
        })
        .or(b.lage.rolle())
        .unwrap_or(Rolle::Offen)
}

/// Der Ergebnisblock am Schluss: Ergebnis, Eingabe, Klären, Beachten, Mehr.
fn ergebnisblock(aus: &mut String, b: &Bericht, d: &Darstellung) {
    let e = d.englisch;
    let breite = breite_von(d);
    let beschriftung = ausgeben(ERGEBNIS.in_sprache(e), d);
    let pad = SP_TEXT - spalten(&beschriftung);
    match b.hoechste() {
        None => {
            let text = ausgeben(KEINE_FUNDE.in_sprache(e), d);
            let _ = writeln!(aus, "{beschriftung}{:pad$}{}", "", fett(&text, d));
        }
        Some(hoechste) => {
            let (p, pb) = plakette(hoechste, d);
            let anfang = format!("{beschriftung}{:pad$}{p}", "");
            let abstand = if d.farbe { 1 } else { 3 };
            let (titel, zaehlung, eigene) = ergebnis_texte(b, hoechste, e);
            let platz = breite.saturating_sub(SP_TEXT + pb + abstand);
            let zusammen = format!("{titel} · {zaehlung}");
            if !eigene && spalten(&ausgeben(&zusammen, d)) <= platz {
                let mut st = stuecke(&titel, d);
                if let Some(letztes) = st.last_mut() {
                    letztes.wort.push_str(&ausgeben(" ·", d));
                }
                st.extend(stuecke(&zaehlung, d));
                zeilen_schreiben(aus, &anfang, SP_TEXT + pb, abstand, SP_TEXT, st, breite);
            } else {
                zeilen_schreiben(
                    aus,
                    &anfang,
                    SP_TEXT + pb,
                    abstand,
                    SP_TEXT,
                    stuecke(&titel, d),
                    breite,
                );
                zeilen_schreiben(aus, "", 0, 0, SP_TEXT, stuecke(&zaehlung, d), breite);
            }
        }
    }
    beschriftet(aus, OHNE_ETIKETT, &Eingabe::von(&b.text).meldungen(e), d);
    beschriftet(aus, KLAEREN, &klaeren(b, e), d);
    beschriftet(aus, BEACHTEN, &beachten(b, e), d);
    let befehle = mehr(b, e);
    if !befehle.is_empty() {
        let anfang = ausgeben(MEHR.in_sprache(e), d);
        let n = befehle.len();
        let st: Vec<Stueck> = befehle
            .iter()
            .enumerate()
            .map(|(i, befehl)| {
                let mut s = ganz(befehl, d);
                if i + 1 < n {
                    s.wort.push_str(&ausgeben(" ·", d));
                }
                s
            })
            .collect();
        zeilen_schreiben(aus, &anfang, spalten(&anfang), 1, SP_TEXT, st, breite);
    }
}

/// Die Frage zu ROT je Geheimhaltungspflicht.
fn klaeren_rot(rolle: Rolle, e: bool) -> String {
    match rolle {
        Rolle::Clergy
        | Rolle::Lawyer
        | Rolle::Notary
        | Rolle::PatentAttorney
        | Rolle::Auditor
        | Rolle::Doctor
        | Rolle::Psychologist => KLAEREN_321.in_sprache(e).to_string(),
        Rolle::Student => KLAEREN_321_STUDIERENDE.in_sprache(e).to_string(),
        Rolle::Authority | Rolle::Court | Rolle::CriminalAuthority => {
            KLAEREN_320.in_sprache(e).to_string()
        }
        Rolle::Bank => KLAEREN_BANK.in_sprache(e).to_string(),
        Rolle::FinancialInstitution => KLAEREN_FINIG.in_sprache(e).to_string(),
        Rolle::Professional => KLAEREN_62.in_sprache(e).to_string(),
        Rolle::Offen | Rolle::None => format!(
            "{} {} --role {}",
            BRAUCHT_DIE_KI.in_sprache(e),
            WELCHE_PFLICHT.in_sprache(e),
            offen_angebot()
        ),
    }
}

/// Die Fragen der Zeile «Klären»: zur höchsten Stufe, ohne Rolle die Frage nach einer
/// Geheimhaltungspflicht, bei lokalem Modell, zu jeder Art von Hinweis, zuletzt zum Ausland.
fn klaeren(b: &Bericht, e: bool) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let geheimnis_rolle = b.funde.iter().find_map(|f| match f.grund {
        Grund::Geheimnis { rolle, .. } if f.stufe == Stufe::Rot => Some(rolle),
        _ => None,
    });
    match b.hoechste() {
        Some(Stufe::Rot) => v.push(klaeren_rot(rot_rolle(b), e)),
        Some(Stufe::Orange) => v.push(KLAEREN_ORANGE.in_sprache(e).to_string()),
        Some(Stufe::Gelb) => v.push(KLAEREN_GELB.in_sprache(e).to_string()),
        _ => {}
    }
    // Weder --role noch Konfiguration, keine Rolle aus dem Text, Funde GELB oder
    // ORANGE; nicht bei --role none.
    let ohne_rolle = b.optionen.rolle.is_none() && b.lage == Lage::Keine;
    if ohne_rolle && b.anzahl(Stufe::Gelb) + b.anzahl(Stufe::Orange) > 0 {
        v.push(KLAEREN_OHNE_ROLLE.in_sprache(e).to_string());
    }
    if b.funde
        .iter()
        .any(|f| matches!(f.grund, Grund::Lokal { .. }))
    {
        v.push(KLAEREN_LOKAL.in_sprache(e).to_string());
    }
    for f in b.funde.iter().filter(|f| f.stufe == Stufe::Hinweis) {
        let frage = match (&f.grund, f.regel) {
            (Grund::Regelhinweis, "fundstellen") => {
                Some(KLAEREN_FUNDSTELLEN.in_sprache(e).to_string())
            }
            (Grund::Regelhinweis, "vermerk") => Some(KLAEREN_VERMERK.in_sprache(e).to_string()),
            (Grund::Regelhinweis, "ahv-sammlung") => {
                Some(KLAEREN_AHV_SAMMLUNG.in_sprache(e).to_string())
            }
            (
                Grund::Geheimniskontext {
                    rolle: Rolle::Offen,
                },
                _,
            ) if geheimnis_rolle != Some(Rolle::Offen) => Some(format!(
                "{} --role {}",
                WELCHE_PFLICHT.in_sprache(e),
                offen_angebot()
            )),
            (Grund::Geheimniskontext { rolle }, _) if *rolle != Rolle::Offen => {
                Some(KLAEREN_KONTEXT.in_sprache(e).to_string())
            }
            (Grund::OhneIdentifikator, _) => Some(KLAEREN_PERSONENBEZUG.in_sprache(e).to_string()),
            _ => None,
        };
        if let Some(frage) = frage
            && !v.contains(&frage)
        {
            v.push(frage);
        }
    }
    let ab_gelb = b.funde.iter().any(|f| f.stufe != Stufe::Hinweis);
    if b.optionen.ziel == Ziel::Cloud && ab_gelb {
        let behoerde = matches!(
            b.lage.rolle(),
            Some(Rolle::Authority | Rolle::Court | Rolle::CriminalAuthority)
        );
        v.push(
            if behoerde { AUSLAND_BEHOERDE } else { AUSLAND }
                .in_sprache(e)
                .to_string(),
        );
    }
    v
}

/// Die Zeilen «Beachten» (docs/AUSGABE.md):
///
/// * Ohne Funde: Zusammenhang und Geschäftsgeheimnisse prüft promptscan nicht; was es
///   prüft, zeigt `promptscan rules`.
/// * Bei `authority`, `court` und `criminal-authority` mit Fund über HINWEIS: welches
///   Datenschutzrecht gilt (Art. 2 DSG).
fn beachten(b: &Bericht, e: bool) -> Vec<String> {
    if b.funde.is_empty() {
        return vec![
            BLINDER_FLECK.in_sprache(e).to_string(),
            WAS_GEPRUEFT.in_sprache(e).to_string(),
        ];
    }
    let mut v = Vec::new();
    let behoerde = matches!(
        b.lage.rolle(),
        Some(Rolle::Authority | Rolle::Court | Rolle::CriminalAuthority)
    );
    if behoerde && b.funde.iter().any(|f| f.stufe != Stufe::Hinweis) {
        v.push(DATENSCHUTZ_BEHOERDE.in_sprache(e).to_string());
    }
    v
}

/// Die Befehle der Zeile «Mehr»: `explain` zur höchsten Stufe, dann zu den Regeln, deren
/// Begründung im Bericht nur kurz steht (Art. 5 lit. c DSG und Hinweise).
fn mehr(b: &Bericht, e: bool) -> Vec<String> {
    let lokal = b
        .funde
        .iter()
        .any(|f| matches!(f.grund, Grund::Lokal { .. }));
    let mut v = Vec::new();
    if let Some(h) = b.hoechste()
        && (h != Stufe::Hinweis || lokal)
    {
        v.push(format!("promptscan explain {}", stufe_name(h, e)));
    }
    let mut regeln: Vec<&str> = Vec::new();
    for s in STUFEN {
        for f in b.funde.iter().filter(|f| f.stufe == s) {
            let regel = match &f.grund {
                Grund::Lokal { .. } => continue,
                Grund::OhneIdentifikator => "personenbezug",
                Grund::BesondersSchuetzenswert { .. }
                | Grund::Geheimnis {
                    ziffer: Some(_), ..
                } => f.regel,
                _ if s == Stufe::Hinweis => f.regel,
                _ => continue,
            };
            if !regeln.contains(&regel) {
                regeln.push(regel);
            }
        }
    }
    v.extend(
        regeln
            .into_iter()
            .take(MEHR_REGELN)
            .map(|r| format!("promptscan explain {r}")),
    );
    v
}

/// Schreibt ein Zeichen der Eingabe für die Zeilenansicht und gibt zurück, wie viele
/// Spalten es belegt. Tabulatoren werden zu Leerzeichen bis zum nächsten Stopp;
/// Steuerzeichen und Bidi-Steuerzeichen werden sichtbar ersetzt (docs/AUSGABE.md).
fn zeichen_anzeigen(c: char, spalte: usize, aus: &mut String) -> usize {
    if c == '\t' {
        let n = TAB_BREITE - spalte % TAB_BREITE;
        aus.extend(std::iter::repeat_n(' ', n));
        return n;
    }
    if c.is_control() || ist_bidi_steuerzeichen(c) {
        aus.push(ERSATZ);
        return 1;
    }
    aus.push(c);
    zeichenbreite(c)
}

/// Bidi-Steuerzeichen (General_Category Cf): Sie ordnen die Anzeige im Terminal um und
/// könnten Text verdecken; die Zeilenansicht ersetzt sie darum sichtbar.
fn ist_bidi_steuerzeichen(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// Spaltenbreite eines Zeichens im Terminal, nach den Regeln von Markus Kuhn,
/// «wcwidth.c» (2007), mit den Unicode-Daten 18.0.0 (docs/AUSGABE.md):
/// U+00AD eine Spalte; Marks und Formatzeichen keine; East_Asian_Width W und F zwei;
/// alle übrigen eine.
fn zeichenbreite(c: char) -> usize {
    if c == '\u{00AD}' {
        1
    } else if is_combining_mark(c) || in_bereichen(c, NULLBREIT) {
        0
    } else if in_bereichen(c, BREIT) {
        2
    } else {
        1
    }
}

fn in_bereichen(c: char, bereiche: &[(u32, u32)]) -> bool {
    let c = u32::from(c);
    bereiche
        .binary_search_by(|&(von, bis)| {
            if bis < c {
                Ordering::Less
            } else if von > c {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// Formatzeichen ohne Breite: General_Category Cf aus DerivedGeneralCategory-18.0.0.txt
/// ohne U+00AD und ohne die Bidi-Steuerzeichen; dazu U+1160–U+11FF (Hangul Jamo,
/// Vokale und Schlusskonsonanten) nach Kuhn.
///
/// Diese und die folgende Tabelle sind aus Unicode-Datendateien abgeleitet (Unicode
/// License V3; der verlangte Hinweis steht in docs/AUSGABE.md, Abschnitt 7).
const NULLBREIT: &[(u32, u32)] = &[
    (0x0600, 0x0605),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x1160, 0x11FF),
    (0x180E, 0x180E),
    (0x200B, 0x200D),
    (0x2060, 0x2064),
    (0x206A, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// Zeichen mit East_Asian_Width W oder F (UAX #11) aus EastAsianWidth-18.0.0.txt,
/// zu 126 Bereichen zusammengefasst. Sie belegen im Terminal zwei Spalten.
const BREIT: &[(u32, u32)] = &[
    (0x1100, 0x115F),
    (0x231A, 0x231B),
    (0x2329, 0x232A),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2630, 0x2637),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x268A, 0x268F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x2E80, 0x2E99),
    (0x2E9B, 0x2EF3),
    (0x2F00, 0x2FD5),
    (0x2FF0, 0x303E),
    (0x3041, 0x3096),
    (0x3099, 0x30FF),
    (0x3105, 0x312F),
    (0x3131, 0x318E),
    (0x3190, 0x31E5),
    (0x31EF, 0x321E),
    (0x3220, 0x3247),
    (0x3250, 0xA48C),
    (0xA490, 0xA4C6),
    (0xA960, 0xA97C),
    (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF),
    (0xFE10, 0xFE19),
    (0xFE30, 0xFE52),
    (0xFE54, 0xFE66),
    (0xFE68, 0xFE6B),
    (0xFF01, 0xFF60),
    (0xFFE0, 0xFFE6),
    (0x16FE0, 0x16FE4),
    (0x16FF0, 0x16FF6),
    (0x17000, 0x18CDA),
    (0x18CFF, 0x18D20),
    (0x18D80, 0x18DF2),
    (0x18E00, 0x19191),
    (0x191A0, 0x191D2),
    (0x1AFF0, 0x1AFF3),
    (0x1AFF5, 0x1AFFB),
    (0x1AFFD, 0x1AFFE),
    (0x1B000, 0x1B128),
    (0x1B132, 0x1B132),
    (0x1B150, 0x1B152),
    (0x1B155, 0x1B155),
    (0x1B164, 0x1B168),
    (0x1B170, 0x1B2FB),
    (0x1D300, 0x1D356),
    (0x1D360, 0x1D376),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F1AE, 0x1F1AE),
    (0x1F200, 0x1F202),
    (0x1F210, 0x1F23B),
    (0x1F240, 0x1F248),
    (0x1F250, 0x1F251),
    (0x1F260, 0x1F265),
    (0x1F300, 0x1F320),
    (0x1F32D, 0x1F335),
    (0x1F337, 0x1F37C),
    (0x1F37E, 0x1F393),
    (0x1F3A0, 0x1F3CA),
    (0x1F3CF, 0x1F3D3),
    (0x1F3E0, 0x1F3F0),
    (0x1F3F4, 0x1F3F4),
    (0x1F3F8, 0x1F43E),
    (0x1F440, 0x1F440),
    (0x1F442, 0x1F4FC),
    (0x1F4FF, 0x1F53D),
    (0x1F54B, 0x1F54E),
    (0x1F550, 0x1F567),
    (0x1F57A, 0x1F57A),
    (0x1F595, 0x1F596),
    (0x1F5A4, 0x1F5A4),
    (0x1F5FB, 0x1F64F),
    (0x1F680, 0x1F6C5),
    (0x1F6CC, 0x1F6CC),
    (0x1F6D0, 0x1F6D2),
    (0x1F6D5, 0x1F6D9),
    (0x1F6DC, 0x1F6DF),
    (0x1F6EB, 0x1F6EC),
    (0x1F6F4, 0x1F6FC),
    (0x1F7DA, 0x1F7DA),
    (0x1F7E0, 0x1F7EB),
    (0x1F7F0, 0x1F7F0),
    (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945),
    (0x1F947, 0x1F9FF),
    (0x1FA70, 0x1FA7C),
    (0x1FA80, 0x1FAC6),
    (0x1FAC8, 0x1FAC8),
    (0x1FACC, 0x1FADD),
    (0x1FADF, 0x1FAEB),
    (0x1FAEF, 0x1FAFA),
    (0x20000, 0x2FFFD),
    (0x30000, 0x3FFFD),
];

/// Die Stelle auf einer Zeile, für die Gruppen und die Kopfzeile. Ein Zeilenumbruch in
/// der Stelle steht mit dem Leerraum um ihn als ein Leerzeichen da («Meine⏎Mandantin»
/// wird «Meine Mandantin»); liegt eine Leerzeile dazwischen, die erste Zeile und « …».
/// Tabulatoren als Leerzeichen, Steuerzeichen als U+FFFD.
fn stelle_einzeilig(stelle: &str) -> String {
    let teile: Vec<&str> = stelle.split('\n').collect();
    let n = teile.len();
    let leerzeile = n > 2 && teile[1..n - 1].iter().any(|z| z.trim().is_empty());
    let einzeilig = if n == 1 {
        stelle.to_string()
    } else if leerzeile {
        format!("{} …", teile[0].trim_end())
    } else {
        teile
            .iter()
            .enumerate()
            .map(|(i, z)| {
                let z = if i > 0 { z.trim_start() } else { z };
                if i + 1 < n { z.trim_end() } else { z }
            })
            .filter(|z| !z.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    einzeilig
        .chars()
        .map(|c| {
            if c == '\t' {
                ' '
            } else if c.is_control() || ist_bidi_steuerzeichen(c) {
                ERSATZ
            } else {
                c
            }
        })
        .collect()
}

/// Kurzbeschreibung der Lage für Kopfzeilen.
pub fn lage_text(lage: &Lage, englisch: bool) -> String {
    match lage {
        Lage::Keine => String::new(),
        Lage::Angegeben { rolle } => format!("--role {}", rolle.name()),
        Lage::AusText {
            rolle: Rolle::Offen,
            stelle,
            ..
        } => {
            if englisch {
                format!("profession open after «{stelle}»")
            } else {
                format!("Beruf offen nach «{stelle}»")
            }
        }
        Lage::AusText { rolle, stelle, .. } => {
            if englisch {
                format!("role {} inferred from «{stelle}»", rolle.name())
            } else {
                format!("Rolle {} erschlossen aus «{stelle}»", rolle.name())
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Meldungen zur Eingabe
// ---------------------------------------------------------------------------------------

/// So viele verschiedene Formatzeichen nennt die Meldung, danach «…».
const FORMATZEICHEN_GENANNT: usize = 4;

/// So viele Zeichen vor und nach dem «?» zeigt die Meldung zur ASCII-Pipe höchstens.
const PIPE_UMGEBUNG: usize = 15;

/// Damit schickt Windows PowerShell 5.1 Text als UTF-8 ohne BOM an ein Programm statt als
/// ASCII.
pub const PS_KODIERUNG: &str = "$OutputEncoding = [Text.UTF8Encoding]::new($false)";

/// Was an der Eingabe selbst auffällt (docs/AUSGABE.md, «Meldungen zur Eingabe»):
/// unsichtbare Formatzeichen, Ersatzzeichen U+FFFD und der Verdacht, dass eine Pipe
/// Zeichen ausserhalb von ASCII zu «?» gemacht hat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Eingabe {
    /// Zahl der unsichtbaren Formatzeichen ([`Text::unsichtbare_formatzeichen`]).
    pub unsichtbare_formatzeichen: usize,
    /// Die verschiedenen Formatzeichen in der Reihenfolge des Textes; höchstens eines mehr
    /// als `FORMATZEICHEN_GENANNT`.
    pub formatzeichen: Vec<char>,
    /// Zahl der Ersatzzeichen U+FFFD.
    pub ersatzzeichen: usize,
    /// Bei Verdacht auf eine ASCII-Pipe das erste verdächtige Wort
    /// ([`Text::ascii_pipe_wort`]), gekürzt und mit Steuerzeichen als U+FFFD.
    pub ascii_pipe: Option<String>,
}

impl Eingabe {
    pub fn von(text: &Text) -> Eingabe {
        let gefunden = text.unsichtbare_formatzeichen();
        let mut formatzeichen = Vec::new();
        for &(_, c) in &gefunden {
            if formatzeichen.len() > FORMATZEICHEN_GENANNT {
                break;
            }
            if !formatzeichen.contains(&c) {
                formatzeichen.push(c);
            }
        }
        Eingabe {
            unsichtbare_formatzeichen: gefunden.len(),
            formatzeichen,
            ersatzzeichen: text.ersatzzeichen(),
            ascii_pipe: text.ascii_pipe_wort().map(pipe_wort_zeigen),
        }
    }

    /// Alle Meldungen, je eine Zeile; leer, wenn nichts auffällt.
    pub fn meldungen(&self, englisch: bool) -> Vec<String> {
        let mut v = Vec::new();
        let n = self.unsichtbare_formatzeichen;
        if n > 0 {
            let mut liste: Vec<String> = self
                .formatzeichen
                .iter()
                .take(FORMATZEICHEN_GENANNT)
                .map(|&c| format!("U+{:04X}", u32::from(c)))
                .collect();
            if self.formatzeichen.len() > FORMATZEICHEN_GENANNT {
                liste.push("…".to_string());
            }
            let liste = liste.join(", ");
            v.push(match (englisch, n) {
                (false, 1) => format!("Eingabe enthält 1 unsichtbares Formatzeichen ({liste})"),
                (false, _) => format!("Eingabe enthält {n} unsichtbare Formatzeichen ({liste})"),
                (true, 1) => format!("Input contains 1 invisible format character ({liste})"),
                (true, _) => format!("Input contains {n} invisible format characters ({liste})"),
            });
        }
        v.extend(self.kodierung_meldungen(englisch));
        v
    }

    /// Nur die Meldungen, die auf eine beschädigte Kodierung deuten. `main.rs` schreibt sie
    /// zusätzlich als Warnung auf stderr.
    pub fn kodierung_meldungen(&self, englisch: bool) -> Vec<String> {
        let mut v = Vec::new();
        let n = self.ersatzzeichen;
        if n > 0 {
            v.push(match (englisch, n) {
                (false, _) => format!(
                    "Eingabe enthält {n} Ersatzzeichen U+FFFD: Beim Umwandeln der Kodierung \
                     gingen vermutlich Zeichen verloren; Funde können fehlen"
                ),
                (true, 1) => "Input contains 1 replacement character U+FFFD: characters were \
                              probably lost in an encoding conversion; findings may be missing"
                    .to_string(),
                (true, _) => format!(
                    "Input contains {n} replacement characters U+FFFD: characters were \
                     probably lost in an encoding conversion; findings may be missing"
                ),
            });
        }
        if let Some(wort) = &self.ascii_pipe {
            if englisch {
                v.push(format!(
                    "Input is ASCII only but has «?» inside a word («{wort}»): a pipe probably \
                     turned umlauts and accents into «?»; findings may be missing"
                ));
                v.push(format!(
                    "Windows PowerShell 5.1: first set {PS_KODIERUNG} or pass the prompt as a file"
                ));
            } else {
                v.push(format!(
                    "Eingabe nur in ASCII, aber mit «?» im Wort («{wort}»): Vermutlich hat eine \
                     Pipe Umlaute und Akzente zu «?» gemacht; Funde können fehlen"
                ));
                v.push(format!(
                    "Windows PowerShell 5.1: vorher {PS_KODIERUNG} setzen oder den Prompt als \
                     Datei übergeben"
                ));
            }
        }
        v
    }
}

/// Das verdächtige Wort für die Meldung: höchstens [`PIPE_UMGEBUNG`] Zeichen vor und nach
/// dem ersten «?» zwischen Buchstaben, gekürzt mit «…», Steuerzeichen als U+FFFD. Das Wort
/// ist ASCII (siehe [`Text::ascii_pipe_wort`]).
fn pipe_wort_zeigen(wort: &str) -> String {
    let fragezeichen = crate::text::fragezeichen_im_wort(wort).unwrap_or(0);
    let von = fragezeichen.saturating_sub(PIPE_UMGEBUNG);
    let bis = (fragezeichen + 1 + PIPE_UMGEBUNG).min(wort.len());
    let mut aus = String::new();
    if von > 0 {
        aus.push('…');
    }
    aus.extend(
        wort.get(von..bis)
            .unwrap_or(wort)
            .chars()
            .map(|c| if c.is_control() { ERSATZ } else { c }),
    );
    if bis < wort.len() {
        aus.push('…');
    }
    aus
}

#[derive(Serialize)]
struct EingabeJson {
    unsichtbare_formatzeichen: usize,
    ersatzzeichen: usize,
    ascii_pipe: Option<String>,
    meldungen: ListeJson,
}

fn eingabe_json(text: &Text) -> EingabeJson {
    let e = Eingabe::von(text);
    EingabeJson {
        unsichtbare_formatzeichen: e.unsichtbare_formatzeichen,
        ersatzzeichen: e.ersatzzeichen,
        meldungen: ListeJson {
            de: e.meldungen(false),
            en: e.meldungen(true),
        },
        ascii_pipe: e.ascii_pipe,
    }
}

#[cfg(test)]
mod meldungen_zur_eingabe_tests {
    use super::*;

    #[test]
    fn langes_wort_wird_gekuerzt_und_bereinigt() {
        let wort = format!("{}Anw?ltin\u{1B}[31m{}", "x".repeat(40), "y".repeat(40));
        let text = Text::neu(&format!("Siehe {wort} dort."));
        let e = Eingabe::von(&text);
        assert_eq!(
            e.ascii_pipe.as_deref(),
            Some("…xxxxxxxxxxxxAnw?ltin\u{FFFD}[31myyyyyy…")
        );
        assert_eq!(
            Eingabe::von(&Text::neu("Anw?ltin")).ascii_pipe.as_deref(),
            Some("Anw?ltin")
        );
    }

    #[test]
    fn nichts_auffaellig_keine_meldung() {
        let e = Eingabe::von(&Text::neu("Fasse den Absatz kürzer."));
        assert_eq!(e, Eingabe::default());
        assert!(e.meldungen(false).is_empty() && e.meldungen(true).is_empty());
    }

    #[test]
    fn formatzeichen_werden_genannt() {
        let e = Eingabe::von(&Text::neu(
            "a\u{AD}b\u{AD}c\u{200B}d\u{200C}e\u{2060}f\u{FEFF}g\u{2061}h",
        ));
        assert_eq!(e.unsichtbare_formatzeichen, 7);
        assert_eq!(
            e.formatzeichen,
            ['\u{AD}', '\u{200B}', '\u{200C}', '\u{2060}', '\u{FEFF}']
        );
        assert_eq!(
            e.meldungen(false),
            ["Eingabe enthält 7 unsichtbare Formatzeichen (U+00AD, U+200B, U+200C, U+2060, …)"]
        );
        assert!(e.kodierung_meldungen(false).is_empty());
    }
}

// ---------------------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------------------

/// Ein Text in beiden Ausgabesprachen.
#[derive(Serialize)]
struct ZweiJson<'a> {
    de: &'a str,
    en: &'a str,
}

fn zwei(t: &Txt) -> ZweiJson<'static> {
    ZweiJson { de: t.de, en: t.en }
}

#[derive(Serialize)]
struct ListeJson {
    de: Vec<String>,
    en: Vec<String>,
}

#[derive(Serialize)]
struct BerichtJson<'a> {
    schema_version: u32,
    dokument: &'static str,
    programm: &'static str,
    version: &'static str,
    optionen: OptionenJson,
    lage: LageJson<'a>,
    funde: Vec<FundJson<'a>>,
    anzahl: AnzahlJson,
    hoechste_stufe: Option<Stufe>,
    eingabe: EingabeJson,
}

#[derive(Serialize)]
struct OptionenJson {
    rolle: Option<Rolle>,
    /// Woher `rolle` stammt (Kopfzeile): keine, option, konfiguration.
    rolle_quelle: RolleQuelle,
    ziel: Ziel,
}

#[derive(Serialize)]
#[serde(tag = "herkunft", rename_all = "kebab-case")]
enum LageJson<'a> {
    Keine,
    Angegeben {
        rolle: Rolle,
    },
    AusText {
        rolle: Rolle,
        stelle: &'a str,
        zeile: usize,
        spalte: usize,
    },
}

#[derive(Serialize)]
struct FundJson<'a> {
    regel: &'a str,
    stufe: Stufe,
    zeile: usize,
    spalte: usize,
    start_zeichen: usize,
    ende_zeichen: usize,
    stelle: &'a str,
    zusatz: Option<ZweiJson<'a>>,
    grund: GrundJson<'a>,
    normen: Vec<String>,
    erklaerung: ListeJson,
    beleg: Option<BelegJson<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
enum GrundJson<'a> {
    Personendaten,
    BesondersSchuetzenswert {
        ziffer: u8,
    },
    Geheimnis {
        rolle: Rolle,
        aus_text: Option<&'a str>,
        ziffer: Option<u8>,
    },
    OhneIdentifikator,
    Geheimniskontext {
        rolle: Rolle,
    },
    Lokal {
        sonst: Stufe,
    },
    Regelhinweis,
}

#[derive(Serialize)]
struct BelegJson<'a> {
    datei: &'a str,
    begriff: &'a str,
    quelle: &'a str,
    fassung: &'a str,
    url: &'a str,
}

#[derive(Serialize)]
struct AnzahlJson {
    gesamt: usize,
    rot: usize,
    orange: usize,
    gelb: usize,
    hinweis: usize,
}

fn lage_json(lage: &Lage) -> LageJson<'_> {
    match lage {
        Lage::Keine => LageJson::Keine,
        Lage::Angegeben { rolle } => LageJson::Angegeben { rolle: *rolle },
        Lage::AusText {
            rolle,
            stelle,
            zeile,
            spalte,
        } => LageJson::AusText {
            rolle: *rolle,
            stelle,
            zeile: *zeile,
            spalte: *spalte,
        },
    }
}

fn grund_json(g: &Grund) -> GrundJson<'_> {
    match g {
        Grund::Personendaten => GrundJson::Personendaten,
        Grund::BesondersSchuetzenswert { ziffer } => {
            GrundJson::BesondersSchuetzenswert { ziffer: *ziffer }
        }
        Grund::Geheimnis {
            rolle,
            aus_text,
            ziffer,
            ..
        } => GrundJson::Geheimnis {
            rolle: *rolle,
            aus_text: aus_text.as_deref(),
            ziffer: *ziffer,
        },
        Grund::OhneIdentifikator => GrundJson::OhneIdentifikator,
        Grund::Geheimniskontext { rolle } => GrundJson::Geheimniskontext { rolle: *rolle },
        Grund::Lokal { sonst } => GrundJson::Lokal { sonst: *sonst },
        Grund::Regelhinweis => GrundJson::Regelhinweis,
    }
}

fn fund_json(f: &Fund) -> FundJson<'_> {
    FundJson {
        regel: f.regel,
        stufe: f.stufe,
        zeile: f.zeile,
        spalte: f.spalte,
        start_zeichen: f.start_zeichen,
        ende_zeichen: f.ende_zeichen,
        stelle: &f.stelle,
        zusatz: f.zusatz.as_ref().map(|z| ZweiJson {
            de: &z.de,
            en: &z.en,
        }),
        grund: grund_json(&f.grund),
        normen: normzitate(f),
        erklaerung: ListeJson {
            de: grund_zeilen(f, false),
            en: grund_zeilen(f, true),
        },
        beleg: f.beleg.as_ref().map(|b| BelegJson {
            datei: &b.datei,
            begriff: &b.begriff,
            quelle: &b.quelle,
            fassung: &b.fassung,
            url: &b.url,
        }),
    }
}

fn als_json<T: Serialize>(wert: &T) -> String {
    serde_json::to_string_pretty(wert).expect("Ausgabe als JSON")
}

/// Der Bericht als JSON, Schema-Version [`SCHEMA_VERSION`] (docs/AUSGABE.md).
pub fn bericht_json(b: &Bericht) -> String {
    als_json(&BerichtJson {
        schema_version: SCHEMA_VERSION,
        dokument: "bericht",
        programm: "promptscan",
        version: env!("CARGO_PKG_VERSION"),
        optionen: OptionenJson {
            rolle: b.optionen.rolle,
            rolle_quelle: b.rolle_quelle,
            ziel: b.optionen.ziel,
        },
        lage: lage_json(&b.lage),
        funde: b.funde.iter().map(fund_json).collect(),
        anzahl: AnzahlJson {
            gesamt: b.funde.len(),
            rot: b.anzahl(Stufe::Rot),
            orange: b.anzahl(Stufe::Orange),
            gelb: b.anzahl(Stufe::Gelb),
            hinweis: b.anzahl(Stufe::Hinweis),
        },
        hoechste_stufe: b.hoechste(),
        eingabe: eingabe_json(&b.text),
    })
}

#[derive(Serialize)]
struct BerichtKurzJson {
    schema_version: u32,
    dokument: &'static str,
    programm: &'static str,
    version: &'static str,
    anzahl: AnzahlJson,
    hoechste_stufe: Option<Stufe>,
}

/// Kurzfassung als JSON für `--quiet --format json`: Anzahl je Stufe und höchste Stufe.
pub fn bericht_kurz_json(b: &Bericht) -> String {
    als_json(&BerichtKurzJson {
        schema_version: SCHEMA_VERSION,
        dokument: "bericht-kurz",
        programm: "promptscan",
        version: env!("CARGO_PKG_VERSION"),
        anzahl: AnzahlJson {
            gesamt: b.funde.len(),
            rot: b.anzahl(Stufe::Rot),
            orange: b.anzahl(Stufe::Orange),
            gelb: b.anzahl(Stufe::Gelb),
            hinweis: b.anzahl(Stufe::Hinweis),
        },
        hoechste_stufe: b.hoechste(),
    })
}

// ---------------------------------------------------------------------------------------
// Fliesstext für rules und explain
// ---------------------------------------------------------------------------------------

thread_local! {
    /// Breite und ASCII-Wahl für den Fliesstext von `rules` und `explain`; gesetzt von
    /// [`fliesstext`] für die Dauer einer Ausgabe.
    static FLIESSTEXT: std::cell::Cell<(usize, bool)> =
        const { std::cell::Cell::new((ZEILENBREITE, false)) };
}

/// Erzeugt eine Ausgabe von `rules` oder `explain` mit der Breite von `d` (höchstens
/// [`ZEILENBREITE`]) für [`absatz`]; mit `--ascii` wird das Ergebnis zuletzt ganz
/// umgeschrieben.
fn fliesstext(d: &Darstellung, f: impl FnOnce() -> String) -> String {
    let alt = FLIESSTEXT.replace((breite_von(d).min(ZEILENBREITE), d.ascii));
    let aus = f();
    FLIESSTEXT.set(alt);
    ausgeben(&aus, d)
}

/// Die Darstellung, mit der [`absatz`] gerade umbricht.
fn fliesstext_darstellung() -> (usize, Darstellung) {
    let (breite, ascii) = FLIESSTEXT.get();
    (
        breite,
        Darstellung {
            ascii,
            ..Darstellung::default()
        },
    )
}

/// Bricht `text` an Leerzeichen auf die Breite von [`fliesstext`] um. Die erste Zeile
/// beginnt mit `anfang`, jede weitere mit `folgeeinzug` Leerzeichen. Normzitate bleiben
/// zusammen; ein Wort, das allein zu lang ist, wird getrennt.
fn absatz(aus: &mut String, text: &str, anfang: &str, folgeeinzug: usize) {
    let (breite, d) = fliesstext_darstellung();
    let anfang = ausgeben(anfang, &d);
    let ab = spalten(&anfang);
    let zeilen = umbrechen(
        stuecke(text, &d),
        breite.saturating_sub(ab),
        breite.saturating_sub(folgeeinzug),
    );
    for (i, z) in zeilen.iter().enumerate() {
        if i == 0 {
            let _ = writeln!(aus, "{}", format!("{anfang}{z}").trim_end());
        } else {
            let _ = writeln!(aus, "{:folgeeinzug$}{z}", "");
        }
    }
}

/// Wie [`absatz`], mit einer Beschriftung in den ersten zehn Spalten («Grund», «Wann» …).
fn feld(aus: &mut String, beschriftung: &str, text: &str) {
    let (_, d) = fliesstext_darstellung();
    let beschriftung = ausgeben(beschriftung, &d);
    let anfang = format!("{beschriftung:SP_TEXT$}");
    absatz(aus, text, &anfang, SP_TEXT);
}

fn ueberschrift(aus: &mut String, titel: &str, d: &Darstellung) {
    let _ = writeln!(aus, "\n{}", fett(titel, d));
}

fn klassenname(k: Klasse, e: bool) -> &'static str {
    match (k, e) {
        (Klasse::A, false) => "formal prüfbar",
        (Klasse::A, true) => "formally verifiable",
        (Klasse::B, false) => "Wortliste oder Register",
        (Klasse::B, true) => "word list or register",
        (Klasse::C, false) => "besonders schützenswerte Personendaten",
        (Klasse::C, true) => "sensitive personal data",
        (Klasse::Hinweis, false) => "Hinweis; die Rechtsfolge beurteilt promptscan nicht",
        (Klasse::Hinweis, true) => "note; promptscan does not assess the legal consequence",
        (Klasse::Kontext, false) => "Kontext: Geheimnis-Kontext oder Personenbezug",
        (Klasse::Kontext, true) => "context: secrecy context or reference to a person",
    }
}

/// Eine Norm mit Zitat, Erlass, Fassung, amtlichem Wortlaut und URL.
fn norm_block(aus: &mut String, n: &Norm, einzug: usize, e: bool) {
    let kopf = format!(
        "{} — {}, {} {}",
        n.zitat,
        n.erlass,
        if e { "version of" } else { "Fassung" },
        n.fassung
    );
    absatz(aus, &kopf, &" ".repeat(einzug), einzug + 2);
    absatz(
        aus,
        &format!("«{}»", n.wortlaut),
        &" ".repeat(einzug + 2),
        einzug + 2,
    );
    let _ = writeln!(aus, "{:w$}{}", "", n.url, w = einzug + 2);
}

fn stand_text(stand: Stand, e: bool) -> String {
    match (stand, e) {
        (Stand::Geprueft(datum), false) => format!("geprüft am {datum}"),
        (Stand::Geprueft(datum), true) => format!("checked on {datum}"),
        (Stand::ZuPruefen(was), false) => format!("zu prüfen: {was}"),
        (Stand::ZuPruefen(was), true) => format!("to be checked: {was}"),
    }
}

fn quelle_block(aus: &mut String, q: &Quelle, e: bool) {
    absatz(aus, q.titel, "  ", 4);
    absatz(aus, &stand_text(q.stand, e), "    ", 4);
    if let Some(lizenz) = q.lizenz {
        let wort = if e { "Licence" } else { "Lizenz" };
        absatz(aus, &format!("{wort}: {lizenz}"), "    ", 6);
    }
    if !q.url.is_empty() {
        let _ = writeln!(aus, "    {}", q.url);
    }
}

/// Literatur zu einem Streitstand.
struct Literatur {
    zitat: &'static str,
    url: Option<&'static str>,
    stand: Txt,
}

/// Ein Punkt «Was promptscan nicht entscheidet»: der volle Text für `explain <STUFE>`, die
/// Kurzform für `explain <REGEL>`, die Normen, deren Wortlaut `explain <STUFE>` unter
/// «Normen» zeigt, und Literatur.
struct Abschnitt {
    text: Txt,
    kurz: Txt,
    normen: &'static [&'static Norm],
    literatur: &'static [Literatur],
}

const NOCH_NICHT_GELESEN: Txt = Txt::neu("noch nicht gelesen", "not yet read");

/// Bibliografische Angaben bestätigt in der Bibliografie der Konferenz «Les solutions
/// Cloud dans les études d'avocats» (Ordre des avocats de Genève, 05.12.2019);
/// der Beitrag selbst ist noch nicht gelesen.
const CHAPPUIS_ALBERINI: Literatur = Literatur {
    zitat: "Chappuis/Alberini, Secret professionnel de l’avocat et solutions cloud, Anwaltsrevue 8/2017, 337 ff.",
    url: None,
    stand: NOCH_NICHT_GELESEN,
};

/// Titelseite und Zusammenfassung am 06.11.2026 gesehen; der Volltext ist noch nicht
/// gelesen.
const SCHWARZENEGGER_U_A: Literatur = Literatur {
    zitat: "Schwarzenegger/Thouvenin/Stiller/George, Nutzung von Cloud-Diensten durch Anwältinnen und Anwälte, Anwaltsrevue 1/2019, 25 ff.",
    url: Some(
        "https://www.ius.uzh.ch/dam/jcr:d8363d82-0129-47cc-834a-948e871f0fee/2019%20-%20Schwarzenegger%20Thouvenin%20Stiller%20George%20-%20Nutzung%20von%20Cloud-Diensten%20durch%20Anw%c3%a4ltinnen%20und%20Anw%c3%a4lte.pdf",
    ),
    stand: NOCH_NICHT_GELESEN,
};

/// Überschrift der Literatur im Punkt [`HILFSPERSON`]; beide Beiträge betreffen die
/// Anwaltschaft und sind noch nicht gelesen.
const LITERATUR_ANWALTSCHAFT: Txt = Txt::neu(
    "Literatur zur Anwaltschaft, noch nicht gelesen:",
    "Literature on lawyers, not yet read:",
);

// Die Punkte «Was promptscan nicht entscheidet». Wo ein Punkt erscheint, sagt
// [`nicht_entschieden`]; die Kurzform für `explain <REGEL>` sagt nur, worum es geht, und
// verweist auf den vollen Text.

/// Rechtfertigung (explain gelb, orange, hinweis, jede Regel). Art. 31 DSG steht wie
/// Art. 30 DSG im 5. Kapitel und gilt für private Personen.
const RECHTFERTIGUNG_DSG: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob eine Persönlichkeitsverletzung vorliegt (Art. 30 DSG) und ob sie gerechtfertigt ist: durch Einwilligung der betroffenen Person, ein überwiegendes privates oder öffentliches Interesse oder Gesetz (Art. 31 Abs. 1 DSG; gilt für private Personen).",
        "Whether there is a breach of personality rights (Art. 30 DSG) and whether it is justified: by the consent of the data subject, by an overriding private or public interest, or by the law (Art. 31 Abs. 1 DSG; applies to private persons).",
    ),
    kurz: Txt::neu(
        "Ob eine Persönlichkeitsverletzung vorliegt (Art. 30 DSG) und ob sie gerechtfertigt ist (Art. 31 Abs. 1 DSG).",
        "Whether there is a breach of personality rights (Art. 30 DSG) and whether it is justified (Art. 31 Abs. 1 DSG).",
    ),
    normen: &[&normen::DSG_30, &normen::DSG_31_1],
    literatur: &[],
};

/// Ausnahmen der Strafnormen (explain rot, hinweis, jede Regel).
const EINWILLIGUNG_STGB: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob eine Ausnahme greift: nach Art. 321 Ziff. 2 StGB die Einwilligung der berechtigten Person oder die schriftliche Bewilligung der vorgesetzten Behörde oder Aufsichtsbehörde, nach Art. 320 Ziff. 2 StGB die schriftliche Einwilligung der vorgesetzten Behörde; bei Art. 47 BankG, Art. 69 FINIG und Art. 62 DSG, ob die Person eingewilligt hat, die das Geheimnis betrifft.",
        "Whether an exception applies: under Art. 321 Ziff. 2 StGB, the consent of the person to whom the information pertains or written authorisation by a superior authority or supervisory authority; under Art. 320 Ziff. 2 StGB, the written consent of the superior authority; under Art. 47 BankG, Art. 69 FINIG and Art. 62 DSG, whether the person concerned consented.",
    ),
    kurz: Txt::neu(
        "Ob eine Ausnahme greift, etwa eine Einwilligung oder Bewilligung.",
        "Whether an exception applies, such as consent or authorisation.",
    ),
    normen: &[
        &normen::STGB_321_2,
        &normen::STGB_320_2,
        &normen::BANKG_47_1_A,
        &normen::FINIG_69_1_A,
        &normen::DSG_62_1,
    ],
    literatur: &[],
};

/// Der Anbieter als Hilfsperson (explain rot, hinweis, jede Regel). Nennt keinen
/// Meinungsstand: Die beiden Beiträge sind noch nicht gelesen.
const HILFSPERSON: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob die Norm der Rolle auch den KI-Anbieter erfasst: als Hilfsperson (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB), als Beauftragten (Art. 47 Abs. 1 lit. a BankG, Art. 69 Abs. 1 lit. a FINIG) oder als Person, die für Sie tätig ist (Art. 62 Abs. 2 DSG). Wann ein Anbieter dazu zählt und ob die Weitergabe an ihn dann ein Offenbaren ist, sagt der Wortlaut nicht. Anwältinnen und Anwälte sorgen für die Wahrung des Berufsgeheimnisses durch ihre Hilfspersonen (Art. 13 Abs. 2 BGFA). promptscan meldet ROT und entscheidet diese Fragen nicht.",
        "Whether the role’s provision also covers the AI provider: as an assistant or auxiliary (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB), as an agent (Art. 47 Abs. 1 lit. a BankG, Art. 69 Abs. 1 lit. a FINIG) or as a person carrying on an activity for you (Art. 62 Abs. 2 DSG). The wording does not say when a provider counts as such, nor whether passing data to it is then a disclosure. Lawyers ensure that their auxiliary staff preserve professional confidentiality (Art. 13 Abs. 2 BGFA). promptscan reports RED and does not decide these questions.",
    ),
    kurz: Txt::neu(
        "Ob die Norm der Rolle auch den KI-Anbieter erfasst, etwa als Hilfsperson oder Beauftragten (Literatur noch nicht gelesen).",
        "Whether the role’s provision also covers the AI provider, e.g. as an assistant or agent (literature not yet read).",
    ),
    normen: &[
        &normen::STGB_321_1,
        &normen::STGB_320_1,
        &normen::BANKG_47_1_A,
        &normen::FINIG_69_1_A,
        &normen::DSG_62_2,
        &normen::BGFA_13_2,
    ],
    literatur: &[CHAPPUIS_ALBERINI, SCHWARZENEGGER_U_A],
};

/// Auftragsbearbeitung, bei allen Stufen und jeder Regel.
const AUFTRAGSBEARBEITUNG: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist. Das hängt daran, ob er nur im Auftrag bearbeitet (Art. 5 lit. j und k DSG); ein Vertrag allein entscheidet das nicht. Nutzt der Anbieter Eingaben für eigene Zwecke, etwa zum Training, ist er insoweit Dritter. Übertragen werden darf nur, wenn er die Daten so bearbeitet, wie der Verantwortliche selbst es dürfte (Art. 9 Abs. 1 lit. a DSG), keine gesetzliche oder vertragliche Geheimhaltungspflicht es verbietet (Art. 9 Abs. 1 lit. b DSG) und der Verantwortliche sich vergewissert hat, dass der Anbieter die Datensicherheit gewährleisten kann (Art. 9 Abs. 2 DSG). Einem Dritten übertragen darf der Anbieter die Bearbeitung nur mit vorgängiger Genehmigung (Art. 9 Abs. 3 DSG). Private, die vorsätzlich die Datenbearbeitung einem Auftragsbearbeiter übergeben, ohne dass die Voraussetzungen von Art. 9 Abs. 1 und 2 DSG erfüllt sind, werden auf Antrag mit Busse bis zu 250\u{A0}000 Franken bestraft (Art. 61 lit. b DSG).",
        "Whether the AI provider is a processor or a third party. That depends on whether it processes only on behalf of the controller (Art. 5 lit. j und k DSG); a contract alone does not decide this. If the provider uses inputs for its own purposes, such as training, it is a third party to that extent. Processing may be assigned only if the data is processed only in the manner in which the controller itself is permitted to do it (Art. 9 Abs. 1 lit. a DSG), no statutory or contractual duty of confidentiality prohibits assignment (Art. 9 Abs. 1 lit. b DSG) and the controller has satisfied itself that the provider is able to guarantee data security (Art. 9 Abs. 2 DSG). The provider may assign processing to a third party only with prior approval (Art. 9 Abs. 3 DSG). On complaint, private persons who wilfully assign the data processing to a processor without satisfying the requirements of Art. 9 Abs. 1 und 2 DSG are liable to a fine not exceeding 250,000 francs (Art. 61 lit. b DSG).",
    ),
    kurz: Txt::neu(
        "Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist (Art. 9 DSG).",
        "Whether the AI provider is a processor or a third party (Art. 9 DSG).",
    ),
    normen: &[
        &normen::DSG_5_J_K,
        &normen::DSG_9_1_A,
        &normen::DSG_9_1_B,
        &normen::DSG_9_2,
        &normen::DSG_9_3,
        &normen::DSG_61_B,
    ],
    literatur: &[],
};

/// Ausland, bei allen Stufen und jeder Regel.
const AUSLAND_PUNKT: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob Daten ins Ausland gehen und wohin. Art. 16 Abs. 1 DSG lässt das zu, wenn der Bundesrat festgestellt hat, dass die Gesetzgebung des Staates einen angemessenen Schutz gewährleistet (Liste: Anhang 1 DSV). Für die USA gilt das nur bei Organisationen, die nach den Grundsätzen des Datenschutzrahmens zwischen der Schweiz und den USA zertifiziert sind (Anhang 1 Ziff. 44 DSV). Sonst muss ein geeigneter Datenschutz gewährleistet sein, etwa durch Standarddatenschutzklauseln, die der EDÖB vorgängig genehmigt, ausgestellt oder anerkannt hat (Art. 16 Abs. 2 lit. d DSG), oder eine Ausnahme greifen, etwa die ausdrückliche Einwilligung (Art. 17 Abs. 1 lit. a DSG). Private, die vorsätzlich Personendaten ins Ausland bekanntgeben, ohne dass eine dieser Voraussetzungen erfüllt ist, werden auf Antrag mit Busse bis zu 250\u{A0}000 Franken bestraft (Art. 61 lit. a DSG).",
        "Whether data goes abroad, and where. Art. 16 Abs. 1 DSG allows this if the Federal Council has decided that the legislation of the State concerned guarantees an adequate level of protection (list: Anhang 1 DSV). For the United States, this applies only to organisations certified under the Principles of the Swiss-US Privacy Framework (Anhang 1 Ziff. 44 DSV). Otherwise an appropriate level of data protection must be guaranteed, for example by standard data protection clauses that the FDPIC has approved, issued or recognised beforehand (Art. 16 Abs. 2 lit. d DSG), or an exception must apply, such as explicit consent (Art. 17 Abs. 1 lit. a DSG). On complaint, private persons who wilfully disclose personal data abroad without meeting these requirements are liable to a fine not exceeding 250,000 francs (Art. 61 lit. a DSG).",
    ),
    kurz: Txt::neu(
        "Ob Daten ins Ausland gehen und wohin (Art. 16 f. DSG).",
        "Whether data goes abroad, and where (Art. 16 f. DSG).",
    ),
    normen: &[
        &normen::DSG_16_1,
        &normen::DSG_16_2_D,
        &normen::DSV_ANHANG1_44,
        &normen::DSG_17_1_A,
        &normen::DSG_61_A,
    ],
    literatur: &[],
};

/// Geltungsbereich (explain gelb, orange, hinweis, jede Regel; mit Satz 2 von
/// Art. 2 Abs. 3 DSG). Explain rot sagt dasselbe im Grund.
const GELTUNGSBEREICH: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Welches Datenschutzrecht für Sie gilt. Die Begründung oben gilt für private Personen. Auf Daten, die eine natürliche Person ausschliesslich zum persönlichen Gebrauch bearbeitet, ist das DSG nicht anwendbar (Art. 2 Abs. 2 lit. a DSG). Bundesorgane brauchen für Bearbeitung und Bekanntgabe grundsätzlich eine gesetzliche Grundlage (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; Ausnahmen: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG). Für kantonale und kommunale Organe gilt nicht das DSG (Art. 2 Abs. 1 DSG), sondern kantonales Datenschutzrecht. In Gerichtsverfahren und in Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 Abs. 3 DSG).",
        "Which data protection law applies to you. The reasoning above applies to private persons. The DSG does not apply to personal data processed by a natural person exclusively for personal use (Art. 2 Abs. 2 lit. a DSG). Federal bodies in principle need a statutory basis for processing and disclosure (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; exceptions: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG). The DSG does not apply to cantonal and communal bodies (Art. 2 Abs. 1 DSG); cantonal data protection law does. In court proceedings and in proceedings governed by federal procedural regulations, procedural law applies; in first instance administrative proceedings, the DSG applies (Art. 2 Abs. 3 DSG).",
    ),
    kurz: Txt::neu(
        "Welches Datenschutzrecht für Sie gilt (Art. 2 DSG).",
        "Which data protection law applies to you (Art. 2 DSG).",
    ),
    normen: &[
        &normen::DSG_2_1,
        &normen::DSG_2_2_A,
        &normen::DSG_2_3,
        &normen::DSG_34_1,
        &normen::DSG_34_4,
        &normen::DSG_36_1,
        &normen::DSG_36_2,
    ],
    literatur: &[],
};

/// Ob eine Angabe geheim ist (explain rot, hinweis, jede Regel).
const GEHEIM: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Ob eine Angabe überhaupt geheim ist.",
        "Whether a detail is secret at all.",
    ),
    kurz: Txt::neu(
        "Ob eine Angabe überhaupt geheim ist.",
        "Whether a detail is secret at all.",
    ),
    normen: &[],
    literatur: &[],
};

/// Geschäftsgeheimnisse, bei allen Stufen und jeder Regel.
const GESCHAEFTSGEHEIMNISSE: Abschnitt = Abschnitt {
    text: Txt::neu(
        "Geschäfts- und Fabrikationsgeheimnisse und vertragliche Schweigepflichten (Art. 162 StGB; für Arbeitnehmende Art. 321a Abs. 4 OR) prüft promptscan nicht. Wenige oder keine Funde heissen nicht, dass der Prompt nichts Geheimes enthält.",
        "Manufacturing and trade secrets and contractual duties of confidentiality (Art. 162 StGB; for employees Art. 321a Abs. 4 OR) are not checked. Few or no findings do not mean that the prompt contains nothing secret.",
    ),
    kurz: Txt::neu(
        "Geschäfts- und Fabrikationsgeheimnisse und vertragliche Schweigepflichten (Art. 162 StGB; für Arbeitnehmende Art. 321a Abs. 4 OR).",
        "Manufacturing and trade secrets and contractual duties of confidentiality (Art. 162 StGB; for employees Art. 321a Abs. 4 OR).",
    ),
    normen: &[&normen::STGB_162, &normen::OR_321A_4],
    literatur: &[],
};

/// Alle Punkte: `explain hinweis` und `explain <REGEL>` zeigen sie alle, weil ein Fund dort
/// mit `--role` ROT werden kann.
const ALLE_PUNKTE: &[&Abschnitt] = &[
    &RECHTFERTIGUNG_DSG,
    &EINWILLIGUNG_STGB,
    &HILFSPERSON,
    &AUFTRAGSBEARBEITUNG,
    &AUSLAND_PUNKT,
    &GELTUNGSBEREICH,
    &GEHEIM,
    &GESCHAEFTSGEHEIMNISSE,
];

/// Was promptscan nicht entscheidet, je Stufe: GELB und ORANGE die Rechtfertigung nach DSG
/// und den Geltungsbereich, ROT die Ausnahmen der Strafnormen, den Anbieter als Hilfsperson
/// oder Beauftragten und ob etwas geheim ist; alle die Auftragsbearbeitung, das Ausland und
/// die Geschäftsgeheimnisse; HINWEIS alle Punkte.
fn nicht_entschieden(stufe: Stufe) -> &'static [&'static Abschnitt] {
    match stufe {
        Stufe::Gelb | Stufe::Orange => &[
            &RECHTFERTIGUNG_DSG,
            &AUFTRAGSBEARBEITUNG,
            &AUSLAND_PUNKT,
            &GELTUNGSBEREICH,
            &GESCHAEFTSGEHEIMNISSE,
        ],
        Stufe::Rot => &[
            &EINWILLIGUNG_STGB,
            &HILFSPERSON,
            &AUFTRAGSBEARBEITUNG,
            &AUSLAND_PUNKT,
            &GEHEIM,
            &GESCHAEFTSGEHEIMNISSE,
        ],
        Stufe::Hinweis => ALLE_PUNKTE,
    }
}

/// Ein Eintrag der Literatur: Zitat mit hängendem Einzug, darunter die Fundstelle.
fn literatur_block(aus: &mut String, l: &Literatur, einzug: usize) {
    absatz(aus, l.zitat, &" ".repeat(einzug), einzug + 2);
    if let Some(url) = l.url {
        let _ = writeln!(aus, "{:w$}{url}", "", w = einzug + 2);
    }
}

/// «Was promptscan nicht entscheidet» in `explain <STUFE>`: jeder Punkt voll, darunter die
/// Literatur; den Wortlaut der Normen zeigt der Abschnitt «Normen».
fn nicht_entschieden_text(aus: &mut String, stufe: Stufe, d: &Darstellung) {
    let e = d.englisch;
    ueberschrift(
        aus,
        if e {
            "What promptscan does not decide"
        } else {
            "Was promptscan nicht entscheidet"
        },
        d,
    );
    for a in nicht_entschieden(stufe) {
        absatz(aus, a.text.in_sprache(e), "  - ", 4);
        if !a.literatur.is_empty() {
            absatz(aus, LITERATUR_ANWALTSCHAFT.in_sprache(e), "      ", 6);
            for l in a.literatur {
                literatur_block(aus, l, 6);
            }
        }
    }
}

/// Regeln für besonders schützenswerte Personendaten (SPEC 5.3). Ihr JSON führt im Feld
/// `bedingung_dritte` den Punkt «Auftragsbearbeitung».
fn ist_art5c(r: &RegelInfo) -> bool {
    r.klasse == Klasse::C || r.id.starts_with("art5c-")
}

// ---------------------------------------------------------------------------------------
// rules
// ---------------------------------------------------------------------------------------

/// Die feste Zeile unter `rules` (und in `--help`).
const HILFSMITTEL: Txt = Txt::neu(
    "Hilfsmittel, keine Rechtsberatung · erkennt Identifikatoren und Wörter aus Listen, nicht den Zusammenhang · Güte noch nicht gemessen",
    "An aid, not legal advice · detects identifiers and listed words, not the context · quality not yet measured",
);

/// Einzug der Titel und Normen in `rules`: zwei Leerzeichen und die längste Kennung.
const RULES_EINZUG: usize = 16;

/// `promptscan rules` als Text: die Regeln nach Stufe, je Regel Kennung und Titel, darunter
/// der rechtliche Bezug und die Grundlage der Erkennung; zuletzt die Zeile «Hilfsmittel».
pub fn regeln_text(d: &Darstellung) -> String {
    fliesstext(d, || {
        let e = d.englisch;
        let (breite, dd) = fliesstext_darstellung();
        let mut aus = String::new();
        let _ = writeln!(
            aus,
            "{}",
            fett(
                if e {
                    "Rules by level"
                } else {
                    "Regeln nach Stufe"
                },
                d
            )
        );
        let alle = katalog::alle();
        for stufe in [Stufe::Gelb, Stufe::Orange, Stufe::Rot, Stufe::Hinweis] {
            let regeln: Vec<&RegelInfo> = alle.iter().filter(|r| regel_stufe(r) == stufe).collect();
            if regeln.is_empty() {
                continue;
            }
            aus.push('\n');
            let titel = match stufe {
                Stufe::Rot => TITEL_ROT,
                Stufe::Orange => BESONDERS,
                Stufe::Gelb => PERSONENDATEN,
                Stufe::Hinweis => NICHT_BEURTEILT,
            };
            let (p, pb) = plakette(stufe, d);
            zeilen_schreiben(
                &mut aus,
                &p,
                pb,
                1,
                SP_TEXT,
                stuecke(titel.in_sprache(e), &dd),
                breite,
            );
            for r in regeln {
                let anfang = format!("  {}", r.id);
                zeilen_schreiben(
                    &mut aus,
                    &anfang,
                    spalten(&anfang),
                    1,
                    RULES_EINZUG,
                    stuecke(r.titel.in_sprache(e), &dd),
                    breite,
                );
                let (bezug, erkennung): (Vec<&'static Norm>, Vec<&'static Norm>) = r
                    .grundlage
                    .iter()
                    .copied()
                    .partition(|n| normen::ist_bezug(n));
                let einzug = " ".repeat(RULES_EINZUG);
                if !bezug.is_empty() {
                    let wort = if e { "Reference:" } else { "Bezug:" };
                    absatz(
                        &mut aus,
                        &format!("{wort} {}", zitate_kurz(&bezug)),
                        &einzug,
                        RULES_EINZUG,
                    );
                }
                if !erkennung.is_empty() {
                    let wort = if e { "Detection:" } else { "Erkennung:" };
                    absatz(
                        &mut aus,
                        &format!("{wort} {}", zitate_kurz(&erkennung)),
                        &einzug,
                        RULES_EINZUG,
                    );
                }
            }
        }
        aus.push('\n');
        absatz(
            &mut aus,
            if e {
                "Wording, sources and limits: promptscan explain <RULE>"
            } else {
                "Wortlaut, Quellen und Grenzen: promptscan explain <REGEL>"
            },
            "",
            0,
        );
        absatz(&mut aus, HILFSMITTEL.in_sprache(e), "", 0);
        aus
    })
}

#[derive(Serialize)]
struct NormKurzJson {
    zitat: &'static str,
    erlass: &'static str,
    fassung: &'static str,
    url: &'static str,
}

#[derive(Serialize)]
struct NormJson {
    zitat: &'static str,
    erlass: &'static str,
    fassung: &'static str,
    url: &'static str,
    wortlaut: &'static str,
}

fn norm_json(n: &Norm) -> NormJson {
    NormJson {
        zitat: n.zitat,
        erlass: n.erlass,
        fassung: n.fassung,
        url: n.url,
        wortlaut: n.wortlaut,
    }
}

#[derive(Serialize)]
struct RegelKurzJson {
    id: &'static str,
    klasse: &'static str,
    titel: ZweiJson<'static>,
    grundlage: Vec<NormKurzJson>,
}

#[derive(Serialize)]
struct RegelnJson {
    schema_version: u32,
    dokument: &'static str,
    regeln: Vec<RegelKurzJson>,
}

/// `promptscan rules` als JSON (docs/AUSGABE.md).
pub fn regeln_json() -> String {
    als_json(&RegelnJson {
        schema_version: SCHEMA_VERSION,
        dokument: "regeln",
        regeln: katalog::alle()
            .iter()
            .map(|r| RegelKurzJson {
                id: r.id,
                klasse: r.klasse.kurz(),
                titel: zwei(&r.titel),
                grundlage: r
                    .grundlage
                    .iter()
                    .map(|n| NormKurzJson {
                        zitat: n.zitat,
                        erlass: n.erlass,
                        fassung: n.fassung,
                        url: n.url,
                    })
                    .collect(),
            })
            .collect(),
    })
}

// ---------------------------------------------------------------------------------------
// explain <REGEL>
// ---------------------------------------------------------------------------------------

/// Bestand einer Wortliste für eine Regel: Einträge je Sprache, verschiedene
/// Fundstellen (Feld `quelle`) und Einträge anderer Regeln in derselben Datei.
#[derive(Debug, Default, PartialEq, Eq)]
struct Bestand {
    de: usize,
    fr: usize,
    it: usize,
    fundstellen: usize,
    andere_regeln: BTreeMap<String, usize>,
}

impl Bestand {
    fn gesamt(&self) -> usize {
        self.de + self.fr + self.it
    }
}

fn bestand(lex: &Lexikon, regel: &str) -> Bestand {
    let mut b = Bestand::default();
    let mut fundstellen = BTreeSet::new();
    for e in &lex.eintraege {
        if e.regel != regel {
            *b.andere_regeln.entry(e.regel.clone()).or_default() += 1;
            continue;
        }
        match e.sprache {
            Sprache::De => b.de += 1,
            Sprache::Fr => b.fr += 1,
            Sprache::It => b.it += 1,
        }
        fundstellen.insert(e.quelle.trim());
    }
    b.fundstellen = fundstellen.len();
    b
}

fn ist_register(datei: &str) -> bool {
    datei.ends_with(".tsv")
}

fn wortliste_text(aus: &mut String, datei: &str, lex: &Lexikon, regel: &str, e: bool) {
    let b = bestand(lex, regel);
    let mut zeile = if e {
        format!(
            "{datei}: entries {} (de {}, fr {}, it {}), distinct references {}",
            b.gesamt(),
            b.de,
            b.fr,
            b.it,
            b.fundstellen
        )
    } else {
        format!(
            "{datei}: Einträge {} (de {}, fr {}, it {}), verschiedene Fundstellen {}",
            b.gesamt(),
            b.de,
            b.fr,
            b.it,
            b.fundstellen
        )
    };
    if !b.andere_regeln.is_empty() {
        let andere = b
            .andere_regeln
            .iter()
            .map(|(r, n)| format!("{r} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(
            zeile,
            "; {} {andere}",
            if e {
                "also entries of other rules:"
            } else {
                "dazu Einträge anderer Regeln:"
            }
        );
    }
    absatz(aus, &zeile, "  ", 4);
}

fn register_text(aus: &mut String, datei: &str, reg: &Register, e: bool) {
    let feld = |k: &str| reg.kopf(k).unwrap_or("");
    let _ = writeln!(
        aus,
        "  {datei}: {} {}",
        if e {
            "register, rows"
        } else {
            "Register, Zeilen"
        },
        reg.zeilen.len()
    );
    absatz(
        aus,
        &format!("{}; {}", feld("titel"), feld("herausgeber")),
        "    ",
        4,
    );
    let (abgerufen, lizenz, filter, sha) = if e {
        ("retrieved", "licence", "filter", "SHA-256 of the raw file")
    } else {
        ("abgerufen", "Lizenz", "Filter", "SHA-256 der Rohdatei")
    };
    absatz(
        aus,
        &format!(
            "{abgerufen} {}; {lizenz}: {}",
            feld("abgerufen"),
            feld("lizenz")
        ),
        "    ",
        6,
    );
    absatz(aus, &format!("{filter}: {}", feld("filter")), "    ", 6);
    absatz(
        aus,
        &format!("{sha}: {}", feld("rohdatei-sha256")),
        "    ",
        6,
    );
    let _ = writeln!(aus, "    {}", feld("url"));
}

fn lexikon_text(aus: &mut String, datei: &str, regel: &str, e: bool) {
    if ist_register(datei) {
        if let Some(reg) = lexikon::register_falls(datei) {
            register_text(aus, datei, reg, e);
            return;
        }
    } else if let Some(lex) = lexikon::lexikon_falls(datei) {
        wortliste_text(aus, datei, lex, regel, e);
        return;
    }
    let _ = writeln!(
        aus,
        "  {datei}: {}",
        if e {
            "missing from the program"
        } else {
            "fehlt im Programm"
        }
    );
}

/// `promptscan explain <REGEL>` als Text: alles aus [`RegelInfo`], die Wortlisten mit
/// ihrem Bestand, bei Regeln nach Art. 5 lit. c DSG die Bedingung «Dritter oder
/// Auftragsbearbeiter» und immer «Was promptscan nicht entscheidet» (SPEC 4.5).
pub fn erklaerung_text(r: &RegelInfo, d: &Darstellung) -> String {
    fliesstext(d, || erklaerung_text_roh(r, d))
}

/// `explain fundstellen`, «Rechtlicher Bezug».
const FUNDSTELLEN_BEZUG: Txt = Txt::neu(
    "promptscan kennt keine Norm, die an die Bitte um Fundstellen selbst eine Rechtsfolge knüpft. Für Anwältinnen und Anwälte gilt: «Sie üben ihren Beruf sorgfältig und gewissenhaft aus» (Art. 12 lit. a BGFA).",
    "promptscan knows of no norm that attaches a legal consequence to asking for citations as such. For lawyers: «They shall practise their profession diligently and conscientiously» (Art. 12 lit. a BGFA).",
);

/// `explain fundstellen`, «Grundlage der Erkennung».
const FUNDSTELLEN_ERKENNUNG: Txt = Txt::neu(
    "Die Wortliste stammt aus amtlichen Texten und von der Website des Bundesgerichts. Das Bundesgericht informiert die Öffentlichkeit über seine Rechtsprechung (Art. 27 Abs. 1 BGG), unter anderem in der Amtlichen Sammlung (BGE) und im Internet (Art. 57 Abs. 1 BGerR); dort lassen sich Zitate seiner Entscheide nachprüfen. Den Grund des Hinweises belegt die Studie unter «Quellen».",
    "The word list comes from official texts and from the website of the Federal Supreme Court. The Court informs the public about its case law (Art. 27 Abs. 1 BGG), among other means through the official collection (BGE) and the internet (Art. 57 Abs. 1 BGerR); citations of its decisions can be checked there. The study under «Sources» supports the note.",
);

/// Erläuterungen einer Regel zu «Rechtlicher Bezug» und «Grundlage der Erkennung», je vor
/// den Normen.
fn bezug_erlaeuterung(regel: &str) -> (Option<Txt>, Option<Txt>) {
    match regel {
        "fundstellen" => (Some(FUNDSTELLEN_BEZUG), Some(FUNDSTELLEN_ERKENNUNG)),
        _ => (None, None),
    }
}

fn erklaerung_text_roh(r: &RegelInfo, d: &Darstellung) -> String {
    let e = d.englisch;
    let mut aus = String::new();
    let mut kopf = String::new();
    absatz(
        &mut kopf,
        &format!("{} · {}", r.id, r.titel.in_sprache(e)),
        "",
        2,
    );
    for zeile in kopf.lines() {
        let _ = writeln!(aus, "{}", fett(zeile, d));
    }
    aus.push('\n');
    feld(
        &mut aus,
        if e { "Level" } else { "Stufe" },
        stufe_satz(r, e),
    );
    feld(
        &mut aus,
        if e { "Check" } else { "Prüfung" },
        &format!(
            "{} {}: {}",
            if e { "Class" } else { "Klasse" },
            r.klasse.kurz(),
            klassenname(r.klasse, e)
        ),
    );

    ueberschrift(&mut aus, if e { "Detects" } else { "Erkennt" }, d);
    absatz(&mut aus, r.erkennt.in_sprache(e), "  ", 2);

    ueberschrift(&mut aus, if e { "Method" } else { "Methode" }, d);
    absatz(&mut aus, r.methode.in_sprache(e), "  ", 2);

    if let Some(h) = &r.hinweis {
        ueberschrift(
            &mut aus,
            if e {
                "Note in the report"
            } else {
                "Hinweis in der Ausgabe"
            },
            d,
        );
        absatz(&mut aus, h.in_sprache(e), "  ", 2);
    }

    let keine = if e { "  none" } else { "  keine" };

    // Rechtlicher Bezug und Grundlage der Erkennung getrennt, je mit der
    // Erläuterung der Regel vor den Normen.
    let (bezug, erkennung): (Vec<&'static Norm>, Vec<&'static Norm>) = r
        .grundlage
        .iter()
        .copied()
        .partition(|n| normen::ist_bezug(n));
    let (bezug_text, erkennung_text) = bezug_erlaeuterung(r.id);
    ueberschrift(
        &mut aus,
        if e {
            "Legal reference (official wording)"
        } else {
            "Rechtlicher Bezug (amtlicher Wortlaut)"
        },
        d,
    );
    if let Some(t) = bezug_text {
        absatz(&mut aus, t.in_sprache(e), "  ", 2);
    }
    if bezug.is_empty() && bezug_text.is_none() {
        let _ = writeln!(aus, "{keine}");
    }
    for n in &bezug {
        norm_block(&mut aus, n, 2, e);
    }
    if !erkennung.is_empty() || erkennung_text.is_some() {
        ueberschrift(
            &mut aus,
            if e {
                "Basis of detection (official wording)"
            } else {
                "Grundlage der Erkennung (amtlicher Wortlaut)"
            },
            d,
        );
        if let Some(t) = erkennung_text {
            absatz(&mut aus, t.in_sprache(e), "  ", 2);
        }
        for n in &erkennung {
            norm_block(&mut aus, n, 2, e);
        }
    }

    ueberschrift(&mut aus, if e { "Sources" } else { "Quellen" }, d);
    if r.quellen.is_empty() {
        let _ = writeln!(aus, "{keine}");
    }
    for q in &r.quellen {
        quelle_block(&mut aus, q, e);
    }

    ueberschrift(
        &mut aus,
        if e {
            "Word lists and registers"
        } else {
            "Wortlisten und Register"
        },
        d,
    );
    if r.lexika.is_empty() {
        let _ = writeln!(aus, "{keine}");
    }
    for datei in &r.lexika {
        lexikon_text(&mut aus, datei, r.id, e);
    }

    ueberschrift(
        &mut aus,
        if e {
            "Measured quality"
        } else {
            "Gemessene Güte"
        },
        d,
    );
    match r.guete {
        Guete::NichtGemessen => {
            let _ = writeln!(
                aus,
                "  {}",
                if e { "not measured" } else { "nicht gemessen" }
            );
        }
        Guete::Nachgewiesen(t) => absatz(&mut aus, t.in_sprache(e), "  ", 2),
    }

    ueberschrift(&mut aus, if e { "Limits" } else { "Grenzen" }, d);
    if r.grenzen.is_empty() {
        let _ = writeln!(aus, "{keine}");
    }
    for g in &r.grenzen {
        absatz(&mut aus, g.in_sprache(e), "  - ", 4);
    }

    nicht_entschieden_kurz(&mut aus, regel_stufe(r), d);
    aus
}

/// Die Stufe, unter der `rules` eine Regel führt: A und B GELB, aber `firma` ROT (eine
/// Firma ergibt nur mit Geheimhaltungspflicht einen Fund), C ORANGE, Kontext und Hinweis
/// HINWEIS.
fn regel_stufe(r: &RegelInfo) -> Stufe {
    match r.klasse {
        Klasse::A | Klasse::B if r.id == "firma" => Stufe::Rot,
        Klasse::A | Klasse::B => Stufe::Gelb,
        Klasse::C => Stufe::Orange,
        Klasse::Hinweis | Klasse::Kontext => Stufe::Hinweis,
    }
}

/// Der Satz «Stufe» in `explain <REGEL>`: welche Stufe die Regel ergibt und wann eine
/// andere; die Stufe steht zuoberst.
fn stufe_satz(r: &RegelInfo, e: bool) -> &'static str {
    let ohne_identifikator = matches!(r.id, "geburtsdatum" | "wohnort");
    match (regel_stufe(r), r.id, e) {
        (Stufe::Gelb, _, false) if ohne_identifikator => {
            "GELB: Personendaten (Art. 5 lit. a DSG); ohne direkten Identifikator im Prompt HINWEIS. ROT mit Geheimhaltungspflicht, HINWEIS mit --target local."
        }
        (Stufe::Gelb, _, true) if ohne_identifikator => {
            "YELLOW: personal data (Art. 5 lit. a DSG); without a direct identifier in the prompt a NOTE. RED with a duty of confidentiality, NOTE with --target local."
        }
        (Stufe::Gelb, _, false) => {
            "GELB: Personendaten (Art. 5 lit. a DSG). ROT mit Geheimhaltungspflicht, HINWEIS mit --target local."
        }
        (Stufe::Gelb, _, true) => {
            "YELLOW: personal data (Art. 5 lit. a DSG). RED with a duty of confidentiality, NOTE with --target local."
        }
        (Stufe::Orange, _, false) => {
            "ORANGE: besonders schützenswerte Personendaten (Art. 5 lit. c DSG). ROT mit Geheimhaltungspflicht; ohne direkten Identifikator HINWEIS oder kein Fund; HINWEIS mit --target local."
        }
        (Stufe::Orange, _, true) => {
            "ORANGE: sensitive personal data (Art. 5 lit. c DSG). RED with a duty of confidentiality; without a direct identifier a NOTE or no finding; NOTE with --target local."
        }
        (Stufe::Rot, _, false) => {
            "ROT nur mit Geheimhaltungspflicht (--role oder Geheimnis-Kontext im Text); sonst kein Fund, eine Kollektiv- oder Kommanditgesellschaft wie ein Geburtsdatum."
        }
        (Stufe::Rot, _, true) => {
            "RED only with a duty of confidentiality (--role or a secrecy context in the text); otherwise no finding, a general or limited partnership like a date of birth."
        }
        (Stufe::Hinweis, "kontext", false) => {
            "HINWEIS: Rechtsfolge nicht beurteilt. Hebt ohne angegebene Rolle die anderen Funde auf ROT."
        }
        (Stufe::Hinweis, "kontext", true) => {
            "NOTE: legal consequence not assessed. Without a given role it raises the other findings to RED."
        }
        (Stufe::Hinweis, _, false) => "HINWEIS: Rechtsfolge nicht beurteilt.",
        (Stufe::Hinweis, _, true) => "NOTE: legal consequence not assessed.",
    }
}

/// Die Stufen, deren `explain` zusammen alle Punkte «Was promptscan nicht entscheidet» voll
/// zeigen: HINWEIS allein, sonst die Stufe der Regel und dazu ROT oder GELB.
fn stufen_mit_allen_punkten(stufe: Stufe) -> &'static [Stufe] {
    match stufe {
        Stufe::Hinweis => &[Stufe::Hinweis],
        Stufe::Rot => &[Stufe::Rot, Stufe::Gelb],
        Stufe::Orange => &[Stufe::Orange, Stufe::Rot],
        Stufe::Gelb => &[Stufe::Gelb, Stufe::Rot],
    }
}

/// «Was promptscan nicht entscheidet» in `explain <REGEL>`: alle Punkte, kurz und ohne
/// Wortlaut; den vollen Text und die Literatur nennt der Verweis auf `explain <STUFE>`.
fn nicht_entschieden_kurz(aus: &mut String, stufe: Stufe, d: &Darstellung) {
    let e = d.englisch;
    ueberschrift(
        aus,
        if e {
            "What promptscan does not decide"
        } else {
            "Was promptscan nicht entscheidet"
        },
        d,
    );
    for a in ALLE_PUNKTE {
        absatz(aus, a.kurz.in_sprache(e), "  - ", 4);
    }
    let befehle: Vec<String> = stufen_mit_allen_punkten(stufe)
        .iter()
        .map(|s| format!("promptscan explain {}", stufe_name(*s, e)))
        .collect();
    let befehle = befehle.join(", ");
    absatz(
        aus,
        &if e {
            format!("Wording and literature: {befehle}")
        } else {
            format!("Wortlaut und Literatur: {befehle}")
        },
        "  ",
        2,
    );
}

#[derive(Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
enum StandJson {
    Geprueft { datum: &'static str },
    ZuPruefen { was: &'static str },
}

#[derive(Serialize)]
struct QuelleJson {
    titel: &'static str,
    url: &'static str,
    lizenz: Option<&'static str>,
    stand: StandJson,
}

#[derive(Serialize)]
struct EintraegeJson {
    gesamt: usize,
    de: usize,
    fr: usize,
    it: usize,
}

#[derive(Serialize)]
struct AndereRegelJson {
    regel: String,
    eintraege: usize,
}

#[derive(Serialize)]
struct RegisterquelleJson {
    titel: String,
    herausgeber: String,
    url: String,
    lizenz: String,
    abgerufen: String,
    rohdatei_sha256: String,
    filter: String,
}

#[derive(Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
enum LexikonJson {
    Wortliste {
        datei: String,
        eintraege: EintraegeJson,
        fundstellen: usize,
        andere_regeln: Vec<AndereRegelJson>,
    },
    Register {
        datei: String,
        zeilen: usize,
        spalten: Vec<String>,
        quelle: RegisterquelleJson,
    },
    Fehlt {
        datei: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
enum GueteJson {
    NichtGemessen,
    Nachgewiesen { text: ZweiJson<'static> },
}

#[derive(Serialize)]
struct LiteraturJson {
    zitat: &'static str,
    url: Option<&'static str>,
    stand: ZweiJson<'static>,
}

#[derive(Serialize)]
struct AbschnittJson {
    text: ZweiJson<'static>,
    normen: Vec<NormJson>,
    literatur: Vec<LiteraturJson>,
}

fn abschnitt_json(a: &Abschnitt) -> AbschnittJson {
    AbschnittJson {
        text: zwei(&a.text),
        normen: a.normen.iter().map(|n| norm_json(n)).collect(),
        literatur: a
            .literatur
            .iter()
            .map(|l| LiteraturJson {
                zitat: l.zitat,
                url: l.url,
                stand: zwei(&l.stand),
            })
            .collect(),
    }
}

fn wortliste_json(datei: &str, lex: &Lexikon, regel: &str) -> LexikonJson {
    let b = bestand(lex, regel);
    LexikonJson::Wortliste {
        datei: datei.to_string(),
        eintraege: EintraegeJson {
            gesamt: b.gesamt(),
            de: b.de,
            fr: b.fr,
            it: b.it,
        },
        fundstellen: b.fundstellen,
        andere_regeln: b
            .andere_regeln
            .into_iter()
            .map(|(regel, eintraege)| AndereRegelJson { regel, eintraege })
            .collect(),
    }
}

fn register_json(datei: &str, reg: &Register) -> LexikonJson {
    let feld = |k: &str| reg.kopf(k).unwrap_or("").to_string();
    LexikonJson::Register {
        datei: datei.to_string(),
        zeilen: reg.zeilen.len(),
        spalten: reg.spalten.clone(),
        quelle: RegisterquelleJson {
            titel: feld("titel"),
            herausgeber: feld("herausgeber"),
            url: feld("url"),
            lizenz: feld("lizenz"),
            abgerufen: feld("abgerufen"),
            rohdatei_sha256: feld("rohdatei-sha256"),
            filter: feld("filter"),
        },
    }
}

fn lexikon_json(datei: &str, regel: &str) -> LexikonJson {
    let gefunden = if ist_register(datei) {
        lexikon::register_falls(datei).map(|reg| register_json(datei, reg))
    } else {
        lexikon::lexikon_falls(datei).map(|lex| wortliste_json(datei, lex, regel))
    };
    gefunden.unwrap_or_else(|| LexikonJson::Fehlt {
        datei: datei.to_string(),
    })
}

#[derive(Serialize)]
struct ErklaerungJson {
    schema_version: u32,
    dokument: &'static str,
    id: &'static str,
    klasse: &'static str,
    titel: ZweiJson<'static>,
    erkennt: ZweiJson<'static>,
    methode: ZweiJson<'static>,
    hinweis: Option<ZweiJson<'static>>,
    grundlage: Vec<NormJson>,
    quellen: Vec<QuelleJson>,
    lexika: Vec<LexikonJson>,
    guete: GueteJson,
    grenzen: Vec<ZweiJson<'static>>,
    bedingung_dritte: Option<AbschnittJson>,
    nicht_entschieden: Vec<AbschnittJson>,
}

/// `promptscan explain <REGEL>` als JSON (docs/AUSGABE.md).
pub fn erklaerung_json(r: &RegelInfo) -> String {
    als_json(&ErklaerungJson {
        schema_version: SCHEMA_VERSION,
        dokument: "erklaerung",
        id: r.id,
        klasse: r.klasse.kurz(),
        titel: zwei(&r.titel),
        erkennt: zwei(&r.erkennt),
        methode: zwei(&r.methode),
        hinweis: r.hinweis.as_ref().map(zwei),
        grundlage: r.grundlage.iter().map(|n| norm_json(n)).collect(),
        quellen: r
            .quellen
            .iter()
            .map(|q| QuelleJson {
                titel: q.titel,
                url: q.url,
                lizenz: q.lizenz,
                stand: match q.stand {
                    Stand::Geprueft(datum) => StandJson::Geprueft { datum },
                    Stand::ZuPruefen(was) => StandJson::ZuPruefen { was },
                },
            })
            .collect(),
        lexika: r.lexika.iter().map(|d| lexikon_json(d, r.id)).collect(),
        guete: match r.guete {
            Guete::NichtGemessen => GueteJson::NichtGemessen,
            Guete::Nachgewiesen(t) => GueteJson::Nachgewiesen { text: zwei(&t) },
        },
        grenzen: r.grenzen.iter().map(zwei).collect(),
        bedingung_dritte: ist_art5c(r).then(|| abschnitt_json(&AUFTRAGSBEARBEITUNG)),
        nicht_entschieden: ALLE_PUNKTE.iter().map(|a| abschnitt_json(a)).collect(),
    })
}

// ---------------------------------------------------------------------------------------
// explain <STUFE>
// ---------------------------------------------------------------------------------------

/// Was eine Stufe voraussetzt und warum sie gilt (SPEC 4.2 bis 4.4, umgesetzt in
/// `stufen.rs`).
struct Stufeninfo {
    titel: Txt,
    voraussetzung: &'static [Txt],
    grund: &'static [Txt],
    normen: Vec<&'static Norm>,
}

/// Die Normen aller Rollen ohne Doppel, in der Reihenfolge von [`Rolle::ALLE`].
fn rollennormen() -> Vec<&'static Norm> {
    let mut v: Vec<&'static Norm> = Vec::new();
    for r in Rolle::ALLE {
        for n in stufen::normen_fuer(r) {
            if !v.iter().any(|m| m.zitat == n.zitat) {
                v.push(n);
            }
        }
    }
    v
}

const GELB_VORAUSSETZUNG: &[Txt] = &[
    Txt::neu(
        "Der Prompt enthält einen direkten Identifikator einer natürlichen Person: Name, Adresse, AHV-Nummer, IBAN, Telefonnummer oder E-Mail-Adresse (Regeln name, adresse, ahv, iban, tel, mail).",
        "The prompt contains a direct identifier of a natural person: name, address, AHV number, IBAN, telephone number or e-mail address (rules name, adresse, ahv, iban, tel, mail).",
    ),
    Txt::neu(
        "Geburtsdatum und Wohnort werden nur GELB, wenn der Prompt zugleich einen direkten Identifikator enthält; sonst erscheinen sie als HINWEIS.",
        "Date of birth and place of residence only become YELLOW if the prompt also contains a direct identifier; otherwise they appear as a NOTE.",
    ),
    Txt::neu(
        "Ein Identifikator irgendwo im Prompt genügt; promptscan prüft nicht, ob er dieselbe Person betrifft wie Geburtsdatum oder Wohnort. Auch die eigene Unterschrift oder Kontaktzeile zählt als Identifikator.",
        "An identifier anywhere in the prompt is enough; promptscan does not check whether it concerns the same person as the date of birth or place of residence. The writer's own signature or contact line counts as an identifier too.",
    ),
    firma::DSG_NATUERLICHE_PERSONEN,
    Txt::neu(
        "Juristische Personen (Regel firma) werden darum nie GELB (Art. 5 lit. a DSG), ebenso wenig die KmGK, deren unbeschränkt haftende Gesellschafter Aktiengesellschaften sind (Art. 98 Abs. 2 KAG). Kollektiv- und Kommanditgesellschaften (Zusatz «Personengesellschaft») zählen wie Geburtsdatum und Wohnort: GELB nur mit einem direkten Identifikator im Prompt, sonst HINWEIS (Art. 552 Abs. 1, Art. 568 Abs. 1 und Art. 594 Abs. 2 OR).",
        "Legal persons (rule firma) therefore never become YELLOW (Art. 5 lit. a DSG), nor does the KmGK, whose partners with unlimited liability are companies limited by shares (Art. 98 Abs. 2 KAG). General and limited partnerships (addition «partnership») count like date of birth and place of residence: YELLOW only with a direct identifier in the prompt, otherwise a NOTE (Art. 552 Abs. 1, Art. 568 Abs. 1 and Art. 594 Abs. 2 OR).",
    ),
    Txt::neu(
        "Besteht eine Geheimhaltungspflicht (--role oder Geheimnis-Kontext im Text), wird der Fund ROT (promptscan explain rot).",
        "If a duty of confidentiality applies (--role or a secrecy context in the text), the finding becomes RED (promptscan explain red).",
    ),
];

/// Zweiter Absatz des Grunds bei GELB und ORANGE: Arbeitgeber.
const ARBEITGEBER: Txt = Txt::neu(
    "Arbeitgeber dürfen Daten über Arbeitnehmende nur bearbeiten, soweit sie deren Eignung für das Arbeitsverhältnis betreffen oder zur Durchführung des Arbeitsvertrags erforderlich sind (Art. 328b OR); im Übrigen gilt das DSG. Vorschriften über das öffentlich-rechtliche Dienstverhältnis bleiben vorbehalten (Art. 342 Abs. 1 lit. a OR).",
    "Employers may handle data concerning employees only to the extent that such data concern their suitability for the job or are necessary for the performance of the employment contract (Art. 328b OR); in all other respects, the DSG applies. Art. 342 Abs. 1 lit. a OR reserves the provisions regarding employment relationships under public law.",
);

/// Die Normen von [`ARBEITGEBER`] und des Punkts [`GELTUNGSBEREICH`], bei GELB und
/// ORANGE.
const NORMEN_ARBEITGEBER_GELTUNG: &[&Norm] = &[
    &normen::OR_328B,
    &normen::OR_342_1_A,
    &normen::DSG_34_1,
    &normen::DSG_34_4,
    &normen::DSG_36_1,
    &normen::DSG_36_2,
    &normen::DSG_2_1,
    &normen::DSG_2_3,
];

/// Grund bei GELB, mit dem Absatz zu Arbeitgebern. Den Geltungsbereich nennt der Punkt
/// [`GELTUNGSBEREICH`] unter «Was promptscan nicht entscheidet».
const GELB_GRUND: &[Txt] = &[
    Txt::neu(
        "Private dürfen gewöhnliche Personendaten ohne Rechtfertigungsgrund bearbeiten, solange sie die Persönlichkeit nicht widerrechtlich verletzen (Art. 30 Abs. 1 DSG). Massstab ist die Verhältnismässigkeit (Art. 6 Abs. 2 DSG): Braucht die KI die Angabe? Gehen die Daten ins Ausland, gelten zusätzlich Art. 16 f. DSG.",
        "Private persons may process ordinary personal data without grounds for justification as long as they do not unlawfully breach the data subjects’ personality rights (Art. 30 Abs. 1 DSG). The yardstick is proportionality (Art. 6 Abs. 2 DSG): does the AI need the detail? If the data goes abroad, Art. 16 f. DSG apply as well.",
    ),
    ARBEITGEBER,
];

const ORANGE_VORAUSSETZUNG: &[Txt] = &[
    Txt::neu(
        "Eine Angabe aus einer Kategorie von Art. 5 lit. c Ziff. 1–6 DSG (Regeln art5c-1 bis art5c-6), und der Prompt enthält einen direkten Identifikator.",
        "Information from one of the categories of Art. 5 lit. c Ziff. 1–6 DSG (rules art5c-1 to art5c-6), and the prompt contains a direct identifier.",
    ),
    Txt::neu(
        "Ein Identifikator irgendwo im Prompt genügt; promptscan prüft nicht, ob er dieselbe Person betrifft wie die Angabe. Auch die eigene Unterschrift oder Kontaktzeile zählt: «Was gilt bei Arbeitsunfähigkeit?» wird ORANGE, wenn «Erika Beispiel, Seestrasse 12, 8002 Zürich» darunter steht.",
        "An identifier anywhere in the prompt is enough; promptscan does not check whether it concerns the same person as the information. The writer's own signature or contact line counts too: «Was gilt bei Arbeitsunfähigkeit?» becomes ORANGE when «Erika Beispiel, Seestrasse 12, 8002 Zürich» stands below it.",
    ),
    Txt::neu(
        "Ohne direkten Identifikator gibt es keine Stufe nach DSG. Bezeichnet derselbe Satz eine Person, etwa «mein Mandant», erscheint ein HINWEIS; ob der Kontext eine Person bestimmbar macht, beurteilt promptscan nicht.",
        "Without a direct identifier there is no DSG level. If the same sentence refers to a person, e.g. «mein Mandant», a NOTE appears; promptscan does not assess whether the context makes a person identifiable.",
    ),
    Txt::neu(
        "Besteht eine Geheimhaltungspflicht (--role oder Geheimnis-Kontext im Text), wird der Fund ROT (promptscan explain rot).",
        "If a duty of confidentiality applies (--role or a secrecy context in the text), the finding becomes RED (promptscan explain red).",
    ),
];

/// Grund bei ORANGE, mit dem Absatz zu Arbeitgebern; Geltungsbereich wie bei [`GELB_GRUND`].
const ORANGE_GRUND: &[Txt] = &[
    Txt::neu(
        "Die Bekanntgabe besonders schützenswerter Personendaten an Dritte durch Private ist eine Persönlichkeitsverletzung (Art. 30 Abs. 2 lit. c DSG). Ist eine Einwilligung erforderlich, muss sie ausdrücklich erfolgen (Art. 6 Abs. 7 lit. a DSG).",
        "Disclosure of sensitive personal data to third parties by private persons is a breach of personality rights (Art. 30 Abs. 2 lit. c DSG). If consent is required, it must be given explicitly (Art. 6 Abs. 7 lit. a DSG).",
    ),
    ARBEITGEBER,
];

const ROT_VORAUSSETZUNG: &[Txt] = &[
    Txt::neu(
        "Eine Geheimhaltungspflicht besteht: angegeben mit --role oder in der Konfigurationsdatei oder, ohne Angabe, aus dem Text erschlossen (Geheimnis-Kontext, etwa «Als Anwältin habe ich»). Die Tabelle «Rollen» nennt die Normen je Rolle.",
        "A duty of confidentiality applies: given with --role or in the configuration file or, if none is given, inferred from the text (secrecy context, e.g. «Als Anwältin habe ich»). The table «Roles» lists the provisions per role.",
    ),
    Txt::neu(
        "Zeigt der Text nur Klientschaft oder eine Vertretung («meine Mandantin», «Ich vertrete Herrn Hans Muster»), bleibt der Beruf offen: ROT mit den Normen, die in Frage kommen (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB, Art. 47 Abs. 1 lit. a BankG). Den Beruf mit --role festlegen.",
        "If the text only shows clients or representation («meine Mandantin», «Ich vertrete Herrn Hans Muster»), the profession stays open: RED with the provisions that may apply (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB, Art. 47 Abs. 1 lit. a BankG). Set the profession with --role.",
    ),
    Txt::neu(
        "Dann wird ROT, was sonst GELB oder ORANGE wäre. Die DSG-Normen der Rolle und «zugleich besonders schützenswert» nennt ein Fund nur bei einer natürlichen Person (Art. 5 lit. a DSG).",
        "Then everything that would otherwise be YELLOW or ORANGE becomes RED. A finding names the role's DSG provisions and «also sensitive» only for a natural person (Art. 5 lit. a DSG).",
    ),
    Txt::neu(
        "Eine juristische Person (Regel firma) wird ROT und zählt als Identifikator für Geburtsdatum, Wohnort und Angaben nach Art. 5 lit. c DSG, wenn die Pflicht der Rolle «ein Geheimnis» schützt, gleich wessen: Art. 320 und 321 StGB, Art. 47 BankG, Art. 69 FINIG. --role professional (Art. 62 DSG) schützt nur geheime Personendaten: Eine Firma ergibt dann keinen Fund, eine Kollektiv- oder Kommanditgesellschaft nur mit einem Identifikator. Ob eine Firma ein Fabrikations- oder Geschäftsgeheimnis hat (Art. 162 StGB), entscheidet promptscan nicht.",
        "A legal person (rule firma) becomes RED and counts as an identifier for date of birth, place of residence and information under Art. 5 lit. c DSG if the role's duty protects «a secret», whoever's it is: Art. 320 und 321 StGB, Art. 47 BankG, Art. 69 FINIG. --role professional (Art. 62 DSG) only protects secret personal data: a company then gives no finding, a general or limited partnership only together with an identifier. promptscan does not decide whether a company has a manufacturing or trade secret (Art. 162 StGB).",
    ),
    Txt::neu(
        "Auch die eigene Unterschrift oder Kontaktzeile wird ROT: promptscan unterscheidet nicht, wessen Name oder Adresse im Prompt steht.",
        "The writer's own signature or contact line becomes RED too: promptscan does not distinguish whose name or address is in the prompt.",
    ),
    Txt::neu(
        "Nicht abgedeckt ist das Berufsgeheimnis der Finanzmarktinfrastrukturen (Art. 147 FinfraG): Keine Rolle von --role nennt es.",
        "The professional confidentiality of financial market infrastructures (Art. 147 FinfraG) is not covered: no --role value names it.",
    ),
    Txt::neu(
        "--role none schliesst ROT aus, auch wenn der Text einen Geheimnis-Kontext zeigt.",
        "--role none rules out RED, even if the text shows a secrecy context.",
    ),
];

const ROT_GRUND: &[Txt] = &[
    Txt::neu(
        "Je nach Rolle ist das Offenbaren eines Geheimnisses strafbar (Normen in der Tabelle «Rollen»). Ob hier offenbart wird und ob eine Ausnahme greift, entscheidet promptscan nicht (siehe unten). Zudem darf die Bearbeitung keinem Auftragsbearbeiter übertragen werden, wenn eine gesetzliche oder vertragliche Geheimhaltungspflicht es verbietet (Art. 9 Abs. 1 lit. b DSG).",
        "Depending on the role, disclosing a secret is a criminal offence (norms in the table «Roles»). Whether this prompt discloses one and whether an exception applies is not decided by promptscan (see below). In addition, processing may not be assigned to a processor if a statutory or contractual duty of confidentiality prohibits it (Art. 9 Abs. 1 lit. b DSG).",
    ),
    // Zweiter Absatz: der Geltungsbereich mit Satz 2 von Art. 2 Abs. 3 DSG. Zitate bleiben
    // auch englisch deutsch, auch ihr «und» («Art. 320 und 321 StGB»).
    Txt::neu(
        "Art. 320 und 321 StGB, Art. 47 BankG und Art. 69 FINIG gelten unabhängig davon, welches Datenschutzrecht anwendbar ist. Art. 9 DSG gilt für private Personen und Bundesorgane (Art. 2 Abs. 1 DSG); Bundesorgane brauchen zudem grundsätzlich eine gesetzliche Grundlage (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; Ausnahmen: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG). Für kantonale und kommunale Organe gilt kantonales Datenschutzrecht. In Gerichtsverfahren und in Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 Abs. 3 DSG).",
        "Art. 320 und 321 StGB, Art. 47 BankG and Art. 69 FINIG apply regardless of which data protection law is applicable. Art. 9 DSG applies to private persons and federal bodies (Art. 2 Abs. 1 DSG); federal bodies also need, in principle, a statutory basis (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; exceptions: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG). Cantonal and communal bodies are subject to cantonal data protection law. In court proceedings and in proceedings governed by federal procedural regulations, procedural law applies; in first instance administrative proceedings, the DSG applies (Art. 2 Abs. 3 DSG).",
    ),
];

const HINWEIS_VORAUSSETZUNG: &[Txt] = &[
    Txt::neu(
        "Geburtsdatum, Wohnort oder eine Kollektiv- oder Kommanditgesellschaft (Regel firma) ohne direkten Identifikator im Prompt.",
        "Date of birth, place of residence or a general or limited partnership (rule firma) without a direct identifier in the prompt.",
    ),
    Txt::neu(
        "Ein besonders schützenswerter Inhalt ohne direkten Identifikator in einem Satz, der eine Person bezeichnet, etwa «mein Mandant».",
        "Sensitive content without a direct identifier, in a sentence that refers to a person, e.g. «mein Mandant».",
    ),
    Txt::neu(
        "Ein Geheimnis-Kontext aus dem Text, der Funde auf ROT gehoben hat; mit --role bestätigen oder mit --role none ausschliessen. Bleibt der Beruf offen («meine Klientin»), nennt der Hinweis die Normen, die in Frage kommen, und die Rollen zur Auswahl.",
        "A secrecy context from the text that raised findings to RED; confirm it with --role or rule it out with --role none. If the profession stays open («meine Klientin»), the note names the provisions that may apply and the roles to choose from.",
    ),
    // Punkt --target local. Der letzte Satz sagt nicht, dass die Kopfzeile dann
    // «(Konfiguration)» nennt: Die Kopfzeile kann die Herkunft des Ziels erst nennen, wenn
    // der Bericht ein Feld dafür hat und main.rs es setzt. Die Warnung selbst stimmt schon
    // heute: main.rs liest target aus der Konfigurationsdatei.
    Txt::neu(
        "--target local gilt für ein Modell, das auf diesem Rechner läuft, ohne Synchronisierung in eine Cloud. Läuft es so, wird nichts übermittelt oder zugänglich gemacht: keine Bekanntgabe (Art. 5 lit. e DSG) und kein Offenbaren nach den Normen der Rolle. Jeder Fund der Stufe GELB, ORANGE oder ROT wird zum HINWEIS und nennt die Stufe, die er sonst hätte. Bearbeitet werden die Daten trotzdem (Art. 5 lit. d DSG): Die Grundsätze von Art. 6 DSG gelten weiter, etwa die Verhältnismässigkeit (Art. 6 Abs. 2 DSG). Ob das Modell wirklich lokal läuft, prüft promptscan nicht. Steht target = \"local\" in der Konfigurationsdatei, gilt das für jeden Prompt, auch für einen, der doch in eine Cloud geht.",
        "--target local is meant for a model that runs on this computer, without syncing to a cloud. If it runs that way, nothing is transmitted or made accessible: no disclosure (Art. 5 lit. e DSG) and no disclosure of secrets under the norms of the role. Every finding of level YELLOW, ORANGE or RED becomes a NOTE and names the level it would otherwise have. The data is still processed (Art. 5 lit. d DSG): the principles of Art. 6 DSG still apply, such as proportionality (Art. 6 Abs. 2 DSG). promptscan does not check whether the model really runs locally. If target = \"local\" is set in the configuration file, it applies to every prompt, including one that does go to a cloud.",
    ),
    Txt::neu(
        "Hinweisregeln wie vermerk, fundstellen und ahv-sammlung; ihre Grundlage nennt promptscan explain <REGEL>.",
        "Note rules such as vermerk, fundstellen and ahv-sammlung; promptscan explain <RULE> gives their basis.",
    ),
];

const HINWEIS_GRUND: &[Txt] = &[Txt::neu(
    "Die Bewertung hängt von Tatsachen ab, die promptscan nicht kennt. Darum ordnet promptscan keine Rechtsfolge zu; ausgeschlossen ist sie damit nicht.",
    "The assessment depends on facts promptscan does not know. promptscan therefore assigns no legal consequence; that does not rule one out.",
)];

fn stufeninfo(stufe: Stufe) -> Stufeninfo {
    match stufe {
        Stufe::Gelb => Stufeninfo {
            titel: PERSONENDATEN,
            voraussetzung: GELB_VORAUSSETZUNG,
            grund: GELB_GRUND,
            normen: [
                &[
                    &normen::DSG_5_A,
                    &normen::DSG_30_1,
                    &normen::DSG_6_2,
                    &normen::DSG_16_1,
                ][..],
                NORMEN_ARBEITGEBER_GELTUNG,
                &[
                    &normen::KAG_98_2,
                    &normen::OR_552_1,
                    &normen::OR_568_1,
                    &normen::OR_594_2,
                ][..],
            ]
            .concat(),
        },
        Stufe::Orange => Stufeninfo {
            titel: BESONDERS,
            voraussetzung: ORANGE_VORAUSSETZUNG,
            grund: ORANGE_GRUND,
            normen: [
                &[&normen::DSG_5_C, &normen::DSG_30_2_C, &normen::DSG_6_7_A][..],
                NORMEN_ARBEITGEBER_GELTUNG,
            ]
            .concat(),
        },
        Stufe::Rot => {
            let mut liste = rollennormen();
            liste.extend([
                &normen::DSG_9_1_B,
                &normen::DSG_2_1,
                &normen::DSG_34_1,
                &normen::DSG_34_4,
                &normen::DSG_36_1,
                &normen::DSG_36_2,
                &normen::DSG_2_3,
                &normen::STGB_162,
                &normen::FINFRAG_147_1_A,
                // Normen der Tabelle «Rollen» und der Hinweise darunter.
                &normen::STGB_110_3,
                &normen::STPO_12,
                &normen::STPO_13,
                &normen::FINIG_2_1,
                &normen::BGFA_2_1,
            ]);
            Stufeninfo {
                titel: TITEL_ROT,
                voraussetzung: ROT_VORAUSSETZUNG,
                grund: ROT_GRUND,
                normen: liste,
            }
        }
        Stufe::Hinweis => Stufeninfo {
            titel: NICHT_BEURTEILT,
            voraussetzung: HINWEIS_VORAUSSETZUNG,
            grund: HINWEIS_GRUND,
            // Punkt --target local.
            normen: vec![&normen::DSG_5_E, &normen::DSG_5_D, &normen::DSG_6_2],
        },
    }
}

/// Die Normen im Abschnitt «Normen» von `explain <STUFE>`: die der Stufe, dann die der
/// Punkte «Was promptscan nicht entscheidet», jede einmal.
fn stufen_normen(stufe: Stufe, info: &Stufeninfo) -> Vec<&'static Norm> {
    let mut v: Vec<&'static Norm> = Vec::new();
    let punkte = nicht_entschieden(stufe)
        .iter()
        .flat_map(|a| a.normen.iter().copied());
    for n in info.normen.iter().copied().chain(punkte) {
        if !v.iter().any(|m| m.zitat == n.zitat) {
            v.push(n);
        }
    }
    v
}

/// Hinweise unter der Tabelle «Rollen»: wer welche Rolle wählt, und für wen das BGFA gilt.
const ROLLEN_HINWEIS: Txt = Txt::neu(
    "Hilfspersonen wählen die Rolle der Person, für die sie arbeiten (Art. 321 Ziff. 1 StGB: «sowie ihre Hilfspersonen»; Art. 320 Ziff. 1 StGB: «als Hilfsperson eines Beamten oder einer Behörde»). Angestellte einer öffentlichen Verwaltung oder der Rechtspflege gelten als Beamte (Art. 110 Abs. 3 StGB): authority oder court. Gerichte im Strafverfahren (Art. 13 StPO): criminal-authority. Finanzinstitute sind Vermögensverwalter, Trustees, Verwalter von Kollektivvermögen, Fondsleitungen und Wertpapierhäuser (Art. 2 Abs. 1 FINIG). Das BGFA gilt für Personen mit Anwaltspatent, die in der Schweiz im Rahmen des Anwaltsmonopols Parteien vor Gerichtsbehörden vertreten (Art. 2 Abs. 1 BGFA).",
    "Assistants choose the role of the person they work for (Art. 321 Ziff. 1 StGB: «as an assistant to any of the foregoing persons»; Art. 320 Ziff. 1 StGB: «as an auxiliary to a public official or an authority»). Employees of a public administrative authority or of an authority for the administration of justice are public officials (Art. 110 Abs. 3 StGB): authority or court. Courts in criminal proceedings (Art. 13 StPO): criminal-authority. Financial institutions are portfolio managers, trustees, managers of collective assets, fund management companies and securities firms (Art. 2 Abs. 1 FINIG). The BGFA applies to persons who hold a lawyer’s practising certificate and represent parties before judicial authorities in Switzerland in terms of the lawyers’ monopoly (Art. 2 Abs. 1 BGFA).",
);

/// Die Tabelle «Rollen» in `explain rot`: je Rolle der Wert für --role, der Name und die
/// Pflicht, darunter die Normen ab Spalte 25; zuletzt, wer welche Rolle wählt.
fn rollen_text(aus: &mut String, d: &Darstellung) {
    let e = d.englisch;
    ueberschrift(
        aus,
        if e {
            "Roles (--role)"
        } else {
            "Rollen (--role)"
        },
        d,
    );
    let leer = " ".repeat(SP_WERT);
    for r in Rolle::ALLE {
        let anfang = format!("  {:<w$}", r.name(), w = SP_WERT - 2);
        let (pflicht, zweite) = if r == Rolle::None {
            if e {
                (
                    "no duty of confidentiality",
                    "no RED, not even from the text".to_string(),
                )
            } else {
                (
                    "keine Geheimhaltungspflicht",
                    "kein ROT, auch nicht aus dem Text".to_string(),
                )
            }
        } else {
            (
                stufen::geheimnis_name(r, e),
                zitate(&stufen::normen_fuer(r)),
            )
        };
        absatz(
            aus,
            &format!("{} · {pflicht}", r.anzeige(e)),
            &anfang,
            SP_WERT,
        );
        absatz(aus, &zweite, &leer, SP_WERT);
    }
    aus.push('\n');
    absatz(aus, ROLLEN_HINWEIS.in_sprache(e), "  ", 2);
}

/// `promptscan explain gelb|orange|rot|hinweis` als Text: die Stufe mit Plakette zuoberst,
/// «Grund», die Voraussetzungen, bei ROT die Tabelle «Rollen», «Was promptscan nicht
/// entscheidet» voll und die Normen im Wortlaut.
pub fn stufen_erklaerung_text(stufe: Stufe, d: &Darstellung) -> String {
    fliesstext(d, || {
        let e = d.englisch;
        let info = stufeninfo(stufe);
        let (breite, _) = fliesstext_darstellung();
        let mut aus = String::new();
        let (p, pb) = plakette(stufe, d);
        zeilen_schreiben(
            &mut aus,
            &p,
            pb,
            1,
            SP_TEXT,
            stuecke(info.titel.in_sprache(e), d),
            breite,
        );
        aus.push('\n');
        for (i, t) in info.grund.iter().enumerate() {
            if i == 0 {
                feld(
                    &mut aus,
                    if e { "Reason" } else { "Grund" },
                    t.in_sprache(e),
                );
            } else {
                aus.push('\n');
                absatz(&mut aus, t.in_sprache(e), &" ".repeat(SP_TEXT), SP_TEXT);
            }
        }
        ueberschrift(&mut aus, if e { "Condition" } else { "Voraussetzung" }, d);
        for t in info.voraussetzung {
            absatz(&mut aus, t.in_sprache(e), "  - ", 4);
        }
        if stufe == Stufe::Rot {
            rollen_text(&mut aus, d);
        }
        nicht_entschieden_text(&mut aus, stufe, d);
        ueberschrift(
            &mut aus,
            if e {
                "Provisions (official German wording)"
            } else {
                "Normen (amtlicher Wortlaut)"
            },
            d,
        );
        for n in stufen_normen(stufe, &info) {
            norm_block(&mut aus, n, 2, e);
        }
        aus
    })
}

#[derive(Serialize)]
struct RolleJson {
    rolle: Rolle,
    pflicht: ZweiJson<'static>,
    normen: Vec<&'static str>,
}

#[derive(Serialize)]
struct StufeJson {
    schema_version: u32,
    dokument: &'static str,
    stufe: Stufe,
    wort: ZweiJson<'static>,
    titel: ZweiJson<'static>,
    voraussetzung: Vec<ZweiJson<'static>>,
    grund: Vec<ZweiJson<'static>>,
    normen: Vec<NormJson>,
    rollen: Vec<RolleJson>,
    bedingung_dritte: Option<AbschnittJson>,
    nicht_entschieden: Vec<AbschnittJson>,
}

/// `promptscan explain gelb|orange|rot|hinweis` als JSON (docs/AUSGABE.md). Das Feld
/// `bedingung_dritte` führt seit Version 2 den Punkt «Auftragsbearbeitung».
pub fn stufen_erklaerung_json(stufe: Stufe) -> String {
    let info = stufeninfo(stufe);
    let rollen = if stufe == Stufe::Rot {
        Rolle::ALLE
            .iter()
            .map(|&r| RolleJson {
                rolle: r,
                pflicht: ZweiJson {
                    de: stufen::geheimnis_name(r, false),
                    en: stufen::geheimnis_name(r, true),
                },
                normen: stufen::normen_fuer(r).iter().map(|n| n.zitat).collect(),
            })
            .collect()
    } else {
        Vec::new()
    };
    als_json(&StufeJson {
        schema_version: SCHEMA_VERSION,
        dokument: "stufe",
        stufe,
        wort: ZweiJson {
            de: stufe.wort(false),
            en: stufe.wort(true),
        },
        titel: zwei(&info.titel),
        voraussetzung: info.voraussetzung.iter().map(zwei).collect(),
        grund: info.grund.iter().map(zwei).collect(),
        normen: info.normen.iter().map(|n| norm_json(n)).collect(),
        rollen,
        bedingung_dritte: (stufe == Stufe::Orange).then(|| abschnitt_json(&AUFTRAGSBEARBEITUNG)),
        nicht_entschieden: nicht_entschieden(stufe)
            .iter()
            .map(|a| abschnitt_json(a))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Optionen;
    use crate::fund::{Beleg, Zweisprachig};
    use crate::lexikon::{Eintrag, Teilwort};
    use serde_json::Value;

    const DOKU: &str = include_str!("../docs/AUSGABE.md");

    // --- Breitenmodell ------------------------------------------------------------------

    #[test]
    fn tabellen_sortiert_und_ueberlappungsfrei() {
        for bereiche in [NULLBREIT, BREIT] {
            for (von, bis) in bereiche {
                assert!(von <= bis, "{von:X}..{bis:X}");
            }
            for paar in bereiche.windows(2) {
                assert!(paar[0].1 < paar[1].0, "{:X?} vor {:X?}", paar[0], paar[1]);
            }
        }
        assert_eq!(
            BREIT.len(),
            126,
            "Zahl der Bereiche aus EastAsianWidth-18.0.0.txt"
        );
    }

    #[test]
    fn zeichenbreiten() {
        assert_eq!(zeichenbreite('a'), 1);
        assert_eq!(zeichenbreite('ü'), 1);
        assert_eq!(zeichenbreite('€'), 1);
        assert_eq!(zeichenbreite('\u{4E2D}'), 2, "CJK-Ideogramm");
        assert_eq!(zeichenbreite('\u{1F600}'), 2, "Emoji, East_Asian_Width W");
        assert_eq!(zeichenbreite('\u{FF21}'), 2, "Vollbreite A");
        assert_eq!(zeichenbreite('\u{0308}'), 0, "kombinierendes Trema");
        assert_eq!(zeichenbreite('\u{200B}'), 0, "Nullbreites Leerzeichen");
        assert_eq!(zeichenbreite('\u{FEFF}'), 0);
        assert_eq!(
            zeichenbreite('\u{00AD}'),
            1,
            "weiches Trennzeichen nach Kuhn"
        );
        assert_eq!(zeichenbreite('\u{303F}'), 1, "U+303F ist N, nicht W");
    }

    #[test]
    fn steuerzeichen_und_bidi_werden_sichtbar() {
        for c in [
            '\u{1B}', '\u{7F}', '\u{85}', '\u{202E}', '\u{2066}', '\u{200F}',
        ] {
            let mut aus = String::new();
            assert_eq!(zeichen_anzeigen(c, 0, &mut aus), 1, "{c:?}");
            assert_eq!(aus, "\u{FFFD}", "{c:?}");
        }
    }

    #[test]
    fn stelle_ueber_einen_umbruch_auf_einer_zeile() {
        // Ein weicher Umbruch in der Stelle als Leerzeichen.
        assert_eq!(stelle_einzeilig("Meine\nMandantin"), "Meine Mandantin");
        assert_eq!(
            stelle_einzeilig("756.1234.\r\n5678.97"),
            "756.1234. 5678.97"
        );
        assert_eq!(stelle_einzeilig("Erika \t\n  Beispiel"), "Erika Beispiel");
        assert_eq!(stelle_einzeilig("a\nb\nc"), "a b c");
        // Über eine Leerzeile hinweg bleibt es bei der ersten Zeile.
        assert_eq!(stelle_einzeilig("Hans Muster \n\nAHV"), "Hans Muster …");
        assert_eq!(stelle_einzeilig("a\tb\u{7}"), "a b\u{FFFD}");
    }

    #[test]
    fn tabulator_bis_zum_naechsten_stopp() {
        for (spalte, erwartet) in [(0, 8), (1, 7), (7, 1), (8, 8), (13, 3)] {
            let mut aus = String::new();
            assert_eq!(zeichen_anzeigen('\t', spalte, &mut aus), erwartet);
            assert_eq!(aus, " ".repeat(erwartet));
        }
    }

    // --- Wortlisten ----------------------------------------------------------------------

    fn eintrag(begriff: &str, sprache: Sprache, regel: &str, quelle: &str) -> Eintrag {
        Eintrag {
            begriff: begriff.into(),
            formen: Vec::new(),
            sprache,
            regel: regel.into(),
            quelle: quelle.into(),
            fassung: "2026-01-01".into(),
            url: "https://example.org".into(),
            gross: false,
            teilwort: Teilwort::Nein,
            merkmal: None,
            notiz: None,
        }
    }

    fn testlexikon() -> Lexikon {
        Lexikon::aus_eintraegen(
            "test.toml",
            vec![
                eintrag("Alpha", Sprache::De, "x", "SR 1 Art. 1"),
                eintrag("Beta", Sprache::De, "x", "SR 1 Art. 2"),
                eintrag("alpha", Sprache::Fr, "x", "SR 1 Art. 1"),
                eintrag("alfa", Sprache::It, "x", " SR 1 Art. 1 "),
                eintrag("Gamma", Sprache::De, "y", "SR 1 Art. 3"),
                eintrag("Delta", Sprache::De, "z", "SR 1 Art. 3"),
                eintrag("Epsilon", Sprache::Fr, "z", "SR 1 Art. 3"),
            ],
        )
    }

    fn testregister() -> Register {
        Register::aus_tsv(
            "register/test.tsv",
            "# titel: Testregister\n# herausgeber: Herausgeber\n# url: https://example.org/r\n# lizenz: frei\n# abgerufen: 2026-11-06\n# rohdatei-sha256: abc\n# filter: alle\n# spalten: name\tanzahl\nAnna\t5\nBeat\t7\n",
        )
        .expect("Testregister")
    }

    #[test]
    fn bestand_zaehlt_je_sprache_und_fundstelle() {
        let b = bestand(&testlexikon(), "x");
        assert_eq!((b.de, b.fr, b.it, b.gesamt()), (2, 1, 1, 4));
        assert_eq!(
            b.fundstellen, 2,
            "«SR 1 Art. 1» dreimal, davon einmal mit Leerzeichen"
        );
        assert_eq!(
            b.andere_regeln.into_iter().collect::<Vec<_>>(),
            vec![("y".to_string(), 1), ("z".to_string(), 2)]
        );
        // Gegenprobe: eine Regel ohne Einträge zählt nichts für sich.
        assert_eq!(bestand(&testlexikon(), "q").gesamt(), 0);
    }

    #[test]
    fn wortliste_und_register_als_text() {
        let mut aus = String::new();
        wortliste_text(&mut aus, "test.toml", &testlexikon(), "x", false);
        assert_eq!(
            aus,
            "  test.toml: Einträge 4 (de 2, fr 1, it 1), verschiedene Fundstellen 2; dazu\n    Einträge anderer Regeln: y 1, z 2\n"
        );
        let mut aus = String::new();
        register_text(&mut aus, "register/test.tsv", &testregister(), true);
        assert!(
            aus.starts_with("  register/test.tsv: register, rows 2\n"),
            "{aus}"
        );
        assert!(aus.contains("    Testregister; Herausgeber\n"), "{aus}");
        assert!(
            aus.contains("    retrieved 2026-11-06; licence: frei\n"),
            "{aus}"
        );
        assert!(aus.contains("    SHA-256 of the raw file: abc\n"), "{aus}");
        assert!(aus.ends_with("    https://example.org/r\n"), "{aus}");
    }

    #[test]
    fn fehlende_wortliste_wird_genannt() {
        let mut aus = String::new();
        lexikon_text(&mut aus, "gibt-es-nicht.toml", "x", false);
        assert_eq!(aus, "  gibt-es-nicht.toml: fehlt im Programm\n");
    }

    // --- Umbruch -------------------------------------------------------------------------

    #[test]
    fn absatz_bricht_bei_achtzig_spalten() {
        let text = "wort ".repeat(40);
        let mut aus = String::new();
        absatz(&mut aus, &text, "  - ", 4);
        for zeile in aus.lines() {
            assert!(zeile.chars().count() <= ZEILENBREITE, "{zeile}");
        }
        let zeilen: Vec<&str> = aus.lines().collect();
        assert!(zeilen[0].starts_with("  - wort"));
        assert!(zeilen[1].starts_with("    wort"));
        assert_eq!(aus.matches("wort").count(), 40, "kein Wort verloren");
    }

    // --- JSON gegen docs/AUSGABE.md -------------------------------------------------------

    /// Die dokumentierten Pfade unter der Überschrift `## {abschnitt}`.
    fn dokumentierte_pfade(abschnitt: &str) -> BTreeSet<String> {
        let mut drin = false;
        let mut pfade = BTreeSet::new();
        for zeile in DOKU.lines() {
            if let Some(titel) = zeile.strip_prefix("## ") {
                drin = titel.trim() == abschnitt;
                continue;
            }
            if !drin {
                continue;
            }
            if let Some((pfad, _)) = zeile.strip_prefix("| `").and_then(|r| r.split_once('`')) {
                pfade.insert(pfad.to_string());
            }
        }
        assert!(!pfade.is_empty(), "Abschnitt «## {abschnitt}» ohne Felder");
        pfade
    }

    /// Alle Pfade eines JSON-Werts: Objektfelder mit «.», Listenelemente mit «[]».
    fn pfade(wert: &Value, praefix: &str, aus: &mut BTreeSet<String>) {
        match wert {
            Value::Object(felder) => {
                for (k, v) in felder {
                    let p = if praefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{praefix}.{k}")
                    };
                    aus.insert(p.clone());
                    pfade(v, &p, aus);
                }
            }
            Value::Array(liste) => {
                for v in liste {
                    pfade(v, &format!("{praefix}[]"), aus);
                }
            }
            _ => {}
        }
    }

    fn pfade_von(json: &str) -> BTreeSet<String> {
        let wert: Value = serde_json::from_str(json).expect("gültiges JSON");
        let mut aus = BTreeSet::new();
        pfade(&wert, "", &mut aus);
        aus
    }

    fn fund(text: &Text, regel: &'static str, stelle: &str, stufe: Stufe, grund: Grund) -> Fund {
        let start = text.als_str().find(stelle).expect("Stelle im Text");
        let ende = start + stelle.len();
        let (zeile, spalte) = text.position(start);
        Fund {
            regel,
            stufe,
            start,
            ende,
            zeile,
            spalte,
            start_zeichen: text.zeichenversatz(start),
            ende_zeichen: text.zeichenversatz(ende),
            stelle: stelle.to_string(),
            grund,
            zusatz: None,
            beleg: None,
        }
    }

    /// Ein Bericht mit jeder Art von Grund, Zusatz und Beleg.
    fn musterbericht(lage: Lage) -> Bericht {
        let text = Text::neu(
            "Meine Mandantin Erika Beispiel ist arbeitsunfähig.\nAHV 756.1234.5678.97, Beat Muster, Diagnose",
        );
        let mut name = fund(
            &text,
            "name",
            "Erika Beispiel",
            Stufe::Rot,
            Grund::Geheimnis {
                rolle: Rolle::Lawyer,
                aus_text: Some("Meine Mandantin".into()),
                ziffer: Some(2),
                natuerliche_person: true,
            },
        );
        name.beleg = Some(Beleg {
            datei: "register/vornamen.tsv".into(),
            begriff: "Erika".into(),
            quelle: "BFS".into(),
            fassung: "2026-01-01".into(),
            url: "https://example.org".into(),
        });
        let mut ahv = fund(
            &text,
            "ahv",
            "756.1234.5678.97",
            Stufe::Gelb,
            Grund::Personendaten,
        );
        ahv.zusatz = Some(Zweisprachig::neu("Prüfziffer gültig", "check digit valid"));
        let funde = vec![
            fund(
                &text,
                "kontext",
                "Meine Mandantin",
                Stufe::Hinweis,
                Grund::Geheimniskontext {
                    rolle: Rolle::Lawyer,
                },
            ),
            name,
            fund(
                &text,
                "art5c-2",
                "arbeitsunfähig",
                Stufe::Orange,
                Grund::BesondersSchuetzenswert { ziffer: 2 },
            ),
            ahv,
            fund(
                &text,
                "ahv-sammlung",
                "AHV",
                Stufe::Hinweis,
                Grund::Regelhinweis,
            ),
            fund(
                &text,
                "name",
                "Beat Muster",
                Stufe::Hinweis,
                Grund::Lokal { sonst: Stufe::Gelb },
            ),
            fund(
                &text,
                "art5c-2",
                "Diagnose",
                Stufe::Hinweis,
                Grund::OhneIdentifikator,
            ),
        ];
        Bericht {
            text,
            funde,
            lage,
            optionen: Optionen {
                rolle: Some(Rolle::Lawyer),
                ziel: Ziel::Cloud,
            },
            rolle_quelle: crate::fund::RolleQuelle::Option,
        }
    }

    #[test]
    fn bericht_json_hat_genau_die_dokumentierten_felder() {
        let mut erzeugt = BTreeSet::new();
        for lage in [
            Lage::Keine,
            Lage::Angegeben {
                rolle: Rolle::Lawyer,
            },
            Lage::AusText {
                rolle: Rolle::Lawyer,
                stelle: "Meine Mandantin".into(),
                zeile: 1,
                spalte: 1,
            },
        ] {
            erzeugt.extend(pfade_von(&bericht_json(&musterbericht(lage))));
        }
        assert_eq!(erzeugt, dokumentierte_pfade("JSON: Bericht"));
    }

    #[test]
    fn regeln_json_hat_genau_die_dokumentierten_felder() {
        assert_eq!(
            pfade_von(&regeln_json()),
            dokumentierte_pfade("JSON: Regeln")
        );
    }

    fn musterregel(klasse: Klasse, guete: Guete) -> RegelInfo {
        RegelInfo {
            id: "art5c-9",
            klasse,
            titel: Txt::neu("Testregel", "test rule"),
            erkennt: Txt::neu("Erkennt etwas.", "Detects something."),
            methode: Txt::neu("Vergleicht.", "Compares."),
            hinweis: Some(Txt::neu("Hinweis.", "Note.")),
            grundlage: vec![&normen::DSG_5_A],
            quellen: vec![
                Quelle {
                    titel: "Quelle A",
                    url: "https://example.org/a",
                    lizenz: Some("frei"),
                    stand: Stand::Geprueft("2026-11-06"),
                },
                Quelle {
                    titel: "Quelle B",
                    url: "https://example.org/b",
                    lizenz: None,
                    stand: Stand::ZuPruefen("Fundstelle fehlt"),
                },
            ],
            lexika: vec!["gibt-es-nicht.toml"],
            guete,
            grenzen: vec![Txt::neu("Grenze.", "Limit.")],
        }
    }

    #[test]
    fn erklaerung_json_hat_genau_die_dokumentierten_felder() {
        let mut erzeugt = pfade_von(&erklaerung_json(&musterregel(
            Klasse::C,
            Guete::NichtGemessen,
        )));
        erzeugt.extend(pfade_von(&erklaerung_json(&musterregel(
            Klasse::A,
            Guete::Nachgewiesen(Txt::neu("nachgewiesen", "proven")),
        ))));
        // Eine Kontextregel zeigt die Punkte von ROT, darunter einen mit Literatur.
        erzeugt.extend(pfade_von(&erklaerung_json(&musterregel(
            Klasse::Kontext,
            Guete::NichtGemessen,
        ))));
        // Wortliste und Register gibt es in dieser Kopie nicht eingebettet; ihre
        // JSON-Form wird darum direkt geprüft, unter dem Pfad `lexika[]`.
        for variante in [
            wortliste_json("test.toml", &testlexikon(), "x"),
            register_json("register/test.tsv", &testregister()),
        ] {
            let wert = serde_json::to_value(&variante).expect("JSON");
            pfade(&wert, "lexika[]", &mut erzeugt);
        }
        assert_eq!(erzeugt, dokumentierte_pfade("JSON: Regelerklärung"));
    }

    #[test]
    fn stufen_json_hat_genau_die_dokumentierten_felder() {
        let mut erzeugt = BTreeSet::new();
        for s in STUFEN {
            erzeugt.extend(pfade_von(&stufen_erklaerung_json(s)));
        }
        assert_eq!(erzeugt, dokumentierte_pfade("JSON: Stufenerklärung"));
    }

    #[test]
    fn normzitate_je_grund() {
        let b = musterbericht(Lage::Keine);
        let zitate: Vec<Vec<String>> = b.funde.iter().map(normzitate).collect();
        assert_eq!(
            zitate[0],
            vec!["Art. 321 Ziff. 1 StGB", "Art. 13 Abs. 1 BGFA"],
            "Geheimnis-Kontext lawyer"
        );
        assert_eq!(
            zitate[1],
            vec![
                "Art. 321 Ziff. 1 StGB",
                "Art. 13 Abs. 1 BGFA",
                "Art. 5 lit. c Ziff. 2 DSG"
            ]
        );
        assert_eq!(
            zitate[2],
            vec!["Art. 5 lit. c Ziff. 2 DSG", "Art. 30 Abs. 2 lit. c DSG"]
        );
        assert_eq!(zitate[3], vec!["Art. 5 lit. a DSG"]);
        assert_eq!(zitate[4], vec!["Art. 153b AHVG", "Art. 153c Abs. 1 AHVG"]);
        assert_eq!(zitate[5], vec!["Art. 5 lit. e DSG"]);
        assert!(zitate[6].is_empty(), "ohne Identifikator keine Norm");
    }

    #[test]
    fn regelhinweis_ohne_text_gibt_keine_leere_zeile() {
        let text = Text::neu("abc");
        let f = fund(
            &text,
            "gibt-es-nicht",
            "abc",
            Stufe::Hinweis,
            Grund::Regelhinweis,
        );
        assert!(grund_zeilen(&f, false).is_empty());
        assert!(normzitate(&f).is_empty());
    }
}
