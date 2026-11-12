//! «promptscan sendet nichts und schreibt nur auf Befehl» als Test (SPEC 2, Grundsatz 5;
//! SPEC 6).
//!
//! Vier Prüfungen, jede mit Gegenprobe, die zeigt, dass sie anschlagen kann:
//!
//! 1. Quelltext (`src/`, `build.rs`, `examples/`): kein Netzwerkzugriff, kein Start anderer
//!    Programme, kein Nachladen von Bibliotheken, keine zusätzlich gelinkte Bibliothek ausser
//!    kernel32 (`#[link(…)]`, auch mit Leerraum vor der Klammer, und `link!(…)` von
//!    windows-link).
//! 2. Quelltext des Programms (`src/`): Dateien anlegen, schreiben, kopieren, umbenennen oder
//!    löschen nur an den Stellen in `ERLAUBTE_SCHREIBSTELLEN`, die ein ausdrücklicher Befehl
//!    auslöst (`promptscan config --role …` bzw. `--lang …`, `--output`).
//! 3. Cargo.lock: keine Netzwerkbibliothek; Cargo.toml: keine Netzwerkfunktionen von
//!    windows-sys bzw. windows (Features `Win32_Networking*`, `Win32_NetworkManagement*`).
//! 4. Programmdatei: Die Importtabelle (PE) nennt nur freigegebene Systembibliotheken und keine
//!    Funktion, die Netz, Programmstart (auch über die C-Laufzeit: system, _popen, _spawn*,
//!    _exec*) oder Nachladen von Bibliotheken erlaubt. Gelesen wird mit einem eigenen kleinen
//!    Parser, normale und verzögerte Importe.
//!
//! Zu 4, Testbau und Release-Bau: `cargo test` prüft die Programmdatei des Testbaus. Ohne
//! Optimierung und LTO stehen darin zusätzlich 25 Funktionen aus WS2_32.dll,
//! GetUserProfileDirectoryW aus USERENV.dll und CreateProcessW aus der Standardbibliothek,
//! obwohl promptscan sie nicht aufruft; im Release-Bau mit `lto = true` fehlen sie (gemessen am
//! 06.11.2026, Rust 1.99.1). Die strenge Liste gilt darum für jeden optimierten Bau und für die
//! Datei in der Umgebungsvariable `PROMPTSCAN_EXE`; `tools/freigabe.ps1` setzt sie auf die
//! Release-Datei. Im Testbau ohne Optimierung sind genau diese Funktionen namentlich zusätzlich
//! zugelassen (`NUR_TESTBAU_IMPORTE`); jede andere Funktion dieser DLLs, jede andere
//! Netzwerk-DLL und jede andere verbotene Funktion schlägt auch dort an. Meldet der Testbau
//! nach einer Änderung am Programm eine neue Funktion der Standardbibliothek, zuerst die
//! Release-Datei streng prüfen (`PROMPTSCAN_EXE`), erst dann die Liste ergänzen.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Muster, die im Quelltext Netzwerkzugriff, den Start anderer Programme oder das Nachladen
/// von Bibliotheken bedeuten. «Command::new» träfe auch `clap::Command::new`; promptscan nutzt
/// clap über `derive`. Die Funktionen der C-Laufzeit (`_spawnl…`, `_execv…`, `fn system(`)
/// stehen hier zusätzlich zur Importprüfung, die sie in jedem Bau findet.
const VERBOTEN_IM_CODE: &[&str] = &[
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "ToSocketAddrs",
    "Networking",
    "WinSock",
    "process::Command",
    "Command::new",
    "std::process as",
    "CreateProcess",
    "NtCreateUserProcess",
    "NtCreateProcess",
    "ZwCreateUserProcess",
    "ZwCreateProcess",
    "RtlCreateUserProcess",
    "WinExec",
    "ShellExecute",
    "LoadModule",
    "fn system(",
    "_wsystem",
    "_popen",
    "_wpopen",
    "_spawnl",
    "_spawnv",
    "_wspawnl",
    "_wspawnv",
    "_execl",
    "_execv",
    "_wexecl",
    "_wexecv",
    "URLDownloadTo",
    "LoadLibrary",
    "LdrLoadDll",
    "GetProcAddress",
];

/// Muster, mit denen das Programm Dateien anlegen, schreiben, kopieren, umbenennen oder löschen
/// würde. Geprüft wird nur `src/`: `build.rs` schreibt beim Bauen nach OUT_DIR, und
/// `examples/drittvermerke.rs` erzeugt THIRD-PARTY-NOTICES.md; beide gehören nicht zur
/// Programmdatei.
const SCHREIBEN_IM_PROGRAMM: &[&str] = &[
    "fs::write",
    "File::create",
    "File::options",
    "OpenOptions",
    "create_dir",
    "fs::copy",
    "fs::rename",
    "remove_file",
    "remove_dir",
    "hard_link",
    "symlink",
    "set_permissions",
];

/// Bibliotheken, die Netzwerkzugriff bringen.
const VERBOTEN_IN_LOCK: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "curl",
    "curl-sys",
    "isahc",
    "attohttpc",
    "surf",
    "h2",
    "socket2",
    "mio",
    "tokio",
    "async-std",
    "native-tls",
    "rustls",
    "openssl",
    "openssl-sys",
    "tungstenite",
    "websocket",
];

/// Anfänge von Features der Crates windows-sys und windows, die Netzwerkfunktionen einbinden
/// (WinSock, WinHttp, WinInet, DNS, IP-Hilfsfunktionen u. a.).
const VERBOTENE_FEATURES: &[&str] = &["Win32_Networking", "Win32_NetworkManagement"];

/// Die einzige Bibliothek, die der eigene Quelltext mit `#[link(name = …)]` oder
/// `link!("…")` nennen darf: Konsolenfunktionen wie GetConsoleProcessList,
/// GetConsoleScreenBufferInfo und ReadConsoleW.
const ERLAUBT_IN_LINK: &[&str] = &["kernel32"];

