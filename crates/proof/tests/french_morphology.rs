//! Separate-process gates for the optional data and its runtime cache.
#![cfg(not(target_arch = "wasm32"))]

use wordcraft_proof::{
    ProofLang,
    fr::{
        grammar,
        morphology::{self, Lexicon, Person},
    },
};

fn agreement(text: &str) -> Vec<wordcraft_proof::Issue> {
    grammar::check(text).into_iter().filter(|i| i.message.starts_with("Accord du verbe avec l’antécédent")).collect()
}

#[test]
fn optional_file_failure_fallback_precedence_and_reload() {
    let folder = tempfile::tempdir().unwrap();
    let managed = folder.path().join(morphology::MANAGED_DIRECTORY);
    std::fs::create_dir(&managed).unwrap();
    let data = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned()
        + &format!("1\t0\tmanges\tmanger\tv1 ipre 2sg{}\n2\t0\tmangeons\tmanger\tv1 ipre 1pl{}\n", "\t".repeat(15), "\t".repeat(15));
    let installed = managed.join(morphology::FILE_NAME);
    let manual = folder.path().join(morphology::FILE_NAME);
    std::fs::write(&installed, &data).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.path().to_owned()));
    assert_eq!(morphology::installed_path(), Some(installed.clone()));
    assert_eq!(morphology::data_status(), morphology::DataStatus::Ready { path: installed.clone(), analyses: 2 });
    assert_eq!(agreement("C’est nous qui manges.")[0].suggestions, ["mangeons"]);
    assert!(morphology::load_error().is_none());
    std::fs::write(&manual, "invalid file").unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.path().to_owned()));
    assert_eq!(morphology::installed_path(), Some(manual.clone()), "manual file wins, even when it needs repair");
    assert_eq!(agreement("C’est moi qui est ici.")[0].suggestions, ["suis"], "original builtin fallback remains available");
    assert!(agreement("C’est nous qui manges.").is_empty(), "do not invent unknown forms");
    assert_eq!(morphology::load_error().unwrap().0, manual);
    assert!(matches!(morphology::data_status(), morphology::DataStatus::Unreadable { path, .. } if path == manual));
    std::fs::write(&manual, data).unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.path().to_owned()));
    assert_eq!(agreement("C’est nous qui manges.")[0].suggestions, ["mangeons"]);
    assert!(morphology::load_error().is_none(), "a successful retry clears the diagnostic");
    assert_eq!(morphology::data_status(), morphology::DataStatus::Ready { path: manual, analyses: 2 });
    wordcraft_proof::set_dictionaries_folder(None);
}

#[test]
#[ignore = "requires the pinned external lexicon in WORDCRAFT_FRENCH_LEXICON"]
fn real_lexicon_preserves_persons_tenses_and_ambiguous_readings() {
    let path = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("set WORDCRAFT_FRENCH_LEXICON to the published 7.7 text file");
    let started = std::time::Instant::now();
    let lexicon = Lexicon::from_reader(std::fs::File::open(&path).unwrap()).unwrap();
    eprintln!("{} finite analyses loaded in {:?}", lexicon.len(), started.elapsed());
    assert!(lexicon.len() > 200_000, "real data, not the tiny fixture");
    for (wrong, target, expected) in [
        ("manges", Person::FirstPlural, "mangeons"),
        ("finissaient", Person::SecondPlural, "finissiez"),
        ("viendra", Person::SecondSingular, "viendras"),
        ("mangèrent", Person::FirstPlural, "mangeâmes"),
        ("eussent", Person::FirstPlural, "eussions"),
        ("serait", Person::FirstPlural, "serions"),
        ("soyez", Person::FirstSingular, "sois"),
    ] {
        let suggestions = lexicon.agreement_forms(wrong, target).unwrap();
        assert!(suggestions.iter().any(|s| s == expected), "{wrong} → {suggestions:?}, expected {expected}");
        assert!(lexicon.agreement_forms(expected, target).is_none(), "suggestions must be stable: {expected}");
    }
    for form in ["suis", "sommes", "portes"] {
        assert!(lexicon.agreement_forms(form, Person::SecondSingular).is_none(), "preserve a valid reading of {form}");
    }
    let folder = std::path::Path::new(&path).parent().unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.to_owned()));
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        let text = "C’est nous qui manges une pomme.";
        let issues = grammar::check_in(text, lang);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(&text[issues[0].start..issues[0].end], "manges");
        assert!(issues[0].suggestions.iter().any(|s| s == "mangeons"));
        // `manges` is also present subjunctive: keep that reading, rather than pretending
        // a lexical index alone can choose the mood. Both suggestions agree with `nous`.
        assert!(issues[0].suggestions.iter().all(|s| lexicon.agreement_forms(s, Person::FirstPlural).is_none()));
        assert_eq!(issues[0].lang, lang);
    }
    assert!(morphology::load_error().is_none());
}
