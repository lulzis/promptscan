//! Erzeugt THIRD-PARTY-NOTICES.md: die Vermerke zu allem, was in die Programmdatei gelangt und
//! nicht von promptscan selbst stammt (Crates, Rust-Standardbibliothek, MinGW-w64-Laufzeit,
//! eingebettete Daten). Die Prosa steht in `tools/drittvermerke/vorlage.md`.
//!
//! Aufruf im Projektordner (Git Bash oder PowerShell):
//!
//! ```text
//! cargo tree -e normal --target x86_64-pc-windows-gnu --prefix none --no-dedupe --offline | cargo run --example drittvermerke
//! ```
//!
//! Mit `-- --pruefen` wird nur verglichen: Exit-Code 1, wenn die Datei veraltet ist.
//!
//! Gelesen werden nur Dateien: Cargo.lock und Cargo.toml dieses Pakets, die Pakete im
//! Cargo-Register (`CARGO_HOME`), das Sysroot der Werkzeugkette (neben dem `cargo`, das dieses
//! Beispiel gebaut hat), `lexika/` und `tools/drittvermerke/` (Vorlage, Lizenzdateien der
//! Standardbibliothek, `zusatz.toml` für übernommenen Code). Aus `cargo tree` (Standardeingabe)
//! kommt nur, welche Crates für das Ziel gebaut werden. Kein Netz, kein Prozessstart. In die
//! erzeugte Datei gelangt kein Pfad dieses Rechners; das wird vor dem Schreiben geprüft.
//!
//! Was Cargo.lock ausserhalb der normalen und Bau-Abhängigkeiten führt, steht als «nur für
//! Tests» in Abschnitt 4.2, mit Lizenzangabe, soweit das Paket im Cargo-Register entpackt ist.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Zielsystem der ausgelieferten Programmdatei.
const ZIEL: &str = "x86_64-pc-windows-gnu";

/// Bibliotheken im Sysroot, die zum Rust-Baum gehören (gedeckt durch COPYRIGHT-library.html
/// bzw. `[[im_baum]]` in std.toml).
const STD_IM_BAUM: &[&str] = &[
    "alloc",
    "compiler_builtins",
    "core",
    "panic_unwind",
    "rustc_std_workspace_alloc",
    "rustc_std_workspace_core",
    "rustc_std_workspace_std",
    "std",
    "std_detect",
    "unwind",
    "windows_link",
];

/// Bibliotheken im Sysroot, die nicht in ein gewöhnliches Programm gelangen: Testrahmen,
/// Makro-Schnittstelle des Compilers, Abbruch statt Abwicklung bei Panik (promptscan wickelt ab).
const STD_NICHT_GEBUNDEN: &[&str] = &[
    "getopts",
    "panic_abort",
    "proc_macro",
    "rustc_literal_escaper",
    "sysroot",
    "test",
];

fn main() -> ExitCode {
    match ausfuehren() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("drittvermerke: {e}");
            ExitCode::from(2)
        }
    }
}

fn ausfuehren() -> Result<ExitCode, String> {
    let pruefen = std::env::args().skip(1).any(|a| a == "--pruefen");
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Err(format!(
            "Die Ausgabe von «cargo tree -e normal --target {ZIEL} --prefix none --no-dedupe \
             --offline» fehlt auf der Standardeingabe; Aufruf siehe Kopf von examples/drittvermerke.rs."
        ));
    }
    let mut baum_text = String::new();
    stdin
        .read_to_string(&mut baum_text)
        .map_err(|e| format!("Standardeingabe: {e}"))?;
    let baum = baum_lesen(&baum_text);
    if baum.is_empty() {
        return Err("Standardeingabe enthält keine Crates aus cargo tree".into());
    }

    let lock = lock_lesen(&lesen(&wurzel.join("Cargo.lock"))?)?;
    let manifest: toml::Table = toml::from_str(&lesen(&wurzel.join("Cargo.toml"))?)
        .map_err(|e| format!("Cargo.toml: {e}"))?;
    let register = Register::finden()?;
    let sysroot = sysroot()?;
    let tools = wurzel.join("tools").join("drittvermerke");

    let crates = crates_einteilen(&lock, &manifest, &baum, &register)?;
    let std_teil = StdTeil::lesen(&sysroot, &tools)?;
    let zusatz = zusatz_lesen(&tools)?;
    for z in &zusatz {
        let gebunden = crates
            .gebunden
            .iter()
            .chain(&std_teil.crates)
            .any(|c| c.name == z.name && c.version == z.version);
        if !gebunden {
            return Err(format!(
                "tools/drittvermerke/zusatz.toml: {} {} wird nicht mehr gebunden; Eintrag prüfen und nachführen",
                z.name, z.version
            ));
        }
    }

    let mut texte = Texte::default();
    for c in crates.gebunden.iter().chain(&std_teil.crates) {
        texte.crate_aufnehmen(c)?;
        for z in zusatz
            .iter()
            .filter(|z| z.name == c.name && z.version == c.version)
        {
            texte.zusatz_aufnehmen(z, &tools)?;
        }
    }

    let vorlage = lesen(
        &wurzel
            .join("tools")
            .join("drittvermerke")
            .join("vorlage.md"),
    )?;
    let mut aus = vorlage;
    let ersetzen = |aus: &mut String, platzhalter: &str, wert: &str| -> Result<(), String> {
        let marke = format!("{{{{{platzhalter}}}}}");
        if !aus.contains(&marke) {
            return Err(format!("Vorlage: Platzhalter {marke} fehlt"));
        }
        *aus = aus.replace(&marke, wert.trim_end_matches('\n'));
        Ok(())
    };
    ersetzen(
        &mut aus,
        "STAND",
        &format!(
            "Stand: promptscan {}, Rust {}, Ziel {ZIEL}.",
            env!("CARGO_PKG_VERSION"),
            std_teil.rustc
        ),
    )?;
    ersetzen(
        &mut aus,
        "ANZAHL_GEBUNDEN",
        &crates.gebunden.len().to_string(),
    )?;
    ersetzen(&mut aus, "ANZAHL_BAU", &crates.bau.len().to_string())?;
    ersetzen(
        &mut aus,
        "ANZAHL_NICHT",
        &crates.nicht_gebaut.len().to_string(),
    )?;
    ersetzen(
        &mut aus,
        "TABELLE_GEBUNDEN",
        &tabelle_gebunden(&crates.gebunden),
    )?;
    ersetzen(&mut aus, "TABELLE_BAU", &tabelle_bau(&crates.bau))?;
    ersetzen(
        &mut aus,
        "TABELLE_NICHT_GEBAUT",
        &tabelle_nicht_gebaut(&crates.nicht_gebaut),
    )?;
    ersetzen(&mut aus, "ZUSATZ", &zusatz_hinweise(&zusatz))?;
    ersetzen(
        &mut aus,
        "ANZAHL_TESTS",
        &crates.nur_tests.len().to_string(),
    )?;
    ersetzen(
        &mut aus,
        "TABELLE_NUR_TESTS",
        &tabelle_nur_tests(&crates.nur_tests),
    )?;
    ersetzen(&mut aus, "STD_RUST", &std_teil.rustc)?;
    ersetzen(&mut aus, "STD_IM_BAUM", &std_teil.tabelle_im_baum())?;
    ersetzen(&mut aus, "STD_CRATES", &tabelle_gebunden(&std_teil.crates))?;
    ersetzen(&mut aus, "STD_EIGENE", &std_teil.tabelle_eigene())?;
    ersetzen(
        &mut aus,
        "DATEN_REGISTER",
        &register_daten(&wurzel.join("lexika").join("register"))?,
    )?;
    ersetzen(
        &mut aus,
        "DATEN_QUELLEN",
        &quellen_daten(&wurzel.join("lexika"))?,
    )?;
    let unicode = unicode_texte(
        &wurzel.join("lexika"),
        &sysroot,
        &wurzel.join("tools").join("drittvermerke"),
    )?;
    ersetzen(
        &mut aus,
        "TEXTE",
        &texte.ausgeben(&std_teil, &unicode, &sysroot, wurzel)?,
    )?;
    if aus.contains("{{") {
        return Err("Vorlage: unbekannter Platzhalter übrig".into());
    }

    keine_lokalen_pfade(&aus, &[wurzel, &register.home, &sysroot])?;

    let ziel = wurzel.join("THIRD-PARTY-NOTICES.md");
    if pruefen {
        let bisher = fs::read_to_string(&ziel).unwrap_or_default();
        if bisher.replace("\r\n", "\n") == aus {
            println!("THIRD-PARTY-NOTICES.md ist aktuell.");
            Ok(ExitCode::SUCCESS)
        } else {
            println!("THIRD-PARTY-NOTICES.md ist veraltet; ohne --pruefen neu erzeugen.");
            Ok(ExitCode::from(1))
        }
    } else {
        fs::write(&ziel, &aus).map_err(|e| format!("THIRD-PARTY-NOTICES.md schreiben: {e}"))?;
        println!(
            "THIRD-PARTY-NOTICES.md geschrieben: {} gebundene Crates, {} nur für den Bau, {} nicht gebaut, {} nur für Tests, {} Crates der Standardbibliothek, {} zusätzliche Vermerke.",
            crates.gebunden.len(),
            crates.bau.len(),
            crates.nicht_gebaut.len(),
            crates.nur_tests.len(),
            std_teil.crates.len(),
            zusatz.len()
        );
        Ok(ExitCode::SUCCESS)
    }
}

