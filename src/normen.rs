//! Die Normen, auf denen die Stufen beruhen, mit amtlichem Wortlaut.
//!
//! Abgerufen am 05.11.2026 von Fedlex (fedlex.data.admin.ch), geltende Fassung; die
//! Absatznummern am 06.11.2026 am HTML der Fassung nachgeprüft. Art. 2, 31, 34 und 36 DSG,
//! Art. 69 FINIG, Art. 328b, 342, 552, 568 und 594 OR, Art. 98 KAG und die vollständigen
//! Fassungen von Art. 320 Ziff. 1 und
//! Art. 321 Ziff. 1 StGB sowie Art. 13 Abs. 1 BGFA am 06.11.2026 gelesen. Gesetze sind nach
//! Art. 5 Abs. 1 lit. a URG urheberrechtlich nicht geschützt; die Auszüge sind zulässig.
//! Wörtliche Auszüge stehen in «»; «…» markiert Kürzungen. Endet ein Auszug vor dem Ende
//! der zitierten Einheit, endet er mit «…» (Test `auszug_vollstaendig_oder_mit_auslassung`).
//!
//! Die Normen der Endfassung der Rechtstexte (Runde 2, ab [`DSG_5_D`]) stammen mit Zitat,
//! Fassung, Fundstelle und Wortlaut aus deren Liste `normen_neu`; dazu [`DSG_34_4`] und
//! [`DSG_36_2`], die ihr Text «Geltungsbereich» zitiert. Jeder Wortlaut ist am 06.11.2026
//! mit `fedlex.mjs` gegen die genannte Fassung geprüft: jedes Stück zwischen «…» steht so
//! im Erlass, und ein Auszug ohne «…» am Schluss reicht bis zum Ende der Einheit.

/// Eine zitierte Norm.
#[derive(Debug, PartialEq, Eq)]
pub struct Norm {
    /// Zitierweise, z. B. «Art. 5 lit. a DSG».
    pub zitat: &'static str,
    /// Erlass und SR-Nummer, z. B. «DSG, SR 235.1».
    pub erlass: &'static str,
    /// Datum der abgerufenen Fassung.
    pub fassung: &'static str,
    /// Öffentlich lesbare Fundstelle.
    pub url: &'static str,
    /// Wörtlicher Auszug, gekürzt mit «…».
    pub wortlaut: &'static str,
}

/// Erlasse, deren Normen eine Rechtsfolge oder Pflicht tragen, an die promptscan anknüpft
/// (recht-normen-8, Rechtstext B5 «Rechtlicher Bezug»): Datenschutz, Geheimhaltungspflichten,
/// AHV-Nummer, Informationssicherheit. Normen anderer Erlasse beschreiben, woran ein
/// Erkenner eine Angabe erkennt («Grundlage der Erkennung»): Nummernformate (AHVV, AEFV,
/// TAV), Adressen und Orte (GeoNV, VPG, ZGB), Rechtsformen (OR, HRegV, KAG), Fundstellen
/// (BGG, BGerR).
pub const BEZUG_ERLASSE: &[&str] = &[
    "DSG", "StGB", "BGFA", "BankG", "FINIG", "FinfraG", "StPO", "AHVG", "ISG",
];

/// Normen des OR, die eine Pflicht oder die Bearbeitung von Daten regeln und darum zum
/// rechtlichen Bezug gehören; die übrigen Artikel des OR (Rechtsformen der Regel firma)
/// sind Grundlage der Erkennung.
pub const BEZUG_OR: &[&str] = &[
    "Art. 321a Abs. 4 OR",
    "Art. 328b OR",
    "Art. 342 Abs. 1 lit. a OR",
    "Art. 730b Abs. 2 OR",
];

/// Gehört die Norm zum rechtlichen Bezug (sonst zur Grundlage der Erkennung)?
pub fn ist_bezug(n: &Norm) -> bool {
    let erlass = n.erlass.split(',').next().unwrap_or("").trim();
    BEZUG_ERLASSE.contains(&erlass) || BEZUG_OR.contains(&n.zitat)
}

/// Geltungsbereich: private Personen und Bundesorgane, nicht kantonale Organe.
pub const DSG_2_1: Norm = Norm {
    zitat: "Art. 2 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_2",
    wortlaut: "Dieses Gesetz gilt für die Bearbeitung von Personendaten natürlicher Personen durch: a. private Personen; b. Bundesorgane.",
};

/// Gerichtsverfahren und Verfahren nach bundesrechtlichen Verfahrensordnungen.
pub const DSG_2_3: Norm = Norm {
    zitat: "Art. 2 Abs. 3 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_2",
    wortlaut: "Das anwendbare Verfahrensrecht regelt die Bearbeitung von Personendaten und die Rechte der betroffenen Personen in Gerichtsverfahren und in Verfahren nach bundesrechtlichen Verfahrensordnungen. Auf erstinstanzliche Verwaltungsverfahren sind die Bestimmungen dieses Gesetzes anwendbar.",
};

pub const DSG_5_A: Norm = Norm {
    zitat: "Art. 5 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
    wortlaut: "Personendaten: alle Angaben, die sich auf eine bestimmte oder bestimmbare natürliche Person beziehen",
};

pub const DSG_5_C: Norm = Norm {
    zitat: "Art. 5 lit. c DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
    wortlaut: "besonders schützenswerte Personendaten: 1. Daten über religiöse, weltanschauliche, politische oder gewerkschaftliche Ansichten oder Tätigkeiten, 2. Daten über die Gesundheit, die Intimsphäre oder die Zugehörigkeit zu einer Rasse oder Ethnie, 3. genetische Daten, 4. biometrische Daten, die eine natürliche Person eindeutig identifizieren, 5. Daten über verwaltungs- und strafrechtliche Verfolgungen oder Sanktionen, 6. Daten über Massnahmen der sozialen Hilfe",
};

pub const DSG_5_E: Norm = Norm {
    zitat: "Art. 5 lit. e DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
    wortlaut: "Bekanntgeben: das Übermitteln oder Zugänglichmachen von Personendaten",
};

pub const DSG_6_2: Norm = Norm {
    zitat: "Art. 6 Abs. 2 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_6",
    wortlaut: "Die Bearbeitung muss nach Treu und Glauben erfolgen und verhältnismässig sein.",
};

