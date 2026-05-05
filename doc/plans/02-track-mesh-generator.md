# Plan 02 — Track Mesh Generator

## Goal

Generate a drivable track surface mesh from a spline definition file, in a format Blender can open for scenery work and then re-export through the existing asset pipeline.

The runtime keeps the spline as the source of truth for *gameplay* queries (centerline, width, banking, surface height).
The generated mesh is the source of truth for *visuals*.
The two are linked by sharing the same input definition file at content-build time.

## Scope

In scope:

- A new binary tool (e.g. `crates/asset-build-tool/src/bin/track-gen.rs`) that consumes a spline definition file and emits a `.obj` track surface.
- A spline definition format (RON or TOML) listing control points and per-control-point attributes.
- Sampling strategy and tessellation settings (triangle density along arc length, mesh patch chunking compatible with `MAX_PATCH_VERTICES` / `MAX_PATCH_INDICES`).
- A round-trip story: artist opens the generated `.obj` in Blender, adds scenery as additional objects (kerbs, trees, signs) without altering the road object, then exports a single `.obj` that the existing `asset-prep` pipeline bakes into mesh patches.

Out of scope:

- Tooling for editing the spline visually.
  Authoring is text-file editing for now.
- Track-side textures and materials beyond a single road surface texture.
- Multiple surface types (tarmac vs. dirt) per section — defer.

## Key open questions

- **Definition file format: RON vs TOML.**
  RON preserves Rust-y enums/types and is ergonomic to deserialise with `serde`.
  TOML is friendlier to non-Rust tooling.
  Recommendation: RON, since the consumer is a Rust build tool and the file lives next to Rust sources.
- **Embedding the spline in the runtime.**
  Two options:
  (a) the build tool emits both the `.obj` and a Rust source file (`tracks/oval_generated.rs`) holding the static `[TrackControlPoint; N]`,
  (b) the runtime parses the RON at build time via `build.rs`.
  Recommendation: (a) — zero runtime dependencies, direct `#include`-style use.
- **How does the runtime know which mesh chunks are the road?**
  In Blender the artist may add scenery; the asset pipeline currently flattens everything into mesh patches.
  Need a marker — e.g. a naming convention (`road_*` object names) that the asset tool tags so the runtime can render road and scenery with different `RenderFlags`.
  Defer to first time we hit it; for v1 the whole `.obj` is the road.
- **Vertex budget per patch.**
  `MAX_PATCH_VERTICES = 128`, `MAX_PATCH_INDICES = 384`.
  A simple ribbon (2 verts per arc-length sample) gives 64 samples per patch.
  Decide tessellation density (e.g. 1 sample per metre); a 200 m oval needs ~4 patches.

## Tasks

1. Define the spline definition schema in a new `crates/asset-build-tool/src/track_def.rs`.
   Fields per control point: `position`, `half_width`, `banking`, optional `surface_marker`.
   Top-level: `name`, optional `texture_path`.
2. Implement Catmull-Rom evaluation in the build tool.
   Reuse the math from `pico-racer-core::track::spline` if practical (the core implementation is `no_std` but the algorithm is identical; consider extracting to a small `pico-racer-spline` crate that both can depend on).
3. Sample generation: walk arc length at fixed step (default 1 m), at each sample compute `(left_edge, right_edge)` from `centerline ± right_vector × half_width`, with optional banking lift.
4. Emit two outputs:
   - A `.obj` with vertices, normals (computed per-quad), and UVs (`U` across the road, `V` along arc length scaled to repeat the texture every N metres).
   - A Rust source file with `pub const CONTROL_POINTS: &[TrackControlPoint] = &[...];` for runtime use.
5. Wire one example track (`assets/tracks/oval.ron`) through the tool in `pico-racer-rp2350/build.rs`, producing a generated source consumed by the runtime.
6. Document the round-trip workflow (export → Blender edit → re-export) in this file once proven.

## Acceptance

- `cargo run -p asset-prep --bin track-gen -- assets/tracks/oval.ron` writes `oval.obj` and `oval_generated.rs`.
- Opening `oval.obj` in Blender shows a continuous closed-loop ribbon with no holes or self-intersection.
- The runtime, fed the generated `CONTROL_POINTS`, produces the same centerline positions (within 1 cm) as the build tool.
- A modified `.obj` (Blender adds a few scenery cubes off the road) still bakes through `asset-prep` without errors and renders.
