//! Two coordinated tonic pronouns, explicitly resumed by a matching plural subject.
//! This does not infer bare coordination, unquoted titles, clitics or negation.
use std::ops::Range;

use super::{
    morphology::{Lexicon, Person},
    subject_agreement,
};
use crate::Issue;

pub(super) const MESSAGE: &str = "Accord du verbe avec les sujets coordonnés";

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

fn word<'a>(text: &'a str, ws: &[(usize, usize)], index: usize) -> Option<&'a str> {
    let &(start, end) = ws.get(index)?;
    text.get(start..end).filter(|word| !word.is_empty() && word.len() <= 128)
}

fn person(word: &str) -> Option<Person> {
    Some(match word {
        "moi" | "nous" => Person::FirstPlural,
        "toi" | "vous" => Person::SecondPlural,
        "lui" | "elle" | "eux" | "elles" => Person::ThirdPlural,
        _ => return None,
    })
}

/// Return the explicit recap's original byte start after an exact two-pronoun head.
fn recap_start(text: &str, ws: &[(usize, usize)]) -> Option<usize> {
    if !super::conjugation::is_prose(text) {
        return None;
    }
    let &(start, first_end) = ws.first()?;
    if !text.get(..start)?.trim().is_empty() {
        return None;
    }
    let first = word(text, ws, 0)?;
    if first.len() > 16 || first.chars().skip(1).any(char::is_uppercase) {
        return None;
    }
    let first = first.to_lowercase();
    let first_person = person(&first)?;
    let &(et_start, et_end) = ws.get(1)?;
    let &(second_start, second_end) = ws.get(2)?;
    let &(recap_start, recap_end) = ws.get(3)?;
    let &(verb_start, _) = ws.get(4)?;
    if word(text, ws, 1)? != "et"
        || !horizontal(text.get(first_end..et_start)?)
        || !horizontal(text.get(et_end..second_start)?)
        || !text.get(second_end..recap_start)?.strip_prefix(',').is_some_and(horizontal)
        || !horizontal(text.get(recap_end..verb_start)?)
    {
        return None;
    }
    let second = word(text, ws, 2)?;
    if first == second {
        return None;
    }
    let second_person = person(second)?;
    let target = if first_person == Person::FirstPlural || second_person == Person::FirstPlural {
        Person::FirstPlural
    } else if first_person == Person::SecondPlural || second_person == Person::SecondPlural {
        Person::SecondPlural
    } else {
        Person::ThirdPlural
    };
    let recap = match word(text, ws, 3)? {
        "nous" => Person::FirstPlural,
        "vous" => Person::SecondPlural,
        "ils" | "elles" => Person::ThirdPlural,
        _ => return None,
    };
    if recap != target || !word(text, ws, 4)?.chars().all(char::is_lowercase) {
        return None;
    }
    Some(recap_start)
}

pub(super) fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon) -> Vec<(Issue, Range<usize>)> {
    let Some(recap) = recap_start(text, ws) else { return Vec::new() };
    let (Some(&(head_start, _)), Some(&(verb_start, verb_end)), Some(subject_words)) = (ws.first(), ws.get(4), ws.get(3..)) else {
        return Vec::new();
    };
    let Some(form) = word(text, ws, 4) else { return Vec::new() };
    // A finite/nominal or adjectival homograph can still introduce an apposition.
    // Decline it without inventing another following-context or valence heuristic.
    if lexicon.has_nonverbal_reading(form) {
        return Vec::new();
    }
    let Some(mut issue) = subject_agreement::check_at_subject(text, subject_words, recap, lexicon) else {
        return Vec::new();
    };
    // Only an immediately following, separate finite word belongs to this first increment.
    // The shared parser may understand wider clitic/negative prefixes; this rule does not.
    if issue.message != subject_agreement::MESSAGE || issue.start != verb_start || issue.end != verb_end {
        return Vec::new();
    }
    if text.get(head_start..verb_end).is_none() {
        return Vec::new();
    }
    issue.message = MESSAGE.into();
    vec![(issue, head_start..verb_end)]
}

