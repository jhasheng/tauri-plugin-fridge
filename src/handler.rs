//! `Handler` impl that forwards every event to the Tauri app as IPC events.

use tauri::{AppHandle, Emitter, Runtime};

use fridge::{DetachReason, Event, Handler};

use crate::types::{DetachedEvent, EmittedEvent};

pub(crate) struct EmitHandler<R: Runtime> {
    pub app: AppHandle<R>,
    pub capture_id: u32,
}

impl<R: Runtime> Handler for EmitHandler<R> {
    fn on_message(&mut self, evt: &Event, data: Option<&[u8]>) {
        let _ = self.app.emit(
            "fridge:event",
            EmittedEvent {
                capture_id: self.capture_id,
                event: evt.clone(),
                data_len: data.map(|d| d.len()).unwrap_or(0),
            },
        );
    }

    fn on_detached(&mut self, reason: DetachReason) {
        let _ = self.app.emit(
            "fridge:detached",
            DetachedEvent {
                capture_id: self.capture_id,
                reason: format!("{reason:?}"),
            },
        );
    }
}
