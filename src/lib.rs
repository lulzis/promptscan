//! promptscan prüft einen Prompt, bevor er an eine KI geht, und markiert, welche Stellen
//! nach Schweizer Recht nicht ohne Weiteres hinausgehen sollten, jeweils mit der Norm.
//!
//! Ablauf: [`text::Text`] normalisiert, die Erkenner ([`klasse_a`], [`regeln`]) melden
//! [`Treffer`], das rechtliche Modell ([`stufen`]) macht daraus [`Fund`]e, [`ausgabe`]
//! stellt sie dar. Das Programm enthält keinen Netzwerkcode.

pub mod ausgabe;
pub mod fund;
pub mod katalog;
pub mod klasse_a;
pub mod lexikon;
pub mod lizenzen;
pub mod normen;
pub mod regeln;
pub mod stufen;
pub mod text;

pub use fund::{Art, Fund, Grund, Rolle, RolleQuelle, Stufe, Treffer, Ziel};
pub use stufen::{Lage, Optionen};

/// Ergebnis einer Prüfung.
#[derive(Clone, Debug)]
pub struct Bericht {
    pub text: text::Text,
    pub funde: Vec<Fund>,
    pub lage: Lage,
    pub optionen: Optionen,
    /// Woher die Rolle stammt (Kopfzeile): `pruefe` setzt `Option`, wenn eine Rolle
    /// angegeben ist, sonst `Keine`; `main.rs` setzt `Konfiguration`, wenn die Rolle aus
    /// der Konfigurationsdatei kommt.
    pub rolle_quelle: RolleQuelle,
}

impl Bericht {
    /// Anzahl Funde einer Stufe.
    pub fn anzahl(&self, stufe: Stufe) -> usize {
        self.funde.iter().filter(|f| f.stufe == stufe).count()
    }

    /// Die höchste Stufe aller Funde.
    pub fn hoechste(&self) -> Option<Stufe> {
        self.funde.iter().map(|f| f.stufe).max()
    }
}

/// Prüft einen Text.
pub fn pruefe(roh: &str, optionen: &Optionen) -> Bericht {
    let text = text::Text::neu(roh);
    let treffer = regeln::erkenne(&text);
    let (funde, lage) = stufen::bewerte(&text, treffer, optionen);
    Bericht {
        text,
        funde,
        lage,
        optionen: *optionen,
        rolle_quelle: if optionen.rolle.is_some() {
            RolleQuelle::Option
        } else {
            RolleQuelle::Keine
        },
    }
}
