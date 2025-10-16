# Release Announcement: OctaIndex3D v0.4.0

**Release Date:** 2025-10-15
**Version:** 0.4.0
**Repository:** https://github.com/FunKite/OctaIndex3D

---

## Overview

OctaIndex3D v0.4.0 delivers major performance improvements and comprehensive cross-platform optimizations. This release focuses on profiling-driven enhancements that significantly improve throughput for core spatial operations.

---

## 🚀 Major Performance Improvements

### Morton Decode Optimization (37% faster)
- **Before:** 115M ops/sec
- **After:** 157M ops/sec
- **Improvement:** 37% speedup

**Technical Details:**
- Identified bottleneck in `morton_decode_lut()` using nested loops
- Created 9 specialized lookup tables (3 bytes × 3 axes) with axis-specific bit shifts
- Reduced decode overhead from 48 loop iterations to direct table lookups

### Batch Neighbor Calculation (86% faster)
- **Before:** 27M routes/sec (10K batches)
- **After:** 50M routes/sec (10K batches)
- **Improvement:** 86% speedup

**Technical Details:**
- Fixed parallel overhead issue where thread spawning cost exceeded computation time
- Raised parallel threshold from 1K to 50K routes
- Increased chunk size from 256 to 2048 routes
- Implemented cache-blocked kernel for batches ≤50K

### Index64 Decode Operations (28% faster)
- **Before:** ~250M ops/sec
- **After:** 321M ops/sec
- **Improvement:** 28% speedup
- Benefits from Morton decode optimizations

---

## 🏗️ New Performance Module

Added comprehensive performance optimization infrastructure:

- **Architecture-Optimized Code:**
  - BMI2 hardware acceleration (x86_64 Intel/AMD)
  - AVX2 SIMD operations (x86_64)
  - ARM NEON acceleration (Apple Silicon)

- **Adaptive Batch Sizing:**
  - Small batches (<100): Prefetch-optimized
  - Medium batches (100-50K): Cache-blocked
  - Large batches (>50K): Parallel processing with Rayon

- **Smart Threshold Detection:**
  - Automatically selects optimal processing strategy
  - Avoids parallel overhead for fast operations

---

## 📊 Cross-Platform Testing

Performance has been validated across three major CPU architectures:

### Apple Silicon (M1 Max - Mac Studio)
- **Morton Decode:** 157M ops/sec
- **Batch Neighbors:** 50M routes/sec (10K batch)
- **Index64 Encode:** 467M elem/sec
- **Strengths:** Best consistency, batch operations, unified memory architecture

### AMD EPYC 7R13 (Zen 3)
- **Morton Decode:** 505M ops/sec (BMI2 hardware)
- **Batch Neighbors (small):** 32.1M routes/sec
- **Strengths:** Best single-threaded performance, Morton operations

### Intel Xeon 8488C (Sapphire Rapids)
- **Morton Decode:** 424M ops/sec
- **Batch Neighbors (large):** 37.8M routes/sec
- **Strengths:** Best for large batches (105MB L3 cache)

### GPU Analysis
- Testing on NVIDIA L4 showed CPU is **10x faster** than GPU
- GPU acceleration not recommended due to PCIe transfer overhead

---

## 📚 New Documentation

This release includes comprehensive documentation for performance:

- **CPU_COMPARISON.md** - Detailed 3-way CPU comparison with architecture insights
- **GPU_ACCELERATION.md** - Analysis of why CPU outperforms GPU
- **APPLE_SILICON_OPTIMIZATIONS.md** - Apple Silicon specific optimizations
- **PERFORMANCE.md** - Updated usage examples and batch size recommendations

---

## 🔧 Feature Flags

### New Features in v0.4.0

- **`parallel`** - Multi-threaded batch operations with Rayon (recommended)
- **`simd`** - SIMD-accelerated operations (BMI2, AVX2, NEON)

### Existing Features

