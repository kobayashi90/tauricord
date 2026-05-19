#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod tray;
mod about;

use tauri::{
    WebviewUrl, WebviewWindowBuilder, Manager,
    webview::NewWindowResponse,
};

const INIT_SCRIPT: &str = include_str!("../assets/inject.js");
const DISCORD_URL: &str = "https://discord.com/app";

fn is_discord_url(url: &tauri::Url) -> bool {
    match url.host_str() {
        Some(host) => {
            host == "discord.com"
                || host.ends_with(".discord.com")
                || host == "hcaptcha.com"
                || host.ends_with(".hcaptcha.com")
                || host == "challenges.cloudflare.com"
                || host == "localhost"
        }
        None => true,
    }
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    let parsed = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
    match parsed.scheme() {
        "http" | "https" | "mailto" => {}
        scheme => return Err(format!("unsupported external URL scheme: {scheme}")),
    }
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_unread_badge(app: tauri::AppHandle, count: Option<i64>) -> Result<(), String> {
    platform::set_unread_badge(&app, count.filter(|c| *c != 0))
}

fn main() {
    #[cfg(target_os = "windows")]
    platform::set_windows_app_id();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![open_external, set_unread_badge])
        .setup(|app| {
            let url = WebviewUrl::External(
                DISCORD_URL.parse().expect("hardcoded Discord URL should be valid"),
            );

            let main_window = WebviewWindowBuilder::new(app, "main", url)
                .title("Tauricord")
                .inner_size(800.0, 600.0)
                .resizable(true)
                .fullscreen(false)
                .user_agent(platform::user_agent())
                .initialization_script(INIT_SCRIPT);

            #[cfg(target_os = "windows")]
            let main_window = main_window.disable_drag_drop_handler();

            let main_window = main_window
                .on_navigation(|url| {
                    if is_discord_url(url) {
                        true
                    } else {
                        let _ = open::that(url.as_str());
                        false
                    }
                })
                .on_new_window(|url, _features| {
                    if is_discord_url(&url) {
                        if let Some(host) = url.host_str() {
                            if host == "hcaptcha.com"
                                || host.ends_with(".hcaptcha.com")
                                || host == "challenges.cloudflare.com"
                            {
                                return NewWindowResponse::Allow;
                            }
                        }
                        NewWindowResponse::Deny
                    } else {
                        let _ = open::that(url.as_str());
                        NewWindowResponse::Deny
                    }
                })
                .build()
                .expect("failed to build main window");

            #[cfg(target_os = "windows")]
            if let Err(error) = platform::install_taskbar_hook(&main_window) {
                log::error!("Failed to install Windows taskbar hook: {error}");
            }

            #[cfg(target_os = "windows")]
            platform::set_window_icon(&main_window);

            #[cfg(debug_assertions)]
            main_window.open_devtools();

            #[cfg(feature = "with-tray")]
            if let Err(error) = tray::setup_tray(app.handle(), &main_window) {
                log::error!("Failed to setup tray: {error}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
