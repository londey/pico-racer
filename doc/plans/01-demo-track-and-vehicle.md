# Plan 01 — Demo Track and Vehicle

## Goal

Get the new game-side modules running end-to-end on screen.
A single car drives a hand-coded track under keyboard input, seen through the chase camera.
Visuals can be placeholder geometry; the point is to validate the contracts between `vehicle`, `track`, `camera`, `game`, and the existing `render`/`gpu` stack.

## Scope

In scope:

- A demo `Track` defined as a `&'static [TrackControlPoint]` constant (oval, ~12 control points, flat — no banking yet).
- A placeholder car: stretched-cube body + four short-cylinder wheels, authored in Blender or generated.
- Keyboard → `Controls` mapping in the firmware/PC input layer.
- A new scene/demo entry that owns a `Game` instance and emits render commands per frame for: track mesh, body, four wheels.
- Chase camera by default; key to toggle to bumper.

Out of scope:

- Lap counting, AI, opponents, HUD.
- Skybox, scenery, lighting tuning.
- A track surface mesh generated from the spline (covered by Plan 02 — for now use a flat ground plane plus a debug wireframe spline).

## Key open questions

- **Where does `Game` live in the scene model?**
  Two choices: (a) replace `scene::demos` with a `Game`-driven mode and keep the existing demos behind a debug menu, or (b) add a fourth demo variant `Demo::Race` that owns a `Game` alongside the teapot/triangle demos.
  Recommendation: (b) for now — minimal disruption, lets the spinning teapot keep working as a smoke test.
- **Placeholder geometry source.**
  Generate the cube/cylinders programmatically at startup (like the current Utah teapot), or bake them via the asset pipeline?
  Recommendation: programmatic — fewer moving parts, no asset-build dependency on Plan 02.
- **Track visual without a generated mesh.**
  Until Plan 02 lands, draw the centerline as a debug line strip using a thin quad along each segment.
  Quick to implement and useful for visualising the spline math.

## Tasks

1. Add a `tracks::oval()` constructor in `pico-racer-core` returning a `Track` with a static `&[TrackControlPoint]`.
   Aim for ~200 m total length to give meaningful driving distance.
2. Add `Demo::Race` (or equivalent) that constructs a `Game::new(track, 0.0, CameraMode::Chase)` on enter and steps it on each frame.
3. Add a `placeholder_car` asset module producing a body and wheel meshes from primitives, mirroring `assets::teapot`.
4. Map keyboard inputs in `pico-racer-rp2350` (and `pico-racer-pc` if its main loop is restored) into `Controls`:
   `W` throttle, `S` brake, `A`/`D` steering, `Space` handbrake, `R` reverse, `C` toggle chase/bumper.
5. Per-frame render command emission:
   `view = camera.view_matrix()`, `proj = camera.projection(aspect, near, far)`, then `mvp = proj * view * body_transform` for the body and similarly for each wheel and the (placeholder) ground plane.
6. Verify on PC host first if the firmware drift in Plan 04 has not yet been fixed.

## Acceptance

- Holding `W` accelerates the car and the chase camera follows.
- `A` and `D` steer; the front wheels visibly steer; the body rolls cosmetically.
- Bumps in the (initially flat) terrain produce no jitter; suspension wheels visibly compress when accelerating/braking due to pitch.
- `C` toggles to bumper view; the body disappears from view in the bumper view (or is suppressed).
- All host-side checks remain green.
