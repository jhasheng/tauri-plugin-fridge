const COMMANDS: &[&str] = &[
    "compile_script",
    "start_capture",
    "stop_capture",
    "list_captures",
    "list_recordings",
    "read_recording",
    "reload_script",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
