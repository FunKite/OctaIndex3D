# Quick Start Guide

This quick start is for readers who want to see OctaIndex3D in action **before** reading the full story. In about five minutes you will:

- Install the crate
- Build and run a tiny example
- See how identifiers and queries feel in real code

After that, you can decide whether to continue with Part I (Foundations) or jump ahead to the architecture and implementation chapters.

---

## 0.1 Who This Is For

Use this quick start if:

- You are comfortable with Rust and `cargo`
- You prefer to learn by running code
- You want a small end‑to‑end path from “nothing” to “working example”

If you are new to Rust or want more context before touching code, start with Chapter 1 and come back here later.

---

## 0.2 Install the Crate

Add OctaIndex3D to your project:

```toml
[dependencies]
octaindex3d = "0.4"
```bash

Then fetch dependencies:

```bash
cargo build
```rust

If you see build errors related to optional CPU features (BMI2, AVX2, NEON), disable them for now and come back to Appendix G (Performance Cookbook) later for tuning advice.

---

## 0.3 Your First BCC Query

Create a new binary project:

```bash
cargo new oi3d-hello
cd oi3d-hello
```rust

Replace `src/main.rs` with:

```rust
use octaindex3d::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Pick a coordinate at a given level of detail (LOD).
    //    Here we use an arbitrary point near the origin.
    let lod = 10u8;
    let p = BccCoord::new(2, 4, -6, lod)?;

    // 2. Convert it into an Index64 identifier for storage and querying.
    let id = Index64::from_coord(p);

    // 3. Fetch its 14 neighbors in the BCC lattice.
    let neighbors = id.neighbors();

    println!("Center cell:  {:?}", id);
    println!("LOD:          {}", lod);
    println!("Neighbor IDs: {neighbors:#?}");

    Ok(())
}
```

Build and run:

```bash
cargo run
```rust

You should see one identifier for the center cell and 14 neighbor identifiers. The exact numeric values are not important yet—the point is that:

- You work with **typed coordinates** (`BccCoord`) rather than raw `(x, y, z)` triples
- You get **hierarchical identifiers** (`Index64`) that encode level of detail
- Neighbor queries are a single method call, not hand‑rolled index arithmetic

If the program does not compile:

- Make sure your `Cargo.toml` uses a released `octaindex3d` version that matches the crate on `crates.io`.
- Check that your Rust toolchain is recent enough (see the main `README.md` for the recommended version).
- If you are compiling on older hardware, temporarily turn off CPU‑specific features and re‑enable them later using the Performance Tuning Cookbook.

---

## 0.4 Where to Go Next

Once you have the quick‑start example running, you have options:

- **For a deeper understanding of the geometry**, continue with Chapter 2 (Mathematical Foundations) and Appendix A.
- **For system design and integration questions**, skip to Part II (System Architecture, Identifier Types, Coordinate Systems).
- **For performance and deployment concerns**, read Chapter 7 (Performance Optimization), Chapter 8 (Container Formats), and Appendix C (Benchmarks).

If you get stuck during installation or with platform‑specific quirks, Appendix D (Installation and Setup) contains an expanding troubleshooting section. For definitions of terms like BCC, LOD, Morton encoding, and Hilbert curves, see the Glossary in the back matter.
