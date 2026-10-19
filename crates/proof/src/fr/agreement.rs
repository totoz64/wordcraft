//! Person agreement in a deliberately narrow cleft: `C'est moi qui suis…`.
//! A personal antecedent avoids the variable agreements of noun/demonstrative attributes.

use super::morphology::{Lexicon, Person};
use crate::Issue;

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

fn candidate(text: &str, ws: &[(usize, usize)]) -> Option<(usize, usize, Person)> {
    let &(a, b) = ws.first()?;
    if !text.get(..a)?.trim().is_empty() {
        return None;
    }
    let first = text.get(a..b)?;
    let (antecedent_index, plural_copula) = match first {
        "C'est" | "C’est" | "c'est" | "c’est" => (1, false),
        "Ce" | "ce" if ws.get(1).and_then(|&(c, d)| text.get(c..d)) == Some("sont") => (2, true),
        _ => return None,
    };
    let &(c, d) = ws.get(antecedent_index)?;
    let target = match text.get(c..d)? {
        "moi" => Person::FirstSingular,
        "toi" => Person::SecondSingular,
        "lui" | "elle" => Person::ThirdSingular,
        "nous" => Person::FirstPlural,
        "vous" => Person::SecondPlural,
        "eux" | "elles" => Person::ThirdPlural,
        _ => return None,
    };
    if plural_copula && target != Person::ThirdPlural {
        return None;
    }
    let &(e, f) = ws.get(antecedent_index + 1)?;
    let &(start, end) = ws.get(antecedent_index + 2)?;
    if text.get(e..f)? != "qui" {
        return None;
    }
    for pair in ws.get(..antecedent_index + 3)?.windows(2) {
        let &[(_, previous), (next, _)] = pair else { return None };
        if !horizontal(text.get(previous..next)?) {
            return None;
        }
    }
    let verb = text.get(start..end)?;
    if verb.len() > 128 || !verb.chars().all(char::is_lowercase) {
        return None;
    }
    // A prose-only first rule. Do not interpret quotations, code, tables or examples.
    if text.chars().any(|c| matches!(c, '"' | '«' | '»' | '“' | '”' | '`' | '<' | '>' | '=' | '@' | '\\' | '\t' | '\n' | '\r')) {
        return None;
    }
    if ws.iter().skip(antecedent_index + 3).take(8).any(|&(a, b)| {
        text.get(a..b).is_some_and(|w| {
            w.len() <= 32 && matches!(w.to_lowercase().as_str(), "mot" | "mots" | "verbe" | "verbes" | "forme" | "formes" | "exemple" | "exemples")
        })
    }) {
        return None;
    }
    Some((start, end, target))
}

fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon, issues: &mut Vec<Issue>) {
    let Some((start, end, target)) = candidate(text, ws) else { return };
    let Some(form) = text.get(start..end) else { return };
    let Some(suggestions) = lexicon.agreement_forms(form, target) else { return };
    issues.push(super::grammar::issue(start, end, "Accord du verbe avec l’antécédent de qui".into(), suggestions));
}

pub(super) fn check(text: &str, ws: &[(usize, usize)], issues: &mut Vec<Issue>) {
    // Do not load optional data for every unrelated paragraph.
    if candidate(text, ws).is_some() {
        check_with(text, ws, &super::morphology::lexicon(), issues);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<Issue> {
        let mut issues = Vec::new();
        check_with(text, &crate::words(text), &Lexicon::builtins(), &mut issues);
        issues
    }

    #[test]
    fn personal_clefts_have_exact_ranges_and_stable_suggestions() {
        for (text, wrong, right) in [
            ("C’est moi qui est responsable.", "est", "suis"),
            ("C'est toi qui a la clé.", "a", "as"),
            ("C’est lui qui as gagné.", "as", "a"),
            ("C’est elle qui avez gagné.", "avez", "a"),
            ("C’est nous qui ont raison.", "ont", "avons"),
            ("C’est vous qui avons raison.", "avons", "avez"),
            ("Ce sont eux qui a raison.", "a", "ont"),
            ("Ce sont elles qui est présentes.", "est", "sont"),
            ("  C’est\u{a0}moi qui est responsable.", "est", "suis"),
        ] {
            let issues = found(text);
            assert_eq!(issues.len(), 1, "{text}: {issues:?}");
            let issue = &issues[0];
            assert_eq!(&text[issue.start..issue.end], wrong);
            assert_eq!(issue.suggestions, [right]);
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(found(&fixed).is_empty(), "{fixed}");
        }
    }

    #[test]
    fn valid_homographs_and_out_of_scope_constructions_stay_quiet() {
        for text in [
            "C’est moi qui suis responsable.",
            "C’est toi qui suis cette piste.",
            "C’est toi qui sommes le débiteur de payer.",
            "C’est nous qui sommes prêts.",
            "C’est vous qui avez raison.",
            "Ce sont eux qui sont présents.",
            "Je suis celle qui est responsable.",
            "Je suis celle qui suis responsable.",
            "Nous sommes ceux qui ont gagné.",
            "Nous sommes ceux qui avons gagné.",
            "C’est Marie qui est responsable.",
            "C’est nous qui lui a parlé.",
            "C’est toi qui ne a rien dit.",
            "Il dit que c’est moi qui est responsable.",
            "C’est moi, qui est responsable.",
            "C’est moi qui\nest ici.",
            "« C’est moi qui est ici. »",
            "C’est moi qui est dans cet exemple.",
            "C’est moi qui Est ici.",
            "C’est moi qui xyz.",
            "C’est moi qui manger.",
            "Ce sont moi qui est responsable.",
            "C’est moi qui-est ici.",
        ] {
            assert!(found(text).is_empty(), "{text}: {:?}", found(text));
        }
    }
}
