#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), eframe::Error> {
    oris_mania_utils_lib::run()
}
