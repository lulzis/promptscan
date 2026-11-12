//! Alle Belege von promptscan als JSON, für die Belegprüfung gegen Fedlex (SPEC 10).
//!
//! `cargo run --example belege > belege.json` gibt aus: jeden Wortlisteneintrag, den Kopf
//! jedes Registers, jede zitierte Norm (aus `normen.rs`, den Rollen und den Regeln) und jede
//! Quelle der Regeln. Das Beispiel greift nicht aufs Netz; geprüft wird ausserhalb.

use std::collections::BTreeMap;

use promptscan::katalog::{self, Stand};
use promptscan::lexikon::{self, Lexikon, Register};
use promptscan::normen::{self, Norm};
use promptscan::{Rolle, stufen};
use serde_json::{Value, json};

fn main() {
    let mut eintraege = Vec::new();
    let mut register = Vec::new();
    for datei in lexikon::dateien() {
        let inhalt = lexikon::inhalt(datei).expect("eingebettet");
        if datei.ends_with(".toml") {
            let lex = Lexikon::aus_toml(datei, inhalt).expect("gültige Wortliste");
            for (i, e) in lex.eintraege.iter().enumerate() {
                let mut v = serde_json::to_value(e).expect("serialisierbar");
                v["datei"] = json!(datei);
                v["index"] = json!(i);
                eintraege.push(v);
            }
        } else {
            let reg = Register::aus_tsv(datei, inhalt).expect("gültiges Register");
            let kopf: BTreeMap<&str, &str> = reg
                .kopf
                .iter()
                .map(|(k, w)| (k.as_str(), w.as_str()))
                .collect();
            register.push(json!({
                "datei": datei,
                "kopf": kopf,
                "spalten": reg.spalten,
                "zeilen": reg.zeilen.len(),
            }));
        }
    }

    // Jede Norm einmal, mit allen Stellen, die sie benutzen.
    let regeln = katalog::alle();
    let mut normen_nach_schluessel: BTreeMap<(&str, &str), (&Norm, Vec<String>)> = BTreeMap::new();
    let mut merken = |n: &'static Norm, wer: String| {
        normen_nach_schluessel
            .entry((n.zitat, n.url))
            .or_insert((n, Vec::new()))
            .1
            .push(wer);
    };
    for &n in normen::ALLE {
        merken(n, "normen.rs".to_string());
    }
    for rolle in Rolle::ALLE {
        for n in stufen::normen_fuer(rolle) {
            merken(n, format!("rolle:{}", rolle.name()));
        }
    }
    for r in &regeln {
        for &n in &r.grundlage {
            merken(n, format!("regel:{}", r.id));
        }
    }
    let normen_aus: Vec<Value> = normen_nach_schluessel
        .values()
        .map(|(n, wer)| {
            json!({
                "zitat": n.zitat,
                "erlass": n.erlass,
                "fassung": n.fassung,
                "url": n.url,
                "wortlaut": n.wortlaut,
                "benutzt": wer,
            })
        })
        .collect();

    let quellen: Vec<Value> = regeln
        .iter()
        .flat_map(|r| {
            r.quellen.iter().map(move |q| {
                let (stand, notiz) = match q.stand {
                    Stand::Geprueft(d) => ("geprueft", d),
                    Stand::ZuPruefen(w) => ("zu pruefen", w),
                };
                json!({
                    "regel": r.id,
                    "titel": q.titel,
                    "url": q.url,
                    "lizenz": q.lizenz,
                    "stand": stand,
                    "notiz": notiz,
                })
            })
        })
        .collect();

    let alles = json!({
        "eintraege": eintraege,
        "register": register,
        "normen": normen_aus,
        "quellen": quellen,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&alles).expect("serialisierbar")
    );
}
