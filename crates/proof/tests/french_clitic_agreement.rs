//! Original clitic regressions. Ambiguous/unsupported errors are not precision data.
use wordcraft_proof::{ProofLang, fr::grammar};

fn person(text: &str, lang: ProofLang) -> Vec<wordcraft_proof::Issue> {
    grammar::check_in(text, lang).into_iter().filter(|i| i.message == "Accord du verbe avec le pronom sujet").collect()
}

#[test]
fn clitic_elision_replaces_only_the_intended_prefix_and_keeps_other_diagnostics() {
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("Je m’est préparé.", "m’est", "me suis"),
            ("Il s’es préparé.", "s’es", "s’est"),
            ("Je le avons lu.", "le avons", "l’ai"),
            ("Je ne me y êtes préparé.", "ne me y êtes", "ne m’y suis"),
            ("J’y est bien.", "J’y est", "J’y suis"),
            ("Je en ont besoin.", "Je en ont", "J’en ai"),
            ("Je m'est préparé.", "m'est", "me suis"),
            ("Il s'es préparé.", "s'es", "s’est"),
        ] {
            let found = grammar::check_in(text, lang);
            assert_eq!(found.len(), 1, "{lang:?}: {text}: {found:?}");
            assert_eq!(&text[found[0].start..found[0].end], wrong);
            assert_eq!(found[0].suggestions, [right]);
            let fixed = format!("{}{right}{}", &text[..found[0].start], &text[found[0].end..]);
            assert!(grammar::check_in(&fixed, lang).is_empty(), "{fixed}");
        }
    }
    let text = "Je le avons vu avant de avoir peur!";
    let found = grammar::check(text);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().any(|i| i.message.starts_with("Élision") && &text[i.start..i.end] == "de avoir"));
    assert!(found.iter().any(|i| &text[i.start..i.end] == "!"));
}

#[test]
fn invalid_orders_unknown_gender_and_reflexive_auxiliary_choice_are_not_guessed() {
    for text in [
        "Je lui le ont donné.",
        "Je me me ont parlé.",
        "Je me le lui y ont donné.",
        "Nous se est trompés.",
        "Je me as offert un cadeau.",
        "Il s’ont trompé.",
        "Je l’est responsable.",
        "Je ne l’est pas responsable.",
        "Je les; est.",
        "Je les\tont vus.",
        "Je les\nont vus.",
        "Je LES ont vus.",
        "Je les ont comme verbe dans cet exemple.",
        "« Je les ont vus. »",
        "Je les `ont` dans cette liste.",
        "Voici une phrase. Je les ont vus.",
        "Je les xyzxyz.",
        "Je les manger.",
    ] {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(person(text, lang).is_empty(), "{lang:?}: {text}");
        }
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore = "requires the pinned full lexicon in WORDCRAFT_FRENCH_LEXICON"]
fn real_clitic_persons_keep_objects_participles_and_valid_homographs() {
    use wordcraft_proof::fr::morphology;
    let path = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("set the pinned full lexicon path");
    wordcraft_proof::set_dictionaries_folder(Some(std::path::Path::new(&path).parent().unwrap().to_owned()));
    assert!(matches!(morphology::data_status(), morphology::DataStatus::Ready { analyses: 274427, .. }));
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("Je les voit souvent.", "les voit", "les vois"),
            ("Tu me dit la vérité.", "me dit", "me dis"),
            ("Nous lui dis la vérité.", "lui dis", "lui disons"),
            ("Vous leur a parlé.", "leur a", "leur avez"),
            ("Il se demandent pourquoi.", "se demandent", "se demande"),
            ("Je ne les voit pas.", "ne les voit", "ne les vois"),
            ("Je n’en voit pas.", "n’en voit", "n’en vois"),
            ("Je n'en voit pas.", "n'en voit", "n’en vois"),
            ("J’y vont demain.", "J’y vont", "J’y vais"),
            ("J'y vont demain.", "J'y vont", "J’y vais"),
            ("Je m’en vont bientôt.", "m’en vont", "m’en vais"),
            ("Ils s’endort ici.", "s’endort", "s’endorment"),
            ("Nous vous écrivait cette lettre.", "vous écrivait", "vous écrivions"),
            ("Vous nous prenait au sérieux.", "nous prenait", "nous preniez"),
            ("Elles lui diras demain.", "lui diras", "lui diront"),
            ("On te feront confiance.", "te feront", "te fera"),
            ("Je t’es indispensable.", "t’es", "te suis"),
            ("Tu m’est indispensable.", "m’est", "m’es"),
            ("Nous nous est trompés.", "nous est", "nous sommes"),
            ("Je le avons lu.", "le avons", "l’ai"),
            ("Il ne me ont pas répondu.", "ne me ont", "ne m’a"),
            ("Je ne vous ont pas répondu.", "ne vous ont", "ne vous ai"),
            ("Il le lui ont donné.", "le lui ont", "le lui a"),
            ("Tu m’y ont conduit.", "m’y ont", "m’y as"),
            ("Nous les y voit.", "les y voit", "les y voyons"),
            ("Je les ont confiés.", "les ont", "les ai"),
            ("Nous les as offertes.", "les as", "les avons"),
            ("Nous ne les a pas prises.", "ne les a", "ne les avons"),
        ] {
            let found = grammar::check_in(text, lang);
            assert_eq!(found.len(), 1, "{lang:?}: {text}: {found:?}");
            assert_eq!(&text[found[0].start..found[0].end], wrong);
            assert!(found[0].suggestions.iter().any(|s| s == right), "{text}: {found:?}");
            for suggestion in &found[0].suggestions {
                let fixed = format!("{}{suggestion}{}", &text[..found[0].start], &text[found[0].end..]);
                assert!(grammar::check_in(&fixed, lang).is_empty(), "{fixed}");
            }
        }
        for text in [
            "Je les vois souvent.",
            "Tu me dis la vérité.",
            "Nous lui disons la vérité.",
            "Vous leur avez parlé.",
            "Il se demande pourquoi.",
            "Je ne les vois pas.",
            "Je n’en vois pas.",
            "Je m’en vais.",
            "Elle s’en est aperçue.",
            "Nous nous en sommes souvenus.",
            "Tu nous sommes de payer.",
            "Vous le leur donnez.",
            "Je ne m’y suis pas trompé.",
            "Je les leur ai confiés.",
            "Nous les avons offertes.",
            "Nous ne les avons pas prises.",
            "Il nous voit écrire.",
            "Je vais les voir.",
            "Nous voulons le lui dire.",
            "Je le fais relire.",
            "Nous les guides de montagne accompagnons ce groupe.",
            "Nous en juges impartiaux décidons.",
        ] {
            assert!(person(text, lang).is_empty(), "{text}: {:?}", person(text, lang));
        }
        // Genuine ambiguities/unsupported errors, not correct-text precision evidence.
        for text in ["Nous les portes demain.", "Je m’ai offert un cadeau.", "Ils s’a préparé.", "Nous l’hissez."] {
            assert!(person(text, lang).is_empty(), "{text}");
        }
    }
}
