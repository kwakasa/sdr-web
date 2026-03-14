use num_complex::Complex;

/// FM demodulator using polar discriminator (atan2).
///
/// Computes instantaneous frequency from phase difference between consecutive
/// samples. Output is normalized to [-1.0, 1.0].
pub struct FmDemodulator {
    prev_sample: Complex<f32>,
}

impl FmDemodulator {
    /// Create a new FM demodulator.
    pub fn new() -> Self {
        Self {
            prev_sample: Complex::new(0.0, 0.0),
        }
    }

    /// Demodulate FM signal, returning instantaneous frequency values.
    ///
    /// Algorithm: output[n] = arg(conj(input[n-1]) * input[n]) / PI
    /// Normalized to [-1.0, 1.0] range.
    pub fn demodulate(&mut self, input: &[Complex<f32>]) -> Vec<f32> {
        let mut output = Vec::with_capacity(input.len());

        for &sample in input {
            let product = self.prev_sample.conj() * sample;
            let phase_diff = product.im.atan2(product.re);
            output.push(phase_diff / std::f32::consts::PI);
            self.prev_sample = sample;
        }

        output
    }
}

impl Default for FmDemodulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.05;

    #[test]
    fn test_constant_frequency_produces_constant_output() {
        // A signal rotating at a constant rate should produce constant output.
        // Generate a signal at a fixed frequency: e^(j * 2 * pi * f * n / fs)
        let n_samples = 256;
        let freq_ratio = 0.1; // frequency as fraction of sample rate
        let input: Vec<Complex<f32>> = (0..n_samples)
            .map(|n| {
                let phase = 2.0 * std::f32::consts::PI * freq_ratio * n as f32;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let mut demod = FmDemodulator::new();
        let output = demod.demodulate(&input);

        // Skip the first sample (transition from zero prev_sample)
        // All remaining should be approximately equal
        let expected = freq_ratio * 2.0; // normalized: phase_diff/PI = 2*pi*f / pi = 2*f
                                         // But capped to [-1, 1], and freq_ratio=0.1 -> expected=0.2
        for (i, &val) in output.iter().enumerate().skip(1) {
            assert!(
                (val - expected).abs() < EPSILON,
                "sample {}: got {:.4}, expected {:.4}",
                i,
                val,
                expected
            );
        }
    }

    #[test]
    fn test_fm_modulated_signal_recovery() {
        // Create an FM-modulated signal with a known modulating tone,
        // then verify that demodulation recovers a signal resembling the modulator.
        let n_samples = 2048;
        let fs = 256000.0_f32; // sample rate
        let carrier_freq = 0.0; // baseband (already mixed down)
        let mod_freq = 1000.0; // modulating frequency
        let deviation = 75000.0; // FM deviation

        // Generate FM signal: phase(t) = 2*pi*fc*t + (deviation/mod_freq)*sin(2*pi*fm*t)
        let mod_index = deviation / mod_freq;
        let input: Vec<Complex<f32>> = (0..n_samples)
            .map(|n| {
                let t = n as f32 / fs;
                let phase = 2.0 * std::f32::consts::PI * carrier_freq * t
                    + mod_index * (2.0 * std::f32::consts::PI * mod_freq * t).sin();
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let mut demod = FmDemodulator::new();
        let output = demod.demodulate(&input);

        // The demodulated output should be a sinusoid at mod_freq.
        // Check that it has periodicity matching mod_freq.
        // At fs=256000, one period of 1000 Hz = 256 samples.
        let period_samples = (fs / mod_freq) as usize;

        // Compare a segment in the middle with a segment one period later
        let start = n_samples / 4;
        let segment_len = period_samples;
        let mut correlation = 0.0_f32;
        let mut energy = 0.0_f32;
        for i in 0..segment_len {
            correlation += output[start + i] * output[start + period_samples + i];
            energy += output[start + i] * output[start + i];
        }

        // Normalized correlation should be close to 1.0 for periodic signal
        let norm_corr = correlation / energy;
        assert!(
            norm_corr > 0.9,
            "demodulated signal should be periodic at mod_freq: correlation = {:.3}",
            norm_corr
        );
    }
}
