// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            
            // Inject script to handle permissions and hide Discord's in-app screen share bar.
            // NOTE: The browser-level WebView2 "sharing your screen" notification bar
            // CANNOT be hidden — WebView2/Tauri has no equivalent to Electron's
            // setDisplayMediaRequestHandler(). Dorion and WebCord both only hide
            // Discord's own DOM bar, not the browser chrome bar.
            main_window.eval(r#"
                (function() {
                    // 1. Hide Discord's in-app screen-share notification bar via CSS
                    //    Same approach as Dorion (extra.css) and WebCord (discord.css):
                    //    - WebCord:  div[class^=bar_] { height: 0px }
                    //    - Dorion:   div[class^='base'] div[class^='bar_'] { z-index/bg }
                    const style = document.createElement('style');
                    style.textContent = `
                        /* Hide Discord's in-app screen share / Go Live bar */
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
                })();
            "#).ok();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
