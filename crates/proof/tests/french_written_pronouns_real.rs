//! Explicit real-data gate for written-pronoun mentions, not general autonym detection.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{ProofLang, fr, set_dictionaries_folder};

#[test]
#[ignore = "requires the pinned real French dictionary folder in WORDCRAFT_DICTIONARIES"]
fn real_written_pronouns_preserve_mentions_and_existing_rules() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit real dictionary folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert_eq!(fr::morphology::installed_path().unwrap().parent(), Some(folder.as_path()));
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for subject in ["Je", "Tu", "Nous", "Vous", "Ils", "Elles"] {
            for copula in [
                "est",
                "était",
                "fut",
                "sera",
                "serait",
                "soit",
                "fût",
                "a été",
                "avait été",
                "eut été",
                "aura été",
                "aurait été",
                "ait été",
                "eût été",
            ] {
                for tail in ["un pronom personnel.", "écrit en gras sur cette affiche."] {
                    let text = format!("{subject} {copula} {tail}");
                    assert!(fr::grammar::check_in(&text, lang).is_empty(), "{lang:?}: {text}");
                }
            }
        }
        for (text, wanted) in [
            ("Je est responsable.", "suis"),
            ("Nous a un rendez-vous.", "avons"),
            ("Je était responsable.", "J’étais"),
            ("Nous écrivait en gras.", "écrivions"),
            ("Je est responsable, ce pronom vient ensuite.", "suis"),
            ("Je est responsable de ce pronom.", "suis"),
        ] {
            let found = fr::grammar::check_in(text, lang);
            assert!(found.iter().any(|issue| issue.suggestions.iter().any(|suggestion| suggestion == wanted)), "{text}: {found:?}");
        }
    }
}
