//! Eigenschaftstests für Klasse A (SPEC 7): AHV-Nummer und IBAN, dazu ein Lauf aller vier
//! Regeln über beliebigen Text.
//!
//! Die Referenzrechnungen hier sind unabhängig vom Erkenner geschrieben: die
//! AHV-Kontrollziffer von links mit den Gewichten 1, 3, 1, … (GS1 General Specifications,
//! Tabelle 7-8, GTIN-13), die IBAN-Prüfziffern über die ausgeschriebene Ziffernfolge nach
//! ISO 13616-1:2020, Ziffer 6.3. Die Läufe haben feste Startwerte und prüfen bei jedem
//! Lauf dieselben Fälle. «Erschöpfend» heisst: jede Stelle mit jedem anderen Zeichen.
//! Wo eine Aussage für alle Nummern gilt, rechnet ein `beweis_…`-Test sie über alle Fälle
//! der Rechnung modulo 10 oder 97 nach.

use std::cell::Cell;

use promptscan::klasse_a::erkenne;
use promptscan::text::{Text, ist_unsichtbar};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};

// --- Werkzeuge ------------------------------------------------------------------------

/// Ein Läufer mit festem Startwert, ohne Ablage fehlgeschlagener Fälle.
fn laeufer(startwert: u64, faelle: u32) -> TestRunner {
    let config = Config {
        cases: faelle,
        failure_persistence: None,
        ..Config::default()
    };
    let mut saat = [0u8; 32];
    saat[..8].copy_from_slice(&startwert.to_le_bytes());
    TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &saat))
}

/// Byte-Bereiche der Treffer von `regel`.
fn stellen(roh: &str, regel: &str) -> Vec<(usize, usize)> {
    erkenne(&Text::neu(roh))
        .into_iter()
        .filter(|t| t.regel == regel)
        .map(|t| (t.start, t.ende))
        .collect()
}

fn erkannt(roh: &str, regel: &str) -> bool {
    !stellen(roh, regel).is_empty()
}

/// Beliebiger Text aus beliebigen Unicode-Zeichen.
fn umgebung() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..16).prop_map(String::from_iter)
}

/// Ein Zeichen zwischen Umgebung und Nummer: kein Buchstabe, keine Ziffer, kein «@» und
/// kein Punkt (ein Punkt mit Ziffer davor setzt eine Ziffernfolge fort).
fn trenner() -> impl Strategy<Value = char> {
    prop::sample::select(vec![
        ' ', '\n', '\t', ',', ';', ':', '(', ')', '«', '»', '"', '!', '?', '/', '\u{00A0}',
    ])
}

/// Text aus Umgebung, Trenner, Nummer, Trenner, Umgebung; dazu der Byte-Bereich der
/// Nummer im normalisierten Text. Das Ende ist aus dem normalisierten Text gerechnet, weil
/// ein CRLF in der Nummer dort zu einem Zeichen wird.
fn eingebettet(
    nummer: &str,
    vor: &str,
    t1: char,
    t2: char,
    nach: &str,
) -> (String, (usize, usize)) {
    let davor = format!("{vor}{t1}");
    let start = Text::neu(&davor).als_str().len();
    let ende = Text::neu(&format!("{davor}{nummer}")).als_str().len();
    (format!("{davor}{nummer}{t2}{nach}"), (start, ende))
}

// --- AHV ------------------------------------------------------------------------------

/// Gewichte der 13 Stellen in der Prüfsumme, die durch 10 teilbar sein muss: 1, 3, …, 3
/// für die ersten zwölf Ziffern, 1 für die Kontrollziffer selbst.
const GEWICHTE: [i32; 13] = [1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1, 3, 1];

/// Kontrollziffer nach GS1, Tabelle 7-8 (GTIN-13): von links Gewichte 1, 3, 1, 3, …,
/// dann die Ergänzung zum nächsten Vielfachen von 10.
fn ahv_kontrollziffer(zwoelf: &[u8]) -> u8 {
    let summe: u32 = zwoelf
        .iter()
        .zip([1, 3].into_iter().cycle())
        .map(|(&z, g)| u32::from(z) * g)
        .sum();
    ((10 - summe % 10) % 10) as u8
}

/// 756, neun Ziffern und die Kontrollziffer.
fn ahv_nummer(neun: &[u8]) -> [u8; 13] {
    let mut z = [0u8; 13];
    z[..3].copy_from_slice(&[7, 5, 6]);
    z[3..12].copy_from_slice(neun);
    z[12] = ahv_kontrollziffer(&z[..12]);
    z
}

/// Leerzeichen zwischen Zifferngruppen: das gewöhnliche, das geschützte, das schmale
/// geschützte, das schmale und der Ziffernabstand.
const LEERZEICHEN: [char; 5] = [' ', '\u{00A0}', '\u{202F}', '\u{2009}', '\u{2007}'];

/// Leerraum nach einem Punkt, wenn die Nummer dort auf die nächste Zeile umbricht: genau
/// ein Zeilenumbruch, mit Leerraum höchstens drei Zeichen.
const UMBRUECHE: [&str; 7] = ["\n", " \n", "\n ", "\n\t", "\t\n", " \n ", "\u{2028}"];

