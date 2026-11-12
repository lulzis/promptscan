//! Lizenzstand von promptscan und Vermerke zu den fremden Bestandteilen der Programmdatei,
//! für `promptscan licenses`.
//!
//! Beide Dateien sind eingebettet, damit die Vermerke mit jeder Kopie der Programmdatei
//! weitergehen: Die MIT-Lizenz verlangt sie «in all copies or substantial portions of the
//! Software». `THIRD-PARTY-NOTICES.md` erzeugt `examples/drittvermerke.rs`.

/// Lizenzstand von promptscan (`LICENSE.md`).
pub const LIZENZ: &str = include_str!("../LICENSE.md");

/// Vermerke zu den fremden Bestandteilen (`THIRD-PARTY-NOTICES.md`).
pub const DRITTVERMERKE: &str = include_str!("../THIRD-PARTY-NOTICES.md");

/// Ausgabe von `promptscan licenses`: ein kurzer Kopf in der Sprache der Ausgabe, dann beide
/// Dateien im Wortlaut.
pub fn text(englisch: bool) -> String {
    let kopf = if englisch {
        "promptscan: licence status and third-party notices. LICENSE.md (with an English \
         section) and THIRD-PARTY-NOTICES.md (in German; licence texts in their original \
         wording) follow verbatim."
    } else {
        "promptscan: Lizenzstand und Vermerke zu fremden Bestandteilen. Es folgen LICENSE.md \
         und THIRD-PARTY-NOTICES.md im Wortlaut."
    };
    format!(
        "{kopf}\n\n{}\n\n{}\n",
        LIZENZ.trim_end(),
        DRITTVERMERKE.trim_end()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn datei(name: &str) -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    /// Normale Abhängigkeiten laut Cargo.toml (ohne dev- und build-dependencies), mit dem
    /// Paketnamen bei Umbenennung, auch aus Ziel-Abschnitten.
    fn normale_abhaengigkeiten(manifest: &toml::Table) -> BTreeSet<String> {
        let mut namen = BTreeSet::new();
        let mut aus = |t: &toml::Table| {
            if let Some(d) = t.get("dependencies").and_then(|v| v.as_table()) {
                for (schluessel, wert) in d {
                    let name = wert
                        .get("package")
                        .and_then(|v| v.as_str())
                        .unwrap_or(schluessel);
                    namen.insert(name.to_string());
                }
            }
        };
        aus(manifest);
        if let Some(ziele) = manifest.get("target").and_then(|v| v.as_table()) {
            for z in ziele.values().filter_map(|v| v.as_table()) {
                aus(z);
            }
        }
        namen
    }

    /// Alle Pakete aus Cargo.lock, die von den normalen Abhängigkeiten aus erreichbar sind,
    /// als (Name, Fassung). Das schliesst prozedurale Makros und Pakete ein, die erst eine
    /// nicht eingeschaltete Funktion braucht; THIRD-PARTY-NOTICES.md führt sie in eigenen
    /// Abschnitten.
    fn huelle() -> BTreeSet<(String, String)> {
        let lock: toml::Table = toml::from_str(&datei("Cargo.lock")).expect("Cargo.lock");
        let manifest: toml::Table = toml::from_str(&datei("Cargo.toml")).expect("Cargo.toml");
        let pakete: Vec<(String, String, Vec<String>)> = lock["package"]
            .as_array()
            .expect("[[package]] in Cargo.lock")
            .iter()
            .map(|p| {
                let s = |k: &str| p[k].as_str().expect(k).to_string();
                let abh = p
                    .get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                (s("name"), s("version"), abh)
            })
            .collect();
        // Ein Eintrag lautet «name», «name fassung» oder «name fassung (quelle)».
        let finde = |eintrag: &String| -> usize {
            let mut teile = eintrag.split_whitespace();
            let name = teile.next().unwrap_or_default();
            let fassung = teile.next();
            let treffer: Vec<usize> = pakete
                .iter()
                .enumerate()
                .filter(|(_, (n, v, _))| n == name && fassung.is_none_or(|f| v == f))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(treffer.len(), 1, "Cargo.lock: «{eintrag}» nicht eindeutig");
            treffer[0]
        };
        let wurzel = pakete
            .iter()
            .position(|(n, _, _)| n == env!("CARGO_PKG_NAME"))
            .expect("eigenes Paket in Cargo.lock");
        let normal = normale_abhaengigkeiten(&manifest);
        let mut stapel: Vec<usize> = pakete[wurzel]
            .2
            .iter()
            .map(finde)
            .filter(|&i| normal.contains(&pakete[i].0))
            .collect();
        let mut gesehen = BTreeSet::new();
        while let Some(i) = stapel.pop() {
            if gesehen.insert(i) {
                stapel.extend(pakete[i].2.iter().map(finde));
            }
        }
        gesehen
            .into_iter()
            .map(|i| (pakete[i].0.clone(), pakete[i].1.clone()))
            .collect()
    }

    /// Pakete, die in den Tabellen der Vermerke fehlen (Zeile «| `name` | fassung |»).
    fn fehlende(huelle: &BTreeSet<(String, String)>, vermerke: &str) -> Vec<String> {
        let vermerke = vermerke.replace("\r\n", "\n");
        huelle
            .iter()
            .filter(|(n, v)| !vermerke.contains(&format!("| `{n}` | {v} |")))
            .map(|(n, v)| format!("{n} {v}"))
            .collect()
    }

    #[test]
    fn jede_normale_abhaengigkeit_steht_in_den_vermerken() {
        let huelle = huelle();
        // Stand 06.11.2026: 10 direkte, 44 insgesamt; weniger hiesse, die Hülle ist falsch.
        assert!(huelle.len() >= 10, "nur {} Pakete gefunden", huelle.len());
        // Direkte und mittelbare (regex-syntax über regex, proc-macro2 über serde_derive).
        for name in [
            "clap",
            "regex",
            "serde",
            "toml",
            "regex-syntax",
            "proc-macro2",
        ] {
            assert!(
                huelle.iter().any(|(n, _)| n == name),
                "{name} fehlt in der Hülle"
            );
        }
        let fehlen = fehlende(&huelle, DRITTVERMERKE);
        assert!(
            fehlen.is_empty(),
            "THIRD-PARTY-NOTICES.md ist veraltet, es fehlen: {}. Nachführen nach Abschnitt 9 \
             der Datei.",
            fehlen.join(", ")
        );
    }

    #[test]
    fn gegenprobe_fehlendes_paket_faellt_auf() {
        let echt = huelle();
        let mut erweitert = echt.clone();
        erweitert.insert(("erfundenes-paket".into(), "9.9.9".into()));
        assert_eq!(
            fehlende(&erweitert, DRITTVERMERKE),
            vec!["erfundenes-paket 9.9.9".to_string()]
        );
        // Eine andere Fassung zählt nicht als vorhanden.
        let (_, fassung) = echt.iter().find(|(n, _)| n == "regex").expect("regex");
        let alt = DRITTVERMERKE.replace(&format!("| `regex` | {fassung} |"), "| `regex` | 0.0.1 |");
        assert_eq!(fehlende(&echt, &alt), vec![format!("regex {fassung}")]);
    }

    #[test]
    fn lizenzstand_ohne_platzhalter_und_ohne_namen() {
        assert!(
            LIZENZ.contains("Alle Rechte vorbehalten. Für promptscan ist keine Lizenz erteilt.")
        );
        assert!(LIZENZ.contains("All rights reserved. No licence is granted for promptscan."));
        assert!(LIZENZ.contains("Zustimmung des Rechteinhabers"));
        assert!(LIZENZ.contains("consent of the rights holder"));
        assert!(
            !LIZENZ.contains('['),
            "kein Platzhalter in eckigen Klammern"
        );
        assert!(
            !LIZENZ.contains("Copyright"),
            "kein Name in der Lizenzdatei"
        );
    }

    #[test]
    fn keine_lokalen_pfade() {
        for (name, inhalt) in [
            ("LICENSE.md", LIZENZ),
            ("THIRD-PARTY-NOTICES.md", DRITTVERMERKE),
        ] {
            let klein = inhalt.to_lowercase();
            for muster in [
                "c:\\users\\",
                "c:/users/",
                "/home/",
                "appdata",
                ".cargo",
                ".rustup",
            ] {
                assert!(!klein.contains(muster), "{name} enthält «{muster}»");
            }
        }
    }

    #[test]
    fn text_hat_kopf_und_beide_dateien() {
        let de = text(false);
        assert!(de.starts_with("promptscan: Lizenzstand und Vermerke"));
        let en = text(true);
        assert!(en.starts_with("promptscan: licence status and third-party notices."));
        for t in [&de, &en] {
            let lizenz = t.find("# Lizenz").expect("LICENSE.md");
            let vermerke = t
                .find("# Vermerke zu fremden Bestandteilen")
                .expect("THIRD-PARTY-NOTICES.md");
            assert!(lizenz < vermerke, "erst der eigene Lizenzstand");
            assert!(t.ends_with("\n") && !t.ends_with("\n\n"));
        }
    }

    // --- Vollständigkeit: alle Pakete aus Cargo.lock, Daten aus lexika/ ---------------------

    /// Alle fremden Pakete aus Cargo.lock, als (Name, Fassung).
    fn alle_pakete() -> BTreeSet<(String, String)> {
        let lock: toml::Table = toml::from_str(&datei("Cargo.lock")).expect("Cargo.lock");
        lock["package"]
            .as_array()
            .expect("[[package]] in Cargo.lock")
            .iter()
            .map(|p| {
                let s = |k: &str| p[k].as_str().expect(k).to_string();
                (s("name"), s("version"))
            })
            .filter(|(n, _)| n != env!("CARGO_PKG_NAME"))
            .collect()
    }

    #[test]
    fn jedes_paket_aus_cargo_lock_steht_in_den_vermerken() {
        let alle = alle_pakete();
        // Stand 06.11.2026: 90 fremde Pakete, 44 davon aus den normalen Abhängigkeiten, die
        // übrigen nur für Tests (Abschnitt 4.2).
        assert!(
            alle.len() > huelle().len(),
            "Cargo.lock ohne Pakete der dev-dependencies?"
        );
        let fehlen = fehlende(&alle, DRITTVERMERKE);
        assert!(
            fehlen.is_empty(),
            "THIRD-PARTY-NOTICES.md ist veraltet, es fehlen: {}. Nachführen nach Abschnitt 9 \
             der Datei.",
            fehlen.join(", ")
        );
    }

    /// Text der Vermerke von der Überschrift `von` bis vor die Überschrift `bis`.
    fn zwischen<'a>(text: &'a str, von: &str, bis: &str) -> &'a str {
        let a = text.find(von).unwrap_or_else(|| panic!("«{von}» fehlt"));
        let rest = &text[a..];
        &rest[..rest
            .find(bis)
            .unwrap_or_else(|| panic!("«{bis}» fehlt nach «{von}»"))]
    }

    /// Zeilen ohne Leerraum an den Enden, leere weggelassen.
    fn zeilen<'a>(z: impl Iterator<Item = &'a str>) -> Vec<String> {
        z.map(str::trim)
            .filter(|z| !z.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Dateien eines Ordners unter lexika/ (rekursiv, .toml und .tsv wie in build.rs), mit Pfad
    /// relativ zu `basis` und Inhalt; `ohne` wird übergangen.
    fn lexika_dateien(basis: &Path, ordner: &Path, ohne: &Path, aus: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(ordner).expect("lexika/ lesbar").flatten() {
            let p = e.path();
            if p.is_dir() {
                if p != ohne {
                    lexika_dateien(basis, &p, ohne, aus);
                }
            } else if p.extension().is_some_and(|x| x == "toml" || x == "tsv") {
                let relativ = p
                    .strip_prefix(basis)
                    .expect("unter lexika/")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                aus.push((relativ, std::fs::read_to_string(&p).expect("lesbar")));
            }
        }
        aus.sort();
    }

    fn lexika() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("lexika")
    }

    /// Die Register unter lexika/register/.
    fn register() -> Vec<(String, String)> {
        let mut aus = Vec::new();
        let ordner = lexika().join("register");
        lexika_dateien(&ordner, &ordner, Path::new(""), &mut aus);
        aus
    }

    /// Die übrigen Dateien unter lexika/ (ohne lexika/register/).
    fn wortlisten() -> Vec<(String, String)> {
        let mut aus = Vec::new();
        lexika_dateien(&lexika(), &lexika(), &lexika().join("register"), &mut aus);
        aus
    }

    /// Angaben aus dem Kopf eines Registers (`# schluessel: wert`), die Abschnitt 7.2 wortgleich
    /// wiedergeben muss.
    const REGISTERANGABEN: &[&str] = &[
        "titel",
        "herausgeber",
        "datensatz",
        "url",
        "url-2",
        "url-li",
        "veroeffentlicht",
        "stichtag",
        "abgerufen",
        "quellenangabe",
        "lizenz",
    ];

    /// Abweichungen zwischen den Registern (Name, Inhalt) und Abschnitt 7.2.
    fn registerfehler(register: &[(String, String)], abschnitt: &str) -> Vec<String> {
        let mut aus = Vec::new();
        for (name, text) in register {
            if !abschnitt.contains(&format!("`lexika/register/{name}`")) {
                aus.push(format!("{name}: fehlt"));
                continue;
            }
            for z in text.lines().take_while(|z| z.starts_with('#')) {
                let Some((k, v)) = z.trim_start_matches('#').trim().split_once(": ") else {
                    continue;
                };
                if REGISTERANGABEN.contains(&k.trim()) && !abschnitt.contains(v.trim()) {
                    aus.push(format!("{name}: «{}» nicht wortgleich", k.trim()));
                }
            }
        }
        for teil in abschnitt.split("`lexika/register/").skip(1) {
            // «`lexika/register/`» allein ist der Ordner, keine Datei.
            let name = teil.split('`').next().unwrap_or_default();
            if !name.is_empty() && !register.iter().any(|(n, _)| n == name) {
                aus.push(format!("{name}: genannt, aber nicht vorhanden"));
            }
        }
        aus
    }

    /// Lizenzkopf einer Datei: die Kommentarzeilen am Dateianfang von der ersten Zeile mit
    /// «Copyright ©», «Copyright (c)» oder «LICENSE V» bis zur Zeile
    /// «SPDX-License-Identifier:», ohne «#». None, wenn der Kopf keine Lizenz nennt; Err, wenn
    /// er eine nennt, aber Anfang oder Ende fehlen (dann deckt der Generator ihn nicht ab).
    fn lizenzkopf(text: &str) -> Option<Result<Vec<String>, String>> {
        let kopf: Vec<&str> = text
            .lines()
            .take_while(|z| z.starts_with('#') || z.trim().is_empty())
            .map(|z| z.trim_start_matches('#').trim())
            .collect();
        let nennt = kopf.iter().any(|z| {
            let z = z.to_lowercase();
            ["lizenz", "license", "licence", "copyright", "spdx", "©"]
                .iter()
                .any(|m| z.contains(m))
        });
        if !nennt {
            return None;
        }
        let anfang = kopf.iter().position(|z| {
            z.contains("Copyright ©") || z.contains("Copyright (c)") || z.contains("LICENSE V")
        });
        let ende = kopf
            .iter()
            .position(|z| z.starts_with("SPDX-License-Identifier:"));
        Some(match (anfang, ende) {
            (Some(a), Some(e)) if a <= e => Ok(zeilen(kopf[a..=e].iter().copied())),
            _ => Err(
                "Lizenzkopf ohne Anfang («Copyright ©», «LICENSE V…») oder Ende \
                      («SPDX-License-Identifier:»); Generator und Abschnitt 7 nachführen"
                    .to_string(),
            ),
        })
    }

    /// Der erste Textblock nach der ersten Überschrift «#### …», auf die `ueberschrift` passt.
    fn block_nach(abschnitt: &str, ueberschrift: impl Fn(&str) -> bool) -> Option<Vec<String>> {
        let mut z = abschnitt.lines();
        z.by_ref()
            .find(|z| z.starts_with("#### ") && ueberschrift(z.trim_end()))?;
        z.by_ref().find(|z| z.starts_with("```"))?;
        Some(zeilen(z.take_while(|z| !z.starts_with("```"))))
    }

    /// Abweichungen zwischen den Lizenzköpfen der Dateien und Abschnitt 8.
    fn lizenzkopffehler(dateien: &[(String, String)], abschnitt: &str) -> Vec<String> {
        let mut aus = Vec::new();
        for (name, text) in dateien {
            let marke = format!("lexika/{name}");
            match lizenzkopf(text) {
                None => {}
                Some(Err(e)) => aus.push(format!("{marke}: {e}")),
                Some(Ok(kopf)) => match block_nach(abschnitt, |u| u.ends_with(&marke)) {
                    None => aus.push(format!("{marke}: Vermerk fehlt")),
                    Some(block) if block != kopf => {
                        aus.push(format!("{marke}: Vermerk nicht wortgleich"))
                    }
                    Some(_) => {}
                },
            }
        }
        aus
    }

    #[test]
    fn jedes_register_steht_wortgleich_in_abschnitt_7_2() {
        let register = register();
        // Stand 06.11.2026: gemeinden, nachnamen, plz, strassentypen, vornamen.
        assert!(
            register.len() >= 5,
            "nur {} Register gefunden",
            register.len()
        );
        let fehler = registerfehler(&register, zwischen(DRITTVERMERKE, "### 7.2 ", "### 7.3 "));
        assert!(
            fehler.is_empty(),
            "THIRD-PARTY-NOTICES.md, Abschnitt 7.2, ist veraltet: {}. Nachführen nach \
             Abschnitt 9 der Datei.",
            fehler.join("; ")
        );
    }

    #[test]
    fn jeder_lizenzkopf_aus_lexika_steht_wortgleich_in_abschnitt_8() {
        let dateien = wortlisten();
        // Stand 06.11.2026: abkuerzungen.toml und geburtsdatum-monate.toml (Unicode CLDR).
        let mit_kopf = dateien
            .iter()
            .filter(|(_, t)| lizenzkopf(t).is_some())
            .count();
        assert!(mit_kopf >= 2, "nur {mit_kopf} Lizenzköpfe gefunden");
        let fehler = lizenzkopffehler(&dateien, zwischen(DRITTVERMERKE, "## 8. ", "## 9. "));
        assert!(
            fehler.is_empty(),
            "THIRD-PARTY-NOTICES.md, Abschnitt 8, ist veraltet: {}. Nachführen nach Abschnitt \
             9 der Datei.",
            fehler.join("; ")
        );
    }

    #[test]
    fn jeder_zusatzvermerk_steht_wortgleich_in_abschnitt_8_1() {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/drittvermerke");
        let liste: toml::Table = toml::from_str(
            &std::fs::read_to_string(wurzel.join("zusatz.toml")).expect("zusatz.toml"),
        )
        .expect("zusatz.toml");
        let eintraege = liste["zusatz"].as_array().expect("[[zusatz]]");
        assert!(!eintraege.is_empty(), "Stand 06.11.2026: zmij");
        let abschnitt = zwischen(DRITTVERMERKE, "### 8.1 ", "### 8.2 ");
        for e in eintraege {
            let s = |k: &str| e[k].as_str().expect(k);
            let text = std::fs::read_to_string(wurzel.join(s("datei"))).expect("Zusatzdatei");
            let marke = format!(
                "(tools/drittvermerke/{}): {} {}",
                s("datei"),
                s("crate"),
                s("version")
            );
            assert_eq!(
                block_nach(abschnitt, |u| u.ends_with(&marke)),
                Some(zeilen(text.lines())),
                "THIRD-PARTY-NOTICES.md, Abschnitt 8.1: Vermerk zu {} fehlt oder ist veraltet",
                s("crate")
            );
        }
    }

    #[test]
    fn gegenprobe_datenteil_faellt_auf() {
        // Register: neu, geänderte Nutzungsbedingungen, entfernt.
        let register = register();
        let abschnitt = zwischen(DRITTVERMERKE, "### 7.2 ", "### 7.3 ");
        let mut geaendert = register.clone();
        geaendert[0].1 = geaendert[0]
            .1
            .replacen("# lizenz: ", "# lizenz: GEÄNDERT ", 1);
        geaendert.push(("neu.tsv".into(), "# titel: Neues Register\n".into()));
        let fehler = registerfehler(&geaendert, abschnitt);
        assert_eq!(
            fehler,
            vec![
                format!("{}: «lizenz» nicht wortgleich", register[0].0),
                "neu.tsv: fehlt".to_string()
            ]
        );
        let (letztes, ohne_letztes) = register.split_last().expect("Register");
        assert_eq!(
            registerfehler(ohne_letztes, abschnitt),
            vec![format!("{}: genannt, aber nicht vorhanden", letztes.0)]
        );

        // Lizenzköpfe: geänderter Vermerk, fehlender Vermerk, Kopf ohne erkennbaren Rahmen.
        let abschnitt = zwischen(DRITTVERMERKE, "## 8. ", "## 9. ");
        let dateien = wortlisten();
        let (name, text) = dateien
            .iter()
            .find(|(_, t)| matches!(lizenzkopf(t), Some(Ok(_))))
            .expect("eine Wortliste mit Lizenzkopf");
        let anders = text.replacen("Unicode, Inc.", "Unicode, Inc. (geändert)", 1);
        assert_eq!(
            lizenzkopffehler(&[(name.clone(), anders)], abschnitt),
            vec![format!("lexika/{name}: Vermerk nicht wortgleich")]
        );
        assert_eq!(
            lizenzkopffehler(&[("neu.toml".into(), text.clone())], abschnitt),
            vec!["lexika/neu.toml: Vermerk fehlt".to_string()]
        );
        let fremd = "# Lizenz: CC BY 4.0\n\n[[eintrag]]\n".to_string();
        assert_eq!(
            lizenzkopffehler(&[("cc.toml".into(), fremd)], abschnitt).len(),
            1
        );
        assert!(lizenzkopf("# Monatsnamen\n\n[[eintrag]]\n").is_none());
    }
}
