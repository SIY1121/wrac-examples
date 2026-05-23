use std::sync::Arc;

use wrac_clap_adapter::{
    ParameterInfo, ParameterValueEvent, PluginError, PluginParameters, PluginResult,
};

use crate::state::SharedState;

/// The parameter API as seen by the host.
///
/// Schema and values are read concurrently from generic editors, automation, and post-restore
/// rescans. Touching only the atomic source of truth in [`SharedState`] — without reaching
/// into the GUI runtime or project state — decouples host queries from the plugin lifecycle.
pub(super) struct WracAnalyzer3DParameters {
    shared: Arc<SharedState>,
}

impl WracAnalyzer3DParameters {
    pub(super) fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }
}

// The host-facing publication point for new parameters (schema and string representation).
impl PluginParameters for WracAnalyzer3DParameters {
    fn parameter_count(&self) -> u32 {
        // When adding a new parameter: keep this count in sync with the `parameter_info()` match.
        log::debug!("parameter_count -> 0");
        0
    }

    fn parameter_info(&self, _index: u32) -> Option<ParameterInfo> {
        None
    }

    /// Answers the host's query for the current value of a parameter.
    fn parameter_value(&self, _parameter_id: u32) -> PluginResult<f64> {
        Err(PluginError::InvalidParameter)
    }

    /// Called when a parameter value arrives from the host as an input event.
    fn apply_parameter_value(&self, _event: ParameterValueEvent) -> PluginResult<f64> {
        Err(PluginError::InvalidParameter)
    }

    /// Converts an internal value to a display string. Example: 1.0 → "0.0 dB".
    fn parameter_value_to_text(&self, _parameter_id: u32, _value: f64) -> PluginResult<String> {
        Err(PluginError::InvalidParameter)
    }

    /// Converts a display string to an internal value. Called when the user types "3 dB" into the host UI.
    fn parameter_text_to_value(&self, _parameter_id: u32, _text: &str) -> PluginResult<f64> {
        Err(PluginError::InvalidParameter)
    }
}