/// DLLs, die die Programmdatei laden darf (Vergleich ohne Gross- und Kleinschreibung), mit
/// Herkunft laut Linker-Map des Release-Baus vom 06.11.2026 (Option `-Wl,-Map`).
const ERLAUBTE_DLLS: &[(&str, &str)] = &[
    (
        "kernel32.dll",
        "Windows-Grundfunktionen (Dateien, Speicher, Threads, Konsole): Startcode der \
         MinGW-w64-Laufzeit (crt2.o), Standardbibliothek, windows-sys (anstyle-query, \
         anstyle-wincon). Darunter GetModuleHandleA/W und GetProcAddress: die \
         Kompatibilitätsschicht der Standardbibliothek sucht damit SetThreadDescription, \
         GetThreadDescription und GetTempPath2W in kernel32 \
         (library/std/src/sys/pal/windows/compat.rs und c.rs, Rust 1.99.1).",
    ),
    (
        "ntdll.dll",
        "NtReadFile, NtWriteFile, RtlNtStatusToDosError: Ein- und Ausgabe über Handles \
         (Importbibliothek libntdll.a und windows-sys).",
    ),
    (
        "msvcrt.dll",
        "C-Laufzeit (Programmstart, malloc, memcpy): MinGW-w64-Laufzeit (crt2.o).",
    ),
    (
        "api-ms-win-core-synch-l1-2-0.dll",
        "WaitOnAddress, WakeByAddressSingle, WakeByAddressAll: Standardbibliothek (Warten und \
         Wecken von Threads; Importeintrag aus libstd).",
    ),
    (
        "bcryptprimitives.dll",
        "ProcessPrng: Standardbibliothek (Zufallszahlen; Importeintrag aus libstd).",
    ),
];

/// DLLs für Netz, Fernzugriff oder die Shell: in keinem Bau erlaubt (ausser den Funktionen in
/// `NUR_TESTBAU_IMPORTE` im Testbau ohne Optimierung).
const VERBOTENE_DLLS: &[&str] = &[
    "ws2_32.dll",
    "wsock32.dll",
    "mswsock.dll",
    "wininet.dll",
    "winhttp.dll",
    "urlmon.dll",
    "webio.dll",
    "httpapi.dll",
    "iphlpapi.dll",
    "dnsapi.dll",
    "netapi32.dll",
    "wldap32.dll",
    "rasapi32.dll",
    "mpr.dll",
    "shell32.dll",
];

/// Anfänge von Funktionsnamen, die Programme starten, Dateien aus dem Netz holen oder
/// Bibliotheken nachladen (Vergleich mit Beachtung der Gross- und Kleinschreibung, darum trifft
/// «system» nicht GetSystemInfo). Die C-Laufzeit msvcrt.dll exportiert 36 Startfunktionen:
/// system, _wsystem, _popen, _wpopen und je acht _execl…/_execv…, _wexecl…/_wexecv…,
/// _spawnl…/_spawnv… und _wspawnl…/_wspawnv… (Exporttabelle von msvcrt.dll, Windows 11,
/// 06.11.2026). ntdll.dll führt die Systemaufrufe auch unter «Zw…».
const VERBOTENE_FUNKTIONEN: &[&str] = &[
    "CreateProcess",
    "NtCreateUserProcess",
    "NtCreateProcess",
    "ZwCreateUserProcess",
    "ZwCreateProcess",
    "RtlCreateUserProcess",
    "WinExec",
    "LoadModule",
    "ShellExecute",
    "system",
    "_wsystem",
    "_popen",
    "_wpopen",
    "_execl",
    "_execv",
    "_wexecl",
    "_wexecv",
    "_spawnl",
    "_spawnv",
    "_wspawnl",
    "_wspawnv",
    "URLDownloadTo",
    "LoadLibrary",
    "LoadPackagedLibrary",
    "LdrLoadDll",
];

/// Im Testbau ohne Optimierung zusätzlich zugelassen, je DLL (klein geschrieben) namentlich:
/// Code der Standardbibliothek, den promptscan nicht aufruft (std::net, std::env::home_dir,
/// std::process). Gemessen an der Importtabelle des Testbaus vom 06.11.2026 (Rust 1.99.1).
const NUR_TESTBAU_IMPORTE: &[(&str, &[&str])] = &[
    (
        "ws2_32.dll",
        &[
            "GetHostNameW",
            "WSADuplicateSocketW",
            "WSAGetLastError",
            "WSARecv",
            "WSASend",
            "WSASocketW",
            "WSAStartup",
            "accept",
            "bind",
            "closesocket",
            "connect",
            "freeaddrinfo",
            "getaddrinfo",
            "getpeername",
            "getsockname",
            "getsockopt",
            "ioctlsocket",
            "listen",
            "recv",
            "recvfrom",
            "select",
            "send",
            "sendto",
            "setsockopt",
            "shutdown",
        ],
    ),
    ("userenv.dll", &["GetUserProfileDirectoryW"]),
    ("kernel32.dll", &["CreateProcessW"]),
];

fn verbotene_stellen(quelltext: &str) -> Vec<&'static str> {
    VERBOTEN_IM_CODE
        .iter()
        .copied()
        .filter(|m| quelltext.contains(m))
        .collect()
}

fn schreibstellen(quelltext: &str) -> Vec<&'static str> {
    SCHREIBEN_IM_PROGRAMM
        .iter()
        .copied()
        .filter(|m| quelltext.contains(m))
        .collect()
}

/// Die einzigen Stellen, an denen das Programm Dateien schreibt. Je Eintrag: Datei unter
/// `src/`, Funktion, Muster aus `SCHREIBEN_IM_PROGRAMM`, Anzahl. `datei_schreiben` ist die
/// einzige Schreibhilfe; `konfiguration_schreiben` legt dazu den Ordner der
/// Konfigurationsdatei an.
const ERLAUBTE_SCHREIBSTELLEN: &[(&str, &str, &str, usize)] = &[
    ("main.rs", "datei_schreiben", "fs::write", 1),
    ("main.rs", "konfiguration_schreiben", "create_dir", 1),
];

