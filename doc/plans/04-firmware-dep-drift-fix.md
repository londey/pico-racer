# Plan 04 — Firmware Dependency Drift Fix

## Goal

Restore `./build.sh` to green by repairing pre-existing build breakage in `crates/pico-racer-rp2350`.
This breakage existed on the branch before the game-side scaffolding landed and blocks the firmware build (and therefore the full build script).

## Symptoms

`cargo clippy --workspace -- -D warnings` against `pico-racer-rp2350` produces five errors:

1. `error[E0432]: unresolved import 'hal::multicore'` — the `multicore` module in `rp235x-hal` is gated behind a feature flag.
2. `error[E0433]: failed to resolve: could not find 'arch_entry' in 'rp235x_hal'` — the `arch_entry` re-export is gated behind `arm` / `riscv32` features.
3. `error[E0609]: no field 'mair' on type '&cortex_m::peripheral::mpu::RegisterBlock'` (`main.rs:74`) — newer `cortex-m` versions renamed/relocated this MPU register.
4. `error[E0609]: no field 'rlar' on type '&cortex_m::peripheral::mpu::RegisterBlock'` (`main.rs:81`) — same.
5. `error[E0609]: no field 'rlar' on type '&cortex_m::peripheral::mpu::RegisterBlock'` (`main.rs:86`) — same.

Errors 1–2 indicate Cargo features that need adding.
Errors 3–5 indicate an API change in an external crate that needs to be either tracked forward (update local code) or pinned backward (downgrade the crate).

## Scope

In scope:

- Audit `crates/pico-racer-rp2350/Cargo.toml` and add the missing `rp235x-hal` feature flags (`arm`, `multicore`).
- Decide whether to update the MPU code to the current `cortex-m` API or pin `cortex-m` to a version where the previous fields existed.
- Verify `cargo build --target thumbv8m.main-none-eabihf -p pico-racer-rp2350` succeeds.
- Verify `./build.sh` is green end-to-end.

Out of scope:

- Any functional change to the firmware.
- Bumping or downgrading other dependencies beyond what is required to clear these five errors.

## Key open questions

- **Track-forward or pin-backward on `cortex-m`?**
  Track-forward is healthier long-term but means understanding the new MPU API.
  Pin-backward is a one-line fix in `Cargo.toml` and defers the work.
  Recommendation: track-forward.
  The MPU setup is small (three lines around `main.rs:74-86`) and the new API is documented; no good reason to accumulate dependency debt.
- **Lockfile.**
  The repo's `Cargo.lock` may be ahead of what the firmware crate compiled against historically.
  Decide whether to commit a lockfile update as part of this fix.
  Recommendation: yes — the existing lockfile is the source of the drift.

## Tasks

1. Read `crates/pico-racer-rp2350/Cargo.toml` and `src/main.rs` (lines around 60–95).
2. Add `features = ["arm", "multicore"]` (or the current equivalents) to the `rp235x-hal` dependency.
3. Look up the current `cortex_m::peripheral::mpu::RegisterBlock` field names; rewrite the three offending lines.
   Likely candidates: `rbar` for region base address with the `RGN` and `VALID` bits encoded, then `rasr` (or whatever the current crate calls the attribute register) for size + permissions.
   Verify by reading the `cortex-m` crate's MPU example or its `mpu.rs` source.
4. Rebuild firmware: `cargo build --target thumbv8m.main-none-eabihf -p pico-racer-rp2350`.
5. Run `./build.sh` and confirm green.
6. Commit with a message scoped to the firmware drift fix.

## Acceptance

- `./build.sh` succeeds with no flags.
- The firmware ELF still passes any existing smoke test on hardware (see `pico-racer-rp2350` README/notes for how the prior author tested it).
- No game-side code changed.
