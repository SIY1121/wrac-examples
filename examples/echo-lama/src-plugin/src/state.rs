//! Plugin state shared by the audio thread, GUI, and host.

use std::sync::atomic::{AtomicBool, Ordering};

use atomic_float::AtomicF32;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::plugin::{
    DEFAULT_DELAY_FEEDBACK, DEFAULT_DELAY_MIX, DEFAULT_DELAY_TIME, DEFAULT_GAIN, DEFAULT_PITCH,
    DEFAULT_PORTAMENTO, DEFAULT_VOWEL, PARAM_BYPASS_ID, PARAM_DELAY_FEEDBACK_ID,
    PARAM_DELAY_MIX_ID, PARAM_DELAY_TIME_ID, PARAM_GAIN_ID, PARAM_PITCH_ID, PARAM_PORTAMENTO_ID,
    PARAM_VOWEL_ID, clamp_delay_feedback, clamp_delay_mix, clamp_delay_time, clamp_gain,
    clamp_pitch, clamp_portamento, clamp_vowel,
};

/// Example of editor state that is saved with the project but never read from the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EditorPage {
    Controls,
    About,
}

impl Default for EditorPage {
    fn default() -> Self {
        Self::Controls
    }
}

impl EditorPage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Controls => "controls",
            Self::About => "about",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "controls" => Some(Self::Controls),
            "about" => Some(Self::About),
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
pub(crate) struct ParameterStateSnapshot {
    pub(crate) gain: f32,
    pub(crate) pitch: f32,
    pub(crate) vowel: f32,
    pub(crate) portamento: f32,
    pub(crate) delay_time: f32,
    pub(crate) delay_mix: f32,
    pub(crate) delay_feedback: f32,
    pub(crate) bypass: bool,
}

/// Current values of realtime parameters, accessed concurrently from three threads.
pub(crate) struct SharedState {
    gain: AtomicF32,
    pitch: AtomicF32,
    vowel: AtomicF32,
    portamento: AtomicF32,
    delay_time: AtomicF32,
    delay_mix: AtomicF32,
    delay_feedback: AtomicF32,
    bypass: AtomicBool,
    /// Whether the synth currently has any active voice. The audio thread sets this each
    /// `process()`; the GUI subscribes to it to drive the character's mouth open/closed.
    is_voicing: AtomicBool,
}

impl SharedState {
    pub(crate) fn new() -> Self {
        Self {
            gain: AtomicF32::new(DEFAULT_GAIN),
            pitch: AtomicF32::new(DEFAULT_PITCH),
            vowel: AtomicF32::new(DEFAULT_VOWEL),
            portamento: AtomicF32::new(DEFAULT_PORTAMENTO),
            delay_time: AtomicF32::new(DEFAULT_DELAY_TIME),
            delay_mix: AtomicF32::new(DEFAULT_DELAY_MIX),
            delay_feedback: AtomicF32::new(DEFAULT_DELAY_FEEDBACK),
            bypass: AtomicBool::new(false),
            is_voicing: AtomicBool::new(false),
        }
    }

    pub(crate) fn gain(&self) -> f32 {
        self.gain.load(Ordering::Acquire)
    }

    pub(crate) fn pitch(&self) -> f32 {
        self.pitch.load(Ordering::Acquire)
    }

    pub(crate) fn vowel(&self) -> f32 {
        self.vowel.load(Ordering::Acquire)
    }

    pub(crate) fn portamento(&self) -> f32 {
        self.portamento.load(Ordering::Acquire)
    }

    pub(crate) fn delay_time(&self) -> f32 {
        self.delay_time.load(Ordering::Acquire)
    }

    pub(crate) fn delay_mix(&self) -> f32 {
        self.delay_mix.load(Ordering::Acquire)
    }

    pub(crate) fn delay_feedback(&self) -> f32 {
        self.delay_feedback.load(Ordering::Acquire)
    }

    pub(crate) fn bypass(&self) -> bool {
        self.bypass.load(Ordering::Acquire)
    }

    pub(crate) fn is_voicing(&self) -> bool {
        self.is_voicing.load(Ordering::Acquire)
    }

