use crate::gui::GuiStateNotifier;
use crate::state::SharedState;
use rtrb;
use rustfft::{Fft, FftDirection, algorithm::Radix4, num_complex::Complex};
use serde::Deserialize;
use std::sync::{Arc, mpsc};
use std::thread;

pub(crate) enum AnalyzerEvent {
    NewAudioData,
    Stop,
}

#[derive(PartialEq, Eq, Clone, Deserialize)]
pub(crate) struct AnalyzerSpectrogramConfig {
    pub(crate) fft_size: usize,
}

#[derive(PartialEq, Eq, Clone, Deserialize)]
pub(crate) struct AnalyzerImagerConfig {
    // Placeholder for future imager configuration options
    pub(crate) size: usize,
}

/// Worker thread that performs FFT processing for the spectrogram. The audio thread pushes
/// audio data into the worker's ring buffer and sends a notification via `event_tx`.
/// The worker thread waits for notifications, processes available audio data when notified,
/// and pushes the processed spectrogram data to the GUI via `gui_notifier`.
pub(crate) struct FftWorker {
    event_tx: mpsc::SyncSender<AnalyzerEvent>,

    buffer_tx: rtrb::Producer<f32>,

    handle: Option<thread::JoinHandle<()>>,
}

impl FftWorker {
    pub(crate) fn new(
        max_frame_count: usize,
        channel_count: usize,
        shared: Arc<SharedState>,
        gui_notifier: Arc<GuiStateNotifier>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel(10);
        let (buffer_tx, mut buffer_rx) =
            rtrb::RingBuffer::<f32>::new(max_frame_count * channel_count * 100);

        let handle = thread::Builder::new()
            .name("FftWorkerThread".to_string())
            .spawn(move || {
                let mut fft_processor = FftProcessor::new(channel_count);

                loop {
                    match event_rx.recv() {
                        Ok(AnalyzerEvent::NewAudioData) => {
                            loop {
                                fft_processor.update_config(AnalyzerSpectrogramConfig {
                                    fft_size: shared.fft_size() as usize,
                                });
                                if buffer_rx
                                    .pop_entire_slice(&mut fft_processor.input_buffer)
                                    .is_err()
                                {
                                    // Not enough data to fill the FFT input buffer, skip this round.
                                    break;
                                }
                                fft_processor.process();

                                gui_notifier.notify_spectrogram(
                                    &fft_processor
                                        .output_buffer
                                        .iter()
                                        .map(|c| c.norm())
                                        .collect(),
                                );
                            }
                        }
                        Ok(AnalyzerEvent::Stop) | Err(_) => {
                            // Stop signal received or channel closed
                            break;
                        }
                    }
                }
            })
            .unwrap();

        Self {
            event_tx,
            buffer_tx,
            handle: Some(handle),
        }
    }

    pub(crate) fn push(&mut self, data: f32) {
        let _ = self.buffer_tx.push(data);
    }

    pub(crate) fn notify(&mut self) {
        let _ = self.event_tx.try_send(AnalyzerEvent::NewAudioData);
    }
}

impl Drop for FftWorker {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // Send a stop signal to the worker thread.
            let _ = self.event_tx.send(AnalyzerEvent::Stop);
            // Wait for the worker thread to finish.
            let _ = handle.join();
        }
    }
}

/// Processor that performs FFT calculations for the spectrogram.
struct FftProcessor {
    channel_count: usize,
    prev_config: Option<AnalyzerSpectrogramConfig>,

    fft: Option<Radix4<f32>>,
    pub input_buffer: Vec<f32>,

    pub output_buffer: Vec<Complex<f32>>,
}

impl FftProcessor {
    fn new(channel_count: usize) -> Self {
        let fft = None;
        let prev_config = None;
        let input_buffer = vec![0.0; 0];
        let output_buffer = vec![Complex::new(0.0, 0.0); 0];
        Self {
            channel_count,
            prev_config,
            fft,
            input_buffer,
            output_buffer,
        }
    }

    fn update_config(&mut self, config: AnalyzerSpectrogramConfig) {
        if Some(config.clone()) != self.prev_config {
            self._update_config(config.clone());
            self.prev_config = Some(config);
        }
    }

    fn _update_config(&mut self, config: AnalyzerSpectrogramConfig) {
        self.fft = Some(Radix4::new(config.fft_size, FftDirection::Forward));
        self.input_buffer
            .resize(config.fft_size * self.channel_count, 0.0);
        self.output_buffer
            .resize(config.fft_size, Complex::new(0.0, 0.0));
    }

