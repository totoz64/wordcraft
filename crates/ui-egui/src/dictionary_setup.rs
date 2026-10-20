//! Async desktop French proofing installation. Only the host downloads and writes files.
use crate::WordApp;
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, TryRecvError};
use wordcraft_engine::proof::fr::morphology::{self, DataStatus};

/// Host installation into the configured folder, or its own profile; returns the folder to activate.
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
#[derive(Clone, Copy)]
enum Tool {
    Dictionary,
    Grammar,
}
const TOOLS: [Tool; 2] = [Tool::Dictionary, Tool::Grammar];
impl Tool {
    fn state(self, app: &WordApp) -> &State {
        match self {
            Self::Dictionary => &app.dictionary_setup,
            Self::Grammar => &app.grammar_data_setup,
        }
    }
    fn state_mut(self, app: &mut WordApp) -> &mut State {
        match self {
            Self::Dictionary => &mut app.dictionary_setup,
            Self::Grammar => &mut app.grammar_data_setup,
        }
    }
    fn service(self, app: &WordApp) -> Option<&Install> {
        match self {
            Self::Dictionary => app.services.install_french_dictionary.as_ref(),
            Self::Grammar => app.services.install_french_grammar_data.as_ref(),
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Dictionary => "ui.frenchDictionary",
            Self::Grammar => "ui.frenchGrammarData",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Dictionary => "Install French dictionary",
            Self::Grammar => "Add French verb forms",
        }
    }
    fn translated_label(self) -> &'static str {
        match self {
            Self::Dictionary => tl!("Install French dictionary"),
            Self::Grammar => tl!("Add French verb forms"),
        }
    }
    fn ready(self) -> &'static str {
        match self {
            Self::Dictionary => tl!("The French dictionary is ready."),
            Self::Grammar => tl!("The French verb forms are ready."),
        }
    }
    fn stopped(self) -> &'static str {
        match self {
            Self::Dictionary => tl!("The dictionary download stopped. Please try again."),
            Self::Grammar => tl!("The French grammar data download stopped. Please try again."),
        }
    }
    fn failure(self, reason: &str) -> String {
        let message = match self {
            Self::Dictionary => tl!("Dictionary installation failed: {reason}"),
            Self::Grammar => tl!("French grammar data installation failed: {reason}"),
        };
        crate::i18n::fmt(message, &[("reason", reason)])
    }
}
pub fn command(app: &mut WordApp, params: &Value) -> Result<Value, String> {
    command_for(app, params, Tool::Dictionary)
}
pub fn grammar_command(app: &mut WordApp, params: &Value) -> Result<Value, String> {
    command_for(app, params, Tool::Grammar)
}
fn command_for(app: &mut WordApp, params: &Value, tool: Tool) -> Result<Value, String> {
    let object = params.as_object().ok_or("French proofing setup parameters must be an object")?;
    if object.keys().any(|k| k != "install") {
        return Err("use only install: true to add French proofing data".into());
    }
    let start = object.get("install").map(|v| v.as_bool().ok_or("install must be a boolean")).transpose()?.unwrap_or(false);
    if start && !tool.state(app).busy() {
        if TOOLS.iter().any(|other| other.state(app).busy()) {
            return Err(tl!("A French proofing installation is already in progress.").into());
        }
        let install = tool.service(app).ok_or("French proofing installation is available in the desktop app")?;
        let configured = app.session.prefs.dictionaries_folder.clone();
        let receiver = install(configured.clone());
        let state = tool.state_mut(app);
        state.pending = Some((configured, receiver));
        state.error = None;
        state.ready = false;
    }
    let state = tool.state(app);
    let mut result = json!({"available": tool.service(app).is_some(), "busy": state.busy(), "ready": state.ready, "error": state.error});
    if matches!(tool, Tool::Grammar) {
        let (ready, analyses, data_error) = grammar_data_status();
        result["ready"] = json!(ready);
        result["analyses"] = json!(analyses);
        result["dataError"] = json!(data_error);
        if result["error"].is_null() {
            result["error"] = json!(data_error);
        }
    }
    Ok(result)
}

fn grammar_data_status() -> (bool, usize, Option<String>) {
    match morphology::data_status() {
        DataStatus::Missing => (false, 0, None),
        DataStatus::Ready { analyses, .. } => (true, analyses, None),
        DataStatus::Unreadable { path, reason } => (false, 0, Some(format!("{}: {reason}", path.display()))),
    }
}
pub fn commands(app: &WordApp) -> Vec<Value> {
    TOOLS.into_iter().map(|tool| json!({"id": tool.id(), "label": tool.label(), "location": "File › Options › Proofing", "params": "{install?: bool}; omit to read status; install: true adds fixed French proofing data into the configured folder", "enabled": tool.service(app).is_some()})).collect()
}
pub(crate) fn poll(app: &mut WordApp, ctx: &egui::Context) {
    for tool in TOOLS {
        poll_one(app, ctx, tool);
    }
}
fn poll_one(app: &mut WordApp, ctx: &egui::Context, tool: Tool) {
    let Some((_, receiver)) = tool.state(app).pending.as_ref() else { return };
    let answer = match receiver.try_recv() {
        Ok(result) => result,
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        Err(TryRecvError::Disconnected) => Err(tool.stopped().into()),
    };
    let Some((original, _)) = tool.state_mut(app).pending.take() else { return };
    let result = answer.and_then(|folder| {
        if app.session.prefs.dictionaries_folder != original {
            return Err(tl!("The dictionaries folder changed during installation. Please try again.").into());
        }
        app.run("file.options", json!({"dictionariesFolder": folder})).map(|_| ())
    });
    match result {
        Ok(()) => {
            let state = tool.state_mut(app);
            state.ready = true;
            state.error = None;
            app.status(tool.ready());
        }
        Err(error) => {
            tool.state_mut(app).error = Some(error.clone());
            app.status(tool.failure(&error));
        }
    }
    ctx.request_repaint();
}
pub(crate) fn show(app: &mut WordApp, ui: &mut egui::Ui) {
    for tool in TOOLS {
        if tool.service(app).is_none() {
            continue;
        }
        ui.add_space(8.0);
        match tool {
            Tool::Dictionary => {
                ui.label(tl!("French dictionary (Dicollecte 7.7, MPL 2.0)"));
                ui.label(egui::RichText::new(tl!("Downloaded from GitHub only when you click Install.")).small().weak());
            }
            Tool::Grammar => {
                ui.label(tl!("French grammar data"));
                ui.label(egui::RichText::new(tl!("More verbs for French agreement checks (12 MB download).")).small().weak());
            }
        }
        let any_busy = TOOLS.iter().any(|tool| tool.state(app).busy());
        if ui.add_enabled(!any_busy, egui::Button::new(tool.translated_label())).clicked()
            && let Err(error) = app.run(tool.id(), json!({"install": true}))
        {
            tool.state_mut(app).error = Some(error);
        }
        let state = tool.state(app);
        let (ready, data_error) = if matches!(tool, Tool::Grammar) {
            let (ready, _, error) = grammar_data_status();
            (ready, error)
        } else {
            (state.ready, None)
        };
        if state.busy() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(match tool {
                    Tool::Dictionary => tl!("Installing the French dictionary…"),
                    Tool::Grammar => tl!("Adding French verb forms…"),
                });
            });
        } else if ready {
            ui.label(tool.ready());
        }
        if let Some(error) = state.error.as_ref().or(data_error.as_ref()) {
            ui.label(tool.failure(error));
        }
    }
}
