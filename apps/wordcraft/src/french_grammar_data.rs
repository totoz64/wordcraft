//! Explicit optional finite-morphology installation. Only the native host performs I/O.
//! The archive is pinned before decompression; only two named, bounded members are read.
//! Existing files are preserved and a complete validated directory is published atomically.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

use sha2::{Digest, Sha256};
use wordcraft_proof::fr::morphology::{self, Lexicon};

const SOURCE: &str = "https://www.grammalecte.net/dic/lexique-grammalecte-fr-v7.7.zip";
const ARCHIVE_SHA256: &str = "bdccf2065e252010253171000a38de84581407896b6bc60aca7d4d69dab7e4d2";
const MAX_ARCHIVE: u64 = 16 * 1024 * 1024;
const FILES: &[(&str, &str, u64)] = &[
    (morphology::FILE_NAME, "dc1fa3740e6353bccdcb435ea8f1c8a65287726026abd53dd232d5d494548eff", 64 * 1024 * 1024),
    ("README_lexique.txt", "2f12da0c8bd1c7899f75dc5437164f61e234b55133ccdfbac06dafbe7f531e8e", 64 * 1024),
];

pub fn hook(ctx: &egui::Context) -> wordcraft_ui_egui::dictionary_setup::Install {
    let ctx = ctx.clone();
    Box::new(move |configured| {
        let (tx, rx) = channel();
        let failure = tx.clone();
        let ctx = ctx.clone();
        if let Err(error) = std::thread::Builder::new().name("french-verb-forms".into()).spawn(move || {
            let result =
                super::french_dictionary::destination(configured).and_then(|root| install(&root, download)).map(|p| p.to_string_lossy().into_owned());
            let _ = tx.send(result);
            ctx.request_repaint();
        }) {
            let _ = failure.send(Err(format!("Could not start French grammar data installation: {error}")));
        }
        rx
    })
}

fn download() -> Result<Vec<u8>, String> {
    let agent: ureq::Agent =
        ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(60))).max_redirects(0).https_only(true).build().into();
    agent
        .get(SOURCE)
        .call()
        .map_err(|e| format!("Could not download French grammar data: {e}"))?
        .body_mut()
        .with_config()
        .limit(MAX_ARCHIVE)
        .read_to_vec()
        .map_err(|e| format!("Could not read French grammar data: {e}"))
}

fn verify(name: &str, bytes: &[u8], hash: &str, limit: u64) -> Result<(), String> {
    let digest: String = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    if u64::try_from(bytes.len()).map_or(true, |size| size > limit) || digest != hash {
        return Err(format!("French grammar data verification failed for {name}; existing files were kept"));
    }
    Ok(())
}

fn read_member(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>, name: &str, limit: u64) -> Result<Vec<u8>, String> {
    let member = archive.by_name(name).map_err(|e| format!("Missing French grammar data member {name}: {e}"))?;
    if !member.is_file() || member.size() > limit {
        return Err(format!("French grammar data member {name} exceeds its size limit or is not a file"));
    }
    let mut bytes = Vec::new();
    member.take(limit.saturating_add(1)).read_to_end(&mut bytes).map_err(|e| format!("Could not read French grammar data member {name}: {e}"))?;
    if u64::try_from(bytes.len()).map_or(true, |size| size > limit) {
        return Err(format!("French grammar data member {name} exceeds its size limit"));
    }
    Ok(bytes)
}

fn validate_managed(target: &Path) -> Result<(), String> {
    for &(name, hash, limit) in FILES {
        let mut bytes = Vec::new();
        std::fs::File::open(target.join(name))
            .and_then(|file| file.take(limit.saturating_add(1)).read_to_end(&mut bytes))
            .map_err(|e| format!("Could not read installed French grammar data: {e}"))?;
        verify(name, &bytes, hash, limit)?;
        if name == morphology::FILE_NAME {
            let _ = Lexicon::from_reader(bytes.as_slice())?;
        }
    }
    Ok(())
}