/// Wie eine AHV-Nummer geschrieben ist.
#[derive(Clone, Debug)]
enum Schreibweise {
    /// Ohne Trenner (eCH-0044).
    Ohne,
    /// Gruppen 3.4.4.2 mit Punkten (Rz 1202 WL VA/IK); auf Wunsch nach dem n-ten Punkt
    /// (1 bis 3) ein Umbruch auf die nächste Zeile.
    Punkte(Option<(usize, &'static str)>),
    /// Gruppen 3-4-4-2 mit je einem Leerzeichen aus [`LEERZEICHEN`].
    Leerzeichen([char; 3]),
}

/// Jede Schreibweise mit gleichem Gewicht.
fn schreibweise() -> impl Strategy<Value = Schreibweise> {
    prop_oneof![
        Just(Schreibweise::Ohne),
        Just(Schreibweise::Punkte(None)),
        (1usize..=3, prop::sample::select(UMBRUECHE.to_vec()))
            .prop_map(|umbruch| Schreibweise::Punkte(Some(umbruch))),
        prop::array::uniform3(prop::sample::select(LEERZEICHEN.to_vec()))
            .prop_map(Schreibweise::Leerzeichen),
    ]
}

/// Die festen Schreibweisen der erschöpfenden Tests: ohne Trenner, mit Punkten, mit einem
/// Umbruch nach jedem der drei Punkte und mit jeder Art Leerzeichen.
fn alle_schreibweisen() -> Vec<Schreibweise> {
    let mut v = vec![Schreibweise::Ohne, Schreibweise::Punkte(None)];
    v.extend((1..=3).map(|n| Schreibweise::Punkte(Some((n, "\n")))));
    v.extend(LEERZEICHEN.map(|l| Schreibweise::Leerzeichen([l; 3])));
    v
}

fn ahv_schreiben(z: &[u8; 13], weise: &Schreibweise) -> String {
    let mut s = String::with_capacity(24);
    for (i, &d) in z.iter().enumerate() {
        if let Some(luecke) = [3, 7, 11].iter().position(|&g| g == i) {
            match weise {
                Schreibweise::Ohne => {}
                Schreibweise::Punkte(umbruch) => {
                    s.push('.');
                    if let Some((n, leer)) = umbruch
                        && *n == luecke + 1
                    {
                        s.push_str(leer);
                    }
                }
                Schreibweise::Leerzeichen(leer) => s.push(leer[luecke]),
            }
        }
        s.push(char::from(b'0' + d));
    }
    s
}

/// Die erste Ersetzung einer einzelnen Ziffer, die trotzdem erkannt wird; geprüft an jeder
/// der 13 Stellen mit jeder der neun anderen Ziffern.
fn ahv_erkannte_ersetzung(z: &[u8; 13], weise: &Schreibweise) -> Option<String> {
    for (stelle, &alt) in z.iter().enumerate() {
        for neu in (0..10).filter(|&d| d != alt) {
            let mut f = *z;
            f[stelle] = neu;
            let text = ahv_schreiben(&f, weise);
            if erkannt(&text, "ahv") {
                return Some(text);
            }
        }
    }
    None
}

/// Das nächste sichtbare Zeichen vor `i` oder ab `i`; unsichtbare Formatzeichen übergeht
/// der Erkenner.
fn sichtbar_davor(s: &str, i: usize) -> Option<char> {
    s[..i].chars().rev().find(|&c| !ist_unsichtbar(c))
}

fn sichtbar_danach(s: &str, i: usize) -> Option<char> {
    s[i..].chars().find(|&c| !ist_unsichtbar(c))
}

/// Die Referenz trifft die Beispiele der Wegleitung (Rz 1202 und Anhang 7 WL VA/IK).
#[test]
fn ahv_referenz_trifft_die_beispiele_der_wegleitung() {
    let punkte = Schreibweise::Punkte(None);
    assert_eq!(
        ahv_schreiben(&ahv_nummer(&[1, 2, 3, 4, 5, 6, 7, 8, 9]), &punkte),
        "756.1234.5678.97"
    );
    assert_eq!(
        ahv_schreiben(&ahv_nummer(&[3, 0, 4, 7, 5, 0, 0, 9, 6]), &punkte),
        "756.3047.5009.62"
    );
    let z = ahv_nummer(&[9, 8, 7, 6, 5, 4, 3, 2, 1]);
    assert_eq!(
        ahv_schreiben(&z, &Schreibweise::Leerzeichen([' ', '\u{A0}', ' '])),
        "756 9876\u{A0}5432 17"
    );
    assert_eq!(
        ahv_schreiben(&z, &Schreibweise::Punkte(Some((2, " \n")))),
        "756.9876. \n5432.17"
    );
    assert_eq!(ahv_schreiben(&z, &Schreibweise::Ohne), "7569876543217");
}

/// (1) Jede gültige Nummer wird in beliebigem Umgebungstext an ihrer Stelle erkannt, in
/// jeder Schreibweise: ohne Trenner, mit Punkten, mit einem Umbruch nach einem Punkt und mit
/// Leerzeichen. Einzige Ausnahme, und sie kommt im Lauf vor: In der Schreibweise mit
/// Leerzeichen verbindet ein Leerzeichen davor oder danach die Nummer mit einer Ziffer; dann
/// ist sie Teil einer Zahlenkolonne. 4000 Fälle auf vier Schreibweisen, je etwa 1000.
#[test]
fn ahv_jede_gueltige_nummer_wird_erkannt() {
    let strategie = (
        prop::collection::vec(0u8..10, 9),
        schreibweise(),
        umgebung(),
        trenner(),
        trenner(),
        umgebung(),
    );
    let kolonnen = Cell::new(0usize);
    laeufer(0xA4F0_0001, 4000)
        .run(&strategie, |(neun, weise, vor, t1, t2, nach)| {
            let z = ahv_nummer(&neun);
            let summe: i32 = z.iter().zip(GEWICHTE).map(|(&d, g)| i32::from(d) * g).sum();
            prop_assert_eq!(summe % 10, 0);
            let (text, bereich) = eingebettet(&ahv_schreiben(&z, &weise), &vor, t1, t2, &nach);
            let normalisiert = Text::neu(&text);
            let s = normalisiert.als_str();
            let ziffer = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
            let kolonne = matches!(weise, Schreibweise::Leerzeichen(_))
                && ((LEERZEICHEN.contains(&t1)
                    && ziffer(sichtbar_davor(s, bereich.0 - t1.len_utf8())))
                    || (LEERZEICHEN.contains(&t2)
                        && ziffer(sichtbar_danach(s, bereich.1 + t2.len_utf8()))));
            if kolonne {
                kolonnen.set(kolonnen.get() + 1);
            }
            prop_assert_eq!(
                stellen(&text, "ahv").contains(&bereich),
                !kolonne,
                "{:?}",
                text
            );
            Ok(())
        })
        .unwrap();
    assert!(kolonnen.get() > 0, "keine Zahlenkolonne im Lauf");
}

/// (2) Jede Ersetzung einer einzelnen Ziffer wird abgewiesen, an jeder Stelle mit jeder
/// anderen Ziffer: mit Punkten, ohne Trenner und in einer zufälligen Schreibweise.
#[test]
fn ahv_jede_ersetzung_einer_ziffer_wird_abgewiesen() {
    let strategie = (prop::collection::vec(0u8..10, 9), schreibweise());
    laeufer(0xA4F0_0002, 250)
        .run(&strategie, |(neun, weise)| {
            let z = ahv_nummer(&neun);
            for weise in [Schreibweise::Punkte(None), Schreibweise::Ohne, weise] {
                prop_assert!(erkannt(&ahv_schreiben(&z, &weise), "ahv"));
                prop_assert_eq!(ahv_erkannte_ersetzung(&z, &weise), None);
            }
            Ok(())
        })
        .unwrap();
}

/// (2) Dasselbe erschöpfend an festen Nummern, in allen festen Schreibweisen: den
/// Beispielen der Wegleitung und Nummern aus lauter gleichen oder abwechselnden Ziffern.
#[test]
fn ahv_ersetzung_an_festen_nummern() {
    for neun in [
        [1, 2, 3, 4, 5, 6, 7, 8, 9],
        [3, 0, 4, 7, 5, 0, 0, 9, 6],
        [0; 9],
        [9; 9],
        [1, 6, 0, 0, 0, 0, 0, 0, 0],
        [5, 0, 5, 0, 5, 0, 5, 0, 5],
    ] {
        let z = ahv_nummer(&neun);
        for weise in alle_schreibweisen() {
            assert!(erkannt(&ahv_schreiben(&z, &weise), "ahv"), "{weise:?}");
            assert_eq!(ahv_erkannte_ersetzung(&z, &weise), None, "{weise:?}");
        }
    }
}

/// (2) Beweis für jede Nummer: Eine Ersetzung von a durch b an einer Stelle mit Gewicht g
/// ändert die Prüfsumme um g·(b − a). Für g = 1 oder 3 und a ≠ b ist das nie durch 10
/// teilbar; die Prüfung hängt nur von der Summe modulo 10 ab.
#[test]
fn beweis_ahv_ersetzung() {
    for g in GEWICHTE {
        for a in 0..10 {
            for b in (0..10).filter(|&b| b != a) {
                assert_ne!((g * (b - a)).rem_euclid(10), 0, "g={g} a={a} b={b}");
            }
        }
    }
}

/// (3) Jede Vertauschung zweier verschiedener Nachbarziffern wird abgewiesen, ausser beide
/// stehen nach 756 und unterscheiden sich um 5, in jeder Schreibweise. Genau diese Ausnahme
/// gilt, und sie kommt im Lauf vor.
#[test]
fn ahv_vertauschung_nur_bei_differenz_5_unerkannt() {
    let ausnahmen = Cell::new(0usize);
    let strategie = (prop::collection::vec(0u8..10, 9), schreibweise());
    laeufer(0xA4F0_0003, 1000)
        .run(&strategie, |(neun, weise)| {
            let z = ahv_nummer(&neun);
            for (i, paar) in z.windows(2).enumerate() {
                let (a, b) = (paar[0], paar[1]);
                if a == b {
                    continue;
                }
                let mut f = z;
                f.swap(i, i + 1);
                let erwartet = i >= 3 && a.abs_diff(b) == 5;
                if erwartet {
                    ausnahmen.set(ausnahmen.get() + 1);
                }
                prop_assert_eq!(
                    erkannt(&ahv_schreiben(&f, &weise), "ahv"),
                    erwartet,
                    "Stellen {} und {} vertauscht in {:?}",
                    i + 1,
                    i + 2,
                    ahv_schreiben(&z, &weise)
                );
            }
            Ok(())
        })
        .unwrap();
    assert!(ausnahmen.get() > 0, "die Ausnahme kam im Lauf nicht vor");
}

/// (3) Beweis für jede Nummer: Nachbarstellen haben die Gewichte 1 und 3. Die Vertauschung
/// von a und b ändert die Prüfsumme um (g1 − g2)·(b − a) = ±2·(b − a); durch 10 teilbar
/// genau dann, wenn |a − b| = 5. Vertauschungen in 756 zerstören den Ländercode.
#[test]
fn beweis_ahv_vertauschung() {
    for paar in GEWICHTE.windows(2) {
        for a in 0..10i32 {
            for b in (0..10).filter(|&b| b != a) {
                let aenderung = (paar[0] - paar[1]) * (b - a);
                assert_eq!(aenderung.rem_euclid(10) == 0, (a - b).abs() == 5);
            }
        }
    }
}

/// (3) Die Ausnahme an einem Beispiel: 1 und 6 unterscheiden sich um 5.
#[test]
fn ahv_ausnahme_an_einem_beispiel() {
    assert!(erkannt("756.1600.0000.03", "ahv"));
    assert!(erkannt("756.6100.0000.03", "ahv"));
    // 1 und 2 unterscheiden sich um 1: Die Vertauschung fällt auf.
    assert!(erkannt("756.1234.5678.97", "ahv"));
    assert!(!erkannt("756.2134.5678.97", "ahv"));
}

/// Zahlenkolonnen: 756 und zehn zufällige Ziffern in einer zufälligen Schreibweise, auch mit
/// Leerzeichen und mit einem Umbruch nach einem Punkt. Erkannt genau dann, wenn die
/// Kontrollziffer stimmt; meist stimmt sie nicht, manchmal zufällig. Beides kommt im Lauf
/// vor.
#[test]
fn ahv_zahlenkolonnen_nur_mit_stimmender_kontrollziffer() {
    let strategie = (prop::collection::vec(0u8..10, 10), schreibweise());
    let (stimmt, falsch) = (Cell::new(0usize), Cell::new(0usize));
    laeufer(0xA4F0_0004, 2000)
        .run(&strategie, |(zehn, weise)| {
            let mut z = [0u8; 13];
            z[..3].copy_from_slice(&[7, 5, 6]);
            z[3..].copy_from_slice(&zehn);
            let erwartet = z[12] == ahv_kontrollziffer(&z[..12]);
            let text = ahv_schreiben(&z, &weise);
            prop_assert_eq!(erkannt(&text, "ahv"), erwartet, "{:?}", text);
            let zaehler = if erwartet { &stimmt } else { &falsch };
            zaehler.set(zaehler.get() + 1);
            Ok(())
        })
        .unwrap();
    assert!(stimmt.get() > 0, "keine stimmende Kontrollziffer im Lauf");
    assert!(falsch.get() > 0, "keine falsche Kontrollziffer im Lauf");
}

// --- IBAN -----------------------------------------------------------------------------

/// Alle Zeichen, die in einer IBAN stehen dürfen (ISO 13616-1:2020, Ziffer 4: «c»).
fn alphabet() -> Vec<char> {
    ('0'..='9').chain('A'..='Z').chain('a'..='z').collect()
}

/// Ausgeschriebene Ziffern eines Zeichens nach ISO 13616-1:2020, Ziffer 6.2.3: die Ziffer
/// selbst, ein Buchstabe gross oder klein als 10 (A) bis 35 (Z).
fn ausgeschrieben(c: char) -> String {
    match c {
        '0'..='9' => c.to_string(),
        'A'..='Z' => (u32::from(c) - u32::from('A') + 10).to_string(),
        'a'..='z' => (u32::from(c) - u32::from('a') + 10).to_string(),
        _ => panic!("kein IBAN-Zeichen: {c:?}"),
    }
}

/// Rest modulo 97 der ausgeschriebenen Ziffernfolge, Ziffer für Ziffer (Ziffer 6.2.4).
fn rest97(zeichen: &str) -> u32 {
    zeichen
        .chars()
        .flat_map(|c| ausgeschrieben(c).into_bytes())
        .fold(0, |r, b| (r * 10 + u32::from(b - b'0')) % 97)
}

/// IBAN nach Ziffer 6.3: Ländercode und «00» an die BBAN hängen, Rest modulo 97 von 98
/// abziehen.
fn iban_erzeugen(land: &str, bban: &str) -> String {
    let pruefziffern = 98 - rest97(&format!("{bban}{land}00"));
    format!("{land}{pruefziffern:02}{bban}")
}

/// Gültig nach Ziffer 5, 6.2 und 6.3.3 in der 21-stelligen Ausprägung für CH und LI.
fn iban_gueltig(iban: &str) -> bool {
    let z: Vec<char> = iban.chars().collect();
    z.len() == 21
        && z.iter().all(char::is_ascii_alphanumeric)
        && matches!(&iban[..2], "CH" | "LI")
        && z[2..9].iter().all(char::is_ascii_digit)
        && (2..=98).contains(&iban[2..4].parse::<u32>().unwrap_or(0))
        && rest97(&format!("{}{}", &iban[4..], &iban[..4])) == 1
}

/// Papierformat: Vierergruppen, getrennt durch `leer`.
fn gruppiert(iban: &str, leer: char) -> String {
    let mut s = String::new();
    for (i, c) in iban.chars().enumerate() {
        if i > 0 && i % 4 == 0 {
            s.push(leer);
        }
        s.push(c);
    }
    s
}

/// Leerraum ohne Zeilenumbruch zwischen zwei Vierergruppen: Leerzeichen, Tabulator, die
/// Varianten aus [`LEERZEICHEN`], das Geviert-Leerzeichen und das ideographische.
const IBAN_LEERRAUM: [char; 8] = [
    ' ', '\t', '\u{00A0}', '\u{202F}', '\u{2009}', '\u{2007}', '\u{2003}', '\u{3000}',
];

/// Zeilenumbrüche zwischen zwei Vierergruppen; CRLF wird beim Einlesen zu `\n`.
const IBAN_UMBRUECHE: [&str; 7] = [
    "\n", "\r\n", "\u{0B}", "\u{0C}", "\u{85}", "\u{2028}", "\u{2029}",
];

/// Leerraum zwischen zwei Vierergruppen: ein bis drei Zeichen, darunter höchstens ein
/// Zeilenumbruch.
fn iban_luecke() -> impl Strategy<Value = String> {
    let leer = || prop::sample::select(IBAN_LEERRAUM.to_vec());
    prop_oneof![
        prop::collection::vec(leer(), 1..=3).prop_map(String::from_iter),
        (
            prop::collection::vec(leer(), 0..=2),
            prop::sample::select(IBAN_UMBRUECHE.to_vec()),
            0usize..=2,
        )
            .prop_map(|(leer, umbruch, stelle)| {
                let (vor, nach) = leer.split_at(stelle.min(leer.len()));
                format!(
                    "{}{umbruch}{}",
                    String::from_iter(vor),
                    String::from_iter(nach)
                )
            }),
    ]
}

/// Papierformat mit eigenem Leerraum in jeder der fünf Lücken.
fn mit_luecken(iban: &str, luecken: &[String; 5]) -> String {
    let mut s = String::new();
    for (i, c) in iban.chars().enumerate() {
        if i > 0 && i % 4 == 0 {
            s.push_str(&luecken[i / 4 - 1]);
        }
        s.push(c);
    }
    s
}

/// Zahl der Zeilenumbrüche im Leerraum der Lücken.
fn umbrueche(luecken: &[String; 5]) -> usize {
    luecken
        .iter()
        .filter(|l| IBAN_UMBRUECHE.iter().any(|u| l.contains(u)))
        .count()
}

fn kontozeichen() -> impl Strategy<Value = char> {
    prop_oneof![
        6 => prop::char::range('0', '9'),
        2 => prop::char::range('A', 'Z'),
        1 => prop::char::range('a', 'z'),
    ]
}

/// Gültige IBAN aus CH oder LI: IID aus fünf Ziffern, Kontonummer aus zwölf Ziffern oder
/// Buchstaben, Prüfziffern nach der Referenz.
fn iban_strategie() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["CH", "LI"]),
        prop::collection::vec(prop::char::range('0', '9'), 5),
        prop::collection::vec(kontozeichen(), 12),
    )
        .prop_map(|(land, iid, konto)| {
            let bban: String = iid.into_iter().chain(konto).collect();
            iban_erzeugen(land, &bban)
        })
}

