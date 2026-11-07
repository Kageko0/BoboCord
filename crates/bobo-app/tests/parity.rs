use bobo_app::{Action, Application, MAX_LOADED_MESSAGES, Reply, Request, Response, Worker};
use bobo_core::{Command, Draft, Error, Repository, Theme};
use bobo_storage::SqliteRepository;

fn call(app: &mut Application, action: Action, draft: &str) -> Reply {
    app.handle(Request {
        action,
        draft: draft.into(),
    })
    .unwrap()
}

fn receive(replies: &async_channel::Receiver<Response>) -> bobo_core::Result<Reply> {
    loop {
        match replies.recv_blocking().unwrap() {
            Response::Updated { result, .. } => return result.map(|reply| *reply),
            Response::DraftSaved(result) => result.unwrap(),
        }
    }
}

#[test]
fn typed_actions_preserve_the_legacy_ui_workflow_and_existing_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bobocord.sqlite3");
    let mut app = Application::open(&path).unwrap();
    assert!(
        call(&mut app, Action::Reload, "")
            .snapshot
            .spaces
            .is_empty()
    );
    let reply = call(
        &mut app,
        Action::Command(Command::CreateSpace("Amigos".into())),
        "",
    );
    let space = reply.snapshot.selected_space.unwrap();
    call(
        &mut app,
        Action::Command(Command::RenameSpace(space, "Casa".into())),
        "",
    );
    let reply = call(
        &mut app,
        Action::Command(Command::CreateChannel(space, "geral".into())),
        "",
    );
    let channel = reply.snapshot.selected_channel.unwrap();
    call(
        &mut app,
        Action::Command(Command::RenameChannel(channel, "conversa".into())),
        "",
    );
    assert!(matches!(
        app.handle(Request {
            action: Action::Command(Command::SendMessage(channel, "  ".into())),
            draft: "  ".into()
        }),
        Err(Error::Validation(_))
    ));
    let reply = call(
        &mut app,
        Action::Command(Command::SendMessage(channel, "Olá, local!".into())),
        "Olá, local!",
    );
    assert!(reply.draft.is_empty());
    assert_eq!(reply.snapshot.messages[0].body, "Olá, local!");
    let reply = call(
        &mut app,
        Action::Command(Command::CreateChannel(space, "segundo".into())),
        "",
    );
    let second = reply.snapshot.selected_channel.unwrap();
    assert!(reply.snapshot.messages.is_empty());
    let reply = call(
        &mut app,
        Action::SelectChannel(channel),
        "rascunho do segundo",
    );
    assert!(reply.draft.is_empty());
    let reply = call(&mut app, Action::SelectChannel(second), "");
    assert_eq!(reply.draft, "rascunho do segundo");
    call(
        &mut app,
        Action::Command(Command::SetTheme(Theme::Light)),
        &reply.draft,
    );
    call(
        &mut app,
        Action::Command(Command::RenameProfile("Felipe".into())),
        &reply.draft,
    );
    drop(app);
    let repository = SqliteRepository::open(&path).unwrap();
    assert_eq!(repository.spaces().unwrap()[0].name, "Casa");
    assert_eq!(repository.profile().unwrap().display_name, "Felipe");
    assert_eq!(repository.preferences().unwrap().theme, Theme::Light);
    assert_eq!(repository.messages(channel).unwrap()[0].body, "Olá, local!");
    drop(repository);
    let mut app = Application::open(&path).unwrap();
    let reply = call(
        &mut app,
        Action::Command(Command::DeleteChannel(second)),
        "",
    );
    assert_eq!(reply.snapshot.channels.len(), 1);
    assert_eq!(reply.snapshot.messages.len(), 1);
    let reply = call(&mut app, Action::Command(Command::DeleteSpace(space)), "");
    assert!(reply.snapshot.spaces.is_empty());
    assert!(reply.snapshot.channels.is_empty());
    assert!(reply.snapshot.messages.is_empty());
}

#[test]
fn worker_can_retry_after_open_failure_without_polling_or_losing_commands() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("blocked");
    std::fs::write(&parent, "not a directory").unwrap();
    let (worker, replies) = Worker::start(parent.join("db.sqlite3")).unwrap();
    assert!(matches!(receive(&replies), Err(Error::Storage(_))));
    std::fs::remove_file(&parent).unwrap();
    worker
        .submit(Request {
            action: Action::Reload,
            draft: String::new(),
        })
        .unwrap();
    assert!(receive(&replies).unwrap().snapshot.spaces.is_empty());
    worker
        .submit(Request {
            action: Action::Command(Command::CreateSpace("Real".into())),
            draft: String::new(),
        })
        .unwrap();
    assert_eq!(receive(&replies).unwrap().snapshot.spaces.len(), 1);
    drop(worker);
}

#[test]
fn worker_shutdown_does_not_wait_for_an_unconsumed_response() {
    let dir = tempfile::tempdir().unwrap();
    let (worker, _replies) = Worker::start(dir.path().join("db.sqlite3")).unwrap();
    drop(worker);
}

