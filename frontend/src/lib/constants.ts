export const DEFAULT_WS_URL = "ws://localhost:8080";
export const DEFAULT_FREQUENCY = 90_100_000; // 90.1 MHz
export const DEFAULT_SAMPLE_RATE = 2_048_000;
export const FFT_SIZE = 2048;

// RTL-SDR tuner frequency range
export const MIN_FREQUENCY_HZ = 24_000_000; // 24 MHz
export const MAX_FREQUENCY_HZ = 1_766_000_000; // 1766 MHz

// Gain range for R820T tuner
export const MIN_GAIN_DB = 0;
export const MAX_GAIN_DB = 49.6;
export const GAIN_STEP_DB = 1;

// Reconnection
export const RECONNECT_BASE_DELAY_MS = 1000;
export const RECONNECT_MAX_DELAY_MS = 10_000;
