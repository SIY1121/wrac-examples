use std::sync::Arc;

use wrac_clap_adapter::{
    ParameterFlags, ParameterInfo, ParameterValueEvent, PluginError, PluginParameters, PluginResult,
};

use crate::state::SharedState;

// Parameter IDs are stable values used by the host for automation and project saving.
// Never change them after publishing. To add a new parameter: append an ID here and
// keep the `PluginParameters` impl and `SharedState` match arms in sync.
pub(crate) const PARAM_GAIN_ID: u32 = 1;
pub(crate) const PARAM_PITCH_ID: u32 = 2;
pub(crate) const PARAM_VOWEL_ID: u32 = 3;
pub(crate) const PARAM_PORTAMENTO_ID: u32 = 4;
pub(crate) const PARAM_DELAY_TIME_ID: u32 = 5;
pub(crate) const PARAM_DELAY_MIX_ID: u32 = 6;
pub(crate) const PARAM_DELAY_FEEDBACK_ID: u32 = 7;
pub(crate) const PARAM_BYPASS_ID: u32 = 9;

// Gain is a linear amplitude. 1.0 = 0 dB (unity), 0.0 = silence, 2.0 = +6 dB.
pub(crate) const DEFAULT_GAIN: f32 = 1.0;
pub(crate) const MIN_GAIN: f32 = 0.0;
pub(crate) const MAX_GAIN: f32 = 2.0;

/// Pitch offset in semitones, applied on top of incoming MIDI notes.
pub(crate) const DEFAULT_PITCH: f32 = 0.0;
pub(crate) const MIN_PITCH: f32 = -12.0;
pub(crate) const MAX_PITCH: f32 = 12.0;

/// Vowel position: 0 = U, 1 = I, with O / A / E in between.
pub(crate) const DEFAULT_VOWEL: f32 = 0.5;
pub(crate) const MIN_VOWEL: f32 = 0.0;
pub(crate) const MAX_VOWEL: f32 = 1.0;

/// Portamento time in seconds.
pub(crate) const DEFAULT_PORTAMENTO: f32 = 0.3;
pub(crate) const MIN_PORTAMENTO: f32 = 0.0;
pub(crate) const MAX_PORTAMENTO: f32 = 2.0;

pub(crate) const DEFAULT_DELAY_TIME: f32 = 0.2;
pub(crate) const MIN_DELAY_TIME: f32 = 0.0;
pub(crate) const MAX_DELAY_TIME: f32 = 2.0;

pub(crate) const DEFAULT_DELAY_MIX: f32 = 0.3;
pub(crate) const MIN_DELAY_MIX: f32 = 0.0;
pub(crate) const MAX_DELAY_MIX: f32 = 1.0;

pub(crate) const DEFAULT_DELAY_FEEDBACK: f32 = 0.5;
pub(crate) const MIN_DELAY_FEEDBACK: f32 = 0.0;
pub(crate) const MAX_DELAY_FEEDBACK: f32 = 0.95;

/// The parameter API as seen by the host.
pub(super) struct EchoLamaParameters {
    shared: Arc<SharedState>,
}

impl EchoLamaParameters {
    pub(super) fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }
}

impl PluginParameters for EchoLamaParameters {
    fn parameter_count(&self) -> u32 {
        8
    }

    fn parameter_info(&self, index: u32) -> Option<ParameterInfo> {
        let info = match index {
            0 => Some(gain_parameter_info()),
            1 => Some(pitch_parameter_info()),
            2 => Some(vowel_parameter_info()),
            3 => Some(portamento_parameter_info()),
            4 => Some(delay_time_parameter_info()),
            5 => Some(delay_mix_parameter_info()),
            6 => Some(delay_feedback_parameter_info()),
            7 => Some(bypass_parameter_info()),
            _ => None,
        };
        log::debug!(
            "parameter_info: index={index} -> {:?}",
            info.as_ref().map(|info| (info.id, info.name))
        );
        info
    }

    fn parameter_value(&self, parameter_id: u32) -> PluginResult<f64> {
        let value = self
            .shared
            .parameter_value(parameter_id)
            .ok_or(PluginError::InvalidParameter)?;
        parameter_host_value(parameter_id, value)
    }

