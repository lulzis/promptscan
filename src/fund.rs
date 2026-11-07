//! Gemeinsame Typen.
//!
//! Ein Erkenner meldet [`Treffer`]: eine Stelle im Text und welche Art von Angabe dort steht.
//! Erst das rechtliche Modell ([`crate::stufen`]) macht daraus [`Fund`]e mit Stufe und Grund.
//! Die Erkenner wissen nichts von Stufen; das hält die Rechtslogik an einer Stelle.

use serde::Serialize;

/// Stufe nach Abschnitt 4.2 der Spezifikation. Die Reihenfolge ist die Schwere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stufe {
    /// Keine Rechtsfolge, die das Werkzeug zuordnet.
    Hinweis,
    /// Personendaten, Art. 5 lit. a DSG.
    Gelb,
    /// Besonders schützenswerte Personendaten, Art. 5 lit. c DSG.
    Orange,
    /// Daten unter strafbewehrter Geheimhaltungspflicht.
    Rot,
}

impl Stufe {
    /// Name in der Ausgabe, in Grossbuchstaben wie in der Spezifikation.
    pub fn wort(self, englisch: bool) -> &'static str {
        match (self, englisch) {
            (Stufe::Hinweis, false) => "HINWEIS",
            (Stufe::Gelb, false) => "GELB",
            (Stufe::Orange, false) => "ORANGE",
            (Stufe::Rot, false) => "ROT",
            (Stufe::Hinweis, true) => "NOTE",
            (Stufe::Gelb, true) => "YELLOW",
            (Stufe::Orange, true) => "ORANGE",
            (Stufe::Rot, true) => "RED",
        }
    }
}

/// Berufsrolle nach Abschnitt 4.3. Die Namen sind die Werte von `--role`, ausser
/// [`Rolle::Offen`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rolle {
    None,
    Clergy,
    Lawyer,
    Notary,
    #[serde(rename = "patent-attorney")]
    PatentAttorney,
    Auditor,
    Doctor,
    Psychologist,
    Bank,
    /// Finanzinstitut nach Art. 2 Abs. 1 FINIG; Berufsgeheimnis nach Art. 69 FINIG.
    #[serde(rename = "financial-institution")]
    FinancialInstitution,
    Authority,
    Court,
    #[serde(rename = "criminal-authority")]
    CriminalAuthority,
    Student,
    Professional,
    /// Kein Wert von `--role`: Der Text zeigt einen Geheimnis-Kontext, aber nicht den
    /// Beruf, etwa «meine Klientin» (Anwältin, Psychologin, Sozialdienst, Bank). ROT bleibt
    /// die vorsichtige Stufe; die Ausgabe nennt die möglichen Normen und bietet die Rollen
    /// zur Auswahl an (SPEC 4.3).
    Offen,
}

