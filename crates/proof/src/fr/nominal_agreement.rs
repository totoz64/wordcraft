//! Bounded determiner, common-noun and adjective agreement from optional lexical facts.
//! No guessed endings, participial adjectives, complements or coordinated noun phrases.

use super::{
    grammar,
    morphology::{Lexicon, NOMINAL_FEMININE, NOMINAL_MASCULINE, NOMINAL_PLURAL, NOMINAL_SINGULAR, NominalPart},
};
use crate::Issue;
use std::ops::Range;

pub(super) const MESSAGE: &str = "Accord dans le groupe nominal";

// Their masculine singular spelling depends on position and the next sound.
// Missing variants in a custom file must not turn one spelling into a certain fix.
const POSITIONAL_ADJECTIVE_FORMS: &[&str] = &["beau", "bel", "vieux", "vieil", "nouveau", "nouvel", "fou", "fol", "mou", "mol"];

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

fn determiner(word: &str) -> Option<u8> {
    Some(match word {
        "le" | "un" | "ce" | "cet" => NOMINAL_MASCULINE & NOMINAL_SINGULAR,
        "la" | "une" | "cette" => NOMINAL_FEMININE & NOMINAL_SINGULAR,
        // Masculine possessives also occur before a feminine vowel-initial noun.
        "mon" | "ma" | "ton" | "ta" | "son" | "sa" | "notre" | "votre" | "leur" => NOMINAL_SINGULAR,
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => NOMINAL_PLURAL,
        _ => return None,
    })
}

fn determiner_gender_form(original: &str, next_word: &str, features: u8) -> Option<String> {
    let singular = features & NOMINAL_SINGULAR != 0 && features & NOMINAL_PLURAL == 0;
    let masculine = features & NOMINAL_MASCULINE != 0 && features & NOMINAL_FEMININE == 0;
    let feminine = features & NOMINAL_FEMININE != 0 && features & NOMINAL_MASCULINE == 0;
    if !singular || (!masculine && !feminine) {
        return None;
    }
    let lower = original.to_lowercase();
    let replacement = match lower.as_str() {
        "un" | "une" => {
            if masculine {
                "un"
            } else {
                "une"
            }
        }
        "le" | "la" => {
            // A word-only replacement cannot represent l’ + noun without its old space.
            if grammar::takes_elision(next_word) || next_word.starts_with('h') {
                return None;
            }
            if masculine { "le" } else { "la" }
        }
        "ce" | "cet" | "cette" => {
            if feminine {
                "cette"
            } else if grammar::takes_elision(next_word) {
                "cet"
            } else if next_word.starts_with('h') {
                return None;
            } else {
                "ce"
            }
        }
        _ => return None,
    };
    if lower == replacement {
        return None;
    }
    Some(if original.chars().next().is_some_and(char::is_uppercase) {
        replacement.chars().next()?.to_uppercase().chain(replacement.chars().skip(1)).collect()
    } else {
        replacement.to_owned()
    })
}

fn lexical_word<'a>(text: &'a str, ws: &[(usize, usize)], index: usize) -> Option<&'a str> {
    let &(a, b) = ws.get(index)?;
    let word = text.get(a..b)?;
    (word.len() <= 128 && word.chars().all(char::is_lowercase)).then_some(word)
}

fn connected(text: &str, ws: &[(usize, usize)], a: usize, b: usize) -> bool {
    let (Some(&(_, end)), Some(&(start, _))) = (ws.get(a), ws.get(b)) else {
        return false;
    };
    text.get(end..start).is_some_and(horizontal)
}

fn finite(lexicon: &Lexicon, form: &str) -> bool {
    [
        super::morphology::Person::FirstSingular,
        super::morphology::Person::SecondSingular,
        super::morphology::Person::ThirdSingular,
        super::morphology::Person::FirstPlural,
        super::morphology::Person::SecondPlural,
        super::morphology::Person::ThirdPlural,
    ]
    .into_iter()
    .any(|person| lexicon.has_person_reading(form, person))
}

