# Appendix G: Performance Tuning Cookbook

This appendix is a **quick-reference decision guide** for making OctaIndex3D fast on your hardware. Instead of re-deriving theory, it focuses on "if you see X, try Y" style recipes, decision trees, and actionable checklists.

Use this cookbook when you need immediate performance improvements. For detailed explanations, see Chapter 7 (Performance Optimization) and Appendix C (Performance Benchmarks).

---

## G.1 Quick Performance Diagnosis

### G.1.1 Performance Problem Decision Tree

```text
Is your application slow?
│
├─> CPU-bound? (high CPU%, low memory bandwidth)
│   ├─> Encoding/decoding hot? → See G.2 (CPU Features)
│   ├─> Neighbor queries hot? → See G.4 (Algorithmic Tuning)
│   └─> Serialization hot? → See G.5 (Container Selection)
│
├─> Memory-bound? (high memory bandwidth, cache misses)
│   ├─> Large datasets? → See G.3 (Memory Optimization)
│   ├─> Random access patterns? → See G.4.3 (Spatial Locality)
│   └─> Frequent allocations? → See G.6 (Allocation Tuning)
│
└─> I/O-bound? (waiting on disk/network)
    ├─> Reading containers? → See G.5 (Streaming vs Sequential)
    ├─> Network latency? → See G.7 (Distribution Tuning)
    └─> Disk throughput? → See G.5.4 (Compression)
```

### G.1.2 Quick Profiling Commands

**Linux (perf):**
```bash
# CPU profiling
perf record -g --call-graph dwarf ./your_app
perf report

# Cache analysis
perf stat -e cache-references,cache-misses,L1-dcache-load-misses ./your_app

# Memory bandwidth
perf stat -e cycles,instructions,mem-loads,mem-stores ./your_app
```text

**macOS (Instruments):**
```bash
# Time profiler
xcrun xctrace record --template 'Time Profiler' --launch ./your_app

# Allocations
xcrun xctrace record --template 'Allocations' --launch ./your_app
```

**Cross-platform (Criterion benchmarks):**
```bash
cargo bench --bench your_benchmark -- --profile-time=5
```toml

---

## G.2 CPU Feature Selection

### G.2.1 Feature Flag Decision Matrix

| Platform | Feature Flags | When to Use | Performance Gain |
|----------|---------------|-------------|------------------|
| **x86-64 (Modern)** | `bmi2`, `avx2` | Intel Haswell+ (2013+), AMD Ryzen | **5-7× faster** encoding |
| **x86-64 (Older)** | `sse2`, `sse4.1` | Pre-Haswell Intel, older AMD | 2-3× faster |
| **x86-64 (Baseline)** | None | Maximum portability | Baseline (slowest) |
| **ARM (Modern)** | `neon` | ARM Cortex-A series, Apple Silicon | **3-5× faster** encoding |
| **ARM (Baseline)** | None | ARMv7 and older | Baseline |
| **RISC-V** | None | Not yet optimized | Baseline |

### G.2.2 Enabling CPU Features

**Cargo.toml (recommended for uniform deployment):**
```toml
[profile.release]
# For x86-64 Haswell+
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=haswell"]

# For Apple Silicon (M1/M2/M3)
[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=apple-m1"]
```

**Runtime CPU feature detection (recommended for distributed binaries):**
```rust
use octaindex3d::morton;

// Library automatically selects best implementation at runtime
let encoded = morton::encode(x, y, z); // Uses BMI2 if available, falls back otherwise
```toml

**Feature flags (for fine-grained control):**
```toml
# Cargo.toml
[dependencies]
octaindex3d = { version = "0.1", features = ["simd", "bmi2"] }
```

### G.2.3 When NOT to Enable Features

**Avoid CPU-specific features if:**
- Deploying to heterogeneous fleets (AWS Lambda, cloud functions)
- Building portable binaries for distribution
- Targeting older hardware (pre-2013 CPUs)

