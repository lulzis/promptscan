//! Die Regeln art5c-1 bis art5c-6 von aussen: über `pruefe` und über die Stufenlogik.
//!
//! Der Identifikator wird von Hand gesetzt oder als erfundene AHV-Nummer geschrieben;
//! Namen erkennt die Regel «name», auf die sich die Namensproben stützen. Alle Personen
//! und Sachverhalte sind erfunden.

use promptscan::regeln::art5c;
use promptscan::stufen::bewerte;
use promptscan::text::Text;
use promptscan::{Art, Fund, Grund, Optionen, Stufe, Treffer, pruefe};

/// Ein Treffer «Identifikator» an der Stelle `stelle`.
fn von_hand(text: &Text, stelle: &str, art: Art) -> Treffer {
    let start = text.als_str().find(stelle).expect("Stelle im Text");
    Treffer::neu("test-hand", start, start + stelle.len(), art)
}

/// Bewertet die art5c-Treffer zusammen mit einem Namen von Hand.
fn mit_name(roh: &str, name: &str) -> Vec<Fund> {
    let text = Text::neu(roh);
    let mut treffer = art5c::erkenne(&text);
    treffer.push(von_hand(&text, name, Art::Identifikator));
    bewerte(&text, treffer, &Optionen::default()).0
}

#[test]
fn reine_rechtsfrage_gibt_keinen_fund() {
    for frage in [
        "Was gilt bei Arbeitsunfähigkeit?",
        "Que vaut l'incapacité de travail en droit suisse ?",
        "Cosa vale in caso di incapacità al lavoro?",
    ] {
        let bericht = pruefe(frage, &Optionen::default());
        assert!(bericht.funde.is_empty(), "«{frage}»: {:?}", bericht.funde);
    }
}

#[test]
fn rechtsfrage_hat_trotzdem_einen_treffer() {
    // Gegenprobe zur stillen Rechtsfrage: Die Regel erkennt den Begriff; still bleibt
    // es nur, weil die Stufenlogik ohne Identifikator nichts meldet.
    let text = Text::neu("Was gilt bei Arbeitsunfähigkeit?");
    let t = art5c::erkenne(&text);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].art, Art::BesondersSchuetzenswert(2));
}

#[test]
fn name_und_arbeitsunfaehig_ist_orange() {
    let funde = mit_name("Hans Muster ist arbeitsunfähig.", "Hans Muster");
    let f = funde
        .iter()
        .find(|f| f.regel == "art5c-2")
        .expect("art5c-2 gemeldet");
    assert_eq!(f.stufe, Stufe::Orange);
    assert_eq!(f.grund, Grund::BesondersSchuetzenswert { ziffer: 2 });
    assert_eq!(f.stelle, "arbeitsunfähig");
    let beleg = f.beleg.as_ref().expect("Beleg");
    assert_eq!(beleg.quelle, "SR 832.20 Art. 16 Abs. 1");
}

#[test]
fn gegenprobe_ohne_regel_kein_orange() {
    // Ohne die art5c-Treffer bleibt derselbe Prompt bei GELB für den Namen.
    let text = Text::neu("Hans Muster ist arbeitsunfähig.");
    let nur_name = vec![von_hand(&text, "Hans Muster", Art::Identifikator)];
    let funde = bewerte(&text, nur_name, &Optionen::default()).0;
    assert!(funde.iter().all(|f| f.stufe != Stufe::Orange), "{funde:?}");
}

#[test]
fn jede_ziffer_wird_mit_name_orange() {
    for (roh, name, ziffer) in [
        (
            "Hans Muster ist Mitglied einer Gewerkschaft.",
            "Hans Muster",
            1u8,
        ),
        (
            "Marie Dupont souffre de troubles psychiques.",
            "Marie Dupont",
            2,
        ),
        (
            "Von Hans Muster liegt ein DNA-Profil vor.",
            "Hans Muster",
            3,
        ),
        (
            "Le impronte digitali di Mario Rossi sono registrate.",
            "Mario Rossi",
            4,
        ),
        (
            "Le prévenu Jean Dupont conteste les faits.",
            "Jean Dupont",
            5,
        ),
        ("Mario Rossi dipende dall'aiuto sociale.", "Mario Rossi", 6),
    ] {
        let funde = mit_name(roh, name);
        let regel = format!("art5c-{ziffer}");
        let f = funde
            .iter()
            .find(|f| f.regel == regel)
            .unwrap_or_else(|| panic!("«{roh}»: {regel} fehlt in {funde:?}"));
        assert_eq!(f.stufe, Stufe::Orange, "«{roh}»");
        assert_eq!(f.grund, Grund::BesondersSchuetzenswert { ziffer });
    }
}

