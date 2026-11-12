# Ausgabe von promptscan

Stand 07.11.2026 · JSON-Schema-Version 2 · umgesetzt in `src/ausgabe.rs`

Dieses Dokument beschreibt, was promptscan ausgibt: den Bericht als Text und als JSON,
`promptscan rules` und `promptscan explain`. Grundlage ist SPEC Abschnitt 3.

Zwei Tests halten das Dokument und das Programm beisammen:

- Die Modultests in `src/ausgabe.rs` lesen die vier Tabellen «JSON: …» unten und
  verlangen, dass die Ausgabe genau diese Felder hat, nicht mehr und nicht weniger.
- `tests/ausgabe.rs` vergleicht das Beispiel in Abschnitt 1 Zeichen für Zeichen mit der
  Ausgabe für denselben Prompt; ebenso die zwei Beispiele in SPEC 3.

Die Texte mit rechtlichem Gehalt stehen in `src/ausgabe.rs` beisammen.

## 1. Bericht als Text

Von oben nach unten, damit in fünf Sekunden Ergebnis, Handlung und Grund zu sehen sind:

1. **Kopfzeile:** `Lokal geprüft · Rolle: … · Ziel: …`. Die Rolle mit
   Herkunft: `Anwältin/Anwalt (--role)`, `(Konfiguration)`, `Anwältin/Anwalt, aus «…»`
   (aus dem Text erschlossen), `Beruf offen, aus «…»`, `keine (--role none)` oder
   `nicht angegeben`. Passt die Zeile nicht in 70 Spalten, beginnt `Ziel: …` eine
   zweite Zeile; läuft auch die erste über, wird das auslösende Wort nach 20 Zeichen
   mit `…` gekürzt. Ziel ist `Cloud` oder `lokales Modell`. Ob das Ziel aus `--target`
   oder aus der Konfigurationsdatei stammt, nennt die Kopfzeile nicht: Der
   Bericht trägt die Herkunft des Ziels nicht; `explain hinweis` warnt, dass
   `target = "local"` in der Datei für jeden Prompt gilt.
2. eine Leerzeile, dann die **Zeilenansicht** des Prompts mit den markierten Stellen;
3. die **Gruppen** nach Stufe: ROT, ORANGE, GELB, HINWEIS, innerhalb nach Position;
4. der **Ergebnisblock**: `Ergebnis` (darunter, falls nötig, die Meldungen zur
   Eingabe), `Klären`, `Beachten`, `Mehr`.

Beispiel ohne Farbe (`--no-color`), Prompt f04 aus SPEC 3, 80 Spalten:

<!-- beispiel-ohne-farbe -->
```text
Lokal geprüft · Rolle: nicht angegeben · Ziel: Cloud

   │ Unser Mitarbeiter Hans Muster, geb. 12.03.1985, Seestrasse 12, 8002 Zürich,
   │                   G^^^^^^^^^^       G^^^^^^^^^  G^^^^^^^^^^^^^^^^^^^^^^^^^
   │ AHV 756.1234.5678.97, ist seit März zu 50 %
   │     G^^^^^^^^^^^^^^^
   │ arbeitsunfähig (Diagnose F32.1).
   │ O^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   │ Formuliere eine Kündigung und nenne passende BGE zum Kündigungsschutz.
   │                               H^^^^^^^^^^^^^^^^^
     O ORANGE · G GELB · H HINWEIS

ORANGE    Besonders schützenswerte Personendaten, Art. 5 lit. c DSG
          Bekanntgabe an Dritte durch Private: Art. 30 Abs. 2 lit. c DSG
          Ziff. 2       arbeitsunfähig (Diagnose F32.1)

GELB      Personendaten, Art. 5 lit. a DSG
          Name          Hans Muster
          Geburtsdatum  12.03.1985
          Adresse       Seestrasse 12, 8002 Zürich
          AHV-Nummer    756.1234.5678.97 (Prüfziffer gültig)

HINWEIS   Rechtsfolge nicht beurteilt
          Fundstellen   «nenne passende BGE»
                        Sprachmodelle erfinden Urteile.

Ergebnis  ORANGE   Besonders schützenswerte Personendaten
          6 Funde: 1 orange, 4 gelb, 1 Hinweis
Klären    Braucht die KI diese Angaben? Nutzt der Anbieter sie nur in Ihrem
          Auftrag oder auch für eigene Zwecke, etwa zum Training (Art. 9 DSG)?
          Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)?
          Dann --role angeben (Rollen: promptscan explain rot).
          Jede Fundstelle in der Antwort an der Quelle prüfen.
          Gehen die Daten ins Ausland? Dann gelten Art. 16 f. DSG, Anhang 1 DSV.
Mehr      promptscan explain orange · promptscan explain art5c-2 ·
          promptscan explain fundstellen
```

### Zeilenansicht

- Jede Zeile beginnt mit dem Rand `│`. Hat der Prompt mehr als 10 Zeilen
  (`ausgabe::ZEILEN_OHNE_NUMMER`), steht davor die Zeilennummer, rechtsbündig.
- **Umbruch:** Promptzeilen brechen an Leerzeichen auf die Breite um und
  behalten den Rand; jede Teilzeile hat ihre eigene Markierungszeile. Eine markierte
  Stelle wird nicht getrennt, ausser sie passt allein nicht in eine Zeile.
- **Mit Farbe** trägt jede Stelle einen Textmarker in der Farbe ihrer Stufe (unten,
  «Farbe»). Überdecken sich Funde, trägt jedes Zeichen die höchste Stufe.
- **Ohne Farbe** folgt unter jeder Teilzeile mit Fund eine Markierungszeile: am Anfang
  jeder Stelle die Initiale der Stufe, dann `^` (R, O, G, H; englisch R, O, Y, N).
  Unter der Zeilenansicht steht einmal die Legende der vorkommenden Stufen, etwa
  `O ORANGE · G GELB · H HINWEIS`.
- **Lange Eingaben:** Bis 40 Zeilen (`ausgabe::ZEILEN_GANZ`) zeigt promptscan alle
  Zeilen. Hat der Prompt mehr, zeigt es nur die Zeilen mit Fund und für jede Lücke
  `… │ N Zeilen ohne Fund`. Ein Fund über mehrere Zeilen zeigt alle seine Zeilen.

### Gruppen

Je Stufe eine Gruppe: in den Spalten 1 bis 10 die Stufe (mit Farbe als Plakette), ab
Spalte 11 der Kopf mit den Normen, darunter je Art eine Reihe: ab Spalte 11 die Art in
Worten (Name, Geburtsdatum, Adresse, AHV-Nummer, IBAN, Telefon, E-Mail, Firma …), ab
Spalte 25 die Werte, mit ` · ` getrennt. Ist die Art länger als 13 Zeichen, steht sie
auf einer eigenen Zeile.

- Gleiche Werte stehen einmal, mit `(2×)`. Ein Zusatz, den alle Werte einer Reihe
  tragen, etwa `(Prüfziffer gültig)`, entfällt bei mehreren Werten.
- Wert und Zusatz bleiben auf einer Zeile (`078 765 43 21 (Bereich Mobilnetz)`),
  solange sie zusammen hineinpassen.
- Bei mehr als 10 Zeilen folgt jedem Wert `(Zeile N)`; das Paar bricht nicht um.
- Reicht eine Stelle über einen Zeilenumbruch, steht sie auf einer Zeile, der Umbruch
  als Leerzeichen (`Meine Mandantin`), auch in der Kopfzeile; nur über eine Leerzeile
  hinweg die erste Zeile und `…`.
- **ROT:** Kopf mit der Pflicht und ihren Normen, etwa
  `Berufsgeheimnis, Art. 321 Ziff. 1 StGB, Art. 13 Abs. 1 BGFA`; bei offenem Beruf
  `Beruf offen:` mit jeder Norm, die in Frage kommt, und dem Beruf in Klammern.
  Angaben nach Art. 5 lit. c DSG folgen unter einer eigenen Zeile
  `zugleich besonders schützenswert, Art. 5 lit. c Ziff. N DSG:`, nur bei einer
  natürlichen Person.
- **ORANGE:** `Besonders schützenswerte Personendaten, Art. 5 lit. c DSG`, darunter
  `Bekanntgabe an Dritte durch Private: Art. 30 Abs. 2 lit. c DSG`; die Art ist die
  Ziffer (`Ziff. 2`).
- **GELB:** `Personendaten, Art. 5 lit. a DSG`.
- **HINWEIS:** `Rechtsfolge nicht beurteilt`; die Werte in «…», darunter ab Spalte 25
  der Hinweis der Regel. Was unter `Klären` als Handlung steht, wiederholt die Gruppe
  nicht (bei `fundstellen` nur `Sprachmodelle erfinden Urteile.`). Angaben ohne
  Identifikator stehen unter
  `Personenbezug, aber kein direkter Identifikator im Prompt:`; Geburtsdatum und
  Wohnort mit ihrer Art, die Kategorien von Art. 5 lit. c DSG wie bei ORANGE als
  `Ziff. N` unter `Kategorie nach Art. 5 lit. c DSG:`.
