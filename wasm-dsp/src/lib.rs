use wasm_bindgen::prelude::*;

mod convert;
mod deemphasis;
mod fft;
mod filter;
mod fm_demod;

use convert::u8_iq_to_complex;
use deemphasis::DeemphasisFilter;
use fft::{db_to_u8, FftProcessor};
use filter::{Decimator, RealDecimator};
use fm_demod::FmDemodulator;

/// SDR processor that runs entirely in the browser via WASM.
/// Handles FFT computation and WFM demodulation.
#[wasm_bindgen]
pub struct SdrProcessor {
    fft: FftProcessor,
    decimator1: Decimator, // 2.048M -> 256k (÷8)
    fm_demod: FmDemodulator,
    decimator2: RealDecimator, // 256k -> ~51.2k (÷5)
    deemphasis: DeemphasisFilter,
}

#[wasm_bindgen]
impl SdrProcessor {
    /// Create a new SDR processor.
    /// fft_size: FFT size (typically 2048)
    /// deemphasis_tc_us: de-emphasis time constant in microseconds (50.0 for Japan, 75.0 for US)
    #[wasm_bindgen(constructor)]
    pub fn new(fft_size: usize, deemphasis_tc_us: f32) -> Self {
        let sample_rate = 2_048_000.0_f32;
        let decimation1_factor = 8;
        let decimated1_rate = sample_rate / decimation1_factor as f32; // 256kHz
        let decimation2_factor = 5;
        let decimated2_rate = decimated1_rate / decimation2_factor as f32; // 51.2kHz

        Self {
            fft: FftProcessor::new(fft_size),
            decimator1: Decimator::new(
                0.45 / decimation1_factor as f32, // cutoff ratio
                63,                               // num taps
                decimation1_factor,
            ),
            fm_demod: FmDemodulator::new(),
            decimator2: RealDecimator::new(
                0.45 / decimation2_factor as f32,
                63,
                decimation2_factor,
            ),
            deemphasis: DeemphasisFilter::new(deemphasis_tc_us, decimated2_rate),
        }
    }

    /// Process raw IQ bytes and return FFT magnitude as u8 array.
    /// Input: interleaved u8 IQ pairs.
    /// Output: u8 array of FFT magnitudes (0-255), length = fft_size.
    pub fn compute_fft(&mut self, iq_data: &[u8]) -> Vec<u8> {
        let complex = u8_iq_to_complex(iq_data);
        let db = self.fft.compute_magnitude_db(&complex);
        db_to_u8(&db, -60.0, 0.0)
    }

    /// Process raw IQ bytes and return demodulated FM audio as f32 array.
    /// Input: interleaved u8 IQ pairs.
    /// Output: f32 audio samples (normalized to [-1.0, 1.0]), at ~51.2 kHz.
    pub fn demodulate_audio(&mut self, iq_data: &[u8]) -> Vec<f32> {
        let complex = u8_iq_to_complex(iq_data);

        // Decimate from 2.048M to 256k
        let decimated = self.decimator1.process(&complex);

        // FM demodulate
        let demodulated = self.fm_demod.demodulate(&decimated);

        // Decimate from 256k to ~51.2k
        let audio = self.decimator2.process(&demodulated);

        // Apply de-emphasis
        self.deemphasis.process(&audio)
    }
}
