"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  getDefaultWebSocketUrl,
  RECONNECT_BASE_DELAY_MS,
  RECONNECT_MAX_DELAY_MS,
} from "@/lib/constants";
import {
  MSG_RAW_IQ,
  MSG_STATUS,
  parseFrame,
  encodeCommand,
  parseStatusMessage,
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
  const resolvedUrl = options?.url || getDefaultWebSocketUrl();
  const urlRef = useRef(resolvedUrl);
  urlRef.current = resolvedUrl;

  const onIqDataRef = useRef(options?.onIqData);
  onIqDataRef.current = options?.onIqData;

  const [connected, setConnected] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [status, setStatus] = useState<StatusMessage | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(true);
  const backoffRef = useRef(RECONNECT_BASE_DELAY_MS);
  const textDecoderRef = useRef(new TextDecoder());
  const connectWsRef = useRef<() => void>(() => {});

  const clearReconnectTimer = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }, []);

  const scheduleReconnect = useCallback(() => {
    if (!mountedRef.current || reconnectTimerRef.current !== null) return;

    setReconnecting(true);
    const delay = backoffRef.current;
    backoffRef.current = Math.min(delay * 2, RECONNECT_MAX_DELAY_MS);

    reconnectTimerRef.current = setTimeout(() => {
      reconnectTimerRef.current = null;
      if (mountedRef.current) {
        connectWsRef.current();
      }
    }, delay);
  }, []);

  const connectWs = useCallback(() => {
    if (!mountedRef.current) return;

    clearReconnectTimer();

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    const ws = new WebSocket(urlRef.current);
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
      if (wsRef.current === ws) {
        wsRef.current = null;
      }

      if (mountedRef.current) {
        setConnected(false);
        scheduleReconnect();
      }
    };

    ws.onerror = () => {
      ws.close();
    };

    ws.onmessage = (event: MessageEvent) => {
      if (!mountedRef.current || !(event.data instanceof ArrayBuffer)) return;

      try {
        const frame = parseFrame(event.data);

        if (frame.type === MSG_RAW_IQ) {
          onIqDataRef.current?.(frame.payload);
          return;
        }

        if (frame.type === MSG_STATUS) {
          const json = textDecoderRef.current.decode(frame.payload);
          setStatus(parseStatusMessage(json));
        }
      } catch (err) {
        console.error("Failed to parse WebSocket frame:", err);
      }
    };
  }, [clearReconnectTimer, scheduleReconnect]);

  connectWsRef.current = connectWs;

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

  // Reconnect when the URL changes
  const prevUrlRef = useRef(resolvedUrl);
  useEffect(() => {
    if (prevUrlRef.current !== resolvedUrl) {
      prevUrlRef.current = resolvedUrl;
      backoffRef.current = RECONNECT_BASE_DELAY_MS;
      connectWs();
    }
  }, [resolvedUrl, connectWs]);

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