pub const DSG_6_7_A: Norm = Norm {
    zitat: "Art. 6 Abs. 7 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_6",
    wortlaut: "Die Einwilligung muss ausdrücklich erfolgen für: a. die Bearbeitung von besonders schützenswerten Personendaten; …",
};

pub const DSG_9_1_B: Norm = Norm {
    zitat: "Art. 9 Abs. 1 lit. b DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_9",
    wortlaut: "Die Bearbeitung von Personendaten kann … einem Auftragsbearbeiter übertragen werden, wenn: … b. keine gesetzliche oder vertragliche Geheimhaltungspflicht die Übertragung verbietet.",
};

pub const DSG_30_1: Norm = Norm {
    zitat: "Art. 30 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_30",
    wortlaut: "Wer Personendaten bearbeitet, darf die Persönlichkeit der betroffenen Personen nicht widerrechtlich verletzen.",
};

pub const DSG_30_2_C: Norm = Norm {
    zitat: "Art. 30 Abs. 2 lit. c DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_30",
    wortlaut: "Eine Persönlichkeitsverletzung liegt insbesondere vor, wenn: … c. Dritten besonders schützenswerte Personendaten bekanntgegeben werden.",
};

/// Rechtfertigungsgründe im 5. Kapitel (Datenbearbeitung durch private Personen).
pub const DSG_31_1: Norm = Norm {
    zitat: "Art. 31 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_31",
    wortlaut: "Eine Persönlichkeitsverletzung ist widerrechtlich, wenn sie nicht durch Einwilligung der betroffenen Person, durch ein überwiegendes privates oder öffentliches Interesse oder durch Gesetz gerechtfertigt ist.",
};

/// Bundesorgane: Bearbeiten nur mit gesetzlicher Grundlage (6. Kapitel).
pub const DSG_34_1: Norm = Norm {
    zitat: "Art. 34 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_34",
    wortlaut: "Bundesorgane dürfen Personendaten nur bearbeiten, wenn dafür eine gesetzliche Grundlage besteht.",
};

/// Bundesorgane: Bearbeiten ohne gesetzliche Grundlage, ausnahmsweise (6. Kapitel).
pub const DSG_34_4: Norm = Norm {
    zitat: "Art. 34 Abs. 4 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_34",
    wortlaut: "In Abweichung von den Absätzen 1–3 dürfen Bundesorgane Personendaten bearbeiten, wenn eine der folgenden Voraussetzungen erfüllt ist: …",
};

/// Bundesorgane: Bekanntgeben nur mit gesetzlicher Grundlage (6. Kapitel).
pub const DSG_36_1: Norm = Norm {
    zitat: "Art. 36 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_36",
    wortlaut: "Bundesorgane dürfen Personendaten nur bekanntgeben, wenn dafür eine gesetzliche Grundlage nach Artikel 34 Absätze 1–3 besteht.",
};

/// Bundesorgane: Bekanntgeben im Einzelfall ohne gesetzliche Grundlage (6. Kapitel).
pub const DSG_36_2: Norm = Norm {
    zitat: "Art. 36 Abs. 2 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_36",
    wortlaut: "Sie dürfen Personendaten in Abweichung von Absatz 1 im Einzelfall bekanntgeben, wenn eine der folgenden Voraussetzungen erfüllt ist: …",
};

pub const DSG_62_1: Norm = Norm {
    zitat: "Art. 62 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_62",
    wortlaut: "Wer geheime Personendaten vorsätzlich offenbart, von denen sie oder er bei der Ausübung ihres oder seines Berufes, der die Kenntnis solcher Daten erfordert, Kenntnis erlangt hat, wird auf Antrag mit Busse bis zu 250 000 Franken bestraft.",
};

pub const DSG_62_2: Norm = Norm {
    zitat: "Art. 62 Abs. 2 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_62",
    wortlaut: "Gleich wird bestraft, wer vorsätzlich geheime Personendaten offenbart, von denen sie oder er bei der Tätigkeit für eine geheimhaltungspflichtige Person oder während der Ausbildung bei dieser Kenntnis erlangt hat.",
};

/// Fabrikations- und Geschäftsgeheimnis. Ob eine Firma eines hat, entscheidet promptscan
/// nicht; kein Fund nennt die Norm (SPEC 4.3).
pub const STGB_162: Norm = Norm {
    zitat: "Art. 162 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_162",
    wortlaut: "Wer ein Fabrikations- oder Geschäftsgeheimnis, das er infolge einer gesetzlichen oder vertraglichen Pflicht bewahren sollte, verrät, wer den Verrat für sich oder einen andern ausnützt, wird, auf Antrag, mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe bestraft.",
};

pub const STGB_320_1: Norm = Norm {
    zitat: "Art. 320 Ziff. 1 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_320",
    wortlaut: "Wer ein Geheimnis offenbart, das ihm in seiner Eigenschaft als Mitglied einer Behörde oder als Beamter anvertraut worden ist oder das er in seiner amtlichen oder dienstlichen Stellung oder als Hilfsperson eines Beamten oder einer Behörde wahrgenommen hat, wird mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe bestraft. Die Verletzung des Amtsgeheimnisses ist auch nach Beendigung des amtlichen oder dienstlichen Verhältnisses oder der Hilfstätigkeit strafbar.",
};

pub const STGB_320_2: Norm = Norm {
    zitat: "Art. 320 Ziff. 2 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_320",
    wortlaut: "Der Täter ist nicht strafbar, wenn er das Geheimnis mit schriftlicher Einwilligung seiner vorgesetzten Behörde offenbart hat.",
};

pub const STGB_321_1: Norm = Norm {
    zitat: "Art. 321 Ziff. 1 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_321",
    wortlaut: "Geistliche, Rechtsanwälte, Verteidiger, Notare, Patentanwälte, nach Obligationenrecht zur Verschwiegenheit verpflichtete Revisoren, Ärzte, Zahnärzte, Chiropraktoren, Apotheker, Hebammen, Psychologen, Pflegefachpersonen, Physiotherapeuten, Ergotherapeuten, Ernährungsberater, Optometristen, Osteopathen sowie ihre Hilfspersonen, die ein Geheimnis offenbaren, das ihnen infolge ihres Berufes anvertraut worden ist oder das sie in dessen Ausübung wahrgenommen haben, werden, auf Antrag, mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe bestraft. Ebenso werden Studierende bestraft, die ein Geheimnis offenbaren, das sie bei ihrem Studium wahrnehmen. Die Verletzung des Berufsgeheimnisses ist auch nach Beendigung der Berufsausübung oder der Studien strafbar.",
};

