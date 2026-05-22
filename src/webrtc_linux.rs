use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

pub fn webrtc_help_text() -> Option<&'static str> {
    Some(
        "Discord voice needs WebRTC-enabled WebKitGTK.\n\
         \n\
         === Ubuntu 22.04 (Ubuntu-only — not for Debian) ===\n\
         sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-experimental\n\
         sudo apt update && sudo apt upgrade\n\
         sudo apt install gstreamer1.0-plugins-bad libnice10 \\\n\
                        libwebrtc-audio-processing1\n\
         \n\
         === Ubuntu 24.04+ (stock WebKit has WebRTC) ===\n\
         sudo apt install gstreamer1.0-plugins-bad libnice10 \\\n\
                        libwebrtc-audio-processing1\n\
         \n\
         === Debian 12 (stock WebKit 2.50.x lacks WebRTC) ===\n\
         sudo apt install gstreamer1.0-plugins-bad libnice10 \\\n\
                        libwebrtc-audio-processing1\n\
         Then upgrade WebKitGTK from Debian experimental:\n\
         echo 'deb http://deb.debian.org/debian experimental main' \\\n\
           | sudo tee -a /etc/apt/sources.list\n\
         sudo apt update && sudo apt install -t experimental \\\n\
           libwebkit2gtk-4.1-0\n\
         Or use the Flatpak build which bundles WebRTC.\n\
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

pub fn setup_webrtc(window: &tauri::WebviewWindow, discord_url: &str) -> Result<(), String> {
    let url = discord_url.to_owned();
    let window = window.clone();
    window.as_ref().with_webview(move |pw| {
        let wv: webkit2gtk::WebView = pw.inner();

        if let Some(settings) = wv.settings() {
            settings.set_enable_media_stream(true);
            settings.set_enable_webrtc(true);
            settings.set_enable_developer_extras(true);
            log::info!("[WebRTC] Enabled media stream + WebRTC + devtools in WebKit settings");
        } else {
            log::warn!("[WebRTC] Could not access WebKit settings");
        }

        wv.connect_permission_request(move |_webview, request| {
            request.allow();
            log::info!("[WebRTC] Permission auto-granted");
            true
        });

        // Navigate to Discord now — WebRTC settings are applied before the
        // page's JS context is created, so RTCPeerConnection will be exposed.
        wv.load_uri(&url);
        log::info!("[WebRTC] Setup complete, navigating to Discord");
    }).map_err(|e| format!("with_webview failed: {e}"))?;

    Ok(())
}
