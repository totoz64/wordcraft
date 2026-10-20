//! Explicit real-dictionary gate. The normal synthetic suite stays offline; CI installs the
//! pinned Dicollecte dictionary and runs this test with `--ignored`. Missing data is a failure,
//! never a silently successful test that checked nothing.

#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{IssueKind, Options, ProofLang, fr};

#[test]
#[ignore = "requires a real French dictionary in WORDCRAFT_DICTIONARIES"]
fn real_french_dictionary_spelling_suggestions_and_dispatch() {
    let directory = std::env::var_os("WORDCRAFT_DICTIONARIES").expect("set WORDCRAFT_DICTIONARIES to the test dictionary folder");
    let installed = fr::installed_path().expect("the real French dictionary must be installed");
    assert_eq!(installed.parent(), Some(std::path::Path::new(&directory)), "do not silently use another system dictionary");
    let dictionary = fr::dictionary();
    assert!(dictionary.len() > 50_000, "expected Dicollecte, not the synthetic fixture: {}", dictionary.len());
    for word in [
        "maison",
        "maisons",
        "Paris",
        "mangeons",
        "finissaient",
        "étaient",
        "l'arbre",
        "l’arbre",
        "qu'elles",
        "école",
        "œuvre",
        "aujourd'hui",
        "chevaux",
        "heureuse",
        "va-t-il",
    ] {
        assert!(fr::is_correct(word), "correct French word: {word}");
    }
    for (typo, expected) in [("maisonn", "maison"), ("ecole", "école"), ("l’ecole", "l’école"), ("documant", "document")] {
        assert!(!fr::is_correct(typo), "misspelling: {typo}");
        let suggestions = fr::suggest(typo, 8);
        assert!(suggestions.iter().any(|s| s == expected), "{typo} -> {suggestions:?}, expected {expected}");
    }
    let text = "Une maisonn près de l’école.";
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let issues = wordcraft_proof::check_text(text, &[(0..text.len(), lang)]);
        assert_eq!(issues.len(), 1, "{lang:?}: {issues:?}");
        let issue = &issues[0];
        assert_eq!(&text[issue.start..issue.end], "maisonn");
        assert_eq!(issue.lang, lang);
        assert_eq!(issue.message, "Faute d’orthographe possible");
    }
    check_combining_marks_and_compounds();
    assert!(wordcraft_proof::dictionary_load_error(ProofLang::Fr).is_none());
    let contraction = "J’est maisonn.";
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let spans = [(0..contraction.len(), lang)];
        let found = wordcraft_proof::check_text(contraction, &spans);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().any(|i| i.kind == wordcraft_proof::IssueKind::Grammar && i.suggestions == ["Je suis"]));
        assert!(found.iter().any(|i| i.kind == wordcraft_proof::IssueKind::Spelling && &contraction[i.start..i.end] == "maisonn"));
        let spelling_only = wordcraft_proof::check_text_with(contraction, &spans, &Default::default(), false, false);
        assert_eq!(spelling_only.len(), 2, "spelling-only mode must keep the invalid contraction: {spelling_only:?}");
        assert!(spelling_only.iter().all(|i| i.kind == wordcraft_proof::IssueKind::Spelling));
    }
    // Grammar must coexist with real spelling, not only with a synthetic fixture: both the
    // infinitive and the participle are valid dictionary words, but only one fits this phrase.
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, flagged, replacement) in [
            ("J’ai manger.", "manger", "mangé"),
            ("Si il vient.", "Si il", "S’il"),
            ("J’est responsable.", "J’est", "Je suis"),
            ("Je m’est préparé.", "m’est", "me suis"),
            ("Il s'es trompé.", "s'es", "s’est"),
            ("Je le avons lu.", "le avons", "l’ai"),
            ("J’y est bien.", "J’y est", "J’y suis"),
            ("Je n’en ont pas besoin.", "n’en ont", "n’en ai"),
        ] {
            let issues = wordcraft_proof::check_text(text, &[(0..text.len(), lang)]);
            assert_eq!(issues.len(), 1, "{lang:?}: {text}: {issues:?}");
            let issue = &issues[0];
            assert_eq!(issue.kind, wordcraft_proof::IssueKind::Grammar);
            assert_eq!(&text[issue.start..issue.end], flagged);
            assert_eq!(issue.suggestions, [replacement]);
            let fixed = format!("{}{}{}", &text[..issue.start], replacement, &text[issue.end..]);
            assert!(wordcraft_proof::check_text(&fixed, &[(0..fixed.len(), lang)]).is_empty(), "{fixed}");
        }
        let correct = "Une question de hiérarchie.";
        assert!(wordcraft_proof::check_text(correct, &[(0..correct.len(), lang)]).is_empty());
    }
}

fn check_combining_marks_and_compounds() {
    let spelling = |text: &str, spans: &[(std::ops::Range<usize>, ProofLang)]| {
        wordcraft_proof::check_text_with(text, spans, &Options::default(), false, false)
            .into_iter()
            .filter(|i| i.kind == IssueKind::Spelling)
            .map(|i| (i.start, i.end, i.lang))
            .collect::<Vec<_>>()
    };
    let mut failures = Vec::new();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for text in
            ["école", "e\u{301}cole", "l’école", "l’e\u{301}cole", "café", "cafe\u{301}", "porte-monnaie", "va-t-il", "a-t-il", "maison-école"]
        {
            assert!(fr::dictionary().check(text) || ["va-t-il", "a-t-il", "maison-école"].contains(&text), "real dictionary recognizes {text:?}");
            let issues = spelling(text, &[(0..text.len(), lang)]);
            if !issues.is_empty() {
                failures.push(format!("correct {lang:?} {text:?}: {issues:?}"));
            }
        }
        for text in ["porte--monnaie", "va--t-il", "maison--école", "e\u{301}colllle", "cafffe\u{301}"] {
            assert!(!fr::dictionary().check(text), "the explicit dictionary entry must not already accept {text:?}");
            let issues = spelling(text, &[(0..text.len(), lang)]);
            let expected = vec![(0, text.len(), lang)];
            if issues != expected {
                failures.push(format!("typo {lang:?} {text:?}: {issues:?}, wanted {expected:?}"));
            }
        }
        let text = "e\u{301}cole cafe\u{301} xyzxyz porte--monnaie";
        let foreign = text.find("xyzxyz").unwrap();
        let compound = text.find("porte").unwrap();
        let spans = [(0..foreign, lang), (foreign..compound, ProofLang::Other), (compound..text.len(), lang)];
        let issues = spelling(text, &spans);
        if issues != [(compound, text.len(), lang)] {
            failures.push(format!("mixed {lang:?}: {issues:?}"));
        }
        assert!(spelling(text, &[(0..text.len(), ProofLang::Other)]).is_empty());
        // An English word remains in its own dictionary next to the original French bytes.
        let text = "e\u{301}cole hello cafe\u{301}";
        let en = text.find("hello").unwrap();
        let spans = [(0..en, lang), (en..en + 5, ProofLang::En), (en + 5..text.len(), lang)];
        let issues = spelling(text, &spans);
        if !issues.is_empty() {
            failures.push(format!("English boundary {lang:?}: {issues:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
