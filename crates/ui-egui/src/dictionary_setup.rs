//! Async desktop dictionary installation. The UI owns progress; only the host downloads/writes.

use std::sync::mpsc::{Receiver, TryRecvError};

use serde_json::{Value, json};

use crate::WordApp;

/// The host installs into the configured dictionaries folder, or its own profile when unset,
/// then returns the folder to activate. The host wakes the UI when the answer is ready.
pub type Install = Box<dyn Fn(Option<String>) -> Receiver<Result<String, String>>>;

#[derive(Default)]
pub(crate) struct State {
    pending: Option<(Option<String>, Receiver<Result<String, String>>)>,
    pub ready: bool,
    pub error: Option<String>,
}

impl State {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
}

pub fn command(app: &mut WordApp, params: &Value) -> Result<Value, String> {
    let object = params.as_object().ok_or("dictionary setup parameters must be an object")?;
    if object.keys().any(|k| k != "install") {
        return Err("use only install: true to download the French dictionary".into());
    }
    let start = object.get("install").map(|v| v.as_bool().ok_or("install must be a boolean")).transpose()?.unwrap_or(false);
    if start && !app.dictionary_setup.busy() {
        let install = app.services.install_french_dictionary.as_ref().ok_or("French dictionary installation is available in the desktop app")?;
        let configured = app.session.prefs.dictionaries_folder.clone();
        app.dictionary_setup.pending = Some((configured.clone(), install(configured)));
        app.dictionary_setup.error = None;
        app.dictionary_setup.ready = false;
    }
    Ok(json!({
        "available": app.services.install_french_dictionary.is_some(),
        "busy": app.dictionary_setup.busy(),
        "ready": app.dictionary_setup.ready,
        "error": app.dictionary_setup.error,
    }))
}

pub fn commands(app: &WordApp) -> Vec<Value> {
    vec![json!({
        "id": "ui.frenchDictionary", "label": "Install French dictionary", "location": "File › Options › Proofing",
        "params": "{install?: bool}; omit to read status; install: true downloads fixed Dicollecte files into the configured folder",
        "enabled": app.services.install_french_dictionary.is_some(),
    })]
}

pub(crate) fn poll(app: &mut WordApp, ctx: &egui::Context) {
    let Some((_, receiver)) = app.dictionary_setup.pending.as_ref() else { return };
    let answer = match receiver.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        Err(TryRecvError::Disconnected) => Err(tl!("The dictionary download stopped. Please try again.").into()),
    };
    let Some((original, _)) = app.dictionary_setup.pending.take() else { return };
    let result = answer.and_then(|folder| {
        if app.session.prefs.dictionaries_folder != original {
            return Err(tl!("The dictionaries folder changed during installation. Please try again.").into());
        }
        app.run("file.options", json!({"dictionariesFolder": folder})).map(|_| ())
    });
    match result {
        Ok(()) => {
            app.dictionary_setup.ready = true;
            app.dictionary_setup.error = None;
            app.status(tl!("The French dictionary is ready."));
        }
        Err(error) => {
            app.dictionary_setup.error = Some(error.clone());
            app.status(crate::i18n::fmt(tl!("Dictionary installation failed: {reason}"), &[("reason", &error)]));
        }
    }
    ctx.request_repaint();
}

pub(crate) fn show(app: &mut WordApp, ui: &mut egui::Ui) {
    if app.services.install_french_dictionary.is_none() {
        return;
    }
    ui.add_space(8.0);
    ui.label(tl!("French dictionary (Dicollecte 7.7, MPL 2.0)"));
    ui.label(egui::RichText::new(tl!("Downloaded from GitHub only when you click Install.")).small().weak());
    let busy = app.dictionary_setup.busy();
    if ui.add_enabled(!busy, egui::Button::new(tl!("Install French dictionary"))).clicked()
        && let Err(error) = app.run("ui.frenchDictionary", json!({"install": true}))
    {
        app.dictionary_setup.error = Some(error);
    }
    if busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tl!("Installing the French dictionary…"));
        });
    } else if app.dictionary_setup.ready {
        ui.label(tl!("The French dictionary is ready."));
    }
    if let Some(error) = &app.dictionary_setup.error {
        ui.label(crate::i18n::fmt(tl!("Dictionary installation failed: {reason}"), &[("reason", error)]));
    }
}