fn lesen(pfad: &Path) -> Result<String, String> {
    fs::read_to_string(pfad).map_err(|e| {
        format!(
            "{} nicht lesbar: {e}",
            pfad.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        )
    })
}

/// Bricht ab, wenn ein Pfad dieses Rechners in die Ausgabe geraten ist.
fn keine_lokalen_pfade(aus: &str, pfade: &[&Path]) -> Result<(), String> {
    for p in pfade {
        let s = p.to_string_lossy();
        let vorwaerts = s.replace('\\', "/");
        if aus.contains(s.as_ref()) || aus.contains(&vorwaerts) {
            return Err("ein lokaler Pfad stünde in der Ausgabe".into());
        }
    }
    for muster in ["C:\\Users\\", "C:/Users/", "/home/"] {
        if aus.contains(muster) {
            return Err(format!("«{muster}» stünde in der Ausgabe"));
        }
    }
    Ok(())
}

// --- cargo tree und Cargo.lock ------------------------------------------------------------

/// Crates aus der Ausgabe von `cargo tree`: (Name, Fassung). Baumzeichen, «(*)» und
/// «(proc-macro)» werden übergangen, die Wurzelzeile mit ihrem Pfad ebenso wie jede andere
/// Zeile behandelt (der Pfad wird nicht gelesen).
fn baum_lesen(text: &str) -> BTreeSet<(String, String)> {
    let mut aus = BTreeSet::new();
    for zeile in text.lines() {
        let zeile = zeile.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
        let mut teile = zeile.split_whitespace();
        let (Some(name), Some(version)) = (teile.next(), teile.next()) else {
            continue;
        };
        let Some(version) = version.strip_prefix('v') else {
            continue;
        };
        if name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            aus.insert((name.to_string(), version.to_string()));
        }
    }
    aus
}

/// Ein Paket aus Cargo.lock.
struct LockPaket {
    name: String,
    version: String,
    quelle: Option<String>,
    abh: Vec<String>,
}

