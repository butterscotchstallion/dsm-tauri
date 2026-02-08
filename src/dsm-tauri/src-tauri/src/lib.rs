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
                let mut is_low = false; // Store the state
                let normal_icon = app_handle.default_window_icon().unwrap().clone();
                let mut last_check = std::time::Instant::now() - Duration::from_secs(15 * 60);

                loop {
                    // Only refresh disk info every 15 minutes
                    if last_check.elapsed() >= Duration::from_secs(15 * 60) {
                        let disks = Disks::new_with_refreshed_list();
                        is_low = disks.iter().any(|d| {
                            let ratio = d.available_space() as f64 / d.total_space() as f64;
                            ratio < 0.10
                        });
                        last_check = std::time::Instant::now();
                    }

                    if is_low {
                        visible = !visible;
                        let _ = tray_handle.set_icon(if visible { Some(normal_icon.clone()) } else { None });
                    } else if !visible {
                        let _ = tray_handle.set_icon(Some(normal_icon.clone()));
                        visible = true;
                    }

                    // Blink interval stays fast, but disk check above is slow
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![disk::get_disks])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
