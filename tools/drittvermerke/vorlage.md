# Vermerke zu fremden Bestandteilen

Diese Datei nennt alles, was in der Programmdatei von promptscan steckt und nicht von promptscan
selbst stammt, und gibt die Vermerke wieder, die die jeweiligen Lizenzen verlangen.
`promptscan licenses` gibt sie zusammen mit LICENSE.md aus.

Erzeugt mit `examples/drittvermerke.rs` aus Cargo.lock, den Paketen im Cargo-Register, dem
Sysroot der Rust-Werkzeugkette, `lexika/` und `tools/drittvermerke/`. Nicht von Hand ändern;
nachführen nach Abschnitt 9.

{{STAND}}

## 1. Übersicht

| Bestandteil | in der Programmdatei | Abschnitt |
|---|---|---|
| Crates von crates.io | {{ANZAHL_GEBUNDEN}} Crates | 2 |
| Crates nur für den Bau (Makros) | nein ({{ANZAHL_BAU}} Crates) | 3 |
| In Cargo.lock verzeichnet, nicht gebaut | nein ({{ANZAHL_NICHT}} Crates) | 4.1 |
| Crates nur für Tests (dev-dependencies) | nein ({{ANZAHL_TESTS}} Crates) | 4.2 |
| Rust-Standardbibliothek | ja | 5 |
| MinGW-w64-Laufzeit und libgcc | ja, statisch gelinkt | 6 |
| Daten: Unicode, BFS, swisstopo, Wörterbuchzitate, Gesetzestexte | ja, eingebettet | 7 |
| Lizenztexte | | 8 |

Stellt ein Paket mehrere Lizenzen zur Wahl, nutzt promptscan die mit der geringsten
Vermerkpflicht: 0BSD, dann Unlicense, dann Zlib, dann MIT, dann Apache-2.0. Die Spalte
«gewählt» nennt sie. 0BSD und Unlicense verlangen keinen Vermerk; Zlib verlangt ihn nur bei der
Weitergabe von Quelltext. Ihre Texte stehen trotzdem in Abschnitt 8.

## 2. Crates in der Programmdatei

{{TABELLE_GEBUNDEN}}

Die Tabelle nennt jedes Crate, das für die Programmdatei übersetzt und gelinkt wird. Was der
Linker danach als ungenutzt weglässt (LTO), ist nicht abgezogen; die Vermerke gelten
vorsorglich für alle.

{{ZUSATZ}}

## 3. Crates nur für den Bau

Prozedurale Makros laufen während des Übersetzens im Compiler. Ihr Code und der Code ihrer
Abhängigkeiten gelangt nicht in die Programmdatei; ein Vermerk ist darum nicht nötig.

{{TABELLE_BAU}}

## 4. In Cargo.lock verzeichnet, nicht in der Programmdatei

### 4.1 Nicht gebaut

Cargo.lock führt auch Abhängigkeiten, die erst eine nicht eingeschaltete Funktion (feature)
eines Pakets braucht. `cargo tree -e normal --target x86_64-pc-windows-gnu` zeigt sie nicht;
sie werden nicht übersetzt.

{{TABELLE_NICHT_GEBAUT}}

### 4.2 Nur für Tests (dev-dependencies)

Diese Pakete braucht nur `cargo test`: die Einträge unter `[dev-dependencies]` in Cargo.toml
und ihre Abhängigkeiten, auch solche für andere Zielsysteme. Sie gelangen nicht in die
Programmdatei; ein Vermerk ist nicht nötig, ihre Lizenztexte stehen darum nicht in Abschnitt 8.
«nicht entpackt» heisst: Das Paket liegt im Cargo-Register des erzeugenden Rechners nicht
entpackt vor, seine Lizenzangabe ist dort also nicht lesbar. Die Spalte «über» nennt, wer es
verlangt, und die Bedingung aus dessen Cargo.toml, wenn es nur für bestimmte Zielsysteme gilt.

