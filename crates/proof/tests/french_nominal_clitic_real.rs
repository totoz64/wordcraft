//! Original manual cases plus a bounded lexical construction sweep, not a global accuracy claim.
#![cfg(not(target_arch = "wasm32"))]
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};
use wordcraft_proof::{
    Issue, IssueKind, Options, ProofLang, check_text_with,
    fr::{
        self,
        morphology::{Lexicon, Person},
    },
    set_dictionaries_folder,
};

const SUBJECT: &str = "Accord du verbe avec le groupe nominal sujet";
const TENSES: [&str; 7] = ["ipre", "iimp", "ipsi", "ifut", "spre", "simp", "cond"];
const PAIRS_PER_TENSE: usize = 8;
const CLITICS: [&str; 4] = ["lui", "leur", "y", "en"];
type RecordedGroups = BTreeMap<(String, String), (BTreeSet<String>, BTreeSet<String>)>;
type Case = (String, String, String);

fn nominal(issue: &Issue) -> bool {
    issue.kind == IssueKind::Grammar && issue.message == SUBJECT
}

fn check(text: &str, language: ProofLang) -> Vec<Issue> {
    check_text_with(text, &[(0..text.len(), language)], &Options::default(), true, false).into_iter().filter(nominal).collect()
}

fn assert_case(text: &str, wrong: &str, replacement: &str, language: ProofLang, unique: bool) {
    let start = text.find(wrong).unwrap();
    let issues = check(text, language);
    assert_eq!(issues.len(), 1, "{language:?}: {text}: {issues:?}");
    let issue = &issues[0];
    assert_eq!(issue.lang, language);
    assert_eq!((issue.start, issue.end), (start, start + wrong.len()), "{text}: {issue:?}");
    if unique {
        assert_eq!(issue.suggestions, [replacement], "{text}: {issue:?}");
    } else {
        assert!(issue.suggestions.iter().any(|suggestion| suggestion == replacement), "{text}: {issue:?}");
    }
    let direct = fr::grammar::check_in(text, language);
    assert_eq!(direct.iter().filter(|issue| nominal(issue)).count(), 1, "direct grammar: {text}: {direct:?}");
    assert!(
        !direct.iter().any(|issue| issue.message.starts_with("Élision") && issue.start < start + wrong.len() && start < issue.end),
        "accepted nominal correction precedes an overlapping elision: {text}: {direct:?}"
    );
    for suggestion in &issue.suggestions {
        let fixed = format!("{}{suggestion}{}", &text[..start], &text[start + wrong.len()..]);
        assert!(check(&fixed, language).is_empty(), "unstable nominal correction: {fixed}");
    }
    assert!(!check_text_with(text, &[(0..text.len(), language)], &Options::default(), false, false).iter().any(nominal));
}

// Only enumerate attested lemma/tense/person pairs. Lexicon's public API below
// supplies the construction oracle; these helpers do not generate verb endings.
fn recorded_groups(path: &Path) -> RecordedGroups {
    let mut input = BufReader::new(File::open(path).unwrap());
    let mut line = String::new();
    let (mut bytes, mut rows) = (0usize, 0usize);
    let mut groups = RecordedGroups::new();
    loop {
        line.clear();
        let read = (&mut input).take(16 * 1024 + 1).read_line(&mut line).unwrap();
        if read == 0 {
            break;
        }
        bytes += read;
        assert!(read <= 16 * 1024 && bytes <= 64 * 1024 * 1024, "bounded pinned TSV input");
        let fields: Vec<_> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
        if fields.len() != 20 || fields[0].parse::<u32>().is_err() {
            continue;
        }
        rows += 1;
        assert!(rows <= 1_000_000);
        let (form, lemma) = (fields[2], fields[3]);
        let tags: Vec<_> = fields[4].split_whitespace().collect();
        if !tags.first().is_some_and(|tag| tag.starts_with('v'))
            || tags.contains(&"impe")
            || form.is_empty()
            || form.len() > 128
            || !form.chars().all(char::is_lowercase)
        {
            continue;
        }
        for tense in TENSES.into_iter().filter(|tense| tags.contains(tense)) {
            let singular = tags.iter().any(|tag| tag.trim_end_matches('!') == "3sg");
            let plural = tags.iter().any(|tag| tag.trim_end_matches('!') == "3pl");
            if singular || plural {
                let group = groups.entry((lemma.to_owned(), tense.to_owned())).or_default();
                if singular {
                    group.0.insert(form.to_owned());
                }
                if plural {
                    group.1.insert(form.to_owned());
                }
            }
        }
        assert!(groups.len() <= 100_000);
    }
    assert_eq!(rows, 515_693, "the same pinned MPL data rows, not an arbitrary local lexicon");
    groups
}