impl Rolle {
    /// Die Werte von `--role`, ohne [`Rolle::Offen`].
    pub const ALLE: [Rolle; 15] = [
        Rolle::None,
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
        Rolle::Professional,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Rolle::None => "none",
            Rolle::Clergy => "clergy",
            Rolle::Lawyer => "lawyer",
            Rolle::Notary => "notary",
            Rolle::PatentAttorney => "patent-attorney",
            Rolle::Auditor => "auditor",
            Rolle::Doctor => "doctor",
            Rolle::Psychologist => "psychologist",
            Rolle::Bank => "bank",
            Rolle::FinancialInstitution => "financial-institution",
            Rolle::Authority => "authority",
            Rolle::Court => "court",
            Rolle::CriminalAuthority => "criminal-authority",
            Rolle::Student => "student",
            Rolle::Professional => "professional",
            Rolle::Offen => "offen",
        }
    }

    /// Anzeigename in Sätzen der Ausgabe, statt der englischen Kennung von `--role`:
    /// Paarform, wo die Rolle Personen benennt, keine bei Einrichtungen; `doctor` heisst
    /// «Gesundheitsberuf», weil die Rolle alle Gesundheitsberufe aus Art. 321 Ziff. 1 StGB
    /// trägt.
    pub fn anzeige(self, englisch: bool) -> &'static str {
        match (self, englisch) {
            (Rolle::None, false) => "keine",
            (Rolle::None, true) => "none",
            (Rolle::Clergy, false) => "Geistliche/Geistlicher",
            (Rolle::Clergy, true) => "member of the clergy",
            (Rolle::Lawyer, false) => "Anwältin/Anwalt",
            (Rolle::Lawyer, true) => "lawyer",
            (Rolle::Notary, false) => "Notarin/Notar",
            (Rolle::Notary, true) => "notary",
            (Rolle::PatentAttorney, false) => "Patentanwältin/Patentanwalt",
            (Rolle::PatentAttorney, true) => "patent attorney",
            (Rolle::Auditor, false) => "Revisorin/Revisor",
            (Rolle::Auditor, true) => "auditor",
            (Rolle::Doctor, false) => "Gesundheitsberuf",
            (Rolle::Doctor, true) => "health profession",
            (Rolle::Psychologist, false) => "Psychologin/Psychologe",
            (Rolle::Psychologist, true) => "psychologist",
            (Rolle::Bank, false) => "Bank",
            (Rolle::Bank, true) => "bank",
            (Rolle::FinancialInstitution, false) => "Finanzinstitut",
            (Rolle::FinancialInstitution, true) => "financial institution",
            (Rolle::Authority, false) => "Behörde",
            (Rolle::Authority, true) => "authority",
            (Rolle::Court, false) => "Gericht",
            (Rolle::Court, true) => "court",
            (Rolle::CriminalAuthority, false) => "Strafbehörde",
            (Rolle::CriminalAuthority, true) => "criminal justice authority",
            (Rolle::Student, false) => "Studentin/Student",
            (Rolle::Student, true) => "student",
            (Rolle::Professional, false) => "Beruf mit Schweigepflicht",
            (Rolle::Professional, true) => "professional",
            (Rolle::Offen, false) => "Beruf offen",
            (Rolle::Offen, true) => "profession open",
        }
    }

    /// Die Rolle zu einem Wert von `--role`; «offen» ist keiner.
    pub fn aus_name(name: &str) -> Option<Rolle> {
        Rolle::ALLE.into_iter().find(|r| r.name() == name)
    }

    /// Die Rolle zu einer Eingabe von `--role` oder `role` in der Konfiguration: die
    /// Kennung oder ein deutsches Eingabewort aus [`EINGABEWOERTER`], ohne Rücksicht auf
    /// Gross- und Kleinschreibung und Leerraum am Rand, in NFC («a» mit kombinierendem
    /// Trema gilt wie «ä»). «offen» ist keine Eingabe.
    pub fn aus_eingabe(eingabe: &str) -> Option<Rolle> {
        use unicode_normalization::UnicodeNormalization;
        let klein: String = eingabe.trim().nfc().collect::<String>().to_lowercase();
        if let Some(rolle) = Rolle::aus_name(&klein) {
            return Some(rolle);
        }
        EINGABEWOERTER
            .iter()
            .find(|(wort, _)| *wort == klein)
            .map(|(_, rolle)| *rolle)
    }
}

