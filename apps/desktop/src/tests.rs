//! Port of the original Slint callback parity test to Dioxus signals and actions.
use crate::{
    components::shell::AppView,
    state::{Actions, ModalKind, UiState, use_ui_state, use_worker},
};
use bobo_app::Action;
use bobo_core::{Command, Repository, Theme};
use bobo_storage::SqliteRepository;
use dioxus::dioxus_core::NoOpMutations;
use dioxus::prelude::*;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Clone)]
struct Probe(Rc<RefCell<Option<(UiState, Actions)>>>);
impl PartialEq for Probe {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
fn TestApp(probe: Probe, path: PathBuf) -> Element {
    let state = use_ui_state();
    let actions = use_worker(state, Ok(path));
    use_context_provider(|| state);
    use_context_provider(|| actions);
    *probe.0.borrow_mut() = Some((state, actions));
    rsx! { AppView {} }
}

fn settle(dom: &mut VirtualDom, state: UiState) {
    pollster::block_on(async {
        loop {
            dom.render_immediate(&mut NoOpMutations);
            if !state.status.peek().busy {
                break;
            }
            dom.wait_for_work().await;
        }
    });
}

#[test]
fn ui_callbacks_persist_crud_messages_theme_and_recover_from_validation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ui.sqlite3");
    let probe = Probe(Rc::new(RefCell::new(None)));
    let mut dom = VirtualDom::new_with_props(
        TestApp,
        TestAppProps {
            probe: probe.clone(),
            path: path.clone(),
        },
    );
    dom.rebuild_in_place();
    let (mut state, actions) = probe.0.borrow().unwrap();
    settle(&mut dom, state);
    assert!(state.status.peek().ready);
    assert!(state.navigation.peek().spaces.is_empty());
    state.open_modal(ModalKind::CreateSpace, "Criar espaço", "");
    actions.command(Command::CreateSpace("Amigos".into()));
    settle(&mut dom, state);
    let space = state.navigation.peek().selected_space.unwrap();
    assert!(state.modal.peek().is_none());
    actions.command(Command::RenameSpace(space, "Casa".into()));
    settle(&mut dom, state);
    assert_eq!(state.navigation.peek().space_name(), "Casa");
    actions.command(Command::CreateChannel(space, "geral".into()));
    settle(&mut dom, state);
    let channel = state.navigation.peek().selected_channel.unwrap();
    actions.command(Command::RenameChannel(channel, "conversa".into()));
    settle(&mut dom, state);
    assert_eq!(state.navigation.peek().channel_name(), "conversa");
    state.draft.set("  ".into());
    actions.command(Command::SendMessage(channel, "  ".into()));
    settle(&mut dom, state);
    assert!(state.status.peek().error.is_some());
    assert_eq!(*state.draft.peek(), "  ");
    assert!(state.history.peek().messages.is_empty());
    state
        .draft
        .set("Olá, local! <script>alert(1)</script>".into());
    actions.command(Command::SendMessage(channel, state.draft.peek().clone()));
    settle(&mut dom, state);
    assert!(state.status.peek().error.is_none());
    assert_eq!(state.history.peek().messages.len(), 1);
    assert!(state.draft.peek().is_empty());
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("&#60;script&#62;"));
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(html.contains("UTC"));
    actions.command(Command::CreateChannel(space, "segundo".into()));
    settle(&mut dom, state);
    let second = state.navigation.peek().selected_channel.unwrap();
    assert!(state.history.peek().messages.is_empty());
    state.draft.set("rascunho do segundo".into());
    actions.dispatch(Action::SelectChannel(channel));
    settle(&mut dom, state);
    assert!(state.draft.peek().is_empty());
    assert_eq!(state.history.peek().messages.len(), 1);
    actions.dispatch(Action::SelectChannel(second));
    settle(&mut dom, state);
    assert_eq!(*state.draft.peek(), "rascunho do segundo");
    actions.command(Command::SetTheme(Theme::Light));
    settle(&mut dom, state);
    assert_eq!(*state.theme.peek(), Theme::Light);
    actions.command(Command::RenameProfile("Felipe".into()));
    settle(&mut dom, state);
    assert_eq!(
        state.profile.peek().as_ref().unwrap().display_name,
        "Felipe"
    );
    let repository = SqliteRepository::open(&path).unwrap();
    assert_eq!(repository.preferences().unwrap().theme, Theme::Light);
    assert_eq!(repository.profile().unwrap().display_name, "Felipe");
    assert_eq!(repository.messages(channel).unwrap().len(), 1);
    drop(repository);
    actions.command(Command::DeleteChannel(second));
    settle(&mut dom, state);
    assert_eq!(state.navigation.peek().channels.len(), 1);
    assert_eq!(state.history.peek().messages.len(), 1);
    actions.command(Command::DeleteSpace(space));
    settle(&mut dom, state);
    assert!(state.navigation.peek().spaces.is_empty());
    assert!(state.navigation.peek().channels.is_empty());
    assert!(state.history.peek().messages.is_empty());
    drop(dom);
}

#[test]
fn typing_does_not_rebuild_message_history() {
    let dir = tempfile::tempdir().unwrap();
    let probe = Probe(Rc::new(RefCell::new(None)));
    let mut dom = VirtualDom::new_with_props(
        TestApp,
        TestAppProps {
            probe: probe.clone(),
            path: dir.path().join("db.sqlite3"),
        },
    );
    dom.rebuild_in_place();
    let (state, actions) = probe.0.borrow().unwrap();
    settle(&mut dom, state);
    actions.command(Command::CreateSpace("Local".into()));
    settle(&mut dom, state);
    let space = state.navigation.peek().selected_space.unwrap();
    actions.command(Command::CreateChannel(space, "texto".into()));
    settle(&mut dom, state);
    let channel = state.navigation.peek().selected_channel.unwrap();
    actions.command(Command::SendMessage(channel, "Histórico estável".into()));
    settle(&mut dom, state);
    let original = state.history.peek().messages.clone();
    for draft in ["a", "ab", "abc"] {
        actions.edit_draft(draft.into());
        let mutations = dom.render_immediate_to_vec();
        assert!(Rc::ptr_eq(&original, &state.history.peek().messages));
        assert!(!format!("{:?}", mutations.edits).contains("Histórico estável"));
    }
    // A pending draft wake must not reject a foreground send.
    actions.command(Command::SendMessage(channel, state.draft.peek().clone()));
    settle(&mut dom, state);
    assert!(state.status.peek().error.is_none());
    assert!(state.draft.peek().is_empty());
    assert_eq!(state.history.peek().messages.len(), 2);
}
