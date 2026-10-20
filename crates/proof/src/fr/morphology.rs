//! French verb morphology, read from the external MPL-2.0 Dicollecte lexicon.
//!
//! This reader is original Rust code. It consumes the published TSV data format, not any
//! Grammalecte grammar code. The optional file is never bundled. Without it, four small
//! original present-tense paradigms support the first agreement cases and their homographs.

use std::collections::{HashMap, HashSet};
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
const INFINITIVE: u8 = 1;
const PARTICIPLE: u8 = 2;
const AVOIR: u8 = 1;
const AVOIR_WITH_OBJECT: u8 = 2;
const IMPERSONAL: u8 = 4;

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
    nonfinite: u8,
    auxiliary: u8,
}

/// Immutable compact indexes: a form may have several lemmas, tenses and persons.
#[derive(Debug, Default)]
pub struct Lexicon {
    lemmas: Vec<String>,
    records: Vec<Record>,
    by_lemma: Vec<(u32, usize)>,
    finite_len: usize,
    nonverbal: HashSet<String>,
}

/// In the pinned 7.7 annotation, the last two slots concern être/avoir. `zz` does
/// not identify an auxiliary. In that case only an explicitly transitive active
/// construction can establish avoir (OQLF's auxiliary table); intransitive-only
/// and unfamiliar annotations remain unsupported. This is not a guess from spelling.
fn avoir_policy(tag: &str) -> u8 {
    let bytes = tag.as_bytes();
    if bytes.len() != 10 || !tag.is_ascii() {
        return 0;
    }
    let policy = match bytes.get(8..10) {
        Some(b"_a") => AVOIR,
        Some(b"ea") => AVOIR_WITH_OBJECT,
        Some(b"zz") if bytes.get(4) == Some(&b't') || bytes.get(5) == Some(&b'n') => match bytes.get(3) {
            Some(b'_') => AVOIR,
            Some(b'i') => AVOIR_WITH_OBJECT,
            _ => 0,
        },
        _ => 0,
    };
    policy | if bytes.get(7) == Some(&b'm') { IMPERSONAL } else { 0 }
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
    fn remember_nonverbal(&mut self, form: &str, line_number: usize) -> Result<(), String> {
        if form.is_empty() || form.len() > MAX_WORD_BYTES {
            return Err(format!("Invalid French morphology word at line {line_number}"));
        }
        self.nonverbal.insert(form.to_owned());
        if self.nonverbal.len() > MAX_RECORDS {
            return Err("French morphology has too many nonverbal forms".into());
        }
        Ok(())
    }

    /// Parse the public 20-column TSV. Reads, rows, fields and index fan-out are bounded.
    /// Retains finite readings and canonical (masculine singular/invariable) participles,
    /// with infinitive/auxiliary annotations. Nonverbal homographs are retained separately
    /// as ambiguity facts, never as verb/person/tense readings.
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
        let mut retained = 0usize;
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
            let Some(tag) = features.next() else { continue };
            if !tag.starts_with('v') {
                lexicon.remember_nonverbal(form, line_number)?;
                continue;
            }
            let (mut tenses, mut persons, mut imperative) = (0u8, 0u8, false);
            let (mut infinitive, mut participle, mut masculine, mut epicene, mut singular, mut invariant) =
                (false, false, false, false, false, false);
            let mut adjective = false;
            for feature in features {
                tenses |= tense(feature);
                persons |= person(feature);
                imperative |= feature == "impe";
                infinitive |= feature == "infi";
                participle |= feature == "ppas";
                masculine |= feature == "mas";
                epicene |= feature == "epi";
                singular |= feature == "sg";
                invariant |= feature == "inv";
                adjective |= feature == "adj";
            }
            if participle && adjective {
                lexicon.remember_nonverbal(form, line_number)?;
            }
            if imperative {
                continue;
            }
            let finite = tenses != 0 && persons != 0;
            let nonfinite = if infinitive { INFINITIVE } else { 0 }
                | if participle && ((masculine && (singular || invariant)) || (epicene && invariant)) { PARTICIPLE } else { 0 };
            if !finite && nonfinite == 0 {
                continue;
            }
            if !finite {
                tenses = 0;
                persons = 0;
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
            retained = retained.saturating_add(1);
            if *count > MAX_LEMMA_RECORDS || retained > MAX_RECORDS {
                return Err("French morphology has too many entries for one lemma or file".into());
            }
            lexicon.records.push(Record {
                form: form.to_owned(),
                lemma: id,
                tenses,
                persons,
                nonfinite,
                auxiliary: if infinitive { avoir_policy(tag) } else { 0 },
            });
        }
        if !header || !lexicon.records.iter().any(|record| record.tenses != 0) {
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
        self.records.sort_unstable_by(|a, b| {
            (&a.form, a.lemma, a.tenses, a.persons, a.nonfinite, a.auxiliary).cmp(&(&b.form, b.lemma, b.tenses, b.persons, b.nonfinite, b.auxiliary))
        });
        self.records.dedup_by(|a, b| {
            a.form == b.form
                && a.lemma == b.lemma
                && a.tenses == b.tenses
                && a.persons == b.persons
                && a.nonfinite == b.nonfinite
                && a.auxiliary == b.auxiliary
        });
        self.finite_len = self.records.iter().filter(|record| record.tenses != 0).count();
        self.by_lemma = self.records.iter().enumerate().map(|(i, record)| (record.lemma, i)).collect();
        self.by_lemma.sort_unstable();
        // Only homographs of finite forms are needed to avoid treating a noun/apposition
        // as a verb. Nonverbal rows never become person/tense analyses or lemma records.
        let records = &self.records;
        self.nonverbal.retain(|form| {
            let start = records.partition_point(|r| r.form.as_str() < form.as_str());
            let end = records.partition_point(|r| r.form.as_str() <= form.as_str());
            records.get(start..end).unwrap_or_default().iter().any(|r| r.tenses != 0)
        });
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
                lexicon.records.push(Record { form: form.to_owned(), lemma: id, tenses: 1, persons, nonfinite: 0, auxiliary: 0 });
            }
        }
        lexicon.nonverbal.extend(["a", "as", "est", "sommes"].into_iter().map(str::to_owned));
        lexicon.finish();
        lexicon
    }

    /// Number of retained finite-verb analyses, not a count of all lexical entries.
    pub fn len(&self) -> usize {
        self.finite_len
    }

    pub fn is_empty(&self) -> bool {
        self.finite_len == 0
    }

    fn analyses(&self, form: &str) -> &[Record] {
        let start = self.records.partition_point(|record| record.form.as_str() < form);
        let end = self.records.partition_point(|record| record.form.as_str() <= form);
        self.records.get(start..end).unwrap_or_default()
    }

    /// A finite person reading exists; this does not choose an intended mood or word sense.
    pub fn has_person_reading(&self, form: &str, target: Person) -> bool {
        form.len() <= MAX_WORD_BYTES && self.analyses(form).iter().any(|r| r.tenses != 0 && r.persons & target.bit() != 0)
    }

    /// A recorded nonverbal homograph of a finite form requires context before correction.
    pub fn has_nonverbal_reading(&self, form: &str) -> bool {
        form.len() <= MAX_WORD_BYTES && self.nonverbal.contains(form)
    }

    /// Whether an être/avoir finite reading exists, independently of other homographs.
    pub fn has_auxiliary_reading(&self, form: &str) -> bool {
        form.len() <= MAX_WORD_BYTES
            && self.analyses(form).iter().any(|r| {
                r.tenses != 0
                    && usize::try_from(r.lemma).ok().and_then(|i| self.lemmas.get(i)).is_some_and(|s| matches!(s.as_str(), "être" | "avoir"))
            })
    }

    pub(super) fn has_finite_lemma(&self, form: &str, lemma: &str) -> bool {
        form.len() <= MAX_WORD_BYTES
            && lemma.len() <= MAX_WORD_BYTES
            && self
                .analyses(form)
                .iter()
                .any(|r| r.tenses != 0 && usize::try_from(r.lemma).ok().and_then(|i| self.lemmas.get(i)).is_some_and(|s| s == lemma))
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

    /// A supported canonical past-participle reading must not be "corrected" as an infinitive.
    pub fn is_past_participle(&self, form: &str) -> bool {
        form.len() <= MAX_WORD_BYTES && self.analyses(form).iter().any(|r| r.nonfinite & PARTICIPLE != 0)
    }

    /// Whether the data explicitly records this infinitive, regardless of its auxiliary.
    pub fn is_infinitive(&self, form: &str) -> bool {
        form.len() <= MAX_WORD_BYTES && self.analyses(form).iter().any(|r| r.nonfinite & INFINITIVE != 0)
    }

    /// Known infinitive → masculine-singular/invariable participles, after an already matched
    /// subject + avoir. `impersonal_il` means the actual subject is `il`, not merely any third
    /// singular pronoun. Mixed/unspecified transitive constructions require `direct_object`.
    /// This does not choose an auxiliary, infer gender, derive endings or resolve word senses.
    pub fn participle_forms(&self, form: &str, subject: Person, impersonal_il: bool, direct_object: bool) -> Option<Vec<String>> {
        if form.len() > MAX_WORD_BYTES || self.is_past_participle(form) {
            return None;
        }
        let mut found = false;
        let mut suggestions = Vec::new();
        for infinitive in self.analyses(form).iter().filter(|r| r.nonfinite & INFINITIVE != 0) {
            found = true;
            let policy = infinitive.auxiliary;
            if policy & (AVOIR | AVOIR_WITH_OBJECT) == 0
                || (policy & AVOIR_WITH_OBJECT != 0 && !direct_object)
                || (policy & IMPERSONAL != 0 && !impersonal_il)
            {
                return None; // An unsupported reading is not silently discarded.
            }
            let start = self.by_lemma.partition_point(|(lemma, _)| *lemma < infinitive.lemma);
            let end = self.by_lemma.partition_point(|(lemma, _)| *lemma <= infinitive.lemma);
            let records = self.by_lemma.get(start..end).unwrap_or_default();
            if !records.iter().any(|(_, index)| self.records.get(*index).is_some_and(|r| r.tenses != 0 && r.persons & subject.bit() != 0)) {
                return None; // Preserve defective/impersonal paradigms and incomplete data.
            }
            let previous_count = suggestions.len();
            for (_, index) in records {
                if let Some(record) = self.records.get(*index)
                    && record.nonfinite & PARTICIPLE != 0
                {
                    suggestions.push(record.form.as_str());
                }
            }
            if suggestions.len() == previous_count {
                return None; // Do not hide an incomplete homograph behind another lemma's forms.
            }
        }
        suggestions.sort_unstable();
        suggestions.dedup();
        if !found || suggestions.is_empty() { None } else { Some(suggestions.into_iter().take(8).map(str::to_owned).collect()) }
    }
}

