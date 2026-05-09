// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let start_minimized = std::env::args()
        .any(|a| a.eq_ignore_ascii_case("/minimized") || a.eq_ignore_ascii_case("--minimized"));

    dsm_tauri_lib::run(start_minimized);
}
