use tauri::{
    AppHandle, WebviewWindow, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::about;

pub fn setup_tray(app: &AppHandle, main_window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_item = MenuItem::with_id(app, "toggle", "Hide Tauricord", true, None::<&str>)?;
    let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_item, &about_item, &separator, &quit_item])?;

    {
        let window = main_window.clone();
        let toggle_item = toggle_item.clone();
        main_window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let _ = toggle_item.set_text("Show Tauricord");
            }
        });
    }

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

    let _tray = TrayIconBuilder::new()
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
