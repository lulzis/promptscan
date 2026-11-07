# Weitergabe der Programmdatei

Stand 06.11.2026. Gilt für die Windows-Programmdatei (Rust-Ziel `x86_64-pc-windows-gnu`).
Wann weitergegeben wird, regelt SPEC 8 (lokal, privates Repo, öffentlich).

## 1. Was weitergegeben wird

- **`promptscan.exe`**, eine einzige Datei, gebaut mit `tools/freigabe.ps1` (Abschnitt 5) im
  Profil `release` (`lto = true`, `codegen-units = 1`, `strip = true`). Sie braucht weder Rust
  noch MinGW noch eine Visual-C++-Laufzeit: Ihre Importtabelle nennt nur kernel32, ntdll,
  msvcrt, bcryptprimitives und api-ms-win-core-synch-l1-2-0. Das prüft `tests/offline.rs`
  (SPEC 6).
- **`promptscan.exe.sha256`**, die Prüfsumme (Abschnitt 4.3).
- **`LICENSE.md` und `THIRD-PARTY-NOTICES.md`**, von `tools/freigabe.ps1` neben die
  Programmdatei gelegt. Beide sind auch in die Programmdatei eingebettet; `promptscan licenses`
  gibt sie aus (Abschnitt 6). Die Vermerke gehen also auch mit, wenn nur die Programmdatei
  weitergegeben wird.

## 2. Mindestanforderungen

| System | läuft | Beleg |
| --- | --- | --- |
| Windows 10, Windows 11 (x64) | ja | Rust, Platform Support: «x86_64-pc-windows-gnu 64-bit MinGW (Windows 10+, Windows Server 2016+)» |
| Windows Server 2016 und neuer | ja | ebenda |
| Windows 11 on Arm | ja, über x64-Emulation | Microsoft: «Windows 11 on Arm supports emulation of both x86 and x64 apps.» |
| Windows 10 on Arm | nein | Microsoft: «Windows 10 on Arm also supports emulation, but only for x86 apps.» |
| ältere Windows, 32-Bit-Windows | nein | Ziel verlangt Windows 10+ und 64 Bit |

Geprüft ist bisher nur Windows 11 Pro x64 (Build 26200).

Quellen: <https://doc.rust-lang.org/rustc/platform-support.html>;
<https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-x86-emulation> (Stand 06.11.2025).

## 3. Was Windows mit einer unsignierten Programmdatei tut

Die Programmdatei ist heute nicht signiert und hat keine Ressourcen (keine Versionsangaben,
kein Manifest).

**Mark of the Web.** Eine heruntergeladene Datei trägt eine Herkunftsmarke. Microsoft zu
`Unblock-File` (Windows PowerShell 5.1): «Internally, the Unblock-File cmdlet removes the
Zone.Identifier alternate data stream, which has a value of 3 to indicate that it was downloaded
from the internet.»

**SmartScreen.** Microsoft: SmartScreen prüft durch «Checking downloaded files against a list of
files that are well known and downloaded frequently. If the file isn't on that list, Microsoft
Defender SmartScreen shows a warning, advising caution.» Und: «If a URL, a file, an app, or a
certificate has an established reputation, users don't see any warnings. If there's no
reputation, the item is marked as a higher risk and presents a warning to the user.» Eine neue,
unsignierte Datei hat keine Reputation.

**Smart App Control.** Microsoft beschreibt zwei Schritte: «When you try to run an app on
Windows, Smart App Control will check to see if our intelligent cloud-powered security service
can make a confident prediction about its safety. If the service believes the app to be safe,
Smart App Control will let it run. If the app is believed to be malicious or potentially
unwanted, then Smart App Control will block it. If the security service is unable to make a
confident prediction about the app, then Smart App Control checks to see if the app has a valid
signature. If the app has a valid signature, Smart App Control will let it run. If the app is
unsigned, or the signature is invalid, Smart App Control will consider it untrusted and block it
for your protection.» Und: «There is currently no way to bypass Smart App Control protection
for individual apps.» Auf einem Gerät mit eingeschaltetem Smart App Control startet die
unsignierte Programmdatei also nur, wenn der Cloud-Dienst sie als sicher einstuft; kann er
keine sichere Vorhersage treffen, sperrt Smart App Control sie. Ob er das bei einer neuen Datei
kann, sagt Microsoft nicht voraus: «However, in some cases, the service is unable to make a
confident prediction either way.» Der Rat an Entwickler: «The simple answer is, sign your app
with a valid certificate.»