pub const STGB_321_1_ABS2: Norm = Norm {
    zitat: "Art. 321 Ziff. 1 Abs. 2 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_321",
    wortlaut: "Ebenso werden Studierende bestraft, die ein Geheimnis offenbaren, das sie bei ihrem Studium wahrnehmen.",
};

pub const STGB_321_2: Norm = Norm {
    zitat: "Art. 321 Ziff. 2 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_321",
    wortlaut: "Der Täter ist nicht strafbar, wenn er das Geheimnis auf Grund einer Einwilligung des Berechtigten oder einer auf Gesuch des Täters erteilten schriftlichen Bewilligung der vorgesetzten Behörde oder Aufsichtsbehörde offenbart hat.",
};

pub const BGFA_13_1: Norm = Norm {
    zitat: "Art. 13 Abs. 1 BGFA",
    erlass: "BGFA, SR 935.61",
    fassung: "2025-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_13",
    wortlaut: "Anwältinnen und Anwälte unterstehen zeitlich unbegrenzt und gegenüber jedermann dem Berufsgeheimnis über alles, was ihnen infolge ihres Berufes von ihrer Klientschaft anvertraut worden ist. Die Entbindung verpflichtet sie nicht zur Preisgabe von Anvertrautem.",
};

pub const BANKG_47_1_A: Norm = Norm {
    zitat: "Art. 47 Abs. 1 lit. a BankG",
    erlass: "BankG, SR 952.0",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/51/117_121_129/de#art_47",
    wortlaut: "Mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe wird bestraft, wer vorsätzlich: a. ein Geheimnis offenbart, das ihm in seiner Eigenschaft als Organ, Angestellter, Beauftragter oder Liquidator einer Bank … anvertraut worden ist oder das er in dieser Eigenschaft wahrgenommen hat; …",
};

pub const BANKG_47_2: Norm = Norm {
    zitat: "Art. 47 Abs. 2 BankG",
    erlass: "BankG, SR 952.0",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/51/117_121_129/de#art_47",
    wortlaut: "Wer fahrlässig handelt, wird mit Busse bis zu 250 000 Franken bestraft.",
};

/// Berufsgeheimnis der Finanzinstitute nach Art. 2 Abs. 1 FINIG.
pub const FINIG_69_1_A: Norm = Norm {
    zitat: "Art. 69 Abs. 1 lit. a FINIG",
    erlass: "FINIG, SR 954.1",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2018/801/de#art_69",
    wortlaut: "Mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe wird bestraft, wer vorsätzlich: a. ein Geheimnis offenbart, das ihr oder ihm in der Eigenschaft als Organ, als Angestellte oder Angestellter, als Beauftragte oder Beauftragter oder als Liquidatorin oder Liquidator eines Finanzinstituts anvertraut worden ist oder das sie oder er in dieser Eigenschaft wahrgenommen hat; …",
};

pub const FINIG_69_3: Norm = Norm {
    zitat: "Art. 69 Abs. 3 FINIG",
    erlass: "FINIG, SR 954.1",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2018/801/de#art_69",
    wortlaut: "Wer fahrlässig handelt, wird mit Busse bis zu 250 000 Franken bestraft.",
};

/// Berufsgeheimnis der Finanzmarktinfrastrukturen. Keine Rolle von `--role` nennt es;
/// explain rot sagt, dass es nicht abgedeckt ist (SPEC 4.3).
pub const FINFRAG_147_1_A: Norm = Norm {
    zitat: "Art. 147 Abs. 1 lit. a FinfraG",
    erlass: "FinfraG, SR 958.1",
    fassung: "2024-02-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2015/853/de#art_147",
    wortlaut: "Mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe wird bestraft, wer vorsätzlich: a. ein Geheimnis offenbart, das ihm oder ihr in seiner oder ihrer Eigenschaft als Organ, Angestellte oder Angestellter, Beauftragte oder Beauftragter oder Liquidatorin oder Liquidator einer Finanzmarktinfrastruktur anvertraut worden ist oder das sie oder er in dieser Eigenschaft wahrgenommen hat; …",
};

/// Private Arbeitgeber: Daten über Arbeitnehmende.
pub const OR_328B: Norm = Norm {
    zitat: "Art. 328b OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_328_b",
    wortlaut: "Der Arbeitgeber darf Daten über den Arbeitnehmer nur bearbeiten, soweit sie dessen Eignung für das Arbeitsverhältnis betreffen oder zur Durchführung des Arbeitsvertrages erforderlich sind. Im Übrigen gelten die Bestimmungen des Datenschutzgesetzes vom 25. September 2020.",
};

/// Öffentlich-rechtliche Dienstverhältnisse: das Personalrecht geht vor.
pub const OR_342_1_A: Norm = Norm {
    zitat: "Art. 342 Abs. 1 lit. a OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_342",
    wortlaut: "Vorbehalten bleiben: a. Vorschriften des Bundes, der Kantone und Gemeinden über das öffentlich-rechtliche Dienstverhältnis, …",
};

/// Kollektivgesellschaft: natürliche Personen, unbeschränkt haftend.
pub const OR_552_1: Norm = Norm {
    zitat: "Art. 552 Abs. 1 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_552",
    wortlaut: "Die Kollektivgesellschaft ist eine Gesellschaft, in der zwei oder mehrere natürliche Personen, ohne Beschränkung ihrer Haftung gegenüber den Gesellschaftsgläubigern, sich zum Zwecke vereinigen, unter einer gemeinsamen Firma ein Handels-, ein Fabrikations- oder ein anderes nach kaufmännischer Art geführtes Gewerbe zu betreiben.",
};

/// Kollektivgesellschaft: persönliche Haftung der Gesellschafter.
pub const OR_568_1: Norm = Norm {
    zitat: "Art. 568 Abs. 1 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_568",
    wortlaut: "Die Gesellschafter haften für alle Verbindlichkeiten der Gesellschaft solidarisch und mit ihrem ganzen Vermögen.",
};

/// Kommanditgesellschaft: unbeschränkt haftende Gesellschafter sind natürliche Personen.
pub const OR_594_2: Norm = Norm {
    zitat: "Art. 594 Abs. 2 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_594",
    wortlaut: "Unbeschränkt haftende Gesellschafter können nur natürliche Personen, Kommanditäre jedoch auch juristische Personen und Handelsgesellschaften sein.",
};

