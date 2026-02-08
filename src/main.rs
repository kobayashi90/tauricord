// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    WebviewUrl, WebviewWindowBuilder,
    webview::NewWindowResponse,
};

#[cfg(feature = "with-tray")]
use tauri::{
    AppHandle, Manager,
    WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    image::Image,
};

#[cfg(feature = "with-tray")]
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

#[cfg(feature = "with-tray")]
use base64::Engine;

/// JS that runs BEFORE any page script (via initialization_script).
/// This guarantees our window.open override is in place before Discord
/// or WebView2 can handle it at the native level.
const INIT_SCRIPT: &str = r#"
(function() {
    // 1. Hide Discord's in-app screen-share notification bar via CSS
    const style = document.createElement('style');
    style.textContent = `
        div[class^='base'] div[class^='bar_'] {
            display: none !important;
        }
    `;
    const inject = () => {
        if (document.head) {
            document.head.appendChild(style);
        } else {
            document.addEventListener('DOMContentLoaded', () => {
                document.head.appendChild(style);
            });
        }
    };
    inject();

    // 2. Handle permission requests
    if (navigator.permissions) {
        const originalQuery = navigator.permissions.query;
        navigator.permissions.query = async (params) => {
            if (params.name === 'microphone' || params.name === 'camera') {
                return { state: 'granted' };
            }
            return originalQuery.call(navigator.permissions, params);
        };
    }

    // 3. Auto-grant getUserMedia
    if (navigator.mediaDevices) {
        const originalGetUserMedia = navigator.mediaDevices.getUserMedia;
        navigator.mediaDevices.getUserMedia = async function(constraints) {
            try {
                return await originalGetUserMedia.call(this, constraints);
            } catch (err) {
                console.log('Media request:', constraints, err);
                throw err;
            }
        };
    }

    // 4. Redirect window.open() to the default browser.
    //    Runs before Discord's JS so we catch every call.
    const originalOpen = window.open;
    window.open = function(url, ...args) {
        if (url) {
            try {
                const u = new URL(url, location.origin);
                const isDiscord = u.hostname === location.hostname
                    || u.hostname.endsWith('.discord.com')
                    || u.hostname === 'discord.com';
                const isExternal = !isDiscord
                    && (u.protocol === 'http:' || u.protocol === 'https:');
                if (isExternal) {
                    window.__TAURI__.shell.open(url);
                    return null;
                }
            } catch {}
        }
        return originalOpen.call(this, url, ...args);
    };

})();
"#;

/// Returns true if a URL should be allowed to load inside the webview.
fn is_discord_url(url: &tauri::Url) -> bool {
    match url.host_str() {
        Some(host) => {
            host == "discord.com"
                || host.ends_with(".discord.com")
                || host == "localhost"
        }
        // about:blank, data:, blob:, etc.
        None => true,
    }
}

