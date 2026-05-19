//! `#[tauri::command]` entry points.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime, State};

use fridge::record::{list_captures as fridge_list_captures, read_iter, Writer};
use fridge::{Capture, Event, Target};

use crate::handler::{EmitHandler, SharedWriter};
use crate::state::FridgeState;
use crate::types::{
    CaptureInfo, RecordingChunk, RecordingFile, ScriptSpec, StartOptions, TargetSpec,
};
use crate::RECORD_TAG;

/// File extension used for recordings — paired with `RECORD_TAG`.
const RECORDING_EXT: &str = "bin";
/// Event batch size streamed via `read_recording`. 64 keeps IPC traffic
/// modest while still giving the UI smooth render-as-you-go.
const READ_BATCH: usize = 64;

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

    let writer = match opts.record_path {
        Some(p) => Some(open_writer(&p)?),
        None => None,
    };

    let emit = EmitHandler {
        app: app.clone(),
        capture_id: id,
        writer,
    };
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
pub(crate) async fn stop_capture(state: State<'_, FridgeState>, id: u32) -> Result<(), String> {
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
        .map(|(id, h)| CaptureInfo {
            id: *id,
            pid: h.pid(),
        })
        .collect())
}

/// Hot-reload the running script on capture `id` from a disk path.
/// `.js` → source, anything else → bytecode bytes — same dispatch as
/// `fridge::CaptureBuilder::script_from_disk`.
#[tauri::command]
pub(crate) async fn reload_script(
    state: State<'_, FridgeState>,
    id: u32,
    path: PathBuf,
) -> Result<(), String> {
    let captures = state
        .captures
        .lock()
        .map_err(|e| format!("captures mutex poisoned: {e}"))?;
    let handle = captures
        .get(&id)
        .ok_or_else(|| format!("no capture with id {id}"))?;
    handle.reload_from_disk(&path).map_err(|e| e.to_string())
}

/// List `.bin` recordings in `dir` (mtime DESC). Empty when the dir is
/// missing — same semantics as `fridge::record::list_captures`.
#[tauri::command]
pub(crate) async fn list_recordings(dir: PathBuf) -> Result<Vec<RecordingFile>, String> {
    let files = fridge_list_captures(&dir, RECORDING_EXT).map_err(|e| e.to_string())?;
    Ok(files
        .into_iter()
        .map(|f| RecordingFile {
            name: f.name,
            size: f.size,
            mtime_ms: f
                .mtime
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            path: f.path,
        })
        .collect())
}

/// Stream a recording back to JS in `READ_BATCH`-sized chunks via a
/// Tauri `Channel`. Resolves once the final `done: true` chunk has
/// been sent (or an error is hit).
#[tauri::command]
pub(crate) async fn read_recording(
    path: PathBuf,
    channel: Channel<RecordingChunk>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || stream_recording(&path, &channel))
        .await
        .map_err(|e| format!("join error: {e}"))?
}

fn stream_recording(
    path: &std::path::Path,
    channel: &Channel<RecordingChunk>,
) -> Result<(), String> {
    let mut iter = read_iter::<Event>(path, RECORD_TAG).map_err(|e| e.to_string())?;
    let total = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut buf: Vec<Event> = Vec::with_capacity(READ_BATCH);

    loop {
        match iter.next() {
            Some(Ok(evt)) => {
                buf.push(evt);
                if buf.len() >= READ_BATCH {
                    flush_chunk(channel, &mut buf, iter.bytes_read(), total, false)?;
                }
            }
            Some(Err(e)) => return Err(e.to_string()),
            None => {
                flush_chunk(channel, &mut buf, iter.bytes_read(), total, true)?;
                return Ok(());
            }
        }
    }
}

fn flush_chunk(
    channel: &Channel<RecordingChunk>,
    buf: &mut Vec<Event>,
    read_bytes: u64,
    total_bytes: u64,
    done: bool,
) -> Result<(), String> {
    let events = std::mem::take(buf);
    channel
        .send(RecordingChunk {
            events,
            read_bytes,
            total_bytes,
            done,
        })
        .map_err(|e| format!("channel send: {e}"))
}

fn open_writer(path: &std::path::Path) -> Result<SharedWriter, String> {
    let w = Writer::<Event>::create(path.to_path_buf(), RECORD_TAG).map_err(|e| e.to_string())?;
    Ok(Arc::new(Mutex::new(Some(w))))
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
