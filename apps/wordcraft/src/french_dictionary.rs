//! Desktop-only, explicit dictionary download. Fixed HTTPS inputs, bounded reads and hashes;
//! a staged directory is renamed into place only after the complete dictionary validates.
//! No document content is read or sent. Existing dictionaries are never replaced.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use sha2::{Digest, Sha256};
use wordcraft_proof::{fr, hunspell};

const SOURCE: &str = "https://raw.githubusercontent.com/LibreOffice/dictionaries/67032ee957631908098a72edf48dc81cac27e369/fr_FR/dictionaries";
const MAX_DOWNLOAD: usize = 8 * 1024 * 1024;
const FILES: &[(&str, &str)] = &[
    ("fr.aff", "c176610cd5dc4846806a65ddd029f422d87978bf58f224aa44222662a16a2de5"),
    ("fr.dic", "b78a868e31dd6e373b6c3217969afb898a9acde828a5e7ef97308da42218c88c"),
    ("README_dict_fr.txt", "4a25abf740ad6ba5e8d0cfc0d2ffadc3c13c303f8fe389f323b061a389350f83"),
];

pub fn hook(ctx: &egui::Context) -> wordcraft_ui_egui::dictionary_setup::Install {
    let ctx = ctx.clone();
    Box::new(move |configured| {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        let failure = tx.clone();
        if let Err(error) = std::thread::Builder::new().name("french-dictionary".into()).spawn(move || {
            let result = destination(configured).and_then(|root| install(&root, download)).map(|p| p.to_string_lossy().into_owned());
            let _ = tx.send(result);
            ctx.request_repaint();
        }) {
            let _ = failure.send(Err(format!("Could not start dictionary download: {error}")));
        }
        rx
    })
}

fn destination(configured: Option<String>) -> Result<PathBuf, String> {
    if let Some(folder) = configured.filter(|f| !f.trim().is_empty()) {
        return Ok(PathBuf::from(folder));
    }
    wordcraft_control_key::settings_dir().map(|p| p.join("dictionaries")).ok_or_else(|| "No user settings folder is available".into())
}

fn download(name: &str) -> Result<Vec<u8>, String> {
    // `name` comes only from FILES; no URLs, credentials or document data are accepted as input.
    let agent: ureq::Agent =
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).max_redirects(0).https_only(true).build().into();
    agent
        .get(format!("{SOURCE}/{name}"))
        .call()
        .map_err(|e| format!("Could not download the French dictionary: {e}"))?
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD as u64)
        .read_to_vec()
        .map_err(|e| format!("Could not read the French dictionary download: {e}"))
}

fn verify(name: &str, bytes: &[u8], expected: &str) -> Result<(), String> {
    let digest: String = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    if bytes.len() > MAX_DOWNLOAD || digest != expected {
        return Err(format!("French dictionary verification failed for {name}; no dictionary was activated"));
    }
    Ok(())
}

fn validate_managed(target: &Path) -> Result<(), String> {
    for &(name, hash) in FILES {
        let file = target.join(name);
        let mut bytes = Vec::new();
        std::fs::File::open(&file)
            .and_then(|file| file.take(MAX_DOWNLOAD as u64 + 1).read_to_end(&mut bytes))
            .map_err(|e| format!("Could not read the existing dictionary: {e}"))?;
        verify(name, &bytes, hash)?;
    }
    Ok(())
}

fn validate_pair(aff: &Path, dic: &Path) -> Result<(), String> {
    let dictionary = hunspell::load(aff, dic)?;
    if dictionary.is_empty() { Err("The French dictionary is empty".into()) } else { Ok(()) }
}