/// KmGK: Der Komplementär ist eine Aktiengesellschaft.
pub const KAG_98_2: Norm = Norm {
    zitat: "Art. 98 Abs. 2 KAG",
    erlass: "KAG, SR 951.31",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2006/822/de#art_98",
    wortlaut: "Komplementäre müssen Aktiengesellschaften mit Sitz in der Schweiz sein. …",
};

pub const STPO_73_1: Norm = Norm {
    zitat: "Art. 73 Abs. 1 StPO",
    erlass: "StPO, SR 312.0",
    fassung: "2025-04-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_73",
    wortlaut: "Die Mitglieder von Strafbehörden, ihre Mitarbeiterinnen und Mitarbeiter sowie die von Strafbehörden ernannten Sachverständigen bewahren Stillschweigen hinsichtlich Tatsachen, die ihnen in Ausübung ihrer amtlichen Tätigkeit zur Kenntnis gelangt sind.",
};

pub const AHVG_153B: Norm = Norm {
    zitat: "Art. 153b AHVG",
    erlass: "AHVG, SR 831.10",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_153_b",
    wortlaut: "Die Verwendung der AHV-Nummer nach Artikel 50c gilt als systematisch, wenn die ganze AHV-Nummer, ein Teil davon oder eine geänderte Form dieser Nummer mit Personendaten verbunden wird und diese Daten in strukturierter Form gesammelt werden.",
};

pub const URG_5_1_A: Norm = Norm {
    zitat: "Art. 5 Abs. 1 lit. a URG",
    erlass: "URG, SR 231.1",
    fassung: "2025-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/1993/1798_1798_1798/de#art_5",
    wortlaut: "Durch das Urheberrecht nicht geschützt sind: a. Gesetze, Verordnungen, völkerrechtliche Verträge und andere amtliche Erlasse; …",
};

pub const BV_70_1: Norm = Norm {
    zitat: "Art. 70 Abs. 1 BV",
    erlass: "BV, SR 101",
    fassung: "2024-03-03",
    url: "https://www.fedlex.admin.ch/eli/cc/1999/404/de#art_70",
    wortlaut: "Die Amtssprachen des Bundes sind Deutsch, Französisch und Italienisch. Im Verkehr mit Personen rätoromanischer Sprache ist auch das Rätoromanische Amtssprache des Bundes.",
};

// --- Endfassung der Rechtstexte (Runde 2), Liste `normen_neu` ---------------------------

/// Begriff des Bearbeitens (Rechtstext B3, `--target local`).
pub const DSG_5_D: Norm = Norm {
    zitat: "Art. 5 lit. d DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
    wortlaut: "Bearbeiten: jeder Umgang mit Personendaten, unabhängig von den angewandten Mitteln und Verfahren, insbesondere das Beschaffen, Speichern, Aufbewahren, Verwenden, Verändern, Bekanntgeben, Archivieren, Löschen oder Vernichten von Daten",
};

/// Verantwortlicher und Auftragsbearbeiter (Rechtstexte A5, B2 «auftragsbearbeitung»).
pub const DSG_5_J_K: Norm = Norm {
    zitat: "Art. 5 lit. j und k DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_5",
    wortlaut: "j. Verantwortlicher: private Person oder Bundesorgan, die oder das allein oder zusammen mit anderen über den Zweck und die Mittel der Bearbeitung entscheidet; k. Auftragsbearbeiter: private Person oder Bundesorgan, die oder das im Auftrag des Verantwortlichen Personendaten bearbeitet.",
};

pub const DSG_9_1_A: Norm = Norm {
    zitat: "Art. 9 Abs. 1 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_9",
    wortlaut: "Die Bearbeitung von Personendaten kann vertraglich oder durch die Gesetzgebung einem Auftragsbearbeiter übertragen werden, wenn: a. die Daten so bearbeitet werden, wie der Verantwortliche selbst es tun dürfte; …",
};

pub const DSG_9_2: Norm = Norm {
    zitat: "Art. 9 Abs. 2 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_9",
    wortlaut: "Der Verantwortliche muss sich insbesondere vergewissern, dass der Auftragsbearbeiter in der Lage ist, die Datensicherheit zu gewährleisten.",
};

pub const DSG_9_3: Norm = Norm {
    zitat: "Art. 9 Abs. 3 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_9",
    wortlaut: "Der Auftragsbearbeiter darf die Bearbeitung nur mit vorgängiger Genehmigung des Verantwortlichen einem Dritten übertragen.",
};

pub const DSG_16_1: Norm = Norm {
    zitat: "Art. 16 Abs. 1 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_16",
    wortlaut: "Personendaten dürfen ins Ausland bekanntgegeben werden, wenn der Bundesrat festgestellt hat, dass die Gesetzgebung des betreffenden Staates oder das internationale Organ einen angemessenen Schutz gewährleistet.",
};

pub const DSG_16_2: Norm = Norm {
    zitat: "Art. 16 Abs. 2 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_16",
    wortlaut: "Liegt kein Entscheid des Bundesrates nach Absatz 1 vor, so dürfen Personendaten ins Ausland bekanntgegeben werden, wenn ein geeigneter Datenschutz gewährleistet wird durch: … b. Datenschutzklauseln in einem Vertrag zwischen dem Verantwortlichen oder dem Auftragsbearbeiter und seiner Vertragspartnerin oder seinem Vertragspartner, die dem EDÖB vorgängig mitgeteilt wurden; … d. Standarddatenschutzklauseln, die der EDÖB vorgängig genehmigt, ausgestellt oder anerkannt hat; …",
};

pub const DSG_17_1_A: Norm = Norm {
    zitat: "Art. 17 Abs. 1 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_17",
    wortlaut: "Abweichend von Artikel 16 Absätze 1 und 2 dürfen in den folgenden Fällen Personendaten ins Ausland bekanntgegeben werden: a. Die betroffene Person hat ausdrücklich in die Bekanntgabe eingewilligt. …",
};

pub const DSG_61_A: Norm = Norm {
    zitat: "Art. 61 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_61",
    wortlaut: "Mit Busse bis zu 250 000 Franken werden private Personen auf Antrag bestraft, die vorsätzlich: a. unter Verstoss gegen Artikel 16 Absätze 1 und 2 und ohne dass die Voraussetzungen nach Artikel 17 erfüllt sind, Personendaten ins Ausland bekanntgeben; …",
};