/// Deutsche Eingabewörter, die `--role` und `role` in der Konfiguration zusätzlich zu den
/// Kennungen annehmen, klein geschrieben. Nur Berufe, die die Norm der Rolle nennt;
/// Hilfspersonen wählen die Rolle der Person, für die sie arbeiten. Öffentlich, damit
/// `main.rs` bei einem Tippfehler die nächste Rolle vorschlagen kann.
pub const EINGABEWOERTER: &[(&str, Rolle)] = &[
    ("keine", Rolle::None),
    ("kein", Rolle::None),
    ("geistliche", Rolle::Clergy),
    ("geistlicher", Rolle::Clergy),
    ("pfarrer", Rolle::Clergy),
    ("pfarrerin", Rolle::Clergy),
    ("anwalt", Rolle::Lawyer),
    ("anwältin", Rolle::Lawyer),
    ("anwaeltin", Rolle::Lawyer),
    ("rechtsanwalt", Rolle::Lawyer),
    ("rechtsanwältin", Rolle::Lawyer),
    ("rechtsanwaeltin", Rolle::Lawyer),
    ("verteidiger", Rolle::Lawyer),
    ("verteidigerin", Rolle::Lawyer),
    ("advokat", Rolle::Lawyer),
    ("advokatin", Rolle::Lawyer),
    ("fürsprecher", Rolle::Lawyer),
    ("fuersprecher", Rolle::Lawyer),
    ("fürsprecherin", Rolle::Lawyer),
    ("fuersprecherin", Rolle::Lawyer),
    ("notar", Rolle::Notary),
    ("notarin", Rolle::Notary),
    ("patentanwalt", Rolle::PatentAttorney),
    ("patentanwältin", Rolle::PatentAttorney),
    ("patentanwaeltin", Rolle::PatentAttorney),
    ("revisor", Rolle::Auditor),
    ("revisorin", Rolle::Auditor),
    ("revisionsstelle", Rolle::Auditor),
    ("arzt", Rolle::Doctor),
    ("ärztin", Rolle::Doctor),
    ("aerztin", Rolle::Doctor),
    ("zahnarzt", Rolle::Doctor),
    ("zahnärztin", Rolle::Doctor),
    ("zahnaerztin", Rolle::Doctor),
    ("chiropraktor", Rolle::Doctor),
    ("chiropraktorin", Rolle::Doctor),
    ("apotheker", Rolle::Doctor),
    ("apothekerin", Rolle::Doctor),
    ("hebamme", Rolle::Doctor),
    ("pflegefachperson", Rolle::Doctor),
    ("pflegefachfrau", Rolle::Doctor),
    ("pflegefachmann", Rolle::Doctor),
    ("physiotherapeut", Rolle::Doctor),
    ("physiotherapeutin", Rolle::Doctor),
    ("ergotherapeut", Rolle::Doctor),
    ("ergotherapeutin", Rolle::Doctor),
    ("ernährungsberater", Rolle::Doctor),
    ("ernaehrungsberater", Rolle::Doctor),
    ("ernährungsberaterin", Rolle::Doctor),
    ("ernaehrungsberaterin", Rolle::Doctor),
    ("optometrist", Rolle::Doctor),
    ("optometristin", Rolle::Doctor),
    ("osteopath", Rolle::Doctor),
    ("osteopathin", Rolle::Doctor),
    ("psychologe", Rolle::Psychologist),
    ("psychologin", Rolle::Psychologist),
    ("psychotherapeut", Rolle::Psychologist),
    ("psychotherapeutin", Rolle::Psychologist),
    ("banker", Rolle::Bank),
    ("bankerin", Rolle::Bank),
    ("bankangestellte", Rolle::Bank),
    ("bankangestellter", Rolle::Bank),
    ("finanzinstitut", Rolle::FinancialInstitution),
    ("vermögensverwalter", Rolle::FinancialInstitution),
    ("vermoegensverwalter", Rolle::FinancialInstitution),
    ("vermögensverwalterin", Rolle::FinancialInstitution),
    ("vermoegensverwalterin", Rolle::FinancialInstitution),
    ("trustee", Rolle::FinancialInstitution),
    ("fondsleitung", Rolle::FinancialInstitution),
    ("wertpapierhaus", Rolle::FinancialInstitution),
    ("behörde", Rolle::Authority),
    ("behoerde", Rolle::Authority),
    ("verwaltung", Rolle::Authority),
    ("beamter", Rolle::Authority),
    ("beamtin", Rolle::Authority),
    ("gericht", Rolle::Court),
    ("richter", Rolle::Court),
    ("richterin", Rolle::Court),
    ("gerichtsschreiber", Rolle::Court),
    ("gerichtsschreiberin", Rolle::Court),
    ("strafbehörde", Rolle::CriminalAuthority),
    ("strafbehoerde", Rolle::CriminalAuthority),
    ("staatsanwaltschaft", Rolle::CriminalAuthority),
    ("staatsanwalt", Rolle::CriminalAuthority),
    ("staatsanwältin", Rolle::CriminalAuthority),
    ("staatsanwaeltin", Rolle::CriminalAuthority),
    ("polizei", Rolle::CriminalAuthority),
    ("strafgericht", Rolle::CriminalAuthority),
    ("studentin", Rolle::Student),
    ("studierende", Rolle::Student),
    ("studierender", Rolle::Student),
    ("beruf", Rolle::Professional),
    ("schweigepflicht", Rolle::Professional),
];

