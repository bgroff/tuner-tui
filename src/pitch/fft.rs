use super::PitchDetector;
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

/// FFT-based pitch detection with Harmonic Product Spectrum (HPS).
///
/// Plain FFT often picks harmonics instead of the fundamental on bright
/// instruments like mandolin. HPS fixes this by downsampling the spectrum
/// at integer ratios (2x, 3x, ...) and multiplying them together. The
/// fundamental is the only frequency where all harmonics align, so it
/// dominates the product even when individual harmonics are louder.
pub struct FftPitch {
    planner: FftPlanner<f64>,
}

impl FftPitch {
    pub fn new() -> Self {
        Self {
            planner: FftPlanner::new(),
        }
    }
}

/// Number of harmonic products to use. 5 is standard for instrument tuners —
/// enough to resolve the fundamental reliably without over-suppressing.
const HPS_HARMONICS: usize = 5;

impl PitchDetector for FftPitch {
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<f64> {
        let fft_size = samples.len().next_power_of_two();
        let fft = self.planner.plan_fft_forward(fft_size);

        // Apply Hann window and convert to complex
        let mut buffer: Vec<Complex<f64>> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                let window = 0.5
                    * (1.0
                        - (2.0 * std::f64::consts::PI * i as f64
                            / (samples.len() as f64 - 1.0))
                            .cos());
                Complex::new(s as f64 * window, 0.0)
            })
            .collect();

        buffer.resize(fft_size, Complex::new(0.0, 0.0));
        fft.process(&mut buffer);

        // Compute magnitude spectrum (first half only, up to Nyquist)
        let nyquist = fft_size / 2;
        let magnitudes: Vec<f64> = buffer[..nyquist].iter().map(|c| c.norm()).collect();

        let freq_resolution = sample_rate as f64 / fft_size as f64;

        let min_bin = (50.0 / freq_resolution).ceil() as usize;
        let max_bin = (2000.0 / freq_resolution).floor() as usize;
        let max_bin = max_bin.min(nyquist - 1);

        if min_bin >= max_bin {
            return None;
        }

        // Harmonic Product Spectrum: multiply the spectrum by downsampled
        // copies at 2x, 3x, ... HPS_HARMONICS. The usable range shrinks
        // by 1/HPS_HARMONICS since we need bins up to max_bin * HPS_HARMONICS.
        let hps_len = nyquist / HPS_HARMONICS;
        let hps_max_bin = max_bin.min(hps_len.saturating_sub(1));

        if min_bin >= hps_max_bin {
            return None;
        }

        let mut hps = vec![1.0f64; hps_max_bin + 1];
        for bin in min_bin..=hps_max_bin {
            for h in 1..=HPS_HARMONICS {
                let idx = bin * h;
                if idx < nyquist {
                    hps[bin] *= magnitudes[idx];
                }
            }
        }

        // Find peak in HPS
        let mut peak_bin = min_bin;
        for bin in min_bin..=hps_max_bin {
            if hps[bin] > hps[peak_bin] {
                peak_bin = bin;
            }
        }

        // Check that the peak is significantly above the noise floor
        let mean_hps: f64 =
            hps[min_bin..=hps_max_bin].iter().sum::<f64>() / (hps_max_bin - min_bin + 1) as f64;
        if mean_hps <= 0.0 || hps[peak_bin] < mean_hps * 3.0 {
            return None;
        }

        // Parabolic interpolation on the original magnitude spectrum for
        // sub-bin accuracy (HPS identifies the right bin, magnitudes give
        // smoother interpolation).
        let refined_bin = if peak_bin > 0 && peak_bin + 1 < nyquist {
            let alpha = magnitudes[peak_bin - 1].ln();
            let beta = magnitudes[peak_bin].ln();
            let gamma = magnitudes[peak_bin + 1].ln();
            let denom = alpha - 2.0 * beta + gamma;
            if denom.abs() > 1e-12 {
                let p = 0.5 * (alpha - gamma) / denom;
                peak_bin as f64 + p
            } else {
                peak_bin as f64
            }
        } else {
            peak_bin as f64
        };

        let frequency = refined_bin * freq_resolution;

        if frequency > 50.0 && frequency < 2000.0 {
            Some(frequency)
        } else {
            None
        }
    }
}
