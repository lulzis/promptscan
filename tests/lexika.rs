//! Jede Wortliste und jedes Register unter `lexika/` ist gültig, belegt und benutzt.

use promptscan::katalog;
use promptscan::lexikon::{self, Lexikon, Register};

/// Dateien, die keine Regel trägt, sondern die Textverarbeitung.
const OHNE_REGEL: &[&str] = &["abkuerzungen.toml"];

#[test]
fn alle_dateien_gueltig_und_nicht_leer() {
    for datei in lexikon::dateien() {
        let inhalt = lexikon::inhalt(datei).expect("eingebettet");
        if datei.ends_with(".toml") {
            let lex = Lexikon::aus_toml(datei, inhalt).unwrap_or_else(|e| panic!("{e}"));
            assert!(!lex.eintraege.is_empty(), "lexika/{datei} ist leer");
        } else {
            let reg = Register::aus_tsv(datei, inhalt).unwrap_or_else(|e| panic!("{e}"));
            assert!(!reg.zeilen.is_empty(), "lexika/{datei} ist leer");
        }
    }
}

#[test]
fn jede_datei_gehoert_zu_einer_regel() {
    let benutzt: Vec<&str> = katalog::alle()
        .iter()
        .flat_map(|r| r.lexika.clone())
        .collect();
    for datei in lexikon::dateien() {
        assert!(
            benutzt.contains(&datei) || OHNE_REGEL.contains(&datei),
            "lexika/{datei} wird von keiner Regel genannt (RegelInfo::lexika)"
        );
    }
}

#[test]
fn keine_doppelten_formen_je_datei_und_sprache() {
    for datei in lexikon::dateien().filter(|d| d.ends_with(".toml")) {
        let lex = Lexikon::aus_toml(datei, lexikon::inhalt(datei).unwrap()).unwrap();
        let mut gesehen = std::collections::HashSet::new();
        for e in &lex.eintraege {
            for form in std::iter::once(&e.begriff).chain(e.formen.iter()) {
                // Mit «gross»: Dieselbe Form darf einmal nur gross geschrieben und einmal in
                // jeder Schreibung stehen.
                let schluessel = (
                    form.to_lowercase(),
                    e.sprache,
                    e.regel.clone(),
                    e.merkmal.clone(),
                    e.gross,
                );
                assert!(
                    gesehen.insert(schluessel),
                    "lexika/{datei}: Form «{form}» doppelt"
                );
            }
        }
    }
}
