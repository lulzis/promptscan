//! Der Regelkatalog für `promptscan rules` und `promptscan explain <REGEL>`.
//!
//! Jeder Erkenner beschreibt seine Regeln selbst (`info()` im jeweiligen Modul): was sie
//! erkennt, nach welcher Methode, auf welcher Norm die Stufe beruht, welche Quellen die
//! Methode trägt, wie gut sie gemessen ist und was sie nicht entscheidet.

use crate::normen::Norm;

/// Ein fester Text in beiden Ausgabesprachen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Txt {
    pub de: &'static str,
    pub en: &'static str,
}

impl Txt {
    pub const fn neu(de: &'static str, en: &'static str) -> Txt {
        Txt { de, en }
    }

    pub fn in_sprache(&self, englisch: bool) -> &'static str {
        if englisch { self.en } else { self.de }
    }
}

/// Klasse einer Regel (SPEC 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Klasse {
    /// Formal prüfbar: Prüfziffer oder amtliches Format (5.1).
    A,
    /// Wortliste oder Register mit Quelle (5.2).
    B,
    /// Besonders schützenswerte Personendaten, Art. 5 lit. c DSG (5.3).
    C,
    /// Hinweis ohne Rechtsfolge (5.4).
    Hinweis,
    /// Kontext: erkennt keinen Fund, sondern die Lage (Geheimnis-Kontext, Personenbezug).
    Kontext,
}

impl Klasse {
    pub fn kurz(self) -> &'static str {
        match self {
            Klasse::A => "A",
            Klasse::B => "B",
            Klasse::C => "C",
            Klasse::Hinweis => "H",
            Klasse::Kontext => "K",
        }
    }
}

/// Prüfstand einer Quelle (SPEC 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stand {
    /// Gelesen und gegen die Regel geprüft, am angegebenen Datum.
    Geprueft(&'static str),
    /// Noch nicht oder nur aus zweiter Hand geprüft; der Text sagt, was fehlt.
    ZuPruefen(&'static str),
}

/// Eine Quelle der Methode oder der Wortliste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quelle {
    pub titel: &'static str,
    pub url: &'static str,
    /// Lizenz bei Datensätzen.
    pub lizenz: Option<&'static str>,
    pub stand: Stand,
}

/// Gemessene Güte (SPEC 2, Grundsatz 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guete {
    /// Noch keine Messung am Korpus.
    NichtGemessen,
    /// Eigenschaft mathematisch nachgewiesen (Eigenschaftstest), z. B. bei Prüfziffern.
    Nachgewiesen(Txt),
}

/// Beschreibung einer Regel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegelInfo {
    pub id: &'static str,
    pub klasse: Klasse,
    /// Kurzer Titel, z. B. «AHV-Nummer».
    pub titel: Txt,
    /// Was die Regel erkennt.
    pub erkennt: Txt,
    /// Wie, in zwei bis vier Sätzen.
    pub methode: Txt,
    /// Kurzzeile unter einem HINWEIS dieser Regel in der Ausgabe, z. B. «Sprachmodelle
    /// erfinden Urteile. Jede Fundstelle prüfen.» Nur für Hinweisregeln nötig.
    pub hinweis: Option<Txt>,
    /// Normen, auf denen die Stufe oder die Methode beruht.
    pub grundlage: Vec<&'static Norm>,
    pub quellen: Vec<Quelle>,
    /// Eingebettete Dateien unter `lexika/`, die die Regel nutzt.
    pub lexika: Vec<&'static str>,
    pub guete: Guete,
    /// Was die Regel nicht entscheidet oder nicht erkennt.
    pub grenzen: Vec<Txt>,
}

/// Alle Regeln, in der Reihenfolge der Spezifikation.
pub fn alle() -> Vec<RegelInfo> {
    let mut v = crate::klasse_a::info();
    v.extend(crate::regeln::info());
    v.extend(crate::stufen::info());
    v
}

/// Die Regel mit dieser Kennung, ohne Rücksicht auf Gross- und Kleinschreibung und
/// Leerraum am Rand («AHV», « Art5c-2 »), wie die Stufennamen in `explain`.
pub fn finde(id: &str) -> Option<RegelInfo> {
    let gesucht = id.trim().to_lowercase();
    alle().into_iter().find(|r| r.id == gesucht)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kennungen_eindeutig() {
        let mut ids: Vec<_> = alle().iter().map(|r| r.id).collect();
        let anzahl = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), anzahl, "doppelte Regelkennung");
    }

    /// Kennungen sind klein geschrieben; `finde` nimmt sie in jeder Schreibweise an.
    #[test]
    fn finde_ohne_gross_und_kleinschreibung() {
        for r in alle() {
            assert_eq!(r.id, r.id.to_lowercase(), "Kennung {} nicht klein", r.id);
            for eingabe in [r.id.to_string(), r.id.to_uppercase(), format!(" {} ", r.id)] {
                assert_eq!(finde(&eingabe).map(|f| f.id), Some(r.id), "{eingabe:?}");
            }
        }
        assert_eq!(finde("AHV").map(|f| f.id), Some("ahv"));
        assert_eq!(finde("Art5c-2").map(|f| f.id), Some("art5c-2"));
        assert!(finde("gibt-es-nicht").is_none());
        assert!(finde("").is_none());
    }

    #[test]
    fn lexika_existieren() {
        for r in alle() {
            for d in &r.lexika {
                assert!(
                    crate::lexikon::inhalt(d).is_some(),
                    "Regel {}: lexika/{} fehlt",
                    r.id,
                    d
                );
            }
        }
    }
}
