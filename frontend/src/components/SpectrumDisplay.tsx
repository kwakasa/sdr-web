"use client";

import { useCallback, useEffect, useRef } from "react";
import { formatFreqMHzLabel } from "@/lib/constants";

interface SpectrumDisplayProps {
  readonly fftData: Uint8Array | null;
  readonly centerFrequency: number;
  readonly sampleRate: number;
  readonly height?: number;
}

const BACKGROUND_COLOR = "#030712";
const GRID_COLOR = "rgba(75, 85, 99, 0.4)";
const LABEL_COLOR = "rgba(156, 163, 175, 0.8)";
const SPECTRUM_COLOR = "#22d3ee";
const PEAK_COLOR = "rgba(34, 211, 238, 0.4)";
const CENTER_LINE_COLOR = "rgba(234, 179, 8, 0.5)";
const GRID_LINES_HORIZONTAL = 5;
const LABEL_AREA_HEIGHT = 24;
const PEAK_DECAY_RATE = 0.5;

export function SpectrumDisplay({
  fftData,
  centerFrequency,
  sampleRate,
  height = 320,
}: SpectrumDisplayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const animationRef = useRef<number>(0);
  const dataRef = useRef<Uint8Array | null>(null);
  const peakRef = useRef<Float32Array | null>(null);
  const widthRef = useRef(800);

  dataRef.current = fftData;

  // Update peak hold data
  useEffect(() => {
    if (fftData === null) return;

    const prev = peakRef.current;
    if (prev === null || prev.length !== fftData.length) {
      peakRef.current = new Float32Array(fftData);
      return;
    }

    const updated = new Float32Array(prev.length);
    for (let i = 0; i < prev.length; i++) {
      const current = fftData[i];
      const peak = prev[i];
      updated[i] = current > peak ? current : Math.max(peak - PEAK_DECAY_RATE, current);
    }
    peakRef.current = updated;
  }, [fftData]);

  // Handle responsive width
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        widthRef.current = entry.contentRect.width;
        const canvas = canvasRef.current;
        if (canvas) {
          canvas.width = entry.contentRect.width;
        }
      }
    });

    observer.observe(container);
    widthRef.current = container.clientWidth;

    return () => observer.disconnect();
  }, []);

  const drawGrid = useCallback(
    (ctx: CanvasRenderingContext2D, w: number, h: number, drawH: number) => {
      ctx.strokeStyle = GRID_COLOR;
      ctx.lineWidth = 1;

      // Horizontal grid lines (power level)
      for (let i = 1; i <= GRID_LINES_HORIZONTAL; i++) {
        const y = (drawH / (GRID_LINES_HORIZONTAL + 1)) * i;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
      }

      // Vertical grid lines with frequency labels
      const startFreq = centerFrequency - sampleRate / 2;
      const endFreq = centerFrequency + sampleRate / 2;
      const freqRange = endFreq - startFreq;

      // Pick a label interval (~500 kHz)
      const labelInterval = 500_000;
      const firstLabel = Math.ceil(startFreq / labelInterval) * labelInterval;

      ctx.fillStyle = LABEL_COLOR;
      ctx.font = "11px ui-monospace, monospace";
      ctx.textAlign = "center";

      for (let freq = firstLabel; freq <= endFreq; freq += labelInterval) {
        const x = ((freq - startFreq) / freqRange) * w;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, drawH);
        ctx.stroke();

        ctx.fillText(`${formatFreqMHzLabel(freq)}`, x, drawH + 16);
      }

      // "MHz" label at right edge
      ctx.textAlign = "right";
      ctx.fillText("MHz", w - 2, drawH + 16);

      // Center frequency vertical line
      ctx.strokeStyle = CENTER_LINE_COLOR;
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      const centerX = w / 2;
      ctx.beginPath();
      ctx.moveTo(centerX, 0);
      ctx.lineTo(centerX, drawH);
      ctx.stroke();
      ctx.setLineDash([]);
    },
    [centerFrequency, sampleRate]
  );

  const drawSpectrum = useCallback(
    (ctx: CanvasRenderingContext2D, data: Uint8Array, w: number, drawH: number) => {
      const binCount = data.length;
      const binWidth = w / binCount;

      // Draw peak hold line
      const peaks = peakRef.current;
      if (peaks !== null && peaks.length === binCount) {
        ctx.strokeStyle = PEAK_COLOR;
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i < binCount; i++) {
          const x = i * binWidth;
          const magnitude = peaks[i] / 255;
          const y = drawH - magnitude * drawH;
          if (i === 0) {
            ctx.moveTo(x, y);
          } else {
            ctx.lineTo(x, y);
          }
        }
        ctx.stroke();
      }

      // Draw spectrum line
      ctx.strokeStyle = SPECTRUM_COLOR;
      ctx.lineWidth = 1.5;
      ctx.beginPath();

      for (let i = 0; i < binCount; i++) {
        const x = i * binWidth;
        const magnitude = data[i] / 255;
        const y = drawH - magnitude * drawH;
        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      ctx.stroke();

      // Gradient fill below
      const gradient = ctx.createLinearGradient(0, 0, 0, drawH);
      gradient.addColorStop(0, "rgba(34, 211, 238, 0.15)");
      gradient.addColorStop(1, "rgba(34, 211, 238, 0)");
      ctx.fillStyle = gradient;
      ctx.lineTo((binCount - 1) * binWidth, drawH);
      ctx.lineTo(0, drawH);
      ctx.closePath();
      ctx.fill();
    },
    []
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const render = () => {
      const w = widthRef.current;
      const totalH = height;
      const drawH = totalH - LABEL_AREA_HEIGHT;

      canvas.width = w;
      canvas.height = totalH;

      ctx.fillStyle = BACKGROUND_COLOR;
      ctx.fillRect(0, 0, w, totalH);

      drawGrid(ctx, w, totalH, drawH);

      const data = dataRef.current;
      if (data !== null) {
        drawSpectrum(ctx, data, w, drawH);
      }

      animationRef.current = requestAnimationFrame(render);
    };

    animationRef.current = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animationRef.current);
    };
  }, [height, drawGrid, drawSpectrum]);

  return (
    <div ref={containerRef} className="w-full">
      <canvas
        ref={canvasRef}
        height={height}
        className="block w-full rounded border border-gray-800"
      />
    </div>
  );
}
