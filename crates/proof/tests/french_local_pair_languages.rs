//! A local French replacement must not consume bytes from another proofing language.
use wordcraft_proof::{IssueKind, Options, ProofLang, check_text_with};

fn family(issue: &wordcraft_proof::Issue, prefix: &str) -> bool {
    issue.kind == IssueKind::Grammar && matches!(issue.lang, ProofLang::Fr | ProofLang::FrCa) && issue.message.starts_with(prefix)
}

#[test]
fn french_local_word_pair_suggestions_do_not_cross_language_ranges() {
    let options = Options::default();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, split, prefix, replacement) in [("le ami", 3usize, "Élision", "l’ami"), ("test test", 5usize, "Mot répété", "test")] {
            for spans in [vec![(0..text.len(), lang)], vec![(0..split, lang), (split..text.len(), lang)]] {
                let ordinary = check_text_with(text, &spans, &options, true, false);
                assert!(ordinary.iter().any(|issue| family(issue, prefix) && issue.suggestions.iter().any(|s| s == replacement)), "{ordinary:?}");
            }
            let other_region = if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr };
            for foreign in [ProofLang::Other, ProofLang::En, other_region] {
                for spans in [[(0..split, lang), (split..text.len(), foreign)], [(0..split, foreign), (split..text.len(), lang)]] {
                    let actual = check_text_with(text, &spans, &options, true, false);
                    assert!(actual.iter().all(|issue| !family(issue, prefix)), "cross-language {prefix}: {text}, {spans:?}: {actual:?}");
                    let disabled = check_text_with(text, &spans, &options, false, false);
                    assert!(disabled.iter().all(|issue| issue.kind != IssueKind::Grammar));
                    let spelling = |issues: &[wordcraft_proof::Issue]| {
                        issues
                            .iter()
                            .filter(|issue| issue.kind == IssueKind::Spelling)
                            .map(|issue| (issue.start, issue.end, issue.lang))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(spelling(&actual), spelling(&disabled), "rejected grammar must not suppress spelling");
                }
            }
        }
        // An uncovered byte has the default English language; it is not a French bridge.
        let gap = check_text_with("le ami", &[(0..2, lang), (3..6, lang)], &options, true, false);
        assert!(gap.iter().all(|issue| !family(issue, "Élision")));
        // Local replacement bytes, unlike an initial syntactic context, do not need a French paragraph prefix.
        for text in ["xyz le ami", "Été, le élan."] {
            let start = text.find("le ").unwrap();
            let end = text.trim_end_matches('.').len();
            let spans = [(0..start, ProofLang::Other), (start..text.len(), lang)];
            assert!(text.is_char_boundary(start) && text.is_char_boundary(end));
            let actual = check_text_with(text, &spans, &options, true, false);
            assert!(actual.iter().any(|issue| family(issue, "Élision") && issue.start == start && issue.end == end), "{actual:?}");
        }
        let text = "été été";
        let split = "été ".len();
        let adjacent = check_text_with(text, &[(0..split, lang), (split..text.len(), lang)], &options, true, false);
        assert!(adjacent.iter().any(|issue| family(issue, "Mot répété") && issue.start == 0 && issue.end == text.len()));
        let mixed = check_text_with(text, &[(0..split, lang), (split..text.len(), ProofLang::Other)], &options, true, false);
        assert!(mixed.iter().all(|issue| !family(issue, "Mot répété")));
    }
}

#[test]
fn french_quote_replacements_keep_foreign_content_and_local_punctuation_separate() {
    let options = Options::default();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let text = "\"hello\"";
        let ordinary = check_text_with(text, &[(0..text.len(), lang)], &options, true, false);
        assert!(ordinary.iter().any(|issue| family(issue, "Guillemets français") && issue.start == 0 && issue.end == text.len()));
        for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            let actual = check_text_with(text, &[(0..1, lang), (1..6, foreign), (6..7, lang)], &options, true, false);
            assert!(actual.iter().all(|issue| !family(issue, "Guillemets français")), "{actual:?}");
        }
        // Each French guillemet has a quote-only replacement, so the foreign interior is untouched.
        let text = "«hello»";
        let close = text.len() - '»'.len_utf8();
        let actual = check_text_with(text, &[(0..2, lang), (2..close, ProofLang::En), (close..text.len(), lang)], &options, true, false);
        for (start, end, replacement) in [(0, 2, "«\u{a0}"), (close, text.len(), "\u{a0}»")] {
            assert!(
                actual.iter().any(|issue| issue.kind == IssueKind::Grammar
                    && issue.lang == lang
                    && issue.start == start
                    && issue.end == end
                    && issue.suggestions == [replacement]),
                "{actual:?}"
            );
        }
        // Neither an outside full stop nor the preceding foreign word is replaced by this local issue.
        let actual = check_text_with("le ami.", &[(0..6, lang), (6..7, ProofLang::Other)], &options, true, false);
        assert!(actual.iter().any(|issue| family(issue, "Élision") && issue.start == 0 && issue.end == 6));
        let actual = check_text_with("hello:", &[(0..5, ProofLang::En), (5..6, lang)], &options, true, false);
        assert!(actual.iter().any(|issue| issue.kind == IssueKind::Grammar
            && issue.lang == lang
            && issue.start == 5
            && issue.end == 6
            && issue.suggestions == ["\u{a0}:"]));
    }
}
