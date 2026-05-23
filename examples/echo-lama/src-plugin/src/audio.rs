//! DSP running on the audio thread.
//!
//! Generates a monophonic vowel-shaped synth voice. The signal chain is:
//!
//!   sawtooth voice(s) → 3 parallel bandpass formant filters → mono → delay → stereo out
//!
//! Each MIDI note triggers a free voice that runs a naive sawtooth at the note's pitch
//! (shifted by the global Pitch parameter). The mixed mono output is then shaped by a bank
//! of three biquad bandpass filters tuned to the current Vowel parameter, which morphs
//! through U → O → A → E → I.

use std::f32::consts::TAU;
use std::sync::Arc;

use wrac_clap_adapter::{
    AudioPortChannels, AudioProcessBuffer, InputEvent, NoteEvent, PluginResult, ProcessContext,
    ProcessStatus, Processor,
};

use crate::plugin::{MAX_DELAY_TIME, host_value_to_plain};
use crate::state::SharedState;

/// Maximum simultaneous voices. The plugin is currently monophonic — a new note-on while
/// another note is held steals the active voice rather than adding to it.
const MAX_VOICES: usize = 1;

/// Q (resonance) for the formant bandpass filters.
const FORMANT_Q: f32 = 8.0;

/// Per-formant gain weights (F1 dominates, F3 contributes "brightness").
const FORMANT_GAINS: [f32; 3] = [1.0, 0.7, 0.4];

/// Per-voice attenuation applied before mixing.
const PER_VOICE_GAIN: f32 = 0.3;

/// Pitch smoother + oscillator-frequency update rate, in samples.
const PITCH_SMOOTH_CHUNK: usize = 16;

/// Precomputed log2(440).
const LOG2_A4: f32 = 8.781_359_7;

/// Reference formant frequencies (Hz) for U, O, A, E, I.
const FORMANT_TABLE: [[f32; 3]; 5] = [
    [300.0, 870.0, 2240.0],  // U
    [570.0, 840.0, 2410.0],  // O
    [730.0, 1090.0, 2440.0], // A
    [530.0, 1840.0, 2480.0], // E
    [270.0, 2290.0, 3010.0], // I
];

/// Audio thread DSP for Echo Lama.
pub(crate) struct EchoLamaAudioProcessor {
    shared: Arc<SharedState>,
    sample_rate: f32,
    voices: VoiceBank,
    formants: FormantBank,
    delay: Delay,
    /// Scratch mono buffer the synth writes into before being fanned out to all output
    /// channels. Pre-allocated at activate time so `process()` never allocates.
    mono_buffer: Vec<f32>,
}

impl EchoLamaAudioProcessor {
    pub(crate) fn new(shared: Arc<SharedState>, sample_rate: f32, max_frames_count: u32) -> Self {
        Self {
            shared,
            sample_rate,
            voices: VoiceBank::new(sample_rate),
            formants: FormantBank::new(sample_rate),
            delay: Delay::new(sample_rate),
            mono_buffer: vec![0.0; max_frames_count.max(1) as usize],
        }
    }
}

impl Processor for EchoLamaAudioProcessor {
    fn process(&mut self, context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        // Wrap the realtime path in `assert_no_alloc!` in debug so any accidental
        // heap traffic on the audio thread aborts immediately. Production builds skip
        // the assertion to avoid the per-call overhead.
        #[cfg(debug_assertions)]
        {
            assert_no_alloc::assert_no_alloc(|| self.process_no_alloc(context))
        }

        #[cfg(not(debug_assertions))]
        {
            self.process_no_alloc(context)
        }
    }

    fn reset(&mut self) {
        self.voices.all_notes_off();
        self.formants.reset();
        self.delay.reset();
        let _ = self.shared.set_voicing(false);
    }
}

impl EchoLamaAudioProcessor {
    fn process_no_alloc(&mut self, mut context: ProcessContext<'_>) -> PluginResult<ProcessStatus> {
        let frames_count = context.frames_count as usize;
        if frames_count == 0 {
            return Ok(ProcessStatus::Sleep);
        }

        // The scratch buffer is sized at activate-time, but defend against pathological
        // max_frames updates by truncating below capacity. `clear` + `resize(n, 0.0)`
        // with n <= capacity is allocation-free.
        let frames_count = frames_count.min(self.mono_buffer.capacity());
        self.mono_buffer.clear();
        self.mono_buffer.resize(frames_count, 0.0);

        // Walk input events in order, splitting at each event boundary so parameter and
        // note events take effect on the *upcoming* samples — sample-accurate timing.
        let mut segment_start = 0usize;

        for event in context.events.input.iter() {
            let event_time = (event.time() as usize).min(frames_count);
            if event_time > segment_start {
                self.render_segment(segment_start, event_time);
                segment_start = event_time;
            }
            self.handle_event(&event);
        }

        if segment_start < frames_count {
            self.render_segment(segment_start, frames_count);
        }

        // Fan the mono synth output out to every output channel. `render_segment`
        // already wrote zeros for any range where Bypass was on, so this is a
        // straight copy with no bypass branching here.
        write_outputs(&mut context.audio, &self.mono_buffer)?;

        let voices_active = self.voices.has_active_voices();
        let _ = self.shared.set_voicing(voices_active);

        if voices_active || self.shared.delay_mix() > 0.0 {
            Ok(ProcessStatus::Continue)
        } else {
            Ok(ProcessStatus::Sleep)
        }
    }