- **`--target local`:** eine eigene HINWEIS-Gruppe
  `Läuft das Modell lokal: keine Bekanntgabe (Art. 5 lit. e DSG)`, bei
  Funden, die sonst ROT wären, dazu `kein Offenbaren (…)` mit den Normen der Rolle;
  die Reihen heissen `sonst ROT`, `sonst ORANGE`, `sonst GELB` (englisch `else …`).

Regelkennungen stehen nur in der Zeile `Mehr`.

### Ergebnisblock

- **Ergebnis:** die höchste Stufe und ihr Titel, dann die Zählung aller Funde, die
  Hinweise eingeschlossen: `6 Funde: 1 orange, 4 gelb, 1 Hinweis`.
  Haben alle Funde dieselbe Stufe und passt die Zeile in 70 Spalten, steht die Zahl
  in derselben Zeile (`GELB   Personendaten · 3 Funde`). Bei offenem Beruf trägt der
  Titel `(Beruf offen)`. Ohne Fund: `Keine Funde in den geprüften Mustern` (mit Farbe
  fett). Mit `--target local`:
  `Läuft das Modell lokal: keine Bekanntgabe`, mit `, kein Offenbaren`, wenn in der
  Cloud ein Fund ROT wäre, darunter `N Funde: N Hinweise; in der Cloud …`.
- **Eingabe:** die Meldungen zur Eingabe (unten), falls etwas auffällt; gleich unter
  dem Ergebnis, weil sie sagen, dass Funde fehlen können.
- **Klären:** die Frage zur höchsten Stufe, bei ROT je nach
  Geheimhaltungspflicht; ohne Rolle (weder `--role` noch Konfiguration noch aus dem
  Text) und mit einem Fund GELB oder ORANGE
  `Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)? Dann --role angeben`
  (nicht bei `role = "none"`); bei `--target local` die Frage nach dem lokalen
  Modell, die die Fragen der Stufen ersetzt; die Fragen der Hinweisregeln; zuletzt,
  nur bei Ziel Cloud und einem Fund über HINWEIS, die Frage nach dem Ausland (für
  `authority`, `court`, `criminal-authority` nur `Für Bundesorgane: Art. 16 f. DSG.`).
- **Beachten:** ohne Fund die blinden Flecken (`Ob der Zusammenhang
  eine Person erkennbar macht und ob der Text Geschäftsgeheimnisse enthält, prüft
  promptscan nicht.` und `Was geprüft wird: promptscan rules`); mit `authority`,
  `court` oder `criminal-authority` und einem Fund über HINWEIS das anwendbare
  Datenschutzrecht (`Für kantonale und kommunale Organe gilt
  kantonales Datenschutzrecht, für Bundesorgane das DSG. …`, Art. 2 DSG).
- **Mehr:** `promptscan explain <stufe>` und höchstens drei Regeln.

`ausgabe::bericht_kurz_text` gibt nur den Ergebnisblock aus (für `--quiet`).

### Breite

Alles wird auf `Darstellung::breite` umgebrochen, mindestens 60 Spalten; ist die Breite
unbekannt, setzt `main.rs` 80. Fortsetzungen stehen hängend unter dem Anfang ihres
Feldes. Normzitate, Ausschnitte in «…», Befehle (`promptscan explain …`) und
`--role …` bleiben zusammen, ausser sie passen allein nicht. `tests/ausgabe.rs` prüft
jeden Prüffall bei 80 und 60 Spalten, gezählt nach dem Breitenmodell unten.

### Breitenmodell

Damit die `^` genau unter der Stelle stehen, zählt promptscan Spalten im Terminal,
nicht Bytes. Die Regeln folgen Markus Kuhn, `wcwidth.c` (2007-05-26), mit den Daten
von Unicode 18.0.0; vier Abweichungen sind markiert.

- **Tabulator:** Leerzeichen bis zum nächsten Vielfachen von 8, gezählt ab dem
  Zeilenanfang des Prompts (Abweichung: Kuhn behandelt ihn als Steuerzeichen).
- **Steuerzeichen** (General_Category Cc, z. B. ESC): erscheinen als `�` (U+FFFD) und
  belegen eine Spalte (Abweichung: Kuhn gibt -1, für U+0000 aber 0). So kann ein
  eingefügter Text das Terminal nicht steuern.
- **Bidi-Steuerzeichen** U+061C, U+200E, U+200F, U+202A–U+202E, U+2066–U+2069:
  erscheinen als `�` und belegen eine Spalte (Abweichung: Kuhn gibt 0). Sie würden die
  Anzeige umordnen und könnten Text verdecken.
- **U+00AD** (weiches Trennzeichen): eine Spalte.
- **Marks** (General_Category M, nach `unicode-normalization` 0.1.26, Unicode 18.0.0):
  keine Spalte (Abweichung: Kuhn zählt nur Mn und Me als 0, Mc als 1; Mc-Zeichen kommen
  in Schriften Süd- und Südostasiens vor). Nach der Normalform C bleiben in deutschen,
  französischen und italienischen Texten fast keine Marks übrig.
- **Übrige Formatzeichen** (General_Category Cf ohne U+00AD und ohne die
  Bidi-Steuerzeichen, 19 Bereiche aus `DerivedGeneralCategory-18.0.0.txt`) und
  U+1160–U+11FF (Hangul Jamo, Vokale und Schlusskonsonanten): keine Spalte.
