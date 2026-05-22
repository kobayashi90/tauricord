use std::path::PathBuf;
use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

pub fn start_css_watcher(app: AppHandle, css_path: PathBuf) {
    if !css_path.exists() {
        log::info!("Custom CSS not found at {:?}, skipping watcher", css_path);
        return;
    }

    log::info!("Watching custom CSS file: {:?}", css_path);

    let app_clone = app.clone();
    let path_clone = css_path.clone();

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create file watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&path_clone, RecursiveMode::NonRecursive) {
            log::error!("Failed to watch CSS file: {e}");
            return;
        }

        inject_css_from_file(&app_clone, &path_clone);

        for event in rx {
            if matches!(event.kind, notify::EventKind::Modify(_) | notify::EventKind::Create(_)) {
                inject_css_from_file(&app_clone, &path_clone);
            }
        }
    });
}

fn inject_css_from_file(app: &AppHandle, path: &PathBuf) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to read CSS file: {e}");
            return;
        }
    };

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(&format!(
            r#"document.getElementById('tauricord-css')?.remove();
const s = document.createElement('style');
s.id = 'tauricord-css';
s.textContent = {};
document.head?.appendChild(s);"#,
            serde_json::to_string(&content).unwrap_or_default()
        ));
    }
}
