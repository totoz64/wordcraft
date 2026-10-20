//! The deciding French subject/auxiliary cannot be borrowed from another language span.
use wordcraft_proof::{Issue, IssueKind, Options, ProofLang, check_text_with, fr, set_dictionaries_folder};

fn syntactic(issues: &[Issue]) -> Vec<&Issue> {
    issues
        .iter()
        .filter(|issue| {
            issue.kind == IssueKind::Grammar
                && (issue.message == "Accord du verbe avec le pronom sujet" || issue.message.starts_with("Participe passé attendu après avoir"))
        })
        .collect()
}

#[test]
fn french_syntactic_context_stays_within_one_language() {
    let folder = tempfile::tempdir().unwrap();
    set_dictionaries_folder(Some(folder.path().to_path_buf()));
    fr::install_dictionary("SET UTF-8\n", "7\nNous\nmanges\nmangeons\nécrire\nécrit\navons\nbonjour\n").unwrap();
    let options = Options::default();
    let cases: [(&str, &str, &str, &[&str]); 7] = [
        ("Nous a raison.", "a", "Nous avons raison.", &["Nous", "a"]),
        ("J’ai écrire.", "écrire", "J’ai écrit.", &["J’ai", "écrire"]),
        ("J’ai déjà écrire.", "écrire", "J’ai déjà écrit.", &["J’ai", "déjà", "écrire"]),
        ("Je n’ai pas écrire.", "écrire", "Je n’ai pas écrit.", &["Je", "n’ai", "pas", "écrire"]),
        ("\u{a0}Nous a raison.", "a", "\u{a0}Nous avons raison.", &["Nous", "a"]),
        ("—\u{202f}J’ai déjà écrire.", "écrire", "—\u{202f}J’ai déjà écrit.", &["J’ai", "déjà", "écrire"]),
        ("J’ai pouvoir partir.", "pouvoir", "J’ai pu partir.", &["J’ai", "pouvoir", "partir"]),
    ];
    let mut failures = Vec::new();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, target, corrected, deciding_words) in cases {
            let same = check_text_with(text, &[(0..text.len(), lang)], &options, true, false);
            assert_eq!(syntactic(&same).len(), 1, "same-language positive control: {text}: {same:?}");
            let fixed = check_text_with(corrected, &[(0..corrected.len(), lang)], &options, true, false);
            assert!(syntactic(&fixed).is_empty(), "corrected quiet control: {corrected}: {fixed:?}");
            let split = text.find(target).unwrap();
            let neighbouring = check_text_with(text, &[(0..split, lang), (split..text.len(), lang)], &options, true, false);
            assert_eq!(syntactic(&neighbouring).len(), 1, "neighbouring spans of the same language merge");
            let with_tail = format!("{text} English");
            let tail_spans = [(0..text.len(), lang), (text.len()..with_tail.len(), ProofLang::En)];
            assert_eq!(
                syntactic(&check_text_with(&with_tail, &tail_spans, &options, true, false)).len(),
                1,
                "an unrelated foreign tail is outside the deciding context"
            );
            for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                let spans = [(0..split, foreign), (split..text.len(), lang)];
                let found = check_text_with(text, &spans, &options, true, false);
                if !syntactic(&found).is_empty() {
                    failures.push(format!("{lang:?}/{foreign:?}: {text:?}: {:?}", syntactic(&found)));
                }
                let disabled = check_text_with(text, &spans, &options, false, false);
                assert!(syntactic(&disabled).is_empty());
                for word in deciding_words {
                    let start = text.find(word).unwrap();
                    let end = start + word.len();
                    let spans = [(0..start, lang), (start..end, foreign), (end..text.len(), lang)];
                    let found = check_text_with(text, &spans, &options, true, false);
                    if !syntactic(&found).is_empty() {
                        failures.push(format!("foreign deciding word {word:?}: {lang:?}/{foreign:?}: {text:?}: {:?}", syntactic(&found)));
                    }
                }
            }
        }
        let phrase = "Tu ne a pas raison.";
        let all_french = check_text_with(phrase, &[(0..phrase.len(), lang)], &options, true, false);
        assert!(syntactic(&all_french).iter().any(|issue| issue.suggestions.iter().any(|suggestion| suggestion == "n’as")));
        assert!(!all_french.iter().any(|issue| issue.message.starts_with("Élision")));
        let foreign_subject = [(0..2, ProofLang::Other), (2..phrase.len(), lang)];
        let mixed = check_text_with(phrase, &foreign_subject, &options, true, false);
        assert!(syntactic(&mixed).is_empty());
        assert!(
            mixed.iter().any(|issue| issue.message.starts_with("Élision") && issue.suggestions.iter().any(|suggestion| suggestion == "n’a")),
            "rejected agreement must retain independent elision: {mixed:?}"
        );
        let text = "Nous a raison xyzxyz.";
        let spans = [(0..4, ProofLang::Other), (4..text.len(), lang)];
        let on = check_text_with(text, &spans, &options, true, false);
        let off = check_text_with(text, &spans, &options, false, false);
        let spelling = |issues: Vec<Issue>| {
            issues
                .into_iter()
                .filter(|issue| issue.kind == IssueKind::Spelling)
                .map(|issue| (issue.start, issue.end, issue.lang, issue.message))
                .collect::<Vec<_>>()
        };
        assert!(on.iter().any(|issue| issue.kind == IssueKind::Spelling && &text[issue.start..issue.end] == "xyzxyz"));
        assert_eq!(spelling(on), spelling(off), "rejected agreement must not suppress spelling");
        let text = "Je est.";
        let off = check_text_with(text, &[(0..text.len(), lang)], &options, false, false);
        let spells_est = |issues: &[Issue]| issues.iter().any(|issue| issue.kind == IssueKind::Spelling && &text[issue.start..issue.end] == "est");
        assert!(spells_est(&off), "the explicit miniature dictionary excludes est");
        let on = check_text_with(text, &[(0..text.len(), lang)], &options, true, false);
        assert!(!syntactic(&on).is_empty());
        assert!(!spells_est(&on), "accepted agreement keeps its existing spelling priority");
        for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
            let spans = [(0..2, foreign), (2..text.len(), lang)];
            let on = check_text_with(text, &spans, &options, true, false);
            let off = check_text_with(text, &spans, &options, false, false);
            assert!(syntactic(&on).is_empty());
            assert!(spells_est(&on) && spells_est(&off), "rejected agreement must retain spelling inside its former replacement: {on:?}");
        }
        let text = "Nous a raison xyzxyz.";
        wordcraft_proof::add_word("xyzxyz");
        let same = check_text_with(text, &[(0..text.len(), lang)], &options, true, false);
        assert_eq!(syntactic(&same).len(), 1, "a user spelling entry must not hide agreement");
        assert!(!same.iter().any(|issue| issue.kind == IssueKind::Spelling && &text[issue.start..issue.end] == "xyzxyz"));
        assert!(wordcraft_proof::remove_word("xyzxyz"));
        for text in ["J’ai e\u{301}crire.", "J’ai e\u{301}crit."] {
            assert!(
                syntactic(&check_text_with(text, &[(0..text.len(), lang)], &options, true, false)).is_empty(),
                "unsupported decomposed forms stay unchanged"
            );
        }
    }
    // Original small records isolate an object cue that is required, and one
    // that is irrelevant. A foreign word after the minimal context must not
    // silence an otherwise supported rule.
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in [
        ("rentrer", "rentrer", "v1_it___zz infi"),
        ("rentre", "rentrer", "v1_it___zz ipre 1sg 3sg"),
        ("rentré", "rentrer", "v1_it___zz ppas mas sg"),
        ("classer", "classer", "v1__t___zz infi"),
        ("classe", "classer", "v1__t___zz ipre 1sg 3sg"),
        ("classé", "classer", "v1__t___zz ppas mas sg"),
    ]
    .into_iter()
    .enumerate()
    {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(folder.path().join(fr::morphology::FILE_NAME), table).unwrap();
    fr::morphology::reload();
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, object_decides) in [("J’ai rentrer les chaises.", true), ("J’ai classer les archives.", false), ("J’ai manger les pommes.", false)]
        {
            assert_eq!(syntactic(&check_text_with(text, &[(0..text.len(), lang)], &options, true, false)).len(), 1);
            let start = text.find("les").unwrap();
            let end = start + "les".len();
            for foreign in [ProofLang::Other, ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                let spans = [(0..start, lang), (start..end, foreign), (end..text.len(), lang)];
                let found = check_text_with(text, &spans, &options, true, false);
                assert_eq!(syntactic(&found).is_empty(), object_decides, "minimal lexical object context: {text}, {foreign:?}: {found:?}");
            }
        }
        for text in ["J’ai rentrer.", "J’ai rentré les chaises."] {
            assert!(syntactic(&check_text_with(text, &[(0..text.len(), lang)], &options, true, false)).is_empty());
        }
    }
    assert!(failures.is_empty(), "French syntax used a foreign deciding context in {} cases:\n{}", failures.len(), failures.join("\n"));
}
