use num_complex::Complex;

/// FIR filter with stateful delay line for streaming operation.
pub struct FirFilter {
    coefficients: Vec<f32>,
    delay_line: Vec<f32>,
    delay_index: usize,
}

impl FirFilter {
    /// Create a new FIR filter with the given coefficients.
    pub fn new(coefficients: Vec<f32>) -> Self {
        let len = coefficients.len();
        Self {
            coefficients,
            delay_line: vec![0.0; len],
            delay_index: 0,
        }
    }

    /// Process a single sample through the filter.
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        self.delay_line[self.delay_index] = sample;

        let mut output = 0.0;
        let len = self.coefficients.len();
        for i in 0..len {
            let delay_pos = (self.delay_index + len - i) % len;
            output += self.coefficients[i] * self.delay_line[delay_pos];
        }

        self.delay_index = (self.delay_index + 1) % len;
        output
    }

    /// Filter a block of samples, returning a new Vec.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&s| self.process_sample(s)).collect()
    }
}

/// Design a lowpass FIR filter using windowed-sinc method with Blackman window.
///
/// `cutoff_ratio`: cutoff frequency / sample rate (0.0 to 0.5).
/// `num_taps`: filter length (odd number recommended, higher = sharper cutoff).
pub fn design_lowpass(cutoff_ratio: f32, num_taps: usize) -> Vec<f32> {
    let m = num_taps - 1;
    let mid = m as f32 / 2.0;

    let mut coefficients: Vec<f32> = (0..num_taps)
        .map(|n| {
            let n_f = n as f32;
            // Blackman window
            let window = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * n_f / m as f32).cos()
                + 0.08 * (4.0 * std::f32::consts::PI * n_f / m as f32).cos();

            // Sinc function
            let sinc = if (n_f - mid).abs() < 1e-6 {
                2.0 * cutoff_ratio
            } else {
                let x = n_f - mid;
                (2.0 * std::f32::consts::PI * cutoff_ratio * x).sin() / (std::f32::consts::PI * x)
            };

            sinc * window
        })
        .collect();

    // Normalize so coefficients sum to 1.0
    let sum: f32 = coefficients.iter().sum();
    if sum.abs() > 1e-10 {
        for c in &mut coefficients {
            *c /= sum;
        }
    }

    coefficients
}

/// Complex FIR filter for IQ data.
/// Applies the same real-valued filter to both I and Q channels independently.
pub struct ComplexFirFilter {
    filter_i: FirFilter,
    filter_q: FirFilter,
}

impl ComplexFirFilter {
    /// Create a new complex FIR filter with the given coefficients.
    pub fn new(coefficients: Vec<f32>) -> Self {
        Self {
            filter_i: FirFilter::new(coefficients.clone()),
            filter_q: FirFilter::new(coefficients),
        }
    }

    /// Filter a block of complex samples, returning a new Vec.
    pub fn process(&mut self, input: &[Complex<f32>]) -> Vec<Complex<f32>> {
        input
            .iter()
            .map(|c| {
                let i = self.filter_i.process_sample(c.re);
                let q = self.filter_q.process_sample(c.im);
                Complex::new(i, q)
            })
            .collect()
    }
}

/// Decimator: lowpass filter + downsample for complex IQ data.
pub struct Decimator {
    filter: ComplexFirFilter,
    factor: usize,
}

impl Decimator {
    /// Create a new decimator with anti-aliasing filter.
    ///
    /// `cutoff_ratio`: cutoff as a fraction of the input sample rate.
    /// `num_taps`: FIR filter length.
    /// `factor`: decimation factor (keep every Nth sample).
    pub fn new(cutoff_ratio: f32, num_taps: usize, factor: usize) -> Self {
        let coefficients = design_lowpass(cutoff_ratio, num_taps);
        Self {
            filter: ComplexFirFilter::new(coefficients),
            factor,
        }
    }

    /// Filter and decimate, returning every Nth filtered sample.
    pub fn process(&mut self, input: &[Complex<f32>]) -> Vec<Complex<f32>> {
        let filtered = self.filter.process(input);
        filtered.into_iter().step_by(self.factor).collect()
    }
}

/// Real-valued decimator for audio data.
pub struct RealDecimator {
    filter: FirFilter,
    factor: usize,
}

