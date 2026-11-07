//! Local domain, independent of UI and database implementations.
use std::fmt;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

pub const NAME_LIMIT: usize = 80;
pub const MESSAGE_LIMIT: usize = 4_000;
pub const DEFAULT_PAGE_SIZE: u32 = 50;
pub const MAX_PAGE_SIZE: u32 = 200;
pub const DRAFT_LIMIT: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| Error::Validation("Identificador inválido.".into()))
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Milliseconds since the Unix epoch (UTC), not a synchronization clock.
pub type Timestamp = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Space {
    pub id: Id,
    pub name: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: Id,
    pub space_id: Id,
    pub name: String,
    pub kind: ChannelKind,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: Id,
    pub channel_id: Id,
    pub author_id: Id,
    pub body: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageCursor {
    pub created_at: Timestamp,
    pub id: Id,
}

impl From<&Message> for MessageCursor {
    fn from(message: &Message) -> Self {
        Self {
            created_at: message.created_at,
            id: message.id,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MessagePage {
    /// Chronological presentation order; SQL reads at most limit + one lookahead row.
    pub messages: Vec<Message>,
    pub before: Option<MessageCursor>,
    pub has_more: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NavigationState {
    pub space: Option<Id>,
    pub channel: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub channel_id: Id,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalProfile {
    pub id: Id,
    pub display_name: String,
    pub avatar_path: Option<String>,
}

impl Default for LocalProfile {
    fn default() -> Self {
        Self {
            id: Id::new(),
            display_name: "Você".into(),
            avatar_path: None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Preferences {
    pub theme: Theme,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("O item não existe mais. Atualize a lista.")]
    NotFound,
    #[error("Já existe um item com esse nome neste contexto.")]
    Conflict,
    #[error("Falha no armazenamento local: {0}")]
    Storage(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn validate_name(input: &str) -> Result<String> {
    let name = input.trim();
    if name.is_empty() || name.chars().count() > NAME_LIMIT || name.chars().any(char::is_control) {
        return Err(Error::Validation(format!(
            "Use um nome de 1 a {NAME_LIMIT} caracteres, sem caracteres de controle."
        )));
    }
    Ok(name.into())
}

pub fn validate_body(input: &str) -> Result<String> {
    let body = input.trim();
    if body.is_empty() || body.chars().count() > MESSAGE_LIMIT || body.contains('\0') {
        return Err(Error::Validation(format!(
            "A mensagem deve ter de 1 a {MESSAGE_LIMIT} caracteres e não pode conter NUL."
        )));
    }
    Ok(body.into())
}

/// Each write must be atomic. Deleting a space/channel cascades through its messages.
pub trait Repository {
    fn spaces(&self) -> Result<Vec<Space>>;
    fn channels(&self, space_id: Id) -> Result<Vec<Channel>>;
    fn messages_page(
        &self,
        channel_id: Id,
        before: Option<MessageCursor>,
        limit: u32,
    ) -> Result<MessagePage>;
    /// Compatibility helper for small callers. Always bounded; use cursors for full history.
    fn messages(&self, channel_id: Id) -> Result<Vec<Message>> {
        Ok(self
            .messages_page(channel_id, None, MAX_PAGE_SIZE)?
            .messages)
    }
    fn navigation(&self) -> Result<NavigationState>;
    fn draft(&self, channel_id: Id) -> Result<String>;
    fn save_local_state(
        &mut self,
        navigation: Option<NavigationState>,
        drafts: &[Draft],
    ) -> Result<()>;
    fn profile(&self) -> Result<LocalProfile>;
    fn preferences(&self) -> Result<Preferences>;
    fn insert_space(&mut self, space: &Space) -> Result<()>;
    fn rename_space(&mut self, id: Id, name: &str) -> Result<()>;
    fn delete_space(&mut self, id: Id) -> Result<()>;
    fn insert_channel(&mut self, channel: &Channel) -> Result<()>;
    fn rename_channel(&mut self, id: Id, name: &str) -> Result<()>;
    fn delete_channel(&mut self, id: Id) -> Result<()>;
    fn insert_message(&mut self, message: &Message) -> Result<()>;
    fn set_theme(&mut self, theme: Theme) -> Result<()>;
    fn rename_profile(&mut self, name: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum Command {
    CreateSpace(String),
    RenameSpace(Id, String),
    DeleteSpace(Id),
    CreateChannel(Id, String),
    RenameChannel(Id, String),
    DeleteChannel(Id),
    SendMessage(Id, String),
    SetTheme(Theme),
    RenameProfile(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Updated,
    SpaceCreated(Id),
    ChannelCreated(Id),
}

pub struct Snapshot {
    pub spaces: Vec<Space>,
    pub channels: Vec<Channel>,
    pub messages: Vec<Message>,
    pub selected_space: Option<Id>,
    pub selected_channel: Option<Id>,
    pub profile: LocalProfile,
    pub preferences: Preferences,
}

pub struct Service<R> {
    repository: R,
}

impl<R: Repository> Service<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(&mut self, command: Command) -> Result<Outcome> {
        let now = Utc::now().timestamp_millis();
        match command {
            Command::CreateSpace(name) => {
                let space = Space {
                    id: Id::new(),
                    name: validate_name(&name)?,
                    created_at: now,
                };
                self.repository.insert_space(&space)?;
                return Ok(Outcome::SpaceCreated(space.id));
            }
            Command::RenameSpace(id, name) => {
                self.repository.rename_space(id, &validate_name(&name)?)?
            }
            Command::DeleteSpace(id) => self.repository.delete_space(id)?,
            Command::CreateChannel(space_id, name) => {
                let channel = Channel {
                    id: Id::new(),
                    space_id,
                    name: validate_name(&name)?,
                    kind: ChannelKind::Text,
                    created_at: now,
                };
                self.repository.insert_channel(&channel)?;
                return Ok(Outcome::ChannelCreated(channel.id));
            }
            Command::RenameChannel(id, name) => {
                self.repository.rename_channel(id, &validate_name(&name)?)?
            }
            Command::DeleteChannel(id) => self.repository.delete_channel(id)?,
            Command::SendMessage(channel_id, body) => {
                let body = validate_body(&body)?;
                let profile = self.repository.profile()?;
                self.repository.insert_message(&Message {
                    id: Id::new(),
                    channel_id,
                    author_id: profile.id,
                    body,
                    created_at: now,
                })?;
            }
            Command::SetTheme(theme) => self.repository.set_theme(theme)?,
            Command::RenameProfile(name) => {
                self.repository.rename_profile(&validate_name(&name)?)?
            }
        }
        Ok(Outcome::Updated)
    }

    /// Selection is reconciled with persisted objects after every operation.
    pub fn snapshot(&self, space: Option<Id>, channel: Option<Id>) -> Result<Snapshot> {
        let mut snapshot = self.metadata_snapshot(space, channel)?;
        if let Some(id) = snapshot.selected_channel {
            snapshot.messages = self.messages_page(id, None, DEFAULT_PAGE_SIZE)?.messages;
        }
        Ok(snapshot)
    }

    pub fn messages_page(
        &self,
        channel: Id,
        before: Option<MessageCursor>,
        limit: u32,
    ) -> Result<MessagePage> {
        self.repository
            .messages_page(channel, before, limit.clamp(1, MAX_PAGE_SIZE))
    }

    pub fn navigation(&self) -> Result<NavigationState> {
        self.repository.navigation()
    }
    pub fn draft(&self, channel: Id) -> Result<String> {
        self.repository.draft(channel)
    }
    pub fn save_local_state(
        &mut self,
        navigation: Option<NavigationState>,
        drafts: &[Draft],
    ) -> Result<()> {
        for draft in drafts {
            if draft.body.chars().count() > DRAFT_LIMIT || draft.body.contains('\0') {
                return Err(Error::Validation(format!(
                    "O rascunho aceita até {DRAFT_LIMIT} caracteres e não pode conter NUL."
                )));
            }
        }
        self.repository.save_local_state(navigation, drafts)
    }

    /// Metadata only: changes to theme, profile or draft never read message history.
    pub fn metadata_snapshot(&self, space: Option<Id>, channel: Option<Id>) -> Result<Snapshot> {
        let spaces = self.repository.spaces()?;
        let selected_space = spaces
            .iter()
            .find(|s| Some(s.id) == space)
            .or(spaces.first())
            .map(|s| s.id);
        let channels = match selected_space {
            Some(id) => self.repository.channels(id)?,
            None => vec![],
        };
        let selected_channel = channels
            .iter()
            .find(|c| Some(c.id) == channel)
            .or(channels.first())
            .map(|c| c.id);
        Ok(Snapshot {
            spaces,
            channels,
            messages: Vec::new(),
            selected_space,
            selected_channel,
            profile: self.repository.profile()?,
            preferences: self.repository.preferences()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_count_unicode_characters() {
        assert_eq!(validate_name("  Café  ").unwrap(), "Café");
        assert!(validate_name(&"é".repeat(NAME_LIMIT)).is_ok());
        for value in [
            " ".into(),
            "a\nb".into(),
            "a\0b".into(),
            "é".repeat(NAME_LIMIT + 1),
        ] {
            assert!(matches!(validate_name(&value), Err(Error::Validation(_))));
        }
    }

    #[test]
    fn messages_allow_multiline_but_reject_empty_oversized_and_nul() {
        assert_eq!(validate_body("  oi\nmundo  ").unwrap(), "oi\nmundo");
        assert!(validate_body(&"🙂".repeat(MESSAGE_LIMIT)).is_ok());
        for value in ["\n ".into(), "a\0b".into(), "🙂".repeat(MESSAGE_LIMIT + 1)] {
            assert!(matches!(validate_body(&value), Err(Error::Validation(_))));
        }
    }

    #[test]
    fn ids_round_trip_and_are_distinct() {
        let id = Id::new();
        assert_eq!(Id::parse(&id.to_string()).unwrap(), id);
        assert_ne!(id, Id::new());
        assert!(Id::parse("not-an-id").is_err());
    }
}
