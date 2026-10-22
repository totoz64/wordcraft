//! Bounded agreement with a determiner-led common-noun subject.
//! This recognises sentence starts and one optional agreeing adjective, not clauses,
//! coordination, inversion, proper names or relative subjects. One non-reflexive
//! lui/leur/y/en clitic and explicit ne/n’ + negative marker are recognised.
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

struct Predicate<'a> {
    index: usize,
    start: usize,
    end: usize,
    form: &'a str,
    clitic: Option<&'a str>,
    negative_marker: Option<usize>,
}

fn negative_marker(form: &str) -> bool {
    matches!(form, "pas" | "plus" | "jamais" | "guère" | "point")
}

/// A single explicit negative construction is a bounded bridge, never arbitrary
/// skipped words. Elision and the following marker are part of the deciding range.
fn predicate<'a>(text: &'a str, ws: &[(usize, usize)], index: usize, lexicon: &Lexicon) -> Option<Predicate<'a>> {
    let &(start, end) = ws.get(index)?;
    let token = word(text, ws, index)?;
    let (mut index, mut end, mut form, negative) = if token == "ne" {
        let next = index.checked_add(1)?;
        if !adjacent(text, ws, index, next) {
            return None;
        }
        let &(_, end) = ws.get(next)?;
        (next, end, lower_word(text, ws, next)?, true)
    } else if let Some(form) = token.strip_prefix("n’").or_else(|| token.strip_prefix("n'")) {
        if form.is_empty() || !form.chars().all(char::is_lowercase) {
            return None;
        }
        (index, end, form, true)
    } else {
        (index, end, lower_word(text, ws, index)?, false)
    };
    let clitic = if matches!(form, "lui" | "leur" | "y" | "en") {
        let clitic = form;
        let next = index.checked_add(1)?;
        if !adjacent(text, ws, index, next) {
            return None;
        }
        let &(_, next_end) = ws.get(next)?;
        form = lower_word(text, ws, next)?;
        index = next;
        end = next_end;
        Some(clitic)
    } else {
        None
    };
    // En can be a preposition ("Les robes en voile"), leur a possessive
    // determiner. Do not infer a verb from any ambiguous nominal/adjectival form.
    if !finite(lexicon, form) || (clitic.is_some() && lexicon.has_nonverbal_reading(form)) {
        return None;
    }
    let negative_marker = if negative {
        let marker = index.checked_add(1)?;
        if !adjacent(text, ws, index, marker) || !negative_marker(lower_word(text, ws, marker)?) {
            return None;
        }
        let after = marker.checked_add(1)?;
        if adjacent(text, ws, marker, after) && lower_word(text, ws, after).is_some_and(negative_marker) {
            return None;
        }
        Some(marker)
    } else {
        let next = index.checked_add(1)?;
        if clitic.is_some() && adjacent(text, ws, index, next) && lower_word(text, ws, next).is_some_and(negative_marker) {
            return None; // A bare adjacent marker does not establish an explicit negative bridge.
        }
        None
    };
    Some(Predicate { index, start, end, form, clitic, negative_marker })
}

