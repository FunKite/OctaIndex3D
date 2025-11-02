# Release Notes: OctaIndex3D v0.4.3

**Release Date:** 2025-11-02
**Version:** 0.4.3
**Repository:** https://github.com/FunKite/OctaIndex3D
**Crates.io:** https://crates.io/crates/octaindex3d

---

## 🎮 Interactive 3D Maze Game & Advanced Demos

This release introduces an **interactive CLI maze game** built on the BCC lattice, along with comprehensive demos showcasing the power of BCC-14 connectivity for pathfinding algorithms.

---

## 🆕 What's New in v0.4.3

### Interactive 3D Octahedral Maze Game CLI
- **Play mazes with difficulty levels:** Easy, Medium, and Hard
- **Compete against A* pathfinding:** See how your solution compares to optimal paths
- **Track statistics:** View your solve times, path lengths, and efficiency
- **Full CLI integration:** Run with `cargo run --release --features cli -- play`

### BCC-14 Prim's Algorithm → A* Demo
- **Comprehensive showcase** of spanning tree generation on 549K BCC lattice nodes
- **Prim's randomized algorithm:** Builds maze structure using all 14 BCC neighbors
- **A* pathfinding:** Finds shortest paths through carved tree structure
- **Performance metrics:**
  - Build time: 131 ms (8x under 1.0s target)
  - Solve time: 1 ms (200x under 200ms target)
  - Memory: 11 MB (32x under 350MB target)
- **Dynamic seeding:** Reproducible runs with `--seed=<u64>` argument
- **5/5 validation checks:** Spanning tree property, full coverage, frontier deduplication, path verification, BFS cross-check

### GitHub Community Standards
- **CONTRIBUTING.md** - Contribution guidelines
- **Issue templates** - Bug reports and feature requests
- **PR template** - Pull request guidelines
- **Community guidelines** - Code of conduct improvements

### Security Enhancements
- **CodeQL security analysis** - Automatic scanning workflow
- **Dependency auditing** - Enhanced security monitoring

### CLI Utility Functions
- **Encode/decode coordinates** - Convert between coordinate systems
- **Calculate distances** - BCC lattice distance calculations
- **Get BCC neighbors** - Query all 14 neighbors for any point

---

## 🔄 Dependency Updates

### Major Version Updates
- **`ordered-float`** 4.6.0 → 5.1.0 (breaking change)
- **`rand`** 0.8.5 → 0.9.2 (breaking change)
- **`criterion`** (dev) 0.5.1 → 0.7.0

### Minor Updates
- **`proptest`** (dev) 1.8.0 → 1.9.0
- **`github/codeql-action`** v3 → v4

### Other Changes
- Simplified CI/CD pipeline for better reliability
- Updated GitHub Actions MSRV check to Rust 1.77
- Revised Code of Conduct for improved clarity

---

## 🐛 Bug Fixes

1. **Half Dependency Fix**
   - Downgraded `half` to v2.4.1 to avoid yanked version
   - Resolved transitive dependency conflict via Criterion → CBOR stack

2. **Code Quality Improvements**
   - Fixed all remaining clippy errors and warnings
   - Fixed cargo-deny configuration for better compatibility

3. **Platform-Specific Fixes**
   - Fixed CUDA test failures with proper panic handling
   - Fixed AVX-512 type errors in SIMD batch operations
   - Fixed platform-specific GPU module guards

---

## 📦 Installation

### From Crates.io

```toml
[dependencies]
octaindex3d = "0.4.3"

# With CLI features for the maze game
octaindex3d = { version = "0.4.3", features = ["cli"] }

# With recommended features (default includes parallel + simd)
octaindex3d = { version = "0.4.3", default-features = true }

# With all optional features
octaindex3d = { version = "0.4.3", features = ["hilbert", "container_v2", "gis_geojson", "cli"] }
```

### Play the Maze Game

```bash
# Install from crates.io
cargo install octaindex3d --features cli

# Or run from source
git clone https://github.com/FunKite/OctaIndex3D
cd octaindex3d
cargo run --release --features cli -- play --difficulty easy
```

### Run the BCC-14 Demo

```bash
# From source
cargo run --release --example bcc14_prim_astar_demo

# With custom seed for reproducibility
cargo run --release --example bcc14_prim_astar_demo -- --seed=42
```

---

## ✨ Key Features

- **Three ID Types**: Galactic128 (global), Index64 (Morton), Route64 (local routing)
- **High Performance**: Cross-platform optimizations (Apple Silicon, AMD, Intel)
- **14-Neighbor Connectivity**: More isotropic than cubic grids (6 neighbors)
- **Space-Filling Curves**: Morton and Hilbert encoding for spatial locality
- **Hierarchical Refinement**: 8:1 parent-child relationships across resolutions
- **Bech32m Encoding**: Human-readable IDs with checksums
- **Compression**: LZ4 (default) and optional Zstd support
- **Streaming Container Format**: Append-friendly compressed spatial data storage
- **GeoJSON Export**: WGS84 coordinate export for GIS integration
- **Interactive CLI**: 3D maze game with pathfinding demonstrations

