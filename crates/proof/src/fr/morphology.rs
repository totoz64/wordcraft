//! French finite-verb morphology, read from the external MPL-2.0 Dicollecte lexicon.
//!
//! This reader is original Rust code. It consumes the published TSV data format, not any
//! Grammalecte grammar code. The optional file is never bundled. Without it, four small
//! original present-tense paradigms support the first agreement cases and their homographs.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// The published 7.7 data file, found beside the spelling dictionaries.
pub const FILE_NAME: &str = "lexique-grammalecte-fr-v7.7.txt";
/// A complete optional installation can also live in this versioned directory.
pub const MANAGED_DIRECTORY: &str = "wordcraft-french-grammar-7.7-bdccf206";
const ERROR_KEY: &str = "fr-morphology";
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_LINE: usize = 16 * 1024;
const MAX_ROWS: usize = 1_000_000;
const MAX_RECORDS: usize = 600_000;
const MAX_LEMMA_RECORDS: usize = 2048;
const MAX_FORM_RECORDS: usize = 64;
const MAX_WORD_BYTES: usize = 128;

/// Grammatical person and number. Polite singular `vous` still takes SecondPlural.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Person {
    FirstSingular,
    SecondSingular,
    ThirdSingular,
    FirstPlural,
    SecondPlural,
    ThirdPlural,
}

impl Person {
    fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Debug)]
struct Record {
    form: String,
    lemma: u32,
    tenses: u8,
    persons: u8,
}

/// Immutable compact indexes: a form may have several lemmas, tenses and persons.
#[derive(Debug, Default)]
pub struct Lexicon {
    lemmas: Vec<String>,
    records: Vec<Record>,
    by_lemma: Vec<(u32, usize)>,
}

fn tense(feature: &str) -> u8 {
    match feature {
        "ipre" => 1,
        "iimp" => 2,
        "ipsi" => 4,
        "ifut" => 8,
        "spre" => 16,
        "simp" => 32,
        "cond" => 64,
        _ => 0, // Imperatives and participles cannot be the finite verb of the relative clause.
    }
}

fn person(feature: &str) -> u8 {
    match feature.trim_end_matches('!') {
        "1sg" => Person::FirstSingular.bit(),
        "2sg" => Person::SecondSingular.bit(),
        "3sg" => Person::ThirdSingular.bit(),
        "1pl" => Person::FirstPlural.bit(),
        "2pl" => Person::SecondPlural.bit(),
        "3pl" => Person::ThirdPlural.bit(),
        _ => 0, // Unknown annotations, including special inversion forms, are not guessed.
    }
}

impl Lexicon {
    /// Parse the public 20-column TSV. Reads, rows, fields and index fan-out are bounded.
    /// Non-verbal and non-finite entries are deliberately outside this first index.
    pub fn from_reader(input: impl Read) -> Result<Self, String> {
        let mut input = BufReader::new(input);
        let mut line = String::new();
        let mut bytes = 0usize;
        let mut line_number = 0usize;
        let mut rows = 0usize;
        let mut header = false;
        let mut lexicon = Self::default();
        let mut lemma_ids = HashMap::<String, u32>::new();
        let mut lemma_counts = Vec::<usize>::new();
        loop {
            line.clear();
            let read = (&mut input).take(MAX_LINE as u64 + 1).read_line(&mut line).map_err(|e| format!("French morphology: {e}"))?;
            if read == 0 {
                break;
            }
            bytes = bytes.saturating_add(read);
            line_number = line_number.saturating_add(1);
            if bytes > MAX_BYTES || read > MAX_LINE {
                return Err("French morphology exceeds the file or line size limit".into());
            }
            let text = line.trim_start_matches('\u{feff}').trim_end_matches(['\r', '\n']);
            if text.is_empty() || text.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = text.split('\t').take(21).collect();
            if fields.get(2) == Some(&"Flexion") && fields.get(3) == Some(&"Lemme") && fields.get(4) == Some(&"Étiquettes") {
                if header || fields.len() != 20 {
                    return Err("French morphology has an invalid or repeated header".into());
                }
                header = true;
                continue;
            }
            if !header {
                continue; // Published licence and corpus metadata precede the column header.
            }
            rows = rows.saturating_add(1);
            if rows > MAX_ROWS || fields.len() != 20 || fields.first().is_none_or(|id| id.parse::<u32>().is_err()) {
                return Err(format!("Invalid French morphology row at line {line_number}"));
            }
            let (Some(&form), Some(&lemma), Some(&features)) = (fields.get(2), fields.get(3), fields.get(4)) else {
                return Err(format!("Missing French morphology fields at line {line_number}"));
            };
            let mut features = features.split_whitespace();
            if !features.next().is_some_and(|tag| tag.starts_with('v')) {
                continue;
            }
            let (mut tenses, mut persons, mut imperative) = (0u8, 0u8, false);
            for feature in features {
                tenses |= tense(feature);
                persons |= person(feature);
                imperative |= feature == "impe";
            }
            if tenses == 0 || persons == 0 || imperative {
                continue;
            }
            if form.is_empty() || lemma.is_empty() || form.len() > MAX_WORD_BYTES || lemma.len() > MAX_WORD_BYTES {
                return Err(format!("Invalid French morphology word at line {line_number}"));
            }
            let id = if let Some(&id) = lemma_ids.get(lemma) {
                id
            } else {
                let id = u32::try_from(lexicon.lemmas.len()).map_err(|_| "Too many French lemmas")?;
                lemma_ids.insert(lemma.to_owned(), id);
                lexicon.lemmas.push(lemma.to_owned());
                lemma_counts.push(0);
                id
            };
            let count = usize::try_from(id).ok().and_then(|i| lemma_counts.get_mut(i)).ok_or("Invalid French lemma index")?;
            *count = count.saturating_add(1);
            if *count > MAX_LEMMA_RECORDS || lexicon.records.len() >= MAX_RECORDS {
                return Err("French morphology has too many entries for one lemma or file".into());
            }
            lexicon.records.push(Record { form: form.to_owned(), lemma: id, tenses, persons });
        }
        if !header || lexicon.records.is_empty() {
            return Err("French morphology has no supported finite-verb entries".into());
        }
        lexicon.finish();
        let mut previous = "";
        let mut count = 0usize;
        for record in &lexicon.records {
            count = if record.form == previous { count.saturating_add(1) } else { 1 };
            previous = &record.form;
            if count > MAX_FORM_RECORDS {
                return Err("French morphology has too many analyses for one form".into());
            }
        }
        Ok(lexicon)
    }

