# Migration Notes: pico-gs → pico-racer

## What was migrated

The Rust host application crates were extracted from `external/pico-gs/crates/` into this repository's `crates/` directory, with all crates renamed from `pico-gs-*` to `pico-racer-*`.

### Crate mapping

| pico-gs (original) | pico-racer (new) | Package name |
|---------------------|------------------|--------------|
| `crates/pico-gs-hal/` | `crates/pico-racer-hal/` | `pico-racer-hal` |
| `crates/pico-gs-core/` | `crates/pico-racer-core/` | `pico-racer-core` |
| `crates/pico-gs-rp2350/` | `crates/pico-racer-rp2350/` | `pico-racer-rp2350` |
| `crates/pico-gs-pc/` | `crates/pico-racer-pc/` | `pico-racer-pc` |
| `crates/asset-build-tool/` | `crates/asset-build-tool/` | `asset-prep` (unchanged) |

### What stays in pico-gs (submodule)

- `registers/` — `gpu-registers` crate and `rdl/gpu_regs.rdl` (referenced via path dependency)
- `spi_gpu/` — FPGA RTL (SystemVerilog)
- `doc/` — GPU hardware specifications
- `ARCHITECTURE.md` — GPU architecture document

### What's new in pico-racer

- Workspace `Cargo.toml` at repo root
- `.cargo/config.toml` — RP2350 cross-compilation config
- `clippy.toml` — Clippy lint config
- `build.sh` — Host-only build script (lint + test + build)
- `CLAUDE.md` — Development guidelines
- `.devcontainer/` — Slimmed devcontainer (no FPGA tools)
- `.claude/` — Settings and syskit commands
- `doc/` — Empty syskit spec structure (start fresh)

## Verification steps (run inside devcontainer)

Open this repository in the devcontainer, then run these commands:

```bash
# 1. Verify submodule is initialized
git submodule update --init --recursive
ls external/pico-gs/registers/Cargo.toml  # Should exist

# 2. Check host-native crates compile
cargo check -p pico-racer-hal -p pico-racer-core -p pico-racer-pc -p asset-prep

# 3. Run core tests
cargo test -p pico-racer-core

# 4. Check formatting
cargo fmt --check

# 5. Run clippy
cargo clippy -- -D warnings

# 6. Build RP2350 firmware (cross-compile)
cargo build -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf

# 7. Run full build script
./build.sh

# 8. If all passes, delete this file
rm MIGRATION_NOTES.md
```

## Verification results (2026-03-03)

| Step | Command | Result | Notes |
|------|---------|--------|-------|
| 1. Submodule | `git submodule update --init` | PASS | `external/pico-gs` at `0a2505d` |
| 2. Check crates | `cargo check -p ...` | PASS | All four host crates compile |
| 3. Core tests | `cargo test -p pico-racer-core` | PASS | 26 passed, 0 failed |
| 4. Formatting | `cargo fmt --check` | **FAIL** | Trailing-comment alignment and line-length diffs in `driver_tests.rs`, `packing_tests.rs` — fixed with `cargo fmt` |
| 5. Clippy | `cargo clippy -- -D warnings` | **FAIL** | New `manual_is_multiple_of` lint in `textures.rs:62,67` (Rust 1.93) — needs `% 2 == 0` → `.is_multiple_of(2)` |
| 6. RP2350 firmware | `cargo build -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf` | PASS | 7 `static_mut_refs` warnings (upstream `rp235x-hal` / `cortex-m`; not actionable until dependencies update) |
| 7. Full build | `./build.sh` | **FAIL** | Blocked by steps 4 and 5 above |

### Fixes required before `./build.sh` passes

1. **Formatting** — Run `cargo fmt`.
   The diffs are cosmetic (rustfmt alignment rules changed between toolchain versions).

2. **Clippy `manual_is_multiple_of`** — In `crates/pico-racer-core/src/assets/textures.rs`, replace:
   ```rust
   // line 62
   if (block_x0 + block_y) % 2 == 0 {
   // line 67
   if (block_x1 + block_y) % 2 == 0 {
   ```
   with:
   ```rust
   if (block_x0 + block_y).is_multiple_of(2) {
   if (block_x1 + block_y).is_multiple_of(2) {
   ```
   This lint was stabilised in Rust 1.93; the original code predates it.

## Crate rename completeness

All references to `pico-gs-*` package names have been replaced with `pico-racer-*` across:
- `Cargo.toml` package names and dependency declarations
- `use` statements and `extern crate` references
- Module-level doc comments

The only remaining `pico-gs` path is the intentional submodule dependency in `crates/pico-racer-core/Cargo.toml`:
```toml
gpu-registers = { path = "../../external/pico-gs/registers" }
```

## Spec-ref comments

Ten `Spec-ref:` comments exist in the source code, all referencing pico-gs design unit IDs (dated 2026-02-25):

| Design unit | Files |
|-------------|-------|
| `unit_020_core_0_scene_manager.md` | `pico-racer-rp2350/src/main.rs` |
| `unit_021_core_1_render_executor.md` | `pico-racer-rp2350/src/core1.rs` |
| `unit_025_usb_keyboard_handler.md` | `pico-racer-rp2350/src/input.rs` |
| `unit_026_intercore_queue.md` | `pico-racer-rp2350/src/queue.rs` |
| `unit_027_demo_state_machine.md` | `pico-racer-core/src/scene/demos.rs` |
| `unit_035_pc_spi_driver.md` | `pico-racer-pc/src/transport.rs` |
| `unit_036_pc_input_handler.md` | `pico-racer-pc/src/input.rs` |

These are preserved intentionally.
They will be updated (or replaced) as equivalent pico-racer specs are written under `doc/design/`.

## PC debug host (`pico-racer-pc`)

This crate is an intentional stub.
All hardware integration is deferred behind `TODO` comments (12 total across `main.rs`, `transport.rs`, `input.rs`).
The `ftdi` and `crossterm` dependencies are commented out in `Cargo.toml`.
The crate compiles and links, but does nothing at runtime.

## Syskit state

- The `.claude/commands/syskit-*.md` files are present, so syskit slash commands are available.
- The `.syskit/` working directory does not yet exist.
  It will be created automatically the first time a syskit command writes analysis or task files.
- The `doc/` spec directories contain only placeholder `README.md` files — no specs have been written yet.

## Known considerations

- The `gpu-registers` dependency uses a path into the submodule: `path = "../../external/pico-gs/registers"`.
  The submodule must be initialized before any cargo command will work.
- The `asset-build-tool` crate's `Cargo.lock` was copied as-is from pico-gs.
  It can be deleted if it causes issues (the workspace-level lock file will be generated).
- The RP2350 firmware build emits 7 `static_mut_refs` warnings from upstream crates (`rp235x-hal`, `cortex-m`, `defmt-rtt`).
  These are Rust 2024 compatibility warnings and will be resolved when the dependencies release updates.
  They do not affect correctness.
