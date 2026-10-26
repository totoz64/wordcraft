//! Small French context rules, intentionally narrower than a general grammar parser.
//! Compare composed lexical copies; retain the original token bounds and document bytes.
use crate::{Issue, IssueKind, ProofLang};

/// Common first-group verbs; avoid deriving verbs from arbitrary words such as hier or cher.
const PAST_FORMS: &[(&str, &str)] = &[
    ("aimer", "aimé"),
    ("chanter", "chanté"),
    ("chercher", "cherché"),
    ("commencer", "commencé"),
    ("danser", "dansé"),
    ("dessiner", "dessiné"),
    ("donner", "donné"),
    ("écouter", "écouté"),
    ("envoyer", "envoyé"),
    ("fermer", "fermé"),
    ("garder", "gardé"),
    ("jouer", "joué"),
    ("laisser", "laissé"),
    ("laver", "lavé"),
    ("manger", "mangé"),
    ("marcher", "marché"),
    ("oublier", "oublié"),
    ("parler", "parlé"),
    ("passer", "passé"),
    ("penser", "pensé"),
    ("porter", "porté"),
    ("préparer", "préparé"),
    ("ranger", "rangé"),
    ("regarder", "regardé"),
    ("terminer", "terminé"),
    ("travailler", "travaillé"),
    ("trouver", "trouvé"),
    ("visiter", "visité"),
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
            || !matches!(
                (first.as_str(), aux),
                ("tu", "as") | ("il" | "elle" | "on", "a") | ("nous", "avons") | ("vous", "avez") | ("ils" | "elles", "ont")
            )
        {
            return;
        }
        ws.get(2).copied().filter(|&(a, _)| joined(text, b1, a))
    };
    let Some((start, end)) = next else { return };
    let Some(word) = text.get(start..end) else { return };
    let word = crate::composed(word);
    if let Some((_, past)) = PAST_FORMS.iter().find(|(infinitive, _)| *infinitive == word.as_ref()) {
        suggest(v, start, end, "Participe passé attendu après l’auxiliaire avoir", past);
    }
}

/// A single missing-hyphen expression; the predicate bounds every word used as context.
/// Compare exact source tokens: do not erase marks, change case or repair whitespace here.
pub(super) fn grand_chose(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>, context_allowed: impl Fn(usize, usize) -> bool) {
    for triple in ws.windows(3) {
        let [(a0, b0), (a1, b1), (a2, b2)] = triple else { continue };
        let (Some(negative), Some(left), Some(right)) = (text.get(*a0..*b0), text.get(*a1..*b1), text.get(*a2..*b2)) else { continue };
        if matches!(negative, "pas" | "plus" | "jamais")
            && left == "grand"
            && right == "chose"
            && text.get(*b0..*a1) == Some(" ")
            && text.get(*b1..*a2) == Some(" ")
            // words() trims terminal hyphens; leave an unfinished token alone.
            && !text.get(*b2..).is_some_and(|tail| tail.starts_with('-'))
            && context_allowed(*a0, *b2)
        {
            suggest(v, *a1, *b2, "Trait d’union attendu dans la locution grand-chose", "grand-chose");
        }
    }
}

