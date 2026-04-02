export const MSG_RAW_IQ = 0x01;
export const MSG_STATUS = 0x03;

export interface ControlCommand {
  readonly command: string;
  readonly params: Record<string, unknown>;
}

export interface StatusMessage {
  readonly frequency: number;
  readonly sampleRate: number;
  readonly gain: number;
  readonly agcEnabled?: boolean;
  readonly tunerType?: number;
  readonly gainCount?: number;
}

/** Raw status JSON from the backend (snake_case field names). */
interface RawStatusMessage {
  readonly frequency?: number;
  readonly sample_rate?: number;
  readonly gain?: number;
  readonly agc_enabled?: boolean;
  readonly tuner_type?: number;
  readonly gain_count?: number;
}

/** Map backend snake_case status to frontend camelCase StatusMessage. */
export function parseStatusMessage(json: string): StatusMessage {
  const raw: RawStatusMessage = JSON.parse(json);
  return {
    frequency: raw.frequency ?? 0,
    sampleRate: raw.sample_rate ?? 0,
    gain: raw.gain ?? 0,
    agcEnabled: raw.agc_enabled,
    tunerType: raw.tuner_type,
    gainCount: raw.gain_count,
  };
}

export interface ParsedFrame {
  readonly type: number;
  readonly payload: Uint8Array;
}

export function parseFrame(data: ArrayBuffer): ParsedFrame {
  const bytes = new Uint8Array(data);
  if (bytes.length < 1) {
    throw new Error("Empty frame received");
  }
  return {
    type: bytes[0],
    payload: bytes.subarray(1),
  };
}

export function encodeCommand(
  command: string,
  params: Record<string, unknown>
): string {
  const msg: ControlCommand = { command, params };
  return JSON.stringify(msg);
}