/// Ersatz von `alt` durch `neu` in der Kontonummer bei Rest `p` der Zeichen davor (ab der
/// IID): Der Rest modulo 97 bleibt gleich genau dann, wenn (siehe `beweis_iban_ersetzung`)
/// zwei Buchstaben denselben Wert haben oder eine Ziffer d und ein Buchstabe mit Wert V
/// die Bedingung V − d − 7·p ≡ 0 (mod 97) erfüllen. Zwei Ziffern nie.
fn ersatz_bleibt_gleich(p: u32, alt: char, neu: char) -> bool {
    let wert = |c: char| i64::from(c.to_digit(36).expect("IBAN-Zeichen"));
    match (alt.is_ascii_digit(), neu.is_ascii_digit()) {
        (true, true) => false,
        (false, false) => wert(alt) == wert(neu),
        (alt_ziffer, _) => {
            let (d, v) = if alt_ziffer {
                (wert(alt), wert(neu))
            } else {
                (wert(neu), wert(alt))
            };
            (v - d - 7 * i64::from(p)).rem_euclid(97) == 0
        }
    }
}

/// Bleibt die Ersetzung an `stelle` (0-basiert) unerkannt? Nur in der Kontonummer
/// (Stellen 10 bis 21): Ländercode, Prüfziffern und IID ändern sich sonst oder verlieren
/// ihren Aufbau.
fn ersetzung_bleibt_unerkannt(iban: &str, stelle: usize, alt: char, neu: char) -> bool {
    stelle >= 9 && ersatz_bleibt_gleich(rest97(&iban[4..stelle]), alt, neu)
}