pub const DSG_61_B: Norm = Norm {
    zitat: "Art. 61 lit. b DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_61",
    wortlaut: "Mit Busse bis zu 250 000 Franken werden private Personen auf Antrag bestraft, die vorsätzlich: … b. die Datenbearbeitung einem Auftragsbearbeiter übergeben, ohne dass die Voraussetzungen nach Artikel 9 Absätze 1 und 2 erfüllt sind; …",
};

/// Angemessener Schutz in den USA nur für zertifizierte Organisationen.
pub const DSV_ANHANG1_44: Norm = Norm {
    zitat: "Anhang 1 Ziff. 44 DSV",
    erlass: "DSV, SR 235.11",
    fassung: "2025-12-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/568/de#annex_1",
    wortlaut: "Vereinigte Staaten*** Für Personendaten, die von Organisationen bearbeitet werden, die gemäss den Grundsätzen des Datenschutzrahmens zwischen der Schweiz und den USA zertifiziert sind, gilt ein angemessenes Schutzniveau als gewährleistet, …",
};

pub const AHVG_49: Norm = Norm {
    zitat: "Art. 49 AHVG",
    erlass: "AHVG, SR 831.10",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_49",
    wortlaut: "Die Durchführung der Alters- und Hinterlassenenversicherung erfolgt unter der Aufsicht des Bundes (Art. 76 ATSG) durch die Arbeitgeber und Arbeitnehmer sowie durch die Verbandsausgleichskassen, kantonalen Ausgleichskassen, Ausgleichskassen des Bundes und eine zentrale Ausgleichsstelle (Durchführungsstellen).",
};

pub const AHVG_51_3: Norm = Norm {
    zitat: "Art. 51 Abs. 3 AHVG",
    erlass: "AHVG, SR 831.10",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_51",
    wortlaut: "… Sie rechnen mit der Ausgleichskasse über die abgezogenen und die selbst geschuldeten Beiträge sowie über die ausbezahlten Renten und Hilflosenentschädigungen periodisch ab und machen die erforderlichen Angaben für die Führung der individuellen Konten der Arbeitnehmer.",
};

/// Wer die AHV-Nummer systematisch verwenden darf (ersetzt das frühere `AHVG_153C`).
pub const AHVG_153C_1: Norm = Norm {
    zitat: "Art. 153c Abs. 1 AHVG",
    erlass: "AHVG, SR 831.10",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_153_c",
    wortlaut: "Nur folgende Behörden, Organisationen und Personen sind berechtigt, die AHV-Nummer systematisch zu verwenden: a. soweit sie zur Erfüllung ihrer gesetzlichen Aufgaben erforderlich ist: 1. die eidgenössischen Departemente und die Bundeskanzlei, 2. die dezentralisierten Einheiten der Bundesverwaltung, 3. die Einheiten der Kantons- und Gemeindeverwaltungen, 4. die Organisationen und Personen des öffentlichen oder privaten Rechts, die nicht den Verwaltungen nach den Ziffern 1–3 angehören und die durch Bundesrecht, kantonales Recht oder kommunales Recht oder durch Vertrag mit Verwaltungsaufgaben betraut sind, sofern das anwendbare Recht die systematische Verwendung der AHV-Nummer vorsieht, 5. die Bildungsinstitutionen; b. die privaten Versicherungsunternehmen in Fällen nach Artikel 47a des Versicherungsvertragsgesetzes vom 2. April 1908; c. die Organe, die beauftragt sind, die in einem allgemeinverbindlich erklärten Gesamtarbeitsvertrag vorgesehenen Kontrollen durchzuführen.",
};

pub const BGFA_12_A: Norm = Norm {
    zitat: "Art. 12 lit. a BGFA",
    erlass: "BGFA, SR 935.61",
    fassung: "2025-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_12",
    wortlaut: "Für Anwältinnen und Anwälte gelten folgende Berufsregeln: a. Sie üben ihren Beruf sorgfältig und gewissenhaft aus.",
};

pub const OR_321A_4: Norm = Norm {
    zitat: "Art. 321a Abs. 4 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_321_a",
    wortlaut: "Der Arbeitnehmer darf geheim zu haltende Tatsachen, wie namentlich Fabrikations- und Geschäftsgeheimnisse, von denen er im Dienst des Arbeitgebers Kenntnis erlangt, während des Arbeitsverhältnisses nicht verwerten oder anderen mitteilen; auch nach dessen Beendigung bleibt er zur Verschwiegenheit verpflichtet, soweit es zur Wahrung der berechtigten Interessen des Arbeitgebers erforderlich ist.",
};

/// Schweigepflicht der Revisionsstelle (Rolle `auditor`, Rechtstexte A2 und C2).
pub const OR_730B_2: Norm = Norm {
    zitat: "Art. 730b Abs. 2 OR",
    erlass: "OR, SR 220",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/27/317_321_377/de#art_730_b",
    wortlaut: "Die Revisionsstelle wahrt das Geheimnis über ihre Feststellungen, soweit sie nicht von Gesetzes wegen zur Bekanntgabe verpflichtet ist. Sie wahrt bei der Berichterstattung, bei der Erstattung von Anzeigen und bei der Auskunftserteilung an die Generalversammlung die Geschäftsgeheimnisse der Gesellschaft.",
};

pub const STGB_110_3: Norm = Norm {
    zitat: "Art. 110 Abs. 3 StGB",
    erlass: "StGB, SR 311.0",
    fassung: "2026-11-01",
    url: "https://www.fedlex.admin.ch/eli/cc/54/757_781_799/de#art_110",
    wortlaut: "Als Beamte gelten die Beamten und Angestellten einer öffentlichen Verwaltung und der Rechtspflege sowie die Personen, die provisorisch ein Amt bekleiden oder provisorisch bei einer öffentlichen Verwaltung oder der Rechtspflege angestellt sind oder vorübergehend amtliche Funktionen ausüben.",
};

pub const STPO_12: Norm = Norm {
    zitat: "Art. 12 StPO",
    erlass: "StPO, SR 312.0",
    fassung: "2025-04-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_12",
    wortlaut: "Strafverfolgungsbehörden sind: a. die Polizei; b. die Staatsanwaltschaft; c. die Übertretungsstrafbehörden.",
};