- **`hilbert`** - Hilbert64 space-filling curve with better locality than Morton
- **`container_v2`** - Append-friendly streaming container format
- **`gis_geojson`** - GeoJSON export with WGS84 coordinate conversion
- **`zstd_compression`** - Zstd compression (in addition to default LZ4)

---

## 📦 Installation

### From Crates.io

```toml
[dependencies]
octaindex3d = "0.4"

# With recommended features
octaindex3d = { version = "0.4", features = ["parallel", "simd", "hilbert"] }
```

### From Source

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd octaindex3d
RUSTFLAGS="-C target-cpu=native" cargo build --release --features parallel
```

---

## 🔬 Benchmarking Infrastructure

This release includes comprehensive benchmarking tools:

- **benches/core_operations.rs** - Baseline performance measurements
- **benches/performance_optimizations.rs** - Before/after comparisons
- **benches/simd_batch_optimizations.rs** - SIMD operation benchmarks
- **benches/tier1_optimizations.rs** - Tier-1 optimization tests
- **examples/profile_hotspots.rs** - Comprehensive profiling harness

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --features parallel

# Profile hotspots
cargo run --release --example profile_hotspots --features parallel
```

---

## 🎯 Performance Recommendations

### Compiler Flags

For maximum performance, build with native CPU optimizations:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

This enables:
- Apple Silicon: NEON instructions, Firestorm/Icestorm optimizations
- AMD EPYC: BMI2, AVX2, Zen 3 tuning
- Intel Xeon: BMI2, AVX2, Golden Cove tuning

### Batch Size Selection

- **Small batches (<100 elements):** Use default API, optimized for prefetching
- **Medium batches (100-50K):** Use batch operations, cache-blocked processing
- **Large batches (>50K):** Parallel processing automatically engaged

---

## 🐛 Issues Fixed

1. **Route Validation Test Failure**
   - Fixed BCC lattice parity requirement in tests
   - Ensured all test routes use valid coordinates: `(x + y + z) % 2 == 0`

2. **Morton Decode Lookup Table Bug**
   - Fixed incorrect decode results
   - Created byte-specific lookup tables with axis-specific shifts

3. **CUDA Backend Compilation**
   - Simplified cudarc API type handling
   - Note: GPU acceleration not recommended (see GPU_ACCELERATION.md)

4. **Apple Hardware Misidentification**
   - Corrected documentation to reflect M1 Max Mac Studio
   - Updated cache and core count specifications

---

## 🔄 Migration from v0.3.x

v0.4.0 is fully backward compatible with v0.3.x. No API changes required.

**Performance Improvements Are Automatic:**
- Simply rebuild with v0.4.0 to get performance benefits
- Recommended: Add `parallel` and `simd` features for best performance

---

## 🙏 Acknowledgments

This release was developed with extensive AI assistance:
- Performance profiling and optimization by Claude (Anthropic)
- Architecture-specific testing across Apple Silicon, AMD EPYC, and Intel Xeon
- Comprehensive documentation and benchmarking infrastructure

---

## 📝 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright (c) 2025 Michael A. McLarney

---

## 🔗 Links

- **Repository:** https://github.com/FunKite/OctaIndex3D
- **Documentation:** https://docs.rs/octaindex3d
- **Crates.io:** https://crates.io/crates/octaindex3d
- **Whitepaper:** [WHITEPAPER.md](WHITEPAPER.md)
- **Issues:** https://github.com/FunKite/OctaIndex3D/issues

---

**Next Steps:**

Consider these future optimization opportunities:
1. AVX-512 implementation for Intel Xeon (potential 2x speedup)
2. Improved AMD EPYC large batch performance
3. Further NEON optimizations for Apple Silicon

For detailed technical analysis, see:
- [CPU_COMPARISON.md](docs/CPU_COMPARISON.md)
- [APPLE_SILICON_OPTIMIZATIONS.md](docs/APPLE_SILICON_OPTIMIZATIONS.md)
- [PERFORMANCE.md](PERFORMANCE.md)
