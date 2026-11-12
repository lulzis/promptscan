# promptscan — Spezifikation v0.1

Stand 12.11.2026

## 1. Zweck

promptscan prüft einen Prompt, bevor er an eine KI geht. Es markiert farbig, welche Stellen nach Schweizer Recht nicht ohne Weiteres hinausgehen sollten, und nennt zu jeder Markierung die Norm, aus der sie folgt.

```
promptscan entwurf.txt
```

## 2. Grundsätze

1. **Jede Regel hat eine Rechtsgrundlage** im geltenden amtlichen Wortlaut, mit Fassungsdatum (Abschnitt 10).
2. **Jedes Erkennungsverfahren hat eine belegte Methode**: Prüfziffer, amtliches Format oder eine Wortliste, in der jeder Eintrag seine Quelle trägt. Ein Eintrag ohne Quelle kommt nicht hinein.
3. **Jede Güteangabe ist gemessen**, mit 95-%-Intervall. Was nicht gemessen ist, heisst «nicht gemessen».
4. **promptscan meldet Datenkategorien, nie Rechtsverletzungen.** Ob eine Übermittlung rechtswidrig ist, hängt an Tatsachen, die das Werkzeug nicht kennt: Einwilligung, Entbindung, Vertrag mit dem Anbieter, Hilfsperson-Status des Anbieters, Ort der Bearbeitung (4.5).
5. **Alles lokal.** Das Programm enthält keinen Netzwerkcode, startet keine anderen Programme und schreibt Dateien nur auf ausdrücklichen Befehl: `promptscan config --role …` bzw. `--lang …` die Konfigurationsdatei, `--output` den Bericht. Tests prüfen das am Quelltext, an Cargo.lock und Cargo.toml und an der Importtabelle der Programmdatei, das Schreiben am eigenen Quelltext (Abschnitt 6).
6. **Datenherkunft:** nur amtliche oder öffentliche, fremde Quellen. Keine Daten und kein Code aus anderen Projekten.
7. **Abgrenzung:** promptscan anonymisiert keine Texte, schätzt kein Re-Identifikationsrisiko aus dem Kontext und verarbeitet keine Gerichtsentscheide.

## 3. Bedienung

Nur Terminal. Die Farbe trägt die Stufe, und das Wort daneben trägt sie auch. Ohne Farbe bleibt die Ausgabe lesbar; die Stellen werden dann mit `^^^` unter der Zeile markiert. Hilfe (`promptscan --help`) und Meldungen gibt es deutsch und englisch, wie die Ausgabe (`--lang`). Aufruf, Konfiguration und Umleitung im Einzelnen: docs/AUSGABE.md, Abschnitt 8.

```
promptscan                    Prompt einfügen, Enter, dann Ctrl+Z und Enter (Windows)
                              bzw. Ctrl+D
promptscan DATEI              Datei prüfen, UTF-8 oder UTF-16 mit BOM
promptscan -t "TEXT"          Text direkt prüfen. Er steht dann in der Befehlszeile und
                              kann in den Shell-Verlauf und in Prozessprotokolle gelangen,
                              ebenso bei «echo … |». Vertrauliches als Datei oder durch
                              Einfügen prüfen.
promptscan rules              alle Regeln mit Rechtsgrundlage
promptscan explain <REGEL>    Normtext, Methode, Quellen der Wortliste, gemessene Güte,
                              Streitstand; auch für eine Stufe (rot, orange, gelb, hinweis)
promptscan config             Konfigurationsdatei zeigen; mit --role oder --lang nach
                              «config» schreiben

  --role <ROLLE>              none | clergy | lawyer | notary | patent-attorney | auditor
                              | doctor | psychologist | bank | financial-institution
                              | authority | court | criminal-authority | student
                              | professional; auch deutsch (anwalt, ärztin, behörde …);
                              ohne Angabe zeigt der Text die Rolle (4.3);
                              dauerhaft: promptscan config --role <ROLLE>
  --target cloud|local        Wohin der Prompt geht; local nur, wenn das Modell auf diesem
                              Rechner läuft, ohne Synchronisierung in eine Cloud (4.4).
                              Vorgabe cloud; gilt je Aufruf
  --format text|json          json in reinem ASCII
  --lang de|en                Sprache der Ausgabe; erkannt wird immer Deutsch, Französisch, Italienisch
  --output <DATEI>            Ausgabe in eine Datei: UTF-8 mit BOM, ohne Farbe
  --quiet                     nur das Ergebnis, ohne Prompt und Funde
  --fail-on red|orange|yellow|never    Exit-Code 1 ab dieser Stufe, für Skripte
  --no-color                  auch über NO_COLOR; automatisch, wenn die Ausgabe keine Konsole ist
  --ascii                     nur ASCII-Zeichen
  -h, --help                  Hilfe; englisch: --help --lang en
```

Optionen stehen vor `-t` und vor der Datei, bei einem Befehl davor oder danach: `promptscan --role lawyer -t "…"`. Eine Datei, die wie ein Befehl heisst, erreicht man mit `./`: `promptscan ./rules`. Bei `config` schreiben nur `--role` und `--lang` nach dem Befehl die Datei; davor gelten sie wie bei `rules` nur für diesen Aufruf.

Konfiguration: `%APPDATA%\promptscan\config.toml` (ausserhalb von Windows `$XDG_CONFIG_HOME/promptscan/config.toml`, sonst `~/.config/promptscan/config.toml`); die Umgebungsvariable `PROMPTSCAN_CONFIG` geht vor. Schlüssel `role` und `lang`; das Ziel gilt je Aufruf. `promptscan config` zeigt Pfad und Inhalt.

Exit-Codes: `0` nichts ab Schwelle · `1` Fund ab Schwelle · `2` Bedienungs-, Lese- oder Schreibfehler, auch eine Eingabe in UTF-32 oder mit dem Zeichen NUL. Schliesst der Empfänger die Pipe vorzeitig (`| head`, `Select-Object -First`), endet promptscan ohne Meldung mit dem Code der Funde. Einzelheiten: docs/AUSGABE.md, Abschnitt 6.

### Beispiel 1 — ohne Berufsrolle

```
$ promptscan kuendigung.txt
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

Oben die Kopfzeile, dann der Prompt mit den markierten Stellen, die Funde nach Stufe und zuletzt der Ergebnisblock (Aufbau, Farben und Breite: AUSGABE.md, Abschnitt 1). Die Frage «Gilt für Sie eine Geheimhaltungspflicht (Beruf, Amt, Bank)? Dann --role angeben» steht unter `Klären`, wenn weder `--role` noch die Konfiguration eine Rolle nennt, der Text keine zeigt und ein Fund GELB oder ORANGE ist; `role = "none"` unterdrückt sie. Ist die Rolle `authority`, `court` oder `criminal-authority`, steht unter `Beachten`, welches Datenschutzrecht gilt («Für kantonale und kommunale Organe gilt kantonales Datenschutzrecht, für Bundesorgane das DSG. …»), und die Frage nach dem Ausland nennt nur Art. 16 f. DSG für Bundesorgane.

### Beispiel 2 — mit Berufsrolle

Die Datei `mandantin.txt` enthält den Satz in der Zeilenansicht. Vertrauliche Texte als Datei oder durch Einfügen prüfen: Mit `-t` steht der Text in der Befehlszeile, im Shell-Verlauf und in Prozessprotokollen.

```
$ promptscan --role lawyer mandantin.txt
Lokal geprüft · Rolle: Anwältin/Anwalt (--role) · Ziel: Cloud

   │ Meine Mandantin Erika Beispiel, IBAN CH93 0076 2011 6238 5295 7, will das
   │                 R^^^^^^^^^^^^^       R^^^^^^^^^^^^^^^^^^^^^^^^^
   │ Erbe ausschlagen.
     R ROT

ROT       Berufsgeheimnis, Art. 321 Ziff. 1 StGB, Art. 13 Abs. 1 BGFA
          Name          Erika Beispiel
          IBAN          CH93 0076 2011 6238 5295 7 (Prüfziffer gültig)

Ergebnis  ROT   Strafbewehrte Geheimhaltungspflicht · 2 Funde
Klären    Braucht die KI diese Angaben? Wenn ja: Liegt eine Einwilligung oder
          schriftliche Bewilligung vor (Art. 321 Ziff. 2 StGB)? Ist der Anbieter
          Ihre Hilfsperson (Art. 321 Ziff. 1 StGB)?
          Gehen die Daten ins Ausland? Dann gelten Art. 16 f. DSG, Anhang 1 DSV.
