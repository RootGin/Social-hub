use crate::state::{self, Account, AppState, generate_id};
use std::io::Read;
use std::sync::mpsc;
use tauri::{Manager, State};
use tauri::{LogicalPosition, LogicalSize, WebviewBuilder, WebviewUrl};
use url::Url;

/// Runs `f` on the GTK main thread and blocks the calling (worker) thread
/// until it finishes, forwarding back whatever `f` returns.
///
/// All widget-mutating Tauri/WebKitGTK calls (add_child, set_position,
/// set_size, show, hide, with_webview, open_devtools) MUST happen on the
/// main thread on Linux — GTK is not thread-safe. Tauri commands that are
/// plain `fn` (not `async fn`) are dispatched onto a blocking worker thread
/// pool by default, NOT the main thread, so calling these APIs directly
/// from a command body races with GTK's own layout pass and produces
/// exactly the kind of "renders, but geometry is scrambled" bug we saw
/// (tabs positioned over the sidebar, pinned to the wrong half of the
/// window, etc). Route everything through here instead.
fn on_main_thread<F, T>(app: &tauri::AppHandle, f: F) -> Result<T, String>
where
    F: FnOnce(&tauri::AppHandle) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let app_for_main_thread = app.clone();
    app.run_on_main_thread(move || {
        let result = f(&app_for_main_thread);
        let _ = tx.send(result);
    })
    .map_err(|e| format!("Failed to schedule on main thread: {e}"))?;
    rx.recv()
        .map_err(|e| format!("Main thread task dropped: {e}"))?
}

// Platform-specific CSS fixes (like Ferdium's service.css).
// SCROLLBAR snippet is manually inlined since Rust's concat!() doesn't accept consts.
const FACEBOOK_CSS: &str = "::-webkit-scrollbar{width:6px!important}::-webkit-scrollbar-track{background:transparent!important}::-webkit-scrollbar-thumb{background:rgba(255,255,255,.15)!important;border-radius:3px!important}";
const TWITTER_CSS: &str = "::-webkit-scrollbar{width:6px!important}::-webkit-scrollbar-track{background:transparent!important}::-webkit-scrollbar-thumb{background:rgba(255,255,255,.15)!important;border-radius:3px!important}";
const INSTAGRAM_CSS: &str = "::-webkit-scrollbar{width:6px!important}::-webkit-scrollbar-track{background:transparent!important}::-webkit-scrollbar-thumb{background:rgba(255,255,255,.15)!important;border-radius:3px!important}.oYYFH{padding:0!important}.MWDvN,.oYYFH>div{max-width:100%!important}._lz6s{border-bottom:0!important}";
const ZALO_CSS: &str = "::-webkit-scrollbar{width:6px!important}::-webkit-scrollbar-track{background:transparent!important}::-webkit-scrollbar-thumb{background:rgba(255,255,255,.15)!important;border-radius:3px!important}[class*=leftbar-conversations]{height:100%!important}[class*=app-leftbar-promo]{display:none!important}";
const TIKTOK_CSS: &str = "::-webkit-scrollbar{width:6px!important}::-webkit-scrollbar-track{background:transparent!important}::-webkit-scrollbar-thumb{background:rgba(255,255,255,.15)!important;border-radius:3px!important}[class*=DivPlayerContainer]{max-width:100%!important}[data-e2e=download-dialog]{display:none!important}";

fn platform_css(platform: &str) -> &'static str {
    match platform {
        "facebook" => FACEBOOK_CSS,
        "twitter" => TWITTER_CSS,
        "instagram" => INSTAGRAM_CSS,
        "zalo" => ZALO_CSS,
        "tiktok" => TIKTOK_CSS,
        _ => "",
    }
}

/// Velocity + friction physics-based scrolling. Each wheel tick adds an instant
/// burst of speed (no lerp lag). Friction brakes it immediately when you stop.
/// Bypasses nested scrollable containers (modals, overlays) so they can scroll
/// natively — only smooth-scrolls the top-level document.
const WHEEL_SMOOTHER: &str = r#"(function(){
  if(window.__smScroll)return;window.__smScroll=true;
  function isNestedScroll(el){
    for(;el;el=el.parentElement){
      if(el===document.documentElement)return false;
      var s=getComputedStyle(el);
      if((s.overflowY==='auto'||s.overflowY==='scroll')&&el.scrollHeight>el.clientHeight)return true;
    }
    return false;
  }
  var v=0;
  var moving=false;
  window.addEventListener('wheel',function(e){
    if(Math.abs(e.deltaX)>Math.abs(e.deltaY)||e.ctrlKey)return;
    if(isNestedScroll(e.target))return;
    e.preventDefault();
    var d=e.deltaY>0?1:-1;
    v+=d*60;
    if(v>800)v=800;if(v<-800)v=-800;
    if(!moving)rAF();
    moving=true;
  },{passive:false});
  function rAF(){
    window.scrollBy(0,v);
    v*=0.82;
    if(Math.abs(v)>0.5){requestAnimationFrame(rAF)}else{v=0;moving=false}
  }
})();
"#;

