export const MSG_FFT = 0x01;
export const MSG_AUDIO = 0x02;
export const MSG_STATUS = 0x03;

export interface ControlCommand {
  readonly command: string;
  readonly params: Record<string, unknown>;
}

export interface StatusMessage {
  readonly frequency: number;
  readonly sampleRate: number;
  readonly gain: number;
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
    payload: bytes.slice(1),
  };
}

export function encodeCommand(
  command: string,
  params: Record<string, unknown>
): string {
  const msg: ControlCommand = { command, params };
  return JSON.stringify(msg);
}
