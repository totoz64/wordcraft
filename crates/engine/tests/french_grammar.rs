//! French grammar through the same commands used by F7 and Change, with isolated dictionary
//! state so the synthetic fixture cannot replace another test's installed dictionary.

use serde_json::{Value, json};
use wordcraft_doc::Document;
use wordcraft_engine::Session;

fn run(session: &mut Session, command: &str, params: Value) -> Value {
    session.run(command, &params).unwrap()
}

fn drawn_grammar_marks(session: &mut Session) -> usize {
    session
        .layout()
        .pages
        .iter()
        .flat_map(|p| &p.items)
        .map(|item| match item {
            wordcraft_layout::Placed::Lines { para, .. } => para.issues.iter().filter(|i| i.2).count(),
            _ => 0,
        })
        .sum()
}

#[test]
fn french_grammar_suggestions_apply_and_undo_in_both_regions() {
    wordcraft_proof::fr::install_dictionary(
        "SET UTF-8\nICONV 1\nICONV ’ '\n",
        "25\nj'ai\nmanger\nmangé\nsi\nil\ns'il\nvient\ntu\nas\nc'est\nmoi\nqui\nest\nsuis\nj'avais\nécrire\nécrit\nje\navais\nn'ai\nn'avais\npas\ndéjà\nencore\nbien\n",
    )
    .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (original, flagged, suggestion) in [
            ("J’ai manger.", "manger", "mangé"),
            ("Si il vient.", "Si il", "S’il"),
            ("C’est moi qui est.", "est", "suis"),
            ("J’avais écrire.", "écrire", "écrit"),
            ("Je n’ai pas écrire.", "écrire", "écrit"),
            ("J’ai déjà écrire.", "écrire", "écrit"),
            ("Je n’avais pas encore bien écrire.", "écrire", "écrit"),
        ] {
            let mut session = Session::new(Document::from_text(original));
            run(&mut session, "select.all", json!({}));
            run(&mut session, "review.language", json!({"lang": language, "default": true}));
            let issues = run(&mut session, "review.issues", json!({}));
            assert_eq!(issues.as_array().unwrap().len(), 1, "{language}: {issues}");
            assert_eq!(issues[0]["kind"], "grammar");
            assert_eq!(issues[0]["text"], flagged);
            assert_eq!(drawn_grammar_marks(&mut session), 1);
            run(&mut session, "file.options", json!({"markGrammar": false}));
            assert_eq!(drawn_grammar_marks(&mut session), 0, "only as-you-type marks are disabled");
            assert_eq!(run(&mut session, "review.issues", json!({})).as_array().unwrap().len(), 1, "manual Review remains available");
            run(&mut session, "file.options", json!({"markGrammar": true}));
            assert_eq!(drawn_grammar_marks(&mut session), 1);
            run(&mut session, "caret.docStart", json!({}));
            let selected = run(&mut session, "review.spelling", json!({}));
            assert_eq!(selected["text"], flagged);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|s| s == suggestion));
            run(&mut session, "review.applySuggestion", json!({"text": suggestion}));
            assert!(run(&mut session, "review.issues", json!({})).as_array().unwrap().is_empty());
            run(&mut session, "edit.undo", json!({}));
            let restored = run(&mut session, "review.issues", json!({}));
            assert_eq!(restored.as_array().unwrap().len(), 1);
            assert_eq!(restored[0]["text"], flagged);
        }
        // Elision changes the UTF-8 byte length before the new participle issue. F7 must
        // recompute its range, then two independent undos must restore both original issues.
        let mut session = Session::new(Document::from_text("Je avais déjà écrire."));
        run(&mut session, "select.all", json!({}));
        run(&mut session, "review.language", json!({"lang":language,"default":true}));
        assert_eq!(run(&mut session, "review.issues", json!({})).as_array().unwrap().len(), 2);
        for (wrong, right) in [("Je avais", "J’avais"), ("écrire", "écrit")] {
            run(&mut session, "caret.docStart", json!({}));
            let issue = run(&mut session, "review.spelling", json!({}));
            assert_eq!(issue["text"], wrong);
            assert!(issue["suggestions"].as_array().unwrap().iter().any(|s| s == right));
            run(&mut session, "review.applySuggestion", json!({"text":right}));
        }
        assert!(run(&mut session, "review.issues", json!({})).as_array().unwrap().is_empty());
        run(&mut session, "edit.undo", json!({}));
        run(&mut session, "edit.undo", json!({}));
        let restored = run(&mut session, "review.issues", json!({}));
        assert_eq!(restored.as_array().unwrap().len(), 2);
        assert_eq!(restored[0]["text"], "Je avais");
        assert_eq!(restored[1]["text"], "écrire");
    }
}