/// Bleibt die Vertauschung der Nachbarn an `stelle` und `stelle + 1` unerkannt? Genau dann,
/// wenn beide in der Kontonummer stehen und «ab» und «ba» ausgeschrieben gleich sind
/// (siehe `beweis_iban_vertauschung`).
fn vertauschung_bleibt_unerkannt(stelle: usize, a: char, b: char) -> bool {
    stelle >= 9 && ausgeschrieben(a) + &ausgeschrieben(b) == ausgeschrieben(b) + &ausgeschrieben(a)
}

/// Die Referenz trifft die Beispiele von SIX.
#[test]
fn iban_referenz_trifft_die_beispiele_von_six() {
    assert_eq!(
        iban_erzeugen("CH", "00762011623852957"),
        "CH9300762011623852957"
    );
    assert_eq!(
        iban_erzeugen("CH", "06470016006671002"),
        "CH6906470016006671002"
    );
    assert!(iban_gueltig("CH9300762011623852957"));
    // Das Beispiel der SIX-FAQ mit Buchstabe hat nicht stimmende Prüfziffern.
    assert!(!iban_gueltig("CH350023023012345601X"));
    assert!(iban_gueltig("CH640023023012345601X"));
}

/// (1) Jede gültige IBAN wird ohne Trenner und in Vierergruppen in beliebigem
/// Umgebungstext an ihrer Stelle erkannt.
#[test]
fn iban_jede_gueltige_wird_erkannt() {
    let leer = prop::option::of(prop::sample::select(vec![
        ' ', '\u{00A0}', '\u{202F}', '\u{2009}', '\u{2007}',
    ]));
    let strategie = (
        iban_strategie(),
        leer,
        umgebung(),
        trenner(),
        trenner(),
        umgebung(),
    );
    laeufer(0x1BA4_0001, 2000)
        .run(&strategie, |(iban, leer, vor, t1, t2, nach)| {
            prop_assert!(iban_gueltig(&iban));
            let geschrieben = match leer {
                Some(l) => gruppiert(&iban, l),
                None => iban.clone(),
            };
            let (text, bereich) = eingebettet(&geschrieben, &vor, t1, t2, &nach);
            prop_assert!(
                stellen(&text, "iban").contains(&bereich),
                "nicht erkannt: {:?}",
                text
            );
            Ok(())
        })
        .unwrap();
}