---

## 🔧 Feature Flags

### Performance Features (Enabled by Default)
- **`parallel`** - Multi-threaded batch operations with Rayon
- **`simd`** - SIMD-accelerated operations (BMI2, AVX2, NEON)
- **`serde`** - Serialization support
- **`lz4`** - LZ4 compression

### Optional Features
- **`cli`** - Interactive CLI maze game and utilities
- **`hilbert`** - Hilbert64 space-filling curve (better locality than Morton)
- **`container_v2`** - Streaming container format with checkpoints
- **`gis_geojson`** - GeoJSON export with WGS84 coordinates
- **`zstd`** - Zstd compression (in addition to LZ4)
- **`pathfinding`** - Advanced pathfinding with petgraph

### GPU Features (Not Recommended)
- **`gpu-metal`**, **`gpu-vulkan`**, **`gpu-cuda`** - CPU is 10x faster

---

## 🎯 BCC-14 Connectivity Advantages

The BCC (Body-Centered Cubic) lattice with 14-neighbor connectivity offers significant advantages over traditional cubic grids:

- **More isotropic:** Equal distance to all 14 neighbors
- **Better pathfinding:** Diagonal moves integrated into lattice structure
- **Optimal packing:** Truncated octahedral cells tile 3D space perfectly
- **Efficient mazes:** Prim's algorithm preserves optimal connectivity
- **Theoretical optimality:** Many paths at or near theoretical minimum length

---

## 📊 Cross-Platform Performance

### Apple Silicon (M1 Max)
- **Morton Decode:** 157M ops/sec
- **Batch Neighbors:** 50M routes/sec (10K batch)
- **Index64 Encode:** 467M elem/sec
- **Maze Generation:** 131 ms (549K nodes)
- **A* Pathfinding:** 1 ms (tree-constrained)

### AMD EPYC 7R13 (Zen 3)
- **Morton Decode:** 505M ops/sec (BMI2 hardware)
- **Batch Neighbors (small):** 32.1M routes/sec

### Intel Xeon 8488C (Sapphire Rapids)
- **Morton Decode:** 424M ops/sec
- **Batch Neighbors (large):** 37.8M routes/sec

---

## 📚 Documentation

**Included in Crate:**
- README.md - Quick start and examples
- LICENSE - MIT License

**Available on GitHub:**
- WHITEPAPER.md - Comprehensive technical analysis
- PERFORMANCE.md - Performance guide and benchmarks
- CPU_COMPARISON.md - Cross-platform performance analysis
- GPU_ACCELERATION.md - Why CPU is faster than GPU
- OPTIMIZATION_GUIDE.md - Tier-1 CPU/GPU optimization guide
- CONTRIBUTING.md - How to contribute to the project

---

## 🔄 Migration from v0.4.2

v0.4.3 maintains API compatibility with v0.4.2. Update your Cargo.toml:

```toml
[dependencies]
octaindex3d = "0.4.3"
```

### Breaking Changes in Dependencies

Note that `ordered-float` and `rand` have been updated to new major versions. If you use these dependencies directly, you may need to update:

- `ordered-float` 4.x → 5.x
- `rand` 0.8.x → 0.9.x

---

## 🎯 Performance Tips

### Compiler Flags (Recommended)

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Enables:
- **Apple Silicon:** NEON instructions, Firestorm/Icestorm optimizations
- **AMD:** BMI2, AVX2, Zen 3+ tuning
- **Intel:** BMI2, AVX2, architecture-specific tuning

### Batch Size Recommendations

- **Small (<100):** Default API, optimized for prefetching
- **Medium (100-50K):** Batch operations, cache-blocked processing
- **Large (>50K):** Parallel processing automatically engaged

---

## ✅ Quality Metrics

```
✓ Compiler warnings: 0
✓ Test pass rate: 100% (110/110)
✓ Clippy warnings: 0
✓ Build time: ~5.5s
✓ Documentation: Complete
```

---

## 🙏 Acknowledgments

This release was developed with extensive AI assistance:
- Interactive maze game implementation by Claude (Anthropic)
- BCC-14 algorithm demos and validation
- Community standards and security enhancements
- Dependency management and testing infrastructure

---

## 📝 License

Licensed under the MIT License.

Copyright (c) 2025 Michael A. McLarney

---

## 🔗 Links

- **Repository:** https://github.com/FunKite/OctaIndex3D
- **Documentation:** https://docs.rs/octaindex3d
- **Crates.io:** https://crates.io/crates/octaindex3d
- **Issues:** https://github.com/FunKite/OctaIndex3D/issues

---

## 🔮 Future Improvements

1. **Monitor Half Dependency** - Watch for future `half` releases (currently pinned to v2.4.1)
2. **AVX-512 Implementation** - Potential 2x speedup for Intel Xeon
3. **Improved AMD Large Batch Performance** - Target 40M routes/sec
4. **Advanced NEON Optimizations** - Further Apple Silicon improvements

---

**Thank you for using OctaIndex3D!**

For detailed technical information, visit the [GitHub repository](https://github.com/FunKite/OctaIndex3D).
