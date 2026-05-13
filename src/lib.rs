//! Tauri 2 plugin wrapping [`fridge`].
//!
//! Mount in your `tauri::Builder`:
//!
//! ```ignore
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_fridge::init())
//!     .run(tauri::generate_context!())
//!     .expect("run");
//! ```
//!
//! Then from JS:
//!
//! ```ts
//! import { compileScript, startCapture, onFridgeEvent } from 'tauri-plugin-fridge';
//!
//! const bc = await compileScript(jsSource);
//! const { id, pid } = await startCapture({
//!   target: { kind: 'name', value: 'Weixin.exe' },
//!   script: { kind: 'bytes', value: bc },
//! });
//! const unlisten = await onFridgeEvent(e => {
//!   if (e.capture_id === id) console.log(e.event);
//! });
//! ```

mod commands;
mod handler;
mod state;
mod types;

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use types::{CaptureInfo, ScriptSpec, StartOptions, TargetSpec};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("fridge")
        .invoke_handler(tauri::generate_handler![
            commands::compile_script,
            commands::start_capture,
            commands::stop_capture,
            commands::list_captures,
        ])
        .setup(|app, _api| {
            app.manage(state::FridgeState::default());
            Ok(())
        })
        .build()
}
