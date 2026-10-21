//! Original synthetic regressions for bounded, language-aware autonym protection.
use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr};

const PERSONAL: &str = "Accord du verbe avec le pronom sujet";

fn suggested_at(issues: &[Issue], start: usize, end: usize, suggestion: &str) -> bool {
    issues.iter().any(|issue| issue.start == start && issue.end == end && issue.suggestions.iter().any(|candidate| candidate == suggestion))
}

fn initial_elision(issues: &[Issue], subject_start: usize) -> bool {
    issues.iter().any(|issue| issue.start == subject_start && issue.message.starts_with("Élision"))
}

#[test]
fn explicit_word_mentions_preserve_independent_local_elisions() {
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for text in [
            "Je est un mot français.",
            "Nous est un mot court.",
            "Je était le mot retenu.",
            "Je sera ce mot répété.",
            "Je a été le mot retenu.",
            "Je est un pronom personnel.",
        ] {
            let issues = fr::grammar::check_in(text, language);
            assert!(!initial_elision(&issues, 0), "{language:?}: {text}: {issues:?}");
            assert!(!issues.iter().any(|issue| issue.message == PERSONAL), "{language:?}: {text}: {issues:?}");
        }
        for (text, target, correction) in [
            ("Je est un mot de ami.", "de ami", "d’ami"),
            ("Je est un mot français. Il refuse de avoir peur.", "de avoir", "d’avoir"),
            ("Je est un pronom. Il refuse de avoir peur.", "de avoir", "d’avoir"),
            ("Je est écrit en gras, le ami arrive.", "le ami", "l’ami"),
            ("Nous a été écrit en gras. Le ami arrive.", "Le ami", "L’ami"),
        ] {
            let issues = fr::grammar::check_in(text, language);
            let start = text.find(target).unwrap();
            assert!(suggested_at(&issues, start, start + target.len(), correction), "{language:?}: {text}: {issues:?}");
            assert!(!initial_elision(&issues, 0), "{language:?}: {text}: {issues:?}");
        }
        let text = "Je est un mot de de français: le ami arrive.";
        let issues = fr::grammar::check_in(text, language);
        assert!(issues.iter().any(|issue| issue.message.contains("répété")), "{issues:?}");
        let colon = text.find(':').unwrap();
        assert!(suggested_at(&issues, colon, colon + 1, "\u{a0}:"), "{issues:?}");
        let ordinary = fr::grammar::check_in("Je est responsable de ce pronom.", language);
        assert!(ordinary.iter().any(|issue| issue.message == PERSONAL && issue.suggestions.iter().any(|word| word == "suis")), "{ordinary:?}");
        for (unsupported, candidate_allowed) in [
            ("Je est responsable de ce mot.", false),
            ("Je est un motard.", true),
            ("Je est un moteur.", true),
            ("Je est un mots.", false),
            ("Je est mot pour mot identique.", false),
            ("Je est mot à mot identique.", false),
        ] {
            let issues = fr::grammar::check_in(unsupported, language);
            if candidate_allowed {
                assert!(
                    issues.iter().any(|issue| issue.message == PERSONAL && issue.suggestions.iter().any(|word| word == "suis")),
                    "no new mention exemption: {language:?}: {unsupported}: {issues:?}"
                );
            } else {
                assert!(
                    initial_elision(&issues, 0),
                    "keep the existing candidate veto without adding mention protection: {language:?}: {unsupported}: {issues:?}"
                );
            }
        }
        assert!(fr::grammar::check_in("Nous est absent de ce mot.", language).is_empty(), "keep the existing conservative candidate veto");
        for hostile in ["", "Je", "Je a", "Je a été", "Je est un", "Je est un,mot.", "Je est un; mot.", "Je est un\nmot.", "Je est un\tmot."] {
            let _ = fr::grammar::check_in(hostile, language);
        }
    }
}

