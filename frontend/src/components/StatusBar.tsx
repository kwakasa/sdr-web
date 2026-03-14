"use client";

interface StatusBarProps {
  readonly connected: boolean;
  readonly reconnecting: boolean;
  readonly frequency: number;
  readonly sampleRate: number;
  readonly audioPlaying: boolean;
  readonly bufferHealth: number;
  readonly tunerType?: string;
  readonly wasmReady: boolean;
}

function formatFrequencyMHz(hz: number): string {
  return (hz / 1_000_000).toFixed(3);
}

function formatSampleRateMsps(hz: number): string {
  return (hz / 1_000_000).toFixed(3);
}

function getBufferBarColor(health: number): string {
  if (health > 0.3) return "bg-green-500";
  if (health > 0.1) return "bg-yellow-500";
  return "bg-red-500";
}

function getConnectionDot(connected: boolean, reconnecting: boolean): string {
  if (connected) return "bg-green-500";
  if (reconnecting) return "bg-yellow-500 animate-pulse";
  return "bg-red-500";
}

function getConnectionText(connected: boolean, reconnecting: boolean): string {
  if (connected) return "Connected";
  if (reconnecting) return "Reconnecting...";
  return "Disconnected";
}

export function StatusBar({
  connected,
  reconnecting,
  frequency,
  sampleRate,
  audioPlaying,
  bufferHealth,
  tunerType,
  wasmReady,
}: StatusBarProps) {
  return (
    <div className="flex flex-wrap items-center gap-4 rounded bg-gray-900 px-4 py-2 text-sm lg:gap-6">
      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${getConnectionDot(connected, reconnecting)}`}
        />
        <span className="text-gray-400">
          {getConnectionText(connected, reconnecting)}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-gray-500">Freq:</span>
        <span className="font-mono text-cyan-400">
          {formatFrequencyMHz(frequency)} MHz
        </span>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-gray-500">Rate:</span>
        <span className="font-mono text-cyan-400">
          {formatSampleRateMsps(sampleRate)} Msps
        </span>
      </div>

      {tunerType && (
        <div className="flex items-center gap-2">
          <span className="text-gray-500">Tuner:</span>
          <span className="font-mono text-cyan-400">{tunerType}</span>
        </div>
      )}

      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${
            wasmReady ? "bg-green-500" : "bg-yellow-500 animate-pulse"
          }`}
        />
        <span className="text-gray-400">
          {wasmReady ? "Browser DSP" : "Loading WASM..."}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${
            audioPlaying ? "bg-green-500 animate-pulse" : "bg-gray-600"
          }`}
        />
        <span className="text-gray-400">
          {audioPlaying ? "Audio" : "Muted"}
        </span>
        {audioPlaying && (
          <div className="flex items-center gap-1">
            <div className="h-1.5 w-10 overflow-hidden rounded-full bg-gray-700">
              <div
                className={`h-full transition-all ${getBufferBarColor(bufferHealth)}`}
                style={{ width: `${Math.round(bufferHealth * 100)}%` }}
              />
            </div>
          </div>
        )}
      </div>

      <div className="hidden text-xs text-gray-600 lg:block">
        ↑↓ freq | Space play
      </div>
    </div>
  );
}
