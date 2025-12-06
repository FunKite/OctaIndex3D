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
octaindex3d = "0.5"
```

Then fetch dependencies:

```bash
cargo build
```

If you see build errors related to optional CPU features (BMI2, AVX2, NEON), disable them for now and come back to Appendix G (Performance Cookbook) later for tuning advice.

---

## 0.3 Your First BCC Query

Create a new binary project:

```bash
cargo new oi3d-hello
cd oi3d-hello
```

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
```

You should see one identifier for the center cell and 14 neighbor identifiers. The exact numeric values are not important yet—the point is that:

- You work with **typed coordinates** (`BccCoord`) rather than raw `(x, y, z)` triples
- You get **hierarchical identifiers** (`Index64`) that encode level of detail
- Neighbor queries are a single method call, not hand‑rolled index arithmetic

If the program does not compile:

- Make sure your `Cargo.toml` uses a released `octaindex3d` version that matches the crate on `crates.io`.
- Check that your Rust toolchain is recent enough (see the main `README.md` for the recommended version).
- If you are compiling on older hardware, temporarily turn off CPU‑specific features and re‑enable them later using the Performance Tuning Cookbook.

---

## 0.4 Autonomous Mapping (NEW in v0.5.0)

OctaIndex3D now includes a complete autonomous 3D mapping stack. Here's a quick taste:

```rust
use octaindex3d::occupancy::{OccupancyLayer, OccupancyConfig};
use octaindex3d::exploration::{FrontierDetectionConfig, InformationGainConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create an occupancy map
    let config = OccupancyConfig {
        resolution: 0.1,  // 10cm voxels
        prob_hit: 0.7,
        prob_miss: 0.4,
        clamp_min: 0.12,
        clamp_max: 0.97,
    };

    let mut layer = OccupancyLayer::new(config)?;

    // 2. Integrate a depth sensor measurement
    let sensor_pos = (0.0, 0.0, 0.0);
    let obstacle_pos = (5.0, 2.0, 1.0);
    layer.integrate_ray(sensor_pos, obstacle_pos)?;

    // 3. Detect frontiers (boundaries between known and unknown space)
    let frontier_config = FrontierDetectionConfig {
        min_cluster_size: 10,
        max_distance: 10.0,
        cluster_distance: 0.3,
    };

    let frontiers = layer.detect_frontiers(&frontier_config)?;
    println!("Found {} frontiers to explore", frontiers.len());

    // 4. Generate viewpoint candidates ranked by information gain
    let ig_config = InformationGainConfig {
        sensor_range: 5.0,
        sensor_fov: std::f32::consts::PI / 3.0,  // 60°
        ray_resolution: 5.0,
        unknown_weight: 1.0,
    };

    let candidates = layer.generate_viewpoint_candidates(&frontiers, &ig_config);

    if let Some(best) = candidates.first() {
        println!(
            "Best viewpoint: {:?} with {:.2} bits of information gain",
            best.position,
            best.information_gain
        );
    }

    Ok(())
}
```

This example demonstrates:
- **Probabilistic occupancy mapping** with Bayesian log-odds updates
- **Frontier detection** to find unexplored boundaries
- **Information gain calculation** to evaluate viewpoint quality
- **Viewpoint candidate generation** for next-best-view planning

For the complete autonomous mapping tutorial, see Chapter 10.

---

## 0.5 Where to Go Next

Once you have the quick‑start examples running, you have options:

- **For autonomous robotics applications**, jump directly to Chapter 10 (Robotics and Autonomous Systems) to see the complete mapping stack in action.
- **For a deeper understanding of the geometry**, continue with Chapter 2 (Mathematical Foundations) and Appendix A.
- **For system design and integration questions**, skip to Part II (System Architecture, Identifier Types, Coordinate Systems).
- **For performance and deployment concerns**, read Chapter 7 (Performance Optimization), Chapter 8 (Container Formats), and Appendix C (Benchmarks).

If you get stuck during installation or with platform‑specific quirks, Appendix D (Installation and Setup) contains an expanding troubleshooting section. For definitions of terms like BCC, LOD, Morton encoding, and Hilbert curves, see the Glossary in the back matter.
