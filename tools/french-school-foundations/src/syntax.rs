//! Bounded structural hypotheses for one subject followed by one finite verb.
//! No agreement filtering, semantic interpretation, diagnosis or correction.
use crate::{Reading, Token};
use serde::Serialize;

pub const MAX_SYNTAX_TOKENS: usize = 128;
pub const MAX_SYNTAX_READINGS: usize = 64;
pub const MAX_SYNTAX_HYPOTHESES: usize = 16;
const MAX_DEPTH: usize = 2;
const MAX_WORK: usize = 4096;

#[derive(Debug, Serialize)]
pub struct SyntaxAnalysis {
    pub statut: &'static str,
    pub portee: &'static str,
    pub hypotheses: Vec<SyntaxHypothesis>,
    pub raisons: Vec<&'static str>,
    pub limites: [&'static str; 4],
}

#[derive(Debug, Serialize)]
pub struct SyntaxHypothesis {
    pub groupes: Vec<Group>,
    pub liens: Vec<Link>,
    pub raisons: Vec<&'static str>,
    /// Original token indexes; this tail has no syntactic function assigned.
    pub reste_non_analyse_tokens: Option<[usize; 2]>,
}

#[derive(Debug, Serialize)]
pub struct Group {
    pub categorie: &'static str,
    /// Half-open token indexes in the original lexical token array.
    pub plage_tokens: [usize; 2],
    pub plage_utf8: [usize; 2],
    pub plage_utf16: [usize; 2],
    pub noyau_token: usize,
    /// Indexes into tokens[noyau_token].lectures, never copied/rewritten readings.
    pub lectures_noyau: Vec<usize>,
    pub enfants: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct Link {
    pub fonction: &'static str,
    pub source_groupe: usize,
    pub cible_token: usize,
}

#[derive(Clone)]
struct NounPhrase {
    start: usize,
    end: usize,
    head: usize,
    readings: Vec<usize>,
    adjectives: Vec<usize>,
    complement: Option<Box<NounPhrase>>,
}

struct Budget {
    remaining: usize,
    exhausted: bool,
}
impl Budget {
    fn spend(&mut self) -> bool {
        if self.remaining == 0 {
            self.exhausted = true;
            return false;
        }
        self.remaining -= 1;
        true
    }
}

fn trait_is(reading: &Reading, key: &str, value: &str) -> bool {
    reading.traits.get(key).is_some_and(|actual| actual == value)
}
fn readings(token: &Token, nature: &str) -> Vec<usize> {
    token.lectures.iter().enumerate().filter_map(|(i, r)| (r.nature == nature).then_some(i)).collect()
}
fn determiner(token: &Token) -> bool {
    token.lectures.iter().any(|r| r.nature == "determinant" && !trait_is(r, "type", "article_contracte"))
}
fn subject_pronoun(token: &Token) -> Vec<usize> {
    token
        .lectures
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            (r.nature == "pronom"
                && trait_is(r, "type", "personnel")
                && matches!(r.lemme.as_str(), "je" | "tu" | "il" | "elle" | "on" | "nous" | "vous"))
            .then_some(i)
        })
        .collect()
}
fn finite_readings(token: &Token) -> Vec<usize> {
    token
        .lectures
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            (r.nature == "verbe" && r.traits.get("mode").is_some_and(|m| matches!(m.as_str(), "indicatif" | "subjonctif" | "conditionnel")))
                .then_some(i)
        })
        .collect()
}
fn complement_marker(token: &Token) -> bool {
    token.lectures.iter().any(|r| {
        (r.nature == "preposition" && r.lemme == "de")
            || (r.nature == "determinant" && trait_is(r, "type", "article_contracte") && trait_is(r, "preposition", "de"))
    })
}
fn contracted_marker(token: &Token) -> bool {
    token.lectures.iter().any(|r| r.nature == "determinant" && trait_is(r, "type", "article_contracte") && trait_is(r, "preposition", "de"))
}

