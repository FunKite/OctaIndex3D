# OctaIndex3D

<div align="center">

**A 3D Spatial Indexing and Routing System based on BCC Lattice**

[![Crates.io](https://img.shields.io/crates/v/octaindex3d.svg)](https://crates.io/crates/octaindex3d)
[![Documentation](https://docs.rs/octaindex3d/badge.svg)](https://docs.rs/octaindex3d)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82+-orange.svg)](https://www.rust-lang.org)

[Documentation](https://docs.rs/octaindex3d) | [Crates.io](https://crates.io/crates/octaindex3d) | [Examples](#examples)

</div>

## Overview

OctaIndex3D is a high-performance 3D spatial indexing and routing library based on a **Body-Centered Cubic (BCC) lattice** with **truncated octahedral cells**. Version 0.3.1 introduces a unified ID system with three interoperable formats, space-filling curves, and streaming container support.

### Key Features

- **Three ID Types**: Galactic128 (global), Index64 (Morton), Route64 (local routing)
- **14-Neighbor Connectivity**: More isotropic than cubic grids (6 neighbors)
- **Space-Filling Curves**: Morton and Hilbert encoding for spatial locality
- **Hierarchical Refinement**: 8:1 parent-child relationships across resolutions
- **Bech32m Encoding**: Human-readable IDs with checksums
- **Compression**: LZ4 (default) and optional Zstd support
- **Frame Registry**: Coordinate reference system management
- **Container Formats**: Compressed spatial data storage with v2 streaming support
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

## Installation

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
octaindex3d = "0.3.1"

# Optional features
octaindex3d = { version = "0.3.1", features = ["hilbert", "container_v2", "gis_geojson"] }
```

### Available Features

- **`hilbert`**: Hilbert64 space-filling curve with better locality than Morton
- **`container_v2`**: Append-friendly streaming container format with checkpoints
- **`gis_geojson`**: GeoJSON export with WGS84 coordinate conversion
- **`zstd_compression`**: Zstd compression (in addition to default LZ4)

### Build from Source

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd octaindex3d
cargo build --release
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

### Container Storage (v2)

```rust
use octaindex3d::container_v2::{ContainerWriterV2, StreamConfig};
use std::fs::File;

// Create streaming container
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
│  │ Frame  │ Tier │ LOD │ Attr │    Coordinates (90b)    │  │
│  │ 8 bits │ 2b   │ 4b  │ 24b  │    X, Y, Z (30b each)   │  │
│  └────────┴──────┴─────┴──────┴──────────────────────────┘  │
│  HRP: g3d1                                                   │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                        Index64                              │
│  64-bit Morton-encoded spatial index (Z-order curve)        │
│  ┌────┬────────┬──────┬─────┬──────────────────────────┐    │
│  │ Hdr│ Frame  │ Tier │ LOD │  Morton Code (48 bits)  │    │
│  │ 2b │ 8 bits │ 2b   │ 4b  │  16b/axis interleaved   │    │
│  └────┴────────┴──────┴─────┴──────────────────────────┘    │
│  HRP: i3d1  |  BMI2 PDEP/PEXT optimized                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                        Route64                              │
│  64-bit signed local routing coordinates                    │
│  ┌────┬────────┬──────────────────────────────────────┐     │
│  │ Hdr│ Parity │    X, Y, Z (20 bits each, signed)   │     │
│  │ 2b │  2b    │    ±524k range per axis            │     │
│  └────┴────────┴──────────────────────────────────────┘     │
│  HRP: r3d1  |  Fast neighbor lookup                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                       Hilbert64                             │
│  64-bit Hilbert curve spatial index (Gray code)             │
│  ┌────┬────────┬──────┬─────┬──────────────────────────┐    │
│  │ Hdr│ Frame  │ Tier │ LOD │  Hilbert Code (48 bits) │    │
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

## Container Formats

### Container v1 (Default)

Simple compressed format with frame headers and CRC32 checksums:

```
[Header] [Frame 1] [Frame 2] ... [Frame N]
```

- Fixed header with metadata
- Per-frame compression (LZ4 or Zstd)
- Sequential write/read
- CRC32 integrity checking

### Container v2 (Feature: `container_v2`)

Append-friendly streaming format:

```
[Header] [Frame 1] [Frame 2] ... [TOC] [Footer]
```

**Features:**
- Append without full rewrite
- Fast open via footer + TOC (target: <50ms for 100k frames)
- Checkpoint-based crash recovery
- Optional SHA-256 integrity
- Configurable checkpoint intervals (frames/bytes)

**Use Cases:**
- Real-time sensor data streaming
- Incremental dataset updates
- Long-running data collection

## Performance

Preliminary benchmarks on Apple M1 Pro:

| Operation | Time | Notes |
|-----------|------|-------|
| Index64 creation | ~5ns | Morton encoding |
| Hilbert64 creation | ~8ns | Gray code transform |
| Neighbor lookup (14) | ~10ns | BCC lattice |
| Bech32m encode | ~200ns | With checksum |
| Bech32m decode | ~250ns | With validation |
| Container write | ~50µs/frame | LZ4 compression |
| A* pathfinding | ~1M nodes/sec | Legacy API |

## Use Cases

- **Robotics**: 3D occupancy grids, UAV path planning, obstacle avoidance
- **Geospatial**: Volumetric environmental data, atmospheric modeling, ocean data
- **Gaming**: 3D spatial partitioning, NPC navigation, voxel worlds
- **Scientific**: Crystallography, molecular modeling, particle simulations
- **Urban Planning**: 3D city models, airspace management, building information
- **GIS Integration**: Export to WGS84 for visualization in QGIS, ArcGIS, etc.

## Migration from v0.2.0

Version 0.3.0+ introduces a new ID system while maintaining backward compatibility:

```rust
// Old API (still works)
use octaindex3d::CellID;
let cell = CellID::from_coords(0, 5, 100, 200, 300)?;

// New API (recommended)
use octaindex3d::{Galactic128, Index64};
let galactic = Galactic128::new(0, 5, 0, 0, 0, 100, 200, 300)?;
let index = Index64::new(0, 0, 5, 100, 200, 300)?;
```

The legacy `CellID` API remains available for compatibility but new projects should use the v0.3.0+ ID types.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright (c) 2025 Michael A. McLarney

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
