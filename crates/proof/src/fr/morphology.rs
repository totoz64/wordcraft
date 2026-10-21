//! French verb and bounded nominal morphology, read from the external MPL-2.0 Dicollecte lexicon.
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
const ANY_PARTICIPLE: u8 = 4;
const AVOIR: u8 = 1;
const AVOIR_WITH_OBJECT: u8 = 2;
const IMPERSONAL: u8 = 4;

// Original common-verb scope for compound reflexives. A pronominal tag alone also
// covers restricted constructions; do not infer compound availability from it.
const REFLEXIVE_LEMMAS: &[&str] = &[
    "aider",
    "apercevoir",
    "appeler",
    "arrêter",
    "blesser",
    "cacher",
    "confier",
    "coucher",
    "demander",
    "donner",
    "écrire",
    "enfuir",
    "expliquer",
    "faire",
    "habiller",
    "inscrire",
    "laver",
    "lever",
    "mettre",
    "occuper",
    "offrir",
    "parler",
    "préparer",
    "présenter",
    "procurer",
    "promener",
    "rappeler",
    "raser",
    "regarder",
    "rendre",
    "réveiller",
    "sentir",
    "souvenir",
    "taire",
    "tromper",
];

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

/// Four explicit agreement readings: masculine/feminine singular, masculine/feminine plural.
/// Their correlation is preserved when a surface form has several lexical analyses.
pub(super) const NOMINAL_SINGULAR: u8 = 0b0011;
pub(super) const NOMINAL_PLURAL: u8 = 0b1100;
pub(super) const NOMINAL_MASCULINE: u8 = 0b0101;
pub(super) const NOMINAL_FEMININE: u8 = 0b1010;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NominalPart {
    Noun,
    Adjective,
}

#[derive(Debug)]
struct NominalRecord {
    form: String,
    lemma: u32,
    part: NominalPart,
    features: u8,
}

