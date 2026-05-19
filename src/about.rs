use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use base64::Engine;

pub fn show_about_window(app: &AppHandle) {
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

    let about_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>About Tauricord</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body {{ width: 100%; height: 100%; }}
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
        @media (prefers-color-scheme: light) {{
            body {{ background: #f2f3f5; color: #313338; }}
            .version {{ color: #6d6f78; }}
            .desc {{ color: #5e5f64; }}
            a {{ background: #e0e1e6; }}
            a:hover {{ background: #d1d2d8; }}
            .footer {{ color: #a0a1a8; }}
        }}
        h1 {{
            font-size: 26px; font-weight: 700; color: #f2f3f5;
            margin-bottom: 8px; letter-spacing: -0.5px;
        }}
        @media (prefers-color-scheme: light) {{
            h1 {{ color: #1e1f22; }}
        }}
        .version {{ font-size: 12px; color: #949ba4; margin-bottom: 20px; letter-spacing: 0.5px; }}
        .desc {{
            font-size: 13px; color: #b5bac1; text-align: center;
            line-height: 1.6; margin-bottom: 28px; max-width: 380px;
        }}
        .links {{ display: flex; flex-direction: column; gap: 10px; width: 100%; max-width: 260px; margin-bottom: 32px; }}
        a {{
            display: block; padding: 11px 16px; background: #2b2d31;
            color: #00a8fc; text-decoration: none; border-radius: 8px;
            font-size: 14px; font-weight: 500; text-align: center;
            cursor: pointer; transition: background 0.15s;
        }}
        a:hover {{ background: #383a40; }}
        .footer {{ font-size: 11px; color: #6d6f78; line-height: 1.6; }}
        .icon {{ width: 80px; height: 80px; margin-bottom: 16px; border-radius: 16px; box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3); }}
    </style>
    <script>document.addEventListener('contextmenu', (e) => e.preventDefault());</script>
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
    let about_url = tauri::Url::parse(&data_url).expect("about URL should always be valid");

    match WebviewWindowBuilder::new(app, "about", WebviewUrl::External(about_url))
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
            log::error!("Failed to create about window: {e}");
        }
    }
}
