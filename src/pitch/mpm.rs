use super::PitchDetector;

/// McLeod Pitch Method - improved autocorrelation-based pitch detection.
/// Better noise rejection than YIN, similar accuracy.
pub struct Mpm;

impl Mpm {
    pub fn new() -> Self {
        Self
    }
}

impl PitchDetector for Mpm {
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<f64> {
        let len = samples.len();
        if len < 2 {
            return None;
        }

        let min_tau = sample_rate as usize / 1000; // ~1000 Hz max
        let max_tau = len.min(sample_rate as usize / 50); // ~50 Hz min

        if min_tau >= max_tau || max_tau >= len {
            return None;
        }

        // Compute NSDF only up to max_tau (no need to compute beyond what we use)
        let mut nsdf = vec![0.0f64; max_tau + 1];
        for tau in 0..=max_tau {
            let mut acf = 0.0;
            let mut energy_a = 0.0;
            let mut energy_b = 0.0;
            let window = len - tau;
            for i in 0..window {
                let a = samples[i] as f64;
                let b = samples[i + tau] as f64;
                acf += a * b;
                energy_a += a * a;
                energy_b += b * b;
            }
            let energy = energy_a + energy_b;
            nsdf[tau] = if energy > 0.0 { 2.0 * acf / energy } else { 0.0 };
        }

        // Find positive-going zero crossings and peaks between them
        struct Peak {
            tau: usize,
            value: f64,
        }

        let mut peaks: Vec<Peak> = Vec::new();
        let mut positive_region = false;
        let mut current_max = f64::NEG_INFINITY;
        let mut current_max_tau = 0;

        for tau in min_tau..=max_tau {
            if nsdf[tau] > 0.0 {
                if !positive_region {
                    positive_region = true;
                    current_max = nsdf[tau];
                    current_max_tau = tau;
                } else if nsdf[tau] > current_max {
                    current_max = nsdf[tau];
                    current_max_tau = tau;
                }
            } else if positive_region {
                peaks.push(Peak {
                    tau: current_max_tau,
                    value: current_max,
                });
                positive_region = false;
                current_max = f64::NEG_INFINITY;
            }
        }

        // Capture final region
        if positive_region {
            peaks.push(Peak {
                tau: current_max_tau,
                value: current_max,
            });
        }

        if peaks.is_empty() {
            return None;
        }

        // Select the first peak that is at least 0.65 * the highest peak.
        // Lowered from 0.8 — mandolin's sharp attack/decay means NSDF peaks
        // are often shorter than sustained instruments like voice or cello.
        let max_peak_value = peaks.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);
        let threshold = 0.65 * max_peak_value;

        let best_peak = peaks.iter().find(|p| p.value >= threshold)?;
        let tau = best_peak.tau;

        // Parabolic interpolation
        let refined_tau = if tau > 0 && tau + 1 <= max_tau {
            let s0 = nsdf[tau - 1];
            let s1 = nsdf[tau];
            let s2 = nsdf[tau + 1];
            let denom = s0 - 2.0 * s1 + s2;
            if denom.abs() > 1e-12 {
                let shift = (s0 - s2) / (2.0 * denom);
                tau as f64 + shift
            } else {
                tau as f64
            }
        } else {
            tau as f64
        };

        let frequency = sample_rate as f64 / refined_tau;

        if frequency > 50.0 && frequency < 2000.0 {
            Some(frequency)
        } else {
            None
        }
    }
}