    fn finish(&mut self) {
        self.records.sort_unstable_by(|a, b| (&a.form, a.lemma, a.tenses, a.persons).cmp(&(&b.form, b.lemma, b.tenses, b.persons)));
        self.records.dedup_by(|a, b| a.form == b.form && a.lemma == b.lemma && a.tenses == b.tenses && a.persons == b.persons);
        self.by_lemma = self.records.iter().enumerate().map(|(i, record)| (record.lemma, i)).collect();
        self.by_lemma.sort_unstable();
    }

    /// Original present-tense facts. `suivre` and `sommer` preserve valid readings of `suis`
    /// and `sommes`; the mere presence of an `être` analysis must never produce a false mark.
    pub fn builtins() -> Self {
        let mut lexicon = Self::default();
        for (lemma, forms) in [
            ("être", ["suis", "es", "est", "sommes", "êtes", "sont"]),
            ("avoir", ["ai", "as", "a", "avons", "avez", "ont"]),
            ("suivre", ["suis", "suis", "suit", "suivons", "suivez", "suivent"]),
            ("sommer", ["somme", "sommes", "somme", "sommons", "sommez", "somment"]),
        ] {
            let id = u32::try_from(lexicon.lemmas.len()).unwrap_or_default();
            lexicon.lemmas.push(lemma.to_owned());
            for (index, form) in forms.into_iter().enumerate() {
                let persons = 1u8.checked_shl(u32::try_from(index).unwrap_or(u32::MAX)).unwrap_or_default();
                lexicon.records.push(Record { form: form.to_owned(), lemma: id, tenses: 1, persons });
            }
        }
        lexicon.finish();
        lexicon
    }

    /// Number of retained finite-verb analyses, not a count of all lexical entries.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn analyses(&self, form: &str) -> &[Record] {
        let start = self.records.partition_point(|record| record.form.as_str() < form);
        let end = self.records.partition_point(|record| record.form.as_str() <= form);
        self.records.get(start..end).unwrap_or_default()
    }

    /// Preserve any valid finite reading. Otherwise suggest forms of the same lemmas and
    /// overlapping tenses, bounded to eight alternatives. Unknown forms stay unmarked.
    pub fn agreement_forms(&self, form: &str, target: Person) -> Option<Vec<String>> {
        if form.len() > MAX_WORD_BYTES {
            return None;
        }
        let records = self.analyses(form);
        if records.is_empty() || records.iter().any(|record| record.persons & target.bit() != 0) {
            return None;
        }
        let mut suggestions = Vec::new();
        for record in records {
            let start = self.by_lemma.partition_point(|(lemma, _)| *lemma < record.lemma);
            let end = self.by_lemma.partition_point(|(lemma, _)| *lemma <= record.lemma);
            for (_, index) in self.by_lemma.get(start..end).unwrap_or_default() {
                if let Some(candidate) = self.records.get(*index)
                    && candidate.persons & target.bit() != 0
                    && candidate.tenses & record.tenses != 0
                {
                    suggestions.push(candidate.form.as_str());
                }
            }
        }
        suggestions.sort_unstable();
        suggestions.dedup();
        if suggestions.is_empty() { None } else { Some(suggestions.into_iter().take(8).map(str::to_owned).collect()) }
    }
}

