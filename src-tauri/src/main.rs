// Hide the console window in release builds. In debug we keep it, because the
// school's power cuts and printer problems are much easier to diagnose with
// stderr visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tariqi_lib::run()
}
