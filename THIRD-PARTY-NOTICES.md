# Vermerke zu fremden Bestandteilen

Diese Datei nennt alles, was in der Programmdatei von promptscan steckt und nicht von promptscan
selbst stammt, und gibt die Vermerke wieder, die die jeweiligen Lizenzen verlangen.
`promptscan licenses` gibt sie zusammen mit LICENSE.md aus.

Erzeugt mit `examples/drittvermerke.rs` aus Cargo.lock, den Paketen im Cargo-Register, dem
Sysroot der Rust-Werkzeugkette, `lexika/` und `tools/drittvermerke/`. Nicht von Hand ändern;
nachführen nach Abschnitt 9.

Stand: promptscan 0.1.0, Rust 1.99.1 (58cf8f9c3 2026-10-13), Ziel x86_64-pc-windows-gnu.

## 1. Übersicht

| Bestandteil | in der Programmdatei | Abschnitt |
|---|---|---|
| Crates von crates.io | 34 Crates | 2 |
| Crates nur für den Bau (Makros) | nein (7 Crates) | 3 |
| In Cargo.lock verzeichnet, nicht gebaut | nein (3 Crates) | 4.1 |
| Crates nur für Tests (dev-dependencies) | nein (46 Crates) | 4.2 |
| Rust-Standardbibliothek | ja | 5 |
| MinGW-w64-Laufzeit und libgcc | ja, statisch gelinkt | 6 |
| Daten: Unicode, BFS, swisstopo, Wörterbuchzitate, Gesetzestexte | ja, eingebettet | 7 |
| Lizenztexte | | 8 |

Stellt ein Paket mehrere Lizenzen zur Wahl, nutzt promptscan die mit der geringsten
Vermerkpflicht: 0BSD, dann Unlicense, dann Zlib, dann MIT, dann Apache-2.0. Die Spalte
«gewählt» nennt sie. 0BSD und Unlicense verlangen keinen Vermerk; Zlib verlangt ihn nur bei der
Weitergabe von Quelltext. Ihre Texte stehen trotzdem in Abschnitt 8.

## 2. Crates in der Programmdatei

| Crate | Fassung | Lizenz laut Paket | gewählt | verlangter Vermerk |
|---|---|---|---|---|
| `aho-corasick` | 1.1.5 | Unlicense OR MIT | Unlicense | keiner |
| `anstream` | 1.0.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `anstyle-parse` | 1.0.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `anstyle-query` | 1.1.5 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `anstyle-wincon` | 3.0.11 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `clap` | 4.6.7 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `clap_builder` | 4.6.7 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `clap_lex` | 1.1.1 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `colorchoice` | 1.0.5 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `is_terminal_polyfill` | 1.70.2 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `memchr` | 2.8.3 | Unlicense OR MIT | Unlicense | keiner |
| `once_cell_polyfill` | 1.70.2 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `regex` | 1.13.1 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `serde_spanned` | 1.1.2 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `strsim` | 0.11.1 | MIT | MIT | Copyright- und Lizenzvermerk |
| `tinyvec` | 1.13.3 | Zlib OR Apache-2.0 OR MIT | Zlib | keiner (Lizenzvermerk nur bei Weitergabe von Quelltext) |
| `toml` | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `toml_datetime` | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `toml_parser` | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `toml_writer` | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `unicode-normalization` | 0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | MIT AND Unicode-3.0 | MIT: Copyright- und Lizenzvermerk; Unicode-3.0: Copyright- und Lizenzvermerk bei jeder Kopie oder in der Begleitdokumentation |
| `unicode-segmentation` | 1.13.3 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `utf8parse` | 0.2.2 | Apache-2.0 OR MIT | MIT | Copyright- und Lizenzvermerk |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `winnow` | 1.0.5 | MIT | MIT | Copyright- und Lizenzvermerk |
| `zmij` | 1.0.23 | MIT | MIT | Copyright- und Lizenzvermerk |

Die Tabelle nennt jedes Crate, das für die Programmdatei übersetzt und gelinkt wird. Was der
Linker danach als ungenutzt weglässt (LTO), ist nicht abgezogen; die Vermerke gelten
vorsorglich für alle.

Zusätzliche Vermerke für Code, den ein Crate aus einem anderen Projekt übernommen hat (Text in Abschnitt 8.1):

- `zmij` 1.0.23: zmij ist laut README.md und src/lib.rs des Pakets eine Zeile für Zeile übertragene Fassung eines C++-Originals; dessen Lizenzdatei (Datei LICENSE am Commit b32688a1849d1b8f092c1740c61a6b05ea4679ae, auf den README.md verweist, abgerufen am 06.11.2026, SHA-256 a69b36adb8d116376bcd580772fa45f08fffecc17833cb9822532f4e4f5ec372) trägt einen eigenen Copyright-Vermerk.

## 3. Crates nur für den Bau

Prozedurale Makros laufen während des Übersetzens im Compiler. Ihr Code und der Code ihrer
Abhängigkeiten gelangt nicht in die Programmdatei; ein Vermerk ist darum nicht nötig.

| Crate | Fassung | Lizenz laut Paket | Rolle |
|---|---|---|---|
| `clap_derive` | 4.6.7 | MIT OR Apache-2.0 | prozedurales Makro |
| `heck` | 0.5.0 | MIT OR Apache-2.0 | Abhängigkeit eines Makros |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 | Abhängigkeit eines Makros |
| `quote` | 1.0.47 | MIT OR Apache-2.0 | Abhängigkeit eines Makros |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | prozedurales Makro |
| `syn` | 3.0.9 | MIT OR Apache-2.0 | Abhängigkeit eines Makros |
| `unicode-ident` | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | Abhängigkeit eines Makros |