Mehr      promptscan explain rot
```

Beide Nummern sind gängige Beispielnummern; ihre Prüfziffern sind nachgerechnet.

## 4. Rechtliches Modell

### 4.1 Rechtslage in einem Absatz

Die Schweiz hat kein KI-Gesetz. Das DSG ist technologieneutral und gilt nach dem EDÖB direkt auch für KI-gestützte Bearbeitungen (Mitteilung vom 09.11.2023). Der Bundesrat will die KI-Konvention des Europarats übernehmen, sektoriell durch Anpassung bestehender Gesetze, und hat die Vernehmlassung für Ende 2026 angekündigt. Massstab für einen Prompt bleiben damit das DSG und die bestehenden Geheimnisnormen. Genau die bildet promptscan ab.

### 4.2 Vier Stufen

Jede Stufe entspricht einem eigenen Rechtsregime mit stärkerer Rechtsfolge. Ein Element erhält die höchste zutreffende Stufe. Die Spalte «Grund» ist der Text, den `promptscan explain <stufe>` unter «Grund» zeigt (Wortlaut in `src/ausgabe.rs`).

| Stufe | Voraussetzung | Grund für diese Stufe |
|---|---|---|
| GELB | Personendaten: Angabe zu einer bestimmten oder bestimmbaren natürlichen Person (Art. 5 lit. a DSG) | Private dürfen gewöhnliche Personendaten ohne Rechtfertigungsgrund bearbeiten, solange sie die Persönlichkeit nicht widerrechtlich verletzen (Art. 30 Abs. 1 DSG). Massstab ist die Verhältnismässigkeit (Art. 6 Abs. 2 DSG): Braucht die KI die Angabe? Gehen die Daten ins Ausland, gelten zusätzlich Art. 16 f. DSG. |
| ORANGE | besonders schützenswerte Personendaten (Art. 5 lit. c Ziff. 1–6 DSG) | Die Bekanntgabe besonders schützenswerter Personendaten an Dritte durch Private ist eine Persönlichkeitsverletzung (Art. 30 Abs. 2 lit. c DSG). Ist eine Einwilligung erforderlich, muss sie ausdrücklich erfolgen (Art. 6 Abs. 7 lit. a DSG). |
| ROT | Daten unter strafbewehrter Geheimhaltungspflicht (4.3) | Je nach Rolle ist das Offenbaren eines Geheimnisses strafbar (Normen in 4.3). Ob hier offenbart wird und ob eine Ausnahme greift, entscheidet promptscan nicht (4.5). Zudem darf die Bearbeitung keinem Auftragsbearbeiter übertragen werden, wenn eine gesetzliche oder vertragliche Geheimhaltungspflicht es verbietet (Art. 9 Abs. 1 lit. b DSG). |
| HINWEIS | keine Rechtsfolge, die das Werkzeug zuordnet: Vertraulichkeitsvermerk, Bitte um Fundstellen, AHV-Nummern in Tabellenform, lokales Modell, Personenbezug ohne Identifikator, Geheimnis-Kontext | Die Bewertung hängt von Tatsachen ab, die promptscan nicht kennt. Darum ordnet promptscan keine Rechtsfolge zu; ausgeschlossen ist sie damit nicht. |

**Auftragsbearbeitung.** Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist, hängt daran, ob er nur im Auftrag bearbeitet (Art. 5 lit. j und k DSG); ein Vertrag allein entscheidet das nicht. Nutzt der Anbieter Eingaben für eigene Zwecke, etwa zum Training, ist er insoweit Dritter. Übertragen werden darf nur, wenn er die Daten so bearbeitet, wie der Verantwortliche selbst es dürfte (Art. 9 Abs. 1 lit. a DSG), keine gesetzliche oder vertragliche Geheimhaltungspflicht es verbietet (Art. 9 Abs. 1 lit. b DSG) und der Verantwortliche sich vergewissert hat, dass der Anbieter die Datensicherheit gewährleisten kann (Art. 9 Abs. 2 DSG); einem Dritten übertragen darf der Anbieter nur mit vorgängiger Genehmigung (Art. 9 Abs. 3 DSG). Private, die vorsätzlich ohne die Voraussetzungen von Art. 9 Abs. 1 und 2 DSG übertragen, werden auf Antrag mit Busse bis zu 250 000 Franken bestraft (Art. 61 lit. b DSG). promptscan kennt weder den Vertrag noch die Nutzung. ORANGE markiert deshalb die Kategorie; der Bericht fragt unter `Klären` nach der Tatsache («Nutzt der Anbieter sie nur in Ihrem Auftrag oder auch für eigene Zwecke, etwa zum Training (Art. 9 DSG)?»), jedes `explain` nennt den Punkt unter «Was promptscan nicht entscheidet» (4.5), das JSON in `bedingung_dritte`.

**Geltungsbereich.** Diese Begründung gilt für private Personen (5. Kapitel DSG). Auf Daten, die eine natürliche Person ausschliesslich zum persönlichen Gebrauch bearbeitet, ist das DSG nicht anwendbar (Art. 2 Abs. 2 lit. a DSG). Bundesorgane brauchen für Bearbeitung und Bekanntgabe grundsätzlich eine gesetzliche Grundlage (Art. 34 Abs. 1 DSG, Art. 36 Abs. 1 DSG; Ausnahmen: Art. 34 Abs. 4 DSG, Art. 36 Abs. 2 DSG). Für kantonale und kommunale Organe gilt nicht das DSG (Art. 2 Abs. 1 DSG), sondern kantonales Datenschutzrecht. In Gerichtsverfahren und in Verfahren nach bundesrechtlichen Verfahrensordnungen gilt das Verfahrensrecht, in erstinstanzlichen Verwaltungsverfahren das DSG (Art. 2 Abs. 3 DSG). Bei ROT gelten Art. 320 und 321 StGB, Art. 47 BankG und Art. 69 FINIG unabhängig davon, welches Datenschutzrecht anwendbar ist; Art. 9 DSG steht im 2. Kapitel «Allgemeine Bestimmungen» und gilt für private Personen und Bundesorgane (Art. 2 Abs. 1 DSG). `explain gelb`, `explain orange` und `explain hinweis` nennen den Geltungsbereich unter «Was promptscan nicht entscheidet», `explain rot` unter «Grund»; bei den Rollen `authority`, `court` und `criminal-authority` steht er im Bericht unter `Beachten`. Der Kopf der Gruppe ORANGE lautet darum «Bekanntgabe an Dritte durch Private: Art. 30 Abs. 2 lit. c DSG».

**Arbeitgeber.** Arbeitgeber dürfen Daten über Arbeitnehmende nur bearbeiten, soweit sie deren Eignung für das Arbeitsverhältnis betreffen oder zur Durchführung des Arbeitsvertrags erforderlich sind (Art. 328b OR); im Übrigen gilt das DSG. Vorschriften über das öffentlich-rechtliche Dienstverhältnis bleiben vorbehalten (Art. 342 Abs. 1 lit. a OR). `explain gelb` und `explain orange` nennen beides unter «Grund».

### 4.3 Geheimnis-Kontext (ROT)

Die Berufsrolle ist eine Tatsache, die der Nutzer kennt und das Werkzeug nicht sicher erschliessen kann. Deshalb wird sie erfragt (`--role`) statt geraten.

| Rolle | Norm | Aus dem Wortlaut |
|---|---|---|
| lawyer | Art. 321 Ziff. 1 StGB; Art. 13 BGFA | «alles, was ihnen infolge ihres Berufes von ihrer Klientschaft anvertraut worden ist», zeitlich unbegrenzt; Hilfspersonen erfasst (Art. 13 Abs. 2 BGFA) |
| clergy, notary, patent-attorney, auditor | Art. 321 Ziff. 1 StGB | Geistliche, Notare, Patentanwälte; «nach Obligationenrecht zur Verschwiegenheit verpflichtete Revisoren» |
| doctor, psychologist | Art. 321 Ziff. 1 StGB | abschliessende Berufsliste im Wortlaut, u. a. Ärzte, Zahnärzte, Psychologen, Pflegefachpersonen |
| bank | Art. 47 BankG | auch die fahrlässige Verletzung ist strafbar (Busse bis CHF 250 000) |
| financial-institution | Art. 69 Abs. 1 lit. a und Abs. 3 FINIG | Finanzinstitute nach Art. 2 Abs. 1 FINIG: Vermögensverwalter, Trustees, Verwalter von Kollektivvermögen, Fondsleitungen, Wertpapierhäuser; «ein Geheimnis», das Organen, Angestellten, Beauftragten oder Liquidatoren eines Finanzinstituts anvertraut worden ist; auch fahrlässig strafbar (Busse bis CHF 250 000) |
| authority, court | Art. 320 Ziff. 1 StGB | Amtsgeheimnis: «als Mitglied einer Behörde oder als Beamter» |
| criminal-authority | Art. 320 Ziff. 1 StGB; Art. 73 Abs. 1 StPO | Strafbehörden nach Art. 12 und 13 StPO (Polizei, Staatsanwaltschaft, Übertretungsstrafbehörden, Gerichte im Strafverfahren): «Die Mitglieder von Strafbehörden, ihre Mitarbeiterinnen und Mitarbeiter sowie die von Strafbehörden ernannten Sachverständigen bewahren Stillschweigen» |
| student | Art. 321 Ziff. 1 Abs. 2 StGB; Art. 62 Abs. 2 DSG | «Studierende, die ein Geheimnis offenbaren, das sie bei ihrem Studium wahrnehmen»; Ausbildung bei einer geheimhaltungspflichtigen Person |
| professional | Art. 62 Abs. 1 DSG | jeder Beruf, der die Kenntnis geheimer Personendaten erfordert; auf Antrag, Busse bis CHF 250 000 |
| none | — | keine Stufe ROT aus der Rolle |

Nicht abgedeckt ist das Berufsgeheimnis der Finanzmarktinfrastrukturen (Art. 147 FinfraG): Keine Rolle nennt es.

**Ohne `--role`** hebt promptscan auf ROT, wenn der Text selbst einen Geheimnis-Kontext zeigt. Dazu gehören die Berufsbezeichnungen aus Art. 320 und 321 StGB, Art. 13 BGFA, der StPO und Art. 2 Abs. 1 FINIG in den amtlichen Fassungen DE, FR und IT, gleichbedeutende aus GesBG, KVV, StBOG, RVOG und Art. 635a OR und, wo kein Erlass das Wort braucht, aus dem Wörterbuch (Duden, Treccani), jede in einem Rahmen der ersten Person («ich bin Ärztin», «Als Anwältin habe ich», «en tant qu'avocat, je», «sono avvocato»); der Eintrag zeigt die Rolle. Klientschaft in der ersten Person («meine Klientin», «mein Mandant», «mon client», «il mio cliente») und die Vertretung einer Person («Ich vertrete Herrn …», «Je représente …», «Rappresento …») zeigen den Beruf nicht: Die Wörter der Klientschaft stehen für Anwältinnen und Notare (Art. 18a Abs. 3 GwG), für die Psychotherapie (Art. 27 lit. c PsyG), in der französischen und italienischen Fassung auch für die Kundschaft einer Bank (Art. 4bis Abs. 1 BankG) und für die Partei jedes Auftrags (Art. 398 Abs. 2 OR); vertreten kann jede dazu ermächtigte Person (Art. 32 Abs. 1 OR), etwa ein Beistand (Art. 394 Abs. 1 ZGB). Keine Vertretung in diesem Sinn sind die Stellvertretung («Ich vertrete meine Kollegin Anna Roth», «Ich vertrete Frau Anna Roth während ihrer Ferien») und das Darstellen («Dans ce tableau, je représente …», «Nel mio quadro rappresento …»); umgekehrt zählt eine Vertretung mit Zeitangabe nicht («Ich vertrete Frau Anna Roth bis zum Abschluss des Verfahrens»), und andere Lesarten bleiben Fehltreffer («Morgen vertrete ich Herrn Hans Muster im Unterricht»). ROT bleibt die vorsichtige Stufe, mit «Beruf offen» und den Normen, die in Frage kommen (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB, Art. 47 Abs. 1 lit. a BankG); Art. 13 BGFA nennt erst `--role lawyer`, und der Fund bietet die Rollen zur Auswahl an. «meine Patientin» zeigt doctor. Wer ausschliesslich Vermögenswerte wirtschaftlich oder familiär verbundener Personen verwaltet, untersteht dem FINIG nicht (Art. 2 Abs. 2 lit. a FINIG): «Ich bin Vermögensverwalter meiner Mutter», «Je suis gestionnaire de fortune de ma mère» und «Sono gestore patrimoniale di mia madre» zeigen keine Rolle, vor einem Wort der Kundschaft («meiner Kundin») bleibt sie. Für die Angestellten einer Bank gilt Art. 47 BankG statt Art. 69 FINIG (Art. 2 Abs. 2 lit. j FINIG); der Text zeigt dann financial-institution, die Bank legt `--role bank` fest. Im Obligationenrecht ist «procuratore» der Prokurist und «procura generale» eine Generalvollmacht (Art. 458 Abs. 1, Art. 535 Abs. 3 OR): «procuratore generale di …» zählt nur vor dem Amt («della Confederazione», «del Cantone Ticino»). «Revisor», «réviseur» und «revisore» bezeichnen auch, wer Texte oder Druckbogen prüft, FR «contrôleur» auch andere Kontrolleure; diese Fehltreffer nennt `explain kontext`. Keine Rolle aus dem Text gibt es für bank, student und professional: hier gilt nur `--role`. Jeder Eintrag nennt seine Norm. Diese Erkennung wird getrennt gemessen. Reicht ihre Präzision nicht, gilt ROT nur noch über `--role`.

**Juristische Personen.** Das DSG schützt nur natürliche Personen. AG, GmbH, Genossenschaft, KmAG, SICAV und SICAF sind juristische Personen; bei Kollektiv- und Kommanditgesellschaften haften natürliche Personen persönlich. Die Regel `firma` ergibt darum bei juristischen Personen nie GELB oder ORANGE (Art. 5 lit. a DSG), ebenso bei der KmGK, deren unbeschränkt haftende Gesellschafter Aktiengesellschaften sind (Art. 98 Abs. 2 KAG). ROT ergibt sie, und als Identifikator für Art. 5 lit. c DSG zählt sie, nur wenn die Pflicht der Rolle «ein Geheimnis» schützt, gleich wessen: Art. 320 und 321 StGB, Art. 47 BankG, Art. 69 FINIG; das Bundesgericht anonymisiert auch juristische Privatpersonen (BGer-Anonymisierungsregeln Art. 1 Abs. 1). `professional` (Art. 62 DSG) schützt nur geheime Personendaten: Eine Firma bleibt dann ohne Fund. Die Normen des DSG und die Zeile «zugleich besonders schützenswert» nennt ein Fund nur bei einer natürlichen Person; bei `student` nennt der Fund einer Firma nur Art. 321 Ziff. 1 Abs. 2 StGB. Kollektiv- und Kommanditgesellschaften zählen wie eine Personenangabe: ohne direkten Identifikator HINWEIS, mit Identifikator GELB, im Geheimnis-Kontext ROT, mit `professional` nur zusammen mit einem Identifikator. Ob eine Firma ein Fabrikations- oder Geschäftsgeheimnis hat (Art. 162 StGB), entscheidet promptscan nicht.

### 4.4 Wann es keine Stufe gibt

- **Ohne direkten Identifikator** im Prompt (Name, Adresse, AHV-Nummer, IBAN, Telefon, E-Mail) erhalten Inhalte wie eine Diagnose keine DSG-Stufe. Hat der Satz einen Personenbezug, etwa «mein Mandant», erscheint ein HINWEIS. Ob der Kontext eine Person bestimmbar macht, beurteilt promptscan nicht (Grundsatz 7).
- **Ein Identifikator irgendwo im Prompt genügt**; promptscan prüft nicht, ob er dieselbe Person betrifft. Auch die eigene Unterschrift oder Kontaktzeile zählt: «Was gilt bei Arbeitsunfähigkeit?» wird ORANGE, wenn «Erika Beispiel, Seestrasse 12, 8002 Zürich» darunter steht, und mit einer Geheimhaltungspflicht wird auch die Kontaktzeile selbst ROT. `explain gelb`, `orange` und `rot` nennen diese Grenze.
- **`--target local`**: gilt für ein Modell, das auf diesem Rechner läuft, ohne Synchronisierung in eine Cloud. Läuft es so, wird nichts übermittelt oder zugänglich gemacht: keine Bekanntgabe (Art. 5 lit. e DSG) und kein Offenbaren nach den Normen der Rolle. Jeder Fund der Stufe GELB, ORANGE oder ROT wird zum HINWEIS und nennt die Stufe, die er sonst hätte. Bearbeitet werden die Daten trotzdem (Art. 5 lit. d DSG): Die Grundsätze von Art. 6 DSG gelten weiter, etwa die Verhältnismässigkeit (Art. 6 Abs. 2 DSG). Ob das Modell wirklich lokal läuft, prüft promptscan nicht. Steht `target = "local"` in der Konfigurationsdatei, gilt das für jeden Prompt, auch für einen, der doch in eine Cloud geht. Woher das Ziel stammt, nennt die Kopfzeile nicht; der Bericht trägt die Herkunft des Ziels nicht. Der Bericht stellt die Folge unter die Bedingung («Läuft das Modell lokal: keine Bekanntgabe …») und fragt unter `Klären`: «Läuft das Modell wirklich hier, ohne Synchronisierung in eine Cloud?» (`explain hinweis`).

### 4.5 Was promptscan nicht entscheidet

`explain <stufe>` nennt die Punkte der Stufe mit vollem Wortlaut und der Literatur; `explain hinweis` und jedes `explain <regel>` nennen alle acht Punkte, weil ein Fund mit `--role` ROT werden kann, `explain <regel>` je in einer Kurzform mit Verweis auf `explain <stufe>` für Wortlaut und Literatur. Im JSON stehen die vollen Texte in `nicht_entschieden`. Wortlaut in `src/ausgabe.rs` (`nicht_entschieden`).

- Ob eine Persönlichkeitsverletzung vorliegt (Art. 30 DSG) und ob sie gerechtfertigt ist: durch Einwilligung der betroffenen Person, ein überwiegendes privates oder öffentliches Interesse oder Gesetz (Art. 31 Abs. 1 DSG; gilt für private Personen). Bei GELB, ORANGE und HINWEIS.
- Ob eine Ausnahme greift: nach Art. 321 Ziff. 2 StGB die Einwilligung der berechtigten Person oder die schriftliche Bewilligung der vorgesetzten Behörde oder Aufsichtsbehörde, nach Art. 320 Ziff. 2 StGB die schriftliche Einwilligung der vorgesetzten Behörde; bei Art. 47 BankG, Art. 69 FINIG und Art. 62 DSG, ob die Person eingewilligt hat, die das Geheimnis betrifft. Bei ROT und HINWEIS.
- Ob die Norm der Rolle auch den KI-Anbieter erfasst: als Hilfsperson (Art. 321 Ziff. 1 StGB, Art. 320 Ziff. 1 StGB), als Beauftragten (Art. 47 Abs. 1 lit. a BankG, Art. 69 Abs. 1 lit. a FINIG) oder als Person, die für Sie tätig ist (Art. 62 Abs. 2 DSG). Wann ein Anbieter dazu zählt und ob die Weitergabe an ihn dann ein Offenbaren ist, sagt der Wortlaut nicht. Anwältinnen und Anwälte sorgen für die Wahrung des Berufsgeheimnisses durch ihre Hilfspersonen (Art. 13 Abs. 2 BGFA). promptscan meldet ROT und entscheidet diese Fragen nicht. Literatur zur Anwaltschaft, noch nicht gelesen (10.2): Chappuis/Alberini, Secret professionnel de l’avocat et solutions cloud, Anwaltsrevue 8/2017, 337 ff.; Schwarzenegger/Thouvenin/Stiller/George, Nutzung von Cloud-Diensten durch Anwältinnen und Anwälte, Anwaltsrevue 1/2019, 25 ff. Bei ROT und HINWEIS.
- Ob der KI-Anbieter Auftragsbearbeiter oder Dritter ist (4.2, «Auftragsbearbeitung»; Art. 5 lit. j und k, Art. 9 und Art. 61 lit. b DSG). Bei jeder Stufe.
- Ob Daten ins Ausland gehen und wohin. Art. 16 Abs. 1 DSG lässt das zu, wenn der Bundesrat festgestellt hat, dass die Gesetzgebung des Staates einen angemessenen Schutz gewährleistet (Liste: Anhang 1 DSV); für die USA nur bei Organisationen, die nach den Grundsätzen des Datenschutzrahmens zwischen der Schweiz und den USA zertifiziert sind (Anhang 1 Ziff. 44 DSV). Sonst muss ein geeigneter Datenschutz gewährleistet sein, etwa durch Standarddatenschutzklauseln, die der EDÖB vorgängig genehmigt, ausgestellt oder anerkannt hat (Art. 16 Abs. 2 lit. d DSG), oder eine Ausnahme greifen, etwa die ausdrückliche Einwilligung (Art. 17 Abs. 1 lit. a DSG); Busse auf Antrag nach Art. 61 lit. a DSG. Bei jeder Stufe.
- Welches Datenschutzrecht gilt (4.2, «Geltungsbereich»). Bei GELB, ORANGE und HINWEIS; bei ROT steht der Geltungsbereich unter «Grund».
- Ob eine Angabe überhaupt geheim ist. Bei ROT und HINWEIS.
- Geschäfts- und Fabrikationsgeheimnisse und vertragliche Schweigepflichten (Art. 162 StGB; für Arbeitnehmende Art. 321a Abs. 4 OR) prüft promptscan nicht. Wenige oder keine Funde heissen nicht, dass der Prompt nichts Geheimes enthält. Bei jeder Stufe.

## 5. Regeln v0.1

### 5.1 Klasse A — formal prüfbar

| Regel | erkennt | Methode | Quelle der Methode |
|---|---|---|---|
| ahv | AHV-Nummer | 756 und 10 Ziffern, gruppiert 3.4.4.2 (auch mit einem Umbruch nach einem Punkt), 3-4-4-2 mit Leerzeichen oder ohne Trenner; Prüfziffer nach EAN-13 | BSV, WL VA/IK Rz 1201 f. und Anhang 7; GS1 General Specifications, Ziffer 7.9.1; eCH-0044 (ohne Trenner) |
| iban | IBAN aus CH und LI | 21 Zeichen, ohne Trenner oder in Vierergruppen mit 1 bis 3 Leerraumzeichen, darunter höchstens ein Umbruch; Prüfung nach ISO 7064 MOD 97-10 | ISO 13616-1:2020, Ziffer 6; ISO 7064; SIX (Aufbau für CH und LI, Viererblöcke) |
| tel | Schweizer Telefonnummer | +41, 0041 oder 0, dann 9 Ziffern nach dem Schweizer Nummernplan, nur Festnetz- und Mobilbereiche; Gruppen nach TAV und E.123, Bindestrich auch U+2010 oder U+2011; zehn Ziffern direkt nach einem Nummernwort («Rechnung Nr. 0791234567») gelten nicht als Telefonnummer, ausser ein blosses Wort ohne «Nummer» und ohne Kürzel steht nach «zur», «zum», «sur (la)» oder «sulla» und nennt nur das Thema («Bei Fragen zur Rechnung 0791234567 anrufen») | BAKOM, TAV E.164 (SR 784.101.113/2.2), Ausgabe 8; ITU-T E.123; Nummernwörter: MWSTG Art. 26, VVG Art. 11, AHVV Art. 71 Abs. 3, Duden, Larousse, Treccani |
| mail | E-Mail-Adresse | addr-spec, als Teilmenge dokumentiert; auch «(at)», «[at]» und «[dot]»; « at » nur mit « dot » und mit klein geschriebener Domain oder Punkt, Ziffer, Unterstrich im lokalen Teil; keine Dateinamen («logo@2x.png», «daten@2024.json») und keine Rollenadressen | RFC 5322, Abschnitt 3.4.1; RFC 5321, 6531, 6532, 1123, 5890; RFC 2142 (Rollenadressen) |

Unsichtbare Formatzeichen wie U+00AD oder U+200B werden in allen vier Regeln übergangen.

Die Eigenschaften der Prüfziffern halten Eigenschaftstests fest (Abschnitt 7). EAN-13 mit den Gewichten 1 und 3 erkennt jeden Einzelziffernfehler. Vertauschte Nachbarziffern erkennt es nur, wenn sie sich nicht um 5 unterscheiden. MOD 97-10 erkennt jede ersetzte Ziffer und jede Vertauschung zweier benachbarter Ziffern. Bei Buchstaben in der Kontonummer gibt es Ausnahmen: Grossbuchstabe statt Kleinbuchstabe, manche Ziffer statt eines Buchstabens und die Vertauschung der Paare 1B, 2M und 3X, die ausgeschrieben gleich bleiben. Fehlalarme entstehen nur aus Zeichenfolgen, deren Prüfziffer zufällig stimmt. Das trifft etwa jede zehnte zufällige 756-Nummer und etwa jede 97. zufällige CH-Zeichenfolge. Den Rest müssen Format und Präfix tragen. Wie gut das auf echten Prompts gelingt, ist nicht gemessen (Abschnitt 8).

**Zusatz zu `ahv`:** Stehen mehrere AHV-Nummern mit Namen oder in Tabellenform im Prompt, erscheint zusätzlich ein HINWEIS auf Art. 153b f. AHVG (Regel `ahv-sammlung`). Die Verbindung der Nummer mit Personendaten in strukturierter Sammlung gilt dort als systematische Verwendung, und die ist nur den Berechtigten nach Art. 153c AHVG erlaubt. Gezählt werden Zeilen mit gültiger AHV-Nummer, die einen Namen enthalten oder Zeilen einer Tabelle sind; ab zwei solchen Zeilen erscheint der Hinweis. Eine Tabellenzeile trennt Zellen mit «,» «;» Tab oder «|», und die Nummer steht allein in ihrer Zelle, höchstens mit einer Bezeichnung davor («AHV: 756…»). Keine Zelle der Zeile ist ein Satz, sie endet also nicht auf «?» oder «!» und nicht auf «.» nach mindestens drei Wörtern: «756…, das ist meine Nummer.» und «756…, gültig?» sind keine Tabellenzeilen. Sie zählt, wenn eine zweite solche Zeile dieselbe Folge von Trennern hat oder über ihr eine Kopfzeile mit «AHV» oder «AVS» und einem Trenner steht. So trägt auch eine Liste den Hinweis, deren Namen die Regel `name` nicht erkennt. Der Zusatz nennt Namen nur, wenn sie erkannt sind: «2 Zeilen mit AHV-Nummer und Name», «2 Zeilen mit AHV-Nummer in Tabellenform», «2 Zeilen mit AHV-Nummer, 1 davon mit Name».

### 5.2 Klasse B — Wortliste mit Quelle und Satzfenster

Das Fenster ist der Satz nach Unicode UAX #29, mit den Anpassungen aus Abschnitt 6. Ein einzelner Zeilenumbruch mitten in einem Absatz, wie ihn hart umbrochene E-Mails und aus einem PDF kopierter Text enthalten, ist keine Satzgrenze. Eine Grenze bleibt er, wenn die Zeile danach leer ist, wenn die Zeile davor auf `.`, `!`, `?` oder `:` endet (ausser auf eine Abkürzung oder Ordinalzahl, die verbindet) oder wenn eine der beiden Zeilen wie ein Listenpunkt beginnt (`-`, `•`, `1.`, `a)` und ähnliche); das trifft auch ein Datum am Zeilenanfang («geboren am⏎12. März 1985» bleibt getrennt). Über einen solchen weichen Umbruch hinweg gelten auch Begriffe aus mehreren Wörtern («psychischen⏎Störung»), und ein Trennstrich zwischen zwei Kleinbuchstaben wird für den Vergleich zusammengezogen («arbeits-⏎unfähig»); nach einem anderen Bindestrich fällt nur der Umbruch weg («AHV-⏎Nummer»). Grenze: Überschriften, Adressblöcke und Unterschriften ohne Satzzeichen am Zeilenende werden mit der nächsten Zeile zu einem Satz, ebenso eine Anrede («Guten Tag», «Sehr geehrte Damen und Herren,»), ein Betreff oder ein Name. Ob ein Wort am Satzanfang steht, ist darum im Satzstück zwischen zwei weichen Umbrüchen zu prüfen (`Text::satzanfang_bei`, `Text::satzstueck_bei`); dort gilt der Anfang einer Zeile wie vor dieser Regel als möglicher Satzanfang. So prüfen die Selbstbezeichnungen der Regel `kontext`, die einen Satzanfang verlangen («Als Anwältin …», «En tant qu'avocate, je», «Sono avvocata»), und ihre Prüfung auf eine Frage, das gross geschriebene Partizip der Regel `geburtsdatum` («Geboren am», «Née le», «Nata il») und die Aufforderungen der Regel `fundstellen` am Satzanfang («Cite les arrêts», «Liste die BGE …»): «Guten Tag⏎Als Anwältin vertrete ich …» ergibt den Geheimnis-Kontext wie einzeilig, und in «Seit 2010 bin ich Ärztin⏎Darf ich …?» zählt das Fragezeichen der nächsten Zeile nicht für «bin ich Ärztin». Grenze wie vor dieser Regel: «Sie arbeitet⏎als Anwältin, habe ich gehört» gilt als Selbstbezeichnung, und in «Ab wann bin ich Ärztin, wenn⏎ich … bestanden habe?» zählt das Fragezeichen nicht; einzeilig ergeben beide keinen Kontext. Ob der Satz das bessere Fenster ist als der ganze Prompt, ist nicht gemessen (Abschnitt 8).

| Regel | erkennt | Methode | Quellen |
|---|---|---|---|
| name | Name einer natürlichen Person | Anrede oder Titel, Verfahrensrolle oder Vorname aus amtlicher Statistik, gefolgt von Wörtern mit Grossbuchstaben; nach einer Anrede oder Rolle auch Initialen vor einem Nachnamen aus dem Register, der kein Satzwort in der Sprache des Auslösers ist («der Beschwerdeführer A. Meier», nicht «der Beschwerdeführerin A. Daher ist …»); in Listen (Zeilenanfang, Aufzählungszeichen, Trenner «,» «;» Tab «|», Doppelpunkt) ein Paar aus Nachname und Vorname des Registers in beiden Reihenfolgen («Müller Andreas, Lager», «Muster;Hans»), ein Nachname in Versalien vor einem Vornamen überall («KELLER Sabine»); in einer Zelle nur mit eindeutigem Nachnamen, wobei Grussformeln, Wochentage und Alltagswörter des Registers als mehrdeutig gelten («Liebe Anna, Lieber Peter,»), und am Zeilenanfang nur, wenn die nächste Zelle kein Satz ist («Bonjour Pierre, Merci pour ton message.»); auch über einen weichen Umbruch und Formatzeichen hinweg, zwischen Vor- und Nachnamen aber nur nach Anrede, Titel, Verfahrensrolle oder Possessivrahmen («Meine Patientin Erika⏎Beispiel»; nicht «Liebe Erika⏎Danke …»); der Artikel «Le», im Register zugleich Vorname, löst am Satzanfang (ein weicher Umbruch beginnt keinen Satz) und vor einer Amtsbezeichnung («Le Grand Conseil», «Le Maire») keinen Namen aus, nach der Amtsbezeichnung zählt ein Vorname mit Nachnamen wie nach einer Anrede («Le Greffier Marie Exemple») | Vor- und Nachnamen: BFS (opendata.swiss); Verfahrensrollen: amtliche Texte von StPO, ZPO und BGG in DE, FR, IT, dazu VwVG, ZGB, MStP und VStrR in DE, je Eintrag mit Artikel; Amtsbezeichnungen nach «Le»: BV, StPO und BGG in FR, Verfassungen der Kantone Waadt und Jura |
| firma | juristische Person | Rechtsformzusatz; «AG» ist Kantonskürzel, keine Rechtsform, wenn der ganze Name davor (ohne Postleitzahl) eine Aargauer Gemeinde oder eine Ortschaft «… AG» ist («von Muri AG», «5610 Wohlen AG»); «Garage Wohlen AG» bleibt eine Firma | Art. 950 Abs. 1 Satz 2 OR (die Firma muss die Rechtsform angeben); zulässige Abkürzungen legt der Bundesrat fest (Art. 950 Abs. 2 OR, Verordnungsstelle zu bestimmen); Aargauer Gemeinden: BFS, Amtliches Gemeindeverzeichnis, Stand 06.11.2026 |
| adresse | Postadresse | Strasse, Nummer, PLZ, Ort; PLZ und Ort gegen das amtliche Verzeichnis; Endungen der Strassentypen aus dem amtlichen Strassenverzeichnis abgeleitet, Abdeckung gemessen. PLZ und Ort allein meldet sie nicht (Art. 4 Abs. 1 lit. e der Anonymisierungsregeln); eine Adresse direkt nach einer erkannten juristischen Person gilt als Adresse der Firma (`explain adresse`) | swisstopo: Amtliches Ortschaftenverzeichnis, Amtliches Strassenverzeichnis (Nutzungsbedingungen: THIRD-PARTY-NOTICES.md, 7.2) |
| wohnort | Wohnort einer Person | «wohnhaft in», «domicilié à», «domiciliato a» und eine Gemeinde, auch nach einem weichen Umbruch («wohnhaft in⏎Winterthur»); ohne Auslösewort nichts (5.5) | BFS: Amtliches Gemeindeverzeichnis |
| geburtsdatum | Geburtsdatum | Datum mit Auslöser: geb., geboren am, Geburtsdatum, né le, née le, date de naissance, nato il, nata il, data di nascita, dazu Feldnamen aus Formularen («Geb.-Datum», «date naiss.», «data nascita») und nach «/» der Zusatz «Alter», «âge» oder «età»; zwischen Auslöser und Datum zählt ein weicher Umbruch als Leerzeichen | Datumsformen: CLDR 49, Schreibweisungen der Bundeskanzlei, RFC 3339; Auslöser: ZGB, ZStV, Ausweisverordnung, Duden; die Kurzformen der Feldnamen sind ohne eigene Fundstelle (die Einträge sagen es); Zusatz: BV Art. 8 Abs. 2 (DE, FR, IT) |

**Ausnahmen** folgen den Anonymisierungsregeln des Bundesgerichts, Art. 4 Abs. 1. Nicht markiert werden Behörden und öffentlich-rechtliche Körperschaften (lit. a), Gemeinden (lit. b), Ortsnamen (lit. e) und die Rechtsvertreter der Parteien (lit. g). Diese Regeln sind die Schweizer Referenz dafür, was einen Rechtstext identifizierend macht. Anonymisiert werden natürliche und juristische Privatpersonen (Art. 1 Abs. 1). Adressen werden «in jedem Fall weggestrichen» (Art. 7 Abs. 3).

Lit. g hängt an der Rolle im Verfahren, nicht am Beruf: Eine Anwältin kann selbst Patientin, Angestellte oder Partei sein. Ein Name bleibt darum nur unmarkiert, wenn der Satz die Person als Vertretung einer Partei zeigt: durch ein Wort, das die Vertretung schon bezeichnet («Verteidigerin», «Rechtsbeistand», «défenseur», «conseil juridique», «patrocinatore», «mandataires professionnellement qualifiés»), oder durch einen Anwaltstitel oder -beruf («RA», «Rechtsanwältin», «Me», «avocat», «avv.») mit einem Vertretungsmerkmal (`name-vertretung.toml`): ein Partizip davor («vertreten durch», «représenté par», «rappresentato dall'»), ein Possessivpronomen davor («mon avocat», «Ihr Anwalt», «il suo avvocato»), eine Partei danach («avocat de la partie adverse», «Anwalt der Gegenpartei»), ein Verb der Vertretung mit einer Partei danach («plaide pour la recourante», «vertritt die Klägerin») oder eine Apposition zu einer solchen Bezeichnung («votre avocate, Me …»). Ohne Vertretungsmerkmal wirkt der Anwaltstitel wie ein Titel, und der Name wird gemeldet («Mon patient, Me Laurent Dupont, souffre …»). Sicher gemeldet wird er, wenn der Satz die Person zur Betroffenen macht: Ein Possessivpronomen zeigt die Vertretung nur, wenn der Satz keine Angabe nach Art. 5 lit. c DSG enthält («Mon avocat, Me Jean Dupont, a écrit.» bleibt unmarkiert, «Mon avocat, Me Jean Dupont, est en incapacité de travail.» und «Jean Dupont, son avocat, est en incapacité de travail.» melden den Namen), und ein Verb im übertragenen Gebrauch ohne Partei danach zählt nicht («vertritt die Auffassung», «plaide coupable», «représente un danger»). «conseil» heisst auch Berater oder Rat und zeigt die Vertretung nur mit einer Partei danach oder einem Partizip davor («Me X, conseil de Y», «assistée de son conseil»); «mandataire» umfasst auch die Vorsorgebeauftragte nach Art. 360 ZGB und zeigt sie nur mit «partie» oder einer Verfahrensrolle danach («le mandataire de la partie»). «avocat» und «avvocato» lösen allein nichts aus, denn danach folgt oft ein Adjektiv oder Fachgebiet («un avocat Genevois»).

**Deutsch schreibt alle Substantive gross.** Ein grossgeschriebenes Wort nach einer Rolle genügt darum nicht als Namenshinweis. Deshalb braucht es die amtlichen Namenslisten. Nach einem Vornamen gilt dagegen jedes grossgeschriebene Wort als Nachname; `explain name` nennt diese Grenze («Peter Geld geben»).

### 5.3 Besonders schützenswerte Personendaten — je eine Regel pro Ziffer von Art. 5 lit. c DSG

| Regel | Ziffer | Wortliste aus | Stand |
|---|---|---|---|
| art5c-1 | religiöse, weltanschauliche, politische oder gewerkschaftliche Ansichten oder Tätigkeiten | BV Art. 8 und 15, ZGB Art. 303, AsylG Art. 3, NDG Art. 5, OR Art. 336, IAO-Übereinkommen Nr. 98 Art. 1 («Gewerkschaft», FR «syndical», IT «sindacali» auch am Wortanfang), UNO-Pakt I Art. 8 Abs. 1 Bst. a (IT «sindacato»); Konfessionen nach der Religionstabelle und der Seite «Religionen» des BFS («Muslim», «muslimisch»); «Jude», «Jüdin», «Konfession» und IT «ebreo» nach Duden und Treccani; «Jude» zählt nicht in einem Namen, den die Regel `name` erkennt (Merkmal «name»); Ausnahmen «syndicat de communes», «syndicat de remaniement» und nach Treccani «sindacato di legittimità», «sindacato di merito», «sindacato di costituzionalità», «sindacato azionario», «sindacato di blocco», «ordinanza sindacale», «deliberazione sindacale». Parteinamen fehlen; «collegio sindacale» und «patto di sindacato» zählen (keine Fundstelle für eine Ausnahme) | Wortlaute und Zitate geprüft 2026-11-06 |
| art5c-2 | Gesundheit, Intimsphäre, Rasse oder Ethnie | ATSG Art. 3 bis 8, UVG Art. 16, KVG Art. 1a, 31 und 42, KLV (u. a. Art. 7 und 11c: «Diabetiker», «diabetisch»), EpG Art. 5 und 33 («krank», «malades», «malate»), Krebsregistrierungsgesetz, BetmG Art. 3b (FR «addiction»), IVG, IVV Art. 31 («IV-Rente», «rente AI», «rendita AI»), ZGB Art. 390, BV Art. 8, BehiG, ArG, StGB Art. 115, 118 und 261bis, MStG Art. 109 («mise enceinte»), StPO Art. 195, SVG Art. 16d, AsylG Art. 99a («ethnie»), Übereinkommen gegen Rassendiskriminierung (SR 0.104) Art. 4 («origine etnica»); Ethnie nach den Botschaften zum DSG (BBl 2017 7020, FF 2017 6640, FF 2017 6012); «Suizid», «Depression» und ihre Gegenstücke nach BAG; «krankgeschrieben», «Burn-out» und FR «enceinte» nach Duden und Larousse. «Krankheit», «Erkrankung» und «Diagnose» zählen auch am Wortende, «Suizid» und «Selbstmord» am Wortanfang. Ausnahmen sind nur diese Namen von Versicherungen, Versicherern und Kassen: «Kranken-» (BV Art. 117), «assurance-maladie», «caisse-maladie», «assureur-maladie» (KVV Art. 11), «assicurazione malattie», «assicurazione malattia» (KVAG Art. 7), «assicuratore malattia» (IVG Art. 57), «assicuratore-malattie» (KVV Art. 11) und «cassa malati»; andere Namen wie «assurance perte de gain maladie» zählen. Ausnahmen sind auch «dépendance économique» und «dipendenza economica» (OR Art. 728). FR «cancer» zählt nicht in einem Namen, den die Regel `name` erkennt; FR «enceinte de» zählt nur mit Dauer. ICD-10-Kodestruktur (Buchstabe, zwei Ziffern, optional Punkt und Ziffer), nur mit Auslöser «Diagnose», «diagnostic», «diagnosi» oder «ICD» im selben Satz, auch über einen weichen Umbruch («Diagnose⏎F32.1») | Wortlaute und Zitate geprüft 2026-11-06 |
| art5c-3 | genetische Daten | GUMG Art. 3, DNA-Profil-Gesetz Art. 2, DSG-Wortlaut; DNA-Profil nach der Botschaft (BBl 2017 7020); Ausnahme ist der Erlasstitel «DNA-Profil-Gesetz» | Wortlaute geprüft 2026-11-06 |
| art5c-4 | biometrische Daten | DSG Art. 5 Bst. c Ziff. 4; Ausweisgesetz Art. 2 Abs. 2bis («Fingerabdrücke», «Gesichtsbild», «immagine del viso»); FR «images faciales» aus der Botschaft (FF 2017 6641), weil das Ausweisgesetz auf Französisch «photographie» schreibt; gewöhnliche Fotos fehlen (BBl 2017 7020) | Wortlaute geprüft (Belegprüfung 2026-11-06) |
| art5c-5 | verwaltungs- und strafrechtliche Verfolgungen oder Sanktionen | StPO (u. a. Art. 111, 112 («Strafuntersuchung», «enquête pénale», «inchiesta penale»), 115, 118, 195, 220 und 352), StGB Art. 30, 34, 40 und 66a, OBG, StReG, BGFA Art. 16, SVG Art. 16, 16a und 16c, BPG Art. 34c, URG Art. 77i, DSG; «vorbestraft» nach Duden. Mehrdeutige FR- und IT-Wörter nur in Wendungen («plainte pénale», «porter plainte», «déposer plainte», «dénonciation pénale», «sporto denuncia», «retrait du permis de conduire»); Ausnahmen sind die Erlasstitel «Code de procédure pénale», «Procédure pénale applicable aux mineurs» (SR 312.1) und «Procédure pénale militaire» (SR 322.1, nur gross geschrieben) sowie die Beschwerde an die Aufsichtsbehörde («plainte à l'autorité de surveillance», SchKG Art. 17; «plainte auprès de l'autorité de surveillance», ZGB Art. 84) | Wortlaute und Zitate geprüft 2026-11-06 |
| art5c-6 | Massnahmen der sozialen Hilfe | ZGB Art. 311, 312, 314b, 327a, 327b, 390, 393 und 394 (Beistandschaft, fürsorgerische Unterbringung, Entziehung der elterlichen Sorge, Vormund, Vormundschaft; FR «tuteur» und IT «tutore» nur in Wendungen); AIG Art. 62 (Sozialhilfe); ZUG Art. 3; DSG-Wortlaut («sozialen Hilfe», «assistenza sociale») | Abgrenzung: Leistungen der Sozialversicherungen meldet diese Regel nicht (BBl 1988 II 446; SR 851.1 Art. 3 Abs. 2 Bst. a); Invalidität und IV-Rente meldet art5c-2 |

Alle Wortlisten stammen aus den amtlichen Fassungen in Deutsch, Französisch und Italienisch, den Amtssprachen des Bundes (Art. 70 Abs. 1 BV), und aus den Botschaften zum DSG. Es wird nicht selbst übersetzt: Jeder Eintrag hat eine Fundstelle in seiner eigenen Sprache (treffen gleich geschriebene Einträge mehrerer Sprachen dieselbe Stelle, etwa «HIV» deutsch und italienisch, trägt der Treffer den Beleg in der Sprache des Textes; sie wird an häufigen Funktionswörtern geschätzt, einer Liste im Code ohne Quelle, und ist sie unklar, gilt der frühere Eintrag, denn ein Fund trägt einen Beleg, siehe `explain art5c-2`), und jede Ziffer löst in allen drei Sprachen schon mit dem Wortlaut von Art. 5 Bst. c DSG aus. Alltagswörter, die in keinem durchsuchten Erlass stehen, sind mit Seiten von BFS oder BAG belegt, sonst mit Duden, Larousse oder Treccani; neue Einträge tragen das wörtliche Zitat im Feld `quelle`. Mehrdeutige Wörter stehen, soweit sich eine Fundstelle fand, nur in Wendungen oder mit Ausnahmen (Merkmal «ausnahme»), jeweils mit Gegenbeispielen in den Tests; wo sich keine fand, nennt `explain` die Grenze, und Tests halten fest, dass der übertragene Gebrauch zählt («Sucht nach Anerkennung», «état de santé financière»). Begriffe, die auch Namen sind («Jude», FR «cancer»), zählen nicht in einem Namen, den die Regel `name` erkennt (Merkmal «name»). Rätoromanisch fehlt in v0.1: Es ist Amtssprache nur im Verkehr mit Personen rätoromanischer Sprache, und amtliche Texte gibt es nur für ausgewählte Erlasse.

Die Regeln `art5c-*` markieren nur, wenn der Prompt einen direkten Identifikator enthält (4.4).

### 5.4 Hinweise

| Regel | erkennt | Grundlage |
|---|---|---|
| vermerk | Klassifizierungsvermerk «intern», «vertraulich» oder «geheim» als Kopf- oder Einzelzeile, dazu die Begriffe der amtlichen FR- und IT-Fassung | Bezug: Art. 13 ISG, Art. 2 Abs. 1 und 2 ISG, Art. 3 ISG |
| fundstellen | Bitte um Urteile, Fundstellen oder Literatur, in DE, FR, IT | Bezug: Art. 12 lit. a BGFA; Erkennung: Art. 27 Abs. 1 BGG, Art. 57 Abs. 1 BGerR; Dahl/Magesh/Suzgun/Ho, «Large Legal Fictions», Journal of Legal Analysis 16 (2024), doi:10.1093/jla/laae003 |
| ahv-sammlung | mindestens zwei Zeilen mit je einer gültigen AHV-Nummer und einem Namen | Bezug: Art. 153b AHVG, Art. 153c Abs. 1 AHVG |

«Bezug» ist die Norm, an deren Rechtsfolge oder Pflicht der Hinweis anknüpft; «Erkennung» die Norm, nach der die Regel etwas erkennt. `promptscan rules` trennt beides über `normen::ist_bezug`: zum Bezug gehören die Normen von DSG, StGB, BGFA, BankG, FINIG, FinfraG, StPO, AHVG und ISG (`normen::BEZUG_ERLASSE`) und die Normen des OR, die eine Pflicht oder die Bearbeitung von Daten regeln (`normen::BEZUG_OR`: Art. 321a Abs. 4, 328b, 342 Abs. 1 lit. a und 730b Abs. 2 OR); alle übrigen sind Grundlage der Erkennung. `ahv-sammlung` sagt im Bericht: «Systematisch verwenden darf die AHV-Nummer ausserhalb der AHV nur, wer nach Art. 153c AHVG berechtigt ist.» Die Hinweise `kontext` und `personenbezug` beschreiben 4.3 und 4.4.

Zu `fundstellen`: Bei überprüfbaren Fragen zu Urteilen der US-Bundesgerichte machten die Modelle in mindestens 58 % (GPT-4) bis 88 % (Llama 2) der Fälle falsche Angaben, bei weniger prominenten Gerichten öfter. Dass es für Schweizer Recht ähnlich oder schlechter steht, ist eine Vermutung, und `explain` weist sie als solche aus. promptscan prüft selbst keine Fundstellen.

### 5.5 Bewusst nicht in v0.1

- Anonymisieren oder Schwärzen, Re-Identifikationsrisiko aus dem Kontext, Gerichtsentscheide, Geschäftsnummern (Grundsatz 7).
- Kantonale Besonderheiten.
- Ein Wohnort ohne Auslösewort («Die Patientin aus Ossingen»): Die Regel `wohnort` kann Ort und Person dann nicht sicher verbinden. Den Wohnort einer Partei anonymisiert das Bundesgericht in der Regel, auch als Gemeindename (Art. 7 Abs. 1 der Anonymisierungsregeln). «aus» vor einem Gemeindenamen ist darum kein Auslöser.
- Rätoromanisch.
- KI-gestützte Erkennung. Alles ist regelbasiert und deterministisch: Dieselbe Eingabe ergibt immer dieselbe Ausgabe.

## 6. Technik: Rust

- **Nachweisbar offline.** Das Ergebnis ist eine Binärdatei ohne Laufzeitumgebung. `tests/offline.rs` prüft bei jedem `cargo test`:
  - im Quelltext (`src/`, `build.rs`, `examples/`): kein `std::net` und kein Netzwerkmodul von windows-sys (`Networking`, `WinSock`), kein `std::process::Command`, keine Funktion zum Programmstart oder Nachladen, weder von Windows (CreateProcess, NtCreateUserProcess und die Zw…-Formen, ShellExecute, WinExec, LoadModule, LoadLibrary, GetProcAddress u. a.) noch der C-Laufzeit (`system`, `_popen`, `_exec…`, `_spawn…` und die `_w…`-Formen), kein `#[link]` und kein `link!` (windows-link) ausser für kernel32, auch mit Leerraum vor der Klammer;
  - im Quelltext des Programms (`src/`): Anlegen, Schreiben, Kopieren, Umbenennen oder Löschen von Dateien (`fs::write`, `File::create`, `OpenOptions` u. a.) nur über die Schreibhilfe `datei_schreiben`, und die rufen allein die zwei Befehle auf: `konfiguration_schreiben` (`promptscan config --role …` bzw. `--lang …`; nur die Schlüssel role und lang; legt dazu den Ordner der Datei an) und `abschliessen` (`--output`). Testmodule am Dateiende zählen nicht. Eine weitere Stelle oder ein weiterer Aufrufer schlägt an, eine entfallene Stelle auch. `build.rs` und `examples/drittvermerke.rs` schreiben beim Bauen bzw. Erzeugen der Vermerke; sie gehören nicht zur Programmdatei. Fremden Code erfasst diese Prüfung nicht;
  - in Cargo.lock: keine Netzwerkbibliothek; in Cargo.toml: keine Netzwerk-Features von windows-sys oder windows (`Win32_Networking…`, `Win32_NetworkManagement…`), auch nicht unter `[target.…]` oder `[features]`;
  - in der Importtabelle der Programmdatei (PE, normale und verzögerte Importe): nur kernel32, ntdll, msvcrt, bcryptprimitives und api-ms-win-core-synch-l1-2-0; keine Netzwerk-DLL (ws2_32, wininet, winhttp, urlmon u. a.), keine Funktion zum Programmstart (auch keine der 36 Startfunktionen von msvcrt), zum Herunterladen oder Nachladen. Streng gilt das für jeden optimierten Bau und für die Datei in `PROMPTSCAN_EXE`; so prüft `tools/freigabe.ps1` die Release-Datei. Der Testbau ohne LTO enthält ungenutzten Netzwerk- und Prozesscode der Standardbibliothek; dort sind genau diese Funktionen namentlich zugelassen: 25 aus WS2_32.dll, GetUserProfileDirectoryW aus USERENV.dll und CreateProcessW. Jede andere Funktion schlägt auch im Testbau an. Begründung je DLL im Kopf von `tests/offline.rs`.

  Jede Prüfung hat eine Gegenprobe, die zeigt, dass sie anschlägt. Bekannte Lücken in den Abhängigkeiten prüft cargo-audit in der CI. «promptscan sendet nichts und schreibt nur auf Befehl» ist damit prüfbar, nicht bloss versprochen.
