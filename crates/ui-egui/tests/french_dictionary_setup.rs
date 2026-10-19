//! Host installation activates proofing without changing a document. Separate test process:
//! successful activation reloads the global dictionaries and must not race synthetic fixtures.
use serde_json::json;
use std::{cell::Cell, rc::Rc, sync::mpsc::channel};
use wordcraft_ui_egui::WordApp;

fn app() -> WordApp {
    WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Default::default())
}

#[test]
fn unavailable_hosts_and_invalid_parameters_never_start_a_download() {
    let mut app = app();
    assert_eq!(app.run("ui.frenchDictionary", json!({})).unwrap()["available"], false);
    for params in [json!({"install": true}), json!({"install": "yes"}), json!(null), json!({"url": "https://example.com"})] {
        assert!(app.run("ui.frenchDictionary", params).is_err());
    }
    assert!(!app.session.dirty);
}

#[test]
fn inspection_does_not_download_and_repeated_start_keeps_one_worker() {
    let mut app = app();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let (tx, rx) = channel();
    let receiver = std::cell::RefCell::new(Some(rx));
    app.services.install_french_dictionary = Some(Box::new(move |_| {
        seen.set(seen.get() + 1);
        receiver.borrow_mut().take().expect("only one worker")
    }));
    assert_eq!(app.run("ui.frenchDictionary", json!({})).unwrap()["busy"], false);
    assert_eq!(calls.get(), 0);
    assert!(app.run("ui.frenchDictionary", json!({"url": "https://example.com"})).is_err());
    app.run("ui.frenchDictionary", json!({"install": true})).unwrap();
    app.run("ui.frenchDictionary", json!({"install": true})).unwrap();
    assert_eq!(calls.get(), 1);
    app.logic(&egui::Context::default());
    assert!(app.run("ui.frenchDictionary", json!({})).unwrap()["busy"].as_bool().unwrap());
    tx.send(Ok(std::env::temp_dir().to_string_lossy().into_owned())).unwrap();
    app.logic(&egui::Context::default());
    assert!(app.run("ui.frenchDictionary", json!({})).unwrap()["ready"].as_bool().unwrap());
    assert!(!app.session.dirty, "installation does not edit the document");
    assert!(app.prefs().editing.dictionaries_folder.is_some(), "the choice survives restart");
}

#[test]
fn failed_workers_can_retry_and_changed_preferences_are_not_overwritten() {
    let mut app = app();
    app.services.install_french_dictionary = Some(Box::new(|_| {
        let (tx, rx) = channel();
        tx.send(Err("offline".into())).unwrap();
        rx
    }));
    app.run("ui.frenchDictionary", json!({"install": true})).unwrap();
    app.logic(&egui::Context::default());
    assert_eq!(app.run("ui.frenchDictionary", json!({})).unwrap()["error"].as_str(), Some("offline"));
    app.services.install_french_dictionary = Some(Box::new(|_| {
        let (tx, rx) = channel();
        tx.send(Ok("downloaded folder".into())).unwrap();
        rx
    }));
    app.run("ui.frenchDictionary", json!({"install": true})).unwrap();
    app.session.prefs.dictionaries_folder = Some("new user choice".into());
    app.logic(&egui::Context::default());
    assert_eq!(app.session.prefs.dictionaries_folder.as_deref(), Some("new user choice"));
    assert!(!app.run("ui.frenchDictionary", json!({})).unwrap()["ready"].as_bool().unwrap());
    assert!(!app.run("ui.frenchDictionary", json!({})).unwrap()["error"].is_null());
    app.services.install_french_dictionary = Some(Box::new(|_| channel().1));
    app.run("ui.frenchDictionary", json!({"install": true})).unwrap();
    app.logic(&egui::Context::default());
    assert!(!app.run("ui.frenchDictionary", json!({})).unwrap()["busy"].as_bool().unwrap());
    assert!(!app.run("ui.frenchDictionary", json!({})).unwrap()["error"].is_null());
}