fn prefixes(clitic: &str) -> Vec<String> {
    let mut result = vec![clitic.to_owned()];
    if matches!(clitic, "y" | "en") {
        result.extend([format!("n’{clitic}"), format!("n'{clitic}")]);
    } else {
        result.push(format!("ne {clitic}"));
    }
    result
}

fn generated_cases(groups: &RecordedGroups, lexicon: &Lexicon) -> BTreeSet<Case> {
    let mut cases = BTreeSet::new();
    for tense in TENSES {
        let eligible: Vec<_> = groups
            .iter()
            .filter(|((_, recorded_tense), (sg, pl))| recorded_tense == tense && sg.len() == 1 && pl.len() == 1)
            .filter_map(|(_, (sg, pl))| Some((sg.first()?, pl.first()?)))
            .filter(|(sg, pl)| {
                sg != pl
                    && !lexicon.has_nonverbal_reading(sg)
                    && !lexicon.has_nonverbal_reading(pl)
                    && lexicon.agreement_forms(sg, Person::ThirdPlural).as_deref() == Some(std::slice::from_ref(*pl))
                    && lexicon.agreement_forms(pl, Person::ThirdSingular).as_deref() == Some(std::slice::from_ref(*sg))
            })
            .collect();
        assert!(eligible.len() >= PAIRS_PER_TENSE, "recorded unambiguous pairs for {tense}");
        // Spread across the sorted lexical class, rather than choosing named verbs.
        for slot in 0..PAIRS_PER_TENSE {
            let index = (slot + 1) * eligible.len() / (PAIRS_PER_TENSE + 1);
            let (singular, plural) = eligible[index];
            for (subject, source, target) in [("Les enfants", singular, plural), ("Le petit enfant", plural, singular)] {
                for clitic in CLITICS {
                    for prefix in prefixes(clitic) {
                        let tail = if prefix == clitic { "." } else { " pas." };
                        let wrong = format!("{prefix} {source}");
                        let canonical = prefix.replace('\'', "’");
                        let right = format!("{canonical} {target}");
                        cases.insert((format!("{subject} {wrong}{tail}"), wrong, right));
                    }
                }
            }
        }
        eprintln!("{tense}: {} admissible recorded sg/pl pairs; {PAIRS_PER_TENSE} deterministically sampled", eligible.len());
    }
    assert!(!cases.is_empty() && cases.len() <= TENSES.len() * PAIRS_PER_TENSE * 2 * 10);
    cases
}

