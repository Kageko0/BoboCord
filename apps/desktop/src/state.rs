use std::{path::PathBuf, rc::Rc};

use bobo_app::{Action, Request, Response, ScrollIntent, Worker};
use bobo_core::{Channel, Command, Draft, Id, LocalProfile, Message, Space, Theme};
use dioxus::prelude::*;

#[derive(Clone, Default, PartialEq)]
pub struct Navigation {
    pub spaces: Vec<Space>,
    pub channels: Vec<Channel>,
    pub selected_space: Option<Id>,
    pub selected_channel: Option<Id>,
}

impl Navigation {
    pub fn space_name(&self) -> &str {
        self.spaces
            .iter()
            .find(|s| Some(s.id) == self.selected_space)
            .map_or("Seus espaços", |s| s.name.as_str())
    }
    pub fn channel_name(&self) -> &str {
        self.channels
            .iter()
            .find(|c| Some(c.id) == self.selected_channel)
            .map_or("Conversa local", |c| c.name.as_str())
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct History {
    pub channel: Option<Id>,
    pub messages: Rc<Vec<Message>>,
    pub has_more: bool,
    pub has_newer: bool,
    pub scroll: ScrollIntent,
}

#[derive(Clone, PartialEq)]
pub struct Status {
    pub busy: bool,
    pub ready: bool,
    pub error: Option<String>,
    pub feedback: String,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            busy: true,
            ready: false,
            error: None,
            feedback: "Abrindo seus dados locais…".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    CreateSpace,
    RenameSpace(Id),
    DeleteSpace(Id),
    CreateChannel(Id),
    RenameChannel(Id),
    DeleteChannel(Id),
}

impl ModalKind {
    pub fn deleting(self) -> bool {
        matches!(self, Self::DeleteSpace(_) | Self::DeleteChannel(_))
    }
    pub fn command(self, value: String) -> Command {
        match self {
            Self::CreateSpace => Command::CreateSpace(value),
            Self::RenameSpace(id) => Command::RenameSpace(id, value),
            Self::DeleteSpace(id) => Command::DeleteSpace(id),
            Self::CreateChannel(id) => Command::CreateChannel(id, value),
            Self::RenameChannel(id) => Command::RenameChannel(id, value),
            Self::DeleteChannel(id) => Command::DeleteChannel(id),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Modal {
    pub kind: ModalKind,
    pub title: String,
    pub value: String,
}

#[derive(Clone, PartialEq)]
pub struct ContextTarget {
    pub id: Id,
    pub name: String,
    pub space: bool,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy)]
pub struct UiState {
    pub navigation: Signal<Navigation>,
    pub history: Signal<History>,
    pub profile: Signal<Option<LocalProfile>>,
    pub theme: Signal<Theme>,
    pub status: Signal<Status>,
    pub draft: Signal<String>,
    pub draft_error: Signal<Option<String>>,
    pub settings: Signal<bool>,
    pub modal: Signal<Option<Modal>>,
    pub context: Signal<Option<ContextTarget>>,
}

#[derive(Clone, Copy)]
pub struct Actions {
    dispatch: Callback<Action>,
    edit: Callback<String>,
}

impl Actions {
    pub fn dispatch(self, action: Action) {
        self.dispatch.call(action);
    }
    pub fn command(self, command: Command) {
        self.dispatch(Action::Command(command));
    }
    pub fn edit_draft(self, body: String) {
        self.edit.call(body);
    }
}

pub fn use_ui_state() -> UiState {
    UiState {
        navigation: use_signal(Navigation::default),
        history: use_signal(History::default),
        profile: use_signal(|| None),
        theme: use_signal(|| Theme::Dark),
        status: use_signal(Status::default),
        draft: use_signal(String::new),
        draft_error: use_signal(|| None),
        settings: use_signal(|| false),
        modal: use_signal(|| None),
        context: use_signal(|| None),
    }
}

pub fn replace<T: PartialEq + 'static>(mut signal: Signal<T>, value: T) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

impl UiState {
    pub fn open_modal(
        mut self,
        kind: ModalKind,
        title: impl Into<String>,
        value: impl Into<String>,
    ) {
        if self.status.peek().busy {
            return;
        }
        self.status.write().error = None;
        self.context.set(None);
        self.modal.set(Some(Modal {
            kind,
            title: title.into(),
            value: value.into(),
        }));
    }

    pub fn open_context(mut self, target: ContextTarget) {
        if !self.status.peek().busy {
            self.context.set(Some(target));
        }
    }

    pub fn apply(mut self, response: Response) {
        let (action, result) = match response {
            Response::Updated { action, result } => (action, result),
            Response::DraftSaved(result) => {
                replace(
                    self.draft_error,
                    result.err().map(|error| error.to_string()),
                );
                return;
            }
        };
        match result {
            Ok(reply) => {
                replace(self.draft_error, None);
                let snapshot = reply.snapshot;
                replace(
                    self.navigation,
                    Navigation {
                        spaces: snapshot.spaces,
                        channels: snapshot.channels,
                        selected_space: snapshot.selected_space,
                        selected_channel: snapshot.selected_channel,
                    },
                );
                replace(
                    self.history,
                    History {
                        channel: snapshot.selected_channel,
                        messages: Rc::new(snapshot.messages),
                        has_more: reply.has_more,
                        has_newer: reply.has_newer,
                        scroll: reply.scroll,
                    },
                );
                replace(self.profile, Some(snapshot.profile));
                replace(self.theme, snapshot.preferences.theme);
                replace(self.draft, reply.draft);
                let wrote = matches!(action, Action::Command(_));
                if wrote {
                    self.modal.set(None);
                }
                self.status.set(Status {
                    busy: false,
                    ready: true,
                    error: None,
                    feedback: if wrote {
                        "Salvo neste dispositivo."
                    } else {
                        "Tudo em dia · armazenamento local"
                    }
                    .into(),
                });
            }
            Err(error) => {
                let mut status = self.status.write();
                status.busy = false;
                status.error = Some(error.to_string());
            }
        }
    }
}

pub fn use_worker(mut state: UiState, path: Result<PathBuf, String>) -> Actions {
    let connection = use_hook(move || {
        Rc::new(path.and_then(|path| Worker::start(path).map_err(|e| e.to_string())))
    });
    let reader = connection.clone();
    let writer = connection.clone();
    use_future(move || {
        let reader = reader.clone();
        async move {
            match reader.as_ref() {
                Ok((_, replies)) => {
                    while let Ok(response) = replies.recv().await {
                        state.apply(response);
                    }
                }
                Err(error) => {
                    state.status.set(Status {
                        busy: false,
                        error: Some(error.clone()),
                        ..Status::default()
                    });
                }
            }
        }
    });
    let dispatch = use_callback(move |action: Action| {
        if state.status.peek().busy {
            return;
        }
        match connection.as_ref() {
            Ok((worker, _)) => {
                let request = Request {
                    action,
                    draft: state.draft.peek().clone(),
                };
                match worker.submit(request) {
                    Ok(()) => {
                        let mut status = state.status.write();
                        status.busy = true;
                        status.error = None;
                    }
                    Err(error) => state.status.write().error = Some(error.to_string()),
                }
            }
            Err(error) => state.status.write().error = Some(error.clone()),
        }
    });
    let edit = use_callback(move |body: String| {
        state.draft.set(body.clone());
        if let Some(id) = state.navigation.peek().selected_channel {
            let result = match writer.as_ref() {
                Ok((worker, _)) => worker.edit_draft(Draft {
                    channel_id: id,
                    body,
                }),
                Err(error) => Err(bobo_core::Error::Storage(error.clone())),
            };
            if let Err(error) = result {
                state.draft_error.set(Some(error.to_string()));
            }
        }
    });
    Actions { dispatch, edit }
}

pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}
