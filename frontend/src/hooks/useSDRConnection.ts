"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  DEFAULT_WS_URL,
  RECONNECT_BASE_DELAY_MS,
  RECONNECT_MAX_DELAY_MS,
} from "@/lib/constants";
import {
  MSG_RAW_IQ,
  MSG_STATUS,
  parseFrame,
  encodeCommand,
  type StatusMessage,
} from "@/lib/protocol";

interface SDRConnectionOptions {
  readonly onIqData?: (data: Uint8Array) => void;
  readonly url?: string;
}

interface SDRConnectionState {
  readonly connected: boolean;
  readonly reconnecting: boolean;
  readonly status: StatusMessage | null;
  readonly sendCommand: (
    command: string,
    params: Record<string, unknown>
  ) => void;
}

export function useSDRConnection(
  options?: SDRConnectionOptions
): SDRConnectionState {
  const url = options?.url ?? DEFAULT_WS_URL;
  const onIqDataRef = useRef(options?.onIqData);
  onIqDataRef.current = options?.onIqData;

  const [connected, setConnected] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [status, setStatus] = useState<StatusMessage | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const backoffRef = useRef(RECONNECT_BASE_DELAY_MS);

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const scheduleReconnect = useCallback(() => {
    if (!mountedRef.current) return;

    setReconnecting(true);
    const delay = backoffRef.current;
    backoffRef.current = Math.min(delay * 2, RECONNECT_MAX_DELAY_MS);

    reconnectTimerRef.current = setTimeout(() => {
      if (mountedRef.current) {
        connectWs();
      }
    }, delay);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const connectWs = useCallback(() => {
    if (!mountedRef.current) return;

    clearReconnectTimer();

    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    wsRef.current = ws;

    ws.onopen = () => {
      if (mountedRef.current) {
        setConnected(true);
        setReconnecting(false);
        backoffRef.current = RECONNECT_BASE_DELAY_MS;
      }
    };

    ws.onclose = () => {
      if (mountedRef.current) {
        setConnected(false);
        scheduleReconnect();
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

          if (frame.type === MSG_RAW_IQ) {
            const callback = onIqDataRef.current;
            if (callback) {
              callback(frame.payload);
            }
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
  }, [url, clearReconnectTimer, scheduleReconnect]);

  useEffect(() => {
    mountedRef.current = true;
    connectWs();

    return () => {
      mountedRef.current = false;
      clearReconnectTimer();
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [connectWs, clearReconnectTimer]);

  const sendCommand = useCallback(
    (command: string, params: Record<string, unknown>) => {
      const ws = wsRef.current;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(encodeCommand(command, params));
      }
    },
    []
  );

  return { connected, reconnecting, status, sendCommand };
}
