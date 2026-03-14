"use client";

interface StatusBarProps {
  readonly connected: boolean;
  readonly frequency: number;
  readonly sampleRate: number;
}

function formatFrequencyMHz(hz: number): string {
  return (hz / 1_000_000).toFixed(3);
}

function formatSampleRateMsps(hz: number): string {
  return (hz / 1_000_000).toFixed(3);
}

export function StatusBar({ connected, frequency, sampleRate }: StatusBarProps) {
  return (
    <div className="flex items-center gap-6 rounded bg-gray-900 px-4 py-2 text-sm">
      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${
            connected ? "bg-green-500" : "bg-red-500"
          }`}
        />
        <span className="text-gray-400">
          {connected ? "Connected" : "Disconnected"}
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
    </div>
  );
}
