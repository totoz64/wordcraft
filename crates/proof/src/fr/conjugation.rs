//! A small, deliberately bounded first conjugation rule, not a morphological analyser.
//!
//! At the start of a paragraph, a subject immediately followed by present `avoir` needs a
//! participle, not a first-group infinitive: `J'ai manger` → `J'ai mangé`. Only the original
//! list below is covered. No inference from an arbitrary `-er` ending, no agreement guess,
//! no object clitics, no correction inside a quotation or a metalinguistic example.

use crate::Issue;

/// Original small table of ordinary verb forms, not extracted from a linguistic corpus.
/// No nouns merely ending in `er` and no verbs whose compound form needs `être`.
const PARTICIPLES: &[(&str, &str)] = &[
    ("aider", "aidé"),
    ("aimer", "aimé"),
    ("appeler", "appelé"),
    ("chanter", "chanté"),
    ("chercher", "cherché"),
    ("demander", "demandé"),
    ("donner", "donné"),
    ("écouter", "écouté"),
    ("étudier", "étudié"),
    ("fermer", "fermé"),
    ("gagner", "gagné"),
    ("garder", "gardé"),
    ("jouer", "joué"),
    ("manger", "mangé"),
    ("montrer", "montré"),
    ("parler", "parlé"),
    ("penser", "pensé"),
    ("porter", "porté"),
    ("préparer", "préparé"),
    ("regarder", "regardé"),
    ("travailler", "travaillé"),
    ("trouver", "trouvé"),
    ("utiliser", "utilisé"),
    ("visiter", "visité"),
];

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

pub(super) fn check(text: &str, ws: &[(usize, usize)], issues: &mut Vec<Issue>) {
    let Some(&(a, b)) = ws.first() else { return };
    let (Some(before), Some(first)) = (text.get(..a), text.get(a..b)) else { return };
    if !before.trim().is_empty() || first.len() > 16 || first.chars().skip(1).any(char::is_uppercase) {
        return;
    }
    let lower = first.to_lowercase().replace('’', "'");
    let verb_index = if lower == "j'ai" {
        1
    } else {
        let auxiliary = match lower.as_str() {
            "tu" => "as",
            "il" | "elle" | "on" => "a",
            "nous" => "avons",
            "vous" => "avez",
            "ils" | "elles" => "ont",
            _ => return,
        };
        let Some(&(c, d)) = ws.get(1) else { return };
        if text.get(c..d) != Some(auxiliary) || !text.get(b..c).is_some_and(horizontal) {
            return;
        }
        2
    };
    let (Some(&(start, end)), Some(&(_, previous_end))) = (ws.get(verb_index), ws.get(verb_index - 1)) else { return };
    let Some(word) = text.get(start..end) else { return };
    let Some(&(_, participle)) = PARTICIPLES.iter().find(|(infinitive, _)| *infinitive == word) else { return };
    if !text.get(previous_end..start).is_some_and(horizontal) {
        return;
    }
    // Prose only. A quoted example, code fragment or table is too ambiguous for this first rule.
    if text.chars().any(|c| matches!(c, '"' | '«' | '»' | '“' | '”' | '`' | '<' | '>' | '=' | '@' | '\\' | '\t' | '\n' | '\r')) {
        return;
    }
    // `J'ai manger comme infinitif dans cet exercice` mentions a word, rather than using it.
    if ws.iter().skip(verb_index + 1).take(8).any(|&(c, d)| {
        text.get(c..d).is_some_and(|w| {
            w.len() <= 32
                && matches!(
                    w.to_lowercase().as_str(),
                    "mot" | "mots" | "verbe" | "verbes" | "infinitif" | "infinitifs" | "forme" | "formes" | "exemple" | "exemples"
                )
        })
    }) {
        return;
    }
    // A bare infinitive can also be an autonym: `J'ai chanter à conjuguer pour demain`.
    let following = ws.get(verb_index + 1..).unwrap_or_default();
    if following.windows(2).take(8).any(|pair| {
        let [(a, b), (c, d)] = pair else { return false };
        text.get(*a..*b) == Some("à")
            && matches!(
                text.get(*c..*d),
                Some("conjuguer" | "analyser" | "classer" | "citer" | "définir" | "traduire" | "corriger" | "identifier" | "apprendre")
            )
    }) {
        return;
    }
    issues.push(super::grammar::issue(
        start,
        end,
        format!("Participe passé attendu après avoir : «\u{a0}{participle}\u{a0}»"),
        vec![participle.to_string()],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_verb_is_checked_for_each_unambiguous_subject() {
        for prefix in ["J’ai", "J'ai", "Tu as", "Il a", "Elle a", "On a", "Nous avons", "Vous avez", "Ils ont", "Elles ont"] {
            for &(infinitive, participle) in PARTICIPLES {
                let text = format!("{prefix} {infinitive}.");
                let mut issues = Vec::new();
                check(&text, &crate::words(&text), &mut issues);
                assert_eq!(issues.len(), 1, "{text}");
                let issue = &issues[0];
                assert_eq!(&text[issue.start..issue.end], infinitive);
                assert_eq!(issue.suggestions, [participle]);
                let correct = format!("{prefix} {participle}.");
                let mut quiet = Vec::new();
                check(&correct, &crate::words(&correct), &mut quiet);
                assert!(quiet.is_empty(), "{correct}");
            }
        }
    }
}
