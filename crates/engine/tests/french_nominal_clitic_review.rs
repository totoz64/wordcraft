//! Original single-clitic common-noun subject cases through F7, Change and Undo, isolated from installed data.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-nominal-clitic-review-{}", std::process::id()));
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
fn nominal_clitic_review_keeps_objects_regions_marks_spelling_priority_and_undo() {
    let scratch = Scratch::new();
    let rows = [
        ("enfants", "enfant", "nom epi pl"),
        ("voisin", "voisin", "nom mas sg"),
        ("petits", "petit", "adj mas pl"),
        ("parle", "parler", "v1 ipre 1sg 3sg"),
        ("parlent", "parler", "v1 ipre 3pl"),
        ("parlait", "parler", "v1 iimp 1sg 3sg"),
        ("parlaient", "parler", "v1 iimp 3pl"),
        ("répond", "répondre", "v3 ipre 3sg"),
        ("répondent", "répondre", "v3 ipre 3pl"),
        ("va", "aller", "v3 ipre 3sg"),
        ("vont", "aller", "v3 ipre 3pl"),
        ("ont", "avoir", "v0 ipre 3pl"),
        ("a", "avoir", "v0 ipre 3sg"),
        ("a", "a", "nom mas inv"),
        ("route", "route", "nom fem sg"),
        ("route", "router", "v1 ipre 1sg 3sg"),
        ("routent", "router", "v1 ipre 3pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(wordcraft_proof::fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));
    let cases = [
        ("Les enfants lui parle.", "lui parle", "lui parlent"),
        ("Le voisin leur parlent.", "leur parlent", "leur parle"),
        ("Les petits enfants lui parlait.", "lui parlait", "lui parlaient"),
        ("Les enfants leur répond.", "leur répond", "leur répondent"),
        ("Les enfants y va.", "y va", "y vont"),
        ("Le voisin en ont besoin.", "en ont", "en a"),
        ("Les enfants ne lui parle pas.", "ne lui parle", "ne lui parlent"),
        ("Les enfants ne leur répond plus.", "ne leur répond", "ne leur répondent"),
        ("Les enfants n’y va jamais.", "n’y va", "n’y vont"),
        ("Le voisin n'en ont guère besoin.", "n'en ont", "n’en a"),
        ("Les enfants ne lui parlait point.", "ne lui parlait", "ne lui parlaient"),
        ("Été. Les enfants lui parle.", "lui parle", "lui parlent"),
        ("Les\u{a0}enfants\u{202f}ne\u{a0}lui\u{202f}parle pas.", "ne\u{a0}lui\u{202f}parle", "ne lui parlent"),
    ];
    let mut words = std::collections::BTreeSet::new();
    for (text, _, replacement) in cases {
        for text in [text, replacement] {
            for (a, b) in wordcraft_proof::words(text) {
                words.insert(text[a..b].to_lowercase().replace('’', "'"));
            }
        }
    }
    words.remove("parle"); // A valid syntactic correction, not spelling, must be selected by F7.
    wordcraft_proof::fr::install_dictionary("SET UTF-8\n", &format!("{}\n{}\n", words.len(), words.into_iter().collect::<Vec<_>>().join("\n")))
        .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (text, wrong, replacement) in cases {
            let mut ordinary = session(text, language);
            let before = language_runs(&ordinary);
            let issues = run(&mut ordinary, "review.issues", json!({}));
            let found: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| family(issue, SUBJECT)).collect();
            let start = text.find(wrong).unwrap();
            assert_eq!(found.len(), 1, "{language}: {text}: {issues}");
            assert_eq!(found[0]["text"], wrong);
            assert_eq!(found[0]["start"]["off"].as_u64(), Some(start as u64));
            assert_eq!(found[0]["end"]["off"].as_u64(), Some((start + wrong.len()) as u64));
            assert!(grammar_marks(&mut ordinary) > 0);
            run(&mut ordinary, "file.options", json!({"markGrammar":false}));
            assert_eq!(grammar_marks(&mut ordinary), 0);
            assert!(run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)));
            run(&mut ordinary, "file.options", json!({"markGrammar":true}));
            run(&mut ordinary, "caret.docStart", json!({}));
            let selected = run(&mut ordinary, "review.spelling", json!({}));
            assert!(family(&selected, SUBJECT), "{text}: {selected}");
            assert_eq!(selected["text"], wrong);
            run(&mut ordinary, "review.applySuggestion", json!({"text":replacement}));
            let fixed = text.replacen(wrong, replacement, 1);
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), fixed);
            assert_eq!(language_runs(&ordinary), vec![(0, fixed.len(), language.to_owned())]);
            assert!(!run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)));
            run(&mut ordinary, "edit.undo", json!({}));
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), text);
            assert_eq!(language_runs(&ordinary), before);
        }
        for text in ["Les enfants lui parle.", "Les enfants n’y va pas."] {
            let mut segments = Vec::new();
            let mut cursor = 0;
            for (a, b) in wordcraft_proof::words(text) {
                if cursor < a {
                    segments.push(cursor..a);
                }
                segments.push(a..b);
                cursor = b;
            }
            if let Some(a) = text.find("n’y") {
                segments.extend([a..a + 1, a + 1..a + 4, a + 4..a + 5]);
            }
            for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
                for segment in &segments {
                    let mut mixed = session(text, language);
                    select_language(&mut mixed, segment.start, segment.end, foreign);
                    let before = language_runs(&mixed);
                    let issues = run(&mut mixed, "review.issues", json!({}));
                    assert!(!issues.as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)), "{text}: {segment:?}: {issues}");
                    assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                    assert_eq!(language_runs(&mixed), before);
                }
            }
        }
        // Rejected syntax cannot hide independent spelling inside a French replacement.
        let text = "Les enfants lui parle.";
        let start = text.find("parle").unwrap();
        let mut mixed = session(text, language);
        select_language(&mut mixed, 0, 3, "en-US");
        let issues = run(&mut mixed, "review.issues", json!({}));
        assert!(!issues.as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)));
        assert!(
            issues.as_array().unwrap().iter().any(|issue| issue["kind"] == "spelling" && issue["start"]["off"].as_u64() == Some(start as u64)),
            "{issues}"
        );
        for text in ["Les enfants en route.", "Les enfants lui en parle.", "Les enfants leur parlent."] {
            let mut s = session(text, language);
            assert!(!run(&mut s, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, SUBJECT)), "{text}");
        }
    }
}
