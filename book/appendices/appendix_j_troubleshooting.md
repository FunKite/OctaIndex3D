# Appendix J: Troubleshooting Guide

This appendix collects common problems, their symptoms, root causes, and solutions. If you're stuck, start here.

---

## Symptom Index

| Symptom | Likely Cause | Section |
|---------|--------------|---------|
| Paths are blocky/zigzag | Wrong neighbor connectivity | §J.1 |
| Memory didn't decrease 29% | Sparse data or wrong comparison | §J.2 |
| BMI2 not helping | Memory-bound, not compute-bound | §J.3 |
| Morton decode is slow | Fallback path being used | §J.4 |
| Coordinates rejected as invalid | Parity constraint violation | §J.5 |
| Frame transformation fails | Missing frame registration | §J.6 |
| Container won't open | Version mismatch or corruption | §J.7 |
| GPU slower than CPU | Batch too small or transfer overhead | §J.8 |
| Hierarchical queries return wrong cells | LOD mismatch | §J.9 |
| Test failures after upgrade | Breaking API change | §J.10 |

---

## J.1 Paths Are Blocky or Zigzag

### Symptoms
- A* paths prefer cardinal directions over diagonals
- Paths take unnecessary 90° turns
- Path length is 20-40% longer than Euclidean distance

### Root Cause
You're using 6-neighbor (face-only) connectivity instead of 14-neighbor BCC connectivity.

### Diagnosis
```rust
// Check your neighbor expansion
let neighbors = get_neighbors(current_cell);
println!("Neighbor count: {}", neighbors.len()); // Should be 14, not 6 or 26
```

### Solution
Use the full BCC neighbor set:

```rust
use octaindex3d::BCC_NEIGHBORS_14;

fn expand_neighbors(cell: Index64) -> Vec<Index64> {
    BCC_NEIGHBORS_14.iter()
        .filter_map(|&(dx, dy, dz)| {
            let (x, y, z) = cell.decode();
            Index64::encode(x + dx, y + dy, z + dz, cell.lod()).ok()
        })
        .collect()
}
```

### Verification
After fixing, path error relative to Euclidean should be ±5%, not ±41%.

---

## J.2 Memory Usage Didn't Decrease 29%

### Symptoms
- Expected 29% memory savings after BCC migration
- Actual savings are much smaller or zero

### Root Causes

**Cause 1: Sparse Data**
BCC's 29% savings apply to *dense* grids. If your data is sparse (e.g., 1% occupancy), you're dominated by pointer/index overhead, not cell storage.

**Diagnosis:**
```rust
let density = container.cell_count() as f64 / container.bounding_volume();
println!("Density: {:.4}", density); // If < 0.01, you're sparse
```

**Solution:**
For sparse data, consider sparse container formats or octrees with empty-subtree pruning.

**Cause 2: Wrong Comparison Baseline**
You're comparing BCC to a 6-neighbor cubic grid, but the fair comparison is to a 26-neighbor grid at equivalent sampling quality.

**Cause 3: Overhead Not Counted**
Container metadata, indices, and compression tables add overhead that dominates for small datasets.

**Solution:**
Benchmark with >1M cells to see the 29% savings clearly.

---

## J.3 BMI2 Optimization Not Helping

### Symptoms
- Enabled BMI2, but Morton encode/decode still slow
- Profile shows Morton functions not in top hotspots

### Root Cause
Your workload is **memory-bound**, not **compute-bound**. Morton encoding is already fast enough; the bottleneck is elsewhere.

### Diagnosis
```bash
perf stat -e cache-misses,cache-references ./your_benchmark
```

If cache miss rate > 10%, you're memory-bound.

### Solution
Focus on:
- Data layout (SoA vs AoS)
- Morton-ordered iteration
- Prefetching
- Reducing working set size

---

## J.4 Morton Decode Is Slow

### Symptoms
- Morton decode taking 25-35 cycles instead of 3-4
- Flamegraph shows `morton_decode_fallback` not `morton_decode_bmi2`

### Root Cause
BMI2 feature not detected or not enabled at compile time.

### Diagnosis
```rust
#[cfg(target_arch = "x86_64")]
println!("BMI2 available: {}", is_x86_feature_detected!("bmi2"));
```

### Solutions

**Solution 1: Enable at compile time**
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

**Solution 2: Check for AMD Zen 1/2 quirk**
Early AMD Zen processors have slow BMI2 (`pdep`/`pext` are microcoded). On Zen 1/2, fallback may actually be faster. Zen 3+ has fast BMI2.

**Solution 3: Force specific feature**
```bash
RUSTFLAGS="-C target-feature=+bmi2" cargo build --release
```

---

## J.5 Coordinates Rejected as Invalid

### Symptoms
- `Index64::encode()` returns `Err(ParityViolation)`
- Points that "should" be valid are rejected

### Root Cause
The coordinates don't satisfy the BCC parity constraint: $(x + y + z) \mod 2 \neq 0$

### Diagnosis
```rust
let (x, y, z) = (3, 5, 7);
println!("Sum: {}, Parity: {}", x + y + z, (x + y + z) % 2);
// Sum: 15, Parity: 1 -- INVALID for BCC
```

### Solutions

