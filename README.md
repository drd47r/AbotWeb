# AbotWeb

Minimal resource website application designed with a retro-cybernetic / technician terminal aesthetic and ultra-low CPU & RAM consumption.

## Tech Stack & Architecture

- **Backend:** [Axum 0.7](https://github.com/tokio-rs/axum) + [Tokio](https://tokio.rs) (async runtime).
- **Template Engine:** [Maud 0.26](https://maud.lambda.xyz) (zero-cost compile-time HTML generation; zero runtime parsing).
- **Hardware Telemetry:** Direct Linux `/proc` filesystem metrics (CPU tick difference, Meminfo, Uptime, LoadAvg).
- **Styling:** Micro-CSS with CRT scanlines, phosphor-glow palette, monospaced typography, and responsive grid layout (< 2 KB).
- **Client Script:** Vanilla JS polling loop (< 30 lines) without external JS libraries or frameworks.

## Resource Profile

- **Idle RAM usage:** ~6–9 MB
- **Idle CPU usage:** 0.0% – 0.1%
- **Binary Footprint:** ~5 MB (release build)

## Quick Start

```bash
# Run in development mode
cargo run

# Or build optimized binary
cargo run --release
```

Open your browser at:
```
http://localhost:3000
```