impl RealDecimator {
    /// Create a new real-valued decimator with anti-aliasing filter.
    pub fn new(cutoff_ratio: f32, num_taps: usize, factor: usize) -> Self {
        let coefficients = design_lowpass(cutoff_ratio, num_taps);
        Self {
            filter: FirFilter::new(coefficients),
            factor,
        }
    }

    /// Filter and decimate, returning every Nth filtered sample.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let filtered = self.filter.process(input);
        filtered.into_iter().step_by(self.factor).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_design_lowpass_symmetric() {
        let taps = design_lowpass(0.2, 31);
        assert_eq!(taps.len(), 31);

        // Verify symmetry
        for i in 0..taps.len() / 2 {
            let diff = (taps[i] - taps[taps.len() - 1 - i]).abs();
            assert!(
                diff < 1e-6,
                "taps not symmetric at index {}: {} vs {}",
                i,
                taps[i],
                taps[taps.len() - 1 - i]
            );
        }
    }

    #[test]
    fn test_design_lowpass_sums_to_one() {
        let taps = design_lowpass(0.25, 51);
        let sum: f32 = taps.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "coefficients sum to {}, expected ~1.0",
            sum
        );
    }

    #[test]
    fn test_fir_filter_passband() {
        // 48 kHz sample rate, lowpass at 2 kHz -> cutoff_ratio = 2000/48000 ~ 0.0417
        // Test: 1 kHz signal should pass through with little attenuation
        let sample_rate = 48000.0_f32;
        let cutoff = 2000.0 / sample_rate;
        let taps = design_lowpass(cutoff, 101);
        let mut filter = FirFilter::new(taps);

        let freq = 1000.0;
        let n_samples = 1000;
        let input: Vec<f32> = (0..n_samples)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate).sin())
            .collect();

        let output = filter.process(&input);

        // Measure RMS of the last half (after transient settles)
        let start = n_samples / 2;
        let rms_in: f32 =
            (input[start..].iter().map(|x| x * x).sum::<f32>() / (n_samples - start) as f32).sqrt();
        let rms_out: f32 = (output[start..].iter().map(|x| x * x).sum::<f32>()
            / (n_samples - start) as f32)
            .sqrt();

        let ratio = rms_out / rms_in;
        assert!(
            ratio > 0.8,
            "1 kHz signal should pass: ratio = {:.3}",
            ratio
        );
    }

    #[test]
    fn test_fir_filter_stopband() {
        // 48 kHz sample rate, lowpass at 2 kHz
        // Test: 10 kHz signal should be attenuated
        let sample_rate = 48000.0_f32;
        let cutoff = 2000.0 / sample_rate;
        let taps = design_lowpass(cutoff, 101);
        let mut filter = FirFilter::new(taps);

        let freq = 10000.0;
        let n_samples = 1000;
        let input: Vec<f32> = (0..n_samples)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate).sin())
            .collect();

        let output = filter.process(&input);

        // Measure RMS of the last half
        let start = n_samples / 2;
        let rms_in: f32 =
            (input[start..].iter().map(|x| x * x).sum::<f32>() / (n_samples - start) as f32).sqrt();
        let rms_out: f32 = (output[start..].iter().map(|x| x * x).sum::<f32>()
            / (n_samples - start) as f32)
            .sqrt();

        let ratio = rms_out / rms_in;
        assert!(
            ratio < 0.1,
            "10 kHz signal should be attenuated: ratio = {:.3}",
            ratio
        );
    }

    #[test]
    fn test_decimator_output_length() {
        let factor = 8;
        let mut decimator = Decimator::new(0.45 / factor as f32, 51, factor);

        let input: Vec<Complex<f32>> = (0..1024).map(|_| Complex::new(1.0, 0.0)).collect();

        let output = decimator.process(&input);
        assert_eq!(output.len(), 1024 / factor);
    }

    #[test]
    fn test_real_decimator_output_length() {
        let factor = 5;
        let mut decimator = RealDecimator::new(0.45 / factor as f32, 51, factor);

        let input: Vec<f32> = vec![1.0; 500];
        let output = decimator.process(&input);
        assert_eq!(output.len(), 500 / factor);
    }
}
