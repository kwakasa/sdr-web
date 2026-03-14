"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSDRConnection } from "@/hooks/useSDRConnection";
import { useAudioPlayback } from "@/hooks/useAudioPlayback";
import { useSpectrumData } from "@/hooks/useSpectrumData";
import { SpectrumDisplay } from "@/components/SpectrumDisplay";
import { StatusBar } from "@/components/StatusBar";
import { DEFAULT_FREQUENCY, DEFAULT_SAMPLE_RATE } from "@/lib/constants";

const SPECTRUM_HEIGHT = 320;

function getAudioStatusText(playing: boolean, bufferHealth: number): string {
  if (!playing) return "Stopped";
  if (bufferHealth < 0.02) return "Buffering...";
  return "Playing";
}

export default function Home() {
  const { playing, bufferHealth, togglePlayback, feedAudio } =
    useAudioPlayback();

  const connectionOptions = useMemo(
    () => ({ onAudioData: feedAudio }),
    [feedAudio]
  );

  const { connected, fftData, status, sendCommand } =
    useSDRConnection(connectionOptions);

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

  const audioStatusText = getAudioStatusText(playing, bufferHealth);

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
          audioPlaying={playing}
          bufferHealth={bufferHealth}
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

        <div className="flex items-center justify-center gap-4">
          <button
            onClick={togglePlayback}
            className={`rounded px-6 py-2 text-sm font-medium ${
              playing
                ? "bg-red-600 text-white hover:bg-red-700"
                : "bg-green-600 text-white hover:bg-green-700"
            }`}
          >
            {playing ? "Stop" : "Play"}
          </button>

          <span className="text-sm text-gray-400">
            Audio: {audioStatusText}
          </span>

          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">Buffer</span>
            <div className="h-2 w-16 overflow-hidden rounded-full bg-gray-800">
              <div
                className={`h-full transition-all ${
                  bufferHealth > 0.3
                    ? "bg-green-500"
                    : bufferHealth > 0.1
                      ? "bg-yellow-500"
                      : "bg-red-500"
                }`}
                style={{ width: `${Math.round(bufferHealth * 100)}%` }}
              />
            </div>
            <span className="text-xs font-mono text-gray-500">
              {Math.round(bufferHealth * 100)}%
            </span>
          </div>
        </div>
      </main>
    </div>
  );
}
