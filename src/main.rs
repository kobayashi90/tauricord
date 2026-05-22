#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;
mod tray;
mod about;
mod settings;
mod settings_window;
mod theme;

#[cfg(target_os = "linux")]
mod webrtc_linux;

use std::sync::Mutex;
use tauri::{
    WebviewUrl, WebviewWindowBuilder, Manager,
    webview::NewWindowResponse,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const INIT_SCRIPT: &str = include_str!("../assets/inject.js");
const DISCORD_URL: &str = "https://discord.com/app";

struct SettingsState(Mutex<settings::Settings>);

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

#[tauri::command]
fn get_settings(state: tauri::State<SettingsState>) -> Result<settings::Settings, String> {
    Ok(state.0.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<SettingsState>,
    new_settings: settings::Settings,
) -> Result<(), String> {
    *state.0.lock().map_err(|e| e.to_string())? = new_settings.clone();
    new_settings.save(&app)
}

#[tauri::command]
fn get_webrtc_help() -> Option<&'static str> {
    #[cfg(target_os = "linux")]
    {
        return webrtc_linux::webrtc_help_text();
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[tauri::command]
fn set_presence(app: tauri::AppHandle, game: Option<String>, in_voice: bool) -> Result<(), String> {
    let tooltip = match (game.as_deref(), in_voice) {
        (Some(g), true) => format!("Tauricord — Playing {} • In Voice", g),
        (Some(g), false) => format!("Tauricord — Playing {}", g),
        (None, true) => "Tauricord — In Voice".to_string(),
        (None, false) => "Tauricord".to_string(),
    };
    app.tray_by_id("main")
        .ok_or("no tray")?
        .set_tooltip(Some(&tooltip))
        .map_err(|e| e.to_string())
}

fn build_main_window(app: &tauri::AppHandle, settings: &settings::Settings) -> tauri::WebviewWindow {
    let url = WebviewUrl::External(
        DISCORD_URL.parse().expect("hardcoded Discord URL should be valid"),
    );

    let geom = &settings.window_geometry;
    let (w, h) = if geom.width < 200.0 { (800.0, 600.0) } else { (geom.width, geom.height) };

    let win = WebviewWindowBuilder::new(app, "main", url)
        .title("Tauricord")
        .inner_size(w, h);

    #[cfg(target_os = "windows")]
    let win = win.disable_drag_drop_handler();

    let win = win
        .resizable(true)
        .fullscreen(false)
        .user_agent(platform::user_agent())
        .initialization_script(INIT_SCRIPT)
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

    let _ = win.set_position(tauri::PhysicalPosition::new(
        if geom.x < 0.0 { 100.0 } else { geom.x } as i32,
        if geom.y < 0.0 { 100.0 } else { geom.y } as i32,
    ));

    win
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            open_external,
            set_unread_badge,
            get_settings,
            save_settings,
            set_presence,
            get_webrtc_help,
        ])
        .setup(|app| {
            let settings = settings::Settings::load(app.handle());
            app.manage(SettingsState(Mutex::new(settings.clone())));

            if settings.launch_on_boot {
                let handle = app.handle();
                let _ = handle.autolaunch().enable();
            }

            let main_window = build_main_window(app.handle(), &settings);

            main_window.on_window_event({
                let app_handle = app.handle().clone();
                let window = main_window.clone();
                move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if let Some(state) = app_handle.try_state::<SettingsState>() {
                            let do_hide = state.0.lock().map(|s| s.hide_on_close).unwrap_or(true);
                            if do_hide {
                                api.prevent_close();
                                let _ = window.hide();
                            }
                            if let Ok(pos) = window.outer_position() {
                                if let Ok(size) = window.outer_size() {
                                    if let Ok(mut s) = state.0.lock() {
                                        s.window_geometry = settings::WindowGeometry {
                                            x: pos.x as f64,
                                            y: pos.y as f64,
                                            width: size.width as f64,
                                            height: size.height as f64,
                                        };
                                        let _ = s.save(&app_handle);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            if let Some(css_path) = &settings.custom_css_path {
                theme::start_css_watcher(app.handle().clone(), std::path::PathBuf::from(css_path));
            }

            {
                let gs = app.handle().global_shortcut();
                let _ = gs.on_shortcut("Ctrl+Shift+M", {
                    let window = main_window.clone();
                    move |_app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            let _ = window.eval(
                                r#"document.dispatchEvent(new KeyboardEvent('keydown', {
                                    key: 'M', code: 'KeyM', ctrlKey: true, shiftKey: true, bubbles: true
                                }));"#,
                            );
                        }
                    }
                });
                let _ = gs.on_shortcut("Ctrl+Shift+D", {
                    let window = main_window.clone();
                    move |_app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            let _ = window.eval(
                                r#"document.dispatchEvent(new KeyboardEvent('keydown', {
                                    key: 'D', code: 'KeyD', ctrlKey: true, shiftKey: true, bubbles: true
                                }));"#,
                            );
                        }
                    }
                });
            }

            #[cfg(target_os = "windows")]
            if let Err(error) = platform::install_taskbar_hook(&main_window) {
                log::error!("Failed to install Windows taskbar hook: {error}");
            }

            #[cfg(target_os = "windows")]
            platform::set_window_icon(&main_window);

            #[cfg(target_os = "linux")]
            if let Err(e) = webrtc_linux::setup_webrtc(&main_window) {
                log::error!("[WebRTC] Setup failed: {e}");
            }

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
