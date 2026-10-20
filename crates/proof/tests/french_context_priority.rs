//! Original process-isolated regression for French context filtering and issue precedence.
//! No installed dictionary, external morphology, environment mutation or network is needed.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text, check_text_with, fr, set_dictionaries_folder};

const AUXILIARY_MESSAGE: &str = "Auxiliaire être avec un verbe pronominal";

// This integration-test file deliberately has one test. Dictionary and morphology
// caches are process-global; a separate test executable owns this tiny fixture.
struct Fixture {
    folder: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self { folder: tempfile::Builder::new().prefix("wordcraft-french-context-priority-").tempdir().unwrap() };
        let rows = [
            ("ont", "avoir", "v0ait____a ipre 3pl"),
            ("sont", "être", "v0ei_____a ipre 3pl"),
            ("trompent", "tromper", "v1_it_q__a ipre 3pl"),
            ("trompés", "tromper", "v1_it_q__a ppas mas pl"),
        ];
        let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
        for (index, (form, lemma, tags)) in rows.iter().enumerate() {
            table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
        }
        assert!(table.lines().all(|line| line.split('\t').count() == 20));
        std::fs::write(fixture.folder.path().join(fr::morphology::FILE_NAME), table).unwrap();
        set_dictionaries_folder(Some(fixture.folder.path().to_owned()));
        // Deliberately omit ont, se, trompés and xyzxyz. Their spelling marks let
        // this test distinguish accepted replacement ranges from deciding contexts.
        fr::install_dictionary("SET UTF-8\n", "3\nIls\net\nsont\n").unwrap();
        fixture
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Release global caches before TempDir removes only its own unique directory.
        set_dictionaries_folder(None);
    }
}

fn has_spelling(text: &str, issues: &[Issue], token: &str) -> bool {
    issues.iter().any(|issue| issue.kind == IssueKind::Spelling && text.get(issue.start..issue.end) == Some(token))
}

#[test]
fn french_context_filtering_preserves_independent_elisions_and_spelling() {
    let fixture = Fixture::new();
    match fr::morphology::data_status() {
        fr::morphology::DataStatus::Ready { path, analyses } => {
            assert_eq!(path, fixture.folder.path().join(fr::morphology::FILE_NAME));
            assert_eq!(analyses, 3, "only the process-owned finite fixture is used");
        }
        status => panic!("synthetic morphology must be loaded: {status:?}"),
    }
    let text = "Ils se ont trompés et xyzxyz.";
    let participle_start = text.find("trompés").unwrap();
    let participle_end = participle_start + "trompés".len();

    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let whole = [(0..text.len(), lang)];
        let spelling_only = check_text_with(text, &whole, &Options::default(), false, false);
        assert!(spelling_only.iter().all(|issue| issue.kind == IssueKind::Spelling));
        for token in ["ont", "trompés", "xyzxyz"] {
            assert!(has_spelling(text, &spelling_only, token), "fixture spelling: {lang:?}: {token}");
        }

        let direct = fr::grammar::check_in(text, lang);
        assert_eq!(direct.len(), 1, "public single-language grammar keeps its precedence");
        assert_eq!(direct[0].message, AUXILIARY_MESSAGE);

        let full = check_text(text, &whole);
        let auxiliary = full.iter().find(|issue| issue.message == AUXILIARY_MESSAGE).expect("the full French context permits auxiliary correction");
        assert_eq!(auxiliary.lang, lang);
        assert_eq!(&text[auxiliary.start..auxiliary.end], "se ont");
        assert_eq!(auxiliary.suggestions, ["se sont"]);
        assert!(!full.iter().any(|issue| issue.message.starts_with("Élision")), "accepted auxiliary keeps precedence");
        assert!(!has_spelling(text, &full, "ont"), "accepted replacement suppresses conflicting spelling");
        for token in ["trompés", "xyzxyz"] {
            assert!(has_spelling(text, &full, token), "unchanged context retains spelling: {token}");
        }

        for foreign in [ProofLang::En, ProofLang::Other, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            let spans = [(0..participle_start, lang), (participle_start..participle_end, foreign), (participle_end..text.len(), lang)];
            let mixed = check_text(text, &spans);
            assert!(
                !mixed.iter().any(|issue| issue.message == AUXILIARY_MESSAGE),
                "a foreign participle cannot choose an auxiliary: {lang:?}/{foreign:?}"
            );
            let elision = mixed
                .iter()
                .find(|issue| issue.message.starts_with("Élision"))
                .expect("rejecting the auxiliary must retain the independent French elision");
            assert_eq!(elision.lang, lang);
            assert_eq!(&text[elision.start..elision.end], "se ont");
            assert_eq!(elision.suggestions, ["s’ont"]);
            assert!(has_spelling(text, &mixed, "ont"), "a rejected auxiliary cannot consume prefix spelling");
            assert!(has_spelling(text, &mixed, "xyzxyz"), "unrelated French spelling remains");

            let grammar_off = check_text_with(text, &spans, &Options::default(), false, false);
            assert!(grammar_off.iter().all(|issue| issue.kind == IssueKind::Spelling));
            assert!(has_spelling(text, &grammar_off, "ont"));
            assert!(has_spelling(text, &grammar_off, "xyzxyz"));
        }
    }
}