**Solution:** Use runtime feature detection instead:
```rust
#[cfg(target_feature = "bmi2")]
fn encode_fast(x: i32, y: i32, z: i32) -> u64 {
    // BMI2 implementation
}

#[cfg(not(target_feature = "bmi2"))]
fn encode_fast(x: i32, y: i32, z: i32) -> u64 {
    // Fallback implementation
}
```rust

---

## G.3 Memory Optimization

### G.3.1 Memory Usage Checklist

**Problem: Excessive memory usage**

Checklist:
- [ ] Are you using the right LOD? (Higher LOD = exponentially more points)
- [ ] Are you storing unnecessary metadata? (Consider sparse containers)
- [ ] Are you duplicating data? (Use references instead of clones)
- [ ] Are you using streaming containers when sequential would suffice?
- [ ] Have you enabled compression? (See G.5.4)

### G.3.2 Container Memory Footprint

| Container Type | Memory Overhead | When to Use |
|----------------|-----------------|-------------|
| **InMemory** | ~8-16 bytes/point | Small datasets (<1M points), random access |
| **Sequential** | ~4-8 bytes/point | Large datasets (1M-100M points), sequential access |
| **Streaming** | ~16-32 bytes/point | Unbounded streams, real-time ingestion |
| **Compressed** | ~1-4 bytes/point | Archival, read-heavy workloads |

**Example: Switching from InMemory to Sequential**

```rust
// ❌ High memory usage
let mut container = InMemoryContainer::new();
for i in 0..10_000_000 {
    container.insert(index, data); // ~160 MB for 10M points
}

// ✅ Lower memory usage
let writer = SequentialContainerWriter::new("data.bcc")?;
for i in 0..10_000_000 {
    writer.write(index, data)?; // ~40 MB peak memory
}
writer.finalize()?;
```

### G.3.3 LOD Selection Guide

**Picking the right Level of Detail:**

| Use Case | Recommended LOD | Spacing (meters)* | Points/km³ |
|----------|-----------------|-------------------|------------|
| **Global terrain** | 10-12 | 1-4 m | 15K - 1M |
| **City-scale** | 12-14 | 0.25-1 m | 1M - 64M |
| **Building interior** | 14-16 | 0.06-0.25 m | 64M - 4B |
| **Robotics (close range)** | 16-18 | 0.015-0.06 m | 4B - 260B |
| **Microscopy** | 20-24 | <0.001 m | >1T |

*At LOD L, spacing ≈ 2^(-L) in canonical units

**Rule of thumb:** Each LOD increment doubles resolution and increases point count by ~8×.

### G.3.4 Reducing Allocations

**Problem: High allocation rate causing GC pressure (when using bindings) or fragmentation**

**Solution 1: Object pooling**
```rust
use std::sync::Arc;
use parking_lot::Mutex;

struct BufferPool {
    pool: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl BufferPool {
    fn acquire(&self, size: usize) -> Vec<u8> {
        self.pool.lock()
            .pop()
            .filter(|b| b.capacity() >= size)
            .unwrap_or_else(|| Vec::with_capacity(size))
    }

    fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        self.pool.lock().push(buffer);
    }
}
```rust

**Solution 2: Pre-allocate with capacity**
```rust
// ❌ Many reallocations
let mut results = Vec::new();
for id in index.range_query(center, radius) {
    results.push(id); // Reallocates ~log(n) times
}

// ✅ Single allocation
let estimated_size = (radius.powi(3) * density) as usize;
let mut results = Vec::with_capacity(estimated_size);
for id in index.range_query(center, radius) {
    results.push(id); // No reallocation if estimate is good
}
```

---

## G.4 Algorithmic Tuning

### G.4.1 Neighbor Query Optimization

**Problem: Slow neighbor finding**

| Symptom | Solution | Expected Speedup |
|---------|----------|------------------|
| Calling `neighbors()` in tight loop | Batch queries, cache results | 2-5× |
| Querying multiple LODs | Use hierarchical traversal | 3-10× |
| Repeated queries on same region | Build temporary spatial index | 10-100× |

