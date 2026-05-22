use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use urlencoding::encode;

pub fn show_settings_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }

    let settings_script = r"
function getSettings() {
    return window.__TAURI__.core.invoke('get_settings');
}
function saveSettings(s) {
    return window.__TAURI__.core.invoke('save_settings', { newSettings: s });
}
";

    let css = r"
*{margin:0;padding:0;box-sizing:border-box}
html,body{width:100%;height:100%}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#1e1f22;color:#dbdee1;padding:24px;user-select:none;-webkit-user-select:none}
@media(prefers-color-scheme:light){body{background:#f2f3f5;color:#313338}}
h1{font-size:20px;font-weight:700;margin-bottom:20px;color:#f2f3f5}
@media(prefers-color-scheme:light){h1{color:#1e1f22}}
.group{margin-bottom:16px;background:#2b2d31;border-radius:8px;padding:12px}
@media(prefers-color-scheme:light){.group{background:#e0e1e6}}
.group label{display:flex;align-items:center;justify-content:space-between;font-size:13px;cursor:pointer;padding:6px 0}
.group label span{flex:1}
.group .desc{font-size:11px;color:#949ba4;margin-top:-2px;margin-bottom:8px}
@media(prefers-color-scheme:light){.group .desc{color:#6d6f78}}
input[type=checkbox]{accent-color:#5865f2;width:18px;height:18px;cursor:pointer}
input[type=text]{width:100%;padding:8px 10px;border-radius:6px;border:1px solid #3f4148;background:#1e1f22;color:#dbdee1;font-size:13px;outline:none;margin-top:6px}
input[type=text]:focus{border-color:#5865f2}
@media(prefers-color-scheme:light){input[type=text]{background:#f2f3f5;border-color:#ccc;color:#313338}}
.btn{padding:10px 24px;background:#5865f2;color:#fff;border:none;border-radius:8px;font-size:14px;cursor:pointer;width:100%;font-weight:500;margin-top:8px}
.btn:hover{background:#4752c4}
.status{text-align:center;font-size:12px;margin-top:8px;min-height:18px;transition:opacity .3s}
";

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Settings</title><style>{css}</style><script>{settings_script}</script></head>
<body>
<h1>Settings</h1>
<div class="group">
    <label><span>Hide on close</span><input type="checkbox" id="hide_on_close"></label>
    <div class="desc">Minimize to tray instead of closing</div>
    <label><span>Start minimized</span><input type="checkbox" id="start_minimized"></label>
    <div class="desc">Start Tauricord minimized to tray</div>
    <label><span>Launch on boot</span><input type="checkbox" id="launch_on_boot"></label>
    <div class="desc">Auto-start with your system</div>
</div>
<div class="group">
    <label><span>GPU rendering</span><input type="checkbox" id="gpu_rendering"></label>
    <div class="desc">Enable hardware acceleration (requires restart)</div>
</div>
<div class="group">
    <label><span>Custom CSS path</span></label>
    <div class="desc">Path to a .css file to inject into Discord</div>
    <input type="text" id="custom_css_path" placeholder="/home/user/.config/tauricord/theme.css">
</div>
<div class="group">
    <label><span>Memory saver (minutes)</span></label>
    <div class="desc">Unload Discord after inactivity (0 = disabled)</div>
    <input type="text" id="memory_saver_minutes" placeholder="0">
</div>
<button class="btn" onclick="onSave()">Save</button>
<div class="status" id="status"></div>
<script>
let currentSettings = {{}};

window.addEventListener('DOMContentLoaded', async () => {{
    try {{
        currentSettings = await getSettings();
        document.getElementById('hide_on_close').checked = currentSettings.hide_on_close;
        document.getElementById('start_minimized').checked = currentSettings.start_minimized;
        document.getElementById('launch_on_boot').checked = currentSettings.launch_on_boot;
        document.getElementById('gpu_rendering').checked = currentSettings.gpu_rendering;
        document.getElementById('custom_css_path').value = currentSettings.custom_css_path || '';
        document.getElementById('memory_saver_minutes').value = currentSettings.memory_saver_minutes || '';
    }} catch(e) {{
        document.getElementById('status').textContent = 'Failed to load settings: ' + e;
    }}
}});

async function onSave() {{
    try {{
        const ms = document.getElementById('memory_saver_minutes').value;
        const updated = {{
            ...currentSettings,
            hide_on_close: document.getElementById('hide_on_close').checked,
            start_minimized: document.getElementById('start_minimized').checked,
            launch_on_boot: document.getElementById('launch_on_boot').checked,
            gpu_rendering: document.getElementById('gpu_rendering').checked,
            custom_css_path: document.getElementById('custom_css_path').value || null,
            memory_saver_minutes: ms ? parseInt(ms, 10) : null,
        }};
        await saveSettings(updated);
        currentSettings = updated;
        document.getElementById('status').textContent = 'Saved!';
        setTimeout(() => document.getElementById('status').textContent = '', 2000);
    }} catch(e) {{
        document.getElementById('status').textContent = 'Error: ' + e;
    }}
}}
</script>
</body>
</html>"#
    );

    let data_url = format!("data:text/html;charset=utf-8,{}", encode(&html));
    let about_url = match tauri::Url::parse(&data_url) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Failed to parse settings URL: {e}");
            return;
        }
    };

    match WebviewWindowBuilder::new(app, "settings", WebviewUrl::External(about_url))
        .title("Tauricord Settings")
        .inner_size(440.0, 540.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .center()
        .build()
    {
        Ok(win) => {
            let _ = win.show();
            let _ = win.set_focus();
        }
        Err(e) => {
            log::error!("Failed to create settings window: {e}");
        }
    }
}
