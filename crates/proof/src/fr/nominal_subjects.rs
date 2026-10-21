//! Bounded agreement with a determiner-led common-noun subject.
//! This recognises sentence starts and one optional agreeing adjective, not clauses,
//! coordination, inversion, proper names, relative subjects or intervening clitics.
use std::ops::Range;

use super::{
    grammar,
    morphology::{Lexicon, Person},
};
use crate::Issue;

pub(super) const MESSAGE: &str = "Accord du verbe avec le groupe nominal sujet";

// Nominal epithets can label a group even when their written number is wrong.
// Their finite homographs cannot establish a verbal reading in this parser.
// Primary descriptions: Académie française, type (A9T2786), modèle (A9M2435);
// Canada's Clés de la rédaction, pilote, and Juridictionnaire, cadre.
const NOMINAL_EPITHETS: &[&str] = &["type", "types", "modèle", "modèles", "pilote", "pilotes", "cadre", "cadres"];

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

fn word<'a>(text: &'a str, ws: &[(usize, usize)], index: usize) -> Option<&'a str> {
    let &(start, end) = ws.get(index)?;
    text.get(start..end).filter(|w| w.len() <= 128)
}

fn lower_word<'a>(text: &'a str, ws: &[(usize, usize)], index: usize) -> Option<&'a str> {
    word(text, ws, index).filter(|w| !w.is_empty() && w.chars().all(char::is_lowercase))
}

fn adjacent(text: &str, ws: &[(usize, usize)], left: usize, right: usize) -> bool {
    let (Some(&(_, end)), Some(&(start, _))) = (ws.get(left), ws.get(right)) else {
        return false;
    };
    text.get(end..start).is_some_and(horizontal)
}

/// Reuse typography's abbreviation, number and dialogue checks, but accept only
/// bare sentence punctuation here. Quoted examples, brackets and dialogue stay out.
pub(super) fn sentence_starts(text: &str, ws: &[(usize, usize)]) -> Vec<usize> {
    if !super::conjugation::is_prose(text) || text.trim_start().starts_with(['—', '–', '-']) {
        return Vec::new();
    }
    let mut starts = Vec::new();
    if ws.first().is_some_and(|&(start, _)| text.get(..start).is_some_and(|prefix| prefix.trim().is_empty())) {
        starts.push(0);
    }
    for (offset, pair) in ws.windows(2).enumerate() {
        let [(start, end), (next_start, next_end)] = pair else {
            continue;
        };
        let (Some(prev), Some(gap), Some(next)) = (text.get(*start..*end), text.get(*end..*next_start), text.get(*next_start..*next_end)) else {
            continue;
        };
        if gap.chars().all(|c| c.is_whitespace() || matches!(c, '.' | '!' | '?'))
            && gap.chars().filter(|c| matches!(c, '.' | '!' | '?')).count() == 1
            && grammar::ends_sentence(gap, prev, next, false)
        {
            starts.push(offset.saturating_add(1));
        }
    }
    starts
}

fn determiner(word: &str) -> Option<Person> {
    if word.chars().skip(1).any(char::is_uppercase) {
        return None;
    }
    Some(match word.to_lowercase().as_str() {
        "le" | "la" | "un" | "une" | "ce" | "cet" | "cette" | "mon" | "ma" | "ton" | "ta" | "son" | "sa" | "notre" | "votre" | "leur" => {
            Person::ThirdSingular
        }
        "les" | "des" | "ces" | "mes" | "tes" | "ses" | "nos" | "vos" | "leurs" => Person::ThirdPlural,
        _ => return None,
    })
}

fn finite(lexicon: &Lexicon, form: &str) -> bool {
    [Person::FirstSingular, Person::SecondSingular, Person::ThirdSingular, Person::FirstPlural, Person::SecondPlural, Person::ThirdPlural]
        .into_iter()
        .any(|person| lexicon.has_person_reading(form, person))
}