fn install(root: &Path, fetch: impl FnOnce() -> Result<Vec<u8>, String>) -> Result<PathBuf, String> {
    let target = root.join(morphology::MANAGED_DIRECTORY);
    if let Some(file) = morphology::find_in(root) {
        if file.parent() == Some(target.as_path()) {
            validate_managed(&target)?;
        } else {
            let source = std::fs::File::open(&file).map_err(|e| format!("Could not read existing French grammar data: {e}"))?;
            let _ = Lexicon::from_reader(source).map_err(|e| format!("Existing French grammar data was kept unchanged: {e}"))?;
        }
        return Ok(root.to_path_buf());
    }
    if std::fs::symlink_metadata(&target).is_ok() {
        validate_managed(&target)?;
        return Ok(root.to_path_buf());
    }
    std::fs::create_dir_all(root).map_err(|e| format!("Could not create the grammar data folder: {e}"))?;
    let staging = tempfile::Builder::new()
        .prefix(".wordcraft-french-grammar-")
        .tempdir_in(root)
        .map_err(|e| format!("Could not stage French grammar data: {e}"))?;
    let archive = fetch()?;
    verify("lexicon archive", &archive, ARCHIVE_SHA256, MAX_ARCHIVE)?;
    let mut archive = zip::ZipArchive::new(Cursor::new(archive)).map_err(|e| format!("Could not open the French lexicon archive: {e}"))?;
    for &(name, hash, limit) in FILES {
        let bytes = read_member(&mut archive, name, limit)?;
        verify(name, &bytes, hash, limit)?;
        if name == morphology::FILE_NAME {
            let _ = Lexicon::from_reader(bytes.as_slice())?;
        }
        std::fs::write(staging.path().join(name), bytes).map_err(|e| format!("Could not write French grammar data: {e}"))?;
    }
    std::fs::rename(staging.path(), &target).map_err(|e| format!("Could not activate French grammar data; existing files were kept: {e}"))?;
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn fixture() -> String {
        format!(
            "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n1\t0\tmanges\tmanger\tv1 ipre 2sg{}\n",
            "\t".repeat(15)
        )
    }

    #[test]
    fn manual_data_is_preserved_and_validated_without_a_download() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join(morphology::FILE_NAME);
        std::fs::write(&file, fixture()).unwrap();
        std::fs::write(root.path().join("fr.dic"), "untouched spelling dictionary").unwrap();
        install(root.path(), || panic!("manual data must not download")).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), fixture());
        assert_eq!(std::fs::read_to_string(root.path().join("fr.dic")).unwrap(), "untouched spelling dictionary");
        std::fs::write(&file, "broken manual file").unwrap();
        assert!(install(root.path(), || panic!("invalid manual data must not be overwritten")).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "broken manual file");
    }

    #[test]
    fn interrupted_or_unverified_download_leaves_no_partial_install() {
        let root = tempfile::tempdir().unwrap();
        for corrupt in [false, true] {
            assert!(
                install(root.path(), || if corrupt { Ok(b"not the pinned archive".to_vec()) } else { Err("connection interrupted".into()) }).is_err()
            );
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
        let target = root.path().join(morphology::MANAGED_DIRECTORY);
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("user-note.txt"), "keep").unwrap();
        assert!(install(root.path(), || panic!("existing target must not be replaced")).is_err());
        assert_eq!(std::fs::read_to_string(target.join("user-note.txt")).unwrap(), "keep");
    }

    #[test]
    fn only_named_bounded_zip_members_are_read() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file(morphology::FILE_NAME, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"12345").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(writer.finish().unwrap().into_inner())).unwrap();
        assert!(read_member(&mut archive, "missing", 10).is_err());
        assert!(read_member(&mut archive, morphology::FILE_NAME, 4).is_err());
        assert_eq!(read_member(&mut archive, morphology::FILE_NAME, 5).unwrap(), b"12345");
    }

    #[test]
    #[ignore = "requires the pinned archive in WORDCRAFT_FRENCH_LEXICON_ARCHIVE"]
    fn real_french_grammar_data_is_atomic_verified_and_repeatable() {
        let source = std::env::var_os("WORDCRAFT_FRENCH_LEXICON_ARCHIVE").expect("set WORDCRAFT_FRENCH_LEXICON_ARCHIVE");
        let root = tempfile::tempdir().unwrap();
        install(root.path(), || std::fs::read(&source).map_err(|e| e.to_string())).unwrap();
        let file = morphology::find_in(root.path()).unwrap();
        let lexicon = Lexicon::from_reader(std::fs::File::open(&file).unwrap()).unwrap();
        assert_eq!(lexicon.len(), 274_427);
        assert!(file.parent().unwrap().join("README_lexique.txt").is_file());
        install(root.path(), || panic!("repeat installation must not download")).unwrap();
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        std::fs::write(&file, "corrupt managed data").unwrap();
        assert!(install(root.path(), || panic!("existing corrupt data must be preserved")).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "corrupt managed data");
    }
}
