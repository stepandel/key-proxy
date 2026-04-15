pub mod commands;
pub mod config;
pub mod keychain;
pub mod network;
pub mod proxy;
pub mod stats;

use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent, WindowEvent};
use tokio::sync::Mutex;

use crate::commands::AppState;
use crate::config::ConfigStore;
use crate::proxy::cert::CertStore;
use crate::stats::Stats;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Install default rustls crypto provider (ring) once.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let config = Arc::new(ConfigStore::new());
    let certs = match CertStore::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("failed to init CertStore: {e}");
            std::process::exit(1);
        }
    };
    let stats = Arc::new(Stats::new());

    let state = AppState {
        config,
        certs,
        stats,
        proxy: Mutex::new(None),
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_rules,
            commands::add_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::toggle_rule,
            commands::get_proxy_status,
            commands::set_proxy_active,
            commands::get_stats,
            commands::get_ca_trusted,
            commands::trust_ca,
            commands::untrust_ca,
            commands::export_ca_cert,
            commands::regenerate_ca,
            commands::get_port,
            commands::set_port,
            commands::open_settings,
            commands::quit_app,
        ])
        .setup(|app| {
            build_tray(app.handle())?;
            // Hide dock icon on macOS — menu bar app
            #[cfg(target_os = "macos")]
            {
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide rather than quit
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build tauri app");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { .. } = event {
            let _ = crate::network::disable_proxy();
            // Best-effort stop proxy
            let state = app_handle.state::<AppState>();
            tauri::async_runtime::block_on(async move {
                if let Some(h) = state.proxy.lock().await.take() {
                    h.shutdown().await;
                }
            });
        }
    });
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Toggle Proxy", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit KeyProxy", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &toggle, &quit])?;

    let icon = default_tray_icon();

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                show_main(app);
            }
            "settings" => {
                if let Some(w) = app.get_webview_window("settings") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    let active = state.proxy.lock().await.is_some();
                    let _ = commands::set_proxy_active(app.clone(), !active).await;
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn default_tray_icon() -> Image<'static> {
    // 1x1 transparent PNG — user can replace with real icon.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    Image::from_bytes(PNG).expect("tray icon")
}
