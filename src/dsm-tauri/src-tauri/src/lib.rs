use crate::disk::get_disks;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

const LOW_SPACE_THRESHOLD: f64 = 0.10;
const MINUTES_IN_A_DAY: i32 = 1440;
const CHECK_INTERVAL_DEFAULT_MINUTES: i32 = 15;
mod disk;

struct AppState {
    is_low_space: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let is_low_space = Arc::new(AtomicBool::new(false));
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default()
            .targets([
                Target::new(TargetKind::Stdout),
                Target::new(TargetKind::Webview),
                // LogDir maps to %APPDATA%/{identifier}/logs on Windows.
                // This directory is always writable by the user.
                Target::new(TargetKind::LogDir { file_name: Some("app".to_string()) }),
            ])
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    message
                ))
            })
            .rotation_strategy(RotationStrategy::KeepSome(5))
            .max_file_size(10 * 1024 * 1024) // 10MB
            .level(log::LevelFilter::Info)
            .build())
        .manage(AppState { is_low_space: is_low_space.clone() })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let version = app.config().version.clone().unwrap_or_default();
            let window = app.get_webview_window("main").unwrap();
            let app_name_and_version = format!("Disk Space Monitor v{}", version);
            let _ = window.set_title(&app_name_and_version);

            log::info!("Starting {}", app_name_and_version);

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
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            // 3. Background Loop for Tooltips
            let tray_handle = tray.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    log::info!("Checking disk space...");
                    let mut check_space_interval_minutes: u64 = MINUTES_IN_A_DAY as u64;
                    let disks = disk::get_disks_list();
                    let names: Vec<String> = disk::get_low_disk_names(&disks, LOW_SPACE_THRESHOLD);

                    // Update the tooltip during background check too
                    if names.is_empty() {
                        let _: Option<String> = Some("Disk Space Monitor: All clear".into());
                    } else {
                        let csv_names = names.join(", ");
                        let _ = tray_handle.set_tooltip(Some(format!("Low Space Warning: {}", csv_names)));
                        check_space_interval_minutes = CHECK_INTERVAL_DEFAULT_MINUTES as u64;
                        log::info!("Low space warning: {} - setting interval to {}", csv_names, check_space_interval_minutes);
                    }

                    tokio::time::sleep(Duration::from_mins(check_space_interval_minutes)).await;
                }
            });

            // Handle blinking
            let is_low_blinker = is_low_space;
            let tray_for_blink = tray;
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
    fn test_low_space_state_lifecycle() {
        // 1. Initialize State
        let state = AppState {
            is_low_space: Arc::new(AtomicBool::new(false)),
        };

        // 2. Simulate Scenario: One disk is low
        let disks_with_low = vec![
            DiskInfo { name: "C:".into(), total_space: 100, available_space: 5 }, // 5% < 10%
        ];

        let low_names = get_low_disk_names(&disks_with_low, LOW_SPACE_THRESHOLD);
        state.is_low_space.store(!low_names.is_empty(), Ordering::Relaxed);

        assert!(state.is_low_space.load(Ordering::Relaxed), "Blinking should be ENABLED when a disk is low");

        // 3. Simulate Scenario: Drive space is freed up
        let disks_healthy = vec![
            DiskInfo { name: "C:".into(), total_space: 100, available_space: 50 }, // 50% > 10%
        ];

        let low_names_cleared = get_low_disk_names(&disks_healthy, LOW_SPACE_THRESHOLD);
        state.is_low_space.store(!low_names_cleared.is_empty(), Ordering::Relaxed);

        assert!(!state.is_low_space.load(Ordering::Relaxed), "Blinking should be DISABLED when no disks are low");
    }

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