# Plans

Working notes for the next pieces of work on pico-racer.

These are *plans*, not specs.
The syskit specification slots (`doc/requirements/`, `doc/interfaces/`, `doc/design/`, `doc/verification/`) remain reserved for formal artifacts.
Plans here describe intent and open questions; they are deleted or rewritten as the work lands.

| File | What it covers |
|------|----------------|
| `01-demo-track-and-vehicle.md` | Wire `Game`/`Vehicle`/`Track` into a runnable demo with a placeholder car and a hand-coded track. |
| `02-track-mesh-generator.md` | Asset-build-tool extension: spline definition → `.obj` track surface, round-trippable through Blender. |
| `03-vehicle-tuning-pass.md` | First on-device tuning pass of `VehicleSpec::arcade()` once we can drive. |
| `04-firmware-dep-drift-fix.md` | Restore `./build.sh` green by fixing pre-existing `pico-racer-rp2350` build breakage. |
