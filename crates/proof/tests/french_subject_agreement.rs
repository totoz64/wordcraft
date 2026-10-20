//! Independent original examples through the public French grammar dispatcher.
use wordcraft_proof::{ProofLang, fr::grammar};

#[test]
fn person_correction_precedes_conflicting_elision_in_both_regions() {
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("Je est responsable.", "est", "suis"),
            ("Je avons raison.", "Je avons", "J’ai"),
            ("Vous a raison.", "a", "avez"),
            ("Je ne est pas responsable.", "ne est", "ne suis"),
            ("Je n’est pas responsable.", "n’est", "ne suis"),
            ("Nous n’as pas raison.", "n’as", "n’avons"),
            ("J’est responsable.", "J’est", "Je suis"),
        ] {
            let found = grammar::check_in(text, lang);
            assert_eq!(found.len(), 1, "{lang:?}: {text}: {found:?}");
            let issue = &found[0];
            assert_eq!(&text[issue.start..issue.end], wrong);
            assert_eq!(issue.suggestions, [right]);
            assert_eq!(issue.lang, lang);
            assert_eq!(issue.message, "Accord du verbe avec le pronom sujet");
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(grammar::check_in(&fixed, lang).is_empty(), "{fixed}");
        }
    }
}

#[test]
fn unrelated_elisions_and_punctuation_are_retained() {
    let text = "Vous a envie de avoir raison!";
    let found = grammar::check(text);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().any(|i| i.message == "Accord du verbe avec le pronom sujet" && &text[i.start..i.end] == "a"));
    assert!(found.iter().any(|i| i.message.starts_with("Élision") && &text[i.start..i.end] == "de avoir"));
    assert!(found.iter().any(|i| &text[i.start..i.end] == "!"));
}

#[test]
fn whole_subject_replacements_do_not_cross_a_language_boundary() {
    let text = "Je ont raison.";
    let spans = [(0..3, ProofLang::Fr), (3..6, ProofLang::En), (6..text.len(), ProofLang::Fr)];
    let found = wordcraft_proof::check_text(text, &spans);
    assert!(!found.iter().any(|i| i.message == "Accord du verbe avec le pronom sujet"), "{found:?}");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore = "requires the pinned full lexicon in WORDCRAFT_FRENCH_LEXICON"]
fn real_person_readings_and_nominal_appositions_are_preserved() {
    use wordcraft_proof::fr::morphology;
    let path = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("set the pinned full lexicon path");
    let folder = std::path::Path::new(&path).parent().unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.to_owned()));
    assert!(matches!(morphology::data_status(), morphology::DataStatus::Ready { analyses: 274427, .. }));
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("Nous manges une pomme.", "manges", "mangeons"),
            ("Vous finissaient ce travail.", "finissaient", "finissiez"),
            ("Tu viendra demain.", "viendra", "viendras"),
            ("Nous mangèrent une pomme.", "mangèrent", "mangeâmes"),
            ("Nous eussent raison.", "eussent", "eussions"),
            ("Nous serait prêts.", "serait", "serions"),
            ("Vous soient prêts.", "soient", "soyez"),
            ("Il prendras ce train.", "prendras", "prendra"),
            ("Elle finissaient ce travail.", "finissaient", "finissait"),
            ("On viendras demain.", "viendras", "viendra"),
            ("Ils écouteras cette chanson.", "écouteras", "écouteront"),
            ("Elles choisira cette place.", "choisira", "choisiront"),
            ("Je étaient ici.", "Je étaient", "J’étais"),
            ("Je avons raison.", "Je avons", "J’ai"),
            ("Je ne est pas responsable.", "ne est", "ne suis"),
            ("Tu n’étaient pas ici.", "n’étaient", "n’étais"),
            ("Nous ne avait pas raison.", "ne avait", "n’avions"),
            ("Je n’habites pas ici.", "n’habites", "n’habite"),
            ("Je habites ici.", "Je habites", "J’habite"),
            ("Nous habites ici.", "habites", "habitons"),
            ("Je habitions ici.", "Je habitions", "J’habitais"),
            ("Vous a peur et savez pourquoi.", "a", "avez"),
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
            "Nous guides de montagne accompagnons ce groupe.",
            "Nous juges rendons cette décision.",
            "Vous élèves de ce collège savez la réponse.",
            "Nous pilotes expérimentés connaissons ce trajet.",
            "Nous guides et pilotes sommes prudents.",
            "Nous as du volant sommes prudents.",
            "Vous as du volant êtes prudents.",
            "Vous comme moi savons cela.",
            "Tu suis cette piste.",
            "Tu sommes le débiteur de payer.",
            "Nous avions notre billet.",
            "Je ne suis pas responsable.",
            "Nous ne sommes pas responsables.",
            "Vous avez raison.",
            "Nous abasourdis restons sans voix.",
            "Vous affaiblis prenez du repos.",
        ] {
            assert!(grammar::check_in(text, lang).is_empty(), "{text}: {:?}", grammar::check_in(text, lang));
        }
        // Ambiguous or unsupported errors, not correct-text precision data.
        for text in ["Nous portes ces cartons.", "Vous pilotes cet avion.", "Je habiterons ici.", "Nous n’hissez pas ce drapeau."] {
            assert!(!grammar::check_in(text, lang).iter().any(|i| i.message == "Accord du verbe avec le pronom sujet"), "{text}");
        }
    }
    assert!(morphology::load_error().is_none());
}