/// Wer die Schreibhilfe aufruft, je auf ausdrücklichen Befehl: `konfiguration_schreiben` für
/// `promptscan config --role …` bzw. `--lang …` (nur die Schlüssel role und lang),
/// `abschliessen` für `--output <datei>` (der Bericht in die genannte Datei).
const AUFRUFER_DER_SCHREIBHILFE: &[&str] = &["abschliessen", "konfiguration_schreiben"];

/// Der Programmteil einer Quelldatei: ohne die Testmodule (`#[cfg(test)]` vor `mod`), die in
/// diesem Projekt am Dateiende stehen. Tests legen Testdateien an; in der Programmdatei stehen
/// sie nicht.
fn ohne_testmodule(quelltext: &str) -> &str {
    for (i, m) in quelltext.match_indices("#[cfg(test)]") {
        if quelltext[i + m.len()..].trim_start().starts_with("mod ") {
            return &quelltext[..i];
        }
    }
    quelltext
}

/// Jede Fundstelle eines Musters mit der Funktion, in der sie steht, als (Funktion, Muster).
/// Die Funktion ist die letzte Zeile davor, die eine Funktion beginnt; eine Fundstelle direkt
/// hinter `fn ` ist die Definition selbst und zählt nicht.
fn stellen_je_funktion<'m>(quelltext: &str, muster: &[&'m str]) -> Vec<(String, &'m str)> {
    let mut aus = Vec::new();
    for &m in muster {
        for (i, _) in quelltext.match_indices(m) {
            if quelltext[..i].ends_with("fn ") {
                continue;
            }
            let funktion = quelltext[..i]
                .lines()
                .rev()
                .find_map(funktionsname)
                .unwrap_or_default();
            aus.push((funktion, m));
        }
    }
    aus.sort();
    aus
}

fn schreibstellen_je_funktion(quelltext: &str) -> Vec<(String, &'static str)> {
    stellen_je_funktion(quelltext, SCHREIBEN_IM_PROGRAMM)
}

/// Der Name der Funktion, wenn die Zeile eine beginnt (auch mit `pub`, `pub(crate)`, `const`,
/// `async`, `unsafe` davor).
fn funktionsname(zeile: &str) -> Option<String> {
    let mut rest = zeile.trim_start();
    for vorsatz in ["pub(crate) ", "pub ", "const ", "async ", "unsafe "] {
        rest = rest.strip_prefix(vorsatz).unwrap_or(rest);
    }
    let name: String = rest
        .strip_prefix("fn ")?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// `link(…)`-Attribute (auch in `cfg_attr` und mit Leerraum vor der Klammer) und
/// `link!(…)`-Makros von windows-link, die eine andere Bibliothek als kernel32 nennen oder
/// keinen Namen haben. `#[link_name]`, `#[link_section]`, `#[link_ordinal]` und
/// `fs::hard_link(…)` zählen nicht.
fn fremde_bibliotheken(quelltext: &str) -> Vec<String> {
    let mut aus = Vec::new();
    for (i, _) in quelltext.match_indices("link") {
        let davor = quelltext[..i].chars().next_back();
        if davor.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let nach = quelltext[i + "link".len()..].trim_start();
        let (makro, nach) = match nach.strip_prefix('!') {
            Some(r) => (true, r.trim_start()),
            None => (false, nach),
        };
        let mut zeichen = nach.chars();
        let schluss = match zeichen.next() {
            Some('(') => ')',
            Some('[') if makro => ']',
            Some('{') if makro => '}',
            _ => continue,
        };
        let rest = zeichen.as_str();
        let inhalt = &rest[..rest.find(schluss).unwrap_or(rest.len())];
        let name = if makro {
            // windows-link: link!("name.dll" "system" fn …)
            inhalt
                .trim_start()
                .strip_prefix('"')
                .and_then(|n| Some(n[..n.find('"')?].to_string()))
        } else {
            inhalt.split_once("name").and_then(|(_, n)| {
                let n = n
                    .trim_start()
                    .strip_prefix('=')?
                    .trim_start()
                    .strip_prefix('"')?;
                Some(n[..n.find('"')?].to_string())
            })
        };
        let erlaubt = name.as_deref().is_some_and(|n| {
            let n = n.to_ascii_lowercase();
            ERLAUBT_IN_LINK.contains(&n.trim_end_matches(".dll"))
        });
        if !erlaubt {
            aus.push(format!("link{}({inhalt})", if makro { "!" } else { "" }));
        }
    }
    aus
}

fn verbotene_pakete(lock: &str) -> Vec<String> {
    lock.lines()
        .filter_map(|z| z.strip_prefix("name = \""))
        .map(|z| z.trim_end_matches('"').to_string())
        .filter(|n| VERBOTEN_IN_LOCK.contains(&n.as_str()))
        .collect()
}

/// Features aus `VERBOTENE_FEATURES` in Cargo.toml: bei normalen und Bau-Abhängigkeiten (auch
/// in Ziel-Abschnitten) und im eigenen Abschnitt `[features]` («paket/Feature»).
fn verbotene_features(manifest: &toml::Table) -> Vec<String> {
    let verboten = |f: &str| VERBOTENE_FEATURES.iter().any(|p| f.starts_with(p));
    let mut aus = Vec::new();
    let mut tabellen = vec![manifest];
    if let Some(ziele) = manifest.get("target").and_then(|v| v.as_table()) {
        tabellen.extend(ziele.values().filter_map(|v| v.as_table()));
    }
    for t in tabellen {
        for abschnitt in ["dependencies", "build-dependencies"] {
            let Some(abh) = t.get(abschnitt).and_then(|v| v.as_table()) else {
                continue;
            };
            for (paket, wert) in abh {
                let features = wert.get("features").and_then(|v| v.as_array());
                for f in features.into_iter().flatten().filter_map(|f| f.as_str()) {
                    if verboten(f) {
                        aus.push(format!("{paket}: {f}"));
                    }
                }
            }
        }
    }
    if let Some(eigene) = manifest.get("features").and_then(|v| v.as_table()) {
        for (name, liste) in eigene {
            for f in liste
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f.as_str())
            {
                if f.split_once('/').is_some_and(|(_, rest)| verboten(rest)) {
                    aus.push(format!("[features] {name}: {f}"));
                }
            }
        }
    }
    aus
}