    fn apply_parameter_value(&self, event: ParameterValueEvent) -> PluginResult<f64> {
        let plain = host_value_to_plain(event.parameter_id, event.value);
        let value = self
            .shared
            .set_parameter_value(event.parameter_id, plain)
            .ok_or(PluginError::InvalidParameter)?;
        parameter_host_value(event.parameter_id, value)
    }

    fn parameter_value_to_text(&self, parameter_id: u32, value: f64) -> PluginResult<String> {
        let plain = host_value_to_plain(parameter_id, value);
        parameter_value_text(parameter_id, plain)
    }

    fn parameter_text_to_value(&self, parameter_id: u32, text: &str) -> PluginResult<f64> {
        let plain = parameter_text_value(parameter_id, text)?;
        parameter_host_value(parameter_id, plain as f32)
    }
}

/// Clamps gain to the valid range. All externally supplied values must pass through this.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.clamp(MIN_GAIN, MAX_GAIN)
}

pub(crate) fn clamp_pitch(pitch: f32) -> f32 {
    pitch.clamp(MIN_PITCH, MAX_PITCH)
}

pub(crate) fn clamp_vowel(vowel: f32) -> f32 {
    vowel.clamp(MIN_VOWEL, MAX_VOWEL)
}

pub(crate) fn clamp_portamento(portamento: f32) -> f32 {
    portamento.clamp(MIN_PORTAMENTO, MAX_PORTAMENTO)
}

pub(crate) fn clamp_delay_time(delay_time: f32) -> f32 {
    delay_time.clamp(MIN_DELAY_TIME, MAX_DELAY_TIME)
}

pub(crate) fn clamp_delay_mix(delay_mix: f32) -> f32 {
    delay_mix.clamp(MIN_DELAY_MIX, MAX_DELAY_MIX)
}

pub(crate) fn clamp_delay_feedback(delay_feedback: f32) -> f32 {
    delay_feedback.clamp(MIN_DELAY_FEEDBACK, MAX_DELAY_FEEDBACK)
}

pub(crate) fn gain_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_GAIN_ID,
        name: "Gain",
        module: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: gain_to_host_value(DEFAULT_GAIN),
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn pitch_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_PITCH_ID,
        name: "Pitch",
        module: "",
        min_value: MIN_PITCH as f64,
        max_value: MAX_PITCH as f64,
        default_value: DEFAULT_PITCH as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn vowel_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_VOWEL_ID,
        name: "Vowel",
        module: "",
        min_value: MIN_VOWEL as f64,
        max_value: MAX_VOWEL as f64,
        default_value: DEFAULT_VOWEL as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn portamento_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_PORTAMENTO_ID,
        name: "Portamento",
        module: "",
        min_value: MIN_PORTAMENTO as f64,
        max_value: MAX_PORTAMENTO as f64,
        default_value: DEFAULT_PORTAMENTO as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn delay_time_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_DELAY_TIME_ID,
        name: "Delay Time",
        module: "",
        min_value: MIN_DELAY_TIME as f64,
        max_value: MAX_DELAY_TIME as f64,
        default_value: DEFAULT_DELAY_TIME as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn delay_mix_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_DELAY_MIX_ID,
        name: "Delay Mix",
        module: "",
        min_value: MIN_DELAY_MIX as f64,
        max_value: MAX_DELAY_MIX as f64,
        default_value: DEFAULT_DELAY_MIX as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn delay_feedback_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_DELAY_FEEDBACK_ID,
        name: "Delay Feedback",
        module: "",
        min_value: MIN_DELAY_FEEDBACK as f64,
        max_value: MAX_DELAY_FEEDBACK as f64,
        default_value: DEFAULT_DELAY_FEEDBACK as f64,
        flags: ParameterFlags {
            is_automatable: true,
            ..ParameterFlags::default()
        },
    }
}

pub(crate) fn bypass_parameter_info() -> ParameterInfo {
    ParameterInfo {
        id: PARAM_BYPASS_ID,
        name: "Bypass",
        module: "",
        min_value: 0.0,
        max_value: 1.0,
        default_value: 0.0,
        flags: ParameterFlags {
            is_automatable: true,
            is_stepped: true,
            is_enum: true,
            is_bypass: true,
            ..ParameterFlags::default()
        },
    }
}

