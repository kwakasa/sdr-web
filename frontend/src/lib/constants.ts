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

// IQ / DSP
export const IQ_CHUNK_SIZE = 16384;

/** Clamp a frequency value to the valid RTL-SDR range. */
export function clampFrequency(hz: number): number {
  return Math.max(MIN_FREQUENCY_HZ, Math.min(MAX_FREQUENCY_HZ, hz));
}

// Reconnection
export const RECONNECT_BASE_DELAY_MS = 1000;
export const RECONNECT_MAX_DELAY_MS = 10_000;

/** Format Hz as a concise MHz label (e.g. "90.10", "1234"). */
export function formatFreqMHzLabel(hz: number): string {
  const mhz = hz / 1_000_000;
  if (mhz >= 1000) return `${mhz.toFixed(0)}`;
  if (mhz >= 100) return `${mhz.toFixed(1)}`;
  return `${mhz.toFixed(2)}`;
}

export function getDefaultWebSocketUrl(): string {
  if (typeof window === "undefined") {
    return "ws://localhost:8080";
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.hostname}:8080`;
}
