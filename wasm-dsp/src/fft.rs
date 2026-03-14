use num_complex::Complex;
use rustfft::FftPlanner;
use std::sync::Arc;

/// FFT processor that applies a Hann window, computes FFT, and produces
/// magnitude in dB with DC-centered (FFT-shifted) output.
pub struct FftProcessor {
    fft: Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    size: usize,
}

impl FftProcessor {
    /// Create a new FFT processor with a Hann window of the given size.
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(size);
        let scratch_len = fft.get_inplace_scratch_len();
        let scratch = vec![Complex::new(0.0, 0.0); scratch_len];

        // Precompute Hann window coefficients
        let window: Vec<f32> = (0..size)
            .map(|n| {
                let phase = 2.0 * std::f32::consts::PI * n as f32 / size as f32;
                0.5 * (1.0 - phase.cos())
            })
            .collect();

        Self {
            fft,
            scratch,
            window,
            size,
        }
    }

    /// Return the FFT size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Apply Hann window, compute FFT, and return magnitude in dB.
    ///
    /// The output is FFT-shifted so DC is centered. If the input has fewer
    /// samples than the FFT size, it is zero-padded. If it has more, it is
    /// truncated.
    pub fn compute_magnitude_db(&mut self, iq: &[Complex<f32>]) -> Vec<f32> {
        // Prepare windowed buffer (zero-padded if needed)
        let mut buffer: Vec<Complex<f32>> = (0..self.size)
            .map(|i| {
                if i < iq.len() {
                    iq[i] * self.window[i]
                } else {
                    Complex::new(0.0, 0.0)
                }
            })
            .collect();

        // In-place FFT
        self.fft
            .process_with_scratch(&mut buffer, &mut self.scratch);

        // Compute magnitude in dB
        let magnitudes: Vec<f32> = buffer
            .iter()
            .map(|c| {
                let mag = c.norm();
                // Avoid log10(0) by clamping to a small value
                let mag_clamped = mag.max(1e-10);
                20.0 * mag_clamped.log10()
            })
            .collect();

        // FFT shift: swap first and second halves so DC is centered
        let half = self.size / 2;
        let mut shifted = vec![0.0_f32; self.size];
        shifted[..half].copy_from_slice(&magnitudes[half..]);
        shifted[half..].copy_from_slice(&magnitudes[..half]);

        shifted
    }
}

/// Map dB values to u8 range [0, 255] with linear scaling.
///
/// Values at or below `min_db` map to 0, values at or above `max_db` map to 255.
pub fn db_to_u8(db: &[f32], min_db: f32, max_db: f32) -> Vec<u8> {
    let range = max_db - min_db;
    if range <= 0.0 {
        return vec![0; db.len()];
    }

    db.iter()
        .map(|&val| {
            let normalized = (val - min_db) / range;
            let clamped = normalized.clamp(0.0, 1.0);
            (clamped * 255.0) as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_fft_processor_creation() {
        let proc = FftProcessor::new(2048);
        assert_eq!(proc.size(), 2048);
    }

    #[test]
    fn test_fft_sine_wave_peak() {
        let size = 1024;
        let mut proc = FftProcessor::new(size);

        // Generate a pure sine wave at bin 100
        let bin = 100;
        let iq: Vec<Complex<f32>> = (0..size)
            .map(|n| {
                let phase = 2.0 * PI * bin as f32 * n as f32 / size as f32;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let db = proc.compute_magnitude_db(&iq);
        assert_eq!(db.len(), size);

        // After FFT shift, bin 100 maps to index size/2 + 100
        let peak_idx = size / 2 + bin;
        let peak_val = db[peak_idx];

        // The peak should be significantly above the noise floor
        let avg: f32 = db.iter().sum::<f32>() / db.len() as f32;
        assert!(
            peak_val > avg + 20.0,
            "peak at bin {} = {:.1} dB, average = {:.1} dB (expected >20 dB above average)",
            peak_idx,
            peak_val,
            avg
        );
    }

    #[test]
    fn test_fft_shift_dc_centered() {
        let size = 256;
        let mut proc = FftProcessor::new(size);

        // DC signal: constant value
        let iq: Vec<Complex<f32>> = vec![Complex::new(1.0, 0.0); size];
        let db = proc.compute_magnitude_db(&iq);

        // DC component should be at the center (index size/2)
        let dc_idx = size / 2;
        let dc_val = db[dc_idx];

        // DC should be the strongest component
        for (i, &val) in db.iter().enumerate() {
            if i != dc_idx {
                assert!(
                    dc_val >= val,
                    "DC at index {} ({:.1} dB) should be >= index {} ({:.1} dB)",
                    dc_idx,
                    dc_val,
                    i,
                    val
                );
            }
        }
    }

    #[test]
    fn test_fft_zero_padding() {
        let size = 1024;
        let mut proc = FftProcessor::new(size);

        // Provide fewer samples than FFT size
        let short_iq: Vec<Complex<f32>> = (0..512)
            .map(|n| {
                let phase = 2.0 * PI * 50.0 * n as f32 / size as f32;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();

        let db = proc.compute_magnitude_db(&short_iq);
        assert_eq!(db.len(), size);
    }

    #[test]
    fn test_db_to_u8_basic() {
        let db = vec![-40.0, -20.0, 0.0];
        let result = db_to_u8(&db, -40.0, 0.0);
        assert_eq!(result[0], 0); // min_db -> 0
        assert_eq!(result[1], 127); // midpoint -> ~127
        assert_eq!(result[2], 255); // max_db -> 255
    }

    #[test]
    fn test_db_to_u8_clamping() {
        let db = vec![-100.0, 100.0];
        let result = db_to_u8(&db, -40.0, 0.0);
        assert_eq!(result[0], 0); // below min clamps to 0
        assert_eq!(result[1], 255); // above max clamps to 255
    }

    #[test]
    fn test_db_to_u8_zero_range() {
        let db = vec![5.0, 10.0];
        let result = db_to_u8(&db, 10.0, 10.0);
        assert_eq!(result, vec![0, 0]);
    }

    #[test]
    fn test_db_to_u8_empty() {
        let db: Vec<f32> = vec![];
        let result = db_to_u8(&db, -40.0, 0.0);
        assert!(result.is_empty());
    }
}