    /// Renders `[start, end)` of the scratch mono buffer at the parameter values currently
    /// stored in `SharedState`. Called once per event-batched segment so automation lands
    /// at the right sample — including Bypass, which is consulted here per-segment so a
    /// bypass-on event mid-block takes effect from the event's frame onwards.
    fn render_segment(&mut self, start: usize, end: usize) {
        if end <= start {
            return;
        }
        let slice = &mut self.mono_buffer[start..end];

        if self.shared.bypass() {
            // For a synth-only plugin "bypass" means silence: no input to pass through.
            // Voice phase and delay state intentionally freeze here so audio resumes
            // smoothly when bypass turns off.
            slice.fill(0.0);
            return;
        }

        let gain = self.shared.gain();
        let pitch_log_offset = self.shared.pitch() / 12.0;
        let chunk_coef = portamento_coefficient(
            self.shared.portamento(),
            self.sample_rate,
            PITCH_SMOOTH_CHUNK,
        );
        self.formants.set_vowel(self.shared.vowel());
        let delay_time = self.shared.delay_time();
        let delay_mix = self.shared.delay_mix();
        let delay_feedback = self.shared.delay_feedback();

        self.voices
            .render_additive(slice, gain, pitch_log_offset, chunk_coef);
        self.formants.process_in_place(slice);
        self.delay
            .process_in_place(slice, delay_time, delay_mix, delay_feedback);
    }

    fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::NoteOn(note) => self.handle_note_on(note),
            InputEvent::NoteOff(note) | InputEvent::NoteEnd(note) => self.handle_note_off(note),
            InputEvent::NoteChoke(_) => {
                self.voices.all_notes_off();
            }
            InputEvent::ParamValue(event) => {
                // Host-driven param events are delivered in the host's range (Gain is
                // 0..1, Bypass is 0/1), so convert back to plain before storing. This
                // mirrors `PluginParameters::apply_parameter_value`.
                let plain = host_value_to_plain(event.parameter_id, event.value);
                let _ = self
                    .shared
                    .set_parameter_value(event.parameter_id, plain);
            }
            _ => {}
        }
    }

    fn handle_note_on(&mut self, note: &NoteEvent) {
        if note.port_index != 0 {
            return;
        }
        let key = note.key.clamp(0, 127) as u8;
        let channel = note.channel.max(0).min(15) as u8;
        let velocity = (note.velocity as f32).clamp(0.0, 1.0);
        let note_id = if note.note_id >= 0 {
            Some(note.note_id as u32)
        } else {
            None
        };
        self.voices.note_on(channel, key, velocity, note_id);
    }

    fn handle_note_off(&mut self, note: &NoteEvent) {
        if note.port_index != 0 {
            return;
        }
        let channel_match = if note.channel >= 0 {
            Some(note.channel as u8)
        } else {
            None
        };
        let key_match = if note.key >= 0 {
            Some(note.key as u8)
        } else {
            None
        };
        let note_id_match = if note.note_id >= 0 {
            Some(note.note_id as u32)
        } else {
            None
        };
        self.voices
            .note_off(channel_match, key_match, note_id_match);
    }
}

/// Fans out the mono synth signal to every output channel of every output port.
///
/// Bypass-on segments were already zeroed in `render_segment`, so this pass is purely
/// a fan-out copy. The mono buffer is in `f32`; the host can request either `f32` or
/// `f64` per port, so a generic helper converts on the way out.
fn write_outputs(audio: &mut AudioProcessBuffer<'_>, mono: &[f32]) -> PluginResult<()> {
    for mut port_pair in audio {
        match port_pair.channels()? {
            AudioPortChannels::F32(channels) => copy_mono_to_outputs(channels, mono, |s| s),
            AudioPortChannels::F64(channels) => {
                copy_mono_to_outputs(channels, mono, |s| s as f64)
            }
        }
    }
    Ok(())
}

