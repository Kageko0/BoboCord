use crate::state::{Actions, ContextTarget, ModalKind, UiState, initials};
use bobo_app::Action;
use dioxus::prelude::*;

#[component]
pub fn SpaceRail() -> Element {
    let state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let navigation = state.navigation.read();
    let status = state.status.read();
    rsx! {
        nav { class: "space-rail", "aria-label": "Espaços locais",
            div { class: "brand-mark", title: "BoboCord · Foundation", "b" }
            div { class: "rail-divider" }
            div { class: "space-list",
                for space in &navigation.spaces {
                    button {
                        key: "{space.id}", class: "space-button",
                        title: "{space.name}", "aria-label": "Abrir espaço {space.name}",
                        "aria-pressed": navigation.selected_space == Some(space.id),
                        disabled: status.busy,
                        onclick: { let id = space.id; move |_| actions.dispatch(Action::SelectSpace(id)) },
                        "aria-haspopup": "menu",
                        oncontextmenu: { let id = space.id; let name = space.name.clone(); move |event| { event.prevent_default(); let p = event.client_coordinates(); state.open_context(ContextTarget { id, name: name.clone(), space: true, x: p.x as i32, y: p.y as i32 }); } },
                        onkeydown: { let id = space.id; let name = space.name.clone(); move |event| { if event.key() == Key::ContextMenu || (event.key() == Key::F10 && event.modifiers().contains(Modifiers::SHIFT)) { event.prevent_default(); state.open_context(ContextTarget { id, name: name.clone(), space: true, x: 60, y: 100 }); } } },
                        "{initials(&space.name)}"
                    }
                }
            }
            button { class: "space-button add-space", title: "Criar espaço", "aria-label": "Criar espaço",
                disabled: !status.ready || status.busy,
                onclick: move |_| state.open_modal(ModalKind::CreateSpace, "Novo espaço", ""), "+"
            }
            span { class: "rail-caption", "LOCAL" }
        }
    }
}
