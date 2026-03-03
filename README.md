# pico-racer

Host application for the [pico-gs](https://github.com/londey/pico-gs) SPI GPU.

This repository contains the Rust host-side software: RP2350 firmware, PC debug host, asset pipeline, and platform abstraction layer.
The FPGA RTL, register definitions, and GPU specifications live in the pico-gs submodule at `external/pico-gs/`.

## Crates

| Crate | Description |
|-------|-------------|
| `pico-racer-hal` | Platform abstraction traits (`no_std`) |
| `pico-racer-core` | GPU driver, rendering, scene management (`no_std`) |
| `pico-racer-rp2350` | RP2350 embedded firmware (Cortex-M33, dual-core) |
| `pico-racer-pc` | PC debug host (FT232H SPI) |
| `asset-build-tool` | Asset preparation (PNG/OBJ to GPU format) |

## Getting Started

```bash
# Clone with submodules
git clone --recursive git@github.com:londey/pico-racer.git

# Build everything (lint + test + build)
./build.sh

# Build firmware only
./build.sh --firmware-only

# Run tests only
./build.sh --test-only
```

## License

MIT
