#![allow(clippy::unwrap_used)]
use french_school_foundations::{analyze, analyze_sentence};

#[test]
fn independent_subject_relations_and_abstentions() {
    let cases: serde_json::Value = serde_json::from_str(include_str!("../data/syntax-cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let actual = analyze_sentence(text).unwrap();
        // Structural analysis must not mutate or prune any lexical reading.
        assert_eq!(serde_json::to_value(&actual.analyse_lexicale).unwrap(), serde_json::to_value(analyze(text).unwrap()).unwrap());
        assert!(actual.analyse_lexicale.corrections.is_empty());
        let expected = &case["expectation"];
        if expected["state"] == "abstention_relation_principale" {
            assert!(actual.analyse_syntaxique.hypotheses.is_empty(), "{}: {text}", case["id"]);
            continue;
        }
        let subject = &expected["subject"];
        let verb = &expected["verb"];
        assert!(
            actual.analyse_syntaxique.hypotheses.iter().any(|hypothesis| {
                hypothesis.liens.iter().filter(|l| l.fonction == "sujet").any(|link| {
                    let Some(group) = hypothesis.groupes.get(link.source_groupe) else {
                        return false;
                    };
                    let Some(head) = actual.analyse_lexicale.tokens.get(group.noyau_token) else {
                        return false;
                    };
                    let Some(predicate) = actual.analyse_lexicale.tokens.get(link.cible_token) else {
                        return false;
                    };
                    group.plage_utf8
                        == [subject["span"]["start_utf8"].as_u64().unwrap() as usize, subject["span"]["end_utf8"].as_u64().unwrap() as usize]
                        && head.texte == subject["head"]["text"].as_str().unwrap()
                        && head.plage_utf8
                            == [subject["head"]["start_utf8"].as_u64().unwrap() as usize, subject["head"]["end_utf8"].as_u64().unwrap() as usize]
                        && predicate.texte == verb["span"]["text"].as_str().unwrap()
                        && predicate.plage_utf8
                            == [verb["span"]["start_utf8"].as_u64().unwrap() as usize, verb["span"]["end_utf8"].as_u64().unwrap() as usize]
                        && predicate.lectures.iter().any(|r| r.nature == "verbe" && r.lemme == verb["lemma"].as_str().unwrap())
                })
            }),
            "{}: missing subject relation for {text}",
            case["id"]
        );
        if let Some(complements) = expected.get("noun_complements").and_then(|v| v.as_array()) {
            for complement in complements {
                assert!(actual.analyse_syntaxique.hypotheses.iter().any(|h| h.liens.iter().any(|l| {
                    l.fonction == "complement_du_nom"
                        && h.groupes.get(l.source_groupe).is_some_and(|g| {
                            g.plage_utf8
                                == [
                                    complement["span"]["start_utf8"].as_u64().unwrap() as usize,
                                    complement["span"]["end_utf8"].as_u64().unwrap() as usize,
                                ]
                                && actual
                                    .analyse_lexicale
                                    .tokens
                                    .get(l.cible_token)
                                    .is_some_and(|t| t.texte == complement["attaches_to"].as_str().unwrap())
                        })
                })));
            }
        }
    }
}

#[test]
fn syntax_is_explicitly_opt_in_for_cli() {
    let binary = env!("CARGO_BIN_EXE_french-school-foundations");
    let lexical = std::process::Command::new(binary).args(["--text", "Le chien aboie."]).output().unwrap();
    assert!(lexical.status.success());
    let lexical: serde_json::Value = serde_json::from_slice(&lexical.stdout).unwrap();
    assert!(lexical.get("analyse_syntaxique").is_none());
    let structural = std::process::Command::new(binary).args(["--syntax", "--text", "Le chien aboie."]).output().unwrap();
    assert!(structural.status.success());
    let structural: serde_json::Value = serde_json::from_slice(&structural.stdout).unwrap();
    assert_eq!(lexical, structural["analyse_lexicale"]);
    assert!(!structural["analyse_syntaxique"]["hypotheses"].as_array().unwrap().is_empty());
}

#[test]
fn partial_tail_never_decides_object_attribute_or_intention() {
    for text in ["Tu portes les livres.", "Je suis le voisin.", "Ils portent les livres.", "Elles portent les livres."] {
        let analysis = analyze_sentence(text).unwrap();
        assert_eq!(analysis.analyse_syntaxique.statut, "hypotheses_partielles_avec_reste_non_analyse");
        assert!(analysis.analyse_syntaxique.hypotheses.iter().all(|h| h.reste_non_analyse_tokens.is_some()));
        assert!(analysis.analyse_syntaxique.hypotheses.iter().flat_map(|h| &h.liens).all(|l| l.fonction == "sujet"));
        assert!(analysis.analyse_lexicale.corrections.is_empty());
    }
    let analysis = analyze_sentence("Je suis le voisin.").unwrap();
    let lexical = &analysis.analyse_lexicale;
    let predicate = lexical.tokens.get(1).unwrap();
    assert!(predicate.lectures.iter().any(|r| r.lemme == "être"));
    assert!(predicate.lectures.iter().any(|r| r.lemme == "suivre"));
    for text in ["Nous parle le voisin.", "Vous parle le voisin.", "Tu portes les livres joue."] {
        assert!(analyze_sentence(text).unwrap().analyse_syntaxique.hypotheses.is_empty());
    }
}

#[test]
fn imperative_stays_lexical_but_is_not_attached_to_an_explicit_subject() {
    let analysis = analyze_sentence("Je suis.").unwrap();
    assert!(analysis.analyse_lexicale.tokens.get(1).unwrap().lectures.iter().any(|r| r.traits.get("mode").is_some_and(|m| m == "imperatif")));
    for h in &analysis.analyse_syntaxique.hypotheses {
        for g in h.groupes.iter().filter(|g| g.categorie == "groupe_verbal_minimal") {
            let token = analysis.analyse_lexicale.tokens.get(g.noyau_token).unwrap();
            assert!(g.lectures_noyau.iter().all(|&i| token.lectures.get(i).unwrap().traits.get("mode").is_none_or(|m| m != "imperatif")));
        }
    }
}