#[test]
fn recent_history_is_bounded_and_older_messages_remain_available() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = Application::open(&dir.path().join("db.sqlite3")).unwrap();
    let reply = call(
        &mut app,
        Action::Command(Command::CreateSpace("Volume".into())),
        "",
    );
    let space = reply.snapshot.selected_space.unwrap();
    let reply = call(
        &mut app,
        Action::Command(Command::CreateChannel(space, "texto".into())),
        "",
    );
    let channel = reply.snapshot.selected_channel.unwrap();
    for index in 0..205 {
        call(
            &mut app,
            Action::Command(Command::SendMessage(channel, format!("Mensagem {index}"))),
            "",
        );
    }
    let reply = call(&mut app, Action::Latest, "");
    assert_eq!(reply.snapshot.messages.len(), 50);
    assert!(reply.has_more);
    let reply = call(&mut app, Action::LoadEarlier, "");
    assert_eq!(reply.snapshot.messages.len(), 100);
    assert!(reply.has_more);
    let mut reply = reply;
    while reply.has_more {
        reply = call(&mut app, Action::LoadEarlier, "");
    }
    assert_eq!(reply.snapshot.messages.len(), 205);
    assert!(reply.snapshot.messages.len() <= MAX_LOADED_MESSAGES);
}

#[test]
fn draft_burst_is_coalesced_and_flushed_before_shutdown_and_restored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("draft.sqlite3");
    let (worker, replies) = Worker::start(path.clone()).unwrap();
    receive(&replies).unwrap();
    worker
        .submit(Request {
            action: Action::Command(Command::CreateSpace("Persistente".into())),
            draft: String::new(),
        })
        .unwrap();
    let space = receive(&replies).unwrap().snapshot.selected_space.unwrap();
    worker
        .submit(Request {
            action: Action::Command(Command::CreateChannel(space, "rascunho".into())),
            draft: String::new(),
        })
        .unwrap();
    let channel = receive(&replies)
        .unwrap()
        .snapshot
        .selected_channel
        .unwrap();
    for index in 0..1000 {
        worker
            .edit_draft(Draft {
                channel_id: channel,
                body: format!("rascunho {index}\n🙂"),
            })
            .unwrap();
    }
    drop(worker); // No sleep/debounce: shutdown must flush the latest value.
    let repository = SqliteRepository::open(&path).unwrap();
    assert_eq!(repository.draft(channel).unwrap(), "rascunho 999\n🙂");
    assert_eq!(repository.navigation().unwrap().channel, Some(channel));
    drop(repository);
    let (worker, replies) = Worker::start(path).unwrap();
    let restored = receive(&replies).unwrap();
    assert_eq!(restored.draft, "rascunho 999\n🙂");
    assert_eq!(restored.snapshot.selected_channel, Some(channel));
    drop(worker);
}

#[test]
fn autosave_works_without_navigation_and_an_empty_edit_removes_the_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("draft.sqlite3");
    let mut app = Application::open(&path).unwrap();
    let space = call(
        &mut app,
        Action::Command(Command::CreateSpace("Autosave".into())),
        "",
    )
    .snapshot
    .selected_space
    .unwrap();
    let channel = call(
        &mut app,
        Action::Command(Command::CreateChannel(space, "teste".into())),
        "",
    )
    .snapshot
    .selected_channel
    .unwrap();
    drop(app);
    let (worker, replies) = Worker::start(path.clone()).unwrap();
    receive(&replies).unwrap();
    for body in ["  texto ainda não enviado  ", ""] {
        worker
            .edit_draft(Draft {
                channel_id: channel,
                body: body.into(),
            })
            .unwrap();
        match replies.recv_blocking().unwrap() {
            Response::DraftSaved(result) => result.unwrap(),
            _ => panic!("expected autosave ack"),
        }
        assert_eq!(
            SqliteRepository::open(&path)
                .unwrap()
                .draft(channel)
                .unwrap(),
            body
        );
    }
    drop(worker);
}

#[test]
fn repeated_older_pages_are_deduplicated_and_never_grow_beyond_the_window() {
    use bobo_core::{Id, Message, MessageCursor};
    use std::collections::HashSet;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("window.sqlite3");
    let mut app = Application::open(&path).unwrap();
    let space = call(
        &mut app,
        Action::Command(Command::CreateSpace("Janela".into())),
        "",
    )
    .snapshot
    .selected_space
    .unwrap();
    let channel = call(
        &mut app,
        Action::Command(Command::CreateChannel(space, "histórico".into())),
        "",
    )
    .snapshot
    .selected_channel
    .unwrap();
    let mut repository = SqliteRepository::open(&path).unwrap();
    let author_id = repository.profile().unwrap().id;
    for index in 0..700 {
        repository
            .insert_message(&Message {
                id: Id::new(),
                channel_id: channel,
                author_id,
                body: format!("{index}"),
                created_at: index / 10,
            })
            .unwrap();
    }
    drop(repository);
    let mut reply = call(&mut app, Action::Latest, "");
    let mut seen = HashSet::new();
    loop {
        assert!(reply.snapshot.messages.len() <= MAX_LOADED_MESSAGES);
        assert_eq!(
            reply
                .snapshot
                .messages
                .iter()
                .map(|m| m.id)
                .collect::<HashSet<_>>()
                .len(),
            reply.snapshot.messages.len()
        );
        assert!(
            reply
                .snapshot
                .messages
                .windows(2)
                .all(|pair| MessageCursor::from(&pair[0]) < MessageCursor::from(&pair[1]))
        );
        seen.extend(reply.snapshot.messages.iter().map(|m| m.id));
        if !reply.has_more {
            break;
        }
        reply = call(&mut app, Action::LoadEarlier, "");
    }
    assert_eq!(seen.len(), 700);
    assert!(reply.has_newer);
    let oldest = reply.before;
    assert_eq!(call(&mut app, Action::Reload, "").before, oldest);
    let reply = call(&mut app, Action::Latest, "");
    assert_eq!(reply.snapshot.messages.len(), 50);
    assert!(!reply.has_newer);
}