Folge für den Nutzertest (SPEC 7.6): vorher klären, ob Smart App Control auf den Geräten der
Testpersonen eingeschaltet ist. Für die öffentliche Stufe ist die Signatur nötig (Abschnitt 4).

Quellen: <https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/unblock-file?view=powershell-5.1>
(Stand 12.12.2022);
<https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/>
(Stand 23.04.2026);
<https://support.microsoft.com/en-us/windows/security/threat-malware-protection/smart-app-control-frequently-asked-questions>
(Stand 17.08.2026).

## 4. Plan vor der Weitergabe an Fremde

### 4.1 Authenticode-Signatur mit Zeitstempel

- **Zertifikat.** Microsoft: «Code can be signed with any certificate, but Smart App Control
  only considers certificates issued by trusted providers.» Und: «Artifact Signing is the
  preferred way to sign your app.» Wahl des Anbieters und Kosten: offen.
- **Werkzeug.** «Signtool.exe is an app included with Visual Studio that can sign apps with a
  digital certificate.» Auf dem Entwicklungsrechner steht es nicht im Pfad (geprüft
  06.11.2026).
- **Aufruf** (Platzhalter in spitzen Klammern):

  ```text
  signtool sign /fd SHA256 /tr <URL des Zeitstempeldienstes> /td SHA256 <Zertifikatswahl> promptscan.exe
  signtool verify /pa promptscan.exe
  ```

  SignTool-Dokumentation: «/fd alg Specifies the file digest algorithm to use for creating file
  signatures.» «/tr URL Specifies the URL of the RFC 3161 time stamp server. If this option or
  /t isn't specified, the signed file isn't time stamped.» «/td alg Used with the /tr option to
  request a digest algorithm used by the RFC 3161 time stamp server.» «/pa Specifies that the
  Default Authentication Verification Policy is used.»
- **Reihenfolge:** bauen und prüfen (`tools/freigabe.ps1`), signieren, dann die Prüfsumme neu
  bilden; die Signatur ändert die Datei.

Quellen: <https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control>
(Stand 25.09.2026); <https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool>
(Stand 21.11.2024).

### 4.2 Versionsangaben und Manifest

- **VERSIONINFO.** Microsoft: «The resource contains such information about the file as its
  version number, its intended operating system, and its original filename.»
- **Manifest mit `supportedOS`.** Microsoft nennt für Windows 10 und 11
  `{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`: «Windows 10, Windows 11, Windows Server 2016, Windows
  Server 2019 and Windows Server 2022».
- **Hindernis: kein Ressourcen-Compiler.** Die Werkzeugkette `stable-x86_64-pc-windows-gnu`
  (Rust 1.99.1) bringt keinen mit. Geprüft am 06.11.2026: weder `windres` (auch nicht
  `x86_64-w64-mingw32-windres`) noch `rc.exe`, `llvm-rc` oder `llvm-windres`, weder im Pfad
  noch in der Werkzeugkette. Deren `x86_64-w64-mingw32-gcc.exe` dient nur dem Linken; die
  beiliegende `GCC-WARNING.txt` sagt: «gcc.exe contained in this folder cannot be used for
  compiling C files - it is only used as a linker.» Wege: `rc.exe` aus dem Windows SDK
  installieren, `llvm-rc` installieren oder ein Crate dafür aufnehmen (neue Abhängigkeit). Die
  übersetzte Ressource würde dann in `build.rs` an den Linker übergeben.

Quellen: <https://learn.microsoft.com/en-us/windows/win32/menurc/versioninfo-resource> (Stand
31.05.2018); <https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests> (Stand
08.10.2020).

### 4.3 Prüfsumme je Freigabe

