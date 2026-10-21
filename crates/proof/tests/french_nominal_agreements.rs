//! Original minimal nominal agreement regression through the public proof API.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{IssueKind, Options, ProofLang, check_text_with};

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordcraft-nominal-public-{}", std::process::id()));
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

#[test]
fn nominal_group_and_nominal_subject_agreements_use_recorded_forms() {
    let scratch = Scratch::new();
    let rows = [
        ("maison", "maison", "nom fem sg"),
        ("maisons", "maison", "nom fem pl"),
        ("petit", "petit", "nom adj mas sg"),
        ("petite", "petit", "nom adj fem sg"),
        ("petits", "petit", "nom adj mas pl"),
        ("petites", "petit", "nom adj fem pl"),
        ("enfant", "enfant", "nom epi sg"),
        ("enfants", "enfant", "nom epi pl"),
        ("joue", "joue", "nom fem sg"),
        ("joue", "jouer", "v1_itnq__a ipre spre 1sg 3sg"),
        ("jouent", "jouer", "v1_itnq__a ipre spre 3pl"),
    ];
    let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
    for (index, (form, lemma, tags)) in rows.iter().enumerate() {
        table.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
    }
    std::fs::write(scratch.0.join(wordcraft_proof::fr::morphology::FILE_NAME), table).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(scratch.0.clone()));
    wordcraft_proof::fr::install_dictionary("SET UTF-8\n", "8\nles\npetite\npetites\nmaisons\nenfants\njoue\njouent\nmaison\n").unwrap();
    let mut failures = Vec::new();
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, replacement, prefix) in [
            ("Les petite maisons.", "petite", "petites", "Accord dans le groupe nominal"),
            ("Les enfants joue.", "joue", "jouent", "Accord du verbe avec le groupe nominal sujet"),
        ] {
            let issues = check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false);
            let start = text.find(wrong).unwrap();
            let matching: Vec<_> = issues
                .iter()
                .filter(|issue| issue.kind == IssueKind::Grammar && issue.lang == language && issue.message.starts_with(prefix))
                .collect();
            if matching.len() != 1
                || !matching.first().is_some_and(|issue| {
                    issue.start == start && issue.end == start + wrong.len() && issue.suggestions.iter().any(|suggestion| suggestion == replacement)
                })
            {
                failures.push(format!("{language:?}: {text}: expected {wrong} -> {replacement}; actual {issues:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