/// (1) mit Leerraum: Jede gültige IBAN in Vierergruppen wird an ihrer Stelle erkannt, wenn
/// zwischen je zwei Gruppen ein bis drei Leerraumzeichen jeder Art stehen, darunter höchstens
/// ein Zeilenumbruch: umbrochen wie in einer E-Mail oder einem PDF, mit Tabulatoren wie aus
/// einer Tabelle. Umbrüche in mehreren Lücken kommen im Lauf vor.
#[test]
fn iban_jede_gueltige_mit_leerraum_wird_erkannt() {
    let strategie = (
        iban_strategie(),
        prop::array::uniform5(iban_luecke()),
        umgebung(),
        trenner(),
        trenner(),
        umgebung(),
    );
    let mehrfach_umbrochen = Cell::new(0usize);
    laeufer(0x1BA4_0004, 2000)
        .run(&strategie, |(iban, luecken, vor, t1, t2, nach)| {
            if umbrueche(&luecken) > 1 {
                mehrfach_umbrochen.set(mehrfach_umbrochen.get() + 1);
            }
            let geschrieben = mit_luecken(&iban, &luecken);
            let (text, bereich) = eingebettet(&geschrieben, &vor, t1, t2, &nach);
            prop_assert!(
                stellen(&text, "iban").contains(&bereich),
                "nicht erkannt: {:?}",
                text
            );
            Ok(())
        })
        .unwrap();
    assert!(
        mehrfach_umbrochen.get() > 0,
        "kein mehrfacher Umbruch im Lauf"
    );
}