/// Immutable compact indexes: a form may have several lemmas, tenses and persons.
#[derive(Debug, Default)]
pub struct Lexicon {
    lemmas: Vec<String>,
    records: Vec<Record>,
    by_lemma: Vec<(u32, usize)>,
    finite_len: usize,
    nonverbal: HashSet<String>,
    nominal_lemmas: Vec<String>,
    nominal_records: Vec<NominalRecord>,
    nominal_by_lemma: Vec<(u32, usize)>,
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
    /// Retains finite readings, infinitive/auxiliary annotations and recorded participles.
    /// Canonical (masculine singular/invariable) participles keep a separate bit so the
    /// avoir+infinitive rule retains its narrower scope. Nonverbal homographs are retained separately
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
        let mut nominal_ids = HashMap::<String, u32>::new();
        let mut nominal_counts = Vec::<usize>::new();
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
                let tags: Vec<&str> = fields.get(4).copied().unwrap_or_default().split_whitespace().take(33).collect();
                if tags.len() > 32 {
                    return Err(format!("Too many French nominal features at line {line_number}"));
                }
                if tags.contains(&"nom") || tags.contains(&"adj") {
                    if lemma.is_empty() || lemma.len() > MAX_WORD_BYTES {
                        return Err(format!("Invalid French nominal lemma at line {line_number}"));
                    }
                    let id = if let Some(&id) = nominal_ids.get(lemma) {
                        id
                    } else {
                        let id = u32::try_from(lexicon.nominal_lemmas.len()).map_err(|_| "Too many French nominal lemmas")?;
                        nominal_ids.insert(lemma.to_owned(), id);
                        lexicon.nominal_lemmas.push(lemma.to_owned());
                        nominal_counts.push(0);
                        id
                    };
                    let gender = if tags.contains(&"epi") {
                        NOMINAL_MASCULINE | NOMINAL_FEMININE
                    } else {
                        (if tags.contains(&"mas") { NOMINAL_MASCULINE } else { 0 }) | (if tags.contains(&"fem") { NOMINAL_FEMININE } else { 0 })
                    };
                    let number = if tags.contains(&"inv") {
                        NOMINAL_SINGULAR | NOMINAL_PLURAL
                    } else {
                        (if tags.contains(&"sg") { NOMINAL_SINGULAR } else { 0 }) | (if tags.contains(&"pl") { NOMINAL_PLURAL } else { 0 })
                    };
                    let count = usize::try_from(id).ok().and_then(|i| nominal_counts.get_mut(i)).ok_or("Invalid French nominal lemma index")?;
                    for (name, part) in [("nom", NominalPart::Noun), ("adj", NominalPart::Adjective)] {
                        if tags.contains(&name) {
                            *count = count.saturating_add(1);
                            if *count > MAX_LEMMA_RECORDS || lexicon.nominal_records.len() >= MAX_RECORDS {
                                return Err("French nominal morphology exceeds its entry limit".into());
                            }
                            lexicon.nominal_records.push(NominalRecord { form: form.to_owned(), lemma: id, part, features: gender & number });
                        }
                    }
                }
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
                | if participle && ((masculine && (singular || invariant)) || (epicene && invariant)) { PARTICIPLE } else { 0 }
                | if participle { ANY_PARTICIPLE } else { 0 };
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
        let mut previous = "";
        let mut count = 0usize;
        for record in &lexicon.nominal_records {
            count = if record.form == previous { count.saturating_add(1) } else { 1 };
            previous = &record.form;
            if count > MAX_FORM_RECORDS {
                return Err("French morphology has too many nominal analyses for one form".into());
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
        self.nominal_records.sort_unstable_by(|a, b| (&a.form, a.lemma, a.part as u8, a.features).cmp(&(&b.form, b.lemma, b.part as u8, b.features)));
        self.nominal_records.dedup_by(|a, b| a.form == b.form && a.lemma == b.lemma && a.part == b.part && a.features == b.features);
        self.nominal_by_lemma = self.nominal_records.iter().enumerate().map(|(i, record)| (record.lemma, i)).collect();
        self.nominal_by_lemma.sort_unstable();
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

    fn nominal_analyses(&self, form: &str) -> &[NominalRecord] {
        if form.len() > MAX_WORD_BYTES {
            return &[];
        }
        let start = self.nominal_records.partition_point(|record| record.form.as_str() < form);
        let end = self.nominal_records.partition_point(|record| record.form.as_str() <= form);
        self.nominal_records.get(start..end).unwrap_or_default()
    }

    pub(super) fn nominal_features(&self, form: &str, part: NominalPart) -> Option<u8> {
        let mut features = 0;
        for record in self.nominal_analyses(form).iter().filter(|r| r.part == part) {
            if record.features == 0 {
                return None; // An unsupported reading cannot be silently discarded.
            }
            features |= record.features;
        }
        (features != 0).then_some(features)
    }

    /// Only a common noun with one recorded number establishes a nominal subject.
    pub(super) fn unique_noun_number(&self, form: &str) -> Option<Person> {
        let features = self.nominal_features(form, NominalPart::Noun)?;
        match (features & NOMINAL_SINGULAR != 0, features & NOMINAL_PLURAL != 0) {
            (true, false) => Some(Person::ThirdSingular),
            (false, true) => Some(Person::ThirdPlural),
            _ => None,
        }
    }

    /// An incomplete nominal annotation cannot license treating a homograph as a verb.
    pub(super) fn has_unknown_nominal_reading(&self, form: &str) -> bool {
        self.nominal_analyses(form).iter().any(|record| record.features == 0)
    }

    pub(super) fn noun_matches_number(&self, form: &str, target: Person) -> bool {
        let mask = match target {
            Person::ThirdSingular => NOMINAL_SINGULAR,
            Person::ThirdPlural => NOMINAL_PLURAL,
            _ => return false,
        };
        self.nominal_features(form, NominalPart::Noun).is_some_and(|features| features & mask != 0)
    }

    pub(super) fn adjective_matches_number(&self, form: &str, target: Person) -> bool {
        let mask = match target {
            Person::ThirdSingular => NOMINAL_SINGULAR,
            Person::ThirdPlural => NOMINAL_PLURAL,
            _ => return false,
        };
        self.nominal_features(form, NominalPart::Adjective).is_some_and(|features| features & mask != 0)
    }

    /// Suggestions are attested forms of the same lexical lemma and part of speech.
    /// Any supported current reading is preserved; no suffix or gender is inferred.
    pub(super) fn nominal_forms(&self, form: &str, part: NominalPart, target: u8) -> Option<Vec<String>> {
        let features = self.nominal_features(form, part)?;
        if target == 0 || features & target != 0 {
            return None;
        }
        let mut forms = Vec::new();
        let mut found = false;
        for original in self.nominal_analyses(form).iter().filter(|r| r.part == part) {
            found = true;
            let start = self.nominal_by_lemma.partition_point(|(lemma, _)| *lemma < original.lemma);
            let end = self.nominal_by_lemma.partition_point(|(lemma, _)| *lemma <= original.lemma);
            let mut covered = false;
            for (_, index) in self.nominal_by_lemma.get(start..end).unwrap_or_default() {
                if let Some(record) = self.nominal_records.get(*index)
                    && record.part == part
                    && record.features & target != 0
                {
                    covered = true;
                    forms.push(record.form.as_str());
                }
            }
            if !covered {
                return None;
            }
        }
        forms.sort_unstable();
        forms.dedup();
        if !found || forms.is_empty() || forms.len() > 8 { None } else { Some(forms.into_iter().map(str::to_owned).collect()) }
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

    /// A recorded participle of a common compound-reflexive verb permits changing
    /// finite avoir to être, retaining the written participle and the auxiliary's mood/tense.
    /// Missing data, unsupported participle readings and incomplete paradigms stay undecided.
    pub(super) fn reflexive_auxiliary_forms(&self, form: &str, participle: &str, target: Person) -> Option<Vec<String>> {
        if form.len() > MAX_WORD_BYTES || participle.len() > MAX_WORD_BYTES {
            return None;
        }
        let lemma_name = |id| usize::try_from(id).ok().and_then(|i| self.lemmas.get(i)).map(String::as_str);
        let mut tenses = 0u8;
        for record in self.analyses(form).iter().filter(|r| r.tenses != 0) {
            if lemma_name(record.lemma) != Some("avoir") {
                return None;
            }
            tenses |= record.tenses;
        }
        if tenses == 0 {
            return None;
        }
        let mut found_participle = false;
        for record in self.analyses(participle).iter().filter(|r| r.nonfinite & ANY_PARTICIPLE != 0) {
            found_participle = true;
            if !lemma_name(record.lemma).is_some_and(|lemma| REFLEXIVE_LEMMAS.contains(&lemma)) {
                return None;
            }
            let start = self.by_lemma.partition_point(|(lemma, _)| *lemma < record.lemma);
            let end = self.by_lemma.partition_point(|(lemma, _)| *lemma <= record.lemma);
            let covered = self.by_lemma.get(start..end).unwrap_or_default().iter().fold(0, |covered, (_, index)| {
                covered | self.records.get(*index).filter(|r| r.persons & target.bit() != 0).map_or(0, |r| r.tenses & tenses)
            });
            if covered != tenses {
                return None;
            }
        }
        if !found_participle {
            return None;
        }
        let etre = self.lemmas.iter().position(|lemma| lemma == "être").and_then(|id| u32::try_from(id).ok())?;
        let start = self.by_lemma.partition_point(|(lemma, _)| *lemma < etre);
        let end = self.by_lemma.partition_point(|(lemma, _)| *lemma <= etre);
        let mut covered = 0u8;
        let mut forms = Vec::new();
        for (_, index) in self.by_lemma.get(start..end).unwrap_or_default() {
            if let Some(record) = self.records.get(*index)
                && record.persons & target.bit() != 0
                && record.tenses & tenses != 0
            {
                covered |= record.tenses & tenses;
                forms.push(record.form.as_str());
            }
        }
        forms.sort_unstable();
        forms.dedup();
        if covered != tenses || forms.is_empty() || forms.len() > 8 { None } else { Some(forms.into_iter().map(str::to_owned).collect()) }
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

    fn reflexive_rows() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("ai", "avoir", "v0ait____a ipre 1sg"),
            ("avais", "avoir", "v0ait____a iimp 1sg 2sg"),
            ("eus", "avoir", "v0ait____a ipsi 1sg 2sg"),
            ("aurai", "avoir", "v0ait____a ifut 1sg"),
            ("aurais", "avoir", "v0ait____a cond 1sg 2sg"),
            ("aie", "avoir", "v0ait____a spre 1sg"),
            ("eusse", "avoir", "v0ait____a simp 1sg"),
            ("suis", "être", "v0ei_____a ipre 1sg"),
            ("étais", "être", "v0ei_____a iimp 1sg 2sg"),
            ("fus", "être", "v0ei_____a ipsi 1sg 2sg"),
            ("serai", "être", "v0ei_____a ifut 1sg"),
            ("serais", "être", "v0ei_____a cond 1sg 2sg"),
            ("sois", "être", "v0ei_____a spre 1sg"),
            ("fusse", "être", "v0ei_____a simp 1sg"),
            ("trompe", "tromper", "v1_it_q__a ipre spre 1sg"),
            ("trompais", "tromper", "v1_it_q__a iimp 1sg"),
            ("trompai", "tromper", "v1_it_q__a ipsi 1sg"),
            ("tromperai", "tromper", "v1_it_q__a ifut 1sg"),
            ("tromperais", "tromper", "v1_it_q__a cond 1sg"),
            ("trompasse", "tromper", "v1_it_q__a simp 1sg"),
            ("trompé", "tromper", "v1_it_q__a ppas adj mas sg"),
            ("trompée", "tromper", "v1_it_q__a ppas adj fem sg"),
            ("trompés", "tromper", "v1_it_q__a ppas adj mas pl"),
            ("trompées", "tromper", "v1_it_q__a ppas adj fem pl"),
        ]
    }

    #[test]
    fn reflexive_auxiliaries_follow_the_written_tense_and_all_recorded_participles() {
        let lexicon = Lexicon::from_reader(table(&reflexive_rows()).as_bytes()).unwrap();
        assert_eq!(lexicon.len(), 20, "participle readings do not inflate the finite count");
        for (avoir, etre) in
            [("ai", "suis"), ("avais", "étais"), ("eus", "fus"), ("aurai", "serai"), ("aurais", "serais"), ("aie", "sois"), ("eusse", "fusse")]
        {
            for participle in ["trompé", "trompée", "trompés", "trompées"] {
                assert_eq!(lexicon.reflexive_auxiliary_forms(avoir, participle, Person::FirstSingular), Some(vec![etre.into()]));
            }
        }
        assert!(lexicon.is_past_participle("trompé"));
        assert!(!lexicon.is_past_participle("trompée"), "the older avoir+infinitive rule still requests only canonical forms");
        assert!(lexicon.reflexive_auxiliary_forms("suis", "trompée", Person::FirstSingular).is_none());
        assert!(lexicon.reflexive_auxiliary_forms("ai", "inconnue", Person::FirstSingular).is_none());
    }

    #[test]
    fn reflexive_auxiliaries_decline_incomplete_and_unsupported_readings() {
        let mut rows = reflexive_rows();
        rows.retain(|(form, lemma, _)| !(*lemma == "être" && *form == "suis"));
        let incomplete = Lexicon::from_reader(table(&rows).as_bytes()).unwrap();
        assert!(incomplete.reflexive_auxiliary_forms("ai", "trompée", Person::FirstSingular).is_none());
        let mut rows = reflexive_rows();
        rows.retain(|(_, lemma, tags)| !(*lemma == "tromper" && !tags.split_whitespace().any(|tag| tag == "ppas")));
        let incomplete = Lexicon::from_reader(table(&rows).as_bytes()).unwrap();
        assert!(incomplete.reflexive_auxiliary_forms("ai", "trompée", Person::FirstSingular).is_none());
        let mut rows = reflexive_rows();
        rows.push(("ai", "fictional-homograph", "v1 ipre 1sg"));
        let ambiguous = Lexicon::from_reader(table(&rows).as_bytes()).unwrap();
        assert!(ambiguous.reflexive_auxiliary_forms("ai", "trompée", Person::FirstSingular).is_none());
        let mut rows = reflexive_rows();
        rows.extend([("mort", "mourir", "v3_i__q_e_ ppas mas sg"), ("mourais", "mourir", "v3_i__q_e_ iimp 1sg")]);
        let lexicon = Lexicon::from_reader(table(&rows).as_bytes()).unwrap();
        assert!(lexicon.has_person_reading("mourais", Person::FirstSingular), "the unsupported lemma has a complete relevant paradigm");
        assert!(lexicon.reflexive_auxiliary_forms("avais", "mort", Person::FirstSingular).is_none());
        assert!(lexicon.reflexive_auxiliary_forms(&"x".repeat(129), "trompée", Person::FirstSingular).is_none());
        assert!(lexicon.reflexive_auxiliary_forms("ai", &"x".repeat(129), Person::FirstSingular).is_none());
        let mut rows = reflexive_rows();
        rows.extend([("trompée", "unsupported-homograph", "v1 ppas fem sg"), ("homographe", "unsupported-homograph", "v1 ipre 1sg")]);
        let ambiguous = Lexicon::from_reader(table(&rows).as_bytes()).unwrap();
        assert!(ambiguous.reflexive_auxiliary_forms("ai", "trompée", Person::FirstSingular).is_none(), "every participle reading must be supported");
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
    #[test]
    fn nominal_readings_preserve_gender_number_correlation_and_invariants() {
        let data = table(&[
            ("joue", "jouer", "v1 ipre 3sg"),
            ("mixte", "accord", "nom mas sg"),
            ("mixte", "accord", "nom fem pl"),
            ("mixtes", "accord", "nom mas pl"),
            ("autre", "différent", "nom fem sg"),
            ("autres", "différent", "nom fem pl"),
            ("orange", "orange", "adj epi inv"),
            ("enfants", "enfant", "nom epi pl"),
        ]);
        let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
        assert_eq!(lexicon.len(), 1);
        assert_eq!(lexicon.nominal_features("mixte", NominalPart::Noun), Some(0b1001));
        assert_eq!(lexicon.nominal_forms("mixte", NominalPart::Noun, 0b0100), Some(vec!["mixtes".into()]));
        assert!(lexicon.nominal_forms("mixte", NominalPart::Noun, 0b1000).is_none(), "retain the feminine plural reading");
        assert!(lexicon.nominal_forms("mixte", NominalPart::Noun, 0b0010).is_none(), "another lemma is never a suggestion");
        assert!(lexicon.unique_noun_number("mixte").is_none(), "do not flatten correlated gender/number readings");
        assert!(lexicon.nominal_forms("orange", NominalPart::Adjective, 0b1000).is_none());
        assert!(lexicon.adjective_matches_number("orange", Person::ThirdPlural));
        assert_eq!(lexicon.unique_noun_number("enfants"), Some(Person::ThirdPlural));
        assert!(lexicon.noun_matches_number("enfants", Person::ThirdPlural));
    }

    #[test]
    fn nominal_ambiguous_or_unsupported_readings_are_never_guessed() {
        let data = table(&[
            ("joue", "jouer", "v1 ipre 3sg"),
            ("joue", "joue", "nom mas mystery"),
            ("joue", "joue", "nom fem sg"),
            ("joues", "joue", "nom fem pl"),
            ("gris", "gris", "adj mas inv"),
            ("gris", "autre", "adj mas sg"),
            ("grise", "gris", "adj fem sg"),
        ]);
        let lexicon = Lexicon::from_reader(data.as_bytes()).unwrap();
        assert!(lexicon.has_unknown_nominal_reading("joue"));
        assert!(!lexicon.has_unknown_nominal_reading("absent"));
        assert!(lexicon.nominal_features("joue", NominalPart::Noun).is_none());
        assert!(lexicon.unique_noun_number("joue").is_none());
        assert!(lexicon.nominal_forms("joue", NominalPart::Noun, NOMINAL_PLURAL).is_none());
        assert!(lexicon.nominal_forms("gris", NominalPart::Adjective, NOMINAL_FEMININE).is_none(), "every lexical reading needs a target form");
        assert!(lexicon.has_person_reading("joue", Person::ThirdSingular), "keep nominal and finite analyses separate");
    }

    #[test]
    fn nominal_reader_bounds_features_and_index_fanout() {
        let finite = ("joue", "jouer", "v1 ipre 3sg");
        let excessive_tags = format!("nom mas sg {}", "unknown ".repeat(31));
        assert!(Lexicon::from_reader(table(&[finite, ("nom", "nom", &excessive_tags)]).as_bytes()).is_err());
        let mut excessive_lemma = vec![finite];
        excessive_lemma.extend(vec![("nom", "nom", "nom mas sg"); MAX_LEMMA_RECORDS + 1]);
        assert!(Lexicon::from_reader(table(&excessive_lemma).as_bytes()).is_err());
        let lemmas: Vec<_> = (0..=MAX_FORM_RECORDS).map(|index| format!("nom{index}")).collect();
        let mut excessive_form = vec![finite];
        excessive_form.extend(lemmas.iter().map(|lemma| ("nom", lemma.as_str(), "nom mas sg")));
        assert!(Lexicon::from_reader(table(&excessive_form).as_bytes()).is_err());
    }
}
