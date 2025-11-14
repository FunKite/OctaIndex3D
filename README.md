# OctaIndex3D

<div align="center">

**A 3D Spatial Indexing and Routing System based on BCC Lattice**

[![Crates.io](https://img.shields.io/crates/v/octaindex3d.svg)](https://crates.io/crates/octaindex3d)
[![Documentation](https://docs.rs/octaindex3d/badge.svg)](https://docs.rs/octaindex3d)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77+-orange.svg)](https://www.rust-lang.org)
[![CI](https://github.com/FunKite/OctaIndex3D/workflows/CI/badge.svg)](https://github.com/FunKite/OctaIndex3D/actions)
[![Downloads](https://img.shields.io/crates/d/octaindex3d.svg)](https://crates.io/crates/octaindex3d)

[Documentation](https://docs.rs/octaindex3d) | [Whitepaper](WHITEPAPER.md) | [Crates.io](https://crates.io/crates/octaindex3d) | [Examples](#examples) | [Changelog](CHANGELOG.md)

</div>

## Table of Contents

- [What's New](#whats-new)
- [Overview](#overview)
- [Why BCC Lattice?](#why-bcc-lattice)
- [Interactive 3D Maze Game](#-interactive-3d-maze-game)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [ID System Architecture](#id-system-architecture-v030)
- [Examples](#examples)
- [Streaming Container Format](#streaming-container-format)
- [Performance](#performance)
- [Use Cases](#use-cases)
- [Comparison with Alternatives](#comparison-with-alternatives)
- [Platform Support](#platform-support)
- [FAQ](#faq)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Research and Citation](#research-and-citation)

## What's New

### Version 0.4.3 (Latest)

- **Interactive 3D Octahedral Maze Game**: Play procedurally-generated mazes with BCC lattice pathfinding
- **BCC-14 Prim's Algorithm Demo**: Spanning tree generation on 549K nodes with A* pathfinding
- **GitHub Community Standards**: Full CONTRIBUTING.md, issue templates, security policies
- **Enhanced Security**: CodeQL analysis and automated security scanning
- **CLI Utilities**: Encode/decode coordinates, calculate distances, explore neighbors

See the full [Changelog](CHANGELOG.md) for detailed release history.

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
use octaindex3d::{Route64, neighbors::neighbors_route64};

// Create a BCC lattice point
let point = Route64::new(0, 10, 20, 30)?;

// Get all 14 neighbors
let neighbors = neighbors_route64(point);
assert_eq!(neighbors.len(), 14);
```

### Key Features

- 🎮 **Interactive 3D Maze Game**: Play through procedurally-generated octahedral mazes with BCC lattice pathfinding
- **Three ID Types**: Galactic128 (global), Index64 (Morton), Route64 (local routing)
- **High Performance**: Cross-platform optimizations for modern CPU architectures
- **14-Neighbor Connectivity**: More isotropic than cubic grids (6 neighbors)
- **Space-Filling Curves**: Morton and Hilbert encoding for spatial locality
- **Hierarchical Refinement**: 8:1 parent-child relationships across resolutions
- **Bech32m Encoding**: Human-readable IDs with checksums
- **Compression**: LZ4 (default) and optional Zstd support
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

### Quick Start

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

```
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
octaindex3d = "0.4"

# Recommended (includes common features)
octaindex3d = { version = "0.4", features = ["hilbert", "parallel", "container_v2"] }

# Full-featured (for advanced use cases)
octaindex3d = { version = "0.4", features = ["hilbert", "parallel", "container_v2", "gis_geojson", "zstd"] }
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

| Feature | Default | Description | When to Use |
|---------|---------|-------------|-------------|
| **`serde`** | ✅ Yes | Serialization support | Data persistence, JSON export |
| **`parallel`** | ✅ Yes | Multi-threaded batch operations (Rayon) | Processing 1000+ items |
| **`simd`** | ✅ Yes | SIMD acceleration (BMI2, AVX2, NEON) | Performance optimization |
| **`lz4`** | ✅ Yes | LZ4 compression | Container storage |
| **`hilbert`** | ❌ No | Hilbert64 space-filling curve | Better spatial locality than Morton |
| **`container_v2`** | ❌ No | Streaming container format | Append-friendly storage, large datasets |
| **`gis_geojson`** | ❌ No | GeoJSON export (WGS84) | GIS integration (QGIS, ArcGIS) |
| **`cli`** | ❌ No | Interactive maze game & CLI utilities | Interactive use, demos |
| **`zstd`** | ❌ No | Zstd compression (slower, better ratio) | High compression needs |
| **`pathfinding`** | ❌ No | Legacy pathfinding APIs | Compatibility with v0.2.x |
| **`gpu-metal`** | ❌ No | Metal GPU acceleration (macOS) | Massive batch operations (millions) |
| **`gpu-cuda`** | ❌ No | CUDA GPU acceleration (Linux) | Massive batch operations (millions) |
| **`gpu-vulkan`** | ❌ No | Vulkan GPU acceleration (experimental) | Experimental GPU support |

**Recommended combinations:**
```toml
# For general use
octaindex3d = { version = "0.4", features = ["hilbert", "parallel"] }

# For GIS applications
octaindex3d = { version = "0.4", features = ["hilbert", "parallel", "gis_geojson"] }

# For data storage systems
octaindex3d = { version = "0.4", features = ["hilbert", "parallel", "container_v2", "zstd"] }

# For interactive development
octaindex3d = { version = "0.4", features = ["cli"] }
```

### Build from Source

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd OctaIndex3D
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Run the maze game
cargo run --release --features cli --bin octaindex3d -- play
```

## Quick Start

### Basic Usage

```rust
use octaindex3d::{Galactic128, Index64, Route64, Result};

fn main() -> Result<()> {
    // Create a global ID (128-bit)
    let galactic = Galactic128::new(0, 5, 1, 10, 0, 2, 4, 6)?;
    println!("Galactic ID: {}", galactic.to_bech32m()?);

    // Create a Morton-encoded index (64-bit)
    let index = Index64::new(0, 0, 5, 100, 200, 300)?;
    println!("Morton coordinates: {:?}", index.decode_coords());

    // Create a local routing coordinate (64-bit)
    let route = Route64::new(0, 100, 200, 300)?;
    println!("Route: ({}, {}, {})", route.x(), route.y(), route.z());

    // Get 14 neighbors
    let neighbors = octaindex3d::neighbors::neighbors_route64(route);
    assert_eq!(neighbors.len(), 14);

    Ok(())
}
```

### Working with Hilbert Curves

```rust
use octaindex3d::{Hilbert64, Index64};

// Create Hilbert-encoded ID (better spatial locality than Morton)
let hilbert = Hilbert64::new(0, 0, 5, 100, 200, 300)?;

// Hierarchical operations
let parent = hilbert.parent().unwrap();
let children = hilbert.children();

// Convert between Morton and Hilbert
let index: Index64 = hilbert.into();
let hilbert2: Hilbert64 = index.try_into()?;

// Batch encoding
let coords = vec![(0, 0, 0), (1, 1, 1), (2, 2, 2)];
let hilbert_ids = Hilbert64::encode_batch(&coords, 0, 0, 5)?;
```

### Streaming Container Storage

```rust
use octaindex3d::container_v2::{ContainerWriterV2, StreamConfig};
use std::fs::File;

// Create streaming container with append support
let file = File::create("data.octa3d")?;
let config = StreamConfig {
    checkpoint_frames: 1000,
    checkpoint_bytes: 64 * 1024 * 1024,
    enable_sha256: false,
};

let mut writer = ContainerWriterV2::new(file, config)?;

// Write spatial data frames
for data in spatial_data {
    writer.write_frame(&data)?;
}

writer.finish()?; // Writes final TOC and footer
```

### GeoJSON Export

```rust
use octaindex3d::geojson::{to_geojson_points, write_geojson_linestring, GeoJsonOptions};
use std::path::Path;

// Export points to GeoJSON
let ids = vec![
    Galactic128::new(0, 0, 0, 0, 0, 0, 0, 0)?,
    Galactic128::new(0, 0, 0, 0, 0, 1000, 1000, 0)?,
];

let opts = GeoJsonOptions {
    include_properties: true,
    precision: 7, // ~1cm precision
};

let geojson = to_geojson_points(&ids, &opts);
println!("{}", serde_json::to_string_pretty(&geojson)?);

// Export path as LineString
write_geojson_linestring(Path::new("path.geojson"), &path_ids, &opts)?;
```

## ID System Architecture (v0.3.0+)

### Three Interoperable ID Types

```
┌─────────────────────────────────────────────────────────────┐
│                       Galactic128                           │
│  128-bit global ID with frame, tier, LOD, and coordinates   │
│  ┌────────┬──────┬─────┬──────┬──────────────────────────┐  │
│  │ Frame  │ Tier │ LOD │ Attr │    Coordinates (90b)     │  │
│  │ 8 bits │ 2b   │ 4b  │ 24b  │    X, Y, Z (30b each)    │  │
│  └────────┴──────┴─────┴──────┴──────────────────────────┘  │
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
```
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

```rust
use octaindex3d::{Route64, path::{astar, EuclideanCost}};

let start = Route64::new(0, 0, 0, 0)?;
let goal = Route64::new(0, 10, 10, 10)?;

// Use legacy pathfinding (from v0.2.0)
use octaindex3d::CellID;
let start_cell = CellID::from_coords(0, 5, 0, 0, 0)?;
let goal_cell = CellID::from_coords(0, 5, 10, 10, 10)?;
let path = astar(start_cell, goal_cell, &EuclideanCost)?;

println!("Path length: {} cells", path.len());
```

### Data Layers and Aggregation

```rust
use octaindex3d::layer::{Layer, Aggregation};

// Create data layer (legacy API from v0.2.0)
let mut layer = Layer::new("temperature");

for cell in cells {
    layer.set(cell, temperature_value);
}

// Aggregate over region
let mean_temp = layer.aggregate(&region_cells, Aggregation::Mean)?;

// Roll up to coarser resolution
let coarse_layer = layer.rollup(Aggregation::Mean)?;
```

### Frame Registry

```rust
use octaindex3d::frame::{FrameDescriptor, register_frame};

// Register custom coordinate system
let frame = FrameDescriptor {
    id: 1,
    name: "LocalENU".to_string(),
    description: "East-North-Up local frame".to_string(),
    base_unit: 1.0, // meters
    origin: [0.0, 0.0, 0.0],
    srid: None,
};

register_frame(frame)?;
```

## Streaming Container Format

The container format provides efficient storage for spatial data with streaming support:

```
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

For detailed performance analysis and benchmarks, see:
- [Performance Guide](PERFORMANCE.md) - Usage examples and optimization tips
- [CPU Comparison](docs/CPU_COMPARISON.md) - Cross-platform performance analysis
- [Benchmark Suite](benches/README.md) - Criterion benchmarks and profiling tools

## Use Cases

- 🎮 **Gaming & Interactive**: 3D maze games, spatial partitioning, NPC navigation with 14-neighbor pathfinding, procedural generation, voxel worlds
- **Robotics**: 3D occupancy grids, UAV path planning, obstacle avoidance
- **Geospatial**: Volumetric environmental data, atmospheric modeling, ocean data
- **Scientific**: Crystallography, molecular modeling, particle simulations
- **Urban Planning**: 3D city models, airspace management, building information
- **GIS Integration**: Export to WGS84 for visualization in QGIS, ArcGIS, etc.

## Comparison with Alternatives

| Feature | OctaIndex3D (BCC) | H3 (Hexagonal) | S2 (Spherical) | Octree |
|---------|-------------------|----------------|----------------|--------|
| **Dimensionality** | 3D | 2D (Earth surface) | 2D (Sphere) | 3D |
| **Cell Shape** | Truncated Octahedron | Hexagon | Spherical quad | Cube |
| **Neighbors** | 14 (uniform) | 6 | 4-8 (variable) | 6-26 |
| **Isotropy** | Excellent | Good | Excellent | Poor |
| **Hierarchical** | Yes (8:1) | Yes (7:1) | Yes (4:1) | Yes (8:1) |
| **Space-Filling Curve** | Morton/Hilbert | H3 | S2 Cell | Z-order |
| **Efficiency vs Cubic** | +29% | N/A | N/A | Baseline |
| **Best For** | 3D volumes | Geospatial 2D | Global spherical | Adaptive 3D |
| **Rust Native** | Yes | No (C bindings) | No (C++) | Various |

**When to choose OctaIndex3D:**
- You need true 3D volumetric indexing (not just surface)
- You want optimal sampling efficiency (29% fewer points than cubic)
- You need isotropic neighbor relationships for pathfinding or analysis
- You're working with atmospheric, oceanic, geological, or urban 3D data
- You want a pure Rust implementation with modern performance features

## Platform Support

### Supported Platforms

| Platform | Architecture | Status | SIMD | GPU |
|----------|-------------|--------|------|-----|
| **Linux** | x86_64 | ✅ Full | BMI2, AVX2, AVX-512 | CUDA, Vulkan |
| **Linux** | aarch64 | ✅ Full | NEON | - |
| **macOS** | Apple Silicon (M1+) | ✅ Full | NEON | Metal |
| **macOS** | x86_64 | ✅ Full | BMI2, AVX2 | - |
| **Windows** | x86_64 | ✅ Full | BMI2, AVX2 | - |
| **Windows** | aarch64 | ⚠️ Tier 2 | NEON | - |

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

**Q: How does this compare to octrees?**
A: While octrees partition space hierarchically, OctaIndex3D uses a regular BCC lattice with truncated octahedral cells. This provides consistent topology, isotropic neighbor relationships, and efficient space-filling curves, making it better for uniform spatial indexing and pathfinding.

**Q: Can I use this for 2D applications?**
A: While optimized for 3D, you can use OctaIndex3D for 2D by fixing one coordinate (e.g., z=0). However, dedicated 2D libraries like H3 may be more efficient for purely 2D use cases.

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
A: Use the `From`/`Into` traits:
```rust
let index: Index64 = galactic128.try_into()?;
let hilbert: Hilbert64 = index.try_into()?;
let route: Route64 = index.try_into()?;
```

**Q: How do I get a cell's neighbors?**
A: Use the neighbor functions:
```rust
use octaindex3d::neighbors::neighbors_route64;
let neighbors = neighbors_route64(route); // Returns Vec<Route64> with 14 neighbors
```

**Q: Can I store custom data with cells?**
A: Yes, use your own HashMap or spatial data structure with IDs as keys. For legacy code, see the `Layer` API in the documentation.

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
**Solution**: BCC lattice points must satisfy `(x + y + z) % 2 == 0`. Ensure your coordinates follow this constraint:
```rust
// Valid BCC points (even sum)
Route64::new(0, 0, 0, 0)?;  // 0+0+0 = 0 ✓
Route64::new(0, 1, 1, 0)?;  // 1+1+0 = 2 ✓
Route64::new(0, 2, 3, 1)?;  // 2+3+1 = 6 ✓

// Invalid (odd sum)
Route64::new(0, 1, 0, 0)?;  // 1+0+0 = 1 ✗ Error!
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
octaindex3d = { version = "0.4", features = ["container_v2"] }
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

Copyright (c) 2025 Michael A. McLarney

## Research and Citation

For an in-depth technical analysis, see the [**OctaIndex3D Whitepaper**](WHITEPAPER.md), which covers:
- Mathematical foundations of BCC lattice geometry
- Detailed architecture and implementation
- Performance benchmarks and analysis
- Applications across multiple domains
- Future research directions

If you use OctaIndex3D in academic work, please cite:

```bibtex
@techreport{mclarney2025octaindex3d,
  title={OctaIndex3D: A High-Performance 3D Spatial Indexing System Based on Body-Centered Cubic Lattice},
  author={McLarney, Michael A. and Claude},
  year={2025},
  institution={GitHub},
  url={https://github.com/FunKite/OctaIndex3D}
}
```

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