#[test]
fn ohne_identifikator_mit_personenbezug_nur_hinweis() {
    let roh = "Die betroffene Person ist schwanger.";
    let text = Text::neu(roh);
    let mut treffer = art5c::erkenne(&text);
    treffer.push(von_hand(&text, "betroffene Person", Art::Personenbezug));
    let funde = bewerte(&text, treffer, &Optionen::default()).0;
    let f = funde
        .iter()
        .find(|f| f.regel == "art5c-2")
        .expect("art5c-2 gemeldet");
    assert_eq!(f.stufe, Stufe::Hinweis);
}

/// Fund der Regel `regel` mit Stufe ORANGE im Bericht über `roh`?
fn orange_durch(roh: &str, regel: &str) -> bool {
    pruefe(roh, &Optionen::default())
        .funde
        .iter()
        .any(|f| f.regel == regel && f.stufe == Stufe::Orange)
}

#[test]
fn beispiel_der_nennungsgrenze_ist_orange() {
    // RegelInfo::grenzen (NENNUNG): «Hans Muster fragt nach Krankheit im Arbeitsrecht»
    // meldet ORANGE, obwohl über Hans Muster nichts Gesundheitliches steht.
    let funde = mit_name(
        "Hans Muster fragt nach Krankheit im Arbeitsrecht",
        "Hans Muster",
    );
    let f = funde
        .iter()
        .find(|f| f.regel == "art5c-2")
        .expect("art5c-2 gemeldet");
    assert_eq!(f.stufe, Stufe::Orange);
    assert_eq!(f.stelle, "Krankheit");
}

#[test]
fn wendungen_der_wortlisten_sind_mit_ahv_orange() {
    // Wörter und Wendungen aus den Wortlisten, mit einer erfundenen AHV-Nummer als
    // Identifikator.
    for (roh, regel) in [
        (
            "Versicherte 756.1234.5678.97 ist seit drei Wochen krank.",
            "art5c-2",
        ),
        (
            "L'assurée 756.1234.5678.97 est malade depuis trois semaines.",
            "art5c-2",
        ),
        (
            "L'assicurata 756.1234.5678.97 è malata da tre settimane.",
            "art5c-2",
        ),
        ("Versicherter 756.1234.5678.97 ist suizidal.", "art5c-2"),
        (
            "Versicherter 756.1234.5678.97 hat einen Suizidversuch hinter sich.",
            "art5c-2",
        ),
        (
            "Der Patient 756.1234.5678.97 hat eine körperliche Behinderung.",
            "art5c-2",
        ),
        ("Patient 756.1234.5678.97, Hauptdiagnose: F32.1", "art5c-2"),
        (
            "Die ethnische Herkunft von 756.1234.5678.97 ist vermerkt.",
            "art5c-2",
        ),
        (
            "L'origine ethnique de 756.1234.5678.97 est indiquée.",
            "art5c-2",
        ),
        (
            "L'appartenenza etnica di 756.1234.5678.97 è indicata.",
            "art5c-2",
        ),
        (
            "L'origine etnica di 756.1234.5678.97 è indicata.",
            "art5c-2",
        ),
        (
            "L'employée 756.1234.5678.97 est enceinte de quatre mois.",
            "art5c-2",
        ),
        ("Der Versicherte 756.1234.5678.97 ist Muslim.", "art5c-1"),
        (
            "Der Versicherte 756.1234.5678.97 ist muslimischen Glaubens.",
            "art5c-1",
        ),
        ("Die Versicherte 756.1234.5678.97 ist Jüdin.", "art5c-1"),
        ("L'assicurata 756.1234.5678.97 è ebrea.", "art5c-1"),
        (
            "Personalblatt 756.1234.5678.97, Konfession: röm.-kath.",
            "art5c-1",
        ),
        (
            "L'assuré 756.1234.5678.97 est membre d'un syndicat.",
            "art5c-1",
        ),
        (
            "Les images faciales de 756.1234.5678.97 sont enregistrées.",
            "art5c-4",
        ),
        ("Der Bewerber 756.1234.5678.97 ist vorbestraft.", "art5c-5"),
        ("Der Bewerber 756.1234.5678.97 hat Vorstrafen.", "art5c-5"),
        (
            "L'assuré 756.1234.5678.97 a déposé une plainte pénale.",
            "art5c-5",
        ),
        ("L'assuré 756.1234.5678.97 a porté plainte.", "art5c-5"),
        (
            "L'assuré 756.1234.5678.97 a subi un retrait du permis de conduire.",
            "art5c-5",
        ),
        (
            "L'assicurato 756.1234.5678.97 ha presentato querela.",
            "art5c-5",
        ),
        (
            "L'assicurato 756.1234.5678.97 riceve l'assistenza sociale.",
            "art5c-6",
        ),
        ("Das Kind 756.1234.5678.97 hat einen Vormund.", "art5c-6"),
        (
            "Der Versicherte 756.1234.5678.97 braucht Massnahmen der sozialen Hilfe.",
            "art5c-6",
        ),
    ] {
        assert!(
            orange_durch(roh, regel),
            "«{roh}»: {:?}",
            pruefe(roh, &Optionen::default()).funde
        );
    }
}