fn lock_lesen(text: &str) -> Result<Vec<LockPaket>, String> {
    let t: toml::Table = toml::from_str(text).map_err(|e| format!("Cargo.lock: {e}"))?;
    let pakete = t
        .get("package")
        .and_then(|v| v.as_array())
        .ok_or("Cargo.lock: keine Pakete")?;
    pakete
        .iter()
        .map(|p| {
            let s = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_string);
            Ok(LockPaket {
                name: s("name").ok_or("Cargo.lock: Paket ohne Namen")?,
                version: s("version").ok_or("Cargo.lock: Paket ohne Fassung")?,
                quelle: s("source"),
                abh: p
                    .get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Löst einen Abhängigkeitseintrag von Cargo.lock auf («name», «name fassung» oder
/// «name fassung (quelle)»).
fn aufloesen(pakete: &[LockPaket], eintrag: &str) -> Result<usize, String> {
    let mut teile = eintrag.split_whitespace();
    let name = teile.next().unwrap_or_default();
    let version = teile.next();
    let treffer: Vec<usize> = pakete
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name == name && version.is_none_or(|v| p.version == v))
        .map(|(i, _)| i)
        .collect();
    match treffer.as_slice() {
        [i] => Ok(*i),
        _ => Err(format!(
            "Cargo.lock: Abhängigkeit «{eintrag}» nicht eindeutig"
        )),
    }
}

/// Namen der Abhängigkeiten eines Abschnitts (z. B. «dependencies») im Manifest, auch in
/// Ziel-Abschnitten; ein umbenanntes Paket zählt mit seinem Paketnamen.
fn abh_namen(manifest: &toml::Table, abschnitt: &str) -> BTreeSet<String> {
    let mut namen = BTreeSet::new();
    let mut aus_tabelle = |t: &toml::Table| {
        if let Some(d) = t.get(abschnitt).and_then(|v| v.as_table()) {
            for (schluessel, wert) in d {
                let name = wert
                    .get("package")
                    .and_then(|v| v.as_str())
                    .unwrap_or(schluessel);
                namen.insert(name.to_string());
            }
        }
    };
    aus_tabelle(manifest);
    if let Some(ziele) = manifest.get("target").and_then(|v| v.as_table()) {
        for z in ziele.values().filter_map(|v| v.as_table()) {
            aus_tabelle(z);
        }
    }
    namen
}

// --- Pakete im Cargo-Register ---------------------------------------------------------------

struct Register {
    home: PathBuf,
    indizes: Vec<PathBuf>,
}

impl Register {
    fn finden() -> Result<Register, String> {
        let home = std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".cargo")))
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
            .ok_or("CARGO_HOME nicht bestimmbar")?;
        let src = home.join("registry").join("src");
        let mut indizes: Vec<PathBuf> = fs::read_dir(&src)
            .map_err(|_| "Cargo-Register (registry/src) nicht gefunden; zuerst «cargo fetch»")?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        indizes.sort();
        Ok(Register { home, indizes })
    }

    fn ordner(&self, name: &str, version: &str) -> Result<PathBuf, String> {
        self.indizes
            .iter()
            .map(|i| i.join(format!("{name}-{version}")))
            .find(|p| p.is_dir())
            .ok_or_else(|| {
                format!("{name} {version} fehlt im Cargo-Register; zuerst «cargo fetch»")
            })
    }
}

/// Ein Crate mit Lizenzangabe und dem Ordner, in dem seine Lizenzdateien liegen.
struct Crate {
    name: String,
    version: String,
    lizenz: String,
    ordner: PathBuf,
    proc_macro: bool,
    manifest: Option<toml::Table>,
}

impl Crate {
    fn aus_register(register: &Register, name: &str, version: &str) -> Result<Crate, String> {
        let ordner = register.ordner(name, version)?;
        let manifest: toml::Table = toml::from_str(&lesen(&ordner.join("Cargo.toml"))?)
            .map_err(|e| format!("{name} {version}: Cargo.toml: {e}"))?;
        let paket = manifest.get("package").and_then(|v| v.as_table());
        let lizenz = paket
            .and_then(|p| p.get("license"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                format!(
                    "{name} {version}: kein SPDX-Ausdruck in «license»; Vermerk von Hand klären"
                )
            })?;
        let lib = manifest.get("lib").and_then(|v| v.as_table());
        let proc_macro = lib
            .and_then(|l| l.get("proc-macro").or_else(|| l.get("proc_macro")))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(Crate {
            name: name.to_string(),
            version: version.to_string(),
            lizenz,
            ordner,
            proc_macro,
            manifest: Some(manifest),
        })
    }

    /// Ist `abh` in diesem Paket eine optionale Abhängigkeit?
    fn optional(&self, abh: &str) -> bool {
        let Some(m) = &self.manifest else {
            return false;
        };
        let in_tabelle = |t: &toml::Table| {
            t.get("dependencies")
                .and_then(|v| v.as_table())
                .and_then(|d| d.get(abh))
                .and_then(|d| d.get("optional"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        };
        in_tabelle(m)
            || m.get("target")
                .and_then(|v| v.as_table())
                .is_some_and(|z| z.values().filter_map(|v| v.as_table()).any(in_tabelle))
    }

    /// Unter welchen Ziel-Bedingungen (`[target.'cfg(…)'.dependencies]`) verlangt dieses Paket
    /// `abh`? Leer, wenn es `abh` ohne Bedingung verlangt oder das Manifest fehlt.
    fn bedingungen(&self, abh: &str) -> Vec<String> {
        let Some(m) = &self.manifest else {
            return Vec::new();
        };
        let nennt = |t: &toml::Table| {
            ["dependencies", "build-dependencies"].iter().any(|a| {
                t.get(*a).and_then(|v| v.as_table()).is_some_and(|d| {
                    d.iter().any(|(schluessel, wert)| {
                        wert.get("package")
                            .and_then(|p| p.as_str())
                            .unwrap_or(schluessel)
                            == abh
                    })
                })
            })
        };
        if nennt(m) {
            return Vec::new();
        }
        m.get("target")
            .and_then(|v| v.as_table())
            .map(|ziele| {
                ziele
                    .iter()
                    .filter(|(_, t)| t.as_table().is_some_and(nennt))
                    .map(|(bedingung, _)| bedingung.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Ein nicht gebautes Crate mit Grund.
struct NichtGebaut {
    c: Crate,
    grund: String,
}

/// Ein Paket aus Cargo.lock, das nur über dev-dependencies erreichbar ist.
struct NurTest {
    name: String,
    version: String,
    /// Lizenz laut Paket; None, wenn das Paket im Cargo-Register nicht entpackt ist.
    lizenz: Option<String>,
    /// Wer es verlangt, mit der Bedingung aus dessen Manifest.
    ueber: Vec<String>,
}

struct Einteilung {
    gebunden: Vec<Crate>,
    bau: Vec<Crate>,
    nicht_gebaut: Vec<NichtGebaut>,
    nur_tests: Vec<NurTest>,
}

/// Liest das Paket `i` aus Cargo.lock einmal aus dem Register.
fn holen<'a>(
    crates: &'a mut BTreeMap<usize, Crate>,
    lock: &[LockPaket],
    register: &Register,
    i: usize,
) -> Result<&'a Crate, String> {
    use std::collections::btree_map::Entry;
    match crates.entry(i) {
        Entry::Occupied(e) => Ok(e.into_mut()),
        Entry::Vacant(e) => {
            let p = &lock[i];
            if p.quelle
                .as_deref()
                .is_none_or(|q| !q.starts_with("registry+"))
            {
                return Err(format!(
                    "{} {}: nicht aus einem Register; Vermerk von Hand klären",
                    p.name, p.version
                ));
            }
            Ok(e.insert(Crate::aus_register(register, &p.name, &p.version)?))
        }
    }
}

/// Teilt die normalen Abhängigkeiten aus Cargo.lock ein: in der Programmdatei, nur für den Bau
/// (prozedurale Makros und ihre Abhängigkeiten), in Cargo.lock aber nicht gebaut.
fn crates_einteilen(
    lock: &[LockPaket],
    manifest: &toml::Table,
    baum: &BTreeSet<(String, String)>,
    register: &Register,
) -> Result<Einteilung, String> {
    let eigen = env!("CARGO_PKG_NAME");
    let wurzel = lock
        .iter()
        .position(|p| p.name == eigen)
        .ok_or("Cargo.lock: eigenes Paket fehlt")?;
    let normal = abh_namen(manifest, "dependencies");
    let bau_namen = abh_namen(manifest, "build-dependencies");

    let mut crates: BTreeMap<usize, Crate> = BTreeMap::new();

    // Hülle über Cargo.lock: alles, was von den normalen und Bau-Abhängigkeiten aus erreichbar ist.
    let mut start_laufzeit = Vec::new();
    let mut start_bau = Vec::new();
    for e in &lock[wurzel].abh {
        let i = aufloesen(lock, e)?;
        if normal.contains(&lock[i].name) {
            start_laufzeit.push(i);
        } else if bau_namen.contains(&lock[i].name) {
            start_bau.push(i);
        }
    }
    let mut huelle = BTreeSet::new();
    let mut laufzeit = BTreeSet::new();
    let mut stapel: Vec<(usize, bool)> = start_laufzeit
        .iter()
        .map(|&i| (i, true))
        .chain(start_bau.iter().map(|&i| (i, false)))
        .collect();
    while let Some((i, zur_laufzeit)) = stapel.pop() {
        let proc_macro = holen(&mut crates, lock, register, i)?.proc_macro;
        let zur_laufzeit = zur_laufzeit && !proc_macro;
        let neu_huelle = huelle.insert(i);
        let neu_laufzeit = zur_laufzeit && laufzeit.insert(i);
        if neu_huelle || neu_laufzeit {
            for e in &lock[i].abh {
                stapel.push((aufloesen(lock, e)?, zur_laufzeit));
            }
        }
    }

    let im_baum = |i: usize| baum.contains(&(lock[i].name.clone(), lock[i].version.clone()));
    for (name, version) in baum {
        let bekannt = huelle
            .iter()
            .any(|&i| &lock[i].name == name && &lock[i].version == version);
        if !bekannt && name != eigen {
            return Err(format!(
                "cargo tree nennt {name} {version}, das nicht zu den normalen Abhängigkeiten in Cargo.lock gehört; \
                 cargo tree mit «-e normal» aufrufen"
            ));
        }
    }

    let mut gebunden = Vec::new();
    let mut bau = Vec::new();
    let mut nicht = Vec::new();
    let mut rest: BTreeMap<usize, Crate> = BTreeMap::new();
    for i in &huelle {
        let c = crates
            .remove(i)
            .ok_or_else(|| format!("{}: nicht gelesen", lock[*i].name))?;
        if !im_baum(*i) {
            rest.insert(*i, c);
        } else if laufzeit.contains(i) {
            gebunden.push(c);
        } else {
            bau.push(c);
        }
    }
    for (i, c) in rest {
        let eltern: Vec<&str> = huelle
            .iter()
            .filter(|&&j| {
                lock[j]
                    .abh
                    .iter()
                    .any(|e| aufloesen(lock, e).is_ok_and(|k| k == i))
            })
            .map(|&j| lock[j].name.as_str())
            .collect();
        let optional_in: Vec<&str> = eltern
            .iter()
            .copied()
            .filter(|e| {
                gebunden
                    .iter()
                    .chain(bau.iter())
                    .any(|g| g.name == *e && g.optional(&c.name))
            })
            .collect();
        let grund = if !optional_in.is_empty() {
            format!(
                "optionale Abhängigkeit von {}, nicht eingeschaltet",
                optional_in.join(", ")
            )
        } else {
            format!("nur über {}, das nicht gebaut wird", eltern.join(", "))
        };
        nicht.push(NichtGebaut { c, grund });
    }
    // Der Rest von Cargo.lock ist nur über dev-dependencies erreichbar.
    let dev_namen = abh_namen(manifest, "dev-dependencies");
    let mut nur_tests = Vec::new();
    for (i, paket) in lock.iter().enumerate() {
        if i == wurzel || huelle.contains(&i) {
            continue;
        }
        let lizenz = if register.ordner(&paket.name, &paket.version).is_ok() {
            Some(holen(&mut crates, lock, register, i)?.lizenz.clone())
        } else {
            None
        };
        let mut ueber = Vec::new();
        for (j, eltern) in lock.iter().enumerate() {
            let verlangt = eltern
                .abh
                .iter()
                .any(|e| aufloesen(lock, e).is_ok_and(|k| k == i));
            if !verlangt {
                continue;
            }
            if j == wurzel {
                ueber.push(if dev_namen.contains(&paket.name) {
                    "Cargo.toml [dev-dependencies]".to_string()
                } else {
                    format!("{eigen}, nicht als dev-dependency")
                });
                continue;
            }
            let bedingungen = if register.ordner(&eltern.name, &eltern.version).is_ok() {
                holen(&mut crates, lock, register, j)?.bedingungen(&paket.name)
            } else {
                Vec::new()
            };
            ueber.push(match bedingungen.as_slice() {
                [] => eltern.name.clone(),
                [eine] => format!("{} (nur `{eine}`)", eltern.name),
                mehrere => format!(
                    "{} (nur unter {} Ziel-Bedingungen)",
                    eltern.name,
                    mehrere.len()
                ),
            });
        }
        nur_tests.push(NurTest {
            name: paket.name.clone(),
            version: paket.version.clone(),
            lizenz,
            ueber,
        });
    }

    let sortieren =
        |v: &mut Vec<Crate>| v.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    sortieren(&mut gebunden);
    sortieren(&mut bau);
    nicht.sort_by(|a, b| (&a.c.name, &a.c.version).cmp(&(&b.c.name, &b.c.version)));
    nur_tests.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    Ok(Einteilung {
        gebunden,
        bau,
        nicht_gebaut: nicht,
        nur_tests,
    })
}

// --- Lizenzwahl ------------------------------------------------------------------------------

/// Rang einer Lizenz nach der Vermerkpflicht bei Weitergabe als Programmdatei (klein = gering).
fn rang(id: &str) -> u8 {
    match id {
        "0BSD" => 0,
        "Unlicense" | "CC0-1.0" => 1,
        "Zlib" | "BSL-1.0" => 2,
        "MIT" | "ISC" => 3,
        "BSD-2-Clause" | "BSD-3-Clause" => 4,
        "Apache-2.0" => 5,
        _ => 9,
    }
}

/// Teilt einen SPDX-Ausdruck an `trenner`, nur auf oberster Klammerebene.
fn teilen<'a>(s: &'a str, trenner: &str) -> Vec<&'a str> {
    let mut teile = Vec::new();
    let mut tiefe = 0i32;
    let mut anfang = 0;
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        match bytes[i] {
            b'(' => tiefe += 1,
            b')' => tiefe -= 1,
            _ => {}
        }
        if tiefe == 0 && s.is_char_boundary(i) && s[i..].starts_with(trenner) {
            teile.push(s[anfang..i].trim());
            i += trenner.len();
            anfang = i;
            continue;
        }
        i += 1;
    }
    teile.push(s[anfang..].trim());
    teile
}

/// Entfernt Klammern, die den ganzen Ausdruck umschliessen.
fn ohne_klammern(s: &str) -> &str {
    let s = s.trim();
    if !(s.starts_with('(') && s.ends_with(')')) {
        return s;
    }
    let mut tiefe = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => tiefe += 1,
            ')' => tiefe -= 1,
            _ => {}
        }
        if tiefe == 0 && i + 1 < s.len() {
            return s;
        }
    }
    ohne_klammern(&s[1..s.len() - 1])
}

/// Wählt aus einem SPDX-Ausdruck je UND-Glied die Lizenz mit der geringsten Vermerkpflicht.
/// «MIT/Apache-2.0» (alte Schreibweise) gilt als «MIT OR Apache-2.0».
fn waehlen(ausdruck: &str) -> Vec<String> {
    let ausdruck = ausdruck.replace('/', " OR ");
    teilen(&ausdruck, " AND ")
        .into_iter()
        .map(|glied| {
            teilen(ohne_klammern(glied), " OR ")
                .into_iter()
                .map(ohne_klammern)
                .min_by_key(|a| rang(a))
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

/// Was die gewählte Lizenz bei Weitergabe der Programmdatei verlangt.
fn vermerk(id: &str) -> &'static str {
    match id {
        "0BSD" | "Unlicense" | "CC0-1.0" => "keiner",
        "Zlib" | "BSL-1.0" => "keiner (Lizenzvermerk nur bei Weitergabe von Quelltext)",
        "MIT" | "ISC" => "Copyright- und Lizenzvermerk",
        "Unicode-3.0" => {
            "Copyright- und Lizenzvermerk bei jeder Kopie oder in der Begleitdokumentation"
        }
        "Apache-2.0" => "Lizenztext, NOTICE-Datei, Hinweis auf Änderungen",
        _ => "siehe Lizenztext",
    }
}

fn tabelle_gebunden(crates: &[Crate]) -> String {
    let mut t = String::from(
        "| Crate | Fassung | Lizenz laut Paket | gewählt | verlangter Vermerk |\n|---|---|---|---|---|\n",
    );
    for c in crates {
        let gewaehlt = waehlen(&c.lizenz);
        let vermerke = match gewaehlt.as_slice() {
            [eine] => vermerk(eine).to_string(),
            mehrere => mehrere
                .iter()
                .map(|g| format!("{g}: {}", vermerk(g)))
                .collect::<Vec<_>>()
                .join("; "),
        };
        t.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            c.name,
            c.version,
            c.lizenz,
            gewaehlt.join(" AND "),
            vermerke
        ));
    }
    t
}

fn tabelle_bau(crates: &[Crate]) -> String {
    let mut t =
        String::from("| Crate | Fassung | Lizenz laut Paket | Rolle |\n|---|---|---|---|\n");
    for c in crates {
        t.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            c.name,
            c.version,
            c.lizenz,
            if c.proc_macro {
                "prozedurales Makro"
            } else {
                "Abhängigkeit eines Makros"
            }
        ));
    }
    t
}

