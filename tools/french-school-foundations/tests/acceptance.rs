#![allow(clippy::unwrap_used)]
use french_school_foundations::{AnalysisError, MAX_BYTES, analyze};

fn reading(text: &str, form: &str, lemma: &str, nature: &str, traits: &[(&str, &str)]) -> bool {
    analyze(text).unwrap().tokens.iter().filter(|t| t.texte == form).any(|t|
        t.lectures.iter().any(|r| r.lemme == lemma && r.nature == nature && traits.iter().all(|(k,v)| r.traits.get(k).is_some_and(|actual| actual == v))))
}

#[test]
fn school_word_forms_are_separate_from_sentence_roles() {
    let sentence = "Les petits chiens jouent.";
    assert!(reading(sentence, "chiens", "chien", "nom", &[("genre", "masculin"), ("nombre", "pluriel")]));
    assert!(reading(sentence, "petits", "petit", "adjectif", &[("genre", "masculin"), ("nombre", "pluriel")]));
    assert!(reading(sentence, "jouent", "jouer", "verbe", &[("mode", "indicatif"), ("temps", "present"), ("personne", "3"), ("nombre", "pluriel")]));
    assert!(reading(sentence, "jouent", "jouer", "verbe", &[("mode", "subjonctif")]));
    let result = serde_json::to_value(analyze(sentence).unwrap()).unwrap();
    assert!(result.get("groupes").is_none());
    assert!(result.get("fonctions").is_none());
    assert_eq!(result["corrections"], serde_json::json!([]));
}

#[test]
fn homographs_remain_ambiguous_even_in_suggestive_context() {
    for sentence in ["La porte.", "Elle porte un sac."] {
        assert!(reading(sentence, "porte", "porte", "nom", &[("genre", "feminin")]));
        assert!(reading(sentence, "porte", "porter", "verbe", &[("mode", "indicatif")]));
        assert!(reading(sentence, "porte", "porter", "verbe", &[("mode", "imperatif")]));
    }
    assert!(reading("Je suis.", "suis", "être", "verbe", &[("personne", "1")]));
    assert!(reading("Je suis.", "suis", "suivre", "verbe", &[("personne", "1")]));
    assert!(reading("Tu suis.", "suis", "suivre", "verbe", &[("personne", "2"), ("mode", "indicatif")]));
    assert!(reading("Suis !", "Suis", "suivre", "verbe", &[("mode", "imperatif")]));
}

#[test]
fn gender_and_number_are_not_guessed_from_endings() {
    assert!(reading("chevaux", "chevaux", "cheval", "nom", &[("nombre", "pluriel")]));
    assert!(reading("heureux", "heureux", "heureux", "adjectif", &[("nombre", "singulier")]));
    assert!(reading("heureux", "heureux", "heureux", "adjectif", &[("nombre", "pluriel")]));
    for nature in ["determinant", "pronom"] { assert!(reading("les", "les", "le", nature, &[("nombre", "pluriel")])); }
    for item in analyze("les").unwrap().tokens.iter().flat_map(|t| &t.lectures) { assert!(!item.traits.contains_key("genre")); }
    assert!(reading("enfants", "enfants", "enfant", "nom", &[("genre", "masculin")]));
    assert!(reading("enfants", "enfants", "enfant", "nom", &[("genre", "feminin")]));
}

#[test]
fn elision_contractions_and_word_boundaries_preserve_the_source() {
    for sentence in ["L’école", "l'e\u{301}cole", "J’ai deux pommes."] {
        let result = analyze(sentence).unwrap();
        assert_eq!(result.texte, sentence);
        for token in result.tokens {
            assert_eq!(sentence.get(token.plage_utf8[0]..token.plage_utf8[1]), Some(token.texte.as_str()));
        }
    }
    assert!(reading("L’école", "L’", "le", "determinant", &[("elision", "oui")]));
    assert!(reading("des", "des", "le", "determinant", &[("type", "article_contracte"), ("preposition", "de")]));
    assert!(reading("des", "des", "un", "determinant", &[("type", "article_indefini")]));
    let segments: Vec<_> = analyze("porte-monnaie").unwrap().tokens.into_iter().map(|t| t.texte).collect();
    assert_eq!(segments, ["porte", "-", "monnaie"]);
}

#[test]
fn normalized_lookup_never_rewrites_original_offsets() {
    for sentence in ["😀 L’e\u{301}cole : chiens !", "\u{301}école", "ÉCOLE\r\nchiens", "a\u{301}\u{327} ?"] {
        let result = analyze(sentence).unwrap();
        let utf16: Vec<_> = sentence.encode_utf16().collect();
        let mut end8 = 0;
        let mut end16 = 0;
        for t in result.tokens {
            assert!(t.plage_utf8[0] >= end8 && t.plage_utf16[0] >= end16);
            assert_eq!(sentence.get(t.plage_utf8[0]..t.plage_utf8[1]), Some(t.texte.as_str()));
            assert_eq!(String::from_utf16(&utf16[t.plage_utf16[0]..t.plage_utf16[1]]).unwrap(), t.texte);
            end8 = t.plage_utf8[1]; end16 = t.plage_utf16[1];
        }
    }
    assert!(reading("e\u{301}cole", "e\u{301}cole", "école", "nom", &[]));
    assert!(reading("ÉCOLE", "ÉCOLE", "école", "nom", &[]));
}