fn noun_phrases(tokens: &[Token], start: usize, end: usize, depth: usize, implicit_determiner: bool, budget: &mut Budget) -> Vec<NounPhrase> {
    if !budget.spend() || start >= end {
        return Vec::new();
    }
    let Some(first) = tokens.get(start) else {
        return Vec::new();
    };
    let mut position = start;
    if determiner(first) {
        position += 1;
    } else if !implicit_determiner {
        return Vec::new();
    }
    let mut adjectives = Vec::new();
    let mut result = Vec::new();
    while position < end && budget.spend() {
        let Some(token) = tokens.get(position) else {
            break;
        };
        let nominal_readings = readings(token, "nom");
        if !nominal_readings.is_empty() {
            let base =
                NounPhrase { start, end: position + 1, head: position, readings: nominal_readings, adjectives: adjectives.clone(), complement: None };
            result.push(base.clone());
            if depth < MAX_DEPTH
                && let Some(marker) = tokens.get(position + 1).filter(|_| position + 1 < end)
                && complement_marker(marker)
            {
                let implicit = contracted_marker(marker);
                for mut child in noun_phrases(tokens, position + 2, end, depth + 1, implicit, budget) {
                    child.start = position + 1;
                    let mut extended = base.clone();
                    extended.end = child.end;
                    extended.complement = Some(Box::new(child));
                    result.push(extended);
                    if result.len() > MAX_SYNTAX_HYPOTHESES {
                        budget.exhausted = true;
                        return Vec::new();
                    }
                }
            }
        }
        if readings(token, "adjectif").is_empty() {
            break;
        }
        adjectives.push(position);
        position += 1;
    }
    result
}

fn make_group(tokens: &[Token], category: &'static str, start: usize, end: usize, head: usize, selected: Vec<usize>) -> Option<Group> {
    let first = tokens.get(start)?;
    let last = tokens.get(end.checked_sub(1)?)?;
    Some(Group {
        categorie: category,
        plage_tokens: [start, end],
        plage_utf8: [first.plage_utf8[0], last.plage_utf8[1]],
        plage_utf16: [first.plage_utf16[0], last.plage_utf16[1]],
        noyau_token: head,
        lectures_noyau: selected,
        enfants: Vec::new(),
    })
}
fn emit_nominal(tokens: &[Token], phrase: &NounPhrase, hypothesis: &mut SyntaxHypothesis) -> Option<usize> {
    let id = hypothesis.groupes.len();
    hypothesis.groupes.push(make_group(tokens, "groupe_nominal", phrase.start, phrase.end, phrase.head, phrase.readings.clone())?);
    for &adjective in &phrase.adjectives {
        let token = tokens.get(adjective)?;
        let child = hypothesis.groupes.len();
        hypothesis.groupes.push(make_group(tokens, "groupe_adjectival", adjective, adjective + 1, adjective, readings(token, "adjectif"))?);
        hypothesis.groupes.get_mut(id)?.enfants.push(child);
        hypothesis.liens.push(Link { fonction: "epithete", source_groupe: child, cible_token: phrase.head });
    }
    if let Some(complement) = &phrase.complement {
        let child = emit_nominal(tokens, complement, hypothesis)?;
        hypothesis.groupes.get_mut(child)?.categorie = "groupe_prepositionnel_complement_du_nom";
        hypothesis.groupes.get_mut(id)?.enfants.push(child);
        hypothesis.liens.push(Link { fonction: "complement_du_nom", source_groupe: child, cible_token: phrase.head });
    }
    Some(id)
}
fn hypothesis(tokens: &[Token], subject: &NounPhrase, pronoun: bool, verb: usize, selected: Vec<usize>) -> Option<SyntaxHypothesis> {
    let mut result = SyntaxHypothesis {
        groupes: Vec::new(),
        reste_non_analyse_tokens: None,
        liens: Vec::new(),
        raisons: vec![
            "Hypothèse de structure locale, sans contrôle d’accord ni interprétation du sens.",
            "Les lectures lexicales restent intactes ; l’impératif est exclu du verbe de cette hypothèse déclarative à sujet exprimé.",
        ],
    };
    let subject_id = if pronoun {
        result.groupes.push(make_group(tokens, "groupe_pronominal", subject.start, subject.end, subject.head, subject.readings.clone())?);
        0
    } else {
        emit_nominal(tokens, subject, &mut result)?
    };
    result.groupes.push(make_group(tokens, "groupe_verbal_minimal", verb, verb + 1, verb, selected)?);
    result.liens.push(Link { fonction: "sujet", source_groupe: subject_id, cible_token: verb });
    Some(result)
}
fn empty(reason: &'static str) -> SyntaxAnalysis {
    SyntaxAnalysis {
        statut: "abstention",
        portee: "hypotheses_declarative_sujet_verbe_minimal",
        hypotheses: Vec::new(),
        raisons: vec![reason],
        limites: [
            "Une phrase, sujet antéposé et verbe déclaratif ; un GN restant peut être laissé non analysé après certaines formes pronominales.",
            "GN : déterminant, adjectifs antéposés, noyau nominal ; complément de/des/du/d’ avec GN limité à deux imbrications.",
            "Objets, clitiques, attributs, négation, coordination, relatives, temps composés et interprétation sont hors portée.",
            "Aucun accord vérifié, aucun choix de correction ; un résultat est une hypothèse, pas une preuve d’unicité.",
        ],
    }
}

