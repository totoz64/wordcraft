//! Original teaching lexicon with opt-in bounded structural hypotheses. No corrections.
pub mod syntax;

use serde::Serialize;
use std::collections::BTreeMap;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::UnicodeNormalization;

pub const MAX_BYTES: usize = 65_536;
pub const MAX_TOKENS: usize = 8_192;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reading {
    pub lemme: String,
    pub nature: &'static str,
    pub provenance: &'static str,
    pub traits: BTreeMap<&'static str, String>,
}

#[derive(Debug, Serialize)]
pub struct Token {
    pub texte: String,
    pub plage_utf8: [usize; 2],
    pub plage_utf16: [usize; 2],
    pub type_segment: &'static str,
    pub statut: &'static str,
    pub lectures: Vec<Reading>,
}

#[derive(Debug, Serialize)]
pub struct Analysis {
    pub version: u8,
    pub texte: String,
    pub langue: &'static str,
    pub portee: &'static str,
    pub formes_indexees: usize,
    pub tokens: Vec<Token>,
    pub corrections: Vec<String>,
    pub limites: [&'static str; 4],
}

#[derive(Debug, PartialEq, Eq)]
pub enum AnalysisError {
    InputTooLong,
    TooManyTokens,
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InputTooLong => "texte trop long (maximum 65536 octets UTF-8)",
            Self::TooManyTokens => "trop de segments (maximum 8192)",
        })
    }
}
impl std::error::Error for AnalysisError {}

type Lexicon = BTreeMap<String, Vec<Reading>>;

fn key(text: &str) -> String {
    text.nfc().flat_map(char::to_lowercase).collect::<String>().nfc().collect()
}

fn add(lex: &mut Lexicon, form: &str, lemma: &str, pos: &'static str, traits: &[(&'static str, &str)]) {
    let reading = Reading {
        lemme: lemma.into(),
        nature: pos,
        provenance: "tables_originales_scolaires_v1",
        traits: traits.iter().map(|(k, v)| (*k, (*v).into())).collect(),
    };
    let values = lex.entry(key(form)).or_default();
    if !values.contains(&reading) {
        values.push(reading);
    }
}

fn nominal(lex: &mut Lexicon, lemma: &str, plural: &str, gender: &str, pos: &'static str) {
    add(lex, lemma, lemma, pos, &[("genre", gender), ("nombre", "singulier")]);
    add(lex, plural, lemma, pos, &[("genre", gender), ("nombre", "pluriel")]);
}

fn verb(lex: &mut Lexicon, lemma: &str, present: [&str; 6], subjunctive: [&str; 6], imperative: [&str; 3]) {
    add(lex, lemma, lemma, "verbe", &[("mode", "infinitif")]);
    for (mode, forms) in [("indicatif", present), ("subjonctif", subjunctive)] {
        for (i, form) in forms.iter().enumerate() {
            let person = match i % 3 {
                0 => "1",
                1 => "2",
                _ => "3",
            };
            let number = if i < 3 { "singulier" } else { "pluriel" };
            add(lex, form, lemma, "verbe", &[("mode", mode), ("temps", "present"), ("personne", person), ("nombre", number)]);
        }
    }
    for ((person, number), form) in [("2", "singulier"), ("1", "pluriel"), ("2", "pluriel")].iter().zip(imperative) {
        add(lex, form, lemma, "verbe", &[("mode", "imperatif"), ("temps", "present"), ("personne", person), ("nombre", number)]);
    }
}

