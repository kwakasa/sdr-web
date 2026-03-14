"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useSDRConnection } from "@/hooks/useSDRConnection";
import { useAudioPlayback } from "@/hooks/useAudioPlayback";
import { useSpectrumData } from "@/hooks/useSpectrumData";
import { useWasmDsp } from "@/hooks/useWasmDsp";
import { SpectrumDisplay } from "@/components/SpectrumDisplay";
import { WaterfallDisplay } from "@/components/WaterfallDisplay";
import { FrequencyControl } from "@/components/FrequencyControl";
import { GainControl } from "@/components/GainControl";
import { StatusBar } from "@/components/StatusBar";
import {
  DEFAULT_FREQUENCY,
  DEFAULT_SAMPLE_RATE,
  MIN_FREQUENCY_HZ,
  MAX_FREQUENCY_HZ,
} from "@/lib/constants";

const SPECTRUM_HEIGHT = 300;
const WATERFALL_HEIGHT = 200;

function clampFrequency(hz: number): number {
  return Math.max(MIN_FREQUENCY_HZ, Math.min(MAX_FREQUENCY_HZ, hz));
}

function getAudioStatusText(playing: boolean, bufferHealth: number): string {
  if (!playing) return "Stopped";
  if (bufferHealth < 0.02) return "Buffering...";
  return "Playing";
}

export default function Home() {
  const { playing, bufferHealth, togglePlayback, feedAudioFloat } =
    useAudioPlayback();

  // FFT data from WASM DSP, stored as state for spectrum/waterfall
  const [fftData, setFftData] = useState<Uint8Array | null>(null);

  // WASM DSP hook: processes raw IQ into FFT + audio
  const dspCallbacks = useMemo(
    () => ({
      onFftData: (data: Uint8Array) => setFftData(data),
      onAudioData: feedAudioFloat,
    }),
    [feedAudioFloat]
  );

  const { ready: wasmReady, sendIqData } = useWasmDsp(dspCallbacks);

  // WebSocket connection: receives raw IQ and forwards to WASM worker
  const connectionOptions = useMemo(
    () => ({ onIqData: sendIqData }),
    [sendIqData]
  );

  const { connected, reconnecting, status, sendCommand } =
    useSDRConnection(connectionOptions);

  const smoothedData = useSpectrumData(fftData);

  // Local frequency/gain/agc state, synced from server status
  const [frequency, setFrequency] = useState(DEFAULT_FREQUENCY);
  const [gain, setGain] = useState(0);
  const [agcEnabled, setAgcEnabled] = useState(false);

  // Sync state from server status messages
  useEffect(() => {
    if (status === null) return;
    if (status.frequency !== undefined) setFrequency(status.frequency);
    if (status.gain !== undefined) setGain(status.gain);
    if (status.agcEnabled !== undefined) setAgcEnabled(status.agcEnabled);
  }, [status]);

  const sampleRate = status?.sampleRate ?? DEFAULT_SAMPLE_RATE;
  const tunerType = status?.tunerType;

  const handleFrequencyChange = useCallback(
    (freqHz: number) => {
      const clamped = clampFrequency(freqHz);
      setFrequency(clamped);
      sendCommand("set_frequency", { value: clamped });
    },
    [sendCommand]
  );

  const handleGainChange = useCallback(
    (newGain: number) => {
      setGain(newGain);
      sendCommand("set_gain", { value: newGain });
    },
    [sendCommand]
  );

  const handleAgcToggle = useCallback(
    (enabled: boolean) => {
      setAgcEnabled(enabled);
      sendCommand("set_agc", { value: enabled });
    },
    [sendCommand]
  );

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if user is typing in an input field
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      switch (e.key) {
        case "ArrowUp":
          e.preventDefault();
          handleFrequencyChange(frequency + 100_000);
          break;
        case "ArrowDown":
          e.preventDefault();
          handleFrequencyChange(frequency - 100_000);
          break;
        case "PageUp":
          e.preventDefault();
          handleFrequencyChange(frequency + 1_000_000);
          break;
        case "PageDown":
          e.preventDefault();
          handleFrequencyChange(frequency - 1_000_000);
          break;
        case " ":
          e.preventDefault();
          togglePlayback();
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [frequency, handleFrequencyChange, togglePlayback]);

  const audioStatusText = getAudioStatusText(playing, bufferHealth);

  return (
    <div className="flex min-h-screen flex-col">
      {/* Status Bar */}
      <header className="flex items-center justify-between border-b border-gray-800 px-6 py-3">
        <h1 className="text-xl font-bold tracking-tight text-white">
          SDR-Web
        </h1>
        <StatusBar
          connected={connected}
          reconnecting={reconnecting}
          frequency={frequency}
          sampleRate={sampleRate}
          audioPlaying={playing}
          bufferHealth={bufferHealth}
          tunerType={tunerType}
          wasmReady={wasmReady}
        />
      </header>

      <main className="flex flex-1 flex-col gap-4 p-4 lg:p-6">
        {/* Controls Row: Frequency + Gain */}
        <div className="flex flex-wrap items-center justify-between gap-4 rounded bg-gray-900/50 px-4 py-3">
          <FrequencyControl
            frequency={frequency}
            onFrequencyChange={handleFrequencyChange}
          />
          <GainControl
            gain={gain}
            agcEnabled={agcEnabled}
            onGainChange={handleGainChange}
            onAgcToggle={handleAgcToggle}
          />
        </div>

        {/* Spectrum Display */}
        <SpectrumDisplay
          fftData={smoothedData}
          centerFrequency={frequency}
          sampleRate={sampleRate}
          height={SPECTRUM_HEIGHT}
        />

        {/* Waterfall Display */}
        <WaterfallDisplay
          fftData={smoothedData}
          centerFrequency={frequency}
          sampleRate={sampleRate}
          height={WATERFALL_HEIGHT}
        />

        {/* Playback Controls */}
        <div className="flex items-center justify-center gap-4 rounded bg-gray-900/50 px-4 py-3">
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
            <span className="font-mono text-xs text-gray-500">
              {Math.round(bufferHealth * 100)}%
            </span>
          </div>
        </div>
      </main>
    </div>
  );
}
