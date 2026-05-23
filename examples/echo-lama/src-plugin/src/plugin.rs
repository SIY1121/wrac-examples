//! The plugin contract as seen by the host.

use std::sync::Arc;

mod audio_ports;
mod note_ports;
mod parameters;
mod state_support;

pub(crate) use parameters::{
    DEFAULT_DELAY_FEEDBACK, DEFAULT_DELAY_MIX, DEFAULT_DELAY_TIME, DEFAULT_GAIN, DEFAULT_PITCH,
    DEFAULT_PORTAMENTO, DEFAULT_VOWEL, MAX_DELAY_TIME, PARAM_BYPASS_ID, PARAM_DELAY_FEEDBACK_ID,
    PARAM_DELAY_MIX_ID, PARAM_DELAY_TIME_ID, PARAM_GAIN_ID, PARAM_PITCH_ID, PARAM_PORTAMENTO_ID,
    PARAM_VOWEL_ID, clamp_delay_feedback, clamp_delay_mix, clamp_delay_time, clamp_gain,
    clamp_pitch, clamp_portamento, clamp_vowel, host_value_to_plain, parameter_default_value,
    parameter_host_value, parameter_text_value, parameter_value_text,
};

use audio_ports::{EchoLamaAudioPorts, EchoLamaConfigurableAudioPorts};
use note_ports::EchoLamaNotePorts;
use parameters::EchoLamaParameters;
use state_support::EchoLamaStateSupport;
use wrac_clap_adapter::{
    ActivateContext, Auv2Descriptor, PluginAudioPorts, PluginConfigurableAudioPorts, PluginCore,
    PluginCoreContext, PluginDescriptor, PluginFeature, PluginGui, PluginNotePorts,
    PluginParameters, PluginResult, PluginStateSupport, Processor,
};
use wrac_wxp_gui::WxpGuiController;

use crate::audio::EchoLamaAudioProcessor;
use crate::gui::create_gui_integration;
use crate::state::{ProjectStateStore, SharedState};

pub(crate) const PLUGIN_ID: &str = env!("WRAC_PLUGIN_ID");
pub(crate) const PLUGIN_NAME: &str = env!("WRAC_PLUGIN_NAME");
pub(crate) const COMPANY_NAME: &str = env!("WRAC_COMPANY_NAME");
const AUV2_TYPE: [u8; 4] = four_char_code(env!("WRAC_AUV2_TYPE"));
const AUV2_SUBTYPE: [u8; 4] = four_char_code(env!("WRAC_AUV2_SUBTYPE"));
const AUV2_MANUFACTURER_CODE: [u8; 4] = four_char_code(env!("WRAC_AUV2_MANUFACTURER_CODE"));

// Echo Lama is an instrument (synthesizer) with stereo output.
pub(crate) const PLUGIN_DESCRIPTOR: PluginDescriptor = PluginDescriptor {
    id: PLUGIN_ID,
    name: PLUGIN_NAME,
    vendor: COMPANY_NAME,
    url: "",
    manual_url: "",
    support_url: "",
    version: env!("CARGO_PKG_VERSION"),
    description: "Delay Lama-style vowel-morphing synth",
    features: &[PluginFeature::Instrument, PluginFeature::Stereo],
    auv2: Some(Auv2Descriptor {
        manufacturer_code: AUV2_MANUFACTURER_CODE,
        manufacturer_name: COMPANY_NAME,
        plugin_type: AUV2_TYPE,
        plugin_subtype: AUV2_SUBTYPE,
    }),
};

const fn four_char_code(value: &str) -> [u8; 4] {
    let bytes = value.as_bytes();
    if bytes.len() != 4 {
        panic!("AUv2 code must be exactly 4 ASCII bytes");
    }
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// One instance of the plugin.
pub(crate) struct EchoLamaPlugin {
    shared: Arc<SharedState>,
    audio_ports: Arc<EchoLamaAudioPorts>,
    configurable_audio_ports: Arc<EchoLamaConfigurableAudioPorts>,
    note_ports: Arc<EchoLamaNotePorts>,
    parameters: Arc<EchoLamaParameters>,
    gui: Arc<WxpGuiController>,
    state_support: Arc<EchoLamaStateSupport>,
}

impl EchoLamaPlugin {
    pub(crate) fn new(context: PluginCoreContext) -> Self {
        let shared = Arc::new(SharedState::new());
        let audio_ports = Arc::new(EchoLamaAudioPorts::new());
        let configurable_audio_ports = Arc::new(EchoLamaConfigurableAudioPorts::new());
        let note_ports = Arc::new(EchoLamaNotePorts::new());
        let parameters = Arc::new(EchoLamaParameters::new(shared.clone()));
        let project_state = Arc::new(ProjectStateStore::new());
        let gui = create_gui_integration(
            project_state.clone(),
            shared.clone(),
            context.host_parameter_edit_notifier,
            context.host_gui_resize_requester,
        );
        let state_support = Arc::new(EchoLamaStateSupport::new(
            project_state,
            shared.clone(),
            gui.notifier.clone(),
        ));

        Self {
            shared,
            audio_ports,
            configurable_audio_ports,
            note_ports,
            parameters,
            gui: gui.controller,
            state_support,
        }
    }
}

pub(crate) fn create_plugin_core(context: PluginCoreContext) -> Box<dyn PluginCore> {
    crate::logging::init_debug_logging_once(PLUGIN_DESCRIPTOR.name);

    log::debug!(
        "creating plugin core: id={}, name={}",
        PLUGIN_DESCRIPTOR.id,
        PLUGIN_DESCRIPTOR.name
    );
    Box::new(EchoLamaPlugin::new(context))
}

impl PluginCore for EchoLamaPlugin {
    fn activate(&mut self, context: ActivateContext) -> PluginResult<Box<dyn Processor>> {
        log::debug!(
            "activating audio processor: sample_rate={}, max_frames_count={}",
            context.sample_rate,
            context.max_frames_count,
        );
        Ok(Box::new(EchoLamaAudioProcessor::new(
            self.shared.clone(),
            context.sample_rate as f32,
            context.max_frames_count,
        )))
    }

    fn deactivate(&mut self, _processor: Box<dyn Processor>) -> PluginResult<()> {
        log::debug!("deactivating audio processor");
        Ok(())
    }

    fn audio_ports(&self) -> Option<Arc<dyn PluginAudioPorts>> {
        Some(self.audio_ports.clone())
    }

    fn configurable_audio_ports(&self) -> Option<Arc<dyn PluginConfigurableAudioPorts>> {
        Some(self.configurable_audio_ports.clone())
    }

    fn note_ports(&self) -> Option<Arc<dyn PluginNotePorts>> {
        Some(self.note_ports.clone())
    }

    fn parameters(&self) -> Option<Arc<dyn PluginParameters>> {
        Some(self.parameters.clone())
    }

    fn state(&self) -> Option<Arc<dyn PluginStateSupport>> {
        Some(self.state_support.clone())
    }

    fn gui(&self) -> Option<Arc<dyn PluginGui>> {
        Some(self.gui.clone())
    }
}
