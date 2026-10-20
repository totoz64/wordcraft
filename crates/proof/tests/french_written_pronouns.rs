//! Process-owned original examples for explicit written-pronoun mentions.
use wordcraft_proof::{IssueKind, Options, ProofLang, check_text_with, fr};

#[test]
fn explicit_mentions_preserve_typography_and_ordinary_subject_errors() {
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for subject in ["Je", "Tu", "Il", "Elle", "On", "Nous", "Vous", "Ils", "Elles"] {
            for copula in ["est", "était", "fut", "sera", "serait", "soit", "fût"] {
                for tail in [
                    "un pronom personnel.",
                    "écrit en gras sur cette affiche.",
                    "écrit en italique.",
                    "écrit en italiques.",
                    "écrit en majuscules.",
                    "écrit en minuscules.",
                ] {
                    let text = format!("{subject} {copula} {tail}");
                    assert!(fr::grammar::check_in(&text, lang).is_empty(), "{lang:?}: {text}");
                }
            }
            for auxiliary in ["a", "avait", "eut", "aura", "aurait", "ait", "eût"] {
                let text = format!("{subject} {auxiliary} été écrit en gras.");
                assert!(fr::grammar::check_in(&text, lang).is_empty(), "{lang:?}: {text}");
            }
        }
        for (text, suggestion) in [
            ("Je est responsable.", "suis"),
            ("Nous a un rendez-vous.", "avons"),
            ("Je a faim.", "J’ai"),
            ("Je est un acteur.", "suis"),
            ("Je est responsable. Le texte est en gras.", "suis"),
            ("Je est responsable, ce pronom vient ensuite.", "suis"),
            ("Je est responsable de ce pronom.", "suis"),
            ("Je est dans cette grande salle devant ces nombreuses personnes un pronom.", "suis"),
        ] {
            let issues = fr::grammar::check_in(text, lang);
            assert!(
                issues.iter().any(|issue| issue.message == "Accord du verbe avec le pronom sujet"
                    && issue.suggestions.iter().any(|candidate| candidate == suggestion)),
                "{lang:?}: {text}: {issues:?}"
            );
        }
        let repeated = fr::grammar::check_in("Je est un pronom de de politesse.", lang);
        assert!(repeated.iter().any(|issue| issue.message.contains("répété")), "{repeated:?}");
        let punctuation = fr::grammar::check_in("Je est un pronom!", lang);
        assert!(!punctuation.iter().any(|issue| issue.message.starts_with("Élision") || issue.message == "Accord du verbe avec le pronom sujet"));
        if lang == ProofLang::Fr {
            assert!(punctuation.iter().any(|issue| issue.message.contains("Espace")), "typography remains: {punctuation:?}");
        }
    }
    // An explicit process-owned spelling fixture avoids depending on installed data.
    fr::install_dictionary("SET UTF-8\n", "5\nJe\nest\nun\npronom\npersonnel\n").unwrap();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let text = "Je est un pronom xyzxyz.";
        let spans = [(0..text.len(), lang)];
        let on = check_text_with(text, &spans, &Options::default(), true, false);
        let off = check_text_with(text, &spans, &Options::default(), false, false);
        assert_eq!(on, off, "the mention exemption does not consume spelling");
        assert!(on.iter().all(|issue| issue.kind == IssueKind::Spelling));
        assert!(on.iter().any(|issue| &text[issue.start..issue.end] == "xyzxyz"));
    }
}
