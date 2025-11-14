# Appendix D: Installation and Setup

This appendix provides practical guidance for installing and configuring OctaIndex3D.

Topics include:

- System requirements and supported platforms  
- Building from source with `cargo`  
- Enabling optional features (e.g., BMI2, SIMD, serialization)  
- Troubleshooting common issues  

---

## D.1 System Requirements

- A recent stable Rust toolchain (see the main `README.md` for the recommended version).
- A 64‑bit CPU; optional optimizations are available for x86‑64 (BMI2, AVX2) and ARM (NEON).
- Enough memory to hold your working dataset plus index structures.

## D.2 Installation Instructions

For most users:

1. Install Rust via `rustup` if you have not already.
2. Add `octaindex3d = "0.4"` to your project’s `Cargo.toml`.
3. Run `cargo build` to fetch and compile dependencies.

If you prefer to work from a local checkout, clone the repository and run `cargo test` to verify your environment.

## D.3 Feature Flags

OctaIndex3D exposes feature flags to control optional dependencies and CPU‑specific optimizations. Typical categories include:

- `bmi2`, `avx2`, or `neon` for accelerated bit‑manipulation paths.
- `serde` for serialization support.
- `rayon` or similar crates for parallel processing.

Consult the main crate documentation for the current list of flags and recommended combinations.

## D.4 Building from Source

To build directly from the repository:

```bash
git clone https://github.com/FunKite/OctaIndex3D.git
cd OctaIndex3D
cargo build --release
```

You can add CPU‑specific flags via `RUSTFLAGS` or a `.cargo/config.toml` file once you are comfortable with the baseline build.

## D.5 Troubleshooting

Common issues and remedies:

- **Build fails with unknown CPU features**  
  Disable the corresponding feature flags (for example, `bmi2` or `avx2`) or ensure you are compiling for a compatible target.

- **Examples or benchmarks run slowly on laptop hardware**  
  Start with smaller datasets and lower levels of detail (LOD). Use the Performance Tuning Cookbook (Appendix G) to identify safe optimizations.

- **Parity assertion failures or “not in BCC lattice” errors**  
  Check that you are constructing coordinates through the provided types (such as `BccCoord`) rather than hand‑crafting indices. Chapter 2 explains the parity constraint in detail.

As the book and codebase evolve, this section will grow to cover platform‑specific quirks and integration issues reported by readers.
