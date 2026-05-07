# pico-racer Development Guidelines

## Key Rules

- `./build.sh` must pass after every change (builds all software, runs all tests).
- `external/pico-gs/ARCHITECTURE.md` is the authoritative high-level GPU architecture document.
- `external/pico-gs/rtl/components/registers/rdl/gpu_regs.rdl` is the authoritative GPU register definition; the `gpu-registers` crate (in the submodule) is what code must reference for register values and constants.
- All Rust code follows the style guide below.

## Project Structure

```text
pico-racer/
├── crates/
│   ├── pico-racer-hal/          # Platform abstraction traits (no_std)
│   ├── pico-racer-core/         # Platform-agnostic GPU driver, rendering, scene (no_std)
│   │   ├── src/
│   │   │   ├── gpu/             # GpuDriver<S>, registers, vertex packing
│   │   │   ├── math/            # Fixed-point math (Q12.4, Q1.15)
│   │   │   ├── render/          # Commands, mesh rendering, transform, lighting
│   │   │   └── scene/           # Scene management, demo definitions
│   │   ├── assets/              # Source assets (.obj, .png)
│   │   └── tests/               # Integration tests
│   ├── pico-racer-rp2350/       # RP2350 firmware (dual-core, USB keyboard, SPI GPIO)
│   │   ├── src/                 # Firmware source (transport, input, core1, main)
│   │   ├── build.rs             # Asset conversion via asset-build-tool
│   │   └── memory.x             # RP2350 linker script
│   ├── pico-racer-pc/           # PC debug host (FT232H stub, terminal input)
│   └── asset-build-tool/        # Asset preparation tool (.obj/.png → GPU format)
├── external/
│   └── pico-gs/                 # GPU submodule (FPGA RTL, registers, specs)
│       ├── registers/           # gpu-registers crate (single source of truth)
│       │   └── rdl/gpu_regs.rdl # SystemRDL register definitions
│       ├── spi_gpu/             # FPGA RTL (SystemVerilog)
│       └── doc/                 # GPU specifications
├── doc/                         # pico-racer syskit specifications
│   ├── requirements/            # REQ-NNN documents
│   ├── interfaces/              # INT-NNN documents
│   ├── design/                  # UNIT-NNN documents
│   └── verification/            # VER-NNN documents
├── build.sh                     # Host-only build script
└── Cargo.toml                   # Workspace root
```

## Commands

# Build entire project (lint + test + build)
./build.sh

# Build specific component
./build.sh --firmware-only
./build.sh --pc-only
./build.sh --test-only

# Build in release mode
./build.sh --release

# Firmware-specific builds (RP2350)
cargo build -p pico-racer-rp2350 --target thumbv8m.main-none-eabihf
cargo test -p pico-racer-core

# PC debug host build
cargo build -p pico-racer-pc

## Rust Code Style

- Follow standard Rust conventions and idioms; use `rustfmt` for formatting
- Prefer modern `module_name.rs` file style over `mod.rs` (Rust 2018+)
- All public items require `///` doc comments (modules use `//!`); functions need `# Arguments`, `# Returns`, `# Errors` sections
- Document constants with purpose and spec reference where applicable
- Blank lines between module-level items, between doc-commented struct fields, and after `use` blocks
- Avoid `.unwrap()` / `.expect()` in production code; use `Result<T, E>` + `?` operator
- Libraries: `thiserror` for error types; applications: `anyhow` (std crates only; no_std uses custom enums)
- Logging: `defmt` for no_std/embedded, `log` crate for std; avoid `println!`/`eprintln!`
- Add dependencies with `default-features = false`, explicitly enable only needed features
- Crate-level lints: `#![deny(unsafe_code)]`, clippy pedantic + `missing_docs` gated on release builds via `cfg_attr`

### Build Verification (Rust)

After changes: `cargo fmt` → `cargo clippy -- -D warnings` → `cargo test` → `cargo build --release`

## Markdown Style

- Use semantic line breaks: start each sentence on its own line.
  Adjacent lines render as a single paragraph in HTML, but one-sentence-per-line produces cleaner diffs and easier code review.

## Fixed-Point Notation

All fixed-point values use TI-style Q notation:
- `Qm.n` — signed: m integer bits (including sign bit), n fractional bits, total width = m + n bits.
- `UQm.n` — unsigned: m integer bits, n fractional bits, total width = m + n bits.

Examples:
- `Q2.2` is a signed 4-bit value with resolution 2⁻² (1/4), range −2.0 to +1.75.
- `UQ2.2` is an unsigned 4-bit value with resolution 2⁻² (1/4), range 0.0 to +3.75.

Apply this notation consistently in documentation, code comments, and specifications.
When sign/unsigned is ambiguous, always use the explicit `Q` or `UQ` prefix.

## Register Interface

The register interface lives in the pico-gs submodule at `external/pico-gs/twin/components/registers/`.
It is **NOT managed by syskit** in this repository.

- **SystemRDL source:** `external/pico-gs/rtl/components/registers/rdl/gpu_regs.rdl` — canonical machine-readable definition
- **Rust crate:** `external/pico-gs/twin/components/registers/src/lib.rs` (`gpu-registers`, `no_std`) — generated from the RDL by PeakRDL-rust, with hand-maintained flat constants in the same file
- **Specs:** `external/pico-gs/doc/interfaces/` — INT-010 through INT-014

To update registers, work in the pico-gs repository directly.

<!-- syskit-start -->
## syskit

This project uses **syskit** for specification-driven development. Specifications in `doc/` define what the system must do, how components interact, and how the design is structured. Implementation follows from specs. When creating new specifications, define interfaces and requirements before design — understand the contracts and constraints before deciding how to build.

### Working with code

- Source files may contain `Spec-ref:` comments linking to design units — preserve them; they are navigational pointers to the governing spec.
- Before modifying code, check `doc/design/` for a relevant design unit (`unit_NNN_*.md`) that describes the component's intended behavior.
- After code changes, run `.syskit/scripts/impl-check.sh` to verify Spec-ref consistency (missing / orphan / untracked).

### Documentation principle

- **Reference, don't reproduce.** Don't duplicate definitions, requirements, or design descriptions — reference the authoritative source instead. For project documents, reference by ID (`REQ-NNN`, `INT-NNN`, `UNIT-NNN`, `VER-NNN`). For external standards, reference by name, version/year, and section number (e.g., "IEEE 802.3-2022 §4.2.1", "RFC 9293 §3.1"). This applies to specification documents and code comments alike.

### Making changes

For non-trivial changes affecting system behavior, use the syskit workflow:

1. `/syskit-impact <change>` — Analyze what specifications are affected
2. `/syskit-propose` — Propose specification updates
3. `/syskit-refine --feedback "<issues>"` — Iterate on proposed changes based on review feedback (optional, repeatable)
4. `/syskit-approve` — Approve changes (works across sessions, enables overnight review)
5. `/syskit-plan` — Break into implementation tasks
6. `/syskit-implement` — Execute with traceability

New to syskit? Run `/syskit-guide` for an interactive walkthrough.

### Reference

- Specifications: `doc/requirements/`, `doc/interfaces/`, `doc/design/`, `doc/verification/`
- Working documents: `.syskit/analysis/`, `.syskit/tasks/`
- Scripts: `.syskit/scripts/`
- Full instructions: `.syskit/AGENTS.md` (read on demand, not auto-loaded)
<!-- syskit-end -->
