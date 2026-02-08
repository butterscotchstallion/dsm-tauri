use crate::disk::get_disks;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use sysinfo::Disks;

mod disk;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Background thread for the "Blinking" effect
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

                        // Collect names of disks with < 10% space
                        low_disk_names = disks.iter()
                            .filter(|d| {
                                let ratio = d.available_space() as f64 / d.total_space() as f64;
                                ratio < 0.10
                            })
                            .map(|d| d.name().to_string_lossy().into_owned())
                            .collect();

                        // Update tooltip based on status
                        if low_disk_names.is_empty() {
                            let _ = tray_handle.set_tooltip(Some("Disk Space Monitor: All clear".to_string()));
                        } else {
                            let msg = format!("Low Space Warning: {}", low_disk_names.join(", "));
                            let _ = tray_handle.set_tooltip(Some(msg));
                        }

                        last_check = std::time::Instant::now();
                    }

                    if !low_disk_names.is_empty() {
                        visible = !visible;
                        let _ = tray_handle.set_icon(if visible { Some(normal_icon.clone()) } else { None });
                    } else if !visible {
                        let _ = tray_handle.set_icon(Some(normal_icon.clone()));
                        visible = true;
                    }

                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![disk::get_disks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
