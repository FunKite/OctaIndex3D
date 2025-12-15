# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Updated `criterion` from 0.7.0 to 0.8.1
- Updated `bech32` from 0.11.0 to 0.11.1
- Updated `zerocopy` from 0.8.28 to 0.8.31
- Updated `cudarc` from 0.18.1 to 0.18.2
- Updated GitHub Actions `actions/cache` from v4 to v5

## [0.5.1] - 2025-12-05

### Added
- **ARM64 NEON intrinsics** for Apple Silicon optimization
  - `batch_manhattan_distance_neon()`: SIMD-accelerated distance calculations
  - `batch_bounding_box_query_neon()`: SIMD-accelerated spatial queries
  - Auto-detection and dispatch on aarch64 targets
- **AVX-512 support** for Intel Xeon processors
  - `batch_euclidean_distance_squared_avx512()`: True 64-bit multiply via `_mm512_mullox_epi64`
  - 8-wide SIMD lanes (vs 4 with AVX2) for 2x throughput
  - Auto-detection via `avx512f` + `avx512dq` feature flags

### Changed
- Updated `zerocopy` from 0.8.27 to 0.8.28
- Updated `clap` from 4.5.52 to 4.5.53
- Updated GitHub Actions `actions/checkout` from v5 to v6

### Fixed
- Fixed version mismatch in `src/lib.rs` doc comment (v0.4.3 → v0.5.0)
- Removed unused `COORD_BITS` constant from `Route64`

### Performance
- Added `#[cold]` and `#[inline(never)]` to error paths for better instruction cache utilization
  - `Parity::invalid_parity_error()` - cold path for parity validation
  - `Route64::invalid_tier_error()` - cold path for tier validation
  - `Route64::coord_out_of_range_error()` - cold path for range validation
- Added `#[inline]` hints to hot path functions (`Parity::from_coords`, `Route64::new`)

## [0.5.0] - 2025-11-19

### Added
- **Exploration Primitives** for autonomous navigation
  - `Frontier` detection: clustering of unknown/free boundaries
  - Information gain calculation from viewpoints
  - Viewpoint candidate generation with ranking
  - Building blocks for Next-Best-View (NBV) planning
  - No prescribed policy - users control exploration strategy
- **3D Occupancy Framework** with probabilistic sensor fusion
  - `OccupancyLayer`: Bayesian log-odds updates for probabilistic mapping
  - `TemporalOccupancyLayer`: Time-aware occupancy with decay for dynamic environments
  - `CompressedOccupancyLayer`: Block-based compression (10-100x) for large maps
  - GPU-accelerated ray casting (Metal for Apple Silicon, CUDA for NVIDIA)
  - ROS2 integration bridge with OccupancyGrid and PointCloud2 message types
- **Layered 3D Mapping System**
  - `TSDFLayer`: Truncated Signed Distance Fields for surface reconstruction
  - `ESDFLayer`: Euclidean Signed Distance Fields for path planning
  - `MeshLayer`: Surface extraction with marching tetrahedra
  - Unified measurement system for depth, TSDF, occupancy, and ESDF data
- **Mesh Export Formats**
  - PLY (Stanford Polygon File Format) - ASCII and binary
  - OBJ (Wavefront Object) - with normals
  - STL (Stereolithography) - ASCII and binary
- **Advanced Occupancy Features**
  - Temporal filtering with configurable decay rates
  - Multiple compression methods: None, LZ4, RLE, Octree
  - Multi-sensor fusion with noise tolerance
  - Ray integration for depth camera simulation
  - Frontier detection for autonomous exploration
- **Examples**
  - `occupancy_fusion.rs`: Bayesian fusion, multi-sensor integration
  - `advanced_occupancy.rs`: GPU, temporal filtering, compression, ROS2
  - `tsdf_reconstruction.rs`: Surface reconstruction from depth
  - `mesh_reconstruction.rs`: Mesh extraction and export
  - `esdf_path_planning.rs`: Distance field path planning

