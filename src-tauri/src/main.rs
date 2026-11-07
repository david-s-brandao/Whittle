#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    whittle_lib::platform::apply_gsettings_workaround();
    whittle_lib::run()
}
