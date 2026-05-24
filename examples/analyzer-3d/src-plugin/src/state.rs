//! Plugin state shared by the audio thread, GUI, and host.
//!
//! This module holds only the source of truth for values and the minimal operations
//! needed to prevent inconsistency. Delivering changes to the GUI and notifying the host
//! of edits are the responsibility of `gui.rs` and `commands.rs`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use atomic_float::AtomicF32;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Example of editor state that is saved with the project but never read from the audio thread.
/// In a real product this category includes things like IR paths, track colours, and
/// editor-only preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EditorPage {
    Spectrogram,
    Imager,
}

impl Default for EditorPage {
    fn default() -> Self {
        Self::Spectrogram
    }
}

impl EditorPage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Spectrogram => "spectrogram",
            Self::Imager => "imager",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "spectrogram" => Some(Self::Spectrogram),
            "imager" => Some(Self::Imager),
            _ => None,
        }
    }
}

/// Non-realtime state saved with the project.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProjectState {
    pub(crate) editor_page: EditorPage,
}

/// Source of truth for [`ProjectState`]. The audio thread never touches this lock.
///
/// The lock is used only for snapshot and commit operations. Do not perform serialisation,
/// host callbacks, GUI dispatch, or file I/O while holding it (to minimise lock duration).
pub(crate) struct ProjectStateStore {
    state: RwLock<ProjectState>,
}

impl ProjectStateStore {
    pub(crate) fn new() -> Self {
        Self {
            state: RwLock::new(ProjectState::default()),
        }
    }

    pub(crate) fn snapshot(&self) -> ProjectState {
        *self.state.read()
    }

    pub(crate) fn commit(&self, state: ProjectState) {
        *self.state.write() = state;
    }

    pub(crate) fn editor_page(&self) -> EditorPage {
        self.snapshot().editor_page
    }

    pub(crate) fn set_editor_page(&self, editor_page: EditorPage) {
        self.state.write().editor_page = editor_page;
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ParameterStateSnapshot {}

/// Current values of realtime parameters, accessed concurrently from three threads:
/// - GUI thread  : reads/writes when the use changes a analyzer settings.
/// - audio thread: reads should_provide_spectrogram_data / should_provide_imager_data in `process()` and decides whether to produce data
/// - analyzer threads: read FFT size and subscription counts to decide whether to produce data
///
/// Atomics are used instead of a lock so the audio thread never has to wait.
/// Non-realtime state that belongs only in the project is separated into [`ProjectStateStore`].
pub(crate) struct SharedState {
    spectrogram_subscription_count: AtomicU32,
    imager_subscription_count: AtomicU32,

    fft_size: AtomicU32,
    imager_block_size: AtomicU32,
}

impl SharedState {
    pub(crate) fn new() -> Self {
        Self {
            spectrogram_subscription_count: AtomicU32::new(0),
            imager_subscription_count: AtomicU32::new(0),
            fft_size: AtomicU32::new(1024),
            imager_block_size: AtomicU32::new(512),
        }
    }

    pub(crate) fn snapshot_parameters(&self) -> ParameterStateSnapshot {
        ParameterStateSnapshot {}
    }

    pub(crate) fn should_provide_spectrogram_data(&self) -> bool {
        self.spectrogram_subscription_count.load(Ordering::Acquire) > 0
    }

    pub(crate) fn should_provide_imager_data(&self) -> bool {
        self.imager_subscription_count.load(Ordering::Acquire) > 0
    }

    pub(crate) fn add_spectrogram_subscription(&self) {
        self.spectrogram_subscription_count
            .fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn remove_spectrogram_subscription(&self) {
        self.spectrogram_subscription_count
            .fetch_sub(1, Ordering::AcqRel);
    }

    pub(crate) fn add_imager_subscription(&self) {
        self.imager_subscription_count
            .fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn remove_imager_subscription(&self) {
        self.imager_subscription_count
            .fetch_sub(1, Ordering::AcqRel);
    }

    pub(crate) fn fft_size(&self) -> u32 {
        self.fft_size.load(Ordering::Acquire)
    }

    pub(crate) fn set_fft_size(&self, fft_size: u32) {
        self.fft_size.store(fft_size, Ordering::Release);
    }

    pub(crate) fn imager_block_size(&self) -> u32 {
        self.imager_block_size.load(Ordering::Acquire)
    }

    pub(crate) fn restore_parameters(&self, _snapshot: ParameterStateSnapshot) {}
}

#[cfg(test)]
mod tests {
    use super::{ProjectStateStore, SharedState};

    const fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn shared_state_is_send_sync() {
        assert_send_sync::<SharedState>();
    }

    #[test]
    fn project_state_store_is_send_sync() {
        assert_send_sync::<ProjectStateStore>();
    }
}