**Example: Batch neighbor queries**
```rust
// ❌ Slow - repeated hashmap lookups
for id in ids.iter() {
    for neighbor in id.get_14_neighbors() {
        if container.contains(neighbor) {
            // Process
        }
    }
}

// ✅ Fast - batch lookup
let all_candidates: HashSet<Index64> = ids.iter()
    .flat_map(|id| id.get_14_neighbors())
    .collect();
let existing: Vec<_> = container.batch_lookup(&all_candidates);
```rust

### G.4.2 Range Query Optimization

**Problem: Slow range queries**

**Solution: Use hierarchical queries**
```rust
// ❌ Slow - check every point
fn slow_range_query(container: &Container, center: Point3D, radius: f64) -> Vec<Index64> {
    container.all_points()
        .filter(|p| distance(p, center) <= radius)
        .collect()
}

// ✅ Fast - hierarchical culling
fn fast_range_query(container: &Container, center: Index64, radius_cells: i32) -> Vec<Index64> {
    // Use BCC's hierarchical structure to cull entire subtrees
    container.hierarchical_range_query(center, radius_cells)
}
```

### G.4.3 Improving Spatial Locality

**Problem: Random access pattern causing cache misses**

**Solution: Sort by Morton/Hilbert order**
```rust
use octaindex3d::hilbert;

// ❌ Bad locality - random order
let mut points: Vec<(Index64, Data)> = load_points();
for (id, data) in points.iter() {
    process(id, data); // Thrashes cache
}

// ✅ Good locality - spatially sorted
points.sort_by_key(|(id, _)| hilbert::encode(*id));
for (id, data) in points.iter() {
    process(id, data); // Sequential access, cache-friendly
}
```python

**Benchmark results (from Appendix C):**
- Unsorted: 250 MB/s throughput, 45% cache miss rate
- Hilbert-sorted: 850 MB/s throughput, 12% cache miss rate (**3.4× speedup**)

---

## G.5 Container Format Selection

### G.5.1 Container Decision Tree

```text
What's your access pattern?
│
├─> Mostly sequential reads? → SequentialContainer
│   ├─> Need compression? → SequentialContainer + LZ4/Zstd
│   └─> Need fast seeking? → Use indexed blocks (see Chapter 8)
│
├─> Random access during queries? → InMemoryContainer
│   ├─> Dataset fits in RAM? → Load entire container
│   └─> Dataset too large? → Memory-map SequentialContainer
│
├─> Streaming writes (real-time)? → StreamingContainer
│   ├─> Need durability? → Enable WAL (write-ahead log)
│   └─> Low latency critical? → Use ring buffer mode
│
└─> Mixed read/write? → InMemoryContainer + periodic flush to Sequential
```rust

### G.5.2 Container Performance Comparison

| Operation | InMemory | Sequential | Streaming |
|-----------|----------|------------|-----------|
| **Random read** | 50-100 ns | 500-2000 ns (uncached) | Not supported |
| **Sequential read** | 100-200 ns | 80-150 ns | N/A |
| **Insert** | 100-300 ns | N/A | 200-500 ns |
| **Batch write** | N/A | 50-80 ns/point | 100-200 ns/point |
| **Memory usage** | High (16 bytes/pt) | Low (mmap) | Medium (8 bytes/pt) |

(See Appendix C for detailed benchmarks)

### G.5.3 Streaming vs Sequential

**When to use StreamingContainer:**
- Real-time data ingestion (robotics sensors, live simulations)
- Unbounded data streams
- Low-latency writes required

**When to use SequentialContainer:**
- Batch processing
- Archival storage
- Read-heavy workloads

