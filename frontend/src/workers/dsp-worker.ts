// Web Worker for WASM-based DSP processing.
// Runs FFT and FM demodulation off the main thread.

interface SdrProcessorLike {
  compute_fft(iq_data: Uint8Array): Uint8Array;
  demodulate_audio(iq_data: Uint8Array): Float32Array;
  free(): void;
}

let processor: SdrProcessorLike | null = null;
let fftFrameCount = 0;
let fftInterval = 0; // disabled by default; UI can enable by setting a positive interval

self.onmessage = async (e: MessageEvent) => {
  const { type, data } = e.data;

  switch (type) {
    case "init": {
      try {
        // Dynamic import of WASM module
        const wasm = await import("../wasm/sdr_web_wasm_dsp");
        const wasmAssetUrl = new URL(
          "../wasm/sdr_web_wasm_dsp_bg.wasm",
          import.meta.url
        ).toString();
        const wasmUrl = wasmAssetUrl.startsWith("/")
          ? new URL(wasmAssetUrl, self.location.origin)
          : wasmAssetUrl;
        await wasm.default(wasmUrl);
        processor?.free();
        processor = new wasm.SdrProcessor(
          data?.fftSize ?? 2048,
          data?.deemphasisTcUs ?? 50.0
        );
        fftInterval = data?.fftInterval ?? 0;
        fftFrameCount = 0;
        self.postMessage({ type: "ready" });
      } catch (error) {
        const message =
          error instanceof Error ? error.message : "Unknown WASM init failure";
        self.postMessage({ type: "error", data: message });
      }
      break;
    }

    case "iq_data": {
      if (!processor) return;
      const iqData = new Uint8Array(data);

      // FFT is disabled by default. Only compute when a positive interval is set.
      if (fftInterval > 0) {
        fftFrameCount++;
        if (fftFrameCount >= fftInterval) {
          const fftData = processor.compute_fft(iqData);
          self.postMessage({ type: "fft", data: fftData });
          fftFrameCount = 0;
        }
      }

      // Audio demodulation every chunk
      const audioData = processor.demodulate_audio(iqData);
      self.postMessage(
        { type: "audio", data: audioData },
        { transfer: [audioData.buffer] }
      );
      break;
    }

    case "set_fft_interval": {
      fftInterval = Math.max(0, Number(data) || 0);
      fftFrameCount = 0;
      break;
    }
  }
};
