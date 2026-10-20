//! French local replacements through F7/Change and Undo, with source language runs intact.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-local-french-review-{}", std::process::id()));
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

fn family(issue: &Value, prefix: &str) -> bool {
    issue["kind"] == "grammar" && issue["lang"] == "fr" && issue["message"].as_str().is_some_and(|message| message.starts_with(prefix))
}

fn language_runs(session: &Session) -> Vec<(usize, usize, String)> {
    let para = session.doc.para(StoryRef::Body, &Path::top(0)).unwrap();
    let mut covered = 0;
    let mut result: Vec<(usize, usize, String)> = Vec::new();
    for (range, props) in para.run_ranges() {
        assert_eq!(range.start, covered);
        assert!(para.text.is_char_boundary(range.start) && para.text.is_char_boundary(range.end));
        let language = props.lang.clone().unwrap();
        if let Some(previous) = result.last_mut().filter(|previous| previous.1 == range.start && previous.2 == language) {
            previous.1 = range.end;
        } else {
            result.push((range.start, range.end, language));
        }
        covered = range.end;
    }
    assert_eq!(covered, para.text.len());
    result
}

fn select_language(session: &mut Session, start: usize, end: usize, language: &str) {
    run(
        session,
        "select.range",
        json!({
            "anchor":{"story":"body","path":[0],"off":start},
            "focus":{"story":"body","path":[0],"off":end}
        }),
    );
    run(session, "review.language", json!({"lang":language}));
}

fn session(text: &str, language: &str) -> Session {
    let mut session = Session::new(Document::from_text(text));
    run(&mut session, "select.all", json!({}));
    run(&mut session, "review.language", json!({"lang":language,"default":true}));
    session
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
fn review_local_french_ranges_preserve_languages_apply_and_undo() {
    let _scratch = Scratch::new();
    let cases = [
        ("le ami", "le ami", "l’ami", "Élision", 0usize),
        ("test test", "test test", "test", "Mot répété", 0),
        ("\"hello\"", "\"hello\"", "«\u{a0}hello\u{a0}»", "Guillemets français", 0),
        ("Été, le élan.", "le élan", "l’élan", "Élision", 0),
        ("xyz le ami", "le ami", "l’ami", "Élision", 4),
    ];
    let mut words = std::collections::BTreeSet::new();
    for (text, _, replacement, _, _) in cases {
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
        for (text, wrong, replacement, prefix, foreign_prefix) in cases {
            let mut local = session(text, language);
            if foreign_prefix > 0 {
                select_language(&mut local, 0, foreign_prefix, "de-DE");
            }
            let original_runs = language_runs(&local);
            let issues = run(&mut local, "review.issues", json!({}));
            let matches: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| family(issue, prefix)).collect();
            assert_eq!(matches.len(), 1, "{language}: {text}: {issues}");
            let start = text.find(wrong).unwrap();
            assert_eq!(matches[0]["text"], wrong);
            assert_eq!(matches[0]["start"]["off"].as_u64(), Some(start as u64));
            assert_eq!(matches[0]["end"]["off"].as_u64(), Some((start + wrong.len()) as u64));
            assert!(grammar_marks(&mut local) > 0);
            run(&mut local, "file.options", json!({"markGrammar":false}));
            assert_eq!(grammar_marks(&mut local), 0);
            // Hiding live grammar marks deliberately keeps manual review available.
            assert!(run(&mut local, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, prefix)));
            run(&mut local, "file.options", json!({"markGrammar":true}));
            run(&mut local, "caret.docStart", json!({}));
            let selected = run(&mut local, "review.spelling", json!({}));
            assert!(family(&selected, prefix), "{selected}");
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|value| value == replacement));
            run(&mut local, "review.applySuggestion", json!({"text":replacement}));
            let corrected = text.replacen(wrong, replacement, 1);
            assert_eq!(local.doc.plain_text(StoryRef::Body), corrected);
            let expected_runs = if foreign_prefix == 0 {
                vec![(0, corrected.len(), language.to_owned())]
            } else {
                vec![(0, foreign_prefix, "de-DE".to_owned()), (foreign_prefix, corrected.len(), language.to_owned())]
            };
            assert_eq!(language_runs(&local), expected_runs);
            assert!(!run(&mut local, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, prefix)));
            run(&mut local, "edit.undo", json!({}));
            assert_eq!(local.doc.plain_text(StoryRef::Body), text);
            assert_eq!(language_runs(&local), original_runs);
        }
        for (text, prefix, ranges) in [
            ("le ami", "Élision", vec![(0, 2), (3, 6)]),
            ("test test", "Mot répété", vec![(0, 4), (5, 9)]),
            ("\"hello\"", "Guillemets français", vec![(1, 6)]),
            ("le élan", "Élision", vec![(3, "le élan".len())]),
        ] {
            for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
                for (start, end) in &ranges {
                    assert!(text.is_char_boundary(*start) && text.is_char_boundary(*end));
                    let mut mixed = session(text, language);
                    select_language(&mut mixed, *start, *end, foreign);
                    let original_runs = language_runs(&mixed);
                    let issues = run(&mut mixed, "review.issues", json!({}));
                    assert!(!issues.as_array().unwrap().iter().any(|issue| family(issue, prefix)), "{language}/{foreign}: {text}: {issues}");
                    run(&mut mixed, "caret.docStart", json!({}));
                    let next = run(&mut mixed, "review.spelling", json!({}));
                    assert!(!family(&next, prefix), "F7 consumed a foreign run: {next}");
                    assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                    assert_eq!(language_runs(&mixed), original_runs);
                }
            }
        }
    }
}