/// Distinguish là où from an article elision and a few paragraph-initial avoir expressions.
pub(super) fn check(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    for pair in ws.windows(2) {
        let [(a0, b0), (a1, b1)] = pair else { continue };
        let (Some(left), Some(right)) = (text.get(*a0..*b0), text.get(*a1..*b1)) else { continue };
        let right = crate::composed(right);
        if matches!(left, "la" | "La") && right == "où" && joined(text, *b0, *a1) {
            suggest(v, *a0, *b0, "L’adverbe de lieu là est attendu devant où", if left == "La" { "Là" } else { "là" });
        }
    }
    if let [first, accent, noun, ..] = ws {
        let (a0, b0) = *first;
        let (a1, b1) = *accent;
        let (a2, b2) = *noun;
        let (Some(subject), Some(mark), Some(word)) = (text.get(a0..b0), text.get(a1..b1), text.get(a2..b2)) else { return };
        let mark = crate::composed(mark);
        let word = crate::composed(word);
        if text.get(..a0).is_some_and(|prefix| prefix.trim().is_empty())
            && matches!(subject.to_lowercase().as_str(), "il" | "elle" | "on")
            && mark == "à"
            && matches!(word.as_ref(), "faim" | "soif" | "besoin" | "envie" | "peur" | "raison" | "tort" | "hâte")
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
                ("Il à faim.", "à", "a"),
                ("Elle à soif.", "à", "a"),
                ("On à besoin de repos.", "à", "a"),
                ("Ils ont manger une pomme.", "manger", "mangé"),
                ("Elle a préparer le repas.", "préparer", "préparé"),
                ("Nous avons terminer le travail.", "terminer", "terminé"),
                ("J’ai écouter le concert.", "écouter", "écouté"),
                ("Tu as chercher ton livre.", "chercher", "cherché"),
                ("Vous avez regarder le ciel.", "regarder", "regardé"),
                ("Les livres sont la où il reste de la place.", "la", "là"),
                ("La où il habite, il neige.", "La", "Là"),
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
            "Il a faim.",
            "Elle a soif.",
            "On a besoin de repos.",
            "Il arrive à pied.",
            "As-tu à manger ?",
            "Ils ont mangé une pomme.",
            "Elle a préparé le repas.",
            "Nous avons terminé le travail.",
            "Il veut manger une pomme.",
            "Ils ont à manger.",
            "Il a cher payé.",
            "Nous avons hier terminé.",
            "Elle a Roger au téléphone.",
            "Il dit avoir préparer le repas.",
            "La pomme que nous avons manger est rouge.",
            "Nous les avons regarder.",
            "Nous avons bien manger.",
            "Les livres sont là où il reste de la place.",
            "Prends-la où tu veux.",
            "Il dit de où il vient.",
        ] {
            // Existing elision can independently flag de où; this rule must not create an article elision.
            let mut own = Vec::new();
            check(text, &crate::words(text), &mut own);
            assert!(own.is_empty(), "{text}: {own:?}");
        }
    }

    #[test]
    fn french_contextual_rules_match_nfc_and_nfd_at_original_bytes() {
        use unicode_normalization::UnicodeNormalization;

        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            for (source, old, replacement) in [
                ("\u{A0}\u{202F}Il à faim.", "à", "a"),
                ("\u{A0}\u{202F}Elle à hâte.", "à", "a"),
                ("\u{A0}\u{202F}Elle a préparer le repas.", "préparer", "préparé"),
                ("\u{A0}\u{202F}J’ai écouter le concert.", "écouter", "écouté"),
                ("😀 Café\u{301} : la où il habite.", "la", "là"),
            ] {
                for decomposed in [false, true] {
                    let text = if decomposed { source.nfd().collect::<String>() } else { source.into() };
                    let old = if decomposed { old.nfd().collect::<String>() } else { old.into() };
                    let before = text.clone();
                    let issues = super::super::grammar::check_in(&text, lang);
                    assert_eq!(issues.len(), 1, "{lang:?} {text:?}: {issues:?}");
                    let issue = &issues[0];
                    let start = text.find(&old).unwrap();
                    assert_eq!((issue.start, issue.end), (start, start + old.len()), "{text:?}");
                    assert_eq!(issue.lang, lang);
                    assert_eq!(issue.suggestions, [replacement.to_string()]);
                    assert_eq!(text, before, "lookup does not rewrite the source");
                    let mut fixed = text.clone();
                    fixed.replace_range(issue.start..issue.end, replacement);
                    assert_eq!(&fixed[..start], &text[..start], "preserve the original prefix");
                    assert_eq!(&fixed[start + replacement.len()..], &text[start + old.len()..], "preserve the original suffix");
                    assert!(super::super::grammar::check_in(&fixed, lang).is_empty(), "{fixed:?}");
                }
            }
        }
    }

    #[test]
    fn french_contextual_normalization_preserves_quiet_cases_and_marks() {
        use unicode_normalization::UnicodeNormalization;

        for source in [
            "Il a faim.",
            "Elle a hâte.",
            "Elle a préparé le repas.",
            "J’ai écouté le concert.",
            "Les livres sont là où il reste de la place.",
            "Il arrive à pied.",
            "Il veut préparer le repas.",
            "Nous les avons préparer.",
            "Il à\u{301} faim.",
            "Elle a préparer\u{301} le repas.",
            "La où\u{301} il habite.",
        ] {
            for text in [source.to_string(), source.nfd().collect()] {
                let mut own = Vec::new();
                check(&text, &crate::words(&text), &mut own);
                assert!(own.is_empty(), "{text:?}: {own:?}");
            }
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

    fn locution_fixes(text: &str) -> Vec<Issue> {
        let mut issues = Vec::new();
        grand_chose(text, &crate::words(text), &mut issues, |_, _| true);
        issues
    }

    #[test]
    fn french_locution_original_positive_cases_keep_original_utf8_ranges() {
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            for (id, text) in [
                ("gc-01", "Cette boîte ne contient pas grand chose."),
                ("gc-02", "Nous ne trouvons plus grand chose ici."),
                ("gc-03", "Elle ne raconte jamais grand chose."),
                ("gc-04", "Tu gardes pas grand chose pour demain."),
                ("gc-05", "Cela ne laisse pas grand chose de visible."),
                ("gc-06", "Après vérification, il ne reste pas grand chose."),
                ("gc-07", "🧭 Élodie ne distingue pas grand chose."),
                ("gc-08", "Élodie ne reçoit plus grand chose."),
            ] {
                let before = text.to_string();
                let own = locution_fixes(text);
                assert_eq!(own.len(), 1, "{id} {lang:?}: {own:?}");
                let issues = super::super::grammar::check_in(text, lang);
                let issue = issues.iter().find(|i| i.suggestions == ["grand-chose"]).unwrap();
                let start = text.find("grand chose").unwrap();
                assert_eq!((issue.start, issue.end), (start, start + "grand chose".len()), "{id}");
                assert_eq!(text.get(issue.start..issue.end), Some("grand chose"));
                assert_eq!(issue.lang, lang);
                assert_eq!(issue.kind, IssueKind::Grammar);
                assert_eq!(issue.suggestions, ["grand-chose".to_string()]);
                assert_eq!(text, before, "lookup preserves the original document");
                let mut fixed = text.to_string();
                fixed.replace_range(issue.start..issue.end, "grand-chose");
                assert_eq!(&fixed[..start], &text[..start], "{id}: preserve prefix, including NFD");
                assert_eq!(&fixed[start + "grand-chose".len()..], &text[start + "grand chose".len()..], "{id}: preserve suffix");
                assert!(locution_fixes(&fixed).is_empty(), "{id}: do not repeat the suggestion");
            }
        }
    }

    #[test]
    fn french_locution_original_correct_counterexamples_stay_quiet() {
        for (id, text) in [
            ("gc-09", "Cette boîte ne contient pas grand-chose."),
            ("gc-10", "Il ne reste plus grand-chose à décrire."),
            ("gc-11", "Elle prépare quelque chose de simple."),
            ("gc-12", "Il me faut autre chose de solide."),
            ("gc-13", "C’est peu de chose à transporter."),
            ("gc-14", "Cette grande chose en bois bouche la porte."),
            ("gc-15", "Elle entreprend de grandes choses."),
            ("gc-16", "Après cette marche, nous avons grand faim."),
            ("gc-17", "Ce petit avait grand peur du tonnerre."),
            ("ad-05", "Le drone reste au-dessus du bassin."),
            ("ad-06", "Le bureau est au-dessus."),
            ("ad-07", "La toile est bleue en dessus et grise en dessous."),
            ("ad-08", "Retire ce carton de dessus la table."),
            ("ad-09", "Le dessus du coffre est rayé."),
            ("bd-06", "Il ne garde pas grand, chose étrange."),
        ] {
            assert!(locution_fixes(text).is_empty(), "{id}: {text:?}");
        }
    }

    #[test]
    fn french_locution_original_abstentions_are_not_accepted_spellings() {
        // These are scope limits or unresolved readings, not a list of correct spellings.
        // bd-07 is exercised through the mixed-language API and review-command tests below.
        for (id, text) in [
            ("gc-18", "Elle ne conserve pas grand’chose."),
            ("gc-19", "Il repart sans grand chose."),
            ("gc-20", "Pas grand chose ne change au programme."),
            ("gc-21", "Il ne conserve pas GRAND CHOSE."),
            ("gc-22", "Le titre du fichier est Grand Chose."),
            ("ad-01", "Le drone reste au dessus du bassin."),
            ("ad-02", "Cette réponse se situe au dessus de nos attentes."),
            ("ad-03", "Le bureau est au dessus."),
            ("ad-04", "Au dessus du garage, la lampe clignote."),
            ("ad-10", "Je préfère le dessous au dessus de cette planche."),
            ("ad-11", "Le restaurateur consacre son attention au dessus du coffret."),
            ("bd-01", "Il ne garde pas grand\nchose."),
            ("bd-02", "Il ne garde pas grand\tchose."),
            ("bd-03", "Il ne garde pas grand chose."),
            ("bd-04", "Il ne garde pas grand chose."),
            ("bd-05", "Il ne garde pas grand  chose."),
            ("bd-08", "Il ne garde pas grandchose."),
            ("bd-09", "Le drone reste audessus du bassin."),
            ("bd-10", "Il ne garde pas grand́ chose."),
        ] {
            assert!(locution_fixes(text).is_empty(), "{id}: {text:?}");
        }
    }

    #[test]
    fn french_locution_preserves_unrelated_traditional_and_rectified_spellings() {
        // External dictionary variants do not configure this fixed expression's spelling.
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            for text in ["Ce coût ne révèle pas grand chose.", "Ce cout ne révèle pas grand chose."] {
                let issues = super::super::grammar::check_in(text, lang);
                let issue = issues.iter().find(|i| i.suggestions == ["grand-chose"]).unwrap();
                let start = text.find("grand chose").unwrap();
                let mut fixed = text.to_string();
                fixed.replace_range(issue.start..issue.end, "grand-chose");
                assert_eq!(&fixed[..start], &text[..start]);
                assert_eq!(fixed.get(start..), Some("grand-chose."));
            }
        }
    }

    #[test]
    fn french_locution_context_stays_in_one_language_region() {
        let text = "Il ne garde pas grand chose.";
        let marker = text.find("pas").unwrap();
        let grand = text.find("grand").unwrap();
        let chose = text.find("chose").unwrap();
        let has_locution = |issues: Vec<Issue>| issues.iter().any(|i| i.suggestions == ["grand-chose"]);
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert!(has_locution(crate::check_text(text, &[(0..text.len(), lang)])));
            assert!(has_locution(crate::check_text(text, &[(0..marker, ProofLang::En), (marker..text.len(), lang)])));
            // Same-region formatting spans are merged, not mistaken for language barriers.
            assert!(has_locution(crate::check_text(text, &[(0..grand, lang), (grand..text.len(), lang)])));
            for other in [ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                for cut in [marker + 1, grand, chose] {
                    for (left, right) in [(lang, other), (other, lang)] {
                        assert!(!has_locution(crate::check_text(text, &[(0..cut, left), (cut..text.len(), right)])), "{left:?} → {right:?} at {cut}");
                    }
                }
            }
        }
    }

    #[test]
    fn french_locution_rejects_marker_gaps_case_and_partial_tokens() {
        for text in [
            "Il ne garde pas\tgrand chose.",
            "Il ne garde pas\ngrand chose.",
            "Il ne garde pas, grand chose.",
            "Il ne garde pas  grand chose.",
            "Il ne garde Pas grand chose.",
            "Il ne garde PLUS grand chose.",
            "Il ne garde pas Grand chose.",
            "Il ne garde pas grand Chose.",
            "Il ne garde pas grand choses.",
            "Il ne garde pas grand chose-là.",
            "Il ne garde pas grand-chose.",
            "Il ne garde pas grand chose-",
            "Il ne garde pas grand chose--",
        ] {
            assert!(locution_fixes(text).is_empty(), "{text:?}");
        }
        let long = "x".repeat(200_000);
        assert!(locution_fixes(&long).is_empty());
    }
}
