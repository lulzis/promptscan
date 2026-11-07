//! Selbstbezeichnung und Personenbezug von aussen, über `pruefe`. Alle Sätze sind erfunden.

use promptscan::{Optionen, Stufe, pruefe};

/// Eine Selbstbezeichnung («Als Ärztin frage ich») bestimmt die Rolle, bezeichnet aber
/// keine andere Person: Ein besonders schützenswerter Begriff im selben Satz bleibt ohne
/// Fund. «Meine Patientin» bezeichnet eine bestimmte Person: Derselbe Begriff wird ein
/// HINWEIS (SPEC 4.4).
#[test]
fn selbstbezeichnung_ist_kein_personenbezug() {
    for (satz, erwartet) in [
        (
            "Als Ärztin frage ich mich, was bei Arbeitsunfähigkeit gilt.",
            None,
        ),
        (
            "Meine Patientin ist arbeitsunfähig.",
            Some("arbeitsunfähig"),
        ),
        (
            "En tant que médecin, je me demande ce qui vaut en cas d'incapacité de travail.",
            None,
        ),
        (
            "Ma patiente est en incapacité de travail.",
            Some("incapacité de travail"),
        ),
        (
            "Come medico mi chiedo cosa valga in caso di incapacità al lavoro.",
            None,
        ),
        (
            "La mia paziente è in incapacità al lavoro.",
            Some("incapacità al lavoro"),
        ),
    ] {
        let bericht = pruefe(satz, &Optionen::default());
        assert!(bericht.lage.rolle().is_some(), "{satz}: {:?}", bericht.lage);
        let funde: Vec<(&str, Stufe)> = bericht
            .funde
            .iter()
            .map(|f| (f.stelle.as_str(), f.stufe))
            .collect();
        match erwartet {
            None => assert!(funde.is_empty(), "{satz}: {funde:?}"),
            Some(stelle) => assert_eq!(funde, vec![(stelle, Stufe::Hinweis)], "{satz}"),
        }
    }
}
