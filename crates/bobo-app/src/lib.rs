//! Application commands and SQLite worker, independent of the rendering framework.
mod worker;
pub use worker::{DRAFT_DEBOUNCE, Response, Worker};

use bobo_core::{
    Command, DEFAULT_PAGE_SIZE, Draft, Error, Id, Message, MessageCursor, NavigationState, Outcome,
    Result, Service, Snapshot,
};
use bobo_storage::SqliteRepository;
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

pub fn database_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("BOBOCORD_DATA_DIR").filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(path).join("bobocord.sqlite3"));
    }
    #[cfg(target_os = "windows")]
    let base = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.filter(|path| path.is_absolute())
        .map(|path| path.join("BoboCord").join("bobocord.sqlite3"))
        .ok_or_else(|| {
            Error::Storage("Diretório de usuário indisponível. Defina BOBOCORD_DATA_DIR.".into())
        })
}

#[derive(Debug, Clone)]
pub enum Action {
    Reload,
    LoadEarlier,
    Latest,
    SelectSpace(Id),
    SelectChannel(Id),
    Command(Command),
}

#[derive(Debug, Clone)]
pub struct Request {
    pub action: Action,
    pub draft: String,
}

pub struct Reply {
    pub snapshot: Snapshot,
    pub draft: String,
    pub has_more: bool,
    pub has_newer: bool,
    pub before: Option<MessageCursor>,
    pub scroll: ScrollIntent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScrollIntent {
    Reset,
    Prepend,
    Append,
    #[default]
    Preserve,
}

pub const MAX_LOADED_MESSAGES: usize = 300;

pub struct Application {
    service: Service<SqliteRepository>,
    space: Option<Id>,
    channel: Option<Id>,
    messages: Vec<Message>,
    has_more: bool,
    has_newer: bool,
    initialized: bool,
    persisted_navigation: NavigationState,
}

impl Application {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| Error::Storage(error.to_string()))?;
        }
        let service = Service::new(SqliteRepository::open(path)?);
        let navigation = service.navigation()?;
        Ok(Self {
            service,
            space: navigation.space,
            channel: navigation.channel,
            messages: Vec::new(),
            has_more: false,
            has_newer: false,
            initialized: false,
            persisted_navigation: navigation,
        })
    }

    pub fn handle(&mut self, request: Request) -> Result<Reply> {
        if self.initialized
            && let Some(channel) = self.channel
            && self.service.draft(channel)? != request.draft
        {
            self.persist_drafts(&[Draft {
                channel_id: channel,
                body: request.draft,
            }])?;
        }
        let previous_channel = self.channel;
        let earlier = matches!(request.action, Action::LoadEarlier);
        let latest = matches!(request.action, Action::Latest);
        let refresh = matches!(
            request.action,
            Action::Reload | Action::Command(Command::SendMessage(..))
        );
        match request.action {
            Action::Reload | Action::LoadEarlier | Action::Latest => {}
            Action::SelectSpace(id) => {
                self.space = Some(id);
                self.channel = None;
            }
            Action::SelectChannel(id) => {
                self.channel = Some(id);
            }
            Action::Command(command) => match self.service.execute(command)? {
                Outcome::SpaceCreated(id) => {
                    self.space = Some(id);
                    self.channel = None;
                }
                Outcome::ChannelCreated(id) => self.channel = Some(id),
                Outcome::Updated => {}
            },
        }
        let mut snapshot = self.service.metadata_snapshot(self.space, self.channel)?;
        let mut scroll = ScrollIntent::Preserve;
        if let Some(channel) = snapshot.selected_channel {
            if !self.initialized || snapshot.selected_channel != previous_channel || latest {
                let page = self
                    .service
                    .messages_page(channel, None, DEFAULT_PAGE_SIZE)?;
                self.messages = page.messages;
                self.has_more = page.has_more;
                self.has_newer = false;
                scroll = ScrollIntent::Reset;
            } else if earlier && self.has_more {
                let before = self.messages.first().map(MessageCursor::from);
                let page = self
                    .service
                    .messages_page(channel, before, DEFAULT_PAGE_SIZE)?;
                self.messages = merge_messages(std::mem::take(&mut self.messages), page.messages);
                self.has_more = page.has_more;
                if self.messages.len() > MAX_LOADED_MESSAGES {
                    self.messages.truncate(MAX_LOADED_MESSAGES);
                    self.has_newer = true;
                }
                scroll = ScrollIntent::Prepend;
            } else if refresh && !self.has_newer {
                let page = self
                    .service
                    .messages_page(channel, None, DEFAULT_PAGE_SIZE)?;
                let overlap = self.messages.is_empty()
                    || page
                        .messages
                        .iter()
                        .any(|new| self.messages.iter().any(|old| old.id == new.id));
                if overlap {
                    if self.messages.is_empty() {
                        self.has_more = page.has_more;
                    }
                    self.messages =
                        merge_messages(std::mem::take(&mut self.messages), page.messages);
                    if self.messages.len() > MAX_LOADED_MESSAGES {
                        self.messages = self
                            .messages
                            .split_off(self.messages.len() - MAX_LOADED_MESSAGES);
                        self.has_more = true;
                    }
                    scroll = ScrollIntent::Append;
                } else {
                    // Do not present a discontinuous history after a large external batch.
                    self.has_newer = true;
                }
            }
        } else {
            self.messages.clear();
            self.has_more = false;
            self.has_newer = false;
            scroll = ScrollIntent::Reset;
        }
        self.space = snapshot.selected_space;
        self.channel = snapshot.selected_channel;
        let navigation = NavigationState {
            space: self.space,
            channel: self.channel,
        };
        if navigation != self.persisted_navigation {
            self.service.save_local_state(Some(navigation), &[])?;
            self.persisted_navigation = navigation;
        }
        self.initialized = true;
        snapshot.messages = self.messages.clone();
        let draft = match self.channel {
            Some(id) => self.service.draft(id)?,
            None => String::new(),
        };
        Ok(Reply {
            snapshot,
            draft,
            has_more: self.has_more,
            has_newer: self.has_newer,
            before: self.messages.first().map(MessageCursor::from),
            scroll,
        })
    }

    pub fn persist_drafts(&mut self, drafts: &[Draft]) -> Result<()> {
        self.service.save_local_state(None, drafts)
    }
}

fn merge_messages(existing: Vec<Message>, incoming: Vec<Message>) -> Vec<Message> {
    let mut by_id: BTreeMap<_, _> = existing
        .into_iter()
        .map(|message| (message.id, message))
        .collect();
    for message in incoming {
        by_id.insert(message.id, message);
    }
    let mut messages: Vec<_> = by_id.into_values().collect();
    messages.sort_by_key(|message| MessageCursor::from(message));
    messages
}
