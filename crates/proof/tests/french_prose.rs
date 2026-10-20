//! Explicit external prose smoke, not a population-wide precision estimate. No prose is bundled.
#![cfg(not(target_arch = "wasm32"))]
use wordcraft_proof::{
    ProofLang,
    fr::{grammar, morphology},
};

fn auxiliary_beginning(text: &str) -> bool {
    let lower = text.trim_start().to_lowercase().replace('’', "'");
    let mut words = lower.split_whitespace();
    let Some(mut first) = words.next() else { return false };
    if matches!(first, "—" | "–" | "-") {
        let Some(next) = words.next() else { return false };
        first = next;
    }
    if matches!(first, "j'ai" | "j'avais" | "j'aurai" | "j'aurais" | "j'eus") {
        return true;
    }
    let auxiliaries = match first {
        "je" => ["ai", "avais", "aurai", "aurais", "eus"],
        "tu" => ["as", "avais", "auras", "aurais", "eus"],
        "il" | "elle" | "on" => ["a", "avait", "aura", "aurait", "eut"],
        "nous" => ["avons", "avions", "aurons", "aurions", "eûmes"],
        "vous" => ["avez", "aviez", "aurez", "auriez", "eûtes"],
        "ils" | "elles" => ["ont", "avaient", "auront", "auraient", "eurent"],
        _ => return false,
    };
    words.next().is_some_and(|word| auxiliaries.contains(&word))
}

#[test]
#[ignore = "requires original external prose in WORDCRAFT_FRENCH_PROSE and full WORDCRAFT_FRENCH_LEXICON"]
fn external_prose_reports_actual_auxiliary_contexts_and_new_alerts() {
    let path = std::env::var_os("WORDCRAFT_FRENCH_PROSE").expect("set WORDCRAFT_FRENCH_PROSE");
    let lexicon = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("set WORDCRAFT_FRENCH_LEXICON");
    wordcraft_proof::set_dictionaries_folder(Some(std::path::Path::new(&lexicon).parent().unwrap().to_owned()));
    assert!(matches!(morphology::data_status(),morphology::DataStatus::Ready{analyses,..} if analyses>200_000));
    use std::io::Read;
    let mut input = String::new();
    std::fs::File::open(path).unwrap().take(8 * 1024 * 1024 + 1).read_to_string(&mut input).unwrap();
    assert!(input.len() <= 8 * 1024 * 1024);
    let (mut paragraphs, mut beginnings, mut unquoted, mut words) = (0usize, 0usize, 0usize, 0usize);
    let (mut originals, mut variants, mut original_aux, mut variant_aux) = (0usize, 0usize, 0usize, 0usize);
    let mut alerts = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let (source, text) = line.split_once('\t').expect("source<TAB>one normalized paragraph");
        paragraphs += 1;
        let variant = source.ends_with("-modern-dash");
        if variant {
            variants += 1;
        } else {
            originals += 1;
        }
        words += text.split_whitespace().count();
        if auxiliary_beginning(text) {
            beginnings += 1;
            if variant {
                variant_aux += 1;
            } else {
                original_aux += 1;
            }
            if !text.chars().any(|c| matches!(c, '"' | '«' | '»' | '“' | '”' | '`' | '<' | '>' | '=' | '@' | '\\' | '\t' | '\n' | '\r')) {
                unquoted += 1;
            }
        }
        for issue in grammar::check_in(text, ProofLang::Fr).into_iter().filter(|i| i.message.starts_with("Participe passé attendu après avoir")) {
            assert!(text.get(issue.start..issue.end).is_some());
            alerts.push(format!("{source} paragraph{index}: {text} → {:?}", issue.suggestions));
        }
    }
    eprintln!(
        "prose smoke: {paragraphs} input records, {words} whitespace words, {beginnings} auxiliary beginnings, {unquoted} unquoted auxiliary beginnings, {} new participle alerts",
        alerts.len()
    );
    eprintln!(
        "input composition: {originals} original paragraphs ({original_aux} auxiliary beginnings), {variants} separately labelled modern dialogue-marker variants ({variant_aux} auxiliary beginnings). Word total includes variants; do not treat it as independent precision data."
    );
    for alert in &alerts {
        eprintln!("{alert}");
    }
    assert!(paragraphs > 1000 && beginnings > 20, "real external prose must exercise actual trigger contexts");
    assert!(alerts.is_empty(), "inspect source and classify any alert before accepting it; this is not a blind precision claim");
}