- **Lineare Laufzeit.** Die `regex`-Bibliothek garantiert lineare Laufzeit. Beliebig eingefügter Text kann kein katastrophales Backtracking auslösen. Die eigenen Schleifen bleiben auch bei Wiederholungen linear oder fast linear: Die Satzgrenzen prüfen am Ende jedes Abschnitts nur dieses Ende, nicht den ganzen schon verbundenen Satz, und eine Wortliste wählt unter k Treffern die überlappungsfreien in O(k log k). Drei Tests verlangen für die achtfache Eingabe höchstens die zwanzigfache Zeit und 20 ms; quadratischer Aufwand bräuchte 64-mal so lange (`text::tests::satzgrenzen_in_linearer_zeit`, `lexikon::tests::viele_treffer_in_fast_linearer_zeit`, `lexikon::tests::finden_in_fast_linearer_zeit`). Die beiden Wortlisten-Tests schlagen mit der früheren Auswahl fehl; die frühere Satzgrenzen-Schleife brauchte für «Art. » 8000-mal wiederholt rund 30-mal so lange wie 1000-mal. Gemessen mit dem ganzen Programm (`--format json`, Debug-Bau auf einer geteilten Maschine, kleinster Wert aus zwei bis vier Läufen, 6.11.2026):

  | Eingabe | 1 MB | 2 MB | 4 MB | 8 MB |
  |---|---|---|---|---|
  | gewöhnlicher Text: vier Sätze eines Arbeitsrechtsfalls je auf einer Zeile, wiederholt | 5,1 s | 10,8 s | 24,5 s | 58,9 s |
  | «die » wiederholt, eine Zeile | 5,2 s | 10,8 s | 21,0 s | 41,1 s |
  | «Art. » wiederholt, eine Zeile | 5,7 s | 10,9 s | 20,7 s | 42,1 s |

  Vorher brauchte «Art. » schon für 64 KB 26 s, «die » für 256 KB 9,9 s und der gewöhnliche Text für 1 MB 10,8 s. Jetzt wachsen «die » und «Art. » je Verdoppelung um das 1,9- bis 2,1-Fache. Derselbe gewöhnliche Text als Absätze, nach 72 Zeichen umbrochen, sodass fast jedes Zeilenende ein weicher Umbruch ist (5.2), brauchte in abwechselnden Läufen 2 bis 4 % länger (8 MB: 78,4 s gegen 75,3 s). Die Regel `hinweise` findet die Treffer eines Satzes und einer Zeile über ihre Reihenfolge im Text (binäre Suche statt eines Durchlaufs über alle Treffer). Mit einer Fundstellen-Bitte in jedem Satz wuchs der Lauf von 256 auf 512 KB um das 1,8-Fache (vorher 2,9), mit einem Vermerk auf jeder Zeile um das 1,9-Fache (vorher 3,7). Der gewöhnliche Text bis 8 MB ist danach nicht neu gemessen. Ein Release-Bau ist nicht gemessen.
