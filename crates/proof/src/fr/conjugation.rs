//! Bounded subject + avoir + infinitive checks, with original fallback facts and optional data.
//!
//! At the start of a paragraph, a subject immediately followed by correctly matched `avoir`
//! needs a participle: `J'avais écrire` → `J'avais écrit`. Five simple auxiliary tenses are
//! supported. More infinitives/participles come from the optional MPL-2.0 lexicon.
//! No inference from an arbitrary `-er` ending, no agreement or auxiliary-choice guess,
//! no object clitics, no correction inside a quotation or a metalinguistic example.

use super::morphology::Person;
use crate::Issue;

/// Original small table of ordinary verb forms, not extracted from a linguistic corpus.
/// No nouns merely ending in `er` and no verbs whose compound form needs `être`.
const PARTICIPLES: &[(&str, &str)] = &[
    ("agir", "agi"),
    ("aider", "aidé"),
    ("aimer", "aimé"),
    ("appeler", "appelé"),
    ("apprendre", "appris"),
    ("attendre", "attendu"),
    ("avoir", "eu"),
    ("boire", "bu"),
    ("chanter", "chanté"),
    ("chercher", "cherché"),
    ("choisir", "choisi"),
    ("comprendre", "compris"),
    ("conduire", "conduit"),
    ("connaître", "connu"),
    ("construire", "construit"),
    ("courir", "couru"),
    ("croire", "cru"),
    ("demander", "demandé"),
    ("devoir", "dû"),
    ("dire", "dit"),
    ("donner", "donné"),
    ("écouter", "écouté"),
    ("écrire", "écrit"),
    ("entendre", "entendu"),
    ("étudier", "étudié"),
    ("être", "été"),
    ("faire", "fait"),
    ("fermer", "fermé"),
    ("finir", "fini"),
    ("fuir", "fui"),
    ("gagner", "gagné"),
    ("garder", "gardé"),
    ("jouer", "joué"),
    ("lire", "lu"),
    ("manger", "mangé"),
    ("mettre", "mis"),
    ("montrer", "montré"),
    ("offrir", "offert"),
    ("ouvrir", "ouvert"),
    ("parler", "parlé"),
    ("penser", "pensé"),
    ("perdre", "perdu"),
    ("permettre", "permis"),
    ("porter", "porté"),
    ("pouvoir", "pu"),
    ("prendre", "pris"),
    ("préparer", "préparé"),
    ("produire", "produit"),
    ("promettre", "promis"),
    ("recevoir", "reçu"),
    ("réfléchir", "réfléchi"),
    ("regarder", "regardé"),
    ("répondre", "répondu"),
    ("réussir", "réussi"),
    ("savoir", "su"),
    ("sentir", "senti"),
    ("suivre", "suivi"),
    ("tenir", "tenu"),
    ("travailler", "travaillé"),
    ("trouver", "trouvé"),
    ("utiliser", "utilisé"),
    ("vendre", "vendu"),
    ("visiter", "visité"),
    ("vivre", "vécu"),
    ("voir", "vu"),
    ("vouloir", "voulu"),
];

