# OctaIndex3D

<div align="center">

**A 3D Spatial Indexing and Routing System based on BCC Lattice**

[![Crates.io](https://img.shields.io/crates/v/octaindex3d.svg)](https://crates.io/crates/octaindex3d)
[![Documentation](https://docs.rs/octaindex3d/badge.svg)](https://docs.rs/octaindex3d)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82+-orange.svg)](https://www.rust-lang.org)

[Documentation](https://docs.rs/octaindex3d) | [Crates.io](https://crates.io/crates/octaindex3d) | [Examples](#examples) | [CLI Reference](#cli-usage)

</div>

## Overview

OctaIndex3D is a high-performance 3D spatial indexing and routing library based on a **Body-Centered Cubic (BCC) lattice** with **truncated octahedral cells**. It provides:

- **Uniform 3D Tiling**: Truncated octahedra tile 3D space without gaps
- **14-Neighbor Connectivity**: More isotropic than standard cubic grids (6 neighbors)
- **Hierarchical 8:1 Refinement**: Multiresolution support for efficient spatial queries
- **Robust Cell IDs**: 128-bit format with Bech32m human-readable encoding
- **First-Class Routing**: A* pathfinding with pluggable cost functions
- **Data Aggregation**: Efficient spatial queries and hierarchical roll-ups
- **Parallel Processing**: Built with Rayon for concurrent operations

## Key Advantages of Our Approach

Our system is built on a Body-Centered Cubic (BCC) lattice, which offers fundamental advantages over traditional grid-based systems for 3D spatial analysis.

### 1. Superior Efficiency and Fidelity

The BCC lattice is the optimal structure for sampling three-dimensional signals. It achieves the same level of analytical fidelity with approximately **29% fewer data points** than a standard cubic grid. This translates to significant reductions in memory usage, storage costs, and processing time for large-scale datasets, without sacrificing precision.

### 2. Enhanced Isotropy for Realistic Analysis

Spatial relationships in the real world are continuous, not confined to rigid, 90-degree angles. Our system's cells have **14 neighbors**, a significant increase from the 6 offered by cubic cells. This near-uniform connectivity in all directions results in:
- **More realistic pathfinding**: Routes are not biased along cardinal axes.
- **Smoother data interpolation**: Gradients and fields are represented more naturally.
- **Unbiased neighborhood analysis**: Operations like k-rings and spatial statistics are not distorted by grid orientation.

### 3. Consistent and Unambiguous Topology

Every cell in our system is a **truncated octahedron**, a shape that tiles 3D space perfectly without gaps or overlaps. This guarantees a consistent and unambiguous topology, which is critical for:
- **Reliable data aggregation**: No double-counting or missed regions.
- **Simplified hierarchical models**: Parent-child relationships (8:1 refinement) are clear and consistent across all resolutions.
- **Robust algorithms**: Eliminates the need for complex edge cases to handle topological inconsistencies found in other tiling systems.

## Features

### Core Capabilities

- ✅ **BCC Lattice Mathematics**: Complete implementation with parity validation
- ✅ **Cell ID System**: 128-bit format with frame, resolution, coordinates, and checksum
- ✅ **Bech32m Encoding**: Human-readable cell IDs (e.g., `cx3d1x5yfk...`)
- ✅ **Neighbor Lookup**: Fast 14-neighbor queries
- ✅ **Hierarchical Navigation**: Parent/child relationships (8:1 refinement)
- ✅ **K-Rings & K-Shells**: Breadth-first spatial queries
- ✅ **A* Pathfinding**: Shortest path with pluggable cost functions
- ✅ **Line Tracing**: 3D Bresenham-like line traversal
- ✅ **Data Layers**: Attribute storage with aggregation operations
- ✅ **I/O Support**: JSON, CBOR, GeoJSON, and optional Arrow/Parquet export

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
octaindex3d = "0.2.0"
```

### As a CLI Tool

```bash
cargo install octaindex3d
```

Or build from source:

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd octaindex3d
cargo build --release
```

## Quick Start

### Library Usage

```rust
use octaindex3d::{CellID, path::{astar, k_ring, EuclideanCost}};

// Create a cell at coordinates (0, 0, 0) at resolution 5
let cell = CellID::from_coords(0, 5, 0, 0, 0)?;

// Get its 14 neighbors
let neighbors = cell.neighbors();
assert_eq!(neighbors.len(), 14);

// Get all cells within 2 steps
let ring = k_ring(cell, 2);

// Find a path between two cells
let start = CellID::from_coords(0, 5, 0, 0, 0)?;
let goal = CellID::from_coords(0, 5, 10, 10, 10)?;
let path = astar(start, goal, &EuclideanCost)?;

println!("Path length: {} cells, cost: {:.2}", path.len(), path.cost);
```

### Working with Data Layers

```rust
use octaindex3d::layer::{Layer, Aggregation};

// Create a data layer
let mut layer = Layer::new("elevation");

// Add cell values
for cell in cells {
    layer.set(cell, elevation_value);
}

// Aggregate over a region
let sum = layer.aggregate(&region_cells, Aggregation::Sum)?;
let mean = layer.aggregate(&region_cells, Aggregation::Mean)?;

// Roll up to coarser resolution
let coarse_layer = layer.rollup(Aggregation::Mean)?;
```

## CLI Usage

The `octaindex3d` CLI provides command-line tools for spatial operations:

### Cell ID Operations

```bash
# Convert coordinates to cell ID
octaindex3d id-from-coord 2 4 6 -r 5
# Output: Cell ID with Bech32m encoding

# Decode cell ID
octaindex3d id-to-coord cx3d1x5yfk...

# Get neighbors
octaindex3d neighbors cx3d1x5yfk...

# Get parent/children
octaindex3d parent cx3d1x5yfk...
octaindex3d children cx3d1x5yfk...
```

### Spatial Queries

```bash
# K-ring (all cells within k steps)
octaindex3d k-ring cx3d1x5yfk... -k 2 --format geojson > ring.geojson

# K-shell (cells at exactly k steps)
octaindex3d k-shell cx3d1x5yfk... -k 2

# Find path between two points
octaindex3d route --start "0,0,0" --goal "10,10,10" -r 5

# Trace line between two points
octaindex3d trace-line --start "0,0,0" --end "10,0,0" -r 5
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                OctaIndex3D                      │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ Lattice  │  │  CellID  │  │  Layers  │       │
│  │          │  │          │  │          │       │
│  │ • BCC    │  │ • 128bit │  │ • Data   │       │
│  │ • Parity │  │ • Bech32m│  │ • Agg    │       │
│  │ • 14-nbr │  │ • Hier   │  │ • Flags  │       │
│  └──────────┘  └──────────┘  └──────────┘       │
│                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │   Path   │  │    I/O   │  │   CLI    │       │
│  │          │  │          │  │          │       │
│  │ • A*     │  │ • JSON   │  │ • Tools  │       │
│  │ • K-ring │  │ • CBOR   │  │ • Batch  │       │
│  │ • Trace  │  │ • GeoJSON│  │ • Script │       │
│  └──────────┘  └──────────┘  └──────────┘       │
│                                                 │
└─────────────────────────────────────────────────┘
```

## Cell ID Format (128-bit) - v0.2.0

```
┌──────────┬────────────┬──────────┬─────────┬────────────┐
│  Frame   │ Resolution │ Exponent │  Flags  │  Reserved  │
│  8 bits  │   8 bits   │  4 bits  │ 8 bits  │   4 bits   │
│  (0-7)   │   (8-15)   │ (16-19)  │ (20-27) │  (28-31)   │
├──────────┴────────────┴──────────┴─────────┴────────────┤
│                  Coordinates (96 bits)                  │
│      X (32 bits)   │   Y (32 bits)   │   Z (32 bits)    │
│      (32-63)       │     (64-95)     │    (96-127)      │
└─────────────────────────────────────────────────────────┘
```

### Field Descriptions:
- **Frame** (8 bits): Coordinate reference system (0-255)
- **Resolution** (8 bits): Level of detail (0-255, higher = finer)
- **Exponent** (4 bits): Scale factor for extreme ranges (0-15)
- **Flags** (8 bits): Cell properties
- **Reserved** (4 bits): Future expansion
- **Coordinates** (96 bits): Signed 32-bit per axis (±2.1B range each)

## Examples

### Pathfinding with Obstacles

```rust
use octaindex3d::path::{AvoidBlockedCost, astar};
use octaindex3d::layer::{Layer, CellFlags};

// Create obstacle map
let mut flags = Layer::new("obstacles");
for cell in obstacle_cells {
    let mut cell_flags = CellFlags::empty();
    cell_flags.set_flag(CellFlags::BLOCKED);
    flags.set(cell, cell_flags);
}

// Find path avoiding obstacles
let cost_fn = AvoidBlockedCost::new(flags, 1000.0);
let path = astar(start, goal, &cost_fn)?;
```

### Hierarchical Data Aggregation

```rust
// Fine-resolution data
let mut fine_layer = Layer::new("temperature");
// ... populate with sensor data ...

// Roll up to coarser resolution
let coarse_layer = fine_layer.rollup(Aggregation::Mean)?;

// Multi-level pyramid
let level2 = coarse_layer.rollup(Aggregation::Mean)?;
let level3 = level2.rollup(Aggregation::Mean)?;
```

## Performance

Benchmarks on Apple M1 Pro (preliminary):

- **Neighbor Lookup**: ~10ns per cell
- **A* Pathfinding**: ~1M expansions/sec
- **K-Ring (k=2)**: ~500ns for 211 cells
- **Cell ID Encoding**: ~200ns (Bech32m)

## Use Cases

- **Robotics**: 3D occupancy grids, UAV path planning
- **Geospatial**: Volumetric environmental data, atmospheric modeling
- **Gaming**: 3D spatial partitioning, NPC navigation
- **Scientific**: Crystallography, molecular modeling
- **Urban Planning**: 3D city models, airspace management

## Mathematical Background

The BCC lattice is defined by points whose coordinates have identical parity (all even or all odd). This creates two interpenetrating cubic sub-lattices. The Voronoi cell of each lattice point is a truncated octahedron - a 14-faced polyhedron that perfectly tiles 3D space.

**Key Properties:**
- Parity constraint: `(x + y + z) % 2 == 0` for all lattice points
- 8 opposite-parity neighbors at distance `√3` (hexagonal faces)
- 6 same-parity neighbors at distance `2` (square faces)
- 8:1 hierarchical refinement: each parent has 8 children

## License

Licensed under the MIT License.

## References

- [Wikipedia - "Truncated octahedron"](https://en.wikipedia.org/wiki/Truncated_octahedron)

---

<div align="center">

**Made with ❤️ and Rust**

[Report Bug](https://github.com/FunKite/OctaIndex3D/issues) · [Request Feature](https://github.com/FunKite/OctaIndex3D/issues)

</div>
