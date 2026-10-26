//! Original prefix-to-suffix continuation regressions; no reference-engine source copied.
use wordcraft_proof::affix::{AffixDict, MAX_FORMS, MAX_WORD_BYTES};

#[test]
fn prefix_continuations_require_both_cross_flags_and_one_prefix() {
    for prefix in ["0", "pre"] {
        for pcross in ["Y", "N"] {
            for scross in ["Y", "N"] {
                let aff = format!("SET UTF-8\nPFX P {pcross} 1\nPFX P 0 {prefix}/PS .\nSFX S {scross} 1\nSFX S 0 s/S .\n");
                let original = AffixDict::from_hunspell(&aff, "2\nroot/P\nplain\n", &[]).unwrap();
                let compact = AffixDict::from_bytes(&original.to_bytes()).unwrap();
                for d in [&original, &compact] {
                    let derived = if prefix == "0" { "roots" } else { "preroots" };
                    assert_eq!(d.contains(derived), pcross == "Y" && scross == "Y", "{aff}");
                    assert_eq!(d.forms("root").iter().any(|w| w == derived), pcross == "Y" && scross == "Y", "{aff}");
                    for word in ["prepreroots", "prerootss", "plains", "preplains"] {
                        assert!(!d.contains(word), "{aff}: {word}");
                    }
                    for word in d.forms("root") {
                        assert!(d.contains(&word), "generated form rejected: {aff}: {word}");
                    }
                }
            }
        }
    }
}

#[test]
fn prefix_continuations_keep_stem_restrictions_and_needaffix() {
    let aff = "SET UTF-8\nNEEDAFFIX N\nFORBIDDENWORD F\nNOSUGGEST X\nKEEPCASE K\nPFX P Y 1\nPFX P 0 pre/SN .\nSFX S Y 1\nSFX S 0 s .\n";
    let d = AffixDict::from_hunspell(aff, "5\nroot/P\nhidden/PX\nbanned/PF\nFixed/PK\nbound/PN\n", &[]).unwrap();
    for word in ["preroots", "prehiddens", "preFixeds", "prebounds"] {
        assert!(d.contains(word), "{word}");
    }
    for word in ["preroot", "roots", "prebanned", "prebanneds", "bound", "bounds"] {
        assert!(!d.contains(word), "{word}");
    }
    assert!(d.can_suggest("preroots"));
    assert!(d.check("PREHIDDENS") && !d.can_suggest("PREHIDDENS"));
    assert!(d.contains_suggestible("preroots") && !d.contains_suggestible("prehiddens"));
    assert!(!d.check("PREFIXEDS"), "KEEPCASE still forbids folding a derived form");
    assert!(d.forms("banned").is_empty());
    assert!(!d.forms("root").contains(&"preroot".into()), "NEEDAFFIX prefix alone is not a word");
}

#[test]
fn prefix_continuations_respect_unicode_strip_conditions_and_bounds() {
    let aff = "SET UTF-8\nFLAG long\nNEEDAFFIX N.\nPFX P. Y 1\nPFX P. é ré/S.N. é\nSFX S. Y 1\nSFX S. e és [^s]e\n";
    let d = AffixDict::from_hunspell(aff, "2\nétage/P.\ncage/P.\n", &[]).unwrap();
    assert!(d.contains("rétagés"));
    assert!(d.forms("étage").contains(&"rétagés".into()));
    for word in ["étagés", "rcagés", "récagés", "rérétagés", "rétagéss"] {
        assert!(!d.contains(word), "{word}");
    }
    assert!(!d.contains(&"é".repeat(MAX_WORD_BYTES)));
    let many = AffixDict::from_hunspell(
        "NEEDAFFIX N\nPFX P Y 1\nPFX P 0 pre/SN .\nSFX S Y 1\nSFX S 0 s .\n",
        &format!("1\n{}/P\n", "a".repeat(MAX_WORD_BYTES)),
        &[],
    )
    .unwrap();
    let forms = many.forms(&"a".repeat(MAX_WORD_BYTES));
    assert!(forms.len() <= MAX_FORMS);
    assert!(forms.iter().all(|w| w.len() <= MAX_WORD_BYTES));
}

#[test]
#[ignore = "requires the checksum-pinned French dictionary in WORDCRAFT_DICTIONARIES"]
fn real_french_metric_units_use_prefix_continuations() {
    let directory = std::env::var_os("WORDCRAFT_DICTIONARIES").expect("set WORDCRAFT_DICTIONARIES");
    let installed = wordcraft_proof::fr::installed_path().expect("real French dictionary required");
    assert_eq!(installed.parent(), Some(std::path::Path::new(&directory)));
    let d = wordcraft_proof::fr::dictionary();
    assert!(d.len() > 50_000);
    for word in ["grammes", "centimètres", "milligrammes", "nanomètres", "kilogrammes", "mètres"] {
        assert!(wordcraft_proof::fr::is_correct(word), "{word}");
    }
    for word in ["centtimètres", "grammess", "centimètress", "nanomètress"] {
        assert!(!wordcraft_proof::fr::is_correct(word), "{word}");
    }
}
