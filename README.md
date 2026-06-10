# OctaIndex3D

<div align="center">

**A 3D Spatial Indexing and Routing System based on BCC Lattice**

[![Crates.io](https://img.shields.io/crates/v/octaindex3d.svg)](https://crates.io/crates/octaindex3d)
[![Documentation](https://docs.rs/octaindex3d/badge.svg)](https://docs.rs/octaindex3d)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77+-orange.svg)](https://www.rust-lang.org)
[![CI](https://github.com/FunKite/OctaIndex3D/workflows/Rust%20CI/badge.svg)](https://github.com/FunKite/OctaIndex3D/actions)
[![Downloads](https://img.shields.io/crates/d/octaindex3d.svg)](https://crates.io/crates/octaindex3d)

[Documentation](https://docs.rs/octaindex3d) | [Book](https://github.com/FunKite/OctaIndex3D/blob/main/book/README.md) | [Crates.io](https://crates.io/crates/octaindex3d) | [Examples](#examples) | [Changelog](CHANGELOG.md)

</div>

## Table of Contents

- [What's New in v0.5.6](#whats-new-in-v056)
- [Overview](#overview)
- [Why BCC Lattice?](#why-bcc-lattice)
- [Interactive 3D Maze Game](#-interactive-3d-maze-game)
- [Installation](#installation)
- [Quick Start](#30-second-quick-start)
- [ID System Architecture](#id-system-architecture-v030)
- [Examples](#examples)
- [Streaming Container Format](#streaming-container-format)
- [Performance](#performance)
- [Use Cases](#use-cases)
- [Platform Support](#platform-support)
- [FAQ](#faq)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Research and Citation](#research-and-citation)

## What's New in v0.5.6

This release introduces the high-level `BccGrid` API, corrects several BCC lattice operations, and adds a new survival mode to the interactive maze.

- **New `BccGrid` facade** - Convert physical points to cells, query `neighbors`, `k_ring`, `k_shell`, and `distance`, and run A* pathfinding on modern `Route64` IDs without handling parity, tiers, or coordinate ranges manually. See the [30-Second Quick Start](#30-second-quick-start) and `examples/quickstart.rs`.
- **Lattice correctness fixes** - `physical_to_lattice` now snaps any finite in-range point to the nearest valid BCC point and honors its `resolution` parameter; `get_children` produces the 8 parity-valid children with `get_parent` as its exact inverse; `batch_validate_routes` applies the correct all-same-parity rule.
- **Legacy API deprecations** - The v0.2-era `CellID`, `path::*`, and `Layer` APIs are deprecated in favor of `BccGrid` and the modern ID types. All remain available for compatibility.
- **README doctests** - The README's Rust code blocks now compile as doctests, so documentation examples stay in sync with the implementation.
- **Bloodhound survival mode** - A new mode for `octaindex3d play`: reach the goal before pursuing bloodhounds catch you, with spike traps, scent trails, and progressive level sizing.
- **Security** - Replaced the unmaintained `serde_cbor` dependency with `ciborium` (RUSTSEC-2021-0127); the `Dataset` CBOR API is unchanged.
- **Documentation coverage** - Documented the remaining public API items (container v2 format types, GPU backends, AVX-512 feature detection) for complete docs.rs coverage.

See the full [Changelog](CHANGELOG.md) for release history.

## Overview

OctaIndex3D is a high-performance 3D spatial indexing and routing library based on a **Body-Centered Cubic (BCC) lattice** with **truncated octahedral cells**.

### 30-Second Quick Start

```bash
# Try the interactive 3D maze game (fastest way to experience BCC lattice!)
cargo install octaindex3d --features cli
octaindex3d play --difficulty medium

# Or use as a library
cargo add octaindex3d
```

```rust
use octaindex3d::BccGrid;

fn main() -> octaindex3d::Result<()> {
    let grid = BccGrid::new(0.5)?;            // cells 0.5 units across
    let cell = grid.cell_at(1.2, 3.4, 5.6)?;  // physical point -> cell
    let neighbors = grid.neighbors(cell);     // 14-neighbor connectivity
    assert_eq!(neighbors.len(), 14);

    let path = grid.astar(
        grid.cell_at(0.0, 0.0, 0.0)?,
        grid.cell_at(5.0, 5.0, 5.0)?,
    )?;                                       // A* over the lattice
    println!("{} cells, {:.2} units", path.len(), path.cost);
    Ok(())
}
```

**For code examples and tutorials**, see the [OctaIndex3D Book](https://github.com/FunKite/OctaIndex3D/blob/main/book/README.md):
- [Quick Start Guide](https://github.com/FunKite/OctaIndex3D/blob/main/book/front_matter/10_quick_start.md) - Basic usage and autonomous mapping examples
- [Chapter 10: Robotics & Autonomous Systems](https://github.com/FunKite/OctaIndex3D/blob/main/book/part4_applications/chapter10_robotics_and_autonomy.md) - Complete autonomous mapping tutorial
- [API Documentation](https://docs.rs/octaindex3d) - Full API reference

### Key Features

- 🎮 **Interactive 3D Maze Game**: Play through procedurally-generated octahedral mazes with BCC lattice pathfinding
- 🤖 **Complete Autonomous Mapping Stack**: Production-ready occupancy mapping, sensor fusion, and exploration
- **Three ID Types**: Galactic128 (global), Index64 (Morton), Route64 (local routing)
- **High Performance**: Cross-platform optimizations for modern CPU architectures
- **14-Neighbor Connectivity**: More isotropic than cubic grids (6 neighbors)
- **Space-Filling Curves**: Morton and Hilbert encoding for spatial locality
- **Hierarchical Refinement**: 8:1 parent-child relationships across resolutions
- **3D Occupancy Mapping**: Bayesian log-odds updates with multi-sensor fusion
- **Exploration Primitives**: Frontier detection, information gain, viewpoint generation
- **GPU Acceleration**: Metal and CUDA-accelerated ray casting
- **Temporal Filtering**: Dynamic environment tracking with decay
- **Compression**: 89x ratio with RLE, plus LZ4 and optional Zstd
- **ROS2 Integration**: Bridge for robotics middleware
- **Bech32m Encoding**: Human-readable IDs with checksums
- **Frame Registry**: Coordinate reference system management
- **Streaming Container Format**: Append-friendly compressed spatial data storage (v2)
- **GeoJSON Export**: WGS84 coordinate export for GIS integration

## Why BCC Lattice?

Our system is built on a Body-Centered Cubic (BCC) lattice, which offers fundamental advantages over traditional grid-based systems for 3D spatial analysis.

### 1. Superior Efficiency and Fidelity

The BCC lattice is the optimal structure for sampling three-dimensional signals. It achieves the same level of analytical fidelity with approximately **29% fewer data points** than a standard cubic grid. This translates to significant reductions in memory usage, storage costs, and processing time for large-scale datasets, without sacrificing precision.

### 2. Enhanced Isotropy for Realistic Analysis

Spatial relationships in the real world are continuous, not confined to rigid, 90-degree angles. Our system's cells have **14 neighbors**, a significant increase from the 6 offered by cubic cells. This near-uniform connectivity in all directions results in:
- **More realistic pathfinding**: Routes are not biased along cardinal axes
- **Smoother data interpolation**: Gradients and fields are represented more naturally
- **Unbiased neighborhood analysis**: Operations like k-rings and spatial statistics are not distorted by grid orientation

### 3. Consistent and Unambiguous Topology

Every cell in our system is a **truncated octahedron**, a shape that tiles 3D space perfectly without gaps or overlaps. This guarantees a consistent and unambiguous topology, which is critical for:
- **Reliable data aggregation**: No double-counting or missed regions
- **Simplified hierarchical models**: Parent-child relationships (8:1 refinement) are clear and consistent across all resolutions
- **Robust algorithms**: Eliminates the need for complex edge cases to handle topological inconsistencies found in other tiling systems

## 🎮 Interactive 3D Maze Game

Experience the power of BCC lattice pathfinding with our **interactive 3D octahedral maze game**! Navigate through procedurally-generated mazes using 14-neighbor connectivity and compete against optimal A* pathfinding.

### Features

- **Three difficulty levels**: Easy (8³), Medium (20³), Hard (40³)
- **Procedural generation**: Randomized Prim's algorithm creates unique mazes every time
- **Deterministic seeds**: Replay specific mazes or share challenges with friends
- **Competitive stats**: Track your performance against optimal A* solutions
- **Real-time feedback**: See your efficiency compared to the theoretical minimum path
- **BCC lattice navigation**: Experience true 3D movement with 14-neighbor connectivity

### How to Play

```bash
# Install the CLI (requires 'cli' feature)
cargo install octaindex3d --features cli

# Play on medium difficulty
octaindex3d play --difficulty medium

# Try a specific seed (reproducible maze)
octaindex3d play --seed 42 --size 20

# View your competitive stats
octaindex3d stats
```

### Game Controls

- **Arrow keys**: Navigate in X/Y plane
- **W/S**: Move up/down in Z axis
- **Q**: Quit game
- **Goal**: Reach the target coordinates in as few moves as possible

### Example Session

```text
🎮 3D Octahedral Maze Game - BCC Lattice Edition
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Maze: 20×20×20 | Seed: 42
Start: (0, 0, 0) → Goal: (18, 18, 18)
Optimal moves: 18 | Your moves: 19 | Efficiency: 94.7%

Position: (10, 10, 10) | Distance to goal: 13.9
Available moves: 14 (full BCC connectivity)

[Navigate with arrow keys, W/S for Z-axis, Q to quit]
```

### Performance Metrics

The game demonstrates real-world BCC lattice performance:
- **Maze generation**: <200ms for 8,000 cells using Prim's algorithm
- **A* pathfinding**: <5ms for optimal path computation
- **Memory efficient**: <10MB for medium-sized mazes

### Try the BCC-14 Demo

For a comprehensive demonstration of the algorithms powering the game:

```bash
# Run the BCC-14 Prim's → A* showcase
cargo run --release --example bcc14_prim_astar_demo

# Features:
# - Builds spanning tree on 549K valid BCC nodes in 131ms
# - Solves optimal path with A* in 1ms
# - Comprehensive validation (5/5 checks)
# - Dynamic seeding with reproducible results
```

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
# Minimal installation
octaindex3d = "0.5"

# Recommended (includes common features)
octaindex3d = { version = "0.5", features = ["hilbert", "parallel", "container_v2"] }

# Full-featured (for advanced use cases)
octaindex3d = { version = "0.5", features = ["hilbert", "parallel", "container_v2", "gis_geojson", "zstd"] }
```

### As a CLI Tool

```bash
# Install the interactive maze game and utilities
cargo install octaindex3d --features cli

# Run the maze game
octaindex3d play --difficulty medium

# Explore other CLI commands
octaindex3d --help
```

### Available Features

| Feature | Enabled by Default | Description | When to Use |
|---------|-------------------|-------------|-------------|
| **`serde`** | ✅ Yes | Serialization support | Data persistence, JSON export |
| **`parallel`** | ✅ Yes | Multi-threaded batch operations (Rayon) | Processing 1000+ items |
| **`simd`** | ✅ Yes | SIMD acceleration (BMI2, AVX2, NEON) | Performance optimization |
| **`lz4`** | ✅ Yes | LZ4 compression | Container storage |
| **`hilbert`** | No | Hilbert64 space-filling curve | Better spatial locality than Morton |
| **`container_v2`** | No | Streaming container format | Append-friendly storage, large datasets |
| **`gis_geojson`** | No | GeoJSON export (WGS84) | GIS integration (QGIS, ArcGIS) |
| **`cli`** | No | Interactive maze game & CLI utilities | Interactive use, demos |
| **`zstd`** | No | Zstd compression (slower, better ratio) | High compression needs |
| **`pathfinding`** | No | Legacy pathfinding APIs | Compatibility with v0.2.x |
| **`gpu-metal`** | No | Metal GPU acceleration (macOS) | Massive batch operations (millions) |
| **`gpu-cuda`** | No | CUDA GPU acceleration (Linux) | Massive batch operations (millions) |
| **`gpu-vulkan`** | No | Vulkan GPU acceleration (experimental) | Experimental GPU support |

**Recommended combinations:**
```toml
# For general use
octaindex3d = { version = "0.5", features = ["hilbert", "parallel"] }

# For GIS applications
octaindex3d = { version = "0.5", features = ["hilbert", "parallel", "gis_geojson"] }

# For data storage systems
octaindex3d = { version = "0.5", features = ["hilbert", "parallel", "container_v2", "zstd"] }

# For interactive development
octaindex3d = { version = "0.5", features = ["cli"] }
```

### Build from Source

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd OctaIndex3D
cargo build --release

# Run tests
cargo test

# Safer local test path (offline + locked + advisory preflight)
./scripts/safe_local_test.sh

# Run benchmarks
cargo bench

# Run the maze game
cargo run --release --features cli --bin octaindex3d -- play
```

## Core Capabilities

OctaIndex3D provides three main capability areas:

### 1. Spatial Indexing & Data Structures

- **Multiple ID types** for different use cases (Galactic128, Index64, Route64, Hilbert64)
- **14-neighbor connectivity** on BCC lattice for isotropic operations
- **Hierarchical refinement** with 8:1 parent-child relationships
- **Space-filling curves** (Morton and Hilbert) for spatial locality
- **Frame registry** for coordinate system management
- **Streaming containers** for append-friendly storage (v2 format)
- **GeoJSON export** for GIS integration

### 2. Autonomous 3D Mapping (NEW in v0.5.0)

- **Probabilistic occupancy mapping** with Bayesian log-odds updates
- **Multi-sensor fusion** for LiDAR, RGB-D, depth cameras, and radar
- **Exploration primitives**: frontier detection, information gain, next-best-view planning
- **GPU acceleration** (Metal + CUDA) for real-time ray casting
- **Temporal filtering** for dynamic environments with time decay
- **89x compression** with RLE for efficient storage
- **ROS2 integration** for robotics middleware

### 3. High-Performance Computing

- **Cross-platform SIMD** (BMI2, AVX2, NEON) for encoding operations
- **GPU backends** for massive batch operations (millions of points)
- **Parallel processing** with Rayon for multi-threaded workloads
- **Memory-efficient** representations with optional compression

**For detailed code examples and tutorials**, see the [OctaIndex3D Book](https://github.com/FunKite/OctaIndex3D/blob/main/book/README.md):
- [Quick Start Guide](https://github.com/FunKite/OctaIndex3D/blob/main/book/front_matter/10_quick_start.md) - Basic usage and autonomous mapping examples
- [Chapter 10: Robotics & Autonomous Systems](https://github.com/FunKite/OctaIndex3D/blob/main/book/part4_applications/chapter10_robotics_and_autonomy.md) - Complete autonomous mapping tutorial with working code
- [API Documentation](https://docs.rs/octaindex3d) - Full API reference

## ID System Architecture (v0.3.0+)

### Three Interoperable ID Types

```text
┌─────────────────────────────────────────────────────────────┐
│                       Galactic128                           │
│  128-bit global ID with scale, LOD, frame, and coordinates  │
│  ┌───────┬──────┬─────┬───────┬──────┬───────────────────┐  │
│  │ Scale │ Tier │ LOD │ Frame │ Attr │ Coordinates (96b) │  │
│  │ 8b    │ 2b   │ 6b  │ 8b    │ 4+4b │ X, Y, Z (32b ea)  │  │
│  └───────┴──────┴─────┴───────┴──────┴───────────────────┘  │
│  HRP: g3d1                                                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                        Index64                              │
│  64-bit Morton-encoded spatial index (Z-order curve)        │
│  ┌────┬────────┬──────┬─────┬──────────────────────────┐    │
│  │ Hdr│ Frame  │ Tier │ LOD │  Morton Code (48 bits )  │    │
│  │ 2b │ 8 bits │ 2b   │ 4b  │  16b/axis interleaved    │    │
│  └────┴────────┴──────┴─────┴──────────────────────────┘    │
│  HRP: i3d1  |  BMI2 PDEP/PEXT optimized                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                        Route64                              │
│  64-bit signed local routing coordinates                    │
│  ┌────┬────────┬──────────────────────────────────────┐     │
│  │ Hdr│ Parity │    X, Y, Z (20 bits each, signed)    │     │
│  │ 2b │  2b    │    ±524k range per axis              │     │
│  └────┴────────┴──────────────────────────────────────┘     │
│  HRP: r3d1  |  Fast neighbor lookup                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                       Hilbert64                             │
│  64-bit Hilbert curve spatial index (Gray code)             │
│  ┌────┬────────┬──────┬─────┬──────────────────────────┐    │
│  │ Hdr│ Frame  │ Tier │ LOD │  Hilbert Code (48 bits)  │    │
│  │ 2b │ 8 bits │ 2b   │ 4b  │  Better locality         │    │
│  └────┴────────┴──────┴─────┴──────────────────────────┘    │
│  HRP: h3d1  |  Requires 'hilbert' feature                   │
└─────────────────────────────────────────────────────────────┘
```

### BCC Lattice Properties

- **Parity Constraint**: `(x + y + z) % 2 == 0` for all lattice points
- **14 Neighbors**: 8 opposite-parity (distance √3) + 6 same-parity (distance 2)
- **Hierarchical**: 8:1 refinement, each parent has 8 children
- **Voronoi Cell**: Truncated octahedron (14 faces: 6 squares + 8 hexagons)

## Examples

### 🎮 Interactive Maze Game

The fastest way to experience BCC lattice pathfinding:

```bash
# Play the interactive 3D maze game
cargo run --release --features cli --bin octaindex3d -- play --difficulty medium

# Try specific challenges
cargo run --release --features cli --bin octaindex3d -- play --seed 42 --size 30

# View your stats
cargo run --release --features cli --bin octaindex3d -- stats
```

### 🚀 BCC-14 Prim's Algorithm → A* Demo

Run the comprehensive showcase example demonstrating the algorithms behind the game:

```bash
cargo run --release --example bcc14_prim_astar_demo
```

**What it demonstrates:**
- **Prim's Algorithm**: Generate spanning tree on 549,450 valid BCC nodes
- **14-Neighbor Connectivity**: All edges preserve BCC lattice parity
- **A* Pathfinding**: Heuristic-guided search with Euclidean distance
- **Performance**: 131ms tree generation, 1ms pathfinding on Apple M1 Max
- **Validation**: 5 comprehensive checks ensuring correctness

**Sample output:**
```text
🚀 BCC-14 3D Graph: Randomized Prim's Algorithm → A* Pathfinding
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Configuration
  Extent: 130×130×130 (2,197,000 total, 549,450 valid BCC)
  Seed: 42 🍀
  Start: (0, 0, 0) → Goal: (128, 128, 128)

BUILD: Prim's Algorithm
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✓ Carved 549,450 nodes (100.0% coverage) in 131 ms
  Performance: 4,194,656 nodes/sec | 11 MB memory
  Validation: ✓ Tree structure valid (E = N-1)

SOLVE: A* Pathfinding
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✓ Found path: 129 hops in 1 ms
  Performance: 200,000 nodes/sec
  Validation: ✓ All edges verified on spanning tree
```

### Pathfinding with A*

The `BccGrid` facade works directly in physical units and handles lattice details for you:

```rust
use octaindex3d::BccGrid;

fn main() -> octaindex3d::Result<()> {
    let grid = BccGrid::new(1.0)?; // 1-unit cells

    let start = grid.cell_at(0.0, 0.0, 0.0)?;
    let goal = grid.cell_at(10.0, 10.0, 10.0)?;

    // Shortest path over 14-neighbor BCC connectivity
    let path = grid.astar(start, goal)?;
    println!("Path: {} cells, {:.1} units long", path.len(), path.cost);

    // Route around obstacles with a traversability predicate
    let blocked = grid.k_ring(grid.cell_at(5.0, 5.0, 5.0)?, 1);
    let detour = grid.astar_where(start, goal, |cell| !blocked.contains(&cell))?;
    assert!(detour.cost >= path.cost);
    Ok(())
}
```

### Neighborhood Queries

```rust
use octaindex3d::BccGrid;

fn main() -> octaindex3d::Result<()> {
    let grid = BccGrid::new(0.5)?;
    let cell = grid.cell_at(1.2, 3.4, 5.6)?;

    // Every interior cell has exactly 14 neighbors
    assert_eq!(grid.neighbors(cell).len(), 14);

    // All cells within k hops (k-ring) or at exactly k hops (k-shell)
    let nearby = grid.k_ring(cell, 2);
    let shell = grid.k_shell(cell, 2);
    assert_eq!(shell.len(), nearby.len() - grid.k_ring(cell, 1).len());
    Ok(())
}
```

### Storing Data with Cells

Pair any map type with the ID types as keys:

```rust
use octaindex3d::{BccGrid, Route64};
use std::collections::HashMap;

fn main() -> octaindex3d::Result<()> {
    let grid = BccGrid::new(1.0)?;
    let mut temperatures: HashMap<Route64, f64> = HashMap::new();

    temperatures.insert(grid.cell_at(0.0, 0.0, 0.0)?, 21.5);
    temperatures.insert(grid.cell_at(2.0, 0.0, 0.0)?, 22.1);

    let mean: f64 = temperatures.values().sum::<f64>() / temperatures.len() as f64;
    assert!(mean > 21.0 && mean < 23.0);
    Ok(())
}
```

For probabilistic occupancy mapping, TSDF reconstruction, and sensor fusion, see the [`layers` module](https://docs.rs/octaindex3d/latest/octaindex3d/layers/index.html).

### Frame Registry

```rust
use octaindex3d::{FrameDescriptor, register_frame};

fn main() -> octaindex3d::Result<()> {
    // Register a custom coordinate system as frame 42
    let frame = FrameDescriptor::new(
        "LocalENU",                 // name
        "WGS-84",                   // datum
        "East-North-Up local frame", // description
        true,                       // right-handed
        1.0,                        // base unit in meters
    );
    register_frame(42, frame)?;
    Ok(())
}
```

## Streaming Container Format

The container format provides efficient storage for spatial data with streaming support:

```text
[Header] [Frame 1] [Frame 2] ... [TOC] [Footer]
```

**Features:**
- **Append-friendly**: Add data without full rewrite
- **Fast loading**: Footer + TOC enables <50ms open time for 100k frames
- **Crash recovery**: Checkpoint-based resilience
- **Compression**: LZ4 (default) or Zstd per-frame compression
- **Integrity**: Optional SHA-256 checksums
- **Configurable**: Adjust checkpoint intervals (frames/bytes)

**Use Cases:**
- Real-time sensor data streaming
- Incremental dataset updates
- Long-running data collection

## Performance

OctaIndex3D is optimized for modern CPU architectures with support for:
- **BMI2 hardware acceleration** (x86_64 Intel/AMD)
- **NEON SIMD** (Apple Silicon, ARM)
- **AVX2 vectorization** (x86_64)
- **Adaptive batch processing** with automatic threshold selection

## The Complete Autonomous Mapping Stack

OctaIndex3D now provides a **autonomous 3D mapping system** with all the layers needed for real-world robotics applications:

| Layer | Purpose | Lines | Features |
|-------|---------|-------|----------|
| **TSDF** | Surface reconstruction | 410 | Signed distance fields, mesh extraction |
| **ESDF** | Distance fields | 398 | Euclidean distance, gradient computation |
| **Occupancy** | Probabilistic mapping | 541 | Bayesian log-odds, multi-sensor fusion |
| **Temporal** | Dynamic environments | 319 | Time-decayed occupancy, moving objects |
| **Compressed** | Memory efficiency | 346 | 89x compression with RLE |
| **GPU** | Ray casting acceleration | 248 | Metal + CUDA support |
| **ROS2** | Robotics integration | 361 | Bridge for ROS2 middleware |
| **Exploration** | Autonomous navigation | 407 | Frontier detection, information gain, NBV |
| **Mesh Export** | Visualization | 398 | PLY/OBJ/STL formats |

**Total: 3,428 lines of autonomous mapping infrastructure!**

### Why "Primitives, Not Policy"?

We provide **building blocks** rather than a complete exploration planner:

**✅ What We Provide**
- `detect_frontiers()` - Find unexplored boundaries
- `information_gain_from()` - Evaluate viewpoint quality
- `generate_viewpoint_candidates()` - Sample observation poses

**❌ What You Implement**
- `next_best_view()` - Depends on robot constraints
- `exploration_path()` - Requires your path planner
- `multi_robot_planner()` - Application-specific

This gives you **flexibility**, **composability**, and **control** over your exploration strategy.

### Example Use Case: Autonomous Exploration

Using these primitives, you can build various exploration strategies:
1. **Detect frontiers** - find boundaries between known and unknown space
2. **Generate viewpoint candidates** - sample observation poses around frontiers
3. **Calculate information gain** - evaluate how much each viewpoint would reveal
4. **Select next-best-view** - balance information gain against distance/cost

**For complete working examples**, see [Chapter 10: Robotics & Autonomous Systems](https://github.com/FunKite/OctaIndex3D/blob/main/book/part4_applications/chapter10_robotics_and_autonomy.md).

## Use Cases

- 🤖 **Robotics & Autonomous Systems**: Complete mapping stack with occupancy grids, multi-sensor fusion, frontier exploration, UAV navigation, obstacle avoidance, SLAM integration
- 🎮 **Gaming & Interactive**: 3D maze games, spatial partitioning, NPC navigation with 14-neighbor pathfinding, procedural generation, voxel worlds
- **Exploration & Planning**: Next-best-view planning, information-driven exploration, viewpoint selection, coverage optimization
- **Geospatial**: Volumetric environmental data, atmospheric modeling, ocean data
- **Scientific**: Crystallography, molecular modeling, particle simulations
- **Urban Planning**: 3D city models, airspace management, building information
- **GIS Integration**: Export to WGS84 for visualization in QGIS, ArcGIS, etc.

## Platform Support

### Supported Platforms

| Platform | Architecture | Status | SIMD | GPU |
|----------|-------------|--------|------|-----|
| **Linux** | x86_64 | ✅ Full | BMI2, AVX2, AVX-512 | CUDA, Vulkan |
| **Linux** | aarch64 | ✅ Full | NEON | - |
| **macOS** | Apple Silicon (M1+) | ✅ Full | NEON | Metal |
| **macOS** | x86_64 | ✅ Full | BMI2, AVX2 | - |
| **Windows** | x86_64 | ✅ Full | BMI2, AVX2 | - |

### Minimum Requirements

- **Rust**: 1.77+ (MSRV)
- **CPU**: Any 64-bit processor
- **Memory**: 100MB+ recommended for typical workloads
- **Optional**: BMI2 support for hardware-accelerated Morton encoding (Intel Haswell+, AMD Zen+)

### GPU Acceleration (Optional)

- **Metal**: macOS with Metal-capable GPU (M1+ or Intel with Metal support)
- **CUDA**: NVIDIA GPU with CUDA 12.0+ and compute capability 5.0+
- **Vulkan**: Linux with Vulkan-capable GPU (experimental)

## FAQ

### General Questions

**Q: What is a BCC lattice?**
A: A Body-Centered Cubic lattice is a 3D crystal structure where each point has one point at the center of each cube. It's the optimal structure for sampling 3D space, requiring 29% fewer points than a cubic grid for the same fidelity.

**Q: Can I use this for 2D applications?**
A: While optimized for 3D, you can use OctaIndex3D for 2D by fixing one coordinate (e.g., z=0). Dedicated 2D spatial indexing libraries may be more efficient for purely 2D use cases.

**Q: What are the ID types used for?**
A:
- **Galactic128**: Global unique IDs with frame/tier/LOD hierarchy (128-bit)
- **Index64**: Morton-encoded IDs for spatial locality and range queries (64-bit)
- **Hilbert64**: Hilbert curve IDs with better locality than Morton (64-bit, requires `hilbert` feature)
- **Route64**: Local routing coordinates for neighbor traversal (64-bit, signed)

**Q: Is this suitable for real-time applications?**
A: Yes! OctaIndex3D is designed for high performance with SIMD acceleration, hardware Morton encoding (BMI2), and efficient neighbor lookups. The maze game demonstrates real-time pathfinding on large graphs.

### Performance Questions

**Q: Do I need a special CPU for good performance?**
A: No. OctaIndex3D works on any 64-bit CPU. However, modern CPUs with BMI2 (Intel Haswell 2013+, AMD Zen 2017+) get hardware-accelerated Morton encoding for 5-10x faster performance on encoding operations.

**Q: Should I enable the `parallel` feature?**
A: Yes, for batch operations on datasets with 1000+ items. The `parallel` feature (enabled by default) uses Rayon for multi-threaded processing.

**Q: What about GPU acceleration?**
A: GPU features are optional and experimental. They're useful for massive batch operations (millions of points) but add complexity. Start with CPU features first.

### Usage Questions

**Q: How do I convert between ID types?**
A: Each ID type exposes its coordinates, so you can construct one type from another's components. `Index64` and `Hilbert64` (with the `hilbert` feature) also support direct `TryFrom` conversion:

```rust
use octaindex3d::{Index64, Route64};

fn main() -> octaindex3d::Result<()> {
    let index = Index64::new(0, 0, 5, 100, 200, 300)?;

    // Rebuild as a local routing coordinate from the decoded coordinates
    let (x, y, z) = index.decode_coords();
    let route = Route64::new(index.scale_tier(), x as i32, y as i32, z as i32)?;
    assert_eq!((route.x(), route.y(), route.z()), (100, 200, 300));
    Ok(())
}
```

**Q: How do I get a cell's neighbors?**
A: Use the neighbor functions:
```rust
use octaindex3d::Route64;
use octaindex3d::neighbors::neighbors_route64;

fn main() -> octaindex3d::Result<()> {
    let route = Route64::new(0, 100, 200, 300)?;
    let neighbors = neighbors_route64(route); // Vec<Route64> with 14 neighbors
    assert_eq!(neighbors.len(), 14);
    Ok(())
}
```

**Q: Can I store custom data with cells?**
A: Yes, use your own HashMap or spatial data structure with IDs as keys (see [Storing Data with Cells](#storing-data-with-cells)).

## Troubleshooting

### Build Issues

**Issue**: Build fails with "feature `xyz` not found"
**Solution**: Update your Cargo.toml to use the correct feature names. See [Installation](#installation) for available features.

**Issue**: CUDA build fails
**Solution**: CUDA support requires CUDA 12.0+ and is only available on non-Windows platforms. Ensure you have CUDA toolkit installed:
```bash
# Ubuntu/Debian
sudo apt-get install nvidia-cuda-toolkit

# Verify
nvcc --version
```

**Issue**: Metal build fails on macOS
**Solution**: Ensure you're using a Metal-capable macOS version (10.11+). Update Xcode command-line tools:
```bash
xcode-select --install
```

### Runtime Issues

**Issue**: "Parity violation" error when creating coordinates
**Solution**: BCC lattice points must have *identical parity* on all three axes: all coordinates even, or all coordinates odd. Mixed parity is invalid:

```rust
use octaindex3d::Route64;

fn main() {
    // Valid: all even
    assert!(Route64::new(0, 0, 0, 0).is_ok());
    assert!(Route64::new(0, 2, 4, 6).is_ok());

    // Valid: all odd
    assert!(Route64::new(0, 1, 1, 1).is_ok());
    assert!(Route64::new(0, 3, 5, 7).is_ok());

    // Invalid: mixed parity
    assert!(Route64::new(0, 1, 1, 0).is_err());
    assert!(Route64::new(0, 2, 3, 1).is_err());
}
```

If you are converting arbitrary points, let the library snap them to the nearest valid cell instead of constructing IDs manually:

```rust
use octaindex3d::BccGrid;

fn main() -> octaindex3d::Result<()> {
    let grid = BccGrid::new(1.0)?;
    let cell = grid.cell_at(1.0, 0.0, 0.0)?; // snapped to a valid BCC cell
    println!("{}", cell);
    Ok(())
}
```

**Issue**: Morton encoding seems slow
**Solution**: If you have a modern CPU (Intel Haswell 2013+ or AMD Zen 2017+), ensure the `simd` feature is enabled (it's on by default). Check if BMI2 is being used:
```bash
# Linux
lscpu | grep bmi2

# macOS
sysctl machdep.cpu.features | grep BMI2
```

**Issue**: Container v2 files won't open
**Solution**: Ensure you're using the `container_v2` feature. V2 containers are incompatible with v0.2.x readers:
```toml
octaindex3d = { version = "0.5", features = ["container_v2"] }
```

### Getting Help

- **Documentation**: Check [docs.rs/octaindex3d](https://docs.rs/octaindex3d)
- **Examples**: See the [examples/](examples/) directory
- **Issues**: [Open an issue](https://github.com/FunKite/OctaIndex3D/issues) for bugs or feature requests
- **Discussions**: [GitHub Discussions](https://github.com/FunKite/OctaIndex3D/discussions) for questions
- **Security**: See [SECURITY.md](SECURITY.md) for reporting vulnerabilities

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:
- Code of conduct and community guidelines
- How to submit bug reports and feature requests
- Development setup and coding standards
- Pull request process and review guidelines

Feel free to:
- Open an [issue](https://github.com/FunKite/OctaIndex3D/issues) for bugs or feature requests
- Submit a pull request with improvements
- Start a [discussion](https://github.com/FunKite/OctaIndex3D/discussions) for questions or ideas
- Improve documentation or examples

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Research and Citation

For comprehensive technical coverage, see the [**OctaIndex3D Book**](https://github.com/FunKite/OctaIndex3D/blob/main/book/README.md), which provides:
- Mathematical foundations of BCC lattice geometry with rigorous proofs
- Detailed architecture and implementation recipes
- Performance benchmarks tied to real-world scenarios
- End-to-end application walkthroughs (robotics, geospatial, gaming, scientific computing)
- Working code examples and design patterns
- Advanced topics and future research directions

The book transforms theoretical foundations into practical guidance for building production systems.

## References

- [Wikipedia - "Body-centered cubic"](https://en.wikipedia.org/wiki/Body-centered_cubic)
- [Wikipedia - "Truncated octahedron"](https://en.wikipedia.org/wiki/Truncated_octahedron)
- [Bech32m Specification](https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki)
- [Morton Encoding](https://en.wikipedia.org/wiki/Z-order_curve)
- [Hilbert Curve](https://en.wikipedia.org/wiki/Hilbert_curve)

---

<div align="center">

**Made with ❤️ and Rust**

[Report Bug](https://github.com/FunKite/OctaIndex3D/issues) · [Request Feature](https://github.com/FunKite/OctaIndex3D/issues)

</div>