/// (2) mit Leerraum: Ersetzt eine Ziffer an beliebiger Stelle eine andere, wird die IBAN
/// abgewiesen, auch in Vierergruppen mit Leerraum und Umbrüchen dazwischen.
#[test]
fn iban_ersetzte_ziffer_auch_mit_leerraum_abgewiesen() {
    let strategie = (
        iban_strategie(),
        prop::array::uniform5(iban_luecke()),
        any::<prop::sample::Index>(),
        1u8..10,
    );
    laeufer(0x1BA4_0005, 2000)
        .run(&strategie, |(iban, luecken, wahl, plus)| {
            let ziffern: Vec<usize> = iban
                .bytes()
                .enumerate()
                .filter(|(_, b)| b.is_ascii_digit())
                .map(|(i, _)| i)
                .collect();
            let stelle = ziffern[wahl.index(ziffern.len())];
            let mut falsch = iban.clone().into_bytes();
            falsch[stelle] = b'0' + (falsch[stelle] - b'0' + plus) % 10;
            let falsch = String::from_utf8(falsch).expect("ASCII");
            prop_assert!(!iban_gueltig(&falsch), "Referenz: {}", falsch);
            let text = mit_luecken(&falsch, &luecken);
            prop_assert!(!erkannt(&text, "iban"), "erkannt: {:?}", text);
            Ok(())
        })
        .unwrap();
}

/// Zahlenkolonnen: «CH» oder «LI» und 19 zufällige Ziffern in Vierergruppen, mit Leerraum
/// und Umbrüchen dazwischen. Erkannt genau dann, wenn die
/// Referenz die Prüfziffern gelten lässt; meist stimmen sie nicht, manchmal zufällig.
/// Beides kommt im Lauf vor.
#[test]
fn iban_zahlenkolonnen_nur_mit_stimmenden_pruefziffern() {
    let strategie = (
        prop::sample::select(vec!["CH", "LI"]),
        prop::collection::vec(prop::char::range('0', '9'), 19),
        prop::array::uniform5(iban_luecke()),
    );
    let (stimmt, falsch) = (Cell::new(0usize), Cell::new(0usize));
    laeufer(0x1BA4_0006, 3000)
        .run(&strategie, |(land, ziffern, luecken)| {
            let kompakt = format!("{land}{}", String::from_iter(ziffern));
            let erwartet = iban_gueltig(&kompakt);
            let text = mit_luecken(&kompakt, &luecken);
            prop_assert_eq!(erkannt(&text, "iban"), erwartet, "{:?}", text);
            let zaehler = if erwartet { &stimmt } else { &falsch };
            zaehler.set(zaehler.get() + 1);
            Ok(())
        })
        .unwrap();
    assert!(stimmt.get() > 0, "keine stimmenden Prüfziffern im Lauf");
    assert!(falsch.get() > 0, "keine falschen Prüfziffern im Lauf");
}

/// Mehr Leerraum, als der Erkenner zulässt, verbindet keine Gruppen: vier Leerraumzeichen
/// oder zwei Umbrüche in einer Lücke, also eine Leerzeile.
#[test]
fn iban_zu_viel_leerraum_trennt_die_gruppen() {
    let iban = "CH9300762011623852957";
    assert!(erkannt(&gruppiert(iban, ' '), "iban"));
    for zuviel in ["    ", "\n\n", " \n\n", "\t\t\t\t", "\n \n", "\u{2028}\n"] {
        for luecke in 0..5 {
            let mut luecken: [String; 5] = Default::default();
            for (i, l) in luecken.iter_mut().enumerate() {
                *l = if i == luecke { zuviel } else { " " }.to_string();
            }
            let text = mit_luecken(iban, &luecken);
            assert!(!erkannt(&text, "iban"), "{text:?}");
        }
    }
}

