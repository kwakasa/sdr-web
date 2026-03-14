"use client";

import { useRef } from "react";

const DEFAULT_ALPHA = 0.3;

export function useSpectrumData(
  rawData: Uint8Array | null,
  alpha: number = DEFAULT_ALPHA
): Uint8Array | null {
  const smoothedRef = useRef<Uint8Array | null>(null);

  if (rawData === null) {
    return null;
  }

  const prev = smoothedRef.current;

  if (prev === null || prev.length !== rawData.length) {
    const copy = new Uint8Array(rawData);
    smoothedRef.current = copy;
    return copy;
  }

  const result = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; i++) {
    result[i] = Math.round(alpha * rawData[i] + (1 - alpha) * prev[i]);
  }
  smoothedRef.current = result;

  return result;
}
