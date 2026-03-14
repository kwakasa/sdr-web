use num_complex::Complex;

/// Convert interleaved unsigned 8-bit IQ pairs to Complex<f32>.
///
/// Input format: [I0, Q0, I1, Q1, ...] where each byte is 0-255.
/// Conversion: (val - 127.5) / 127.5 maps 0->-1.0, 255->1.0, 128->~0.0.
///
/// # Panics
/// Panics if input length is odd (not complete IQ pairs).
pub fn u8_iq_to_complex(input: &[u8]) -> Vec<Complex<f32>> {
    assert!(
        input.len() % 2 == 0,
        "input length must be even (complete IQ pairs), got {}",
        input.len()
    );

    input
        .chunks_exact(2)
        .map(|pair| {
            let i = (pair[0] as f32 - 127.5) / 127.5;
            let q = (pair[1] as f32 - 127.5) / 127.5;
            Complex::new(i, q)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 0.01;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_extremes() {
        // [0, 255] -> (-1.0, 1.0)
        let result = u8_iq_to_complex(&[0, 255]);
        assert_eq!(result.len(), 1);
        assert!(approx_eq(result[0].re, -1.0), "re={}", result[0].re);
        assert!(approx_eq(result[0].im, 1.0), "im={}", result[0].im);
    }

    #[test]
    fn test_midpoint() {
        // [128, 128] -> (~0.004, ~0.004)
        let result = u8_iq_to_complex(&[128, 128]);
        assert_eq!(result.len(), 1);
        assert!(
            approx_eq(result[0].re, 0.004),
            "re={}, expected ~0.004",
            result[0].re
        );
        assert!(
            approx_eq(result[0].im, 0.004),
            "im={}, expected ~0.004",
            result[0].im
        );
    }

    #[test]
    fn test_near_midpoint() {
        // [127, 128] -> (~-0.004, ~0.004)
        let result = u8_iq_to_complex(&[127, 128]);
        assert_eq!(result.len(), 1);
        assert!(
            approx_eq(result[0].re, -0.004),
            "re={}, expected ~-0.004",
            result[0].re
        );
        assert!(
            approx_eq(result[0].im, 0.004),
            "im={}, expected ~0.004",
            result[0].im
        );
    }

    #[test]
    fn test_multiple_pairs() {
        let result = u8_iq_to_complex(&[0, 255, 128, 128]);
        assert_eq!(result.len(), 2);
        assert!(approx_eq(result[0].re, -1.0));
        assert!(approx_eq(result[0].im, 1.0));
        assert!(approx_eq(result[1].re, 0.004));
        assert!(approx_eq(result[1].im, 0.004));
    }

    #[test]
    fn test_empty_input() {
        let result = u8_iq_to_complex(&[]);
        assert!(result.is_empty());
    }

    #[test]
    #[should_panic(expected = "input length must be even")]
    fn test_odd_length_panics() {
        u8_iq_to_complex(&[0, 128, 255]);
    }
}
