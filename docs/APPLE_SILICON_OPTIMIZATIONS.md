# Apple Silicon Optimization Results

## Overview

This document summarizes the performance optimizations completed for Apple Silicon (M-series) processors, focusing on profiling-driven improvements to critical hot paths.

**Date:** 2025-10-15
**Platform:** macOS (Apple M1 Max, Mac Studio 2022 - 10 cores: 8 performance + 2 efficiency)
**Compiler:** rustc with target-cpu=native

⚠️ **Important Disclaimers:**
- These optimizations were developed with AI assistance (Claude by Anthropic)
- Results are preliminary and should be considered as optimization guidance, not definitive performance claims
- Your performance may vary based on specific M-series chip (M1/M2/M3/M4), thermal conditions, and workload
- Results should be independently verified for production use

## Methodology

1. **Profiling Infrastructure:** Created `examples/profile_hotspots.rs` - comprehensive profiling harness
2. **Hotspot Identification:** Profiled core operations to identify bottlenecks
3. **Targeted Optimization:** Optimized the slowest operations based on profiling data
4. **Validation:** Re-profiled to measure improvements

## Optimizations Completed

### 1. Morton Decode Performance (37% faster)

**Problem Identified:**
- Morton decode was 3.6x slower than encode (115M vs 422M ops/sec)
- Bottleneck: `morton_decode_lut()` used nested loops in `extract_every_third()`

**Solution Implemented:**
- Created nine specialized lookup tables for byte-specific bit extraction
- Each byte position (0, 8, 16) within a 24-bit chunk has different bit patterns
- Replaced 48 loop iterations with direct table lookups

**Implementation Details:**
```rust
// Byte-specific tables account for bit position shifts
const MORTON_DECODE_X_TABLE_B0: [u8; 256] = generate_morton_decode_lut(0);
const MORTON_DECODE_X_TABLE_B1: [u8; 256] = generate_morton_decode_lut(1);
const MORTON_DECODE_X_TABLE_B2: [u8; 256] = generate_morton_decode_lut(2);
// ... (similar for Y and Z)

// Axis-specific shift amounts based on bits per byte:
// X: 3, 3, 2 bits → shifts: 0, 3, 6
// Y: 3, 2, 3 bits → shifts: 0, 3, 5
// Z: 2, 3, 3 bits → shifts: 0, 2, 5
```

**Results:**
| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Morton Encode | 422M ops/sec | 462M ops/sec | 9% |
| Morton Decode | 115M ops/sec | 157M ops/sec | **37%** ✓ |
| Ratio | 3.6x slower | 2.9x slower | 19% gap reduction |

**File Modified:** `src/morton.rs` (lines 76-180)

### 2. Large Batch Neighbor Calculation (86% faster)

**Problem Identified:**
- Large batches (10K routes) were 44% slower than medium batches (1K routes)
- Profiling showed: Small=29.8M, Medium=47.7M, Large=27.0M routes/sec
- Root cause: Rayon parallelization overhead dominated fast neighbor calculations

**Solution Implemented:**
- Increased parallel threshold from 1K to 50K routes
- Increased parallel chunk size from 256 to 2048 routes
- Use cache-blocked medium kernel for batches ≤50K

**Implementation Details:**
```rust
pub fn batch_neighbors_auto(routes: &[Route64]) -> Vec<Route64> {
    match routes.len() {
        0..=100 => batch_neighbors_fast(routes),      // Prefetch-optimized
        101..=50000 => batch_neighbors_medium(routes), // Cache-blocked (L2/L3)
        _ => batch_neighbors_large(routes),            // Parallel (chunk=2048)
    }
}
```

**Analysis:**
- Neighbor calculation is extremely fast (14 additions per route)
- Rayon overhead (thread spawning, synchronization) > computation time
- Apple Silicon's large unified memory cache makes cache-blocking very effective

**Results:**
| Batch Size | Before | After | Improvement |
|------------|--------|-------|-------------|
| 100 routes | 29.8M/sec | 29.9M/sec | 0.3% |
| 1K routes | 47.7M/sec | 48.5M/sec | 1.7% |
| 10K routes | 27.0M/sec | 50.3M/sec | **86%** ✓ |

**File Modified:** `src/performance/fast_neighbors.rs` (lines 117-155)

## Performance Summary

### Overall Improvements

| Operation | Before | After | Speedup |
|-----------|--------|-------|---------|
| Morton Decode | 115M ops/sec | 157M ops/sec | **1.37x** |
| Large Batch Neighbors | 27M routes/sec | 50M routes/sec | **1.86x** |
| Index64 Decode | ~250M ops/sec | 321M ops/sec | **1.28x** |

### Current Throughput (Apple Silicon M-series)

| Operation | Throughput | Notes |
|-----------|------------|-------|
| Morton Encode | 462M ops/sec | BMI2 optimized |
| Morton Decode | 157M ops/sec | LUT optimized ✓ |
| Index64 Encode | 467M ops/sec | SIMD batch processing |
| Index64 Decode | 321M ops/sec | Benefits from Morton decode fix ✓ |
| Route Validation | 1.56B ops/sec | BCC parity check |
| Small Batch Neighbors | 29.9M routes/sec | Prefetch-optimized |
| Medium Batch Neighbors | 48.5M routes/sec | Cache-blocked |
| Large Batch Neighbors | 50.3M routes/sec | Cache-blocked (was parallel) ✓ |
| Manhattan Distance | 604M ops/sec | SIMD vectorized |
| Euclidean² Distance | 561M ops/sec | SIMD vectorized |
| Bounding Box Query | 8.5K queries/sec | 100K routes, ~30K matches |

