// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    WebviewUrl, WebviewWindowBuilder,
    webview::NewWindowResponse,
};

#[cfg(target_os = "windows")]
use std::{collections::HashMap, sync::{Mutex, OnceLock}};

#[cfg(target_os = "windows")]
const WINDOWS_APP_ID: &str = "io.tauricord.dev";

#[cfg(target_os = "windows")]
const WINDOWS_TASKBAR_SUBCLASS_ID: usize = 1;

#[cfg(target_os = "windows")]
static WINDOWS_TASKBAR_READY: OnceLock<Mutex<HashMap<isize, bool>>> = OnceLock::new();

#[cfg(target_os = "windows")]
static WINDOWS_TASKBAR_BUTTON_CREATED_MSG: OnceLock<u32> = OnceLock::new();

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
    // Force Tauri IPC to always use the postMessage transport instead of the
    // http://ipc.localhost fetch path, which Discord's CSP would block and log
    // as a console error. Rejecting ipc.localhost fetches immediately makes
    // Tauri fall back to postMessage silently on the very first call.
    const _origFetch = window.fetch;
    window.fetch = function(url, ...args) {
        if (typeof url === 'string' && url.startsWith('http://ipc.localhost/')) {
            return Promise.reject(new TypeError('IPC: use postMessage'));
        }
        return _origFetch.call(this, url, ...args);
    };

    const invokeTauriCommand = (cmd, payload) => {
        const invoke = window.__TAURI__?.core?.invoke ?? window.__TAURI_INTERNALS__?.invoke;
        if (!invoke) {
            return Promise.resolve();
        }
        return invoke(cmd, payload).catch((err) => {
            console.debug(`[IPC] ${cmd} failed:`, err?.message ?? err);
        });
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

    const openExternalUrl = (url) => {
        try {
            const absoluteUrl = new URL(url, location.origin).toString();
            window.location.assign(absoluteUrl);
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

    const discordStores = {
        guildRead: null,
        relationship: null,
        notificationSettings: null,
    };
    let storeSearchFailed = false;

    const subscribedDiscordStores = new WeakSet();

    const isStoreCandidate = (candidate) => candidate && typeof candidate === 'object';

    const getStoreName = (candidate) => {
        if (!isStoreCandidate(candidate)) {
            return null;
        }

        let getName;
        try { getName = typeof candidate.getName === 'function' && candidate.getName.length === 0 ? candidate.getName() : undefined; } catch (_) { getName = undefined; }
        const names = [
            getName,
            candidate.displayName,
            candidate.persistKey,
            candidate.constructor?.displayName,
            candidate.constructor?.persistKey,
            candidate.constructor?.name,
        ];

        for (const name of names) {
            if (typeof name === 'string' && name.length > 0) {
                return name;
            }
        }

        return null;
    };

    const subscribeToDiscordStore = (store, name) => {
        if (!store || typeof store.addChangeListener !== 'function' || subscribedDiscordStores.has(store)) {
            return;
        }
        store.addChangeListener(syncUnreadBadge);
        subscribedDiscordStores.add(store);
    };

    const subscribeToResolvedDiscordStores = () => {
        subscribeToDiscordStore(discordStores.guildRead, 'GuildReadStateStore');
        subscribeToDiscordStore(discordStores.relationship, 'RelationshipStore');
        subscribeToDiscordStore(discordStores.notificationSettings, 'NotificationSettingsStore');
    };

    const resolveDiscordStores = () => {
        if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings) {
            subscribeToResolvedDiscordStores();
            return discordStores;
        }

        if (storeSearchFailed) return null;

        const chunk = window.webpackChunkdiscord_app;
        if (!Array.isArray(chunk) || typeof chunk.push !== 'function') {
            return null;
        }

        let webpackRequire;
        try {
            chunk.push([[Symbol('tauricord-badge')], {}, (req) => {
                webpackRequire = req;
            }]);
        } catch (err) {
            return null;
        }

        const modules = Object.values(webpackRequire?.c || {});
        let found = 0;
        
        for (const module of modules) {
            const exported = module?.exports;
            const candidates = [
                exported,
                exported?.default,
                ...(isStoreCandidate(exported) ? Object.values(exported) : []),
            ];

            for (const candidate of candidates) {
                if (!isStoreCandidate(candidate)) {
                    continue;
                }

                const storeName = getStoreName(candidate);

                if (!discordStores.guildRead
                    && (storeName === 'GuildReadStateStore'
                        || (typeof candidate.getTotalMentionCount === 'function'
                            && typeof candidate.hasAnyUnread === 'function'))) {
                    discordStores.guildRead = candidate;
                    found++;
                }

                if (!discordStores.relationship
                    && (storeName === 'RelationshipStore'
                        || typeof candidate.getPendingCount === 'function')) {
                    discordStores.relationship = candidate;
                    found++;
                }

                if (!discordStores.notificationSettings
                    && (storeName === 'NotificationSettingsStore'
                        || typeof candidate.getDisableUnreadBadge === 'function')) {
                    discordStores.notificationSettings = candidate;
                    found++;
                }
            }

            if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings) {
                subscribeToResolvedDiscordStores();
                return discordStores;
            }
        }

        if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings) {
            subscribeToResolvedDiscordStores();
            return discordStores;
        }

        storeSearchFailed = true;
        return null;
    };

    const getDiscordUnreadCount = () => {
        const stores = resolveDiscordStores();
        if (!stores) return undefined;

        try {
            const mentionCount = Number(stores.guildRead.getTotalMentionCount?.() || 0);
            const pendingRequests = Number(stores.relationship.getPendingCount?.() || 0);
            const hasUnread = Boolean(stores.guildRead.hasAnyUnread?.());
            const disableUnreadBadge = Boolean(stores.notificationSettings.getDisableUnreadBadge?.());

            let totalCount = mentionCount + pendingRequests;
            if (!totalCount && hasUnread && !disableUnreadBadge) {
                totalCount = -1;
            }

            return totalCount === 0 ? null : totalCount;
        } catch (_) {
            return undefined;
        }
    };

    let lastUnreadCount = undefined;
    const syncUnreadBadge = () => {
        const unreadCount = getDiscordUnreadCount();
        const count = unreadCount === undefined
            ? parseUnreadCount(document.title)
            : unreadCount;

        if (count === lastUnreadCount) return;
        lastUnreadCount = count;

        void invokeTauriCommand('set_unread_badge', { count }).catch(() => {});
    };

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

    // Spoof window.chrome to make Discord think it's Chrome.
    // This may fail if Chrome or Discord has already sealed the property; catch
    // and fall back to augmenting the existing object so the script keeps running.
    try {
        Object.defineProperty(window, 'chrome', {
            get: () => ({
                runtime: {},
                webstore: {}
            }),
            configurable: true
        });
    } catch (_) {
        // Already defined and non-configurable – just patch missing members
        if (window.chrome) {
            window.chrome.runtime  = window.chrome.runtime  || {};
            window.chrome.webstore = window.chrome.webstore || {};
        }
    }

    // Hide that we're using Tauri/WebKit
    Object.defineProperty(navigator, 'webdriver', {
        get: () => false
    });

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
        openExternalUrl(href);
    };

    document.addEventListener('click', handleExternalAnchorClick, true);
    document.addEventListener('auxclick', handleExternalAnchorClick, true);
    document.addEventListener('keydown', (event) => {
        if (event.key !== 'Enter') {
            return;
        }

        handleExternalAnchorClick(event);
    }, true);

    const setupTitleObserver = () => {
        const el = document.querySelector('title');
        if (el) {
            new MutationObserver(syncUnreadBadge).observe(el, {
                childList: true,
                characterData: true,
                subtree: true,
            });
        }
    };
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', setupTitleObserver);
    } else {
        setupTitleObserver();
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
                    try {
                        return originalOpen.call(this, u.toString(), '_blank', 'noopener,noreferrer');
                    } catch {
                        openExternalUrl(u.toString());
                    }
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
fn set_windows_app_id(app_id: &str) {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let wide: Vec<u16> = app_id.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
}

#[cfg(target_os = "windows")]
fn windows_overlay_icon(count: Option<i64>) -> Option<tauri::image::Image<'static>> {
    // Small 16×16 badge dot for ITaskbarList3::SetOverlayIcon.
    // None = clear the overlay; the main window HICON is set explicitly
    // on the builder via .icon() and is never disturbed by overlay changes.
    let bytes: &'static [u8] = match count {
        None => return None,
        Some(c) if c < 0 => include_bytes!(concat!(env!("OUT_DIR"), "/badge-unread.png")),
        Some(1) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-1.png")),
        Some(2) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-2.png")),
        Some(3) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-3.png")),
        Some(4) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-4.png")),
        Some(5) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-5.png")),
        Some(6) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-6.png")),
        Some(7) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-7.png")),
        Some(8) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-8.png")),
        Some(9) => include_bytes!(concat!(env!("OUT_DIR"), "/badge-9.png")),
        _ => include_bytes!(concat!(env!("OUT_DIR"), "/badge-10.png")),
    };
    Some(
        tauri::image::Image::from_bytes(bytes)
            .expect("generated Windows badge icon should be valid PNG"),
    )
}