fn tabelle_nicht_gebaut(crates: &[NichtGebaut]) -> String {
    let mut t =
        String::from("| Crate | Fassung | Lizenz laut Paket | Grund |\n|---|---|---|---|\n");
    for n in crates {
        t.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            n.c.name, n.c.version, n.c.lizenz, n.grund
        ));
    }
    t
}

fn tabelle_nur_tests(pakete: &[NurTest]) -> String {
    let mut t = String::from("| Crate | Fassung | Lizenz laut Paket | über |\n|---|---|---|---|\n");
    for p in pakete {
        t.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            p.name,
            p.version,
            p.lizenz.as_deref().unwrap_or("nicht entpackt"),
            p.ueber.join("; ")
        ));
    }
    t
}

// --- Zusätzliche Vermerke für übernommenen Code ---------------------------------------------

/// Ein Eintrag aus `tools/drittvermerke/zusatz.toml`.
struct Zusatz {
    name: String,
    version: String,
    lizenz: String,
    datei: String,
    herkunft: String,
}

fn zusatz_lesen(tools: &Path) -> Result<Vec<Zusatz>, String> {
    let t: toml::Table = toml::from_str(&lesen(&tools.join("zusatz.toml"))?)
        .map_err(|e| format!("zusatz.toml: {e}"))?;
    let mut aus = Vec::new();
    for z in t
        .get("zusatz")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let s = |k: &str| -> Result<String, String> {
            z.get(k)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("zusatz.toml: «{k}» fehlt"))
        };
        aus.push(Zusatz {
            name: s("crate")?,
            version: s("version")?,
            lizenz: s("lizenz")?,
            datei: s("datei")?,
            herkunft: s("herkunft")?,
        });
    }
    Ok(aus)
}