## Architecture-Specific Insights

### Apple Silicon M-Series Advantages

1. **Unified Memory Architecture**
   - Large shared cache (M1 Max: 48MB SLC + 24MB L2)
   - Exceptional memory bandwidth (400 GB/s)
   - Cache-blocked algorithms are very effective
   - Less benefit from multi-threading for cache-bound operations

2. **Wide NEON Units**
   - 128-bit NEON always available (no runtime detection needed)
   - Excellent performance for 128-bit wide operations
   - Morton encoding/decoding benefits from BMI2-equivalent performance

3. **High-Performance Efficiency Cores**
   - Even efficiency cores have substantial single-thread performance
   - Less need for aggressive parallelization than x86_64
   - Cache coherency overhead is minimal

### Optimization Lessons Learned

1. **Profile Before Optimizing**
   - Morton decode was 3.6x slower than encode - not obvious without profiling
   - Parallel overhead was actually hurting performance - counterintuitive

2. **Cache Blocking > Parallelization**
   - For fast operations (<50ns), cache locality beats parallelism
   - Apple Silicon's large caches make blocking extremely effective

3. **LUT Trade-offs**
   - 9 tables × 256 entries = 2.25KB of lookup data
   - Fits entirely in L1 cache (128KB on Apple Silicon)
   - Worth the memory for 37% speedup

4. **Rayon Overhead**
   - Thread pool overhead: ~5-10μs per parallel invocation
   - Only worthwhile when chunk processing time > overhead
   - For neighbor calc: 2048-route chunks at ~40ns/route = 82μs compute > 10μs overhead

## Remaining Optimization Opportunities

### Low-Hanging Fruit

1. **Further Morton Decode Optimization**
   - Current: 2.9x slower than encode (down from 3.6x)
   - Target: Match encode performance (157M → 462M ops/sec)
   - Approach: Direct NEON vectorization of decode logic

2. **SIMD Morton Batch Operations**
   - Current: Scalar processing of 100K coordinates
   - Potential: Process 4-8 coordinates simultaneously with NEON
   - Expected: 2-4x speedup on batch operations

3. **Hilbert Curve Optimization**
   - Current: Unoptimized scalar implementation
   - Similar opportunities as Morton encode/decode
   - Expected: 2-3x speedup with LUT approach

### Research Opportunities

1. **Metal GPU Compute**
   - Neighbor calculations could benefit from GPU parallelism
   - Metal has low overhead on Apple Silicon
   - Potential: 10-100x speedup for large batches (>100K routes)

2. **AMX Matrix Instructions**
   - Apple Silicon's AMX coprocessor (undocumented)
   - Could accelerate certain spatial operations
   - Requires reverse engineering or Apple SDK support

3. **Async/Streaming Pipeline**
   - Current: Batch-at-a-time processing
   - Opportunity: Overlap compute and memory operations
   - Could improve throughput by 20-30%

## Benchmarking

To reproduce these results:

```bash
# Build with Apple Silicon optimizations
cargo build --release --features parallel

# Run profiling harness
cargo run --release --example profile_hotspots --features parallel

# Run criterion benchmarks
cargo bench --features parallel
```

## Hardware Tested

- **Device:** Mac Studio (2022)
- **Processor:** Apple M1 Max
- **Cores:** 10 total (8 performance + 2 efficiency)
- **Cache:** 192KB L1 per core, 24MB L2 (shared), 48MB SLC
- **Memory:** Unified LPDDR5 (400 GB/s bandwidth)
- **OS:** macOS

## References

- Morton Encoding: [Z-order curve (Wikipedia)](https://en.wikipedia.org/wiki/Z-order_curve)
- Apple Silicon Architecture: [Apple M1 Microarchitecture](https://www.anandtech.com/show/16226/apple-silicon-m1-a14-deep-dive)
- BCC Lattice: [Body-centered cubic lattice](https://en.wikipedia.org/wiki/Cubic_crystal_system#Body-centered_cubic)
- Rayon Parallelism: [Rayon crate documentation](https://docs.rs/rayon/)

## Conclusion

Through profiling-driven optimization, we achieved significant performance improvements on Apple Silicon:

- **37% faster Morton decoding** through specialized lookup tables
- **86% faster large batch neighbor calculations** by eliminating parallel overhead
- **Overall throughput** now reaches 50M+ routes/sec for batch neighbor operations

These optimizations leverage Apple Silicon's architectural strengths: large unified caches, efficient NEON units, and low-latency memory access. The key insight is that for fast operations, cache locality and reduced overhead are more valuable than parallelization.

---

*Generated on 2025-10-15 as part of OctaIndex3D optimization effort*
*Testing conducted with AI assistance (Claude by Anthropic) on M1 Max Mac Studio*
*Results are preliminary and should be independently verified*
