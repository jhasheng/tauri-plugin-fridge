//! `#[tauri::command]` entry points.

use tauri::{AppHandle, Runtime, State};

use fridge::{Capture, Target};

use crate::handler::EmitHandler;
use crate::state::FridgeState;
use crate::types::{CaptureInfo, ScriptSpec, StartOptions, TargetSpec};

#[tauri::command]
pub(crate) async fn compile_script(source: String) -> Result<Vec<u8>, String> {
    fridge::compile_script(&source).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn start_capture<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, FridgeState>,
    opts: StartOptions,
) -> Result<CaptureInfo, String> {
    let id = state.alloc_id();
    let target = into_fridge_target(opts.target);

    let mut builder = Capture::builder().target(target);
    builder = match opts.script {
        ScriptSpec::Source { value } => builder.script(value),
        ScriptSpec::Bytes { value } => builder.script_bytes(value),
    };

    let emit = EmitHandler { app: app.clone(), capture_id: id };
    let handle = builder.start(emit).map_err(|e| e.to_string())?;
    let pid = handle.pid();

    state
        .captures
        .lock()
        .map_err(|e| format!("captures mutex poisoned: {e}"))?
        .insert(id, handle);

    Ok(CaptureInfo { id, pid })
}

#[tauri::command]
pub(crate) async fn stop_capture(
    state: State<'_, FridgeState>,
    id: u32,
) -> Result<(), String> {
    let handle = state
        .captures
        .lock()
        .map_err(|e| format!("captures mutex poisoned: {e}"))?
        .remove(&id)
        .ok_or_else(|| format!("no capture with id {id}"))?;
    handle.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_captures(
    state: State<'_, FridgeState>,
) -> Result<Vec<CaptureInfo>, String> {
    let captures = state
        .captures
        .lock()
        .map_err(|e| format!("captures mutex poisoned: {e}"))?;
    Ok(captures
        .iter()
        .map(|(id, h)| CaptureInfo { id: *id, pid: h.pid() })
        .collect())
}

fn into_fridge_target(spec: TargetSpec) -> Target {
    match spec {
        TargetSpec::Name { value } => Target::name(value),
        TargetSpec::Pid { value } => Target::pid(value),
        TargetSpec::Spawn { program, args } => {
            let argv: Vec<&str> = args.iter().map(String::as_str).collect();
            Target::spawn(program, &argv)
        }
    }
}
