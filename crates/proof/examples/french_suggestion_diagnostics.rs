#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::error::Error;
use std::path::Path;
use wordcraft_proof::{ProofLang, fr};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::var_os("WORDCRAFT_DICTIONARIES").ok_or("WORDCRAFT_DICTIONARIES is required")?;
    let installed = fr::installed_path().ok_or("the real French dictionary was not installed")?;
    if installed.parent() != Some(Path::new(&directory)) || fr::dictionary().len() <= 50_000 {
        return Err("a real dictionary from the specified runtime directory is required".into());
    }
    println!("source_sha={}", std::env::var("GITHUB_SHA").unwrap_or_default());
    for lang in [ProofLang::Fr, ProofLang::FrCa] {
        for word in ["ot", "CCe", "BBonjour", "DDemain", "ÉÉcole", "MMcDonald", "IIPhone"] {
            for max in [6, 32, 64] {
                let suggestions = wordcraft_proof::suggest_in(word, max, lang);
                let ont_rank = suggestions.iter().position(|s| s == "ont").map(|i| i + 1);
                println!("lang={lang:?} word={word:?} max={max} ont_rank={ont_rank:?} suggestions={suggestions:?}");
                if suggestions.len() > max {
                    return Err("suggestions exceeded the requested limit".into());
                }
            }
        }
        for (word, expected) in [("CCe", "Ce"), ("BBonjour", "Bonjour"), ("DDemain", "Demain"), ("ÉÉcole", "École")] {
            let suggestions = wordcraft_proof::suggest_in(word, 6, lang);
            if suggestions.first().map(String::as_str) != Some(expected) {
                return Err(format!("{lang:?} {word}: expected initial repair {expected}, got {suggestions:?}").into());
            }
        }
        let wide = wordcraft_proof::suggest_in("ot", 64, lang);
        if !wide.iter().any(|s| s == "ont") {
            return Err(format!("{lang:?}: ont was not generated in the bounded wider diagnostic: {wide:?}").into());
        }
    }
    Ok(())
}
