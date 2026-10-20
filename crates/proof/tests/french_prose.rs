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
    words.next().is_some_and(|word| auxiliaries.contains(&word.strip_prefix("n'").unwrap_or(word)))
}

fn finite_personal_beginning(text: &str, lexicon: &morphology::Lexicon) -> Option<bool> {
    use morphology::Person;
    let mut words = text.split_whitespace();
    let mut first = words.next()?;
    if matches!(first, "—" | "–" | "-") {
        first = words.next()?;
    }
    let fused =
        first.strip_prefix("J’").or_else(|| first.strip_prefix("j’")).or_else(|| first.strip_prefix("J'")).or_else(|| first.strip_prefix("j'"));
    if fused.is_none()
        && !matches!(
            first,
            "Je" | "je"
                | "Tu"
                | "tu"
                | "Il"
                | "il"
                | "Elle"
                | "elle"
                | "On"
                | "on"
                | "Nous"
                | "nous"
                | "Vous"
                | "vous"
                | "Ils"
                | "ils"
                | "Elles"
                | "elles"
        )
    {
        return None;
    }
    let mut part = fused.or_else(|| words.next())?;
    let mut clitic = false;
    // Independent exposure counter, not an acceptance parser or precision measure.
    for _ in 0..5 {
        if part.len() > 160 {
            return None;
        }
        if let Some((head, tail)) = part.split_once(['’', '\'']) {
            if !matches!(head, "n" | "m" | "t" | "s" | "l") {
                return None;
            }
            clitic |= head != "n";
            part = tail;
        } else if matches!(part, "ne" | "me" | "te" | "se" | "nous" | "vous" | "le" | "la" | "les" | "lui" | "leur" | "y" | "en") {
            clitic |= part != "ne";
            part = words.next()?;
        } else {
            let form = part.trim_end_matches([',', '.', ';', ':', '!', '?', '…']);
            return [
                Person::FirstSingular,
                Person::SecondSingular,
                Person::ThirdSingular,
                Person::FirstPlural,
                Person::SecondPlural,
                Person::ThirdPlural,
            ]
            .into_iter()
            .any(|p| lexicon.has_person_reading(form, p))
            .then_some(clitic);
        }
    }
    None
}

#[test]
#[ignore = "requires original external prose in WORDCRAFT_FRENCH_PROSE and full WORDCRAFT_FRENCH_LEXICON"]
fn external_prose_reports_actual_auxiliary_contexts_and_new_alerts() {
    let path = std::env::var_os("WORDCRAFT_FRENCH_PROSE").expect("set WORDCRAFT_FRENCH_PROSE");
    let lexicon = std::env::var_os("WORDCRAFT_FRENCH_LEXICON").expect("set WORDCRAFT_FRENCH_LEXICON");
    wordcraft_proof::set_dictionaries_folder(Some(std::path::Path::new(&lexicon).parent().unwrap().to_owned()));
    assert!(matches!(morphology::data_status(),morphology::DataStatus::Ready{analyses,..} if analyses>200_000));
    let readings = morphology::Lexicon::from_reader(std::fs::File::open(&lexicon).unwrap()).unwrap();
    use std::io::Read;
    let mut input = String::new();
    std::fs::File::open(path).unwrap().take(8 * 1024 * 1024 + 1).read_to_string(&mut input).unwrap();
    assert!(input.len() <= 8 * 1024 * 1024);
    let (mut paragraphs, mut beginnings, mut unquoted, mut words) = (0usize, 0usize, 0usize, 0usize);
    let (mut originals, mut variants, mut original_aux, mut variant_aux) = (0usize, 0usize, 0usize, 0usize);
    let mut alerts = Vec::new();
    let (mut personal, mut unquoted_personal) = (0usize, 0usize);
    let (mut clitic_beginnings, mut unquoted_clitic) = (0usize, 0usize);
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
        if let Some(clitic) = finite_personal_beginning(text, &readings) {
            personal += 1;
            if clitic {
                clitic_beginnings += 1;
            }
            if !text.chars().any(|c| matches!(c, '"' | '«' | '»' | '“' | '”' | '`' | '<' | '>' | '=' | '@' | '\\' | '\t' | '\n' | '\r')) {
                unquoted_personal += 1;
                if clitic {
                    unquoted_clitic += 1;
                }
            }
        }
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
        for issue in grammar::check_in(text, ProofLang::Fr)
            .into_iter()
            .filter(|i| i.message.starts_with("Participe passé attendu après avoir") || i.message == "Accord du verbe avec le pronom sujet")
        {
            assert!(text.get(issue.start..issue.end).is_some());
            alerts.push(format!("{source} paragraph{index}: {text} → {:?}", issue.suggestions));
        }
    }
    eprintln!(
        "prose smoke: {paragraphs} input records, {words} whitespace words, {beginnings} auxiliary beginnings, {unquoted} unquoted auxiliary beginnings, {} new participle/subject alerts",
        alerts.len()
    );
    eprintln!(
        "input composition: {originals} original paragraphs ({original_aux} auxiliary beginnings), {variants} separately labelled modern dialogue-marker variants ({variant_aux} auxiliary beginnings). Word total includes variants; do not treat it as independent precision data."
    );
    for alert in &alerts {
        eprintln!("{alert}");
    }
    eprintln!(
        "Known finite personal-subject beginnings: {personal}, including {unquoted_personal} unquoted; prefix counts, not a precision estimate."
    );
    eprintln!(
        "Including clitic beginnings: {clitic_beginnings}, of which {unquoted_clitic} are unquoted; overlapping variants are not independent samples."
    );
    assert!(paragraphs > 1000 && beginnings > 20, "real external prose must exercise actual trigger contexts");
    assert!(personal > 200, "the added subject rule needs actual known finite contexts");
    assert!(clitic_beginnings > 20, "the added clitic rule needs actual known finite contexts");
    assert!(alerts.is_empty(), "inspect source and classify any alert before accepting it; this is not a blind precision claim");
}
