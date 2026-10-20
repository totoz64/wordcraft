//! Original isolated morphology through F7/Change, layout marks and independent Undo.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-reflexive-review-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        wordcraft_proof::set_dictionaries_folder(None);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(session: &mut Session, command: &str, params: Value) -> Value {
    session.run(command, &params).unwrap()
}

fn grammar_marks(session: &mut Session) -> usize {
    session
        .layout()
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| match item {
            wordcraft_layout::Placed::Lines { para, .. } => para.issues.iter().filter(|issue| issue.2).count(),
            _ => 0,
        })
        .sum()
}

#[test]
fn reflexive_auxiliary_suggestions_preserve_objects_and_participles_through_review() {
    let scratch = Scratch::new();
    let rows = [
        ("ai", "avoir", "v0ait____a ipre 1sg"),
        ("as", "avoir", "v0ait____a ipre 2sg"),
        ("a", "avoir", "v0ait____a ipre 3sg"),
        ("avons", "avoir", "v0ait____a ipre 1pl"),
        ("avez", "avoir", "v0ait____a ipre 2pl"),
        ("ont", "avoir", "v0ait____a ipre 3pl"),
        ("suis", "être", "v0ei_____a ipre 1sg"),
        ("es", "être", "v0ei_____a ipre 2sg"),
        ("est", "être", "v0ei_____a ipre 3sg"),
        ("sommes", "être", "v0ei_____a ipre 1pl"),
        ("êtes", "être", "v0ei_____a ipre 2pl"),
        ("sont", "être", "v0ei_____a ipre 3pl"),
        ("trompe", "tromper", "v1_it_q__a ipre 1sg 3sg"),
        ("trompes", "tromper", "v1_it_q__a ipre 2sg"),
        ("trompons", "tromper", "v1_it_q__a ipre 1pl"),
        ("trompez", "tromper", "v1_it_q__a ipre 2pl"),
        ("trompent", "tromper", "v1_it_q__a ipre 3pl"),
        ("trompé", "tromper", "v1_it_q__a ppas mas sg"),
        ("trompée", "tromper", "v1_it_q__a ppas fem sg"),
        ("trompées", "tromper", "v1_it_q__a ppas fem pl"),
        ("parlons", "parler", "v1_it_q__a ipre 1pl"),
        ("parlé", "parler", "v1_it_q__a ppas mas sg"),
        ("écrivez", "écrire", "v3_it_q__a ipre 2pl"),
        ("écrit", "écrire", "v3_it_q__a ppas mas sg"),
        ("offre", "offrir", "v3_it_q__a ipre 3sg"),
        ("offertes", "offrir", "v3_it_q__a ppas fem pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(wordcraft_proof::fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));

    let cases = [
        ("Je m'ai trompée.", "m'ai", "me suis"),
        ("Tu t'as trompé.", "t'as", "t’es"),
        ("Il s'a trompé.", "s'a", "s’est"),
        ("Nous nous avons parlé.", "nous avons", "nous sommes"),
        ("Vous vous avez écrit.", "vous avez", "vous êtes"),
        ("Elles se ont trompées.", "se ont", "se sont"),
        ("Elle se les a offertes.", "se les a", "se les est"),
        ("Nous ne nous avons pas parlé.", "ne nous avons", "ne nous sommes"),
    ];
    // All synthetic words are known here, so any remaining mark has a grammatical cause.
    let mut known = std::collections::BTreeSet::new();
    for (text, _, replacement) in cases.iter().copied().chain([("Je m'ai trompée:", "m'ai", "me suis")]) {
        for source in [text, replacement] {
            for (a, b) in wordcraft_proof::words(source) {
                known.insert(source[a..b].to_lowercase().replace('’', "'"));
            }
        }
    }
    wordcraft_proof::fr::install_dictionary(
        "SET UTF-8\nICONV 1\nICONV ’ '\n",
        &format!("{}\n{}\n", known.len(), known.into_iter().collect::<Vec<_>>().join("\n")),
    )
    .unwrap();

    for lang in ["fr-FR", "fr-CA"] {
        for (text, wrong, right) in cases {
            let mut session = Session::new(Document::from_text(text));
            run(&mut session, "select.all", json!({}));
            run(&mut session, "review.language", json!({"lang":lang,"default":true}));
            let issues = run(&mut session, "review.issues", json!({}));
            assert_eq!(issues.as_array().unwrap().len(), 1, "{lang}: {text}: {issues}");
            assert_eq!(issues[0]["kind"], "grammar");
            assert_eq!(issues[0]["text"], wrong);
            assert_eq!(grammar_marks(&mut session), 1);
            run(&mut session, "file.options", json!({"markGrammar":false}));
            assert_eq!(grammar_marks(&mut session), 0);
            assert_eq!(run(&mut session, "review.issues", json!({})).as_array().unwrap().len(), 1);
            run(&mut session, "file.options", json!({"markGrammar":true}));
            assert_eq!(grammar_marks(&mut session), 1);
            run(&mut session, "caret.docStart", json!({}));
            let selected = run(&mut session, "review.spelling", json!({}));
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|s| s == right), "{selected}");
            run(&mut session, "review.applySuggestion", json!({"text":right}));
            assert_eq!(session.doc.plain_text(StoryRef::Body), text.replacen(wrong, right, 1));
            assert!(run(&mut session, "review.issues", json!({})).as_array().unwrap().is_empty());
            run(&mut session, "edit.undo", json!({}));
            assert_eq!(session.doc.plain_text(StoryRef::Body), text);
            assert_eq!(run(&mut session, "review.issues", json!({}))[0]["text"], wrong);
        }
        // The contraction expands before a later typographic issue: F7 must use its new
        // UTF-8 offset, and Undo must restore both independent original issues.
        let mut session = Session::new(Document::from_text("Je m'ai trompée:"));
        run(&mut session, "select.all", json!({}));
        run(&mut session, "review.language", json!({"lang":lang,"default":true}));
        assert_eq!(run(&mut session, "review.issues", json!({})).as_array().unwrap().len(), 2);
        for (wrong, right) in [("m'ai", "me suis"), (":", "\u{a0}:")] {
            run(&mut session, "caret.docStart", json!({}));
            let selected = run(&mut session, "review.spelling", json!({}));
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|s| s == right), "{selected}");
            run(&mut session, "review.applySuggestion", json!({"text":right}));
        }
        assert!(run(&mut session, "review.issues", json!({})).as_array().unwrap().is_empty());
        assert_eq!(session.doc.plain_text(StoryRef::Body), "Je me suis trompée\u{a0}:");
        run(&mut session, "edit.undo", json!({}));
        run(&mut session, "edit.undo", json!({}));
        let restored = run(&mut session, "review.issues", json!({}));
        assert_eq!(session.doc.plain_text(StoryRef::Body), "Je m'ai trompée:");
        assert_eq!(restored.as_array().unwrap().len(), 2);
        assert_eq!(restored[0]["text"], "m'ai");
        assert_eq!(restored[1]["text"], ":");
    }
}
