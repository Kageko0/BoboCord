use crate::state::{Actions, Modal, UiState};
use dioxus::prelude::*;

#[component]
pub fn ModalHost() -> Element {
    let state = use_context::<UiState>();
    rsx! { if let Some(modal) = state.modal.read().clone() { ModalDialog { modal } } }
}

#[component]
fn ModalDialog(modal: Modal) -> Element {
    let mut state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let mut value = use_signal(|| modal.value.clone());
    let status = state.status.read();
    let deleting = modal.kind.deleting();
    rsx! {
        dialog { id: "edit-dialog", class: "edit-dialog", "aria-labelledby": "dialog-title", "aria-describedby": "dialog-description",
            onmounted: move |_| { document::eval(r#"
                const d=document.getElementById('edit-dialog');
                if(d && !d.open) {
                    d.showModal();
                    d.querySelector('input,button')?.focus();
                    // Manage focus synchronously in the browser, before native Tab handling.
                    d.addEventListener('keydown', event => {
                        if(event.key !== 'Tab') return;
                        const fields=[...d.querySelectorAll('input:not(:disabled),button:not(:disabled)')];
                        const first=fields[0], last=fields.at(-1);
                        if(!first) { event.preventDefault(); return; }
                        if(event.shiftKey && document.activeElement===first) { event.preventDefault(); last.focus(); }
                        else if(!event.shiftKey && document.activeElement===last) { event.preventDefault(); first.focus(); }
                    });
                    // Rust handles Escape, including the in-flight operation guard.
                    d.addEventListener('cancel', event => event.preventDefault());
                }
            "#); },
            onkeydown: move |event| {
                if event.key() == Key::Escape { event.prevent_default(); if !state.status.peek().busy { state.modal.set(None); } }
            },
            form { onsubmit: move |event| { event.prevent_default(); actions.command(modal.kind.command(value.peek().clone())); },
                span { class: "eyebrow", if deleting { "CONFIRMAÇÃO" } else { "ORGANIZE SEU ESPAÇO" } }
                h2 { id: "dialog-title", "{modal.title}" }
                p { id: "dialog-description", if deleting { "Esta ação é permanente. Os canais e mensagens associados também serão removidos." } else { "Escolha um nome de 1 a 80 caracteres." } }
                if !deleting {
                    label { r#for: "item-name", "Nome" }
                    input { id: "item-name", value: "{value}", "aria-invalid": status.error.is_some(), "aria-describedby": "dialog-description dialog-error", disabled: status.busy, autocomplete: "off", oninput: move |event| value.set(event.value()) }
                }
                if let Some(error) = &status.error { p { id: "dialog-error", class: "form-error", role: "alert", "{error}" } }
                div { class: "dialog-actions",
                    button { class: "subtle-button", r#type: "button", disabled: status.busy, onclick: move |_| state.modal.set(None), "Cancelar" }
                    button { class: if deleting { "danger-button" } else { "primary-button" }, r#type: "submit", disabled: status.busy,
                        if status.busy { "Salvando…" } else if deleting { "Excluir definitivamente" } else { "Salvar" }
                    }
                }
            }
        }
    }
}