`tools/freigabe.ps1` schreibt neben die Programmdatei `promptscan.exe.sha256` im Format von
`sha256sum` (Prüfsumme, zwei Leerzeichen, Dateiname). Die Prüfsumme gehört zu jeder Freigabe in
die Freigabenotiz, getrennt vom Download. Prüfen beim Empfänger:

```text
PowerShell:  Get-FileHash -Algorithm SHA256 .\promptscan.exe
Git Bash:    sha256sum -c promptscan.exe.sha256
```

## 5. Release-Bau ohne Entwicklerpfade: `tools/freigabe.ps1`

**Ausgangslage.** Panic-Meldungen der Crates tragen den Pfad ihrer Quelldatei. Ein gewöhnlicher
Release-Bau enthielt 115-mal einen Pfad der Form `C:\Users\<Konto>\.cargo\registry\src\…`, also
den Kontonamen des Entwicklers (gemessen am 06.11.2026). `strip = true` entfernt Symbole, nicht
diese Pfade.

**`trim-paths` geht noch nicht.** Cargo 1.99.1 lehnt `profile.release.trim-paths` ab: «feature
`trim-paths` is required … The package requires the Cargo feature called `trim-paths`, but that
feature is not stabilized in this version of Cargo» (Tracking-Issue rust-lang/cargo#12137).

**Lösung.** Das Skript übergibt `--remap-path-prefix` an rustc, über `cargo --config` unter
demselben Schlüssel, den die Cargo-Konfiguration schon nutzt (hier
`target.x86_64-pc-windows-gnu.rustflags`; bei `build.rustflags` dieser). Grund, aus der
Cargo-Dokumentation:

- «There are four mutually exclusive sources of extra flags. They are checked in order, with the
  first one being used: `CARGO_ENCODED_RUSTFLAGS` environment variable. `RUSTFLAGS` environment
  variable. All matching `target.<triple>.rustflags` and `target.<cfg>.rustflags` config entries
  joined together. `build.rustflags` config value.» `RUSTFLAGS` würde also die Flags der
  Konfiguration ersetzen, etwa das dort gesetzte `dlltool`. Das Skript bricht ab, wenn
  `RUSTFLAGS` oder `CARGO_ENCODED_RUSTFLAGS` gesetzt ist.
- «Arrays will be joined together with higher precedence items being placed later in the merged
  array.» Und zu `--config`: «Configuration values specified this way take precedence over
  environment variables, which take precedence over configuration files.» Die Flags aus
  `--config` kommen also zu den vorhandenen hinzu.

Abbildungen (bei mehreren passenden gilt die letzte, darum die allgemeinste zuerst):

| Pfad | wird zu |
| --- | --- |
| Benutzerprofil (`USERPROFILE`) | `/home` |
| `RUSTUP_HOME` (sonst `<Profil>\.rustup`) | `/rustup` |
| `CARGO_HOME` (sonst `<Profil>\.cargo`) | `/cargo` |
| Projektordner | `/promptscan` |
| Cargo-Zielordner | `/target` |

Aufruf im Projektordner:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File tools\freigabe.ps1 [-Zielordner <Ordner>] [-Jobs <n>] [-OhneImportpruefung] [-Selbsttest]
```

Schritte:

1. Bau mit `--release --locked`.
2. Zählung in UTF-8 (so legt Rust Pfade ab) und UTF-16LE, ohne Beachtung der Gross- und
   Kleinschreibung. Das Skript bricht ab, wenn ein Pfad aus der Tabelle, `C:\Users\`,
   `C:/Users/` oder der Kontoname in Pfadform vorkommt (`\<Konto>\`, `/<Konto>/`,
   `Users\<Konto>`, `Users/<Konto>`). Den Kontonamen allein zählt es nur und meldet ihn: Die
   eingebetteten Namens- und Ortsregister können ihn als Wort enthalten, ohne dass ein Pfad in
   der Datei steht.
3. Strenge Importprüfung (`cargo test --test offline` mit `PROMPTSCAN_EXE`, SPEC 6).
4. Prüfsumme (Abschnitt 4.3).
5. `LICENSE.md` und `THIRD-PARTY-NOTICES.md` neben die Programmdatei kopieren (Abschnitt 1).

`-Selbsttest` prüft nur die Zählung an einer künstlichen Probe (Pfad mit Umlaut in UTF-8 und
UTF-16LE; Kontoname in Pfadform und als Wort in einem Register), ohne Bau.

**Nachweis** (06.11.2026, Rust 1.99.1, eigener Zielordner):

| Zeichenfolge in der Programmdatei | vorher | nachher |
| --- | --- | --- |
| `C:\Users\` | 115 | 0 |
| Kontoname (genau) | 115 | 0 |
| `.cargo\registry` | 115 | 0 |
| `/cargo\registry` (abgebildet) | 0 | 115 |
| `/rustc/` (Standardbibliothek, schon neutral) | 58 | 58 |

Projektordner, Zielordner, `RUSTUP_HOME` und Profil: je 0. Die fünf Treffer für `/home` sind
vorher wie nachher Webadressen in den Daten (bfs.admin.ch, bger.ch), keine Pfade. Die strenge
Importprüfung bestand, die Prüfsumme stimmt mit `sha256sum -c` überein. «vorher» ist ein
Release-Bau des Quelltexts vor den Änderungen für die Weitergabe, ohne Abbildung
(4 808 704 Bytes); «nachher» der Bau mit `tools/freigabe.ps1` (4 910 080 Bytes, grösser vor
allem wegen der eingebetteten Vermerke).

**Zweiter Lauf** (06.11.2026, 19:54 bis 19:55, Skript in der heutigen Fassung, eigener
Zielordner, 4 916 736 Bytes): Alle Pfade aus der Tabelle, `C:\Users\`, `C:/Users/` und die vier
Pfadformen des Kontonamens je 0 (UTF-8 und UTF-16LE, ohne Gross/klein). Den Kontonamen als Wort
meldete das Skript 13-mal; die eingebetteten Register enthalten ihn genau 13-mal
(`lexika/register/nachnamen.tsv` 10, `vornamen.tsv` 3), alle Treffer sind also Namen, keine
Pfade. `/cargo\registry` 115, `/rustc/` 58 und `/home` 5 wie im ersten Lauf. Die strenge
Importprüfung bestand, `sha256sum -c` meldet OK, und `LICENSE.md` und `THIRD-PARTY-NOTICES.md`
liegen bytegleich neben der Programmdatei.

## 6. Drittvermerke

`THIRD-PARTY-NOTICES.md` nennt alles Fremde in der Programmdatei: die Crates (mit der Lizenz mit
der geringsten Vermerkpflicht, wo wählbar, und dem Vermerk für übernommenen Code aus
`tools/drittvermerke/zusatz.toml`), die Rust-Standardbibliothek, die gelinkten Teile der
MinGW-w64-Laufzeit und von libgcc, die Unicode-Daten, die Register von BFS und swisstopo, die
Wörterbuchzitate und die Gesetzestexte. Dazu führt sie ohne Lizenztexte die Pakete aus
Cargo.lock, die nicht in die Programmdatei gelangen (nur für den Bau, nicht gebaut, nur für
Tests). Erzeugt wird sie von `examples/drittvermerke.rs`; nachführen nach jeder Änderung an
Cargo.lock, an `lexika/`, an `tools/drittvermerke/` oder an der Werkzeugkette:

```text
cargo tree -e normal --target x86_64-pc-windows-gnu --prefix none --no-dedupe --offline | cargo run --example drittvermerke
```

Mit `-- --pruefen` vergleicht das Beispiel nur. Die Tests in `src/lizenzen.rs` schlagen an,
wenn ein Paket aus Cargo.lock, ein Register aus `lexika/register/` (Name und Angaben aus dem
Dateikopf), der Lizenzkopf einer Wortliste aus `lexika/` oder ein Zusatzvermerk in der Datei
fehlt oder nicht mehr wortgleich ist.

## 7. Offen vor der öffentlichen Stufe

- Lizenz und Rechteinhaber (SPEC 11, Punkt 6); bis dahin gilt LICENSE.md (Testfassung).
- Zertifikat und Signatur (4.1), Ressourcen-Compiler für VERSIONINFO und Manifest (4.2).
- cargo-deny mit dem Repo (SPEC 6).
