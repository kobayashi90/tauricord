#[cfg(target_os = "windows")]
use std::{collections::HashMap, sync::Mutex};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
const WINDOWS_APP_ID: &str = "io.tauricord.dev";

#[cfg(target_os = "windows")]
const WINDOWS_TASKBAR_SUBCLASS_ID: usize = 1;

#[cfg(target_os = "windows")]
struct WindowsTaskbarState {
    ready_map: Mutex<HashMap<isize, bool>>,
    button_created_msg: OnceLock<u32>,
}

#[cfg(target_os = "windows")]
static WINDOWS_STATE: OnceLock<WindowsTaskbarState> = OnceLock::new();

#[cfg(target_os = "windows")]
fn windows_state() -> &'static WindowsTaskbarState {
    WINDOWS_STATE.get_or_init(|| WindowsTaskbarState {
        ready_map: Mutex::new(HashMap::new()),
        button_created_msg: OnceLock::new(),
    })
}

pub fn user_agent() -> &'static str {
    if cfg!(target_os = "windows") {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    } else if cfg!(target_os = "macos") {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    } else {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    }
}

#[cfg(target_os = "windows")]
pub fn set_windows_app_id() {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    let wide: Vec<u16> = WINDOWS_APP_ID
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
}

#[cfg(target_os = "windows")]
fn overlay_icon(count: Option<i64>) -> Option<tauri::image::Image<'static>> {
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
            .expect("generated badge icon should be valid PNG"),
    )
}

#[cfg(target_os = "windows")]
fn taskbar_button_created_message() -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;

    *windows_state().button_created_msg.get_or_init(|| {
        let wide: Vec<u16> = "TaskbarButtonCreated"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { RegisterWindowMessageW(wide.as_ptr()) }
    })
}

#[cfg(target_os = "windows")]
fn window_hwnd(window: &tauri::WebviewWindow) -> Result<isize, String> {
    window.hwnd().map(|hwnd| hwnd.0 as isize).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn set_taskbar_ready(hwnd: isize, ready: bool) {
    if let Ok(mut map) = windows_state().ready_map.lock() {
        map.insert(hwnd, ready);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn taskbar_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _subclass_id: usize,
    _ref_data: usize,
) -> isize {
    use windows_sys::Win32::UI::Shell::DefSubclassProc;

    if msg == taskbar_button_created_message() {
        set_taskbar_ready(hwnd as isize, true);
    }

    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(target_os = "windows")]
pub fn install_taskbar_hook(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::SetWindowSubclass;

    let hwnd = window_hwnd(window)?;
    set_taskbar_ready(hwnd, false);

    let installed = unsafe {
        SetWindowSubclass(
            hwnd as _,
            Some(taskbar_subclass_proc),
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
        if let Ok(hwnd) = window_hwnd(&delayed_window) {
            set_taskbar_ready(hwnd, true);
        }
    });

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_windows_badge(window: tauri::WebviewWindow, count: Option<i64>) -> Result<(), String> {
    let window_clone = window.clone();
    window
        .run_on_main_thread(move || {
            if let Err(error) = window_clone.set_overlay_icon(overlay_icon(count)) {
                log::error!("[Badge] Failed to set overlay icon: {error}");
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
pub fn set_window_icon(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        LoadIconW, LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON,
        LR_DEFAULTSIZE, WM_SETICON,
    };

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let hinst = GetModuleHandleW(std::ptr::null());
            let res_id = 32512u16 as usize as *const u16;
            let big_icon = LoadIconW(hinst, res_id);
            let small_icon = LoadImageW(hinst, res_id, IMAGE_ICON, 16, 16, LR_DEFAULTSIZE);
            SendMessageW(hwnd.0 as _, WM_SETICON, ICON_BIG as _, big_icon as _);
            SendMessageW(hwnd.0 as _, WM_SETICON, ICON_SMALL as _, small_icon as _);
        }
    }
}

pub fn set_unread_badge(app: &AppHandle, count: Option<i64>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        return apply_windows_badge(window, count);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        window.set_badge_count(count).map_err(|e| e.to_string())
    }
}
