use crate::state::{ContextTarget, ModalKind, UiState};
use dioxus::prelude::*;

#[component]
pub fn ContextMenuHost() -> Element {
    let state = use_context::<UiState>();
    rsx! { if let Some(target) = state.context.read().clone() { ContextMenu { target } } }
}

#[component]
fn ContextMenu(target: ContextTarget) -> Element {
    let mut state = use_context::<UiState>();
    let rename = if target.space {
        ModalKind::RenameSpace(target.id)
    } else {
        ModalKind::RenameChannel(target.id)
    };
    let delete = if target.space {
        ModalKind::DeleteSpace(target.id)
    } else {
        ModalKind::DeleteChannel(target.id)
    };
    use_drop(|| {
        document::eval("window.boboMenuReturnFocus?.(); window.boboMenuReturnFocus=undefined;");
    });
    rsx! {
        button { class: "menu-dismiss", "aria-label": "Fechar menu contextual", tabindex: "-1", onclick: move |_| state.context.set(None) }
        div { class: "context-menu", id: "context-menu", role: "menu", "aria-label": "Ações de {target.name}",
            style: "left:{target.x}px;top:{target.y}px;",
            onmounted: move |_| { document::eval(r#"
                const m=document.getElementById('context-menu');
                if(m) {
                    const previous=document.activeElement;
                    window.boboMenuReturnFocus=()=>previous?.focus();
                    const r=m.getBoundingClientRect();
                    m.style.left=Math.max(8,Math.min(r.left,innerWidth-r.width-8))+'px';
                    m.style.top=Math.max(8,Math.min(r.top,innerHeight-r.height-8))+'px';
                    m.querySelector('button')?.focus();
                    m.addEventListener('keydown', e=>{
                        const items=[...m.querySelectorAll('button')];
                        if(e.key==='ArrowDown'||e.key==='ArrowUp') {
                            e.preventDefault();
                            const i=items.indexOf(document.activeElement);
                            items[(i+(e.key==='ArrowDown'?1:-1)+items.length)%items.length]?.focus();
                        }
                    });
                }
            "#); },
            onkeydown: move |event| { if matches!(event.key(), Key::Escape | Key::Tab) { state.context.set(None); } },
            button { role: "menuitem", onclick: { let name = target.name.clone(); move |_| state.open_modal(rename, "Renomear", name.clone()) }, "Renomear" }
            button { role: "menuitem", class: "danger-text", onclick: move |_| state.open_modal(delete, format!("Excluir {}?", target.name), ""), "Excluir…" }
        }
    }
}