#[test]
fn parallelsaetze_ergeben_dieselbe_stufe() {
    // Derselbe Sachverhalt auf Deutsch, Französisch und Italienisch: dieselbe Regel,
    // dieselbe höchste Stufe (ORANGE).
    let saetze: [(&str, [&str; 3]); 18] = [
        (
            "art5c-2",
            [
                "Die Versicherte 756.9876.5432.17 ist seit Mai krank.",
                "L'assurée 756.9876.5432.17 est malade depuis mai.",
                "L'assicurata 756.9876.5432.17 è malata da maggio.",
            ],
        ),
        (
            "art5c-2",
            [
                "Die Versicherte 756.9876.5432.17 ist schwanger.",
                "L'assurée 756.9876.5432.17 est enceinte.",
                "L'assicurata 756.9876.5432.17 è incinta.",
            ],
        ),
        (
            "art5c-2",
            [
                "Der Versicherte 756.9876.5432.17 lebt mit einer körperlichen Behinderung.",
                "L'assuré 756.9876.5432.17 vit avec une déficience corporelle.",
                "L'assicurato 756.9876.5432.17 vive con menomazioni fisiche.",
            ],
        ),
        (
            "art5c-2",
            [
                "Diagnose zu 756.9876.5432.17: F32.1",
                "Diagnostic pour 756.9876.5432.17 : F32.1",
                "Diagnosi per 756.9876.5432.17: F32.1",
            ],
        ),
        (
            "art5c-2",
            [
                "Die ethnische Herkunft von 756.9876.5432.17 ist vermerkt.",
                "L'origine ethnique de 756.9876.5432.17 est indiquée.",
                "L'appartenenza etnica di 756.9876.5432.17 è indicata.",
            ],
        ),
        (
            "art5c-1",
            [
                "Der Versicherte 756.9876.5432.17 ist Muslim.",
                "L'assuré 756.9876.5432.17 est musulman.",
                "L'assicurato 756.9876.5432.17 è musulmano.",
            ],
        ),
        (
            "art5c-1",
            [
                "Die Versicherte 756.9876.5432.17 ist Jüdin.",
                "L'assurée 756.9876.5432.17 est juive.",
                "L'assicurata 756.9876.5432.17 è ebrea.",
            ],
        ),
        (
            "art5c-1",
            [
                "Der Versicherte 756.9876.5432.17 ist gewerkschaftlich aktiv.",
                "L'assuré 756.9876.5432.17 exerce une activité syndicale.",
                "L'assicurato 756.9876.5432.17 svolge un'attività sindacale.",
            ],
        ),
        (
            "art5c-5",
            [
                "Die Versicherte 756.9876.5432.17 hat Strafantrag gestellt.",
                "L'assurée 756.9876.5432.17 a déposé une plainte pénale.",
                "L'assicurata 756.9876.5432.17 ha presentato querela.",
            ],
        ),
        (
            "art5c-5",
            [
                "Der Versicherte 756.9876.5432.17 hat Vorstrafen.",
                "L'assuré 756.9876.5432.17 a des antécédents judiciaires.",
                "L'assicurato 756.9876.5432.17 ha precedenti penali.",
            ],
        ),
        (
            "art5c-5",
            [
                "Dem Versicherten 756.9876.5432.17 droht ein Führerausweisentzug.",
                "L'assuré 756.9876.5432.17 risque un retrait du permis de conduire.",
                "L'assicurato 756.9876.5432.17 rischia la revoca della licenza di condurre.",
            ],
        ),
        (
            "art5c-6",
            [
                "Der Versicherte 756.9876.5432.17 ist auf Massnahmen der sozialen Hilfe angewiesen.",
                "L'assuré 756.9876.5432.17 dépend de l'aide sociale.",
                "L'assicurato 756.9876.5432.17 riceve l'assistenza sociale.",
            ],
        ),
        (
            "art5c-6",
            [
                "Der Mutter des Kindes 756.9876.5432.17 wurde die elterliche Sorge entzogen.",
                "Le retrait de l'autorité parentale sur l'enfant 756.9876.5432.17 a été prononcé.",
                "La privazione dell'autorità parentale sul figlio 756.9876.5432.17 è stata pronunciata.",
            ],
        ),
        (
            "art5c-1",
            [
                "Der Versicherte 756.9876.5432.17 ist Mitglied einer Gewerkschaft.",
                "L'assuré 756.9876.5432.17 est membre d'un syndicat.",
                "L'assicurato 756.9876.5432.17 è iscritto a un sindacato.",
            ],
        ),
        (
            "art5c-2",
            [
                "Der Versicherte 756.9876.5432.17 ist Diabetiker.",
                "L'assuré 756.9876.5432.17 est diabétique.",
                "L'assicurato 756.9876.5432.17 è diabetico.",
            ],
        ),
        (
            "art5c-2",
            [
                "Die Versicherte 756.9876.5432.17 bezieht eine IV-Rente.",
                "L'assurée 756.9876.5432.17 touche une rente AI.",
                "L'assicurata 756.9876.5432.17 percepisce una rendita AI.",
            ],
        ),
        (
            "art5c-2",
            [
                "Der Versicherte 756.9876.5432.17 leidet an einer Sucht.",
                "L'assuré 756.9876.5432.17 souffre d'une addiction.",
                "L'assicurato 756.9876.5432.17 soffre di una forma di dipendenza.",
            ],
        ),
        (
            "art5c-5",
            [
                "Gegen den Versicherten 756.9876.5432.17 wurde eine Strafuntersuchung eröffnet.",
                "Une enquête pénale a été ouverte contre l'assuré 756.9876.5432.17.",
                "Contro l'assicurato 756.9876.5432.17 è stata avviata un'inchiesta penale.",
            ],
        ),
    ];
    for (regel, drei) in saetze {
        let stufen: Vec<Option<Stufe>> = drei
            .iter()
            .map(|roh| pruefe(roh, &Optionen::default()).hoechste())
            .collect();
        assert!(
            stufen.iter().all(|s| *s == Some(Stufe::Orange)),
            "{regel}: {drei:?} ergeben {stufen:?}"
        );
        for roh in drei {
            assert!(orange_durch(roh, regel), "«{roh}»: {regel} fehlt");
        }
    }
}

