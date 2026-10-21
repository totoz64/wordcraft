//! Explicit written-word mentions protect only their initial predicate and its own language context.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-explicit-autonym-review-{}", std::process::id()));
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

fn french_family(issue: &Value, prefix: &str) -> bool {
    issue["kind"] == "grammar" && issue["lang"] == "fr" && issue["message"].as_str().is_some_and(|message| message.starts_with(prefix))
}

fn language_runs(session: &Session) -> Vec<(usize, usize, String)> {
    let para = session.doc.para(StoryRef::Body, &Path::top(0)).unwrap();
    let mut result: Vec<(usize, usize, String)> = Vec::new();
    let mut covered = 0;
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

fn session(text: &str, language: &str) -> Session {
    let mut session = Session::new(Document::from_text(text));
    run(&mut session, "select.all", json!({}));
    run(&mut session, "review.language", json!({"lang":language,"default":true}));
    session
}

fn set_word_language(session: &mut Session, text: &str, word: &str, language: &str) {
    let start = text.find(word).unwrap();
    let end = start + word.len();
    assert!(text.is_char_boundary(start) && text.is_char_boundary(end));
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

#[test]
fn review_explicit_word_mentions_preserve_local_corrections_and_undo() {
    let _scratch = Scratch::new();
    let positives = [
        ("Je est un mot français, le ami arrive.", "le ami", "l’ami", "Élision"),
        ("Je était le mot de ami.", "de ami", "d’ami", "Élision"),
        ("Nous est ce mot de ami.", "de ami", "d’ami", "Élision"),
        ("Je est écrit en gras, le ami arrive.", "le ami", "l’ami", "Élision"),
        ("Je est responsable.", "est", "suis", "Accord du verbe avec le pronom sujet"),
        ("Je est responsable de ce pronom.", "est", "suis", "Accord du verbe avec le pronom sujet"),
    ];
    let mut words = std::collections::BTreeSet::new();
    for text in positives.iter().flat_map(|(text, _, replacement, _)| [*text, *replacement]).chain(["Je est un mot.", "Je est un pronom."]) {
        for (start, end) in wordcraft_proof::words(text) {
            words.insert(text[start..end].to_lowercase().replace('’', "'"));
        }
    }
    wordcraft_proof::fr::install_dictionary(
        "SET UTF-8\nICONV 1\nICONV ’ '\n",
        &format!("{}\n{}\n", words.len(), words.into_iter().collect::<Vec<_>>().join("\n")),
    )
    .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (text, wrong, right, family) in positives {
            let mut ordinary = session(text, language);
            let original_runs = language_runs(&ordinary);
            let start = text.find(wrong).unwrap();
            let issues = run(&mut ordinary, "review.issues", json!({}));
            let found: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| french_family(issue, family)).collect();
            assert_eq!(found.len(), 1, "{language}: {text}: {issues}");
            assert_eq!(found[0]["text"], wrong);
            assert_eq!(found[0]["start"]["off"].as_u64(), Some(start as u64));
            assert_eq!(found[0]["end"]["off"].as_u64(), Some((start + wrong.len()) as u64));
            // In a mention, neither J’est/J’était nor a subject rewrite may escape alongside the local correction.
            if family == "Élision" {
                assert!(!issues.as_array().unwrap().iter().any(|issue| french_family(issue, "Accord du verbe avec le pronom sujet")));
            }
            run(&mut ordinary, "caret.docStart", json!({}));
            let selected = run(&mut ordinary, "review.spelling", json!({}));
            assert!(french_family(&selected, family), "{selected}");
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|suggestion| suggestion == right));
            run(&mut ordinary, "review.applySuggestion", json!({"text":right}));
            let corrected = text.replacen(wrong, right, 1);
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), corrected);
            assert_eq!(language_runs(&ordinary), vec![(0, corrected.len(), language.to_owned())]);
            run(&mut ordinary, "edit.undo", json!({}));
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), text);
            assert_eq!(language_runs(&ordinary), original_runs);
        }

        // A foreign adjective after the required marker must not widen the veto's deciding context.
        let text = "Je est un mot français, le ami arrive.";
        let mut local = session(text, language);
        set_word_language(&mut local, text, "français", "de-DE");
        let original_runs = language_runs(&local);
        let issues = run(&mut local, "review.issues", json!({}));
        let local_elisions: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| french_family(issue, "Élision")).collect();
        assert_eq!(local_elisions.len(), 1, "{issues}");
        assert_eq!(local_elisions[0]["text"], "le ami");
        run(&mut local, "caret.docStart", json!({}));
        assert_eq!(run(&mut local, "review.spelling", json!({}))["text"], "le ami");
        run(&mut local, "review.applySuggestion", json!({"text":"l’ami"}));
        assert_eq!(local.doc.plain_text(StoryRef::Body), text.replacen("le ami", "l’ami", 1));
        let runs_after = language_runs(&local);
        let foreign_start = text.find("français").unwrap();
        assert!(runs_after.iter().any(|(start, end, lang)| *start == foreign_start && *end == foreign_start + "français".len() && lang == "de-DE"));
        run(&mut local, "edit.undo", json!({}));
        assert_eq!(local.doc.plain_text(StoryRef::Body), text);
        assert_eq!(language_runs(&local), original_runs);

        for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
            for (text, marker, family, wrong, suggestion) in [
                ("Je est un mot.", "mot", "Élision", "Je est", "J’est"),
                ("Je est un pronom.", "pronom", "Accord du verbe avec le pronom sujet", "est", "suis"),
            ] {
                let mut mixed = session(text, language);
                set_word_language(&mut mixed, text, marker, foreign);
                let original_runs = language_runs(&mixed);
                let issues = run(&mut mixed, "review.issues", json!({}));
                // For mot this checks preservation of the pre-existing fallback, not linguistic endorsement of J’est.
                // The separate broad mot/mots agreement veto is deliberately outside this change's scope.
                assert!(
                    issues.as_array().unwrap().iter().any(|issue| french_family(issue, family)
                        && issue["text"] == wrong
                        && issue["suggestions"].as_array().unwrap().iter().any(|value| value == suggestion)),
                    "{language}/{foreign}: {text}: {issues}"
                );
                run(&mut mixed, "caret.docStart", json!({}));
                let selected = run(&mut mixed, "review.spelling", json!({}));
                assert!(french_family(&selected, family), "{selected}");
                assert_eq!(selected["text"], wrong);
                assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                assert_eq!(language_runs(&mixed), original_runs);
            }
        }
    }
}