fn rs_dateien(ordner: &Path, aus: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(ordner).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_dateien(&p, aus);
        } else if p.extension().is_some_and(|x| x == "rs") {
            aus.push(p);
        }
    }
}

#[test]
fn kein_netzwerk_und_kein_programmstart_im_quelltext() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dateien = Vec::new();
    rs_dateien(&wurzel.join("src"), &mut dateien);
    let ohne_beispiele = dateien.len();
    rs_dateien(&wurzel.join("examples"), &mut dateien);
    assert!(ohne_beispiele > 5, "Quelltext nicht gefunden");
    assert!(dateien.len() > ohne_beispiele, "examples/ nicht gefunden");
    dateien.push(wurzel.join("build.rs"));
    for d in dateien {
        let inhalt = std::fs::read_to_string(&d).unwrap();
        assert!(
            verbotene_stellen(&inhalt).is_empty(),
            "{}: {:?}",
            d.display(),
            verbotene_stellen(&inhalt)
        );
        assert!(
            fremde_bibliotheken(&inhalt).is_empty(),
            "{}: {:?}",
            d.display(),
            fremde_bibliotheken(&inhalt)
        );
    }
}

/// Geschrieben wird nur an den Stellen in `ERLAUBTE_SCHREIBSTELLEN`, und die Schreibhilfe
/// rufen nur die Funktionen in `AUFRUFER_DER_SCHREIBHILFE` auf; eine weitere Stelle schlägt
/// an, eine entfallene auch (dann sind die Listen nachzuführen).
#[test]
fn das_programm_schreibt_nur_auf_befehl() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut dateien = Vec::new();
    rs_dateien(&src, &mut dateien);
    assert!(dateien.len() > 5, "Quelltext nicht gefunden");
    let mut gefunden: Vec<(String, String, &str)> = Vec::new();
    let mut aufrufer: Vec<String> = Vec::new();
    for d in dateien {
        let ganz = std::fs::read_to_string(&d).unwrap();
        let inhalt = ohne_testmodule(&ganz);
        aufrufer.extend(
            stellen_je_funktion(inhalt, &["datei_schreiben("])
                .into_iter()
                .map(|(f, _)| f),
        );
        let datei = d
            .strip_prefix(&src)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        for (funktion, muster) in schreibstellen_je_funktion(inhalt) {
            gefunden.push((datei.clone(), funktion, muster));
        }
    }
    aufrufer.sort();
    aufrufer.dedup();
    assert_eq!(
        aufrufer, AUFRUFER_DER_SCHREIBHILFE,
        "Aufrufer der Schreibhilfe datei_schreiben weichen von AUFRUFER_DER_SCHREIBHILFE ab"
    );
    let mut erwartet: Vec<(String, String, &str)> = ERLAUBTE_SCHREIBSTELLEN
        .iter()
        .flat_map(|&(datei, funktion, muster, anzahl)| {
            std::iter::repeat_n((datei.to_string(), funktion.to_string(), muster), anzahl)
        })
        .collect();
    gefunden.sort();
    erwartet.sort();
    assert_eq!(
        gefunden, erwartet,
        "Schreibstellen (Datei, Funktion, Muster) weichen von ERLAUBTE_SCHREIBSTELLEN ab; die Zusage «schreibt nur auf Befehl» hängt daran"
    );
}

#[test]
fn keine_netzwerkbibliothek_in_cargo_lock() {
    let lock =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).unwrap();
    assert!(
        lock.contains("name = \"aho-corasick\""),
        "Cargo.lock nicht gelesen"
    );
    assert!(
        verbotene_pakete(&lock).is_empty(),
        "{:?}",
        verbotene_pakete(&lock)
    );
}

#[test]
fn keine_netzwerkfunktionen_von_windows_sys_in_cargo_toml() {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let manifest: toml::Table = toml::from_str(&text).expect("Cargo.toml");
    assert!(
        manifest.contains_key("dependencies"),
        "Cargo.toml nicht gelesen"
    );
    assert!(
        verbotene_features(&manifest).is_empty(),
        "{:?}",
        verbotene_features(&manifest)
    );
}

#[test]
fn gegenprobe_die_pruefungen_schlagen_an() {
    assert_eq!(
        verbotene_stellen("use std::net::TcpStream;"),
        vec!["std::net", "TcpStream"]
    );
    assert_eq!(
        verbotene_stellen("std::process::Command::new(\"cmd\")"),
        vec!["process::Command", "Command::new"]
    );
    assert_eq!(
        verbotene_stellen("let f = GetProcAddress(h, c\"ShellExecuteW\".as_ptr());"),
        vec!["ShellExecute", "GetProcAddress"]
    );
    assert_eq!(
        verbotene_stellen("use windows_sys::Win32::Networking::WinSock::connect;"),
        vec!["Networking", "WinSock"]
    );
    assert_eq!(
        verbotene_stellen("unsafe extern \"C\" { fn system(c: *const i8) -> i32; }"),
        vec!["fn system("]
    );
    assert_eq!(
        verbotene_stellen("fn _wspawnvp(); fn _popen();"),
        vec!["_popen", "_wspawnv"]
    );
    // Die ABI-Angabe «"system"» ist erlaubt.
    assert!(verbotene_stellen("unsafe extern \"system\" { fn GetConsoleMode(); }").is_empty());
    assert_eq!(
        schreibstellen("std::fs::write(&p, t)?; let f = std::fs::File::create(p)?;"),
        vec!["fs::write", "File::create"]
    );
    assert_eq!(
        schreibstellen("std::fs::OpenOptions::new().append(true).open(p)"),
        vec!["OpenOptions"]
    );
    assert!(schreibstellen("let t = std::fs::read_to_string(p)?; std::fs::read(p)?;").is_empty());
    let quelle = r#"pub(crate) fn speichern(p: &Path) {
    std::fs::write(p, b"x").unwrap();
}
fn lesen(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}"#;
    assert_eq!(
        schreibstellen_je_funktion(quelle),
        vec![("speichern".to_string(), "fs::write")]
    );
    let mit_tests = r#"fn programm() {}