/// Copies `mono` into every output channel of `channels`, converting each sample via
/// `convert`. Any frames past the end of `mono` are zeroed so the host never receives
/// uninitialised audio.
fn copy_mono_to_outputs<T>(
    channels: wrac_clap_adapter::AudioPairedChannels<'_, T>,
    mono: &[f32],
    convert: impl Fn(f32) -> T,
) where
    T: Copy + Default,
{
    for mut channel in channels {
        let Some(output) = channel.output_mut() else {
            continue;
        };
        let copy_len = output.len().min(mono.len());
        for (dest, src) in output[..copy_len].iter_mut().zip(mono[..copy_len].iter()) {
            *dest = convert(*src);
        }
        if copy_len < output.len() {
            for sample in output[copy_len..].iter_mut() {
                *sample = T::default();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Voice bank
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct Voice {
    active: bool,
    note: u8,
    channel: u8,
    note_id: Option<u32>,
    velocity: f32,
    phase: f32,
    target_log2_freq: f32,
    current_log2_freq: f32,
}

struct VoiceBank {
    voices: [Voice; MAX_VOICES],
    freq_to_increment: f32,
    last_target_log2_freq: Option<f32>,
}

impl VoiceBank {
    fn new(sample_rate: f32) -> Self {
        Self {
            voices: [Voice {
                active: false,
                note: 0,
                channel: 0,
                note_id: None,
                velocity: 0.0,
                phase: 0.0,
                target_log2_freq: 0.0,
                current_log2_freq: 0.0,
            }; MAX_VOICES],
            freq_to_increment: TAU / sample_rate,
            last_target_log2_freq: None,
        }
    }

    fn note_to_log2_freq(note: u8) -> f32 {
        LOG2_A4 + (note as f32 - 69.0) / 12.0
    }

    fn note_on(&mut self, channel: u8, note: u8, velocity: f32, note_id: Option<u32>) {
        let target_log2 = Self::note_to_log2_freq(note);

        let (index, is_fresh) = match self.voices.iter().position(|v| !v.active) {
            Some(i) => (i, true),
            None => (0, false),
        };
        let voice = &mut self.voices[index];

        voice.active = true;
        voice.note = note;
        voice.channel = channel;
        voice.note_id = note_id;
        voice.velocity = velocity;
        voice.target_log2_freq = target_log2;

        if is_fresh {
            voice.phase = 0.0;
            voice.current_log2_freq = self.last_target_log2_freq.unwrap_or(target_log2);
        }

        self.last_target_log2_freq = Some(target_log2);
    }

    /// `None` matchers mean "match all" — used when the host sends a note-off without
    /// specifying every field.
    fn note_off(
        &mut self,
        channel: Option<u8>,
        key: Option<u8>,
        note_id: Option<u32>,
    ) {
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            if let Some(ch) = channel {
                if voice.channel != ch {
                    continue;
                }
            }
            if let Some(k) = key {
                if voice.note != k {
                    continue;
                }
            }
            if let Some(id) = note_id {
                if voice.note_id != Some(id) {
                    continue;
                }
            }
            voice.active = false;
        }
    }

    fn all_notes_off(&mut self) {
        for voice in self.voices.iter_mut() {
            voice.active = false;
        }
        self.last_target_log2_freq = None;
    }

    fn has_active_voices(&self) -> bool {
        self.voices.iter().any(|v| v.active)
    }

    fn render_additive(
        &mut self,
        output: &mut [f32],
        gain: f32,
        pitch_log_offset: f32,
        chunk_coef: f32,
    ) {
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            let amplitude = voice.velocity * gain * PER_VOICE_GAIN;

            for chunk in output.chunks_mut(PITCH_SMOOTH_CHUNK) {
                voice.current_log2_freq = voice.target_log2_freq
                    + (voice.current_log2_freq - voice.target_log2_freq) * chunk_coef;
                let freq = (voice.current_log2_freq + pitch_log_offset).exp2();
                let phase_inc = freq * self.freq_to_increment;

                for sample in chunk.iter_mut() {
                    let saw = 2.0 * (voice.phase / TAU) - 1.0;
                    *sample += saw * amplitude;
                    voice.phase += phase_inc;
                    if voice.phase >= TAU {
                        voice.phase -= TAU;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Biquad bandpass formant bank
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
struct Biquad {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    s1: f32,
    s2: f32,
}

impl Biquad {
    const fn passthrough() -> Self {
        Self {
            b0: 1.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    fn set_bpf(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let omega = TAU * freq / sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * cos_omega / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.s1;
        self.s1 = -self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }
}

struct FormantBank {
    filters: [Biquad; 3],
    sample_rate: f32,
}

impl FormantBank {
    fn new(sample_rate: f32) -> Self {
        Self {
            filters: [Biquad::passthrough(); 3],
            sample_rate,
        }
    }

    fn set_vowel(&mut self, vowel: f32) {
        let formants = interpolate_formants(vowel);
        for (filter, &freq) in self.filters.iter_mut().zip(formants.iter()) {
            filter.set_bpf(freq, FORMANT_Q, self.sample_rate);
        }
    }

    fn process_in_place(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            let x = *sample;
            let y0 = self.filters[0].process(x) * FORMANT_GAINS[0];
            let y1 = self.filters[1].process(x) * FORMANT_GAINS[1];
            let y2 = self.filters[2].process(x) * FORMANT_GAINS[2];
            *sample = y0 + y1 + y2;
        }
    }

    fn reset(&mut self) {
        for filter in self.filters.iter_mut() {
            filter.reset();
        }
    }
}

// ---------------------------------------------------------------------------
// Delay
// ---------------------------------------------------------------------------

struct Delay {
    buffer: Vec<f32>,
    write_idx: usize,
    sample_rate: f32,
}

impl Delay {
    fn new(sample_rate: f32) -> Self {
        let buffer_len = (MAX_DELAY_TIME * sample_rate).ceil() as usize + 1;
        Self {
            buffer: vec![0.0; buffer_len.max(2)],
            write_idx: 0,
            sample_rate,
        }
    }

    fn reset(&mut self) {
        for sample in self.buffer.iter_mut() {
            *sample = 0.0;
        }
        self.write_idx = 0;
    }

    fn process_in_place(
        &mut self,
        buffer: &mut [f32],
        delay_seconds: f32,
        mix: f32,
        feedback: f32,
    ) {
        let buf_len = self.buffer.len();
        let delay_samples = ((delay_seconds * self.sample_rate).round() as usize)
            .max(1)
            .min(buf_len - 1);

        for sample in buffer.iter_mut() {
            let x = *sample;
            let read_idx = (self.write_idx + buf_len - delay_samples) % buf_len;
            let delayed = self.buffer[read_idx];
            self.buffer[self.write_idx] = x + delayed * feedback;
            *sample = x * (1.0 - mix) + delayed * mix;
            self.write_idx = if self.write_idx + 1 >= buf_len {
                0
            } else {
                self.write_idx + 1
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Formant interpolation + portamento helper
// ---------------------------------------------------------------------------

fn interpolate_formants(vowel: f32) -> [f32; 3] {
    let n = FORMANT_TABLE.len();
    let max_index = (n - 1) as f32;
    let position = vowel.clamp(0.0, 1.0) * max_index;
    let i1 = (position.floor() as usize).min(n - 2);
    let t = position - i1 as f32;
    let i2 = i1 + 1;

    let mut out = [0.0_f32; 3];
    for k in 0..3 {
        let p1 = FORMANT_TABLE[i1][k].log2();
        let p2 = FORMANT_TABLE[i2][k].log2();
        let p0 = if i1 == 0 {
            2.0 * p1 - p2
        } else {
            FORMANT_TABLE[i1 - 1][k].log2()
        };
        let p3 = if i2 + 1 >= n {
            2.0 * p2 - p1
        } else {
            FORMANT_TABLE[i2 + 1][k].log2()
        };

        let t2 = t * t;
        let t3 = t2 * t;
        let log_freq = 0.5
            * ((-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                + (-p0 + p2) * t
                + 2.0 * p1);
        out[k] = log_freq.exp2();
    }
    out
}

const PORTAMENTO_LN_RESIDUAL: f32 = -6.907_755_3;

fn portamento_coefficient(seconds: f32, sample_rate: f32, chunk_samples: usize) -> f32 {
    if seconds > 1e-4 {
        (PORTAMENTO_LN_RESIDUAL * chunk_samples as f32 / (seconds * sample_rate)).exp()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portamento_zero_returns_zero_coefficient() {
        let coef = portamento_coefficient(0.0, 44_100.0, PITCH_SMOOTH_CHUNK);
        assert_eq!(coef, 0.0);
    }

    #[test]
    fn vowel_interpolation_passes_through_each_reference_vowel() {
        let positions = [0.0, 0.25, 0.5, 0.75, 1.0];
        for (vowel_index, &t) in positions.iter().enumerate() {
            let actual = interpolate_formants(t);
            let expected = FORMANT_TABLE[vowel_index];
            for k in 0..3 {
                let diff = (actual[k] - expected[k]).abs();
                assert!(
                    diff < 0.01,
                    "vowel index {vowel_index} formant {k}: expected {} Hz, got {} Hz",
                    expected[k],
                    actual[k]
                );
            }
        }
    }
}