/// Status of the data actually loaded for checking, not merely a file's presence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataStatus {
    Missing,
    Ready { path: PathBuf, analyses: usize },
    Unreadable { path: PathBuf, reason: String },
}

#[derive(Clone)]
struct Loaded {
    lexicon: Arc<Lexicon>,
    status: DataStatus,
}

static CACHE: RwLock<Option<Loaded>> = RwLock::new(None);

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

/// Validate on first use and then report the cached state until a folder reload.
/// No network access. Missing or unreadable optional data keeps the builtin rules available.
pub fn data_status() -> DataStatus {
    current().status
}

pub(super) fn lexicon() -> Arc<Lexicon> {
    current().lexicon
}

fn current() -> Loaded {
    if let Some(loaded) = CACHE.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return loaded.clone();
    }
    let mut cache = CACHE.write().unwrap_or_else(|e| e.into_inner());
    cache
        .get_or_insert_with(|| {
            let Some(path) = installed_path() else {
                return Loaded { lexicon: Arc::new(Lexicon::builtins()), status: DataStatus::Missing };
            };
            let result = std::fs::File::open(&path).map_err(|e| e.to_string()).and_then(Lexicon::from_reader);
            match crate::hunspell::note_load(ERROR_KEY, &path, result) {
                Ok(lexicon) => Loaded { status: DataStatus::Ready { path, analyses: lexicon.len() }, lexicon: Arc::new(lexicon) },
                Err(reason) => Loaded { lexicon: Arc::new(Lexicon::builtins()), status: DataStatus::Unreadable { path, reason } },
            }
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
    fn participles_follow_recorded_forms_and_retain_auxiliary_uncertainty() {
        let data = table(&[
            ("écrire", "écrire", "v3_itnq__a infi"),
            ("écris", "écrire", "v3_itnq__a ipre 1sg 2sg"),
            ("écrit", "écrire", "v3_itnq__a ppas mas sg"),
            ("classer", "classer", "v1__t___zz infi"),
            ("classe", "classer", "v1__t___zz ipre 1sg 3sg"),
            ("classé", "classer", "v1__t___zz ppas mas sg"),
            ("rentrer", "rentrer", "v1_it___zz infi"),
            ("rentre", "rentrer", "v1_it___zz ipre 1sg 3sg"),
            ("rentré", "rentrer", "v1_it___zz ppas mas sg"),
            ("aller", "aller", "v1_i__e_e_ infi"),
            ("vais", "aller", "v1_i__e_e_ ipre 1sg"),
            ("allé", "aller", "v1_i__e_e_ ppas mas sg"),
            ("jaser", "jaser", "v1_i____zz infi"),
            ("jase", "jaser", "v1_i____zz ipre 1sg 3sg"),
            ("jasé", "jaser", "v1_i____zz ppas mas sg"),
            ("falloir", "falloir", "v3__tnem_a infi"),
            ("faut", "falloir", "v3__tnem_a ipre 3sg"),
            ("fallu", "falloir", "v3__tnem_a ppas epi inv"),
            ("tester", "tester", "v1__t____a infi"),
            ("teste", "tester", "v1__t____a ipre 1sg 3sg"),
        ]);
        let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
        assert_eq!(lexicon.len(), 7, "public analysis count remains finite-only");
        let first = Person::FirstSingular;
        assert_eq!(lexicon.participle_forms("écrire", first, false, false), Some(vec!["écrit".into()]));
        assert_eq!(lexicon.participle_forms("classer", first, false, false), Some(vec!["classé".into()]));
        assert!(lexicon.participle_forms("rentrer", first, false, false).is_none());
        assert_eq!(lexicon.participle_forms("rentrer", first, false, true), Some(vec!["rentré".into()]));
        for form in ["aller", "jaser", "tester", "inventer-un-mot", "écrit"] {
            assert!(lexicon.participle_forms(form, first, false, false).is_none(), "{form}");
        }
        assert!(lexicon.is_past_participle("écrit"));
        assert!(!lexicon.is_past_participle("écrire"));
        assert!(lexicon.participle_forms("falloir", first, true, false).is_none());
        assert!(lexicon.participle_forms("falloir", Person::ThirdSingular, false, false).is_none());
        assert_eq!(lexicon.participle_forms("falloir", Person::ThirdSingular, true, false), Some(vec!["fallu".into()]));
        let incomplete_homograph = table(&[
            ("classer", "classer", "v1__t___zz infi"),
            ("classe", "classer", "v1__t___zz ipre 1sg"),
            ("classé", "classer", "v1__t___zz ppas mas sg"),
            ("classer", "autre-lecture", "v1__t___zz infi"),
            ("classe", "autre-lecture", "v1__t___zz ipre 1sg"),
        ]);
        let incomplete = Lexicon::from_reader(incomplete_homograph.as_bytes()).unwrap();
        assert!(incomplete.participle_forms("classer", first, false, false).is_none(), "every infinitive reading needs a recorded participle");
    }

    #[test]
    fn nonverbal_homographs_stay_separate_from_person_readings() {
        let data = table(&[
            ("guides", "guider", "v1__t____a ipre 2sg"),
            ("guidons", "guider", "v1__t____a ipre 1pl"),
            ("guides", "guide", "nom mas pl"),
            ("comme", "comme", "mg cjsub adv"),
            ("est", "être", "v0ei_____a ipre 3sg"),
            ("est", "est", "nom mas inv"),
            ("abasourdis", "abasourdir", "v2__t____a ipre 1sg 2sg"),
            ("abasourdis", "abasourdir", "v2__t____a ppas adj mas pl"),
            ("abasourdissons", "abasourdir", "v2__t____a ipre 1pl"),
        ]);
        let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
        assert_eq!(lexicon.len(), 5);
        assert!(lexicon.has_nonverbal_reading("guides"));
        assert!(lexicon.has_nonverbal_reading("est"));
        assert!(lexicon.has_nonverbal_reading("abasourdis"), "adjectival participle readings are retained even in a verb-tagged row");
        assert!(!lexicon.has_nonverbal_reading("comme"), "only finite homographs need retained nonverbal facts");
        assert!(lexicon.has_person_reading("guidons", Person::FirstPlural));
        assert!(!lexicon.has_person_reading("guides", Person::FirstPlural));
        assert!(lexicon.has_auxiliary_reading("est"));
        assert!(!lexicon.has_auxiliary_reading("guides"));
        let too_long = "x".repeat(MAX_WORD_BYTES + 1);
        assert!(Lexicon::from_reader(table(&[("est", "être", "v0 ipre 3sg"), (&too_long, "nom", "nom mas sg")]).as_bytes()).is_err());
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
        let mut excessive = vec![("a".to_owned(), "avoir".to_owned(), "v0 ipre 3sg".to_owned())];
        for index in 0..=MAX_FORM_RECORDS {
            excessive.push(("tester".into(), format!("verbe{index}"), "v1__t___zz infi".into()));
        }
        let rows: Vec<_> = excessive.iter().map(|(form, lemma, tags)| (form.as_str(), lemma.as_str(), tags.as_str())).collect();
        assert!(Lexicon::from_reader(table(&rows).as_bytes()).is_err(), "non-finite analyses share the per-form input bound");
    }
}