#[cfg(test)]
mod tests {
    fn probe() {
        std::fs::write("x", b"x").unwrap();
    }
}"#;
    assert!(schreibstellen_je_funktion(ohne_testmodule(mit_tests)).is_empty());
    assert_eq!(schreibstellen_je_funktion(mit_tests).len(), 1);
    assert_eq!(
        stellen_je_funktion(
            "fn hilfe(p: &Path) {}\nfn befehl() {\n    hilfe(p);\n}",
            &["hilfe("]
        ),
        vec![("befehl".to_string(), "hilfe(")]
    );
    assert_eq!(
        verbotene_pakete("[[package]]\nname = \"reqwest\"\n"),
        vec!["reqwest".to_string()]
    );
    assert_eq!(
        fremde_bibliotheken("#[link(name = \"ws2_32\")]\nunsafe extern \"system\" {}"),
        vec!["link(name = \"ws2_32\")".to_string()]
    );
    // Leerraum vor der Klammer ist in Rust erlaubt und darf die Prüfung nicht umgehen.
    assert_eq!(
        fremde_bibliotheken("#[link (name = \"ws2_32\")]\n#[link\n(name = \"wininet\")]"),
        vec![
            "link(name = \"ws2_32\")".to_string(),
            "link(name = \"wininet\")".to_string()
        ]
    );
    assert_eq!(
        fremde_bibliotheken("#[cfg_attr(x, link(name = \"wininet\", kind = \"raw-dylib\"))]"),
        vec!["link(name = \"wininet\", kind = \"raw-dylib\")".to_string()]
    );
    assert_eq!(
        fremde_bibliotheken(
            "windows_link::link!(\"ws2_32.dll\" \"system\" fn connect(s: usize) -> i32);"
        ),
        vec!["link!(\"ws2_32.dll\" \"system\" fn connect(s: usize)".to_string()]
    );
    assert!(fremde_bibliotheken("#[link(name = \"kernel32\")]").is_empty());
    assert!(
        fremde_bibliotheken("#[link(name = \"KERNEL32.dll\", kind = \"raw-dylib\")]").is_empty()
    );
    assert!(
        fremde_bibliotheken(
            "windows_link::link!(\"kernel32.dll\" \"system\" fn GetConsoleMode());"
        )
        .is_empty()
    );
    assert!(
        fremde_bibliotheken(
            "#[link_name = \"x\"] fn f(); std::fs::hard_link(a, b); // Link (zum Datensatz)"
        )
        .is_empty()
    );
    let manifest: toml::Table = toml::from_str(
        "[dependencies]\nwindows-sys = { version = \"0.61\", features = \
         [\"Win32_System_Console\", \"Win32_Networking_WinSock\"] }\n\
         [target.'cfg(windows)'.dependencies.windows]\nversion = \"0.62\"\n\
         features = [\"Win32_NetworkManagement_IpHelper\"]\n\
         [features]\nnetz = [\"windows-sys/Win32_Networking_WinHttp\"]\n",
    )
    .unwrap();
    assert_eq!(
        verbotene_features(&manifest),
        vec![
            "windows-sys: Win32_Networking_WinSock".to_string(),
            "windows: Win32_NetworkManagement_IpHelper".to_string(),
            "[features] netz: windows-sys/Win32_Networking_WinHttp".to_string(),
        ]
    );
    let erlaubt: toml::Table = toml::from_str(
        "[dependencies]\nwindows-sys = { version = \"0.61\", features = [\"Win32_System_Console\"] }\n",
    )
    .unwrap();
    assert!(verbotene_features(&erlaubt).is_empty());
}

// --- Importtabelle der Programmdatei ------------------------------------------------------

/// Ein Eintrag der Importtabelle: DLL und Funktion («#n» bei Import über die Ordinalzahl).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Import {
    dll: String,
    funktion: String,
    verzoegert: bool,
}

fn zahl<const N: usize>(b: &[u8], o: usize) -> Result<[u8; N], String> {
    b.get(o..o + N)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("Datei endet vor Versatz {o:#x}"))
}

fn u16_bei(b: &[u8], o: usize) -> Result<u16, String> {
    zahl::<2>(b, o).map(u16::from_le_bytes)
}

fn u32_bei(b: &[u8], o: usize) -> Result<u32, String> {
    zahl::<4>(b, o).map(u32::from_le_bytes)
}

fn u64_bei(b: &[u8], o: usize) -> Result<u64, String> {
    zahl::<8>(b, o).map(u64::from_le_bytes)
}

