// Web Worker for WASM-based DSP processing.
// Runs FFT and FM demodulation off the main thread.

interface SdrProcessorLike {
  compute_fft(iq_data: Uint8Array): Uint8Array;
  demodulate_audio(iq_data: Uint8Array): Float32Array;
  free(): void;
}

let processor: SdrProcessorLike | null = null;
let fftFrameCount = 0;
let fftInterval = 50; // compute FFT every N chunks

self.onmessage = async (e: MessageEvent) => {
  const { type, data } = e.data;

  switch (type) {
    case "init": {
      // Dynamic import of WASM module
      const wasm = await import("../wasm/sdr_web_wasm_dsp");
      await wasm.default();
      processor = new wasm.SdrProcessor(
        data?.fftSize ?? 2048,
        data?.deemphasisTcUs ?? 50.0
      );
      fftInterval = data?.fftInterval ?? 50;
      self.postMessage({ type: "ready" });
      break;
    }

    case "iq_data": {
      if (!processor) return;
      const iqData = new Uint8Array(data);

      // FFT at reduced rate (~20 fps)
      fftFrameCount++;
      if (fftFrameCount >= fftInterval) {
        const fftData = processor.compute_fft(iqData);
        self.postMessage({ type: "fft", data: fftData });
        fftFrameCount = 0;
      }

      // Audio demodulation every chunk
      const audioData = processor.demodulate_audio(iqData);
      // Transfer the buffer (zero-copy)
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (self.postMessage as any)(
        { type: "audio", data: audioData },
        [audioData.buffer]
      );
      break;
    }

    case "set_fft_interval": {
      fftInterval = data;
      break;
    }
  }
};
