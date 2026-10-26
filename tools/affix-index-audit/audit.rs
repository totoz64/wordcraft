#![allow(dead_code)]
// Differential test of two actual WordCraft production modules fetched by prepare.py.
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

    fn initial(word: &str) -> String {
        let mut chars = word.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    }

    fn compare(aff: &str, dic: &str, stems: &[&str], label: &str) -> usize {
        let old = historical::AffixDict::from_hunspell(aff, dic, &[]).unwrap();
        let new = current::AffixDict::from_hunspell(aff, dic, &[]).unwrap();
        let new_from_old = current::AffixDict::from_bytes(&old.to_bytes()).unwrap();
        let old_from_new = historical::AffixDict::from_bytes(&new.to_bytes()).unwrap();
        let mut words = BTreeSet::new();
        for stem in stems {
            for prefix in ["", "pré", "re", "alt", "prépré", "repré"] {
                for suffix in ["", "s", "z", "x", "é", "ss", "sz", "zs", "sx", "zz", "szx", "z149", "z150", "z151"] {
                    let word = format!("{prefix}{stem}{suffix}");
                    words.insert(initial(&word));
                    words.insert(word.to_uppercase());
                    words.insert(word);
                }
            }
            words.extend(old.forms(stem));
            words.extend(new.forms(stem));
            let expected: BTreeSet<_> = old.forms(stem).into_iter().collect();
            let actual: BTreeSet<_> = new.forms(stem).into_iter().collect();
            assert_eq!(expected, actual, "form-set {label}, stem={stem}");
        }
        words.extend(["".into(), "é".repeat(80), "🙂".into(), "préépreuves".into(), "préépreuvess".into()]);
        let count = words.len();
        for word in words {
            let expected = (old.contains(&word), old.check(&word), old.contains_suggestible(&word), old.can_suggest(&word));
            let actual = (new.contains(&word), new.check(&word), new.contains_suggestible(&word), new.can_suggest(&word));
            assert_eq!(expected, actual, "recognition {label}, word={word:?}");
            assert_eq!(expected.1, new_from_old.check(&word), "old compact {label}, word={word:?}");
            assert_eq!(expected.1, old_from_new.check(&word), "new compact {label}, word={word:?}");
        }
        count
    }

    #[test]
    fn prefix_index_preserves_all_generated_partner_combinations() {
        let mut count = 0usize;
        for variant in 0..128usize {
            let p_cross = if variant & 1 == 0 { "Y" } else { "N" };
            let s_cross = if variant & 2 == 0 { "Y" } else { "N" };
            let p_need = if variant & 4 == 0 { "" } else { "!" };
            let s_need = if variant & 8 == 0 { "" } else { "/!" };
            let add = if variant & 16 == 0 { "0" } else { "pré" };
            let strip = if variant & 32 == 0 { "0" } else { "é" };
            let cond = if variant & 64 == 0 { "." } else { "[^z]" };
            // Two equal prefix texts grant different classes and NEEDAFFIX states.
            // A lookup index must retain per-rule conditions and partner restrictions.
            let aff = format!("SET UTF-8\nFLAG UTF-8\nFORBIDDENWORD F\nNOSUGGEST N\nKEEPCASE K\nNEEDAFFIX !\nPFX P {p_cross} 3\nPFX P {strip} {add}/A{p_need} {cond}\nPFX P {strip} {add}/B .\nPFX P 0 re/C .\nPFX Q Y 1\nPFX Q 0 alt/A .\nSFX A {s_cross} 2\nSFX A 0 s{s_need} .\nSFX A 0 x/B .\nSFX B {s_cross} 2\nSFX B 0 s{s_need} [^z]\nSFX B 0 z .\nSFX C Y 1\nSFX C 0 é .\nSFX D Y 1\nSFX D 0 s .\n");
            let dic = "7\népreuve/P\nseed/PD\nforbid/PF\nquiet/PN\nCase/PK\nneeds/P!\nz/P\n";
            count += compare(&aff, dic, &["épreuve", "seed", "forbid", "quiet", "Case", "needs", "z", "absent"], &format!("variant={variant}"));
        }
        eprintln!("prefix-index differential: dictionaries=128, word observations={count}, APIs=4, compact directions=2");
    }

    #[test]
    fn prefix_index_retains_numeric_high_flags_and_empty_lookup() {
        let mut aff = String::from("SET UTF-8\nFLAG num\nPFX 300 Y 2\nPFX 300 0 pré/150 .\nPFX 300 0 re/149 .\nPFX 301 Y 1\nPFX 301 0 alt/150 .\n");
        for flag in 1..=160 {
            aff.push_str(&format!("SFX {flag} Y 1\nSFX {flag} 0 z{flag} .\n"));
        }
        aff.push_str("SFX 400 Y 1\nSFX 400 0 s .\n");
        let dic = "2\nseed/300\nquiet/400\n";
        compare(&aff, dic, &["seed", "quiet", "absent"], "numeric-high-flags");
    }
}
