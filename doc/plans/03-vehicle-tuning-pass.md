# Plan 03 — Vehicle Tuning Pass

## Goal

Bring `VehicleSpec::arcade()` from "plausible defaults" to "actually feels good to drive" against the demo track from Plan 01.

Arcade feel is tuned, not derived.
The numbers in `spec.rs` are starting estimates; they will all change.
This plan is the hands-on iteration loop.

## Scope

In scope:

- An on-target telemetry channel (defmt over RTT, or UART) emitting per-tick chassis and per-wheel values.
- A small on-screen HUD or serial console readout: speed (km/h), yaw rate (deg/s), suspension travel per wheel, lateral grip status, steering angle.
- Iteration on `VehicleSpec` values: longitudinal accel/brake/drag, top speed, lateral grip, steering response, handbrake grip factor, suspension stiffness/damping/travel, roll/pitch factors.
- A "feel" log captured in this file as decisions are made.

Out of scope:

- Force feedback, vibration, audio.
- Real-time spec editing (no live-reload / parameter UI).
- Differential per-wheel traction effects (still on the deferred list — see ARCHITECTURE.md decisions).

## Key open questions

- **Telemetry transport.**
  Firmware uses `defmt`; RTT over the SWD probe is the natural fit and adds zero pin cost.
  PC host can `printf` directly.
  Recommendation: defmt for the firmware path; nothing extra needed for PC.
- **HUD: on-screen vs serial only.**
  A minimal on-screen HUD (text overlay rendered via small quads) is a chunk of work in itself.
  Serial-only is fine for tuning.
  Recommendation: serial-only for v1; on-screen HUD becomes a separate plan once we have a glyph atlas.
- **Capturing a tuning baseline.**
  Tuning is iterative and easy to lose track of.
  Recommendation: append a dated entry to the "Tuning log" section below per session, with the spec values that produced "good enough" for that day.

## Tuning checklist

For each session, drive the demo track and form an opinion on each row.
Adjust one parameter at a time when something is off.

| Aspect | Symptoms of "off" | First place to look |
|--------|-------------------|---------------------|
| Top speed | Reaches max too fast / never reaches max | `top_speed`, `drag` |
| Acceleration curve | Snappy off the line, dead at speed (or vice versa) | `max_accel`, `drag` |
| Braking | Mushy / locks up / car coasts forever | `max_brake`, `rolling_resistance` |
| Steering at low speed | Twitchy / unresponsive | `max_steering`, `steering_response` |
| Steering at high speed | Spins out / understeers into walls | `lateral_grip`, indirectly `max_steering` |
| Drift initiation | Grips through corners / breaks loose too easily | `lateral_grip` (lower = more drift) |
| Drift recovery | Hard to catch / too sticky | `lateral_grip`, `handbrake_grip_factor` |
| Body roll | Stiff / capsizing | `roll_factor`, `body_response` |
| Body pitch | Flat / nodding | `pitch_factor`, `body_response` |
| Suspension over bumps | Bouncy / harsh | `suspension_stiffness`, `suspension_damping` |
| Suspension under brake/accel | No visible squat or dive / too much | `suspension_stiffness`, indirectly `pitch_factor` |

## Tasks

1. Add a `telemetry` module in `pico-racer-core` exposing a `VehicleTelemetry` snapshot (speed, yaw, yaw rate, steering, per-wheel travel/on_ground, controls).
2. Emit a snapshot every N sim ticks (e.g. every 10 = 12 Hz) via `defmt::info!` in the firmware.
3. Add a `--telemetry` flag to the PC host that prints the same struct.
4. Drive a few laps; note observations against the table above; commit `VehicleSpec` changes one at a time with a short rationale.
5. After each session, append a "Tuning log" entry below.

## Tuning log

(Empty — populated as sessions happen.)
