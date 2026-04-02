"use client";

import { useCallback, useState } from "react";
import { clampFrequency } from "@/lib/constants";

interface FrequencyControlProps {
  readonly frequency: number;
  readonly onFrequencyChange: (freqHz: number) => void;
}

export function FrequencyControl({
  frequency,
  onFrequencyChange,
}: FrequencyControlProps) {
  const [inputValue, setInputValue] = useState("");
  const [editing, setEditing] = useState(false);

  const displayMHz = (frequency / 1_000_000).toFixed(3);

  const handleStep = useCallback(
    (deltaHz: number) => {
      onFrequencyChange(clampFrequency(frequency + deltaHz));
    },
    [frequency, onFrequencyChange]
  );

  const handleInputSubmit = useCallback(() => {
    const parsed = parseFloat(inputValue);
    if (!isNaN(parsed)) {
      const hz = Math.round(parsed * 1_000_000);
      onFrequencyChange(clampFrequency(hz));
    }
    setEditing(false);
    setInputValue("");
  }, [inputValue, onFrequencyChange]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        handleInputSubmit();
      } else if (e.key === "Escape") {
        setEditing(false);
        setInputValue("");
      }
    },
    [handleInputSubmit]
  );

  const startEditing = useCallback(() => {
    setInputValue(displayMHz);
    setEditing(true);
  }, [displayMHz]);

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={() => handleStep(-1_000_000)}
        className="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
        title="-1 MHz"
      >
        -1M
      </button>
      <button
        onClick={() => handleStep(-100_000)}
        className="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
        title="-100 kHz"
      >
        -100k
      </button>

      {editing ? (
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onBlur={handleInputSubmit}
          onKeyDown={handleKeyDown}
          autoFocus
          className="w-36 rounded border border-cyan-600 bg-gray-900 px-2 py-1 text-center font-mono text-lg text-cyan-400 outline-none"
          placeholder="MHz"
        />
      ) : (
        <button
          onClick={startEditing}
          className="w-36 rounded border border-gray-700 bg-gray-900 px-2 py-1 text-center font-mono text-lg text-cyan-400 hover:border-cyan-600"
          title="Click to enter frequency"
        >
          {displayMHz}
        </button>
      )}

      <span className="text-sm text-gray-400">MHz</span>

      <button
        onClick={() => handleStep(100_000)}
        className="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
        title="+100 kHz"
      >
        +100k
      </button>
      <button
        onClick={() => handleStep(1_000_000)}
        className="rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
        title="+1 MHz"
      >
        +1M
      </button>
    </div>
  );
}