    /// Sets the voicing flag and returns `true` when the value changed.
    /// The caller is expected to push a GUI notification on a change.
    pub(crate) fn set_voicing(&self, voicing: bool) -> bool {
        let was = self.is_voicing.swap(voicing, Ordering::AcqRel);
        was != voicing
    }

    pub(crate) fn snapshot_parameters(&self) -> ParameterStateSnapshot {
        ParameterStateSnapshot {
            gain: self.gain(),
            pitch: self.pitch(),
            vowel: self.vowel(),
            portamento: self.portamento(),
            delay_time: self.delay_time(),
            delay_mix: self.delay_mix(),
            delay_feedback: self.delay_feedback(),
            bypass: self.bypass(),
        }
    }

    pub(crate) fn restore_parameters(&self, snapshot: ParameterStateSnapshot) {
        self.gain
            .store(clamp_gain(snapshot.gain), Ordering::Release);
        self.pitch
            .store(clamp_pitch(snapshot.pitch), Ordering::Release);
        self.vowel
            .store(clamp_vowel(snapshot.vowel), Ordering::Release);
        self.portamento
            .store(clamp_portamento(snapshot.portamento), Ordering::Release);
        self.delay_time
            .store(clamp_delay_time(snapshot.delay_time), Ordering::Release);
        self.delay_mix
            .store(clamp_delay_mix(snapshot.delay_mix), Ordering::Release);
        self.delay_feedback.store(
            clamp_delay_feedback(snapshot.delay_feedback),
            Ordering::Release,
        );
        self.bypass.store(snapshot.bypass, Ordering::Release);
    }

    /// Returns the current value of a parameter (plain, not host-normalised).
    pub(crate) fn parameter_value(&self, parameter_id: u32) -> Option<f32> {
        match parameter_id {
            PARAM_GAIN_ID => Some(self.gain()),
            PARAM_PITCH_ID => Some(self.pitch()),
            PARAM_VOWEL_ID => Some(self.vowel()),
            PARAM_PORTAMENTO_ID => Some(self.portamento()),
            PARAM_DELAY_TIME_ID => Some(self.delay_time()),
            PARAM_DELAY_MIX_ID => Some(self.delay_mix()),
            PARAM_DELAY_FEEDBACK_ID => Some(self.delay_feedback()),
            PARAM_BYPASS_ID => Some(f32::from(self.bypass())),
            _ => None,
        }
    }

    /// Clamps an externally supplied plain value to the valid range and stores it.
    pub(crate) fn set_parameter_value(&self, parameter_id: u32, value: f64) -> Option<f32> {
        match parameter_id {
            PARAM_GAIN_ID => {
                let gain = clamp_gain(value as f32);
                self.gain.store(gain, Ordering::Release);
                Some(gain)
            }
            PARAM_PITCH_ID => {
                let pitch = clamp_pitch(value as f32);
                self.pitch.store(pitch, Ordering::Release);
                Some(pitch)
            }
            PARAM_VOWEL_ID => {
                let vowel = clamp_vowel(value as f32);
                self.vowel.store(vowel, Ordering::Release);
                Some(vowel)
            }
            PARAM_PORTAMENTO_ID => {
                let portamento = clamp_portamento(value as f32);
                self.portamento.store(portamento, Ordering::Release);
                Some(portamento)
            }
            PARAM_DELAY_TIME_ID => {
                let delay_time = clamp_delay_time(value as f32);
                self.delay_time.store(delay_time, Ordering::Release);
                Some(delay_time)
            }
            PARAM_DELAY_MIX_ID => {
                let delay_mix = clamp_delay_mix(value as f32);
                self.delay_mix.store(delay_mix, Ordering::Release);
                Some(delay_mix)
            }
            PARAM_DELAY_FEEDBACK_ID => {
                let delay_feedback = clamp_delay_feedback(value as f32);
                self.delay_feedback.store(delay_feedback, Ordering::Release);
                Some(delay_feedback)
            }
            PARAM_BYPASS_ID => {
                let bypass = value >= 0.5;
                self.bypass.store(bypass, Ordering::Release);
                Some(f32::from(bypass))
            }
            _ => None,
        }
    }
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