/// Converts a plain value to a display string. GUI payloads route through here too, so
/// the host UI and plugin GUI always show the same text.
pub(crate) fn parameter_value_text(parameter_id: u32, value: f64) -> PluginResult<String> {
    match parameter_id {
        PARAM_GAIN_ID => Ok(gain_db_text(clamp_gain(value as f32) as f64)),
        PARAM_PITCH_ID => Ok(pitch_text(clamp_pitch(value as f32))),
        PARAM_VOWEL_ID => Ok(vowel_text(clamp_vowel(value as f32))),
        PARAM_PORTAMENTO_ID => Ok(portamento_text(clamp_portamento(value as f32))),
        PARAM_DELAY_TIME_ID => Ok(delay_time_text(clamp_delay_time(value as f32))),
        PARAM_DELAY_MIX_ID => Ok(delay_mix_text(clamp_delay_mix(value as f32))),
        PARAM_DELAY_FEEDBACK_ID => Ok(delay_feedback_text(clamp_delay_feedback(value as f32))),
        PARAM_BYPASS_ID => Ok(if value >= 0.5 { "On" } else { "Off" }.to_string()),
        _ => Err(PluginError::InvalidParameter),
    }
}

pub(crate) fn parameter_default_value(parameter_id: u32) -> PluginResult<f64> {
    match parameter_id {
        PARAM_GAIN_ID => Ok(DEFAULT_GAIN as f64),
        PARAM_PITCH_ID => Ok(DEFAULT_PITCH as f64),
        PARAM_VOWEL_ID => Ok(DEFAULT_VOWEL as f64),
        PARAM_PORTAMENTO_ID => Ok(DEFAULT_PORTAMENTO as f64),
        PARAM_DELAY_TIME_ID => Ok(DEFAULT_DELAY_TIME as f64),
        PARAM_DELAY_MIX_ID => Ok(DEFAULT_DELAY_MIX as f64),
        PARAM_DELAY_FEEDBACK_ID => Ok(DEFAULT_DELAY_FEEDBACK as f64),
        PARAM_BYPASS_ID => Ok(0.0),
        _ => Err(PluginError::InvalidParameter),
    }
}

pub(crate) fn parameter_text_value(parameter_id: u32, text: &str) -> PluginResult<f64> {
    let text = text.trim();
    match parameter_id {
        PARAM_GAIN_ID => {
            let text = text.strip_suffix("dB").unwrap_or(text).trim();
            let db = text
                .parse::<f64>()
                .map_err(|_| PluginError::InvalidParameter)?;
            Ok(clamp_gain(10.0_f64.powf(db / 20.0) as f32) as f64)
        }
        PARAM_PITCH_ID => {
            let text = text.strip_suffix("st").unwrap_or(text).trim();
            let semitones = text
                .parse::<f64>()
                .map_err(|_| PluginError::InvalidParameter)?;
            Ok(clamp_pitch(semitones as f32) as f64)
        }
        PARAM_VOWEL_ID => {
            let upper = text.to_uppercase();
            let value = match upper.as_str() {
                "U" => 0.0,
                "O" => 0.25,
                "A" => 0.5,
                "E" => 0.75,
                "I" => 1.0,
                _ => text
                    .parse::<f64>()
                    .map_err(|_| PluginError::InvalidParameter)?,
            };
            Ok(clamp_vowel(value as f32) as f64)
        }
        PARAM_PORTAMENTO_ID => {
            if text.eq_ignore_ascii_case("off") {
                return Ok(0.0);
            }
            let seconds = parse_seconds(text)?;
            Ok(clamp_portamento(seconds as f32) as f64)
        }
        PARAM_DELAY_TIME_ID => {
            let seconds = parse_seconds(text)?;
            Ok(clamp_delay_time(seconds as f32) as f64)
        }
        PARAM_DELAY_MIX_ID => {
            let mix = parse_fraction(text)?;
            Ok(clamp_delay_mix(mix as f32) as f64)
        }
        PARAM_DELAY_FEEDBACK_ID => {
            let feedback = parse_fraction(text)?;
            Ok(clamp_delay_feedback(feedback as f32) as f64)
        }
        PARAM_BYPASS_ID => match text.to_ascii_lowercase().as_str() {
            "on" | "1" | "true" => Ok(1.0),
            "off" | "0" | "false" => Ok(0.0),
            _ => Err(PluginError::InvalidParameter),
        },
        _ => Err(PluginError::InvalidParameter),
    }
}

