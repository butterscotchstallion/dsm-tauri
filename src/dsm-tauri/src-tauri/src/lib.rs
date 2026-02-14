use crate::disk::get_disks;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

const LOW_SPACE_THRESHOLD: f64 = 0.10;
const CHECK_SPACE_INTERVAL: u64 = 15 * 60;
mod disk;

struct AppState {
    is_low_space: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_path = setup_log_path();
    let log_builder = tauri_plugin_log::Builder::default()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
        ])
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .max_file_size(10 * 1024 * 1024)
        .level(log::LevelFilter::Info);
    if let Some(path) = log_path {
        _ = log_builder.target(Target::new(TargetKind::Folder {
            path,
            file_name: Some("app".to_string()),
        }));
    }

    // 1. Create the shared data structure
    let is_low_space = Arc::new(AtomicBool::new(false));

    // 2. Clone it so we can "move" it into the setup closure
    // while still keeping the original for the manage() call
    let is_low_for_setup = is_low_space.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default()
            .targets([
                Target::new(TargetKind::Stdout),
                Target::new(TargetKind::Webview),
                Target::new(TargetKind::Folder {
                    path: std::env::current_exe()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join("logs"),
                    file_name: Some("app".to_string()),
                }),
            ])
            .rotation_strategy(RotationStrategy::KeepSome(5))
            .max_file_size(10 * 1024 * 1024) // 10MB limit per file
            .level(log::LevelFilter::Info)
            .build())
        .manage(AppState { is_low_space })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let version = app.config().version.clone().unwrap_or_default();
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_title(&format!("Disk Space Monitor v{}", version));

            let is_low_for_loop = is_low_for_setup;

            // 1. Menu and Window Setup
            let show_i = MenuItem::with_id(app, "show", "Show Monitor", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Disk Space Monitor: Checking...")
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
            let is_low_checker = is_low_for_loop.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let disks = disk::get_disks_list();
                    let low = disks.iter().any(
                        |d| (d.available_space as f64 / d.total_space as f64) < LOW_SPACE_THRESHOLD
                    );
                    is_low_checker.store(low, Ordering::Relaxed);

                    // Update the tooltip during background check too
                    let names: Vec<String> = disk::get_low_disk_names(&disks, LOW_SPACE_THRESHOLD);

                    if names.is_empty() {
                        let _: Option<String> = Some("Disk Space Monitor: All clear".into());
                    } else {
                        let csv_names = names.join(", ");
                        let _ = tray_handle.set_tooltip(Some(format!("Low Space Warning: {}", csv_names)));
                        log::info!("Low space warning: {}", csv_names);
                    }

                    tokio::time::sleep(Duration::from_secs(CHECK_SPACE_INTERVAL)).await;
                }
            });

            let is_low_blinker = is_low_for_loop.clone();
            let tray_for_blink = tray.clone();
            tauri::async_runtime::spawn(async move {
                let mut visible = true;
                let normal_icon = app_handle.default_window_icon().unwrap().clone();

                loop {
                    let is_low = is_low_blinker.load(Ordering::Relaxed);

                    if is_low {
                        visible = !visible;
                        let _ = tray_for_blink.set_icon(if visible { Some(normal_icon.clone()) } else { None });
                    } else if !visible {
                        let _ = tray_for_blink.set_icon(Some(normal_icon.clone()));
                        visible = true;
                    }

                    // The tray icon blinks every second
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_disks, launch_disk_cleanup, get_app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running Disk Space Monitor");
}
fn setup_log_path() -> Option<std::path::PathBuf> {
    let log_path = std::env::current_exe()
        .ok()?
        .parent()?
        .join("logs");

    if let Err(e) = std::fs::create_dir_all(&log_path) {
        eprintln!("Warning: Failed to create log directory at {:?}: {}", log_path, e);
        return None;
    }

    Some(log_path)
}

#[tauri::command]
fn launch_disk_cleanup() {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cleanmgr.exe")
            .arg("/lowdisk")
            .spawn();
    }
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.config().version.clone().unwrap_or_else(|| "0.0.0".to_string())
}

#[cfg(test)]
mod tests {
    use crate::disk::{get_low_disk_names, DiskInfo};
    use crate::{AppState, LOW_SPACE_THRESHOLD};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_low_disk_threshold_detection() {
        // 1. Setup mock data
        let disks = vec![
            DiskInfo {
                name: "Healthy".into(),
                total_space: 100,
                available_space: 50,
            },
            DiskInfo {
                name: "Low".into(),
                total_space: 100,
                available_space: 5, // 5% < 10% threshold
            },
        ];

        // 2. Initialize AppState correctly
        let state = AppState {
            is_low_space: Arc::new(AtomicBool::new(false)),
        };

        // 3. Run the same logic used in the get_disks command
        let low_names: Vec<String> = get_low_disk_names(&disks, LOW_SPACE_THRESHOLD);

        // 4. Update the state
        state.is_low_space.store(!low_names.is_empty(), Ordering::Relaxed);

        // 5. Assertions
        assert_eq!(low_names.len(), 1, "Should have found exactly one low disk");
        assert_eq!(low_names[0], "Low");
        assert!(state.is_low_space.load(Ordering::Relaxed), "AtomicBool should be true");
    }
}