/// A noun/adjective homograph can instead be an apposition or a product label.
/// Accept only a bare finite form or a complete short tail of these original
/// common-adverb facts. This is not a general part-of-speech parser. Every checked
/// adverb belongs to the deciding context; arbitrary noun/preposition tails stay quiet.
fn homograph_context_end(text: &str, ws: &[(usize, usize)], verb_index: usize) -> Option<usize> {
    let &(_, mut end) = ws.get(verb_index)?;
    for &(start, next_end) in ws.iter().skip(verb_index.saturating_add(1)).take(8) {
        let gap = text.get(end..start)?;
        if !horizontal(gap) {
            return gap.trim_start().starts_with(['.', '!', '?']).then_some(end);
        }
        let next = text.get(start..next_end)?;
        if next.len() > 128 {
            return None;
        }
        if !matches!(next, "dehors" | "ici" | "là" | "hier" | "demain" | "bientôt" | "souvent" | "toujours" | "rarement" | "vite" | "encore") {
            return None;
        }
        end = next_end;
    }
    let tail = text.get(end..)?;
    (tail.trim().is_empty() || tail.trim_start().starts_with(['.', '!', '?'])).then_some(end)
}

fn subject_verb(text: &str, ws: &[(usize, usize)], start: usize, lexicon: &Lexicon, target: Person) -> Option<usize> {
    let first = start.checked_add(1)?;
    let second = start.checked_add(2)?;
    let third = start.checked_add(3)?;
    if !adjacent(text, ws, start, first) {
        return None;
    }
    let a = lower_word(text, ws, first)?;
    if matches!(a, "mot" | "mots" | "verbe" | "verbes" | "forme" | "formes" | "exemple" | "exemples" | "infinitif" | "infinitifs") {
        return None;
    }
    let b = lower_word(text, ws, second)?;
    if !adjacent(text, ws, first, second) {
        return None;
    }
    // Prefer a complete longer nominal group, even when its verb is already
    // correct: "Les jeunes gardes surveillent" must not become "gardent".
    if let Some(c) = lower_word(text, ws, third)
        && adjacent(text, ws, second, third)
        && finite(lexicon, c)
        && ((lexicon.unique_noun_number(a) == Some(target) && lexicon.adjective_matches_number(b, target))
            || (lexicon.adjective_matches_number(a, target) && lexicon.unique_noun_number(b) == Some(target)))
    {
        return Some(third);
    }
    (lexicon.unique_noun_number(a) == Some(target) && finite(lexicon, b)).then_some(second)
}

pub(super) fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon) -> Vec<(Issue, Range<usize>)> {
    let mut issues = Vec::new();
    for start in sentence_starts(text, ws) {
        let Some(det) = word(text, ws, start) else {
            continue;
        };
        let Some(target) = determiner(det) else {
            continue;
        };
        let Some(verb_index) = subject_verb(text, ws, start, lexicon, target) else {
            continue;
        };
        let Some(form) = lower_word(text, ws, verb_index) else {
            continue;
        };
        // A matching nominal/adjectival reading licenses a label or apposition:
        // “Les enfants guides.” is not necessarily a clause with a wrong verb.
        if NOMINAL_EPITHETS.contains(&form) || lexicon.has_unknown_nominal_reading(form) {
            continue;
        }
        if lexicon.noun_matches_number(form, target) || lexicon.adjective_matches_number(form, target) {
            continue;
        }
        let Some(forms) = lexicon.agreement_forms(form, target) else {
            continue;
        };
        let (Some(&(subject_start, _)), Some(&(verb_start, verb_end))) = (ws.get(start), ws.get(verb_index)) else {
            continue;
        };
        let context_end = if lexicon.has_nonverbal_reading(form) {
            let Some(end) = homograph_context_end(text, ws, verb_index) else {
                continue;
            };
            end
        } else {
            verb_end
        };
        issues.push((grammar::issue(verb_start, verb_end, MESSAGE.into(), forms), subject_start..context_end));
    }
    issues
}