#[test]
fn gegenbeispiele_mit_ahv_bleiben_unter_orange() {
    // Gesetzestitel, Versicherungsnamen, Vertragsrecht, Steuern und übertragener
    // Gebrauch: Der Identifikator allein gibt GELB, die Ausnahmen dürfen kein ORANGE
    // und kein ROT auslösen.
    for roh in [
        "L'assuré 756.1234.5678.97 demande si la loi fédérale sur l'assurance-maladie s'applique.",
        "L'assicurato 756.1234.5678.97 chiede se la legge federale sull'assicurazione malattie si applica.",
        "L'assicurato 756.1234.5678.97 chiede se l'assicurazione sociale contro le malattie paga.",
        "L'assuré 756.1234.5678.97 demande pourquoi sa prime d'assurance maladie augmente.",
        "L'assuré 756.1234.5678.97 a changé de caisse-maladie.",
        "L'assicurato 756.1234.5678.97 chiede perché il premio della cassa malati aumenta.",
        "Der Versicherte 756.1234.5678.97 fragt nach der Kranken- und Unfallversicherung.",
        "L'assuré 756.1234.5678.97 cite le Code de procédure pénale.",
        "Le locataire 756.1234.5678.97 conteste la dénonciation du contrat de bail.",
        "Il contribuente 756.1234.5678.97 ha inviato la denuncia dei redditi.",
        "L'assuré 756.1234.5678.97 attend dans l'enceinte du tribunal.",
        "L'assuré 756.1234.5678.97 a acheté une enceinte acoustique.",
        "La commune de l'assuré 756.1234.5678.97 adhère au syndicat de communes.",
        "Le syndicat de remaniement de l'assuré 756.1234.5678.97 a été créé en 2019.",
        "La syndication du crédit de l'assuré 756.1234.5678.97 est terminée.",
        "L'étudiant 756.1234.5678.97 rencontre son tuteur de thèse.",
        "Il cliente 756.1234.5678.97 compra un tutore per i pomodori.",
        "Le client 756.1234.5678.97 dépose une plainte au magasin.",
        "Le retrait du permis de construire de l'assuré 756.1234.5678.97 est contesté.",
        "Der Versicherte 756.1234.5678.97 sucht eine neue Stelle.",
        "Der Versicherte 756.1234.5678.97 ist konfessionslos.",
        // Versicherer und Versicherungen.
        "L'assuré 756.1234.5678.97 écrit à son assureur-maladie.",
        "Les assureurs-maladie de l'assuré 756.1234.5678.97 ont répondu.",
        "L'assicurato 756.1234.5678.97 chiede se l'assicurazione malattia paga.",
        "L'assicurato 756.1234.5678.97 scrive al suo assicuratore malattia.",
        "Gli assicuratori malattia dell'assicurato 756.1234.5678.97 hanno risposto.",
        "L'assicurato 756.1234.5678.97 ha informato l'assicuratore-malattie.",
        // «Jude» und «Cancer» als Namen.
        "Herr Jude Muster (AHV 756.1234.5678.97) bewirbt sich.",
        "Schreibe an Jude Bellingham, AHV 756.1234.5678.97.",
        "M. Paul Cancer (AVS 756.1234.5678.97) demande un rendez-vous.",
        // «sindacale» des Gemeindepräsidenten, «sindacato» als Überprüfung.
        "Il cittadino 756.1234.5678.97 impugna l'ordinanza sindacale.",
        "Il ricorrente 756.1234.5678.97 critica il sindacato di legittimità.",
        "L'azionista 756.1234.5678.97 aderisce al sindacato azionario.",
        // Wirtschaftliche Abhängigkeit.
        "Le sous-traitant 756.1234.5678.97 vit une forme de dépendance économique.",
        "Il fornitore 756.1234.5678.97 vive una forma di dipendenza economica.",
        // Beschwerde an die Aufsichtsbehörde.
        "Le débiteur 756.1234.5678.97 a porté plainte à l'autorité de surveillance.",
        "Le donateur 756.1234.5678.97 a déposé plainte auprès de l'autorité de surveillance.",
        // Erlasstitel.
        "L'étudiant 756.1234.5678.97 lit la Procédure pénale applicable aux mineurs.",
        "L'étudiant 756.1234.5678.97 cite la Procédure pénale militaire.",
        // «ai» als Präposition.
        "La rendita ai superstiti dell'assicurato 756.1234.5678.97 è calcolata.",
    ] {
        let bericht = pruefe(roh, &Optionen::default());
        assert!(
            bericht
                .funde
                .iter()
                .all(|f| f.stufe != Stufe::Orange && f.stufe != Stufe::Rot),
            "«{roh}»: {:?}",
            bericht.funde
        );
        // Die AHV-Nummer selbst wird weiterhin erkannt.
        assert_eq!(bericht.hoechste(), Some(Stufe::Gelb), "«{roh}»");
    }
}

