use crate::state::{Actions, UiState};
use bobo_core::{Command, Theme};
use dioxus::prelude::*;

#[component]
pub fn SettingsPanel() -> Element {
    let mut state = use_context::<UiState>();
    let actions = use_context::<Actions>();
    let mut name = use_signal(|| {
        state
            .profile
            .peek()
            .as_ref()
            .map(|p| p.display_name.clone())
            .unwrap_or_default()
    });
    let theme = *state.theme.read();
    let busy = state.status.read().busy;
    let path = use_hook(|| {
        bobo_app::database_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|e| e.to_string())
    });
    rsx! {
        section { class: "settings-panel",
            header { class: "settings-header", div { span { class: "eyebrow", "DO SEU JEITO" } h2 { "Configurações" } } button { class: "subtle-button", onclick: move |_| state.settings.set(false), "Voltar à conversa" } }
            div { class: "settings-card",
                h3 { "Aparência" } p { "Uma interface confortável, de dia ou de noite." }
                div { class: "theme-options", role: "group", "aria-label": "Tema",
                    button { class: "theme-option", "aria-pressed": theme == Theme::Dark, disabled: busy, onclick: move |_| actions.command(Command::SetTheme(Theme::Dark)), span { class: "theme-swatch dark-swatch" } "Escuro" }
                    button { class: "theme-option", "aria-pressed": theme == Theme::Light, disabled: busy, onclick: move |_| actions.command(Command::SetTheme(Theme::Light)), span { class: "theme-swatch light-swatch" } "Claro" }
                }
            }
            form { class: "settings-card", onsubmit: move |event| { event.prevent_default(); actions.command(Command::RenameProfile(name.peek().clone())); },
                h3 { "Seu perfil local" }
                label { r#for: "profile-name", "Nome de exibição" }
                div { class: "profile-form", input { id: "profile-name", value: "{name}", disabled: busy, oninput: move |event| name.set(event.value()), autocomplete: "off" } button { class: "primary-button", r#type: "submit", disabled: busy, "Salvar perfil" } }
                p { "Seu perfil identifica as mensagens salvas neste dispositivo. Avatar usa suas iniciais." }
            }
            div { class: "settings-card", h3 { "Seus dados ficam aqui" } p { class: "database-path", "{path}" } p { "SQLite local · banco não criptografado. Feche o app antes de copiar um backup." } }
            p { class: "scope-note", "BoboCord v0.1 Foundation. Texto local, espaços, canais e temas. Recursos de rede e multimídia ainda não fazem parte desta versão." }
        }
    }
}