/// (2) und (4) Ersetzung eines Zeichens, an jeder Stelle durch jedes andere Zeichen: Die
/// Referenz und der Erkenner weisen genau das ab, was die Ausnahmen nicht erlauben. Jede
/// Ziffer statt einer anderen Ziffer wird abgewiesen; MOD 97-10 lässt aber Gross statt
/// Klein und manche Ziffer statt Buchstabe durch. Beides kommt im Lauf vor.
#[test]
fn iban_ersetzung_genau_mit_den_ausnahmen() {
    let alphabet = alphabet();
    let gross_klein = Cell::new(0usize);
    let ziffer_buchstabe = Cell::new(0usize);
    laeufer(0x1BA4_0002, 100)
        .run(&iban_strategie(), |iban| {
            prop_assert!(erkannt(&iban, "iban"));
            let z: Vec<char> = iban.chars().collect();
            for (stelle, &alt) in z.iter().enumerate() {
                for &neu in alphabet.iter().filter(|&&c| c != alt) {
                    let mut f = z.clone();
                    f[stelle] = neu;
                    let falsch: String = f.into_iter().collect();
                    let erwartet = ersetzung_bleibt_unerkannt(&iban, stelle, alt, neu);
                    prop_assert_eq!(iban_gueltig(&falsch), erwartet, "Referenz: {}", falsch);
                    prop_assert_eq!(erkannt(&falsch, "iban"), erwartet, "Erkenner: {}", falsch);
                    if alt.is_ascii_digit() && neu.is_ascii_digit() {
                        prop_assert!(!erwartet);
                    }
                    if erwartet {
                        let zaehler = if alt.is_ascii_digit() == neu.is_ascii_digit() {
                            &gross_klein
                        } else {
                            &ziffer_buchstabe
                        };
                        zaehler.set(zaehler.get() + 1);
                    }
                }
            }
            Ok(())
        })
        .unwrap();
    assert!(gross_klein.get() > 0, "kein Wechsel gross/klein im Lauf");
    assert!(
        ziffer_buchstabe.get() > 0,
        "kein Ersatz Ziffer/Buchstabe im Lauf"
    );
}

/// (4) Beweis für jede Stelle der Kontonummer: P sei der Rest der Zeichen davor, t die Zahl
/// der ausgeschriebenen Ziffern dahinter. Der Rest bleibt gleich genau dann, wenn
/// P·10^len(neu) + Wert(neu) ≡ P·10^len(alt) + Wert(alt) (mod 97), denn 10^t ist
/// teilerfremd zu 97. Nachgerechnet für alle 97 Werte von P und alle Zeichenpaare; die
/// geschlossene Form in `ersatz_bleibt_gleich` stimmt in jedem Fall.
#[test]
fn beweis_iban_ersetzung() {
    let alphabet = alphabet();
    let teil = |p: i64, c: char| {
        let s = ausgeschrieben(c);
        p * 10i64.pow(s.len() as u32) + s.parse::<i64>().expect("Ziffern")
    };
    for p in 0..97u32 {
        for &alt in &alphabet {
            for &neu in alphabet.iter().filter(|&&c| c != alt) {
                let gleich =
                    (teil(i64::from(p), neu) - teil(i64::from(p), alt)).rem_euclid(97) == 0;
                assert_eq!(
                    gleich,
                    ersatz_bleibt_gleich(p, alt, neu),
                    "P={p} {alt}->{neu}"
                );
            }
        }
    }
}

/// (3) und (4) Vertauschung zweier verschiedener Nachbarn: abgewiesen genau dann, wenn
/// sich die Ausschreibung ändert. Jede Vertauschung zweier Ziffern wird abgewiesen.
#[test]
fn iban_vertauschung_genau_mit_den_ausnahmen() {
    laeufer(0x1BA4_0003, 1500)
        .run(&iban_strategie(), |iban| {
            let z: Vec<char> = iban.chars().collect();
            for (stelle, paar) in z.windows(2).enumerate() {
                let (a, b) = (paar[0], paar[1]);
                if a == b {
                    continue;
                }
                let mut f = z.clone();
                f.swap(stelle, stelle + 1);
                let falsch: String = f.into_iter().collect();
                let erwartet = vertauschung_bleibt_unerkannt(stelle, a, b);
                prop_assert_eq!(iban_gueltig(&falsch), erwartet, "Referenz: {}", falsch);
                prop_assert_eq!(erkannt(&falsch, "iban"), erwartet, "Erkenner: {}", falsch);
                if a.is_ascii_digit() && b.is_ascii_digit() {
                    prop_assert!(!erwartet);
                }
            }
            Ok(())
        })
        .unwrap();
}

/// (4) Beweis für jede Stelle der Kontonummer: Die Vertauschung ändert den Rest um
/// 10^t·(Wert(«ba») − Wert(«ab»)), beide gleich lang ausgeschrieben. Für alle Zeichenpaare:
/// durch 97 teilbar genau dann, wenn die Ausschreibungen gleich sind. Das sind zwei
/// Buchstaben gleichen Werts (gross und klein) und genau die Paare 1B, 2M, 3X in beiden
/// Reihenfolgen und Schreibungen.
#[test]
fn beweis_iban_vertauschung() {
    let alphabet = alphabet();
    let mut ziffer_und_buchstabe = Vec::new();
    for &a in &alphabet {
        for &b in alphabet.iter().filter(|&&b| b != a) {
            let ab = ausgeschrieben(a) + &ausgeschrieben(b);
            let ba = ausgeschrieben(b) + &ausgeschrieben(a);
            let differenz =
                ab.parse::<i64>().expect("Ziffern") - ba.parse::<i64>().expect("Ziffern");
            assert_eq!(differenz.rem_euclid(97) == 0, ab == ba, "{a}{b}");
            if ab == ba && a.is_ascii_digit() != b.is_ascii_digit() {
                ziffer_und_buchstabe.push(format!("{a}{b}"));
            }
        }
    }
    assert_eq!(
        ziffer_und_buchstabe,
        [
            "1B", "1b", "2M", "2m", "3X", "3x", "B1", "M2", "X3", "b1", "m2", "x3"
        ]
    );
}