fn lexicon() -> Lexicon {
    let mut lex = Lexicon::new();
    for (lemma, plural, gender) in [
        ("chien", "chiens", "masculin"),
        ("chienne", "chiennes", "feminin"),
        ("chat", "chats", "masculin"),
        ("chatte", "chattes", "feminin"),
        ("enfant", "enfants", "masculin"),
        ("enfant", "enfants", "feminin"),
        ("voisin", "voisins", "masculin"),
        ("voisine", "voisines", "feminin"),
        ("cheval", "chevaux", "masculin"),
        ("journal", "journaux", "masculin"),
        ("porte", "portes", "feminin"),
        ("école", "écoles", "feminin"),
        ("pomme", "pommes", "feminin"),
        ("sac", "sacs", "masculin"),
        ("ami", "amis", "masculin"),
        ("amie", "amies", "feminin"),
        ("livre", "livres", "masculin"),
        ("livre", "livres", "feminin"),
        ("petit", "petits", "masculin"),
    ] {
        nominal(&mut lex, lemma, plural, gender, "nom");
    }
    add(&mut lex, "petite", "petit", "nom", &[("genre", "feminin"), ("nombre", "singulier")]);
    add(&mut lex, "petites", "petit", "nom", &[("genre", "feminin"), ("nombre", "pluriel")]);
    for (lemma, forms) in [
        ("petit", ["petit", "petite", "petits", "petites"]),
        ("grand", ["grand", "grande", "grands", "grandes"]),
        ("heureux", ["heureux", "heureuse", "heureux", "heureuses"]),
        ("rouge", ["rouge", "rouge", "rouges", "rouges"]),
        ("beau", ["beau", "belle", "beaux", "belles"]),
    ] {
        for (i, form) in forms.iter().enumerate() {
            let gender = if i % 2 == 0 { "masculin" } else { "feminin" };
            let number = if i < 2 { "singulier" } else { "pluriel" };
            add(&mut lex, form, lemma, "adjectif", &[("genre", gender), ("nombre", number)]);
        }
    }
    // Explicit paradigms: no guessing from the suffix of an unknown word.
    verb(
        &mut lex,
        "jouer",
        ["joue", "joues", "joue", "jouons", "jouez", "jouent"],
        ["joue", "joues", "joue", "jouions", "jouiez", "jouent"],
        ["joue", "jouons", "jouez"],
    );
    verb(
        &mut lex,
        "porter",
        ["porte", "portes", "porte", "portons", "portez", "portent"],
        ["porte", "portes", "porte", "portions", "portiez", "portent"],
        ["porte", "portons", "portez"],
    );
    verb(
        &mut lex,
        "parler",
        ["parle", "parles", "parle", "parlons", "parlez", "parlent"],
        ["parle", "parles", "parle", "parlions", "parliez", "parlent"],
        ["parle", "parlons", "parlez"],
    );
    verb(
        &mut lex,
        "manger",
        ["mange", "manges", "mange", "mangeons", "mangez", "mangent"],
        ["mange", "manges", "mange", "mangions", "mangiez", "mangent"],
        ["mange", "mangeons", "mangez"],
    );
    verb(
        &mut lex,
        "aboyer",
        ["aboie", "aboies", "aboie", "aboyons", "aboyez", "aboient"],
        ["aboie", "aboies", "aboie", "aboyions", "aboyiez", "aboient"],
        ["aboie", "aboyons", "aboyez"],
    );
    verb(
        &mut lex,
        "être",
        ["suis", "es", "est", "sommes", "êtes", "sont"],
        ["sois", "sois", "soit", "soyons", "soyez", "soient"],
        ["sois", "soyons", "soyez"],
    );
    verb(&mut lex, "avoir", ["ai", "as", "a", "avons", "avez", "ont"], ["aie", "aies", "ait", "ayons", "ayez", "aient"], ["aie", "ayons", "ayez"]);
    verb(
        &mut lex,
        "suivre",
        ["suis", "suis", "suit", "suivons", "suivez", "suivent"],
        ["suive", "suives", "suive", "suivions", "suiviez", "suivent"],
        ["suis", "suivons", "suivez"],
    );
    for (form, lemma, gender) in [("le", "le", "masculin"), ("la", "le", "feminin")] {
        add(&mut lex, form, lemma, "determinant", &[("type", "article_defini"), ("genre", gender), ("nombre", "singulier")]);
        add(&mut lex, form, lemma, "pronom", &[("type", "personnel"), ("genre", gender), ("nombre", "singulier"), ("personne", "3")]);
    }
    add(&mut lex, "les", "le", "determinant", &[("type", "article_defini"), ("nombre", "pluriel")]);
    add(&mut lex, "les", "le", "pronom", &[("type", "personnel"), ("nombre", "pluriel"), ("personne", "3")]);
    for (form, gender) in [("un", "masculin"), ("une", "feminin")] {
        add(&mut lex, form, "un", "determinant", &[("type", "article_indefini"), ("genre", gender), ("nombre", "singulier")]);
    }
    add(&mut lex, "des", "un", "determinant", &[("type", "article_indefini"), ("nombre", "pluriel")]);
    // Contracted articles remain one source token; the composition stays explicit.
    for (form, lemma, prep, number, gender) in [
        ("du", "le", "de", "singulier", "masculin"),
        ("des", "le", "de", "pluriel", "non_specifie"),
        ("au", "le", "à", "singulier", "masculin"),
        ("aux", "le", "à", "pluriel", "non_specifie"),
    ] {
        let mut traits = vec![("type", "article_contracte"), ("preposition", prep), ("nombre", number)];
        if gender != "non_specifie" {
            traits.push(("genre", gender));
        }
        add(&mut lex, form, lemma, "determinant", &traits);
    }
    add(&mut lex, "du", "du", "determinant", &[("type", "article_partitif"), ("genre", "masculin"), ("nombre", "singulier")]);
    for (form, lemma, person, number) in [
        ("je", "je", "1", "singulier"),
        ("tu", "tu", "2", "singulier"),
        ("il", "il", "3", "singulier"),
        ("elle", "elle", "3", "singulier"),
        ("on", "on", "3", "singulier"),
        ("nous", "nous", "1", "pluriel"),
        ("vous", "vous", "2", "pluriel"),
        ("ils", "il", "3", "pluriel"),
        ("elles", "elle", "3", "pluriel"),
    ] {
        let mut traits = vec![("type", "personnel"), ("personne", person), ("nombre", number)];
        if matches!(form, "il" | "ils") {
            traits.push(("genre", "masculin"));
        }
        if matches!(form, "elle" | "elles") {
            traits.push(("genre", "feminin"));
        }
        add(&mut lex, form, lemma, "pronom", &traits);
    }
    for apostrophe in ["'", "’"] {
        let elided = format!("l{apostrophe}");
        add(&mut lex, &elided, "le", "determinant", &[("type", "article_defini"), ("nombre", "singulier"), ("elision", "oui")]);
        add(&mut lex, &elided, "le", "pronom", &[("type", "personnel"), ("personne", "3"), ("nombre", "singulier"), ("elision", "oui")]);
        add(
            &mut lex,
            &format!("j{apostrophe}"),
            "je",
            "pronom",
            &[("type", "personnel"), ("personne", "1"), ("nombre", "singulier"), ("elision", "oui")],
        );
        add(&mut lex, &format!("d{apostrophe}"), "de", "preposition", &[("elision", "oui")]);
    }
    for (pos, words) in [
        ("adverbe", &["vite", "bien", "hier", "demain", "très", "ne", "pas", "plus"][..]),
        ("preposition", &["à", "de", "dans", "sur", "avec", "sans", "pour"][..]),
        ("conjonction", &["et", "ou", "mais", "donc", "car", "si", "que", "quand"][..]),
        ("interjection", &["ah", "oh", "hélas", "zut"][..]),
    ] {
        for word in words {
            let traits = if pos == "conjonction" {
                vec![("type", if matches!(*word, "et" | "ou" | "mais" | "donc" | "car") { "coordination" } else { "subordination" })]
            } else {
                Vec::new()
            };
            add(&mut lex, word, word, pos, &traits);
        }
    }
    add(&mut lex, "donc", "donc", "adverbe", &[]);
    add(&mut lex, "aujourd’hui", "aujourd’hui", "adverbe", &[]);
    add(&mut lex, "aujourd'hui", "aujourd’hui", "adverbe", &[]);
    add(&mut lex, "si", "si", "adverbe", &[]);
    add(&mut lex, "que", "que", "pronom", &[("type", "relatif")]);
    add(&mut lex, "deux", "deux", "determinant", &[("type", "numeral"), ("nombre", "pluriel")]);
    lex
}