#[cfg(feature = "with-tray")]
fn show_about_window(app: &AppHandle) {
    eprintln!("show_about_window called");
    
    // If an about window already exists, just focus it
    if let Some(win) = app.get_webview_window("about") {
        eprintln!("about window already exists, focusing");
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");

    eprintln!("Creating about window with version={}, repo={}", version, repo);

    // Embed icon as base64 for data: URI compatibility
    let icon_bytes = include_bytes!("../icons/icon.png");
    let icon_base64 = base64::engine::general_purpose::STANDARD.encode(icon_bytes);
    let icon_data_url = format!("data:image/png;base64,{}", icon_base64);

    // Create proper HTML with newlines for better rendering
    let about_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>About Tauricord</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        html, body {{
            width: 100%;
            height: 100%;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1e1f22;
            color: #dbdee1;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 40px 24px;
            user-select: none;
            -webkit-user-select: none;
        }}
        h1 {{
            font-size: 26px;
            font-weight: 700;
            color: #f2f3f5;
            margin-bottom: 8px;
            letter-spacing: -0.5px;
        }}
        .version {{
            font-size: 12px;
            color: #949ba4;
            margin-bottom: 20px;
            letter-spacing: 0.5px;
        }}
        .desc {{
            font-size: 13px;
            color: #b5bac1;
            text-align: center;
            line-height: 1.6;
            margin-bottom: 28px;
            max-width: 380px;
        }}
        .links {{
            display: flex;
            flex-direction: column;
            gap: 10px;
            width: 100%;
            max-width: 260px;
            margin-bottom: 32px;
        }}
        a {{
            display: block;
            padding: 11px 16px;
            background: #2b2d31;
            color: #00a8fc;
            text-decoration: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 500;
            text-align: center;
            cursor: pointer;
            transition: background 0.15s;
        }}
        a:hover {{
            background: #383a40;
        }}
        .footer {{
            font-size: 11px;
            color: #6d6f78;
            line-height: 1.6;
        }}
        .icon {{
            width: 80px;
            height: 80px;
            margin-bottom: 16px;
            border-radius: 16px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        }}
    </style>
    <script>
        document.addEventListener('contextmenu', (e) => e.preventDefault());
    </script>
</head>
<body>
    <img src="{icon_data_url}" class="icon" alt="Tauricord">
    <h1>Tauricord</h1>
    <div class="version">v{version}</div>
    <div class="desc">
        A lightweight Discord desktop client built with Tauri &amp; Rust<br>
        Native performance without the Electron overhead
    </div>
    <div class="links">
        <a href="{repo}">GitHub Repository</a>
        <a href="{repo}/issues">Report an Issue</a>
        <a href="{repo}/releases">Releases</a>
    </div>
    <div class="footer">Built with Tauri &amp; Rust</div>
</body>
</html>"#
    );

    let data_url = format!("data:text/html;charset=utf-8,{}", urlencoding::encode(&about_html));
    let about_url = tauri::Url::parse(&data_url).unwrap();
    eprintln!("Parsed about URL, building window...");

    match WebviewWindowBuilder::new(
        app,
        "about",
        WebviewUrl::External(about_url),
    )
    .title("About Tauricord")
    .inner_size(480.0, 550.0)
    .resizable(false)
    .minimizable(false)
    .maximizable(false)
    .always_on_top(true)
    .center()
    .on_navigation(|url| {
        let s = url.as_str();
        if s.starts_with("data:") {
            true
        } else {
            let _ = open::that(s);
            false
        }
    })
    .build()
    {
        Ok(win) => {
            eprintln!("About window built successfully, showing and focusing");
            let _ = win.show();
            let _ = win.set_focus();
        }
        Err(e) => {
            eprintln!("Failed to create about window: {e}");
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // ── Create the main window programmatically ─────────────
            // This lets us use initialization_script (runs before page
            // JS) and on_navigation (blocks external URLs in-webview).
            let url = WebviewUrl::External("https://discord.com/app".parse().unwrap());

            let _main_window = WebviewWindowBuilder::new(app, "main", url)
                .title("Tauricord")
                .inner_size(800.0, 600.0)
                .resizable(true)
                .fullscreen(false)
                .disable_drag_drop_handler()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                .initialization_script(INIT_SCRIPT)
                .on_navigation(|url| {
                    // Only allow discord.com URLs to load inside the webview.
                    // External URLs are opened in the default browser.
                    if is_discord_url(url) {
                        true
                    } else {
                        let _ = open::that(url.as_str());
                        false
                    }
                })
                .on_new_window(|url, _features| {
                    // Intercept ALL new-window requests (target="_blank",
                    // window.open, image lightbox "Open in Browser", etc.)
                    // at the native WebView2 level.
                    if is_discord_url(&url) {
                        // Let Discord open popouts etc. by denying
                        // (they'll fall through to window.open override)
                        NewWindowResponse::Deny
                    } else {
                        let _ = open::that(url.as_str());
                        NewWindowResponse::Deny
                    }
                })
                .build()?;

            // ── Tray Icon ──────────────────────────────────────────────
            #[cfg(feature = "with-tray")]
            {
                let toggle_item = MenuItem::with_id(app, "toggle", "Hide Tauricord", true, None::<&str>)?;
                let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
                let separator = PredefinedMenuItem::separator(app)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&toggle_item, &about_item, &separator, &quit])?;

                // ── Close-to-tray ───────────────────────────────────────────
                // Like Vesktop: clicking X hides to tray instead of quitting.
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

                // Register menu events at the app level (more reliable than on TrayIconBuilder)
                {
                    let window = main_window.clone();
                    let toggle_item = toggle_item.clone();
                    app.on_menu_event(move |app_handle, event| {
                        eprintln!("Menu event: {:?}", event.id);
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
                                eprintln!("About menu clicked!");
                                show_about_window(app_handle);
                            }
                            "quit" => {
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    });
                }

                let _tray = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("Tauricord")
                    .menu(&menu)
                    .show_menu_on_left_click(false) // Right-click → menu, Left-click → toggle
                    .on_tray_icon_event({
                        let window = main_window.clone();
                        let toggle_item = toggle_item.clone();
                        move |_tray, event| {
                            // Only handle left-click-up to toggle visibility
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
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
