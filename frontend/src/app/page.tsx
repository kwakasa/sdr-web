"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useSDRConnection } from "@/hooks/useSDRConnection";
import { useAudioPlayback } from "@/hooks/useAudioPlayback";
import { useSpectrumData } from "@/hooks/useSpectrumData";
import { SpectrumDisplay } from "@/components/SpectrumDisplay";
import { StatusBar } from "@/components/StatusBar";
import { DEFAULT_FREQUENCY, DEFAULT_SAMPLE_RATE } from "@/lib/constants";

const SPECTRUM_HEIGHT = 320;

export default function Home() {
  const { connected, fftData, status, sendCommand } = useSDRConnection();
  const { playing, togglePlayback } = useAudioPlayback();
  const smoothedData = useSpectrumData(fftData);

  const containerRef = useRef<HTMLDivElement>(null);
  const [canvasWidth, setCanvasWidth] = useState(800);

  const updateWidth = useCallback(() => {
    if (containerRef.current) {
      setCanvasWidth(containerRef.current.clientWidth);
    }
  }, []);

  useEffect(() => {
    updateWidth();
    window.addEventListener("resize", updateWidth);
    return () => window.removeEventListener("resize", updateWidth);
  }, [updateWidth]);

  const frequency = status?.frequency ?? DEFAULT_FREQUENCY;
  const sampleRate = status?.sampleRate ?? DEFAULT_SAMPLE_RATE;

  const handleFrequencyChange = useCallback(
    (delta: number) => {
      sendCommand("set_frequency", { value: frequency + delta });
    },
    [sendCommand, frequency]
  );

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex items-center justify-between border-b border-gray-800 px-6 py-3">
        <h1 className="text-xl font-bold tracking-tight text-white">
          SDR-Web
        </h1>
        <StatusBar
          connected={connected}
          frequency={frequency}
          sampleRate={sampleRate}
        />
      </header>

      <main className="flex flex-1 flex-col gap-4 p-6">
        <div ref={containerRef} className="w-full">
          <SpectrumDisplay
            fftData={smoothedData}
            width={canvasWidth}
            height={SPECTRUM_HEIGHT}
          />
        </div>

        <div className="rounded bg-gray-900 p-4 text-center text-sm text-gray-500">
          Waterfall display (Phase 3)
        </div>

        <div className="flex items-center justify-center gap-4">
          <button
            onClick={() => handleFrequencyChange(-100_000)}
            className="rounded bg-gray-800 px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          >
            -100 kHz
          </button>

          <span className="font-mono text-lg text-cyan-400">
            {(frequency / 1_000_000).toFixed(3)} MHz
          </span>

          <button
            onClick={() => handleFrequencyChange(100_000)}
            className="rounded bg-gray-800 px-3 py-1.5 text-sm text-gray-300 hover:bg-gray-700"
          >
            +100 kHz
          </button>
        </div>

        <div className="flex justify-center">
          <button
            onClick={togglePlayback}
            className={`rounded px-6 py-2 text-sm font-medium ${
              playing
                ? "bg-red-600 text-white hover:bg-red-700"
                : "bg-cyan-600 text-white hover:bg-cyan-700"
            }`}
          >
            {playing ? "Stop" : "Play"} (Phase 2)
          </button>
        </div>
      </main>
    </div>
  );
}
