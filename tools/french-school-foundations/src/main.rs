use french_school_foundations::{MAX_BYTES, analyze};
use std::io::{Read, Write};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let text = match args.next().as_deref() {
        Some("--help") => {
            println!("french-school-foundations --text <phrase> | --stdin\nAnalyse lexicale limitée, sans correction ni analyse du contexte.");
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
        _ => return Err("utiliser --text <phrase> ou --stdin ; --help pour l’aide".into()),
    };
    let analysis = analyze(&text)?;
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &analysis)?;
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