/// Liest die normalen und verzögerten Importe einer PE32+-Datei (x86-64).
fn importe(b: &[u8]) -> Result<Vec<Import>, String> {
    if b.get(..2) != Some(b"MZ") {
        return Err("keine MZ-Kennung".into());
    }
    let pe = u32_bei(b, 0x3c)? as usize;
    if b.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("keine PE-Kennung".into());
    }
    let coff = pe + 4;
    if u16_bei(b, coff)? != 0x8664 {
        return Err("nicht für x86-64".into());
    }
    let anzahl_abschnitte = u16_bei(b, coff + 2)? as usize;
    let groesse_opt = u16_bei(b, coff + 16)? as usize;
    let opt = coff + 20;
    if u16_bei(b, opt)? != 0x20b {
        return Err("kein PE32+".into());
    }
    let image_base = u64_bei(b, opt + 24)?;
    let verzeichnisse = u32_bei(b, opt + 108)? as usize;
    let mut abschnitte = Vec::new();
    for i in 0..anzahl_abschnitte {
        let s = opt + groesse_opt + 40 * i;
        abschnitte.push((
            u32_bei(b, s + 12)?,                         // VirtualAddress
            u32_bei(b, s + 8)?.max(u32_bei(b, s + 16)?), // Grösse
            u32_bei(b, s + 20)?,                         // PointerToRawData
        ));
    }
    let versatz = |rva: u32| -> Result<usize, String> {
        abschnitte
            .iter()
            .find(|(va, groesse, _)| rva >= *va && rva - va < *groesse)
            .map(|(va, _, roh)| (rva - va + roh) as usize)
            .ok_or_else(|| format!("RVA {rva:#x} in keinem Abschnitt"))
    };
    let text_bei = |rva: u32| -> Result<String, String> {
        let o = versatz(rva)?;
        let ende = b[o..]
            .iter()
            .position(|&c| c == 0)
            .ok_or("Name ohne Ende")?;
        String::from_utf8(b[o..o + ende].to_vec()).map_err(|_| "Name nicht ASCII".to_string())
    };
    let namen = |rva: u32| -> Result<Vec<String>, String> {
        let mut aus = Vec::new();
        let mut o = versatz(rva)?;
        loop {
            let eintrag = u64_bei(b, o)?;
            if eintrag == 0 {
                return Ok(aus);
            }
            if eintrag & (1 << 63) != 0 {
                aus.push(format!("#{}", eintrag & 0xffff));
            } else {
                // Hinweis (2 Bytes), dann der Name.
                aus.push(text_bei((eintrag & 0x7fff_ffff) as u32 + 2)?);
            }
            o += 8;
            if aus.len() > 100_000 {
                return Err("Namensliste ohne Ende".into());
            }
        }
    };
    let mut aus = Vec::new();
    let verzeichnis = |i: usize| -> Result<u32, String> {
        if i < verzeichnisse {
            u32_bei(b, opt + 112 + 8 * i)
        } else {
            Ok(0)
        }
    };
    // Verzeichnis 1: Importtabelle, je DLL 20 Bytes.
    let rva = verzeichnis(1)?;
    if rva != 0 {
        let mut o = versatz(rva)?;
        loop {
            let (ilt, name, iat) = (u32_bei(b, o)?, u32_bei(b, o + 12)?, u32_bei(b, o + 16)?);
            if ilt == 0 && name == 0 && iat == 0 {
                break;
            }
            let dll = text_bei(name)?;
            for funktion in namen(if ilt != 0 { ilt } else { iat })? {
                aus.push(Import {
                    dll: dll.clone(),
                    funktion,
                    verzoegert: false,
                });
            }
            o += 20;
        }
    }
    // Verzeichnis 13: verzögerte Importe, je DLL 32 Bytes. Ohne Bit 0 in «Attributes» sind
    // die Adressen virtuelle Adressen statt RVA.
    let rva = verzeichnis(13)?;
    if rva != 0 {
        let mut o = versatz(rva)?;
        loop {
            let attribute = u32_bei(b, o)?;
            let (name, int) = (u32_bei(b, o + 4)?, u32_bei(b, o + 16)?);
            if name == 0 {
                break;
            }
            let als_rva = |wert: u32| -> Result<u32, String> {
                if attribute & 1 != 0 {
                    Ok(wert)
                } else {
                    u32::try_from(u64::from(wert).wrapping_sub(image_base))
                        .map_err(|_| "verzögerter Import: Adresse ausserhalb".to_string())
                }
            };
            let dll = text_bei(als_rva(name)?)?;
            for funktion in namen(als_rva(int)?)? {
                aus.push(Import {
                    dll: dll.clone(),
                    funktion,
                    verzoegert: true,
                });
            }
            o += 32;
        }
    }
    Ok(aus)
}

/// Ist der Import im Testbau ohne Optimierung namentlich zugelassen?
fn nur_im_testbau(dll_klein: &str, funktion: &str) -> bool {
    NUR_TESTBAU_IMPORTE
        .iter()
        .any(|(d, funktionen)| *d == dll_klein && funktionen.contains(&funktion))
}

/// Verstösse gegen die Listen, je Import und Art einmal.
fn verstoesse(importe: &[Import], streng: bool) -> BTreeSet<String> {
    let mut aus = BTreeSet::new();
    for i in importe {
        let dll = i.dll.to_ascii_lowercase();
        let art = if i.verzoegert { " (verzögert)" } else { "" };
        if !streng && nur_im_testbau(&dll, &i.funktion) {
            continue;
        }
        if VERBOTENE_DLLS.contains(&dll.as_str()) {
            aus.insert(format!("{}!{}{art}: verbotene DLL", i.dll, i.funktion));
        } else if !ERLAUBTE_DLLS.iter().any(|(d, _)| *d == dll) {
            aus.insert(format!(
                "{}!{}{art}: DLL nicht freigegeben",
                i.dll, i.funktion
            ));
        }
        if VERBOTENE_FUNKTIONEN
            .iter()
            .any(|p| i.funktion.starts_with(p))
        {
            aus.insert(format!("{}!{}{art}: verbotene Funktion", i.dll, i.funktion));
        }
    }
    aus
}

/// Die geprüfte Programmdatei und ob die strenge Liste gilt.
fn programmdatei() -> (PathBuf, bool) {
    match std::env::var_os("PROMPTSCAN_EXE") {
        Some(p) => (PathBuf::from(p), true),
        None => (
            PathBuf::from(env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME")))),
            !cfg!(debug_assertions),
        ),
    }
}

#[cfg(windows)]
#[test]
fn importe_der_programmdatei_sind_freigegeben() {
    let (pfad, streng) = programmdatei();
    let bytes = std::fs::read(&pfad).unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));
    let importe = importe(&bytes).unwrap_or_else(|e| panic!("{}: {e}", pfad.display()));
    // Der Parser liest wirklich: Funktionen, die jedes Rust-Programm unter Windows braucht.
    for (dll, funktion) in [("kernel32.dll", "GetStdHandle"), ("msvcrt.dll", "malloc")] {
        assert!(
            importe
                .iter()
                .any(|i| i.dll.eq_ignore_ascii_case(dll) && i.funktion == funktion),
            "{dll}!{funktion} nicht gefunden; Parser oder Datei falsch"
        );
    }
    let v = verstoesse(&importe, streng);
    assert!(
        v.is_empty(),
        "{} ({}): {v:#?}",
        pfad.display(),
        if streng {
            "strenge Liste"
        } else {
            "Testbau ohne Optimierung"
        }
    );
}

