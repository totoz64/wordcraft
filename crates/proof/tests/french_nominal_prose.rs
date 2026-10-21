//! Optional external prose smoke: no book text, linguistic implementation or external fixture is bundled.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use wordcraft_proof::{ProofLang, fr};

#[test]
#[ignore = "requires original external prose in WORDCRAFT_FRENCH_PROSE and full WORDCRAFT_FRENCH_LEXICON"]
fn external_prose_reports_actual_nominal_agreement_alerts() {
    let prose = std::env::var_os("WORDCRAFT_FRENCH_PROSE").expect("explicit external prose input");
    let lexicon = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("explicit full morphology input");
    wordcraft_proof::set_dictionaries_folder(Some(std::path::Path::new(&lexicon).parent().unwrap().to_owned()));
    assert!(matches!(fr::morphology::data_status(), fr::morphology::DataStatus::Ready { analyses: 274_427, .. }));
    use std::io::Read;
    let mut input = String::new();
    std::fs::File::open(prose).unwrap().take(8 * 1024 * 1024 + 1).read_to_string(&mut input).unwrap();
    assert!(input.len() <= 8 * 1024 * 1024);
    let (mut originals, mut variants, mut words, mut determiner_records) = (0usize, 0usize, 0usize, 0usize);
    let mut by_source = BTreeMap::<String, usize>::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut alerts = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let (source, text) = line.split_once('\t').expect("source<TAB>one normalized paragraph");
        if source.ends_with("-modern-dash") {
            variants += 1;
        } else {
            originals += 1;
        }
        *by_source.entry(source.trim_end_matches("-modern-dash").to_owned()).or_default() += 1;
        words += text.split_whitespace().count();
        // Broad exposure count, deliberately independent of the acceptance parser.
        // It does not measure how many syntax contexts the implementation supports.
        if wordcraft_proof::words(text)
            .into_iter()
            .any(|(a, b)| matches!(text[a..b].to_lowercase().as_str(), "le" | "la" | "un" | "une" | "les" | "des" | "ce" | "cet" | "cette" | "ces"))
        {
            determiner_records += 1;
        }
        // This public API is the all-French grammar path. Mixed-run dispatch and spelling
        // precedence are exercised separately by the proof and engine integration tests.
        for language in [ProofLang::Fr, ProofLang::FrCa] {
            for issue in fr::grammar::check_in(text, language)
                .into_iter()
                .filter(|issue| matches!(issue.message.as_str(), "Accord dans le groupe nominal" | "Accord du verbe avec le groupe nominal sujet"))
            {
                let replaced = text.get(issue.start..issue.end).expect("reported range must remain valid UTF-8");
                let offset = text[..issue.start].chars().count();
                let context: String = text.chars().skip(offset.saturating_sub(80)).take(200).collect();
                *counts.entry(format!("{language:?}:{}", issue.message)).or_default() += 1;
                alerts.push(format!(
                    "{source} input-record {index} {language:?} bytes {}..{} {:?} -> {:?}; context {:?}",
                    issue.start, issue.end, replaced, issue.suggestions, context
                ));
            }
        }
    }
    eprintln!(
        "nominal prose smoke: {originals} original records + {variants} labelled dialogue-marker variants; {words} whitespace words include variants"
    );
    eprintln!("source record counts: {by_source:?}; {determiner_records} records contain a common determiner (broad exposure only)");
    eprintln!("actual alerts by French region and rule: {counts:?}; {} total diagnostic observations", alerts.len());
    for alert in &alerts {
        eprintln!("{alert}");
    }
    assert_eq!(by_source.len(), 4, "this explicit smoke input covers the four documented books");
    assert!(originals > 1000 && determiner_records > 1000, "external prose must provide substantial independent exposure");
    assert!(alerts.is_empty(), "classify all alerts against source before accepting this smoke; this is not a real-world precision estimate");
    wordcraft_proof::set_dictionaries_folder(None);
}