#[test]
fn formen_und_gegenproben_sind_mit_ahv_orange() {
    // Starke Formen, Gegenproben zum Namen und zu den Ausnahmen: Diese Sätze bleiben
    // ORANGE.
    for (roh, regel) in [
        (
            "Herr Muster (AHV 756.1234.5678.97) lebt mit körperlicher Behinderung.",
            "art5c-2",
        ),
        (
            "Kinder ethnischer Herkunft wie 756.1234.5678.97 werden erfasst.",
            "art5c-2",
        ),
        (
            "Die Versicherte 756.1234.5678.97 ist auf Leistungen sozialer Hilfe angewiesen.",
            "art5c-6",
        ),
        (
            "Der Versicherte 756.1234.5678.97 leidet an psychischer Störung.",
            "art5c-2",
        ),
        (
            "Wegen sexueller Orientierung wurde die Versicherte 756.1234.5678.97 benachteiligt.",
            "art5c-2",
        ),
        (
            "Der Versicherte 756.1234.5678.97 handelte aus politischer Überzeugung.",
            "art5c-1",
        ),
        (
            "Das Kind 756.1234.5678.97 wächst mit religiöser Erziehung auf.",
            "art5c-1",
        ),
        (
            "Der Versicherte 756.1234.5678.97 wurde nach fürsorgerischer Unterbringung entlassen.",
            "art5c-6",
        ),
        (
            "Die Rechte beschuldigter Personen wie 756.1234.5678.97 sind gewahrt.",
            "art5c-5",
        ),
        (
            "Die Bearbeitung genetischer Daten von 756.1234.5678.97 ist geplant.",
            "art5c-3",
        ),
        (
            "Die Bearbeitung biometrischer Daten von 756.1234.5678.97 ist geplant.",
            "art5c-4",
        ),
        (
            "Die Versicherte 756.1234.5678.97 hat diabetische Füsse.",
            "art5c-2",
        ),
        (
            "Die Fusspflege der Diabetikerin 756.1234.5678.97 ist verordnet.",
            "art5c-2",
        ),
        (
            "Les rentes AI de l'assuré 756.1234.5678.97 sont versées.",
            "art5c-2",
        ),
        (
            "L'assuré 756.1234.5678.97 souffre d'addictions multiples.",
            "art5c-2",
        ),
        (
            "Die Strafuntersuchungen gegen 756.1234.5678.97 dauern an.",
            "art5c-5",
        ),
        ("Herr Muster (AHV 756.1234.5678.97) ist Jude.", "art5c-1"),
        ("M. Muster (AVS 756.1234.5678.97) a un cancer.", "art5c-2"),
        (
            "L'assicurato 756.1234.5678.97 è iscritto ai sindacati.",
            "art5c-1",
        ),
        (
            "L'assicurato 756.1234.5678.97 è membro del collegio sindacale.",
            "art5c-1",
        ),
        (
            "L'assuré 756.1234.5678.97 souffre d'une forme de dépendance à l'alcool.",
            "art5c-2",
        ),
        (
            "L'assuré 756.1234.5678.97 a porté plainte contre son voisin.",
            "art5c-5",
        ),
        (
            "Une procédure pénale militaire vise l'assuré 756.1234.5678.97.",
            "art5c-5",
        ),
        (
            "L'assuré 756.1234.5678.97 paie l'assurance perte de gain maladie.",
            "art5c-2",
        ),
    ] {
        assert!(
            orange_durch(roh, regel),
            "«{roh}»: {:?}",
            pruefe(roh, &Optionen::default()).funde
        );
    }
}

#[test]
fn icd_kode_mit_name_ist_orange() {
    let funde = mit_name("Diagnose von Anna Beispiel: F32.1", "Anna Beispiel");
    let kode = funde
        .iter()
        .find(|f| f.stelle == "F32.1")
        .expect("ICD-Kode gemeldet");
    assert_eq!(kode.regel, "art5c-2");
    assert_eq!(kode.stufe, Stufe::Orange);
}
