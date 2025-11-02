# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/FunKite/OctaIndex3D/compare/v0.4.3...HEAD
[0.4.3]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.3
[0.4.2]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.2
[0.4.0]: https://github.com/FunKite/OctaIndex3D/releases/tag/v0.4.0
