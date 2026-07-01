use crate::state::{Account, AppState, generate_id};
use std::io::Read;
use tauri::State;
use tauri::WebviewUrl;
use url::Url;

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
        parts.push_str(&format!(
            "(function(){{var s=document.createElement('style');s.textContent='{}';document.head.appendChild(s);}})();",
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

#[tauri::command]
pub fn open_account(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let account = {
        let config = state.config.read().map_err(|e| e.to_string())?;
        config
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .cloned()
            .ok_or_else(|| "Account not found".to_string())?
    };

    let label = format!("account_{}", account.id);
    let target_url = account
        .url
        .parse::<Url>()
        .map_err(|e| format!("Invalid URL: {e}"))?;
    let session_dir = state.sessions_dir.join(&account.id);

    std::fs::create_dir_all(&session_dir).map_err(|e| format!("{e}"))?;

    // Navigate directly to target — TLS policy and init script are set
    // synchronously during build() before the event loop runs, so the
    // first network request always has our config applied.
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(target_url.clone()),
    )
    .data_directory(session_dir)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
    .title(&account.label)
    .inner_size(1280.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .initialization_script(&build_init_script(&account.platform))
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
    })
    .build()
    .map_err(|e| format!("Failed to create window: {e}"))?;

    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::{WebViewExt, WebContextExt, TLSErrorsPolicy, SettingsExt, HardwareAccelerationPolicy};
        // ponytail: clone() so window survives with_webview() consuming self.
        #[allow(deprecated)]
        window
            .clone()
            .with_webview(move |pwv| {
                if let Some(ctx) = pwv.inner().context() {
                    ctx.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
                }
                if let Some(settings) = pwv.inner().settings() {
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
                    settings.set_enable_accelerated_2d_canvas(true);
                    settings.set_enable_developer_extras(false);
                    settings.set_enable_webrtc(true);
                    settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);
                }
            })
            .map_err(|e| format!("Failed to configure webview: {e}"))?;
    }

    // ponytail: devtools active in all builds while debugging Zalo
    window.open_devtools();

    Ok(())
}