- **Unicode nach Standard.** Normalisierung nach UAX #15 (NFC) und Satzgrenzen nach UAX #29 statt eigener Regeln. Angepasst wird nur, wo UAX #29 selbst Anpassungen vorsieht («Tailoring»), und jede Anpassung ist dokumentiert und getestet (`text::saetze_bestimmen`): Abkürzungen aus `lexika/abkuerzungen.toml` und Ordinalzahlen wie «5.» in «am 5. März» beenden keinen Satz, und ein einzelner Zeilenumbruch mitten in einem Absatz auch nicht (5.2, `text::weiche_umbrueche_bestimmen`). Zeichen mit der Unicode-Eigenschaft Default_Ignorable_Code_Point, etwa das weiche Trennzeichen U+00AD oder U+200B, bleiben im Text, damit Zeile und Spalte stimmen. Wortlisten und die Erkenner der Klasse A lesen über sie hinweg, und die Ausgabe meldet sie (AUSGABE.md, «Meldungen zur Eingabe»). Grenze: Die Wortlisten der übrigen Regeln, die Wortfolgen in `kontext` und das Datum in `geburtsdatum` lesen ebenfalls über diese Zeichen hinweg («wohn[U+00AD]haft in Bern», «Meine [U+200B]Mandantin», «geb.[U+200B] 12.03.1985»), die Muster der Regeln `name` und `adresse` und der Zwischenraum vor dem Gemeindenamen in `wohnort` aber nicht. «Erika Bei[U+00AD]spiel» ergibt den Namen «Erika Bei», «See[U+00AD]strasse 12, 8002 Zürich» keine Adresse und «wohnhaft in [U+200B]Bern» keinen Wohnort. Für reguläre Ausdrücke steht dafür `text::OhneUnsichtbare` bereit, die Sicht der Klasse A mit dem Weg zurück zu den Versätzen. Bindestriche U+2010 und U+2011 gelten in Wortlisten und Telefonnummern wie `-`.
- **Kodierung.** Gelesen wird UTF-8 mit oder ohne BOM und UTF-16 mit BOM. Abgewiesen mit Exit-Code 2 werden UTF-32 und jede Eingabe mit dem Zeichen NUL, meist UTF-16 ohne BOM, denn darin griffe keine Regel. Deutet die Eingabe auf eine beschädigte Kodierung, etwa «Anw?ltin» aus einer Pipe von Windows PowerShell 5.1 oder Ersatzzeichen U+FFFD, meldet promptscan das auf stderr und im Bericht; der Exit-Code bleibt der der Funde (AUSGABE.md, «Kodierung der Eingabe»).

