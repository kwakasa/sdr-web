"use client";

import { useCallback, useEffect, useRef } from "react";

interface SpectrumDisplayProps {
  readonly fftData: Uint8Array | null;
  readonly width: number;
  readonly height: number;
}

const BACKGROUND_COLOR = "#030712";
const GRID_COLOR = "rgba(75, 85, 99, 0.4)";
const SPECTRUM_COLOR = "#22d3ee";
const GRID_LINES_HORIZONTAL = 8;
const GRID_LINES_VERTICAL = 10;

export function SpectrumDisplay({
  fftData,
  width,
  height,
}: SpectrumDisplayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>(0);
  const dataRef = useRef<Uint8Array | null>(null);

  dataRef.current = fftData;

  const drawGrid = useCallback(
    (ctx: CanvasRenderingContext2D, w: number, h: number) => {
      ctx.strokeStyle = GRID_COLOR;
      ctx.lineWidth = 1;

      for (let i = 1; i < GRID_LINES_HORIZONTAL; i++) {
        const y = (h / GRID_LINES_HORIZONTAL) * i;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
      }

      for (let i = 1; i < GRID_LINES_VERTICAL; i++) {
        const x = (w / GRID_LINES_VERTICAL) * i;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, h);
        ctx.stroke();
      }
    },
    []
  );

  const drawSpectrum = useCallback(
    (ctx: CanvasRenderingContext2D, data: Uint8Array, w: number, h: number) => {
      const binCount = data.length;
      const binWidth = w / binCount;

      ctx.strokeStyle = SPECTRUM_COLOR;
      ctx.lineWidth = 1.5;
      ctx.beginPath();

      for (let i = 0; i < binCount; i++) {
        const x = i * binWidth;
        const magnitude = data[i] / 255;
        const y = h - magnitude * h;

        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }

      ctx.stroke();

      const gradient = ctx.createLinearGradient(0, 0, 0, h);
      gradient.addColorStop(0, "rgba(34, 211, 238, 0.15)");
      gradient.addColorStop(1, "rgba(34, 211, 238, 0)");
      ctx.fillStyle = gradient;

      ctx.lineTo((binCount - 1) * binWidth, h);
      ctx.lineTo(0, h);
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
      ctx.fillStyle = BACKGROUND_COLOR;
      ctx.fillRect(0, 0, width, height);

      drawGrid(ctx, width, height);

      const data = dataRef.current;
      if (data !== null) {
        drawSpectrum(ctx, data, width, height);
      }

      animationRef.current = requestAnimationFrame(render);
    };

    animationRef.current = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animationRef.current);
    };
  }, [width, height, drawGrid, drawSpectrum]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className="block rounded border border-gray-800"
    />
  );
}
