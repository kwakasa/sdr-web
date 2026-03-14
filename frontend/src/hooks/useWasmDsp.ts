"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { DSP_FFT_INTERVAL, FFT_SIZE } from "@/lib/constants";

interface WasmDspOptions {
  readonly onFftData?: (data: Uint8Array) => void;
  readonly onAudioData?: (data: Float32Array) => void;
  readonly fftSize?: number;
  readonly deemphasisTcUs?: number;
}

interface WasmDspState {
  readonly ready: boolean;
  readonly sendIqData: (data: Uint8Array) => void;
}

export function useWasmDsp(options: WasmDspOptions): WasmDspState {
  const [ready, setReady] = useState(false);
  const workerRef = useRef<Worker | null>(null);

  // Store callbacks in refs to avoid re-creating worker on callback change
  const onFftDataRef = useRef(options.onFftData);
  onFftDataRef.current = options.onFftData;

  const onAudioDataRef = useRef(options.onAudioData);
  onAudioDataRef.current = options.onAudioData;

  const fftSize = options.fftSize ?? FFT_SIZE;
  const deemphasisTcUs = options.deemphasisTcUs ?? 50.0;

  useEffect(() => {
    const worker = new Worker(
      new URL("../workers/dsp-worker.ts", import.meta.url)
    );
    workerRef.current = worker;

    worker.onmessage = (e: MessageEvent) => {
      const { type, data } = e.data;

      switch (type) {
        case "ready":
          setReady(true);
          break;
        case "fft": {
          const cb = onFftDataRef.current;
          if (cb) cb(data as Uint8Array);
          break;
        }
        case "audio": {
          const cb = onAudioDataRef.current;
          if (cb) cb(data as Float32Array);
          break;
        }
      }
    };

    worker.onerror = (err) => {
      console.error("DSP worker error:", err);
    };

    // Initialize WASM in the worker
    worker.postMessage({
      type: "init",
      data: {
        fftSize,
        deemphasisTcUs,
        fftInterval: DSP_FFT_INTERVAL,
      },
    });

    return () => {
      worker.terminate();
      workerRef.current = null;
      setReady(false);
    };
  }, [fftSize, deemphasisTcUs]);

  const sendIqData = useCallback((data: Uint8Array) => {
    const worker = workerRef.current;
    if (!worker) return;

    // Transfer the buffer (zero-copy) by copying first to detach
    const copy = new Uint8Array(data);
    worker.postMessage(
      { type: "iq_data", data: copy.buffer },
      [copy.buffer]
    );
  }, []);

  return { ready, sendIqData };
}
