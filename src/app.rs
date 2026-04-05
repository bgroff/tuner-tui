use crate::audio::{AudioCapture, AudioDevice, ChannelMode};
use crate::pitch::{self, Algorithm, PitchDetector};
use crate::tuning::{InstrumentString, Note, NoteDetection, closest_string, guitar_strings, mandolin_strings};
use crossbeam_channel::{bounded, Receiver};

use std::fmt;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Free,
    Mandolin,
    Guitar,
}

impl Mode {
    pub fn next(self) -> Self {
        match self {
            Mode::Free => Mode::Mandolin,
            Mode::Mandolin => Mode::Guitar,
            Mode::Guitar => Mode::Free,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Mode::Free => Mode::Guitar,
            Mode::Mandolin => Mode::Free,
            Mode::Guitar => Mode::Mandolin,
        }
    }

    pub fn strings(self) -> Option<Vec<InstrumentString>> {
        match self {
            Mode::Free => None,
            Mode::Mandolin => Some(mandolin_strings()),
            Mode::Guitar => Some(guitar_strings()),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Free => write!(f, "Free"),
            Mode::Mandolin => write!(f, "Mandolin"),
            Mode::Guitar => write!(f, "Guitar"),
        }
    }
}

pub struct App {
    pub running: bool,

    // Audio
    pub audio: AudioCapture,
    pub devices: Vec<AudioDevice>,
    pub selected_device: usize,
    pub channel_mode: ChannelMode,
    pub device_channels: u16,
    audio_receiver: Option<Receiver<Vec<f32>>>,
    sample_buffer: Vec<f32>,

    // Pitch detection
    pub algorithm: Algorithm,
    detector: Box<dyn PitchDetector + Send>,

    // Tuning
    pub mode: Mode,
    pub selected_string: usize,
    pub reference_pitch: f64,
    pub sample_rate: u32,

    // Results (smoothed)
    pub detected_frequency: Option<f64>,
    pub detected_note: Option<Note>,
    pub cents_offset: f64,
    pub closest_string_index: Option<usize>,

    // Smoothing state
    smoothed_frequency: Option<f64>,
    last_detection: Option<Instant>,

    // UI state
    pub active_selector: Selector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selector {
    Input,
    Channel,
    Algorithm,
    Mode,
    StringNote,
}

impl Selector {
    pub fn next(self) -> Self {
        match self {
            Selector::Input => Selector::Channel,
            Selector::Channel => Selector::Algorithm,
            Selector::Algorithm => Selector::Mode,
            Selector::Mode => Selector::StringNote,
            Selector::StringNote => Selector::Input,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Selector::Input => Selector::StringNote,
            Selector::Channel => Selector::Input,
            Selector::Algorithm => Selector::Channel,
            Selector::Mode => Selector::Algorithm,
            Selector::StringNote => Selector::Mode,
        }
    }
}

/// Analysis buffer: 8192 samples at 44.1kHz = ~186ms.
/// Larger than the previous 4096 (~93ms) for better frequency resolution,
/// especially on lower strings.
const BUFFER_SIZE: usize = 8192;

/// EMA smoothing factor. Lower = smoother but slower to respond.
/// 0.3 gives a good balance: reacts within ~3 frames but filters jitter.
const SMOOTHING_ALPHA: f64 = 0.3;

/// How long to hold the last reading after signal drops out (in seconds).
/// Prevents the display from flickering to "---" between pick attacks.
const HOLD_TIME_SECS: f64 = 0.5;

/// If a new reading jumps more than this many cents from the smoothed value,
/// treat it as a new note and reset smoothing (don't blend).
const JUMP_THRESHOLD_CENTS: f64 = 100.0;

impl App {
    pub fn new(reference_pitch: f64) -> Self {
        let audio = AudioCapture::new();
        let devices = audio.list_devices();
        let selected_device = audio.default_device_index().unwrap_or(0);
        let device_channels = audio.device_channel_count(selected_device);
        let algorithm = Algorithm::Yin;
        let detector = pitch::create_detector(algorithm);

        Self {
            running: true,
            audio,
            devices,
            selected_device,
            channel_mode: ChannelMode::All,
            device_channels,
            audio_receiver: None,
            sample_buffer: Vec::with_capacity(BUFFER_SIZE),
            algorithm,
            detector,
            mode: Mode::Free,
            selected_string: 0,
            reference_pitch,
            sample_rate: 44100,
            detected_frequency: None,
            detected_note: None,
            cents_offset: 0.0,
            closest_string_index: None,
            smoothed_frequency: None,
            last_detection: None,
            active_selector: Selector::Input,
        }
    }

    pub fn start_audio(&mut self) -> Result<(), String> {
        let (sender, receiver) = bounded(4);
        self.audio.start(
            self.selected_device,
            self.sample_rate,
            self.channel_mode,
            sender,
        )?;
        self.audio_receiver = Some(receiver);
        Ok(())
    }