/// Absatz für Abschnitt 2: welche Crates einen zusätzlichen Vermerk tragen.
fn zusatz_hinweise(zusatz: &[Zusatz]) -> String {
    if zusatz.is_empty() {
        return "Zusätzliche Vermerke für Code, den ein Crate aus einem anderen Projekt übernommen \
                hat: keine."
            .to_string();
    }
    let mut t = String::from(
        "Zusätzliche Vermerke für Code, den ein Crate aus einem anderen Projekt übernommen hat \
         (Text in Abschnitt 8.1):\n\n",
    );
    for z in zusatz {
        t.push_str(&format!("- `{}` {}: {}.\n", z.name, z.version, z.herkunft));
    }
    t
}

// --- Lizenztexte -----------------------------------------------------------------------------

/// Lizenzdateien eines Paketordners: oberste Ebene (LICENSE*, COPYING*, UNLICENSE*, NOTICE*)
/// und verschachtelte LICENSE*/NOTICE* (z. B. regex-syntax/src/unicode_tables/LICENSE-UNICODE).
fn lizenzdateien(ordner: &Path) -> Vec<(String, PathBuf)> {
    fn rekursiv(basis: &Path, ordner: &Path, tiefe: usize, aus: &mut Vec<(String, PathBuf)>) {
        let Ok(eintraege) = fs::read_dir(ordner) else {
            return;
        };
        let mut eintraege: Vec<PathBuf> = eintraege.flatten().map(|e| e.path()).collect();
        eintraege.sort();
        for p in eintraege {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    rekursiv(basis, &p, tiefe + 1, aus);
                }
                continue;
            }
            let oben = tiefe == 0
                && ["licen", "copying", "unlicense", "notice"]
                    .iter()
                    .any(|a| name.starts_with(a));
            let quelltext = [".rs", ".toml", ".json", ".py"]
                .iter()
                .any(|e| name.ends_with(e));
            let tief = tiefe > 0
                && !quelltext
                && (name.starts_with("licen") || name.starts_with("notice"));
            if oben || tief {
                let relativ = p
                    .strip_prefix(basis)
                    .map(|r| {
                        r.components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                aus.push((relativ, p));
            }
        }
    }
    let mut aus = Vec::new();
    rekursiv(ordner, ordner, 0, &mut aus);
    aus
}

/// Passt eine oberste Lizenzdatei zur gewählten Lizenz?
fn passt(id: &str, datei: &str, nur_eine: bool) -> bool {
    let d = datei.to_lowercase();
    match id {
        "MIT" => {
            d.starts_with("license-mit")
                || d.starts_with("licence-mit")
                || (nur_eine && matches!(d.as_str(), "license" | "license.md" | "license.txt"))
        }
        "Unlicense" => d.starts_with("unlicense"),
        "Zlib" => d.starts_with("license-zlib"),
        "0BSD" => d.starts_with("license-0bsd"),
        "Unicode-3.0" => d.starts_with("license-unicode"),
        "Apache-2.0" => d.starts_with("license-apache"),
        _ => false,
    }
}

/// Text vereinheitlicht: Zeilenenden LF, Leerraum am Zeilenende und am Ende entfernt.
fn normalisiert(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_string()
}

/// Gesammelte Lizenztexte, gleiche Texte zusammengefasst, in der Reihenfolge des ersten Auftretens.
#[derive(Default)]
struct Texte {
    gruppen: Vec<Gruppe>,
    /// Pakete, für die Apache-2.0 gewählt werden musste.
    apache: Vec<String>,
}

struct Gruppe {
    art: String,
    datei: String,
    text: String,
    pakete: Vec<String>,
    /// Satz zwischen Überschrift und Text (nur bei zusätzlichen Vermerken).
    hinweis: Option<String>,
}

impl Texte {
    fn aufnehmen(&mut self, art: &str, datei: &str, text: String, paket: String) {
        let text = normalisiert(&text);
        if let Some(g) = self
            .gruppen
            .iter_mut()
            .find(|g| g.art == art && g.text == text && g.hinweis.is_none())
        {
            if !g.pakete.contains(&paket) {
                g.pakete.push(paket);
            }
            return;
        }
        self.gruppen.push(Gruppe {
            art: art.to_string(),
            datei: datei.to_string(),
            text,
            pakete: vec![paket],
            hinweis: None,
        });
    }

    /// Lizenzdatei eines übernommenen Codes, als eigene Gruppe direkt nach dem Crate.
    fn zusatz_aufnehmen(&mut self, z: &Zusatz, tools: &Path) -> Result<(), String> {
        self.gruppen.push(Gruppe {
            art: format!("{} des übernommenen Codes", z.lizenz),
            datei: format!("tools/drittvermerke/{}", z.datei),
            text: normalisiert(&lesen(&tools.join(&z.datei))?),
            pakete: vec![format!("{} {}", z.name, z.version)],
            hinweis: Some(format!("{}.", z.herkunft)),
        });
        Ok(())
    }

