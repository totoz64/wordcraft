//! Reproducible proofing observations; results contain IDs and ranges, never source sentences.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use serde::Deserialize;
use serde_json::{Value, json};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;
use wordcraft_proof::{IssueKind, Options, ProofLang};

#[derive(Deserialize)]
struct Case {
    id: String,
    language: String,
    input: String,
    #[serde(default)]
    custom_dictionary: Vec<String>,
}

/// Resolve only an explicitly supplied proofing language.
fn language(tag: &str) -> Result<ProofLang, Box<dyn Error>> {
    if !matches!(tag, "fr-FR" | "fr-CA" | "en-US") {
        return Err(format!("Unsupported explicit language: {tag}").into());
    }
    Ok(ProofLang::from_tag(Some(tag)))
}

/// Record issue ranges in Word-compatible UTF-16 units and retain native UTF-8 offsets.
fn observe(case: &Case) -> Result<Value, Box<dyn Error>> {
    let started = Instant::now();
    if !case.custom_dictionary.is_empty() {
        return Ok(json!({"id":case.id,"language":case.language,"status":"excluded_custom_dictionary","range_unit":"utf16"}));
    }
    let lang = language(&case.language)?;
    let issues = wordcraft_proof::check_text_with(&case.input, &[(0..case.input.len(), lang)], &Options::default(), true, false);
    let mut spelling = Vec::new();
    let mut grammar = Vec::new();
    for issue in issues {
        let prefix = case.input.get(..issue.start).ok_or("Invalid issue start")?;
        let range = case.input.get(issue.start..issue.end).ok_or("Invalid issue range")?;
        let suggestions = match issue.kind {
            IssueKind::Spelling => wordcraft_proof::suggest_in(range, 6, lang),
            IssueKind::Grammar => issue.suggestions,
        };
        let item = json!({
            "start":prefix.encode_utf16().count(),
            "end":prefix.encode_utf16().count()+range.encode_utf16().count(),
            "start_utf8":issue.start,"end_utf8":issue.end,
            "message":issue.message,"suggestions":suggestions
        });
        match issue.kind {
            IssueKind::Spelling => spelling.push(item),
            IssueKind::Grammar => grammar.push(item),
        }
    }
    Ok(
        json!({"id":case.id,"language":case.language,"status":"ok","range_unit":"utf16","spelling":spelling,"grammar":grammar,"duration_ms":started.elapsed().as_secs_f64()*1000.0}),
    )
}

/// Run a bounded, deterministic shard without changing the dictionary or applying edits.
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).ok_or("Missing input JSONL path")?;
    let output = args.get(2).ok_or("Missing output JSONL path")?;
    let shard: usize = args.get(3).map_or(Ok(0), |v| v.parse())?;
    let shards: usize = args.get(4).map_or(Ok(1), |v| v.parse())?;
    if shards == 0 || shards > 32 || shard >= shards {
        return Err("Invalid shard arguments".into());
    }
    if let Some(dir) = std::env::var_os("WORDCRAFT_DICTIONARIES") {
        wordcraft_proof::set_dictionaries_folder(Some(PathBuf::from(dir)));
    }
    if !wordcraft_proof::fr::has_dictionary() {
        return Err("A real French dictionary is required".into());
    }
    let mut writer = BufWriter::new(File::create(output)?);
    let started = Instant::now();
    let mut completed = 0_usize;
    for (index, line) in BufReader::new(File::open(input)?).lines().enumerate() {
        if index >= 65_000 {
            return Err("Input case limit exceeded".into());
        }
        if index % shards != shard {
            continue;
        }
        let line = line?;
        if line.len() > 65_536 {
            return Err("Input line too long".into());
        }
        let case: Case = serde_json::from_str(&line)?;
        if case.input.len() > 16_384 || case.id.len() > 128 {
            return Err("Case input limit exceeded".into());
        }
        let result = observe(&case)?;
        serde_json::to_writer(&mut writer, &result)?;
        writeln!(writer)?;
        completed += 1;
        if completed.is_multiple_of(250) {
            writer.flush()?;
            eprintln!("shard {shard}/{shards}: {completed} cases, {:.1}s", started.elapsed().as_secs_f64());
        }
    }
    writer.flush()?;
    let metadata = json!({"application":"WordCraft","surface":"wordcraft-proof_public_API","source_sha":std::env::var("GITHUB_SHA").ok(),"engine_baseline_sha":"c394110777924eca9f900941dba4cc040f526573","range_unit":"utf16","native_range_unit":"utf8","shard":shard,"shards":shards,"cases":completed,"duration_seconds":started.elapsed().as_secs_f64(),"options":{"ignore_uppercase":true,"ignore_numbers":true,"ignore_internet":true},"suggestion_limit":6,"grammar_checked":true,"status":"completed"});
    std::fs::write(format!("{output}.metadata.json"), serde_json::to_string_pretty(&metadata)?)?;
    Ok(())
}