#[test]
fn gegenprobe_importliste_schlaegt_an() {
    let imp = |dll: &str, funktion: &str| Import {
        dll: dll.into(),
        funktion: funktion.into(),
        verzoegert: false,
    };
    let erlaubt = [
        imp("KERNEL32.dll", "GetStdHandle"),
        imp("KERNEL32.dll", "GetSystemInfo"),
        imp("kernel32.dll", "GetConsoleProcessList"),
        imp("kernel32.dll", "ReadConsoleW"),
        imp("kernel32.dll", "GetProcAddress"),
        imp("ntdll.dll", "NtReadFile"),
        imp("ntdll.dll", "NtCreateFile"),
        imp("msvcrt.dll", "_exit"),
        imp("msvcrt.dll", "malloc"),
        imp("bcryptprimitives.dll", "ProcessPrng"),
    ];
    assert!(verstoesse(&erlaubt, true).is_empty());

    // Streng: auch die Einträge des Testbaus sind verboten.
    let testbau = [
        imp("WS2_32.dll", "connect"),
        imp("USERENV.dll", "GetUserProfileDirectoryW"),
        imp("KERNEL32.dll", "CreateProcessW"),
    ];
    assert_eq!(
        verstoesse(&testbau, true),
        BTreeSet::from([
            "WS2_32.dll!connect: verbotene DLL".to_string(),
            "USERENV.dll!GetUserProfileDirectoryW: DLL nicht freigegeben".to_string(),
            "KERNEL32.dll!CreateProcessW: verbotene Funktion".to_string(),
        ])
    );
    assert!(verstoesse(&testbau, false).is_empty());

    // Im Testbau nur die Funktionen der Standardbibliothek, nicht die ganze DLL.
    let neu_im_testbau = [
        imp("WS2_32.dll", "WSACleanup"),
        imp("USERENV.dll", "LoadUserProfileW"),
        imp("KERNEL32.dll", "CreateProcessA"),
    ];
    assert_eq!(
        verstoesse(&neu_im_testbau, false),
        BTreeSet::from([
            "WS2_32.dll!WSACleanup: verbotene DLL".to_string(),
            "USERENV.dll!LoadUserProfileW: DLL nicht freigegeben".to_string(),
            "KERNEL32.dll!CreateProcessA: verbotene Funktion".to_string(),
        ])
    );

    // In jedem Bau verboten.
    let immer = [
        imp("WININET.dll", "InternetOpenW"),
        imp("winhttp.dll", "WinHttpOpen"),
        imp("urlmon.dll", "URLDownloadToFileW"),
        imp("SHELL32.dll", "ShellExecuteW"),
        imp("kernel32.dll", "LoadLibraryExW"),
        imp("kernel32.dll", "CreateProcessAsUserW"),
        imp("kernel32.dll", "WinExec"),
        imp("kernel32.dll", "LoadModule"),
        imp("msvcrt.dll", "system"),
        imp("msvcrt.dll", "_wsystem"),
        imp("msvcrt.dll", "_popen"),
        imp("msvcrt.dll", "_wspawnvp"),
        imp("msvcrt.dll", "_execlpe"),
        imp("ntdll.dll", "ZwCreateUserProcess"),
        imp("ntdll.dll", "ZwCreateProcessEx"),
        imp("ntdll.dll", "NtCreateUserProcess"),
        imp("advapi32.dll", "RegOpenKeyExW"),
    ];
    for streng in [true, false] {
        let v = verstoesse(&immer, streng);
        for erwartet in [
            "WININET.dll!InternetOpenW: verbotene DLL",
            "winhttp.dll!WinHttpOpen: verbotene DLL",
            "urlmon.dll!URLDownloadToFileW: verbotene DLL",
            "urlmon.dll!URLDownloadToFileW: verbotene Funktion",
            "SHELL32.dll!ShellExecuteW: verbotene DLL",
            "SHELL32.dll!ShellExecuteW: verbotene Funktion",
            "kernel32.dll!LoadLibraryExW: verbotene Funktion",
            "kernel32.dll!CreateProcessAsUserW: verbotene Funktion",
            "kernel32.dll!WinExec: verbotene Funktion",
            "kernel32.dll!LoadModule: verbotene Funktion",
            "msvcrt.dll!system: verbotene Funktion",
            "msvcrt.dll!_wsystem: verbotene Funktion",
            "msvcrt.dll!_popen: verbotene Funktion",
            "msvcrt.dll!_wspawnvp: verbotene Funktion",
            "msvcrt.dll!_execlpe: verbotene Funktion",
            "ntdll.dll!ZwCreateUserProcess: verbotene Funktion",
            "ntdll.dll!ZwCreateProcessEx: verbotene Funktion",
            "ntdll.dll!NtCreateUserProcess: verbotene Funktion",
            "advapi32.dll!RegOpenKeyExW: DLL nicht freigegeben",
        ] {
            assert!(v.contains(erwartet), "{erwartet} fehlt in {v:?}");
        }
        assert_eq!(v.len(), 19, "{v:?}");
    }
    let verzoegert = [Import {
        dll: "ws2_32.dll".into(),
        funktion: "connect".into(),
        verzoegert: true,
    }];
    assert!(
        verstoesse(&verzoegert, true).contains("ws2_32.dll!connect (verzögert): verbotene DLL")
    );
}

