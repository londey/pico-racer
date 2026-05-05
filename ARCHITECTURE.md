# Architecture

Architecture overview for pico-racer.

## System Description

pico-racer is a `no_std` Rust application that drives the [pico-gs] SPI GPU to
produce an arcade-style racing game.
The current target host MCU is the RP2350, but the host-side architecture is
deliberately MCU-agnostic:
all platform-specific code lives behind traits in `pico-racer-hal`, so the
software can be retargeted to a faster MCU (e.g. STM32H7, iMX RT) if RP2350 +
SPI bandwidth proves insufficient (see [Render Bandwidth](#render-bandwidth)).

The system is functionally split into:

- **Host** (this repository) — game logic, simulation, transform/lighting,
  vertex packing, command submission.
- **GPU** ([pico-gs] submodule) — rasterizer, color combiner, framebuffer,
  exposed over a memory-mapped SPI register interface.

[pico-gs]: https://github.com/londey/pico-gs

## Design Philosophy

- **Layered, hardware-abstracted.**
  Platform traits in `pico-racer-hal`, platform-agnostic logic in
  `pico-racer-core`, MCU-specific firmware in `pico-racer-rp2350`,
  desktop debug harness in `pico-racer-pc`.
- **Reference, don't reproduce** (CLAUDE.md).
  The GPU register interface is owned by the [pico-gs] `gpu-registers` crate;
  game code consumes those constants rather than duplicating them.
- **Software T&L, hardware rasterization.**
  The GPU rasterizes pre-transformed screen-space triangles.
  All world-space math (transform, lighting, culling, vertex packing) runs on
  the host.
  Numeric domain is `f32` end-to-end via [`glam`]; fixed-point appears only
  at the GPU register boundary (`Q12.4` X/Y, `Q1.15` UV/Q, 25-bit Z) — see
  `pico-racer-core/src/math/fixed.rs`.
- **Decouple simulation from presentation.**
  Vehicle dynamics run on a *fixed* simulation timestep
  ([`game::SIM_HZ`] = 120 Hz).
  Cameras, render-command emission, and visual smoothing run at *render*
  rate.
  This is the standard pattern for stable spring/damper integration and for
  decoupling input feel from frame-rate variability.
- **Host bandwidth is the budget.**
  Triangle count per frame is gated by SPI throughput, not by RP2350 CPU.
  Scene complexity decisions are made with that ceiling in mind.

[`glam`]: https://docs.rs/glam

## Component Interactions

```
┌────────────────────────── host (pico-racer-rp2350 / pico-racer-pc) ──────────────────────────┐
│                                                                                              │
│  input ─▶ game::Game ─▶ vehicle::Vehicle ─▶ visual transforms ─▶ render::commands ─▶ queue   │
│             │  ▲                ▲                                                            │
│             │  │                └── track::Track  (spline, width, banking, surface query)    │
│             │  │                                                                             │
│             │  └─ camera::Camera (chase / bumper, smoothed follow)                           │
│             │                                                                                │
│             └─ fixed-timestep accumulator (SIM_HZ = 120 Hz)                                  │
│                                                                                              │
│                                                ┌──────────────── GPU consumer ───────────┐   │
│                                                │ gpu::GpuDriver<S>  ──── SPI ──────────▶ │   │
│                                                │ pack screen verts, write registers      │   │
│                                                └─────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
                                                                                          │
                                                                                          ▼
                                                                              ┌──── pico-gs FPGA ───┐
                                                                              │ rasterizer +        │
                                                                              │ color combiner +    │
                                                                              │ framebuffer + Z     │
                                                                              └─────────────────────┘
```

### Layers

| Layer | Crate | Concerns |
|-------|-------|----------|
| Game logic | `pico-racer-core::game` | Fixed-timestep loop, controls plumbing, top-level state. |
| Simulation | `pico-racer-core::vehicle`, `pico-racer-core::track` | Bicycle-model dynamics, suspension, spline queries. |
| Presentation | `pico-racer-core::camera`, `pico-racer-core::render` | Cameras, MVP composition, lighting, vertex packing. |
| GPU driver | `pico-racer-core::gpu` | Register I/O, triangle submission, texture upload. |
| Platform | `pico-racer-hal` | `SpiTransport`, `FlowControl`, `InputSource` traits. |
| MCU firmware | `pico-racer-rp2350` | Dual-core scheduling, USB keyboard, SPI/GPIO bringup. |
| Debug host | `pico-racer-pc` | FT232H SPI, terminal input — for iteration off-target. |

### Update flow per render frame

1. Host samples input (`InputSource`) and writes `game::Game::controls`.
2. Host calls `Game::tick(frame_dt)`.
   - Time accumulates against `SIM_DT = 1/120 s`.
   - For each whole sim step: `Vehicle::step(SIM_DT, &controls, &Track)` —
     chassis bicycle dynamics, suspension spring integration, terrain queries
     against the spline.
   - Camera updates once per frame at render rate (smoothed follow).
3. Host reads `Vehicle::body_transform()` and `Vehicle::wheel_transforms()`
   to obtain a body `Mat4` plus four wheel `Mat4`s.
4. Host issues render commands to draw track mesh, body, and four wheels —
   each a separate model transform composed with the camera's view +
   projection.
5. Render thread (Core 1 on RP2350) drains the command queue and submits to
   GPU over SPI.

### Coordinate convention

Right-handed world; **`+Y` up**, vehicle yaw 0 faces **`+Z`**, yaw is
positive turning toward `+X` when viewed from above.
The track centerline lives in the same world frame; track-space coordinates
are `(arc_length, lateral_offset)` where positive lateral is left of the
forward tangent.

## Render Bandwidth

A single GPU vertex (`pico-racer-core/src/gpu/vertex.rs`) is ~28 bytes on the
SPI wire after register packing.
At a 25 MHz SPI clock and 8-bit framing the achievable sustained throughput
is ~3 MB/s, which translates to roughly:

- ~600 triangles/frame at 60 Hz, or
- ~1200 triangles/frame at 30 Hz

after flow-control and command overhead.
A typical scene (car ~300 tris + visible track ~600 tris + skybox/scenery)
sits near that ceiling.
This is recorded as a known constraint shaping content budget.

Mitigation paths, in order of ascending upheaval:

1. Push SPI clock past 25 MHz (RP2350 SPI peripheral / PIO can run faster).
2. Quad-SPI or parallel link, if [pico-gs] can be extended.
3. Coarse frustum + occlusion culling on host before submission
   (back-face is already done in `render::transform::is_front_facing`).
4. Move host to a larger MCU (STM32H7, iMX RT, ESP32-P4).

The decision is deferred until a real scene with a real track and car gives
a concrete number.

## Design Decisions

| Decision | Rationale | Alternative considered |
|----------|-----------|------------------------|
| `f32` for game/sim, fixed-point only at GPU boundary. | RP2350 has a Cortex-M33F FPU; bigger candidate MCUs have FPUs/NEON. Fixed-point world coordinates would need per-quantity Q-formats and overflow tracking, with no perf upside. | `Q1.15` / `Q24.8` mixed throughout — rejected on dev-cost and precision grounds. |
| Fixed simulation timestep (120 Hz). | Stable spring/damper integration; deterministic feel; standard pattern. | Variable-rate sim — rejected because spring-mass behavior becomes frame-rate dependent. |
| Bicycle-model dynamics for chassis, cosmetic pitch/roll. | Arcade feel (Ridge Racer-class), ~30 lines of integration, drift comes for free from lateral grip damping. | Full 4-wheel rigid-body — deferred; can be layered on once a track and car exist for tuning. |
| Catmull-Rom (centripetal) spline for track centerline. | Interpolates control points, natural for hand-authored tracks; centripetal variant avoids self-intersection at sharp corners. | Cubic Bezier — control-point handles are extra authoring overhead; B-spline — does not interpolate. |
| Visual track mesh referenced by id, not embedded in `Track`. | Lets us generate a mesh programmatically from the spline during development, then hand-edit in Blender for scenery, without changing gameplay code. | Generated mesh inline — couples authoring tooling to runtime. |
| Soft walls via spline width, not rigid-body collision. | Arcade game; full collision is large-scope work for limited payoff. | Rigid-body collision — deferred to "if it ever becomes worthwhile". |

## Module Layout

`pico-racer-core` (added in this iteration):

```
src/
├── camera/        # Chase + bumper cameras, smoothed follow.
├── game/          # Fixed-timestep loop, top-level state.
├── track/
│   ├── mod.rs     # Track: spline + per-CP width/banking, surface queries.
│   └── spline.rs  # Centripetal Catmull-Rom, arc-length parameterization.
├── vehicle/
│   ├── mod.rs     # Vehicle bundle.
│   ├── chassis.rs # Bicycle-model dynamics + cosmetic pitch/roll.
│   ├── controls.rs# Driver input struct.
│   ├── spec.rs    # Tunable parameters.
│   ├── suspension.rs # Per-wheel damped springs, wheel transforms.
│   └── visual.rs  # Snapshot of body + 4 wheel Mat4s for rendering.
└── (existing: assets/, gpu/, math/, render/, scene/)
```

---

<!-- syskit-arch-start -->
### Block Diagram

```mermaid
flowchart LR
    %% No design units found
```

### Software Units

*No design units found.*
<!-- syskit-arch-end -->
