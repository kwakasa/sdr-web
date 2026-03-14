use serde::Serialize;

/// Shared server state tracking current SDR configuration.
///
/// Protected by `Arc<RwLock<>>` so the WebSocket server can read it
/// and the command handler / main loop can update it.
#[derive(Debug, Clone, Serialize)]
pub struct ServerState {
    pub frequency: u32,
    pub sample_rate: u32,
    pub tuner_type: u32,
    pub gain_count: u32,
}

impl ServerState {
    pub fn new(frequency: u32, sample_rate: u32) -> Self {
        Self {
            frequency,
            sample_rate,
            tuner_type: 0,
            gain_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let state = ServerState::new(90_100_000, 2_048_000);
        assert_eq!(state.frequency, 90_100_000);
        assert_eq!(state.sample_rate, 2_048_000);
        assert_eq!(state.tuner_type, 0);
        assert_eq!(state.gain_count, 0);
    }

    #[test]
    fn test_serialize() {
        let state = ServerState::new(90_100_000, 2_048_000);
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["frequency"], 90_100_000);
        assert_eq!(json["sample_rate"], 2_048_000);
        assert_eq!(json["tuner_type"], 0);
        assert_eq!(json["gain_count"], 0);
    }
}