pub const STPO_13: Norm = Norm {
    zitat: "Art. 13 StPO",
    erlass: "StPO, SR 312.0",
    fassung: "2025-04-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2010/267/de#art_13",
    wortlaut: "Gerichtliche Befugnisse im Strafverfahren haben: a. das Zwangsmassnahmengericht; b. das erstinstanzliche Gericht; c. die Beschwerdeinstanz; d. das Berufungsgericht.",
};

pub const FINIG_2_1: Norm = Norm {
    zitat: "Art. 2 Abs. 1 FINIG",
    erlass: "FINIG, SR 954.1",
    fassung: "2026-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2018/801/de#art_2",
    wortlaut: "Finanzinstitute im Sinne dieses Gesetzes sind, unabhängig von der Rechtsform: a. Vermögensverwalter (Art. 17 Abs. 1); b. Trustees (Art. 17 Abs. 2); c. Verwalter von Kollektivvermögen (Art. 24); d. Fondsleitungen (Art. 32); e. Wertpapierhäuser (Art. 41).",
};

pub const DSG_2_2_A: Norm = Norm {
    zitat: "Art. 2 Abs. 2 lit. a DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_2",
    wortlaut: "Es ist nicht anwendbar auf: a. Personendaten, die von einer natürlichen Person ausschliesslich zum persönlichen Gebrauch bearbeitet werden; …",
};

pub const DSG_16_2_D: Norm = Norm {
    zitat: "Art. 16 Abs. 2 lit. d DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_16",
    wortlaut: "Liegt kein Entscheid des Bundesrates nach Absatz 1 vor, so dürfen Personendaten ins Ausland bekanntgegeben werden, wenn ein geeigneter Datenschutz gewährleistet wird durch: … d. Standarddatenschutzklauseln, die der EDÖB vorgängig genehmigt, ausgestellt oder anerkannt hat; …",
};

/// Persönlichkeitsverletzung, ganzer Artikel (Rechtstext B2 «rechtfertigung»).
pub const DSG_30: Norm = Norm {
    zitat: "Art. 30 DSG",
    erlass: "DSG, SR 235.1",
    fassung: "2025-07-07",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/491/de#art_30",
    wortlaut: "Wer Personendaten bearbeitet, darf die Persönlichkeit der betroffenen Personen nicht widerrechtlich verletzen. Eine Persönlichkeitsverletzung liegt insbesondere vor, wenn: a. Personendaten entgegen den Grundsätzen nach den Artikeln 6 und 8 bearbeitet werden; b. Personendaten entgegen der ausdrücklichen Willenserklärung der betroffenen Person bearbeitet werden; c. Dritten besonders schützenswerte Personendaten bekanntgegeben werden. In der Regel liegt keine Persönlichkeitsverletzung vor, wenn die betroffene Person die Personendaten allgemein zugänglich gemacht und eine Bearbeitung nicht ausdrücklich untersagt hat.",
};

pub const BGFA_2_1: Norm = Norm {
    zitat: "Art. 2 Abs. 1 BGFA",
    erlass: "BGFA, SR 935.61",
    fassung: "2025-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_2",
    wortlaut: "Dieses Gesetz gilt für Personen, die über ein Anwaltspatent verfügen und in der Schweiz im Rahmen des Anwaltsmonopols Parteien vor Gerichtsbehörden vertreten.",
};

pub const BGFA_13_2: Norm = Norm {
    zitat: "Art. 13 Abs. 2 BGFA",
    erlass: "BGFA, SR 935.61",
    fassung: "2025-07-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2002/153/de#art_13",
    wortlaut: "Sie sorgen für die Wahrung des Berufsgeheimnisses durch ihre Hilfspersonen.",
};

/// Geltung des ISG für die Kantone (Rechtstext A5 «vermerk»).
pub const ISG_3: Norm = Norm {
    zitat: "Art. 3 ISG",
    erlass: "ISG, SR 128",
    fassung: "2025-10-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2022/232/de#art_3",
    wortlaut: "Für die Kantone gelten nur die Bestimmungen: a. über klassifizierte Informationen, soweit sie klassifizierte Informationen des Bundes bearbeiten; und b. über die Sicherheit beim Einsatz von Informatikmitteln, soweit sie auf Informatikmittel des Bundes zugreifen. Diese Bestimmungen gelten nicht, wenn die Kantone eine mindestens gleichwertige Informationssicherheit gewährleisten.",
};

/// Information über die Rechtsprechung (Rechtstext B5; ersetzt `hinweise::BGG_27`).
pub const BGG_27_1: Norm = Norm {
    zitat: "Art. 27 Abs. 1 BGG",
    erlass: "BGG, SR 173.110",
    fassung: "2026-04-01",
    url: "https://www.fedlex.admin.ch/eli/cc/2006/218/de#art_27",
    wortlaut: "Das Bundesgericht informiert die Öffentlichkeit über seine Rechtsprechung.",
};

pub const AHVG_153I_1: Norm = Norm {
    zitat: "Art. 153i Abs. 1 AHVG",
    erlass: "AHVG, SR 831.10",
    fassung: "2026-01-01",
    url: "https://www.fedlex.admin.ch/eli/cc/63/837_843_843/de#art_153_i",
    wortlaut: "Wer die AHV-Nummer systematisch verwendet, ohne dazu nach Artikel 153c Absatz 1 berechtigt zu sein, wird mit Geldstrafe bestraft.",
};

/// Alle Normen dieses Moduls, für Prüfungen und `explain`.
pub const ALLE: &[&Norm] = &[
    &DSG_2_1,
    &DSG_2_3,
    &DSG_5_A,
    &DSG_5_C,
    &DSG_5_E,
    &DSG_6_2,
    &DSG_6_7_A,
    &DSG_9_1_B,
    &DSG_30_1,
    &DSG_30_2_C,
    &DSG_31_1,
    &DSG_34_1,
    &DSG_34_4,
    &DSG_36_1,
    &DSG_36_2,
    &DSG_62_1,
    &DSG_62_2,
    &STGB_162,
    &STGB_320_1,
    &STGB_320_2,
    &STGB_321_1,
    &STGB_321_1_ABS2,
    &STGB_321_2,
    &BGFA_13_1,
    &BANKG_47_1_A,
    &BANKG_47_2,
    &FINIG_69_1_A,
    &FINIG_69_3,
    &FINFRAG_147_1_A,
    &OR_328B,
    &OR_342_1_A,
    &OR_552_1,
    &OR_568_1,
    &OR_594_2,
    &KAG_98_2,
    &STPO_73_1,
    &AHVG_153B,
    &URG_5_1_A,
    &BV_70_1,
    &DSG_5_D,
    &DSG_5_J_K,
    &DSG_9_1_A,
    &DSG_9_2,
    &DSG_9_3,
    &DSG_16_1,
    &DSG_16_2,
    &DSG_17_1_A,
    &DSG_61_A,
    &DSG_61_B,
    &DSV_ANHANG1_44,
    &AHVG_49,
    &AHVG_51_3,
    &AHVG_153C_1,
    &BGFA_12_A,
    &OR_321A_4,
    &OR_730B_2,
    &STGB_110_3,
    &STPO_12,
    &STPO_13,
    &FINIG_2_1,
    &DSG_2_2_A,
    &DSG_16_2_D,
    &DSG_30,
    &BGFA_2_1,
    &BGFA_13_2,
    &ISG_3,
    &BGG_27_1,
    &AHVG_153I_1,
];

