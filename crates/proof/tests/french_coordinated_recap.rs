//! Process-owned lexical and spelling fixtures for an explicit coordinated recap.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr};

const MESSAGE: &str = "Accord du verbe avec les sujets coordonnés";

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-coordinated-recap-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        wordcraft_proof::set_dictionaries_folder(None);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn coordinated(issue: &Issue) -> bool {
    issue.kind == IssueKind::Grammar && issue.message == MESSAGE
}

#[test]
fn recapped_coordination_requires_every_deciding_byte_before_replacement_priority() {
    let scratch = Scratch::new();
    let rows = [
        ("parle", "parler", "v1 ipre 1sg 3sg"),
        ("parlons", "parler", "v1 ipre 1pl"),
        ("parlez", "parler", "v1 ipre 2pl"),
        ("parlent", "parler", "v1 ipre 3pl"),
        ("parlait", "parler", "v1 iimp 3sg"),
        ("parlions", "parler", "v1 iimp 1pl"),
        ("parliez", "parler", "v1 iimp 2pl"),
        ("parlaient", "parler", "v1 iimp 3pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));
    // Deliberately omit parlait from this tiny spelling dictionary: the collision
    // tests replacement priority, not whether real French dictionaries know that word.
    fr::install_dictionary("SET UTF-8\n", "12\ntoi\net\nmoi\nlui\nelle\nnous\nvous\nils\nelles\nparlions\nparliez\nparlent\n").unwrap();
    let options = Options::default();
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, correction) in [
            ("Toi et moi, nous parlait.", "parlions"),
            ("Lui et toi, vous parlait.", "parliez"),
            ("Lui et elle, ils parle.", "parlent"),
            ("\u{a0}Toi\u{202f}et moi,\u{a0}nous parlait.", "parlions"),
        ] {
            let all = check_text_with(text, &[(0..text.len(), language)], &options, true, false);
            let found: Vec<_> = all.iter().filter(|issue| coordinated(issue)).collect();
            assert_eq!(found.len(), 1, "{language:?}: {text}: {all:?}");
            assert_eq!(found[0].lang, language);
            assert_eq!(found[0].suggestions, [correction]);
            assert!(text.is_char_boundary(found[0].start) && text.is_char_boundary(found[0].end));
            let split = text.find(',').unwrap();
            assert_eq!(all, check_text_with(text, &[(0..split, language), (split..text.len(), language)], &options, true, false));
        }
        let text = "Toi et moi, nous parlait xyzxyz.";
        let verb_start = text.find("parlait").unwrap();
        let verb_end = verb_start + "parlait".len();
        let all = check_text_with(text, &[(0..text.len(), language)], &options, true, false);
        assert!(all.iter().any(coordinated));
        assert!(!all.iter().any(|issue| issue.kind == IssueKind::Spelling && issue.start == verb_start && issue.end == verb_end));
        assert!(all.iter().any(|issue| issue.kind == IssueKind::Spelling && text.get(issue.start..issue.end) == Some("xyzxyz")));
        let off = check_text_with(text, &[(0..text.len(), language)], &options, false, false);
        assert!(off.iter().all(|issue| issue.kind != IssueKind::Grammar));
        assert!(off.iter().any(|issue| issue.kind == IssueKind::Spelling && issue.start == verb_start && issue.end == verb_end));
        for foreign in [ProofLang::Other, ProofLang::En, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            for (start, end) in [(0, 3), (4, 6), (7, 10), (10, 11), (12, 16), (verb_start, verb_end)] {
                let spans = [(0..start, language), (start..end, foreign), (end..text.len(), language)];
                let actual = check_text_with(text, &spans, &options, true, false);
                assert!(!actual.iter().any(coordinated), "deciding foreign bytes {start}..{end}: {language:?}/{foreign:?}: {actual:?}");
                if end <= verb_start {
                    assert!(
                        actual.iter().any(|issue| issue.kind == IssueKind::Spelling && issue.start == verb_start && issue.end == verb_end),
                        "rejected coordination must retain French spelling: {actual:?}"
                    );
                }
            }
        }
        let gap = check_text_with(text, &[(0..10, language), (11..text.len(), language)], &options, true, false);
        assert!(!gap.iter().any(coordinated), "uncovered comma defaults to English");
        let tail = text.find("xyzxyz").unwrap();
        let foreign_tail = check_text_with(text, &[(0..tail, language), (tail..text.len(), ProofLang::Other)], &options, true, false);
        assert!(foreign_tail.iter().any(coordinated), "unrelated foreign following words are not deciding context");
        for title_or_unsupported in [
            "Toi et moi est un film.",
            "Toi et moi raconte une histoire.",
            "Toi et moi parlait.",
            "Toi et moi, vous parlait.",
            "Lui et elle, nous parlait.",
            "Toi et moi nous parlait.",
        ] {
            let actual = check_text_with(title_or_unsupported, &[(0..title_or_unsupported.len(), language)], &options, true, false);
            assert!(!actual.iter().any(coordinated), "not a covered correction: {title_or_unsupported}: {actual:?}");
        }
    }
}