fn subject_verb<'a>(text: &'a str, ws: &[(usize, usize)], start: usize, lexicon: &Lexicon, target: Person) -> Option<Predicate<'a>> {
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
    let b = word(text, ws, second)?;
    if !adjacent(text, ws, first, second) {
        return None;
    }
    // Prefer a complete longer nominal group, even when its verb is already
    // correct: "Les jeunes gardes surveillent" must not become "gardent".
    if adjacent(text, ws, second, third)
        && ((lexicon.unique_noun_number(a) == Some(target) && lexicon.adjective_matches_number(b, target))
            || (lexicon.adjective_matches_number(a, target) && lexicon.unique_noun_number(b) == Some(target)))
        && let Some(predicate) = predicate(text, ws, third, lexicon)
    {
        return Some(predicate);
    }
    if lexicon.unique_noun_number(a) == Some(target) { predicate(text, ws, second, lexicon) } else { None }
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
        let Some(predicate) = subject_verb(text, ws, start, lexicon, target) else {
            continue;
        };
        let form = predicate.form;
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
        let forms = if predicate.negative_marker.is_some() || predicate.clitic.is_some() {
            let Some(rendered) = forms
                .into_iter()
                .map(|form| {
                    let first = predicate.clitic.unwrap_or(&form);
                    let phrase = predicate.clitic.map_or_else(|| form.clone(), |clitic| format!("{clitic} {form}"));
                    if predicate.negative_marker.is_some() {
                        super::subject_agreement::known_elision(first)
                            .map(|elides| if elides { format!("n’{phrase}") } else { format!("ne {phrase}") })
                    } else {
                        Some(phrase)
                    }
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            rendered
        } else {
            forms
        };
        let Some(&(subject_start, _)) = ws.get(start) else {
            continue;
        };
        let deciding_index = predicate.negative_marker.unwrap_or(predicate.index);
        let context_end = if lexicon.has_nonverbal_reading(form) {
            let Some(end) = homograph_context_end(text, ws, deciding_index) else {
                continue;
            };
            end
        } else {
            let Some(&(_, end)) = ws.get(deciding_index) else {
                continue;
            };
            end
        };
        issues.push((grammar::issue(predicate.start, predicate.end, MESSAGE.into(), forms), subject_start..context_end));
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
            ("parle", "parler", "v1 ipre 1sg 3sg"),
            ("parlent", "parler", "v1 ipre 3pl"),
            ("parlant", "parler", "v1 ppre"),
            ("parlait", "parler", "v1 iimp 1sg 3sg"),
            ("parlaient", "parler", "v1 iimp 3pl"),
            ("répond", "répondre", "v3 ipre 3sg"),
            ("répondent", "répondre", "v3 ipre 3pl"),
            ("va", "aller", "v3 ipre 3sg"),
            ("vont", "aller", "v3 ipre 3pl"),
            ("route", "route", "nom fem sg"),
            ("route", "router", "v1 ipre 1sg 3sg"),
            ("routent", "router", "v1 ipre 3pl"),
            ("enfant", "enfant", "nom epi sg"),
            ("petit", "petit", "adj mas sg"),
            ("est", "être", "v0 ipre 3sg"),
            ("sont", "être", "v0 ipre 3pl"),
            ("harcèle", "harceler", "v1 ipre 3sg"),
            ("harcèlent", "harceler", "v1 ipre 3pl"),
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

    #[test]
    fn nominal_subject_negation_preserves_elision_marker_and_recorded_tense() {
        let lexicon = fixture();
        for (text, wrong, right) in [
            ("Les enfants ne joue pas.", "ne joue", "ne jouent"),
            ("Les enfants ne joue plus.", "ne joue", "ne jouent"),
            ("Les enfants ne joue jamais.", "ne joue", "ne jouent"),
            ("Les enfants ne joue guère.", "ne joue", "ne jouent"),
            ("Les enfants ne joue point.", "ne joue", "ne jouent"),
            ("Les enfants ne joue pas dehors.", "ne joue", "ne jouent"),
            ("Les enfants ne jouait pas.", "ne jouait", "ne jouaient"),
            ("Le enfant ne jouaient pas.", "ne jouaient", "ne jouait"),
            ("Les petits enfants ne joue pas.", "ne joue", "ne jouent"),
            ("Les maisons rouges ne tombe pas.", "ne tombe", "ne tombent"),
            ("Les enfants n’est pas.", "n’est", "ne sont"),
            ("Les enfants n'est pas.", "n'est", "ne sont"),
            ("Le petit enfant ne sont pas.", "ne sont", "n’est"),
            ("Les enfants n’a pas.", "n’a", "n’ont"),
            ("Le petit enfant n'ont pas.", "n'ont", "n’a"),
            ("Bonjour. Les enfants ne joue pas.", "ne joue", "ne jouent"),
            ("Les\u{a0}enfants\u{202f}ne\u{a0}joue\u{202f}pas.", "ne\u{a0}joue", "ne jouent"),
        ] {
            let issues = found(text, &lexicon);
            assert_eq!(issues.len(), 1, "{text}: {issues:?}");
            let (issue, context) = &issues[0];
            assert_eq!(&text[issue.start..issue.end], wrong, "{text}");
            assert_eq!(issue.suggestions, [right], "{text}");
            let marker = ["pas", "plus", "jamais", "guère", "point"].into_iter().find(|marker| text.contains(marker)).unwrap();
            let marker_end = text.find(marker).unwrap() + marker.len();
            assert!(context.end >= marker_end && context.start < issue.start, "{text}: {context:?}");
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(found(&fixed, &lexicon).is_empty(), "unstable correction: {fixed}");
        }
        let text = "Les enfants ne joue pas souvent dehors.";
        assert_eq!(found(text, &lexicon)[0].1, 0..text.len() - 1, "the homograph's complete bounded tail is deciding context");
        let issues = found("Les enfants ne joue pas. Les maisons ne tombe pas.", &lexicon);
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn nominal_subject_single_nonreflexive_clitic_preserves_person_tense_and_negation() {
        let lexicon = fixture();
        for (text, wrong, right) in [
            ("Les enfants lui parle.", "lui parle", "lui parlent"),
            ("Le enfant leur parlent.", "leur parlent", "leur parle"),
            ("Les petits enfants lui parlait.", "lui parlait", "lui parlaient"),
            ("Les enfants leur répond.", "leur répond", "leur répondent"),
            ("Les enfants y va.", "y va", "y vont"),
            ("Le enfant en ont besoin.", "en ont", "en a"),
            ("Les enfants ne lui parle pas.", "ne lui parle", "ne lui parlent"),
            ("Les enfants ne leur répond plus.", "ne leur répond", "ne leur répondent"),
            ("Les enfants n’y va jamais.", "n’y va", "n’y vont"),
            ("Le enfant n'en ont guère besoin.", "n'en ont", "n’en a"),
            ("Les enfants ne lui parlait point.", "ne lui parlait", "ne lui parlaient"),
            ("Bonjour. Les enfants lui parle.", "lui parle", "lui parlent"),
            ("Les\u{a0}enfants\u{202f}ne\u{a0}lui\u{202f}parle pas.", "ne\u{a0}lui\u{202f}parle", "ne lui parlent"),
        ] {
            let found = found(text, &lexicon);
            assert_eq!(found.len(), 1, "{text}: {found:?}");
            let (issue, context) = &found[0];
            assert_eq!(&text[issue.start..issue.end], wrong, "{text}");
            assert_eq!(issue.suggestions, [right], "{text}");
            assert!(context.start < issue.start && context.end >= issue.end);
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(super::check_with(&fixed, &crate::words(&fixed), &lexicon).is_empty(), "{fixed}");
        }
    }

    #[test]
    fn nominal_subject_single_clitic_keeps_correct_nominal_and_unsupported_readings() {
        let lexicon = fixture();
        for text in [
            "Les enfants lui parlent.",
            "Le enfant leur parle.",
            "Les enfants y vont.",
            "Le enfant en a besoin.",
            "Les enfants ne lui parlent pas.",
            "Les enfants n’y vont jamais.",
            "Les enfants en route.",
            "Les enfants leur guide.",
            "Les enfants en garde.",
            "Les enfants en parlant.",
            "Les enfants ne lui joue pas.",
            "Les enfants leur pilote.",
            "Les enfants lui en parle.",
            "Les enfants y en a.",
            "Les enfants lui lui parle.",
            "Les enfants le parle.",
            "Les enfants se parle.",
            "Les enfants l’ont.",
            "Les enfants lui parle pas.",
            "Les enfants ne lui parle.",
            "Les enfants ne lui parle que demain.",
            "Les enfants ne lui parle plus jamais.",
            "Les enfants n’ y va pas.",
            "Les enfants lui\nparle.",
            "Les enfants lui, parle.",
            "Les enfants LUI parle.",
            "Les enfants de la maison lui parle.",
            "Les enfants qui lui parle arrivent.",
            "« Les enfants lui parle. »",
            "— Les enfants lui parle.",
        ] {
            assert!(found(text, &lexicon).is_empty(), "{text}: {:?}", found(text, &lexicon));
        }
        assert!(found("Les enfants lui parle.", &Lexicon::builtins()).is_empty());
    }

    #[test]
    fn nominal_subject_negation_does_not_skip_unsupported_or_ambiguous_material() {
        let lexicon = fixture();
        for text in [
            "Les enfants ne jouent pas.",
            "Le petit enfant n’est pas.",
            "Les enfants ne sont pas.",
            "Les enfants ne joue.",
            "Les enfants ne joue que dehors.",
            "Les enfants ne joue vraiment pas.",
            "Les enfants joue pas.",
            "Les enfants n’ joue pas.",
            "Les enfants ne ne joue pas.",
            "Les enfants ne le joue pas.",
            "Les enfants ne lui joue pas.",
            "Les enfants ne s’est pas.",
            "Les enfants ne joue pas plus.",
            "Les enfants ne joue plus jamais.",
            "Les enfants ne joue pas un air.",
            "Les enfants ne guide pas.",
            "Les enfants ne guides pas.",
            "Les enfants ne garde pas.",
            "Les enfants ne pilote pas.",
            "Les enfants ne harcèle pas.",
            "Les enfants ne jouer pas.",
            "Les enfants ne joué pas.",
            "Les enfants ne xyzxyz pas.",
            "Les enfant ne joue pas.",
            "Les enfants ne\njoue pas.",
            "Les enfants ne joue\npas.",
            "Les enfants ne, joue pas.",
            "Les enfants ne joue, pas.",
            "Les enfants ne Joue pas.",
            "Les enfants NE joue pas.",
            "Les enfants de la maison ne joue pas.",
            "Les enfants et la maison ne joue pas.",
            "Les enfants qui ne joue pas arrivent.",
            "« Les enfants ne joue pas. »",
            "— Les enfants ne joue pas.",
        ] {
            assert!(found(text, &lexicon).is_empty(), "{text}: {:?}", found(text, &lexicon));
        }
        assert!(found("Les enfants ne joue pas.", &Lexicon::builtins()).is_empty());
    }
}
