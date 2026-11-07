use dioxus::prelude::*;
mod components;
mod state;
#[cfg(test)]
mod tests;

fn main() {
    let config = dioxus_desktop::Config::new()
        .with_window(
            dioxus_desktop::WindowBuilder::new()
                .with_title("BoboCord · Foundation")
                .with_inner_size(dioxus_desktop::LogicalSize::new(1160.0, 760.0))
                .with_min_inner_size(dioxus_desktop::LogicalSize::new(740.0, 480.0)),
        )
        .with_menu(None)
        .with_navigation_handler(|_| false);
    let config = match bobo_app::database_path()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("webview")))
    {
        Some(path) => config.with_data_directory(path),
        None => config,
    };
    dioxus_desktop::launch::launch_virtual_dom(VirtualDom::new(components::AppShell), config);
}