fn positional_modifier(word: &str) -> bool {
    // Their position or an adverbial reading can preserve an uninflected surface.
    // Hyphen repair and these exceptional constructions are outside this parser.
    matches!(
        word,
        "demi"
            | "demie"
            | "demis"
            | "demies"
            | "nu"
            | "nue"
            | "nus"
            | "nues"
            | "feu"
            | "feue"
            | "feus"
            | "feues"
            | "fort"
            | "tout"
            | "haut"
            | "nouveau"
    )
}

fn exceptional_noun(word: &str) -> bool {
    // These have agreement dependent on position, register or meaning beyond this parser.
    matches!(word, "gens" | "amour" | "amours" | "délice" | "délices" | "orgue" | "orgues")
}

fn push(
    text: &str,
    ws: &[(usize, usize)],
    start: usize,
    end: usize,
    changed: usize,
    suggestions: Vec<String>,
    issues: &mut Vec<(Issue, Range<usize>)>,
) {
    let (Some(&(context_start, _)), Some(&(_, context_end)), Some(&(a, b))) = (ws.get(start), ws.get(end), ws.get(changed)) else {
        return;
    };
    if text.get(a..b).is_some() && text.get(context_start..context_end).is_some() {
        issues.push((grammar::issue(a, b, MESSAGE.into(), suggestions), context_start..context_end));
    }
}

