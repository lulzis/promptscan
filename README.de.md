# promptscan

[![CI](https://github.com/lulzis/promptscan/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/lulzis/promptscan/actions/workflows/ci.yml)
[![CodeQL](https://github.com/lulzis/promptscan/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/lulzis/promptscan/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/lulzis/promptscan/badge)](https://scorecard.dev/viewer/?uri=github.com/lulzis/promptscan)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange?logo=rust)](Cargo.toml)
[![Windows 10/11 x64](https://img.shields.io/badge/Windows-10%20%7C%2011%20x64-0078D4)](docs/WEITERGABE.md#2-mindestanforderungen)
[![Offline](https://img.shields.io/badge/Netzwerk-keines-2ea44f)](tests/offline.rs)
[![Lizenz: alle Rechte vorbehalten](https://img.shields.io/badge/Lizenz-alle%20Rechte%20vorbehalten-lightgrey)](LICENSE.md)

**Deutsch** · [English](README.md)

promptscan prüft einen Prompt, bevor er an eine KI geht. Es markiert, welche Stellen nach
Schweizer Recht nicht ohne Weiteres hinausgehen sollten, und nennt zu jeder Markierung die
Norm, aus der sie folgt. Erkannt werden Deutsch, Französisch und Italienisch.

promptscan läuft lokal im Terminal. Es enthält keinen Netzwerkcode, startet keine anderen
Programme und schreibt Dateien nur auf ausdrücklichen Befehl.

> **Version 0.1.** Die Erkennungsgüte ist nicht gemessen. promptscan meldet
> Datenkategorien, keine Rechtsverletzungen, und ersetzt keine Rechtsauskunft.

## Beispiel

Fiktiver Prompt, ohne Farbe ausgegeben:

```text
> promptscan entwurf.txt
Lokal geprüft · Rolle: Beruf offen, aus «Mein Mandant» · Ziel: Cloud

   │ Mein Mandant Peter Muster, geboren am 3. Mai 1971, wohnhaft an der
   │ H^^^^^^^^^^^ R^^^^^^^^^^^             R^^^^^^^^^^
   │ Bahnhofstrasse 12 in 8001 Zürich, AHV-Nr. 756.1234.5678.97, leidet an einer
   │ R^^^^^^^^^^^^^^^^                         R^^^^^^^^^^^^^^^
   │ Depression. Darf ihm der Arbeitgeber kündigen?
   │ R^^^^^^^^^
     R ROT · H HINWEIS

ROT       Beruf offen: Art. 321 Ziff. 1 StGB (z. B. Anwalt, Psychologin),
          Art. 320 Ziff. 1 StGB (Behörde), Art. 47 Abs. 1 lit. a BankG (Bank)
          Name          Peter Muster
          Geburtsdatum  3. Mai 1971
          Adresse       Bahnhofstrasse 12
          AHV-Nummer    756.1234.5678.97 (Prüfziffer gültig)
          zugleich besonders schützenswert, Art. 5 lit. c Ziff. 2 DSG:
                        Depression

HINWEIS   Rechtsfolge nicht beurteilt
          Kontext       «Mein Mandant» (eigene Klientschaft)
                        Beruf offen

Ergebnis  ROT   Strafbewehrte Geheimhaltungspflicht (Beruf offen)
          6 Funde: 5 rot, 1 Hinweis
Klären    Braucht die KI diese Angaben? Welche Pflicht gilt? Festlegen mit
          --role lawyer | psychologist | authority | bank | professional | none
          Gehen die Daten ins Ausland? Dann gelten Art. 16 f. DSG, Anhang 1 DSV.
Mehr      promptscan explain rot · promptscan explain art5c-2 ·
          promptscan explain kontext
```

## Stufen

| Stufe | Bedeutung | Grundlage |
| --- | --- | --- |
| ROT | strafbewehrte Geheimhaltungspflicht; nur mit `--role` oder aus dem Text erschlossen | Art. 320, 321 StGB; Art. 47 BankG u. a. |
| ORANGE | besonders schützenswerte Personendaten | Art. 5 lit. c DSG |
| GELB | Personendaten | Art. 5 lit. a DSG |
| HINWEIS | Rechtsfolge nicht beurteilt | – |

`promptscan rules` zeigt alle Regeln mit Rechtsgrundlage, `promptscan explain <REGEL>` den
Normtext, die Methode, die Quellen der Wortliste und die Grenzen einer Regel.

## Bedienung

```text
promptscan                    Prompt einfügen, Enter, dann Ctrl+Z und Enter
promptscan DATEI              Datei prüfen, UTF-8 oder UTF-16 mit BOM
promptscan -t "TEXT"          Text direkt prüfen (steht dann im Shell-Verlauf)
promptscan rules              alle Regeln mit Rechtsgrundlage
promptscan explain REGEL      Norm, Methode, Quellen und Grenzen, z. B. ahv oder rot
promptscan config --role lawyer
                              Berufsrolle dauerhaft setzen
promptscan licenses           Lizenzstand und Vermerke zu fremden Bestandteilen

--role ROLLE                  Berufsrolle mit Geheimhaltungspflicht, z. B. lawyer, doctor, bank
--target cloud|local          Wohin der Prompt geht (Vorgabe cloud)
--lang de|en                  Sprache der Ausgabe (Vorgabe de)
--format text|json            json für Programme, in reinem ASCII
--output DATEI                Ausgabe in DATEI schreiben, UTF-8 mit BOM, ohne Farbe
--quiet                       nur das Ergebnis, ohne Prompt und Funde
--fail-on STUFE               Exit-Code 1 ab red (Vorgabe), orange oder yellow; never: immer 0
--no-color                    ohne Farbe; auch mit NO_COLOR
--ascii                       nur ASCII-Zeichen
```

Vertrauliches nicht mit `-t` übergeben, sondern einfügen oder als Datei prüfen: Text in der
Befehlszeile gelangt in den Verlauf der Shell. Exit-Code 0 heisst «nichts ab der Schwelle»,
1 «Fund ab der Schwelle», 2 «Fehler». Alle Optionen: `promptscan --help` und
[docs/AUSGABE.md](docs/AUSGABE.md).

## Installieren

Mit Rust (siehe [Bauen](#bauen)) legt Cargo die Programmdatei in `%USERPROFILE%\.cargo\bin`,
das schon im Pfad steht:

```text
cargo install --path . --locked
```

Eine weitergegebene `promptscan.exe` (Prüfsumme vorher vergleichen,
[docs/WEITERGABE.md](docs/WEITERGABE.md)) einmal in einen eigenen Ordner kopieren und
diesen in den Benutzerpfad aufnehmen, in PowerShell im Ordner der Datei:

```powershell
$ziel = "$env:LOCALAPPDATA\Programs\promptscan"
New-Item -ItemType Directory -Force $ziel | Out-Null
Copy-Item .\promptscan.exe $ziel
$pfad = [Environment]::GetEnvironmentVariable("Path", "User")
if (";$pfad;" -notlike "*;$ziel;*") {
    [Environment]::SetEnvironmentVariable("Path", "$pfad;$ziel", "User")
}
```

In einem neuen Terminalfenster läuft dann `promptscan` ohne Pfad.

## Grundsätze

- **Jede Regel hat eine Rechtsgrundlage** im geltenden amtlichen Wortlaut, mit Fassungsdatum.
- **Jede Erkennung hat eine belegte Methode:** Prüfziffer, amtliches Format oder eine
  Wortliste, in der jeder Eintrag seine Quelle trägt.
- **Alles lokal.** [`tests/offline.rs`](tests/offline.rs) prüft am Quelltext, an
  `Cargo.lock`, an `Cargo.toml` und an der Importtabelle der Programmdatei, dass promptscan
  nichts sendet und keine Programme startet.
- **Keine erfundenen Zahlen.** Was nicht gemessen ist, heisst «nicht gemessen».

Die Spezifikation mit Rechtsmodell, Quellen und Prüfstand steht in
[docs/SPEC.md](docs/SPEC.md).

## Bauen

Voraussetzungen: Windows 10 oder 11 (x64), Rust 1.88 oder neuer mit dem Ziel
`x86_64-pc-windows-gnu` und `dlltool` aus den MinGW-w64-Binutils. `windows-sys` und
`getrandom` erzeugen damit ihre Importbibliotheken. Das `dlltool` der Rust-Werkzeugkette
braucht einen Assembler, den sie nicht mitbringt; mit MSYS2 zum Beispiel:

```text
pacman -S mingw-w64-ucrt-x86_64-binutils
```

und in `%USERPROFILE%\.cargo\config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "dlltool=C:/msys64/ucrt64/bin/dlltool.exe"]
```

Dann:

```text
cargo test --release --locked
cargo clippy --release --locked --all-targets -- -D warnings
```

Die weitergegebene Programmdatei baut `tools\freigabe.ps1`. Das Skript entfernt
Entwicklerpfade, prüft die Importtabelle und schreibt die SHA-256-Prüfsumme
([docs/WEITERGABE.md](docs/WEITERGABE.md)).

## Dokumentation

| Datei | Inhalt |
| --- | --- |
| [docs/SPEC.md](docs/SPEC.md) | Zweck, Rechtsmodell, Regeln, Prüfstand der Quellen |
| [docs/AUSGABE.md](docs/AUSGABE.md) | Ausgabe, Farben, JSON, Aufruf und Konfiguration |
| [docs/WEITERGABE.md](docs/WEITERGABE.md) | Bau, Prüfung und Weitergabe der Programmdatei |
| [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) | Lizenzen fremder Bestandteile und Daten |
| [SECURITY.md](SECURITY.md) | Sicherheitslücken melden |

## Lizenz

Alle Rechte vorbehalten. Jede Weitergabe der Programmdatei oder des Quelltexts braucht die
vorherige Zustimmung des Rechteinhabers ([LICENSE.md](LICENSE.md)). Fremde Bestandteile stehen unter ihren eigenen Lizenzen
([THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)).