pub(super) fn check(text: &str, ws: &[(usize, usize)]) -> Vec<(Issue, Range<usize>)> {
    // Do not load optional data for unrelated prose.
    if !sentence_starts(text, ws).into_iter().any(|index| word(text, ws, index).and_then(determiner).is_some()) {
        return Vec::new();
    }
    check_with(text, ws, &super::morphology::lexicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Lexicon {
        let rows = [
            ("enfant", "enfant", "nom epi sg"),
            ("enfants", "enfant", "nom epi pl"),
            ("maison", "maison", "nom fem sg"),
            ("maisons", "maison", "nom fem pl"),
            ("petits", "petit", "adj mas pl"),
            ("petites", "petit", "adj fem pl"),
            ("jeunes", "jeune", "nom adj epi pl"),
            ("petits", "petit", "nom mas pl"),
            ("ris", "ris", "nom mas inv"),
            ("ris", "rire", "v3 ipre 1sg 2sg"),
            ("rient", "rire", "v3 ipre 3pl"),
            ("gardes", "garde", "nom epi pl"),
            ("guides", "guide", "nom epi pl"),
            ("rouges", "rouge", "adj epi pl"),
            ("garde", "garde", "nom mas"),
            ("garde", "garder", "v1 ipre 1sg 3sg"),
            ("pilote", "pilote", "adj mas"),
            ("pilote", "piloter", "v1 ipre 1sg 3sg"),
            ("pilotent", "piloter", "v1 ipre 3pl"),
            ("véhicules", "véhicule", "nom mas pl"),
            ("voitures", "voiture", "nom fem pl"),
            ("type", "type", "nom mas sg"),
            ("type", "typer", "v1 ipre 1sg 3sg"),
            ("typent", "typer", "v1 ipre 3pl"),
            ("modèle", "modèle", "nom adj epi sg"),
            ("modèle", "modeler", "v1 ipre 1sg 3sg"),
            ("modèlent", "modeler", "v1 ipre 3pl"),
            ("sport", "sport", "nom mas sg"),
            ("mot", "mot", "nom mas sg"),
            ("mots", "mot", "nom mas pl"),
            ("joue", "joue", "nom fem sg"),
            ("joue", "jouer", "v1 ipre 1sg 3sg"),
            ("joues", "jouer", "v1 ipre 2sg"),
            ("jouent", "jouer", "v1 ipre 3pl"),
            ("jouait", "jouer", "v1 iimp 3sg"),
            ("jouaient", "jouer", "v1 iimp 3pl"),
            ("jouer", "jouer", "v1 infi"),
            ("joué", "jouer", "v1 ppas adj mas sg"),
            ("tombe", "tomber", "v1 ipre 1sg 3sg"),
            ("tombent", "tomber", "v1 ipre 3pl"),
            ("gardes", "garder", "v1 ipre 2sg"),
            ("gardent", "garder", "v1 ipre 3pl"),
            ("guides", "guider", "v1 ipre 2sg"),
            ("guident", "guider", "v1 ipre 3pl"),
            ("surveille", "surveiller", "v1 ipre 1sg 3sg"),
            ("surveillent", "surveiller", "v1 ipre 3pl"),
            ("travaillent", "travailler", "v1 ipre 3pl"),
            ("arrivent", "arriver", "v1 ipre 3pl"),
            ("as", "avoir", "v0 ipre 2sg"),
            ("a", "avoir", "v0 ipre 3sg"),
            ("ont", "avoir", "v0 ipre 3pl"),
        ];
        let mut data = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
        for (index, (form, lemma, tags)) in rows.iter().enumerate() {
            data.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
        }
        Lexicon::from_reader(data.as_bytes()).unwrap()
    }

    fn found(text: &str, lexicon: &Lexicon) -> Vec<(Issue, Range<usize>)> {
        check_with(text, &crate::words(text), lexicon)
    }

    #[test]
    fn nominal_subjects_keep_recorded_person_tense_and_sentence_offsets() {
        let lexicon = fixture();
        for (text, wrong, right) in [
            ("Les enfants joue.", "joue", "jouent"),
            ("Les enfants joue dehors.", "joue", "jouent"),
            ("Les enfants joue souvent dehors.", "joue", "jouent"),
            ("Les enfants jouait.", "jouait", "jouaient"),
            ("Le enfant ont raison.", "ont", "a"),
            ("Les petits enfants joue.", "joue", "jouent"),
            ("Les petites maisons tombe.", "tombe", "tombent"),
            ("Les maisons rouges tombe.", "tombe", "tombent"),
            ("Bonjour. Les enfants joue.", "joue", "jouent"),
            ("Ça va. Les enfants joue dehors.", "joue", "jouent"),
            ("Les enfants joue. Les maisons tombe.", "joue", "jouent"),
        ] {
            let issues = found(text, &lexicon);
            let (issue, context) = issues.first().unwrap_or_else(|| panic!("{text}"));
            assert_eq!(&text[issue.start..issue.end], wrong, "{text}");
            assert_eq!(issue.suggestions, [right], "{text}");
            assert!(text[context.clone()].starts_with("Les") || text[context.clone()].starts_with("Le"), "{text}: {context:?}");
            assert!(context.start <= issue.start && context.end >= issue.end);
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(!found(&fixed, &lexicon).iter().any(|(next, _)| next.start == issue.start), "{fixed}");
        }
        let issues = found("Les enfants joue. Les maisons tombe.", &lexicon);
        assert_eq!(issues.len(), 2);
        let text = "Les enfants joue dehors.";
        assert_eq!(found(text, &lexicon)[0].1, 0..text.len() - 1, "homograph's tail is part of its deciding context");
    }

    #[test]
    fn nominal_subjects_preserve_nominal_readings_and_unparsed_constructions() {
        let lexicon = fixture();
        for text in [
            "Les enfants jouent.",
            "Le enfant joue.",
            "Les enfants jouer.",
            "Les enfants joué.",
            "Les enfants xyzxyz.",
            "Les enfants guides de montagne travaillent.",
            "Les enfants guides.",
            "Les enfants garde.",
            "Les enfants pilote.",
            "Les véhicules type sport.",
            "Les véhicules type de sport.",
            "Les voitures modèle sport.",
            "Les enfants joue un air.",
            "Les petits ris.",
            "Les jeunes gardes.",
            "Les enfants guides et parents arrivent.",
            "Les jeunes gardes surveillent.",
            "Les enfants guides et accompagnateurs arrivent.",
            "Les mots as et a sont courts.",
            "Je regarde les enfants joue.",
            "Les enfants qui joue arrivent.",
            "Les enfants de la maison joue.",
            "Les enfants, joue.",
            "Les enfants ne joue pas.",
            "Les enfants et la maison tombe.",
            "M. Les enfants joue.",
            "Etc. Les enfants joue.",
            "3. Les enfants joue.",
            "— Les enfants joue.",
            "« Les enfants joue. »",
            "Les enfants\njoue.",
            "Les Enfants joue.",
            "LES enfants joue.",
            "Les enfant joue.",
            "Les enfants joue comme un mot français.",
            "Les enfants joue dehors près de la maison près de la montagne aujourd’hui.",
        ] {
            assert!(found(text, &lexicon).is_empty(), "{text}: {:?}", found(text, &lexicon));
        }
        assert!(found("Les enfants joue.", &Lexicon::builtins()).is_empty(), "nominal facts require optional data");
        // A complete longer group must take precedence over its homographic inner noun.
        let issues = found("Les jeunes gardes surveille.", &lexicon);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].0.suggestions, ["surveillent"]);
    }
}
