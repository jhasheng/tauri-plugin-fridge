//! `Handler` impl that forwards every event to the Tauri app as IPC events,
//! and (optionally) appends the same event + data buffer to a
//! `fridge::record::EventWriter` for offline replay.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Runtime};

use fridge::record::EventWriter;
use fridge::{DetachReason, Event, Handler};

use crate::types::{DetachedEvent, EmittedEvent};

/// Shared recording sink. Wrapped in `Arc<Mutex<Option<_>>>` so the
/// handler can null it out on first append failure without dropping the
/// live capture — losing a recording is recoverable; losing the capture
/// is not.
pub(crate) type SharedWriter = Arc<Mutex<Option<EventWriter>>>;

pub(crate) struct EmitHandler<R: Runtime> {
    pub app: AppHandle<R>,
    pub capture_id: u32,
    pub writer: Option<SharedWriter>,
}

impl<R: Runtime> Handler for EmitHandler<R> {
    fn on_message(&mut self, evt: &Event, data: Option<&[u8]>) {
        let _ = self.app.emit(
            "fridge:event",
            EmittedEvent {
                capture_id: self.capture_id,
                event: evt.clone(),
                data: data.map(<[u8]>::to_vec),
            },
        );
        if let Some(shared) = &self.writer {
            if let Ok(mut guard) = shared.lock() {
                let poison = match guard.as_mut() {
                    Some(w) => w.append(evt, data).err(),
                    None => None,
                };
                if let Some(e) = poison {
                    // First failure: log + drop the writer. Subsequent
                    // events bypass the locked None branch in O(1).
                    eprintln!(
                        "tauri-plugin-fridge: recording append failed (capture {}): {e}",
                        self.capture_id
                    );
                    *guard = None;
                }
            }
        }
    }

    fn on_detached(&mut self, reason: DetachReason) {
        // Drop the recording sink first so `BufWriter`'s drop flushes
        // before we emit detached — readers polling the file see a
        // complete tail by the time the JS side learns the capture ended.
        if let Some(shared) = self.writer.take() {
            if let Ok(mut guard) = shared.lock() {
                *guard = None;
            }
        }
        let _ = self.app.emit(
            "fridge:detached",
            DetachedEvent {
                capture_id: self.capture_id,
                reason: format!("{reason:?}"),
            },
        );
    }
}
