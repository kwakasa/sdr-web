# SDR-Web

Web-based Software Defined Radio. Browser-native alternative to [SDR++](https://github.com/AlexandreRouma/SDRPlusPlus).

## What it does

Connects to an RTL-SDR dongle via `rtl_tcp`, demodulates Wideband FM, and streams spectrum + audio to your browser in real time.

```
RTL-SDR → rtl_tcp → [Rust Backend] → WebSocket → [Browser]
                      ├── FFT → spectrum/waterfall (~40 KB/s)
                      └── WFM demod → audio PCM (~96 KB/s)
```

Server-side DSP keeps bandwidth to ~50 KB/s per client (vs ~4 MB/s raw IQ).

## Features

- Real-time spectrum analyzer with frequency axis and peak hold
- Scrolling waterfall spectrogram
- Wideband FM demodulation with browser audio playback
- Frequency tuning and gain control (manual + AGC)
- Keyboard shortcuts (arrow keys for tuning, Space for play/stop)
- Auto-reconnection with exponential backoff
- Multi-client support

## Quick Start

### Prerequisites

- RTL-SDR dongle + `rtl_tcp` (from [librtlsdr](https://github.com/steve-m/librtlsdr))
- Rust toolchain (1.75+)
- Node.js (20+)

### Run

```bash
# Terminal 1: start rtl_tcp
rtl_tcp -a 127.0.0.1

# Terminal 2: start backend
cd backend
cargo run --release -- --frequency 81300000  # J-WAVE 81.3 MHz

# Terminal 3: start frontend
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

### Backend (Rust, ~2,100 LOC)

Custom DSP pipeline with only `rustfft` + `num-complex` as external DSP dependencies. No `rustradio` -- its GNURadio-style block-graph framework is overkill for a fixed WFM pipeline.

```
u8 IQ @ 2.048 Msps
  → Complex<f32> conversion
  → [FFT branch]   2048-pt FFT → dB magnitude → uint8 → browser (20 fps)
  → [Audio branch]  lowpass + decimate ÷8 → 256 kHz
                    → atan2 FM discriminator
                    → lowpass + decimate ÷5 → ~48 kHz
                    → de-emphasis 50μs (Japan)
                    → i16 PCM → browser
```

Key dependencies: `tokio`, `tokio-tungstenite`, `rustfft`, `clap`, `tracing`

### Frontend (Next.js 15 + React 19, ~1,350 LOC)

Canvas-based spectrum and waterfall rendering, Web Audio API for playback.

| Component | Role |
|-----------|------|
| `SpectrumDisplay` | Canvas line graph with frequency axis, peak hold, gradient fill |
| `WaterfallDisplay` | Scrolling spectrogram with color LUT (blue→red) |
| `FrequencyControl` | MHz input + step buttons (±100kHz, ±1MHz) |
| `GainControl` | Gain slider + AGC toggle |
| `useAudioPlayback` | Web Audio API ring buffer, ScriptProcessorNode |
| `useSDRConnection` | WebSocket lifecycle, reconnection, frame parsing |

### WebSocket Protocol

Single connection on `ws://localhost:8080`, multiplexed:

- **Binary frames**: 1-byte type tag + payload
  - `0x01` FFT: uint8 magnitude array (2048 bins)
  - `0x02` Audio: little-endian i16 PCM at ~48 kHz mono
  - `0x03` Status: JSON (tuner info, connection state)
- **Text frames**: JSON control commands (`set_frequency`, `set_gain`, `set_agc`)

## Development

```bash
# Backend
cd backend
cargo build          # build
cargo test           # 47 tests
cargo fmt            # format
cargo clippy         # lint
RUST_LOG=debug cargo run --release  # verbose logging

# Frontend
cd frontend
npm run dev --turbopack  # dev server
npm run build            # production build
npm run lint             # lint
```

## License

MIT