    fn crate_aufnehmen(&mut self, c: &Crate) -> Result<(), String> {
        let paket = format!("{} {}", c.name, c.version);
        let dateien = lizenzdateien(&c.ordner);
        let gewaehlt = waehlen(&c.lizenz);
        let nur_eine = gewaehlt.len() == 1 && !c.lizenz.contains(" OR ") && !c.lizenz.contains('/');
        for id in &gewaehlt {
            if id == "Apache-2.0" {
                // Volltext einmal in Abschnitt 8.7; NOTICE-Dateien folgen unten.
                self.apache.push(paket.clone());
                continue;
            }
            let passend: Vec<&(String, PathBuf)> = dateien
                .iter()
                .filter(|(rel, _)| !rel.contains('/') && passt(id, rel, nur_eine))
                .collect();
            if passend.is_empty() {
                return Err(format!(
                    "{paket}: keine Lizenzdatei für {id} im Paket; Vermerk von Hand klären"
                ));
            }
            for (rel, pfad) in passend {
                self.aufnehmen(id, rel, lesen(pfad)?, paket.clone());
            }
        }
        for (rel, pfad) in &dateien {
            let klein = rel.to_lowercase();
            let basis = klein.rsplit('/').next().unwrap_or(&klein);
            if basis.starts_with("notice") {
                self.aufnehmen("NOTICE", rel, lesen(pfad)?, paket.clone());
            } else if rel.contains('/') {
                self.aufnehmen("weitere Lizenzdatei", rel, lesen(pfad)?, paket.clone());
            } else if basis.starts_with("copying") {
                self.aufnehmen("Erläuterung", rel, lesen(pfad)?, paket.clone());
            }
        }
        Ok(())
    }

    fn ausgeben(
        &self,
        std_teil: &StdTeil,
        unicode: &[(String, String)],
        sysroot: &Path,
        wurzel: &Path,
    ) -> Result<String, String> {
        let mut aus = String::new();
        aus.push_str("### 8.1 Crates\n\n");
        for g in &self.gruppen {
            let hinweis = g
                .hinweis
                .as_deref()
                .map(|h| format!("{h}\n\n"))
                .unwrap_or_default();
            aus.push_str(&format!(
                "#### {} ({}): {}\n\n{hinweis}{}\n\n",
                g.art,
                g.datei,
                g.pakete.join(", "),
                block(&g.text)
            ));
        }

        aus.push_str("### 8.2 Rust-Standardbibliothek (MIT gewählt)\n\n");
        // Nur Teile unter «Apache-2.0 OR MIT»; Unicode steht in 8.4, und die übrigen Ausnahmen
        // (BSD-2-Clause für Fuchsia) werden für dieses Ziel nicht übersetzt.
        let mut copyrights: Vec<&str> = Vec::new();
        for e in &std_teil.im_baum {
            if e.lizenz == "Apache-2.0 OR MIT" {
                for c in &e.copyright {
                    if !copyrights.contains(&c.as_str()) {
                        copyrights.push(c);
                    }
                }
            }
        }
        let mit = lesen(&sysroot.join("share/doc/rust/licenses/MIT.txt"))?;
        let mit_rumpf: String = normalisiert(&mit)
            .lines()
            .filter(|z| !z.starts_with("Copyright (c) <year>") && *z != "MIT License")
            .collect::<Vec<_>>()
            .join("\n");
        let kopf: Vec<String> = copyrights
            .iter()
            .map(|c| format!("Copyright (c) {c}"))
            .collect();
        aus.push_str(&format!(
            "Copyright-Angaben aus COPYRIGHT-library.html, Lizenztext aus `share/doc/rust/licenses/MIT.txt`:\n\n{}\n\n",
            block(&format!("{}\n\n{}", kopf.join("\n"), mit_rumpf.trim_matches('\n')))
        ));

        aus.push_str("### 8.3 compiler_builtins und libm\n\n");
        for e in &std_teil.eigene {
            aus.push_str(&format!(
                "#### {} ({}, {})\n\n{}\n\n",
                e.name,
                e.pfad,
                e.lizenz,
                block(&normalisiert(&lesen(
                    &wurzel.join("tools/drittvermerke/std").join(&e.datei)
                )?))
            ));
        }

        aus.push_str("### 8.4 Unicode\n\n");
        for (titel, text) in unicode {
            aus.push_str(&format!("#### {titel}\n\n{}\n\n", block(text)));
        }

        aus.push_str("### 8.5 MinGW-w64-Laufzeit (COPYING.MinGW-w64-runtime.txt)\n\n");
        aus.push_str(&block(&normalisiert(&lesen(
            &wurzel.join("tools/drittvermerke/COPYING.MinGW-w64-runtime.txt"),
        )?)));
        aus.push_str("\n\n### 8.6 GCC Runtime Library Exception 3.1 (libgcc)\n\n");
        aus.push_str(&block(&normalisiert(&lesen(
            &sysroot.join("share/doc/COPYING.RUNTIME"),
        )?)));
        aus.push_str("\n\n### 8.7 Apache License 2.0 und NOTICE-Dateien\n\n");
        if self.apache.is_empty() {
            aus.push_str(
                "Kein Crate aus den Abschnitten 2 und 5 wird unter Apache-2.0 genutzt. Der Volltext \
                 der Apache License 2.0 steht unverändert im Lizenztext von compiler_builtins \
                 (Abschnitt 8.3), zusammen mit den LLVM-Ausnahmen; er gilt dort für die aus LLVM \
                 compiler-rt abgeleiteten Teile.",
            );
        } else {
            aus.push_str(&format!(
                "Einmal im Volltext, für: {}.\n\n{}",
                self.apache.join(", "),
                block(&normalisiert(&lesen(
                    &sysroot.join("share/doc/rust/licenses/Apache-2.0.txt"),
                )?))
            ));
        }
        if self.gruppen.iter().any(|g| g.art == "NOTICE") {
            aus.push_str("\n\nNOTICE-Dateien stehen in Abschnitt 8.1.\n");
        } else {
            aus.push_str(
                "\n\nNOTICE-Dateien: keine. Gesucht in allen Paketen der Abschnitte 2 und 5 \
                 (Dateien, deren Name mit «NOTICE» beginnt, auch in Unterordnern).\n",
            );
        }
        Ok(aus)
    }
}

/// Text als Codeblock; der Zaun ist länger als jede Backtick-Folge im Text.
fn block(text: &str) -> String {
    let mut zaun = String::from("```");
    while text.contains(&zaun) {
        zaun.push('`');
    }
    format!("{zaun}text\n{text}\n{zaun}")
}

// --- Rust-Standardbibliothek -----------------------------------------------------------------

/// Sysroot der Werkzeugkette, die dieses Beispiel gebaut hat: `<sysroot>/bin/cargo`.
fn sysroot() -> Result<PathBuf, String> {
    let cargo = Path::new(env!("CARGO"));
    let sysroot = cargo
        .parent()
        .and_then(Path::parent)
        .ok_or("Sysroot nicht bestimmbar")?
        .to_path_buf();
    if !sysroot.join("lib").join("rustlib").is_dir() {
        return Err("Sysroot ohne lib/rustlib; Werkzeugkette über rustup erwartet".into());
    }
    Ok(sysroot)
}

struct ImBaum {
    pfad: String,
    lizenz: String,
    copyright: Vec<String>,
}

struct Eigene {
    name: String,
    pfad: String,
    lizenz: String,
    datei: String,
}

struct StdTeil {
    rustc: String,
    im_baum: Vec<ImBaum>,
    crates: Vec<Crate>,
    eigene: Vec<Eigene>,
}