/// Zitat der Ziffer von Art. 5 lit. c DSG, z. B. «Art. 5 lit. c Ziff. 2 DSG».
pub fn art5c_zitat(ziffer: u8) -> String {
    format!("Art. 5 lit. c Ziff. {ziffer} DSG")
}

/// Kurzbezeichnung der Kategorie nach Art. 5 lit. c Ziff. 1–6 DSG.
pub fn art5c_kategorie(ziffer: u8, englisch: bool) -> &'static str {
    match (ziffer, englisch) {
        (1, false) => {
            "Ansichten oder Tätigkeiten (religiös, weltanschaulich, politisch, gewerkschaftlich)"
        }
        (2, false) => "Gesundheit, Intimsphäre, Rasse oder Ethnie",
        (3, false) => "genetische Daten",
        (4, false) => "biometrische Daten",
        (5, false) => "verwaltungs- oder strafrechtliche Verfolgungen oder Sanktionen",
        (6, false) => "Massnahmen der sozialen Hilfe",
        (1, true) => "views or activities (religious, philosophical, political, trade union)",
        (2, true) => "health, intimate sphere, race or ethnicity",
        (3, true) => "genetic data",
        (4, true) => "biometric data",
        (5, true) => "administrative or criminal proceedings or sanctions",
        (6, true) => "social assistance measures",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn felder_vollstaendig() {
        for n in ALLE {
            assert!(
                n.url.starts_with("https://www.fedlex.admin.ch/eli/"),
                "{}",
                n.zitat
            );
            assert_eq!(n.fassung.len(), 10, "{}", n.zitat);
            assert!(!n.wortlaut.is_empty(), "{}", n.zitat);
        }
    }

    /// Das Ende der zitierten Einheit (Artikel, Absatz, Ziffer, Buchstabe, Satz) im Wortlaut
    /// der Fassung, am 06.11.2026 auf Fedlex gelesen. Ein Auszug ohne «…» am Schluss muss
    /// so enden; sonst fehlt Text, ohne dass die Kürzung markiert ist.
    const ENDE_DER_EINHEIT: &[(&str, &str)] = &[
        ("Art. 2 Abs. 1 DSG", "b. Bundesorgane."),
        (
            "Art. 2 Abs. 3 DSG",
            "Auf erstinstanzliche Verwaltungsverfahren sind die Bestimmungen dieses Gesetzes anwendbar.",
        ),
        ("Art. 5 lit. a DSG", "natürliche Person beziehen"),
        (
            "Art. 5 lit. c DSG",
            "6. Daten über Massnahmen der sozialen Hilfe",
        ),
        (
            "Art. 5 lit. e DSG",
            "das Übermitteln oder Zugänglichmachen von Personendaten",
        ),
        ("Art. 6 Abs. 2 DSG", "erfolgen und verhältnismässig sein."),
        (
            "Art. 9 Abs. 1 lit. b DSG",
            "b. keine gesetzliche oder vertragliche Geheimhaltungspflicht die Übertragung verbietet.",
        ),
        ("Art. 30 Abs. 1 DSG", "nicht widerrechtlich verletzen."),
        (
            "Art. 30 Abs. 2 lit. c DSG",
            "c. Dritten besonders schützenswerte Personendaten bekanntgegeben werden.",
        ),
        (
            "Art. 31 Abs. 1 DSG",
            "oder durch Gesetz gerechtfertigt ist.",
        ),
        (
            "Art. 34 Abs. 1 DSG",
            "wenn dafür eine gesetzliche Grundlage besteht.",
        ),
        (
            "Art. 36 Abs. 1 DSG",
            "eine gesetzliche Grundlage nach Artikel 34 Absätze 1–3 besteht.",
        ),
        (
            "Art. 62 Abs. 1 DSG",
            "mit Busse bis zu 250 000 Franken bestraft.",
        ),
        (
            "Art. 62 Abs. 2 DSG",
            "während der Ausbildung bei dieser Kenntnis erlangt hat.",
        ),
        (
            "Art. 162 StGB",
            "auf Antrag, mit Freiheitsstrafe bis zu drei Jahren oder Geldstrafe bestraft.",
        ),
        (
            "Art. 320 Ziff. 1 StGB",
            "Die Verletzung des Amtsgeheimnisses ist auch nach Beendigung des amtlichen oder dienstlichen Verhältnisses oder der Hilfstätigkeit strafbar.",
        ),
        (
            "Art. 320 Ziff. 2 StGB",
            "seiner vorgesetzten Behörde offenbart hat.",
        ),
        (
            "Art. 321 Ziff. 1 StGB",
            "Die Verletzung des Berufsgeheimnisses ist auch nach Beendigung der Berufsausübung oder der Studien strafbar.",
        ),
        (
            "Art. 321 Ziff. 1 Abs. 2 StGB",
            "das sie bei ihrem Studium wahrnehmen.",
        ),
        (
            "Art. 321 Ziff. 2 StGB",
            "oder Aufsichtsbehörde offenbart hat.",
        ),
        (
            "Art. 13 Abs. 1 BGFA",
            "Die Entbindung verpflichtet sie nicht zur Preisgabe von Anvertrautem.",
        ),
        (
            "Art. 47 Abs. 2 BankG",
            "mit Busse bis zu 250 000 Franken bestraft.",
        ),
        (
            "Art. 69 Abs. 3 FINIG",
            "mit Busse bis zu 250 000 Franken bestraft.",
        ),
        (
            "Art. 73 Abs. 1 StPO",
            "in Ausübung ihrer amtlichen Tätigkeit zur Kenntnis gelangt sind.",
        ),
        (
            "Art. 328b OR",
            "Im Übrigen gelten die Bestimmungen des Datenschutzgesetzes vom 25. September 2020.",
        ),
        (
            "Art. 552 Abs. 1 OR",
            "nach kaufmännischer Art geführtes Gewerbe zu betreiben.",
        ),
        (
            "Art. 568 Abs. 1 OR",
            "solidarisch und mit ihrem ganzen Vermögen.",
        ),
        (
            "Art. 594 Abs. 2 OR",
            "auch juristische Personen und Handelsgesellschaften sein.",
        ),
        ("Art. 153b AHVG", "in strukturierter Form gesammelt werden."),
        // Endfassung der Rechtstexte, Ende der Einheit mit fedlex.mjs nachgesehen.
        (
            "Art. 5 lit. d DSG",
            "Bekanntgeben, Archivieren, Löschen oder Vernichten von Daten",
        ),
        (
            "Art. 5 lit. j und k DSG",
            "im Auftrag des Verantwortlichen Personendaten bearbeitet.",
        ),
        (
            "Art. 9 Abs. 2 DSG",
            "in der Lage ist, die Datensicherheit zu gewährleisten.",
        ),
        (
            "Art. 9 Abs. 3 DSG",
            "Genehmigung des Verantwortlichen einem Dritten übertragen.",
        ),
        (
            "Art. 16 Abs. 1 DSG",
            "Organ einen angemessenen Schutz gewährleistet.",
        ),
        (
            "Art. 49 AHVG",
            "und eine zentrale Ausgleichsstelle (Durchführungsstellen).",
        ),
        (
            "Art. 51 Abs. 3 AHVG",
            "für die Führung der individuellen Konten der Arbeitnehmer.",
        ),
        (
            "Art. 153c Abs. 1 AHVG",
            "Gesamtarbeitsvertrag vorgesehenen Kontrollen durchzuführen.",
        ),
        (
            "Art. 12 lit. a BGFA",
            "a. Sie üben ihren Beruf sorgfältig und gewissenhaft aus.",
        ),
        (
            "Art. 321a Abs. 4 OR",
            "berechtigten Interessen des Arbeitgebers erforderlich ist.",
        ),
        (
            "Art. 730b Abs. 2 OR",
            "die Geschäftsgeheimnisse der Gesellschaft.",
        ),
        (
            "Art. 110 Abs. 3 StGB",
            "sind oder vorübergehend amtliche Funktionen ausüben.",
        ),
        (
            "Art. 12 StPO",
            "b. die Staatsanwaltschaft; c. die Übertretungsstrafbehörden.",
        ),
        (
            "Art. 13 StPO",
            "Gericht; c. die Beschwerdeinstanz; d. das Berufungsgericht.",
        ),
        (
            "Art. 2 Abs. 1 FINIG",
            "d. Fondsleitungen (Art. 32); e. Wertpapierhäuser (Art. 41).",
        ),
        (
            "Art. 30 DSG",
            "und eine Bearbeitung nicht ausdrücklich untersagt hat.",
        ),
        (
            "Art. 2 Abs. 1 BGFA",
            "des Anwaltsmonopols Parteien vor Gerichtsbehörden vertreten.",
        ),
        (
            "Art. 13 Abs. 2 BGFA",
            "die Wahrung des Berufsgeheimnisses durch ihre Hilfspersonen.",
        ),
        (
            "Art. 3 ISG",
            "gleichwertige Informationssicherheit gewährleisten.",
        ),
        (
            "Art. 27 Abs. 1 BGG",
            "informiert die Öffentlichkeit über seine Rechtsprechung.",
        ),
        (
            "Art. 153i Abs. 1 AHVG",
            "Absatz 1 berechtigt zu sein, wird mit Geldstrafe bestraft.",
        ),
        (
            "Art. 70 Abs. 1 BV",
            "auch das Rätoromanische Amtssprache des Bundes.",
        ),
    ];

    /// E9: Ein Auszug ist vollständig, oder «…» markiert, dass am Schluss Text fehlt.
    #[test]
    fn auszug_vollstaendig_oder_mit_auslassung() {
        for n in ALLE {
            if n.wortlaut.ends_with('…') {
                continue;
            }
            let (_, ende) = ENDE_DER_EINHEIT
                .iter()
                .find(|(zitat, _)| *zitat == n.zitat)
                .unwrap_or_else(|| panic!("{}: Ende der Einheit nicht erfasst", n.zitat));
            assert!(
                n.wortlaut.ends_with(ende),
                "{}: Auszug endet vor der Einheit, ohne «…»",
                n.zitat
            );
        }
    }

    /// E9: die drei Zitate, die vor dem Ende der zitierten Einheit aufhörten.
    #[test]
    fn drei_zitate_vollstaendig() {
        assert!(STGB_320_1.wortlaut.ends_with(
            "Die Verletzung des Amtsgeheimnisses ist auch nach Beendigung des amtlichen oder dienstlichen Verhältnisses oder der Hilfstätigkeit strafbar."
        ));
        assert!(STGB_321_1.wortlaut.contains(STGB_321_1_ABS2.wortlaut));
        assert!(STGB_321_1.wortlaut.ends_with(
            "Die Verletzung des Berufsgeheimnisses ist auch nach Beendigung der Berufsausübung oder der Studien strafbar."
        ));
        assert!(
            BGFA_13_1
                .wortlaut
                .ends_with("Die Entbindung verpflichtet sie nicht zur Preisgabe von Anvertrautem.")
        );
    }

    /// Rechtstext B5: Pflichten des OR (Revisionsstelle, Arbeitsverhältnis) sind
    /// rechtlicher Bezug, die Rechtsformen der Regel firma Grundlage der Erkennung.
    #[test]
    fn bezug_und_erkennung_im_or() {
        for n in [&OR_730B_2, &OR_321A_4, &OR_328B, &OR_342_1_A] {
            assert!(ist_bezug(n), "{}", n.zitat);
        }
        for n in [&OR_552_1, &OR_568_1, &OR_594_2] {
            assert!(!ist_bezug(n), "{}", n.zitat);
        }
        for zitat in BEZUG_OR {
            assert!(ALLE.iter().any(|n| n.zitat == *zitat), "{zitat}");
        }
        assert!(ist_bezug(&STGB_321_1) && !ist_bezug(&BGG_27_1));
    }
}
