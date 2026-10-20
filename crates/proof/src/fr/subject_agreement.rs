//! Direct personal-subject agreement, preserving finite and nominal ambiguity.
use super::{
    grammar,
    morphology::{Lexicon, Person},
};
use crate::Issue;

pub(super) const MESSAGE: &str = "Accord du verbe avec le pronom sujet";

struct Candidate<'a> {
    start: usize,
    end: usize,
    form: &'a str,
    target: Person,
    je: bool,
    capital: bool,
    subject_start: usize,
    negative: bool,
    next: usize,
    contracted_je: bool,
}

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

fn candidate<'a>(text: &'a str, ws: &[(usize, usize)]) -> Option<Candidate<'a>> {
    let &(a, b) = ws.first()?;
    let before = text.get(..a)?;
    let leading = before.trim();
    if !leading.is_empty() && !(matches!(leading, "—" | "–" | "-") && before.chars().last().is_some_and(char::is_whitespace)) {
        return None;
    }
    let first = text.get(a..b)?;
    if first.len() > 132 || first.chars().skip(1).any(char::is_uppercase) {
        return None;
    }
    let (start, end, form, target, je, negative, next, contracted_je) = if let Some(form) =
        first.strip_prefix("J’").or_else(|| first.strip_prefix("j’")).or_else(|| first.strip_prefix("J'")).or_else(|| first.strip_prefix("j'"))
    {
        (a, b, form, Person::FirstSingular, true, false, 1, true)
    } else {
        let lower = first.to_lowercase();
        let target = match lower.as_str() {
            "je" => Person::FirstSingular,
            "tu" => Person::SecondSingular,
            "il" | "elle" | "on" => Person::ThirdSingular,
            "nous" => Person::FirstPlural,
            "vous" => Person::SecondPlural,
            "ils" | "elles" => Person::ThirdPlural,
            _ => return None,
        };
        let &(start, mut end) = ws.get(1)?;
        if !horizontal(text.get(b..start)?) {
            return None;
        }
        let mut form = text.get(start..end)?;
        let mut next = 2;
        let negative = if form == "ne" {
            let &(c, d) = ws.get(2)?;
            if !horizontal(text.get(end..c)?) {
                return None;
            }
            form = text.get(c..d)?;
            end = d;
            next = 3;
            true
        } else if let Some(verb) = form.strip_prefix("n’").or_else(|| form.strip_prefix("n'")) {
            form = verb;
            true
        } else {
            false
        };
        (start, end, form, target, lower == "je", negative, next, false)
    };
    if form.is_empty() || form.len() > 128 || !form.chars().all(char::is_lowercase) {
        return None;
    }
    if text.chars().any(|c| matches!(c, '"' | '«' | '»' | '“' | '”' | '`' | '<' | '>' | '=' | '@' | '\\' | '\t' | '\n' | '\r')) {
        return None;
    }
    if ws.iter().skip(next).take(8).any(|&(a, b)| {
        text.get(a..b).is_some_and(|w| {
            w.len() <= 32
                && matches!(
                    w.to_lowercase().as_str(),
                    "mot" | "mots" | "verbe" | "verbes" | "forme" | "formes" | "exemple" | "exemples" | "infinitif" | "infinitifs"
                )
        })
    }) {
        return None;
    }
    Some(Candidate {
        start,
        end,
        form,
        target,
        je,
        capital: first.chars().next().is_some_and(char::is_uppercase),
        subject_start: a,
        negative,
        next,
        contracted_je,
    })
}

fn nominal_context(text: &str, ws: &[(usize, usize)], candidate: &Candidate<'_>, lexicon: &Lexicon) -> bool {
    let mut previous = candidate.end;
    for (offset, &(a, b)) in ws.iter().skip(candidate.next).take(8).enumerate() {
        let (Some(gap), Some(word)) = (text.get(previous..a), text.get(a..b)) else {
            return true;
        };
        if !horizontal(gap) {
            return gap.trim_start().starts_with(',');
        }
        if matches!(word, "que" | "qui") {
            return true;
        }
        if matches!(word, "mais" | "donc" | "car" | "si" | "lorsque") {
            return false;
        }
        if matches!(word, "et" | "ou")
            && ws.get(candidate.next.saturating_add(offset).saturating_add(1)).and_then(|&(a, b)| text.get(a..b)).is_some_and(|next| {
                matches!(next, "je" | "tu" | "il" | "elle" | "on" | "nous" | "vous" | "ils" | "elles")
                    || lexicon.has_person_reading(next, candidate.target)
            })
        {
            return false;
        }
        if lexicon.has_person_reading(word, candidate.target) {
            return true;
        }
        previous = b;
    }
    false
}