/// Woher die Rolle eines Berichts stammt; die Rolle aus dem Text steht in
/// [`crate::stufen::Lage::AusText`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RolleQuelle {
    /// Weder `--role` noch die Konfiguration nennen eine Rolle.
    #[default]
    Keine,
    /// Mit `--role` angegeben.
    Option,
    /// Aus der Konfigurationsdatei.
    Konfiguration,
}

/// Wohin der Prompt geht (Abschnitt 4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ziel {
    /// Ein Dienst ausserhalb dieses Rechners. Vorgabe.
    Cloud,
    /// Das Modell läuft auf diesem Rechner; nichts wird übermittelt (Art. 5 lit. e DSG).
    Local,
}

/// Was an einer Stelle steht, vor jeder rechtlichen Bewertung.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Art {
    /// Direkter Identifikator einer natürlichen Person (Abschnitt 4.4):
    /// `name`, `adresse`, `ahv`, `iban`, `tel`, `mail`.
    Identifikator,
    /// Angabe über eine Person, die allein niemanden bestimmt: `geburtsdatum`, `wohnort`.
    Personenangabe,
    /// Juristische Person (`firma`). Das DSG schützt sie nicht (Art. 5 lit. a DSG),
    /// ein Geheimnis nach Art. 320, 321 StGB, Art. 47 BankG oder Art. 69 FINIG schon
    /// (Abschnitt 4.3). Auch die KmGK zählt hier: Ihr unbeschränkt haftender Gesellschafter
    /// ist eine Aktiengesellschaft (Art. 98 Abs. 2 KAG).
    JuristischePerson,
    /// Kollektiv- oder Kommanditgesellschaft (`firma`): keine juristische Person; die
    /// unbeschränkt haftenden Gesellschafter sind natürliche Personen (Art. 552 Abs. 1,
    /// Art. 594 Abs. 2 OR). Wie eine Personenangabe: ohne Identifikator HINWEIS, mit
    /// Identifikator GELB; im Geheimnis-Kontext ROT wie die juristische Person.
    Personengesellschaft,
    /// Besonders schützenswerte Personendaten, Ziffer 1 bis 6 von Art. 5 lit. c DSG.
    BesondersSchuetzenswert(u8),
    /// Der Text zeigt einen Geheimnis-Kontext dieser Rolle, z. B. «meine Mandantin».
    /// Zählt zugleich als Personenbezug.
    Geheimniskontext(Rolle),
    /// Bezug auf eine Person ohne Identifikator, z. B. «die beschuldigte Person».
    /// Wird nie selbst angezeigt; er entscheidet nur, ob ein Inhalt ohne Identifikator
    /// einen Hinweis erhält (Abschnitt 4.4).
    Personenbezug,
    /// Hinweis ohne Rechtsfolge: `vermerk`, `fundstellen`.
    Hinweis,
}

/// Ein Text in beiden Ausgabesprachen.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Zweisprachig {
    pub de: String,
    pub en: String,
}

impl Zweisprachig {
    pub fn neu(de: impl Into<String>, en: impl Into<String>) -> Self {
        Zweisprachig {
            de: de.into(),
            en: en.into(),
        }
    }

    pub fn in_sprache(&self, englisch: bool) -> &str {
        if englisch { &self.en } else { &self.de }
    }
}