pub(super) fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon) -> Vec<(Issue, Range<usize>)> {
    let mut issues = Vec::new();
    if !super::conjugation::is_prose(text)
        || ws.iter().any(|&(a, b)| {
            text.get(a..b).is_some_and(|word| {
                word.len() <= 32
                    && matches!(word.to_lowercase().as_str(), "mot" | "mots" | "verbe" | "verbes" | "forme" | "formes" | "exemple" | "exemples")
            })
        })
    {
        return issues;
    }
    for start in super::nominal_subjects::sentence_starts(text, ws) {
        let Some(&(a, b)) = ws.get(start) else {
            continue;
        };
        let Some(first) = text.get(a..b) else {
            continue;
        };
        if first.len() > 32 || first.chars().skip(1).any(char::is_uppercase) {
            continue;
        }
        let Some(det) = determiner(&first.to_lowercase()) else {
            continue;
        };
        let one = start.saturating_add(1);
        let two = start.saturating_add(2);
        let three = start.saturating_add(3);
        let Some(first_word) = lexical_word(text, ws, one) else {
            continue;
        };
        if !connected(text, ws, start, one) {
            continue;
        }
        // With a plural determiner these forms retain a cardinal reading, even
        // before a substantivised adjective: les neuf petites, ces mille petits.
        if det & NOMINAL_PLURAL != 0 && matches!(first_word, "cent" | "mil" | "mille" | "neuf" | "zéro") {
            continue;
        }
        let second = lexical_word(text, ws, two).filter(|_| connected(text, ws, one, two));
        if positional_modifier(first_word) || second.is_some_and(positional_modifier) {
            continue;
        }
        let preposed = lexicon.nominal_features(first_word, NominalPart::Adjective).is_some()
            && second.is_some_and(|word| {
                lexicon.nominal_features(word, NominalPart::Noun).is_some() && lexicon.nominal_features(word, NominalPart::Adjective).is_none()
            });
        // The first nominal/adjectival word can already be the valid subject:
        // La vieille + dit is not necessarily La + vieille + the noun dit.
        if lexicon.nominal_features(first_word, NominalPart::Noun).is_some_and(|features| features & det != 0)
            && second.is_some_and(|word| finite(lexicon, word))
        {
            continue;
        }
        // Two bare nouns can form a designation, or the first word can have a
        // cardinal reading (mille + toits). Do not infer a lone head from that.
        if !preposed
            && second.is_some_and(|word| {
                lexicon.nominal_features(word, NominalPart::Noun).is_some() && lexicon.nominal_features(word, NominalPart::Adjective).is_none()
            })
        {
            continue;
        }
        let (head_index, head, adjective) = if preposed {
            let Some(head) = second else { continue };
            (two, head, Some((one, first_word)))
        } else if lexicon.nominal_features(first_word, NominalPart::Noun).is_some() {
            let adjective = second.filter(|word| lexicon.nominal_features(word, NominalPart::Adjective).is_some()).map(|word| (two, word));
            (one, first_word, adjective)
        } else {
            continue;
        };
        let mut deciding_end = adjective.map_or(head_index, |(index, _)| head_index.max(index));
        if exceptional_noun(head) {
            continue;
        }
        if adjective.is_none() {
            let Some(&(_, end)) = ws.get(head_index) else {
                continue;
            };
            if text.get(end..).is_some_and(|tail| tail.trim_start().starts_with([',', ';', ':', '(', ')', '[', ']', '—', '–'])) {
                continue;
            }
        }
        if adjective.is_some_and(|(index, _)| index == two) && connected(text, ws, two, three) {
            // A following proper name, code or unknown complement may form a colour
            // or designation (bleu + name, modèle + code), never an inflected adjective.
            let Some(next) = lexical_word(text, ws, three) else {
                continue;
            };
            if !finite(lexicon, next) {
                continue;
            }
            // This finite word licenses the postposed-adjective interpretation.
            // Its bytes and the gap before it are part of the deciding context.
            deciding_end = three;
        }
        // A further noun/adjective can be a compound colour or a more complex group.
        if adjective.is_some()
            && connected(text, ws, two, three)
            && lexical_word(text, ws, three).is_some_and(|word| {
                lexicon.nominal_features(word, NominalPart::Adjective).is_some() || lexicon.nominal_features(word, NominalPart::Noun).is_some()
            })
        {
            continue;
        }
        let Some(head_features) = lexicon.nominal_features(head, NominalPart::Noun) else {
            continue;
        };
        let compatible = head_features & det;
        if compatible == 0 {
            // The written determiner supplies number, while the noun retains its gender.
            // A singular gender conflict uses only the supported determiner paradigms.
            let genders = (if head_features & NOMINAL_MASCULINE != 0 { NOMINAL_MASCULINE } else { 0 })
                | (if head_features & NOMINAL_FEMININE != 0 { NOMINAL_FEMININE } else { 0 });
            let target = det & genders;
            if let Some(form) = determiner_gender_form(first, first_word, head_features) {
                push(text, ws, start, deciding_end, start, vec![form], &mut issues);
            } else if let Some(forms) = lexicon.nominal_forms(head, NominalPart::Noun, target) {
                push(text, ws, start, deciding_end, head_index, forms, &mut issues);
            }
        } else if let Some((index, adjective)) = adjective {
            // Both independent anchors already agree before correcting the modifier.
            if let Some(forms) = lexicon.nominal_forms(adjective, NominalPart::Adjective, compatible)
                && forms.len() == 1
                && !forms.iter().any(|form| POSITIONAL_ADJECTIVE_FORMS.contains(&form.as_str()))
            {
                push(text, ws, start, deciding_end, index, forms, &mut issues);
            }
        }
    }
    issues
}