impl StdTeil {
    fn lesen(sysroot: &Path, tools: &Path) -> Result<StdTeil, String> {
        let rustc = rustc_fassung(sysroot)?;
        let std_toml: toml::Table = toml::from_str(&lesen(&tools.join("std.toml"))?)
            .map_err(|e| format!("std.toml: {e}"))?;
        let soll = std_toml
            .get("rust")
            .and_then(|v| v.as_str())
            .ok_or("std.toml: «rust» fehlt")?;
        if rustc.split_whitespace().next() != Some(soll) {
            return Err(format!(
                "Werkzeugkette ist Rust {rustc}, tools/drittvermerke/std.toml gilt für {soll}; zuerst std.toml nachführen"
            ));
        }
        let s = |t: &toml::Value, k: &str| -> Result<String, String> {
            t.get(k)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("std.toml: «{k}» fehlt"))
        };
        let mut crates = Vec::new();
        for c in std_toml
            .get("crate")
            .and_then(|v| v.as_array())
            .ok_or("std.toml: keine [[crate]]")?
        {
            let (name, version) = (s(c, "name")?, s(c, "version")?);
            let ordner = tools.join("std").join(format!("{name}-{version}"));
            if !ordner.is_dir() {
                return Err(format!(
                    "tools/drittvermerke/std/{name}-{version} fehlt (Lizenzdatei ablegen)"
                ));
            }
            crates.push(Crate {
                lizenz: s(c, "license")?,
                name,
                version,
                ordner,
                proc_macro: false,
                manifest: None,
            });
        }
        let mut eigene = Vec::new();
        for e in std_toml
            .get("im_baum")
            .and_then(|v| v.as_array())
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            eigene.push(Eigene {
                name: s(e, "name")?,
                pfad: s(e, "pfad")?,
                lizenz: s(e, "license")?,
                datei: s(e, "datei")?,
            });
        }

        // Jede Bibliothek im Sysroot muss erklärt sein.
        let lib = sysroot.join("lib").join("rustlib").join(ZIEL).join("lib");
        let mut namen = BTreeSet::new();
        for e in fs::read_dir(&lib)
            .map_err(|_| format!("Sysroot ohne Bibliotheken für {ZIEL}"))?
            .flatten()
        {
            let datei = e.file_name().to_string_lossy().into_owned();
            if let Some(rest) = datei
                .strip_prefix("lib")
                .and_then(|r| r.strip_suffix(".rlib"))
            {
                let name = rest.rsplit_once('-').map_or(rest, |(n, _)| n);
                namen.insert(name.to_string());
            }
        }
        for n in &namen {
            let bekannt = STD_IM_BAUM.contains(&n.as_str())
                || STD_NICHT_GEBUNDEN.contains(&n.as_str())
                || crates.iter().any(|c| c.name.replace('-', "_") == *n);
            if !bekannt {
                return Err(format!(
                    "Sysroot enthält die Bibliothek «{n}», die tools/drittvermerke/std.toml nicht erklärt"
                ));
            }
        }

        let html = lesen(&sysroot.join("share/doc/rust/COPYRIGHT-library.html"))?;
        Ok(StdTeil {
            rustc,
            im_baum: im_baum_lesen(&html)?,
            crates,
            eigene,
        })
    }

    fn tabelle_im_baum(&self) -> String {
        let mut t = String::from("| Pfad im Rust-Baum | Lizenz | Copyright |\n|---|---|---|\n");
        for e in &self.im_baum {
            t.push_str(&format!(
                "| `{}` | {} | {} |\n",
                e.pfad,
                e.lizenz,
                e.copyright.join("; ")
            ));
        }
        t
    }

    fn tabelle_eigene(&self) -> String {
        let mut t = String::from("| Teil | Pfad im Rust-Baum | Lizenz |\n|---|---|---|\n");
        for e in &self.eigene {
            t.push_str(&format!("| {} | `{}` | {} |\n", e.name, e.pfad, e.lizenz));
        }
        t
    }
}

/// Rust-Fassung aus `lib/rustlib/multirust-channel-manifest.toml` (Abschnitt [pkg.rustc]).
fn rustc_fassung(sysroot: &Path) -> Result<String, String> {
    let text = lesen(
        &sysroot
            .join("lib")
            .join("rustlib")
            .join("multirust-channel-manifest.toml"),
    )?;
    let mut im_rustc = false;
    for zeile in text.lines() {
        let zeile = zeile.trim();
        if zeile.starts_with('[') {
            im_rustc = zeile == "[pkg.rustc]";
            continue;
        }
        let wert = zeile.strip_prefix("version = \"").filter(|_| im_rustc);
        if let Some(rest) = wert {
            return Ok(rest.trim_end_matches('"').to_string());
        }
    }
    Err("Rust-Fassung nicht gefunden".into())
}

