use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wrac_clap_adapter::{PluginError, PluginResult, PluginState, PluginStateSupport};

use crate::gui::GuiStateNotifier;
use crate::plugin::{
    DEFAULT_DELAY_FEEDBACK, DEFAULT_DELAY_MIX, DEFAULT_DELAY_TIME, DEFAULT_PITCH,
    DEFAULT_PORTAMENTO, DEFAULT_VOWEL, PARAM_BYPASS_ID, PARAM_DELAY_FEEDBACK_ID,
    PARAM_DELAY_MIX_ID, PARAM_DELAY_TIME_ID, PARAM_GAIN_ID, PARAM_PITCH_ID, PARAM_PORTAMENTO_ID,
    PARAM_VOWEL_ID,
};
use crate::state::{
    EditorPage, ParameterStateSnapshot, ProjectState, ProjectStateStore, SharedState,
};

/// JSON-serialised plugin state saved in a DAW project.
///
/// New fields use `#[serde(default = ...)]` so an older saved state without the field loads
/// with the current factory default instead of `0.0`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SavedPluginState {
    pub(crate) gain: f32,
    #[serde(default)]
    pub(crate) bypass: bool,
    #[serde(default)]
    pub(crate) editor_page: EditorPage,
    #[serde(default = "default_pitch")]
    pub(crate) pitch: f32,
    #[serde(default = "default_vowel")]
    pub(crate) vowel: f32,
    #[serde(default = "default_portamento")]
    pub(crate) portamento: f32,
    #[serde(default = "default_delay_time")]
    pub(crate) delay_time: f32,
    #[serde(default = "default_delay_mix")]
    pub(crate) delay_mix: f32,
    #[serde(default = "default_delay_feedback")]
    pub(crate) delay_feedback: f32,
}

fn default_pitch() -> f32 {
    DEFAULT_PITCH
}

fn default_vowel() -> f32 {
    DEFAULT_VOWEL
}

fn default_portamento() -> f32 {
    DEFAULT_PORTAMENTO
}

fn default_delay_time() -> f32 {
    DEFAULT_DELAY_TIME
}

fn default_delay_mix() -> f32 {
    DEFAULT_DELAY_MIX
}

fn default_delay_feedback() -> f32 {
    DEFAULT_DELAY_FEEDBACK
}

pub(super) struct EchoLamaStateSupport {
    project_state: Arc<ProjectStateStore>,
    shared: Arc<SharedState>,
    gui_notifier: Arc<GuiStateNotifier>,
}

impl EchoLamaStateSupport {
    pub(super) fn new(
        project_state: Arc<ProjectStateStore>,
        shared: Arc<SharedState>,
        gui_notifier: Arc<GuiStateNotifier>,
    ) -> Self {
        Self {
            project_state,
            shared,
            gui_notifier,
        }
    }
}

impl PluginStateSupport for EchoLamaStateSupport {
    fn save_state(&self) -> PluginResult<PluginState> {
        let project = self.project_state.snapshot();
        let params = self.shared.snapshot_parameters();
        log::debug!(
            "saving plugin state: gain={}, pitch={}, vowel={}",
            params.gain,
            params.pitch,
            params.vowel,
        );
        let bytes = serde_json::to_vec(&SavedPluginState {
            gain: params.gain,
            bypass: params.bypass,
            editor_page: project.editor_page,
            pitch: params.pitch,
            vowel: params.vowel,
            portamento: params.portamento,
            delay_time: params.delay_time,
            delay_mix: params.delay_mix,
            delay_feedback: params.delay_feedback,
        })
        .map_err(|_| PluginError::InvalidState)?;
        Ok(PluginState { bytes })
    }

    fn restore_state(&self, state: PluginState) -> PluginResult<()> {
        log::debug!("restoring plugin state: byte_count={}", state.bytes.len());
        let state: SavedPluginState =
            serde_json::from_slice(&state.bytes).map_err(|_| PluginError::InvalidState)?;
        let project = ProjectState {
            editor_page: state.editor_page,
        };
        self.project_state.commit(project);
        self.shared.restore_parameters(ParameterStateSnapshot {
            gain: state.gain,
            pitch: state.pitch,
            vowel: state.vowel,
            portamento: state.portamento,
            delay_time: state.delay_time,
            delay_mix: state.delay_mix,
            delay_feedback: state.delay_feedback,
            bypass: state.bypass,
        });
        // Push the restored values to any active GUI subscribers so the WebView reflects
        // the loaded project state without waiting for the next periodic update.
        self.gui_notifier
            .notify_parameter(PARAM_GAIN_ID, self.shared.gain());
        self.gui_notifier
            .notify_parameter(PARAM_PITCH_ID, self.shared.pitch());
        self.gui_notifier
            .notify_parameter(PARAM_VOWEL_ID, self.shared.vowel());
        self.gui_notifier
            .notify_parameter(PARAM_PORTAMENTO_ID, self.shared.portamento());
        self.gui_notifier
            .notify_parameter(PARAM_DELAY_TIME_ID, self.shared.delay_time());
        self.gui_notifier
            .notify_parameter(PARAM_DELAY_MIX_ID, self.shared.delay_mix());
        self.gui_notifier
            .notify_parameter(PARAM_DELAY_FEEDBACK_ID, self.shared.delay_feedback());
        self.gui_notifier
            .notify_parameter(PARAM_BYPASS_ID, f32::from(self.shared.bypass()));
        self.gui_notifier.notify_editor_page(project.editor_page);
        Ok(())
    }
}