pub(super) fn check(text: &str, ws: &[(usize, usize)]) -> Vec<(Issue, Range<usize>)> {
    if !super::conjugation::is_prose(text)
        || !super::nominal_subjects::sentence_starts(text, ws).iter().any(|&index| {
            ws.get(index).and_then(|&(a, b)| text.get(a..b)).is_some_and(|word| word.len() <= 32 && determiner(&word.to_lowercase()).is_some())
        })
    {
        return Vec::new();
    }
    check_with(text, ws, &super::morphology::lexicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexicon() -> Lexicon {
        let mut data = String::from("id\tX\tFlexion\tLemme\tÉtiquettes\tA\tB\tC\tD\tE\tF\tG\tH\tI\tJ\tK\tL\tM\tN\tO\n");
        for (form, lemma, tag) in [
            ("joue", "jouer", "v1_it____a ipre 3sg"),
            ("jouent", "jouer", "v1_it____a ipre 3pl"),
            ("maison", "maison", "nom fem sg"),
            ("maisons", "maison", "nom fem pl"),
            ("jardin", "jardin", "nom mas sg"),
            ("jardins", "jardin", "nom mas pl"),
            ("petit", "petit", "nom adj mas sg"),
            ("petite", "petit", "nom adj fem sg"),
            ("petits", "petit", "nom adj mas pl"),
            ("petites", "petit", "nom adj fem pl"),
            ("robe", "robe", "nom fem sg"),
            ("robes", "robe", "nom fem pl"),
            ("bleu", "bleu", "adj mas sg"),
            ("bleues", "bleu", "adj fem pl"),
            ("clair", "clair", "adj mas sg"),
            ("ciel", "ciel", "nom mas sg"),
            ("orange", "orange", "adj epi inv"),
            ("gens", "gens", "nom epi pl"),
            ("amours", "amour", "nom epi pl"),
            ("arbre", "arbre", "nom mas sg"),
            ("amie", "ami", "nom fem sg"),
            ("demi", "demi", "nom adj mas sg"),
            ("demie", "demi", "nom adj fem sg"),
            ("demies", "demi", "nom adj fem pl"),
            ("heure", "heure", "nom fem sg"),
            ("heures", "heure", "nom fem pl"),
            ("nu", "nu", "adj mas sg"),
            ("nue", "nu", "adj fem sg"),
            ("feu", "feu", "adj mas sg"),
            ("feue", "feu", "adj fem sg"),
            ("neuf", "neuf", "nb epi pl"),
            ("neuf", "neuf", "nom adj mas sg"),
            ("neuve", "neuf", "nom adj fem sg"),
            ("neufs", "neuf", "nom adj mas pl"),
            ("neuves", "neuf", "nom adj fem pl"),
            ("cent", "cent", "nom mas sg"),
            ("cents", "cent", "nom mas pl"),
            ("mil", "mil", "nom mas sg"),
            ("mils", "mil", "nom mas pl"),
            ("zéro", "zéro", "nom mas sg"),
            ("zéros", "zéro", "nom mas pl"),
            ("vieille", "vieux", "nom adj fem sg"),
            ("dit", "dire", "v3 ipre 3sg"),
            ("dit", "dit", "nom mas sg"),
            ("dits", "dit", "nom mas pl"),
            ("mille", "mille", "nb epi pl"),
            ("mille", "mille", "nom mas sg"),
            ("milles", "mille", "nom mas pl"),
            ("livre", "livre", "nom epi sg"),
            ("livres", "livre", "nom epi pl"),
        ] {
            data.push_str(&format!("0\t0\t{form}\t{lemma}\t{tag}{}\n", "\t".repeat(15)));
        }
        Lexicon::from_reader(data.as_bytes()).unwrap()
    }

    #[test]
    fn nominal_adjective_variants_do_not_guess_position_or_incomplete_data() {
        for include_bel in [true, false] {
            let mut rows = vec![
                ("joue", "jouer", "v1 ipre 3sg"),
                ("maison", "maison", "nom fem sg"),
                ("jardin", "jardin", "nom mas sg"),
                ("arbre", "arbre", "nom mas sg"),
                ("beau", "beau", "nom adj mas sg"),
                ("belle", "beau", "nom adj fem sg"),
            ];
            if include_bel {
                rows.push(("bel", "beau", "nom adj mas sg"));
            }
            let mut data = String::from("id\tX\tFlexion\tLemme\tÉtiquettes\tA\tB\tC\tD\tE\tF\tG\tH\tI\tJ\tK\tL\tM\tN\tO\n");
            for (index, (form, lemma, tags)) in rows.iter().enumerate() {
                data.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
            }
            let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
            for text in ["Un belle arbre.", "Un belle jardin.", "Le jardin belle."] {
                assert!(check_with(text, &crate::words(text), &lexicon).is_empty(), "{include_bel}: {text}");
            }
            let text = "Une beau maison.";
            let found = check_with(text, &crate::words(text), &lexicon);
            assert_eq!(found.len(), 1);
            assert_eq!(&text[found[0].0.start..found[0].0.end], "beau");
            assert_eq!(found[0].0.suggestions, ["belle"]);
        }
    }

    #[test]
    fn nominal_agreement_uses_independent_anchors_and_attested_paradigms() {
        let lexicon = lexicon();
        for (text, wrong, right) in [
            ("Les petite maisons.", "petite", "petites"),
            ("Les maisons petite.", "petite", "petites"),
            ("Une petit maison.", "petit", "petite"),
            ("Les maison.", "maison", "maisons"),
            ("Une maisons.", "maisons", "maison"),
            ("Un maison.", "Un", "Une"),
            ("La jardin.", "La", "Le"),
            ("Cette arbre.", "Cette", "Cet"),
            ("Bonjour. Les petite maisons.", "petite", "petites"),
            ("Les\u{a0}petite\u{202f}maisons.", "petite", "petites"),
        ] {
            let issues = check_with(text, &crate::words(text), &lexicon);
            let start = text.find(wrong).unwrap();
            assert!(
                issues.iter().any(|(issue, context)| issue.start == start
                    && issue.end == start + wrong.len()
                    && issue.suggestions.iter().any(|s| s == right)
                    && context.start <= start
                    && context.end >= issue.end),
                "{text}: {issues:?}"
            );
        }
        for text in [
            "Les robes bleu Zora.",
            "Les robes bleu Q.",
            "Les robes bleu 2.",
            "Une demi heure.",
            "Les demi heures.",
            "Une nu maison.",
            "Une feu maison.",
            "Les petites maisons.",
            "Les maisons petites.",
            "Une petite maison.",
            "Les livres.",
            "La livre.",
            "Les robes orange.",
            "Les robes bleu clair.",
            "Les robes bleu ciel.",
            "Les petite gens.",
            "Les petites amours.",
            "Mon amie.",
            "Le amie.",
            "La arbre.",
            "Les inconnue maisons.",
            "Les petite, maisons.",
            "Les petite\nmaisons.",
            "« Les petite maisons »",
            "Les petite maisons est un exemple.",
        ] {
            assert!(check_with(text, &crate::words(text), &lexicon).is_empty(), "{text}");
        }
        assert_eq!(lexicon.len(), 3, "nominal rows never become finite analyses");
        assert_eq!(lexicon.unique_noun_number("maisons"), Some(super::super::morphology::Person::ThirdPlural));
        assert!(lexicon.adjective_matches_number("orange", super::super::morphology::Person::ThirdPlural));
        assert!(check_with("Les petite maisons.", &crate::words("Les petite maisons."), &Lexicon::builtins()).is_empty());
    }
    #[test]
    fn nominal_agreement_preserves_subject_and_unparsed_number_continuations() {
        let lexicon = lexicon();
        for text in ["La vieille dit bonjour.", "La vieille dit.", "Les petits dit.", "Ces mille maisons.", "Ces mille jardins."] {
            assert!(check_with(text, &crate::words(text), &lexicon).is_empty(), "{text}");
        }
        let core = "Les petite maisons.";
        assert!(check_with(core, &crate::words(core), &lexicon).iter().any(|(issue, _)| issue.suggestions.iter().any(|form| form == "petites")));
    }
    #[test]
    fn nominal_agreement_preserves_plural_cardinal_readings() {
        let lexicon = lexicon();
        for text in ["Les neuf maisons.", "Les neuf petites.", "Ces mille petits.", "Ces cent petits.", "Les mil petits.", "Les zéro petits."] {
            assert!(check_with(text, &crate::words(text), &lexicon).is_empty(), "{text}");
        }
        let singular = "Une neuf maison.";
        let issues = check_with(singular, &crate::words(singular), &lexicon);
        assert!(issues.iter().any(|(issue, _)| issue.start == 4 && issue.end == 8 && issue.suggestions == ["neuve"]), "{issues:?}");
    }
}
