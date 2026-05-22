(function() {
    const SPLASH_ID = 'tauricord-splash';

    const showSplash = () => {
        const check = () => {
            if (!document.body) { setTimeout(check, 10); return; }
            if (document.getElementById(SPLASH_ID)) return;
            const splash = document.createElement('div');
            splash.id = SPLASH_ID;
            splash.style.cssText = 'position:fixed;inset:0;background:#313338;display:flex;flex-direction:column;align-items:center;justify-content:center;z-index:99999;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif';
            splash.innerHTML = '<div style="width:40px;height:40px;border:4px solid #5865f2;border-top-color:transparent;border-radius:50%;animation:ts-spin .8s linear infinite;margin-bottom:16px"></div><div style="font-size:16px;color:#b5bac1;font-weight:500">Loading Tauricord</div><style>@keyframes ts-spin{to{transform:rotate(360deg)}}</style>';
            document.body.appendChild(splash);
        };
        check();
    };

    const hideSplash = () => {
        const el = document.getElementById(SPLASH_ID);
        if (el) {
            el.style.transition = 'opacity .3s';
            el.style.opacity = '0';
            setTimeout(() => el.remove(), 300);
        }
    };

    const waitForDiscordContent = () => {
        const interval = setInterval(() => {
            if (document.querySelector('[class^="app"]') || document.querySelector('[class^="layers"]')) {
                hideSplash();
                clearInterval(interval);
            }
        }, 200);
        setTimeout(() => clearInterval(interval), 30000);
    };

    showSplash();
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', waitForDiscordContent);
    else waitForDiscordContent();

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
            const host = u.hostname;
            return host === location.hostname
                || host === 'discord.com'
                || host.endsWith('.discord.com')
                || host === 'hcaptcha.com'
                || host.endsWith('.hcaptcha.com')
                || host === 'challenges.cloudflare.com';
        } catch {
            return false;
        }
    };

    const openExternalUrl = (url) => {
        try {
            const absoluteUrl = new URL(url, location.origin).toString();
            void invokeTauriCommand('open_external', { url: absoluteUrl });
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

    let discordStores = {};
    let storesResolved = false;
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

    const onStoreChange = () => {
        syncUnreadBadge();
        syncPresence();
    };

    const subscribeToDiscordStore = (store) => {
        if (!store || typeof store.addChangeListener !== 'function' || subscribedDiscordStores.has(store)) {
            return;
        }
        store.addChangeListener(onStoreChange);
        subscribedDiscordStores.add(store);
    };

    const subscribeToResolvedDiscordStores = () => {
        subscribeToDiscordStore(discordStores.guildRead);
        subscribeToDiscordStore(discordStores.relationship);
        subscribeToDiscordStore(discordStores.notificationSettings);
        subscribeToDiscordStore(discordStores.runningGame);
        subscribeToDiscordStore(discordStores.voiceState);
    };

    const resolveDiscordStores = () => {
        if (storesResolved) return discordStores;

        if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings
            && discordStores.runningGame && discordStores.voiceState) {
            subscribeToResolvedDiscordStores();
            storesResolved = true;
            return discordStores;
        }

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
                }

                if (!discordStores.relationship
                    && (storeName === 'RelationshipStore'
                        || typeof candidate.getPendingCount === 'function')) {
                    discordStores.relationship = candidate;
                }

                if (!discordStores.notificationSettings
                    && (storeName === 'NotificationSettingsStore'
                        || typeof candidate.getDisableUnreadBadge === 'function')) {
                    discordStores.notificationSettings = candidate;
                }

                if (!discordStores.runningGame
                    && (storeName === 'RunningGameStore'
                        || typeof candidate.getRunningGames === 'function')) {
                    discordStores.runningGame = candidate;
                }

                if (!discordStores.voiceState
                    && (storeName === 'VoiceStateStore'
                        || typeof candidate.getVoiceState === 'function')) {
                    discordStores.voiceState = candidate;
                }
            }

            if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings
                && discordStores.runningGame && discordStores.voiceState) {
                subscribeToResolvedDiscordStores();
                storesResolved = true;
                return discordStores;
            }
        }

        if (discordStores.guildRead && discordStores.relationship && discordStores.notificationSettings
            && discordStores.runningGame && discordStores.voiceState) {
            subscribeToResolvedDiscordStores();
            storesResolved = true;
            return discordStores;
        }

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

    let lastPresence = { game: null, inVoice: false };

    const syncPresence = () => {
        const stores = resolveDiscordStores();
        if (!stores) return;

        try {
            let gameName = null;
            if (stores.runningGame && typeof stores.runningGame.getRunningGames === 'function') {
                const games = stores.runningGame.getRunningGames();
                if (Array.isArray(games) && games.length > 0) {
                    gameName = games[0].name || null;
                }
            }

            let inVoice = false;
            if (stores.voiceState && typeof stores.voiceState.getVoiceState === 'function') {
                const currentUser = window.__TAURI__?.core?.invoke ? undefined : undefined;
                const vs = stores.voiceState.getVoiceState();
                if (vs && vs.channelId) inVoice = true;
            }

            if (gameName === lastPresence.game && inVoice === lastPresence.inVoice) return;
            lastPresence = { game: gameName, inVoice };

            void invokeTauriCommand('set_presence', { game: gameName, inVoice }).catch(() => {});
        } catch (_) {}
    };

    let lastUnreadCount = undefined;
    let lastDocumentTitle = '';

    const syncUnreadBadge = () => {
        const unreadCount = getDiscordUnreadCount();
        const count = unreadCount === undefined
            ? parseUnreadCount(document.title)
            : unreadCount;

        if (count === lastUnreadCount) return;
        lastUnreadCount = count;

        void invokeTauriCommand('set_unread_badge', { count }).catch(() => {});
    };

    const showLinkToast = (url) => {
        try {
            const toast = document.createElement('div');
            toast.textContent = 'Opened in browser';
            toast.style.cssText = 'position:fixed;bottom:24px;left:50%;transform:translateX(-50%);background:#1e1f22;color:#dbdee1;padding:10px 18px;border-radius:8px;font-size:13px;z-index:99999;box-shadow:0 4px 12px rgba(0,0,0,.4);border:1px solid #2b2d31;max-width:80vw;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;transition:opacity .3s';
            document.body.appendChild(toast);
            setTimeout(() => { toast.style.opacity = '0'; setTimeout(() => toast.remove(), 300); }, 2000);
        } catch (_) {}
    };

    const ERROR_OVERLAY_ID = 'tauricord-error-overlay';
    const showErrorOverlay = () => {
        const check = () => {
            if (!document.body) { setTimeout(check, 100); return; }
            if (document.getElementById(ERROR_OVERLAY_ID)) return;
            const overlay = document.createElement('div');
            overlay.id = ERROR_OVERLAY_ID;
            overlay.style.cssText = 'position:fixed;inset:0;background:#313338;display:flex;flex-direction:column;align-items:center;justify-content:center;z-index:99998;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif';
            overlay.innerHTML = '<div style="font-size:48px;margin-bottom:16px">!</div><div style="font-size:18px;font-weight:600;margin-bottom:8px;color:#f2f3f5">Failed to load Discord</div><div style="font-size:13px;color:#949ba4;margin-bottom:20px">Check your internet connection and try again.</div><button id="tauricord-retry-btn" style="padding:10px 24px;background:#5865f2;color:#fff;border:none;border-radius:8px;font-size:14px;cursor:pointer">Retry</button>';
            document.body.appendChild(overlay);
            document.getElementById('tauricord-retry-btn')?.addEventListener('click', () => location.reload());
        };
        check();
    };

    let loadingCheckAttempts = 0;
    const MAX_LOADING_ATTEMPTS = 6;
    const LOADING_CHECK_INTERVAL = 5000;

    const checkDiscordLoaded = () => {
        loadingCheckAttempts++;
        const hasContent = document.querySelector('[class^="app"]') || document.querySelector('[class^="layers"]');
        if (hasContent) { cancelLoadingTimer(); return; }
        if (loadingCheckAttempts >= MAX_LOADING_ATTEMPTS) showErrorOverlay();
        else setTimeout(checkDiscordLoaded, LOADING_CHECK_INTERVAL);
    };
    setTimeout(checkDiscordLoaded, LOADING_CHECK_INTERVAL);

    const cancelLoadingTimer = () => {
        loadingCheckAttempts = MAX_LOADING_ATTEMPTS;
        const el = document.getElementById(ERROR_OVERLAY_ID);
        if (el) el.remove();
    };

    const MEMORY_SAVER_ID = 'tauricord-memory-saver';

    const showMemorySaver = () => {
        if (document.getElementById(MEMORY_SAVER_ID)) return;
        const overlay = document.createElement('div');
        overlay.id = MEMORY_SAVER_ID;
        overlay.style.cssText = 'position:fixed;inset:0;background:#1e1f22;display:flex;flex-direction:column;align-items:center;justify-content:center;z-index:99997;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;cursor:pointer';
        overlay.innerHTML = '<div style="font-size:32px;margin-bottom:12px;opacity:.6">⏸</div><div style="font-size:16px;color:#b5bac1;font-weight:500">Tauricord is suspended</div><div style="font-size:13px;color:#6d6f78;margin-top:6px">Click anywhere to resume</div>';
        overlay.addEventListener('click', () => { overlay.remove(); });
        document.body.appendChild(overlay);
    };

    const startMemorySaver = (timeoutMs) => {
        if (!timeoutMs || timeoutMs < 60000) return;
        let idleTimer = null;
        const resetIdle = () => {
            if (idleTimer) clearTimeout(idleTimer);
            const el = document.getElementById(MEMORY_SAVER_ID);
            if (el) el.remove();
            idleTimer = setTimeout(showMemorySaver, timeoutMs);
        };
        ['mousedown','keydown','touchstart','scroll','wheel'].forEach(evt =>
            document.addEventListener(evt, resetIdle, { passive: true, capture: true })
        );
        resetIdle();
    };

    (async () => {
        try {
            const invoke = window.__TAURI__?.core?.invoke;
            if (!invoke) return;
            const s = await invoke('get_settings');
            if (s?.memory_saver_minutes > 0) startMemorySaver(s.memory_saver_minutes * 60000);
        } catch (_) {}
    })();

    try {
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
            }),
            configurable: true
        });
    } catch (_) {}

    try {
        Object.defineProperty(navigator, 'vendor', {
            get: () => "Google Inc.",
            configurable: true
        });
    } catch (_) {}

    try {
        Object.defineProperty(navigator, 'platform', {
            get: () => "Linux x86_64",
            configurable: true
        });
    } catch (_) {}

    const pageLoadStartMs = (() => {
        const fromTimeOrigin = Number(performance?.timeOrigin);
        if (Number.isFinite(fromTimeOrigin) && fromTimeOrigin > 0) return Math.round(fromTimeOrigin);
        const fromNavigationStart = Number(performance?.timing?.navigationStart);
        if (Number.isFinite(fromNavigationStart) && fromNavigationStart > 0) return Math.round(fromNavigationStart);
        return Date.now();
    })();
    const pageLoadStartS = pageLoadStartMs / 1000;
    const chromeCsi = () => ({ onloadT: pageLoadStartMs, startE: pageLoadStartMs });
    const chromeLoadTimes = () => ({
        requestTime: pageLoadStartS, startLoadTime: pageLoadStartS,
        commitLoadTime: pageLoadStartS, finishDocumentLoadTime: pageLoadStartS,
        finishLoadTime: pageLoadStartS
    });

    try {
        Object.defineProperty(window, 'chrome', {
            get: () => ({
                runtime: {},
                webstore: {},
                app: {
                    isInstalled: false,
                    InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
                    RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' },
                    getDetails: () => null, getIsInstalled: () => false, runningState: () => 'cannot_run'
                },
                csi: chromeCsi, loadTimes: chromeLoadTimes
            }),
            configurable: true
        });
    } catch (_) {
        if (window.chrome) {
            window.chrome.runtime = window.chrome.runtime || {};
            window.chrome.webstore = window.chrome.webstore || {};
            window.chrome.app = window.chrome.app || {
                isInstalled: false,
                InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
                RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' },
                getDetails: () => null, getIsInstalled: () => false, runningState: () => 'cannot_run'
            };
            window.chrome.csi = window.chrome.csi || chromeCsi;
            window.chrome.loadTimes = window.chrome.loadTimes || chromeLoadTimes;
        }
    }

    Object.defineProperty(navigator, 'webdriver', { get: () => false });

    const hasUnprefixedRTC = typeof window.RTCPeerConnection === 'function';
    const hasPrefixedRTC = typeof window.webkitRTCPeerConnection === 'function';

    if (hasUnprefixedRTC && !hasPrefixedRTC) {
        window.webkitRTCPeerConnection = window.RTCPeerConnection;
        window.webkitRTCSessionDescription = window.RTCSessionDescription;
        window.webkitRTCIceCandidate = window.RTCIceCandidate;
    } else if (hasPrefixedRTC && !hasUnprefixedRTC) {
        window.RTCPeerConnection = window.webkitRTCPeerConnection;
        window.RTCSessionDescription = window.webkitRTCSessionDescription;
        window.RTCIceCandidate = window.webkitRTCIceCandidate;
    }
    document.addEventListener('contextmenu', (e) => {
        if (e.defaultPrevented) return;
        const el = e.target.closest('input, textarea, [contenteditable]');
        if (el && (el instanceof HTMLInputElement
            || el instanceof HTMLTextAreaElement
            || el.isContentEditable)) {
            return;
        }
        e.preventDefault();
    });

    const style = document.createElement('style');
    style.textContent = `
        div[class^='base'] div[class^='bar_'] { display: none !important; }
        nav[aria-label*='Servers'] [class*='listItem'] { display: flex !important; justify-content: center !important; }
        nav[aria-label*='Servers'] [class*='wrapper_'] { transform: translateZ(0); }
    `;
    const injectCSS = () => {
        if (document.head) { document.head.appendChild(style); }
        else { document.addEventListener('DOMContentLoaded', () => document.head.appendChild(style)); }
    };
    injectCSS();

    if (navigator.permissions) {
        const originalQuery = navigator.permissions.query;
        navigator.permissions.query = async (params) => {
            if (params.name === 'microphone' || params.name === 'camera') return { name: params.name, state: 'granted' };
            return originalQuery.call(navigator.permissions, params);
        };
    }

    if (navigator.mediaDevices) {
        const originalGetUserMedia = navigator.mediaDevices.getUserMedia;
        navigator.mediaDevices.getUserMedia = async function(constraints) {
            try { return await originalGetUserMedia.call(this, constraints); }
            catch (err) {
                console.error('[Voice] getUserMedia failed', { constraints, error: err?.name, message: err?.message });
                throw err;
            }
        };
    }

    if (typeof window.RTCPeerConnection !== 'function' && typeof window.webkitRTCPeerConnection !== 'function') {
        console.warn('[Voice] WebRTC not available — WebKitGTK was compiled without WebRTC.');
        const showWebrtcHelp = async () => {
            try {
                const invoke = window.__TAURI__?.core?.invoke;
                if (!invoke) return;
                const help = await invoke('get_webrtc_help');
                if (!help) return;
                const el = document.createElement('div');
                el.style.cssText = 'position:fixed;bottom:24px;right:24px;max-width:480px;background:#1e1f22;color:#dbdee1;padding:16px 20px;border-radius:10px;font-size:12px;line-height:1.5;z-index:99999;box-shadow:0 8px 24px rgba(0,0,0,.5);border:1px solid #2b2d31;font-family:monospace;white-space:pre-wrap';
                el.textContent = help;
                const close = document.createElement('button');
                close.textContent = '✕';
                close.style.cssText = 'position:absolute;top:8px;right:8px;background:none;border:none;color:#949ba4;cursor:pointer;font-size:14px';
                close.onclick = () => el.remove();
                el.prepend(close);
                el.querySelector('button')?.after(document.createElement('br'));
                document.body.appendChild(el);
                setTimeout(() => { el.style.transition = 'opacity .3s'; el.style.opacity = '0'; setTimeout(() => el.remove(), 300); }, 30000);
            } catch (_) {}
        };
        showWebrtcHelp();
    }

    console.info('[Voice] runtime capability snapshot', {
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        vendor: navigator.vendor,
        hasMediaDevices: Boolean(navigator.mediaDevices),
        hasGetUserMedia: typeof navigator.mediaDevices?.getUserMedia === 'function',
        hasEnumerateDevices: typeof navigator.mediaDevices?.enumerateDevices === 'function',
        hasGetDisplayMedia: typeof navigator.mediaDevices?.getDisplayMedia === 'function',
        hasPermissionsApi: Boolean(navigator.permissions),
        hasRTCPeerConnection: typeof window.RTCPeerConnection === 'function',
        hasWebkitRTCPeerConnection: typeof window.webkitRTCPeerConnection === 'function',
        hasRTCSessionDescription: typeof window.RTCSessionDescription === 'function',
        hasRTCIceCandidate: typeof window.RTCIceCandidate === 'function',
        hasAudioContext: typeof window.AudioContext === 'function' || typeof window.webkitAudioContext === 'function',
        hasSetSinkId: typeof window.HTMLMediaElement?.prototype?.setSinkId === 'function',
        chromeKeys: window.chrome ? Object.keys(window.chrome) : [],
        userAgentData: navigator.userAgentData ? { platform: navigator.userAgentData.platform, mobile: navigator.userAgentData.mobile } : null,
    });

    const handleExternalAnchorClick = (event) => {
        const anchor = event.target?.closest?.('a[href]');
        if (!anchor) return;
        const href = anchor.href;
        if (!href || isDiscordUrl(href)) return;
        event.preventDefault();
        event.stopPropagation();
        event.stopImmediatePropagation?.();
        showLinkToast(href);
        openExternalUrl(href);
    };

    document.addEventListener('click', handleExternalAnchorClick, true);
    document.addEventListener('auxclick', handleExternalAnchorClick, true);
    document.addEventListener('keydown', (event) => {
        if (event.key !== 'Enter') return;
        handleExternalAnchorClick(event);
    }, true);

    const setupTitleObserver = () => {
        const el = document.querySelector('title');
        if (el) {
            new MutationObserver(() => {
                if (document.title !== lastDocumentTitle) {
                    lastDocumentTitle = document.title;
                    syncUnreadBadge();
                }
            }).observe(el, { childList: true, characterData: true, subtree: true });
        }
    };
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', setupTitleObserver);
    } else {
        setupTitleObserver();
    }

    document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible') { syncUnreadBadge(); syncPresence(); }
    });
    window.addEventListener('focus', () => { syncUnreadBadge(); syncPresence(); });
    window.addEventListener('blur', () => { syncUnreadBadge(); syncPresence(); });

    syncUnreadBadge();
    syncPresence();

    const originalOpen = window.open;
    window.open = function(url, ...args) {
        if (url) {
            try {
                const u = new URL(url, location.origin);
                const isExternal = !isDiscordUrl(u.toString())
                    && (u.protocol === 'http:' || u.protocol === 'https:' || u.protocol === 'mailto:');
                if (isExternal) {
                    showLinkToast(u.toString());
                    try { return originalOpen.call(this, u.toString(), '_blank', 'noopener,noreferrer'); }
                    catch { openExternalUrl(u.toString()); }
                    return null;
                }
            } catch {}
        }
        return originalOpen.call(this, url, ...args);
    };
})();
