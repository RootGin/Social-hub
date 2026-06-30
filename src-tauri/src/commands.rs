use crate::state::{Account, AppState, generate_id};
use tauri::State;
use tauri::WebviewUrl;
use url::Url;

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

    // ponytail: about:blank first, then navigate after TLS policy is applied.
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::External(Url::parse("about:blank").unwrap()),
    )
    .data_directory(session_dir)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
    .title(&account.label)
    .inner_size(1280.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .build()
    .map_err(|e| format!("Failed to create window: {e}"))?;

    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::{WebViewExt, WebContextExt, TLSErrorsPolicy, SettingsExt};
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
                }
            })
            .map_err(|e| format!("Failed to configure webview: {e}"))?;
    }

    window
        .navigate(target_url)
        .map_err(|e| format!("Failed to navigate: {e}"))?;

    // ponytail: devtools active in all builds while debugging Zalo
    window.open_devtools();

    Ok(())
}
