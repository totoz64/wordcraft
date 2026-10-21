//! Original narrow negative nominal-subject corpus; no Word comparison or general accuracy claim.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr, set_dictionaries_folder};
const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";
fn nominal_subject(issue: &Issue) -> bool {
    issue.kind == IssueKind::Grammar && issue.message == SUBJECT
}
fn check(text: &str, language: ProofLang) -> Vec<Issue> {
    check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false).into_iter().filter(nominal_subject).collect()
}
#[test]
#[ignore = "requires pinned French dictionary and morphology folder in WORDCRAFT_DICTIONARIES"]
fn real_negative_nominal_subjects_preserve_elision_ambiguity_and_language_context() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit pinned French input folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert!(fr::has_dictionary());
    assert_eq!(fr::morphology::installed_path().unwrap().parent(), Some(folder.as_path()));
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    let positives = [
        ("Les enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Les enfants ne joue plus.", "ne joue", "ne jouent"),
        ("Les enfants ne joue jamais.", "ne joue", "ne jouent"),
        ("Les enfants ne joue guère.", "ne joue", "ne jouent"),
        ("Les enfants ne joue point.", "ne joue", "ne jouent"),
        ("Les enfants ne joue pas dehors.", "ne joue", "ne jouent"),
        ("Les enfants ne joue jamais ici.", "ne joue", "ne jouent"),
        ("Les enfants ne jouait pas.", "ne jouait", "ne jouaient"),
        ("Les enfants ne jouera jamais.", "ne jouera", "ne joueront"),
        ("Les petits enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Le petit enfant ne jouent pas.", "ne jouent", "ne joue"),
        ("Les petites maisons ne tombe pas.", "ne tombe", "ne tombent"),
        ("Les maisons rouges ne tombe jamais.", "ne tombe", "ne tombent"),
        ("Les enfants n’est pas ici.", "n’est", "ne sont"),
        ("Les enfants ne est pas ici.", "ne est", "ne sont"),
        ("Le petit enfant n’sont pas ici.", "n’sont", "n’est"),
        ("Les enfants n'est pas ici.", "n'est", "ne sont"),
        ("Le petit enfant ne sont pas ici.", "ne sont", "n’est"),
        ("Les enfants ne guide pas.", "ne guide", "ne guident"),
        ("Le petit enfant n’ont jamais raison.", "n’ont", "n’a"),
        ("Bonjour. Les enfants ne joue pas.", "ne joue", "ne jouent"),
        ("Les\u{a0}enfants ne\u{202f}joue pas.", "ne\u{202f}joue", "ne jouent"),
    ];
    let correct = [
        "Les enfants ne jouent pas.",
        "Les enfants ne jouent plus.",
        "Les enfants ne jouent jamais.",
        "Les enfants ne jouent guère.",
        "Les enfants ne jouent point.",
        "Les petits enfants ne jouent pas dehors.",
        "Le petit enfant ne joue pas.",
        "Les petites maisons ne tombent jamais.",
        "Les maisons rouges ne tombent pas.",
        "Les enfants ne sont pas ici.",
        "Le petit enfant n’est pas ici.",
        "Le petit enfant n'est pas ici.",
        "Les enfants n’ont jamais raison.",
        "Le petit enfant n’a jamais raison.",
        "Les jeunes gardes ne surveillent jamais.",
        "Les enfants guides ne travaillent pas.",
        "Les véhicules pilotes ne roulent pas.",
        "Les élèves modèles ne parlent jamais.",
        "Les enfants ne veulent pas jouer.",
        "Les enfants ne jouent que dehors.",
        "Les enfants ne jouent pas plus.",
        "Les enfants ne jouent plus jamais.",
        "La petite fille ne lit pas.",
        "Les enfants ne rient pas.",
        "Les enfants ne sont pas des guides.",
    ];
    // Wrong or ambiguous text outside the chosen parser. Quiet is not counted as correct-text precision.
    let unsupported = [
        "Les enfants ne joue.",
        "Les enfants ne joue que dehors.",
        "Les enfants ne joue pas plus.",
        "Les enfants ne joue plus jamais.",
        "Les enfants ne joue jamais plus.",
        "Les enfants ne joue pas un rôle.",
        "Les enfants n’a jamais raison.",
        "Les enfants ne guides pas.",
        "Les enfants ne pilote pas.",
        "Les enfants ne jouer pas.",
        "Les enfants ne joué pas.",
        "Les enfants ne le joue pas.",
        "Les enfants ne se joue pas.",
        "Les enfants ne s’est jamais levés.",
        "Les enfants de la maison ne joue pas.",
        "Les enfants et leur mère ne joue pas.",
        "Les enfants qui ne joue pas arrivent.",
        "Les enfants, ne joue pas.",
        "Les enfants ne, joue pas.",
        "Les enfants ne joue, pas.",
        "Les enfants ne joue\npas.",
        "Les enfants n’ est pas ici.",
        "Les enfants N’est pas ici.",
        "Les enfants n’EST pas ici.",
        "Les enfants n’est-pas ici.",
        "Les enfants n’est pas un titre.",
        "Les enfants ne joue xyzxyz.",
        "Les enfants ne joue pas au loin près de la montagne.",
        "Les enfants ne e\u{301}crit pas.",
    ];
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, replacement) in positives {
            let found = check(text, language);
            let start = text.find(wrong).unwrap();
            assert_eq!(found.len(), 1, "listed error {language:?}: {text}: {found:?}");
            let issue = &found[0];
            assert_eq!(issue.lang, language);
            assert_eq!((issue.start, issue.end), (start, start + wrong.len()), "{text}: {found:?}");
            assert!(issue.suggestions.iter().any(|suggestion| suggestion == replacement), "{text}: {found:?}");
            let direct = fr::grammar::check_in(text, language);
            assert_eq!(direct.iter().filter(|issue| nominal_subject(issue)).count(), 1, "direct grammar: {text}: {direct:?}");
            assert!(
                !direct.iter().any(|issue| issue.message.starts_with("Élision") && issue.start < start + wrong.len() && start < issue.end),
                "conflicting direct elision: {text}: {direct:?}"
            );
            let full = check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false);
            assert!(
                !full.iter().any(|issue| (issue.kind == IssueKind::Spelling || issue.message.starts_with("Élision"))
                    && issue.start >= start
                    && issue.end <= start + wrong.len()),
                "accepted nominal keeps replacement priority: {text}: {full:?}"
            );
            let fixed = format!("{}{replacement}{}", &text[..start], &text[start + wrong.len()..]);
            assert!(check(&fixed, language).is_empty(), "unstable correction: {fixed}");
            assert!(!check_text_with(text, &[(0..text.len(), language)], &Options::default(), false, false).iter().any(nominal_subject));
        }
        for text in correct {
            assert!(check(text, language).is_empty(), "correct negative changed: {text}: {:?}", check(text, language));
        }
        for text in unsupported {
            assert!(check(text, language).is_empty(), "unsupported negative guessed: {text}: {:?}", check(text, language));
        }
        for text in ["Les enfants ne joue pas.", "Les enfants n’est pas ici."] {
            let mut segments = Vec::new();
            let mut cursor = 0;
            for (start, end) in wordcraft_proof::words(text) {
                if cursor < start {
                    segments.push(cursor..start);
                }
                segments.push(start..end);
                cursor = end;
            }
            if let Some(start) = text.find("n’est") {
                segments.extend([start..start + 1, start + 1..start + 4, start + 4..start + 7]);
            }
            for region in [ProofLang::En, ProofLang::Other, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                for segment in &segments {
                    let spans = [(0..segment.start, language), (segment.clone(), region), (segment.end..text.len(), language)];
                    assert!(
                        !check_text_with(text, &spans, &Options::default(), true, false).iter().any(nominal_subject),
                        "foreign deciding segment {segment:?}: {text}"
                    );
                }
            }
            let split = text.find("pas").unwrap();
            assert_eq!(
                check_text_with(text, &[(0..split, language), (split..text.len(), language)], &Options::default(), true, false)
                    .iter()
                    .filter(|issue| nominal_subject(issue))
                    .count(),
                1,
                "adjacent equal-region runs merge"
            );
        }
        let text = "Les enfants ne est pas ici.";
        let wrong_start = text.find("ne est").unwrap();
        let marker_start = text.find("pas").unwrap();
        for region in [ProofLang::En, ProofLang::Other, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            for segment in [0..3, marker_start..marker_start + 3] {
                let spans = [(0..segment.start, language), (segment.clone(), region), (segment.end..text.len(), language)];
                let mixed = check_text_with(text, &spans, &Options::default(), true, false);
                assert!(!mixed.iter().any(nominal_subject), "foreign nominal anchor: {mixed:?}");
                assert!(
                    mixed.iter().any(|issue| issue.message.starts_with("Élision")
                        && issue.start == wrong_start
                        && issue.suggestions.iter().any(|s| s == "n’est")),
                    "independent same-region elision: {mixed:?}"
                );
            }
        }
        let text = "Hello. Les enfants n’est pas ici.";
        let found = check_text_with(text, &[(0..7, ProofLang::En), (7..text.len(), language)], &Options::default(), true, false);
        assert!(found.iter().any(|issue| nominal_subject(issue) && issue.suggestions.iter().any(|s| s == "ne sont")));
    }
    eprintln!(
        "{} listed negative-subject errors and {} listed correct negatives, each in two French regions; {} ambiguous/unsupported cases separately counted, not real-world accuracy",
        positives.len(),
        correct.len(),
        unsupported.len()
    );
    set_dictionaries_folder(None);
}
