"use client";

import { useCallback } from "react";
import { MIN_GAIN_DB, MAX_GAIN_DB, GAIN_STEP_DB } from "@/lib/constants";

interface GainControlProps {
  readonly gain: number;
  readonly agcEnabled: boolean;
  readonly onGainChange: (gain: number) => void;
  readonly onAgcToggle: (enabled: boolean) => void;
}

export function GainControl({
  gain,
  agcEnabled,
  onGainChange,
  onAgcToggle,
}: GainControlProps) {
  const handleSliderChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onGainChange(parseFloat(e.target.value));
    },
    [onGainChange]
  );

  const handleAgcChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onAgcToggle(e.target.checked);
    },
    [onAgcToggle]
  );

  return (
    <div className="flex items-center gap-4">
      <label className="flex items-center gap-2 text-sm text-gray-400">
        <input
          type="checkbox"
          checked={agcEnabled}
          onChange={handleAgcChange}
          className="h-4 w-4 rounded border-gray-600 bg-gray-800 accent-cyan-500"
        />
        AGC
      </label>

      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-500">Gain</span>
        <input
          type="range"
          min={MIN_GAIN_DB}
          max={MAX_GAIN_DB}
          step={GAIN_STEP_DB}
          value={gain}
          onChange={handleSliderChange}
          disabled={agcEnabled}
          className="h-1.5 w-28 cursor-pointer appearance-none rounded-full bg-gray-700 accent-cyan-500 disabled:cursor-not-allowed disabled:opacity-40"
        />
        <span className="w-14 font-mono text-sm text-cyan-400">
          {gain.toFixed(0)} dB
        </span>
      </div>
    </div>
  );
}
