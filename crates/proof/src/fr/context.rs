//! Small French context rules, intentionally narrower than a general grammar parser.
use crate::{Issue, IssueKind, ProofLang};

/// Common first-group verbs; avoid deriving verbs from arbitrary words such as hier or cher.
const PAST_FORMS: &[(&str, &str)] = &[
    ("aimer", "aimé"), ("chanter", "chanté"), ("chercher", "cherché"), ("commencer", "commencé"),
    ("danser", "dansé"), ("dessiner", "dessiné"), ("donner", "donné"), ("écouter", "écouté"),
    ("envoyer", "envoyé"), ("fermer", "fermé"), ("garder", "gardé"), ("jouer", "joué"),
    ("laisser", "laissé"), ("laver", "lavé"), ("manger", "mangé"), ("marcher", "marché"),
    ("oublier", "oublié"), ("parler", "parlé"), ("passer", "passé"), ("penser", "pensé"),
    ("porter", "porté"), ("préparer", "préparé"), ("ranger", "rangé"), ("regarder", "regardé"),
    ("terminer", "terminé"), ("travailler", "travaillé"), ("trouver", "trouvé"), ("visiter", "visité"),
];

/// Adjacent words must share a line and have no intervening punctuation or quotation marks.
fn joined(text: &str, end: usize, start: usize) -> bool {
    text.get(end..start).is_some_and(|gap| !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\t' | '\u{A0}' | '\u{202F}')))
}

/// Keep a grammar suggestion on just the token being changed; regional tagging is set by the caller.
fn suggest(v: &mut Vec<Issue>, start: usize, end: usize, message: &str, replacement: &str) {
    v.push(Issue { start, end, kind: IssueKind::Grammar, message: message.into(), suggestions: vec![replacement.into()], lang: ProofLang::Fr });
}

/// A small paragraph-initial avoir construction, with no intervening object or relative clause.
fn initial_past(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    let Some(&(a0, b0)) = ws.first() else { return };
    if !text.get(..a0).is_some_and(|prefix| prefix.trim().is_empty()) {
        return;
    }
    let Some(first) = text.get(a0..b0) else { return };
    let first = first.to_lowercase();
    let next = if matches!(first.as_str(), "j'ai" | "j’ai") {
        ws.get(1).copied().filter(|&(a, _)| joined(text, b0, a))
    } else {
        let Some(&(a1, b1)) = ws.get(1) else { return };
        let Some(aux) = text.get(a1..b1) else { return };
        if !joined(text, b0, a1)
            || !matches!((first.as_str(), aux), ("tu", "as") | ("il" | "elle" | "on", "a") | ("nous", "avons") | ("vous", "avez") | ("ils" | "elles", "ont"))
        {
            return;
        }
        ws.get(2).copied().filter(|&(a, _)| joined(text, b1, a))
    };
    let Some((start, end)) = next else { return };
    let Some(word) = text.get(start..end) else { return };
    if let Some((_, past)) = PAST_FORMS.iter().find(|(infinitive, _)| *infinitive == word) {
        suggest(v, start, end, "Participe passé attendu après l’auxiliaire avoir", past);
    }
}

/// Distinguish là où from an article elision and a few paragraph-initial avoir expressions.
pub(super) fn check(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    for pair in ws.windows(2) {
        let [(a0, b0), (a1, b1)] = pair else { continue };
        let (Some(left), Some(right)) = (text.get(*a0..*b0), text.get(*a1..*b1)) else { continue };
        if matches!(left, "la" | "La") && right == "où" && joined(text, *b0, *a1) {
            suggest(v, *a0, *b0, "L’adverbe de lieu là est attendu devant où", if left == "La" { "Là" } else { "là" });
        }
    }
    if let [first, accent, noun, ..] = ws {
        let (a0, b0) = *first;
        let (a1, b1) = *accent;
        let (a2, b2) = *noun;
        let (Some(subject), Some(mark), Some(word)) = (text.get(a0..b0), text.get(a1..b1), text.get(a2..b2)) else { return };
        if text.get(..a0).is_some_and(|prefix| prefix.trim().is_empty())
            && matches!(subject.to_lowercase().as_str(), "il" | "elle" | "on")
            && mark == "à"
            && matches!(word, "faim" | "soif" | "besoin" | "envie" | "peur" | "raison" | "tort" | "hâte")
            && joined(text, b0, a1)
            && joined(text, b1, a2)
        {
            suggest(v, a1, b1, "Le verbe avoir s’écrit a dans cette expression", "a");
        }
    }
    initial_past(text, ws, v);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixes(text: &str, lang: ProofLang) -> Vec<(String, String)> {
        super::super::grammar::check_in(text, lang)
            .into_iter()
            .map(|i| (text.get(i.start..i.end).unwrap_or_default().into(), i.suggestions.first().cloned().unwrap_or_default()))
            .collect()
    }

    #[test]
    fn french_contextual_rules_correct_the_selected_token_in_both_regions() {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            for (text, old, new) in [
                ("Il à faim.", "à", "a"), ("Elle à soif.", "à", "a"), ("On à besoin de repos.", "à", "a"),
                ("Ils ont manger une pomme.", "manger", "mangé"), ("Elle a préparer le repas.", "préparer", "préparé"),
                ("Nous avons terminer le travail.", "terminer", "terminé"), ("J’ai écouter le concert.", "écouter", "écouté"),
                ("Tu as chercher ton livre.", "chercher", "cherché"), ("Vous avez regarder le ciel.", "regarder", "regardé"),
                ("Les livres sont la où il reste de la place.", "la", "là"), ("La où il habite, il neige.", "La", "Là"),
            ] {
                let issues = super::super::grammar::check_in(text, lang);
                assert_eq!(fixes(text, lang), [(old.into(), new.into())], "{text}");
                assert!(issues.iter().all(|i| i.lang == lang));
                let issue = issues.first().unwrap();
                let mut fixed = text.to_string();
                fixed.replace_range(issue.start..issue.end, new);
                assert!(super::super::grammar::check_in(&fixed, lang).is_empty(), "{fixed}");
            }
        }
    }

    #[test]
    fn french_contextual_rules_leave_correct_and_ambiguous_text_quiet() {
        for text in [
            "Il a faim.", "Elle a soif.", "On a besoin de repos.", "Il arrive à pied.", "As-tu à manger ?",
            "Ils ont mangé une pomme.", "Elle a préparé le repas.", "Nous avons terminé le travail.",
            "Il veut manger une pomme.", "Ils ont à manger.", "Il a cher payé.", "Nous avons hier terminé.",
            "Elle a Roger au téléphone.", "Il dit avoir préparer le repas.", "La pomme que nous avons manger est rouge.",
            "Nous les avons regarder.", "Nous avons bien manger.", "Les livres sont là où il reste de la place.",
            "Prends-la où tu veux.", "Il dit de où il vient.",
        ] {
            // Existing elision can independently flag de où; this rule must not create an article elision.
            let mut own = Vec::new();
            check(text, &crate::words(text), &mut own);
            assert!(own.is_empty(), "{text}: {own:?}");
        }
    }

    #[test]
    fn french_contextual_rules_keep_utf8_ranges_and_boundaries() {
        let text = "  Elle\u{A0}a préparer le repas.";
        assert_eq!(fixes(text, ProofLang::Fr), [("préparer".into(), "préparé".into())]);
        for text in ["Il\nà faim.", "Ils ont\nmanger.", "Il, à faim.", "Elle a « préparer ».", "La, où il vit."] {
            let mut own = Vec::new();
            check(text, &crate::words(text), &mut own);
            assert!(own.is_empty(), "{text}: {own:?}");
        }
        let mut own = Vec::new();
        check(&"x".repeat(200_000), &crate::words(&"x".repeat(200_000)), &mut own);
        assert!(own.is_empty());
    }
}
