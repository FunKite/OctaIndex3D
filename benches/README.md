# OctaIndex3D Benchmarks

This directory contains Criterion benchmarks for measuring OctaIndex3D performance.

---

## ⚠️ Important Disclaimers

**These benchmarks were conducted with AI assistance (Claude by Anthropic) and should be considered preliminary.**

### Testing Limitations

1. **Limited Hardware Coverage:**
   - Intel testing: Conducted on **2 cores only** (AWS c7i.xlarge subset)
   - AMD testing: Conducted on **2 cores only** (AWS g4dn.xlarge equivalent subset)
   - Apple testing: Conducted on M1 Max Mac Studio (10-core chip, full access)

2. **Not Representative of Full System:**
   - Cloud instances tested (Intel, AMD) use only a subset of available cores
   - Multi-tenant cloud environments may have noisy neighbor effects
   - Results may vary on bare-metal systems with full chip access

3. **Preliminary Results:**
   - Benchmarks should be independently verified before making production decisions
   - Your actual performance may differ based on workload, data patterns, and hardware
   - These results are for comparative purposes and optimization guidance

4. **AI-Generated Analysis:**
   - Performance analysis conducted by Claude (Anthropic)
   - Recommendations should be validated by domain experts
   - May contain errors or oversimplifications

### Recommendations

- Run benchmarks on your target hardware before making deployment decisions
- Use these results as a starting point, not as definitive performance claims
- Consider your specific workload characteristics and data patterns
- Test with representative data and batch sizes for your use case

---

## Benchmark Files

### core_operations.rs
Baseline performance measurements for fundamental operations:
- Morton encoding/decoding
- Index64 creation and coordinate extraction
- Route64 validation and neighbor lookup
- Distance calculations (Manhattan, Euclidean)

### performance_optimizations.rs
Before/after comparisons showing optimization impact:
- Morton decode improvements (LUT optimization)
- Batch neighbor calculation improvements (parallel overhead fix)
- Index64 batch operation improvements

### simd_batch_optimizations.rs
SIMD-accelerated batch operation benchmarks:
- Index64 batch encode/decode (NEON, AVX2)
- Route64 validation (SIMD parity checks)
- Distance calculations (vectorized)

### tier1_optimizations.rs
Tier-1 platform optimization tests:
- Architecture-specific code paths (BMI2, AVX2, NEON)
- Cache-blocking strategies
- Parallel processing thresholds

---

## Running Benchmarks

### Quick Run

```bash
cargo bench --features parallel
```

### Specific Benchmark

```bash
cargo bench --bench core_operations --features parallel
cargo bench --bench performance_optimizations --features parallel
cargo bench --bench simd_batch_optimizations --features parallel
cargo bench --bench tier1_optimizations --features parallel
```

### With Native CPU Optimizations

For maximum performance, enable native CPU instructions:

```bash
RUSTFLAGS="-C target-cpu=native" cargo bench --features parallel
```

This enables:
- **Apple Silicon:** NEON instructions, Firestorm/Icestorm optimizations
- **AMD EPYC:** BMI2, AVX2, Zen 3 tuning
- **Intel Xeon:** BMI2, AVX2, Golden Cove tuning

---

## Profiling Harness

For detailed profiling beyond Criterion benchmarks:

```bash
cargo run --release --example profile_hotspots --features parallel
```

This runs a comprehensive profiling harness that measures:
- Morton encode/decode (100K coordinates × 100 iterations)
- Index64 batch operations (50K elements × 200 iterations)
- Neighbor calculations (small/medium/large batches)
- Distance calculations (50K pairs × 200 iterations)
- Bounding box queries

---

## Interpreting Results

### Throughput Metrics

Results are reported as operations per second (ops/sec) or routes per second (routes/sec):
- **Higher is better**
- Results show median performance (50th percentile)
- Criterion provides confidence intervals

### Performance Tiers

Based on preliminary testing:

**Morton Operations:**
- Excellent: >400M ops/sec
- Good: 200-400M ops/sec
- Acceptable: 100-200M ops/sec

**Batch Neighbor Calculations:**
- Excellent: >45M routes/sec
- Good: 30-45M routes/sec
- Acceptable: 15-30M routes/sec

**Index64 Batch Operations:**
- Excellent: >400M elem/sec
- Good: 200-400M elem/sec
- Acceptable: 100-200M elem/sec

