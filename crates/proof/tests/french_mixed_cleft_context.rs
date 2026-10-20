//! A personal cleft must not borrow its antecedent from another proofing language.
use wordcraft_proof::{IssueKind, Options, ProofLang, check_text_with, set_dictionaries_folder};

#[test]
fn cleft_antecedent_and_verb_share_the_same_language() {
    let folder = tempfile::tempdir().unwrap();
    set_dictionaries_folder(Some(folder.path().to_path_buf()));
    let options = Options::default();
    let text = "C’est nous qui ont raison.";
    let split = text.find("ont").unwrap();
    let expected = "Accord du verbe avec l’antécédent de qui";
    let mut failures = Vec::new();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let all_french = check_text_with(text, &[(0..text.len(), lang)], &options, true, false);
        assert!(all_french.iter().any(|issue| issue.kind == IssueKind::Grammar && issue.message == expected && issue.suggestions == ["avons"]));
        for correct in ["C’est nous qui avons raison.", "C’est toi qui suis cette piste."] {
            assert!(!check_text_with(correct, &[(0..correct.len(), lang)], &options, true, false).iter().any(|issue| issue.message == expected));
        }
        let utf8 = "  C’est\u{a0}nous qui ont raison.";
        let issue =
            check_text_with(utf8, &[(0..utf8.len(), lang)], &options, true, false).into_iter().find(|issue| issue.message == expected).unwrap();
        assert_eq!(&utf8[issue.start..issue.end], "ont");
        assert_eq!(issue.start, utf8.find("ont").unwrap());
        for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            let mixed = check_text_with(text, &[(0..split, foreign), (split..text.len(), lang)], &options, true, false);
            if mixed.iter().any(|issue| issue.kind == IssueKind::Grammar && issue.message == expected) {
                failures.push(format!("{lang:?}/{foreign:?}: {mixed:?}"));
            }
            for word in ["C’est", "nous", "qui", "ont"] {
                let start = text.find(word).unwrap();
                let end = start + word.len();
                let mixed = check_text_with(text, &[(0..start, lang), (start..end, foreign), (end..text.len(), lang)], &options, true, false);
                if mixed.iter().any(|issue| issue.message == expected) {
                    failures.push(format!("foreign deciding word {word}: {lang:?}/{foreign:?}: {mixed:?}"));
                }
            }
            let disabled = check_text_with(text, &[(0..split, foreign), (split..text.len(), lang)], &options, false, false);
            assert!(!disabled.iter().any(|issue| issue.kind == IssueKind::Grammar));
        }
    }
    assert!(failures.is_empty(), "{} cross-language cleft corrections:\n{}", failures.len(), failures.join("\n"));
}
