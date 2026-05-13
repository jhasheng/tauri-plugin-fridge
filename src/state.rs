//! Shared plugin state — a map of live `CaptureHandle`s keyed by an ID we
//! mint per `start_capture` call.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use fridge::CaptureHandle;

#[derive(Default)]
pub(crate) struct FridgeState {
    next_id: AtomicU32,
    pub(crate) captures: Mutex<HashMap<u32, CaptureHandle>>,
}

impl FridgeState {
    pub(crate) fn alloc_id(&self) -> u32 {
        // Start at 1 so a 0 returned to JS is unambiguously "uninitialized".
        self.next_id.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
}