| Crate | Zweck |
|---|---|
| clap | Befehle und Optionen |
| regex | Muster der Klasse A, lineare Laufzeit |
| aho-corasick | Wortlisten: viele Begriffe in einem Durchlauf |
| unicode-normalization, unicode-segmentation | NFC, Satzgrenzen |
| serde, serde_json, toml | Wortlisten, Konfiguration, JSON-Ausgabe |
| anstream, anstyle | Farbe: Rot, Orange (256 Farben, sonst Gelb mit dem Wort ORANGE), Gelb, Blau |
| windows-sys | Konsole unter Windows: Fensterbreite, Start per Doppelklick erkennen |
| proptest, assert_cmd, predicates | Eigenschaftstests, CLI-Tests (nur Entwicklung) |

Wortlisten liegen als TOML im Repo und werden beim Bauen eingebettet. Jeder Eintrag trägt seine Quelle:

```toml
[[eintrag]]
begriff = "Arbeitsunfähigkeit"
formen = ["arbeitsunfähig", "arbeitsunfähige", "arbeitsunfähigen"]
sprache = "de"
regel = "art5c-2"
quelle = "SR 830.1 Art. 6"
fassung = "2024-01-01"
url = "https://fedlex.data.admin.ch/eli/cc/2002/510"
```

```
promptscan/
├─ Cargo.toml, Cargo.lock
├─ build.rs     bettet lexika/ in das Programm ein
├─ LICENSE.md, THIRD-PARTY-NOTICES.md   Lizenzstand, Vermerke (`promptscan licenses`)
├─ docs/        SPEC.md, AUSGABE.md (Ausgabe und Aufruf), WEITERGABE.md (Bau und Weitergabe)
├─ tools/       freigabe.ps1 (Release-Bau), drittvermerke/ (Vorlage und Lizenzdateien)
├─ lexika/      je Regel eine TOML-Datei, Quelle je Eintrag; register/ (BFS, swisstopo)
├─ src/
│  ├─ main.rs        CLI
│  ├─ lib.rs         Ablauf der Prüfung
│  ├─ text.rs        NFC, Satzgrenzen, Zeile und Spalte
│  ├─ klasse_a.rs    ahv, iban, tel, mail
│  ├─ lexikon.rs     Laden, Aho-Corasick
│  ├─ regeln/        Regeln der Klasse B, art5c-*, Kontext, Hinweise
│  ├─ fund.rs        Treffer und Funde
│  ├─ stufen.rs      rechtliches Modell (Abschnitt 4)
│  ├─ normen.rs      Normen mit amtlichem Wortlaut
│  ├─ katalog.rs     `promptscan rules` und `explain`
│  ├─ lizenzen.rs    `promptscan licenses`
│  └─ ausgabe.rs     farbiger Text, JSON
├─ examples/    belege (Belegprüfung), drittvermerke, register_namen, register_orte
└─ tests/       Eigenschaftstests, CLI, Regeln, Offline-Prüfung
```

