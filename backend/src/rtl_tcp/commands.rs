/// SET_FREQUENCY command opcode (parameter: frequency in Hz)
pub const SET_FREQUENCY: u8 = 0x01;

/// SET_SAMPLE_RATE command opcode (parameter: sample rate in Hz)
pub const SET_SAMPLE_RATE: u8 = 0x02;

/// SET_GAIN_MODE command opcode (parameter: 0=auto, 1=manual)
pub const SET_GAIN_MODE: u8 = 0x03;

/// SET_GAIN command opcode (parameter: gain in tenths of dB)
pub const SET_GAIN: u8 = 0x04;

/// SET_AGC_MODE command opcode (parameter: 0=off, 1=on)
pub const SET_AGC_MODE: u8 = 0x08;

/// Encode an rtl_tcp command as a 5-byte array.
///
/// Format: 1-byte opcode followed by 4-byte big-endian parameter.
pub fn encode_command(opcode: u8, param: u32) -> [u8; 5] {
    let param_bytes = param.to_be_bytes();
    [
        opcode,
        param_bytes[0],
        param_bytes[1],
        param_bytes[2],
        param_bytes[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_set_frequency() {
        // 90.1 MHz = 90_100_000 Hz = 0x055_E44A0
        let cmd = encode_command(SET_FREQUENCY, 90_100_000);
        assert_eq!(cmd[0], 0x01);
        assert_eq!(&cmd[1..], &90_100_000_u32.to_be_bytes());
    }

    #[test]
    fn test_encode_set_sample_rate() {
        let cmd = encode_command(SET_SAMPLE_RATE, 2_048_000);
        assert_eq!(cmd[0], 0x02);
        assert_eq!(&cmd[1..], &2_048_000_u32.to_be_bytes());
    }

    #[test]
    fn test_encode_set_gain_mode_auto() {
        let cmd = encode_command(SET_GAIN_MODE, 0);
        assert_eq!(cmd[0], 0x03);
        assert_eq!(&cmd[1..], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_set_gain_mode_manual() {
        let cmd = encode_command(SET_GAIN_MODE, 1);
        assert_eq!(cmd[0], 0x03);
        assert_eq!(&cmd[1..], &[0, 0, 0, 1]);
    }

    #[test]
    fn test_encode_set_gain() {
        // 40.0 dB = 400 tenths
        let cmd = encode_command(SET_GAIN, 400);
        assert_eq!(cmd[0], 0x04);
        assert_eq!(&cmd[1..], &400_u32.to_be_bytes());
    }

    #[test]
    fn test_encode_set_agc_mode() {
        let cmd = encode_command(SET_AGC_MODE, 1);
        assert_eq!(cmd[0], 0x08);
        assert_eq!(&cmd[1..], &[0, 0, 0, 1]);
    }

    #[test]
    fn test_encode_zero_param() {
        let cmd = encode_command(0xFF, 0);
        assert_eq!(cmd, [0xFF, 0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_max_param() {
        let cmd = encode_command(0x01, u32::MAX);
        assert_eq!(cmd, [0x01, 0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
