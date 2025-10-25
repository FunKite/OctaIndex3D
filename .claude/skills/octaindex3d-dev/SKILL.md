---
name: octaindex3d-dev
description: Helps develop, build, test, and optimize the OctaIndex3D Rust library - a high-performance BCC lattice spatial indexing system with SIMD acceleration. Use when building the project, running tests, benchmarking performance, managing examples, or preparing releases to crates.io.
---

# OctaIndex3D Development Skill

## Purpose

This skill provides project-specific knowledge and workflows for:
- Building and testing the Rust project with various feature combinations
- Running performance benchmarks and profiling
- Managing examples and demonstrations
- Handling version management and crate publishing
- Optimizing SIMD and performance-critical code

## Key Features

### Build & Test
- Standard Rust workflows: `cargo build`, `cargo test`, `cargo check`
- Feature combinations: `parallel`, `simd`, `hilbert`, `container_v2`, `serde`, `gis_geojson`
- Native compilation: `RUSTFLAGS="-C target-cpu=native"`
- Full feature test suite with 109+ tests

### Performance & Benchmarking
- Criterion benchmarks: `cargo bench`
- Profiling with `examples/profile_hotspots.rs`
- Hotspot analysis on Morton encoding/decoding, SIMD operations
- Performance targets tracked in CLAUDE.md

### Examples
- **BCC-14 Prim's Algorithm → A* Demo** (`bcc14_prim_astar_demo`)
  - Randomized Prim's algorithm on 549K BCC nodes
  - A* pathfinding with tree constraint
  - Dynamic seeding and reproducibility
  - Performance: 131ms build, 1ms solve

- **Game Examples**
  - `mothership_bridge`: Interactive space navigation (30s timeout)
  - `deep_space_explorer`: Large-scale exploration (90s timeout)
  - `interstellar_navigation`: Quick navigation demo (5s timeout)

### Version & Publishing
- Current version: v0.4.2 (published on crates.io)
- Git tags for releases
- Cargo.toml synchronization
- Minimal package size (91 KB compressed)

### Security & Dependencies
- Dependabot: Weekly auto-updates for Cargo & GitHub Actions
- cargo-deny: License and security scanning
- SECURITY.md: Vulnerability reporting policy
- All dependencies up-to-date (thiserror 2.0, petgraph 0.8, rkyv 0.8, dirs 6.0)

## Quick Commands

```bash
# Development build
cargo build

# Optimized release build
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run all tests
cargo test --all-features

# Run benchmarks
cargo bench --features parallel

# Run example with seed
cargo run --release --example bcc14_prim_astar_demo -- --seed=42

# Profile hotspots
cargo run --release --example profile_hotspots

# Security checks
cargo audit
cargo deny check

# Publish to crates.io (dry run first)
cargo publish --dry-run
cargo publish
```

## Project Context

**Repository:** https://github.com/FunKite/OctaIndex3D
**Crates.io:** https://crates.io/crates/octaindex3d
**Documentation:** https://docs.rs/octaindex3d

### Architecture Highlights
- **Morton Encoding/Decoding**: Specialized lookup tables for 3x speedup
- **SIMD Acceleration**: ARM NEON + x86 AVX2 support
- **BCC Lattice**: Body-centered cubic lattice with 14-neighbor connectivity
- **Parallel Operations**: Rayon-based parallelization with optimized thresholds
- **Performance**: 400M+ operations/sec on Apple M1 Max

### Key Files
- `src/morton.rs`: Core Morton code encode/decode (optimized with lookup tables)
- `src/performance/simd_batch.rs`: SIMD-accelerated batch operations
- `src/performance/fast_neighbors.rs`: Parallel neighbor calculations
- `examples/bcc14_prim_astar_demo.rs`: Showcase algorithm demonstration
- `benches/`: Criterion benchmarks
- `CLAUDE.md`: AI development log and notes
- `deny.toml`: Security and license configuration
- `SECURITY.md`: Vulnerability reporting policy

## Development Notes

### Recent Work (v0.4.2)
- Published to crates.io
- Zero compiler warnings in release build
- 109/109 tests passing
- Perfect code quality metrics
- Security infrastructure: Dependabot, cargo-deny, SECURITY.md
- All dependencies updated to latest major versions

### Known Optimizations
- Morton decode: 157M ops/sec (37% improvement)
- Parallel batches: 50M routes/sec (86% improvement)
- Tree-constrained A*: 1ms solve time

### Future Improvements
- AVX-512 implementation for Intel Xeon
- AMD large batch performance tuning
- NEON optimization for Apple Silicon

## Usage in Claude Code

Invoke this skill to:
1. **Build or test** - Build the project or run tests
2. **Optimize performance** - Profile or benchmark specific components
3. **Run examples** - Execute demos with specific seeds or parameters
4. **Manage releases** - Handle versioning and publishing to crates.io
5. **Debug issues** - Investigate performance regressions or failures
6. **Security checks** - Run vulnerability scans and dependency audits

Example prompts:
- "Run the BCC-14 demo with seed 42"
- "Build with all features and run benchmarks"
- "Profile the Morton decoding performance"
- "Check for compiler warnings in release build"
- "Prepare a new release for crates.io"
- "Run security audit and check for vulnerabilities"

---

**Last Updated:** 2025-10-25
**Status:** Production-ready, v0.4.2 published, security configured