## 7. Eigenschaftstests für Klasse A

`tests/klasse_a.rs` prüft AHV-Nummer und IBAN mit proptest. Die Läufe haben feste Startwerte und prüfen bei jedem Lauf dieselben Fälle:

- Jede erzeugte gültige Nummer wird in beliebigem Umgebungstext an ihrer Stelle erkannt, in jeder Schreibweise aus 5.1. Ausgenommen ist eine AHV-Nummer mit Leerzeichen, die über ein weiteres Leerzeichen an einer Ziffer hängt; sie gilt als Teil einer Zahlenkolonne.
- Jede Ersetzung einer einzelnen Ziffer durch eine andere wird abgewiesen.
- Jede Vertauschung zweier verschiedener Nachbarziffern wird abgewiesen, ausser bei der AHV-Nummer zwei Ziffern nach 756, die sich um 5 unterscheiden.
- In der Kontonummer der IBAN lässt MOD 97-10 genau die Ausnahmen aus 5.1 durch, und nur diese.
- Auf beliebigem Text stürzt keiner der vier Erkenner ab.

Dass die Aussagen zu Ersetzung und Vertauschung für jede Nummer gelten, nicht nur für die erzeugten, rechnen die Tests `beweis_…` über alle Fälle der Prüfsumme modulo 10 bzw. 97 nach.

## 8. Erkennungsgüte

Präzision und Trefferquote auf Prompts sind nicht gemessen; ein annotierter Korpus fehlt. Die Tests halten Beispiele und Gegenbeispiele je Regel fest, sind aber keine Stichprobe und ergeben keine Güteangabe. README und `promptscan --help` sagen das.

Zielwerte für eine Messung, je Regel und Sprache, mit 95-%-Intervall:

- `ahv`, `iban`: Präzision und Trefferquote je mindestens 0,99.
- `name`, `adresse`: Trefferquote mindestens 0,90, Präzision mindestens 0,80.
- `art5c-*`: Trefferquote mindestens 0,85.

Die Trefferquote wiegt schwerer als die Präzision: Ein übersehenes Geheimnis lässt sich nach dem Senden nicht zurückholen, ein Fehlalarm kostet einen zweiten Blick.

## 9. Weitergabe

Wie die Programmdatei gebaut, geprüft und weitergegeben wird (Mindestanforderungen, SmartScreen und Smart App Control, Signatur, Prüfsumme, Entwicklerpfade, Drittvermerke), steht in `docs/WEITERGABE.md`.

## 10. Quellen und Prüfstand

### 10.1 Gesetzestexte

Gelesen direkt auf Fedlex (fedlex.data.admin.ch), geltende Fassung. «Fassung» ist das Datum der konsolidierten Fassung, die gelesen wurde. Gesetze sind urheberrechtlich nicht geschützt (Art. 5 Abs. 1 lit. a URG); Normauszüge in `explain` sind also zulässig.

**Normen mit Wortlaut im Programm.** Jede Norm, deren Wortlaut `explain` zeigt: die Konstanten in `src/normen.rs`, die Grundlagen aller Regeln und die Normen aller Rollen. Der Test `spec_10_1_nennt_jede_norm_mit_fassung` (`tests/ausgabe.rs`) erzeugt die Tabelle aus dem Programm und verlangt sie hier wörtlich; fehlt eine Zeile oder steht eine veraltete da, zeigt er die neue Tabelle. Jeder Wortlaut ist mit Fassung und Fundstelle auf Fedlex nachgelesen. Stand: **geprüft**.

| Zitat | Erlass | Fassung |
|---|---|---|
| Art. 2 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 2 Abs. 3 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 5 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 5 lit. c DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 5 lit. e DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 6 Abs. 2 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 6 Abs. 7 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 9 Abs. 1 lit. b DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 30 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 30 Abs. 2 lit. c DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 31 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 34 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 34 Abs. 4 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 36 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 36 Abs. 2 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 62 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 62 Abs. 2 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 162 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 320 Ziff. 1 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 320 Ziff. 2 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 321 Ziff. 1 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 321 Ziff. 1 Abs. 2 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 321 Ziff. 2 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 13 Abs. 1 BGFA | BGFA, SR 935.61 | 2025-07-01 |
| Art. 47 Abs. 1 lit. a BankG | BankG, SR 952.0 | 2026-10-01 |
| Art. 47 Abs. 2 BankG | BankG, SR 952.0 | 2026-10-01 |
| Art. 69 Abs. 1 lit. a FINIG | FINIG, SR 954.1 | 2026-10-01 |
| Art. 69 Abs. 3 FINIG | FINIG, SR 954.1 | 2026-10-01 |
| Art. 147 Abs. 1 lit. a FinfraG | FinfraG, SR 958.1 | 2024-02-01 |
| Art. 328b OR | OR, SR 220 | 2026-10-01 |
| Art. 342 Abs. 1 lit. a OR | OR, SR 220 | 2026-10-01 |
| Art. 552 Abs. 1 OR | OR, SR 220 | 2026-10-01 |
| Art. 568 Abs. 1 OR | OR, SR 220 | 2026-10-01 |
| Art. 594 Abs. 2 OR | OR, SR 220 | 2026-10-01 |
| Art. 98 Abs. 2 KAG | KAG, SR 951.31 | 2026-10-01 |
| Art. 73 Abs. 1 StPO | StPO, SR 312.0 | 2025-04-01 |
| Art. 153b AHVG | AHVG, SR 831.10 | 2026-01-01 |
| Art. 5 Abs. 1 lit. a URG | URG, SR 231.1 | 2025-07-01 |
| Art. 70 Abs. 1 BV | BV, SR 101 | 2024-03-03 |
| Art. 5 lit. d DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 5 lit. j und k DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 9 Abs. 1 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 9 Abs. 2 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 9 Abs. 3 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 16 Abs. 1 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 16 Abs. 2 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 17 Abs. 1 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 61 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 61 lit. b DSG | DSG, SR 235.1 | 2025-07-07 |
| Anhang 1 Ziff. 44 DSV | DSV, SR 235.11 | 2025-12-01 |
| Art. 49 AHVG | AHVG, SR 831.10 | 2026-01-01 |
| Art. 51 Abs. 3 AHVG | AHVG, SR 831.10 | 2026-01-01 |
| Art. 153c Abs. 1 AHVG | AHVG, SR 831.10 | 2026-01-01 |
| Art. 12 lit. a BGFA | BGFA, SR 935.61 | 2025-07-01 |
| Art. 321a Abs. 4 OR | OR, SR 220 | 2026-10-01 |
| Art. 730b Abs. 2 OR | OR, SR 220 | 2026-10-01 |
| Art. 110 Abs. 3 StGB | StGB, SR 311.0 | 2026-11-01 |
| Art. 12 StPO | StPO, SR 312.0 | 2025-04-01 |
| Art. 13 StPO | StPO, SR 312.0 | 2025-04-01 |
| Art. 2 Abs. 1 FINIG | FINIG, SR 954.1 | 2026-10-01 |
| Art. 2 Abs. 2 lit. a DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 16 Abs. 2 lit. d DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 30 DSG | DSG, SR 235.1 | 2025-07-07 |
| Art. 2 Abs. 1 BGFA | BGFA, SR 935.61 | 2025-07-01 |
| Art. 13 Abs. 2 BGFA | BGFA, SR 935.61 | 2025-07-01 |
| Art. 3 ISG | ISG, SR 128 | 2025-10-01 |
| Art. 27 Abs. 1 BGG | BGG, SR 173.110 | 2026-04-01 |
| Art. 153i Abs. 1 AHVG | AHVG, SR 831.10 | 2026-01-01 |
| Art. 133 AHVV | AHVV, SR 831.101 | 2026-01-01 |
| Art. 134quinquies Abs. 2 AHVV | AHVV, SR 831.101 | 2026-01-01 |
| Art. 2 Abs. 1 AEFV | AEFV, SR 784.104 | 2026-07-01 |
| Art. 16 AEFV | AEFV, SR 784.104 | 2026-07-01 |
| Ziff. 2.2.4 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 2.2.6 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 5.1.3 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 5.2.3 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 5.3.3 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 5.4.3 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Ziff. 5.5.3 TAV E.164 | TAV Nummerierungsplan und Aufteilung der E.164-Nummern, Ausgabe 8 (Anhang 2.2 der Verordnung des BAKOM über Fernmeldedienste und Adressierungselemente), SR 784.101.113/2.2 | 2024-03-01 |
| Art. 950 Abs. 1 Satz 2 und Abs. 2 OR | OR, SR 220 | 2026-10-01 |
| Art. 116a HRegV | HRegV, SR 221.411 | 2026-10-01 |
| Art. 26b Abs. 1 und 4 GeoNV | GeoNV, SR 510.625 | 2024-01-01 |
| Art. 23 Abs. 1 lit. b VPG | Postverordnung (VPG), SR 783.01 | 2026-04-01 |
| Art. 3 lit. f GeoNV | GeoNV, SR 510.625 | 2024-01-01 |
| Art. 24 GeoNV | GeoNV, SR 510.625 | 2024-01-01 |
| Art. 23 Abs. 1 ZGB | ZGB, SR 210 | 2026-07-01 |
| Art. 19 Abs. 1 GeoNV | GeoNV, SR 510.625 | 2024-01-01 |
| Art. 13 ISG | ISG, SR 128 | 2025-10-01 |
| Art. 2 Abs. 1 und 2 ISG | ISG, SR 128 | 2025-10-01 |
| Art. 57 Abs. 1 BGerR | BGerR, SR 173.110.131 | 2026-02-01 |

**Nur zitiert.** Diese Normen nennen Texte des Programms, ohne ihren Wortlaut zu zeigen. Die Fassung wurde beim Verfassen der Texte notiert oder stammt aus der Quellenangabe der Regel. Sammelzitate wie «Art. 16 f. DSG» oder «Art. 9 DSG», deren Absätze oben mit Wortlaut stehen, und Fundstellen der Wortlisten (Tabelle unten) fehlen hier. Die Liste ist von Hand aus den Texten gesammelt; der Test `spec_10_1_nennt_jede_norm_mit_fassung` prüft nur, dass keine Norm mit Wortlaut hier steht. Stand: **Wortlaut nicht im Programm, nicht nachgeprüft**.

| Zitat | Erlass | Fassung | zitiert in |
|---|---|---|---|
| Anhang 1 DSV | DSV, SR 235.11 | 2025-12-01 | Zeile `Klären` («Anhang 1 DSV»), Punkt «Ausland» |
| Art. 11 ISG | ISG, SR 128 | 2025-10-01 | `explain vermerk` |
| Art. 17 Abs. 5 ISV | ISV, SR 128.1 | 2025-05-01 | `explain vermerk` |
| Art. 18–20 ISV | ISV, SR 128.1 | 2025-05-01 | `explain vermerk` |
| Art. 4bis Abs. 1 BankG | BankG, SR 952.0 | 2026-10-01 | `explain kontext` |
| Art. 32 Abs. 1 OR | OR, SR 220 | 2026-10-01 | `explain kontext` |
| Art. 40b OR | OR, SR 220 | 2026-10-01 | `explain kontext` |
| Art. 398 Abs. 2 OR | OR, SR 220 | 2026-10-01 | `explain kontext` |
| Art. 394 Abs. 1 ZGB | ZGB, SR 210 | 2026-07-01 | `explain kontext` |
| Art. 27 lit. c PsyG | PsyG, SR 935.81 | 2025-07-01 | `explain kontext` |
| Art. 31 Abs. 2 RVOG | RVOG, SR 172.010 | 2025-05-01 | `explain kontext` |
| Art. 10 Abs. 5 VAwG | VAwG, SR 143.11 | 2026-11-01 | `explain geburtsdatum` |
| Art. 15a Abs. 4bis ZStV | ZStV, SR 211.112.2 | 2025-06-01 | `explain geburtsdatum` |
| Art. 29bis Abs. 2 AHVG | AHVG, SR 831.10 | 2026-01-01 | `explain geburtsdatum` |
| Art. 8 Abs. 1 StBOG | StBOG, SR 173.71 | 2024-01-01 | `explain geburtsdatum` |

**Erlasse der Wortlisten.** Je SR-Nummer und Fassung die Fundstellen aus `lexika/*.toml` (Feld `quelle`); jeder Eintrag ist am Wortlaut des zitierten Artikels nachgeprüft (Kopf der Wortlisten). Der Test `spec_10_1_nennt_die_erlasse_der_wortlisten` erzeugt die Tabelle aus den Wortlisten und verlangt sie hier wörtlich; ändert sich eine Wortliste, zeigt er die neue Tabelle. Quellen ausserhalb des Bundesrechts (BFS, CLDR, Wörterbücher, Schreibweisungen): 10.2.

