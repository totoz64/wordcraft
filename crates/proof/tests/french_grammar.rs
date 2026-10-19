//! Original synthetic acceptance cases. This is a small regression set, not a population-wide
//! precision estimate. No third-party sentences or personal documents are used.

use wordcraft_proof::{Issue, ProofLang, fr::grammar};

fn additions(text: &str, lang: ProofLang) -> Vec<Issue> {
    grammar::check_in(text, lang)
        .into_iter()
        .filter(|i| {
            i.message.starts_with("Participe passé attendu après avoir")
                || text.get(i.start..i.end).is_some_and(|s| matches!(s, "si il" | "Si il" | "si ils" | "Si ils"))
        })
        .collect()
}

#[test]
fn original_error_cases_have_exact_ranges_and_expected_corrections_in_both_regions() {
    let cases = [
        ("J’ai manger une pomme.", "manger", "mangé"),
        ("J'ai travailler ce matin.", "travailler", "travaillé"),
        ("Tu as écouter la radio.", "écouter", "écouté"),
        ("Il a regarder le film.", "regarder", "regardé"),
        ("Elle a préparer un repas.", "préparer", "préparé"),
        ("On a appeler notre ami.", "appeler", "appelé"),
        ("Nous avons chercher la clé.", "chercher", "cherché"),
        ("Vous avez étudier cette question.", "étudier", "étudié"),
        ("Ils ont gagner un prix.", "gagner", "gagné"),
        ("Elles ont trouver une solution.", "trouver", "trouvé"),
        ("  J’ai aider mon voisin.", "aider", "aidé"),
        ("J’ai\u{a0}demander une explication.", "demander", "demandé"),
        ("Si il vient, nous partirons.", "Si il", "S’il"),
        ("Si ils arrivent, préviens-moi.", "Si ils", "S’ils"),
        ("Je partirai si il vient.", "si il", "s’il"),
        ("Je resterai si ils le demandent.", "si ils", "s’ils"),
    ];
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, flagged, replacement) in cases {
            let issues = additions(text, lang);
            assert_eq!(issues.len(), 1, "{lang:?}: {text}: {issues:?}");
            let issue = &issues[0];
            assert_eq!(&text[issue.start..issue.end], flagged);
            assert_eq!(issue.suggestions, [replacement]);
            assert_eq!(issue.lang, lang);
            let fixed = format!("{}{}{}", &text[..issue.start], replacement, &text[issue.end..]);
            assert!(additions(&fixed, lang).is_empty(), "the suggested fix is stable: {fixed}");
        }
    }
}

#[test]
fn original_correct_constructions_stay_quiet() {
    let cases = [
        // Correct participles and infinitives after another verb.
        "J’ai mangé une pomme.",
        "Tu as écouté la radio.",
        "Nous avons travaillé ce matin.",
        "Je vais manger une pomme.",
        "Je peux regarder le film.",
        "Nous souhaitons travailler ensemble.",
        "Vous devez étudier ce dossier.",
        "Ils veulent parler au responsable.",
        "Elles préfèrent chanter ensemble.",
        "Il faut préparer le repas.",
        // An object before the auxiliary can change participle agreement.
        "Les pommes que j’ai mangées sont bonnes.",
        "Je les ai regardées.",
        "Il nous a montré le chemin.",
        "Il vous a aidés.",
        "Nous est parvenue une lettre.",
        "Ce qu’il a tu est important.",
        // Modal/perception constructions and nominal homographs must not be rewritten.
        "J’ai vu arriver mon voisin.",
        "J’ai entendu chanter les oiseaux.",
        "J’ai fait travailler mon équipe.",
        "J’ai laissé parler mon voisin.",
        "J’ai à manger dans mon sac.",
        "J’ai de quoi manger.",
        "J’ai rendez-vous avec Manger.",
        "Nous avons le dîner à préparer.",
        "J’ai un déjeuner à prévoir.",
        // Explicit citations and metalinguistic examples; their content is not asserted wrong.
        "«\u{a0}J’ai manger\u{a0}» est un exemple à discuter.",
        "J’ai «\u{a0}manger\u{a0}» comme infinitif.",
        "J’ai manger comme verbe dans cet exercice.",
        "J’ai aimer comme exemple dans cette liste.",
        "J’ai chanter à conjuguer pour demain.",
        "J’ai écouter à analyser.",
        "J’ai jouer et chanter à conjuguer.",
        "J’ai `manger` dans mon code.",
        "J’ai manger@example.org dans mes contacts.",
        "J’ai manger=valeur dans cette table.",
        "J’ai Manger comme nom propre.",
        // `si` elides only before il/ils.
        "Si elle vient, nous partirons.",
        "Si elles arrivent, préviens-moi.",
        "Si on part, ferme la porte.",
        "Si un voisin vient, préviens-moi.",
        "Si Isabelle vient, préviens-moi.",
        "Si, il a bien téléphoné.",
        "Si si, il vient.",
        "S’il vient, nous partirons.",
        "S'ils arrivent, nous partirons.",
        "SI IL est le nom de cette variable.",
    ];
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for text in cases {
            assert!(additions(text, lang).is_empty(), "{lang:?}: {text}: {:?}", additions(text, lang));
        }
    }
}

#[test]
fn unsupported_errors_do_not_produce_guessed_corrections() {
    // These contain mistakes, unlike the correct-text set above. Silence here measures the
    // deliberate limits of coverage; it must never be counted as true-negative precision data.
    for text in [
        "Elle est aller au marché.",
        "Je les ai regarder.",
        "La page que nous avons regarder est longue.",
        "J’ai finir mon travail.",
        "J’ai venir hier.",
        "J’ai\nmanger.",
        "J’ai\tmanger.",
        "J’ai MANGER.",
        "J’AI MANGER.",
        "Voici une phrase. J’ai manger ensuite.",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(additions(text, lang).is_empty(), "{lang:?}: {text}");
        }
    }
}

#[test]
fn hostile_text_and_long_input_keep_byte_ranges_valid() {
    for text in ["", " ", "J’", "J’ai", "J’ai ", "Tu", "Tu as", "Tu as ", "é", "😀 J’ai manger."] {
        for i in grammar::check(text) {
            assert!(text.get(i.start..i.end).is_some());
        }
    }
    let long = format!("J’ai manger {}.", "é".repeat(200_000));
    let issues = additions(&long, ProofLang::Fr);
    assert_eq!(issues.len(), 1);
    assert_eq!(long.get(issues[0].start..issues[0].end), Some("manger"));
}
