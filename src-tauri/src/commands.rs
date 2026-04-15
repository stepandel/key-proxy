use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex;

use crate::config::{ConfigStore, Rule};
use crate::keychain;
use crate::network;
use crate::proxy::cert::{self as certmod, CertStore};
use crate::proxy::{self, ProxyHandle};
use crate::stats::{Stats, StatsSnapshot};

pub struct AppState {
    pub config: Arc<ConfigStore>,
    pub certs: Arc<CertStore>,
    pub stats: Arc<Stats>,
    pub proxy: Mutex<Option<ProxyHandle>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuleInput {
    pub domain: String,
    pub label: String,
    pub header_name: String,
    pub credential: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProxyStatus {
    pub active: bool,
    pub port: u16,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_rules(state: State<'_, AppState>) -> Vec<Rule> {
    state.config.rules()
}

#[tauri::command]
pub fn add_rule(state: State<'_, AppState>, rule: RuleInput) -> Result<Rule, String> {
    let r = state
        .config
        .add_rule(rule.domain.clone(), rule.label, rule.header_name)
        .map_err(|e| e.to_string())?;
    if let Some(cred) = rule.credential {
        keychain::set_credential(&rule.domain, &cred).map_err(|e| e.to_string())?;
    }
    Ok(r)
}

#[tauri::command]
pub fn update_rule(
    state: State<'_, AppState>,
    id: String,
    rule: RuleInput,
) -> Result<(), String> {
    state
        .config
        .update_rule(&id, rule.domain.clone(), rule.label, rule.header_name)
        .map_err(|e| e.to_string())?;
    if let Some(cred) = rule.credential {
        keychain::set_credential(&rule.domain, &cred).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_rule(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let removed = state.config.delete_rule(&id).map_err(|e| e.to_string())?;
    if let Some(r) = removed {
        keychain::delete_credential(&r.domain).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_rule(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    state
        .config
        .toggle_rule(&id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_proxy_status(state: State<'_, AppState>) -> Result<ProxyStatus, String> {
    let active = state.proxy.lock().await.is_some();
    Ok(ProxyStatus {
        active,
        port: state.config.port(),
        error: None,
    })
}

#[tauri::command]
pub async fn set_proxy_active(
    app: tauri::AppHandle,
    active: bool,
) -> Result<ProxyStatus, String> {
    let state = app.state::<AppState>();
    let mut guard = state.proxy.lock().await;
    let port = state.config.port();

    if active {
        if guard.is_none() {
            let handle = proxy::start(
                port,
                state.config.clone(),
                state.certs.clone(),
                state.stats.clone(),
            )
            .await
            .map_err(|e| e.to_string())?;
            *guard = Some(handle);
            network::enable_proxy("127.0.0.1", port).map_err(|e| e.to_string())?;
        }
    } else if let Some(h) = guard.take() {
        let _ = network::disable_proxy();
        h.shutdown().await;
    }

    Ok(ProxyStatus {
        active: guard.is_some(),
        port,
        error: None,
    })
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> StatsSnapshot {
    state.stats.snapshot()
}

#[tauri::command]
pub fn get_ca_trusted() -> bool {
    certmod::is_ca_trusted()
}

#[tauri::command]
pub fn trust_ca(state: State<'_, AppState>) -> Result<(), String> {
    let path = state.certs.ca_cert_path().map_err(|e| e.to_string())?;
    certmod::trust_ca_via_security_cli(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn untrust_ca() -> Result<(), String> {
    certmod::untrust_ca().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_ca_cert(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let pem = state.certs.ca_cert_pem();
    std::fs::write(path, pem).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn regenerate_ca(state: State<'_, AppState>) -> Result<(), String> {
    // If proxy is running, restart it so cached server configs are rebuilt.
    state.certs.regenerate().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_port(state: State<'_, AppState>) -> u16 {
    state.config.port()
}

#[tauri::command]
pub async fn set_port(app: tauri::AppHandle, port: u16) -> Result<(), String> {
    let state = app.state::<AppState>();
    let was_active = state.proxy.lock().await.is_some();
    if was_active {
        set_proxy_active(app.clone(), false).await?;
    }
    state.config.set_port(port).map_err(|e| e.to_string())?;
    if was_active {
        set_proxy_active(app, true).await?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
