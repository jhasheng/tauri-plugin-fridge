const COMMANDS: &[&str] = &[
    "compile_script",
    "start_capture",
    "stop_capture",
    "list_captures",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