**Example: Converting streaming to sequential**
```rust
use octaindex3d::{StreamingContainer, SequentialContainerWriter};

// During data collection: use streaming
let mut stream = StreamingContainer::new("live_data.bcc.stream")?;
for sensor_reading in sensor.iter() {
    stream.append(sensor_reading)?;
}
stream.close()?;

// After collection: convert to sequential for fast queries
let reader = StreamingContainerReader::open("live_data.bcc.stream")?;
let mut writer = SequentialContainerWriter::new("archive_data.bcc")?;
for entry in reader.iter() {
    writer.write(entry)?;
}
writer.finalize()?; // Creates optimized sequential format
```

### G.5.4 Compression Settings

**Compression Trade-offs:**

| Codec | Compression Ratio | Encode Speed | Decode Speed | Use Case |
|-------|-------------------|--------------|--------------|----------|
| **None** | 1.0× | N/A | N/A | Fast local storage, pre-compressed data |
| **LZ4** | 2-3× | ~500 MB/s | ~2000 MB/s | General-purpose, good balance |
| **Zstd (level 3)** | 3-5× | ~200 MB/s | ~600 MB/s | Better compression, still fast |
| **Zstd (level 9)** | 4-7× | ~40 MB/s | ~600 MB/s | Archival, read-heavy |

**Recommendation:**
- **Default:** LZ4 (fast enough for most use cases)
- **Network transfer:** Zstd level 3 (reduce bandwidth)
- **Long-term storage:** Zstd level 9 (maximize compression)

**Example:**
```rust
use octaindex3d::container::{SequentialContainerWriter, CompressionCodec};

let writer = SequentialContainerWriter::builder()
    .path("data.bcc")
    .compression(CompressionCodec::Lz4)
    .build()?;
```toml

---

## G.6 Platform-Specific Tuning

### G.6.1 x86-64 (Intel/AMD)

**Optimal settings for modern x86-64 (Haswell and newer):**

```toml
# .cargo/config.toml
[build]
rustflags = ["-C", "target-cpu=haswell", "-C", "opt-level=3"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-feature=+bmi2,+avx2"]
```

**Profiling tools:**
- **perf:** Hardware counter access, cache profiling
- **Intel VTune:** Advanced CPU profiling (requires Intel CPU)
- **AMD μProf:** Similar to VTune for AMD CPUs

**Common bottlenecks:**
- Morton encoding without BMI2: Enable `bmi2` feature (**5-7× speedup**)
- Cache misses: Sort data spatially (see G.4.3)
- Branch mispredictions: Use branchless code for hot paths

### G.6.2 ARM (Apple Silicon, Raspberry Pi, Mobile)

**Optimal settings for ARM:**

```toml
# .cargo/config.toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=apple-m1", "-C", "opt-level=3"]

[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "target-feature=+neon"]
```rust

**Apple Silicon specific:**
- Use Instruments for profiling (Time Profiler, Allocations)
- Leverage unified memory architecture (CPU-GPU shared memory)
- Consider Metal acceleration for large-scale queries (see G.6.4)

**ARM Cortex-A (Raspberry Pi, embedded):**
- Memory bandwidth limited: prioritize compression
- Lower power budget: reduce LOD, use coarser grids
- Limited SIMD: NEON helps but gains are smaller than AVX2 on x86

### G.6.3 RISC-V and Other Architectures

**Current status:** No architecture-specific optimizations yet.

**Recommended approach:**
- Use baseline (portable) implementations
- Profile on target hardware
- Contribute RISC-V optimizations upstream (see Chapter 16)

### G.6.4 GPU Acceleration

**When to use GPU acceleration:**
- Batch processing >100K points
- Embarrassingly parallel queries (many independent range queries)
- Already have data on GPU (rendering, ML training)

**Platforms:**

| Platform | API | Use Case |
|----------|-----|----------|
| **NVIDIA** | CUDA | Scientific computing, ML training |
| **AMD** | ROCm/Vulkan | Scientific computing (Linux), cross-platform |
| **Apple** | Metal | Apple Silicon (M1/M2/M3), iOS |
| **Cross-platform** | Vulkan Compute | Widest compatibility |

