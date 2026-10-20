//! Original independent examples; unsupported errors are not correct-text precision data.
use wordcraft_proof::{Issue, ProofLang, fr::grammar};

fn issues(text: &str, lang: ProofLang) -> Vec<Issue> {
    grammar::check_in(text, lang).into_iter().filter(|i| i.message.starts_with("Participe passé attendu après avoir")).collect()
}

#[test]
fn negative_and_adverb_contexts_have_exact_stable_corrections() {
    for (text, wrong, right) in [
        ("Je n’ai pas écrire cette lettre.", "écrire", "écrit"),
        ("Je n'avais jamais prendre ce train.", "prendre", "pris"),
        ("Tu n’as plus finir ce travail.", "finir", "fini"),
        ("Il n’avait guère comprendre cette question.", "comprendre", "compris"),
        ("Elle n’aura point ouvrir la porte.", "ouvrir", "ouvert"),
        ("On n’aurait pas choisir cette place.", "choisir", "choisi"),
        ("Nous n’avions jamais lire ce livre.", "lire", "lu"),
        ("Vous n’auriez pas voir ce film.", "voir", "vu"),
        ("Ils n’eurent pas recevoir de réponse.", "recevoir", "reçu"),
        ("Elles n’avaient plus entendre ce bruit.", "entendre", "entendu"),
        ("Je n’ai vraiment pas encore écrire cette lettre.", "écrire", "écrit"),
        ("Nous n’aurions pas encore bien finir ce travail.", "finir", "fini"),
        ("Tu n’avais presque jamais comprendre cette question.", "comprendre", "compris"),
        ("J’ai déjà écrire cette lettre.", "écrire", "écrit"),
        ("J’avais bien comprendre cette question.", "comprendre", "compris"),
        ("Tu as souvent prendre ce train.", "prendre", "pris"),
        ("Elle aura bientôt finir ce travail.", "finir", "fini"),
        ("Nous aurions probablement aimer ce lieu.", "aimer", "aimé"),
        ("Vous aviez toujours beaucoup lire.", "lire", "lu"),
        ("Ils ont finalement ouvrir la porte.", "ouvrir", "ouvert"),
        ("Elles auront rarement aussi bien comprendre.", "comprendre", "compris"),
        ("J’aurais seulement vouloir partir.", "vouloir", "voulu"),
        ("J’ai déjà pouvoir partir.", "pouvoir", "pu"),
        ("Je n’ai jamais devoir partir.", "devoir", "dû"),
        ("— Je n’avais pas écrire cette lettre.", "écrire", "écrit"),
        ("J’ai\u{a0}déjà\u{202f}écrire cette lettre.", "écrire", "écrit"),
        ("Je n’ai pas du tout écrire cette lettre.", "écrire", "écrit"),
        ("Je n’ai plus jamais écrire cette lettre.", "écrire", "écrit"),
        ("Tu n’avais jamais plus comprendre cette question.", "comprendre", "compris"),
        ("Nous n’avons pas plus lire que vous.", "lire", "lu"),
        ("Il n’avait guère plus comprendre cette question.", "comprendre", "compris"),
        ("J’ai assez lire pour aujourd’hui.", "lire", "lu"),
        ("J’ai mal comprendre cette question.", "comprendre", "compris"),
        ("Tu avais mieux comprendre cette question.", "comprendre", "compris"),
        ("Nous avons peu lire cette semaine.", "lire", "lu"),
        ("Vous avez trop boire ce soir.", "boire", "bu"),
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            let found = issues(text, lang);
            assert_eq!(found.len(), 1, "{lang:?}: {text}: {found:?}");
            let issue = &found[0];
            assert_eq!(&text[issue.start..issue.end], wrong);
            assert_eq!(issue.suggestions, [right]);
            assert_eq!(issue.lang, lang);
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(issues(&fixed, lang).is_empty(), "{fixed}");
        }
    }
}

#[test]
fn correct_negatives_infinitive_complements_and_idioms_stay_quiet() {
    for text in [
        "Je n’ai pas écrit cette lettre.",
        "Tu n’avais jamais pris ce train.",
        "Nous n’aurions pas encore fini ce travail.",
        "J’ai déjà écrit cette lettre.",
        "J’avais bien compris cette question.",
        "Elles auront rarement aussi bien compris.",
        "Je n’ai pas pu partir.",
        "Je n’ai jamais dû partir.",
        "J’ai déjà fait relire cette page.",
        "Je n’ai pas fait relire cette page.",
        "J’ai toujours entendu chanter.",
        "Je n’ai jamais entendu chanter.",
        "J’ai déjà laissé tomber cette idée.",
        "Je n’ai pas à écrire cette lettre.",
        "J’ai seulement à écrire cette lettre.",
        "J’ai déjà de quoi manger.",
        "Je n’ai plus qu’à écrire cette lettre.",
        "Je n’ai qu’à écrire cette lettre.",
        "J’ai beau écrire cette lettre, elle reste ici.",
        "J’ai déjà beau écrire cette lettre, elle reste ici.",
        "Je les avais déjà écrites.",
        "Je ne les avais pas écrites.",
        "Les lettres que je n’ai pas écrites sont ici.",
        "Je n’ai pas envie de partir.",
        "Nous n’avons jamais peur de venir.",
        "J’ai encore cours demain.",
        "J’ai déjà faim.",
        "J’ai bien à manger.",
        "Je n’ai pas du tout écrit cette lettre.",
        "Je n’ai plus jamais écrit cette lettre.",
        "Je n’ai pas du pain pour manger.",
        "Je n’ai pas du tout fait relire cette page.",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(issues(text, lang).is_empty(), "{lang:?}: {text}");
        }
    }
}

#[test]
fn ambiguous_mentions_and_unsupported_negatives_are_not_guessed() {
    for text in [
        "J’ai encore pouvoir sur cette mesure.",
        "Je n’ai pas pouvoir de décider.",
        "Nous n’avions jamais devoir de prudence.",
        "Je n’ai écrire cette lettre.",  // Bare literary ne: outside this increment.
        "J’ai pas écrire cette lettre.", // Omitted ne: outside this increment.
        "J’ai jamais écrire cette lettre.",
        "Je n’ai plus pas écrire cette lettre.",
        "Je n’ai pas encore vraiment bien écrire cette lettre.", // Four intervening words.
        "J’ai déjà vraiment presque bien écrire cette lettre.",
        "Je n’as pas écrire cette lettre.",
        "Nous n’avais jamais écrire cette lettre.",
        "Je ne ai pas écrire cette lettre.",
        "Je n’ai pas aller au marché.",
        "J’ai déjà venir hier.",
        "Je n’ai pas, écrire cette lettre.",
        "J’ai déjà; écrire cette lettre.",
        "Je n’ai pas\técrire cette lettre.",
        "J’ai déjà\nécrire cette lettre.",
        "Je n’ai pas ÉCRIRE cette lettre.",
        "Je N’AI pas écrire cette lettre.",
        "J’ai déjà « écrire » dans cette liste.",
        "Je n’ai pas `écrire` dans mon programme.",
        "J’ai déjà écrire comme infinitif dans cet exercice.",
        "Je n’ai pas écrire à conjuguer pour demain.",
        "Voici une phrase. Je n’ai pas écrire ensuite.",
        "Je n’ai pas xyzxyz.",
        "Je n’ai pas l’écrire.",
        "Je n’ai pas du, tout écrire.",
        "Je n’ai pas du tout déjà écrire.",
        "Je n’ai pas du tout « écrire ».",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(issues(text, lang).is_empty(), "{lang:?}: {text}");
        }
    }
}
