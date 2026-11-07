use bobo_app::Action;
use bobo_core::Theme;
use dioxus::prelude::*;

use super::{
    channels::ChannelSidebar,
    chat::{ChatHeader, MessageComposer, MessageList},
    context_menu::ContextMenuHost,
    modal::ModalHost,
    settings::SettingsPanel,
    spaces::SpaceRail,
};
use crate::state::{Actions, UiState, use_ui_state, use_worker};

const CSS: &str = concat!(
    include_str!("../../assets/tokens.css"),
    include_str!("../../assets/shell.css"),
    include_str!("../../assets/chat.css"),
    include_str!("../../assets/forms.css")
);

#[component]
pub fn AppShell() -> Element {
    let state = use_ui_state();
    let path = use_hook(|| bobo_app::database_path().map_err(|e| e.to_string()));
    let actions = use_worker(state, path);
    use_context_provider(|| state);
    use_context_provider(|| actions);
    rsx! { AppView {} }
}

#[component]
pub fn AppView() -> Element {
    let state = use_context::<UiState>();
    rsx! {
        style { "{CSS}" }
        div { class: "app-shell", "data-theme": if *state.theme.read() == Theme::Light { "light" } else { "dark" },
            SpaceRail {}
            ChannelSidebar {}
            MainPane {}
            ContextMenuHost {}
            ModalHost {}
        }
    }
}

#[component]
fn MainPane() -> Element {
    let state = use_context::<UiState>();
    rsx! {
        main { class: "main-pane", id: "main-content",
            if *state.settings.read() { SettingsPanel {} }
            else { ChatHeader {} MessageList {} MessageComposer {} }
            Feedback {}
        }
    }
}

#[component]
fn Feedback() -> Element {
    let state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let status = state.status.read();
    let draft_error = state.draft_error.read();
    let error = status.error.as_ref().or(draft_error.as_ref());
    rsx! {
        footer { class: "feedback", "data-error": error.is_some(),
            span { role: if error.is_some() { "alert" } else { "status" }, "aria-live": "polite",
                if let Some(error) = error { "{error}" }
                else if status.busy { "Salvando / carregando…" }
                else { "{status.feedback}" }
            }
            button { class: "text-button", disabled: status.busy, onclick: move |_| actions.dispatch(Action::Reload), "Recarregar" }
        }
    }
}
