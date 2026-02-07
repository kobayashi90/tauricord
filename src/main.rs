// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            
            // Inject script to handle permissions and hide notification bar
            main_window.eval(r#"
                (function() {
                    // 1. Hide Discord's screen-share notification bar
                    const hideNotificationBar = () => {
                        const hideElement = (el) => {
                            if (el) {
                                el.style.setProperty('display', 'none', 'important');
                                el.style.setProperty('visibility', 'hidden', 'important');
                                el.style.setProperty('height', '0', 'important');
                                el.style.setProperty('opacity', '0', 'important');
                            }
                        };

                        // Initial scan
                        const scanAndHide = () => {
                            // Try common selectors
                            document.querySelectorAll('[class*="notification"], [class*="banner"], [role="alert"], [class*="notice"], div').forEach(el => {
                                const text = el.textContent || '';
                                if (text.includes('screen') || text.includes('sharing') || text.includes('broadcast') || 
                                    text.includes('Bildschirm') || text.includes('Audio') || text.includes('teilt') || 
                                    text.includes('streaming') || text.includes('going live')) {
                                    hideElement(el.closest('[class*="notification"]') || el.closest('[class*="banner"]') || el.closest('[role="alert"]') || el);
                                }
                            });
                        };

                        // Run immediately
                        scanAndHide();

                        // Watch for new notifications
                        const observer = new MutationObserver(() => {
                            scanAndHide();
                        });
                        observer.observe(document.body, { 
                            childList: true, 
                            subtree: true 
                        });

                        // Also check periodically
                        setInterval(scanAndHide, 500);
                    };
                    
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
                    
                    // 4. Enable drag and drop by preventing default blocking behavior
                    document.addEventListener('dragover', (e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        e.dataTransfer.dropEffect = 'copy';
                    });
                    
                    document.addEventListener('dragleave', (e) => {
                        e.preventDefault();
                        e.stopPropagation();
                    });
                    
                    document.addEventListener('drop', (e) => {
                        e.preventDefault();
                        e.stopPropagation();
                    });
                    
                    // Remove drag-drop prevention on common Discord elements
                    const preventDefaults = ['dragenter', 'dragover', 'dragleave', 'drop'];
                    preventDefaults.forEach(eventName => {
                        document.addEventListener(eventName, (e) => {
                            e.preventDefault();
                            e.stopPropagation();
                        }, true);
                    });
                    
                    hideNotificationBar();
                })();
            "#).ok();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