| Erlass | Fassung | Fundstellen der Wortlisten |
|---|---|---|
| SR 0.103.1 | 2024-11-28 | Art. 8 Abs. 1 Bst. a: «il diritto di ogni individuo di costituire con altri dei sindacati e di aderire al sindacato di sua scelta…» |
| SR 0.104 | 2022-04-21 | Art. 4: «…di un gruppo di individui di un certo colore o di una certa origine etnica…» |
| SR 0.822.719.9 | 2023-08-31 | Art. 1 Ziff. 1: «…tendenti a pregiudicare la libertà sindacale in materia d’impiego.»; Art. 1 Ziff. 1: «…à porter atteinte à la liberté syndicale en matière d’emploi.»; Art. 1 Ziff. 2; Art. 1 Ziff. 2: «…à la condition qu’il ne s’affilie pas à un syndicat ou cesse de faire partie d’un syndicat» |
| BV, SR 101 | 2024-03-03 | Art. 1; Art. 2 Abs. 2; Art. 2 Abs. 3; Art. 2 Abs. 4; Art. 3; Art. 4; Art. 5 Abs. 3; Art. 5a; Art. 6; Art. 8 Abs. 2; Art. 8 Abs. 2: «…e di menomazioni fisiche, mentali o psichiche.»; Art. 8 Abs. 2: «…ni du fait d’une déficience corporelle, mentale ou psychique.»; Art. 8 Abs. 3; Art. 8 Abs. 4; Art. 9; Art. 10 Abs. 3; Art. 10a Abs. 1; Art. 10a Abs. 2; Art. 13 Abs. 1; Art. 15 Abs. 2; Art. 16 Abs. 3; Art. 24 Abs. 1; Art. 25 Abs. 1; Art. 28 Abs. 3; Art. 29 Abs. 1; Art. 29 Abs. 3; Art. 30 Abs. 1; Art. 30 Abs. 2; Art. 31 Abs. 3; Art. 31 Abs. 4; Art. 32 Abs. 1; Art. 32 Abs. 3; Art. 35 Abs. 3; Art. 37 Abs. 1; Art. 39 Abs. 3; Art. 39 Abs. 4; Art. 40 Abs. 1; Art. 41 Abs. 2; Art. 44 Abs. 3; Art. 46 Abs. 2; Art. 49 Abs. 1; Art. 52 Abs. 2; Art. 56 Abs. 2; Art. 61 Abs. 3; Art. 72 Abs. 2; Art. 75b Abs. 1; Art. 76 Abs. 5; Art. 83 Abs. 2; Art. 86 Abs. 4; Art. 87a Abs. 2; Art. 88 Abs. 2; Art. 106 Abs. 6; Art. 117 Abs. 1: «Der Bund erlässt Vorschriften über die Kranken- und die Unfallversicherung.»; Art. 117a Abs. 1; Art. 118b Abs. 2; Art. 119 Abs. 2; Art. 139b; Art. 139b Abs. 2; Art. 144 Abs. 2; Art. 160 Abs. 1; Art. 165 Abs. 1 |
| SR 121 | 2026-11-01 | Art. 5 Abs. 5 |
| ISG, SR 128 | 2025-10-01 | Art. 1, Fussnote 5; Art. 11 Abs. 2; Art. 13 Abs. 1; Art. 13 Abs. 2; Art. 13 Abs. 3 |
| SR 128.1 | 2025-05-01 | Art. 18 (Sachüberschrift) |
| SR 131.212 | 2026-01-01 | Art. 110 Abs. 2: «Les communes peuvent participer à des syndicats de communes ou à d’autres organisations…» |
| SR 142.20 | 2026-11-01 | Art. 4 Abs. 1; Art. 62 Abs. 1 Bst. e |
| SR 142.31 | 2026-06-12 | Art. 3 Abs. 1; Art. 99a Abs. 3 Bst. a: «…la nationalité, l’ethnie, la religion, l’état civil…» |
| SR 143.1 | 2023-09-01 | Art. 2 Abs. 2bis |
| SR 143.11 | 2026-11-01 | Art. 10 Abs. 5 |
| SR 151.3 | 2020-07-01 | Art. 2 Abs. 1 |
| SR 152.3 | 2023-11-01 | Art. 1 |
| SR 161.1 | 2022-10-23 | Art. 33 Abs. 1 |
| SR 170.512 | 2023-09-01 | Art. 5, Fussnote 13; Art. 12 Abs. 2 |
| SR 172.010 | 2025-05-01 | Art. 31 Abs. 2; Art. 47 Abs. 1 |
| SR 172.220.1 | 2024-01-01 | Art. 34c Abs. 1 Bst. a: «…la disdetta è stata pronunciata poiché l’impiegato, in buona fede, ha sporto denuncia…»; Art. 34c Abs. 1 Bst. a: «…l’employé avait, de bonne foi, dénoncé une infraction…» |
| SR 173.71 | 2024-01-01 | Art. 2 Abs. 1 lit. b; Art. 8 Abs. 1; Art. 9 Abs. 1; Art. 14 lit. a; Art. 33 lit. c |
| BGG, SR 173.110 | 2026-04-01 | Art. 16 Abs. 2 Bst. a; Art. 23 (Sachüberschrift); Art. 27 Abs. 1; Art. 27 Abs. 2 |
| BGerR, SR 173.110.131 | 2026-02-01 | Art. 4 (Sachüberschrift); Art. 57 Abs. 1 Bst. a; Art. 58 Abs. 1; Art. 58 Abs. 2 |
| ZGB, SR 210 | 2026-07-01 | Art. 1 Abs. 3; Art. 23 Abs. 1; Art. 30a; Art. 84 Abs. 3: «…peuvent déposer une plainte auprès de l’autorité de surveillance contre les actes ou les omissions des organes de la fondation.»; Art. 252 Abs. 1; Art. 255 Abs. 1; Art. 255a Abs. 1; Art. 255a Abs. 2; Art. 303 Abs. 1; Art. 303 Abs. 3; Art. 311; Art. 311 Abs. 1; Art. 311 Abs. 1: «…l’autorité de protection de l’enfant prononce le retrait de l’autorité parentale…»; Art. 311 Abs. 2: «Wird beiden Eltern die Sorge entzogen, so erhalten die Kinder einen Vormund.»; Art. 311 Abs. 3; Art. 312: «Die Kindesschutzbehörde entzieht die elterliche Sorge: …»; Art. 314b Abs. 1; Art. 327a: «L’autorità di protezione dei minori nomina un tutore al minorenne che non è sotto autorità parentale.»; Art. 327a: «L’autorité de protection de l’enfant nomme un tuteur lorsque l’enfant n’est pas soumis à l’autorité parentale.»; Art. 327a: «Steht ein Kind nicht unter elterlicher Sorge, so ernennt ihm die Kindesschutzbehörde einen Vormund.»; Art. 327b: «Das Kind unter Vormundschaft hat die gleiche Rechtsstellung wie das Kind unter elterlicher Sorge.»; Art. 327b: «Il minorenne sotto tutela ha lo stesso stato giuridico…»; Art. 327b: «Le statut juridique de l’enfant sous tutelle est le même que celui de l’enfant soumis à l’autorité parentale…»; Art. 390 Abs. 1; Art. 390 Abs. 1 Ziff. 1; Art. 393 Abs. 1; Art. 394; Art. 394 Abs. 3; Art. 400 Abs. 1; Art. 669 |
| SR 211.112.2 | 2025-06-01 | Art. 15a Abs. 4bis; Art. 71 Abs. 3 |
| SR 211.231 | 2025-01-01 | Art. 10 Abs. 1 |
| SR 211.412.11 | 2014-01-01 | Art. 2 Abs. 4: «…depuis la création du syndicat de remaniement…» |
| OR, SR 220 | 2026-10-01 | Art. 40b; Art. 50 Abs. 1; Art. 96; Art. 184 Abs. 1; Art. 239 Abs. 2; Art. 253; Art. 266f; Art. 275; Art. 319 Abs. 1; Art. 336 Abs. 2 Bst. a; Art. 336d Abs. 1; Art. 398 Abs. 2; Art. 431 Abs. 2; Art. 635a; Art. 728 Abs. 2 Ziff. 5: «l’acceptation d’un mandat qui entraîne une dépendance économique»; Art. 728 Abs. 2 Ziff. 5: «l’assunzione di un mandato che comporti dipendenza economica» |
| SR 221.229.1 | 2024-01-01 | Art. 11 Abs. 1: «Das Versicherungsunternehmen stellt dem Versicherungsnehmer eine Police aus, welche die Rechte und Pflichten der Parteien festhält.»; art. 11 al. 1: «L'entreprise d'assurance remet au preneur d'assurance une police constatant les droits et les obligations des parties.»; art. 11 cpv. 1: «L’assicuratore rilascia allo stipulante una polizza che stabilisce i diritti e gli obblighi delle parti.» |
| HRegV, SR 221.411 | 2026-10-01 | Anhang 2; Anhang 2, Fussnote zur Überschrift: «l’O du 2 fév. 2022» |
| URG, SR 231.1 | 2025-07-01 | Art. 25 Abs. 1; Art. 25 Abs. 2; Art. 77i Abs. 1: «…pour déposer une plainte ou une dénonciation pénale…» |
| DSG, SR 235.1 | 2025-07-07 | Art. 5 Bst. c Ziff. 2; Art. 5 Bst. c Ziff. 3; Art. 5 Bst. c Ziff. 4; Art. 5 Bst. c Ziff. 5: «Daten über verwaltungs- und strafrechtliche Verfolgungen oder Sanktionen»; Art. 5 Bst. c Ziff. 5: «i dati concernenti perseguimenti e sanzioni amministrativi e penali»; Art. 5 Bst. c Ziff. 5: «les données sur des poursuites ou sanctions pénales et administratives»; Art. 5 Bst. c Ziff. 6: «Daten über Massnahmen der sozialen Hilfe»; Art. 5 Bst. c Ziff. 6: «i dati concernenti le misure d’assistenza sociale»; Art. 5 lit. i |
| SR 281.1 | 2026-01-01 | Art. 17 Abs. 1: «…il peut être porté plainte à l’autorité de surveillance lorsqu’une mesure de l’office est contraire à la loi…»; Art. 50 Abs. 1 |
| SR 291 | 2026-01-01 | Art. 37 Abs. 1; Art. 43 Abs. 1 |
| StGB, SR 311.0 | 2026-11-01 | Art. 30 Abs. 1: «Si une infraction n’est punie que sur plainte, toute personne lésée peut porter plainte contre l’auteur.»; Art. 34 Abs. 1; Art. 40 Abs. 1; Art. 66a Abs. 1: «Il giudice espelle dal territorio svizzero per un tempo da cinque a quindici anni lo straniero condannato per uno dei seguenti reati…»; Art. 66a Abs. 1: «Le juge expulse de Suisse l’étranger qui est condamné pour l’une des infractions suivantes…»; Art. 66a Abs. 2; Art. 115; Art. 118; Art. 201–212, Fussnote 308; Art. 261bis; Art. 261bis: «…wegen ihrer Rasse, Ethnie, Religion oder sexuellen Orientierung…»; Art. 320 Ziff. 1; Art. 321 Ziff. 1 |
| StPO, SR 312.0 | 2025-04-01 | Art. 2 Abs. 2; Art. 5 Abs. 2; Art. 8 Abs. 1; Art. 12; Art. 12 lit. a; Art. 12 lit. b; Art. 13 lit. b; Art. 16 Abs. 1; Art. 32; Art. 73 Abs. 1; Art. 111 Abs. 1; Art. 112 Abs. 3: «L’impresa deve designare un altro rappresentante se per il medesimo fatto o per fatti connessi è avviata un’inchiesta penale nei confronti del rappresentante designato.»; Art. 112 Abs. 3: «Si une enquête pénale est ouverte pour les mêmes faits ou pour des faits connexes à l’encontre de la personne qui représente l’entreprise…»; Art. 112 Abs. 3: «Wird gegen die Person, die das Unternehmen im Strafverfahren vertritt, wegen des gleichen oder eines damit zusammenhängenden Sachverhalts eine Strafuntersuchung eröffnet…»; Art. 115 Abs. 2: «Sont toujours considérées comme des lésés les personnes qui ont qualité pour déposer plainte pénale.»; Art. 118 Abs. 1: «…participer à la procédure pénale comme demandeur au pénal ou au civil.»; Art. 118 Abs. 2: «La querela è equiparata a tale dichiarazione.»; Art. 118 Abs. 2: «Une plainte pénale équivaut à une telle déclaration.»; Art. 195 Abs. 1: «Die Strafbehörden holen amtliche Berichte und Arztzeugnisse über Vorgänge ein…»; Art. 195 Abs. 1: «Le autorità penali richiedono i rapporti ufficiali e i certificati medici relativi a eventi…»; Art. 195 Abs. 1: «Les autorités pénales requièrent les rapports officiels et les certificats médicaux relatifs à des faits…»; Art. 195 Abs. 2: «…Auskünfte über Vorstrafen und den Leumund…»; Art. 195 Abs. 2: «…des renseignements sur les antécédents judiciaires et la réputation du prévenu…»; Art. 195 Abs. 2: «…informazioni relative ai suoi precedenti penali e alla sua reputazione…»; Art. 210 Abs. 2; Art. 217 Abs. 1; Art. 217 Abs. 3 Bst. c; Art. 220 Abs. 1; Art. 220 Abs. 2; Art. 311 Abs. 1; Art. 324 Abs. 1; Art. 324 Abs. 2; Art. 335 Abs. 1; Art. 335 Abs. 2; Art. 352 Abs. 1; Titel: «Code de procédure pénale suisse (Code de procédure pénale, CPP) du 5 octobre 2007» |
| SR 312.1 | 2025-07-01 | Titel: «Loi fédérale sur la procédure pénale applicable aux mineurs (Procédure pénale applicable aux mineurs, PPMin) du 20 mars 2009» |
| SR 314.1 | 2025-01-01 | Art. 1 Abs. 1 |
| SR 321.0 | 2026-06-08 | Art. 109 Abs. 1: «…la détient alors qu’elle a été mise enceinte contre sa volonté…» |
| SR 322.1 | 2024-07-01 | Titel: «Procédure pénale militaire (PPM) du 23 mars 1979» |
| SR 330 | 2026-10-01 | Art. 1 Abs. 1; Art. 3 |
| SR 363 | 2023-09-01 | Art. 2; Titel |
| GeoNV, SR 510.625 | 2024-01-01 | Art. 19 Abs. 1 lit. a |
| SR 641.20 | 2025-03-31 | Art. 26 Abs. 1: «… hat dem Leistungsempfänger oder der Leistungsempfängerin auf Verlangen eine Rechnung auszustellen …»; art. 26 al. 1: «… le fournisseur lui délivre une facture …»; art. 26 cpv. 1: «… il fornitore della prestazione gli rilascia una fattura che soddisfi i requisiti di cui ai capoversi 2 e 3.» |
| SR 661 | 2024-01-01 | Art. 4a Abs. 2 |
| SR 741.01 | 2026-07-01 | Art. 16 Abs. 2; Art. 16 Abs. 2: «…entraîne le retrait du permis d’élève conducteur ou du permis de conduire, ou un avertissement.»; Art. 16a Abs. 2: «…au conducteur qui a fait l’objet d’un retrait de permis ou d’une autre mesure administrative…»; Art. 16c Abs. 1 Bst. f; Art. 16d Abs. 1 Bst. b: «qui souffre d’une forme de dépendance la rendant inapte à la conduite»; Art. 16d Abs. 1 Bst. b: «sie an einer Sucht leidet, welche die Fahreignung ausschliesst»; Art. 16d Abs. 1 Bst. b: «soffre di una forma di dipendenza che esclude l’idoneità alla guida» |
| SR 783.01 | 2026-04-01 | Art. 23 Abs. 1 lit. b; Art. 29 Abs. 4bis |
| SR 810.12 | 2025-08-01 | Art. 3 Bst. a; Art. 3 Bst. j; Art. 3 Bst. k |
| SR 811.11 | 2025-07-01 | Art. 2 Abs. 1 lit. e; Art. 40 lit. c |
| SR 811.21 | 2025-07-01 | Art. 2 Abs. 1 lit. a |
| SR 812.121 | 2023-09-01 | Art. 3b Abs. 1: «…prévention des troubles liés à l’addiction…» |
| SR 812.212.21 | 2026-01-01 | Art. 17 Abs. 2 Bst. b; Art. 33 Abs. 1 |
| SR 818.33 | 2023-09-01 | Art. 3 Abs. 1: «…che diagnosticano o trattano una malattia tumorale…»; Art. 3 Abs. 1: «…die eine Krebserkrankung diagnostizieren oder behandeln…»; Art. 3 Abs. 1: «…qui diagnostiquent ou traitent une maladie oncologique…»; Art. 21 Abs. 1: «…Patientinnen und Patienten, die in jungen Jahren an Krebs erkrankt sind…»; Art. 21 Abs. 1: «…des patients qui ont contracté un cancer dans leur jeune âge.»; Art. 21 Abs. 1: «…i dati dei pazienti colpiti da tumore in giovane età.» |
| SR 818.101 | 2025-08-01 | Art. 5 Abs. 1 Bst. c: «HIV und andere sexuell übertragbare Krankheitserreger»; Art. 5 Abs. 1 Bst. c: «le VIH et autres agents pathogènes de maladies sexuellement transmissibles»; Art. 33: «Eine Person, die krank, krankheitsverdächtig, angesteckt oder ansteckungsverdächtig ist…»; Art. 33: «Le persone malate, sospette malate, contagiate, sospette contagiate o…»; Art. 33: «Les personnes malades, présumées malades, infectées, présumées infectées ou…» |
| SR 822.11 | 2023-09-01 | Art. 17c Abs. 3: «…nella misura in cui non rispondano la cassa malati o un altro assicuratore del lavoratore.»; Art. 17c Abs. 3: «…à moins que la caisse-maladie ou une autre assurance du travailleur ne les assument.»; Art. 35 Abs. 1; Art. 35a Abs. 2 |
| SR 830.1 | 2024-01-01 | Art. 3 Abs. 1; Art. 3 Abs. 2; Art. 5 Abs. 1; Art. 6; Art. 7 Abs. 1; Art. 8 Abs. 1 |
| AHVG, SR 831.10 | 2026-01-01 | Art. 3 Abs. 1bis; Art. 29bis Abs. 2 |
| SR 831.20 | 2026-01-01 | Art. 6 Abs. 1bis; Art. 7a: «…ausgenommen sind Massnahmen, die ihrem Gesundheitszustand nicht angemessen sind.»; Art. 7a: «…fanno eccezione i provvedimenti che non sono adatti allo stato di salute dell’assicurato.»; Art. 7a: «…à l’exception des mesures qui ne sont pas adaptées à son état de santé.»; Art. 28a Abs. 1; Art. 35 Abs. 1: «Männer und Frauen, denen eine Invalidenrente zusteht…»; Art. 57 Abs. 1 Bst. l: «coordinare i provvedimenti sanitari con l’assicuratore malattia e l’assicuratore infortuni» |
| AHVV, SR 831.101 | 2026-01-01 | Art. 71 Abs. 3: «Direktzahlungen … erfolgen in Form von Auszahlungsscheinen mit Referenznummern der Schweizerischen Post.»; art. 71 cpv. 3: «I versamenti diretti … sono effettuati mediante polizze di pagamento con numeri di riferimento della Posta Svizzera.» |
| SR 831.201 | 2025-06-01 | Art. 31 Abs. 1: «La prestation transitoire au sens de l’art. 32 LAI est assimilable à une rente AI.»; Art. 31 Abs. 1: «La prestazione transitoria secondo l’articolo 32 LAI è parificata a una rendita AI.»; Art. 31 Abs. 1: «…einer IV-Rente gleichzustellen.» |
| SR 832.10 | 2026-07-01 | Art. 1a Abs. 1: «La presente legge disciplina l’assicurazione sociale contro le malattie (assicurazione sociale malattie).»; Art. 1a Abs. 1: «La présente loi régit l’assurance-maladie sociale.»; Art. 31 Abs. 1 Bst. a: «…durch eine schwere, nicht vermeidbare Erkrankung des Kausystems…»; Art. 42 Abs. 3bis; Art. 42 Abs. 3bis: «Die Leistungserbringer haben auf der Rechnung nach Absatz 3 die Diagnosen und Prozeduren nach den Klassifikationen in den jeweiligen vom zuständigen Departement herausgegebenen schweizerischen Fassungen codiert aufzuführen.» |
| SR 832.12 | 2026-07-01 | Art. 7 Abs. 2 Bst. p: «se l’assicuratore prevede di esercitare l’assicurazione malattia nel Principato del Liechtenstein…» |
| SR 832.20 | 2026-01-01 | Art. 16 Abs. 1 |
| SR 832.102 | 2026-10-22 | Art. 11 Abs. 2: «…comunicare all’assicuratore-malattie da quando cessa la copertura dell’infortunio»; Art. 11 Abs. 2: «…qu’il doit indiquer à l’assureur-maladie le moment où cesse la couverture des accidents.»; Art. 50; Art. 50c |
| SR 832.112.31 | 2026-08-01 | Art. 7 Abs. 2 Bst. b Ziff. 10: «…sowie Fusspflege bei Diabetikern»; Art. 11c Abs. 1 Bst. a: «…Risikofaktoren für ein diabetisches Fuss-Syndrom vorliegt»; Art. 11c Abs. 1 Bst. a: «…a persone affette da diabete mellito…»; Art. 11c Abs. 1 Bst. a: «…bei Personen mit Diabetes mellitus…»; Art. 11c Abs. 1 Bst. a: «…de développer un syndrome du pied diabétique»; Art. 11c Abs. 1 Bst. a: «…sindrome del piede diabetico…»; Art. 11c Abs. 1 Bst. a: «…à des personnes affectées de diabète sucré…»; Art. 12d Bst. a: «Per i neonati di madri sieropositive.»; Art. 12d Bst. a: «Pour les nourrissons de mères séropositives.»; Art. 12d cpv. 1 lett. a: «Test HIV»; Art. 12e Bst. c: «…bei Programmen zur Früherkennung von Brustkrebs durch Mammografie.»; Art. 12e Bst. c: «…di un programma di diagnosi precoce del cancro del seno…»; Art. 19b Bst. b: «…wegen einer schweren geistigen oder körperlichen Behinderung der versicherten Person…» |
| SR 851.1 | 2017-04-08 | Art. 3 Abs. 1: «Sono prestazioni assistenziali a tenore della presente legge le prestazioni in denaro e in natura di un ente pubblico, pagate secondo il diritto cantonale alle persone nel bisogno…» |
| BGFA, SR 935.61 | 2025-07-01 | Art. 13 Abs. 1; Art. 16 Abs. 1; Art. 16 Abs. 2 |
| KAG, SR 951.31 | 2026-10-01 | Art. 12 Abs. 2; Art. 38 Abs. 1; Art. 101; Art. 111 Abs. 1 |
| FINIG, SR 954.1 | 2026-10-01 | Art. 2 Abs. 1 lit. a; Art. 2 Abs. 1 lit. b; Art. 2 Abs. 1 lit. c; Art. 2 Abs. 1 lit. d; Art. 2 Abs. 1 lit. e |
| SR 955.0 | 2026-10-01 | Art. 18a Abs. 3 |