- **East_Asian_Width W oder F** (UAX #11): zwei Spalten. Die 126 Bereiche sind alle
  Zeilen von `EastAsianWidth-18.0.0.txt` mit dem Wert W oder F, aneinanderstossende
  Bereiche zusammengefasst. Dazu gehören CJK-Schriften, Vollbreitformen und die meisten
  Emoji.
- **Alle übrigen Zeichen:** eine Spalte.

Grenzen: Eine Emoji-Folge mit Verbinder (U+200D) zählt als Summe ihrer Teile, das
Terminal zeigt sie oft als ein Zeichen mit zwei Spalten. Zeichen mit East_Asian_Width A
(mehrdeutig) zählen eine Spalte, wie bei Kuhn; Terminals mit ostasiatischer Einstellung
zeigen sie breiter.

Herkunft der Daten (abgerufen am 06.11.2026 mit User-Agent «promptscan-recherche»):

| Datei | URL | SHA-256 |
| --- | --- | --- |
| EastAsianWidth-18.0.0.txt (Datum 2026-06-29) | <https://www.unicode.org/Public/UCD/latest/ucd/EastAsianWidth.txt> | a0cf29eacd00cfcaec4381c6b7c281685f18dbb4e7ff82b4076ccb342ca839aa |
| DerivedGeneralCategory-18.0.0.txt (Datum 2026-06-29) | <https://www.unicode.org/Public/UCD/latest/ucd/extracted/DerivedGeneralCategory.txt> | d6b151d2d40ee9b1876d26f417980f45ffae47b6055ccf7203cb31f07a030f94 |
| wcwidth.c (Markus Kuhn, 2007-05-26) | <https://www.cl.cam.ac.uk/~mgk25/ucs/wcwidth.c> | 6e661fc7f9e0c7a28905ba5fcb12f6b76dd881f948d4d65cad0e00547890613c |

Die Rohdateien liegen nicht im Repo. Aus `wcwidth.c` stammen nur die Regeln, kein Code.
Die Unicode-Daten stehen unter der Unicode License V3; deren Hinweis steht in
Abschnitt 7, wie die Lizenz es für abgeleitete Daten in der Dokumentation verlangt.

### Meldungen zur Eingabe

Fällt an der Eingabe selbst etwas auf, steht im Ergebnisblock direkt unter `Ergebnis`
je eine Meldung (`ausgabe::Eingabe`), sonst nichts. Sie haben kein eigenes Etikett,
weil sie selbst mit «Eingabe …» beginnen. Im JSON stehen dieselben Zeilen unter
`eingabe.meldungen`.

- **Unsichtbare Formatzeichen** (Unicode-Eigenschaft Default_Ignorable_Code_Point), etwa
  das weiche Trennzeichen U+00AD oder U+200B:
  `Eingabe enthält 3 unsichtbare Formatzeichen (U+00AD, U+200B)`, mit den ersten vier
  verschiedenen Codepunkten und danach `…`. Nicht gezählt werden Variantenwähler, der
  Verbinder U+200D nach einem Symbol und Tag-Zeichen nach der Flagge U+1F3F4: Sie gehören
  zu Emoji-Folgen. Wortlisten und die Erkenner der Klasse A lesen über diese Zeichen
  hinweg (SPEC 6).
- **Ersatzzeichen U+FFFD:** `Eingabe enthält 2 Ersatzzeichen U+FFFD: Beim Umwandeln der
  Kodierung gingen vermutlich Zeichen verloren; Funde können fehlen`.
- **Verdacht auf eine ASCII-Pipe:** Die Eingabe enthält kein Zeichen ausserhalb von
  ASCII, aber ein Wort mit `?` zwischen zwei Buchstaben, das nicht wie eine Webadresse
  aussieht (mit `://`, `/`, `=` oder `&`, oder am Anfang `www.`). Dann zwei Zeilen:
  `Eingabe nur in ASCII, aber mit «?» im Wort («Anw?ltin»): Vermutlich hat eine Pipe
  Umlaute und Akzente zu «?» gemacht; Funde können fehlen` und der Rat für Windows
  PowerShell 5.1 (unten). Das Wort zeigt höchstens 15 Zeichen vor und nach dem `?`,
  Steuerzeichen als `�`.

Die Meldungen zu U+FFFD und zur ASCII-Pipe deuten auf eine beschädigte Kodierung.
`main.rs` schreibt sie zusätzlich auf stderr, jede Zeile mit `promptscan: ` davor. Der
Exit-Code bleibt der der Funde.

### Kodierung der Eingabe

`main.rs` liest Dateien und stdin als UTF-8, mit oder ohne BOM, oder als UTF-16 mit BOM,
wie es Windows PowerShell 5.1 mit `>` schreibt. Mit Exit-Code 2 abgewiesen werden:

- UTF-32, erkannt am BOM (FF FE 00 00 oder 00 00 FE FF): `UTF-32 wird nicht gelesen;
  bitte als UTF-8 speichern`;
- jede Eingabe mit dem Zeichen NUL (U+0000). Ohne BOM ist das meist UTF-16 ohne BOM: In
  reinem ASCII-Text ist dann jedes zweite Byte 0, und das wäre gültiges UTF-8, in dem
  keine Regel mehr greift. Meldung: `enthält das Zeichen NUL (U+0000), vermutlich UTF-16
  ohne BOM; bitte als UTF-8 speichern`;
- was weder UTF-8 noch UTF-16 mit BOM ist, etwa Windows-1252: `weder UTF-8 noch UTF-16
  mit BOM; bitte als UTF-8 speichern`.

Mit `--lang en` erscheinen die Meldungen englisch.

**Windows PowerShell 5.1** schickt Text über eine Pipe vorgabemässig als ASCII an ein
Programm (`$OutputEncoding` ist US-ASCII) und macht jedes andere Zeichen zu `?`. Aus
«Ich bin Anwältin» wird «Ich bin Anw?ltin», und der Geheimnis-Kontext entfällt. Abhilfe:
in der Sitzung vorher

```powershell
$OutputEncoding = [Text.UTF8Encoding]::new($false)
```

setzen oder den Prompt als Datei übergeben (`promptscan prompt.txt`). Nachgeprüft mit
Windows PowerShell 5.1.26100: Ohne die Zeile kam «Anw?ltin» an, mit ihr «Anwältin».

### Sprache

`--lang en` stellt alle Bedienungstexte auf Englisch: Stufenwörter, Kopfzeile,
Gruppenköpfe, Arten, Ergebnisblock, mit den Begriffen der Fedlex-Übersetzung («duty of
confidentiality», «professional confidentiality», «official secrecy»). Normzitate bleiben
im amtlichen deutschen Zitat (`Art. 5 lit. a DSG`, auch `Art. 13 Abs. 2 ISG` im Zusatz
der Regel vermerk), Normtexte im amtlichen deutschen Wortlaut; promptscan übersetzt keine
Normen. Titel, Methode, Hinweis und Grenzen liefert jede Regel selbst in beiden
Sprachen. Deutsch bleiben Quellentitel, der Text eines offenen Prüfstands («zu prüfen:
…») und die Zitate der Literatur.

### Farbe

Ob Farbe erscheint, entscheidet `main.rs`: `--no-color`, die Umgebungsvariable
`NO_COLOR` oder eine Ausgabe, die keine Konsole ist, schalten sie ab; `CLICOLOR_FORCE`
erzwingt sie; `NO_COLOR` geht `CLICOLOR_FORCE` vor. Ohne Farbe entfernt `anstream` auch
jede Escape-Sequenz.

Plaketten (das Stufenwort mit je einem Leerzeichen Innenabstand) und Textmarker im
Prompt tragen feste Paare aus Schrift und Grund (`ausgabe::stil_mit`); der Fliesstext
bleibt in der Vorgabefarbe des Terminals. Kein Grün, kein Grau, kein «dim».

| Stufe | 256 Farben (Schrift auf Grund) | 16 Farben |
| --- | --- | --- |
| ROT | 231 auf 160 | hellweiss auf rot |
| ORANGE | 16 auf 208 | schwarz auf gelb |
| GELB | 16 auf 220 | schwarz auf hellgelb |
| HINWEIS | 16 auf 117 | hellweiss auf blau |

Die 16 Farben gelten, wenn `Darstellung::nur_16_farben` gesetzt ist. Fett ist nur
`Keine Funde in den geprüften Mustern` und die Überschriften von `rules` und `explain`.
Ohne Farbe steht das Stufenwort in Grossbuchstaben an der Stelle der Plakette.

### ASCII

Mit `Darstellung::ascii` gibt promptscan nur ASCII aus: `│` wird `|`,
`·` und Striche werden `-`, Guillemets und typografische Anführungszeichen werden gerade
(`"` und `'`), `…` wird `...`, `×` wird `x`, Umlaute und Akzente werden umschrieben (ä
ae, ö oe, ü ue, é e, à a, ç c usw.), auch im zitierten Prompt; geschützte und schmale
Leerzeichen werden Leerzeichen. Was keinen lateinischen Grundbuchstaben hat, wird `?`,
so oft, wie das Zeichen Spalten belegt. `tests/ausgabe.rs` prüft für jeden Prüffall, für
`rules` und jedes `explain`, dass dann jedes Byte ASCII ist. `main.rs` setzt die Option
mit `--ascii` und nach der Locale (Abschnitt 8, «Ausgabe und Umleitung»).

## 2. `promptscan rules`

Die Regeln nach Stufe gegliedert, höchstens 80 Spalten:

1. Überschrift `Regeln nach Stufe`;
2. je Stufe GELB, ORANGE, ROT, HINWEIS die Plakette und der Titel
   (`Personendaten`, `Besonders schützenswerte Personendaten`,
   `Strafbewehrte Geheimhaltungspflicht`, `Rechtsfolge nicht beurteilt`), darunter je
   Regel die Kennung (ab Spalte 3) und der Titel (ab Spalte 17) und darunter
   `Bezug: …` (rechtlicher Bezug) und `Erkennung: …` (Grundlage der Erkennung).
   Ziffern desselben Erlasses stehen zusammen
   (`Ziff. 2.2.4, 2.2.6 TAV E.164`). Die Regel `firma` steht unter ROT, weil eine
   Firma nur mit einer Geheimhaltungspflicht einen Fund ergibt; die Regeln der Klassen
   H und K unter HINWEIS;
3. `Wortlaut, Quellen und Grenzen: promptscan explain <REGEL>`;
4. die Zeile `Hilfsmittel, keine Rechtsberatung · erkennt Identifikatoren und
   Wörter aus Listen, nicht den Zusammenhang · Güte noch nicht gemessen`.

Welche Norm Bezug und welche Grundlage der Erkennung ist, entscheidet
`normen::ist_bezug`: zum Bezug gehören die Normen der Erlasse in
`normen::BEZUG_ERLASSE` (DSG, StGB, BGFA, BankG, FINIG, FinfraG, StPO, AHVG, ISG) und
die Normen des OR in `normen::BEZUG_OR` (Art. 321a Abs. 4, 328b, 342 Abs. 1 lit. a,
730b Abs. 2 OR); alle übrigen, etwa AHVV, AEFV, TAV E.164, GeoNV, die Rechtsformen
des OR, BGG, BGerR, gelten als Grundlage der Erkennung. Mit `--lang en` englisch (`Rules by level`, `Reference:`,
`Detection:`); mit `--format json` siehe «JSON: Regeln».

## 3. `promptscan explain` für eine Regel

Abschnitte in dieser Reihenfolge, Fliesstext auf die Breite umbrochen, höchstens 80
Spalten; eine lange URL steht allein auf ihrer Zeile:

1. `Kennung · Titel`;
2. `Stufe` zuoberst: welche Stufe die Regel ergibt und wann eine andere (ROT mit
   Geheimhaltungspflicht, HINWEIS mit `--target local` oder ohne Identifikator);
   `Prüfung`: die Klasse in Worten (Klasse H: «Hinweis; die Rechtsfolge beurteilt
   promptscan nicht»);
3. Erkennt; Methode; Hinweis in der Ausgabe (nur Hinweisregeln);
4. `Rechtlicher Bezug (amtlicher Wortlaut)` und, falls vorhanden,
   `Grundlage der Erkennung (amtlicher Wortlaut)`: je Norm Zitat, Erlass, Fassung,
   Wortlaut und URL (Aufteilung wie in `rules`);
5. Quellen mit Prüfstand («geprüft am …» oder «zu prüfen: …») und Lizenz;
6. Wortlisten und Register:
   - Wortliste (`.toml`): Zahl der Einträge **dieser Regel**, gesamt und je Sprache,
     und Zahl der verschiedenen Fundstellen, also der verschiedenen Werte im Feld
     `quelle` (ohne Leerraum am Rand); dazu die Einträge anderer Regeln in derselben
     Datei. Berechnet über `lexikon::lexikon_falls`.
   - Register (`.tsv`): Zahl der Zeilen und der Quellenkopf (Titel, Herausgeber,
     Abruf, Lizenz, Filter, SHA-256 der Rohdatei, URL). Berechnet über
     `lexikon::register_falls`.
   - Fehlt die Datei im Programm, steht «fehlt im Programm».
7. Gemessene Güte («nicht gemessen», solange keine Messung vorliegt);
8. Grenzen;
9. `Was promptscan nicht entscheidet`, kurz: alle acht Punkte, weil ein Fund mit
   `--role` ROT werden kann, je in einer Kurzform ohne
   Wortlaut, darunter `Wortlaut und Literatur:` mit `promptscan explain` der Stufe
   der Regel, bei GELB und ORANGE dazu `promptscan explain rot` (Klassen A und B:
   gelb und rot; C: orange und rot; `firma`: rot und gelb; K und H: hinweis, das alle
   Punkte voll zeigt).

Alle Normtexte stammen aus `src/normen.rs`, den Konstanten der Regeln und
`stufen::normen_fuer`.

Die zwei Beiträge zur Anwaltschaft (im Punkt zur Hilfsperson: «Literatur zur
Anwaltschaft, noch nicht gelesen»; promptscan nennt keinen Meinungsstand):

- Chappuis/Alberini, Secret professionnel de l’avocat et solutions cloud,
  Anwaltsrevue 8/2017, 337 ff. Die Angaben sind bestätigt durch die Bibliografie der
  Konferenz «Les solutions Cloud dans les études d'avocats» der Commission Innovations
  et Modernisation du Barreau, Ordre des avocats de Genève, 05.12.2019
  (<https://odage.ch/users_uploads/events_events_documents/6582ee5530bb6.pdf>, dort
  «Revue de l'avocat 8/2017, p. 337 ss»). Die Endseite ist nicht nachgeprüft. Eine frei
  zugängliche Fassung ist nicht bekannt. Stand: noch nicht gelesen.
- Schwarzenegger/Thouvenin/Stiller/George, Nutzung von Cloud-Diensten durch
  Anwältinnen und Anwälte, Anwaltsrevue 1/2019, 25 ff., Kurzfassung eines Gutachtens
  für den Schweizerischen Anwaltsverband. PDF auf ius.uzh.ch (URL in `explain`),
  SHA-256 5b7cd025e59eb06624ff58b62fd24031da24b432c2b6c43b5da130c5dc99fa0f; gesehen
  sind Titelseite und Zusammenfassung. Das PDF hat 8 Seiten und bricht auf S. 32
  mitten im Wort ab; die französische Fassung steht laut derselben Bibliografie in
  Revue de l'avocat 1/2019, 33 ff. Stand: noch nicht gelesen.

## 4. `promptscan explain` für eine Stufe

`gelb`, `orange`, `rot`, `hinweis`, auch `yellow`, `red`, `note`, ohne Rücksicht auf
Gross- und Kleinschreibung (`ausgabe::stufe_aus_name`). Ausgegeben werden, auf die
Breite umbrochen, höchstens 80 Spalten:

1. die Plakette der Stufe und ihr Titel (wie der Gruppenkopf im Bericht);
2. `Grund`, ab Spalte 11; bei GELB und ORANGE dazu die Arbeitgeber
   (Art. 328b OR, Art. 342 Abs. 1 lit. a OR); bei ROT der Geltungsbereich: Art. 320 und
   321 StGB, Art. 47 BankG und Art. 69 FINIG gelten unabhängig vom anwendbaren
   Datenschutzrecht, Art. 9 DSG für private Personen und Bundesorgane, die gesetzliche
   Grundlage der Bundesorgane (Art. 34 Abs. 1, Art. 36 Abs. 1 DSG), kantonale Organe
   und Verfahren nach Art. 2 Abs. 1 und 3 DSG, mit dem zweiten Satz von Abs. 3;
3. `Voraussetzung`, so wie `src/stufen.rs` sie umsetzt. Bei GELB, ORANGE und ROT gehört
   dazu die Grenze, dass ein Identifikator irgendwo im Prompt genügt, auch die eigene
   Unterschrift oder Kontaktzeile; bei GELB, wen das DSG schützt; bei ROT der offene
   Beruf, die Firma nur bei einer Pflicht für «ein Geheimnis», Art. 162 StGB als nicht
   entschieden und Art. 147 FinfraG als nicht abgedeckt; bei HINWEIS `--target local`;
4. bei ROT die Tabelle `Rollen (--role)`: je Rolle ab Spalte 3 der Wert
   für `--role`, ab Spalte 25 der Name und die Pflicht (`Anwältin/Anwalt ·
   Berufsgeheimnis`), darunter die Normen; `none` mit `kein ROT, auch nicht aus dem
   Text`; darunter, wer welche Rolle wählt (Hilfspersonen, Angestellte der Verwaltung,
   Gerichte im Strafverfahren, Finanzinstitute);
5. `Was promptscan nicht entscheidet`, voll: bei GELB und ORANGE die
   Persönlichkeitsverletzung und ihre Rechtfertigung (Art. 30, Art. 31 Abs. 1 DSG),
   Auftragsbearbeitung, Ausland, Geltungsbereich und Geschäftsgeheimnisse; bei ROT die
   Ausnahmen der Strafnormen, ob die Norm der Rolle den KI-Anbieter erfasst (mit den
   zwei Beiträgen aus Abschnitt 3), Auftragsbearbeitung, Ausland, ob eine Angabe
   geheim ist, und Geschäftsgeheimnisse; bei HINWEIS alle acht Punkte;
6. `Normen (amtlicher Wortlaut)`: jede Norm der Stufe und der Punkte darüber, jede
   einmal; bei ROT alle Normen der Rollen.

Die Funktionen sind `ausgabe::stufen_erklaerung_text` und
`ausgabe::stufen_erklaerung_json`.

Nicht übernommen ist der Vergleich mit Art. 6 Abs. 1 DSGVO aus SPEC 4.2: Die Norm steht
nicht in `src/normen.rs`, und `explain` zitiert im Wortlaut nur Normen von dort. Nur
zitiert, ohne Wortlaut, sind die Normen, die SPEC 10.1 in der zweiten Tabelle führt.

## 5. JSON

- UTF-8, eingerückt, Felder in fester Reihenfolge.
- Jedes Dokument hat `schema_version` (heute 2) und `dokument`
  (`bericht`, `regeln`, `erklaerung` oder `stufe`).
- **Stabilität:** Ein neues Feld lässt die Schema-Version unverändert. Ein entferntes,
  umbenanntes oder umgedeutetes Feld oder ein neuer Wert erhöht sie. Programme, die das
  JSON lesen, sollen unbekannte Felder übergehen.
- **Version 2 gegenüber 1:**
  - neue Werte für `rolle`: `financial-institution` (`--role`) und, nur in
    `lage.rolle` und `funde[].grund.rolle`, `offen` (der Text zeigt einen
    Geheimnis-Kontext, aber nicht den Beruf, etwa «meine Klientin»);
  - neues Feld `optionen.rolle_quelle`: woher die Rolle stammt (`keine`, `option`,
    `konfiguration`), für die Kopfzeile;
  - `bedingung_dritte` und `nicht_entschieden` tragen die Punkte «Was promptscan nicht
    entscheidet»; `bedingung_dritte` ist der Punkt «Auftragsbearbeitung» (früher «Dritter
    oder Auftragsbearbeiter»); `nicht_entschieden` einer Regel trägt alle acht Punkte mit
    vollem Text (Abschnitt 3), einer Stufe die Punkte der Stufe (Abschnitt 4);
  - `titel` der Stufen ROT und HINWEIS: englisch «Duty of confidentiality backed by
    criminal law», deutsch und englisch «Rechtsfolge nicht beurteilt» / «Legal
    consequence not assessed»; `grund` von GELB und ORANGE ohne den Geltungsbereich
    (er steht in `nicht_entschieden`), dafür mit dem Absatz zu Arbeitgebern
    (Art. 328b OR, Art. 342 Abs. 1 lit. a OR); `grund` von ROT mit dem Geltungsbereich;
  - ROT bei `auditor` nennt zusätzlich Art. 730b Abs. 2 OR;
  - der Zusatz der Regel vermerk zitiert englisch `Art. 13 Abs. N ISG` statt
    `para.`.
- **Werte:** `stufe` ist `rot`, `orange`, `gelb` oder `hinweis`; `rolle` ist ein Wert von
  `--role` (`none`, `clergy`, `lawyer`, `notary`, `patent-attorney`, `auditor`, `doctor`,
  `psychologist`, `bank`, `financial-institution`, `authority`, `court`,
  `criminal-authority`, `student`, `professional`) oder `offen` (siehe oben). `ziel` ist
  `cloud` oder `local`.
- **Positionen:** `zeile` und `spalte` zählen ab 1, die Spalte in Unicode-Skalarwerten.
  `start_zeichen` und `ende_zeichen` zählen ab 0 in Unicode-Skalarwerten im
  normalisierten Text (Normalform C, Zeilenende als `\n`); `ende_zeichen` ist exklusiv.
- Felder, die bei einer Variante fehlen (etwa `grund.ziffer` bei `personendaten`), sind
  nicht vorhanden; Felder mit `null` sind immer vorhanden.

## JSON: Bericht

`promptscan --format json`.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| `schema_version` | Zahl | Schema-Version, heute 2 |
| `dokument` | Text | `bericht` |
| `programm` | Text | `promptscan` |
| `version` | Text | Programmversion |
| `optionen` | Objekt | was der Nutzer angegeben hat |
| `optionen.rolle` | Text oder null | Wert von `--role` oder aus der Konfiguration; null, wenn nicht angegeben |
| `optionen.rolle_quelle` | Text | woher die Rolle stammt: `keine`, `option` (`--role`) oder `konfiguration`; neu in Version 2 |
| `optionen.ziel` | Text | `cloud` oder `local` |
| `lage` | Objekt | Geheimhaltungslage des Prompts |
| `lage.herkunft` | Text | `keine`, `angegeben` oder `aus-text` |
| `lage.rolle` | Text | bei `angegeben` und `aus-text`: die Rolle; bei `aus-text` auch `offen` (Beruf offen) |
| `lage.stelle` | Text | bei `aus-text`: die Textstelle, aus der die Rolle folgt |
| `lage.zeile` | Zahl | bei `aus-text`: Zeile der Stelle |
| `lage.spalte` | Zahl | bei `aus-text`: Spalte der Stelle |
| `funde` | Liste | die Funde in der Reihenfolge der Textausgabe |
| `funde[].regel` | Text | Kennung der Regel, z. B. `ahv` |
| `funde[].stufe` | Text | Stufe des Fundes |
| `funde[].zeile` | Zahl | Zeile des Anfangs, ab 1 |
| `funde[].spalte` | Zahl | Spalte des Anfangs, ab 1 |
| `funde[].start_zeichen` | Zahl | Anfang, ab 0 |
| `funde[].ende_zeichen` | Zahl | Ende, exklusiv |
| `funde[].stelle` | Text | der markierte Text |
| `funde[].zusatz` | Objekt oder null | Zusatz des Erkenners, z. B. «Prüfziffer gültig» |
| `funde[].zusatz.de` | Text | deutsch |
| `funde[].zusatz.en` | Text | englisch |
| `funde[].grund` | Objekt | warum der Fund seine Stufe hat |
| `funde[].grund.art` | Text | `personendaten`, `besonders-schuetzenswert`, `geheimnis`, `ohne-identifikator`, `geheimniskontext`, `lokal` oder `regelhinweis` |
| `funde[].grund.ziffer` | Zahl oder null | bei `besonders-schuetzenswert` und `geheimnis`: Ziffer von Art. 5 lit. c DSG |
| `funde[].grund.rolle` | Text | bei `geheimnis` und `geheimniskontext`: die Rolle, auch `offen` |
| `funde[].grund.aus_text` | Text oder null | bei `geheimnis`: die Textstelle, wenn die Rolle aus dem Text erschlossen ist |
| `funde[].grund.sonst` | Text | bei `lokal`: die Stufe ohne `--target local` |
| `funde[].normen` | Liste von Text | die Normzitate des Fundes, in der Reihenfolge der Erklärungszeilen |
| `funde[].erklaerung` | Objekt | die Erklärungszeilen der Textausgabe |
| `funde[].erklaerung.de` | Liste von Text | deutsch |
| `funde[].erklaerung.en` | Liste von Text | englisch |
| `funde[].beleg` | Objekt oder null | Herkunft eines Treffers aus einer Wortliste |
| `funde[].beleg.datei` | Text | Datei unter `lexika/` |
| `funde[].beleg.begriff` | Text | Begriff des Eintrags |
| `funde[].beleg.quelle` | Text | Fundstelle des Eintrags |
| `funde[].beleg.fassung` | Text | Fassung, JJJJ-MM-TT |
| `funde[].beleg.url` | Text | URL der Fundstelle |
| `anzahl` | Objekt | Zahl der Funde |
| `anzahl.gesamt` | Zahl | alle Funde |
| `anzahl.rot` | Zahl | Funde der Stufe ROT |
| `anzahl.orange` | Zahl | Funde der Stufe ORANGE |
| `anzahl.gelb` | Zahl | Funde der Stufe GELB |
| `anzahl.hinweis` | Zahl | Funde der Stufe HINWEIS |
| `hoechste_stufe` | Text oder null | die höchste Stufe; null ohne Fund |
| `eingabe` | Objekt | was an der Eingabe selbst auffällt (Abschnitt 1, «Meldungen zur Eingabe») |
| `eingabe.unsichtbare_formatzeichen` | Zahl | Zahl der unsichtbaren Formatzeichen, ohne Teile von Emoji-Folgen |
| `eingabe.ersatzzeichen` | Zahl | Zahl der Ersatzzeichen U+FFFD |
| `eingabe.ascii_pipe` | Text oder null | bei Verdacht auf eine ASCII-Pipe das erste verdächtige Wort, höchstens 15 Zeichen vor und nach dem `?`, Steuerzeichen als `�`; sonst null |
| `eingabe.meldungen` | Objekt | die Meldungen im Ergebnisblock unter `Ergebnis` |
| `eingabe.meldungen.de` | Liste von Text | deutsch; leer, wenn nichts auffällt |
| `eingabe.meldungen.en` | Liste von Text | englisch |

## JSON: Regeln

`promptscan rules --format json`.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| `schema_version` | Zahl | Schema-Version |
| `dokument` | Text | `regeln` |
| `regeln` | Liste | alle Regeln in der Reihenfolge der Spezifikation |
| `regeln[].id` | Text | Kennung |
| `regeln[].klasse` | Text | `A`, `B`, `C`, `H` oder `K` |
| `regeln[].titel` | Objekt | Titel |
| `regeln[].titel.de` | Text | deutsch |
| `regeln[].titel.en` | Text | englisch |
| `regeln[].grundlage` | Liste | Normen, auf denen die Regel beruht |
| `regeln[].grundlage[].zitat` | Text | z. B. `Art. 5 lit. a DSG` |
| `regeln[].grundlage[].erlass` | Text | Erlass und SR-Nummer |
| `regeln[].grundlage[].fassung` | Text | Datum der Fassung |
| `regeln[].grundlage[].url` | Text | Fedlex-URL |

## JSON: Regelerklärung

`promptscan explain <REGEL> --format json`.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| `schema_version` | Zahl | Schema-Version |
| `dokument` | Text | `erklaerung` |
| `id` | Text | Kennung der Regel |
| `klasse` | Text | `A`, `B`, `C`, `H` oder `K` |
| `titel` | Objekt | Titel |
| `titel.de` | Text | deutsch |
| `titel.en` | Text | englisch |
| `erkennt` | Objekt | was die Regel erkennt |
| `erkennt.de` | Text | deutsch |
| `erkennt.en` | Text | englisch |
| `methode` | Objekt | wie sie es erkennt |
| `methode.de` | Text | deutsch |
| `methode.en` | Text | englisch |
| `hinweis` | Objekt oder null | Zeile unter einem HINWEIS dieser Regel |
| `hinweis.de` | Text | deutsch |
| `hinweis.en` | Text | englisch |
| `grundlage` | Liste | Normen mit Wortlaut |
| `grundlage[].zitat` | Text | Zitat |
| `grundlage[].erlass` | Text | Erlass und SR-Nummer |
| `grundlage[].fassung` | Text | Datum der Fassung |
| `grundlage[].url` | Text | Fedlex-URL |
| `grundlage[].wortlaut` | Text | amtlicher Wortlaut, gekürzt mit «…» |
| `quellen` | Liste | Quellen der Methode |
| `quellen[].titel` | Text | Titel |
| `quellen[].url` | Text | URL |
| `quellen[].lizenz` | Text oder null | Lizenz bei Datensätzen |
| `quellen[].stand` | Objekt | Prüfstand |
| `quellen[].stand.art` | Text | `geprueft` oder `zu-pruefen` |
| `quellen[].stand.datum` | Text | bei `geprueft`: Datum der Prüfung |
| `quellen[].stand.was` | Text | bei `zu-pruefen`: was fehlt |
| `lexika` | Liste | Wortlisten und Register der Regel |
| `lexika[].art` | Text | `wortliste`, `register` oder `fehlt` |
| `lexika[].datei` | Text | Datei unter `lexika/` |
| `lexika[].eintraege` | Objekt | bei `wortliste`: Einträge dieser Regel |
| `lexika[].eintraege.gesamt` | Zahl | alle Sprachen |
| `lexika[].eintraege.de` | Zahl | deutsch |
| `lexika[].eintraege.fr` | Zahl | französisch |
| `lexika[].eintraege.it` | Zahl | italienisch |
| `lexika[].fundstellen` | Zahl | bei `wortliste`: verschiedene Werte des Felds `quelle` |
| `lexika[].andere_regeln` | Liste | bei `wortliste`: Einträge anderer Regeln in derselben Datei |
| `lexika[].andere_regeln[].regel` | Text | Kennung der anderen Regel |
| `lexika[].andere_regeln[].eintraege` | Zahl | Zahl ihrer Einträge |
| `lexika[].zeilen` | Zahl | bei `register`: Datenzeilen |
| `lexika[].spalten` | Liste von Text | bei `register`: Spaltennamen |
| `lexika[].quelle` | Objekt | bei `register`: der Quellenkopf |
| `lexika[].quelle.titel` | Text | Titel des Datensatzes |
| `lexika[].quelle.herausgeber` | Text | Herausgeber |
| `lexika[].quelle.url` | Text | URL |
| `lexika[].quelle.lizenz` | Text | Lizenz im Wortlaut der Quelle |
| `lexika[].quelle.abgerufen` | Text | Abrufdatum |
| `lexika[].quelle.rohdatei_sha256` | Text | SHA-256 der Rohdatei |
| `lexika[].quelle.filter` | Text | wie die Zeilen ausgewählt sind |
| `guete` | Objekt | gemessene Güte |
| `guete.art` | Text | `nicht-gemessen` oder `nachgewiesen` |
| `guete.text` | Objekt | bei `nachgewiesen`: was nachgewiesen ist |
| `guete.text.de` | Text | deutsch |
| `guete.text.en` | Text | englisch |
| `grenzen` | Liste | was die Regel nicht erkennt oder nicht entscheidet |
| `grenzen[].de` | Text | deutsch |
| `grenzen[].en` | Text | englisch |
| `bedingung_dritte` | Objekt oder null | nur bei Regeln nach Art. 5 lit. c DSG: der Punkt «Auftragsbearbeitung» |
| `bedingung_dritte.text` | Objekt | der Punkt |
| `bedingung_dritte.text.de` | Text | deutsch |
| `bedingung_dritte.text.en` | Text | englisch |
| `bedingung_dritte.normen` | Liste | Normen mit Wortlaut |
| `bedingung_dritte.normen[].zitat` | Text | Zitat |
| `bedingung_dritte.normen[].erlass` | Text | Erlass und SR-Nummer |
| `bedingung_dritte.normen[].fassung` | Text | Datum der Fassung |
| `bedingung_dritte.normen[].url` | Text | Fedlex-URL |
| `bedingung_dritte.normen[].wortlaut` | Text | amtlicher Wortlaut |
| `bedingung_dritte.literatur` | Liste | Literatur, hier leer |
| `nicht_entschieden` | Liste | alle acht Punkte «Was promptscan nicht entscheidet» (Abschnitt 3), voller Text |
| `nicht_entschieden[].text` | Objekt | der Punkt |
| `nicht_entschieden[].text.de` | Text | deutsch |
| `nicht_entschieden[].text.en` | Text | englisch |
| `nicht_entschieden[].normen` | Liste | Normen mit Wortlaut |
| `nicht_entschieden[].normen[].zitat` | Text | Zitat |
| `nicht_entschieden[].normen[].erlass` | Text | Erlass und SR-Nummer |
| `nicht_entschieden[].normen[].fassung` | Text | Datum der Fassung |
| `nicht_entschieden[].normen[].url` | Text | Fedlex-URL |
| `nicht_entschieden[].normen[].wortlaut` | Text | amtlicher Wortlaut |
| `nicht_entschieden[].literatur` | Liste | Literatur zum Streitstand |
| `nicht_entschieden[].literatur[].zitat` | Text | Zitat |
| `nicht_entschieden[].literatur[].url` | Text oder null | URL, wenn eine öffentliche Fassung bekannt ist |
| `nicht_entschieden[].literatur[].stand` | Objekt | Prüfstand, z. B. «noch nicht gelesen» |
| `nicht_entschieden[].literatur[].stand.de` | Text | deutsch |
| `nicht_entschieden[].literatur[].stand.en` | Text | englisch |

## JSON: Stufenerklärung

`promptscan explain <stufe> --format json`, etwa `promptscan explain rot --format json`.

| Feld | Typ | Bedeutung |
| --- | --- | --- |
| `schema_version` | Zahl | Schema-Version |
| `dokument` | Text | `stufe` |
| `stufe` | Text | die Stufe |
| `wort` | Objekt | Stufenwort der Ausgabe |
| `wort.de` | Text | z. B. `ROT` |
| `wort.en` | Text | z. B. `RED` |
| `titel` | Objekt | Titel |
| `titel.de` | Text | deutsch |
| `titel.en` | Text | englisch |
| `voraussetzung` | Liste | Voraussetzungen |
| `voraussetzung[].de` | Text | deutsch |
| `voraussetzung[].en` | Text | englisch |
| `grund` | Liste | Grund |
| `grund[].de` | Text | deutsch |
| `grund[].en` | Text | englisch |
| `normen` | Liste | Normen mit Wortlaut |
| `normen[].zitat` | Text | Zitat |
| `normen[].erlass` | Text | Erlass und SR-Nummer |
| `normen[].fassung` | Text | Datum der Fassung |
| `normen[].url` | Text | Fedlex-URL |
| `normen[].wortlaut` | Text | amtlicher Wortlaut |
| `rollen` | Liste | nur bei `rot`: die Rollentabelle, sonst leer |
| `rollen[].rolle` | Text | Wert von `--role` |
| `rollen[].pflicht` | Objekt | Bezeichnung der Pflicht |
| `rollen[].pflicht.de` | Text | deutsch |
| `rollen[].pflicht.en` | Text | englisch |
| `rollen[].normen` | Liste von Text | Normzitate; leer bei `none` |
| `bedingung_dritte` | Objekt oder null | nur bei `orange`: der Punkt «Auftragsbearbeitung» |
| `bedingung_dritte.text` | Objekt | der Punkt |
| `bedingung_dritte.text.de` | Text | deutsch |
| `bedingung_dritte.text.en` | Text | englisch |
| `bedingung_dritte.normen` | Liste | Normen mit Wortlaut |
| `bedingung_dritte.normen[].zitat` | Text | Zitat |
| `bedingung_dritte.normen[].erlass` | Text | Erlass und SR-Nummer |
| `bedingung_dritte.normen[].fassung` | Text | Datum der Fassung |
| `bedingung_dritte.normen[].url` | Text | Fedlex-URL |
| `bedingung_dritte.normen[].wortlaut` | Text | amtlicher Wortlaut |
| `bedingung_dritte.literatur` | Liste | Literatur, hier leer |
| `nicht_entschieden` | Liste | «Was promptscan nicht entscheidet»: fünf Punkte bei GELB und ORANGE, sechs bei ROT, alle acht bei HINWEIS (Abschnitt 4) |
| `nicht_entschieden[].text` | Objekt | der Punkt |
| `nicht_entschieden[].text.de` | Text | deutsch |
| `nicht_entschieden[].text.en` | Text | englisch |
| `nicht_entschieden[].normen` | Liste | Normen mit Wortlaut |
| `nicht_entschieden[].normen[].zitat` | Text | Zitat |
| `nicht_entschieden[].normen[].erlass` | Text | Erlass und SR-Nummer |
| `nicht_entschieden[].normen[].fassung` | Text | Datum der Fassung |
| `nicht_entschieden[].normen[].url` | Text | Fedlex-URL |
| `nicht_entschieden[].normen[].wortlaut` | Text | amtlicher Wortlaut |
| `nicht_entschieden[].literatur` | Liste | Literatur zum Streitstand |
| `nicht_entschieden[].literatur[].zitat` | Text | Zitat |
| `nicht_entschieden[].literatur[].url` | Text oder null | URL |
| `nicht_entschieden[].literatur[].stand` | Objekt | Prüfstand |
| `nicht_entschieden[].literatur[].stand.de` | Text | deutsch |
| `nicht_entschieden[].literatur[].stand.en` | Text | englisch |

## 6. Exit-Codes

Gesetzt in `main.rs` (SPEC 3): `0` kein Fund ab der Schwelle von `--fail-on`
(Vorgabe `red`); `1` mindestens ein Fund ab der Schwelle; `2` Bedienungs-, Lese- oder
Schreibfehler, etwa eine unbekannte Option, Rolle oder Regel, `-t` zusammen mit einer
Datei oder einem Befehl, Optionen im Text von `-t` (Abschnitt 8), eine fehlende Datei,
eine Eingabe in UTF-32 oder mit dem Zeichen NUL, eine Datei, die weder UTF-8 noch UTF-16
mit BOM ist (Abschnitt 1, «Kodierung der Eingabe»), eine fehlerhafte Konfiguration bei
der Prüfung (Abschnitt 8, «Konfiguration») oder ein Fehler beim Schreiben der Ausgabe,
auch mit `--output`.

`--quiet`, `--output` und Warnungen ändern den Exit-Code nicht, ebenso `--ascii`.
`--help` endet mit 0, auch wenn die Befehlszeile sonst fehlerhaft ist; nur wenn sich die
Hilfe nicht schreiben lässt (etwa mit `--output` in einen fehlenden Ordner), mit 2.
`promptscan config` endet auch bei einer fehlerhaften Datei mit 0 (nur Warnungen), mit 2,
wenn sie sich nicht schreiben lässt.

Schliesst der Empfänger die Pipe, bevor alles geschrieben ist (`| head`,
`Select-Object -First`), endet promptscan ohne Meldung mit dem Exit-Code der Funde, bei
`rules` und `explain` mit 0. Die Meldungen zur Eingabe ändern den Exit-Code nicht.

## 7. Unicode License V3

Die Bereiche des Breitenmodells sind aus Unicode-Datendateien abgeleitet. Deren Lizenz
verlangt diesen Hinweis in der Dokumentation (Text abgerufen am 06.11.2026 von
<https://www.unicode.org/license.txt>):

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2026 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.
```

## 8. Aufruf, Konfiguration, Umleitung

Umgesetzt in `src/main.rs`. Geprüft von den Tests in `src/main.rs` und `tests/cli.rs`.

### Aufruf

Drei Wege führen den Prompt hinein:

- **Einfügen:** `promptscan` ohne Datei liest stdin. Ist stdin die Konsole, steht vorher
  auf stderr `promptscan (Terminalprogramm): Prompt einfügen, Enter, dann Ctrl+Z und
  Enter.` Unter Windows beendet Ctrl+Z die Eingabe nur am Anfang einer Zeile, darum
  zuerst Enter; ausserhalb von Windows heisst es `Enter, dann Ctrl+D`.
- **Datei:** `promptscan prompt.txt`, UTF-8 oder UTF-16 mit BOM (Abschnitt 1).
- **`-t "TEXT"`:** Der Text steht dann in der Befehlszeile und kann in den Shell-Verlauf
  und in Prozessprotokolle gelangen, ebenso bei `echo … |`. Vertrauliches als Datei oder
  durch Einfügen prüfen. Ein Text, der mit `-` beginnt (Aufzählung,
  negative Zahl), ist Text: `promptscan -t "- erster Punkt"`.

Optionen stehen vor `-t` und vor der Datei, bei den Befehlen `rules`, `explain` und
`config` davor oder danach (`promptscan --lang en rules`). Bei `config` schreiben nur
`--role` und `--lang` nach dem Befehl die Datei; davor gelten sie nur für diesen Aufruf
(unten, «Konfiguration»). `-t` oder eine Datei zusammen mit einem Befehl ist ein Fehler
(Exit-Code 2), ebenso ein überzähliges Argument eines Befehls (`explain ahv rot`).
Eine Datei, die wie ein Befehl heisst, erreicht man mit `./rules` oder `-- rules`.
Kennungen von Regeln und Stufen gelten in jeder Schreibweise (`explain AHV`,
`explain Rot`), ebenso die Werte der Optionen (`--format JSON`).

**Gerade Anführungszeichen im Text von `-t`** zerlegen ihn in Windows PowerShell 5.1 und
cmd. Ein `\` vor dem schliessenden `"` schützt dieses, und die Shell zieht alle folgenden
Optionen in den Text: Aus `promptscan -t "Akte C:\Akten\" --role lawyer` kommt der Text
`Akte C:\Akten" --role lawyer` ohne Rolle an. promptscan prüft dann nicht, sondern endet
mit Exit-Code 2 und `Im Text nach -t stehen Optionen von promptscan`. Kommt nach `-t`
ein Teil des Texts als eigenes Argument mit Leerzeichen an, in Windows PowerShell 5.1
etwa aus `-t 'Er sagte "ja, gerne" und ging'`, lautet die erste Zeile
`Der Text nach -t ist zerfallen: «gerne und ging» kam einzeln an.` Bei jedem anderen
Fehler der Befehlszeile mit `-t` folgt ein Hinweis auf gerade Anführungszeichen;
Abhilfe ist der Prompt als Datei oder durch Einfügen, in PowerShell auch das
Stoppzeichen `--%`: `promptscan --% -t "Er sagte \"ja\""`.

**Doppelklick:** Startet promptscan per Doppelklick (Windows: promptscan hängt allein an
seiner Konsole, stdin und stdout sind die Konsole), schlösse sich das Fenster mit dem
Ende. promptscan wartet darum zum Schluss: `Zum Schliessen Enter drücken.` Nie bei
Umleitung, Pipe oder in einer Shell. Ein Enter in der ersten Sekunde danach zählt nicht:
Ctrl+Z am Anfang einer Zeile beendet das Einfügen sofort, und das Enter aus «Ctrl+Z und
Enter» schlösse sonst das Fenster, bevor jemand den Bericht liest. Nachgeprüft in einer
eigenen, unsichtbaren Konsole mit eingespeisten Tasten unter Windows 11.

Dasselbe gilt, wenn die Aufgabenplanung oder ein anderes Programm promptscan direkt
startet: Es bekommt eine eigene Konsole, wartet auf Enter und endet nie von selbst. Aus
der Aufgabenplanung darum über `cmd /c` starten (`cmd /c promptscan prompt.txt`); ein
Programm, das promptscan selbst startet, leitet stdin um, etwa auf `NUL`. Nachgeprüft
mit Windows PowerShell 5.1: Direkt mit eigener Konsole gestartet, lief promptscan nach
2,5 s noch; über `cmd /c` und mit umgeleitetem stdin war es beendet.

### Hilfe und Meldungen

`promptscan --help` (auch `-h` oder `promptscan help`) schreibt die Hilfe auf höchstens
80 Spalten: die drei Eingabewege, die Befehle, die Stufen, alle Optionen mit allen
Rollen, den Ort der Konfiguration, die Exit-Codes und die Grenzen des Werkzeugs. Die
Hilfe erscheint auch, wenn die Befehlszeile sonst fehlerhaft ist. Mit `--output DATEI`
geht sie in die Datei (UTF-8 mit BOM, unten), mit `--ascii` in ASCII.

Meldungen gehen nach stderr. Die erste Zeile beginnt mit `promptscan: ` und nennt das
Problem; darunter, um 2 eingerückt, ein Vorschlag und der nächste Schritt. Jede Zeile
ist auf 80 Spalten umgebrochen, die erste eingeschlossen; Befehle wie
`promptscan explain rot` werden dabei nicht getrennt. Eine Meldung nennt keine
Systemcodes und keine englischen Texte des Zerlegers:

```text
promptscan: Die Rolle «anwält» gibt es nicht.
  Gemeint ist wohl: --role lawyer
  Alle Rollen mit Pflicht und Normen: promptscan explain rot
```

Ein Pfad steht hinter `Datei:` am Ende der ersten Zeile oder, wenn er dort nicht mehr
Platz hat, auf einer eigenen Zeile. Länger als die Zeile bleibt er ungebrochen, damit man
ihn kopieren kann. Meldungen aus einer einzigen Zeile, etwa die Meldungen zur Eingabe und
zur Kodierung (Abschnitt 1), bleiben ungebrochen.

Sprache von Hilfe und Meldungen: `--lang`, sonst `lang` aus der Konfiguration, sonst
Deutsch, auch bei Fehlern der Befehlszeile. Ein Vorschlag erscheint bei einem Tippfehler
in einer Rolle (auch in deutschen Wörtern), einer Option (auch in Grossbuchstaben:
`--ROLE` ergibt `--role`), einer Regel oder einer Stufe, und bei deutschen Werten:
`--fail-on rot` ergibt `Gemeint ist wohl: --fail-on red`, ebenso `gelb`, `nie`,
`lokal`, `deutsch` und `englisch`. Hat ein Befehl ein Argument zu viel, nennt die
Meldung seine richtige Form (`Je Aufruf eine Regel oder Stufe, etwa: promptscan explain
ahv`).

`--role` nimmt die Kennung oder ein deutsches Wort, in jeder Schreibweise: `anwalt`,
`Anwältin`, `ärztin`, `notar`, `behörde`, `gericht`, `staatsanwaltschaft` und weitere
(Liste in `src/fund.rs`, `EINGABEWOERTER`).

### Konfiguration

Pfad: `PROMPTSCAN_CONFIG`, sonst unter Windows `%APPDATA%\promptscan\config.toml`,
sonst `$XDG_CONFIG_HOME/promptscan/config.toml` oder `~/.config/promptscan/config.toml`.
Die Datei ist TOML und hält zwei Schlüssel:

| Schlüssel | Werte | Wirkung |
| --- | --- | --- |
| `role` | Kennung oder deutsches Wort wie bei `--role` | Rolle, wenn `--role` fehlt |
| `lang` | `de`, `en` | Sprache, wenn `--lang` fehlt |

`target` wird übergangen, mit einer Warnung: Das Ziel gilt je Aufruf (`--target local`).
Die Warnung sagt, wie sie verschwindet: die Zeile `target` aus der Datei löschen;
`promptscan config` ändert sie nicht.
Gelesen wird die Datei wie die Eingabe: UTF-8 mit oder ohne BOM oder UTF-16 mit BOM. Eine
Datei, die Windows PowerShell 5.1 mit `>` schreibt, gilt also (den Ordner muss es geben;
einfacher ist `promptscan config --role lawyer`):

```powershell
'role = "lawyer"' > $env:APPDATA\promptscan\config.toml
```

`promptscan config` zeigt Pfad, Herkunft des Pfads, Inhalt und gültige Werte.
`promptscan config --role lawyer` und `promptscan config --lang en` schreiben die Datei,
UTF-8 ohne BOM, und legen den Ordner an. Nur `--role` und `--lang` nach «config»
schreiben: `promptscan --lang en config` zeigt die Datei englisch und lässt sie, wie sie
ist; `promptscan --role lawyer config` schreibt nichts und sagt, wie man speichert.
`--target` speichert `config` nicht.

Geschrieben wird zeilenweise. Die Zeile des Schlüssels wird ersetzt, ein Kommentar an
ihrem Ende bleibt; fehlt der Schlüssel, kommt eine Zeile ans Ende. Alle übrigen Zeilen
bleiben, wie sie sind, samt Kommentaren, Reihenfolge und Zeilenenden (CRLF). Die
Bestätigung nennt den alten Wert:

```text
Gespeichert: role = "lawyer" (bisher "doctor"), lang = "en".
```

Ergibt die geänderte Datei nicht genau die erwarteten Werte, etwa bei einem Wert über
mehrere Zeilen, schreibt promptscan sie aus ihren Werten neu und warnt: `Die Datei ist
aus ihren Werten neu geschrieben; Kommentare sind weggefallen.` Eine unlesbare Datei wird
ersetzt, ebenfalls mit einer Warnung. Ist das Laufwerk nicht erreichbar, etwa ein
getrenntes Netzlaufwerk, endet `config` mit Exit-Code 2 und `Das Laufwerk ist nicht
erreichbar`.

Fehler und Warnungen:

| Lage | Folge |
| --- | --- |
| Prüfung ohne `--role`, Datei unlesbar, falsch kodiert, kein TOML, unbekannter Schlüssel, Wert ohne Anführungszeichen, unbekannte Rolle | Exit-Code 2: Eine eingestellte Rolle fiele sonst still weg, und aus ROT würde GELB |
| Prüfung ohne `--lang`, unbekannte Sprache | Exit-Code 2 |
| derselbe Fehler, aber `--role` (bei der Sprache `--lang`) angegeben | Warnung, die Befehlszeile gilt |
| `rules`, `explain`, `config` | jeder Fehler nur als Warnung |
| `target` in der Datei | Warnung, übergangen |

Jede Meldung beginnt mit `Konfiguration:`, nennt das Problem und die Datei, darunter die
Abhilfe:

```text
promptscan: Konfiguration: Zeile 1 ist kein gültiges TOML: «role = lawyer».
  Datei: C:\Users\Erika Beispiel\AppData\Roaming\promptscan\config.toml
  Erwartet je Zeile etwa role = "lawyer" oder lang = "de", mit geraden
  Anführungszeichen; oder: promptscan config --role lawyer
```

### Ausgabe und Umleitung

**JSON** geht auf stdout und in Dateien in reinem ASCII hinaus: jedes andere Zeichen als
`\uXXXX`, ausserhalb der BMP als Ersatzpaar (`\ud83d\ude00`). Das geparste JSON bleibt
gleich; so übersteht es jede Codepage.

**Klartext:** Der Textbericht enthält den Prompt (bis 40 Zeilen ganz, sonst die Zeilen
mit Funden), das JSON jede markierte Stelle, beide im Klartext, auch in der Datei von
`--output`. Bericht und JSON gehören darum nicht in fremde Protokolle oder in
synchronisierte Ordner. Nur das Ergebnis, ohne Prompt und Funde: `--quiet`.

**Umleitung in Windows PowerShell 5.1:** `promptscan prompt.txt > bericht.txt` liest die
Ausgabe in der Codepage der Konsole und schreibt sie als UTF-16; die Umlaute kommen
verstümmelt an. Darum `--output bericht.txt`: UTF-8 mit BOM, das Editor, Excel und
`Get-Content` erkennen, ohne Farbe, für den Bericht, die Hilfe, `rules`, `explain` und
`config`. Der Exit-Code bleibt der der Funde; den Ordner muss es geben. Ist stderr die
Konsole, bestätigt promptscan mit `Ausgabe geschrieben, UTF-8 mit BOM.` und darunter
`Datei: PFAD`.

**Pipe in Windows PowerShell 5.1:** Eine Pipe in ein Cmdlet (`| Select-String`,
`| Out-File`, `| Set-Clipboard`) liest die Ausgabe ebenfalls in der Codepage der Konsole;
aus «Zürich» wird «Z├╝rich», und `--output` hilft dort nicht. In der Sitzung vorher

```powershell
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
```

setzen oder `--format json` verwenden, das in reinem ASCII hinausgeht.

**BOM:** Wer JSON aus einer Datei von `--output` maschinell liest, beachtet das BOM: in
Python `encoding="utf-8-sig"`; in Node.js scheitert `JSON.parse` daran, dort das
Zeichen U+FEFF am Anfang entfernen.

**Schreibfehler:** Lässt sich die Ausgabe nicht schreiben, mit `--output` oder auf stdout,
endet promptscan mit Exit-Code 2. Die Meldung nennt den Grund in eigenen Worten, deutsch
oder englisch: `Den Ordner gibt es nicht` (promptscan legt keinen an), `Der Pfad ist ein
Ordner`, `Die Datei ist in einem anderen Programm geöffnet` (unter Windows etwa in Word
oder Excel), `Zugriff verweigert`, `Der Datenträger ist voll`, `Der Datenträger ist
schreibgeschützt`; sonst den Text des Systems in Klammern. Darunter steht der nächste
Schritt:

```text
promptscan: Die Ausgabe lässt sich nicht schreiben: Den Ordner gibt es nicht.
  Datei: C:\Users\Erika Beispiel\Berichte\bericht.txt
  Pfad prüfen; promptscan legt keinen Ordner an.
```

Hat der Empfänger die Pipe geschlossen, ist das kein Fehler (Abschnitt 6).

**`--quiet`** gibt nur das Ergebnis aus: die Zählzeile und die Zeilen darunter, ohne
Prompt und Funde; mit `--format json` das Dokument `bericht-kurz`. Der Exit-Code bleibt.

**Breite:** `main.rs` bestimmt sie und übergibt sie der Ausgabe (`Darstellung::breite`):
unter Windows die Breite des Konsolenfensters, sonst `COLUMNS`, sonst 80; mindestens 60;
bei Umleitung und mit `--output` immer 80. Der Bericht bricht auf diese Breite um, auch
die Zeilen der Zeilenansicht (Abschnitt 1). Hilfe und Meldungen haben immer höchstens 80
Spalten.

**ASCII:** Mit `--ascii` ist jedes Zeichen der Ausgabe ASCII: in Bericht, Hilfe,
Meldungen, `rules`, `explain` und `config`; mit `--output` steht davor wie immer das BOM.
Eigene Texte (Hilfe, Meldungen, `config`) schreibt promptscan lesbar um: ä → ae,
Eszett → ss, « und » → ", … → ... Im Bericht, in `rules` und `explain` wird jedes
Zeichen zu so vielen Zeichen, wie es Spalten belegt, damit die Marken `^` unter ihrer
Stelle bleiben: ä → a, │ → |, · → -, ein Zeichen über zwei Spalten → ??; was sich nicht
umschreiben lässt, wird zu ?. JSON ist ohnehin ASCII. Ausserhalb von Windows gilt das
auch ohne Option, wenn `LC_ALL`, `LC_CTYPE` oder `LANG` (die erste nicht leere) gesetzt
ist und kein UTF-8 nennt, etwa `C` oder `POSIX`; nicht aber mit `--output`.

Die Abfragen an die Windows-Konsole (Fensterbreite, Prozesse an der Konsole) laufen über
`windows-sys`, das über `anstream` schon in den Abhängigkeiten steht.