fn is_mark(c: char) -> bool {
    matches!(get_general_category(c), GeneralCategory::NonspacingMark | GeneralCategory::SpacingMark | GeneralCategory::EnclosingMark)
}
fn is_letter(c: char) -> bool {
    c.is_alphabetic() && !is_mark(c)
}

pub fn analyze(text: &str) -> Result<Analysis, AnalysisError> {
    if text.len() > MAX_BYTES {
        return Err(AnalysisError::InputTooLong);
    }
    let lex = lexicon();
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut utf16 = 0;
    while let Some((start, c)) = chars.next() {
        let start16 = utf16;
        utf16 += c.len_utf16();
        if c.is_whitespace() {
            continue;
        }
        if tokens.len() >= MAX_TOKENS {
            return Err(AnalysisError::TooManyTokens);
        }
        let word = is_letter(c);
        let number = c.is_numeric() && !word;
        let mut end = start + c.len_utf8();
        if word || number {
            while let Some(&(index, next)) = chars.peek() {
                let continuation = if word { is_letter(next) || is_mark(next) } else { next.is_numeric() && !is_letter(next) };
                if continuation {
                    chars.next();
                    utf16 += next.len_utf16();
                    end = index + next.len_utf8();
                    continue;
                }
                if word && matches!(next, '\'' | '’') {
                    let prefix = text.get(start..end).map(key).unwrap_or_default();
                    let elided = matches!(prefix.as_str(), "l" | "j" | "d");
                    let internal = text.get(index + next.len_utf8()..).and_then(|s| s.chars().next()).is_some_and(is_letter);
                    if elided || internal {
                        chars.next();
                        utf16 += next.len_utf16();
                        end = index + next.len_utf8();
                        if elided {
                            break;
                        }
                        continue;
                    }
                }
                break;
            }
        }
        let Some(surface) = text.get(start..end) else {
            continue;
        };
        let lectures = if word { lex.get(&key(surface)).cloned().unwrap_or_default() } else { Vec::new() };
        let kind = if word {
            "mot"
        } else if number {
            "nombre"
        } else {
            match get_general_category(c) {
                GeneralCategory::ClosePunctuation
                | GeneralCategory::ConnectorPunctuation
                | GeneralCategory::DashPunctuation
                | GeneralCategory::FinalPunctuation
                | GeneralCategory::InitialPunctuation
                | GeneralCategory::OpenPunctuation
                | GeneralCategory::OtherPunctuation => "ponctuation",
                _ => "autre",
            }
        };
        let status = if !word {
            "non_lexical"
        } else if lectures.is_empty() {
            "inconnu_du_lexique"
        } else {
            "lectures_connues_non_exhaustives"
        };
        tokens.push(Token {
            texte: surface.into(),
            plage_utf8: [start, end],
            plage_utf16: [start16, utf16],
            type_segment: kind,
            statut: status,
            lectures,
        });
    }
    Ok(Analysis {
        version: 1,
        texte: text.into(),
        langue: "fr",
        portee: "lexique_scolaire_de_demonstration",
        formes_indexees: lex.len(),
        tokens,
        corrections: Vec::new(),
        limites: [
            "Lexique original miniature ; les lectures connues ne sont pas exhaustives.",
            "Inconnu signifie absent du lexique, pas faute d’orthographe.",
            "Aucune fonction syntaxique, interprétation du contexte ou correction.",
            "Texte original conservé ; NFC et minuscules servent uniquement à la recherche.",
        ],
    })
}

/// Opt-in structural layer; the original lexical readings are preserved verbatim.
#[derive(Debug, Serialize)]
pub struct SentenceAnalysis {
    pub analyse_lexicale: Analysis,
    pub analyse_syntaxique: syntax::SyntaxAnalysis,
}

pub fn analyze_sentence(text: &str) -> Result<SentenceAnalysis, AnalysisError> {
    let analyse_lexicale = analyze(text)?;
    let analyse_syntaxique = syntax::inspect(&analyse_lexicale.tokens);
    Ok(SentenceAnalysis { analyse_lexicale, analyse_syntaxique })
}
