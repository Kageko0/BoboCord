use crate::state::{Actions, ContextTarget, ModalKind, UiState, initials};
use bobo_app::Action;
use dioxus::prelude::*;

#[component]
pub fn ChannelSidebar() -> Element {
    let state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let nav = state.navigation.read();
    let status = state.status.read();
    let space = nav.selected_space;
    let name = nav.space_name().to_owned();
    rsx! {
        aside { class: "channel-sidebar", "aria-label": "Canais de texto",
            header { class: "sidebar-header",
                span { class: "eyebrow", "SEU ESPAÇO" }
                h1 { title: "{name}", "{name}" }
                if let Some(id) = space {
                    div { class: "inline-actions",
                        button { class: "text-button", disabled: status.busy,
                            onclick: { let name = name.clone(); move |_| state.open_modal(ModalKind::RenameSpace(id), "Renomear espaço", name.clone()) }, "Renomear" }
                        button { class: "text-button danger-text", disabled: status.busy,
                            onclick: { let name = name.clone(); move |_| state.open_modal(ModalKind::DeleteSpace(id), format!("Excluir {name}?"), "") }, "Excluir" }
                    }
                }
            }
            div { class: "section-heading",
                h2 { "Canais de texto" }
                button { class: "icon-button", title: "Criar canal", "aria-label": "Criar canal",
                    disabled: space.is_none() || status.busy,
                    onclick: move |_| { if let Some(id) = space { state.open_modal(ModalKind::CreateChannel(id), "Novo canal de texto", ""); } }, "+" }
            }
            nav { class: "channel-list", "aria-label": "Lista de canais",
                for channel in &nav.channels {
                    button { key: "{channel.id}", class: "channel-button", disabled: status.busy,
                        "aria-pressed": nav.selected_channel == Some(channel.id),
                        onclick: { let id = channel.id; move |_| actions.dispatch(Action::SelectChannel(id)) },
                        title: "{channel.name} · botão direito ou Shift+F10 para ações", "aria-haspopup": "menu",
                        oncontextmenu: { let id = channel.id; let name = channel.name.clone(); move |event| { event.prevent_default(); let p = event.client_coordinates(); state.open_context(ContextTarget { id, name: name.clone(), space: false, x: p.x as i32, y: p.y as i32 }); } },
                        onkeydown: { let id = channel.id; let name = channel.name.clone(); move |event| { if event.key() == Key::ContextMenu || (event.key() == Key::F10 && event.modifiers().contains(Modifiers::SHIFT)) { event.prevent_default(); state.open_context(ContextTarget { id, name: name.clone(), space: false, x: 180, y: 140 }); } } },
                        span { class: "channel-hash", "#" }
                        span { class: "truncate", "{channel.name}" }
                    }
                }
                if nav.channels.is_empty() {
                    p { class: "sidebar-empty", if space.is_some() { "Um canal, uma nova conversa. Use + para começar." } else { "Crie um espaço pelo botão + ao lado." } }
                }
            }
            div { class: "local-note", span { class: "status-dot" } "Somente neste dispositivo" }
            ProfileDock {}
        }
    }
}

#[component]
pub fn ProfileDock() -> Element {
    let mut state = use_context::<UiState>();
    let profile = state.profile.read();
    let name = profile
        .as_ref()
        .map_or("Perfil local", |p| p.display_name.as_str());
    rsx! {
        div { class: "profile-dock",
            div { class: "avatar", "aria-hidden": "true", "{initials(name)}" }
            div { class: "profile-copy", strong { class: "truncate", "{name}" } small { "Perfil local" } }
            button { class: "icon-button", title: "Configurações", "aria-label": "Configurações",
                "aria-pressed": *state.settings.read(),
                onclick: move |_| { let open = *state.settings.peek(); state.settings.set(!open); }, "⚙" }
        }
    }
}