**Example: Metal acceleration (macOS/iOS)**
```rust
use octaindex3d::gpu::metal;

// Batch encode on GPU
let coords: Vec<(i32, i32, i32)> = /* ... */;
let device = metal::Device::default();
let encoded = metal::batch_encode(&device, &coords)?; // 10-50× faster for large batches
```

---

## G.7 Distributed Systems Tuning

### G.7.1 Sharding Strategies

**Problem: Dataset too large for single node**

**Spatial sharding:**
```rust
// Shard by top-level region (e.g., top 6 bits of Morton code)
fn shard_id(index: Index64) -> u8 {
    (index.morton_code() >> 58) as u8 // Top 6 bits = 64 shards
}
```rust

**Benefits:**
- Spatially close points on same shard
- Range queries often hit single shard
- Good load balancing for uniform distributions

**Trade-offs:**
- Non-uniform data causes hot shards
- Cross-shard queries require fan-out

**Alternative: Hash sharding (for non-spatial queries)**
```rust
fn shard_id(index: Index64) -> u8 {
    (index.as_u64() % NUM_SHARDS) as u8
}
```

### G.7.2 Network Optimization

**Problem: High latency for distributed queries**

**Solutions:**
1. **Batch requests:** Amortize network RTT over multiple queries
2. **Compression:** Use Zstd for wire protocol (3-5× less bandwidth)
3. **Caching:** Cache recent query results at coordinator node
4. **Speculative execution:** Query multiple shards in parallel

**Example: Batched queries**
```rust
// ❌ Slow - N round trips
for id in ids.iter() {
    let data = remote_shard.get(id).await?; // One network call per query
}

// ✅ Fast - 1 round trip
let all_data = remote_shard.batch_get(&ids).await?; // Single network call
```rust

---

## G.8 Common Performance Anti-Patterns

### G.8.1 Inefficient Patterns to Avoid

**Anti-pattern 1: Repeated parity checks**
```rust
// ❌ Slow - checks parity every time
for _ in 0..1_000_000 {
    if (x + y + z) % 2 == 0 {
        let coord = BccCoord::new(x, y, z)?; // Checks parity again!
    }
}

// ✅ Fast - check once, reuse
if (x + y + z) % 2 == 0 {
    let coord = BccCoord::new_unchecked(x, y, z); // Skip redundant check
    for _ in 0..1_000_000 {
        process(coord);
    }
}
```

**Anti-pattern 2: Unnecessary conversions**
```rust
// ❌ Slow - convert back and forth
let index = Index64::from_bcc_coord(coord, lod);
let coord2 = index.to_bcc_coord(); // Unnecessary round-trip
process(coord2);

// ✅ Fast - keep in most useful form
process(coord); // Work with BccCoord directly
```rust

**Anti-pattern 3: Small allocations in hot loops**
```rust
// ❌ Slow - allocates on every iteration
for id in ids.iter() {
    let neighbors = id.get_14_neighbors(); // Returns Vec (allocates)
    for n in neighbors {
        process(n);
    }
}

// ✅ Fast - reuse allocation
let mut neighbors = Vec::with_capacity(14);
for id in ids.iter() {
    id.get_14_neighbors_into(&mut neighbors); // Reuses buffer
    for &n in &neighbors {
        process(n);
    }
    neighbors.clear();
}
```

### G.8.2 Premature Optimization

**Don't optimize without profiling!**

Common mistakes:
- Optimizing cold code (not in hot path)
- Micro-optimizing when algorithm is wrong
- Sacrificing readability for negligible gains

**Profiling-driven optimization workflow:**
1. **Measure:** Profile to find hot spots
2. **Hypothesize:** Identify bottleneck (CPU, memory, I/O)
3. **Optimize:** Apply targeted fix
4. **Verify:** Re-profile to confirm improvement
5. **Repeat:** Move to next bottleneck

