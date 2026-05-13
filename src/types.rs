//! Wire types — what JS sends + receives.

use serde::{Deserialize, Serialize};

/// Argument shape for `start_capture`.
#[derive(Debug, Clone, Deserialize)]
pub struct StartOptions {
    pub target: TargetSpec,
    pub script: ScriptSpec,
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
