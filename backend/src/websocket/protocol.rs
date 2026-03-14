use serde::Deserialize;

/// Message type tag for FFT spectrum data.
pub const MSG_FFT: u8 = 0x01;

/// Message type tag for audio data (reserved for Phase 2).
pub const MSG_AUDIO: u8 = 0x02;

/// Message type tag for status information (JSON).
pub const MSG_STATUS: u8 = 0x03;

/// Encode an FFT data frame with type tag prepended.
///
/// Output: [0x01, data...]
pub fn encode_fft_frame(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + data.len());
    frame.push(MSG_FFT);
    frame.extend_from_slice(data);
    frame
}

/// Encode a status frame with type tag and JSON payload.
///
/// Output: [0x03, json_bytes...]
pub fn encode_status_frame(status: &serde_json::Value) -> Vec<u8> {
    let json_bytes = serde_json::to_vec(status).unwrap_or_default();
    let mut frame = Vec::with_capacity(1 + json_bytes.len());
    frame.push(MSG_STATUS);
    frame.extend_from_slice(&json_bytes);
    frame
}

/// Control message received from browser clients via WebSocket text frames.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlMessage {
    pub command: String,
    pub params: serde_json::Value,
}

/// Encode audio PCM frame for WebSocket transmission.
///
/// PCM data is little-endian i16 samples at 48kHz mono.
/// Output: [0x02, sample0_lo, sample0_hi, sample1_lo, sample1_hi, ...]
pub fn encode_audio_frame(pcm: &[i16]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + pcm.len() * 2);
    frame.push(MSG_AUDIO);
    for &sample in pcm {
        frame.extend_from_slice(&sample.to_le_bytes());
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_fft_frame() {
        let data = vec![10, 20, 30, 40];
        let frame = encode_fft_frame(&data);
        assert_eq!(frame[0], MSG_FFT);
        assert_eq!(&frame[1..], &data);
    }

    #[test]
    fn test_encode_fft_frame_empty() {
        let frame = encode_fft_frame(&[]);
        assert_eq!(frame.len(), 1);
        assert_eq!(frame[0], MSG_FFT);
    }

    #[test]
    fn test_encode_status_frame() {
        let status = json!({"tuner": "R820T", "frequency": 90100000});
        let frame = encode_status_frame(&status);
        assert_eq!(frame[0], MSG_STATUS);

        let json_str = std::str::from_utf8(&frame[1..]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["tuner"], "R820T");
        assert_eq!(parsed["frequency"], 90100000);
    }

    #[test]
    fn test_control_message_deserialize() {
        let json_str = r#"{"command": "set_frequency", "params": {"value": 90100000}}"#;
        let msg: ControlMessage = serde_json::from_str(json_str).unwrap();
        assert_eq!(msg.command, "set_frequency");
        assert_eq!(msg.params["value"], 90100000);
    }

    #[test]
    fn test_control_message_deserialize_gain() {
        let json_str = r#"{"command": "set_gain", "params": {"value": 400}}"#;
        let msg: ControlMessage = serde_json::from_str(json_str).unwrap();
        assert_eq!(msg.command, "set_gain");
        assert_eq!(msg.params["value"], 400);
    }

    #[test]
    fn test_control_message_deserialize_agc() {
        let json_str = r#"{"command": "set_agc", "params": {"enabled": true}}"#;
        let msg: ControlMessage = serde_json::from_str(json_str).unwrap();
        assert_eq!(msg.command, "set_agc");
        assert_eq!(msg.params["enabled"], true);
    }

    #[test]
    fn test_encode_audio_frame() {
        let pcm: Vec<i16> = vec![0, 1000, -1000, i16::MAX, i16::MIN];
        let frame = encode_audio_frame(&pcm);

        assert_eq!(frame[0], MSG_AUDIO);
        assert_eq!(frame.len(), 1 + pcm.len() * 2);

        // Verify each sample is little-endian encoded
        for (i, &sample) in pcm.iter().enumerate() {
            let offset = 1 + i * 2;
            let decoded = i16::from_le_bytes([frame[offset], frame[offset + 1]]);
            assert_eq!(decoded, sample, "sample {} mismatch", i);
        }
    }

    #[test]
    fn test_encode_audio_frame_empty() {
        let frame = encode_audio_frame(&[]);
        assert_eq!(frame.len(), 1);
        assert_eq!(frame[0], MSG_AUDIO);
    }
}