fn install(root: &Path, mut fetch: impl FnMut(&str) -> Result<Vec<u8>, String>) -> Result<PathBuf, String> {
    let target = root.join(fr::MANAGED_DIRECTORY);
    // Respect existing manually installed pairs (including regional aliases).
    if let Some((aff, dic)) = fr::find_in(root) {
        if dic.parent() == Some(target.as_path()) {
            validate_managed(&target)?;
        }
        validate_pair(&aff, &dic).map_err(|e| format!("An existing French dictionary could not be read; it was kept unchanged: {e}"))?;
        return Ok(root.to_path_buf());
    }
    if std::fs::symlink_metadata(&target).is_ok() {
        validate_managed(&target)?;
        return Ok(root.to_path_buf());
    }
    std::fs::create_dir_all(root).map_err(|e| format!("Could not create the dictionaries folder: {e}"))?;
    let staging =
        tempfile::Builder::new().prefix(".wordcraft-french-").tempdir_in(root).map_err(|e| format!("Could not prepare the dictionary: {e}"))?;
    for &(name, hash) in FILES {
        let bytes = fetch(name)?;
        verify(name, &bytes, hash)?;
        std::fs::write(staging.path().join(name), bytes).map_err(|e| format!("Could not write the dictionary: {e}"))?;
    }
    validate_pair(&staging.path().join("fr.aff"), &staging.path().join("fr.dic"))?;
    if let Err(error) = std::fs::rename(staging.path(), &target) {
        // Another instance may have published the same verified version while we downloaded it.
        validate_managed(&target).map_err(|_| format!("Could not finish installing the dictionary: {error}"))?;
    }
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_dictionary_and_other_languages_are_preserved_without_a_download() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("fr.aff"), "SET UTF-8\n").unwrap();
        std::fs::write(root.path().join("fr.dic"), "1\nmaison\n").unwrap();
        std::fs::write(root.path().join("pl_PL.dic"), "user data").unwrap();
        let result = install(root.path(), |_| panic!("an existing dictionary must not download"));
        assert_eq!(result.unwrap(), root.path());
        assert_eq!(std::fs::read_to_string(root.path().join("fr.dic")).unwrap(), "1\nmaison\n");
        assert_eq!(std::fs::read_to_string(root.path().join("pl_PL.dic")).unwrap(), "user data");
    }

    #[test]
    fn failed_or_unverified_download_never_publishes_a_partial_dictionary() {
        let root = tempfile::tempdir().unwrap();
        for corrupt in [false, true] {
            assert!(install(root.path(), |_| if corrupt { Ok(b"not a dictionary".to_vec()) } else { Err("offline".into()) }).is_err());
            assert!(!root.path().join(fr::MANAGED_DIRECTORY).exists());
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0, "temporary files were removed");
        }
    }

    #[test]
    fn incomplete_existing_installation_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join(fr::MANAGED_DIRECTORY);
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("fr.dic"), "user content").unwrap();
        assert!(install(root.path(), |_| panic!("do not overwrite an existing directory")).is_err());
        assert_eq!(std::fs::read_to_string(target.join("fr.dic")).unwrap(), "user content");
        std::fs::write(target.join("fr.aff"), "SET UTF-8\n").unwrap();
        std::fs::write(target.join("fr.dic"), "1\nmaison\n").unwrap();
        assert!(install(root.path(), |_| panic!("a parseable managed pair must still pass hash verification")).is_err());
    }

    #[test]
    #[ignore = "requires the pinned Dicollecte files in WORDCRAFT_DICTIONARIES"]
    fn real_french_dictionary_installation_is_atomic_and_repeatable() {
        let source = PathBuf::from(std::env::var_os("WORDCRAFT_DICTIONARIES").expect("set WORDCRAFT_DICTIONARIES"));
        let root = tempfile::tempdir().unwrap();
        assert!(
            install(root.path(), |name| {
                if name == "fr.dic" {
                    return Err("connection interrupted".into());
                }
                std::fs::read(source.join(name)).map_err(|e| e.to_string())
            })
            .is_err()
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0, "a failed second file leaves no partial install");
        install(root.path(), |name| std::fs::read(source.join(name)).map_err(|e| e.to_string())).unwrap();
        let (aff, dic) = fr::find_in(root.path()).expect("the managed dictionary is discoverable");
        let dictionary = hunspell::load(&aff, &dic).unwrap();
        assert!(dictionary.len() > 50_000);
        assert!(dictionary.check("maison"));
        assert!(!dictionary.check("maisonn"));
        install(root.path(), |_| panic!("a second install must not download again")).unwrap();
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
