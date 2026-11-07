use bobo_core::{
    Channel, ChannelKind, Draft, Id, LocalProfile, MAX_PAGE_SIZE, Message, MessageCursor,
    NavigationState, Repository, Space,
};
use bobo_storage::SqliteRepository;
use rusqlite::{Connection, params};
use std::{collections::HashSet, time::Instant};

fn channel(repository: &mut SqliteRepository) -> (Id, Id) {
    let space = Id::new();
    let channel = Id::new();
    repository
        .insert_space(&Space {
            id: space,
            name: "Teste".into(),
            created_at: 1,
        })
        .unwrap();
    repository
        .insert_channel(&Channel {
            id: channel,
            space_id: space,
            name: "texto".into(),
            kind: ChannelKind::Text,
            created_at: 1,
        })
        .unwrap();
    (space, channel)
}

#[test]
fn ten_thousand_messages_keyset_is_stable_indexed_and_isolated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("load.sqlite3");
    let mut repository = SqliteRepository::open(&path).unwrap();
    let (space, first) = channel(&mut repository);
    let mut channels = vec![first];
    for index in 1..4 {
        let id = Id::new();
        repository
            .insert_channel(&Channel {
                id,
                space_id: space,
                name: format!("canal-{index}"),
                kind: ChannelKind::Text,
                created_at: 1,
            })
            .unwrap();
        channels.push(id);
    }
    let author = repository.profile().unwrap().id;
    let mut connection = Connection::open(&path).unwrap();
    let transaction = connection.transaction().unwrap();
    let mut expected = Vec::new();
    for index in 0..10_000 {
        let id = Id::new();
        let time = 1_700_000_000_000 + index / 100;
        let destination = channels[index as usize % 4];
        transaction
            .execute(
                "INSERT INTO messages VALUES (?1,?2,?3,?4,?5)",
                params![
                    id.to_string(),
                    destination.to_string(),
                    author.to_string(),
                    format!("Fixture {index}"),
                    time
                ],
            )
            .unwrap();
        if destination == first {
            expected.push(MessageCursor {
                id,
                created_at: time,
            });
        }
    }
    transaction.commit().unwrap();
    expected.sort();
    let started = Instant::now();
    let mut before = None;
    let mut seen = HashSet::new();
    let mut pages = 0;
    loop {
        let page = repository.messages_page(first, before, 50).unwrap();
        assert!(page.messages.len() <= 50);
        assert!(
            page.messages
                .windows(2)
                .all(|pair| MessageCursor::from(&pair[0]) < MessageCursor::from(&pair[1]))
        );
        for message in &page.messages {
            assert_eq!(message.channel_id, first);
            assert!(seen.insert(message.id), "duplicate ID across cursor pages");
        }
        if pages == 0 {
            repository
                .insert_message(&Message {
                    id: Id::new(),
                    channel_id: first,
                    author_id: author,
                    body: "nova após cursor".into(),
                    created_at: 1_800_000_000_000,
                })
                .unwrap();
        }
        pages += 1;
        if !page.has_more {
            break;
        }
        assert!(page.before < before || before.is_none());
        before = page.before;
    }
    assert_eq!(seen.len(), 2500);
    assert!(expected.iter().all(|cursor| seen.contains(&cursor.id)));
    println!(
        "10k rows / four channels: {pages} indexed pages in {:?}",
        started.elapsed()
    );
    assert_eq!(
        repository
            .messages_page(first, None, 0)
            .unwrap()
            .messages
            .len(),
        1
    );
    assert_eq!(
        repository
            .messages_page(first, None, u32::MAX)
            .unwrap()
            .messages
            .len(),
        MAX_PAGE_SIZE as usize
    );
    let plan: Vec<String> = connection.prepare("EXPLAIN QUERY PLAN SELECT id, body FROM messages WHERE channel_id=?1 AND (created_at,id)<(?2,?3) ORDER BY created_at DESC,id DESC LIMIT ?4").unwrap()
        .query_map(params![first.to_string(), i64::MAX, first.to_string(), 51], |row| row.get(3)).unwrap().map(Result::unwrap).collect();
    assert!(
        plan.iter()
            .any(|line| line.contains("SEARCH") && line.contains("messages_by_channel")),
        "{plan:?}"
    );
    assert!(
        !plan.iter().any(|line| line.contains("TEMP B-TREE")),
        "{plan:?}"
    );
    println!("Query plan: {plan:?}");
}

