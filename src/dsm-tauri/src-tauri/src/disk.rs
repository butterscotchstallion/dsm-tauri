use crate::AppState;
use serde::Serialize;
use std::sync::atomic::Ordering;
use sysinfo::Disks;

#[derive(Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub total_space: u64,
    pub available_space: u64,
}

pub fn get_disks_logic() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks.iter().map(|disk| DiskInfo {
        name: disk.name().to_string_lossy().into_owned(),
        total_space: disk.total_space(),
        available_space: disk.available_space(),
    }).collect()
}

#[tauri::command]
pub fn get_disks(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Vec<DiskInfo> {
    let disks = get_disks_logic();

    let low_names: Vec<String> = disks.iter()
        .filter(|d| (d.available_space as f64 / d.total_space as f64) < 0.10)
        .map(|d| d.name.clone())
        .collect();

    // 1. Update the blinking state
    state.is_low_space.store(!low_names.is_empty(), Ordering::Relaxed);

    // 2. Update the tooltip immediately
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip: Option<String> = if low_names.is_empty() {
            Some("Disk Space Monitor: All clear".into())
        } else {
            Some(format!("Low Space Warning: {}", low_names.join(", ")))
        };
        let _ = tray.set_tooltip(tooltip);
    }

    disks
}