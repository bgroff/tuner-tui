mod yin;
mod mpm;
mod fft;

pub use yin::Yin;
pub use mpm::Mpm;
pub use fft::FftPitch;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Yin,
    Mpm,
    Fft,
}

impl Algorithm {
    #[allow(dead_code)]
    pub const ALL: [Algorithm; 3] = [Algorithm::Yin, Algorithm::Mpm, Algorithm::Fft];

    pub fn next(self) -> Self {
        match self {
            Algorithm::Yin => Algorithm::Mpm,
            Algorithm::Mpm => Algorithm::Fft,
            Algorithm::Fft => Algorithm::Yin,
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Algorithm::Yin => write!(f, "YIN"),
            Algorithm::Mpm => write!(f, "MPM"),
            Algorithm::Fft => write!(f, "FFT"),
        }
    }
}

pub trait PitchDetector {
    /// Detect pitch from audio samples. Returns frequency in Hz if detected.
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<f64>;
}

pub fn create_detector(algo: Algorithm) -> Box<dyn PitchDetector + Send> {
    match algo {
        Algorithm::Yin => Box::new(Yin::new(0.15)),
        Algorithm::Mpm => Box::new(Mpm::new()),
        Algorithm::Fft => Box::new(FftPitch::new()),
    }
}
