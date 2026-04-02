use serde::Deserialize;

/// Message type tag for raw IQ data.
pub const MSG_RAW_IQ: u8 = 0x01;

/// Message type tag for status information (JSON).
pub const MSG_STATUS: u8 = 0x03;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