    fn process(&mut self) {
        if let Some(fft) = &self.fft {
            for (index, sample) in self.input_buffer.iter().enumerate() {
                let channel = index % self.channel_count;
                let frame = index / self.channel_count;
                // take average across channels for each frame
                if channel == 0 {
                    self.output_buffer[frame].re = *sample / self.channel_count as f32;
                    self.output_buffer[frame].im = 0.0;
                } else {
                    self.output_buffer[frame].re += *sample / self.channel_count as f32;
                }
            }

            fft.process(&mut self.output_buffer);
        }
    }
}

/// Worker thread that performs mid/side processing for the imager. The audio thread pushes
/// audio data into the worker's ring buffer and sends a notification via `event_tx`. The
/// worker thread waits for notifications, processes available audio data when notified,
/// and pushes the processed imager data to the GUI via `gui_notifier`.
pub(crate) struct ImagerWorker {
    event_tx: mpsc::SyncSender<AnalyzerEvent>,

    buffer_tx: rtrb::Producer<f32>,

    handle: Option<thread::JoinHandle<()>>,
}

impl ImagerWorker {
    pub(crate) fn new(
        max_frame_count: usize,
        channel_count: usize,
        shared: Arc<SharedState>,
        gui_notifier: Arc<GuiStateNotifier>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel(10);
        let (buffer_tx, mut buffer_rx) =
            rtrb::RingBuffer::<f32>::new(max_frame_count * channel_count * 100);

        let handle = thread::Builder::new()
            .name("ImagerWorkerThread".to_string())
            .spawn(move || {
                let mut imager_processor = ImagerProcessor::new();

                loop {
                    match event_rx.recv() {
                        Ok(AnalyzerEvent::NewAudioData) => {
                            loop {
                                imager_processor.update_config(AnalyzerImagerConfig {
                                    size: shared.imager_block_size() as usize,
                                });
                                if buffer_rx
                                    .pop_entire_slice(&mut imager_processor.input_buffer)
                                    .is_err()
                                {
                                    // Not enough data to fill the Imager input buffer, skip this round.
                                    break;
                                }
                                imager_processor.process();

                                gui_notifier.notify_imager(&imager_processor.output_buffer);
                            }
                        }
                        Ok(AnalyzerEvent::Stop) | Err(_) => {
                            // Stop signal received or channel closed
                            break;
                        }
                    }
                }
            })
            .unwrap();

        Self {
            event_tx,
            buffer_tx,
            handle: Some(handle),
        }
    }

    pub(crate) fn push(&mut self, data: f32) {
        let _ = self.buffer_tx.push(data);
    }

    pub(crate) fn notify(&mut self) {
        let _ = self.event_tx.try_send(AnalyzerEvent::NewAudioData);
    }
}

impl Drop for ImagerWorker {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            // Send a stop signal to the worker thread.
            let _ = self.event_tx.send(AnalyzerEvent::Stop);
            // Wait for the worker thread to finish.
            let _ = handle.join();
        }
    }
}

/// Processor that performs mid/side processing for the imager.
struct ImagerProcessor {
    prev_config: Option<AnalyzerImagerConfig>,
    pub input_buffer: Vec<f32>,
    pub output_buffer: Vec<f32>,

    m_buffer: Vec<f32>,
    s_buffer: Vec<f32>,
}

impl ImagerProcessor {
    fn new() -> Self {
        Self {
            prev_config: None,
            input_buffer: vec![0.0; 0],
            output_buffer: vec![0.0; 0],
            m_buffer: vec![0.0; 0],
            s_buffer: vec![0.0; 0],
        }
    }

    fn update_config(&mut self, config: AnalyzerImagerConfig) {
        if Some(config.clone()) != self.prev_config {
            self._update_config(config.clone());
            self.prev_config = Some(config);
        }
    }

    fn _update_config(&mut self, config: AnalyzerImagerConfig) {
        self.input_buffer.resize(config.size, 0.0);
        self.output_buffer.resize(config.size, 0.0);
        self.m_buffer.resize(config.size, 0.0);
        self.s_buffer.resize(config.size, 0.0);
    }

    fn process(&mut self) {
        for i in 0..self.input_buffer.len() / 2 {
            let l = self.input_buffer[2 * i];
            let r = self.input_buffer[2 * i + 1];
            let m = (l + r) / 2.0;
            let s = (l - r) / 2.0;
            let theta = s.atan2(m);
            let radius = (m * m + s * s).sqrt();
            let x = radius * theta.cos();
            let y = radius * theta.sin();
            self.output_buffer[2 * i] = x;
            self.output_buffer[2 * i + 1] = y;
        }
    }
}