/// (4) Die Ausnahmen an Beispielen.
#[test]
fn iban_ausnahmen_an_beispielen() {
    // Ersatz einer Ziffer durch einen Buchstaben: «T» oder «t» statt der «2» an Stelle 14.
    for text in [
        "CH93 0076 2011 6238 5295 7",
        "CH93 0076 2011 6T38 5295 7",
        "CH93 0076 2011 6t38 5295 7",
    ] {
        assert!(erkannt(text, "iban"), "{text}");
    }
    assert!(ersetzung_bleibt_unerkannt(
        "CH9300762011623852957",
        13,
        '2',
        'T'
    ));
    // Vertauschte Nachbarn mit gleicher Ausschreibung, an jeder Stelle der Kontonummer.
    for paar in ["1B", "2M", "3X", "1b", "2m", "3x", "Aa", "zZ"] {
        let p: Vec<char> = paar.chars().collect();
        for stelle in 0..11 {
            let mut konto = vec!['0'; 12];
            konto[stelle] = p[0];
            konto[stelle + 1] = p[1];
            let konto: String = konto.into_iter().collect();
            let iban = iban_erzeugen("CH", &format!("00762{konto}"));
            let mut f: Vec<char> = iban.chars().collect();
            f.swap(9 + stelle, 10 + stelle);
            let getauscht: String = f.into_iter().collect();
            assert!(erkannt(&iban, "iban"), "{iban}");
            assert!(erkannt(&getauscht, "iban"), "{iban} -> {getauscht}");
        }
    }
}

/// Prüfziffern 00, 01 und 99 bestehen die Rechnung modulo 97 wie 97, 98 und 02, entstehen
/// aber bei der Erzeugung nie (ISO 13616-1:2020, Ziffer 6.3.3) und werden abgewiesen.
#[test]
fn iban_pruefziffern_00_01_99_werden_abgewiesen() {
    for (gueltig, ersatz) in [("97", "00"), ("98", "01"), ("02", "99")] {
        let iban = (0u64..)
            .map(|n| iban_erzeugen("CH", &format!("00762{n:012}")))
            .find(|i| &i[2..4] == gueltig)
            .expect("Kontonummer mit diesen Prüfziffern");
        let falsch = format!("CH{ersatz}{}", &iban[4..]);
        assert_eq!(
            rest97(&format!("{}{}", &falsch[4..], &falsch[..4])),
            1,
            "{falsch}"
        );
        assert!(erkannt(&iban, "iban"), "{iban}");
        assert!(!erkannt(&falsch, "iban"), "{falsch}");
        assert!(!iban_gueltig(&falsch), "{falsch}");
    }
}

// --- Alle vier Regeln -----------------------------------------------------------------

/// Bausteine, die die Erkenner an ihre Grenzfälle führen: Ziffern, Trenner, Präfixe, «@»,
/// Punkte, Klammern, Zeichen ausserhalb ASCII, ein kombinierendes Zeichen, das die
/// Normalisierung mit dem Zeichen davor verschmilzt, und ganze gültige Nummern.
fn baustein() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => prop::sample::select(vec![
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", " ", ".", "-", "/", "@", "+",
            "(", ")", "_", ":", "\n", "\u{00A0}", "\u{202F}", "a", "e", "Z", "ä", "é",
            "\u{0301}", "\u{1F600}",
        ])
        .prop_map(str::to_string),
        2 => prop::sample::select(vec![
            "756", "+41", "0041", "(0)", "CH", "LI", "93", "044", "079", "xn--",
            "example.com", "info", "://", "..",
        ])
        .prop_map(str::to_string),
        1 => prop::sample::select(vec![
            "756.1234.5678.97", "7561234567897", "CH9300762011623852957",
            "CH93 0076 2011 6238 5295 7", "+41 44 123 45 67", "(044) 123 45 67",
            "079 123 45 67", "marie@example.com",
        ])
        .prop_map(str::to_string),
        1 => any::<char>().prop_map(String::from),
    ]
}

/// Zusatz zu (1) bis (4): Auf beliebigem Text stürzt kein Erkenner ab. Jede Stelle liegt
/// im normalisierten Text auf Zeichengrenzen und beginnt, wie ihre Regel es verlangt.
#[test]
fn beliebiger_text_ohne_absturz_mit_stellen_auf_zeichengrenzen() {
    let text = prop::collection::vec(baustein(), 0..48).prop_map(|teile| teile.concat());
    let gemeldet: [Cell<u32>; 4] = Default::default();
    laeufer(0x5A1E_0001, 3000)
        .run(&text, |roh| {
            let text = Text::neu(&roh);
            let s = text.als_str();
            for t in erkenne(&text) {
                prop_assert!(t.start < t.ende && t.ende <= s.len(), "{t:?}");
                prop_assert!(s.is_char_boundary(t.start) && s.is_char_boundary(t.ende));
                let stelle = &s[t.start..t.ende];
                let (nr, passt) = match t.regel {
                    "ahv" => (0, stelle.starts_with("756")),
                    "iban" => (1, stelle.starts_with("CH") || stelle.starts_with("LI")),
                    "tel" => (2, stelle.starts_with(['+', '0', '('])),
                    "mail" => (3, stelle.matches('@').count() == 1),
                    _ => (0, false),
                };
                prop_assert!(passt, "{} {stelle:?}", t.regel);
                gemeldet[nr].set(gemeldet[nr].get() + 1);
            }
            Ok(())
        })
        .unwrap();
    // Jeder Erkenner hat im Lauf auch etwas gemeldet.
    for (regel, n) in ["ahv", "iban", "tel", "mail"].iter().zip(&gemeldet) {
        assert!(n.get() > 0, "{regel}");
    }
}
