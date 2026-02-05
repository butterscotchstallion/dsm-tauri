use sysinfo::Disks;
fn get_local_disks() -> Disks {
    let disks: Disks = Disks::new_with_refreshed_list();
    disks
}

fn get_free_disk_space_percentage(written_bytes: i64, total_bytes: i64) -> i32 {
    if total_bytes == 0 {
        return 0;
    }
    let free_space: i64 = total_bytes - written_bytes;
    (free_space * 100 / total_bytes) as i32
}

fn get_available_disk_space_percentage(written_bytes: i64, total_bytes: i64) -> i32 {
    if total_bytes == 0 {
        return 0;
    }
    let free_space: i64 = total_bytes - written_bytes;
    ((free_space / total_bytes) * 100) as i32
}

fn get_low_disk_space_drives(disks: Disks, low_space_threshold_percentage: i32) -> Vec<String> {
    let mut low_disk_space_drives: Vec<String> = Vec::new();
    for disk in &disks {
        let written_bytes: i64 = (disk.total_space() - disk.available_space()) as i64;
        let free_disk_space_percentage: i32 = get_free_disk_space_percentage(written_bytes, disk.total_space() as i64);
        if free_disk_space_percentage <= low_space_threshold_percentage {
            low_disk_space_drives.push(format!("{:?}", disk.name()));
        }
    }
    low_disk_space_drives
}