/// Inspect original lexical tokens without modifying their readings or selecting a correction.
pub fn inspect(tokens: &[Token]) -> SyntaxAnalysis {
    if tokens.is_empty() {
        return empty("Aucun segment à analyser.");
    }
    if tokens.len() > MAX_SYNTAX_TOKENS || tokens.iter().any(|t| t.lectures.len() > MAX_SYNTAX_READINGS) {
        return empty("Borne de taille syntaxique atteinte ; aucune hypothèse publiée.");
    }
    let end = tokens.len() - usize::from(tokens.last().is_some_and(|t| t.type_segment == "ponctuation" && t.texte == "."));
    let Some(content) = tokens.get(..end) else {
        return empty("Segments indisponibles.");
    };
    if content.len() < 2 || content.iter().any(|t| t.type_segment != "mot" || t.lectures.is_empty()) {
        return empty("Segment inconnu, ponctuation interne ou structure hors portée.");
    }
    let mut budget = Budget { remaining: MAX_WORK, exhausted: false };
    let mut subjects = noun_phrases(tokens, 0, end, 0, false, &mut budget).into_iter().map(|np| (np, false)).collect::<Vec<_>>();
    if let Some(first) = tokens.first() {
        let selected = subject_pronoun(first);
        if !selected.is_empty() {
            subjects.push((NounPhrase { start: 0, end: 1, head: 0, readings: selected, adjectives: Vec::new(), complement: None }, true));
        }
    }
    let mut result = empty("Aucune structure sujet antéposé + verbe fini seul dans la portée prise en charge.");
    for (subject, pronoun) in subjects {
        if !budget.spend() {
            break;
        }
        if subject.end >= end {
            continue;
        }
        let tail = if subject.end + 1 < end {
            // Only explicitly supported initial pronoun forms admit an opaque GN tail.
            // In particular, nous/vous may be preverbal objects in an inverted frame.
            let supported = pronoun
                && tokens.get(subject.head).is_some_and(|t| {
                    subject
                        .readings
                        .iter()
                        .any(|&i| t.lectures.get(i).is_some_and(|r| matches!(r.lemme.as_str(), "je" | "tu" | "il" | "elle" | "on")))
                });
            if !supported {
                continue;
            }
            let tail_start = subject.end + 1;
            if tokens.get(tail_start..end).is_none_or(|rest| rest.iter().any(|t| t.lectures.iter().any(|r| r.nature == "verbe"))) {
                continue;
            }
            if !noun_phrases(tokens, tail_start, end, 0, false, &mut budget).iter().any(|np| np.end == end) {
                continue;
            }
            Some([tail_start, end])
        } else {
            None
        };
        let Some(verb) = tokens.get(subject.end) else {
            continue;
        };
        let selected = finite_readings(verb);
        if selected.is_empty() {
            continue;
        }
        if let Some(mut candidate) = hypothesis(tokens, &subject, pronoun, subject.end, selected) {
            candidate.reste_non_analyse_tokens = tail;
            if tail.is_some() {
                candidate.raisons.push("Le groupe restant borne la recherche, mais sa fonction (objet, attribut ou autre) reste non analysée.");
            }
            result.hypotheses.push(candidate);
            if result.hypotheses.len() > MAX_SYNTAX_HYPOTHESES {
                budget.exhausted = true;
                break;
            }
        }
    }
    if budget.exhausted {
        return empty("Borne de recherche syntaxique atteinte ; aucune hypothèse tronquée publiée.");
    }
    if !result.hypotheses.is_empty() {
        let ambiguous = result.hypotheses.len() > 1 || content.iter().any(|t| t.lectures.len() > 1);
        result.statut = if result.hypotheses.iter().any(|h| h.reste_non_analyse_tokens.is_some()) {
            "hypotheses_partielles_avec_reste_non_analyse"
        } else if ambiguous {
            "hypotheses_avec_ambiguite_lexicale"
        } else {
            "hypothese_dans_la_portee"
        };
        result.raisons = vec!["Structure locale couverte ; toutes les lectures lexicales d’origine sont conservées dans les tokens."];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn subject_head(analysis: &SyntaxAnalysis) -> Option<usize> {
        let hypothesis = analysis.hypotheses.first()?;
        let link = hypothesis.liens.iter().find(|link| link.fonction == "sujet")?;
        hypothesis.groupes.get(link.source_groupe).map(|g| g.noyau_token)
    }
    #[test]
    fn nominal_complement_does_not_become_subject() -> Result<(), Box<dyn std::error::Error>> {
        let lexical = crate::analyze("Le chien des voisins aboie.")?;
        let syntax = inspect(&lexical.tokens);
        assert_eq!(subject_head(&syntax), Some(1));
        assert!(syntax.hypotheses.iter().any(|h| h.liens.iter().any(|l| l.fonction == "complement_du_nom" && l.cible_token == 1)));
        Ok(())
    }
    #[test]
    fn agreement_never_gates_subject() -> Result<(), Box<dyn std::error::Error>> {
        for text in ["Ils joue.", "Le chiens aboie.", "Les petits chiens jouent."] {
            let lexical = crate::analyze(text)?;
            let syntax = inspect(&lexical.tokens);
            assert!(!syntax.hypotheses.is_empty(), "{text}");
        }
        Ok(())
    }
    #[test]
    fn ambiguity_and_imperative_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let lexical = crate::analyze("Je suis.")?;
        let syntax = inspect(&lexical.tokens);
        let selected =
            syntax.hypotheses.iter().flat_map(|h| &h.groupes).find(|g| g.categorie == "groupe_verbal_minimal").map(|g| g.lectures_noyau.len());
        assert_eq!(selected, Some(3));
        assert!(lexical.tokens.get(1).is_some_and(|t| t.lectures.iter().any(|r| trait_is(r, "mode", "imperatif"))));
        let lexical = crate::analyze("La porte porte.")?;
        let syntax = inspect(&lexical.tokens);
        assert_eq!(subject_head(&syntax), Some(1));
        assert!(lexical.tokens.get(1).is_some_and(|t| t.lectures.iter().any(|r| r.nature == "verbe")));
        Ok(())
    }
    #[test]
    fn refuses_unsupported_and_unknown_frames() -> Result<(), Box<dyn std::error::Error>> {
        for text in [
            "Les zorpules jouent.",
            "Il la porte.",
            "Il a joue.",
            "Le chien et le chat jouent.",
            "Le chien qui joue aboie.",
            "Il joue ?",
            "Il joue. Il parle.",
        ] {
            let lexical = crate::analyze(text)?;
            assert!(inspect(&lexical.tokens).hypotheses.is_empty(), "{text}");
        }
        Ok(())
    }
    #[test]
    fn size_bound_abstains_without_partial_structure() -> Result<(), Box<dyn std::error::Error>> {
        let lexical = crate::analyze(&"chien ".repeat(MAX_SYNTAX_TOKENS + 1))?;
        assert!(inspect(&lexical.tokens).hypotheses.is_empty());
        assert_eq!(inspect(&[]).statut, "abstention");
        Ok(())
    }
    #[test]
    fn original_unicode_spans_survive_groups() -> Result<(), Box<dyn std::error::Error>> {
        let lexical = crate::analyze("L’e\u{301}cole joue.")?;
        let syntax = inspect(&lexical.tokens);
        let nominal = syntax.hypotheses.iter().flat_map(|h| &h.groupes).find(|g| g.categorie == "groupe_nominal");
        assert!(nominal.is_some_and(|g| g.plage_utf8 == [0, 11] && g.plage_utf16 == [0, 8]));
        Ok(())
    }
}