/// Herkunft eines Treffers aus einer Wortliste: der Eintrag und seine Quelle.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Beleg {
    /// Datei unter `lexika/`.
    pub datei: String,
    /// Der Begriff des Eintrags (Lemma), nicht die gefundene Form.
    pub begriff: String,
    pub quelle: String,
    pub fassung: String,
    pub url: String,
}

/// Meldung eines Erkenners.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Treffer {
    /// Kennung der Regel, z. B. `"ahv"` oder `"art5c-2"`.
    pub regel: &'static str,
    /// Byte-Versatz in [`crate::text::Text::als_str`] (NFC-normalisiert).
    pub start: usize,
    /// Byte-Versatz, exklusiv.
    pub ende: usize,
    pub art: Art,
    /// Kurzer Zusatz hinter der Stelle, z. B. «Prüfziffer gültig».
    pub zusatz: Option<Zweisprachig>,
    pub beleg: Option<Beleg>,
}

impl Treffer {
    pub fn neu(regel: &'static str, start: usize, ende: usize, art: Art) -> Self {
        Treffer {
            regel,
            start,
            ende,
            art,
            zusatz: None,
            beleg: None,
        }
    }

    pub fn mit_zusatz(mut self, de: &str, en: &str) -> Self {
        self.zusatz = Some(Zweisprachig::neu(de, en));
        self
    }

    pub fn mit_beleg(mut self, beleg: Beleg) -> Self {
        self.beleg = Some(beleg);
        self
    }
}

/// Warum ein Fund seine Stufe hat. Die Ausgabe macht daraus Text mit Normen.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
pub enum Grund {
    /// GELB: Personendaten, Art. 5 lit. a DSG.
    Personendaten,
    /// ORANGE: Art. 5 lit. c Ziff. `ziffer` DSG; Bekanntgabe an Dritte Art. 30 Abs. 2 lit. c DSG.
    BesondersSchuetzenswert { ziffer: u8 },
    /// ROT: strafbewehrte Geheimhaltungspflicht der Rolle. `aus_text` nennt die Textstelle,
    /// wenn die Rolle aus dem Text erschlossen und nicht angegeben wurde. `ziffer` ist gesetzt,
    /// wenn die Angabe zugleich besonders schützenswert ist und der Prompt einen
    /// Identifikator einer natürlichen Person enthält. `natuerliche_person` sagt, ob der Fund
    /// eine natürliche Person betrifft; nur dann gelten Normen des DSG (Art. 62 DSG).
    Geheimnis {
        rolle: Rolle,
        aus_text: Option<String>,
        ziffer: Option<u8>,
        #[serde(skip)]
        natuerliche_person: bool,
    },
    /// HINWEIS: Inhalt oder Angabe mit Personenbezug, aber ohne direkten Identifikator im Prompt.
    OhneIdentifikator,
    /// HINWEIS: Der Text zeigt einen Geheimnis-Kontext; er hat Funde auf ROT gehoben.
    Geheimniskontext { rolle: Rolle },
    /// HINWEIS: `--target local`; ohne lokales Modell hätte der Fund diese Stufe.
    Lokal { sonst: Stufe },
    /// HINWEIS einer Hinweisregel (`vermerk`, `fundstellen`, `ahv-sammlung`).
    Regelhinweis,
}

/// Ein bewerteter Fund, bereit für die Ausgabe.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Fund {
    pub regel: &'static str,
    pub stufe: Stufe,
    /// Byte-Versatz im NFC-Text.
    #[serde(skip)]
    pub start: usize,
    #[serde(skip)]
    pub ende: usize,
    /// 1-basiert.
    pub zeile: usize,
    /// 1-basiert, in Zeichen (Unicode-Skalarwerte).
    pub spalte: usize,
    /// Versatz in Zeichen im NFC-Text, 0-basiert, für Auswertungen.
    pub start_zeichen: usize,
    pub ende_zeichen: usize,
    /// Die markierte Stelle.
    pub stelle: String,
    pub grund: Grund,
    pub zusatz: Option<Zweisprachig>,
    pub beleg: Option<Beleg>,
}
