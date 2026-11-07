//! Der geprüfte Text: Normalisierung, Zeilen und Spalten, Satzgrenzen.
//!
//! * Zeilenenden werden zu `\n` (CRLF und CR aus Windows- und alten Mac-Dateien).
//! * Danach Normalform C nach Unicode UAX #15. Ein «ä» aus «a» und Trema ist danach
//!   dasselbe Zeichen wie ein vorkomponiertes «ä»; Wortlisten treffen beide.
//! * Satzgrenzen nach Unicode UAX #29, mit dokumentierten Anpassungen (siehe
//!   [`saetze_bestimmen`]). Ein einzelner Zeilenumbruch mitten in einem Absatz ist keine
//!   Grenze (siehe [`weiche_umbrueche_bestimmen`]). Wer einen Satzanfang oder ein
//!   Satzende braucht, prüft im Satzstück bis zum nächsten weichen Umbruch
//!   ([`Text::satzanfang_bei`], [`Text::satzstueck_bei`]); reguläre Ausdrücke über einen
//!   weichen Umbruch hinweg laufen auf [`Text::verbunden`].
//! * Unsichtbare Formatzeichen (siehe [`ist_unsichtbar`]) bleiben im Text, damit Zeile und
//!   Spalte stimmen. Wortlisten und die Erkenner der Klasse A überspringen sie; für
//!   reguläre Ausdrücke gibt es die Sicht [`OhneUnsichtbare`] mit dem Weg zurück.
//!
//! Alle Versätze in promptscan sind Byte-Versätze in diesem normalisierten Text.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::LazyLock;

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Abstand der Stützstellen für die Umrechnung Byte → Zeichen.
const STUETZ_ABSTAND: usize = 64;

/// Der normalisierte Text mit Zeilen- und Satzindex.
#[derive(Clone, Debug)]
pub struct Text {
    inhalt: String,
    zeilenanfaenge: Vec<usize>,
    saetze: Vec<Range<usize>>,
    /// Byte-Versätze der weichen Zeilenumbrüche, aufsteigend (siehe
    /// [`weiche_umbrueche_bestimmen`]).
    weich: Vec<usize>,
    /// Der Text mit jedem weichen Umbruch als Leerzeichen, `None` ohne weichen Umbruch
    /// (siehe [`Text::verbunden`]).
    verbunden: Option<String>,
    /// Je Block von [`STUETZ_ABSTAND`] Bytes: (Byte-Versatz einer Zeichengrenze am oder vor
    /// dem Blockanfang, Zahl der Zeichen davor). So kostet eine Umrechnung höchstens einen
    /// Block, auch bei Tausenden Funden auf einer langen Zeile.
    stuetzen: Vec<(usize, usize)>,
}