/// Eine kleine PE32+-Datei mit einem Abschnitt: Import von ws2_32.dll (connect und
/// Ordinalzahl 7) und verzögerter Import von wininet.dll (InternetOpenW).
fn kuenstliche_pe_datei(rva_basiert: bool) -> Vec<u8> {
    // Unter 4 GB, damit das alte Format (32-Bit-Adressen statt RVA) darstellbar ist.
    const IMAGE_BASE: u64 = 0x40_0000;
    let mut b = vec![0u8; 0x800];
    let setze = |b: &mut Vec<u8>, o: usize, wert: &[u8]| b[o..o + wert.len()].copy_from_slice(wert);
    setze(&mut b, 0, b"MZ");
    setze(&mut b, 0x3c, &0x80u32.to_le_bytes());
    setze(&mut b, 0x80, b"PE\0\0");
    let coff = 0x84;
    setze(&mut b, coff, &0x8664u16.to_le_bytes());
    setze(&mut b, coff + 2, &1u16.to_le_bytes());
    setze(&mut b, coff + 16, &0xf0u16.to_le_bytes());
    let opt = coff + 20;
    setze(&mut b, opt, &0x20bu16.to_le_bytes());
    setze(&mut b, opt + 24, &IMAGE_BASE.to_le_bytes());
    setze(&mut b, opt + 108, &16u32.to_le_bytes());
    setze(&mut b, opt + 112 + 8, &0x1000u32.to_le_bytes()); // Importtabelle
    setze(&mut b, opt + 112 + 8 * 13, &0x1040u32.to_le_bytes()); // verzögerte Importe
    let abschnitt = opt + 0xf0;
    setze(&mut b, abschnitt, b".idata\0\0");
    setze(&mut b, abschnitt + 8, &0x400u32.to_le_bytes());
    setze(&mut b, abschnitt + 12, &0x1000u32.to_le_bytes());
    setze(&mut b, abschnitt + 16, &0x400u32.to_le_bytes());
    setze(&mut b, abschnitt + 20, &0x400u32.to_le_bytes());
    // RVA r liegt in der Datei bei r - 0x1000 + 0x400.
    let d = |rva: usize| rva - 0x1000 + 0x400;
    setze(&mut b, d(0x1000), &0x1100u32.to_le_bytes()); // Namensliste
    setze(&mut b, d(0x1000) + 12, &0x1200u32.to_le_bytes()); // DLL-Name
    setze(&mut b, d(0x1000) + 16, &0x1100u32.to_le_bytes()); // Adressliste
    let adresse = |rva: u32| {
        if rva_basiert {
            rva
        } else {
            (IMAGE_BASE + u64::from(rva)) as u32
        }
    };
    setze(&mut b, d(0x1040), &u32::from(rva_basiert).to_le_bytes());
    setze(&mut b, d(0x1040) + 4, &adresse(0x1210).to_le_bytes());
    setze(&mut b, d(0x1040) + 12, &adresse(0x1180).to_le_bytes());
    setze(&mut b, d(0x1040) + 16, &adresse(0x1180).to_le_bytes());
    setze(&mut b, d(0x1100), &0x1300u64.to_le_bytes());
    setze(&mut b, d(0x1108), &(1u64 << 63 | 7).to_le_bytes());
    setze(&mut b, d(0x1180), &0x1320u64.to_le_bytes());
    setze(&mut b, d(0x1200), b"ws2_32.dll\0");
    setze(&mut b, d(0x1210), b"wininet.dll\0");
    setze(&mut b, d(0x1302), b"connect\0");
    setze(&mut b, d(0x1322), b"InternetOpenW\0");
    b
}

#[test]
fn gegenprobe_parser_liest_normale_und_verzoegerte_importe() {
    for rva_basiert in [true, false] {
        let gelesen = importe(&kuenstliche_pe_datei(rva_basiert)).expect("künstliche PE-Datei");
        let erwartet = vec![
            Import {
                dll: "ws2_32.dll".into(),
                funktion: "connect".into(),
                verzoegert: false,
            },
            Import {
                dll: "ws2_32.dll".into(),
                funktion: "#7".into(),
                verzoegert: false,
            },
            Import {
                dll: "wininet.dll".into(),
                funktion: "InternetOpenW".into(),
                verzoegert: true,
            },
        ];
        assert_eq!(gelesen, erwartet, "rva_basiert = {rva_basiert}");
        assert_eq!(verstoesse(&gelesen, true).len(), 3);
        // Im Testbau ist connect zugelassen, die Ordinalzahl und wininet nicht.
        assert_eq!(
            verstoesse(&gelesen, false),
            BTreeSet::from([
                "ws2_32.dll!#7: verbotene DLL".to_string(),
                "wininet.dll!InternetOpenW (verzögert): verbotene DLL".to_string(),
            ])
        );
    }
    assert!(importe(b"MZ").is_err());
    assert!(importe(b"kein PE").is_err());
}

/// Die Zusagen reichen nicht weiter, als die Prüfungen dieser Datei belegen: promptscan sendet
/// nichts und schreibt nur auf Befehl (SPEC 6). Dass der Prompt «den Rechner nicht verlässt»,
/// belegen sie nicht: Was in der Befehlszeile steht, liegt im Verlauf der Shell. Darum sagt die
/// README das, und keine Unterlage verspricht mehr.
#[test]
fn unterlagen_sagen_nur_zu_was_geprueft_ist() {
    let lesen = |datei: &str| {
        let text =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(datei)).unwrap();
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    assert!(
        lesen("docs/SPEC.md")
            .contains("«promptscan sendet nichts und schreibt nur auf Befehl» ist damit prüfbar"),
        "SPEC 6: Zusage fehlt"
    );
    for (datei, hinweis) in [
        (
            "README.de.md",
            "Text in der Befehlszeile gelangt in den Verlauf der Shell",
        ),
        (
            "README.md",
            "text on the command line ends up in the shell history",
        ),
    ] {
        assert!(
            lesen(datei).contains(hinweis),
            "{datei}: Hinweis auf den Verlauf der Shell fehlt"
        );
    }
    for datei in [
        "README.md",
        "README.de.md",
        "docs/SPEC.md",
        "docs/WEITERGABE.md",
    ] {
        let text = lesen(datei);
        for zusage in [
            "verlässt den Rechner",
            "leaves your computer",
            "never leaves",
        ] {
            assert!(
                !text.contains(zusage),
                "{datei}: «{zusage}» belegt kein Test"
            );
        }
    }
}