#[cfg(target_os = "windows")]
fn windows_taskbar_ready_map() -> &'static Mutex<HashMap<isize, bool>> {
    WINDOWS_TASKBAR_READY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "windows")]
fn windows_taskbar_button_created_message() -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

    *WINDOWS_TASKBAR_BUTTON_CREATED_MSG.get_or_init(|| {
        let wide: Vec<u16> = "TaskbarButtonCreated"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { RegisterWindowMessageW(wide.as_ptr()) }
    })
}

#[cfg(target_os = "windows")]
fn windows_hwnd(window: &tauri::WebviewWindow) -> Result<isize, String> {
    window.hwnd().map(|hwnd| hwnd.0 as isize).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn set_windows_taskbar_ready(hwnd: isize, ready: bool) {
    if let Ok(mut ready_map) = windows_taskbar_ready_map().lock() {
        ready_map.insert(hwnd, ready);
    }
}

#[cfg(target_os = "windows")]
fn windows_taskbar_ready(window: &tauri::WebviewWindow) -> bool {
    let Ok(hwnd) = windows_hwnd(window) else {
        return false;
    };

    windows_taskbar_ready_map()
        .lock()
        .ok()
        .and_then(|ready_map| ready_map.get(&hwnd).copied())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_taskbar_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _subclass_id: usize,
    _ref_data: usize,
) -> isize {
    use windows_sys::Win32::UI::Shell::DefSubclassProc;

    if msg == windows_taskbar_button_created_message() {
        set_windows_taskbar_ready(hwnd as isize, true);
    }

    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(target_os = "windows")]
fn install_windows_taskbar_hook(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::SetWindowSubclass;

    let hwnd = windows_hwnd(window)?;
    set_windows_taskbar_ready(hwnd, false);

    let installed = unsafe {
        SetWindowSubclass(
            hwnd as _,
            Some(windows_taskbar_subclass_proc),
            WINDOWS_TASKBAR_SUBCLASS_ID,
            0,
        )
    } != 0;

    if !installed {
        return Err("failed to install Windows taskbar subclass".to_string());
    }

    let delayed_window = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        if let Ok(hwnd) = windows_hwnd(&delayed_window) {
            set_windows_taskbar_ready(hwnd, true);
        }
    });

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_windows_unread_badge(window: tauri::WebviewWindow, count: Option<i64>) -> Result<(), String> {
    let apply = move |window: tauri::WebviewWindow, count: Option<i64>| {
        if !windows_taskbar_ready(&window) {}

        if let Err(error) = window.set_overlay_icon(windows_overlay_icon(count)) {
            eprintln!("[Badge] Failed to set overlay icon: {error}");
        }
    };

    let inner = window.clone();
    window
        .run_on_main_thread(move || apply(inner, count))
        .map_err(|e| e.to_string())?;

    Ok(())
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
    let count = count.filter(|count| *count != 0);

    #[cfg(target_os = "windows")]
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        return apply_windows_unread_badge(window, count);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        window.set_badge_count(count).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(feature = "with-tray")]
fn show_about_window(app: &AppHandle) {
    // If an about window already exists, just focus it
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");
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
            let _ = win.show();
            let _ = win.set_focus();
        }
        Err(e) => {
            eprintln!("Failed to create about window: {e}");
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    set_windows_app_id(WINDOWS_APP_ID);

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
                let builder = WebviewWindowBuilder::new(app, "main", url)
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

            #[cfg(target_os = "windows")]
            if let Err(error) = install_windows_taskbar_hook(&main_window) {
                eprintln!("Failed to install Windows taskbar hook: {error}");
            }

            // Set the window icon from the multi-resolution ICO embedded in the EXE
            // by tauri-build (resource ID 32512 = IDI_APPLICATION slot).
            // This is sharper than passing a PNG through Tauri's resize path.
            #[cfg(target_os = "windows")]
            if let Ok(hwnd) = main_window.hwnd() {
                use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    LoadIconW, LoadImageW, SendMessageW,
                    ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTSIZE, WM_SETICON,
                };
                unsafe {
                    let hinst = GetModuleHandleW(std::ptr::null());
                    // MAKEINTRESOURCEW(32512) — the embedded ICO resource
                    let res_id = 32512u16 as usize as *const u16;
                    let big_icon = LoadIconW(hinst, res_id);
                    // Small icon: explicit 16×16 from the same resource
                    let small_icon = LoadImageW(
                        hinst, res_id, IMAGE_ICON, 16, 16, LR_DEFAULTSIZE,
                    );
                    SendMessageW(hwnd.0 as _, WM_SETICON, ICON_BIG as _, big_icon as _);
                    SendMessageW(hwnd.0 as _, WM_SETICON, ICON_SMALL as _, small_icon as _);
                }
            }

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
