//! Direct personal-subject agreement, preserving finite and nominal ambiguity.
use super::{
    grammar,
    morphology::{Lexicon, Person},
};
use crate::Issue;

pub(super) const MESSAGE: &str = "Accord du verbe avec le pronom sujet";
pub(super) const AUXILIARY_MESSAGE: &str = "Auxiliaire être avec un verbe pronominal";

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
    clitics: Vec<Clitic>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Clitic {
    Me,
    Te,
    Se,
    Nous,
    Vous,
    Le,
    La,
    ElidedL,
    Les,
    Lui,
    Leur,
    Y,
    En,
}

impl Clitic {
    fn rank(self) -> u8 {
        match self {
            Self::Me | Self::Te | Self::Se | Self::Nous | Self::Vous => 1,
            Self::Le | Self::La | Self::ElidedL | Self::Les => 2,
            Self::Lui | Self::Leur => 3,
            Self::Y => 4,
            Self::En => 5,
        }
    }
    fn word(self) -> Option<&'static str> {
        Some(match self {
            Self::Me => "me",
            Self::Te => "te",
            Self::Se => "se",
            Self::Nous => "nous",
            Self::Vous => "vous",
            Self::Le => "le",
            Self::La => "la",
            Self::ElidedL => return None,
            Self::Les => "les",
            Self::Lui => "lui",
            Self::Leur => "leur",
            Self::Y => "y",
            Self::En => "en",
        })
    }
    fn short(self) -> Option<&'static str> {
        match self {
            Self::Me => Some("m"),
            Self::Te => Some("t"),
            Self::Se => Some("s"),
            Self::Le | Self::La | Self::ElidedL => Some("l"),
            _ => None,
        }
    }
    fn full(word: &str) -> Option<Self> {
        Some(match word {
            "me" => Self::Me,
            "te" => Self::Te,
            "se" => Self::Se,
            "nous" => Self::Nous,
            "vous" => Self::Vous,
            "le" => Self::Le,
            "la" => Self::La,
            "les" => Self::Les,
            "lui" => Self::Lui,
            "leur" => Self::Leur,
            "y" => Self::Y,
            "en" => Self::En,
            _ => return None,
        })
    }
    fn contracted(word: &str) -> Option<Self> {
        match word {
            "m" => Some(Self::Me),
            "t" => Some(Self::Te),
            "s" => Some(Self::Se),
            "l" => Some(Self::ElidedL),
            _ => None,
        }
    }
    fn reflexive(self, person: Person) -> bool {
        matches!(
            (self, person),
            (Self::Me, Person::FirstSingular)
                | (Self::Te, Person::SecondSingular)
                | (Self::Se, Person::ThirdSingular | Person::ThirdPlural)
                | (Self::Nous, Person::FirstPlural)
                | (Self::Vous, Person::SecondPlural)
        )
    }
    fn finite_context(self) -> bool {
        matches!(self, Self::Me | Self::Te | Self::Se | Self::Nous | Self::Vous | Self::Lui)
    }
}

