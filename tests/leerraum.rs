//! Kein Absturz an Zeichen aus mehreren Bytes: Jede Regel schneidet den Text nur an
//! Zeichengrenzen. Anlass: Ein geschütztes Leerzeichen (U+00A0) zwischen Anrede und Name
//! liess die Regel name abstürzen («Mme Rochat …», Exit 101), weil sie nach dem gefundenen
//! Leerraum ein Byte weiter schnitt statt ein Zeichen.

use promptscan::{Optionen, pruefe};

/// Leerraum-, Format- und Satzzeichen aus mehreren Bytes, wie sie Word, PDF und Mail
/// einfügen, als Codepunkte.
const ZEICHEN: [u32; 10] = [
    0x00A0, // geschütztes Leerzeichen
    0x202F, // schmales geschütztes Leerzeichen (französische Typografie vor «:»)
    0x2009, // schmales Leerzeichen
    0x2007, // Ziffernleerzeichen
    0x2028, // Zeilentrenner
    0x3000, // ideographisches Leerzeichen
    0x00AD, // weiches Trennzeichen
    0x200B, // Leerzeichen ohne Breite
    0xFEFF, // Byte-Order-Mark mitten im Text
    0x2013, // Halbgeviertstrich
];

/// Sätze, die die Pfade der Regeln name (Anrede, Liste, Zelle), ahv, art5c, firma und
/// kontext berühren, in allen drei Sprachen.
const SAETZE: [&str; 6] = [
    "Mme Rochat est en incapacité de travail depuis mars.",
    "Herr Hans Muster, AHV 756.1234.5678.97, CHF 1200, ist krank.",
    "Muster;Hans;Lager\nBrunner;Anna;Büro",
    "La signora Giulia Esempio è malata: art. 5 lett. c LPD.",
    "Als Anwältin der Muster AG habe ich eine Frage zu Frau Erika Beispiel.",
    "- Hans Muster, geb. 12.03.1980, wohnhaft in Muri AG",
];

fn ohne_absturz(text: &str) {
    let _ = pruefe(text, &Optionen::default());
}

#[test]
fn zeichen_aus_mehreren_bytes_an_jeder_stelle_ohne_absturz() {
    for z in ZEICHEN {
        let c = char::from_u32(z).expect("gültiges Zeichen");
        for satz in SAETZE {
            // Jedes Leerzeichen ersetzt.
            let ersetzt: String = satz.chars().map(|x| if x == ' ' { c } else { x }).collect();
            ohne_absturz(&ersetzt);
            // An jeder Zeichengrenze eingefügt.
            let grenzen = satz.char_indices().map(|(i, _)| i).chain([satz.len()]);
            for i in grenzen {
                let mut t = String::with_capacity(satz.len() + 4);
                t.push_str(&satz[..i]);
                t.push(c);
                t.push_str(&satz[i..]);
                ohne_absturz(&t);
            }
        }
    }
}

/// Die bekannten Absturzfälle, einzeln, damit ein Rückfall sofort benannt ist.
#[test]
fn geschuetztes_leerzeichen_nach_anrede_und_betrag() {
    let nbsp = char::from_u32(0x00A0).expect("U+00A0");
    let schmal = char::from_u32(0x202F).expect("U+202F");
    ohne_absturz(&format!(
        "Mme{nbsp}Rochat est en incapacité de travail depuis mars."
    ));
    ohne_absturz(&format!(
        "Le client paie CHF{nbsp}1200 pour Mme{schmal}Dupont."
    ));
    ohne_absturz(&format!("Muster;Hans{nbsp}756.1234.5678.97"));
}
