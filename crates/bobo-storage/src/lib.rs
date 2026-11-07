//! SQLite adapter. The connection belongs to the caller's storage worker.
use std::{path::Path, time::Duration};

use bobo_core::{
    Channel, ChannelKind, Draft, Error, Id, LocalProfile, MAX_PAGE_SIZE, Message, MessageCursor,
    MessagePage, NavigationState, Preferences, Repository, Result, Space, Theme,
};
use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};

pub struct SqliteRepository {
    connection: Connection,
}

fn map_error(error: rusqlite::Error) -> Error {
    match &error {
        rusqlite::Error::SqliteFailure(code, _) if code.code == ErrorCode::ConstraintViolation => {
            match code.extended_code {
                2067 | 1555 => Error::Conflict,
                787 => Error::NotFound,
                _ => Error::Storage("Uma restrição de integridade foi violada.".into()),
            }
        }
        _ => Error::Storage(error.to_string()),
    }
}

fn read_id(row: &Row<'_>, column: usize) -> rusqlite::Result<Id> {
    let value: String = row.get(column)?;
    Id::parse(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

impl SqliteRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut connection = Connection::open(path).map_err(map_error)?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(map_error)?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(map_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(map_error)?;
        if version > 2 {
            return Err(Error::Storage(
                "Banco criado por uma versão mais recente. Nenhuma migração foi aplicada.".into(),
            ));
        }
        if version < 2 {
            let transaction = connection.transaction().map_err(map_error)?;
            if version == 0 {
                transaction
                    .execute_batch(include_str!("../migrations/001_foundation.sql"))
                    .map_err(map_error)?;
                let profile = LocalProfile::default();
                transaction
                    .execute(
                        "INSERT INTO local_profile(singleton, id, display_name) VALUES (1, ?1, ?2)",
                        params![profile.id.to_string(), profile.display_name],
                    )
                    .map_err(map_error)?;
            }
            transaction
                .execute_batch(include_str!("../migrations/002_local_ui.sql"))
                .map_err(map_error)?;
            transaction
                .pragma_update(None, "user_version", 2)
                .map_err(map_error)?;
            transaction.commit().map_err(map_error)?;
        }
        Ok(Self { connection })
    }

    fn change(&mut self, sql: &str, parameters: impl rusqlite::Params) -> Result<()> {
        match self
            .connection
            .execute(sql, parameters)
            .map_err(map_error)?
        {
            0 => Err(Error::NotFound),
            _ => Ok(()),
        }
    }
}

impl Repository for SqliteRepository {
    fn spaces(&self) -> Result<Vec<Space>> {
        let mut statement = self
            .connection
            .prepare("SELECT id, name, created_at FROM spaces ORDER BY created_at, id")
            .map_err(map_error)?;
        statement
            .query_map([], |row| {
                Ok(Space {
                    id: read_id(row, 0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })
            .map_err(map_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_error)
    }

    fn channels(&self, space_id: Id) -> Result<Vec<Channel>> {
        let mut statement = self.connection.prepare("SELECT id, space_id, name, created_at FROM channels WHERE space_id = ?1 ORDER BY created_at, id").map_err(map_error)?;
        statement
            .query_map([space_id.to_string()], |row| {
                Ok(Channel {
                    id: read_id(row, 0)?,
                    space_id: read_id(row, 1)?,
                    name: row.get(2)?,
                    kind: ChannelKind::Text,
                    created_at: row.get(3)?,
                })
            })
            .map_err(map_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_error)
    }

    fn messages_page(
        &self,
        channel_id: Id,
        before: Option<MessageCursor>,
        limit: u32,
    ) -> Result<MessagePage> {
        let limit = limit.clamp(1, MAX_PAGE_SIZE) as usize;
        let sql = if before.is_some() {
            "SELECT id, channel_id, author_id, body, created_at FROM messages WHERE channel_id = ?1 AND (created_at, id) < (?2, ?3) ORDER BY created_at DESC, id DESC LIMIT ?4"
        } else {
            "SELECT id, channel_id, author_id, body, created_at FROM messages WHERE channel_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2"
        };
        let mut statement = self.connection.prepare_cached(sql).map_err(map_error)?;
        let read_message = |row: &Row<'_>| {
            Ok(Message {
                id: read_id(row, 0)?,
                channel_id: read_id(row, 1)?,
                author_id: read_id(row, 2)?,
                body: row.get(3)?,
                created_at: row.get(4)?,
            })
        };
        let mut messages = match before {
            Some(cursor) => statement.query_map(
                params![
                    channel_id.to_string(),
                    cursor.created_at,
                    cursor.id.to_string(),
                    limit + 1
                ],
                read_message,
            ),
            None => statement.query_map(params![channel_id.to_string(), limit + 1], read_message),
        }
        .map_err(map_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(map_error)?;
        let has_more = messages.len() > limit;
        messages.truncate(limit);
        messages.reverse();
        let before = messages.first().map(MessageCursor::from);
        Ok(MessagePage {
            messages,
            before,
            has_more,
        })
    }

    fn navigation(&self) -> Result<NavigationState> {
        self.connection
            .query_row(
                "SELECT space_id, channel_id FROM local_navigation WHERE singleton = 1",
                [],
                |row| {
                    let parse = |index| -> rusqlite::Result<Option<Id>> {
                        let raw: Option<String> = row.get(index)?;
                        raw.map(|value| {
                            Id::parse(&value).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    index,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })
                        })
                        .transpose()
                    };
                    Ok(NavigationState {
                        space: parse(0)?,
                        channel: parse(1)?,
                    })
                },
            )
            .map_err(map_error)
    }

    fn draft(&self, channel_id: Id) -> Result<String> {
        Ok(self
            .connection
            .query_row(
                "SELECT body FROM channel_drafts WHERE channel_id = ?1",
                [channel_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_error)?
            .unwrap_or_default())
    }

    fn save_local_state(
        &mut self,
        navigation: Option<NavigationState>,
        drafts: &[Draft],
    ) -> Result<()> {
        let transaction = self.connection.transaction().map_err(map_error)?;
        for draft in drafts {
            if draft.body.is_empty() {
                transaction
                    .execute(
                        "DELETE FROM channel_drafts WHERE channel_id = ?1",
                        [draft.channel_id.to_string()],
                    )
                    .map_err(map_error)?;
            } else {
                // A delayed autosave for a deleted channel must never resurrect an orphan.
                transaction.execute("INSERT INTO channel_drafts(channel_id, body) SELECT ?1, ?2 WHERE EXISTS(SELECT 1 FROM channels WHERE id = ?1) ON CONFLICT(channel_id) DO UPDATE SET body = excluded.body", params![draft.channel_id.to_string(), draft.body]).map_err(map_error)?;
            }
        }
        if let Some(selection) = navigation {
            transaction.execute("UPDATE local_navigation SET space_id = ?1, channel_id = ?2 WHERE singleton = 1", params![selection.space.map(|id| id.to_string()), selection.channel.map(|id| id.to_string())]).map_err(map_error)?;
        }
        transaction.commit().map_err(map_error)
    }

    fn profile(&self) -> Result<LocalProfile> {
        self.connection
            .query_row(
                "SELECT id, display_name, avatar_path FROM local_profile WHERE singleton = 1",
                [],
                |row| {
                    Ok(LocalProfile {
                        id: read_id(row, 0)?,
                        display_name: row.get(1)?,
                        avatar_path: row.get(2)?,
                    })
                },
            )
            .map_err(map_error)
    }

    fn preferences(&self) -> Result<Preferences> {
        self.connection
            .query_row(
                "SELECT theme FROM preferences WHERE singleton = 1",
                [],
                |row| {
                    let theme: String = row.get(0)?;
                    Ok(Preferences {
                        theme: if theme == "light" {
                            Theme::Light
                        } else {
                            Theme::Dark
                        },
                    })
                },
            )
            .map_err(map_error)
    }

    fn insert_space(&mut self, space: &Space) -> Result<()> {
        self.change(
            "INSERT INTO spaces(id, name, created_at) VALUES (?1, ?2, ?3)",
            params![space.id.to_string(), space.name, space.created_at],
        )
    }

    fn rename_space(&mut self, id: Id, name: &str) -> Result<()> {
        self.change(
            "UPDATE spaces SET name = ?1 WHERE id = ?2",
            params![name, id.to_string()],
        )
    }

    fn delete_space(&mut self, id: Id) -> Result<()> {
        self.change("DELETE FROM spaces WHERE id = ?1", [id.to_string()])
    }

    fn insert_channel(&mut self, channel: &Channel) -> Result<()> {
        self.change("INSERT INTO channels(id, space_id, name, kind, created_at) VALUES (?1, ?2, ?3, 'text', ?4)", params![channel.id.to_string(), channel.space_id.to_string(), channel.name, channel.created_at])
    }

    fn rename_channel(&mut self, id: Id, name: &str) -> Result<()> {
        self.change(
            "UPDATE channels SET name = ?1 WHERE id = ?2",
            params![name, id.to_string()],
        )
    }

    fn delete_channel(&mut self, id: Id) -> Result<()> {
        self.change("DELETE FROM channels WHERE id = ?1", [id.to_string()])
    }

    fn insert_message(&mut self, message: &Message) -> Result<()> {
        let transaction = self.connection.transaction().map_err(map_error)?;
        transaction.execute("INSERT INTO messages(id, channel_id, author_id, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![message.id.to_string(), message.channel_id.to_string(), message.author_id.to_string(), message.body, message.created_at]).map_err(map_error)?;
        transaction
            .execute(
                "DELETE FROM channel_drafts WHERE channel_id = ?1",
                [message.channel_id.to_string()],
            )
            .map_err(map_error)?;
        transaction.commit().map_err(map_error)
    }

    fn set_theme(&mut self, theme: Theme) -> Result<()> {
        self.change(
            "UPDATE preferences SET theme = ?1 WHERE singleton = 1",
            [if theme == Theme::Light {
                "light"
            } else {
                "dark"
            }],
        )
    }

    fn rename_profile(&mut self, name: &str) -> Result<()> {
        self.change(
            "UPDATE local_profile SET display_name = ?1 WHERE singleton = 1",
            [name],
        )
    }
}
