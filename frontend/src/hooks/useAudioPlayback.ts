"use client";

import { useCallback, useEffect, useRef, useState } from "react";

const SAMPLE_RATE = 48000;
const BUFFER_CAPACITY = 48000; // 1 second of audio
const SCRIPT_PROCESSOR_BUFFER_SIZE = 4096;

class RingBuffer {
  private readonly buffer: Float32Array;
  private readPos: number = 0;
  private writePos: number = 0;
  private count: number = 0;

  constructor(readonly capacity: number) {
    this.buffer = new Float32Array(capacity);
  }

  write(data: Float32Array): number {
    const available = this.capacity - this.count;
    const toWrite = Math.min(data.length, available);

    for (let i = 0; i < toWrite; i++) {
      this.buffer[this.writePos] = data[i];
      this.writePos = (this.writePos + 1) % this.capacity;
    }
    this.count += toWrite;

    return toWrite;
  }

  read(output: Float32Array): number {
    const toRead = Math.min(output.length, this.count);

    for (let i = 0; i < toRead; i++) {
      output[i] = this.buffer[this.readPos];
      this.readPos = (this.readPos + 1) % this.capacity;
    }
    this.count -= toRead;

    // Fill remaining output with silence
    for (let i = toRead; i < output.length; i++) {
      output[i] = 0;
    }

    return toRead;
  }

  get fillLevel(): number {
    return this.count / this.capacity;
  }
}

export interface AudioPlaybackState {
  readonly playing: boolean;
  readonly bufferHealth: number;
  readonly togglePlayback: () => void;
  readonly feedAudio: (pcmData: Int16Array) => void;
}

export function useAudioPlayback(): AudioPlaybackState {
  const [playing, setPlaying] = useState(false);
  const [bufferHealth, setBufferHealth] = useState(0);

  const audioContextRef = useRef<AudioContext | null>(null);
  const processorRef = useRef<ScriptProcessorNode | null>(null);
  const ringBufferRef = useRef<RingBuffer>(new RingBuffer(BUFFER_CAPACITY));
  const healthIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const startHealthReporting = useCallback(() => {
    if (healthIntervalRef.current !== null) return;
    healthIntervalRef.current = setInterval(() => {
      setBufferHealth(ringBufferRef.current.fillLevel);
    }, 100);
  }, []);

  const stopHealthReporting = useCallback(() => {
    if (healthIntervalRef.current !== null) {
      clearInterval(healthIntervalRef.current);
      healthIntervalRef.current = null;
    }
    setBufferHealth(0);
  }, []);

  const feedAudio = useCallback((pcmData: Int16Array) => {
    const floatData = new Float32Array(pcmData.length);
    for (let i = 0; i < pcmData.length; i++) {
      floatData[i] = pcmData[i] / 32768.0;
    }
    ringBufferRef.current.write(floatData);
  }, []);

  const togglePlayback = useCallback(() => {
    setPlaying((prev) => {
      if (!prev) {
        // Start playback
        if (audioContextRef.current === null) {
          audioContextRef.current = new AudioContext({ sampleRate: SAMPLE_RATE });
        }

        const ctx = audioContextRef.current;
        void ctx.resume().then(() => {
          const processor = ctx.createScriptProcessor(
            SCRIPT_PROCESSOR_BUFFER_SIZE,
            0,
            1
          );

          processor.onaudioprocess = (event: AudioProcessingEvent) => {
            const output = event.outputBuffer.getChannelData(0);
            ringBufferRef.current.read(output);
          };

          processor.connect(ctx.destination);
          processorRef.current = processor;
        });

        startHealthReporting();
        return true;
      } else {
        // Stop playback
        if (processorRef.current !== null) {
          processorRef.current.disconnect();
          processorRef.current = null;
        }

        const ctx = audioContextRef.current;
        if (ctx !== null) {
          void ctx.suspend();
        }

        stopHealthReporting();
        return false;
      }
    });
  }, [startHealthReporting, stopHealthReporting]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stopHealthReporting();
      if (processorRef.current !== null) {
        processorRef.current.disconnect();
        processorRef.current = null;
      }
      if (audioContextRef.current !== null) {
        void audioContextRef.current.close();
        audioContextRef.current = null;
      }
    };
  }, [stopHealthReporting]);

  return { playing, bufferHealth, togglePlayback, feedAudio };
}