### Architecture Differences

Different CPU architectures have different strengths:

**Apple Silicon (M1/M2/M3 series):**
- Strong: Batch operations, unified memory, consistency
- Uses: NEON SIMD, lookup tables for Morton operations

**AMD EPYC (Zen 3/4/5):**
- Strong: Single-threaded, BMI2 hardware, small batches
- Uses: BMI2 PDEP/PEXT, AVX2

**Intel Xeon (Sapphire Rapids and newer):**
- Strong: Large batches (big L3 cache), DDR5 bandwidth
- Uses: BMI2 PDEP/PEXT, AVX2 (AVX-512 potential)

---

## Benchmark Methodology

### Workload Sizes

Benchmarks use various workload sizes to test different optimization strategies:

- **Small:** 100 elements (prefetch-optimized)
- **Medium:** 1,000 elements (cache-blocked)
- **Large:** 10,000 elements (parallel/cache-aware)
- **Extra Large:** 100,000+ elements (stress test)

### Measurement Approach

- **Warmup:** All benchmarks include warmup iterations
- **Timing:** Wall-clock time using `std::time::Instant`
- **Compiler Optimization Prevention:** Volatile reads to prevent dead code elimination
- **Statistical Analysis:** Criterion performs outlier detection and regression analysis

### Build Configuration

All benchmarks should be run in release mode with optimizations:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

And with native CPU targeting:

```bash
RUSTFLAGS="-C target-cpu=native"
```

---

## Hardware Specifications

### Tested Platforms

**Apple M1 Max (Mac Studio):**
- Cores: 10 (8 performance + 2 efficiency)
- Cache: 192KB L1 per core, 24MB L2 shared, 48MB SLC
- Memory: Unified LPDDR5 (400 GB/s bandwidth)
- SIMD: ARM NEON (128-bit, always available)

**AMD EPYC 7R13 (Cloud Instance):**
- Tested Cores: 2 (from 48-core chip)
- Cache: 32KB L1, 512KB L2, 64MB L3 shared
- Memory: DDR4 (shared in cloud environment)
- SIMD: AVX2 (256-bit), BMI2

**Intel Xeon 8488C (Cloud Instance):**
- Tested Cores: 2 (from 48-core chip)
- Cache: 32KB L1, 2MB L2, 105MB L3 shared
- Memory: DDR5 (shared in cloud environment)
- SIMD: AVX2 (256-bit), BMI2, AVX-512 (not utilized yet)

---

## Future Work

### Optimization Opportunities

1. **AVX-512 Implementation (Intel):**
   - Current: AVX2 (256-bit)
   - Potential: AVX-512 (512-bit) on Sapphire Rapids
   - Expected: 1.5-2x speedup on batch operations

2. **AMD Large Batch Tuning:**
   - Current: 6.5M routes/sec (10K batches)
   - Target: 40M routes/sec
   - Approach: Better cache blocking, increased chunk sizes

3. **Apple NEON Further Optimization:**
   - Hand-tuned NEON assembly for Morton operations
   - Better 128-bit width utilization
   - Expected: 20-30% additional improvement

### Additional Testing Needed

- [ ] Bare-metal system testing (not cloud VMs)
- [ ] Full multi-core scaling tests
- [ ] ARM Graviton processors (AWS)
- [ ] AMD Zen 4/5 with AVX-512
- [ ] Real-world workload validation
- [ ] Memory-constrained scenarios
- [ ] Large dataset (>1GB) testing

---

## Contributing

If you run benchmarks on different hardware, please share results:

1. Run benchmarks with `cargo bench --features parallel`
2. Record system specifications (CPU model, core count, cache size, memory)
3. Note any cloud vs bare-metal differences
4. Share results via GitHub issue or pull request

**Include:**
- CPU model and core configuration
- Operating system and kernel version
- Rust version (`rustc --version`)
- Build flags used
- Raw Criterion output

---

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Intel Intrinsics Guide](https://www.intel.com/content/www/us/en/docs/intrinsics-guide/)
- [ARM NEON Intrinsics](https://developer.arm.com/architectures/instruction-sets/intrinsics/)

---

## License

Benchmarks are part of OctaIndex3D and licensed under the MIT License.

Copyright (c) 2025 Michael A. McLarney
