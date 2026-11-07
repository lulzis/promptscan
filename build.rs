//! Bettet alle Wortlisten und Register unter `lexika/` in das Programm ein.
//!
//! Neue Dateien werden ohne Codeänderung erfasst. Zur Laufzeit liest
//! promptscan keine Datei ausser der Eingabe und der Konfiguration.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let wurzel =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("lexika");
    println!("cargo:rerun-if-changed=lexika");

    let mut dateien = Vec::new();
    if wurzel.is_dir() {
        sammeln(&wurzel, &wurzel, &mut dateien);
    }
    dateien.sort();

    let mut code = String::from(
        "/// Alle Dateien unter `lexika/`, beim Bauen eingebettet: (relativer Pfad, Inhalt).\n\
         pub static DATEIEN: &[(&str, &str)] = &[\n",
    );
    for (relativ, absolut) in &dateien {
        code.push_str(&format!("    ({relativ:?}, include_str!({absolut:?})),\n"));
    }
    code.push_str("];\n");

    let ziel = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("lexika.rs");
    fs::write(ziel, code).expect("lexika.rs schreiben");
}

fn sammeln(wurzel: &Path, ordner: &Path, dateien: &mut Vec<(String, String)>) {
    let mut eintraege: Vec<_> = fs::read_dir(ordner)
        .expect("lexika lesen")
        .flatten()
        .collect();
    eintraege.sort_by_key(|e| e.path());
    for eintrag in eintraege {
        let pfad = eintrag.path();
        if pfad.is_dir() {
            sammeln(wurzel, &pfad, dateien);
            continue;
        }
        let endung = pfad.extension().and_then(|e| e.to_str()).unwrap_or("");
        if endung != "toml" && endung != "tsv" {
            continue;
        }
        let relativ = pfad
            .strip_prefix(wurzel)
            .expect("Pfad unter lexika")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        dateien.push((relativ, pfad.to_string_lossy().into_owned()));
    }
}
