"use client";

import { useCallback, useEffect, useRef } from "react";

interface WaterfallDisplayProps {
  readonly fftData: Uint8Array | null;
  readonly centerFrequency: number;
  readonly sampleRate: number;
  readonly height?: number;
}

const LABEL_AREA_HEIGHT = 24;
const BACKGROUND_COLOR = "#030712";
const LABEL_COLOR = "rgba(156, 163, 175, 0.8)";
const GRID_COLOR = "rgba(75, 85, 99, 0.25)";

function buildColorLUT(): Uint8Array {
  const lut = new Uint8Array(256 * 3);

  for (let i = 0; i < 256; i++) {
    const idx = i * 3;
    if (i < 64) {
      // Dark blue to cyan
      const t = i / 64;
      lut[idx] = 0;
      lut[idx + 1] = Math.round(t * 200);
      lut[idx + 2] = Math.round(40 + t * 215);
    } else if (i < 128) {
      // Cyan to green
      const t = (i - 64) / 64;
      lut[idx] = 0;
      lut[idx + 1] = Math.round(200 + t * 55);
      lut[idx + 2] = Math.round(255 - t * 255);
    } else if (i < 192) {
      // Green to yellow
      const t = (i - 128) / 64;
      lut[idx] = Math.round(t * 255);
      lut[idx + 1] = 255;
      lut[idx + 2] = 0;
    } else {
      // Yellow to red
      const t = (i - 192) / 63;
      lut[idx] = 255;
      lut[idx + 1] = Math.round(255 - t * 255);
      lut[idx + 2] = 0;
    }
  }

  return lut;
}

function formatFreqLabel(hz: number): string {
  const mhz = hz / 1_000_000;
  if (mhz >= 1000) return `${mhz.toFixed(0)}`;
  if (mhz >= 100) return `${mhz.toFixed(1)}`;
  return `${mhz.toFixed(2)}`;
}

export function WaterfallDisplay({
  fftData,
  centerFrequency,
  sampleRate,
  height = 200,
}: WaterfallDisplayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const colorLUTRef = useRef<Uint8Array>(buildColorLUT());
  const widthRef = useRef(800);
  const prevDataRef = useRef<Uint8Array | null>(null);

  // Handle responsive width
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        widthRef.current = entry.contentRect.width;
        const canvas = canvasRef.current;
        if (canvas && canvas.width !== Math.floor(entry.contentRect.width)) {
          canvas.width = Math.floor(entry.contentRect.width);
        }
      }
    });

    observer.observe(container);
    widthRef.current = container.clientWidth;

    return () => observer.disconnect();
  }, []);

  const drawLabels = useCallback(
    (ctx: CanvasRenderingContext2D, w: number, drawH: number) => {
      const startFreq = centerFrequency - sampleRate / 2;
      const endFreq = centerFrequency + sampleRate / 2;
      const freqRange = endFreq - startFreq;

      const labelInterval = 500_000;
      const firstLabel = Math.ceil(startFreq / labelInterval) * labelInterval;

      ctx.fillStyle = BACKGROUND_COLOR;
      ctx.fillRect(0, drawH, w, LABEL_AREA_HEIGHT);

      ctx.strokeStyle = GRID_COLOR;
      ctx.lineWidth = 1;

      ctx.fillStyle = LABEL_COLOR;
      ctx.font = "11px ui-monospace, monospace";
      ctx.textAlign = "center";

      for (let freq = firstLabel; freq <= endFreq; freq += labelInterval) {
        const x = ((freq - startFreq) / freqRange) * w;
        // Vertical grid line through waterfall
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, drawH);
        ctx.stroke();

        ctx.fillText(`${formatFreqLabel(freq)}`, x, drawH + 16);
      }

      ctx.textAlign = "right";
      ctx.fillText("MHz", w - 2, drawH + 16);
    },
    [centerFrequency, sampleRate]
  );

  // Draw new waterfall row when FFT data changes
  useEffect(() => {
    if (fftData === null) return;

    // Only draw when data actually changes (reference check)
    if (fftData === prevDataRef.current) return;
    prevDataRef.current = fftData;

    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const drawH = height - LABEL_AREA_HEIGHT;
    if (w <= 0 || drawH <= 0) return;

    const lut = colorLUTRef.current;

    // Shift existing content down by 1 pixel (within waterfall area only)
    const existingImage = ctx.getImageData(0, 0, w, drawH);
    ctx.putImageData(existingImage, 0, 1);

    // Create new row at top
    const rowData = ctx.createImageData(w, 1);
    const pixels = rowData.data;
    const binCount = fftData.length;

    for (let x = 0; x < w; x++) {
      const binIndex = Math.floor((x / w) * binCount);
      const value = fftData[Math.min(binIndex, binCount - 1)];
      const lutIdx = value * 3;

      const pixelIdx = x * 4;
      pixels[pixelIdx] = lut[lutIdx];
      pixels[pixelIdx + 1] = lut[lutIdx + 1];
      pixels[pixelIdx + 2] = lut[lutIdx + 2];
      pixels[pixelIdx + 3] = 255;
    }

    ctx.putImageData(rowData, 0, 0);

    // Redraw frequency labels
    drawLabels(ctx, w, drawH);
  }, [fftData, height, drawLabels]);

  // Initial canvas setup
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = widthRef.current;
    canvas.width = w;
    canvas.height = height;

    ctx.fillStyle = BACKGROUND_COLOR;
    ctx.fillRect(0, 0, w, height);

    drawLabels(ctx, w, height - LABEL_AREA_HEIGHT);
  }, [height, drawLabels]);

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
