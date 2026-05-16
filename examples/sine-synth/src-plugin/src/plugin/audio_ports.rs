use wrac_clap_adapter::{AudioPortFlags, AudioPortInfo, AudioPortType, PluginAudioPorts};

// A synth produces audio from note events, so it deliberately has no audio input bus.
// Keeping this fixed to stereo avoids configurable-audio-ports in the first instrument
// example and lets the reader focus on note input and rendering.
pub(super) struct WracSineSynthAudioPorts;

impl WracSineSynthAudioPorts {
    pub(super) fn new() -> Self {
        Self
    }
}

impl PluginAudioPorts for WracSineSynthAudioPorts {
    fn audio_port_count(&self, is_input: bool) -> u32 {
        if is_input { 0 } else { 1 }
    }

    fn audio_port_info(&self, index: u32, is_input: bool) -> Option<AudioPortInfo> {
        (index == 0 && !is_input).then_some(AudioPortInfo {
            id: 2,
            name: "Main Out",
            flags: AudioPortFlags {
                is_main: true,
                ..AudioPortFlags::default()
            },
            channel_count: 2,
            port_type: AudioPortType::Stereo,
            in_place_pair: None,
        })
    }
}