**Example:**
```bash
# Step 1: Find hot spots
perf record -g ./your_app
perf report
# Output shows 80% time in morton::encode()

# Step 2: Enable BMI2
export RUSTFLAGS="-C target-feature=+bmi2"
cargo build --release

# Step 3: Verify
perf record -g ./your_app
perf report
# Output now shows morton::encode() is 10% (8× speedup)
```rust

---

## G.9 Performance Tuning Checklist

Use this checklist for systematic performance optimization:

### G.9.1 Initial Profiling
- [ ] Run profiler on representative workload
- [ ] Identify hot functions (>10% total time)
- [ ] Measure baseline performance (ops/sec, latency, memory)
- [ ] Identify bottleneck type (CPU, memory, I/O)

### G.9.2 Low-Hanging Fruit
- [ ] Enable CPU features (BMI2/AVX2 on x86, NEON on ARM)
- [ ] Choose appropriate container format (see G.5.1)
- [ ] Sort data spatially for better locality (see G.4.3)
- [ ] Enable compression if I/O-bound (see G.5.4)
- [ ] Pre-allocate vectors with capacity (see G.3.4)

### G.9.3 Algorithmic Improvements
- [ ] Batch queries instead of one-at-a-time (see G.4.1)
- [ ] Use hierarchical queries for range operations (see G.4.2)
- [ ] Cache frequently-accessed data
- [ ] Reduce unnecessary coordinate conversions (see G.8.1)

### G.9.4 Platform-Specific Tuning
- [ ] Profile on target hardware (not just dev machine)
- [ ] Test on production-like data volumes
- [ ] Measure across different CPUs (x86 vs ARM)
- [ ] Consider GPU acceleration for batch workloads (see G.6.4)

### G.9.5 Validation
- [ ] Re-run profiler to confirm improvements
- [ ] Run benchmarks before and after (use Criterion)
- [ ] Check for regressions in other metrics (e.g., memory usage)
- [ ] Validate correctness (optimizations shouldn't change results)

---

## G.10 Performance Targets by Use Case

### G.10.1 Real-Time Systems (Robotics, Games)

**Targets:**
- Encoding: <50 ns per coordinate
- Neighbor query: <500 ns (14 neighbors)
- Range query: <10 μs for radius=10 cells
- Container insert: <200 ns per point (streaming)

**Critical optimizations:**
- Enable BMI2/NEON
- Use InMemory or Streaming containers
- Pre-allocate neighbor buffers
- Sort by Hilbert curve for cache locality

### G.10.2 Batch Processing (Scientific Computing, GIS)

**Targets:**
- Throughput: >1M points/sec encoding
- Compression: 3-5× for storage
- Sequential read: >500 MB/s
- Parallel speedup: >0.8× per core

**Critical optimizations:**
- Use SIMD (AVX2/NEON)
- Compress with LZ4 or Zstd
- Use Sequential containers
- Parallelize with Rayon

### G.10.3 Distributed Systems (Cloud, Big Data)

**Targets:**
- Query latency: <100 ms (p99)
- Network bandwidth: <10 MB/s per node
- Shard balance: <20% deviation
- Cache hit rate: >80%

**Critical optimizations:**
- Spatial sharding (see G.7.1)
- Batch network requests (see G.7.2)
- Compress wire protocol
- Cache hot data at coordinator

---

## G.11 Further Reading

**Profiling:**
- Chapter 7 (Performance Optimization) - Profiling methodologies
- Appendix C (Performance Benchmarks) - Expected performance baselines
- Gregg, B. (2020). *Systems Performance*, 2nd ed. - Comprehensive profiling guide

**Optimization Techniques:**
- Fog, A. (2023). *Optimizing software in C++* - Low-level optimization patterns
- Chapter 7.4-7.6 - SIMD, BMI2, cache optimization details

**Distributed Systems:**
- Chapter 14 (Distributed and Parallel) - Sharding strategies
- Kleppmann, M. (2017). *Designing Data-Intensive Applications* - Distributed systems patterns

---

**End of Appendix G**
