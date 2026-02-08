use sysinfo::Disks;

use serde::Serialize;

#[derive(Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub total_space: u64,
    pub available_space: u64,
}

#[tauri::command]
pub fn get_disks() -> Vec<DiskInfo> {
    let disks: Disks = Disks::new_with_refreshed_list();
    disks.iter().map(|disk| DiskInfo {
        name: disk.name().to_string_lossy().into_owned(),
        total_space: disk.total_space(),
        available_space: disk.available_space(),
    }).collect()
}