### Performance
- GPU ray casting: 100-1000x speedup on large batches
- Compressed storage: 10-100x memory reduction for sparse maps
- Temporal pruning: Automatic cleanup of stale data
- Block-based compression: 8x8x8 voxel blocks for cache efficiency

### Integration
- ROS2-compatible message types (nav_msgs, sensor_msgs)
- Serde serialization for JSON/CDR export
- BCC lattice integration with 14-neighbor connectivity

## [0.4.4] - 2025-11-18

### Changed
- Updated Rust toolchain from 1.82.0 to 1.91.1
- Updated `lz4_flex` from 0.11.5 to 0.12.0
- Updated `cudarc` from 0.17.8 to 0.18.1
- Updated `clap` from 4.5.51 to 4.5.52
- Updated `metal` from 0.29.0 to 0.32.0
- Updated `glam` from 0.29.3 to 0.30.9
- Updated `pollster` from 0.3.0 to 0.4.0
- Updated `crossterm` from 0.28.1 to 0.29.0
- Updated `zerocopy` from 0.7.35 to 0.8.27

### Fixed
- Fixed clippy lint issues for Rust 1.91.1 (removed invalid `manual_is_multiple_of` lint, added `comparison_chain` allows)
- Added advisory ignore for unmaintained `paste` crate (RUSTSEC-2024-0436) used by metal/wgpu dependencies
- Fixed CUDA backend for cudarc 0.17.7 API changes (CudaDevice → CudaContext)

## [0.4.3] - 2025-11-02

### Added
- **Interactive 3D Octahedral Maze Game CLI** - Play mazes with difficulty levels (easy/medium/hard), compete against A* pathfinding, and track statistics
- **BCC-14 Prim's Algorithm → A* Demo** - Comprehensive example showing spanning tree generation on 549K BCC lattice nodes with pathfinding
- **GitHub Community Standards** - CONTRIBUTING.md, issue templates, PR template, and community guidelines
- **Security Enhancements** - CodeQL security analysis workflow with automatic scanning
- CLI utility functions: encode/decode coordinates, calculate distances, get BCC neighbors

### Changed
- Updated `ordered-float` from 4.6.0 to 5.1.0 (major version)
- Updated `rand` from 0.8.5 to 0.9.2 (major version)
- Updated `criterion` (dev) from 0.5.1 to 0.7.0
- Updated `proptest` (dev) from 1.8.0 to 1.9.0
- Updated `github/codeql-action` workflow from v3 to v4
- Simplified CI/CD pipeline for better reliability
- Updated GitHub Actions MSRV check to Rust 1.77
- Revised Code of Conduct for improved clarity

### Fixed
- Downgraded `half` dependency to v2.4.1 to avoid yanked version
- Fixed all remaining clippy errors and warnings
- Fixed cargo-deny configuration for better compatibility
- Fixed CUDA test failures with proper panic handling
- Fixed AVX-512 type errors in SIMD batch operations
- Fixed platform-specific GPU module guards

### Notes
- All dependency updates maintain compatibility with existing code
- Test suite passes with 100/100 tests
- No breaking API changes in public interface
- Maze game accessible via `cargo run --release --features cli -- play`
- BCC-14 demo runs in 131ms for tree generation, 1ms for pathfinding

## [0.4.2] - 2025-10-16

### Added
- First crates.io release
- Perfect code quality (zero compiler warnings)
- Comprehensive documentation

### Changed
- Package optimized to 91 KB compressed size
- All tests passing (101/101)

### Fixed
- Fixed all clippy warnings
- Applied rustfmt to entire codebase

## [0.4.0] - 2025-10-15

### Added
- Major performance optimizations
- SIMD batch operations
- Parallel processing improvements

### Changed
- Morton decode optimization (37% speedup)
- Parallel overhead fix (86% speedup for 10K batches)

[Unreleased]: https://github.com/FunKite/OctaIndex3D/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.5.1
[0.5.0]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.5.0
[0.4.4]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.4
[0.4.3]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.3
[0.4.2]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.2
[0.4.0]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.0