fn horizontal(gap: &str) -> bool {
    !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

pub(super) fn check(text: &str, ws: &[(usize, usize)], issues: &mut Vec<Issue>) {
    let Some(&(a, b)) = ws.first() else { return };
    let (Some(before), Some(first)) = (text.get(..a), text.get(a..b)) else { return };
    let leading = before.trim();
    let dialogue = matches!(leading, "—" | "–" | "-") && before.chars().last().is_some_and(char::is_whitespace);
    if (!leading.is_empty() && !dialogue) || first.len() > 16 || first.chars().skip(1).any(char::is_uppercase) {
        return;
    }
    let lower = first.to_lowercase().replace('’', "'");
    let (verb_index, subject, impersonal_il) = if matches!(lower.as_str(), "j'ai" | "j'avais" | "j'aurai" | "j'aurais" | "j'eus") {
        (1, Person::FirstSingular, false)
    } else {
        let (person, auxiliaries) = match lower.as_str() {
            "je" => (Person::FirstSingular, ["ai", "avais", "aurai", "aurais", "eus"]),
            "tu" => (Person::SecondSingular, ["as", "avais", "auras", "aurais", "eus"]),
            "il" | "elle" | "on" => (Person::ThirdSingular, ["a", "avait", "aura", "aurait", "eut"]),
            "nous" => (Person::FirstPlural, ["avons", "avions", "aurons", "aurions", "eûmes"]),
            "vous" => (Person::SecondPlural, ["avez", "aviez", "aurez", "auriez", "eûtes"]),
            "ils" | "elles" => (Person::ThirdPlural, ["ont", "avaient", "auront", "auraient", "eurent"]),
            _ => return,
        };
        let Some(&(c, d)) = ws.get(1) else { return };
        if !text.get(c..d).is_some_and(|aux| auxiliaries.contains(&aux)) || !text.get(b..c).is_some_and(horizontal) {
            return;
        }
        (2, person, lower == "il")
    };
    let (Some(&(start, end)), Some(&(_, previous_end))) = (ws.get(verb_index), ws.get(verb_index - 1)) else { return };
    let Some(word) = text.get(start..end) else { return };
    if word.len() > 128 || !word.chars().all(char::is_lowercase) {
        return;
    }
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
    let lexicon = super::morphology::lexicon();
    if lexicon.is_past_participle(word) {
        return;
    }
    // `avoir pouvoir (sur...)`, `avoir devoir de...` can use bare nouns. Require an
    // overt known infinitive complement before treating these two forms as modal verbs.
    if matches!(word, "pouvoir" | "devoir")
        && !following.first().is_some_and(|&(a, b)| {
            text.get(end..a).is_some_and(horizontal)
                && text.get(a..b).is_some_and(|next| {
                    lexicon.is_infinitive(next)
                        || PARTICIPLES.iter().any(|(infinitive, _)| *infinitive == next)
                        || matches!(
                            next,
                            "aller" | "venir" | "partir" | "revenir" | "arriver" | "sortir" | "rentrer" | "devenir" | "rester" | "mourir" | "naître"
                        )
                })
        })
    {
        return;
    }
    let direct_object = following.first().is_some_and(|&(a, b)| {
        text.get(end..a).is_some_and(horizontal)
            && matches!(
                text.get(a..b),
                Some(
                    "le" | "la"
                        | "les"
                        | "un"
                        | "une"
                        | "ce"
                        | "cet"
                        | "cette"
                        | "ces"
                        | "mon"
                        | "ma"
                        | "mes"
                        | "ton"
                        | "ta"
                        | "tes"
                        | "son"
                        | "sa"
                        | "ses"
                        | "notre"
                        | "nos"
                        | "votre"
                        | "vos"
                        | "leur"
                        | "leurs"
                )
            )
    });
    let suggestions = PARTICIPLES
        .iter()
        .find(|(infinitive, _)| *infinitive == word)
        .map(|(_, participle)| vec![(*participle).to_owned()])
        .or_else(|| lexicon.participle_forms(word, subject, impersonal_il, direct_object));
    let Some(suggestions) = suggestions else { return };
    let Some(participle) = suggestions.first() else { return };
    issues.push(super::grammar::issue(start, end, format!("Participe passé attendu après avoir : «\u{a0}{participle}\u{a0}»"), suggestions));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_verb_is_checked_for_each_unambiguous_subject() {
        for prefix in [
            ["J’ai", "J'ai", "Tu as", "Il a", "Elle a", "On a", "Nous avons", "Vous avez", "Ils ont", "Elles ont"],
            ["J’avais", "J'avais", "Tu avais", "Il avait", "Elle avait", "On avait", "Nous avions", "Vous aviez", "Ils avaient", "Elles avaient"],
            ["J’aurai", "J'aurai", "Tu auras", "Il aura", "Elle aura", "On aura", "Nous aurons", "Vous aurez", "Ils auront", "Elles auront"],
            [
                "J’aurais",
                "J'aurais",
                "Tu aurais",
                "Il aurait",
                "Elle aurait",
                "On aurait",
                "Nous aurions",
                "Vous auriez",
                "Ils auraient",
                "Elles auraient",
            ],
            ["J’eus", "J'eus", "Tu eus", "Il eut", "Elle eut", "On eut", "Nous eûmes", "Vous eûtes", "Ils eurent", "Elles eurent"],
        ]
        .into_iter()
        .flatten()
        {
            for &(infinitive, participle) in PARTICIPLES {
                let tail = if matches!(infinitive, "pouvoir" | "devoir") { " partir." } else { "." };
                let text = format!("{prefix} {infinitive}{tail}");
                let mut issues = Vec::new();
                check(&text, &crate::words(&text), &mut issues);
                assert_eq!(issues.len(), 1, "{text}");
                let issue = &issues[0];
                assert_eq!(&text[issue.start..issue.end], infinitive);
                assert_eq!(issue.suggestions, [participle]);
                let correct = format!("{prefix} {participle}{tail}");
                let mut quiet = Vec::new();
                check(&correct, &crate::words(&correct), &mut quiet);
                assert!(quiet.is_empty(), "{correct}");
            }
        }
    }
}