{{TABELLE_NUR_TESTS}}

## 5. Rust-Standardbibliothek

Die Standardbibliothek wird statisch in jede Rust-Programmdatei gelinkt. Sie steht unter
«Apache-2.0 OR MIT»; gewählt ist MIT. Quelle der folgenden Angaben ist die Datei
`share/doc/rust/COPYRIGHT-library.html` der Werkzeugkette Rust {{STD_RUST}}.

Teile im Rust-Baum mit eigener Angabe. Die Tabelle gibt die Datei vollständig wieder, auch
`fuchsia.rs`, das nur für das Zielsystem Fuchsia übersetzt wird. Der Vermerk in Abschnitt 8.2
nennt die Teile unter «Apache-2.0 OR MIT»; der Unicode-Teil steht in Abschnitt 8.4.

{{STD_IM_BAUM}}

Crates ausserhalb des Rust-Baums, die die Standardbibliothek für x86_64-pc-windows-gnu
bindet. COPYRIGHT-library.html ist dafür unvollständig; die Liste stammt darum aus
`library/Cargo.lock` und `library/std/Cargo.toml` am Tag der Fassung (Einzelheiten in
`tools/drittvermerke/std.toml`):

{{STD_CRATES}}

Im Rust-Baum, mit eigener Lizenzdatei (Texte in Abschnitt 8.3):

{{STD_EIGENE}}

## 6. MinGW-w64-Laufzeit und libgcc

Die Programmdatei ist ein Bau für x86_64-pc-windows-gnu. Der Linker der Werkzeugkette bindet
dabei Teile der MinGW-w64-Laufzeit und der GCC-Laufzeit statisch ein. Die Werkzeugkette stammt
aus MinGW-Builds `x86_64-14.2.0-release-posix-seh-msvcrt-rt_v12-rev2` (GCC 14.2.0,
mingw-w64-Laufzeit v12), laut `src/ci/scripts/install-mingw.sh` am Tag 1.99.1 des
Rust-Repos.

Gelinkt wurden laut Linker-Map eines Release-Baus vom 06.11.2026 (Option `-Wl,-Map`):

- `crt2.o` (Programmstart) und aus `libmingw32.a` 20 Objekte (Start, TLS, Pseudo-Relokation,
  Fehlerbehandlung), aus `libmingwex.a` `mingw_matherr.o`, aus `libmsvcrt.a` vier Hilfsobjekte
  und die Importeinträge für `msvcrt.dll`;
- aus `libgcc.a` `_ctors.o` und aus `libgcc_eh.a` `unwind-seh.o` (Ausnahmebehandlung);
- aus `libpthread.a` (winpthreads) kein Objekt, obwohl die Bibliothek auf der Befehlszeile des
  Linkers steht.

Die mingw-w64-Laufzeit verlangt für die Weitergabe als Programmdatei die Vermerke in ihrer
Datei `COPYING.MinGW-w64-runtime.txt` (Abschnitt 8.5). Aufgenommen ist der Text der
Hauptlinie des mingw-w64-Repos (abgerufen am 06.11.2026); er enthält den Text des Tags v12.0.0
vollständig und zusätzlich einen Abschnitt zu Funktionen aus musl.

libgcc steht unter der GPL Version 3 mit der GCC Runtime Library Exception 3.1. Die Ausnahme
erlaubt, ein so gebildetes Kompilat «under terms of your choice» weiterzugeben, sofern der
gesamte Zielcode aus einem «Eligible Compilation Process» stammt; einen Vermerk verlangt sie
nicht. Ihr Text steht in Abschnitt 8.6 (aus `share/doc/COPYING.RUNTIME` der Werkzeugkette).

## 7. Daten

### 7.1 Unicode

Unicode-Daten stecken in:

- `regex-syntax` (Tabellen in `src/unicode_tables/`, eigene Lizenzdatei im Paket);
- `unicode-normalization` («(MIT OR Apache-2.0) AND Unicode-3.0»);
- `unicode-segmentation` (Tabellen in `src/tables.rs`, laut Dateikopf von `scripts/unicode.py`
  erzeugt; das Paket nennt dafür keine eigene Lizenz, vorsorglich gilt der Unicode-Vermerk
  unten mit);
- der Rust-Standardbibliothek (`library/core/src/unicode`, Unicode-3.0);
- `src/ausgabe.rs`: Bereiche des Breitenmodells, abgeleitet aus `EastAsianWidth.txt` und
  `DerivedGeneralCategory.txt` von Unicode 18.0.0 (Herkunft und Prüfsummen in
  `docs/AUSGABE.md`, Abschnitt «Breitenmodell»);
- `lexika/`: Ausnahmen der Satzsegmentierung und Monatsnamen aus Unicode CLDR 49.

Die Texte stehen in Abschnitt 8.4; die Unicode-Texte von `regex-syntax` und
`unicode-normalization` stehen bei den Crates in Abschnitt 8.1.

### 7.2 Register von BFS und swisstopo

Die Register unter `lexika/register/` sind in die Programmdatei eingebettet. Angaben aus den
Dateiköpfen:

{{DATEN_REGISTER}}

### 7.3 Zitate und Gesetzestexte

Jeder Eintrag in `lexika/*.toml` trägt seine Quelle (`quelle`, `url`); `promptscan explain`
zeigt sie bei jeder Regel. Nach Quellen:

{{DATEN_QUELLEN}}

Gesetzestexte sind nicht geschützt. Art. 5 Abs. 1 URG (SR 231.1, Fassung vom 1. Juli 2025):
«Durch das Urheberrecht nicht geschützt sind: a. Gesetze, Verordnungen, völkerrechtliche
Verträge und andere amtliche Erlasse; …». Abs. 2: «Ebenfalls nicht geschützt sind amtliche
oder gesetzlich geforderte Sammlungen und Übersetzungen der Werke nach Absatz 1.»

Wörterbücher sind geschützt; die Einträge zitieren sie. Art. 25 URG: «1 Veröffentlichte Werke
dürfen zitiert werden, wenn das Zitat zur Erläuterung, als Hinweis oder zur Veranschaulichung
dient und der Umfang des Zitats durch diesen Zweck gerechtfertigt ist. 2 Das Zitat als solches
und die Quelle müssen bezeichnet werden. Wird in der Quelle auf die Urheberschaft hingewiesen,
so ist diese ebenfalls anzugeben.»

## 8. Lizenztexte

Wortlaut unverändert aus den Paketen bzw. Dateien. Gleiche Texte stehen nur einmal, mit allen
Paketen, für die sie gelten.

{{TEXTE}}

## 9. Nachführen

Nach jeder Änderung an Cargo.lock, an `lexika/` (Register und Lizenzköpfe der Wortlisten), an
`tools/drittvermerke/` oder an der Werkzeugkette, im Projektordner:

```text
cargo tree -e normal --target x86_64-pc-windows-gnu --prefix none --no-dedupe --offline | cargo run --example drittvermerke
```

Mit `-- --pruefen` am Ende vergleicht das Beispiel nur und endet mit Exit-Code 1, wenn diese
Datei veraltet ist. Eine neue Rust-Fassung verlangt zuerst eine nachgeführte
`tools/drittvermerke/std.toml`; das Beispiel bricht sonst ab. Den Abschnitt 6 belegt nur eine
neue Linker-Map (`cargo rustc --release -- -C link-arg=-Wl,-Map=<Datei>`).

Die Tests in `src/lizenzen.rs` schlagen an, wenn ein Paket aus Cargo.lock, ein Register aus
`lexika/register/` (Name und Angaben aus dem Dateikopf) oder der Lizenzkopf einer Wortliste
aus `lexika/` hier fehlt oder nicht mehr wortgleich ist.