impl Text {
    pub fn neu(roh: &str) -> Text {
        let ohne_cr = roh.replace("\r\n", "\n").replace('\r', "\n");
        let inhalt: String = ohne_cr.nfc().collect();
        let zeilenanfaenge = std::iter::once(0)
            .chain(inhalt.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let weich = weiche_umbrueche_bestimmen(&inhalt);
        let verbunden = verbinden(&inhalt, &weich);
        let saetze = saetze_roh(verbunden.as_deref().unwrap_or(&inhalt), &ABKUERZUNGEN).0;
        let stuetzen = stuetzen_bauen(&inhalt);
        Text {
            inhalt,
            zeilenanfaenge,
            saetze,
            weich,
            verbunden,
            stuetzen,
        }
    }

    pub fn als_str(&self) -> &str {
        &self.inhalt
    }

    /// Der Text, in dem jeder weiche Umbruch ([`Text::weicher_umbruch`]) ein Leerzeichen
    /// ist. Beide sind ein Byte lang, alle Versätze gelten unverändert; ohne weichen Umbruch
    /// ist es [`Text::als_str`]. Für Regeln, die mit einem regulären Ausdruck über ein
    /// Zeilenende hinaus lesen sollen («geb.⏎12.03.1985», «wohnhaft in⏎Winterthur»), ohne
    /// das Muster für jeden Umbruch zu erweitern. Ein Trennstrich am Zeilenende bleibt
    /// stehen («arbeits- unfähig»); zusammengezogen wird er nur beim Vergleich der Wortlisten
    /// ([`crate::lexikon::falten_mit`]).
    pub fn verbunden(&self) -> &str {
        self.verbunden.as_deref().unwrap_or(&self.inhalt)
    }

    pub fn len(&self) -> usize {
        self.inhalt.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inhalt.is_empty()
    }

    /// Der Ausschnitt zwischen zwei Byte-Versätzen.
    pub fn ausschnitt(&self, bereich: Range<usize>) -> &str {
        &self.inhalt[bereich]
    }

    /// Zeile und Spalte zum Byte-Versatz, beide 1-basiert; die Spalte zählt Zeichen.
    pub fn position(&self, byte: usize) -> (usize, usize) {
        let index = match self.zeilenanfaenge.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let anfang = self.zeilenanfaenge[index];
        let spalte = self.zeichenversatz(byte) - self.zeichenversatz(anfang) + 1;
        (index + 1, spalte)
    }

    /// Versatz in Zeichen (0-basiert) zum Byte-Versatz.
    pub fn zeichenversatz(&self, byte: usize) -> usize {
        let (stelle, zeichen) = self.stuetzen[byte / STUETZ_ABSTAND];
        zeichen + self.inhalt[stelle..byte].chars().count()
    }

    /// Anzahl Zeilen; ein leerer Text hat eine leere Zeile.
    pub fn zeilen_anzahl(&self) -> usize {
        self.zeilenanfaenge.len()
    }

    /// Byte-Bereich der Zeile `nr` (1-basiert), ohne Zeilenende.
    pub fn zeilenbereich(&self, nr: usize) -> Range<usize> {
        let anfang = self.zeilenanfaenge[nr - 1];
        let ende = self
            .zeilenanfaenge
            .get(nr)
            .map(|naechste| naechste - 1)
            .unwrap_or(self.inhalt.len());
        anfang..ende
    }

    /// Die Zeile `nr` (1-basiert), ohne Zeilenende.
    pub fn zeile(&self, nr: usize) -> &str {
        &self.inhalt[self.zeilenbereich(nr)]
    }

    /// Alle Sätze als Byte-Bereiche, lückenlos und in Reihenfolge.
    ///
    /// Ein harter Zeilenumbruch beendet immer einen Satz. Ein `\n` zwischen zwei Wörtern
    /// desselben Satzes ist darum stets ein weicher Umbruch ([`Text::weicher_umbruch`]);
    /// so kann auch eine Regel, die nur den Ausschnitt eines Satzes hat, ihn erkennen.
    pub fn saetze(&self) -> &[Range<usize>] {
        &self.saetze
    }

    /// Der Satz, der den Byte-Versatz enthält.
    pub fn satz_bei(&self, byte: usize) -> Range<usize> {
        let index = self.saetze.partition_point(|s| s.end <= byte);
        self.saetze
            .get(index)
            .cloned()
            .unwrap_or(self.inhalt.len()..self.inhalt.len())
    }

    /// Liegen zwei Byte-Versätze im selben Satz?
    pub fn im_selben_satz(&self, a: usize, b: usize) -> bool {
        self.satz_bei(a) == self.satz_bei(b)
    }

    /// Steht bei `byte` ein weicher Zeilenumbruch, also ein `\n` mitten in einem Absatz,
    /// das keine Satzgrenze ist? Die Bedingungen stehen bei [`weiche_umbrueche_bestimmen`].
    ///
    /// Für Regeln, die Wörter über ein Zeilenende hinweg lesen wollen: Ein Name wie
    /// «Erika⏎Beispiel» steht über einem weichen Umbruch, «Erika⏎⏎Beispiel» und
    /// «- Erika⏎- Beispiel» nicht. Wer prüft, ob ein Wort am Satzanfang steht, nimmt
    /// [`Text::satzanfang_bei`]: Eine Anrede, ein Betreff oder ein Name ohne Satzzeichen
    /// («Guten Tag⏎Als Anwältin …») wird mit der nächsten Zeile zu einem Satz, und deren
    /// erstes Wort steht dann nicht mehr am Anfang von [`Text::satz_bei`].
    pub fn weicher_umbruch(&self, byte: usize) -> bool {
        self.weich.binary_search(&byte).is_ok()
    }

    /// Alle weichen Zeilenumbrüche als Byte-Versätze, aufsteigend.
    pub fn weiche_umbrueche(&self) -> &[usize] {
        &self.weich
    }

    /// Das Stück des Satzes um `byte` zwischen zwei Stückgrenzen: vom Zeichen nach der
    /// letzten Stückgrenze vor `byte`, sonst vom Satzanfang, bis einschliesslich zur
    /// nächsten Stückgrenze ab `byte`, sonst bis zum Satzende. Eine Stückgrenze ist ein
    /// weicher Umbruch, nach dem das erste Wortzeichen der nächsten Zeile ein Grossbuchstabe
    /// ist ([`Text::stueckgrenze`]). Ohne Stückgrenze im Satz ist es der ganze Satz
    /// ([`Text::satz_bei`]). Die Stücke eines Satzes liegen lückenlos hintereinander.
    ///
    /// So bleibt eine Prüfung, die nur in einer Zeile gelten soll, für eine Zeile, die gross
    /// beginnt, wie vor der Regel für weiche Umbrüche. Ein Beispiel ist «Steht das
    /// Fragezeichen im selben Stück wie der Treffer?»: In «Seit 2010 bin ich Ärztin⏎Darf ich
    /// die Diagnose weitergeben?» fragt die zweite Zeile, die erste nicht. Beginnt die
    /// nächste Zeile klein, setzt sie den Satz fort und gehört zum selben Stück: «Seit wann
    /// bin ich Ärztin⏎im Sinne des Gesetzes?» ist eine Frage, «Ab wann bin ich Ärztin,
    /// wenn⏎ich das Examen bestanden habe?» auch (Befund C, Erkennung).
    pub fn satzstueck_bei(&self, byte: usize) -> Range<usize> {
        let satz = self.satz_bei(byte);
        let i = self.weich.partition_point(|&p| p < byte);
        let anfang = self.weich[..i]
            .iter()
            .rev()
            .take_while(|&&p| p >= satz.start)
            .find(|&&p| self.stueckgrenze(p))
            .map_or(satz.start, |&p| (p + 1).max(satz.start));
        let ende = self.weich[i..]
            .iter()
            .take_while(|&&p| p < satz.end)
            .find(|&&p| self.stueckgrenze(p))
            .map_or(satz.end, |&p| (p + 1).min(satz.end));
        anfang..ende
    }

    /// Ist der weiche Umbruch bei `byte` eine Grenze zwischen zwei Satzstücken
    /// ([`Text::satzstueck_bei`])? Wahr, wenn das erste Wortzeichen danach ein
    /// Grossbuchstabe ist («Guten Tag⏎Als Anwältin …», «Personalien⏎Geboren am …»), nicht
    /// bei einem Kleinbuchstaben oder einer Ziffer («Sie arbeitet⏎als Anwältin, habe ich
    /// gehört»).
    fn stueckgrenze(&self, byte: usize) -> bool {
        self.inhalt[byte + 1..]
            .chars()
            .find(|&c| ist_wortzeichen(c))
            .is_some_and(char::is_uppercase)
    }

    /// Steht `byte` am Anfang eines Satzes? Wahr, wenn in seinem Satzstück
    /// ([`Text::satzstueck_bei`]) davor kein Buchstabe und keine Ziffer steht, höchstens
    /// Leerraum und Satzzeichen («Guten Tag⏎– Als Anwältin …»). `byte` liegt auf einer
    /// Zeichengrenze.
    ///
    /// Ein gross geschriebenes Wort am Anfang einer Zeile nach einem weichen Umbruch gilt
    /// also wie vor der Regel für weiche Umbrüche als möglicher Satzanfang. Das ist der Fall
    /// bei «Als» in «Sehr geehrte Damen und Herren,⏎Als Anwältin vertrete ich …», bei
    /// «Geboren» in «Personalien⏎Geboren am 12. März 1985» und bei «Cite» in «Bonjour⏎Cite
    /// les arrêts …». Ein klein geschriebenes Wort dort setzt den Satz fort und steht nicht
    /// am Satzanfang («Sie arbeitet⏎als Anwältin, habe ich gehört»). Grenze: Beginnt ein
    /// hart umbrochener Satz eine Zeile mit einem gross geschriebenen Wort, etwa einem Namen
    /// oder Substantiv («Er ist⏎Anwalt …»), gilt es ebenfalls als möglicher Satzanfang.
    pub fn satzanfang_bei(&self, byte: usize) -> bool {
        let stueck = self.satzstueck_bei(byte);
        !self.inhalt[stueck.start..byte].chars().any(ist_wortzeichen)
    }

    /// Die unsichtbaren Formatzeichen ([`ist_unsichtbar`]) für die Meldung in der Ausgabe,
    /// als Byte-Versatz und Zeichen. Nicht gezählt werden Zeichen, die zu einem sichtbaren
    /// Zeichen gehören: Variantenwähler (U+FE00–U+FE0F, U+E0100–U+E01EF), der Verbinder
    /// U+200D nach einem Symbol ausserhalb von ASCII (Emoji-Folgen wie U+1F468 U+200D
    /// U+1F469) und Tag-Zeichen nach der Flagge U+1F3F4.
    pub fn unsichtbare_formatzeichen(&self) -> Vec<(usize, char)> {
        let mut gefunden = Vec::new();
        let mut davor: Option<char> = None;
        for (i, c) in self.inhalt.char_indices() {
            if !ist_unsichtbar(c) {
                davor = Some(c);
                continue;
            }
            let symbol_davor =
                davor.is_some_and(|d| !d.is_ascii() && !d.is_alphanumeric() && !d.is_whitespace());
            let gehoert_zu_zeichen = matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}')
                || (c == '\u{200D}' && symbol_davor)
                || (matches!(c, '\u{E0020}'..='\u{E007F}') && davor == Some('\u{1F3F4}'));
            if !gehoert_zu_zeichen {
                gefunden.push((i, c));
            }
        }
        gefunden
    }

    /// Zahl der Ersatzzeichen U+FFFD. Ein Programm setzt sie, wo es beim Umwandeln eines
    /// Textes ein Zeichen nicht lesen konnte, etwa in einer falschen Kodierung.
    pub fn ersatzzeichen(&self) -> usize {
        self.inhalt.chars().filter(|&c| c == '\u{FFFD}').count()
    }

    /// Verdacht, dass eine Pipe Umlaute und andere Zeichen ausserhalb von ASCII zu «?»
    /// gemacht hat (Windows PowerShell 5.1 schickt Text vorgabemässig als ASCII an ein
    /// Programm): Der Text enthält kein Zeichen ausserhalb von ASCII, aber ein Wort mit «?»
    /// zwischen zwei Buchstaben («Anw?ltin»). Dann das erste solche Wort, ohne Satzzeichen
    /// am Rand ausser «?». Wörter, die wie eine Webadresse aussehen (mit «://», «/», «=»
    /// oder «&», oder am Anfang «www.»), zählen nicht. Wörter trennt ASCII-Leerraum; das
    /// Wort kann Steuerzeichen enthalten.
    pub fn ascii_pipe_wort(&self) -> Option<&str> {
        if !self.inhalt.is_ascii() {
            return None;
        }
        self.inhalt
            .split_ascii_whitespace()
            .find(|wort| !wie_webadresse(wort) && fragezeichen_im_wort(wort).is_some())
            .map(|wort| wort.trim_matches(|c: char| c.is_ascii_punctuation() && c != '?'))
    }
}

/// Byte-Versatz des ersten «?» zwischen zwei ASCII-Buchstaben.
pub fn fragezeichen_im_wort(wort: &str) -> Option<usize> {
    wort.as_bytes()
        .windows(3)
        .position(|w| w[1] == b'?' && w[0].is_ascii_alphabetic() && w[2].is_ascii_alphabetic())
        .map(|i| i + 1)
}

fn wie_webadresse(wort: &str) -> bool {
    wort.contains("://") || wort.starts_with("www.") || wort.contains(['/', '=', '&'])
}

