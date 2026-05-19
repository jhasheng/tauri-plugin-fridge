//! Wire types — what JS sends + receives.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Argument shape for `start_capture`.
#[derive(Debug, Clone, Deserialize)]
pub struct StartOptions {
    pub target: TargetSpec,
    pub script: ScriptSpec,
    /// Optional path to write a length-framed bincode capture file. When
    /// `Some`, every `fridge::Event` emitted to JS is *also* appended to
    /// this file via `fridge::record::Writer`. `None` = live-only.
    #[serde(default)]
    pub record_path: Option<PathBuf>,
}

/// How to find / spawn the target process.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetSpec {
    /// Match process by name (case-insensitive substring).
    Name { value: String },
    /// Attach by exact PID.
    Pid { value: u32 },
    /// Spawn-and-attach.
    Spawn { program: String, args: Vec<String> },
}

/// JS source or precompiled bytecode.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScriptSpec {
    Source { value: String },
    Bytes { value: Vec<u8> },
}

/// Per-capture metadata returned to JS.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureInfo {
    pub id: u32,
    pub pid: u32,
}

/// Event payload emitted on `fridge:event`.
#[derive(Debug, Clone, Serialize)]
pub struct EmittedEvent {
    pub capture_id: u32,
    pub event: fridge::Event,
    pub data_len: usize,
}

/// Event payload emitted on `fridge:detached`.
#[derive(Debug, Clone, Serialize)]
pub struct DetachedEvent {
    pub capture_id: u32,
    pub reason: String,
}

/// Recording file metadata returned by `list_recordings`.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    /// ms since UNIX epoch — JS-friendly `Date(mtime_ms)` consumable.
    pub mtime_ms: u64,
}

/// One batch streamed from `read_recording`. Batched (not per-event) to
/// keep IPC overhead linear in event count, not quadratic.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingChunk {
    pub events: Vec<fridge::Event>,
    pub read_bytes: u64,
    pub total_bytes: u64,
    pub done: bool,
}
