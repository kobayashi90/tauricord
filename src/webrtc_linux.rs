use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

pub fn webrtc_help_text() -> Option<&'static str> {
    Some(
        "Discord voice needs WebRTC-enabled WebKitGTK.\n\
         \n\
         === Ubuntu / Debian ===\n\
         sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-experimental\n\
         sudo apt update && sudo apt upgrade\n\
         sudo apt install gstreamer1.0-nice gstreamer1.0-plugins-bad \\\n\
                        libnice-dev gstreamer1.0-plugins-ugly libsrtp2-dev\n\
         \n\
         === Arch Linux ===\n\
         yay -S webkitgtk-4.1-webrtee\n\
         \n\
         === NixOS ===\n\
         webkitgtk.override { enableWebRTC = true; }\n\
         \n\
         === Fedora ===\n\
         sudo dnf copr enable grul/LibWebKit\n\
         sudo dnf install webkitgtk\n\
         \n\
         Restart Tauricord after installing.\n\
         Flatpak users: use the Flatpak build which bundles WebRTC."
    )
}

pub fn setup_webrtc(window: &tauri::WebviewWindow) -> Result<(), String> {
    let window = window.clone();
    window.as_ref().with_webview(move |pw| {
        let wv: webkit2gtk::WebView = pw.inner();

        if let Some(settings) = wv.settings() {
            settings.set_enable_media_stream(true);
            settings.set_enable_webrtc(true);
            log::info!("[WebRTC] Enabled media stream + WebRTC in WebKit settings");
        } else {
            log::warn!("[WebRTC] Could not access WebKit settings");
        }

        wv.connect_permission_request(move |_webview, request| {
            request.allow();
            log::info!("[WebRTC] Permission auto-granted");
            true
        });

        log::info!("[WebRTC] Setup complete");
    }).map_err(|e| format!("with_webview failed: {e}"))?;

    Ok(())
}
