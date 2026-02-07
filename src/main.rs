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
                    
                    // 4. Handle file drag and drop for Discord uploads
                    const handleDragDrop = () => {
                        let dragCounter = 0;

                        document.addEventListener('dragenter', (e) => {
                            dragCounter++;
                            e.preventDefault();
                            e.stopPropagation();
                        });

                        document.addEventListener('dragleave', (e) => {
                            dragCounter--;
                            e.preventDefault();
                            e.stopPropagation();
                        });

                        document.addEventListener('dragover', (e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            e.dataTransfer.dropEffect = 'copy';
                        });

                        document.addEventListener('drop', (e) => {
                            dragCounter = 0;
                            e.preventDefault();
                            e.stopPropagation();
                            
                            if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length > 0) {
                                // Find Discord's file input or upload element
                                const fileInput = document.querySelector('input[type="file"]');
                                
                                if (fileInput) {
                                    // Transfer files to the input
                                    fileInput.files = e.dataTransfer.files;
                                    
                                    // Trigger change event
                                    const event = new Event('change', { bubbles: true });
                                    fileInput.dispatchEvent(event);
                                    
                                    // Also trigger input event
                                    const inputEvent = new Event('input', { bubbles: true });
                                    fileInput.dispatchEvent(inputEvent);
                                    
                                    console.log('Files dropped:', e.dataTransfer.files.length);
                                } else {
                                    // Alternative: simulate file picker dialog by clicking any upload button
                                    const uploadButtons = document.querySelectorAll('[aria-label*="upload" i], [title*="upload" i], button[class*="upload"]');
                                    if (uploadButtons.length > 0) {
                                        console.log('Found upload button, triggering click');
                                        uploadButtons[0].click();
                                    } else {
                                        console.log('No file input or upload button found');
                                    }
                                }
                            }
                        });
                    };
                    
                    handleDragDrop();
                    
                    hideNotificationBar();
                })();
            "#).ok();
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
