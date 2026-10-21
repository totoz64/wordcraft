//! Original noun-group agreement cases through F7, Change and Undo, isolated from installed data.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_engine::Session;

const GROUP: &str = "Accord dans le groupe nominal";
const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-nominal-review-{}", std::process::id()));
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
fn nominal_agreements_review_preserves_ranges_languages_and_undo() {
    let scratch = Scratch::new();
    // Original miniature paradigms use the already documented public TSV layout.
    // Include noun/verb homography of joue so the ordinary example exercises ambiguity.
    let rows = [
        ("maison", "maison", "nom fem sg"),
        ("maisons", "maison", "nom fem pl"),
        ("enfant", "enfant", "nom epi sg"),
        ("enfants", "enfant", "nom epi pl"),
        ("petit", "petit", "nom adj mas sg"),
        ("petite", "petit", "nom adj fem sg"),
        ("petits", "petit", "nom adj mas pl"),
        ("petites", "petit", "nom adj fem pl"),
        ("joue", "joue", "nom fem sg"),
        ("joue", "jouer", "v1_itnq__a ipre spre 1sg 3sg"),
        ("jouent", "jouer", "v1_itnq__a ipre spre 3pl"),
        ("tombe", "tombe", "nom fem sg"),
        ("tombe", "tomber", "v1_i____zz ipre spre 1sg 3sg"),
        ("tombent", "tomber", "v1_i____zz ipre spre 3pl"),
        ("est", "être", "v0ei_____a ipre 3sg"),
        ("sont", "être", "v0ei_____a ipre 3pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(wordcraft_proof::fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));
    let cases = [
        ("Les petite maisons.", "petite", "petites", GROUP),
        ("Les maison.", "maison", "maisons", GROUP),
        ("Été. Les petite maisons.", "petite", "petites", GROUP),
        ("Les enfants joue.", "joue", "jouent", SUBJECT),
        ("Les petits enfants joue.", "joue", "jouent", SUBJECT),
        ("Les petites maisons tombe.", "tombe", "tombent", SUBJECT),
        ("Été. Les enfants joue.", "joue", "jouent", SUBJECT),
    ];
    let mut words = std::collections::BTreeSet::new();
    for (text, _, replacement, _) in cases {
        for source in [text, replacement] {
            for (a, b) in wordcraft_proof::words(source) {
                words.insert(source[a..b].to_lowercase().replace('’', "'"));
            }
        }
    }
    // A nonempty spelling fixture prevents dictionary availability from hiding grammar bugs.
    wordcraft_proof::fr::install_dictionary("SET UTF-8\n", &format!("{}\n{}\n", words.len(), words.into_iter().collect::<Vec<_>>().join("\n")))
        .unwrap();
    for language in ["fr-FR", "fr-CA"] {
        for (text, wrong, replacement, prefix) in cases {
            let mut ordinary = session(text, language);
            let original_runs = language_runs(&ordinary);
            let issues = run(&mut ordinary, "review.issues", json!({}));
            let found: Vec<_> = issues.as_array().unwrap().iter().filter(|issue| family(issue, prefix)).collect();
            assert_eq!(found.len(), 1, "{language}: {text}: {issues}");
            let start = text.find(wrong).unwrap();
            assert_eq!(found[0]["text"], wrong);
            assert_eq!(found[0]["start"]["off"].as_u64(), Some(start as u64));
            assert_eq!(found[0]["end"]["off"].as_u64(), Some((start + wrong.len()) as u64));
            assert!(grammar_marks(&mut ordinary) > 0);
            run(&mut ordinary, "file.options", json!({"markGrammar":false}));
            assert_eq!(grammar_marks(&mut ordinary), 0);
            // Live marks and manual review remain independent, as in the existing review contract.
            assert!(run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, prefix)));
            run(&mut ordinary, "file.options", json!({"markGrammar":true}));
            run(&mut ordinary, "caret.docStart", json!({}));
            let selected = run(&mut ordinary, "review.spelling", json!({}));
            assert!(family(&selected, prefix), "{selected}");
            assert_eq!(selected["text"], wrong);
            assert!(selected["suggestions"].as_array().unwrap().iter().any(|suggestion| suggestion == replacement));
            run(&mut ordinary, "review.applySuggestion", json!({"text":replacement}));
            let corrected = text.replacen(wrong, replacement, 1);
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), corrected);
            assert_eq!(language_runs(&ordinary), vec![(0, corrected.len(), language.to_owned())]);
            assert!(!run(&mut ordinary, "review.issues", json!({})).as_array().unwrap().iter().any(|issue| family(issue, prefix)));
            run(&mut ordinary, "edit.undo", json!({}));
            assert_eq!(ordinary.doc.plain_text(StoryRef::Body), text);
            assert_eq!(language_runs(&ordinary), original_runs);
        }

        // Validate every deciding word and intervening byte, not only the replacement.
        for (text, prefix, ranges) in [
            ("Les petite maisons.", GROUP, vec![(0, 3), (3, 4), (4, 10), (11, 18)]),
            ("Les enfants joue.", SUBJECT, vec![(0, 3), (3, 4), (4, 11), (12, 16)]),
        ] {
            for foreign in ["de-DE", "en-US", if language == "fr-FR" { "fr-CA" } else { "fr-FR" }] {
                for (start, end) in &ranges {
                    let mut mixed = session(text, language);
                    select_language(&mut mixed, *start, *end, foreign);
                    let before = language_runs(&mixed);
                    let issues = run(&mut mixed, "review.issues", json!({}));
                    assert!(!issues.as_array().unwrap().iter().any(|issue| family(issue, prefix)), "{language}/{foreign}: {text}: {issues}");
                    assert_eq!(mixed.doc.plain_text(StoryRef::Body), text);
                    assert_eq!(language_runs(&mixed), before);
                }
            }
        }

        // The unrelated preceding sentence can have a different language: later syntax is local.
        let text = "Été. Les enfants joue.";
        let mut local = session(text, language);
        select_language(&mut local, 0, "Été".len(), "de-DE");
        let before = language_runs(&local);
        run(&mut local, "caret.docStart", json!({}));
        let selected = run(&mut local, "review.spelling", json!({}));
        assert!(family(&selected, SUBJECT), "{selected}");
        run(&mut local, "review.applySuggestion", json!({"text":"jouent"}));
        let corrected = text.replacen("joue", "jouent", 1);
        assert_eq!(local.doc.plain_text(StoryRef::Body), corrected);
        assert!(language_runs(&local).iter().any(|(a, b, lang)| *a == 0 && *b == "Été".len() && lang == "de-DE"));
        run(&mut local, "edit.undo", json!({}));
        assert_eq!(local.doc.plain_text(StoryRef::Body), text);
        assert_eq!(language_runs(&local), before);

        // Disabling grammar in the proof API must suppress both new rule families.
        let text = "Les petite maisons.";
        let checked = wordcraft_proof::check_text_with(
            text,
            &[(0..text.len(), if language == "fr-FR" { wordcraft_proof::ProofLang::Fr } else { wordcraft_proof::ProofLang::FrCa })],
            &wordcraft_proof::Options::default(),
            false,
            false,
        );
        assert!(checked.iter().all(|issue| issue.kind != wordcraft_proof::IssueKind::Grammar));
    }
}
