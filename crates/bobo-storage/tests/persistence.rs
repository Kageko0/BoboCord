use bobo_core::{Command, Error, Id, Outcome, Repository, Service, Theme};
use bobo_storage::SqliteRepository;
use tempfile::TempDir;

fn create_space(service: &mut Service<SqliteRepository>, name: &str) -> Id {
    match service.execute(Command::CreateSpace(name.into())).unwrap() {
        Outcome::SpaceCreated(id) => id,
        _ => panic!("expected a new space"),
    }
}

fn create_channel(service: &mut Service<SqliteRepository>, space: Id, name: &str) -> Id {
    match service
        .execute(Command::CreateChannel(space, name.into()))
        .unwrap()
    {
        Outcome::ChannelCreated(id) => id,
        _ => panic!("expected a new channel"),
    }
}

#[test]
fn full_local_workflow_survives_reopen_and_migration_is_idempotent() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("local.sqlite3");
    let mut service = Service::new(SqliteRepository::open(&path).unwrap());
    let initial = service.snapshot(None, None).unwrap();
    assert!(initial.spaces.is_empty());
    let profile_id = initial.profile.id;
    let space = create_space(&mut service, " Amigos ");
    let channel = create_channel(&mut service, space, "geral");
    service
        .execute(Command::RenameSpace(space, "Casa".into()))
        .unwrap();
    service
        .execute(Command::RenameChannel(channel, "conversa".into()))
        .unwrap();
    service
        .execute(Command::RenameProfile("Felipe".into()))
        .unwrap();
    service
        .execute(Command::SendMessage(channel, "oi\nmundo".into()))
        .unwrap();
    service.execute(Command::SetTheme(Theme::Light)).unwrap();
    let before = service.snapshot(Some(space), Some(channel)).unwrap();
    assert_eq!(before.messages.len(), 1);
    assert!(before.messages[0].created_at > 0);
    assert_eq!(before.messages[0].author_id, profile_id);
    drop(service);

    let service = Service::new(SqliteRepository::open(&path).unwrap());
    let after = service.snapshot(Some(space), Some(channel)).unwrap();
    assert_eq!(after.spaces, before.spaces);
    assert_eq!(after.channels, before.channels);
    assert_eq!(after.messages, before.messages);
    assert_eq!(after.profile.id, profile_id);
    assert_eq!(after.profile.display_name, "Felipe");
    assert_eq!(after.preferences.theme, Theme::Light);
    assert_eq!(after.spaces[0].name, "Casa");
    assert_eq!(after.channels[0].name, "conversa");
}

#[test]
fn cascades_are_scoped_to_the_deleted_space_or_channel() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("cascade.sqlite3");
    let mut service = Service::new(SqliteRepository::open(&path).unwrap());
    let a = create_space(&mut service, "A");
    let b = create_space(&mut service, "B");
    let a1 = create_channel(&mut service, a, "primeiro");
    let a2 = create_channel(&mut service, a, "segundo");
    let b1 = create_channel(&mut service, b, "primeiro");
    for channel in [a1, a2, b1] {
        service
            .execute(Command::SendMessage(channel, "persistente".into()))
            .unwrap();
    }
    service.execute(Command::DeleteChannel(a1)).unwrap();
    let snapshot = service.snapshot(Some(a), Some(a1)).unwrap();
    assert_eq!(snapshot.selected_channel, Some(a2));
    assert_eq!(snapshot.messages.len(), 1);
    service.execute(Command::DeleteSpace(a)).unwrap();
    let snapshot = service.snapshot(Some(a), Some(a2)).unwrap();
    assert_eq!(snapshot.selected_space, Some(b));
    assert_eq!(snapshot.messages.len(), 1);
    drop(service);
    let repository = SqliteRepository::open(&path).unwrap();
    assert!(repository.messages(a1).unwrap().is_empty());
    assert!(repository.messages(a2).unwrap().is_empty());
    assert!(repository.channels(a).unwrap().is_empty());
    assert_eq!(repository.messages(b1).unwrap().len(), 1);
}

#[test]
fn validation_uniqueness_foreign_keys_and_missing_items_are_recoverable() {
    let directory = TempDir::new().unwrap();
    let mut service =
        Service::new(SqliteRepository::open(directory.path().join("rules.sqlite3")).unwrap());
    assert!(matches!(
        service.execute(Command::CreateSpace(" ".into())),
        Err(Error::Validation(_))
    ));
    let space = create_space(&mut service, "Casa");
    assert!(matches!(
        service.execute(Command::CreateSpace("casa".into())),
        Err(Error::Conflict)
    ));
    let channel = create_channel(&mut service, space, "geral");
    assert!(matches!(
        service.execute(Command::CreateChannel(space, "GERAL".into())),
        Err(Error::Conflict)
    ));
    let other = create_space(&mut service, "Outro");
    create_channel(&mut service, other, "geral");
    assert!(matches!(
        service.execute(Command::CreateChannel(Id::new(), "órfão".into())),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service.execute(Command::SendMessage(Id::new(), "oi".into())),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service.execute(Command::SendMessage(channel, "\n".into())),
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        service.execute(Command::RenameChannel(channel, "\t".into())),
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        service.execute(Command::DeleteSpace(Id::new())),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service.execute(Command::RenameSpace(space, "Outro".into())),
        Err(Error::Conflict)
    ));
    let snapshot = service.snapshot(Some(space), Some(channel)).unwrap();
    assert_eq!(snapshot.spaces.len(), 2);
    assert_eq!(
        snapshot.spaces.iter().find(|s| s.id == space).unwrap().name,
        "Casa"
    );
    assert!(snapshot.messages.is_empty());
}

#[test]
fn newer_schema_is_refused_without_modification() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("future.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    assert!(matches!(
        SqliteRepository::open(&path),
        Err(Error::Storage(_))
    ));
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
}

#[test]
fn failed_migration_rolls_back_schema_and_version() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("broken.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE channels (existing INTEGER);")
        .unwrap();
    assert!(SqliteRepository::open(&path).is_err());
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 0);
    let spaces: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'spaces'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(spaces, 0);
}
