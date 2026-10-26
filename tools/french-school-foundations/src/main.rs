use french_school_foundations::{MAX_BYTES, analyze, analyze_sentence};
use std::io::{Read, Write};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    let (with_syntax, operation) = if first.as_deref() == Some("--syntax") { (true, args.next()) } else { (false, first) };
    let text = match operation.as_deref() {
        Some("--help") => {
            println!(
                "french-school-foundations [--syntax] --text <phrase> | --stdin\nLectures lexicales ; --syntax ajoute des hypothèses sujet–verbe bornées.\nAucune correction ni interprétation du sens."
            );
            return Ok(());
        }
        Some("--text") => {
            let value = args.next().ok_or("phrase manquante après --text")?;
            if args.next().is_some() {
                return Err("argument supplémentaire".into());
            }
            value
        }
        Some("--stdin") => {
            if args.next().is_some() {
                return Err("argument supplémentaire".into());
            }
            let mut value = String::new();
            std::io::stdin().take((MAX_BYTES + 1) as u64).read_to_string(&mut value)?;
            value
        }
        _ => return Err("utiliser [--syntax] --text <phrase> ou --stdin ; --help pour l’aide".into()),
    };
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    if with_syntax {
        serde_json::to_writer_pretty(&mut output, &analyze_sentence(&text)?)?;
    } else {
        serde_json::to_writer_pretty(&mut output, &analyze(&text)?)?;
    }
    writeln!(output)?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
