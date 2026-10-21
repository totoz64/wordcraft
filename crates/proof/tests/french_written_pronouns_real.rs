//! Explicit real-data gate for written-pronoun mentions, not general autonym detection.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{Options, ProofLang, check_text_with, fr, set_dictionaries_folder};

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
                for tail in ["un pronom personnel.", "écrit en gras sur cette affiche.", "un mot français.", "le mot retenu.", "ce mot français."]
                {
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
        for (text, target, suggestion) in [
            ("Je est un mot de ami.", "de ami", "d’ami"),
            ("Je est un mot français. Il refuse de avoir peur.", "de avoir", "d’avoir"),
            ("Je est un pronom. Il refuse de avoir peur.", "de avoir", "d’avoir"),
            ("Je est écrit en gras, le ami arrive.", "le ami", "l’ami"),
            ("Nous a été écrit en gras. Le ami arrive.", "Le ami", "L’ami"),
        ] {
            let found = fr::grammar::check_in(text, lang);
            let start = text.find(target).unwrap();
            assert!(
                found
                    .iter()
                    .any(|issue| issue.start == start && issue.end == start + target.len() && issue.suggestions.iter().any(|fix| fix == suggestion)),
                "{lang:?}: {text}: {found:?}"
            );
            assert!(!found.iter().any(|issue| issue.start == 0 && issue.message.starts_with("Élision")), "mention prefix stays protected: {found:?}");
        }
        let other_region = if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr };
        for foreign in [ProofLang::Other, ProofLang::En, other_region] {
            for (text, marker, candidate_allowed) in [("Je est un pronom.", "pronom", true), ("Je est un mot.", "mot", false)] {
                let start = text.find(marker).unwrap();
                let end = start + marker.len();
                let spans = [(0..start, lang), (start..end, foreign), (end..text.len(), lang)];
                let found = check_text_with(text, &spans, &Options::default(), true, false);
                if candidate_allowed {
                    assert!(
                        found.iter().any(|issue| issue.lang == lang
                            && issue.message == "Accord du verbe avec le pronom sujet"
                            && issue.suggestions.iter().any(|fix| fix == "suis")),
                        "{lang:?}/{foreign:?}: {text}: {found:?}"
                    );
                } else {
                    // Assert the inherited fallback, not its linguistic accuracy:
                    // candidate() still conservatively excludes mot/mots here.
                    assert!(
                        found.iter().any(|issue| issue.lang == lang && issue.start == 0 && issue.message.starts_with("Élision")),
                        "no new veto from foreign mot: {found:?}"
                    );
                }
            }
        }
        assert!(fr::grammar::check_in("Nous est absent de ce mot.", lang).is_empty(), "do not broaden the existing mot/mots candidate rule");
    }
}