/// Right-click an image → passes the URL to Rust which downloads the image
/// (bypassing CORS restrictions) and writes it to the Wayland clipboard.
/// Default context menu still appears; the image is already on the Wayland
/// clipboard by the time you click "Copy Image" (which normally fails).
const IMAGE_COPY_INTERCEPTOR: &str = r#"(function(){
  document.addEventListener('contextmenu',function(e){
    var t=e.target;
    if(t.tagName!=='IMG'||!t.src)return;
    window.__TAURI__.core.invoke('copy_image',{imageUrl:t.src}).catch(function(){});
  });
})();
"#;

/// Build the combined init script: wheel smoother + image copy interceptor +
/// platform CSS injection + fingerprint evasion.
fn build_init_script(platform: &str) -> String {
    let css = platform_css(platform);
    let mut parts = String::from(WHEEL_SMOOTHER);
    parts.push_str(IMAGE_COPY_INTERCEPTOR);
    if !css.is_empty() {
        // Guard against document.head being null at init-script time
        // (init scripts run before DOM parse completes on some pages).
        // Try immediate injection, fall back to DOMContentLoaded.
        parts.push_str(&format!(
            r#"(function(){{
              var css='{}';
              function inject(){{
                if(!document.head)return false;
                var s=document.createElement('style');
                s.textContent=css;
                document.head.appendChild(s);
                return true;
              }}
              if(!inject()){{
                document.addEventListener('DOMContentLoaded',inject,{{once:true}});
              }}
            }})();"#,
            css.replace('\'', r"\'")
        ));
    }
    parts.push_str(FINGERPRINT_SCRIPT);
    parts
}