    pub fn process_audio(&mut self) {
        if let Some(ref receiver) = self.audio_receiver {
            while let Ok(samples) = receiver.try_recv() {
                self.sample_buffer.extend_from_slice(&samples);
            }

            if self.sample_buffer.len() > BUFFER_SIZE {
                let start = self.sample_buffer.len() - BUFFER_SIZE;
                self.sample_buffer = self.sample_buffer[start..].to_vec();
            }

            if self.sample_buffer.len() >= BUFFER_SIZE {
                if let Some(raw_freq) = self.detector.detect(&self.sample_buffer, self.sample_rate) {
                    // Apply EMA smoothing
                    let freq = match self.smoothed_frequency {
                        Some(prev) => {
                            // Check if this is a big jump (new note played)
                            let cents_diff = 1200.0 * (raw_freq / prev).log2();
                            if cents_diff.abs() > JUMP_THRESHOLD_CENTS {
                                // New note — reset, don't blend
                                raw_freq
                            } else {
                                // Smooth: blend with previous
                                SMOOTHING_ALPHA * raw_freq + (1.0 - SMOOTHING_ALPHA) * prev
                            }
                        }
                        None => raw_freq,
                    };

                    self.smoothed_frequency = Some(freq);
                    self.last_detection = Some(Instant::now());
                    self.detected_frequency = Some(freq);

                    let (note, cents) = NoteDetection::detect(freq, self.reference_pitch);
                    self.detected_note = Some(note);
                    self.cents_offset = cents;

                    if let Some(strings) = self.mode.strings() {
                        self.closest_string_index = closest_string(
                            &strings,
                            freq,
                            self.reference_pitch,
                        )
                        .map(|(idx, _, _)| idx);
                    } else {
                        self.closest_string_index = None;
                    }
                } else {
                    // No detection — check hold timer
                    let expired = self
                        .last_detection
                        .map(|t| t.elapsed().as_secs_f64() > HOLD_TIME_SECS)
                        .unwrap_or(true);

                    if expired {
                        self.detected_frequency = None;
                        self.detected_note = None;
                        self.cents_offset = 0.0;
                        self.closest_string_index = None;
                        self.smoothed_frequency = None;
                    }
                    // else: keep showing the last reading
                }
            }
        }
    }

    pub fn cycle_algorithm(&mut self) {
        self.algorithm = self.algorithm.next();
        self.detector = pitch::create_detector(self.algorithm);
        // Reset smoothing when switching algorithms
        self.smoothed_frequency = None;
    }

    /// Get the target note for the currently selected string (in instrument mode).
    #[allow(dead_code)]
    pub fn target_note(&self) -> Option<Note> {
        self.mode.strings().and_then(|strings| {
            strings.get(self.selected_string).map(|s| {
                Note::from_midi(s.midi, self.reference_pitch)
            })
        })
    }

    /// Cents offset relative to the selected target string (or free offset).
    pub fn display_cents(&self) -> f64 {
        if let Some(strings) = self.mode.strings() {
            if let Some(freq) = self.detected_frequency {
                if let Some(s) = strings.get(self.selected_string) {
                    let target = Note::from_midi(s.midi, self.reference_pitch);
                    return 1200.0 * (freq / target.frequency).log2();
                }
            }
        }
        self.cents_offset
    }

    pub fn select_device(&mut self, index: usize) {
        if index < self.devices.len() {
            self.selected_device = index;
            self.device_channels = self.audio.device_channel_count(index);
            self.channel_mode = ChannelMode::All;
            self.smoothed_frequency = None;
            let _ = self.start_audio();
        }
    }

    pub fn next_device(&mut self) {
        if !self.devices.is_empty() {
            let next = (self.selected_device + 1) % self.devices.len();
            self.select_device(next);
        }
    }

    pub fn prev_device(&mut self) {
        if !self.devices.is_empty() {
            let prev = if self.selected_device == 0 {
                self.devices.len() - 1
            } else {
                self.selected_device - 1
            };
            self.select_device(prev);
        }
    }

    pub fn next_channel(&mut self) {
        self.channel_mode = match self.channel_mode {
            ChannelMode::All => {
                if self.device_channels > 1 {
                    ChannelMode::Single(0)
                } else {
                    ChannelMode::All
                }
            }
            ChannelMode::Single(ch) => {
                if ch + 1 < self.device_channels {
                    ChannelMode::Single(ch + 1)
                } else {
                    ChannelMode::All
                }
            }
        };
        let _ = self.start_audio();
    }

    pub fn prev_channel(&mut self) {
        self.channel_mode = match self.channel_mode {
            ChannelMode::All => {
                if self.device_channels > 1 {
                    ChannelMode::Single(self.device_channels - 1)
                } else {
                    ChannelMode::All
                }
            }
            ChannelMode::Single(0) => ChannelMode::All,
            ChannelMode::Single(ch) => ChannelMode::Single(ch - 1),
        };
        let _ = self.start_audio();
    }

    pub fn next_string(&mut self) {
        if let Some(strings) = self.mode.strings() {
            self.selected_string = (self.selected_string + 1) % strings.len();
        }
    }

    pub fn prev_string(&mut self) {
        if let Some(strings) = self.mode.strings() {
            if self.selected_string == 0 {
                self.selected_string = strings.len() - 1;
            } else {
                self.selected_string -= 1;
            }
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.selected_string = 0;
    }

    pub fn handle_left(&mut self) {
        match self.active_selector {
            Selector::Input => self.prev_device(),
            Selector::Channel => self.prev_channel(),
            Selector::Algorithm => self.cycle_algorithm(),
            Selector::Mode => {
                let prev = self.mode.prev();
                self.set_mode(prev);
            }
            Selector::StringNote => self.prev_string(),
        }
    }

    pub fn handle_right(&mut self) {
        match self.active_selector {
            Selector::Input => self.next_device(),
            Selector::Channel => self.next_channel(),
            Selector::Algorithm => self.cycle_algorithm(),
            Selector::Mode => {
                let next = self.mode.next();
                self.set_mode(next);
            }
            Selector::StringNote => self.next_string(),
        }
    }
}