#[test]
fn upgrade_v1_keeps_ids_messages_profile_and_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v1.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_foundation.sql"))
        .unwrap();
    let profile = LocalProfile::default();
    let space = Id::new();
    let channel = Id::new();
    let message = Id::new();
    connection
        .execute(
            "INSERT INTO local_profile VALUES (1,?1,'Antigo',NULL)",
            [profile.id.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO spaces VALUES (?1,'Original',123)",
            [space.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO channels VALUES (?1,?2,'original','text',456)",
            params![channel.to_string(), space.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO messages VALUES (?1,?2,?3,'histórico existente',789)",
            params![
                message.to_string(),
                channel.to_string(),
                profile.id.to_string()
            ],
        )
        .unwrap();
    connection
        .execute_batch("UPDATE preferences SET theme='light'; PRAGMA user_version=1;")
        .unwrap();
    drop(connection);
    for _ in 0..2 {
        let repository = SqliteRepository::open(&path).unwrap();
        assert_eq!(repository.profile().unwrap().id, profile.id);
        assert_eq!(
            repository.preferences().unwrap().theme,
            bobo_core::Theme::Light
        );
        let page = repository.messages_page(channel, None, 50).unwrap();
        assert_eq!(page.messages[0].id, message);
        assert_eq!(page.messages[0].body, "histórico existente");
        assert_eq!(page.messages[0].created_at, 789);
    }
    let connection = Connection::open(path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn failed_second_migration_rolls_back_without_touching_v1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v1.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_foundation.sql"))
        .unwrap();
    connection
        .execute_batch("CREATE TABLE local_navigation(existing INTEGER); PRAGMA user_version=1;")
        .unwrap();
    assert!(SqliteRepository::open(&path).is_err());
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='channel_drafts'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn local_state_is_transactional_and_cascades_and_send_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ui.sqlite3");
    let mut repository = SqliteRepository::open(&path).unwrap();
    let (space, channel) = channel(&mut repository);
    let selection = NavigationState {
        space: Some(space),
        channel: Some(channel),
    };
    let draft = Draft {
        channel_id: channel,
        body: "  rascunho\n🙂  ".into(),
    };
    repository
        .save_local_state(Some(selection), std::slice::from_ref(&draft))
        .unwrap();
    assert!(
        repository
            .save_local_state(
                Some(NavigationState {
                    space: Some(Id::new()),
                    channel: Some(channel)
                }),
                &[Draft {
                    channel_id: channel,
                    body: "não gravar".into()
                }]
            )
            .is_err()
    );
    assert_eq!(repository.draft(channel).unwrap(), draft.body);
    drop(repository);
    let mut repository = SqliteRepository::open(&path).unwrap();
    assert_eq!(repository.navigation().unwrap(), selection);
    assert_eq!(repository.draft(channel).unwrap(), draft.body);
    let message = Message {
        id: Id::new(),
        channel_id: channel,
        author_id: repository.profile().unwrap().id,
        body: "enviado".into(),
        created_at: 1,
    };
    repository.insert_message(&message).unwrap();
    assert!(repository.draft(channel).unwrap().is_empty());
    repository
        .save_local_state(None, std::slice::from_ref(&draft))
        .unwrap();
    assert!(repository.insert_message(&message).is_err());
    assert_eq!(repository.draft(channel).unwrap(), draft.body);
    repository.delete_space(space).unwrap();
    assert_eq!(repository.navigation().unwrap(), NavigationState::default());
    assert!(repository.draft(channel).unwrap().is_empty());
    repository.save_local_state(None, &[draft]).unwrap(); // Delayed save after deletion is ignored.
    assert!(repository.draft(channel).unwrap().is_empty());
}
