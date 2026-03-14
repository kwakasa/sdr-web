# SDR-Web

Web-based Software Defined Radio. Browser-native alternative to [SDR++](https://github.com/AlexandreRouma/SDRPlusPlus).

## What it does

Connects to an RTL-SDR dongle via `rtl_tcp`, streams raw IQ data to the browser, and performs all signal processing (FFT, FM demodulation) client-side via Rust compiled to WebAssembly.

```
[Machine with SDR dongle]                     [Browser]
  rtl_tcp :1234                                 WASM DSP (Web Worker)
       ↓                                         ├── FFT → spectrum/waterfall
  backend (proxy) :8080  ←── WebSocket ──→        └── WFM demod → audio
  (raw IQ passthrough)
```

The backend is a thin TCP→WebSocket proxy with zero per-client DSP cost. All heavy processing runs in the browser.

## Features

- Real-time spectrum analyzer with frequency axis and peak hold
- Scrolling waterfall spectrogram
- Wideband FM demodulation with browser audio playback
- All DSP in browser via Rust→WASM (FFT, FIR filters, FM discriminator, de-emphasis)
- Frequency tuning and gain control (manual + AGC)
- Keyboard shortcuts (arrow keys for tuning, Space for play/stop)
- Auto-reconnection with exponential backoff
- Multi-client support (proxy scales to many clients)

## Quick Start

### Prerequisites

- RTL-SDR dongle + `rtl_tcp` (from [librtlsdr](https://github.com/steve-m/librtlsdr))
- Rust toolchain (1.75+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (`cargo install wasm-pack`)
- Node.js (20+)

### Run

```bash
# Terminal 1: start rtl_tcp
rtl_tcp -a 127.0.0.1

# Terminal 2: start backend (proxy)
cd backend
cargo run --release -- --frequency 81300000  # J-WAVE 81.3 MHz

# Terminal 3: build WASM + start frontend
wasm-pack build --target web --out-dir ../frontend/src/wasm wasm-dsp
cd frontend
npm install
npm run dev --turbopack
# Open http://localhost:3000
```

### Backend CLI options

| Flag | Default | Description |
|------|---------|-------------|
| `--rtl-host` | 127.0.0.1 | rtl_tcp server host |
| `--rtl-port` | 1234 | rtl_tcp server port |
| `--ws-port` | 8080 | WebSocket server port |
| `--frequency` | 90100000 | Initial frequency (Hz) |
| `--sample-rate` | 2048000 | Sample rate (Hz) |

## Architecture

### Backend -- Proxy (Rust)

Thin TCP→WebSocket bridge. Connects to `rtl_tcp`, streams raw IQ data to browsers, relays control commands back. No DSP.

Key dependencies: `tokio`, `tokio-tungstenite`, `clap`, `tracing`

### wasm-dsp (Rust→WASM)

All DSP runs in the browser via WebAssembly, inside a Web Worker (off the main thread):

```
u8 IQ @ 2.048 Msps (from WebSocket)
  → [FFT branch]   2048-pt FFT → dB magnitude → spectrum/waterfall (20 fps)
  → [Audio branch]  lowpass + decimate ÷8 → 256 kHz
                    → atan2 FM discriminator
                    → lowpass + decimate ÷5 → ~48 kHz
                    → de-emphasis 50μs (Japan)
                    → f32 PCM → Web Audio API
```

~200 lines of custom DSP. Only external DSP dep: `rustfft` + `num-complex`. WASM binary: ~217 KB.

### Frontend (Next.js 15 + React 19)

| Component | Role |
|-----------|------|
| `SpectrumDisplay` | Canvas line graph with frequency axis, peak hold, gradient fill |
| `WaterfallDisplay` | Scrolling spectrogram with color LUT (blue→red) |
| `FrequencyControl` | MHz input + step buttons (±100kHz, ±1MHz) |
| `GainControl` | Gain slider + AGC toggle |
| `useWasmDsp` | Web Worker lifecycle, routes IQ→FFT/audio |
| `useAudioPlayback` | Web Audio API ring buffer, ScriptProcessorNode |
| `useSDRConnection` | WebSocket lifecycle, reconnection, frame parsing |

### WebSocket Protocol

Single connection on `ws://localhost:8080`, multiplexed:

- **Binary frames**: 1-byte type tag + payload
  - `0x01` Raw IQ: uint8 interleaved IQ pairs
  - `0x03` Status: JSON (tuner info, connection state)
- **Text frames**: JSON control commands (`set_frequency`, `set_gain`, `set_agc`)

## Development

```bash
# Build and test everything
cargo test                    # 45 tests (20 backend + 25 wasm-dsp)
cargo fmt && cargo clippy     # format + lint

# Backend
cd backend
RUST_LOG=debug cargo run --release

# WASM DSP
wasm-pack build --target web --out-dir ../frontend/src/wasm wasm-dsp

# Frontend
cd frontend
npm run dev --turbopack       # dev server
npm run build                 # production build
npm run lint                  # lint
```

## License

MIT