Dazu: Bundesgericht, Regeln für die Anonymisierung der Urteile (Präsidentenkonferenz 17.12.2019, Verwaltungskommission 13.02.2020), gelesen auf bger.ch; Art. 1, 4, 6, 7. Stand: **geprüft**.

### 10.2 Weitere Quellen

| Quelle | wofür | Stand |
|---|---|---|
| Dahl/Magesh/Suzgun/Ho 2024, Journal of Legal Analysis 16 | fundstellen | Kurzfassung geprüft, Volltext zu lesen |
| Chappuis/Alberini, Anwaltsrevue 2017, 337–343 | Hilfsperson-Streit | zu lesen |
| Schwarzenegger/Thouvenin/Stiller/George, Universität Zürich 2019 | Hilfsperson-Streit | zu lesen |
| ZAS, Berechnung der Kontrollziffer der AHVN13 | ahv | zu lesen: Die bekannte Adresse liefert am 06.11.2026 die BSV-Mitteilung Nr. 222; das Verfahren ist durch Anhang 7 der WL VA/IK (BSV) belegt, geprüft |
| ISO 13616, ISO 7064, GS1, RFC 5322 | Klasse A | Fundstellen in `explain` je Regel: ISO 13616-1:2020 Ziffern 4 bis 6 (öffentliche Vorschau) und GS1 General Specifications Release 26.0, Ziffer 7.9.1, geprüft; Anhänge A und B von ISO 13616-1 und ISO/IEC 7064 selbst nicht gelesen |
| BAKOM, Nummernplan | tel | TAV E.164 (SR 784.101.113/2.2), Ausgabe 8 vom 08.11.2023, in Kraft seit 01.03.2024, Ziffern 2.2, 2.3, 3 und 5; geprüft 06.11.2026 |
| BFS: Vornamen, Nachnamen, Gemeindeverzeichnis; swisstopo: Ortschafts- und Strassenverzeichnis | Klasse B | Quelle, Abrufdatum, Prüfsumme und Nutzungsbedingungen je Datensatz in den Dateiköpfen von `lexika/register/` und in THIRD-PARTY-NOTICES.md, 7.2; `cargo test` prüft, dass beide übereinstimmen |
| Duden online | 53 Einträge in 14 Wortlisten, u. a. Anreden, Begleiter, Funktionen, Vertretung, `kontext`, `personenbezug`, Nummernwörter, `geburtsdatum`, art5c-1, art5c-2, art5c-5 | Wortlaut je Eintrag im Feld `quelle`, abgerufen 06.11.2026 und 07.11.2026 |
| Larousse | 32 Einträge in 7 Wortlisten: Anreden, Funktionen, Rechtsvertretung, Vertretung, Nummernwörter, `geburtsdatum`, art5c-2 | wie Duden |
| Treccani (Vocabolario, Thesaurus, Enciclopedia dell'Italiano) | 24 Einträge in 7 Wortlisten: Anreden, Rechtsvertretung, Vertretung, `kontext`, Nummernwörter, `geburtsdatum`, art5c-1 | wie Duden; kein Eintrag für «burnout» |
| Bundeskanzlei: Schreibweisungen (2. Aufl. 2013, korr. 2015), Gesetzestechnische Richtlinien (GTR, 05.06.2026), Direttive di tecnica legislativa | 34 Einträge: Abkürzungen der Satzgrenze, Monatsnamen, Auslöser von `geburtsdatum`, `firma-grenzen` | Randziffer je Eintrag |
| Unicode CLDR 49 | 57 Einträge: Monatsnamen de-CH, fr-CH, it-CH (46) und Abkürzungen der Satzgrenze (11) | Fundstelle je Eintrag (Gebietsschema und Feld, bei den Abkürzungen Datei und Zeile) |
| BFS, Amtliches Gemeindeverzeichnis, Stand 06.11.2026 | `firma-gemeinden-ag.toml`: 196 Gemeinden des Kantons Aargau, Kantonskürzel «AG» | Gemeindenummer und Bezirk je Eintrag |
| BFS: Seite «Religionen» und Tabelle su-d/f/i-40.02.01.08.26 (Strukturerhebung 2024) | 28 Einträge in art5c-1 (Konfessionen) | Wortlaut je Eintrag |
| BAG, Aktionsplan Suizidprävention (DE, FR, IT) | 8 Einträge in art5c-2 | abgerufen 06.11.2026 |
| BFS, Instrumente zur medizinischen Kodierung (ICD-10-GM); WHO, ICD-10 Band 2 (2019), Abschnitt 2.4 | art5c-2: Auslöser «ICD», Aufbau der ICD-10-Kodes | geprüft 06.11.2026 |
| Bundesgericht: Anonymisierungsregeln; Seiten auf bger.ch | 5 Einträge: `fundstellen`, Funktionen, Rechtsvertretung | Wortlaut je Eintrag |
| EDÖB, Mitteilung vom 09.11.2023 | 4.1 | nur über Sekundärquellen; Mitteilung selbst zu lesen |
| Bundesrat, Entscheid vom 12.02.2025 zur KI-Regulierung | 4.1 | nur über Sekundärquellen; Stand der Vernehmlassung zu prüfen |

Die Zeilen von «Duden online» bis «Bundesgericht» sind aus den Belegen der Wortlisten gezählt (`cargo run --example belege`, Stand 07.11.2026). Gesetzestexte, die Wortlisten belegen, gehören zu 10.1. Die Belegprüfung prüft nur Fedlex-Belege gegen den amtlichen Text; die übrigen listet sie auf.

