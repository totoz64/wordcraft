//! Original sentence cases against the pinned external dictionary and morphology.
//! This measures the listed constructions, not general French or parity with Word.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr, set_dictionaries_folder};

const GROUP: &str = "Accord dans le groupe nominal";
const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";

fn nominal(issue: &Issue) -> bool {
    issue.kind == IssueKind::Grammar && matches!(issue.message.as_str(), GROUP | SUBJECT)
}

fn check(text: &str, language: ProofLang) -> Vec<Issue> {
    check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false).into_iter().filter(nominal).collect()
}

#[test]
#[ignore = "requires the pinned real French dictionary and morphology folder in WORDCRAFT_DICTIONARIES"]
fn real_nominal_groups_and_subjects_preserve_ambiguity_and_language_context() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit pinned French input folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert!(fr::has_dictionary());
    assert_eq!(fr::morphology::installed_path().unwrap().parent(), Some(folder.as_path()));
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    let positives = [
        ("Les petite maisons.", "petite", "petites", GROUP),
        ("Les maisons petite.", "petite", "petites", GROUP),
        ("Les maisons petite arrivent.", "petite", "petites", GROUP),
        ("Les maison petites arrivent.", "maison", "maisons", GROUP),
        ("Une petit maison.", "petit", "petite", GROUP),
        ("Une beau maison.", "beau", "belle", GROUP),
        ("Une neuf maison.", "neuf", "neuve", GROUP),
        ("Les maison.", "maison", "maisons", GROUP),
        ("Les cheval.", "cheval", "chevaux", GROUP),
        ("Un maison.", "Un", "Une", GROUP),
        ("La jardin.", "La", "Le", GROUP),
        ("Cette arbre.", "Cette", "Cet", GROUP),
        ("Cette petit arbre.", "Cette", "Ce", GROUP),
        ("La petit arbre.", "La", "Le", GROUP),
        ("Cette énorme jardin.", "Cette", "Cet", GROUP),
        ("Les enfants joue.", "joue", "jouent", SUBJECT),
        ("Les enfants joue dehors.", "joue", "jouent", SUBJECT),
        ("Les enfants jouait.", "jouait", "jouaient", SUBJECT),
        ("Les petits enfants joue.", "joue", "jouent", SUBJECT),
        ("Le petit enfant jouent.", "jouent", "joue", SUBJECT),
        ("Les petites maisons tombe.", "tombe", "tombent", SUBJECT),
        ("Bonjour. Les enfants joue.", "joue", "jouent", SUBJECT),
        ("Ça va. Les petite maisons.", "petite", "petites", GROUP),
    ];
    let valid = [
        "Les petites maisons.",
        "Une petite maison.",
        "Les maisons sont petites.",
        "Les enfants jouent.",
        "Les petits enfants jouent.",
        "Les jeunes gardes surveillent.",
        "Les jeunes gardes.",
        "Les enfants guides.",
        "Les pilotes as.",
        "Les enfants guides et parents arrivent.",
        "Les enfants guides de montagne jouent.",
        "Les contrats types.",
        "Les élèves modèles.",
        "Les projets pilotes.",
        "Les accords cadres.",
        "Les robes orange.",
        "Les robes marron.",
        "Les robes bleu clair.",
        "Les robes bleu Klein.",
        "Les fleurs rose pâle.",
        "Les yeux bleu-vert.",
        "Les bonnes gens arrivent.",
        "Les vieilles amours.",
        "Les grandes orgues.",
        "Les mains nues.",
        "Le grand-père joue.",
        "La petite-fille joue.",
        "Le peu de maisons suffit.",
        "La plupart des enfants jouent.",
        "Les personnes victimes parlent.",
        "Le professeur femme parle.",
        "Les enfants, joue contre joue, rient.",
        "Les enfants que regarde Paul jouent.",
        "Les enfants et leur mère jouent.",
        "Le nom Les enfants joue un rôle.",
        "Le mot petites est un adjectif.",
        "Les enfants demandent : « Qui joue ? »",
        "Les enfants ! Joue ta pièce.",
        "Mon amie arrive.",
        "La livre.",
        "Le livre.",
        "La vieille dit bonjour.",
        "La vieille.",
        "Ces mille maisons.",
        "Les neuf maisons.",
        "Les neuf petites.",
        "Ces mille petits.",
    ];
    // These are ambiguous labels or unsupported errors. Remaining quiet does not
    // count as correct-text precision or successful error detection.
    let unsupported = [
        "Un belle arbre.",
        "Un belle jardin.",
        "Le jardin belle.",
        "Un vieille arbre.",
        "Un nouvelle arbre.",
        "Un folle arbre.",
        "Un molle arbre.",
        "Les arbres molle.",
        "Les véhicules type sport.",
        "Les voitures modèle sport.",
        "Les voitures modèle Q.",
        "Les véhicules type de sport.",
        "Une demi heure.",
        "Les demi heures.",
    ];
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, replacement, family) in positives {
            let found = check(text, language);
            let start = text.find(wrong).unwrap();
            assert!(
                found.iter().any(|issue| issue.lang == language
                    && issue.message == family
                    && issue.start == start
                    && issue.end == start + wrong.len()
                    && issue.suggestions.iter().any(|suggestion| suggestion == replacement)),
                "{language:?}: {text}: {found:?}"
            );
            let fixed = format!("{}{replacement}{}", &text[..start], &text[start + wrong.len()..]);
            assert!(check(&fixed, language).is_empty(), "unstable correction: {fixed}");
            assert!(
                !check_text_with(text, &[(0..text.len(), language)], &Options::default(), false, false).iter().any(nominal),
                "grammar disabled: {text}"
            );
            // Every word and intervening gap used by these initial groups belongs
            // to one region. The preceding independent sentence is not an anchor.
            let group_start = text.rfind("Les ").unwrap_or(0);
            let group_end = text.trim_end_matches('.').len();
            let group = &text[group_start..group_end];
            let mut segments = Vec::new();
            let mut cursor = group_start;
            for (a, b) in wordcraft_proof::words(group) {
                let (a, b) = (group_start + a, group_start + b);
                if cursor < a {
                    segments.push(cursor..a);
                }
                segments.push(a..b);
                cursor = b;
            }
            for region in [ProofLang::En, ProofLang::Other, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                for segment in &segments {
                    let spans = [(0..segment.start, language), (segment.clone(), region), (segment.end..text.len(), language)];
                    let mixed = check_text_with(text, &spans, &Options::default(), true, false);
                    assert!(
                        !mixed.iter().any(|issue| nominal(issue) && issue.message == family),
                        "foreign deciding segment {segment:?}: {text}: {mixed:?}"
                    );
                }
            }
        }
        for text in valid {
            assert!(check(text, language).is_empty(), "valid text changed: {text}: {:?}", check(text, language));
        }
        for text in unsupported {
            assert!(check(text, language).is_empty(), "unsupported construction guessed: {text}");
        }
        for text in ["Les véhicules type.", "Les voitures modèle.", "Les projets pilote.", "Les accords cadre."] {
            assert!(!check(text, language).iter().any(|issue| issue.message == SUBJECT), "nominal epithet guessed as a verb: {text}");
        }
        // A postposed modifier needs the following finite form to distinguish it from
        // a colour or product designation. Every positively deciding byte is French.
        for text in ["Les maisons petite arrivent.", "Les maison petites arrivent."] {
            let start = text.find("arrivent").unwrap();
            let end = start + "arrivent".len();
            for segment in [start..end, start - 1..start] {
                for foreign in [ProofLang::Other, ProofLang::En, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                    let spans = [(0..segment.start, language), (segment.clone(), foreign), (segment.end..text.len(), language)];
                    let found = check_text_with(text, &spans, &Options::default(), true, false);
                    assert!(!found.iter().any(nominal), "postposed deciding token {segment:?}: {text}: {found:?}");
                }
            }
        }
        // A preposed adjective is already decided by determiner + modifier + noun;
        // an unrelated following finite word must not enlarge that local context.
        let text = "Les petite maisons arrivent.";
        let start = text.find("arrivent").unwrap();
        let end = start + "arrivent".len();
        for segment in [start..end, start - 1..start] {
            for foreign in [ProofLang::Other, ProofLang::En, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                let spans = [(0..segment.start, language), (segment.clone(), foreign), (segment.end..text.len(), language)];
                let found = check_text_with(text, &spans, &Options::default(), true, false);
                assert!(
                    found.iter().any(|issue| nominal(issue) && issue.message == GROUP && issue.suggestions.iter().any(|s| s == "petites")),
                    "preposed local context {segment:?}: {found:?}"
                );
            }
        }
        // A separately tagged preceding sentence does not supply a deciding token.
        let text = "Hello. Les petite maisons.";
        let found = check_text_with(text, &[(0..7, ProofLang::En), (7..text.len(), language)], &Options::default(), true, false);
        assert!(found.iter().any(|issue| { nominal(issue) && issue.suggestions.iter().any(|suggestion| suggestion == "petites") }));
        let text = "Les petite maisons.";
        let found = check_text_with(text, &[(0..4, language), (4..text.len(), language)], &Options::default(), true, false);
        assert!(found.iter().any(nominal), "adjacent equal-region runs merge");
    }
    eprintln!(
        "{} listed error cases and {} listed correct-text cases, each in two French regions; unsupported cases counted separately, not a real-world accuracy estimate",
        positives.len(),
        valid.len()
    );
    set_dictionaries_folder(None);
}
