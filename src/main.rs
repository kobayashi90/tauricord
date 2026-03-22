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
    const invokeTauriCommand = async (cmd, payload) => {
        if (window.__TAURI__?.core?.invoke) {
            return window.__TAURI__.core.invoke(cmd, payload);
        }
        if (window.__TAURI_INTERNALS__?.invoke) {
            return window.__TAURI_INTERNALS__.invoke(cmd, payload);
        }
        throw new Error('Tauri invoke API unavailable');
    };

    const isDiscordUrl = (url) => {
        try {
            const u = new URL(url, location.origin);
            return u.hostname === location.hostname
                || u.hostname.endsWith('.discord.com')
                || u.hostname === 'discord.com';
        } catch {
            return false;
        }
    };

    const openExternalUrl = async (url) => {
        try {
            await invokeTauriCommand('open_external', { url });
            return true;
        } catch (error) {
            console.error('Failed to open external URL:', url, error);
            return false;
        }
    };

    const parseUnreadCount = (title) => {
        const match = /^\((\d+)\)\s/.exec(title || '');
        return match ? Number.parseInt(match[1], 10) : null;
    };

    let lastUnreadCount = undefined;
    const syncUnreadBadge = () => {
        const unreadCount = parseUnreadCount(document.title);
        if (unreadCount === lastUnreadCount) {
            return;
        }

        lastUnreadCount = unreadCount;
        void invokeTauriCommand('set_unread_badge', { count: unreadCount }).catch((error) => {
            console.error('Failed to sync unread badge:', unreadCount, error);
        });
    };

    // Debug: log what Discord will see
    console.log('=== INIT_SCRIPT DEBUG ===');
    console.log('navigator.userAgent:', navigator.userAgent);
    console.log('navigator.userAgentData:', navigator.userAgentData);
    console.log('window.chrome:', window.chrome);
    console.log('navigator.webdriver:', navigator.webdriver);
    console.log('navigator.platform:', navigator.platform);
    console.log('navigator.vendor:', navigator.vendor);
    
    // 0. Spoof browser identity so Discord enables voice/video/screenshare
    Object.defineProperty(navigator, 'userAgentData', {
        get: () => ({
            brands: [
                { brand: "Chromium", version: "131" },
                { brand: "Google Chrome", version: "131" },
                { brand: "Not_A Brand", version: "24" }
            ],
            mobile: false,
            platform: "Linux",
            getHighEntropyValues: async () => ({
                brands: [
                    { brand: "Chromium", version: "131" },
                    { brand: "Google Chrome", version: "131" },
                    { brand: "Not_A Brand", version: "24" }
                ],
                mobile: false,
                platform: "Linux",
                platformVersion: "6.1.0",
                architecture: "x86",
                bitness: "64",
                model: "",
                uaFullVersion: "131.0.0.0",
                fullVersionList: [
                    { brand: "Chromium", version: "131.0.0.0" },
                    { brand: "Google Chrome", version: "131.0.0.0" },
                    { brand: "Not_A Brand", version: "24.0.0.0" }
                ]
            })
        })
    });

    // Spoof window.chrome to make Discord think it's Chrome
    Object.defineProperty(window, 'chrome', {
        get: () => ({
            runtime: {},
            webstore: {}
        }),
        configurable: true
    });

    // Hide that we're using Tauri/WebKit
    Object.defineProperty(navigator, 'webdriver', {
        get: () => false
    });

    // Debug: log after spoofing
    setTimeout(() => {
        console.log('=== AFTER SPOOFING ===');
        console.log('navigator.userAgent:', navigator.userAgent);
        console.log('navigator.userAgentData:', navigator.userAgentData);
        console.log('window.chrome:', window.chrome);
        console.log('navigator.webdriver:', navigator.webdriver);
    }, 100);

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

    // 4. Redirect external links to the default browser.
    const handleExternalAnchorClick = (event) => {
        const anchor = event.target?.closest?.('a[href]');
        if (!anchor) {
            return;
        }

        const href = anchor.href;
        if (!href || isDiscordUrl(href)) {
            return;
        }

        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation?.();
        void openExternalUrl(href);
    };

    document.addEventListener('click', handleExternalAnchorClick, true);
    document.addEventListener('auxclick', handleExternalAnchorClick, true);

    const titleElement = document.querySelector('title');
    if (titleElement) {
        new MutationObserver(syncUnreadBadge).observe(titleElement, {
            childList: true,
            characterData: true,
            subtree: true,
        });
    }

    document.addEventListener('visibilitychange', syncUnreadBadge);
    window.addEventListener('focus', syncUnreadBadge);
    window.addEventListener('blur', syncUnreadBadge);
    setInterval(syncUnreadBadge, 2000);
    syncUnreadBadge();

    // 5. Redirect window.open() to the default browser.
    //    Runs before Discord's JS so we catch every call.
    const originalOpen = window.open;
    window.open = function(url, ...args) {
        if (url) {
            try {
                const u = new URL(url, location.origin);
                const isExternal = !isDiscordUrl(u.toString())
                    && (u.protocol === 'http:' || u.protocol === 'https:' || u.protocol === 'mailto:');
                if (isExternal) {
                    void openExternalUrl(u.toString());
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

#[cfg(target_os = "windows")]
fn windows_badge_label(count: i64) -> Vec<char> {
    if count > 9 {
        vec!['9', '+']
    } else {
        count.to_string().chars().collect()
    }
}

#[cfg(target_os = "windows")]
fn windows_badge_glyph(ch: char) -> Option<[&'static str; 5]> {
    match ch {
        '0' => Some(["111", "101", "101", "101", "111"]),
        '1' => Some(["010", "110", "010", "010", "111"]),
        '2' => Some(["111", "001", "111", "100", "111"]),
        '3' => Some(["111", "001", "111", "001", "111"]),
        '4' => Some(["101", "101", "111", "001", "001"]),
        '5' => Some(["111", "100", "111", "001", "111"]),
        '6' => Some(["111", "100", "111", "101", "111"]),
        '7' => Some(["111", "001", "001", "010", "010"]),
        '8' => Some(["111", "101", "111", "101", "111"]),
        '9' => Some(["111", "101", "111", "001", "111"]),
        '+' => Some(["000", "010", "111", "010", "000"]),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn windows_blend_pixel(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    color: [u8; 4],
) {
    if x < 0 || y < 0 {
        return;
    }

    let (x, y) = (x as usize, y as usize);
    if x >= width || y >= height {
        return;
    }

    let index = (y * width + x) * 4;
    let alpha = color[3] as f32 / 255.0;
    let inverse_alpha = 1.0 - alpha;

    rgba[index] = (color[0] as f32 * alpha + rgba[index] as f32 * inverse_alpha).round() as u8;
    rgba[index + 1] =
        (color[1] as f32 * alpha + rgba[index + 1] as f32 * inverse_alpha).round() as u8;
    rgba[index + 2] =
        (color[2] as f32 * alpha + rgba[index + 2] as f32 * inverse_alpha).round() as u8;
    rgba[index + 3] = ((color[3] as f32) + rgba[index + 3] as f32 * inverse_alpha)
        .round()
        .clamp(0.0, 255.0) as u8;
}

#[cfg(target_os = "windows")]
fn windows_fill_rounded_rect(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    rect_width: i32,
    rect_height: i32,
    radius: i32,
    color: [u8; 4],
) {
    let radius = radius.max(0).min(rect_width / 2).min(rect_height / 2);

    for py in y..(y + rect_height) {
        for px in x..(x + rect_width) {
            let inside = if radius == 0 {
                true
            } else if px < x + radius && py < y + radius {
                let dx = px - (x + radius);
                let dy = py - (y + radius);
                dx * dx + dy * dy <= radius * radius
            } else if px >= x + rect_width - radius && py < y + radius {
                let dx = px - (x + rect_width - radius - 1);
                let dy = py - (y + radius);
                dx * dx + dy * dy <= radius * radius
            } else if px < x + radius && py >= y + rect_height - radius {
                let dx = px - (x + radius);
                let dy = py - (y + rect_height - radius - 1);
                dx * dx + dy * dy <= radius * radius
            } else if px >= x + rect_width - radius && py >= y + rect_height - radius {
                let dx = px - (x + rect_width - radius - 1);
                let dy = py - (y + rect_height - radius - 1);
                dx * dx + dy * dy <= radius * radius
            } else {
                true
            };

            if inside {
                windows_blend_pixel(rgba, width, height, px, py, color);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_draw_glyph(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    ch: char,
    scale: i32,
    color: [u8; 4],
) {
    let Some(glyph) = windows_badge_glyph(ch) else {
        return;
    };

    for (row_index, row) in glyph.iter().enumerate() {
        for (column_index, bit) in row.chars().enumerate() {
            if bit != '1' {
                continue;
            }

            for sy in 0..scale {
                for sx in 0..scale {
                    windows_blend_pixel(
                        rgba,
                        width,
                        height,
                        x + (column_index as i32 * scale) + sx,
                        y + (row_index as i32 * scale) + sy,
                        color,
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_overlay_icon(count: i64) -> tauri::image::Image<'static> {
    const ICON_SIZE: usize = 32;
    let mut rgba = vec![0_u8; ICON_SIZE * ICON_SIZE * 4];

    let label = windows_badge_label(count);
    let scale = if label.len() >= 3 { 2 } else { 3 };
    let spacing = if scale == 2 { 1 } else { 2 };
    let glyph_width = 3 * scale;
    let glyph_height = 5 * scale;
    let label_width = (label.len() as i32 * glyph_width)
        + ((label.len().saturating_sub(1)) as i32 * spacing);
    let badge_height = glyph_height + 6;
    let badge_width = (label_width + 8).max(badge_height);
    let badge_x = ((ICON_SIZE as i32 - badge_width) / 2).max(0);
    let badge_y = ((ICON_SIZE as i32 - badge_height) / 2).max(0);

    windows_fill_rounded_rect(
        &mut rgba,
        ICON_SIZE,
        ICON_SIZE,
        badge_x,
        badge_y,
        badge_width,
        badge_height,
        badge_height / 2,
        [0xED, 0x42, 0x45, 0xFF],
    );

    let text_x = badge_x + ((badge_width - label_width) / 2);
    let text_y = badge_y + ((badge_height - glyph_height) / 2);

    for (index, ch) in label.iter().enumerate() {
        windows_draw_glyph(
            &mut rgba,
            ICON_SIZE,
            ICON_SIZE,
            text_x + index as i32 * (glyph_width + spacing),
            text_y,
            *ch,
            scale,
            [0xFF, 0xFF, 0xFF, 0xFF],
        );
    }

    tauri::image::Image::new_owned(rgba, ICON_SIZE as u32, ICON_SIZE as u32)
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
fn set_unread_badge(window: tauri::WebviewWindow, count: Option<i64>) -> Result<(), String> {
    let count = count.filter(|count| *count > 0);

    #[cfg(target_os = "windows")]
    {
        use tauri::UserAttentionType;

        let overlay = count.map(windows_overlay_icon);

        window
            .set_overlay_icon(overlay)
            .map_err(|e| e.to_string())?;

        let request = if count.is_some() && !window.is_focused().unwrap_or(false) {
            Some(UserAttentionType::Informational)
        } else {
            None
        };

        window
            .request_user_attention(request)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        window.set_badge_count(count).map_err(|e| e.to_string())?;
        Ok(())
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
        .invoke_handler(tauri::generate_handler![open_external, set_unread_badge])
        .setup(|app| {
            // ── Create the main window programmatically ─────────────
            // This lets us use initialization_script (runs before page
            // JS) and on_navigation (blocks external URLs in-webview).
            let url = WebviewUrl::External("https://discord.com/app".parse().unwrap());

            #[cfg(target_os = "windows")]
            let main_window = {
                let mut builder = WebviewWindowBuilder::new(app, "main", url)
                    .title("Tauricord")
                    .inner_size(800.0, 600.0)
                    .resizable(true)
                    .fullscreen(false)
                    .disable_drag_drop_handler();

                builder
                    .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                    .initialization_script(INIT_SCRIPT)
            };

            #[cfg(not(target_os = "windows"))]
            let main_window = WebviewWindowBuilder::new(app, "main", url)
                .title("Tauricord")
                .inner_size(800.0, 600.0)
                .resizable(true)
                .fullscreen(false)
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
                .initialization_script(INIT_SCRIPT);

            let main_window = main_window
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

            #[cfg(debug_assertions)]
            main_window.open_devtools();

            #[cfg(feature = "with-tray")]
            let _unused_release = &main_window;

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