fn push_clitic(clitics: &mut Vec<Clitic>, clitic: Clitic, target: Person) -> Option<()> {
    if clitics.len() >= 3
        || clitics.last().is_some_and(|last| last.rank() >= clitic.rank())
        || (clitic == Clitic::Se && !matches!(target, Person::ThirdSingular | Person::ThirdPlural))
    {
        return None;
    }
    clitics.push(clitic);
    Some(())
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
    if first.len() > 160 || first.chars().skip(1).any(char::is_uppercase) {
        return None;
    }
    let (start, mut end, mut part, target, je, mut index, contracted_je) = if let Some(form) =
        first.strip_prefix("J’").or_else(|| first.strip_prefix("j’")).or_else(|| first.strip_prefix("J'")).or_else(|| first.strip_prefix("j'"))
    {
        (a, b, form, Person::FirstSingular, true, 0usize, true)
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
        let &(start, end) = ws.get(1)?;
        if !horizontal(text.get(b..start)?) {
            return None;
        }
        (start, end, text.get(start..end)?, target, lower == "je", 1usize, false)
    };
    let mut negative = false;
    let mut clitics = Vec::new();
    let mut form = None;
    // At most ne + three ordered clitics + the finite form. Apostrophe-linked parts
    // count too; no unbounded skipping or crossing punctuation/line breaks.
    for _ in 0..5 {
        if part.is_empty() {
            index = index.saturating_add(1);
            let &(c, d) = ws.get(index)?;
            if !horizontal(text.get(end..c)?) {
                return None;
            }
            part = text.get(c..d)?;
            end = d;
        }
        if part.len() > 160 {
            return None;
        }
        if let Some((head, tail)) = part.split_once(['’', '\'']) {
            if head == "n" && !negative && clitics.is_empty() {
                negative = true;
            } else {
                push_clitic(&mut clitics, Clitic::contracted(head)?, target)?;
            }
            if tail.is_empty() {
                return None;
            }
            part = tail;
        } else if part == "ne" && !negative && clitics.is_empty() {
            negative = true;
            part = "";
        } else if let Some(clitic) = Clitic::full(part) {
            push_clitic(&mut clitics, clitic, target)?;
            part = "";
        } else {
            form = Some(part);
            break;
        }
    }
    let form = form?;
    let next = index.saturating_add(1);
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
        clitics,
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

fn check_candidate(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon, candidate: Candidate<'_>) -> Option<Issue> {
    let (forms, message) = if candidate.clitics.iter().any(|c| c.reflexive(candidate.target)) && lexicon.has_finite_lemma(candidate.form, "avoir") {
        if !super::conjugation::is_prose(text) {
            return None;
        }
        let index = super::conjugation::participle_index(text, ws, candidate.next, candidate.negative)?;
        let &(start, end) = ws.get(index)?;
        let participle = text.get(start..end).filter(|word| word.len() <= 128 && word.chars().all(char::is_lowercase))?;
        let &(_, previous_end) = ws.get(index.checked_sub(1)?)?;
        if !horizontal(text.get(previous_end..start)?)
            || ws.iter().skip(index.saturating_add(1)).take(8).any(|&(a, b)| {
                text.get(a..b).is_some_and(|word| {
                    word.len() <= 32
                        && matches!(
                            word.to_lowercase().as_str(),
                            "mot" | "mots" | "verbe" | "verbes" | "infinitif" | "infinitifs" | "forme" | "formes" | "exemple" | "exemples"
                        )
                })
            })
        {
            return None;
        }
        (lexicon.reflexive_auxiliary_forms(candidate.form, participle, candidate.target)?, AUXILIARY_MESSAGE)
    } else {
        (lexicon.agreement_forms(candidate.form, candidate.target)?, MESSAGE)
    };
    if !candidate.negative
        && matches!(candidate.target, Person::FirstPlural | Person::SecondPlural | Person::ThirdPlural)
        && lexicon.has_nonverbal_reading(candidate.form)
        && !candidate.clitics.iter().any(|c| c.finite_context())
        && (!lexicon.has_auxiliary_reading(candidate.form) || nominal_context(text, ws, &candidate, lexicon))
    {
        return None;
    }
    let mut start = candidate.start;
    let suggestions = if !candidate.clitics.is_empty() {
        let mut rendered = Vec::new();
        for form in forms {
            let mut first = form.as_str();
            let mut phrase = form.clone();
            for clitic in candidate.clitics.iter().rev() {
                let elides = if clitic.short().is_some() { known_elision(first)? } else { false };
                if elides {
                    phrase = format!("{}’{phrase}", clitic.short()?);
                    first = clitic.word().unwrap_or("le");
                } else {
                    // An elided l’ lost le/la gender; do not guess it when uneliding.
                    let word = clitic.word()?;
                    phrase = format!("{word} {phrase}");
                    first = word;
                }
            }
            if candidate.negative {
                phrase = if known_elision(first)? { format!("n’{phrase}") } else { format!("ne {phrase}") };
                first = "ne";
            }
            if candidate.je && (candidate.contracted_je || known_elision(first)?) {
                start = candidate.subject_start;
                phrase = if known_elision(first)? {
                    format!("{}’{phrase}", if candidate.capital { "J" } else { "j" })
                } else {
                    format!("{} {phrase}", if candidate.capital { "Je" } else { "je" })
                };
            }
            rendered.push(phrase);
        }
        rendered
    } else if candidate.negative {
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
    Some(grammar::issue(start, candidate.end, message.into(), suggestions))
}

pub(super) fn check(text: &str, ws: &[(usize, usize)]) -> Option<Issue> {
    let parsed = candidate(text, ws)?; // Avoid optional-data loading for unrelated paragraphs.
    check_candidate(text, ws, &super::morphology::lexicon(), parsed)
}

/// The auxiliary decision uses the subject, clitic chain, bridge and participle.
/// All of them must belong to the same French region, even though only the prefix
/// is replaced. Reparse without loading data; a malformed context stays undecided.
pub(super) fn auxiliary_context_range(text: &str) -> Option<std::ops::Range<usize>> {
    let ws = crate::words(text);
    let candidate = candidate(text, &ws)?;
    let index = super::conjugation::participle_index(text, &ws, candidate.next, candidate.negative)?;
    let &(_, end) = ws.get(index)?;
    text.get(candidate.subject_start..end)?;
    Some(candidate.subject_start..end)
}

#[cfg(test)]
fn check_with(text: &str, ws: &[(usize, usize)], lexicon: &Lexicon) -> Option<Issue> {
    check_candidate(text, ws, lexicon, candidate(text, ws)?)
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
            ("Nous lui a parlé.", "lui a", "lui avons"),
            ("Nous ne lui a parlé.", "ne lui a", "ne lui avons"),
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

    #[test]
    fn clitics_and_negation_are_retained_when_person_changes() {
        for (text, wrong, right) in [
            ("Tu me est cher.", "me est", "m’es"),
            ("Elle te es chère.", "te es", "t’est"),
            ("Il s’es préparé.", "s’es", "s’est"),
            ("Je m’est préparé.", "m’est", "me suis"),
            ("Nous nous est trompés.", "nous est", "nous sommes"),
            ("Vous vous es trompés.", "vous es", "vous êtes"),
            ("Il y ont une place.", "y ont", "y a"),
            ("Ils y a une place.", "y a", "y ont"),
            ("Il y en ont deux.", "y en ont", "y en a"),
            ("Il n’y en ont pas.", "n’y en ont", "n’y en a"),
            ("Je n’en ont pas.", "n’en ont", "n’en ai"),
            ("J’y est bien.", "J’y est", "J’y suis"),
            ("J’en ont besoin.", "J’en ont", "J’en ai"),
            ("Je en ont besoin.", "Je en ont", "J’en ai"),
            ("Je l’a lu.", "l’a", "l’ai"),
            ("Tu l’ont lu.", "l’ont", "l’as"),
            ("Je le avons lu.", "le avons", "l’ai"),
            ("Je la avons vue.", "la avons", "l’ai"),
            ("Je le est.", "le est", "le suis"),
            ("Il le es.", "le es", "l’est"),
            ("Je ne me y êtes préparé.", "ne me y êtes", "ne m’y suis"),
            ("Tu me y ont conduit.", "me y ont", "m’y as"),
        ] {
            let issue = found(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(&text[issue.start..issue.end], wrong, "{text}");
            assert_eq!(issue.suggestions, [right], "{text}");
            let fixed = format!("{}{right}{}", &text[..issue.start], &text[issue.end..]);
            assert!(found(&fixed).is_none(), "{fixed}");
        }
        // Ambiguous/unsupported errors, not correct-text precision evidence.
        let ambiguous = found("Elle te sommes.").unwrap();
        assert_eq!(ambiguous.suggestions, ["t’est", "te somme"], "both être and sommer readings remain available");
        for text in [
            "Je l’est.",
            "Je me as offert un cadeau.",
            "Il s’ont trompé.",
            "Nous se est trompés.",
            "Je lui le ont donné.",
            "Je me me ont parlé.",
            "Je me le lui y ont donné.",
            "Je les; est.",
        ] {
            assert!(found(text).is_none(), "{text}");
        }
    }
}
