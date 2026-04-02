/// De-emphasis filter for FM broadcast.
///
/// Standard time constants:
/// - 50 us: Europe, Japan
/// - 75 us: Americas, Korea
pub struct DeemphasisFilter {
    alpha: f32,
    prev_output: f32,
}

impl DeemphasisFilter {
    /// Create a de-emphasis filter from time constant (in microseconds) and sample rate (in Hz).
    ///
    /// alpha = 1 - exp(-1 / (time_constant * sample_rate))
    pub fn new(time_constant_us: f32, sample_rate: f32) -> Self {
        let tc_seconds = time_constant_us * 1e-6;
        let alpha = 1.0 - (-1.0 / (tc_seconds * sample_rate)).exp();
        Self {
            alpha,
            prev_output: 0.0,
        }
    }

    /// Return the computed alpha coefficient.
    #[cfg(test)]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Apply de-emphasis: y[n] = alpha * x[n] + (1 - alpha) * y[n-1]
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = Vec::with_capacity(input.len());
        for &x in input {
            let y = self.alpha * x + (1.0 - self.alpha) * self.prev_output;
            self.prev_output = y;
            output.push(y);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_50us_at_48khz() {
        let filter = DeemphasisFilter::new(50.0, 48000.0);
        // alpha = 1 - exp(-1 / (50e-6 * 48000)) = 1 - exp(-1/2.4) ~ 1 - 0.659 ~ 0.341
        let expected = 1.0 - (-1.0_f32 / (50e-6 * 48000.0)).exp();
        assert!(
            (filter.alpha() - expected).abs() < 1e-5,
            "alpha = {}, expected {}",
            filter.alpha(),
            expected
        );
        // Rough check
        assert!(
            (filter.alpha() - 0.341).abs() < 0.01,
            "alpha ~ 0.341, got {}",
            filter.alpha()
        );
    }

    #[test]
    fn test_alpha_75us_at_48khz() {
        let filter = DeemphasisFilter::new(75.0, 48000.0);
        let expected = 1.0 - (-1.0_f32 / (75e-6 * 48000.0)).exp();
        assert!(
            (filter.alpha() - expected).abs() < 1e-5,
            "alpha = {}, expected {}",
            filter.alpha(),
            expected
        );
    }

    #[test]
    fn test_high_frequency_attenuation() {
        // De-emphasis should attenuate high frequencies more than low frequencies.
        let sample_rate = 48000.0;
        let mut filter_low = DeemphasisFilter::new(50.0, sample_rate);
        let mut filter_high = DeemphasisFilter::new(50.0, sample_rate);

        let n_samples = 4800;

        // Low-frequency signal: 100 Hz
        let low_freq_input: Vec<f32> = (0..n_samples)
            .map(|n| (2.0 * std::f32::consts::PI * 100.0 * n as f32 / sample_rate).sin())
            .collect();

        // High-frequency signal: 10000 Hz
        let high_freq_input: Vec<f32> = (0..n_samples)
            .map(|n| (2.0 * std::f32::consts::PI * 10000.0 * n as f32 / sample_rate).sin())
            .collect();

        let low_output = filter_low.process(&low_freq_input);
        let high_output = filter_high.process(&high_freq_input);

        // Measure RMS of latter half (after transient)
        let start = n_samples / 2;
        let rms_low: f32 = (low_output[start..].iter().map(|x| x * x).sum::<f32>()
            / (n_samples - start) as f32)
            .sqrt();
        let rms_high: f32 = (high_output[start..].iter().map(|x| x * x).sum::<f32>()
            / (n_samples - start) as f32)
            .sqrt();

        // High frequency should be more attenuated than low frequency
        assert!(
            rms_high < rms_low,
            "high freq RMS ({:.4}) should be less than low freq RMS ({:.4})",
            rms_high,
            rms_low
        );
    }
}
