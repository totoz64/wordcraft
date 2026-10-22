//! Bounded original constructions against the installed, pinned MPL lexical data.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr, set_dictionaries_folder};

const NOMINAL: &str = "Accord du verbe avec le groupe nominal sujet";
const COORDINATED: &str = "Accord du verbe avec les sujets coordonnés";

fn family(issue: &Issue) -> bool {
    issue.kind == IssueKind::Grammar && matches!(issue.message.as_str(), NOMINAL | COORDINATED)
}

fn checks(text: &str, language: ProofLang) -> Vec<Issue> {
    check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false).into_iter().filter(family).collect()
}

fn error(text: &str, wrong: &str, expected: &str, language: ProofLang) {
    let found = checks(text, language);
    assert_eq!(found.len(), 1, "{text}: {found:?}");
    let issue = &found[0];
    assert_eq!(&text[issue.start..issue.end], wrong);
    assert!(issue.suggestions.iter().any(|s| s == expected), "{text}: {issue:?}");
    for suggestion in &issue.suggestions {
        let fixed = format!("{}{suggestion}{}", &text[..issue.start], &text[issue.end..]);
        assert!(checks(&fixed, language).is_empty(), "unstable: {fixed}");
    }
}

#[test]
#[ignore = "requires pinned French dictionary and MPL morphology in WORDCRAFT_DICTIONARIES"]
fn real_plural_subject_expansions_preserve_heads_tails_and_languages() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit data folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert!(fr::has_dictionary());
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    let cases = [
        ("Les enfants de la voisine parlait.", "parlait", "parlaient"),
        ("Les enfants de ma sœur parlait.", "parlait", "parlaient"),
        ("Les enfants de ma sœur lui en parlait.", "lui en parlait", "lui en parlaient"),
        ("Les enfants de ma sœur ne leur en parlait jamais.", "ne leur en parlait", "ne leur en parlaient"),
        ("Les enfants des voisins parlait.", "parlait", "parlaient"),
        ("Les enfants du voisin jouait.", "jouait", "jouaient"),
        ("Les enfants de l’amie viendra.", "viendra", "viendront"),
        ("Les enfants de l'école répondait.", "répondait", "répondaient"),
        ("Les filles de la voisine parlait.", "parlait", "parlaient"),
        ("Les petits enfants de ma sœur lui parlait.", "lui parlait", "lui parlaient"),
        ("Les enfants du voisin n’y allait jamais.", "n’y allait", "n’y allaient"),
        ("Bonjour. Les enfants de la voisine parlait souvent ici.", "parlait", "parlaient"),
    ];
    let quiet = [
        "Les enfants de ma sœur parlaient.",
        "Les enfants du voisin n’y allaient jamais.",
        "La majorité des voisins parlent.",
        "La majorité des voisins parle.",
        "Cette file de spectateurs s’impatientent.",
        "La fille des voisins parlent.",
        "Les enfants de voisins parlait.",
        "Des tas de neige tombent.",
        "Les enfants de la sœur des voisins parlait.",
        "Les enfants de la petite voisine parlait.",
        "Les lettres du mot arrive sont nombreuses.",
        "Les lettres du mot arrive.",
        "Les lettres du terme arrive.",
        "Les lettres du titre arrive.",
        "Les lettres du nom arrive.",
        "Les enfants de la voisine joue.",
        "Les enfants de la voisine lui garde.",
        "Les enfants de la voisine parlait les voisins.",
        "Les enfants de la voisine, parlait.",
        "Les enfants de la voisine\nparlait.",
        "Les enfants de les voisins parlait.",
    ];
    let coordinated = [
        ("Toi et moi, nous parlait.", "parlait", "parlions"),
        ("Lui et toi, vous parlait.", "parlait", "parliez"),
        ("Lui et elle, ils parle.", "parle", "parlent"),
        ("Nous et vous, nous parlera.", "parlera", "parlerons"),
        ("Toi et moi, nous viendra.", "viendra", "viendrons"),
    ];
    let coordinated_quiet = [
        "Toi et moi est un film.",
        "Toi et moi raconte une histoire.",
        "Toi et moi, nous parlons.",
        "Lui et toi, vous parliez.",
        "Toi et moi, vous parlait.",
        "Toi et toi, vous parlait.",
        "Toi et moi, nous guides de montagne accompagnons ce groupe.",
        "Toi et moi, nous lui parlait.",
        "Toi et moi, nous ne parlait pas.",
    ];
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, wanted) in coordinated {
            error(text, wrong, wanted, language);
        }
        for text in coordinated_quiet {
            assert!(checks(text, language).is_empty(), "declined {text}: {:?}", checks(text, language));
        }
        for (text, wrong, wanted) in cases {
            error(text, wrong, wanted, language);
        }
        for text in quiet {
            assert!(checks(text, language).is_empty(), "declined {text}: {:?}", checks(text, language));
        }
        for text in [
            "Les enfants de ma sœur parlait souvent ici.",
            "Les enfants de l’amie n’y allait jamais.",
            "Toi et moi, nous parlait.",
            "Les enfants de ma sœur ne leur en parlait jamais ici.",
        ] {
            let mut end = 0;
            for (start, next) in wordcraft_proof::words(text) {
                for segment in [end..start, start..next] {
                    if segment.is_empty() {
                        continue;
                    }
                    for foreign in [ProofLang::Other, ProofLang::En, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                        let spans = [(0..segment.start, language), (segment.clone(), foreign), (segment.end..text.len(), language)];
                        assert!(
                            !check_text_with(text, &spans, &Options::default(), true, false).iter().any(family),
                            "foreign dependency {segment:?} in {text}"
                        );
                    }
                }
                end = next;
            }
            let split = text.find(" de ").or_else(|| text.find(", ")).unwrap();
            assert_eq!(
                check_text_with(text, &[(0..split, language), (split..text.len(), language)], &Options::default(), true, false)
                    .iter()
                    .filter(|i| family(i))
                    .count(),
                1
            );
        }
    }
    eprintln!(
        "{} independently expected expansion errors, {} separately listed correct/unsupported/ambiguous controls; two regions, stable alternatives and per-token/gap foreign context checks. No global precision or Word-parity claim.",
        cases.len(),
        quiet.len()
    );
    eprintln!(
        "{} independently expected recapped-coordination errors and {} separate correct/unsupported controls; original offsets and language context, no title inference.",
        coordinated.len(),
        coordinated_quiet.len()
    );
    set_dictionaries_folder(None);
}
