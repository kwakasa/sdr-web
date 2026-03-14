# SDR-Web

Web-based Software Defined Radio application. Browser-native alternative to SDR++.

## Architecture

Rust backend connects to `rtl_tcp`, performs server-side DSP (FFT, FM demodulation), and streams processed data to browsers via WebSocket (~50 KB/s per client vs ~4 MB/s raw IQ).

```
RTL-SDR → rtl_tcp → [Rust Backend] → WebSocket → [Browser]
                      ├── FFT → spectrum/waterfall
                      └── FM demod → audio PCM
```

## Projects

### backend/ (Rust)

**Commands:**
- Build: `cd backend && cargo build --release`
- Test: `cd backend && cargo test`
- Run: `cd backend && cargo run --release -- --rtl-host 127.0.0.1 --frequency 90100000`
- Format: `cd backend && cargo fmt`
- Lint: `cd backend && cargo clippy`

**CLI args:** `--rtl-host`, `--rtl-port` (1234), `--ws-port` (8080), `--frequency` (Hz), `--sample-rate` (2048000)

### frontend/ (Next.js 15)

**Commands:**
- Dev: `cd frontend && npm run dev --turbopack`
- Build: `cd frontend && npm run build`
- Lint: `cd frontend && npm run lint`

## WebSocket Protocol

Single connection on `ws://localhost:8080`, multiplexed:
- Binary frames: 1-byte type + payload (0x01=FFT, 0x02=Audio, 0x03=Status)
- Text frames: JSON control commands (`set_frequency`, `set_gain`, `set_agc`)

## DSP

Custom ~200-line DSP implementation. Only external DSP dep: `rustfft` + `num-complex`.
No `rustradio` -- its GNURadio-style block-graph framework is overkill for a fixed WFM pipeline.
