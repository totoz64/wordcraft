//! Explicit pinned-data gate for the same mixed-language deciding contexts.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{IssueKind, Options, ProofLang, check_text_with, fr, set_dictionaries_folder};

#[test]
#[ignore = "requires the pinned full French morphology in WORDCRAFT_DICTIONARIES"]
fn real_mixed_context_preserves_minimal_grammatical_dependencies() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit pinned data folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert_eq!(fr::morphology::installed_path().unwrap().parent(), Some(folder.as_path()));
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    let options = Options::default();
    let syntactic = |text: &str, spans: &[(std::ops::Range<usize>, ProofLang)]| {
        check_text_with(text, spans, &options, true, false)
            .into_iter()
            .filter(|issue| {
                issue.kind == IssueKind::Grammar
                    && (matches!(issue.message.as_str(), "Accord du verbe avec le pronom sujet" | "Accord du verbe avec l’antécédent de qui")
                        || issue.message.starts_with("Participe passé attendu après avoir"))
            })
            .count()
    };
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, deciding_word, rejects) in [
            ("Nous a raison.", "Nous", true),
            ("J’ai écrire.", "J’ai", true),
            ("C’est nous qui ont raison.", "nous", true),
            ("J’ai pouvoir partir.", "partir", true),
            ("J’ai rentrer les chaises.", "les", true),
            ("J’ai classer les archives.", "les", false),
            ("J’ai manger les pommes.", "les", false),
        ] {
            assert_eq!(syntactic(text, &[(0..text.len(), lang)]), 1, "full French: {text}");
            let start = text.find(deciding_word).unwrap();
            let end = start + deciding_word.len();
            for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                let spans = [(0..start, lang), (start..end, foreign), (end..text.len(), lang)];
                assert_eq!(syntactic(text, &spans), usize::from(!rejects), "minimal real-data context {text}, {lang:?}/{foreign:?}");
            }
        }
    }
}