## 4. In Cargo.lock verzeichnet, nicht in der Programmdatei

### 4.1 Nicht gebaut

Cargo.lock führt auch Abhängigkeiten, die erst eine nicht eingeschaltete Funktion (feature)
eines Pakets braucht. `cargo tree -e normal --target x86_64-pc-windows-gnu` zeigt sie nicht;
sie werden nicht übersetzt.

| Crate | Fassung | Lizenz laut Paket | Grund |
|---|---|---|---|
| `equivalent` | 1.0.3 | Apache-2.0 OR MIT | nur über indexmap, das nicht gebaut wird |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | nur über indexmap, das nicht gebaut wird |
| `indexmap` | 2.14.2 | Apache-2.0 OR MIT | optionale Abhängigkeit von toml, nicht eingeschaltet |

### 4.2 Nur für Tests (dev-dependencies)

Diese Pakete braucht nur `cargo test`: die Einträge unter `[dev-dependencies]` in Cargo.toml
und ihre Abhängigkeiten, auch solche für andere Zielsysteme. Sie gelangen nicht in die
Programmdatei; ein Vermerk ist nicht nötig, ihre Lizenztexte stehen darum nicht in Abschnitt 8.
«nicht entpackt» heisst: Das Paket liegt im Cargo-Register des erzeugenden Rechners nicht
entpackt vor, seine Lizenzangabe ist dort also nicht lesbar. Die Spalte «über» nennt, wer es
verlangt, und die Bedingung aus dessen Cargo.toml, wenn es nur für bestimmte Zielsysteme gilt.

| Crate | Fassung | Lizenz laut Paket | über |
|---|---|---|---|
| `assert_cmd` | 2.2.2 | MIT OR Apache-2.0 | Cargo.toml [dev-dependencies] |
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT | num-traits |
| `bit-set` | 0.11.1 | Apache-2.0 OR MIT | proptest |
| `bit-vec` | 0.10.1 | Apache-2.0 OR MIT | bit-set; proptest |
| `bitflags` | 2.13.2 | MIT OR Apache-2.0 | proptest; rustix |
| `borsh` | 1.8.1 | MIT OR Apache-2.0 | bit-vec |
| `borsh-derive` | 1.8.1 | Apache-2.0 | borsh |
| `bstr` | 1.13.1 | MIT OR Apache-2.0 | assert_cmd |
| `bytes` | 1.12.1 | MIT | borsh |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 | chacha20; getrandom |
| `cfg_aliases` | 0.2.2 | MIT | borsh |
| `chacha20` | 0.10.2 | MIT OR Apache-2.0 | proptest |
| `console` | 0.16.6 | MIT | insta |
| `core_detect` | 1.0.0 | MIT/Apache-2.0 | proptest |
| `cpufeatures` | 0.3.1 | MIT OR Apache-2.0 | chacha20 (nur `cfg(any(target_arch = "x86_64", target_arch = "x86"))`) |
| `difflib` | 0.4.0 | MIT | predicates |
| `encode_unicode` | 1.0.0 | Apache-2.0 OR MIT | console (nur `cfg(windows)`) |
| `errno` | 0.3.14 | nicht entpackt | rustix |
| `fastrand` | 2.5.0 | Apache-2.0 OR MIT | tempfile |
| `float-cmp` | 0.10.0 | MIT | predicates |
| `fnv` | 1.0.7 | Apache-2.0 / MIT | rusty-fork |
| `getrandom` | 0.4.3 | MIT OR Apache-2.0 | rand; tempfile (nur `cfg(any(unix, windows, target_os = "wasi"))`) |
| `insta` | 1.49.0 | Apache-2.0 | Cargo.toml [dev-dependencies] |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | assert_cmd (nur `cfg(any())`); console; cpufeatures (nur unter 4 Ziel-Bedingungen); errno; getrandom (nur unter 8 Ziel-Bedingungen); rustix; wait-timeout (nur `cfg(unix)`) |
| `linux-raw-sys` | 0.12.1 | nicht entpackt | rustix |
| `normalize-line-endings` | 0.3.0 | Apache-2.0 | predicates |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | float-cmp; proptest |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | borsh-derive; insta; tempfile |
| `predicates` | 3.1.4 | MIT OR Apache-2.0 | assert_cmd; Cargo.toml [dev-dependencies] |
| `predicates-core` | 1.0.10 | MIT OR Apache-2.0 | assert_cmd; predicates; predicates-tree |
| `predicates-tree` | 1.0.13 | MIT OR Apache-2.0 | assert_cmd |
| `proc-macro-crate` | 3.5.0 | MIT OR Apache-2.0 | borsh-derive |
| `proptest` | 1.12.0 | MIT OR Apache-2.0 | Cargo.toml [dev-dependencies] |
| `quick-error` | 1.2.3 | MIT/Apache-2.0 | rusty-fork |
| `r-efi` | 6.0.0 | nicht entpackt | getrandom (nur `cfg(all(target_os = "uefi", getrandom_backend = "efi_rng"))`) |
| `rand` | 0.10.3 | MIT OR Apache-2.0 | proptest |
| `rand_core` | 0.10.1 | MIT OR Apache-2.0 | chacha20; getrandom; rand; rand_xorshift |
| `rand_xorshift` | 0.5.0 | MIT OR Apache-2.0 | proptest |
| `rustix` | 1.1.6 | nicht entpackt | tempfile (nur `cfg(any(unix, target_os = "wasi"))`) |
| `rusty-fork` | 0.3.1 | MIT/Apache-2.0 | proptest |
| `similar` | 2.7.0 | Apache-2.0 | insta |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | insta; proptest; rusty-fork |
| `termtree` | 0.5.1 | MIT | predicates-tree |
| `toml_edit` | 0.25.18+spec-1.1.0 | MIT OR Apache-2.0 | proc-macro-crate |
| `unarray` | 0.1.4 | MIT OR Apache-2.0 | proptest |
| `wait-timeout` | 0.2.1 | MIT/Apache-2.0 | assert_cmd; rusty-fork |

