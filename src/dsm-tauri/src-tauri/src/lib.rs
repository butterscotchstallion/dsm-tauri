use crate::disk::get_disks;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use sysinfo::Disks;
use tauri::Manager;

mod disk;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 1. Menu and Window Setup
            let show_i = MenuItem::with_id(app, "show", "Show Monitor", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // 2. Hide-on-close logic
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            // 3. Background Loop for Tooltips and Blinking
            let tray_handle = tray.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut visible = true;
                let mut low_disk_names: Vec<String> = Vec::new();
                let normal_icon = app_handle.default_window_icon().unwrap().clone();
                let mut last_check = std::time::Instant::now() - Duration::from_secs(15 * 60);

                loop {
                    if last_check.elapsed() >= Duration::from_secs(15 * 60) {
                        let disks = Disks::new_with_refreshed_list();
                        low_disk_names = disks.iter()
                            .filter(|d| (d.available_space() as f64 / d.total_space() as f64) < 0.10)
                            .map(|d| d.name().to_string_lossy().into_owned())
                            .collect();

                        if low_disk_names.is_empty() {
                            let tooltip: Option<String> = Some("Disk Space Monitor: No disks with low space".into());
                            let _ = tray_handle.set_tooltip(tooltip);
                        } else {
                            let msg: String = format!("Low Space Warning: {}", low_disk_names.join(", "));
                            let tooltip: Option<String> = Some(msg);
                            let _ = tray_handle.set_tooltip(tooltip);
                        }

                        last_check = std::time::Instant::now();
                    }

                    if !low_disk_names.is_empty() {
                        // Toggle visibility for blinking
                        visible = !visible;
                        let _ = tray_handle.set_icon(if visible { Some(normal_icon.clone()) } else { None });
                    } else {
                        // FORCE logic: If no disks are low, ensure icon is visible and reset state
                        if !visible {
                            let _ = tray_handle.set_icon(Some(normal_icon.clone()));
                            visible = true;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_disks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
