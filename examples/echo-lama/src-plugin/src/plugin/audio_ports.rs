use wrac_clap_adapter::{
    AudioPortConfigurationRequest, AudioPortFlags, AudioPortInfo, AudioPortType, PluginAudioPorts,
    PluginConfigurableAudioPorts, PluginError, PluginResult,
};

/// Echo Lama is a stereo instrument: no audio input, one stereo output.
///
/// The layout is fixed (the formant bank and delay chain assume a mono-summed synth stage
/// that fans out to two equal channels), so we only accept the default stereo configuration
/// from the host.
pub(super) struct EchoLamaAudioPorts;

impl EchoLamaAudioPorts {
    pub(super) fn new() -> Self {
        Self
    }
}

impl PluginAudioPorts for EchoLamaAudioPorts {
    fn audio_port_count(&self, is_input: bool) -> u32 {
        if is_input { 0 } else { 1 }
    }

    fn audio_port_info(&self, index: u32, is_input: bool) -> Option<AudioPortInfo> {
        if is_input || index != 0 {
            return None;
        }
        Some(AudioPortInfo {
            id: 1,
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

/// Configurable-audio-ports capability that locks the layout to "one stereo output".
///
/// Some hosts (especially clap-wrapper VST3 / AU) need this capability present in order to
/// finalise their bus configuration; without it they may fall back to defaults that the
/// plugin never consents to. We accept the only layout we actually support and reject
/// everything else.
pub(super) struct EchoLamaConfigurableAudioPorts;

impl EchoLamaConfigurableAudioPorts {
    pub(super) fn new() -> Self {
        Self
    }
}

impl PluginConfigurableAudioPorts for EchoLamaConfigurableAudioPorts {
    fn can_apply_audio_port_configuration(
        &self,
        requests: &[AudioPortConfigurationRequest],
    ) -> bool {
        requests.iter().all(is_acceptable_output_request)
    }

    fn apply_audio_port_configuration(
        &self,
        requests: &[AudioPortConfigurationRequest],
    ) -> PluginResult<()> {
        if requests.iter().all(is_acceptable_output_request) {
            Ok(())
        } else {
            log::warn!(
                "rejecting unsupported audio port configuration: request_count={}",
                requests.len()
            );
            Err(PluginError::InvalidState)
        }
    }
}

fn is_acceptable_output_request(request: &AudioPortConfigurationRequest) -> bool {
    // Echo Lama is output-only, fixed stereo. Reject anything that asks for an input
    // port, a non-main output, a non-stereo channel count, or a port type other than
    // stereo / unspecified.
    !request.is_input
        && request.port_index == 0
        && request.channel_count == 2
        && matches!(
            request.port_type,
            AudioPortType::Stereo | AudioPortType::Unspecified
        )
}

#[cfg(test)]
mod tests {
    use wrac_clap_adapter::{AudioPortConfigurationRequest, AudioPortType};

    use super::is_acceptable_output_request;

    fn output(channel_count: u32, port_type: AudioPortType) -> AudioPortConfigurationRequest {
        AudioPortConfigurationRequest {
            is_input: false,
            port_index: 0,
            channel_count,
            port_type,
        }
    }

    #[test]
    fn accepts_default_stereo_output() {
        assert!(is_acceptable_output_request(&output(2, AudioPortType::Stereo)));
        assert!(is_acceptable_output_request(&output(
            2,
            AudioPortType::Unspecified
        )));
    }

    #[test]
    fn rejects_mono_or_input_requests() {
        assert!(!is_acceptable_output_request(&output(1, AudioPortType::Mono)));
        assert!(!is_acceptable_output_request(
            &AudioPortConfigurationRequest {
                is_input: true,
                port_index: 0,
                channel_count: 2,
                port_type: AudioPortType::Stereo,
            }
        ));
    }
}
