//! Independent original examples. Quiet unsupported/ambiguous cases are not precision data.
use wordcraft_proof::{ProofLang, fr::grammar};

fn issues(text: &str, lang: ProofLang) -> Vec<wordcraft_proof::Issue> {
    grammar::check_in(text, lang).into_iter().filter(|i| i.message.starts_with("Participe passé attendu après avoir")).collect()
}

#[test]
fn irregular_forms_and_five_auxiliary_tenses_have_exact_stable_corrections() {
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("J’avais écrire une lettre.", "écrire", "écrit"),
            ("J'aurais prendre ce train.", "prendre", "pris"),
            ("Tu aurais faire ce choix.", "faire", "fait"),
            ("Il aura finir son travail.", "finir", "fini"),
            ("Elle avait comprendre cette question.", "comprendre", "compris"),
            ("Nous aurons lire ce livre.", "lire", "lu"),
            ("Vous auriez voir ce film.", "voir", "vu"),
            ("Ils eurent recevoir une réponse.", "recevoir", "reçu"),
            ("Elles avaient ouvrir la porte.", "ouvrir", "ouvert"),
            ("On aura choisir une place.", "choisir", "choisi"),
            ("J’ai fuir les lieux.", "fuir", "fui"),
            ("Nous eûmes offrir ce cadeau.", "offrir", "offert"),
            ("Vous eûtes dire la vérité.", "dire", "dit"),
            ("J’eus croire ce récit.", "croire", "cru"),
            ("Ils auront vivre cette aventure.", "vivre", "vécu"),
            ("J’avais être surpris.", "être", "été"),
            ("J’aurais avoir une question.", "avoir", "eu"),
            ("J’aurais pouvoir partir.", "pouvoir", "pu"),
            ("Nous avions devoir finir.", "devoir", "dû"),
            ("Tu avais boire un verre.", "boire", "bu"),
            ("— J’avais écrire cette lettre.", "écrire", "écrit"),
            ("–\u{a0}Tu avais écrire cette lettre.", "écrire", "écrit"),
            ("- Nous aurions lire ces pages.", "lire", "lu"),
        ] {
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
fn correct_infinitive_contexts_and_participles_stay_quiet() {
    for text in [
        "Je vais écrire une lettre.",
        "Tu peux prendre ce train.",
        "Elle souhaite ouvrir cette porte.",
        "Nous voulons finir ce travail.",
        "Ils doivent choisir une place.",
        "Vous venez de lire ce livre.",
        "J’avais écrit une lettre.",
        "J’aurais pris ce train.",
        "Tu aurais fait ce choix.",
        "J’ai entendu chanter.",
        "J’ai fait relire cette page.",
        "J’ai laissé tomber cette idée.",
        "J’ai dû partir.",
        "J’ai pu partir.",
        "Les lettres que j’avais écrites sont ici.",
        "Je les avais écrites.",
        "Elle est allée au marché.",
        "Nous sommes revenus hier.",
        "Ils étaient partis avant moi.",
        "J’ai faim et soif.",
        "J’avais peur de venir.",
        "Nous avons cours demain.",
        "— J’avais écrit cette lettre.",
        "– Tu avais écrit cette lettre.",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(issues(text, lang).is_empty(), "{lang:?}: {text}");
        }
    }
}

#[test]
fn ambiguous_mentions_and_unsupported_errors_are_not_guessed() {
    // Not a count of correct French sentences: these intentionally mix ambiguities and errors.
    for text in [
        "J’ai pouvoir de décider.",
        "J’ai pouvoir.",
        "J’aurais pouvoir sur cette mesure.",
        "J’avais pouvoir, partir me plaisait.",
        "Nous avions devoir envers nos proches.",
        "Nous avions devoir de prudence.",
        "J’aurais devoir d’exemplarité.",
        "J’avais écrire à conjuguer pour demain.",
        "J’avais écrire comme infinitif dans cet exercice.",
        "J’aurais « prendre » dans cette liste.",
        "J’aurais `prendre` dans mon programme.",
        "Je les avais écrire.",
        "J’ai aller au marché.",
        "J’avais venir hier.",
        "On avait falloir partir.",
        "Elle avait falloir partir.",
        "J’avais ÉCRIRE.",
        "J’AVAIS ÉCRIRE.",
        "J’avais\nécrire.",
        "J’avais\técrire.",
        "Voici une phrase. J’avais écrire ensuite.",
        "J’avais xyzxyz.",
        "— J’avais écrire comme mot à analyser.",
        "— J’avais « écrire » dans cette liste.",
        "—J’avais écrire.",
        "—— J’avais écrire.",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(issues(text, lang).is_empty(), "{lang:?}: {text}: {:?}", issues(text, lang));
        }
    }
}
