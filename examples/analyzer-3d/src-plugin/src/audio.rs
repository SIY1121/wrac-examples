//! DSP running on the audio thread.
//!
//! This module simply copies input to output and forwards samples to the analyzer via a ring buffer as needed.
//! [`Processor::process`] is a realtime function called repeatedly for each small buffer,
//! so the rule is to **avoid allocations and locks**. Shared state is read lock-free from
//! [`SharedState`].

use std::sync::Arc;

use wrac_clap_adapter::{
    AudioChannelPair, AudioPairedChannels, AudioPortChannels, AudioProcessBuffer, InputEvent,
    PluginError, PluginResult, ProcessContext, ProcessStatus, Processor,
};

use crate::{
    analyzer::{FftWorker, ImagerWorker},
    state::SharedState,
};

/// The DSP instance created at `activate()` and owned by the host's audio thread.
/// It lives until `deactivate()`, during which `process()` is called repeatedly.
///
/// Fields should contain only things **the audio thread can read without waiting**.
/// `shared` uses atomics and is safe to read during `process()`.
/// `audio_channel_count` is a snapshot copied from the plugin's audio layout store at
/// activate time. The adapter rejects layout changes while active, so this value stays
/// fixed for the Processor's whole lifetime. Even when a product's DSP must vary with
/// layout, convert the needed settings at activate time and pass them in rather than
/// storing an `Arc<RwLock<Layout>>`.
pub(crate) struct WracAnalyzer3DAudioProcessor {
    shared: Arc<SharedState>,
    audio_channel_count: u32,
    fft_worker: FftWorker,
    imager_worker: ImagerWorker,
}

impl WracAnalyzer3DAudioProcessor {
    pub(crate) fn new(
        shared: Arc<SharedState>,
        audio_channel_count: u32,
        fft_worker: FftWorker,
        imager_worker: ImagerWorker,
    ) -> Self {
        Self {
            shared,
            audio_channel_count,
            fft_worker,
            imager_worker,
        }
    }
}

impl Processor for WracAnalyzer3DAudioProcessor {
    /// Processes one block. `context` contains the audio I/O, the parameter event list
    /// `events.input` for this block, and the sample count `frames_count`.
    fn process(&mut self, context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        #[cfg(debug_assertions)]
        {
            // Abort immediately on any allocation. Wrapping every process() call in
            // debug builds ensures violations are not swallowed by the DAW or adapter.
            assert_no_alloc::assert_no_alloc(|| self.process_no_alloc(context))
        }

        #[cfg(not(debug_assertions))]
        {
            self.process_no_alloc(context)
        }
    }
}

impl WracAnalyzer3DAudioProcessor {
    fn process_no_alloc(&mut self, mut context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        #[cfg(debug_assertions)]
        assert_audio_layout_matches_processor_snapshot(
            &mut context.audio,
            self.audio_channel_count,
        );

        let mut port_pair = context
            .audio
            .port_pair(0)
            .ok_or(PluginError::InvalidParameter)?;
        let channels = port_pair.channels()?;
        match channels {
            AudioPortChannels::F32(channel_pairs) => {
                self.process_channels(channel_pairs, |x| x);
            }
            AudioPortChannels::F64(channel_pairs) => {
                self.process_channels(channel_pairs, |x| x as f32);
            }
        }

        // Signal that processing should continue for the next block unless the input is silent.
        // Returning `Quiet` lets the host use it as a hint for optimisation.
        Ok(ProcessStatus::ContinueIfNotQuiet)
    }

    fn process_channels<T: Copy>(
        &mut self,
        mut channel_pairs: AudioPairedChannels<'_, T>,
        to_f32: impl Fn(T) -> f32,
    ) {
        let frames_count = channel_pairs.frames_count() as usize;
        let channel_count = channel_pairs.channel_pair_count();

        // Push all channels interleaved for each frame
        // (L[0], R[0], L[1], R[1], ...)
        for frame in 0..frames_count {
            for ch in 0..channel_count {
                if let Some(mut pair) = channel_pairs.channel_pair(ch) {
                    match &mut pair {
                        AudioChannelPair::InputOutput(input, output) => {
                            output[frame] = input[frame];
                            let sample = to_f32(input[frame]);
                            self.fft_worker.push(sample);
                            self.imager_worker.push(sample);
                        }
                        AudioChannelPair::InPlace(buffer) => {
                            let sample = to_f32(buffer[frame]);
                            self.fft_worker.push(sample);
                            self.imager_worker.push(sample);
                        }
                        AudioChannelPair::InputOnly(input) => {
                            let sample = to_f32(input[frame]);
                            self.fft_worker.push(sample);
                            self.imager_worker.push(sample);
                        }
                        AudioChannelPair::OutputOnly(_) => {
                            self.fft_worker.push(0.0);
                            self.imager_worker.push(0.0);
                        }
                    }
                }
            }
        }
        // Notify the worker to process if the GUI needs spectrogram data
        if self.shared.should_provide_spectrogram_data() {
            self.fft_worker.notify();
        }
        // Notify the worker to process if the GUI needs imager data
        if self.shared.should_provide_imager_data() {
            self.imager_worker.notify();
        }
    }
}

#[cfg(debug_assertions)]
fn assert_audio_layout_matches_processor_snapshot(
    audio: &mut AudioProcessBuffer<'_>,
    expected_channel_count: u32,
) {
    // Debug-only verification that the actual buffer matches the activate-time snapshot.
    // The store's lock is not read here. Protecting memory safety from invalid buffers
    // is the adapter's responsibility; this is a demonstration of snapshot usage, not a
    // replacement. Product DSPs that don't use channel count may remove this assertion entirely.
    debug_assert_eq!(
        audio.port_pair_count(),
        1,
        "WRAC Analyzer3D expects exactly one main audio port pair"
    );

    for port_index in 0..audio.port_pair_count() {
        let Some(port_pair) = audio.port_pair(port_index) else {
            continue;
        };
        debug_assert_eq!(
            port_pair.channel_pair_count(),
            expected_channel_count as usize,
            "audio buffer channel count must match the layout captured at activate()"
        );
    }
}