static CACHE: RwLock<Option<Arc<Lexicon>>> = RwLock::new(None);

/// Find optional morphology data in one dictionaries folder, preserving manual-file precedence.
pub fn find_in(folder: &Path) -> Option<PathBuf> {
    [folder.join(FILE_NAME), folder.join(MANAGED_DIRECTORY).join(FILE_NAME)].into_iter().find(|path| path.is_file())
}

pub fn installed_path() -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    crate::hunspell::search_dirs(None).iter().find_map(|folder| find_in(folder))
}

/// Last unreadable-file diagnostic. Builtin rules remain usable when optional data fails.
pub fn load_error() -> Option<(PathBuf, String)> {
    crate::hunspell::load_error(ERROR_KEY)
}

pub fn reload() {
    *CACHE.write().unwrap_or_else(|e| e.into_inner()) = None;
    crate::hunspell::clear_load_error(ERROR_KEY);
}

pub(super) fn lexicon() -> Arc<Lexicon> {
    if let Some(lexicon) = CACHE.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return lexicon.clone();
    }
    let mut cache = CACHE.write().unwrap_or_else(|e| e.into_inner());
    cache
        .get_or_insert_with(|| {
            let loaded = installed_path().and_then(|path| {
                let result = std::fs::File::open(&path).map_err(|e| e.to_string()).and_then(Lexicon::from_reader);
                crate::hunspell::note_load(ERROR_KEY, &path, result).ok()
            });
            Arc::new(loaded.unwrap_or_else(Lexicon::builtins))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[(&str, &str, &str)]) -> String {
        let mut out = "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n".to_owned();
        for (index, (form, lemma, tags)) in rows.iter().enumerate() {
            out.push_str(&format!("{index}\t0\t{form}\t{lemma}\t{tags}{}\n", "\t".repeat(15)));
        }
        out
    }

    #[test]
    fn finite_readings_keep_homographs_and_ignore_imperatives() {
        let data = table(&[
            ("porte", "porte", "nom fem sg"),
            ("porte", "porter", "v1 ipre spre 1sg 3sg"),
            ("porte", "porter", "v1 impe 2sg"),
            ("portes", "porter", "v1 ipre spre 2sg"),
            ("portons", "porter", "v1 ipre 1pl"),
            ("portions", "porter", "v1 iimp spre 1pl"),
            ("portaient", "porter", "v1 iimp 3pl!"),
        ]);
        let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
        assert_eq!(lexicon.len(), 5);
        assert_eq!(lexicon.agreement_forms("porte", Person::SecondSingular), Some(vec!["portes".into()]));
        assert_eq!(lexicon.agreement_forms("portaient", Person::FirstPlural), Some(vec!["portions".into()]));
        assert_eq!(lexicon.agreement_forms("porte", Person::FirstSingular), None);
        assert_eq!(lexicon.agreement_forms("inconnu", Person::FirstSingular), None);
    }

    #[test]
    fn builtin_ambiguities_are_valid_not_automatic_errors() {
        let lexicon = Lexicon::builtins();
        assert_eq!(lexicon.agreement_forms("suis", Person::SecondSingular), None, "tu suis, from suivre");
        assert_eq!(lexicon.agreement_forms("sommes", Person::SecondSingular), None, "tu sommes, from sommer");
        assert_eq!(lexicon.agreement_forms("est", Person::FirstSingular), Some(vec!["suis".into()]));
        assert_eq!(lexicon.agreement_forms("ont", Person::FirstPlural), Some(vec!["avons".into()]));
    }

    #[test]
    fn malformed_or_oversized_data_is_rejected_without_panicking() {
        for data in [String::new(), "not a table".into(), table(&[("a", "avoir", "v0 mystery 3sg")]), "x".repeat(MAX_LINE + 1)] {
            assert!(Lexicon::from_reader(data.as_bytes()).is_err());
        }
        let valid = table(&[("a", "avoir", "v0 ipre 3sg")]);
        assert!(Lexicon::from_reader(format!("{valid}{valid}").as_bytes()).is_err());
        assert!(Lexicon::from_reader(&[0xff, 0xfe][..]).is_err());
        let too_many = table(&vec![("a", "avoir", "v0 ipre 3sg"); MAX_LEMMA_RECORDS + 1]);
        assert!(Lexicon::from_reader(too_many.as_bytes()).is_err());
    }
}
