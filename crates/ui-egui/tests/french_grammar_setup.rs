//! Separate process: activation reloads global proofing data, so it must not race fixtures.
use serde_json::json;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::channel,
};
use wordcraft_ui_egui::WordApp;

#[test]
fn grammar_setup_validates_commands_serializes_jobs_and_activates_without_editing() {
    let mut app = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::from_text("C’est nous qui manges.")), Default::default());
    assert_eq!(app.run("ui.frenchGrammarData", json!({})).unwrap()["available"], false);
    for params in [json!(null), json!({"install":"yes"}), json!({"url":"https://example.com"}), json!({"install":true})] {
        assert!(app.run("ui.frenchGrammarData", params).is_err());
    }
    app.run("select.all", json!({})).unwrap();
    app.run("review.language", json!({"lang":"fr-FR","default":true})).unwrap();
    app.session.dirty = false;
    let before = serde_json::to_value(&app.session.doc).unwrap();
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().to_string_lossy().into_owned();
    app.run("file.options", json!({"dictionariesFolder":folder})).unwrap();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let expected_folder = folder.clone();
    let (tx, rx) = channel();
    let receiver = RefCell::new(Some(rx));
    app.services.install_french_grammar_data = Some(Box::new(move |configured| {
        assert_eq!(configured.as_deref(), Some(expected_folder.as_str()));
        seen.set(seen.get() + 1);
        receiver.borrow_mut().take().expect("only one installation worker")
    }));
    app.services.install_french_dictionary = Some(Box::new(|_| panic!("another installation must not start concurrently")));
    assert_eq!(app.run("ui.frenchGrammarData", json!({})).unwrap()["busy"], false);
    assert_eq!(calls.get(), 0, "inspection never downloads");
    app.run("ui.frenchGrammarData", json!({"install":true})).unwrap();
    app.run("ui.frenchGrammarData", json!({"install":true})).unwrap();
    assert_eq!(calls.get(), 1);
    assert!(app.run("ui.frenchDictionary", json!({"install":true})).is_err());
    app.logic(&egui::Context::default());
    assert_eq!(app.run("ui.frenchGrammarData", json!({})).unwrap()["busy"], true);
    // Original tiny data fixture, not third-party corpus data.
    let data = format!(
        "id\tfid\tFlexion\tLemme\tÉtiquettes\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\t16\t17\t18\t19\n1\t0\tmanges\tmanger\tv1 ipre 2sg{}\n2\t0\tmangeons\tmanger\tv1 ipre 1pl{}\n",
        "\t".repeat(15),
        "\t".repeat(15)
    );
    std::fs::write(root.path().join("lexique-grammalecte-fr-v7.7.txt"), data).unwrap();
    std::fs::write(root.path().join("fr.aff"), "SET UTF-8\nICONV 1\nICONV ’ '\n").unwrap();
    std::fs::write(root.path().join("fr.dic"), "5\nc'est\nnous\nqui\nmanges\nmangeons\n").unwrap();
    tx.send(Ok(folder.clone())).unwrap();
    app.logic(&egui::Context::default());
    assert_eq!(app.run("ui.frenchGrammarData", json!({})).unwrap()["ready"], true);
    assert_eq!(app.prefs().editing.dictionaries_folder.as_deref(), Some(folder.as_str()));
    let issues = app.run("review.issues", json!({})).unwrap();
    let issues = issues.as_array().unwrap();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0]["kind"], "grammar");
    assert_eq!(issues[0]["text"], "manges");
    assert_eq!(issues[0]["suggestions"], json!(["mangeons"]));
    assert_eq!(serde_json::to_value(&app.session.doc).unwrap(), before);
    assert!(!app.session.dirty);

    let mut restarted = WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), Default::default());
    restarted.apply_prefs(app.prefs());
    let restored = restarted.run("ui.frenchGrammarData", json!({})).unwrap();
    assert_eq!(restored["ready"], true, "ready reflects validated data even without a new installation job");
    assert_eq!(restored["analyses"], 2);
    assert!(restored["dataError"].is_null());

    app.services.install_french_grammar_data = Some(Box::new(|_| {
        let (tx, rx) = channel();
        tx.send(Err("offline".into())).unwrap();
        rx
    }));
    app.run("ui.frenchGrammarData", json!({"install":true})).unwrap();
    app.logic(&egui::Context::default());
    assert_eq!(app.run("ui.frenchGrammarData", json!({})).unwrap()["error"], "offline");
    app.services.install_french_grammar_data = Some(Box::new(|_| channel().1));
    app.run("ui.frenchGrammarData", json!({"install":true})).unwrap();
    app.logic(&egui::Context::default());
    let disconnected = app.run("ui.frenchGrammarData", json!({})).unwrap();
    assert_eq!(disconnected["busy"], false);
    assert!(!disconnected["error"].is_null());
    let (tx, rx) = channel();
    let receiver = RefCell::new(Some(rx));
    app.services.install_french_grammar_data = Some(Box::new(move |_| receiver.borrow_mut().take().unwrap()));
    app.run("ui.frenchGrammarData", json!({"install":true})).unwrap();
    let changed_root = tempfile::tempdir().unwrap();
    std::fs::write(changed_root.path().join("lexique-grammalecte-fr-v7.7.txt"), "invalid selected data").unwrap();
    let changed_folder = changed_root.path().to_string_lossy().into_owned();
    app.run("file.options", json!({"dictionariesFolder": changed_folder})).unwrap();
    tx.send(Ok(folder)).unwrap();
    app.logic(&egui::Context::default());
    assert_eq!(app.session.prefs.dictionaries_folder.as_deref(), Some(changed_folder.as_str()));
    let changed = app.run("ui.frenchGrammarData", json!({})).unwrap();
    assert_eq!(changed["ready"], false);
    assert!(!changed["error"].is_null());
    assert!(!changed["dataError"].is_null(), "invalid selected data is diagnosed, not reported ready");
    assert_eq!(serde_json::to_value(&app.session.doc).unwrap(), before);
    assert!(!app.session.dirty);
}