#[test]
fn unknown_is_not_a_spelling_judgment_and_bad_agreement_is_not_repaired() {
    for text in ["plimou", "chantons", "DDemain", "😊"] {
        let result = analyze(text).unwrap();
        assert!(result.corrections.is_empty());
        assert!(result.tokens.iter().all(|t| t.lectures.is_empty()));
    }
    assert_eq!(analyze("plimou").unwrap().tokens[0].statut, "inconnu_du_lexique");
    assert!(reading("Tu joue.", "joue", "jouer", "verbe", &[("personne", "3")]));
    assert!(analyze("Tu joue.").unwrap().corrections.is_empty());
}

#[test]
fn all_nine_school_classes_have_known_examples() {
    for (text, nature) in [("chien", "nom"), ("le", "determinant"), ("petit", "adjectif"), ("je", "pronom"), ("joue", "verbe"), ("vite", "adverbe"), ("avec", "preposition"), ("et", "conjonction"), ("hélas", "interjection")] {
        assert!(analyze(text).unwrap().tokens.iter().flat_map(|t| &t.lectures).any(|r| r.nature == nature));
    }
}

#[test]
fn bounds_and_odd_inputs_never_require_a_correction() {
    assert_eq!(analyze(&"a".repeat(MAX_BYTES + 1)).unwrap_err(), AnalysisError::InputTooLong);
    assert_eq!(analyze(&"!".repeat(8193)).unwrap_err(), AnalysisError::TooManyTokens);
    for text in ["", " \n\t", "\0", "\u{200d}", "🇫🇷", "𐐀 école", "🧑‍🏫", "אב\u{5b0}", "123.5"] {
        assert!(analyze(text).unwrap().corrections.is_empty());
    }
}

#[test]
fn apostrophes_do_not_split_known_whole_words_or_unknown_names() {
    assert!(reading("aujourd’hui", "aujourd’hui", "aujourd’hui", "adverbe", &[]));
    for name in ["O’Neill", "d’Artagnan"] {
        let result = analyze(name).unwrap();
        // d’ remains an explicitly supported prefix; unknown names are never corrected.
        assert!(result.corrections.is_empty());
    }
    assert_eq!(analyze("O’Neill").unwrap().tokens.len(), 1);
    let tokens = analyze("chien’").unwrap().tokens;
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].texte, "chien");
}

#[test]
fn independent_school_reference_cases() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../data/independent-cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let result = analyze(text).unwrap();
        let segments = case["segments"].as_array().unwrap();
        assert_eq!(result.texte, text);
        assert!(result.corrections.is_empty());
        assert_eq!(result.tokens.len(), segments.len(), "{}", case["id"]);
        for (token, expected) in result.tokens.iter().zip(segments) {
            assert_eq!(token.texte, expected["surface"].as_str().unwrap());
            assert_eq!(serde_json::to_value(token.plage_utf8).unwrap(), expected["span_utf8"]);
            assert_eq!(serde_json::to_value(token.plage_utf16).unwrap(), expected["span_utf16"]);
            assert!(token.lectures.iter().all(|r| !r.provenance.is_empty()));
        }
        for focus in case["focus"].as_array().unwrap() {
            let token = result.tokens.iter().find(|t| t.texte == focus["token"].as_str().unwrap()).unwrap();
            if focus.get("lexical_status").is_some() {
                assert!(token.lectures.is_empty());
                assert_eq!(token.statut, "inconnu_du_lexique");
            }
            for expected in focus["required_readings"].as_array().unwrap() {
                if expected["nature"] == "contraction" {
                    // Explicit representation adapter: contracted definite article carries
                    // the preposition and the article's lemma/features on the source token.
                    let components = expected["components"].as_array().unwrap();
                    let prep = components[0]["lemma"].as_str().unwrap();
                    let article = components[1]["lemma"].as_str().unwrap();
                    assert!(token.lectures.iter().any(|r|
                        r.nature == "determinant" && r.lemme == article &&
                        r.traits.get("type").is_some_and(|s| s == "article_contracte") &&
                        r.traits.get("preposition").is_some_and(|s| s == prep) &&
                        r.traits.get("genre").is_some_and(|s| s == "masculin") &&
                        r.traits.get("nombre").is_some_and(|s| s == "singulier")));
                    continue;
                }
                assert!(token.lectures.iter().any(|r| {
                    r.lemme == expected["lemme"].as_str().unwrap() &&
                    r.nature == expected["nature"].as_str().unwrap() &&
                    expected["traits"].as_object().unwrap().iter().all(|(key, value)| {
                        if value.is_null() { !r.traits.contains_key(key.as_str()) }
                        else { r.traits.get(key.as_str()).is_some_and(|s| s == value.as_str().unwrap()) }
                    })
                }), "missing reading in {} for {}: {}", case["id"], token.texte, expected);
            }
        }
    }
}

#[test]
fn quoted_words_keep_their_lexical_identity() {
    for input in ["'chiens'", "’chiens’", "«chiens»"] {
        assert!(reading(input, "chiens", "chien", "nom", &[("nombre", "pluriel")]));
        assert_eq!(analyze(input).unwrap().tokens.len(), 3);
    }
}
