use crate::state::{Actions, ModalKind, UiState};
use bobo_app::{Action, ScrollIntent};
use bobo_core::Command;
use chrono::{DateTime, Utc};
use dioxus::prelude::*;

#[component]
pub fn ChatHeader() -> Element {
    let state = use_context::<UiState>();
    let nav = state.navigation.read();
    let busy = state.status.read().busy;
    let name = nav.channel_name().to_owned();
    rsx! {
        header { class: "chat-header",
            div { class: "chat-heading", span { class: "channel-hash", "#" } div { h2 { "{name}" } p { "Um lugar para suas mensagens locais" } } }
            if let Some(id) = nav.selected_channel {
                div { class: "inline-actions",
                    button { class: "subtle-button", disabled: busy,
                        onclick: { let name = name.clone(); move |_| state.open_modal(ModalKind::RenameChannel(id), "Renomear canal", name.clone()) }, "Renomear" }
                    button { class: "subtle-button danger-text", disabled: busy,
                        onclick: move |_| state.open_modal(ModalKind::DeleteChannel(id), format!("Excluir #{name}?"), ""), "Excluir" }
                }
            }
        }
    }
}

#[component]
pub fn MessageList() -> Element {
    let state = use_context::<UiState>();
    // This component subscribes to history/profile only. Typing does not render it.
    let history = state.history.read();
    let profile = state.profile.read();
    let author = profile.as_ref().map_or("Você", |p| p.display_name.as_str());
    use_drop(|| {
        document::eval("window.boboHistory?.dispose();");
    });
    let channel = history.channel.map(|id| id.to_string()).unwrap_or_default();
    let intent = match history.scroll {
        ScrollIntent::Reset => "reset",
        ScrollIntent::Prepend => "prepend",
        ScrollIntent::Append => "append",
        ScrollIntent::Preserve => "preserve",
    };
    rsx! {
        section { class: "message-list", id: "message-history", role: "region", "aria-label": "Histórico de mensagens locais", tabindex: "0",
            "data-channel": channel, "data-intent": intent,
            onmounted: move |_| { document::eval(include_str!("../../assets/history.js")); },
            HistoryControls {}
            if history.messages.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-symbol", "#" }
                    h3 { if history.channel.is_some() { "A conversa começa com você" } else { "Seu próximo espaço começa aqui" } }
                    p { if history.channel.is_some() { "Escreva a primeira mensagem. Ela fica salva neste dispositivo." } else { "Crie um espaço e um canal de texto para organizar suas conversas locais." } }
                    span { class: "local-badge", "v0.1 Foundation · sem conexão externa" }
                }
            }
            for message in history.messages.iter() {
                article { key: "{message.id}", class: "message-row", "data-message-id": "{message.id}",
                    div { class: "message-avatar", "aria-hidden": "true", "{crate::state::initials(author)}" }
                    div { class: "message-content",
                        div { class: "message-meta", strong { "{author}" } time { "{timestamp(message.created_at)}" } }
                        p { class: "message-body", "{message.body}" }
                    }
                }
            }
            div { id: "history-end", "aria-hidden": "true" }
        }
    }
}

#[component]
fn HistoryControls() -> Element {
    let state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let history = state.history.read();
    let busy = state.status.read().busy;
    rsx! { div { class: "history-controls",
        if history.has_more { button { class: "subtle-button load-earlier", "data-history-action": "older", disabled: busy,
            onclick: move |_| actions.dispatch(Action::LoadEarlier), "Mostrar anteriores" } }
        if history.has_newer { button { class: "subtle-button", "data-history-action": "latest", disabled: busy,
            onclick: move |_| actions.dispatch(Action::Latest), "Voltar às recentes" } }
    } }
}

pub fn timestamp(milliseconds: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(milliseconds)
        .map(|time| time.format("%d/%m/%Y · %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Horário inválido".into())
}

#[component]
pub fn MessageComposer() -> Element {
    let state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let channel = state.navigation.read().selected_channel;
    let busy = state.status.read().busy;
    let send = move || {
        if let Some(id) = channel {
            actions.command(Command::SendMessage(id, state.draft.peek().clone()));
        }
    };
    rsx! {
        form { class: "composer", onsubmit: move |event| { event.prevent_default(); send(); },
            div { class: "composer-input",
                textarea { id: "message-input", "aria-label": "Mensagem local", "aria-describedby": "composer-help", placeholder: "Escreva uma mensagem…",
                    value: "{state.draft}", disabled: busy || channel.is_none(), rows: "2",
                    oninput: move |event| actions.edit_draft(event.value()),
                    onkeydown: move |event| {
                        if event.key() == Key::Enter && !event.modifiers().contains(Modifiers::SHIFT) && !event.is_composing() {
                            event.prevent_default(); send();
                        }
                    }
                }
                button { class: "primary-button send-button", r#type: "submit", disabled: busy || channel.is_none(), if busy { "Salvando…" } else { "Enviar ↑" } }
            }
            div { class: "composer-hints", id: "composer-help", span { "Enter envia · Shift+Enter nova linha · salvamento automático local" } span { "{state.draft.read().chars().count()} / 4.000" } }
        }
    }
}