/// Injected before any page JS runs. Overrides WebKitGTK properties so social
/// platforms see a Chrome 128 fingerprint instead of an embedded webview.
const FINGERPRINT_SCRIPT: &str = r#"(function(){
  // ── 1. window.chrome ─────────────────────────────────────────
  if (!window.chrome) window.chrome = {};
  var cr = window.chrome;
  cr.runtime = cr.runtime || {};
  cr.loadTimes = cr.loadTimes || function(){ return { requestTime: 0, startLoadTime: 0, endLoadTime: 0, finishDocumentLoadTime: 0, finishLoadTime: 0, firstPaintTime: 0, firstPaintAfterLoadTime: 0, navigationType: "other", wasFetchedViaSpdy: false, wasNpnNegotiated: false, npnNegotiatedProtocol: "http/1.1", wasAlternateProtocolAvailable: false, connectionInfo: "http/1.1" }; };
  cr.csi = cr.csi || function(){ return { onloadT: 0, startE: 0, onload: 0, pageT: 0, tran: 15 } };

  // ── 2. navigator.webdriver ───────────────────────────────────
  Object.defineProperty(Object.getPrototypeOf(navigator), "webdriver", { get: function(){return undefined;}, configurable: true });

  // ── 3. navigator.plugins ─────────────────────────────────────
  if (!navigator.plugins || navigator.plugins.length < 3) {
    (function(){
      function P(name, fn) { this.name=name; this.filename=fn; this.description=""; this.length=0; }
      P.prototype.item=function(){return null;}; P.prototype.namedItem=function(){return null;}; P.prototype.refresh=function(){};
      var a=[ new P("Chrome PDF Plugin","internal-pdf-viewer"), new P("Chrome PDF Viewer","mhjfbmdgcfjbbpaeojofohoefgiehjai"), new P("Native Client","internal-nacl-plugin") ];
      a.item=function(i){return this[i]||null;}; a.namedItem=function(n){return this.find(function(p){return p.name===n})||null;}; a.refresh=function(){};
      Object.defineProperty(navigator,"plugins",{get:function(){return a;},configurable:true});
    })();
  }

  // ── 4. navigator.userAgentData (Client Hints — big one) ──────
  if (!navigator.userAgentData) {
    var uad = {
      brands: [{brand:"Chromium",version:"128"},{brand:"Google Chrome",version:"128"},{brand:"Not;A=Brand",version:"99"}],
      mobile: false,
      platform: "Windows",
      getHighEntropyValues: function(){ return Promise.resolve({}); }
    };
    Object.defineProperty(navigator, "userAgentData", { get: function(){return uad;}, configurable: true });
  }

  // ── 5. navigator.pdfViewerEnabled (Chrome → true) ────────────
  Object.defineProperty(navigator, "pdfViewerEnabled", { get: function(){return true;}, configurable: true });

  // ── 6. navigator.languages ───────────────────────────────────
  Object.defineProperty(navigator, "languages", { get: function(){return ["en-US","en"];}, configurable: true });

  // ── 7. navigator.vendor / productSub / platform ──────────────
  Object.defineProperty(navigator, "vendor",     { get: function(){return "Google Inc.";}, configurable: true });
  Object.defineProperty(navigator, "vendorSub",  { get: function(){return "";}, configurable: true });
  Object.defineProperty(navigator, "productSub", { get: function(){return "20030107";}, configurable: true });
  Object.defineProperty(navigator, "platform",   { get: function(){return "Win32";}, configurable: true });
  Object.defineProperty(navigator, "oscpu",      { get: function(){return undefined;}, configurable: true });

  // ── 8. Hardware fingerprint ──────────────────────────────────
  Object.defineProperty(navigator, "hardwareConcurrency", { get: function(){return 8;}, configurable: true });
  Object.defineProperty(navigator, "deviceMemory",        { get: function(){return 8;}, configurable: true });
  Object.defineProperty(navigator, "maxTouchPoints",      { get: function(){return 0;}, configurable: true });

  // ── 9. navigator.connection (Chrome exposes downlink/rtt) ───
  try {
    var c = navigator.connection;
    if (c) {
      Object.defineProperty(c, "effectiveType", { get: function(){return "4g";}, configurable: true });
      Object.defineProperty(c, "downlink",      { get: function(){return 10;}, configurable: true });
      Object.defineProperty(c, "rtt",           { get: function(){return 50;}, configurable: true });
      Object.defineProperty(c, "saveData",      { get: function(){return false;}, configurable: true });
    }
  } catch(e){}

  // ── 10. WebGL vendor / renderer spoof ────────────────────────
  (function(){
    var p1 = window.WebGLRenderingContext && WebGLRenderingContext.prototype;
    var p2 = window.WebGL2RenderingContext && WebGL2RenderingContext.prototype;
    [p1,p2].forEach(function(proto){
      if (!proto) return;
      var orig = proto.getParameter;
      proto.getParameter = function(p){
        if (p === 37445) return "Google Inc. (Intel)";
        if (p === 37446) return "ANGLE (Intel, Intel(R) UHD Graphics Direct3D11 vs_5_0 ps_5_0)";
        return orig.call(this, p);
      };
    });
  })();

  // ── 11. Chrome runtime stubs ────────────────────────────────
  try {
    var r = window.chrome && window.chrome.runtime;
    if (r && !r.id) {
      r.id = "nponmmjfpdnnljmplohfhninjldbffhb";
      r.connect = function(){ return { postMessage: function(){}, onMessage: { addListener: function(){} }, disconnect: function(){}, onDisconnect: { addListener: function(){} } }; };
      r.sendMessage = function(msg, cb){ if(cb) cb(); };
    }
  } catch(e){}
})();
"#;

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, String> {
    state
        .config
        .read()
        .map(|c| c.accounts.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_account(
    state: State<'_, AppState>,
    platform: String,
    label: String,
    url: String,
) -> Result<Account, String> {
    if label.trim().is_empty() {
        return Err("Label cannot be empty".into());
    }
    if url.trim().is_empty() {
        return Err("URL cannot be empty".into());
    }

    let account = Account {
        id: generate_id(),
        platform,
        label,
        url,
        enabled: true,
    };

    let mut config = state.config.write().map_err(|e| e.to_string())?;
    config.accounts.push(account.clone());
    AppState::save_config(&state.config_path, &config)?;
    Ok(account)
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut config = state.config.write().map_err(|e| e.to_string())?;
    config.accounts.retain(|a| a.id != id);
    AppState::save_config(&state.config_path, &config)
}

#[tauri::command]
pub fn rename_account(
    state: State<'_, AppState>,
    id: String,
    label: String,
) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err("Label cannot be empty".into());
    }
    let mut config = state.config.write().map_err(|e| e.to_string())?;
    let account = config
        .accounts
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| "Account not found".to_string())?;
    account.label = label.trim().to_string();
    AppState::save_config(&state.config_path, &config)
}

