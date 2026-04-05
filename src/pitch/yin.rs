use super::PitchDetector;

/// YIN pitch detection algorithm.
/// Good default for monophonic instruments like mandolin.
/// Reference: de Cheveigné & Kawahara (2002)
pub struct Yin {
    threshold: f64,
}

impl Yin {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl PitchDetector for Yin {
    fn detect(&mut self, samples: &[f32], sample_rate: u32) -> Option<f64> {
        let len = samples.len() / 2;
        if len < 2 {
            return None;
        }

        // Step 1 & 2: Difference function and cumulative mean normalized difference
        let mut diff = vec![0.0f64; len];
        for tau in 1..len {
            let mut sum = 0.0;
            for i in 0..len {
                let d = (samples[i] - samples[i + tau]) as f64;
                sum += d * d;
            }
            diff[tau] = sum;
        }

        // Cumulative mean normalized difference function
        let mut cmndf = vec![0.0f64; len];
        cmndf[0] = 1.0;
        let mut running_sum = 0.0;
        for tau in 1..len {
            running_sum += diff[tau];
            if running_sum == 0.0 {
                cmndf[tau] = 1.0;
            } else {
                cmndf[tau] = diff[tau] * tau as f64 / running_sum;
            }
        }

        // Step 3: Absolute threshold - find first dip below threshold
        let min_tau = sample_rate as usize / 1000; // ~1000 Hz max
        let max_tau = len.min(sample_rate as usize / 50); // ~50 Hz min

        let mut tau_estimate = None;
        for tau in min_tau..max_tau {
            if cmndf[tau] < self.threshold {
                // Find the local minimum
                while tau + 1 < max_tau && cmndf[tau + 1] < cmndf[tau] {
                    // We can't mutate tau in a for loop, so we break and search
                    break;
                }
                // Search forward for the true minimum
                let mut best_tau = tau;
                for t in tau..max_tau {
                    if cmndf[t] < cmndf[best_tau] {
                        best_tau = t;
                    } else if cmndf[t] > cmndf[best_tau] + 0.1 {
                        break;
                    }
                }
                tau_estimate = Some(best_tau);
                break;
            }
        }

        let tau = tau_estimate?;

        // Step 4: Parabolic interpolation for sub-sample accuracy
        let refined_tau = if tau > 0 && tau + 1 < len {
            let s0 = cmndf[tau - 1];
            let s1 = cmndf[tau];
            let s2 = cmndf[tau + 1];
            let shift = (s0 - s2) / (2.0 * (s0 - 2.0 * s1 + s2));
            if shift.is_finite() {
                tau as f64 + shift
            } else {
                tau as f64
            }
        } else {
            tau as f64
        };

        let frequency = sample_rate as f64 / refined_tau;

        // Sanity check
        if frequency > 50.0 && frequency < 2000.0 {
            Some(frequency)
        } else {
            None
        }
    }
}