**Solution 1: Round to nearest BCC point**
```rust
fn snap_to_bcc(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    if (x + y + z) & 1 == 0 {
        (x, y, z)
    } else {
        // Round z to nearest valid value
        (x, y, if z & 1 == 0 { z + 1 } else { z - 1 })
    }
}
```

**Solution 2: Use continuous coordinates with automatic snapping**
```rust
let cell = Index64::from_continuous(3.5, 5.2, 7.8, lod)?;
// Automatically snaps to nearest BCC cell
```

---

## J.6 Frame Transformation Fails

### Symptoms
- `transform_to_frame()` returns `Err(FrameNotFound)`
- Cross-frame queries fail silently

### Root Cause
One or both frames are not registered in the FrameRegistry.

### Diagnosis
```rust
let frames = octaindex3d::list_frames();
let source_exists = frames.iter().any(|(id, _)| *id == 10);
let target_exists = frames.iter().any(|(id, _)| *id == 0);
println!("Source frame registered: {}", source_exists);
println!("Target frame registered: {}", target_exists);
```

### Solution
Register frames before use:

```rust
use octaindex3d::{register_frame, FrameDescriptor};

register_frame(10, FrameDescriptor::new("my_frame", "WGS-84", "custom frame", true, 1.0))?;
register_frame(0, FrameDescriptor::new("ECEF", "WGS-84", "Earth-Centered Earth-Fixed", true, 1.0))?;
```

---

## J.7 Container Won't Open

### Symptoms
- `Container::open()` returns error
- Previously saved data inaccessible

### Root Causes

**Cause 1: Version Mismatch**
Container was saved with an older/newer format version.

**Diagnosis:**
```rust
let header = ContainerHeader::read_from_file(path)?;
println!("Container version: {}", header.version);
println!("Library version: {}", octaindex3d::VERSION);
```

**Solution:**
Use the migration tool:
```bash
octaindex3d migrate --from 2 --to 3 old_container.oct new_container.oct
```

**Cause 2: Corruption**
File was truncated or corrupted.

**Diagnosis:**
```bash
octaindex3d verify container.oct
```

**Solution:**
Restore from backup. Consider enabling checksums for future containers.

---

## J.8 GPU Slower Than CPU

### Symptoms
- GPU path takes longer than CPU path
- Expected speedup not achieved

### Root Causes

**Cause 1: Batch Too Small**
GPU has ~100µs startup overhead. For small batches, CPU wins.

**Diagnosis:**
```rust
println!("Batch size: {}", batch.len());
// If < 100,000, CPU is likely faster
```

**Solution:**
Only use GPU for batches > 100,000 elements, or accumulate multiple small batches.

**Cause 2: Transfer Overhead**
Data is being copied to/from GPU every call.

**Solution:**
Keep data resident on GPU:
```rust
let gpu_buffer = GpuBuffer::new_persistent(data)?;
// Reuse gpu_buffer across multiple operations
```

**Cause 3: Wrong Workload**
GPU excels at embarrassingly parallel work. Pointer-chasing, irregular access, and branchy code are slow on GPU.

---

## J.9 Hierarchical Queries Return Wrong Cells

### Symptoms
- Parent/child traversal returns unexpected cells
- Multi-LOD queries miss expected results

### Root Cause
LOD levels are mismatched between query and data.

### Diagnosis
```rust
println!("Query LOD: {}", query.lod());
println!("Data LOD range: {} to {}", container.min_lod(), container.max_lod());
```

### Solution
Ensure query LOD is within container's LOD range:

```rust
let query_lod = query.lod().clamp(container.min_lod(), container.max_lod());
let adjusted_query = query.with_lod(query_lod);
```

---

## J.10 Test Failures After Library Upgrade

### Symptoms
- Tests pass on old version, fail on new version
- Compile errors or runtime panics after upgrade

### Root Cause
Breaking API change between versions.

### Diagnosis
Check the CHANGELOG:
```bash
cat CHANGELOG.md | grep -A20 "\[0.5.0\]"
```

### Common Migration Issues

**Issue: Method renamed**
```rust
// Old (v0.4.x)
container.get_neighbors_12(idx)

// New (v0.5.x)
container.get_neighbors(idx, NeighborSet::BCC12)
```

**Issue: Type signature changed**
```rust
// Old: returned Vec<Index64>
// New: returns impl Iterator<Item = Index64>
let neighbors: Vec<_> = container.neighbors(idx).collect();
```

**Issue: Error type changed**
```rust
// Old: returned Option<T>
// New: returns Result<T, SpatialError>
let value = container.get(idx)?;  // Add ?
```

### Solution
See **Appendix F: Migration Guide** for version-specific upgrade instructions.

---

## General Debugging Tips

### Enable Debug Logging
```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

### Profile Before Guessing
```bash
# CPU profile
perf record --call-graph dwarf ./your_app
perf report

# Memory profile
valgrind --tool=massif ./your_app
ms_print massif.out.*
```

### Minimal Reproduction
When filing issues, provide:
1. OctaIndex3D version
2. Rust version (`rustc --version`)
3. Platform (`uname -a`)
4. Minimal code that reproduces the issue
5. Expected vs actual behavior

---

## Getting Help

- **GitHub Issues**: https://github.com/FunKite/OctaIndex3D/issues
- **API Documentation**: https://docs.rs/octaindex3d
- **Performance Questions**: See Chapter 7 and Appendix G

---

*"The first step in fixing a bug is admitting you have one."*
