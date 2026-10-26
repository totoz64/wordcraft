//! Original, bounded French verb morphology, independent of spelling dictionary flags.
//! All supported simple-tense readings are retained. Unknown lemmas have no analysis;
//! suffix resemblance never turns an arbitrary word into a verb.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Person {
    FirstSingular,
    SecondSingular,
    ThirdSingular,
    FirstPlural,
    SecondPlural,
    ThirdPlural,
}

impl Person {
    fn index(self) -> usize {
        match self {
            Self::FirstSingular => 0,
            Self::SecondSingular => 1,
            Self::ThirdSingular => 2,
            Self::FirstPlural => 3,
            Self::SecondPlural => 4,
            Self::ThirdPlural => 5,
        }
    }
}

const PERSONS: [Person; 6] = [
    Person::FirstSingular,
    Person::SecondSingular,
    Person::ThirdSingular,
    Person::FirstPlural,
    Person::SecondPlural,
    Person::ThirdPlural,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tense {
    Present,
    Imperfect,
    PastSimple,
    Future,
    Conditional,
    SubjunctivePresent,
    SubjunctiveImperfect,
    Imperative,
}

impl Tense {
    fn index(self) -> usize {
        match self {
            Self::Present => 0,
            Self::Imperfect => 1,
            Self::PastSimple => 2,
            Self::Future => 3,
            Self::Conditional => 4,
            Self::SubjunctivePresent => 5,
            Self::SubjunctiveImperfect => 6,
            Self::Imperative => 7,
        }
    }

    fn endings(self) -> [&'static str; 6] {
        match self {
            Self::Present => ["e", "es", "e", "ons", "ez", "ent"],
            Self::Imperfect => ["ais", "ais", "ait", "ions", "iez", "aient"],
            Self::PastSimple => ["ai", "as", "a", "âmes", "âtes", "èrent"],
            Self::Future => ["erai", "eras", "era", "erons", "erez", "eront"],
            Self::Conditional => ["erais", "erais", "erait", "erions", "eriez", "eraient"],
            Self::SubjunctivePresent => ["e", "es", "e", "ions", "iez", "ent"],
            Self::SubjunctiveImperfect => ["asse", "asses", "ât", "assions", "assiez", "assent"],
            Self::Imperative => ["", "e", "", "ons", "ez", ""],
        }
    }
}

const TENSES: [Tense; 8] = [
    Tense::Present,
    Tense::Imperfect,
    Tense::PastSimple,
    Tense::Future,
    Tense::Conditional,
    Tense::SubjunctivePresent,
    Tense::SubjunctiveImperfect,
    Tense::Imperative,
];

/// Closed, original seed lexicon. No productive rule is applied to an unlisted infinitive.
/// These verbs do not have the stem alternations of appeler, céder, payer or envoyer.
const LEMMAS: &[&str] = &[
    "aimer",
    "chanter",
    "chercher",
    "commencer",
    "danser",
    "dessiner",
    "donner",
    "écouter",
    "fermer",
    "garder",
    "jouer",
    "laisser",
    "laver",
    "manger",
    "marcher",
    "oublier",
    "parler",
    "passer",
    "penser",
    "porter",
    "préparer",
    "ranger",
    "regarder",
    "terminer",
    "travailler",
    "trouver",
    "visiter",
];

/// Original closed irregular paradigms. Include suivre to retain the ambiguity of suis.
const IRREGULAR: &[(&str, [[&str; 6]; 8])] = &[
    (
        "être",
        [
            ["suis", "es", "est", "sommes", "êtes", "sont"],
            ["étais", "étais", "était", "étions", "étiez", "étaient"],
            ["fus", "fus", "fut", "fûmes", "fûtes", "furent"],
            ["serai", "seras", "sera", "serons", "serez", "seront"],
            ["serais", "serais", "serait", "serions", "seriez", "seraient"],
            ["sois", "sois", "soit", "soyons", "soyez", "soient"],
            ["fusse", "fusses", "fût", "fussions", "fussiez", "fussent"],
            ["", "sois", "", "soyons", "soyez", ""],
        ],
    ),
    (
        "avoir",
        [
            ["ai", "as", "a", "avons", "avez", "ont"],
            ["avais", "avais", "avait", "avions", "aviez", "avaient"],
            ["eus", "eus", "eut", "eûmes", "eûtes", "eurent"],
            ["aurai", "auras", "aura", "aurons", "aurez", "auront"],
            ["aurais", "aurais", "aurait", "aurions", "auriez", "auraient"],
            ["aie", "aies", "ait", "ayons", "ayez", "aient"],
            ["eusse", "eusses", "eût", "eussions", "eussiez", "eussent"],
            ["", "aie", "", "ayons", "ayez", ""],
        ],
    ),
    (
        "suivre",
        [
            ["suis", "suis", "suit", "suivons", "suivez", "suivent"],
            ["suivais", "suivais", "suivait", "suivions", "suiviez", "suivaient"],
            ["suivis", "suivis", "suivit", "suivîmes", "suivîtes", "suivirent"],
            ["suivrai", "suivras", "suivra", "suivrons", "suivrez", "suivront"],
            ["suivrais", "suivrais", "suivrait", "suivrions", "suivriez", "suivraient"],
            ["suive", "suives", "suive", "suivions", "suiviez", "suivent"],
            ["suivisse", "suivisses", "suivît", "suivissions", "suivissiez", "suivissent"],
            ["", "suis", "", "suivons", "suivez", ""],
        ],
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Reading {
    pub lemma: &'static str,
    pub tense: Tense,
    pub person: Person,
}

/// Generate one form of a known paradigm. Insert e / ç only before a back vowel in -ger/-cer.
fn inflect(lemma: &str, tense: Tense, person: Person) -> Option<String> {
    if let Some((_, forms)) = IRREGULAR.iter().find(|entry| entry.0 == lemma) {
        let form = *forms.get(tense.index())?.get(person.index())?;
        return (!form.is_empty()).then(|| form.to_string());
    }
    let stem = lemma.strip_suffix("er")?;
    let ending = *tense.endings().get(person.index())?;
    if ending.is_empty() {
        return None;
    }
    let back_vowel = ending.starts_with(['a', 'â', 'o']);
    let stem = if back_vowel && lemma.ends_with("ger") {
        format!("{stem}e")
    } else if back_vowel && lemma.ends_with("cer") {
        format!("{}ç", stem.strip_suffix('c')?)
    } else {
        stem.to_string()
    };
    Some(format!("{stem}{ending}"))
}

fn lexicon() -> &'static BTreeMap<String, Vec<Reading>> {
    static LEXICON: OnceLock<BTreeMap<String, Vec<Reading>>> = OnceLock::new();
    LEXICON.get_or_init(|| {
        let mut forms: BTreeMap<String, Vec<Reading>> = BTreeMap::new();
        for lemma in LEMMAS.iter().copied().chain(IRREGULAR.iter().map(|entry| entry.0)) {
            for tense in TENSES {
                for person in PERSONS {
                    if let Some(form) = inflect(lemma, tense, person) {
                        forms.entry(form).or_default().push(Reading { lemma, tense, person });
                    }
                }
            }
        }
        forms
    })
}

/// Surface lookup is case-sensitive and composed; the caller retains original source bounds.
pub(super) fn analyze(word: &str) -> &[Reading] {
    lexicon().get(word).map_or(&[], Vec::as_slice)
}

/// Past-participle readings are retained separately from the finite-person index.
/// In particular eus and suivis must not be treated as unambiguously finite.
pub(super) fn has_past_participle_reading(word: &str) -> bool {
    static PARTICIPLES: OnceLock<BTreeSet<String>> = OnceLock::new();
    PARTICIPLES
        .get_or_init(|| {
            let mut forms: BTreeSet<String> = ["été", "eu", "eue", "eus", "eues", "suivi", "suivie", "suivis", "suivies"]
                .into_iter()
                .map(str::to_string)
                .collect();
            for lemma in LEMMAS {
                if let Some(stem) = lemma.strip_suffix("er") {
                    for ending in ["é", "ée", "és", "ées"] {
                        forms.insert(format!("{stem}{ending}"));
                    }
                }
            }
            forms
        })
        .contains(word)
}

/// After an explicit subject, preserve every simple-tense reading, but not imperative readings
/// (an imperative has no expressed subject). A compatible reading or differing repairs means
/// abstention. No ranking or arbitrary choice of tense is performed.
pub(super) fn agreement_repair(word: &str, person: Person) -> Option<String> {
    let readings = analyze(word);
    if has_past_participle_reading(word) {
        return None;
    }
    if !readings.iter().any(|reading| reading.tense != Tense::Imperative)
        || readings.iter().any(|reading| reading.tense != Tense::Imperative && reading.person == person)
    {
        return None;
    }
    let mut repair: Option<String> = None;
    for reading in readings.iter().filter(|reading| reading.tense != Tense::Imperative) {
        let form = inflect(reading.lemma, reading.tense, person)?;
        if repair.as_ref().is_some_and(|previous| *previous != form) {
            return None;
        }
        repair = Some(form);
    }
    repair
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyses_keep_homographic_persons_and_tenses() {
        let present = analyze("parle");
        assert!(present.iter().any(|r| r.person == Person::FirstSingular && r.tense == Tense::Present));
        assert!(present.iter().any(|r| r.person == Person::ThirdSingular && r.tense == Tense::SubjunctivePresent));
        assert!(present.iter().any(|r| r.person == Person::SecondSingular && r.tense == Tense::Imperative));
        let ambiguous = analyze("parlions");
        assert!(ambiguous.iter().any(|r| r.tense == Tense::Imperfect));
        assert!(ambiguous.iter().any(|r| r.tense == Tense::SubjunctivePresent));
        assert_eq!(agreement_repair("parlions", Person::ThirdSingular), None);
        assert!(analyze("suis").iter().any(|r| r.lemma == "être"));
        assert!(analyze("suis").iter().any(|r| r.lemma == "suivre"));
        assert_eq!(agreement_repair("suis", Person::ThirdSingular), None);
        assert_eq!(agreement_repair("suis", Person::SecondSingular), None);
        for word in ["eus", "suivis"] {
            assert!(has_past_participle_reading(word));
            assert!(!analyze(word).is_empty());
            assert_eq!(agreement_repair(word, Person::ThirdPlural), None);
        }
        assert_eq!(agreement_repair("parlons", Person::ThirdPlural), Some("parlent".into()));
        assert_eq!(agreement_repair("parle", Person::ThirdPlural), Some("parlent".into()));
    }

    #[test]
    fn generation_preserves_soft_consonants_and_written_tense() {
        for (word, person, expected) in [
            ("mangez", Person::FirstPlural, "mangeons"),
            ("mangeait", Person::ThirdPlural, "mangeaient"),
            ("commencez", Person::FirstPlural, "commençons"),
            ("commençait", Person::ThirdPlural, "commençaient"),
            ("commencerons", Person::SecondSingular, "commenceras"),
            ("mangerons", Person::ThirdSingular, "mangera"),
            ("parlèrent", Person::FirstSingular, "parlai"),
            ("écouterez", Person::ThirdPlural, "écouteront"),
        ] {
            assert_eq!(agreement_repair(word, person).as_deref(), Some(expected), "{word}");
        }
        // First plural present and subjunctive have different repairs: do not pick one.
        assert_eq!(agreement_repair("mangent", Person::FirstPlural), None);
    }

    #[test]
    fn unknown_or_nonfinite_words_do_not_acquire_a_guessed_lemma() {
        for word in ["cher", "hier", "mer", "aimer", "aimé", "envoyons", "préfère", "Appelle", "invente", "mange\u{301}", ""] {
            assert!(analyze(word).is_empty(), "{word}");
        }
    }
}