/// Opens an account in a new tab within the main window (instead of a separate
/// OS window). Returns the tab label so the frontend can register the tab.
#[tauri::command]
pub fn open_account(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> Result<String, String> {
    let (account, session_dir) = {
        let config = state.config.read().map_err(|e| e.to_string())?;
        let account = config
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| "Account not found".to_string())?;
        let session_dir = state.sessions_dir.join(&account.id);
        (account, session_dir)
    };

    let tab_label = format!("tab_{}", account.id);
    let target_url = account
        .url
        .parse::<Url>()
        .map_err(|e| format!("Invalid URL: {e}"))?;

    std::fs::create_dir_all(&session_dir).map_err(|e| format!("{e}"))?;

    // If a tab for this account already exists, just switch to it
    if let Some(_existing) = app.get_webview(&tab_label) {
        return Ok(tab_label);
    }

    // Use the viewport rect emitted by the Svelte frontend via update_viewport.
    // The frontend is responsible for measuring the DOM *after* the tab bar
    // has actually been added to the layout (see App.svelte's openAccount),
    // so by the time we get here VIEWPORT should already reflect final,
    // accurate coordinates. We no longer guess — a stale/zeroed viewport is
    // a bug in the caller, not something to silently paper over with magic
    // numbers tied to a specific window size.
    let (px, py, pw, ph) = {
        let vp = state::VIEWPORT.lock().unwrap_or_else(|e| e.into_inner());
        if !vp.is_valid() {
            return Err(
                "Viewport not ready — frontend must call update_viewport with the real \
                 post-tab-bar layout before opening a tab."
                    .to_string(),
            );
        }
        (vp.x, vp.y, vp.width, vp.height)
    };

    let platform = account.platform.clone();

    // Everything below touches GTK widgets and MUST run on the main thread —
    // add_child, with_webview, and open_devtools will misbehave (silently
    // wrong position/size, or worse) if called from this command's worker
    // thread on Linux.
    on_main_thread(&app, move |app| {
        let main_window = app
            .get_window("main")
            .ok_or_else(|| "Main window not found".to_string())?;

        let wv = main_window
            .add_child(
                WebviewBuilder::new(&tab_label, WebviewUrl::External(target_url))
                    .data_directory(session_dir)
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
                    .initialization_script(&build_init_script(&platform))
                    .enable_clipboard_access()
                    // X login opens OAuth popups — allow them by default
                    .on_new_window(|_url, _features| {
                        tauri::webview::NewWindowResponse::Allow
                    })
                    // Native "Save As…" dialog on download
                    .on_download(|_webview, event| {
                        match event {
                            tauri::webview::DownloadEvent::Requested { url, destination } => {
                                let name = url
                                    .path_segments()
                                    .and_then(|s| s.last())
                                    .unwrap_or("download")
                                    .to_string();
                                if let Some(dlg) = rfd::FileDialog::new()
                                    .set_title("Save As…")
                                    .set_file_name(&name)
                                    .save_file()
                                {
                                    *destination = dlg;
                                    true
                                } else {
                                    false
                                }
                            }
                            _ => true,
                        }
                    }),
                LogicalPosition::new(px, py),
                LogicalSize::new(pw.max(200.0), ph.max(100.0)),
            )
            .map_err(|e| format!("Failed to create tab: {e}"))?;

        // WebKitGTK-specific configuration
        #[cfg(target_os = "linux")]
        {
            use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
            use gtk::prelude::{ContainerExt, FixedExt, WidgetExt};
            let _ = wv.with_webview(move |pwv| {
                let webview = pwv.inner();
                if let Some(settings) = WebViewExt::settings(&webview) {
                    settings.set_javascript_can_open_windows_automatically(true);
                    settings.set_javascript_can_access_clipboard(true);
                    settings.set_enable_webgl(true);
                    settings.set_enable_webaudio(true);
                    settings.set_enable_media(true);
                    settings.set_enable_mediasource(true);
                    settings.set_enable_media_capabilities(true);
                    settings.set_enable_media_stream(true);
                    settings.set_enable_encrypted_media(true);
                    settings.set_enable_fullscreen(true);
                    settings.set_enable_page_cache(true);
                    settings.set_enable_smooth_scrolling(false);
                    settings.set_enable_site_specific_quirks(true);
                    settings.set_enable_webrtc(true);
                    settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);
                }
                // Reparent: remove from GtkBox, add to our GtkFixed.
                // The fixed is positioned at the viewport rect by
                // update_viewport, so the webview goes at (0,0)
                // within the fixed. This is the only way to get
                // correct positioning on Wayland (wry's set_bounds
                // is a no-op for GtkBox children).
                use std::sync::atomic::Ordering;
                use webkit2gtk::glib::translate::FromGlibPtrNone;
                use webkit2gtk::glib::Cast;
                if let Some(parent) = webview.parent() {
                    parent
                        .dynamic_cast_ref::<gtk::Container>()
                        .map(|c| c.remove(&webview));
                }
                let ptr = state::FIXED_CONTAINER.load(Ordering::SeqCst);
                if !ptr.is_null() {
                    let fixed = unsafe {
                        <gtk::Fixed as FromGlibPtrNone<*mut gtk::ffi::GtkFixed>>::from_glib_none(
                            ptr as *mut gtk::ffi::GtkFixed,
                        )
                    };
                    fixed.put(&webview, 0, 0);
                }
                webview.set_size_request(
                    (pw.max(200.0)) as i32, (ph.max(100.0)) as i32,
                );
            });
        }

        // devtools always on while debugging
        wv.open_devtools();

        Ok(tab_label)
    })
}