/// Einträge «In-tree files» aus COPYRIGHT-library.html, mit vollem Pfad bei verschachtelten
/// Ausnahmen.
fn im_baum_lesen(html: &str) -> Result<Vec<ImBaum>, String> {
    let anfang = html
        .find("id=\"in-tree-files\"")
        .ok_or("COPYRIGHT-library.html: «In-tree files» fehlt")?;
    let ende = html[anfang..]
        .find("id=\"out-of-tree-dependencies\"")
        .map_or(html.len(), |e| anfang + e);
    let teil = &html[anfang..ende];

    let mut aus: Vec<ImBaum> = Vec::new();
    // Stapel der offenen <div>: Index des Eintrags darin (falls schon bekannt).
    let mut stapel: Vec<Option<usize>> = Vec::new();
    let mut i = 0;
    while i < teil.len() {
        let rest = &teil[i..];
        if rest.starts_with("<div") {
            stapel.push(None);
            i += 4;
        } else if rest.starts_with("</div>") {
            stapel.pop();
            i += 6;
        } else if let Some(r) = rest.strip_prefix("<b>File/Directory:</b> <code>") {
            let pfad = r.split('<').next().unwrap_or_default().trim().to_string();
            let eltern = stapel
                .iter()
                .rev()
                .skip(1)
                .find_map(|e| *e)
                .map(|j: usize| aus[j].pfad.clone());
            let voll = match eltern.as_deref() {
                Some(".") | None => pfad,
                Some(e) => format!("{e}/{pfad}"),
            };
            aus.push(ImBaum {
                pfad: voll,
                lizenz: String::new(),
                copyright: Vec::new(),
            });
            if let Some(oben) = stapel.last_mut() {
                *oben = Some(aus.len() - 1);
            }
            i += 1;
        } else if let Some(r) = rest.strip_prefix("<b>License:</b>") {
            let wert = r.split('<').next().unwrap_or_default().trim().to_string();
            if let Some(j) = stapel.last().copied().flatten() {
                aus[j].lizenz = wert;
            }
            i += 1;
        } else if let Some(r) = rest.strip_prefix("<b>Copyright:</b>") {
            let wert = r.split('<').next().unwrap_or_default().trim().to_string();
            if let Some(j) = stapel.last().copied().flatten() {
                aus[j].copyright.push(wert);
            }
            i += 1;
        } else {
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    if aus.is_empty() {
        return Err("COPYRIGHT-library.html: keine Einträge im Baum gefunden".into());
    }
    Ok(aus)
}

// --- Daten -----------------------------------------------------------------------------------

/// Unicode-Texte, die nicht schon als Lizenzdatei eines Crates in 8.1 stehen: die
/// Standardbibliothek und die CLDR-Vermerke aus den Köpfen von `lexika/*.toml`.
fn unicode_texte(
    lexika: &Path,
    sysroot: &Path,
    tools: &Path,
) -> Result<Vec<(String, String)>, String> {
    let mut aus = vec![
        (
            "Rust-Standardbibliothek, library/core/src/unicode (share/doc/rust/licenses/Unicode-3.0.txt)"
                .to_string(),
            normalisiert(&lesen(&sysroot.join("share/doc/rust/licenses/Unicode-3.0.txt"))?),
        ),
        (
            "Unicode-Daten 18.0.0 im Breitenmodell, src/ausgabe.rs (https://www.unicode.org/license.txt, abgerufen am 06.11.2026)"
                .to_string(),
            normalisiert(&lesen(&tools.join("unicode-license-v3.txt"))?),
        ),
    ];
    let mut dateien: Vec<PathBuf> = fs::read_dir(lexika)
        .map_err(|_| "lexika/ nicht lesbar")?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    dateien.sort();
    for d in dateien {
        let text = lesen(&d)?;
        if !text.contains("SPDX-License-Identifier: Unicode-3.0") {
            continue;
        }
        let kopf: Vec<&str> = text
            .lines()
            .take_while(|z| z.starts_with('#') || z.trim().is_empty())
            .collect();
        let start = kopf
            .iter()
            .position(|z| z.contains("Copyright ©") || z.contains("UNICODE LICENSE V3"))
            .ok_or("CLDR-Vermerk ohne Copyright-Zeile")?;
        let ende = kopf
            .iter()
            .position(|z| z.contains("SPDX-License-Identifier: Unicode-3.0"))
            .ok_or("CLDR-Vermerk ohne Ende")?;
        let zeilen: Vec<String> = kopf[start..=ende]
            .iter()
            .map(|z| z.trim_start_matches('#').to_string())
            .collect();
        let einzug = zeilen
            .iter()
            .filter(|z| !z.trim().is_empty())
            .map(|z| z.len() - z.trim_start().len())
            .min()
            .unwrap_or(0);
        let text: Vec<&str> = zeilen
            .iter()
            .map(|z| {
                if z.len() >= einzug {
                    &z[einzug..]
                } else {
                    z.trim()
                }
            })
            .collect();
        let name = d
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        aus.push((
            format!("Unicode CLDR, Vermerk aus dem Kopf von lexika/{name}"),
            normalisiert(&text.join("\n")),
        ));
    }
    Ok(aus)
}

/// Quellenangaben aus den Köpfen der Register (`# schluessel: wert`).
fn register_daten(ordner: &Path) -> Result<String, String> {
    let mut dateien: Vec<PathBuf> = fs::read_dir(ordner)
        .map_err(|_| "lexika/register/ nicht lesbar")?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .collect();
    dateien.sort();
    let mut aus = String::new();
    for d in dateien {
        let text = lesen(&d)?;
        let mut kopf: BTreeMap<String, String> = BTreeMap::new();
        for z in text.lines().take_while(|z| z.starts_with('#')) {
            if let Some((k, v)) = z.trim_start_matches('#').trim().split_once(": ") {
                kopf.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        let feld = |k: &str| kopf.get(k).map(String::as_str);
        let name = d
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let titel = feld("titel").ok_or_else(|| format!("{name}: «titel» fehlt im Kopf"))?;
        let lizenz = feld("lizenz").ok_or_else(|| format!("{name}: «lizenz» fehlt im Kopf"))?;
        aus.push_str(&format!("- `lexika/register/{name}`: {titel}.\n"));
        if let Some(h) = feld("herausgeber") {
            aus.push_str(&format!("  Herausgeber: {h}.\n"));
        }
        for (k, bezeichnung) in [
            ("datensatz", "Datensatz"),
            ("url", "Quelle"),
            ("url-2", "Quelle"),
            ("url-li", "Quelle (Liechtenstein)"),
        ] {
            if let Some(u) = feld(k) {
                aus.push_str(&format!("  {bezeichnung}: {u}\n"));
            }
        }
        let stand: Vec<String> = [
            ("veroeffentlicht", "veröffentlicht"),
            ("stichtag", "Stichtag"),
            ("abgerufen", "abgerufen"),
        ]
        .iter()
        .filter_map(|(k, b)| feld(k).map(|v| format!("{b} {v}")))
        .collect();
        if !stand.is_empty() {
            aus.push_str(&format!("  Stand: {}.\n", stand.join(", ")));
        }
        if let Some(q) = feld("quellenangabe") {
            aus.push_str(&format!("  Quellenangabe: {q}.\n"));
        }
        aus.push_str(&format!("  Nutzungsbedingungen: {lizenz}\n"));
    }
    Ok(aus)
}

/// Quellen der Wortlisten (`quelle` je Eintrag in `lexika/*.toml`), nach Gruppen gezählt.
fn quellen_daten(lexika: &Path) -> Result<String, String> {
    let mut dateien: Vec<PathBuf> = fs::read_dir(lexika)
        .map_err(|_| "lexika/ nicht lesbar")?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    dateien.sort();
    let mut gruppen: BTreeMap<&'static str, (usize, BTreeSet<String>)> = BTreeMap::new();
    for d in dateien {
        let t: toml::Table = toml::from_str(&lesen(&d)?).map_err(|e| {
            format!(
                "{}: {e}",
                d.file_name().unwrap_or_default().to_string_lossy()
            )
        })?;
        for wert in t.values() {
            let Some(liste) = wert.as_array() else {
                continue;
            };
            for e in liste {
                let Some(quelle) = e.get("quelle").and_then(|v| v.as_str()) else {
                    continue;
                };
                let url = e
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let gruppe = quellengruppe(quelle);
                let g = gruppen.entry(gruppe).or_default();
                g.0 += 1;
                if !url.is_empty() {
                    g.1.insert(url);
                }
            }
        }
    }
    let mut t = String::from("| Quelle | Einträge | Grundlage der Übernahme |\n|---|---|---|\n");
    for (gruppe, (anzahl, _)) in &gruppen {
        t.push_str(&format!(
            "| {gruppe} | {anzahl} | {} |\n",
            grundlage(gruppe)
        ));
    }
    for gruppe in ["Duden online", "Larousse", "Treccani"] {
        if let Some((_, urls)) = gruppen.get(gruppe) {
            t.push_str(&format!(
                "\nZitierte Stichwörter, {gruppe}: {}\n",
                urls.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    Ok(t)
}

fn quellengruppe(quelle: &str) -> &'static str {
    if quelle.starts_with("SR ") || quelle.contains("(SR ") {
        "Erlasse des Bundes (SR)"
    } else if quelle.starts_with("Duden") {
        "Duden online"
    } else if quelle.starts_with("Larousse") {
        "Larousse"
    } else if quelle.starts_with("Treccani") {
        "Treccani"
    } else if quelle.starts_with("Unicode CLDR") {
        "Unicode CLDR"
    } else {
        "weitere: Bundeskanzlei, BFS, Bundesgericht"
    }
}

fn grundlage(gruppe: &str) -> &'static str {
    match gruppe {
        "Erlasse des Bundes (SR)" => "Art. 5 Abs. 1 Bst. a URG: nicht geschützt",
        "Duden online" | "Larousse" | "Treccani" => {
            "Zitat nach Art. 25 URG, mit Quelle und Adresse je Eintrag"
        }
        "Unicode CLDR" => "Unicode License V3 (Abschnitt 8.4)",
        _ => "einzelne Wörter und Abkürzungen, je Eintrag mit Fundstelle belegt",
    }
}