/// Stützstellen für [`Text::zeichenversatz`]: für jeden Block die letzte Zeichengrenze am
/// oder vor dem Blockanfang und die Zahl der Zeichen davor. Ein Eintrag mehr als Blöcke,
/// damit auch der Versatz am Textende eine Stützstelle hat.
fn stuetzen_bauen(s: &str) -> Vec<(usize, usize)> {
    let mut stuetzen = Vec::with_capacity(s.len() / STUETZ_ABSTAND + 1);
    let mut zeichen = 0;
    let mut letzte_grenze = 0;
    let mut zeichen_bis_grenze = 0;
    let mut grenzen = s
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(s.len()));
    let mut naechste = grenzen.next();
    for block in 0..=s.len() / STUETZ_ABSTAND {
        let anfang = block * STUETZ_ABSTAND;
        while let Some(g) = naechste {
            if g > anfang {
                break;
            }
            letzte_grenze = g;
            zeichen_bis_grenze = zeichen;
            zeichen += 1;
            naechste = grenzen.next();
        }
        stuetzen.push((letzte_grenze, zeichen_bis_grenze));
    }
    stuetzen
}

/// Ist das Zeichen Teil eines Wortes? Buchstaben und Ziffern aller Schriften.
pub fn ist_wortzeichen(c: char) -> bool {
    c.is_alphanumeric()
}

/// Unsichtbares Formatzeichen: Unicode-Eigenschaft `Default_Ignorable_Code_Point`
/// (DerivedCoreProperties.txt, Unicode 16.0.0). Dazu gehören das weiche Trennzeichen
/// U+00AD, das Leerzeichen ohne Breite U+200B, die Verbinder U+200C und U+200D, die
/// Steuerzeichen der Schreibrichtung (U+200E, U+200F, U+202A–U+202E, U+2066–U+2069), der
/// Wortverbinder U+2060 und U+FEFF. Sie zeigen nichts an, trennen aber für einen
/// Zeichenvergleich ein Wort oder eine Nummer. Ein Test vergleicht die Tabelle mit der
/// Eigenschaft in der `regex`-Bibliothek.
pub fn ist_unsichtbar(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFF8}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// Ein Text ohne unsichtbare Formatzeichen ([`ist_unsichtbar`]), mit dem Weg zurück zu den
/// Versätzen im ganzen Text. Die Erkenner der Klasse A lesen ihn. Eine Regel mit einem
/// regulären Ausdruck kann es ebenso tun: das Muster auf [`OhneUnsichtbare::text`]
/// anwenden, dann Anfang und Ende jedes Treffers mit [`OhneUnsichtbare::anfang`] und
/// [`OhneUnsichtbare::ende`] zurückrechnen («See\u{AD}strasse 12» liest sich als
/// «Seestrasse 12»). Enthält der Text kein solches Zeichen, wird nichts kopiert.
pub struct OhneUnsichtbare<'a> {
    /// Der Text ohne die unsichtbaren Formatzeichen.
    pub text: Cow<'a, str>,
    /// Je entferntes Zeichen: sein Versatz im gekürzten Text und die Bytes, die bis
    /// einschliesslich zu ihm entfernt wurden.
    entfernt: Vec<(usize, usize)>,
}