/// Stores the latest content-area rectangle from the Svelte layout and
/// repositions all existing tab webviews to match.
#[tauri::command]
pub fn update_viewport(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    {
        let mut vp = state::VIEWPORT.lock().unwrap_or_else(|e| e.into_inner());
        vp.x = x;
        vp.y = y;
        vp.width = width;
        vp.height = height;
    }

    on_main_thread(&app, move |_app| {
        // Move the GtkFixed overlay child to the new viewport rect.
        // Tab webviews inside the GtkFixed are positioned at (0,0)
        // relative to the fixed, so they move with it automatically.
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::WidgetExt;
            use webkit2gtk::glib::translate::FromGlibPtrNone;
            use std::sync::atomic::Ordering;
            let ptr = state::FIXED_CONTAINER.load(Ordering::SeqCst);
            if !ptr.is_null() {
                let fixed = unsafe {
                    <gtk::Fixed as FromGlibPtrNone<*mut gtk::ffi::GtkFixed>>::from_glib_none(
                        ptr as *mut gtk::ffi::GtkFixed,
                    )
                };
                fixed.set_margin_start(x as i32);
                fixed.set_margin_top(y as i32);
                fixed.set_size_request(
                    (width.max(200.0)) as i32,
                    (height.max(100.0)) as i32,
                );
            }
        }
        // Also update Tauri's internal webview state for consistency
        // (keeps wry's bounds tracking in sync even though the GTK
        //  widget is managed by our fixed container).
        if let Some(win) = _app.get_window("main") {
            for wv in win.webviews() {
                if wv.label().starts_with("tab_") {
                    let _ = wv.set_size(LogicalSize::new(
                        width.max(200.0), height.max(100.0),
                    ));
                }
            }
        }
        Ok(())
    })
}

/// Closes a tab webview by label.
#[tauri::command]
pub fn close_tab(app: tauri::AppHandle, tab_label: String) -> Result<(), String> {
    on_main_thread(&app, move |app| {
        if let Some(wv) = app.get_webview(&tab_label) {
            wv.close().map_err(|e| format!("Close failed: {e}"))
        } else {
            Ok(())
        }
    })
}

/// Switches to the given tab — hides all others, shows the target.
#[tauri::command]
pub fn switch_tab(app: tauri::AppHandle, tab_label: String) -> Result<(), String> {
    on_main_thread(&app, move |app| {
        let main_window = app
            .get_window("main")
            .ok_or_else(|| "Main window not found".to_string())?;
        for wv in main_window.webviews() {
            let label = wv.label().to_string();
            if label.starts_with("tab_") {
                if label == tab_label {
                    wv.show().map_err(|e| format!("Show failed: {e}"))?;
                } else {
                    wv.hide().map_err(|e| format!("Hide failed: {e}"))?;
                }
            }
        }
        Ok(())
    })
}

/// Downloads an image from the given URL (bypassing browser CORS) and writes
/// it to the system clipboard. The MIME type is derived from the HTTP response
/// Content-Type header so the clipboard carries the correct image format.
#[tauri::command]
pub fn copy_image(image_url: String) -> Result<(), String> {
    let resp = ureq::get(&image_url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
        )
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let mime = resp
        .header("content-type")
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .unwrap_or_else(|| "image/png".to_string());

    let mut bytes: Vec<u8> = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read body: {e}"))?;

    wl_clipboard_rs::copy::Options::new()
        .clipboard(wl_clipboard_rs::copy::ClipboardType::Regular)
        .foreground(true)
        .clone()
        .copy_multi(vec![wl_clipboard_rs::copy::MimeSource {
            source: wl_clipboard_rs::copy::Source::Bytes(bytes.into_boxed_slice()),
            mime_type: wl_clipboard_rs::copy::MimeType::Specific(mime),
        }])
        .map_err(|e| format!("Clipboard: {e}"))?;

    Ok(())
}