fn generated_homograph_declines(groups: &RecordedGroups, lexicon: &Lexicon) -> BTreeSet<String> {
    let forms: BTreeSet<_> = groups.values().flat_map(|(sg, pl)| sg.iter().chain(pl)).collect();
    let mut eligible = Vec::new();
    for form in forms {
        if !lexicon.has_nonverbal_reading(form) {
            continue;
        }
        for (subject, target) in [("Les enfants", Person::ThirdPlural), ("Le petit enfant", Person::ThirdSingular)] {
            if lexicon.agreement_forms(form, target).is_some() {
                eligible.push((subject, form));
            }
        }
    }
    assert!(eligible.len() >= 8, "nonverbal-source class must actually be exercised");
    let mut cases = BTreeSet::new();
    for slot in 0..8 {
        let (subject, source) = eligible[slot * eligible.len() / 8];
        for clitic in CLITICS {
            cases.insert(format!("{subject} {clitic} {source}."));
            let negative = if matches!(clitic, "y" | "en") { format!("n’{clitic}") } else { format!("ne {clitic}") };
            cases.insert(format!("{subject} {negative} {source} pas."));
        }
    }
    assert!(cases.len() <= 64);
    cases
}

#[test]
#[ignore = "requires pinned French dictionary and MPL morphology folder in WORDCRAFT_DICTIONARIES"]
fn real_nominal_nonreflexive_clitics_cover_lexical_classes_and_context() {
    let folder = std::path::PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("explicit pinned French input folder"));
    set_dictionaries_folder(Some(folder.clone()));
    assert!(fr::has_dictionary());
    let path = fr::morphology::installed_path().unwrap();
    assert_eq!(path.parent(), Some(folder.as_path()));
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 55_312_241);
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    let lexicon = Lexicon::from_reader(File::open(&path).unwrap()).unwrap();
    assert_eq!(lexicon.len(), 274_427);
    let groups = recorded_groups(&path);
    let generated = generated_cases(&groups, &lexicon);
    let generated_declines = generated_homograph_declines(&groups, &lexicon);
    let positives = [
        ("Les enfants lui parlait.", "lui parlait", "lui parlaient"),
        ("Le petit enfant lui parlaient.", "lui parlaient", "lui parlait"),
        ("Les enfants leur répondait.", "leur répondait", "leur répondaient"),
        ("Le petit enfant leur répondaient.", "leur répondaient", "leur répondait"),
        ("Les enfants y viendra.", "y viendra", "y viendront"),
        ("Le petit enfant y viendront.", "y viendront", "y viendra"),
        ("Les enfants en sortait.", "en sortait", "en sortaient"),
        ("Le petit enfant en sortaient.", "en sortaient", "en sortait"),
        ("Les enfants ne lui parlait pas.", "ne lui parlait", "ne lui parlaient"),
        ("Les enfants ne leur répondait jamais.", "ne leur répondait", "ne leur répondaient"),
        ("Les enfants n’y allait pas.", "n’y allait", "n’y allaient"),
        ("Les enfants n'en sortait plus.", "n'en sortait", "n’en sortaient"),
        ("Les enfants ne y allait pas.", "ne y allait", "n’y allaient"),
        ("Les enfants ne en sortait pas.", "ne en sortait", "n’en sortaient"),
        ("Bonjour. Les enfants lui parlait.", "lui parlait", "lui parlaient"),
        ("Les\u{a0}enfants lui\u{202f}parlait.", "lui\u{202f}parlait", "lui parlaient"),
    ];
    let correct = [
        "Les robes en voile.",
        "Les manteaux en laine.",
        "Les murs en pierre.",
        "Les objets en plâtre.",
        "Le mur en briques.",
        "Les enfants en route.",
        "Les enfants en parlant.",
        "Les enfants lui parlent.",
        "Le petit enfant leur répond.",
        "Les enfants y vont.",
        "Les enfants en parlent.",
        "Les enfants ne lui parlent plus.",
        "Le petit enfant n’en sort pas.",
    ];
    let unsupported = [
        "Les enfants lui a.",
        "Les enfants en dispose.",
        "Les enfants leur garde.",
        "Les enfants lui voile.",
        "Les enfants le parlait.",
        "Les enfants se parlait.",
        "Les enfants lui en parlait.",
        "Les enfants y en parlait.",
        "Les enfants ne lui parlait.",
        "Les enfants ne lui parlait que dehors.",
        "Les enfants ne lui parlait pas plus.",
        "Les enfants ne lui parlait vraiment pas.",
        "Les enfants lui, parlait.",
        "Les enfants lui parlait\npas.",
        "Les enfants n’ lui parlait pas.",
        "Les enfants N’y allait pas.",
        "Les enfants n’y Allait pas.",
        "Les enfants n’y allait-pas.",
        "Les enfants qui lui parlait arrivent.",
        "Les enfants et leur mère lui parlait.",
        "Les enfants de la maison lui parlait.",
        "« Les enfants lui parlait. »",
        "— Les enfants lui parlait.",
        "Les enfant lui parlait.",
    ];
    for source in ["a", "dispose", "garde", "voile"] {
        assert!(lexicon.has_nonverbal_reading(source), "manual homograph fixture really has a nonverbal reading: {source}");
        assert!(lexicon.has_person_reading(source, Person::ThirdSingular), "manual fixture is finite too: {source}");
    }
    for language in [ProofLang::Fr, ProofLang::FrCa] {
        for (text, wrong, replacement) in positives {
            assert_case(text, wrong, replacement, language, false);
        }
        for (text, wrong, replacement) in &generated {
            assert_case(text, wrong, replacement, language, true);
        }
        for text in correct {
            assert!(check(text, language).is_empty(), "listed correct clitic: {text}: {:?}", check(text, language));
        }
        for text in unsupported.iter().copied().chain(generated_declines.iter().map(String::as_str)) {
            assert!(check(text, language).is_empty(), "ambiguous/unsupported, not a precision case: {text}: {:?}", check(text, language));
        }
        for text in ["Les enfants lui parlait.", "Les\u{a0}enfants n’y\u{202f}allait pas."] {
            let deciding_end = text.len() - 1;
            let mut segments = Vec::new();
            let mut cursor = 0;
            for (start, end) in wordcraft_proof::words(text) {
                if cursor < start {
                    segments.push(cursor..start);
                }
                segments.push(start..end);
                cursor = end;
            }
            if let Some(start) = text.find("n’y") {
                segments.extend([start..start + 1, start + 1..start + 4, start + 4..start + 5]);
            }
            for foreign in [ProofLang::Other, ProofLang::En, if language == ProofLang::Fr { ProofLang::FrCa } else { ProofLang::Fr }] {
                for segment in &segments {
                    assert!(text.is_char_boundary(segment.start) && text.is_char_boundary(segment.end));
                    let spans = [(0..segment.start, language), (segment.clone(), foreign), (segment.end..text.len(), language)];
                    assert!(
                        !check_text_with(text, &spans, &Options::default(), true, false).iter().any(nominal),
                        "foreign deciding word/gap/apostrophe {language:?}/{foreign:?}: {segment:?}: {text}"
                    );
                }
                let punctuation = [(0..deciding_end, language), (deciding_end..text.len(), foreign)];
                assert_eq!(
                    check_text_with(text, &punctuation, &Options::default(), true, false).iter().filter(|issue| nominal(issue)).count(),
                    1,
                    "terminal punctuation is outside the deciding context"
                );
            }
            let split = text.find("lui").or_else(|| text.find("n’y")).unwrap();
            assert_eq!(
                check_text_with(text, &[(0..split, language), (split..text.len(), language)], &Options::default(), true, false)
                    .iter()
                    .filter(|issue| nominal(issue))
                    .count(),
                1,
                "adjacent same-region spans merge"
            );
        }
    }
    eprintln!(
        "{} independent listed errors; {} listed correct examples; {} listed unsupported/ambiguous cases; {} distinct generated construction errors with fixed-text stability; {} generated homograph declines, each in two regions. The generated oracle is Lexicon agreement self-consistency, not independent linguistic accuracy, clitic valency or a Word comparison.",
        positives.len(),
        correct.len(),
        unsupported.len(),
        generated.len(),
        generated_declines.len()
    );
    set_dictionaries_folder(None);
}
