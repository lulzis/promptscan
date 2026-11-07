//! Die Erkenner der Klassen B und C, der Kontext und die Hinweise.
//!
//! Jedes Modul hat dieselbe Schnittstelle:
//!
//! * `pub fn erkenne(text: &Text) -> Vec<Treffer>` meldet Stellen, ohne Stufe.
//! * `pub fn info() -> Vec<RegelInfo>` beschreibt seine Regeln für `rules` und `explain`.

pub mod art5c;
pub mod firma;
pub mod geburtsdatum;
pub mod hinweise;
pub mod kontext;
pub mod name;
pub mod ort;

use crate::fund::Treffer;
use crate::katalog::RegelInfo;
use crate::text::Text;

/// Alle Erkenner auf einem Text, Klasse A eingeschlossen.
pub fn erkenne(text: &Text) -> Vec<Treffer> {
    let mut t = crate::klasse_a::erkenne(text);
    t.extend(name::erkenne(text));
    t.extend(firma::erkenne(text));
    t.extend(ort::erkenne(text));
    t.extend(geburtsdatum::erkenne(text));
    t.extend(art5c::erkenne(text));
    t.extend(kontext::erkenne(text));
    t.extend(hinweise::erkenne(text));
    t
}

/// Beschreibung aller Regeln dieser Module, in der Reihenfolge der Spezifikation.
pub fn info() -> Vec<RegelInfo> {
    let mut v = name::info();
    v.extend(firma::info());
    v.extend(ort::info());
    v.extend(geburtsdatum::info());
    v.extend(art5c::info());
    v.extend(kontext::info());
    v.extend(hinweise::info());
    v
}
