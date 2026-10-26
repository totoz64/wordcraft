#![allow(dead_code)]
// Independent historical differential harness. The two production modules are
// fetched verbatim by prepare.py; neither is an imitation of the other algorithm.
mod patterns {
    pub type FxHasher = std::collections::hash_map::DefaultHasher;
    pub type FxMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;
}
mod historical;
mod current;

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn sorted(forms: Vec<String>) -> BTreeSet<String> { forms.into_iter().collect() }

    #[test]
    fn unchanged_legacy_language_on_original_generated_dictionaries() {
        let mut lookups = 0usize;
        let mut forms = 0usize;
        for variant in 0..64usize {
            let cont_a = if variant & 1 == 0 { "B" } else { "BC" };
            let cont_b = if variant & 2 == 0 { "C" } else { "AP" };
            let condition = if variant & 4 == 0 { "." } else { "[^x]" };
            let strip = if variant & 8 == 0 { "0" } else { "e" };
            let append = if variant & 16 == 0 { "x" } else { "é" };
            let cross = if variant & 32 == 0 { "Y" } else { "N" };
            let aff = format!("SET UTF-8\nFLAG UTF-8\nFORBIDDENWORD F\nNOSUGGEST N\nNEEDAFFIX !\nKEEPCASE K\nPFX P {cross} 1\nPFX P 0 pre .\nSFX A {cross} 3\nSFX A {strip} {append}/{cont_a} {condition}\nSFX A 0 {append}/C .\nSFX A 0 0/A .\nSFX B {cross} 2\nSFX B 0 z/{cont_b} .\nSFX B 0 z/C [exé]\nSFX C {cross} 1\nSFX C 0 s .\n");
            let dic = "7\nseed/AP\nreed/A\nforbid/AF\nquiet/AN\nneeds/A!\nCase/AK\ncafé/A\n";
            let old = historical::AffixDict::from_hunspell(&aff, dic, &[]).unwrap();
            let new = current::AffixDict::from_hunspell(&aff, dic, &[]).unwrap();
            let new_from_old = current::AffixDict::from_bytes(&old.to_bytes()).unwrap();
            let old_from_new = historical::AffixDict::from_bytes(&new.to_bytes()).unwrap();
            let stems = ["seed", "reed", "forbid", "quiet", "needs", "Case", "café", "absent"];
            let mut words = BTreeSet::new();
            for stem in stems {
                for prefix in ["", "pre", "prepre"] {
                    for suffix in ["", "x", "é", "z", "s", "xz", "éz", "xs", "és", "xzs", "ézs", "xx", "zz"] {
                        let word = format!("{prefix}{stem}{suffix}");
                        words.insert(word.clone());
                        words.insert(word.to_uppercase());
                        words.insert(format!("S{}", word.get(1..).unwrap_or("")));
                    }
                }
                // Exercise stripped forms as well as invalid truncations.
                words.insert(stem.trim_end_matches('e').to_owned());
                words.extend(old.forms(stem));
                words.extend(new.forms(stem));
                assert_eq!(sorted(old.forms(stem)), sorted(new.forms(stem)), "forms variant={variant} stem={stem}");
                forms += 1;
            }
            words.extend(["".into(), "🙂".into(), "é".repeat(80), "seedxzsx".into()]);
            for word in words {
                let expected = (old.contains(&word), old.check(&word), old.contains_suggestible(&word), old.can_suggest(&word));
                let actual = (new.contains(&word), new.check(&word), new.contains_suggestible(&word), new.can_suggest(&word));
                assert_eq!(expected, actual, "variant={variant} word={word:?}");
                assert_eq!(expected.1, new_from_old.check(&word), "legacy compact read {word:?}");
                assert_eq!(expected.1, old_from_new.check(&word), "reverse compact read {word:?}");
                lookups += 1;
            }
        }
        eprintln!("differential dictionaries=64, words={lookups}, form-set comparisons={forms}, recognition APIs=4, compact directions=2");
    }
}
