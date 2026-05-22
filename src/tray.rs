use tauri::{
    AppHandle, WebviewWindow,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::about;

pub fn setup_tray(app: &AppHandle, main_window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_item = MenuItem::with_id(app, "toggle", "Hide Tauricord", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let update_item = MenuItem::with_id(app, "check_update", "Check for Updates", true, None::<&str>)?;
    let devtools_item = MenuItem::with_id(app, "devtools", "Toggle DevTools", true, None::<&str>)?;
    let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_item, &settings_item, &update_item, &devtools_item, &about_item, &separator, &quit_item])?;

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    {
        let window = main_window.clone();
        let toggle_item = toggle_item.clone();
        app.on_menu_event(move |app_handle, event| {
            match event.id.as_ref() {
                "toggle" => {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                        let _ = toggle_item.set_text("Show Tauricord");
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = toggle_item.set_text("Hide Tauricord");
                    }
                }
                "settings" => {
                    crate::settings_window::show_settings_window(app_handle);
                }
                "check_update" => {
                    let handle = app_handle.clone();
                    std::thread::spawn(move || {
                        let rt = tauri::async_runtime::handle();
                        rt.block_on(async {
                            use tauri_plugin_updater::UpdaterExt;
                            match handle.updater() {
                                Ok(updater) => match updater.check().await {
                                    Ok(Some(update)) => {
                                        let msg = format!("Update v{} available", update.version);
                                        let _ = handle.tray_by_id("main")
                                            .and_then(|t| t.set_tooltip(Some(&msg)).ok());
                                    }
                                    Ok(None) => {}
                                    Err(e) => log::error!("Update check failed: {e}"),
                                },
                                Err(e) => log::error!("Updater error: {e}"),
                            }
                        });
                    });
                }
                "devtools" => {
                    let _ = window.as_ref().open_devtools();
                }
                "about" => {
                    about::show_about_window(app_handle);
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            }
        });
    }

    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Tauricord")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event({
            let window = main_window.clone();
            let toggle_item = toggle_item.clone();
            move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                        let _ = toggle_item.set_text("Show Tauricord");
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = toggle_item.set_text("Hide Tauricord");
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