pub(super) fn check(text: &str, ws: &[(usize, usize)]) -> Vec<(Issue, Range<usize>)> {
    // Keep optional data lazy for unrelated paragraphs.
    if recap_start(text, ws).is_none() {
        return Vec::new();
    }
    check_with(text, ws, &super::morphology::lexicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Lexicon {
        let rows = [
            ("parle", "parler", "v1 ipre 1sg 3sg"),
            ("parlons", "parler", "v1 ipre 1pl"),
            ("parlez", "parler", "v1 ipre 2pl"),
            ("parlent", "parler", "v1 ipre 3pl"),
            ("parlait", "parler", "v1 iimp 3sg"),
            ("parlions", "parler", "v1 iimp 1pl"),
            ("parliez", "parler", "v1 iimp 2pl"),
            ("parlaient", "parler", "v1 iimp 3pl"),
            ("guides", "guide", "nom epi pl"),
            ("guides", "guider", "v1 ipre 2sg"),
            ("guidons", "guider", "v1 ipre 1pl"),
        ];
        let mut table = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
        for (index, (form, lemma, tags)) in rows.iter().enumerate() {
            table.push_str(&format!("{}\t0\t{form}\t{lemma}\t{tags}{}\n", index + 1, "\t".repeat(15)));
        }
        Lexicon::from_reader(table.as_bytes()).unwrap()
    }

    #[test]
    fn recapped_coordinates_reuse_recorded_forms_and_original_offsets() {
        let lexicon = fixture();
        for (text, wrong, expected) in [
            ("Toi et moi, nous parlait.", "parlait", "parlions"),
            ("Lui et toi, vous parlait.", "parlait", "parliez"),
            ("Lui et elle, ils parle.", "parle", "parlent"),
            ("Elle et lui, elles parle.", "parle", "parlent"),
            ("Nous et vous, nous parle.", "parle", "parlons"),
            ("\u{a0}Toi\u{202f}et moi,\u{a0}nous parlait.", "parlait", "parlions"),
        ] {
            let found = check_with(text, &crate::words(text), &lexicon);
            assert_eq!(found.len(), 1, "{text}: {found:?}");
            let (issue, context) = &found[0];
            assert_eq!(text.get(issue.start..issue.end), Some(wrong));
            assert_eq!(issue.suggestions, [expected]);
            assert_eq!(context.start, text.find(|c: char| !c.is_whitespace()).unwrap());
            assert_eq!(context.end, issue.end);
            assert!(text.is_char_boundary(issue.start) && text.is_char_boundary(issue.end));
            assert!(text.is_char_boundary(context.start) && text.is_char_boundary(context.end));
        }
    }

    #[test]
    fn correct_titles_bare_coordination_and_unsupported_prefixes_stay_undecided() {
        let lexicon = fixture();
        for text in [
            "Toi et moi, nous parlons.",
            "Lui et toi, vous parliez.",
            "Lui et elle, ils parlent.",
            "Toi et moi parlons.",
            "Lui et elle parlent.",
            "Toi et moi est un film.",
            "Toi et moi raconte une histoire.",
            "Toi et moi, vous parle.",
            "Lui et elle, nous parle.",
            "Toi et moi nous parle.",
            "Toi, et moi, nous parle.",
            "Toi ou moi, nous parle.",
            "Ni toi ni moi, nous parle.",
            "Toi et moi et lui, nous parle.",
            "Toi et toi, vous parle.",
            "Toi et Marie, nous parle.",
            "Tu et moi, nous parle.",
            "Toi et moi, nous ne parle pas.",
            "Toi et moi, nous lui parle.",
            "Toi et moi, nous n’est pas prêts.",
            "Toi et moi, nous guides de montagne accompagnons ce groupe.",
            "« Toi et moi, nous parle. »",
            "Toi et moi, nous\nparle.",
            "Toi et moi, nous xyzxyz.",
            "Toi et moi, nous parler.",
            "",
            "Toi",
            "Toi et",
            "Toi et moi",
            "Toi et moi, nous",
        ] {
            assert!(check_with(text, &crate::words(text), &lexicon).is_empty(), "{text}");
        }
    }
}