#[test]
fn autonym_deciding_ranges_are_validated_before_veto_and_spelling_priority() {
    // This executable owns its explicit spelling fixture; it never installs real files.
    let words = ["Je", "est", "un", "mot", "français", "pronom", "personnel", "écrit", "en", "gras", "suis", "Il", "refuse", "de", "avoir", "peur"];
    let dictionary = format!("{}\n{}\n", words.len(), words.join("\n"));
    fr::install_dictionary("SET UTF-8\n", &dictionary).unwrap();
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, marker, has_candidate) in [
            ("\u{a0}Je est un mot français.", "mot", false),
            ("\u{a0}Je est un pronom personnel.", "pronom", true),
            ("\u{a0}Je est écrit en gras.", "gras", true),
        ] {
            let subject = text.find("Je").unwrap();
            let marker_start = text.find(marker).unwrap();
            let marker_end = marker_start + marker.len();
            let options = Options::default();
            let all = check_text_with(text, &[(0..text.len(), language)], &options, true, false);
            assert!(!initial_elision(&all, subject), "valid whole-French mention: {text}: {all:?}");
            assert!(!all.iter().any(|issue| issue.message == PERSONAL), "{all:?}");
            let adjacent = [(0..marker_start, language), (marker_start..text.len(), language)];
            assert_eq!(all, check_text_with(text, &adjacent, &options, true, false), "same-region neighbours merge");
            let other_region = if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr };
            for foreign in [ProofLang::Other, ProofLang::En, other_region] {
                let spans = [(0..marker_start, language), (marker_start..marker_end, foreign), (marker_end..text.len(), language)];
                let mixed = check_text_with(text, &spans, &options, true, false);
                if has_candidate {
                    assert!(
                        mixed
                            .iter()
                            .any(|issue| issue.lang == language && issue.message == PERSONAL && issue.suggestions.iter().any(|word| word == "suis")),
                        "foreign cue cannot veto agreement: {language:?}/{foreign:?}: {text}: {mixed:?}"
                    );
                    assert!(!initial_elision(&mixed, subject), "accepted agreement retains elision priority: {mixed:?}");
                } else {
                    // The old candidate() mot/mots veto is deliberately retained.
                    assert!(initial_elision(&mixed, subject), "foreign mot cannot introduce the new mention veto: {mixed:?}");
                }
            }
            let with_tail = format!("{text} foreign");
            let tail_spans = [(0..text.len(), language), (text.len()..with_tail.len(), ProofLang::Other)];
            assert!(
                !initial_elision(&check_text_with(&with_tail, &tail_spans, &options, true, false), subject),
                "an unrelated foreign tail is outside the deciding range"
            );
        }
        let text = "Je est un mot français.";
        let un = text.find("un").unwrap();
        let spans = [(0..un, language), (un..un + 2, ProofLang::Other), (un + 2..text.len(), language)];
        assert!(
            initial_elision(&check_text_with(text, &spans, &Options::default(), true, false), 0),
            "a foreign determiner also invalidates the deciding context"
        );
        let marker = text.find("mot").unwrap();
        let gap = [(0..marker, language), (marker + 3..text.len(), language)];
        assert!(initial_elision(&check_text_with(text, &gap, &Options::default(), true, false), 0), "an uncovered marker is default English");
        let text = "Je est un mot xyzxyz.";
        let spans = [(0..text.len(), language)];
        let on = check_text_with(text, &spans, &Options::default(), true, false);
        let off = check_text_with(text, &spans, &Options::default(), false, false);
        let spelling = |issues: Vec<Issue>| issues.into_iter().filter(|issue| issue.kind == IssueKind::Spelling).collect::<Vec<_>>();
        assert!(
            on.iter().any(|issue| issue.kind == IssueKind::Spelling && text.get(issue.start..issue.end) == Some("xyzxyz")),
            "nonempty spelling fixture: {on:?}"
        );
        assert_eq!(spelling(on), spelling(off.clone()), "mention protection does not consume spelling");
        assert!(off.iter().all(|issue| issue.kind != IssueKind::Grammar), "grammar off is preserved");
    }
}
