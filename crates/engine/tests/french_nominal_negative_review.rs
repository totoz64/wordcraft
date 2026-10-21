//! Original negative common-noun subject cases through F7, Change and Undo, isolated from installed data.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-nominal-negative-review-{}", std::process::id()));
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

fn family(issue: &Value, prefix: &str) -> bool {
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

fn select_language(session: &mut Session, start: usize, end: usize, language: &str) {
    run(session, "select.range", json!({"anchor":{"story":"body","path":[0],"off":start},"focus":{"story":"body","path":[0],"off":end}}));
    run(session, "review.language", json!({"lang":language}));
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
fn negative_nominal_subject_review_preserves_elision_context_languages_and_undo() {
    let scratch = Scratch::new();
    let rows = [
        ("enfant", "enfant", "nom epi sg"),
        ("enfants", "enfant", "nom epi pl"),
        ("petit", "petit", "nom adj mas sg"),
        ("petits", "petit", "nom adj mas pl"),
        ("joue", "joue", "nom fem sg"),
        ("joue", "jouer", "v1_itnq__a ipre spre 1sg 3sg"),
        ("jouent", "jouer", "v1_itnq__a ipre spre 3pl"),
        ("jouait", "jouer", "v1_itnq__a iimp 1sg 3sg"),
        ("jouaient", "jouer", "v1_itnq__a iimp 3pl"),
        ("est", "être", "v0ei_____a ipre 3sg"),
        ("est", "est", "nom mas sg"),
        ("sont", "être", "v0ei_____a ipre 3pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(wordcraft_proof::fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));
    let cases = [
        ("Les enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Les enfants ne joue plus dehors.", "ne joue", "ne jouent"),
        ("Les enfants ne jouait jamais.", "ne jouait", "ne jouaient"),
        ("Les enfants ne joue guère.", "ne joue", "ne jouent"),
        ("Les enfants ne joue point.", "ne joue", "ne jouent"),
        ("Les petits enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Le petit enfant ne jouent pas.", "ne jouent", "ne joue"),
        ("Les enfants n’est pas ici.", "n’est", "ne sont"),
        ("Les enfants ne est pas ici.", "ne est", "ne sont"),
        ("Le petit enfant n’sont pas ici.", "n’sont", "n’est"),
        ("Les enfants n'est pas ici.", "n'est", "ne sont"),
        ("Le petit enfant ne sont pas ici.", "ne sont", "n’est"),
        ("Été. Les enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Les\u{a0}enfants ne joue pas.", "ne joue", "ne jouent"),
    ];
    let mut words = std::collections::BTreeSet::new();
    for (text, _, replacement) in cases {
        for source in [text, replacement] {
            for (a, b) in wordcraft_proof::words(source) {
                words.insert(source[a..b].to_lowercase().replace('’', "'"));
            }
        }
    }
    // Deliberate spelling omissions prove priority only after syntax acceptance.
    words.remove("est");
    words.remove("n'sont");
    wordcraft_proof::fr::install_dictionary("SET UTF-8\n", &format!("{}\n{}\n", words.len(), words.into_iter().collect::<Vec<_>>().join("\n")))
        .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (text, wrong, replacement) in cases {
            let mut ordinary = session(text, language);
            let original_runs = language_runs(&ordinary);
            let issues = run(&mut ordinary, "review.issues", json!({}));
            let found: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| family(issue, SUBJECT)).collect();
            assert_eq!(found.len(), 1, "{language}: {text}: {issues}");
            let start = text.find(wrong).unwrap();
            assert_eq!(found[0]["text"], wrong);
            assert_eq!(found[0]["start"]["off"].as_u64(), Some(start as u64));
            assert_eq!(found[0]["end"]["off"].as_u64(), Some((start + wrong.len()) as u64));
            let lang = if language == "fr-FR" { wordcraft_proof::ProofLang::Fr } else { wordcraft_proof::ProofLang::FrCa };
            let direct = wordcraft_proof::fr::grammar::check_in(text, lang);
            assert_eq!(direct.iter().filter(|issue| issue.message == SUBJECT).count(), 1, "direct grammar: {text}: {direct:?}");
            assert!(!direct.iter().any(|issue| issue.message.starts_with("Élision") && issue.start < start + wrong.len() && start < issue.end));
            if text.contains("ne est") || text.contains("n’sont") {
                let off = wordcraft_proof::check_text_with(text, &[(0..text.len(), lang)], &wordcraft_proof::Options::default(), false, false);
                assert!(
                    off.iter()
                        .any(|issue| issue.kind == wordcraft_proof::IssueKind::Spelling && issue.start >= start && issue.end <= start + wrong.len()),
                    "fixture spelling: {text}: {off:?}"
                );
                assert!(
                    !issues.as_array().unwrap().iter().any(|issue| issue["kind"] == "spelling"
                        && issue["start"]["off"].as_u64().is_some_and(|a| a >= start as u64)
                        && issue["end"]["off"].as_u64().is_some_and(|b| b <= (start + wrong.len()) as u64)),
                    "accepted replacement keeps priority: {text}: {issues}"
                );
            }
            assert!(grammar_marks(&mut ordinary) > 0);
            run(&mut ordinary, "file.options", json!({"markGrammar":false}));
            assert_eq!(grammar_marks(&mut ordinary), 0);
            assert!(run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)));
            run(&mut ordinary, "file.options", json!({"markGrammar":true}));
            run(&mut ordinary, "caret.docStart", json!({}));
            let selected = run(&mut ordinary, "review.spelling", json!({}));
            assert!(family(&selected, SUBJECT), "{selected}");
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|suggestion| suggestion == replacement));
            run(&mut ordinary, "review.applySuggestion", json!({"text":replacement}));
            let corrected = text.replacen(wrong, replacement, 1);
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), corrected);
            assert_eq!(language_runs(&ordinary), vec![(0, corrected.len(), language.to_owned())]);
            assert!(!run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)));
            run(&mut ordinary, "edit.undo", json!({}));
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), text);
            assert_eq!(language_runs(&ordinary), original_runs);
        }
        // Each deciding word and horizontal gap must be in the same French region.
        // Split the contraction itself too: n, apostrophe, and finite verb are all anchors.
        for text in ["Les enfants ne joue pas.", "Les enfants n’est pas ici."] {
            let mut segments = Vec::new();
            let mut cursor = 0;
            for (start, end) in wordcraft_proof::words(text) {
                if cursor < start {
                    segments.push(cursor..start);
                }
                segments.push(start..end);
                cursor = end;
            }
            if let Some(start) = text.find("n’est") {
                segments.extend([start..start + 1, start + 1..start + 4, start + 4..start + 7]);
            }
            for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
                for segment in &segments {
                    let mut mixed = session(text, language);
                    select_language(&mut mixed, segment.start, segment.end, foreign);
                    let before = language_runs(&mixed);
                    let issues = run(&mut mixed, "review.issues", json!({}));
                    assert!(
                        !issues.as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)),
                        "{language}/{foreign}: {text}: {segment:?}: {issues}"
                    );
                    assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                    assert_eq!(language_runs(&mixed), before);
                }
            }
        }
        // Rejected nominal syntax must retain the independent same-region elision
        // and spelling inside its former replacement, including a foreign marker.
        let text = "Les enfants ne est pas ici.";
        let wrong_start = text.find("ne est").unwrap();
        let est_start = text.find("est").unwrap();
        let marker_start = text.find("pas").unwrap();
        for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
            for (start, end) in [(0, 3), (marker_start, marker_start + 3)] {
                let mut mixed = session(text, language);
                select_language(&mut mixed, start, end, foreign);
                let issues = run(&mut mixed, "review.issues", json!({}));
                let found = issues.as_array().unwrap();
                assert!(!found.iter().any(|issue| family(issue, SUBJECT)), "rejected nominal: {issues}");
                assert!(
                    found.iter().any(|issue| issue["message"].as_str().is_some_and(|message| message.starts_with("Élision"))
                        && issue["start"]["off"].as_u64() == Some(wrong_start as u64)
                        && issue["suggestions"].as_array().unwrap().iter().any(|suggestion| suggestion == "n’est")),
                    "independent elision: {issues}"
                );
                assert!(
                    found.iter().any(|issue| issue["kind"] == "spelling" && issue["start"]["off"].as_u64() == Some(est_start as u64)),
                    "rejected syntax retains spelling: {issues}"
                );
            }
        }
        // A preceding independent sentence does not supply a deciding token.
        let text = "Été. Les enfants n’est pas ici.";
        let mut local = session(text, language);
        select_language(&mut local, 0, "Été".len(), "de-DE");
        let before = language_runs(&local);
        run(&mut local, "caret.docStart", json!({}));
        let selected = run(&mut local, "review.spelling", json!({}));
        assert!(family(&selected, SUBJECT), "{selected}");
        run(&mut local, "review.applySuggestion", json!({"text":"ne sont"}));
        assert_eq!(local.doc.plain_text(StoryRef::Body), text.replace("n’est", "ne sont"));
        assert!(language_runs(&local).iter().any(|(a, b, lang)| *a == 0 && *b == "Été".len() && lang == "de-DE"));
        run(&mut local, "edit.undo", json!({}));
        assert_eq!(local.doc.plain_text(StoryRef::Body), text);
        assert_eq!(language_runs(&local), before);
    }
}