fn parse_seconds(text: &str) -> PluginResult<f64> {
    let (body, scale) = if let Some(body) = text.strip_suffix("ms") {
        (body.trim(), 0.001)
    } else if let Some(body) = text.strip_suffix("s") {
        (body.trim(), 1.0)
    } else {
        (text, 1.0)
    };
    body.parse::<f64>()
        .map(|value| value * scale)
        .map_err(|_| PluginError::InvalidParameter)
}

fn parse_fraction(text: &str) -> PluginResult<f64> {
    let (body, scale) = if let Some(body) = text.strip_suffix('%') {
        (body.trim(), 0.01)
    } else {
        (text, 1.0)
    };
    body.parse::<f64>()
        .map(|value| value * scale)
        .map_err(|_| PluginError::InvalidParameter)
}

/// Converts a plain (internal) parameter value to the host's normalised range.
///
/// Gain alone is mapped from `[MIN_GAIN, MAX_GAIN]` to `[0, 1]` because hosts treat the
/// `min`/`max` declared in `parameter_info` as authoritative. The remaining parameters use
/// natural ranges (e.g. semitones, seconds), so the plain value is passed straight through.
pub(crate) fn parameter_host_value(parameter_id: u32, value: f32) -> PluginResult<f64> {
    match parameter_id {
        PARAM_GAIN_ID => Ok(gain_to_host_value(value)),
        PARAM_PITCH_ID => Ok(clamp_pitch(value) as f64),
        PARAM_VOWEL_ID => Ok(clamp_vowel(value) as f64),
        PARAM_PORTAMENTO_ID => Ok(clamp_portamento(value) as f64),
        PARAM_DELAY_TIME_ID => Ok(clamp_delay_time(value) as f64),
        PARAM_DELAY_MIX_ID => Ok(clamp_delay_mix(value) as f64),
        PARAM_DELAY_FEEDBACK_ID => Ok(clamp_delay_feedback(value) as f64),
        PARAM_BYPASS_ID => Ok(f64::from(value >= 0.5)),
        _ => Err(PluginError::InvalidParameter),
    }
}

/// Inverse of [`parameter_host_value`]: converts a host-side normalised/natural value back
/// to the plain value stored in [`SharedState`].
pub(crate) fn host_value_to_plain(parameter_id: u32, value: f64) -> f64 {
    match parameter_id {
        PARAM_GAIN_ID => host_value_to_gain(value),
        PARAM_BYPASS_ID => {
            if value >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        _ => value,
    }
}

pub(crate) fn gain_to_host_value(gain: f32) -> f64 {
    let span = MAX_GAIN - MIN_GAIN;
    if span <= 0.0 {
        return 0.0;
    }
    ((clamp_gain(gain) - MIN_GAIN) / span) as f64
}

pub(crate) fn host_value_to_gain(value: f64) -> f64 {
    let value = value.clamp(0.0, 1.0) as f32;
    (MIN_GAIN + value * (MAX_GAIN - MIN_GAIN)) as f64
}

pub(crate) fn gain_db_text(gain: f64) -> String {
    if gain <= 0.0 {
        "-inf dB".to_string()
    } else {
        format!("{:.1} dB", 20.0 * gain.log10())
    }
}

pub(crate) fn pitch_text(pitch: f32) -> String {
    format!("{:+.2} st", pitch)
}

pub(crate) fn vowel_text(vowel: f32) -> String {
    let labels = ["U", "O", "A", "E", "I"];
    let idx = ((vowel.clamp(0.0, 1.0) * 4.0).round() as usize).min(labels.len() - 1);
    labels[idx].to_string()
}

pub(crate) fn portamento_text(portamento: f32) -> String {
    if portamento < 0.001 {
        "off".to_string()
    } else {
        format!("{:.0} ms", portamento * 1000.0)
    }
}

pub(crate) fn delay_time_text(delay_time: f32) -> String {
    if delay_time < 1.0 {
        format!("{:.0} ms", delay_time * 1000.0)
    } else {
        format!("{:.2} s", delay_time)
    }
}

pub(crate) fn delay_mix_text(mix: f32) -> String {
    format!("{:.0}%", mix * 100.0)
}

pub(crate) fn delay_feedback_text(feedback: f32) -> String {
    format!("{:.0}%", feedback * 100.0)
}
