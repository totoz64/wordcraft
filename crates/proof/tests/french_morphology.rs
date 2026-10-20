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
    for (infinitive, expected) in
        [("écrire", "écrit"), ("classer", "classé"), ("numériser", "numérisé"), ("traduire", "traduit"), ("résoudre", "résolu")]
    {
        let forms = lexicon.participle_forms(infinitive, Person::FirstSingular, false, false).unwrap();
        assert!(forms.iter().any(|s| s == expected), "{infinitive}: {forms:?}");
        for form in forms {
            assert!(lexicon.is_past_participle(&form));
        }
    }
    // Data-path coverage and stable outputs, not a real-world precision estimate.
    use std::io::{BufRead, BufReader};
    let mut infinitives = std::collections::BTreeSet::new();
    for line in BufReader::new(std::fs::File::open(&path).unwrap()).lines() {
        let line = line.unwrap();
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() == 20
            && fields.first().is_some_and(|v| v.parse::<u32>().is_ok())
            && fields.get(4).is_some_and(|v| v.split_whitespace().any(|tag| tag == "infi"))
            && let Some(form) = fields.get(2)
            && form.chars().all(char::is_lowercase)
        {
            infinitives.insert((*form).to_owned());
        }
    }
    let folder = std::path::Path::new(&path).parent().unwrap();
    wordcraft_proof::set_dictionaries_folder(Some(folder.to_owned()));
    let mut covered = 0usize;
    for infinitive in infinitives {
        if let Some(forms) = lexicon.participle_forms(&infinitive, Person::FirstSingular, false, false) {
            covered += 1;
            assert!(!forms.is_empty());
            let tail = if matches!(infinitive.as_str(), "pouvoir" | "devoir") { " partir." } else { "." };
            let text = format!("J’ai {infinitive}{tail}");
            let issues = grammar::check(&text);
            assert_eq!(issues.len(), 1, "whole grammar path: {text}: {issues:?}");
            assert_eq!(&text[issues[0].start..issues[0].end], infinitive);
            assert!(issues[0].suggestions.iter().all(|s| lexicon.is_past_participle(s)), "{text}: {issues:?}");
            for suggestion in &issues[0].suggestions {
                let fixed = format!("{}{suggestion}{}", &text[..issues[0].start], &text[issues[0].end..]);
                assert!(grammar::check(&fixed).is_empty(), "stable grammar path: {fixed}");
            }
            for form in forms {
                assert!(lexicon.is_past_participle(&form), "{infinitive} → {form}");
                assert!(lexicon.participle_forms(&form, Person::FirstSingular, false, false).is_none(), "stable: {form}");
            }
        }
    }
    eprintln!("{covered} simple infinitives have a supported avoir/first-singular participle reading without an object cue");
    assert!(covered > 4_000, "exercise broad real lexical coverage, not a tiny table");
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, right) in [
            ("J’ai classer ces archives.", "classer", "classé"),
            ("J’avais numériser ces pages.", "numériser", "numérisé"),
            ("Tu aurais archiver ce dossier.", "archiver", "archivé"),
            ("Nous avons nettoyer ce local.", "nettoyer", "nettoyé"),
            ("Vous auriez traduire cette page.", "traduire", "traduit"),
            ("Elle aura peindre ces murs.", "peindre", "peint"),
            ("Il avait falloir agir.", "falloir", "fallu"),
            ("J’ai rentrer les chaises.", "rentrer", "rentré"),
            ("J’ai monter une étagère.", "monter", "monté"),
        ] {
            let found = grammar::check_in(text, lang);
            assert_eq!(found.len(), 1, "{text}: {found:?}");
            assert_eq!(&text[found[0].start..found[0].end], wrong);
            assert!(found[0].suggestions.iter().any(|s| s == right), "{text}: {found:?}");
            let fixed = format!("{}{right}{}", &text[..found[0].start], &text[found[0].end..]);
            assert!(grammar::check_in(&fixed, lang).is_empty(), "{fixed}");
        }
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
