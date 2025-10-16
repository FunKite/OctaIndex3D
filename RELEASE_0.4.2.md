# Release Notes: OctaIndex3D v0.4.2

**Release Date:** 2025-10-16
**Version:** 0.4.2 (First Published Crate Release)
**Repository:** https://github.com/FunKite/OctaIndex3D
**Crates.io:** https://crates.io/crates/octaindex3d

---

## 🎉 First Crate Release on crates.io

This is the **first published release** of OctaIndex3D on crates.io!

Version 0.4.2 includes all the major performance improvements from v0.4.0/0.4.1 development, plus additional code quality improvements for a production-ready release.

---

## 🆕 What's New in v0.4.2

### Code Quality Improvements
- ✅ **Zero compiler warnings** - All 10 compiler warnings resolved
- ✅ **Clean build** - 100% warning-free compilation
- ✅ **Minimal package** - Lean crate with only essential documentation (91 KB)
- ✅ **All tests passing** - 101/101 tests pass (100% pass rate)

---

## 🚀 Major Performance Improvements (from v0.4.0)

### Morton Decode Optimization (37% faster)
- **Before:** 115M ops/sec
- **After:** 157M ops/sec
- **Improvement:** 37% speedup
- **Details:** Replaced nested loops with specialized lookup tables (9 tables: 3 bytes × 3 axes)

### Batch Neighbor Calculation (86% faster)
- **Before:** 27M routes/sec (10K batches)
- **After:** 50M routes/sec (10K batches)
- **Improvement:** 86% speedup
- **Details:** Fixed parallel overhead, raised threshold from 1K to 50K routes, increased chunk size

### Index64 Decode Operations (28% faster)
- **Before:** ~250M ops/sec
- **After:** 321M ops/sec
- **Improvement:** 28% speedup

---

## 📦 Installation

### From Crates.io

```toml
[dependencies]
octaindex3d = "0.4"

# With recommended features (default includes parallel + simd)
octaindex3d = { version = "0.4", default-features = true }

# With optional features
octaindex3d = { version = "0.4", features = ["hilbert", "container_v2", "gis_geojson"] }
```

### From Source

```bash
git clone https://github.com/FunKite/OctaIndex3D
cd octaindex3d
RUSTFLAGS="-C target-cpu=native" cargo build --release
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

---

## 🔧 Feature Flags

### Performance Features (Enabled by Default)
- **`parallel`** - Multi-threaded batch operations with Rayon
- **`simd`** - SIMD-accelerated operations (BMI2, AVX2, NEON)
- **`serde`** - Serialization support

### Optional Features
- **`hilbert`** - Hilbert64 space-filling curve (better locality than Morton)
- **`container_v2`** - Streaming container format with checkpoints
- **`gis_geojson`** - GeoJSON export with WGS84 coordinates
- **`zstd`** - Zstd compression (in addition to LZ4)

### GPU Features (Not Recommended)
- **`gpu-metal`**, **`gpu-vulkan`**, **`gpu-cuda`** - CPU is 10x faster

---

## 📊 Cross-Platform Performance

### Apple Silicon (M1 Max)
- **Morton Decode:** 157M ops/sec
- **Batch Neighbors:** 50M routes/sec (10K batch)
- **Index64 Encode:** 467M elem/sec

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

---

## 🐛 Fixes in v0.4.2

1. **All Compiler Warnings Resolved** (10 warnings fixed)
   - Fixed unused variables in conditional compilation blocks
   - Removed unnecessary `unsafe` block
   - Fixed unreachable code with proper cfg guards
   - Added appropriate `#[allow]` attributes for dead code

2. **Version Consistency**
   - Updated all version references to 0.4.2
   - Fixed version test in test suite

3. **Package Cleanup**
   - Minimal documentation in crate (91 KB compressed)
   - Detailed docs remain on GitHub for deep dives

---

## 🔄 Migration from v0.3.x

v0.4.2 is fully backward compatible with v0.3.x. No API changes required.

Simply update your Cargo.toml:

```toml
[dependencies]
octaindex3d = "0.4"
```

All performance improvements are automatic!

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

## 🙏 Acknowledgments

This release was developed with extensive AI assistance:
- Performance profiling and optimization by Claude (Anthropic)
- Cross-platform testing on Apple Silicon, AMD EPYC, and Intel Xeon
- Comprehensive documentation and benchmarking infrastructure

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

1. **AVX-512 Implementation** - Potential 2x speedup for Intel Xeon
2. **Improved AMD Large Batch Performance** - Target 40M routes/sec
3. **Advanced NEON Optimizations** - Further Apple Silicon improvements

---

**Thank you for using OctaIndex3D!**

For detailed technical information, visit the [GitHub repository](https://github.com/FunKite/OctaIndex3D).