impl<'a> OhneUnsichtbare<'a> {
    pub fn neu(s: &'a str) -> OhneUnsichtbare<'a> {
        if !s.chars().any(ist_unsichtbar) {
            return OhneUnsichtbare {
                text: Cow::Borrowed(s),
                entfernt: Vec::new(),
            };
        }
        let mut text = String::with_capacity(s.len());
        let mut entfernt = Vec::new();
        let mut bytes = 0;
        for c in s.chars() {
            if ist_unsichtbar(c) {
                bytes += c.len_utf8();
                entfernt.push((text.len(), bytes));
            } else {
                text.push(c);
            }
        }
        OhneUnsichtbare {
            text: Cow::Owned(text),
            entfernt,
        }
    }

    /// Der Versatz im ganzen Text zum Anfang einer Stelle bei `i` im gekürzten Text. Ein
    /// entferntes Zeichen genau dort steht vor der Stelle.
    pub fn anfang(&self, i: usize) -> usize {
        let k = self.entfernt.partition_point(|&(p, _)| p <= i);
        i + k.checked_sub(1).map_or(0, |k| self.entfernt[k].1)
    }

    /// Der Versatz im ganzen Text zum Ende einer Stelle bei `i` im gekürzten Text. Ein
    /// entferntes Zeichen genau dort steht nach der Stelle.
    pub fn ende(&self, i: usize) -> usize {
        let k = self.entfernt.partition_point(|&(p, _)| p < i);
        i + k.checked_sub(1).map_or(0, |k| self.entfernt[k].1)
    }
}

/// Steht der Bereich an Wortgrenzen? Vorher und nachher kein Wortzeichen.
pub fn an_wortgrenzen(s: &str, start: usize, ende: usize) -> bool {
    links_frei(s, start) && rechts_frei(s, ende)
}

/// Kein Wortzeichen unmittelbar vor `start`. Unsichtbare Formatzeichen dazwischen zählen
/// nicht: In «Mus\u{AD}ter» steht vor «ter» das Wortzeichen «s».
pub fn links_frei(s: &str, start: usize) -> bool {
    s[..start]
        .chars()
        .rev()
        .find(|&c| !ist_unsichtbar(c))
        .is_none_or(|c| !ist_wortzeichen(c))
}

/// Kein Wortzeichen unmittelbar ab `ende`; unsichtbare Formatzeichen zählen nicht.
pub fn rechts_frei(s: &str, ende: usize) -> bool {
    s[ende..]
        .chars()
        .find(|&c| !ist_unsichtbar(c))
        .is_none_or(|c| !ist_wortzeichen(c))
}

/// Satzgrenzen nach UAX #29 mit Anpassungen.
///
/// UAX #29 setzt nach jedem Punkt mit folgendem Leerzeichen und Grossbuchstaben eine
/// Grenze, also auch in «Art. 5», «Dr. Muster» oder «am 5. März». UAX #29 sieht für
/// solche Fälle ausdrücklich Anpassungen vor (Abschnitt «Tailoring»). promptscan
/// verbindet zwei Abschnitte wieder, wenn der erste
///
/// 1. auf eine Abkürzung aus `lexika/abkuerzungen.toml` endet (Regel `satzgrenze`;
///    jede Abkürzung mit Quelle; ein geschütztes Leerzeichen zählt wie ein gewöhnliches,
///    eine klein geschriebene Abkürzung gilt am Abschnittsanfang auch gross: «Vgl.»), oder
/// 2. auf eine ein- oder zweistellige Zahl mit Punkt endet (Ordinalzahl wie «5.» in
///    «am 5. März»), ausser die Zahl folgt auf eine solche Abkürzung oder eine römische
///    Zahl: «Art. 12.», «S. 24.» und «BGE 140 III 86.» beenden den Satz,
///
/// und kein harter Zeilenumbruch dazwischen liegt. Ein Zeilenumbruch ist hart, ausser er
/// ist weich nach [`weiche_umbrueche_bestimmen`]; ein weicher Umbruch zählt wie ein
/// Leerzeichen. Ob das Satzfenster überhaupt das richtige Fenster ist, wird gemessen
/// (SPEC 7.3).
///
/// Die Laufzeit ist linear in der Textlänge: Am Ende jedes Abschnitts wird nur sein Ende
/// geprüft, nicht der ganze, schon verbundene Abschnitt.
pub fn saetze_bestimmen(s: &str) -> Vec<Range<usize>> {
    saetze_mit_umbruechen(s, &weiche_umbrueche_bestimmen(s))
}

/// Sätze, wenn die weichen Umbrüche `weich` schon bestimmt sind.
fn saetze_mit_umbruechen(s: &str, weich: &[usize]) -> Vec<Range<usize>> {
    let verbunden = verbinden(s, weich);
    saetze_roh(verbunden.as_deref().unwrap_or(s), &ABKUERZUNGEN).0
}

/// Der Text mit jedem weichen Umbruch `weich` als Leerzeichen, `None` ohne weichen
/// Umbruch. Ein weicher Umbruch zählt so wie ein Leerzeichen; beide sind ein Byte, also
/// bleiben alle Versätze gleich.
fn verbinden(s: &str, weich: &[usize]) -> Option<String> {
    if weich.is_empty() {
        return None;
    }
    let mut bytes = s.as_bytes().to_vec();
    for &p in weich {
        bytes[p] = b' ';
    }
    Some(String::from_utf8(bytes).expect("ein ASCII-Byte durch ein anderes ersetzt"))
}

/// Sätze nach UAX #29 mit den Abkürzungen und Ordinalzahlen aus [`saetze_bestimmen`];
/// jedes `\n` ist eine Grenze. Dazu: Endet der letzte Abschnitt ohne Satzende (auf eine
/// Abkürzung oder Ordinalzahl, die verbindet)?
fn saetze_roh(s: &str, abkuerzungen: &Abkuerzungen) -> (Vec<Range<usize>>, bool) {
    let mut ergebnis: Vec<Range<usize>> = Vec::new();
    // Das erste Wortzeichen des aktuellen Abschnitts, für die Abkürzungen, die nur am
    // Abschnittsanfang gelten. Es wird beim Verbinden mitgeführt statt neu gesucht.
    let mut wortanfang: Option<usize> = None;
    let mut verbinden = false;
    for (start, teil) in s.split_sentence_bound_indices() {
        let bereich = start..start + teil.len();
        match ergebnis.last_mut() {
            Some(letzter) if verbinden => letzter.end = bereich.end,
            _ => {
                ergebnis.push(bereich);
                wortanfang = None;
            }
        }
        if wortanfang.is_none() {
            wortanfang = teil.find(ist_wortzeichen).map(|i| start + i);
        }
        let aktuell = ergebnis.last().cloned().unwrap_or(0..0);
        verbinden = endet_ohne_satzende(s, aktuell, wortanfang, abkuerzungen);
    }
    if ergebnis.is_empty() {
        ergebnis.push(0..s.len());
    }
    (ergebnis, verbinden)
}

/// Endet der Abschnitt `s[abschnitt]` ohne Satzende, auf eine Abkürzung oder Ordinalzahl?
/// Geprüft wird nur sein Ende; `wortanfang` ist das erste Wortzeichen des Abschnitts.
fn endet_ohne_satzende(
    s: &str,
    abschnitt: Range<usize>,
    wortanfang: Option<usize>,
    abkuerzungen: &Abkuerzungen,
) -> bool {
    let basis = abschnitt.start;
    let abschnitt = &s[abschnitt];
    if abschnitt.trim_end_matches([' ', '\t']).ends_with('\n') {
        return false;
    }
    let kern = abschnitt.trim_end_matches(|c: char| c.is_whitespace() || ist_unsichtbar(c));
    let Some(ohne_punkt) = kern.strip_suffix('.') else {
        return false;
    };
    let am_anfang = |i: usize| wortanfang == Some(basis + i);
    // Ordinalzahl: ein oder zwei Ziffern, davor kein Wortzeichen. Keine Ordinalzahl ist
    // eine Nummer nach einer Abkürzung oder einer römischen Zahl: «Art. 12.», «S. 24.»,
    // «BGE 140 III 86.» beenden den Satz.
    let ziffern = ohne_punkt
        .bytes()
        .rev()
        .take(3)
        .take_while(u8::is_ascii_digit)
        .count();
    if (1..=2).contains(&ziffern) {
        let davor = &ohne_punkt[..ohne_punkt.len() - ziffern];
        let frei = davor
            .chars()
            .next_back()
            .is_none_or(|c| !ist_wortzeichen(c) && c != '.');
        let wort_davor = davor.trim_end();
        let nummer = endet_auf_abkuerzung(wort_davor, abkuerzungen, &am_anfang)
            || endet_auf_roemischer_zahl(wort_davor);
        if frei && !nummer {
            return true;
        }
    }
    endet_auf_abkuerzung(kern, abkuerzungen, &am_anfang)
}

/// Endet `t` auf eine Abkürzung? Die Formen aus `ueberall` brauchen davor eine Wortgrenze,
/// die aus `am_anfang` müssen am Abschnittsanfang stehen (`am_anfang(i)` für den Versatz
/// `i` in `t`).
fn endet_auf_abkuerzung(
    t: &str,
    abkuerzungen: &Abkuerzungen,
    am_anfang: &dyn Fn(usize) -> bool,
) -> bool {
    abkuerzungen
        .ueberall
        .iter()
        .any(|abk| endet_mit(t, abk).is_some_and(|i| links_frei(t, i)))
        || abkuerzungen
            .am_anfang
            .iter()
            .any(|abk| endet_mit(t, abk).is_some_and(am_anfang))
}

/// Endet `t` auf `muster`? Dann der Versatz in `t`, an dem es beginnt. Geschützte
/// Leerzeichen in `t` zählen wie gewöhnliche: Word und PDF setzen «z. B.» mit U+00A0 oder
/// U+202F. Verglichen wird von hinten, Zeichen für Zeichen, ohne Kopie.
fn endet_mit(t: &str, muster: &str) -> Option<usize> {
    let mut rest = t;
    for m in muster.chars().rev() {
        let c = rest.chars().next_back()?;
        let gelesen = if matches!(c, '\u{A0}' | '\u{202F}') {
            ' '
        } else {
            c
        };
        if gelesen != m {
            return None;
        }
        rest = &rest[..rest.len() - c.len_utf8()];
    }
    Some(rest.len())
}

/// Endet der Text auf eine römische Zahl als eigenes Wort («BGE 140 III»)?
fn endet_auf_roemischer_zahl(s: &str) -> bool {
    let ohne = s.trim_end_matches(['I', 'V', 'X', 'L', 'C', 'D', 'M']);
    ohne.len() < s.len() && links_frei(s, ohne.len())
}

/// Die weichen Zeilenumbrüche in `s` als Byte-Versätze der `\n`, aufsteigend.
///
/// Ein einzelner Zeilenumbruch mitten in einem Absatz ist weich, also keine Satzgrenze,
/// wenn alle drei Bedingungen gelten:
///
/// 1. Die Zeile danach ist nicht leer, die Zeile davor auch nicht. Eine Leerzeile trennt
///    immer.
/// 2. Die Zeile davor endet nicht auf ein Satzzeichen `.` `!` `?` `:`. Schliessende
///    Anführungszeichen und Klammern nach dem Satzzeichen zählen nicht. Ein Punkt zählt
///    nicht, wenn er wie in [`saetze_bestimmen`] zu einer Abkürzung oder Ordinalzahl
///    gehört, die verbindet («gemäss Art.⏎5 DSG», «am 5.⏎März»).
/// 3. Keine der beiden Zeilen sieht aus wie ein Listenpunkt: am Zeilenanfang `-`, `*`,
///    `•`, `–`, `—`, `◦`, `▪` oder `‣`, oder eine Nummer wie `1.`, `1)`, `(1)`, `a)`,
///    `a.`, `A.` oder `IV.`, jeweils mit Leerraum dahinter.
///
/// Über einen weichen Umbruch hinweg treffen auch Begriffe aus mehreren Wörtern
/// («psychischen⏎Störung», siehe `lexikon::falten_mit`).
///
/// Grenze: Adressblöcke, Unterschriften und Überschriften ohne Satzzeichen werden mit der
/// nächsten Zeile zu einem Satz («INTERN⏎Unsere Mitarbeiterin …»,
/// «Erika Beispiel⏎Musterstrasse 1»). Umgekehrt sieht eine Zeile, die mit einem Datum wie
/// «12. März» beginnt, aus wie ein Listenpunkt: «geboren am⏎12. März 1985» bleibt getrennt.
pub fn weiche_umbrueche_bestimmen(s: &str) -> Vec<usize> {
    let abkuerzungen = &*ABKUERZUNGEN;
    let mut zeilen = Vec::new();
    let mut anfang = 0;
    for (i, _) in s.match_indices('\n') {
        zeilen.push(anfang..i);
        anfang = i + 1;
    }
    zeilen.push(anfang..s.len());
    zeilen
        .windows(2)
        .filter(|paar| {
            let (vorher, nachher) = (&s[paar[0].clone()], &s[paar[1].clone()]);
            !ist_leerzeile(vorher)
                && !ist_leerzeile(nachher)
                && !ist_listenpunkt(vorher)
                && !ist_listenpunkt(nachher)
                && !endet_mit_satzzeichen(vorher, abkuerzungen)
        })
        .map(|paar| paar[0].end)
        .collect()
}

/// Schliessende Anführungszeichen und Klammern. Sie stehen nach dem Satzzeichen.
const SCHLIESSEND: [char; 13] = [
    ')', ']', '}', '"', '\'', '»', '«', '\u{201D}', '\u{201C}', '\u{2019}', '\u{2018}', '›', '‹',
];

fn ist_leerzeile(zeile: &str) -> bool {
    zeile
        .chars()
        .all(|c| c.is_whitespace() || ist_unsichtbar(c))
}

/// Endet die Zeile auf ein Satzzeichen (Bedingung 2 in [`weiche_umbrueche_bestimmen`])?
fn endet_mit_satzzeichen(zeile: &str, abkuerzungen: &Abkuerzungen) -> bool {
    let kern = zeile.trim_end_matches(|c: char| {
        c.is_whitespace() || ist_unsichtbar(c) || SCHLIESSEND.contains(&c)
    });
    match kern.chars().next_back() {
        Some('!' | '?' | ':') => true,
        // Wie die Zeile für sich in Sätze zerfiele: verbindet ihr letzter Abschnitt?
        Some('.') => !saetze_roh(zeile, abkuerzungen).1,
        _ => false,
    }
}

/// Sieht die Zeile aus wie ein Listenpunkt (Bedingung 3 in [`weiche_umbrueche_bestimmen`])?
fn ist_listenpunkt(zeile: &str) -> bool {
    let z = zeile.trim_start();
    let leer_ab = |i: usize| z[i..].chars().next().is_none_or(char::is_whitespace);
    match z.chars().next() {
        Some(c @ ('-' | '*' | '•' | '–' | '—' | '◦' | '▪' | '‣')) => {
            return leer_ab(c.len_utf8());
        }
        Some(_) => {}
        None => return false,
    }
    let klammer = z.starts_with('(');
    let rest = if klammer { &z[1..] } else { z };
    let laenge = rest.bytes().take_while(u8::is_ascii_alphanumeric).count();
    let nummer = &rest[..laenge];
    let schluss = rest[laenge..].chars().next();
    let ist_nummer = match laenge {
        0 => false,
        1 => true,
        _ if nummer.bytes().all(|b| b.is_ascii_digit()) => laenge <= 3,
        _ => {
            laenge <= 4
                && (nummer.bytes().all(|b| matches!(b, b'i' | b'v' | b'x'))
                    || nummer.bytes().all(|b| matches!(b, b'I' | b'V' | b'X')))
        }
    };
    let schluss_passt = match schluss {
        Some(')') => true,
        Some('.') => !klammer,
        _ => false,
    };
    ist_nummer && schluss_passt && leer_ab(z.len() - rest.len() + laenge + 1)
}

/// Die Abkürzungen aus `lexika/abkuerzungen.toml` (Regel `satzgrenze`).
struct Abkuerzungen {
    /// Alle Formen, wie sie in der Liste stehen; sie gelten an jeder Stelle.
    ueberall: Vec<String>,
    /// Klein geschriebene Formen mit grossem Anfangsbuchstaben. Sie gelten nur, wenn sie
    /// den Abschnitt beginnen, denn so stehen sie am Satzanfang («Vgl. Art. 5 DSG»).
    /// Mitten im Satz wäre «Z.» oder «N.» eher ein Name als eine Abkürzung.
    am_anfang: Vec<String>,
}

/// Einmal gelesen, dann behalten.
static ABKUERZUNGEN: LazyLock<Abkuerzungen> = LazyLock::new(abkuerzungen);

fn abkuerzungen() -> Abkuerzungen {
    let ueberall: Vec<String> = match crate::lexikon::lexikon_falls("abkuerzungen.toml") {
        Some(lex) => lex
            .eintraege
            .iter()
            .filter(|e| e.regel == "satzgrenze")
            .flat_map(|e| std::iter::once(e.begriff.clone()).chain(e.formen.iter().cloned()))
            .collect(),
        None => Vec::new(),
    };
    let am_anfang: Vec<String> = ueberall
        .iter()
        .filter(|a| a.starts_with(|c: char| c.is_lowercase()))
        .map(|a| {
            let mut z = a.chars();
            z.next()
                .map(|c| c.to_uppercase().chain(z).collect())
                .unwrap_or_default()
        })
        .filter(|g: &String| !ueberall.contains(g))
        .collect();
    Abkuerzungen {
        ueberall,
        am_anfang,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn crlf_und_nfc() {
        let t = Text::neu("a\r\nb\u{0308}c");
        assert_eq!(t.als_str(), "a\nb\u{0308}c".nfc().collect::<String>());
        assert_eq!(t.zeilen_anzahl(), 2);
        assert_eq!(t.zeile(2), "b\u{0308}c".nfc().collect::<String>());
    }

    #[test]
    fn zerlegtes_umlaut_wird_eins() {
        let t = Text::neu("Zu\u{0308}rich");
        assert_eq!(t.als_str(), "Z\u{00FC}rich");
    }

    #[test]
    fn position_in_zeichen() {
        let t = Text::neu("Grüezi\nUnser Mitarbeiter Hans");
        let byte = t.als_str().find("Hans").unwrap();
        assert_eq!(t.position(byte), (2, 19));
        assert_eq!(t.position(0), (1, 1));
    }

    #[test]
    fn zeichenversatz_wie_gezaehlt() {
        // Mehrbyte-Zeichen (2, 3 und 4 Bytes) über die Grenzen der Stützstellen hinweg.
        let roh: String = (0..700)
            .map(|i| match i % 5 {
                0 => 'ä',
                1 => '€',
                2 => '𝄞',
                3 => '\n',
                _ => 'a',
            })
            .collect();
        let t = Text::neu(&roh);
        let s = t.als_str();
        let grenzen = s
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(s.len()));
        for byte in grenzen {
            assert_eq!(
                t.zeichenversatz(byte),
                s[..byte].chars().count(),
                "Byte {byte}"
            );
            let (zeile, spalte) = t.position(byte);
            let anfang = t.zeilenbereich(zeile).start;
            assert_eq!(spalte, s[anfang..byte].chars().count() + 1, "Byte {byte}");
        }
        assert_eq!(Text::neu("").zeichenversatz(0), 0);
    }

    #[test]
    fn zeilenende_trennt_saetze() {
        // Bis zur Regel für weiche Umbrüche stand hier «Name AHV⏎Zweite Zeile» mit zwei
        // Sätzen. Diese Zeile endet ohne Satzzeichen, der Umbruch ist jetzt weich
        // (siehe `weicher_umbruch_verbindet`). Hart bleibt er nach einem Satzzeichen und
        // vor oder nach einer Leerzeile.
        for s in [
            "Name AHV.\nZweite Zeile",
            "Name AHV\n\nZweite Zeile",
            "Name AHV\n \nZweite Zeile",
        ] {
            assert!(getrennt(s), "{s:?}");
        }
        assert_eq!(Text::neu("Name AHV\nZweite Zeile").saetze().len(), 1);
    }

    /// Liegen Anfang und Ende in verschiedenen Sätzen, und ist kein Umbruch weich?
    fn getrennt(s: &str) -> bool {
        let t = Text::neu(s);
        !t.im_selben_satz(0, s.len() - 1) && t.weiche_umbrueche().is_empty()
    }

    #[test]
    fn weicher_umbruch_verbindet() {
        for s in [
            "Ich bin\nAnwältin und brauche Hilfe.",
            "Je suis\navocate et j'ai besoin d'aide.",
            "Sono\navvocata e ho bisogno di aiuto.",
            "Er leidet an einer psychischen\nStörung.",
            "Ich bin  \n  Anwältin.",
        ] {
            let t = Text::neu(s);
            assert_eq!(t.saetze().len(), 1, "{s:?}");
            let umbruch = t.als_str().find('\n').unwrap();
            assert!(t.weicher_umbruch(umbruch), "{s:?}");
            assert_eq!(t.weiche_umbrueche(), [umbruch]);
            assert!(!t.weicher_umbruch(umbruch - 1));
        }
        // CRLF wird zu `\n`; der Versatz gilt im normalisierten Text.
        let t = Text::neu("Ich bin\r\nAnwältin.");
        assert_eq!(t.saetze().len(), 1);
        assert!(t.weicher_umbruch(7));
    }

    #[test]
    fn harter_umbruch_nach_satzzeichen_und_leerzeile() {
        for s in [
            "Er ging.\nDann kam sie.",
            "Wirklich?\nJa.",
            "Achtung!\nDas gilt.",
            "Zu den Personalien:\nErika Beispiel",
            "Er sagte «Nein.»\nDann ging er.",
            "Er sagte (siehe oben.)\nDann ging er.",
            "Erster Absatz\n\nZweiter Absatz",
            "Erster Absatz\n\t\nZweiter Absatz",
            "Er ging.\ndann kam sie.",
        ] {
            assert!(getrennt(s), "{s:?}");
        }
    }

    #[test]
    fn abkuerzung_und_ordinalzahl_am_zeilenende_verbinden() {
        for s in [
            "Es gilt gemäss Art.\n5 DSG die Regel.",
            "Er kam am 5.\nMärz zurück.",
            "Vgl.\nArt. 5 DSG.",
            "Das betrifft z. B.\nden Mieter.",
            "Il s'applique selon l'art.\n5 LPD.",
            "Si applica secondo l'art.\n5 LPD.",
        ] {
            let t = Text::neu(s);
            assert_eq!(t.saetze().len(), 1, "{s:?}");
            assert_eq!(t.weiche_umbrueche().len(), 1, "{s:?}");
        }
        // Eine Nummer nach einer Abkürzung oder römischen Zahl beendet den Satz.
        for s in [
            "Siehe BGE 140 III 86.\nNächster Satz.",
            "Es gilt Art. 12.\nDann anderes.",
        ] {
            assert!(getrennt(s), "{s:?}");
        }
    }

    #[test]
    fn listenpunkte_bleiben_getrennt() {
        for s in [
            "Bitte prüfen\n- Erika Beispiel\n- Hans Muster",
            "* eins\n* zwei",
            "Einleitung\n• Punkt",
            "Einleitung\n– Gedankenstrich",
            "1. Punkt eins\n2. Punkt zwei",
            "1) Punkt eins\n2) Punkt zwei",
            "(1) Absatz eins\n(2) Absatz zwei",
            "a) erstens\nb) zweitens",
            "a. erstens\nb. zweitens",
            "I. Sachverhalt\nII. Erwägungen",
            "iv) viertens\nv) fünftens",
        ] {
            assert!(getrennt(s), "{s:?}");
        }
        // Dokumentierte Grenze: ein Datum am Zeilenanfang sieht aus wie ein Listenpunkt.
        for s in [
            "Sie wurde am\n12. März 1985 geboren.",
            "Erika Beispiel, geboren am\n5. Mai 1990",
        ] {
            assert!(getrennt(s), "{s:?}");
        }
        // Gegenbeispiele: kein Listenpunkt, der Umbruch bleibt weich.
        for s in [
            "Der Betrag von\n1.5 Millionen wurde bezahlt.",
            "Bitte\n*unbedingt* lesen.",
            "Der Wert ist\n-5 Grad.",
            "Im Jahr\n2024 kam er.",
            "Die Variante\n(siehe oben) gilt.",
        ] {
            let t = Text::neu(s);
            assert_eq!(t.saetze().len(), 1, "{s:?}");
            assert_eq!(t.weiche_umbrueche().len(), 1, "{s:?}");
        }
    }

    #[test]
    fn grenze_ueberschrift_adresse_unterschrift_werden_verbunden() {
        // Dokumentierte Grenze: ohne Satzzeichen und Listenpunkt ist der Umbruch weich.
        for s in [
            "INTERN\nUnsere Mitarbeiterin ist krank.",
            "Erika Beispiel\nMusterstrasse 1\n8000 Zürich",
            "Freundliche Grüsse\nErika Beispiel",
        ] {
            assert_eq!(Text::neu(s).saetze().len(), 1, "{s:?}");
        }
    }

    #[test]
    fn saetze_lueckenlos_mit_weichen_umbruechen() {
        let s = "Ich bin\nAnwältin. Mein Klient\nist krank.\n\n- Punkt\nWeiter geht es\nhier.";
        let t = Text::neu(s);
        let mut ende = 0;
        for satz in t.saetze() {
            assert_eq!(satz.start, ende);
            ende = satz.end;
        }
        assert_eq!(ende, s.len());
    }

    /// Die Belege aus der Prüfung der weichen Umbrüche: Nach einer Anrede, einem Betreff,
    /// einem Namen oder einer Überschrift ohne Satzzeichen steht das erste Wort der
    /// nächsten Zeile nicht mehr am Anfang seines Satzes, aber am Anfang seines Satzstücks.
    #[test]
    fn satzanfang_nach_weichem_umbruch() {
        for (s, wort) in [
            (
                "Sehr geehrte Damen und Herren,\nAls Anwältin vertrete ich Frau Erika Beispiel.",
                "Als",
            ),
            (
                "Sehr geehrte Damen und Herren,\r\nAls Anwältin vertrete ich Frau Erika Beispiel.",
                "Als",
            ),
            (
                "Guten Tag\nAls Arzt frage ich mich, ob ich die Diagnose weitergeben darf.",
                "Als",
            ),
            (
                "Betreff Erbstreit Beispiel\nAls Anwältin habe ich eine Frage.",
                "Als",
            ),
            (
                "Bonjour,\nEn tant qu'avocate, je représente Mme Marie Exemple.",
                "En",
            ),
            (
                "Objet succession Exemple\nComme avocate, je représente Mme Marie Exemple.",
                "Comme",
            ),
            (
                "Madame, Monsieur\nNous, en tant qu'avocats, représentons M. Jean Exemple.",
                "Nous",
            ),
            (
                "Buongiorno\nSono avvocata e rappresento la signora Giulia Esempio.",
                "Sono",
            ),
            (
                "Oggetto successione Esempio\nIn qualità di avvocata, vorrei sapere come procedere.",
                "In",
            ),
            (
                "Oggetto paziente\nCome medico mi chiedo se posso trasmettere i dati.",
                "Come",
            ),
            (
                "Erika Beispiel\nGeboren am 12. März 1985 in Zürich",
                "Geboren",
            ),
            (
                "Personalien\nGeboren am 12. März 1985, leidet an einer psychischen Störung.",
                "Geboren",
            ),
            (
                "Dossier médical\nNée le 12 mars 1985 à Genève, la patiente souffre d'une dépression.",
                "Née",
            ),
            (
                "Cartella clinica\nNata il 12 marzo 1985 a Lugano, la paziente soffre di disturbi.",
                "Nata",
            ),
            (
                "Bonjour\nCite les arrêts du Tribunal fédéral sur l'art. 336 CO.",
                "Cite",
            ),
            (
                "Hallo zusammen\nListe die einschlägigen BGE zur Kündigung.",
                "Liste",
            ),
            ("Guten Tag\n  «Als Anwältin frage ich mich das.»", "Als"),
        ] {
            let t = Text::neu(s);
            let wort = t.als_str().find(wort).unwrap();
            assert_eq!(t.saetze().len(), 1, "{s:?}");
            let satz = t.satz_bei(wort);
            assert!(
                t.als_str()[satz.start..wort]
                    .chars()
                    .any(char::is_alphanumeric),
                "{s:?}: im Satz steht das Wort nicht am Anfang"
            );
            assert!(t.satzanfang_bei(wort), "{s:?}");
            let umbruch = t.weiche_umbrueche()[0];
            assert_eq!(t.satzstueck_bei(wort), umbruch + 1..t.len(), "{s:?}");
            assert!(t.satzanfang_bei(0), "{s:?}");
        }
        // Gegenbeispiele: Wörter mitten in ihrer Zeile.
        for (s, wort) in [
            ("Guten Tag\nIch bin als Anwältin tätig.", "als"),
            (
                "Sehr geehrte Damen\nund Herren, als Anwältin frage ich.",
                "als",
            ),
            ("Sie arbeitet als\nAnwältin, habe ich gehört.", "als"),
            ("Erika Beispiel, geboren\nam 12. März 1985", "geboren"),
            ("Guten Tag\nBitte cite les arrêts.", "cite"),
        ] {
            let t = Text::neu(s);
            let wort = t.als_str().find(wort).unwrap();
            assert!(!t.satzanfang_bei(wort), "{s:?}");
        }
    }

    /// `satzanfang_bei` gibt für jedes Wort dasselbe wie die Satzanfänge, als noch jedes
    /// `\n` den Satz beendete, mit einer Ausnahme: Beginnt die Zeile nach einem weichen
    /// Umbruch klein oder mit einer Ziffer, setzt sie den Satz fort, und ihr erstes Wort steht
    /// nicht mehr am Satzanfang («… Sie⏎ist krank.», [`Text::stueckgrenze`], Befund C).
    #[test]
    fn satzanfang_wie_vor_den_weichen_umbruechen() {
        for s in [
            "Sehr geehrte Damen und Herren,\nAls Anwältin vertrete ich Frau Beispiel. Sie\nist krank.",
            "Guten Tag\nIch bin Ärztin. Als Ärztin frage ich mich\nob ich darf.",
            "Es gilt gemäss Art.\n5 DSG die Regel. Vgl.\nArt. 6 DSG.",
            "Er kam am 5.\nMärz zurück und\n«Als» stand da.",
            "Bitte prüfen\n- Erika Beispiel\n- Hans Muster\n\nDanke\nund Gruss",
            "Objet\nComme avocate, je\nreprésente Mme Exemple. Nous, en tant\nqu'avocats, écrivons.",
            "Oggetto\n  Sono avvocata e\n\tCome medico mi chiedo\r\nse posso.",
            "Wirklich?\nJa. Das\nstimmt: Nein\n1. Punkt\nzwei",
        ] {
            let t = Text::neu(s);
            let s = t.als_str();
            let zeilenweise = saetze_roh(s, &ABKUERZUNGEN).0;
            let woerter = s.char_indices().filter(|&(i, c)| {
                ist_wortzeichen(c)
                    && s[..i]
                        .chars()
                        .next_back()
                        .is_none_or(|d| !ist_wortzeichen(d))
            });
            for (wort, _) in woerter {
                let satz = zeilenweise.iter().find(|b| b.contains(&wort)).unwrap();
                let bisher = !s[satz.start..wort].chars().any(ist_wortzeichen);
                let fortsetzung = t.weiche_umbrueche().iter().any(|&p| {
                    p < wort && !s[p + 1..wort].chars().any(ist_wortzeichen) && !t.stueckgrenze(p)
                });
                assert_eq!(
                    t.satzanfang_bei(wort),
                    bisher && !fortsetzung,
                    "{s:?} bei {wort}"
                );
            }
        }
    }

    /// Ein Satzstück endet am weichen Umbruch: Das Fragezeichen der zweiten Zeile gehört
    /// nicht zum Stück der ersten.
    #[test]
    fn satzstueck_endet_am_weichen_umbruch() {
        let t = Text::neu("Seit 2010 bin ich Ärztin\nDarf ich die Diagnose weitergeben?");
        let s = t.als_str();
        let (aerztin, frage) = (s.find("Ärztin").unwrap(), s.find('?').unwrap());
        assert!(t.im_selben_satz(aerztin, frage));
        assert_eq!(t.satzstueck_bei(aerztin), 0..s.find('\n').unwrap() + 1);
        assert!(!t.satzstueck_bei(aerztin).contains(&frage));
        assert!(t.satzstueck_bei(s.find("Darf").unwrap()).contains(&frage));
        // Steht der Treffer in derselben Zeile wie das Fragezeichen, ist es eine Frage.
        let t = Text::neu("Ab wann bin ich\nÄrztin?");
        let s = t.als_str();
        assert!(
            t.satzstueck_bei(s.find("Ärztin").unwrap())
                .contains(&s.find('?').unwrap())
        );
        // Lückenlos: Die Stücke eines Satzes reichen von Satzanfang bis Satzende, jedes
        // endet nach einem weichen Umbruch oder am Satzende.
        let t = Text::neu("Guten Tag\nmeine Mandantin\nist krank. Er\nauch.\n\nNeu\nda");
        for satz in t.saetze() {
            let mut b = satz.start;
            while b < satz.end {
                let stueck = t.satzstueck_bei(b);
                assert_eq!(stueck.start, b);
                assert!(stueck.end == satz.end || t.weicher_umbruch(stueck.end - 1));
                for i in stueck.clone() {
                    assert_eq!(t.satzstueck_bei(i), stueck);
                }
                b = stueck.end;
            }
        }
        // Ohne weichen Umbruch ist das Stück der Satz.
        let t = Text::neu("Erster Satz.\nZweiter Satz. Dritter.");
        for b in 0..t.len() {
            assert_eq!(t.satzstueck_bei(b), t.satz_bei(b));
        }
    }

    /// Innerhalb eines Satzes ist jedes `\n` vor dem Leerraum am Satzende weich.
    #[test]
    fn zeilenumbruch_im_satz_ist_weich() {
        let s = "Guten Tag\nAls Anwältin\r\nfrage ich.\nDann\n\n- Punkt\neins\n1. zwei\nGeb.\nHans";
        let t = Text::neu(s);
        let mut weich = 0;
        for satz in t.saetze() {
            let inhalt = t.ausschnitt(satz.clone()).trim_end();
            for (i, _) in inhalt.match_indices('\n') {
                assert!(t.weicher_umbruch(satz.start + i), "{inhalt:?}");
                weich += 1;
            }
        }
        assert_eq!(weich, t.weiche_umbrueche().len());
    }

    #[test]
    fn ordinalzahl_trennt_nicht() {
        let t = Text::neu("Er kam am 5. März zurück. Dann ging er.");
        let erster = t.saetze()[0].clone();
        assert!(t.ausschnitt(erster).contains("zurück."));
    }

    #[test]
    fn nummer_nach_abkuerzung_beendet_den_satz() {
        for s in [
            "Siehe ATF 140 III 86. Nächster Satz.",
            "Siehe BGE 140 III 86. Nächster Satz.",
            "Es gilt Art. 12. Dann anderes.",
            "Vgl. dort S. 24. Dann anderes.",
            "Massgebend ist Rz. 7. Dann anderes.",
        ] {
            assert_eq!(Text::neu(s).saetze().len(), 2, "{s}");
        }
        // Die Ordinalzahl verbindet weiter.
        assert_eq!(
            Text::neu("Er kam am 5. März zurück. Dann ging er.")
                .saetze()
                .len(),
            2
        );
        assert_eq!(
            Text::neu("Es gilt Art. 12 Abs. 2. Dann anderes.")
                .saetze()
                .len(),
            2
        );
    }

    #[test]
    fn geschuetztes_leerzeichen_in_abkuerzung_trennt_nicht() {
        for leer in ['\u{A0}', '\u{202F}'] {
            let s = format!("Er nannte z.{leer}B. Herrn Muster. Dann anderes.");
            assert_eq!(Text::neu(&s).saetze().len(), 2, "{s:?}");
            let s = format!("Es gilt Art.{leer}5 DSG. Dann anderes.");
            assert_eq!(Text::neu(&s).saetze().len(), 2, "{s:?}");
        }
    }

    #[test]
    fn klein_geschriebene_abkuerzung_am_satzanfang_trennt_nicht() {
        assert_eq!(
            Text::neu("Vgl. Art. 5 DSG. Dann anderes.").saetze().len(),
            2
        );
        assert_eq!(
            Text::neu("Z. B. Herr Muster. Dann anderes.").saetze().len(),
            2
        );
        assert_eq!(
            Text::neu("Das gilt. Vgl. Art. 5 DSG. Dann anderes.")
                .saetze()
                .len(),
            3
        );
        // Mitten im Satz ist ein grosses «Z.» keine Abkürzung.
        assert_eq!(
            Text::neu("Er wohnt in Haus Z. Dann ging er.")
                .saetze()
                .len(),
            2
        );
    }

    #[test]
    fn satz_bei_findet_bereich() {
        let t = Text::neu("Erster Satz. Zweiter Satz.");
        let b = t.als_str().find("Zweiter").unwrap();
        assert!(t.ausschnitt(t.satz_bei(b)).starts_with("Zweiter"));
        assert!(!t.im_selben_satz(0, b));
    }

    /// Kleinste Laufzeit aus fünf Läufen, damit ein kurzer Stau der Maschine nicht zählt.
    fn kleinste_zeit(f: impl Fn()) -> Duration {
        (0..5)
            .map(|_| {
                let t0 = Instant::now();
                f();
                t0.elapsed()
            })
            .min()
            .unwrap()
    }

    #[test]
    fn satzgrenzen_in_linearer_zeit() {
        // Achtmal so viel Text darf höchstens zwanzigmal so lange brauchen; quadratisch wären
        // es 64-mal. Vor der Änderung brauchte «Art. » 1000-mal wiederholt 0,36 s und
        // 8000-mal 10,7 s, also rund 30-mal so lange (Debug-Bau).
        for stueck in ["Art. ", "die ", "Art.\n", "am 5.\nMärz ", "z.\u{A0}B. "] {
            let zeit = |n: usize| {
                let s = stueck.repeat(n);
                kleinste_zeit(|| {
                    std::hint::black_box(Text::neu(&s));
                })
            };
            zeit(500);
            let klein = zeit(1_000);
            let gross = zeit(8_000);
            assert!(
                gross < klein * 20 + Duration::from_millis(20),
                "{stueck:?}: {klein:?} für 1000, {gross:?} für 8000"
            );
        }
    }

    #[test]
    fn unsichtbare_wie_default_ignorable_code_point() {
        let alle: String = (0..=0x10FFFF_u32).filter_map(char::from_u32).collect();
        let eigenschaft = regex::Regex::new(r"\p{Default_Ignorable_Code_Point}").unwrap();
        let laut_regex: Vec<char> = eigenschaft
            .find_iter(&alle)
            .flat_map(|m| m.as_str().chars())
            .collect();
        let laut_tabelle: Vec<char> = alle.chars().filter(|&c| ist_unsichtbar(c)).collect();
        assert_eq!(laut_tabelle, laut_regex);
        for c in [
            '\u{AD}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}',
        ] {
            assert!(ist_unsichtbar(c), "{c:?}");
        }
        for c in [' ', '\u{A0}', '\u{202F}', '-', '\u{2011}', 'a'] {
            assert!(!ist_unsichtbar(c), "{c:?}");
        }
    }

    #[test]
    fn wortgrenze_ueberspringt_unsichtbare() {
        let s = "Mus\u{AD}ter und \u{200B}Hans";
        let ter = s.find("ter").unwrap();
        assert!(!links_frei(s, ter));
        assert!(!rechts_frei(s, ter - 2));
        let hans = s.find("Hans").unwrap();
        assert!(links_frei(s, hans));
        assert!(rechts_frei(s, s.len()));
        assert!(links_frei(s, 0));
    }

    #[test]
    fn unsichtbare_formatzeichen_zaehlen() {
        let zahl = |s: &str| Text::neu(s).unsichtbare_formatzeichen().len();
        assert_eq!(zahl("Hans Muster"), 0);
        assert_eq!(
            Text::neu("Mus\u{AD}ter 756\u{200B}.1234\u{2060}.5678.97\u{FEFF}")
                .unsichtbare_formatzeichen(),
            [
                (3, '\u{AD}'),
                (12, '\u{200B}'),
                (20, '\u{2060}'),
                (31, '\u{FEFF}')
            ]
        );
        // Schreibrichtung und Tags (unsichtbar eingeschleuster Text) zählen.
        assert_eq!(zahl("a\u{202E}b\u{E0041}\u{E0042}"), 3);
        // Emoji-Folgen nicht: Variantenwähler, Verbinder nach Symbol, Flaggen-Tags.
        let emoji = "\u{2764}\u{FE0F} \u{1F468}\u{200D}\u{1F469} \u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}";
        assert_eq!(zahl(emoji), 0);
        // Der Verbinder zwischen Buchstaben zählt.
        assert_eq!(zahl("Mus\u{200D}ter"), 1);
    }

    #[test]
    fn unsichtbare_rueckweg() {
        let o = OhneUnsichtbare::neu("a\u{200B}b\u{AD}\u{AD}c");
        assert_eq!(o.text, "abc");
        // «b» beginnt nach dem U+200B, «a» endet davor.
        assert_eq!((o.anfang(1), o.ende(1)), (4, 1));
        // «c» beginnt nach beiden U+00AD, «b» endet davor.
        assert_eq!((o.anfang(2), o.ende(2)), (9, 5));
        assert_eq!((o.anfang(0), o.ende(3)), (0, 10));
        let ohne = OhneUnsichtbare::neu("abc");
        assert!(matches!(ohne.text, Cow::Borrowed(_)));
        assert_eq!((ohne.anfang(2), ohne.ende(2)), (2, 2));
        // Ein Muster auf dem gekürzten Text, die Stelle im ganzen Text.
        let s = "Adresse: See\u{AD}strasse 12, 8002 Zürich";
        let o = OhneUnsichtbare::neu(s);
        let a = o.text.find("Seestrasse 12").unwrap();
        let stelle = o.anfang(a)..o.ende(a + "Seestrasse 12".len());
        assert_eq!(&s[stelle], "See\u{AD}strasse 12");
    }

    #[test]
    fn verbunden_liest_weiche_umbrueche_als_leerzeichen() {
        let s = "Die Klientin, geb.\n12.03.1985, wohnt in\nWinterthur.\n\nNeuer Absatz\n- Punkt";
        let t = Text::neu(s);
        let v = t.verbunden();
        assert_eq!(
            v,
            "Die Klientin, geb. 12.03.1985, wohnt in Winterthur.\n\nNeuer Absatz\n- Punkt"
        );
        assert_eq!(v.len(), t.len());
        for (i, (a, b)) in t.als_str().bytes().zip(v.bytes()).enumerate() {
            assert_eq!(a != b, t.weicher_umbruch(i), "Byte {i}");
        }
        // Der Trennstrich bleibt; ohne weichen Umbruch ist es derselbe Text.
        assert_eq!(
            Text::neu("arbeits-\nunfähig").verbunden(),
            "arbeits- unfähig"
        );
        let t = Text::neu("Erster Satz.\nZweiter.");
        assert!(std::ptr::eq(t.verbunden(), t.als_str()));
        // CRLF ist schon `\n`.
        assert_eq!(
            Text::neu("wohnhaft in\r\nBern").verbunden(),
            "wohnhaft in Bern"
        );
    }

    #[test]
    fn ersatzzeichen_zaehlen() {
        assert_eq!(Text::neu("Anw\u{FFFD}ltin \u{FFFD}").ersatzzeichen(), 2);
        assert_eq!(Text::neu("Anwältin").ersatzzeichen(), 0);
    }

    #[test]
    fn verdacht_auf_ascii_pipe() {
        for (s, wort) in [
            ("Ich bin Anw?ltin.", "Anw?ltin"),
            ("Je suis b?n?vole.", "b?n?vole"),
            ("Il signor M?ller ? arrivato.", "M?ller"),
            ("Herr \"M?ller?\", geboren", "M?ller?"),
            ("Wer? Was?\tAnw?ltin\u{1B}[0m", "Anw?ltin\u{1B}[0m"),
        ] {
            assert_eq!(Text::neu(s).ascii_pipe_wort(), Some(wort), "{s:?}");
        }
        for s in [
            "Ich bin Anwältin? Anw?ltin.",
            "Wer? Was? Wann?",
            "Siehe https://beispiel.ch/seite?id=5 und www.beispiel.ch?q",
            "Siehe beispiel.ch/a?b und a?b=c",
            "?abc abc? 5?5",
        ] {
            assert_eq!(Text::neu(s).ascii_pipe_wort(), None, "{s:?}");
        }
        assert_eq!(fragezeichen_im_wort("Anw?ltin"), Some(3));
        assert_eq!(fragezeichen_im_wort("?a?b"), Some(2));
        assert_eq!(fragezeichen_im_wort("a?"), None);
    }
}
