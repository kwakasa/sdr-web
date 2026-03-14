"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { DEFAULT_WS_URL } from "@/lib/constants";
import {
  MSG_FFT,
  MSG_STATUS,
  parseFrame,
  encodeCommand,
  type StatusMessage,
} from "@/lib/protocol";

const RECONNECT_DELAY_MS = 2000;

interface SDRConnectionState {
  readonly connected: boolean;
  readonly fftData: Uint8Array | null;
  readonly status: StatusMessage | null;
  readonly sendCommand: (
    command: string,
    params: Record<string, unknown>
  ) => void;
}

export function useSDRConnection(
  url: string = DEFAULT_WS_URL
): SDRConnectionState {
  const [connected, setConnected] = useState(false);
  const [fftData, setFftData] = useState<Uint8Array | null>(null);
  const [status, setStatus] = useState<StatusMessage | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const connect = useCallback(() => {
    if (!mountedRef.current) return;

    clearReconnectTimer();

    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    wsRef.current = ws;

    ws.onopen = () => {
      if (mountedRef.current) {
        setConnected(true);
      }
    };

    ws.onclose = () => {
      if (mountedRef.current) {
        setConnected(false);
        reconnectTimerRef.current = setTimeout(connect, RECONNECT_DELAY_MS);
      }
    };

    ws.onerror = () => {
      ws.close();
    };

    ws.onmessage = (event: MessageEvent) => {
      if (!mountedRef.current) return;

      if (event.data instanceof ArrayBuffer) {
        try {
          const frame = parseFrame(event.data);

          if (frame.type === MSG_FFT) {
            setFftData(frame.payload);
          } else if (frame.type === MSG_STATUS) {
            const decoder = new TextDecoder();
            const json = decoder.decode(frame.payload);
            const parsed = JSON.parse(json) as StatusMessage;
            setStatus(parsed);
          }
        } catch (err) {
          console.error("Failed to parse WebSocket frame:", err);
        }
      }
    };
  }, [url, clearReconnectTimer]);

  useEffect(() => {
    mountedRef.current = true;
    connect();

    return () => {
      mountedRef.current = false;
      clearReconnectTimer();
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connect, clearReconnectTimer]);

  const sendCommand = useCallback(
    (command: string, params: Record<string, unknown>) => {
      const ws = wsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(encodeCommand(command, params));
      }
    },
    []
  );

  return { connected, fftData, status, sendCommand };
}
