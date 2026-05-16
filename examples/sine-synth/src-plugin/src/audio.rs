//! MIDI-controlled sine oscillator running on the audio thread.

use std::f64::consts::TAU;
use std::sync::Arc;

use wrac_clap_adapter::{
    AudioPairedChannels, AudioPortChannels, AudioProcessBuffer, InputEvent, NoteEvent,
    PluginResult, ProcessContext, ProcessStatus, Processor,
};

use crate::plugin::{PARAM_BYPASS_ID, PARAM_GAIN_ID, host_value_to_gain};
use crate::state::SharedState;

const MAX_VOICES: usize = 16;

// Fixed-size voices keep process() allocation-free. This example intentionally avoids
// a Vec or note allocator so the realtime constraint is visible in the simplest synth.
#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    key: i16,
    phase: f64,
    frequency_hz: f64,
    velocity: f64,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            active: false,
            key: 0,
            phase: 0.0,
            frequency_hz: 0.0,
            velocity: 0.0,
        }
    }
}

pub(crate) struct WracGainAudioProcessor {
    shared: Arc<SharedState>,
    sample_rate: f64,
    voices: [Voice; MAX_VOICES],
    next_voice: usize,
}

impl WracGainAudioProcessor {
    pub(crate) fn new(shared: Arc<SharedState>, sample_rate: f64) -> Self {
        Self {
            shared,
            // Hosts should provide a valid sample rate, but clamping here prevents a
            // divide-by-zero from turning a host bug into NaNs on the audio thread.
            sample_rate: sample_rate.max(1.0),
            voices: [Voice::default(); MAX_VOICES],
            next_voice: 0,
        }
    }
}

impl Processor for WracGainAudioProcessor {
    fn process(&mut self, context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        #[cfg(debug_assertions)]
        {
            assert_no_alloc::assert_no_alloc(|| self.process_no_alloc(context))
        }

        #[cfg(not(debug_assertions))]
        {
            self.process_no_alloc(context)
        }
    }
}

impl WracGainAudioProcessor {
    fn process_no_alloc(&mut self, mut context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        let mut gain = self.shared.gain();
        let mut bypass = self.shared.bypass();
        let mut segment_start = 0;
        let frames_count = context.frames_count as usize;

        for event in context.events.input.iter() {
            // Events are timestamped within the current block. Render the audio before
            // each event with the previous state, then apply the event for subsequent
            // samples. This is the same sample-accurate pattern used for automation.
            let event_time = (event.time() as usize).min(frames_count);
            if event_time > segment_start {
                self.render_range(
                    &mut context.audio,
                    segment_start,
                    event_time,
                    if bypass { 0.0 } else { gain },
                )?;
                segment_start = event_time;
            }

            match event {
                InputEvent::NoteOn(note) if note.velocity > 0.0 => self.note_on(note),
                // Some hosts encode note-off as note-on with zero velocity; handle both
                // shapes and choke as "stop this key" for a compact first synth.
                InputEvent::NoteOn(note)
                | InputEvent::NoteOff(note)
                | InputEvent::NoteChoke(note) => self.note_off(note),
                InputEvent::ParamValue(event) if event.parameter_id == PARAM_GAIN_ID => {
                    gain = self
                        .shared
                        .set_parameter_value(event.parameter_id, host_value_to_gain(event.value))
                        .unwrap_or(gain);
                }
                InputEvent::ParamValue(event) if event.parameter_id == PARAM_BYPASS_ID => {
                    bypass = self
                        .shared
                        .set_parameter_value(event.parameter_id, event.value)
                        .map(|value| value >= 0.5)
                        .unwrap_or(bypass);
                }
                _ => {}
            }
        }

        if segment_start < frames_count {
            self.render_range(
                &mut context.audio,
                segment_start,
                frames_count,
                if bypass { 0.0 } else { gain },
            )?;
        }

        Ok(ProcessStatus::ContinueIfNotQuiet)
    }

    fn note_on(&mut self, event: NoteEvent) {
        let slot = self
            .voices
            .iter()
            .position(|voice| !voice.active)
            .unwrap_or_else(|| {
                // Voice stealing is intentionally simple: when all slots are occupied,
                // rotate through them. A real synth might choose the quietest or oldest
                // released voice, but this keeps the example deterministic and small.
                let slot = self.next_voice;
                self.next_voice = (self.next_voice + 1) % MAX_VOICES;
                slot
            });
        self.voices[slot] = Voice {
            active: true,
            key: event.key,
            phase: 0.0,
            frequency_hz: key_to_frequency_hz(event.key),
            velocity: event.velocity.clamp(0.0, 1.0),
        };
    }

    fn note_off(&mut self, event: NoteEvent) {
        for voice in &mut self.voices {
            if voice.active && voice.key == event.key {
                voice.active = false;
            }
        }
    }

    fn render_range(
        &mut self,
        audio: &mut AudioProcessBuffer<'_>,
        start: usize,
        end: usize,
        gain: f32,
    ) -> PluginResult<()> {
        for mut port_pair in audio {
            match port_pair.channels()? {
                AudioPortChannels::F32(channels) => {
                    self.render_channels_range(channels, start, end, gain as f64)
                }
                AudioPortChannels::F64(channels) => {
                    self.render_channels_range(channels, start, end, gain as f64)
                }
            }
        }
        Ok(())
    }

    fn render_channels_range<T>(
        &mut self,
        mut channels: AudioPairedChannels<'_, T>,
        start: usize,
        end: usize,
        gain: f64,
    ) where
        T: FromSample,
    {
        let mut outputs = [None, None];
        for index in 0..2 {
            // Store raw output pointers before rendering so next_sample() can mutably
            // borrow self without also holding channel wrapper borrows. The pointers
            // remain valid for this process callback because they are host-owned buffers.
            outputs[index] = channels
                .channel_pair(index)
                .and_then(|mut channel| channel.output_mut().map(|output| output.as_mut_ptr()));
        }

        for frame in start..end {
            let sample = self.next_sample() * gain;
            for output in outputs.iter().flatten() {
                // Safety: output pointers come from distinct host-owned channel buffers and
                // frame is bounded by the host-provided process range.
                unsafe {
                    *output.add(frame) = T::from_sample(sample);
                }
            }
        }
    }

    fn next_sample(&mut self) -> f64 {
        let mut sample = 0.0;
        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }
            sample += voice.phase.sin() * voice.velocity * 0.2;
            voice.phase += TAU * voice.frequency_hz / self.sample_rate;
            if voice.phase >= TAU {
                // Keep phase bounded for long-running sessions. This avoids precision
                // loss without changing the oscillator's audible phase continuity.
                voice.phase -= TAU;
            }
        }
        sample
    }
}

trait FromSample {
    fn from_sample(sample: f64) -> Self;
}

impl FromSample for f32 {
    fn from_sample(sample: f64) -> Self {
        sample as f32
    }
}

impl FromSample for f64 {
    fn from_sample(sample: f64) -> Self {
        sample
    }
}

fn key_to_frequency_hz(key: i16) -> f64 {
    440.0 * 2.0_f64.powf((key as f64 - 69.0) / 12.0)
}
