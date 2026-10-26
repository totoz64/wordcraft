//! First consumer of the independent morphology layer: adjacent, paragraph-initial subject
//! pronouns. Do not infer a subject from a spelling-language run or from an object pronoun.
use super::morphology::{self, Person};
use crate::{Issue, IssueKind, ProofLang};

fn subject(word: &str) -> Option<Person> {
    match word {
        "je" | "Je" => Some(Person::FirstSingular),
        "tu" | "Tu" => Some(Person::SecondSingular),
        "il" | "Il" | "elle" | "Elle" | "on" | "On" => Some(Person::ThirdSingular),
        "ils" | "Ils" | "elles" | "Elles" => Some(Person::ThirdPlural),
        // Nous/vous can be object pronouns even at the start of an inverted clause.
        _ => None,
    }
}

fn horizontal_gap(text: &str, end: usize, start: usize) -> bool {
    text.get(end..start).is_some_and(|gap| !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\t' | '\u{A0}' | '\u{202F}')))
}

pub(super) fn check(text: &str, ws: &[(usize, usize)], issues: &mut Vec<Issue>, context_allowed: impl Fn(usize, usize) -> bool) {
    let Some(&(first_start, first_end)) = ws.first() else { return };
    if !text.get(..first_start).is_some_and(|prefix| prefix.chars().all(char::is_whitespace)) {
        return;
    }
    let Some(first) = text.get(first_start..first_end) else { return };
    let elided = ["j’", "J’", "j'", "J'"].into_iter().find_map(|prefix| first.strip_prefix(prefix));
    let (person, start, end) = if let Some(person) = subject(first) {
        let Some(&(start, end)) = ws.get(1) else { return };
        if !horizontal_gap(text, first_end, start) {
            return;
        }
        (person, start, end)
    } else if let Some(tail) = elided {
        let Some(start) = first_end.checked_sub(tail.len()) else { return };
        (Person::FirstSingular, start, first_end)
    } else {
        return;
    };
    if !context_allowed(first_start, end) || text.get(end..).is_some_and(|tail| tail.starts_with(['-', '\'', '’'])) {
        return;
    }
    let Some(original) = text.get(start..end) else { return };
    let word = crate::composed(original);
    let Some(replacement) = morphology::agreement_repair(&word, person) else { return };
    if start < first_end && !replacement.starts_with(['a', 'e', 'é', 'è', 'ê', 'i', 'o', 'u', 'y']) {
        return;
    }
    // Keep the existing elision rule independent: unelided je + vowel is already handled there.
    if matches!(first, "je" | "Je") && replacement.starts_with(['a', 'e', 'é', 'è', 'ê', 'i', 'o', 'u', 'y', 'h']) {
        return;
    }
    issues.push(Issue {
        start,
        end,
        kind: IssueKind::Grammar,
        message: "Le verbe doit s’accorder avec son sujet en personne et en nombre".into(),
        suggestions: vec![replacement],
        lang: ProofLang::Fr,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn own(text: &str) -> Vec<Issue> {
        let mut result = Vec::new();
        check(text, &crate::words(text), &mut result, |_, _| true);
        result
    }

    #[test]
    fn subject_agreement_reuses_paradigms_without_rewriting_surrounding_text() {
        use unicode_normalization::UnicodeNormalization;

        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            for (source, old, new) in [
                ("Ils mange une pomme.", "mange", "mangent"),
                ("Elle parles à Luc.", "parles", "parle"),
                ("Tu commencerons demain.", "commencerons", "commenceras"),
                ("Il travailleraient demain.", "travailleraient", "travaillerait"),
                ("Je parlèrent du trajet.", "parlèrent", "parlai"),
                ("Elles commençait le dessin.", "commençait", "commençaient"),
                ("On dessinez un cercle.", "dessinez", "dessine"),
                ("J’écoutons la radio.", "écoutons", "écoute"),
                ("J'aimons cette chanson.", "aimons", "aime"),
                ("\u{A0}Tu prépare le repas.", "prépare", "prépares"),
            ] {
                for text in [source.to_string(), source.nfd().collect()] {
                    let before = text.clone();
                    let issues = super::super::grammar::check_in(&text, lang);
                    let expected_old = if text == source { old.to_string() } else { old.nfd().collect() };
                    let issue = issues.iter().find(|i| i.message.starts_with("Le verbe doit s’accorder")).unwrap();
                    let start = text.find(&expected_old).unwrap();
                    assert_eq!((issue.start, issue.end), (start, start + expected_old.len()), "{text}");
                    assert_eq!(issue.suggestions, [new.to_string()], "{text}");
                    assert_eq!(issue.lang, lang);
                    assert_eq!(text, before);
                    let mut repaired = text.clone();
                    repaired.replace_range(issue.start..issue.end, new);
                    assert_eq!(&repaired[..start], &text[..start]);
                    assert_eq!(&repaired[start + new.len()..], &text[issue.end..]);
                    assert!(own(&repaired).is_empty(), "{repaired}");
                }
            }
        }
    }

    #[test]
    fn correct_unknown_ambiguous_and_object_readings_abstain() {
        for text in [
            "Ils mangent une pomme.",
            "Je parlais du trajet.",
            "Tu visites le musée.",
            "Elle préparerait le repas.",
            "Nous regardent les enfants.",
            "Vous parle le directeur.",
            "Il nous mange notre goûter.",
            "Elle vous parle souvent.",
            "Il parlions du trajet.",
            "Ils parlaient du trajet.",
            "Je aimer cette chanson.",
            "Je suis la piste.",
            "Ils invente un récit.",
            "Il Parles de ce village.",
            "IL PARLES.",
            "Hier ils mange une pomme.",
            "🧭 Ils mange une pomme.",
            "Il dit : ils mange une pomme.",
            "Ils\nmange une pomme.",
            "Ils « mange » une pomme.",
            "Ils mange-t-il.",
            "Ils mange\u{301} une pomme.",
            "Ilsmange une pomme.",
            "",
        ] {
            assert!(own(text).is_empty(), "{text}: {:?}", own(text));
        }
    }

    #[test]
    fn subject_context_cannot_cross_a_language_or_region_boundary() {
        let text = "Ils mange une pomme.";
        let verb = text.find("mange").unwrap();
        let end = verb + "mange".len();
        let own_issue = |issues: Vec<Issue>| issues.into_iter().filter(|i| i.message.starts_with("Le verbe doit s’accorder")).collect::<Vec<_>>();
        for lang in [ProofLang::Fr, ProofLang::FrCa] {
            assert_eq!(own_issue(crate::check_text(text, &[(0..text.len(), lang)])).len(), 1);
            assert_eq!(own_issue(crate::check_text(text, &[(0..verb, lang), (verb..text.len(), lang)])).len(), 1);
            for other in [ProofLang::En, if lang == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                for cut in [1, verb, end - 1] {
                    assert!(own_issue(crate::check_text(text, &[(0..cut, other), (cut..text.len(), lang)])).is_empty());
                    assert!(own_issue(crate::check_text(text, &[(0..cut, lang), (cut..text.len(), other)])).is_empty());
                }
            }
        }
    }
    /// Authored independently before source review; exposed development, not a blind score.
    /// Quiet entries also include explicit scope abstentions, not only certified correct prose.
    #[test]
    fn independent_original_agreement_challenge() {
        for (id, text, expected) in [
            ("Q01", "Je portais déjà le ruban violet.", None),
            ("Q02", "Tu portais déjà le ruban violet.", None),
            ("Q03", "Il porte la caisse lilas.", None),
            ("Q04", "Elles porteront la caisse lilas.", None),
            ("Q05", "On porterait volontiers la caisse lilas.", None),
            ("Q06", "Ils portèrent la caisse lilas.", None),
            ("Q07", "Tu préparerais la maquette turquoise.", None),
            ("Q08", "Tu préparerais la maquette turquoise.", None),
            ("Q09", "Elle commençait le dessin du phare violet.", None),
            ("Q10", "Il mangeait la galette du phare violet.", None),
            ("Q11", "Il nous parle de la maquette turquoise.", None),
            ("Q12", "Elle vous regarde depuis le phare violet.", None),
            ("Q13", "Nous parle le guide du phare violet.", None),
            ("Q14", "Vous regarde le gardien du phare violet.", None),
            ("Q15", "Il ne parles jamais du phare violet.", None),
            ("Q16", "Elle me regardes près du phare violet.", None),
            ("Q17", "« Il parlons », titre fictif de la maquette turquoise.", None),
            ("Q18", "Exemple : Il parlons sur la maquette turquoise.", None),
            ("Q19", "Il — parles de la maquette turquoise.", None),
            ("Q20", "Il\nparles de la maquette turquoise.", None),
            ("Q21", "Il-Parles est une marque fictive de la maquette turquoise.", None),
            ("Q22", "Il Paris représente un nom fictif sur notre maquette.", None),
            ("Q23", "Elle réviserait un exemplaire orange.", None),
            ("Q24", "Ils envoient une carte du phare violet.", None),
            ("Q25", "Tu joues avec le modèle turquoise.", None),
            ("Q26", "Il regardé la maquette turquoise.", None),
            ("Q27", "Il parlant de la maquette turquoise.", None),
            ("Q28", "Elle a porté la caisse lilas.", None),
            ("Q29", "Ils ont parlé du phare violet.", None),
            ("Q30", "On est partis vers le phare violet.", None),
            ("Q31", "La notice indique il parlons sur fond violet.", None),
            ("Q32", "Tu marches malgré la pluie violette.", None),
            ("A01", "Je parlions du phare violet.", None),
            ("A02", "Il regardions la maquette turquoise.", None),
            ("A03", "Ils portiez la caisse lilas.", None),
            ("A04", "Elle chantions pour la maquette turquoise.", None),
            ("A05", "Il suis le guide du phare violet.", None),
            ("A06", "Je mangions près du phare violet.", None),
            ("P01", "Il portais la caisse lilas.", Some("portait")),
            ("P02", "Elles donnais une carte du phare violet.", Some("donnaient")),
            ("P03", "Ils joua près de la maquette turquoise.", Some("jouèrent")),
            ("P04", "Je dansâmes autour du phare violet.", Some("dansai")),
            ("P05", "Il donnasses une carte du phare violet.", Some("donnât")),
            ("P06", "Il travaillerais sur la maquette turquoise.", Some("travaillerait")),
            ("P07", "Il préparerais la maquette turquoise.", Some("préparerait")),
            ("P08", "  Il portais la caisse lilas.", Some("portait")),
            ("P09", "Il mangeais la galette du phare violet.", Some("mangeait")),
            ("P10", "Elle commençais le dessin du phare violet.", Some("commençait")),
            ("P11", "Il parlent de la maquette turquoise.", Some("parle")),
            ("P12", "Tu donne la carte du phare violet.", Some("donnes")),
        ] {
            let issues = own(text);
            match expected {
                Some(replacement) => {
                    assert_eq!(issues.len(), 1, "{id}: {issues:?}");
                    assert_eq!(issues[0].suggestions, [replacement.to_string()], "{id}");
                    assert!(text.get(issues[0].start..issues[0].end).is_some(), "{id}");
                }
                None => assert!(issues.is_empty(), "{id}: {issues:?}"),
            }
        }
    }
    #[test]
    fn independent_original_irregular_challenge() {
        for (id, text, expected) in [
            ("I01", "Tu suis la trace violette.", None),
            ("I02", "Je suis la trace violette.", None),
            ("I03", "Il suis la trace violette.", None),
            ("I04", "On suis la trace violette.", None),
            ("I05", "Ils suis les traces violettes.", None),
            ("I06", "Il suivions la trace violette.", None),
            ("I07", "Je suivions la trace violette.", None),
            ("I08", "Ils aient une carte violette.", None),
            ("I09", "J’aie une carte violette.", None),
            ("I10", "Il suivi la trace violette.", None),
            ("I11", "Il nous suis la trace violette.", None),
            ("I12", "Vous sommes près du phare violet.", None),
            ("I13", "Il est près du phare violet.", None),
            ("I14", "Tu avais une carte violette.", None),
            ("I15", "Ils fassent une carte violette.", None),
            ("I16", "Il es près du phare violet.", Some("est")),
            ("I17", "Ils sommes près du phare violet.", Some("sont")),
            ("I18", "Il as une carte violette.", Some("a")),
            ("I19", "Ils avais une carte violette.", Some("avaient")),
            ("I20", "J’étiez près du phare violet.", Some("étais")),
            ("I21", "Il aie une carte violette.", Some("ait")),
            ("I22", "Il suivez la trace violette.", Some("suit")),
            ("I23", "Je suit la trace violette.", Some("suis")),
            ("I24", "Il suivis la trace violette.", None),
            ("I27", "Ils suivis par le gardien violet.", None),
            ("I28", "Ils eus une surprise violette.", None),
            ("I25", "Ils suivît la trace violette.", Some("suivissent")),
            ("I26", "J’étiez près du phare violet.", Some("étais")),
        ] {
            let issues = own(text);
            match expected {
                Some(replacement) => {
                    assert_eq!(issues.len(), 1, "{id}: {issues:?}");
                    assert_eq!(issues[0].suggestions, [replacement.to_string()], "{id}");
                }
                None => assert!(issues.is_empty(), "{id}: {issues:?}"),
            }
        }
    }
}