## 5. Rust-Standardbibliothek

Die Standardbibliothek wird statisch in jede Rust-Programmdatei gelinkt. Sie steht unter
«Apache-2.0 OR MIT»; gewählt ist MIT. Quelle der folgenden Angaben ist die Datei
`share/doc/rust/COPYRIGHT-library.html` der Werkzeugkette Rust 1.99.1 (58cf8f9c3 2026-10-13).

Teile im Rust-Baum mit eigener Angabe. Die Tabelle gibt die Datei vollständig wieder, auch
`fuchsia.rs`, das nur für das Zielsystem Fuchsia übersetzt wird. Der Vermerk in Abschnitt 8.2
nennt die Teile unter «Apache-2.0 OR MIT»; der Unicode-Teil steht in Abschnitt 8.4.

| Pfad im Rust-Baum | Lizenz | Copyright |
|---|---|---|
| `.` | Apache-2.0 OR MIT | The Rust Project Developers (see https://thanks.rust-lang.org) |
| `library/core/src/unicode` | Unicode-3.0 | 1991-2024 Unicode, Inc |
| `library/core/src/unicode/mod.rs` | Apache-2.0 OR MIT | The Rust Project Developers (see https://thanks.rust-lang.org) |
| `library/backtrace` | Apache-2.0 OR MIT | 2014 Alex Crichton; The Rust Project Developers (see https://thanks.rust-lang.org) |
| `library/std/src/sync/mpmc` | Apache-2.0 OR MIT | 2019 The Crossbeam Project Developers; The Rust Project Developers (see https://thanks.rust-lang.org) |
| `library/std/src/sys/sync/mutex/fuchsia.rs` | BSD-2-Clause AND (Apache-2.0 OR MIT) | 2016 The Fuchsia Authors; The Rust Project Developers (see https://thanks.rust-lang.org) |

Crates ausserhalb des Rust-Baums, die die Standardbibliothek für x86_64-pc-windows-gnu
bindet. COPYRIGHT-library.html ist dafür unvollständig; die Liste stammt darum aus
`library/Cargo.lock` und `library/std/Cargo.toml` am Tag der Fassung (Einzelheiten in
`tools/drittvermerke/std.toml`):

| Crate | Fassung | Lizenz laut Paket | gewählt | verlangter Vermerk |
|---|---|---|---|---|
| `addr2line` | 0.27.1 | Apache-2.0 OR MIT | MIT | Copyright- und Lizenzvermerk |
| `adler2` | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | 0BSD | keiner |
| `cfg-if` | 1.0.4 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `gimli` | 0.34.0 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `libc` | 0.2.189 | MIT OR Apache-2.0 | MIT | Copyright- und Lizenzvermerk |
| `memchr` | 2.8.3 | Unlicense OR MIT | Unlicense | keiner |
| `miniz_oxide` | 0.9.1 | MIT OR Zlib OR Apache-2.0 | Zlib | keiner (Lizenzvermerk nur bei Weitergabe von Quelltext) |
| `object` | 0.39.1 | Apache-2.0 OR MIT | MIT | Copyright- und Lizenzvermerk |
| `rustc-demangle` | 0.1.28 | MIT/Apache-2.0 | MIT | Copyright- und Lizenzvermerk |

Im Rust-Baum, mit eigener Lizenzdatei (Texte in Abschnitt 8.3):

| Teil | Pfad im Rust-Baum | Lizenz |
|---|---|---|
| compiler_builtins | `library/compiler-builtins` | MIT AND Apache-2.0 WITH LLVM-exception AND (MIT OR Apache-2.0) |
| libm (in compiler_builtins) | `library/compiler-builtins/libm` | MIT |

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

- `lexika/register/gemeinden.tsv`: Historisiertes Gemeindeverzeichnis der Schweiz, Snapshot der Gemeinden.
  Herausgeber: Bundesamt für Statistik BFS.
  Datensatz: https://opendata.swiss/de/dataset/historisiertes-gemeindeverzeichnis-der-schweiz
  Quelle: https://www.agvchapp.bfs.admin.ch/api/communes/snapshot?date=06-11-2026
  Stand: Stichtag 06.11.2026, abgerufen 2026-11-06.
  Nutzungsbedingungen: opendata.swiss, Freie Nutzung, https://opendata.swiss/terms-of-use#terms_open: «Sie dürfen diesen Datensatz für nicht kommerzielle Zwecke nutzen. Sie dürfen diesen Datensatz für kommerzielle Zwecke nutzen. Eine Quellenangabe wird empfohlen (Autor, Titel und Link zum Datensatz).»
- `lexika/register/nachnamen.tsv`: Nachnamen der ständigen Wohnbevölkerung nach Sprachregion, 2025 (über alle Sprachregionen summiert).
  Herausgeber: Bundesamt für Statistik (BFS), Statistik der Bevölkerung und der Haushalte (STATPOP).
  Quelle: https://opendata.swiss/de/dataset/nachnamen-der-standigen-wohnbevolkerung-nach-sprachregion3
  Stand: veröffentlicht 2026-08-21, abgerufen 2026-11-06.
  Nutzungsbedingungen: opendata.swiss, Nutzungsbedingung «Freie Nutzung. Quellenangabe ist Pflicht.» (https://opendata.swiss/terms-of-use#terms_by): nicht kommerzielle und kommerzielle Nutzung erlaubt, Quellenangabe Pflicht (Autor, Titel und Link zum Datensatz). Quelle: BFS – STATPOP, © BFS 2026
- `lexika/register/plz.tsv`: Amtliches Ortschaftenverzeichnis mit Postleitzahl und Perimeter.
  Herausgeber: Bundesamt für Landestopografie swisstopo.
  Quelle: https://data.geo.admin.ch/ch.swisstopo-vd.ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz/ortschaftenverzeichnis_plz_2056.csv.zip
  Stand: abgerufen 2026-11-06.
  Quellenangabe: Bundesamt für Landestopografie swisstopo.
  Nutzungsbedingungen: Nutzungsbedingungen für kostenlose Geodaten und Geodienste (OGD) von swisstopo, https://www.swisstopo.admin.ch/de/nutzungsbedingungen-kostenlose-geodaten-und-geodienste, Abschnitt 2: «Die kostenlosen Geodaten und Geodienste von swisstopo dürfen genutzt, verbreitet und zugänglich gemacht werden. Weiter dürfen sie angereichert, bearbeitet sowie auch kommerziell genutzt werden. Eine Quellenangabe ist Pflicht.»
- `lexika/register/strassentypen.tsv`: Amtliches Verzeichnis der Strassen (Schweiz und Liechtenstein): abgeleitete Strassentypen.
  Herausgeber: Bundesamt für Landestopografie swisstopo.
  Quelle: https://data.geo.admin.ch/ch.swisstopo.amtliches-strassenverzeichnis/amtliches-strassenverzeichnis_ch/amtliches-strassenverzeichnis_ch_2056.csv.zip
  Quelle (Liechtenstein): https://data.geo.admin.ch/ch.swisstopo.amtliches-strassenverzeichnis/amtliches-strassenverzeichnis_li/amtliches-strassenverzeichnis_li_2056.csv.zip
  Stand: abgerufen 2026-11-06.
  Quellenangabe: Bundesamt für Landestopografie swisstopo.
  Nutzungsbedingungen: Nutzungsbedingungen für kostenlose Geodaten und Geodienste (OGD) von swisstopo, https://www.swisstopo.admin.ch/de/nutzungsbedingungen-kostenlose-geodaten-und-geodienste, Abschnitt 2: «Die kostenlosen Geodaten und Geodienste von swisstopo dürfen genutzt, verbreitet und zugänglich gemacht werden. Weiter dürfen sie angereichert, bearbeitet sowie auch kommerziell genutzt werden. Eine Quellenangabe ist Pflicht.»
- `lexika/register/vornamen.tsv`: Vornamen der Bevölkerung nach Jahrgang, Schweiz, 2025 (männlich und weiblich, über alle Jahrgänge summiert).
  Herausgeber: Bundesamt für Statistik (BFS), Statistik der Bevölkerung und der Haushalte (STATPOP).
  Quelle: https://opendata.swiss/de/dataset/mannliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025
  Quelle: https://opendata.swiss/de/dataset/weibliche-vornamen-der-bevolkerung-nach-jahrgang-schweiz-2025
  Stand: veröffentlicht 2026-08-21, abgerufen 2026-11-06.
  Nutzungsbedingungen: opendata.swiss, Nutzungsbedingung «Freie Nutzung. Quellenangabe ist Pflicht.» (https://opendata.swiss/terms-of-use#terms_by): nicht kommerzielle und kommerzielle Nutzung erlaubt, Quellenangabe Pflicht (Autor, Titel und Link zum Datensatz). Quelle: BFS – STATPOP, © BFS 2026

### 7.3 Zitate und Gesetzestexte

Jeder Eintrag in `lexika/*.toml` trägt seine Quelle (`quelle`, `url`); `promptscan explain`
zeigt sie bei jeder Regel. Nach Quellen:

| Quelle | Einträge | Grundlage der Übernahme |
|---|---|---|
| Duden online | 30 | Zitat nach Art. 25 URG, mit Quelle und Adresse je Eintrag |
| Erlasse des Bundes (SR) | 650 | Art. 5 Abs. 1 Bst. a URG: nicht geschützt |
| Larousse | 12 | Zitat nach Art. 25 URG, mit Quelle und Adresse je Eintrag |
| Treccani | 8 | Zitat nach Art. 25 URG, mit Quelle und Adresse je Eintrag |
| Unicode CLDR | 57 | Unicode License V3 (Abschnitt 8.4) |
| weitere: Bundeskanzlei, BFS, Bundesgericht | 66 | einzelne Wörter und Abkürzungen, je Eintrag mit Fundstelle belegt |

Zitierte Stichwörter, Duden online: https://www.duden.de/rechtschreibung/Dr_, https://www.duden.de/rechtschreibung/Frau, https://www.duden.de/rechtschreibung/Fuersprecher, https://www.duden.de/rechtschreibung/Fuersprecherin, https://www.duden.de/rechtschreibung/Gen, https://www.duden.de/rechtschreibung/Herr, https://www.duden.de/rechtschreibung/Mandant, https://www.duden.de/rechtschreibung/Prof_, https://www.duden.de/rechtschreibung/RA, https://www.duden.de/rechtschreibung/Rechtsanwaeltin, https://www.duden.de/rechtschreibung/Rechtsanwalt, https://www.duden.de/rechtschreibung/Sankt, https://www.duden.de/rechtschreibung/der__die_das_bestimmte_Artikel, https://www.duden.de/rechtschreibung/ein_Artikel, https://www.duden.de/rechtschreibung/geb__geboren, https://www.duden.de/rechtschreibung/ihr_sie_ihre, https://www.duden.de/rechtschreibung/mein_Possessivpronomen, https://www.duden.de/rechtschreibung/sein_Pronomen, https://www.duden.de/rechtschreibung/unser_Possessivpronomen

Zitierte Stichwörter, Larousse: https://www.larousse.fr/dictionnaires/francais/docteur/26250, https://www.larousse.fr/dictionnaires/francais/ma%C3%AEtre/48732, https://www.larousse.fr/dictionnaires/francais/madame/48465, https://www.larousse.fr/dictionnaires/francais/mademoiselle/48473, https://www.larousse.fr/dictionnaires/francais/monsieur/52460, https://www.larousse.fr/dictionnaires/francais/professeur/64155, https://www.larousse.fr/dictionnaires/francais/saint/70547

Zitierte Stichwörter, Treccani: https://www.treccani.it/enciclopedia/abbreviazioni_(Enciclopedia-dell'Italiano)/, https://www.treccani.it/vocabolario/avvocato/, https://www.treccani.it/vocabolario/dottore/, https://www.treccani.it/vocabolario/professore/, https://www.treccani.it/vocabolario/santo/, https://www.treccani.it/vocabolario/signora/, https://www.treccani.it/vocabolario/signore/, https://www.treccani.it/vocabolario/signorina/

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

### 8.1 Crates

#### Unlicense (UNLICENSE): aho-corasick 1.1.5, memchr 2.8.3

```text
This is free and unencumbered software released into the public domain.

Anyone is free to copy, modify, publish, use, compile, sell, or
distribute this software, either in source code form or as a compiled
binary, for any purpose, commercial or non-commercial, and by any
means.

In jurisdictions that recognize copyright laws, the author or authors
of this software dedicate any and all copyright interest in the
software to the public domain. We make this dedication for the benefit
of the public at large and to the detriment of our heirs and
successors. We intend this dedication to be an overt act of
relinquishment in perpetuity of all present and future rights to this
software under copyright law.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

For more information, please refer to <http://unlicense.org/>
```

#### Erläuterung (COPYING): aho-corasick 1.1.5, memchr 2.8.3

```text
This project is dual-licensed under the Unlicense and MIT licenses.

You may use this code under the terms of either license.
```

#### MIT (LICENSE-MIT): anstream 1.0.0, anstyle 1.0.14, anstyle-parse 1.0.0, anstyle-query 1.1.5, anstyle-wincon 3.0.11, clap 4.6.7, clap_builder 4.6.7, clap_lex 1.1.1, colorchoice 1.0.5, is_terminal_polyfill 1.70.2, once_cell_polyfill 1.70.2, serde_spanned 1.1.2, toml 1.2.0+spec-1.1.0, toml_datetime 1.1.2+spec-1.1.0, toml_parser 1.1.5+spec-1.1.0, toml_writer 1.2.0+spec-1.1.0

```text
Copyright (c) Individual contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

#### MIT (LICENSE-MIT): itoa 1.0.18, serde 1.0.229, serde_core 1.0.229, serde_json 1.0.154, zmij 1.0.23

```text
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### MIT (LICENSE-MIT): regex 1.13.1, regex-automata 0.4.18, regex-syntax 0.8.11

```text
Copyright (c) 2014 The Rust Project Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### weitere Lizenzdatei (src/unicode_tables/LICENSE-UNICODE): regex-syntax 0.8.11

```text
UNICODE, INC. LICENSE AGREEMENT - DATA FILES AND SOFTWARE

Unicode Data Files include all data files under the directories
http://www.unicode.org/Public/, http://www.unicode.org/reports/,
http://www.unicode.org/cldr/data/, http://source.icu-project.org/repos/icu/, and
http://www.unicode.org/utility/trac/browser/.

Unicode Data Files do not include PDF online code charts under the
directory http://www.unicode.org/Public/.

Software includes any source code published in the Unicode Standard
or under the directories
http://www.unicode.org/Public/, http://www.unicode.org/reports/,
http://www.unicode.org/cldr/data/, http://source.icu-project.org/repos/icu/, and
http://www.unicode.org/utility/trac/browser/.

NOTICE TO USER: Carefully read the following legal agreement.
BY DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING UNICODE INC.'S
DATA FILES ("DATA FILES"), AND/OR SOFTWARE ("SOFTWARE"),
YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT.
IF YOU DO NOT AGREE, DO NOT DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE
THE DATA FILES OR SOFTWARE.

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2018 Unicode, Inc. All rights reserved.
Distributed under the Terms of Use in http://www.unicode.org/copyright.html.

Permission is hereby granted, free of charge, to any person obtaining
a copy of the Unicode data files and any associated documentation
(the "Data Files") or Unicode software and any associated documentation
(the "Software") to deal in the Data Files or Software
without restriction, including without limitation the rights to use,
copy, modify, merge, publish, distribute, and/or sell copies of
the Data Files or Software, and to permit persons to whom the Data Files
or Software are furnished to do so, provided that either
(a) this copyright and permission notice appear with all copies
of the Data Files or Software, or
(b) this copyright and permission notice appear in associated
Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT OF THIRD PARTY RIGHTS.
IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS
NOTICE BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL
DAMAGES, OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE,
DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THE DATA FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in these Data Files or Software without prior
written authorization of the copyright holder.
```

#### MIT (LICENSE): strsim 0.11.1

```text
The MIT License (MIT)

Copyright (c) 2015 Danny Guo
Copyright (c) 2016 Titus Wormer <tituswormer@gmail.com>
Copyright (c) 2018 Akash Kurdekar

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

#### Zlib (LICENSE-ZLIB.md): tinyvec 1.13.3

```text
Copyright (c) 2019 Daniel "Lokathor" Gee.

This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not required.

2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original software.

3. This notice may not be removed or altered from any source distribution.
```

#### MIT (LICENSE-MIT): unicode-normalization 0.1.26, unicode-segmentation 1.13.3, gimli 0.34.0

```text
Copyright (c) 2015 The Rust Project Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### Unicode-3.0 (LICENSE-UNICODE): unicode-normalization 0.1.26

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2023 Unicode, Inc.

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

#### MIT (LICENSE-MIT): utf8parse 0.2.2

```text
Copyright (c) 2016 Joe Wilm

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### MIT (license-mit): windows-link 0.2.1, windows-sys 0.61.2

```text
    MIT License

    Copyright (c) Microsoft Corporation.

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE
```

#### MIT (LICENSE-MIT): winnow 1.0.5

```text
Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

#### MIT des übernommenen Codes (tools/drittvermerke/zusatz/zmij-original-LICENSE): zmij 1.0.23

zmij ist laut README.md und src/lib.rs des Pakets eine Zeile für Zeile übertragene Fassung eines C++-Originals; dessen Lizenzdatei (Datei LICENSE am Commit b32688a1849d1b8f092c1740c61a6b05ea4679ae, auf den README.md verweist, abgerufen am 06.11.2026, SHA-256 a69b36adb8d116376bcd580772fa45f08fffecc17833cb9822532f4e4f5ec372) trägt einen eigenen Copyright-Vermerk.

```text
MIT License

Copyright (c) 2025 Victor Zverovich

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

#### MIT (LICENSE-MIT): addr2line 0.27.1

```text
Copyright (c) 2016-2018 The gimli Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### 0BSD (LICENSE-0BSD): adler2 2.0.1

```text
Copyright (C) Jonas Schievink <jonasschievink@gmail.com>

Permission to use, copy, modify, and/or distribute this software for
any purpose with or without fee is hereby granted.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN
AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
```

#### MIT (LICENSE-MIT): cfg-if 1.0.4, rustc-demangle 0.1.28

```text
Copyright (c) 2014 Alex Crichton

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### MIT (LICENSE-MIT): hashbrown 0.17.1

```text
Copyright (c) 2016 Amanieu d'Antras

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### MIT (LICENSE-MIT): libc 0.2.189

```text
Copyright (c) The Rust Project Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

#### Zlib (LICENSE-ZLIB.md): miniz_oxide 0.9.1

```text
Copyright 2013-2014 RAD Game Tools and Valve Software
Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC
Copyright (c) 2020 Frommi
Copyright (c) 2017-2024 oyvindln

This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software.

Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not claim that you wrote the original software. If you use this software in a product, an acknowledgment in the product documentation would be appreciated but is not required.

2. Altered source versions must be plainly marked as such, and must not be misrepresented as being the original software.

3. This notice may not be removed or altered from any source distribution.
```

#### MIT (LICENSE-MIT): object 0.39.1

```text
Copyright (c) 2015 The Gimli Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

### 8.2 Rust-Standardbibliothek (MIT gewählt)

Copyright-Angaben aus COPYRIGHT-library.html, Lizenztext aus `share/doc/rust/licenses/MIT.txt`:

```text
Copyright (c) The Rust Project Developers (see https://thanks.rust-lang.org)
Copyright (c) 2014 Alex Crichton
Copyright (c) 2019 The Crossbeam Project Developers

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

### 8.3 compiler_builtins und libm

#### compiler_builtins (library/compiler-builtins, MIT AND Apache-2.0 WITH LLVM-exception AND (MIT OR Apache-2.0))

```text
The compiler-builtins crate is available for use under both the MIT license
and the Apache-2.0 license with the LLVM exception (MIT AND Apache-2.0 WITH
LLVM-exception).

The libm crate is available for use under the MIT license.

As a contributor, you agree that your code may be used under any of the
following: the MIT license, the Apache-2.0 license, or the Apache-2.0 license
with the LLVM exception. In other words, original (non-derivative) work is
licensed under MIT OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception. This is
the default license for all other source in this repository.

Text of the relevant licenses is provided below:

------------------------------------------------------------------------------
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
------------------------------------------------------------------------------
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

    TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

    1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

    2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

    3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

    4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

    5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

    6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

    7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

    8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

    9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

    END OF TERMS AND CONDITIONS

    APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.


---- LLVM Exceptions to the Apache 2.0 License ----

As an exception, if, as a result of your compiling your source code, portions
of this Software are embedded into an Object form of such source code, you
may redistribute such embedded portions in such Object form without complying
with the conditions of Sections 4(a), 4(b) and 4(d) of the License.

In addition, if you combine or link compiled forms of this Software with
software that is licensed under the GPLv2 ("Combined Software") and if a
court of competent jurisdiction determines that the patent provision (Section
3), the indemnity provision (Section 9) or other Section of the License
conflicts with the conditions of the GPLv2, you may retroactively and
prospectively choose to deem waived or otherwise exclude such Section(s) of
the License, but only in their entirety and only with respect to the Combined
Software.
------------------------------------------------------------------------------

Portions of this software are derived from third-party works licensed under
terms compatible with the above Apache-2.0 WITH LLVM-exception AND MIT
license:

* compiler-builtins is derived from LLVM's compiler-rt (https://llvm.org/).
  Work derived from compiler-rt prior to 2019-01-19 is usable under the MIT
  license, with the following copyright:

      Copyright (c) 2009-2016 by the contributors listed in CREDITS.TXT

  The relevant CREDITS.TXT is located at
  https://github.com/llvm/llvm-project/blob/main/compiler-rt/CREDITS.TXT.

* Work derived from compiler-rt after 2019-01-19 is usable under the
  Apache-2.0 license with the LLVM exception.

* The bundled `math` module is from the libm crate, usable under the MIT
  license. For further details and copyrights, see see libm/LICENSE.txt at
  https://github.com/rust-lang/compiler-builtins.

Additionally, some source files may contain comments with specific copyrights
or licenses.
```

#### libm (in compiler_builtins) (library/compiler-builtins/libm, MIT)

```text
rust-lang/libm as a whole is available for use under the MIT license:

------------------------------------------------------------------------------
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
------------------------------------------------------------------------------

As a contributor, you agree that your code can be used under either the MIT
license or the Apache-2.0 license:

------------------------------------------------------------------------------
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
------------------------------------------------------------------------------

This Rust library contains the following copyrights:

    Copyright (c) 2018 Jorge Aparicio

Portions of this software are derived from third-party works licensed under
terms compatible with the above MIT license:

* musl libc https://www.musl-libc.org/. This library contains the following
  copyright:

      Copyright © 2005-2020 Rich Felker, et al.

* The CORE-MATH project https://core-math.gitlabpages.inria.fr/. CORE-MATH
  routines are available under the MIT license on a per-file basis.

The musl libc COPYRIGHT file also includes the following notice relevant to
math portions of the library:

------------------------------------------------------------------------------
Much of the math library code (src/math/* and src/complex/*) is
Copyright © 1993,2004 Sun Microsystems or
Copyright © 2003-2011 David Schultz or
Copyright © 2003-2009 Steven G. Kargl or
Copyright © 2003-2009 Bruce D. Evans or
Copyright © 2008 Stephen L. Moshier or
Copyright © 2017-2018 Arm Limited
and labelled as such in comments in the individual source files. All
have been licensed under extremely permissive terms.
------------------------------------------------------------------------------

Copyright notices are retained in src/* files where relevant.
```

### 8.4 Unicode

#### Rust-Standardbibliothek, library/core/src/unicode (share/doc/rust/licenses/Unicode-3.0.txt)

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 1991-2024 Unicode, Inc.

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

#### Unicode-Daten 18.0.0 im Breitenmodell, src/ausgabe.rs (https://www.unicode.org/license.txt, abgerufen am 06.11.2026)

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

#### Unicode CLDR, Vermerk aus dem Kopf von lexika/abkuerzungen.toml

```text
Copyright © 2001–2026 Unicode, Inc.

UNICODE LICENSE V3

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

SPDX-License-Identifier: Unicode-3.0
```

#### Unicode CLDR, Vermerk aus dem Kopf von lexika/geburtsdatum-monate.toml

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright © 2015-2024 Unicode, Inc.

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

SPDX-License-Identifier: Unicode-3.0
```

### 8.5 MinGW-w64-Laufzeit (COPYING.MinGW-w64-runtime.txt)

```text
MinGW-w64 runtime licensing
***************************

This program or library was built using MinGW-w64 and statically
linked against the MinGW-w64 runtime. Some parts of the runtime
are under licenses which require that the copyright and license
notices are included when distributing the code in binary form.
These notices are listed below.


========================
Overall copyright notice
========================

Copyright (c) 2009, 2010, 2011, 2012, 2013 by the mingw-w64 project

This license has been certified as open source. It has also been designated
as GPL compatible by the Free Software Foundation (FSF).

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

   1. Redistributions in source code must retain the accompanying copyright
      notice, this list of conditions, and the following disclaimer.
   2. Redistributions in binary form must reproduce the accompanying
      copyright notice, this list of conditions, and the following disclaimer
      in the documentation and/or other materials provided with the
      distribution.
   3. Names of the copyright holders must not be used to endorse or promote
      products derived from this software without prior written permission
      from the copyright holders.
   4. The right to distribute this software or to use it for any purpose does
      not give you the right to use Servicemarks (sm) or Trademarks (tm) of
      the copyright holders.  Use of them is covered by separate agreement
      with the copyright holders.
   5. If any files are modified, you must cause the modified files to carry
      prominent notices stating that you changed the files and the date of
      any change.

Disclaimer

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY EXPRESSED
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA,
OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

========================================
getopt, getopt_long, and getop_long_only
========================================

Copyright (c) 2002 Todd C. Miller <Todd.Miller@courtesan.com>

Permission to use, copy, modify, and distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

Sponsored in part by the Defense Advanced Research Projects
Agency (DARPA) and Air Force Research Laboratory, Air Force
Materiel Command, USAF, under agreement number F39502-99-1-0512.

        *       *       *       *       *       *       *

Copyright (c) 2000 The NetBSD Foundation, Inc.
All rights reserved.

This code is derived from software contributed to The NetBSD Foundation
by Dieter Baron and Thomas Klausner.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
 1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
 2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.


===============================================================
gdtoa: Converting between IEEE floating point numbers and ASCII
===============================================================

The author of this software is David M. Gay.

Copyright (C) 1997, 1998, 1999, 2000, 2001 by Lucent Technologies
All Rights Reserved

Permission to use, copy, modify, and distribute this software and
its documentation for any purpose and without fee is hereby
granted, provided that the above copyright notice appear in all
copies and that both that the copyright notice and this
permission notice and warranty disclaimer appear in supporting
documentation, and that the name of Lucent or any of its entities
not be used in advertising or publicity pertaining to
distribution of the software without specific, written prior
permission.

LUCENT DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.
IN NO EVENT SHALL LUCENT OR ANY OF ITS ENTITIES BE LIABLE FOR ANY
SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER
IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.

        *       *       *       *       *       *       *

The author of this software is David M. Gay.

Copyright (C) 2005 by David M. Gay
All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that the copyright notice and this permission notice and warranty
disclaimer appear in supporting documentation, and that the name of
the author or any of his current or former employers not be used in
advertising or publicity pertaining to distribution of the software
without specific, written prior permission.

THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.  IN
NO EVENT SHALL THE AUTHOR OR ANY OF HIS CURRENT OR FORMER EMPLOYERS BE
LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY
DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

        *       *       *       *       *       *       *

The author of this software is David M. Gay.

Copyright (C) 2004 by David M. Gay.
All Rights Reserved
Based on material in the rest of /netlib/fp/gdota.tar.gz,
which is copyright (C) 1998, 2000 by Lucent Technologies.

Permission to use, copy, modify, and distribute this software and
its documentation for any purpose and without fee is hereby
granted, provided that the above copyright notice appear in all
copies and that both that the copyright notice and this
permission notice and warranty disclaimer appear in supporting
documentation, and that the name of Lucent or any of its entities
not be used in advertising or publicity pertaining to
distribution of the software without specific, written prior
permission.

LUCENT DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.
IN NO EVENT SHALL LUCENT OR ANY OF ITS ENTITIES BE LIABLE FOR ANY
SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER
IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.


=========================
Parts of the math library
=========================

Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.

Developed at SunSoft, a Sun Microsystems, Inc. business.
Permission to use, copy, modify, and distribute this
software is freely granted, provided that this notice
is preserved.

        *       *       *       *       *       *       *

Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.

Developed at SunPro, a Sun Microsystems, Inc. business.
Permission to use, copy, modify, and distribute this
software is freely granted, provided that this notice
is preserved.

        *       *       *       *       *       *       *

FIXME: Cephes math lib
Copyright (C) 1984-1998 Stephen L. Moshier

It sounds vague, but as to be found at
<http://lists.debian.org/debian-legal/2004/12/msg00295.html>, it gives an
impression that the author could be willing to give an explicit
permission to distribute those files e.g. under a BSD style license. So
probably there is no problem here, although it could be good to get a
permission from the author and then add a license into the Cephes files
in MinGW runtime. At least on follow-up it is marked that debian sees the
version a-like BSD one. As MinGW.org (where those cephes parts are coming
from) distributes them now over 6 years, it should be fine.

=================================================
Some string, memory and time conversion functions
=================================================

Copyright © 2005-2020 Rich Felker, et al.

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

===================================
Headers and IDLs imported from Wine
===================================

Some header and IDL files were imported from the Wine project. These files
are prominent maked in source. Their copyright belongs to contributors and
they are distributed under LGPL license.

Disclaimer

This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Lesser General Public
License as published by the Free Software Foundation; either
version 2.1 of the License, or (at your option) any later version.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Lesser General Public License for more details.
```

### 8.6 GCC Runtime Library Exception 3.1 (libgcc)

```text
GCC RUNTIME LIBRARY EXCEPTION

Version 3.1, 31 March 2009

Copyright (C) 2009 Free Software Foundation, Inc. <http://fsf.org/>

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

This GCC Runtime Library Exception ("Exception") is an additional
permission under section 7 of the GNU General Public License, version
3 ("GPLv3"). It applies to a given file (the "Runtime Library") that
bears a notice placed by the copyright holder of the file stating that
the file is governed by GPLv3 along with this Exception.

When you use GCC to compile a program, GCC may combine portions of
certain GCC header files and runtime libraries with the compiled
program. The purpose of this Exception is to allow compilation of
non-GPL (including proprietary) programs to use, in this way, the
header files and runtime libraries covered by this Exception.

0. Definitions.

A file is an "Independent Module" if it either requires the Runtime
Library for execution after a Compilation Process, or makes use of an
interface provided by the Runtime Library, but is not otherwise based
on the Runtime Library.

"GCC" means a version of the GNU Compiler Collection, with or without
modifications, governed by version 3 (or a specified later version) of
the GNU General Public License (GPL) with the option of using any
subsequent versions published by the FSF.

"GPL-compatible Software" is software whose conditions of propagation,
modification and use would permit combination with GCC in accord with
the license of GCC.

"Target Code" refers to output from any compiler for a real or virtual
target processor architecture, in executable form or suitable for
input to an assembler, loader, linker and/or execution
phase. Notwithstanding that, Target Code does not include data in any
format that is used as a compiler intermediate representation, or used
for producing a compiler intermediate representation.

The "Compilation Process" transforms code entirely represented in
non-intermediate languages designed for human-written code, and/or in
Java Virtual Machine byte code, into Target Code. Thus, for example,
use of source code generators and preprocessors need not be considered
part of the Compilation Process, since the Compilation Process can be
understood as starting with the output of the generators or
preprocessors.

A Compilation Process is "Eligible" if it is done using GCC, alone or
with other GPL-compatible software, or if it is done without using any
work based on GCC. For example, using non-GPL-compatible Software to
optimize any GCC intermediate representations would not qualify as an
Eligible Compilation Process.

1. Grant of Additional Permission.

You have permission to propagate a work of Target Code formed by
combining the Runtime Library with Independent Modules, even if such
propagation would otherwise violate the terms of GPLv3, provided that
all Target Code was generated by Eligible Compilation Processes. You
may then convey such a combination under terms of your choice,
consistent with the licensing of the Independent Modules.

2. No Weakening of GCC Copyleft.

The availability of this Exception does not imply any general
presumption that third-party software is unaffected by the copyleft
requirements of the license of GCC.
```

### 8.7 Apache License 2.0 und NOTICE-Dateien

Kein Crate aus den Abschnitten 2 und 5 wird unter Apache-2.0 genutzt. Der Volltext der Apache License 2.0 steht unverändert im Lizenztext von compiler_builtins (Abschnitt 8.3), zusammen mit den LLVM-Ausnahmen; er gilt dort für die aus LLVM compiler-rt abgeleiteten Teile.

NOTICE-Dateien: keine. Gesucht in allen Paketen der Abschnitte 2 und 5 (Dateien, deren Name mit «NOTICE» beginnt, auch in Unterordnern).

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