fn known_elision(form: &str) -> Option<bool> {
    let elides = grammar::takes_elision(form);
    // The existing phonetic table deliberately does not classify an unknown h word.
    if form.starts_with('h') && !elides { None } else { Some(elides) }
}

fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon) -> Option<Issue> {
    let candidate = candidate(text, ws)?;
    let forms = lexicon.agreement_forms(candidate.form, candidate.target)?;
    if !candidate.negative
        && matches!(candidate.target, Person::FirstPlural | Person::SecondPlural | Person::ThirdPlural)
        && lexicon.has_nonverbal_reading(candidate.form)
        && (!lexicon.has_auxiliary_reading(candidate.form) || nominal_context(text, ws, &candidate, lexicon))
    {
        return None;
    }
    let mut start = candidate.start;
    let suggestions = if candidate.negative {
        forms
            .into_iter()
            .map(|form| known_elision(&form).map(|elides| if elides { format!("n’{form}") } else { format!("ne {form}") }))
            .collect::<Option<Vec<_>>>()?
    } else if candidate.je {
        let elisions = forms.iter().map(|f| known_elision(f)).collect::<Option<Vec<_>>>()?;
        if candidate.contracted_je || elisions.iter().any(|&elides| elides) {
            start = candidate.subject_start;
            forms
                .into_iter()
                .zip(elisions)
                .map(|(form, elides)| {
                    if elides {
                        format!("{}’{form}", if candidate.capital { "J" } else { "j" })
                    } else {
                        format!("{} {form}", if candidate.capital { "Je" } else { "je" })
                    }
                })
                .collect()
        } else {
            forms
        }
    } else {
        forms
    };
    Some(grammar::issue(start, candidate.end, MESSAGE.into(), suggestions))
}

pub(super) fn check(text: &str, ws: &[(usize, usize)]) -> Option<Issue> {
    candidate(text, ws)?; // Avoid optional-data loading for unrelated paragraphs.
    check_with(text, ws, &super::morphology::lexicon())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Option<Issue> {
        check_with(text, &crate::words(text), &Lexicon::builtins())
    }

    #[test]
    fn original_subject_errors_have_stable_person_and_apostrophe_corrections() {
        for (text, wrong, right) in [
            ("Je est responsable.", "est", "suis"),
            ("Tu est responsable.", "est", "es"),
            ("Il as raison.", "as", "a"),
            ("Elle avons raison.", "avons", "a"),
            ("On êtes responsable.", "êtes", "est"),
            ("Nous a raison.", "a", "avons"),
            ("Vous ont raison.", "ont", "avez"),
            ("Ils avez raison.", "avez", "ont"),
            ("Elles es présentes.", "es", "sont"),
            ("Je avons raison.", "Je avons", "J’ai"),
            ("je ont raison.", "je ont", "j’ai"),
            ("Je a raison.", "Je a", "J’ai"),
            ("Je ne est pas responsable.", "ne est", "ne suis"),
            ("Je n'est pas responsable.", "n'est", "ne suis"),
            ("Tu n’ont pas raison.", "n’ont", "n’as"),
            ("Nous n’es pas responsables.", "n’es", "ne sommes"),
            ("Vous ne a pas raison.", "ne a", "n’avez"),
            ("— Nous a raison.", "a", "avons"),
            ("Vous a peur et savez pourquoi.", "a", "avez"),
            ("J’est responsable.", "J’est", "Je suis"),
            ("J'ont raison.", "J'ont", "J’ai"),
            ("j’êtes responsable.", "j’êtes", "je suis"),
        ] {
            let issue = found(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(&text[issue.start..issue.end], wrong, "{text}");
            assert_eq!(issue.suggestions, [right], "{text}");
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(found(&fixed).is_none(), "{fixed}");
        }
    }

    #[test]
    fn valid_readings_and_out_of_scope_constructions_stay_quiet() {
        for text in [
            "Je suis responsable.",
            "J’ai raison.",
            "J’avais raison.",
            "Tu suis cette piste.",
            "Tu sommes le débiteur de payer.",
            "Nous sommes prêts.",
            "Vous avez raison.",
            "Ils sont présents.",
            "Je ne suis pas responsable.",
            "Tu n’as pas raison.",
            "Nous as du volant sommes prudents.",
            "Nous as du volant, nous sommes prudents.",
            "Nous, a raison.",
            "Nous lui a parlé.",
            "Nous ne lui a parlé.",
            "Nous avons manger.",
            "Nous xyzxyz.",
            "Nous manger.",
            "Nous A raison.",
            "NOUS a raison.",
            "Nous\na raison.",
            "Voici une phrase. Nous a raison.",
            "« Nous a raison. »",
            "Nous a comme verbe dans cet exemple.",
        ] {
            assert!(found(text).is_none(), "{text}");
        }
    }
}
