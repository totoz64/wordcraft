//! Regression tests for French grammar across language runs.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-mixed-context-review-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        wordcraft_proof::set_dictionaries_folder(Some(path.clone()));
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

fn french_syntax(issue: &Value) -> bool {
    issue["kind"] == "grammar"
        && issue["lang"] == "fr"
        && issue["message"].as_str().is_some_and(|message| {
            matches!(message, "Accord du verbe avec le pronom sujet" | "Accord du verbe avec l’antécédent de qui")
                || message.starts_with("Participe passé attendu après avoir")
        })
}

fn check_language_runs(session: &Session, text: &str, language: &str, foreign: Option<(usize, usize, &str)>) {
    let para = session.doc.para(StoryRef::Body, &Path::top(0)).unwrap();
    let mut covered = 0;
    for (range, props) in para.run_ranges() {
        assert_eq!(range.start, covered);
        let expected = if let Some((start, end, foreign)) = foreign {
            if range.start < end && start < range.end {
                assert!(start <= range.start && range.end <= end, "language run straddles selected word: {range:?}");
                foreign
            } else {
                language
            }
        } else {
            language
        };
        assert_eq!(props.lang.as_deref(), Some(expected));
        assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
        covered = range.end;
    }
    assert_eq!(covered, text.len());
}

#[test]
fn review_requires_each_deciding_word_to_share_the_french_region() {
    let _scratch = Scratch::new();
    let cases: [(&str, &str, &str, &[&str]); 4] = [
        ("\u{a0}Nous a raison.", "a", "avons", &["Nous", "a"]),
        ("—\u{202f}J’ai déjà écrire.", "écrire", "écrit", &["J’ai", "déjà", "écrire"]),
        ("C’est nous qui ont raison.", "ont", "avons", &["C’est", "nous", "qui", "ont"]),
        ("J’ai pouvoir partir.", "pouvoir", "pu", &["J’ai", "pouvoir", "partir"]),
    ];
    let mut words = std::collections::BTreeSet::new();
    for (text, _, replacement, _) in cases {
        for source in [text, replacement] {
            for (start, end) in wordcraft_proof::words(source) {
                words.insert(source[start..end].to_lowercase().replace('’', "'"));
            }
        }
    }
    wordcraft_proof::fr::install_dictionary(
        "SET UTF-8\nICONV 1\nICONV ’ '\n",
        &format!("{}\n{}\n", words.len(), words.into_iter().collect::<Vec<_>>().join("\n")),
    )
    .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (text, wrong, right, deciding_words) in cases {
            let mut session = Session::new(Document::from_text(text));
            run(&mut session, "select.all", json!({}));
            run(&mut session, "review.language", json!({"lang":language,"default":true}));
            check_language_runs(&session, text, language, None);
            let issues = run(&mut session, "review.issues", json!({}));
            let found: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| french_syntax(issue)).collect();
            assert_eq!(found.len(), 1, "{language}: {text}: {issues}");
            assert_eq!(found[0]["text"], wrong);
            assert_eq!(found[0]["start"]["off"].as_u64(), Some(text.find(wrong).unwrap() as u64));
            run(&mut session, "caret.docStart", json!({}));
            let issue = run(&mut session, "review.spelling", json!({}));
            assert!(french_syntax(&issue), "{issue}");
            assert_eq!(issue["text"], wrong);
            run(&mut session, "review.applySuggestion", json!({"text":right}));
            assert_eq!(session.doc.plain_text(StoryRef::Body), text.replacen(wrong, right, 1));
            assert!(!run(&mut session, "review.issues", json!({})).as_array().unwrap().iter().any(french_syntax));
            run(&mut session, "edit.undo", json!({}));
            assert_eq!(session.doc.plain_text(StoryRef::Body), text);
            for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
                for word in deciding_words {
                    let mut mixed = Session::new(Document::from_text(text));
                    run(&mut mixed, "select.all", json!({}));
                    run(&mut mixed, "review.language", json!({"lang":language,"default":true}));
                    run(&mut mixed, "select.text", json!({"text":word}));
                    run(&mut mixed, "review.language", json!({"lang":foreign}));
                    let start = text.find(word).unwrap();
                    check_language_runs(&mixed, text, language, Some((start, start + word.len(), foreign)));
                    let issues = run(&mut mixed, "review.issues", json!({}));
                    assert!(!issues.as_array().unwrap().iter().any(french_syntax), "{language}/{foreign}, {word}: {issues}");
                    run(&mut mixed, "caret.docStart", json!({}));
                    let next = run(&mut mixed, "review.spelling", json!({}));
                    assert!(!french_syntax(&next), "F7 reused foreign syntax: {next}");
                    assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                }
            }
        }
    }
}
