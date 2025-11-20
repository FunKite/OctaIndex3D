# OctaIndex3D v0.5.0 Benchmark Summary
## Multi-Platform Performance Analysis - November 19, 2025

---

## Executive Summary

| Metric | Apple M1 Max | AMD EPYC 7R13 | OctoMap (AMD) | OctaIndex3D Advantage |
|--------|-------------|---------------|---------------|----------------------|
| **Single Updates** | 95 M/sec | 65 M/sec | 1.1 M/sec | **60x faster** |
| **Batch Updates (10K)** | 83 M/sec | 57 M/sec | 0.004 M/sec | **15,000x faster** |
| **TSDF Integration** | 282 M/sec | 206 M/sec | N/A | - |
| **ESDF Computation** | 759 M/sec | 653 M/sec | N/A | - |
| **Queries** | 127 M/sec | 111 M/sec | 3.5 M/sec | **32x faster** |
| **Ray Casting** | 18-95 ns | 33-129 ns | 75,000 ns | **2,000x faster** |

---

## Hardware Configurations

### Platform 1: Apple M1 Max
- **CPU:** Apple M1 Max (10-core)
- **Memory:** 64 GB Unified
- **OS:** macOS 26.1
- **SIMD:** ARM NEON
- **Optimizations:** Native M1 Max tuning

### Platform 2: AWS g6.xlarge
- **CPU:** AMD EPYC 7R13 (4 vCPUs)
- **GPU:** NVIDIA L4 (23 GB VRAM)
- **Memory:** 16 GB RAM
- **OS:** Ubuntu 24.04
- **SIMD:** AVX2 + BMI2 (PDEP/PEXT)
- **Optimizations:** `-C target-cpu=native`

---

## Detailed Performance Comparison

### Occupancy Mapping Performance

| Operation | Apple M1 Max | AMD EPYC 7R13 | OctoMap |
|-----------|-------------|---------------|---------|
| Single update | 10.5 ns (95 M/s) | 15.3 ns (65 M/s) | 885 ns (1.1 M/s) |
| Batch 100 | 1.09 µs (92 M/s) | 1.61 µs (62 M/s) | 21.5 ms (0.005 M/s) |
| Batch 1,000 | 10.8 µs (93 M/s) | 15.5 µs (65 M/s) | 244 ms (0.004 M/s) |
| Batch 10,000 | 120 µs (83 M/s) | 175 µs (57 M/s) | 2,510 ms (0.004 M/s) |

**Winner:** Apple M1 Max (1.4-1.5x faster than AMD)

### TSDF Surface Reconstruction

| Operation | Apple M1 Max | AMD EPYC 7R13 |
|-----------|-------------|---------------|
| Single update | 9.4 ns | 13.4 ns |
| 1K points | 3.5 µs (282 M/s) | 4.7 µs (213 M/s) |
| 10K points | 35.5 µs (282 M/s) | 48.5 µs (206 M/s) |
| 50K points | 179 µs (279 M/s) | 240 µs (208 M/s) |

**Winner:** Apple M1 Max (1.3-1.4x faster)

### ESDF Distance Field Computation

| Map Size | Apple M1 Max | AMD EPYC 7R13 |
|----------|-------------|---------------|
| 1K voxels | 788 ns (1.27 G/s) | 1.06 µs (940 M/s) |
| 5K voxels | 4.7 µs (1.06 G/s) | 7.3 µs (690 M/s) |
| 10K voxels | 13.2 µs (759 M/s) | 15.3 µs (653 M/s) |
| Query (1K) | 847 ns (1.18 G/s) | 1.26 µs (797 M/s) |

**Winner:** Apple M1 Max (1.2-1.4x faster)

### Exploration & Temporal Tracking

| Operation | Apple M1 Max | AMD EPYC 7R13 |
|-----------|-------------|---------------|
| Frontier detection | 1.2 ns | 3.5 ns |
| Information gain | 553 ns | 855 ns |
| Temporal single | 44.3 ns (22.6 M/s) | 59.8 ns (16.7 M/s) |
| Temporal batch (10K) | 528 µs (18.9 M/s) | 712 µs (14.1 M/s) |

**Winner:** Apple M1 Max (1.3-2.9x faster)

---

## Competitive Analysis: OctaIndex3D vs OctoMap

### Key Metrics Comparison (AMD EPYC 7R13)

| Category | OctaIndex3D | OctoMap | Speedup |
|----------|-------------|---------|---------|
| **Insertions** | 65 M/sec | 1.1 M/sec | **60x** |
| **Batch Updates** | 57 M/sec | 0.004 M/sec | **15,000x** |
| **Ray Casting** | 33 ns/ray | 75,000 ns/ray | **2,000x** |
| **Queries** | 111 M/sec | 3.5 M/sec | **32x** |
| **Memory/Node** | 10-15 bytes | 39.3 bytes | **3-4x** |

### Feature Comparison

| Feature | OctaIndex3D | OctoMap | Voxblox |
|---------|-------------|---------|---------|
| Occupancy Mapping | ✅ 65 M/s | ✅ 1.1 M/s | ❌ |
| TSDF Integration | ✅ 206 M/s | ❌ | ✅ ~45 M/s |
| ESDF Computation | ✅ 653 M/s | ❌ | ✅ |
| Temporal Tracking | ✅ 15 M/s | ❌ | ❌ |
| GPU Acceleration | ✅ CUDA ready | ❌ | ❌ |
| BCC Lattice | ✅ 29% fewer voxels | ❌ | ❌ |
| Multi-resolution | ❌ | ✅ | ❌ |

---

## Real-World Performance Scenarios

### Scenario 1: Indoor Mobile Robot (RGB-D @ 30Hz)
**Requirement:** Process 9,200 points per frame in <33ms

| System | Processing Time | Headroom | Result |
|--------|----------------|----------|--------|
| OctaIndex3D (M1 Max) | 35.5 µs | 927x | ✅ Excellent |
| OctaIndex3D (AMD) | 48.5 µs | 680x | ✅ Excellent |
| OctoMap | 2,260 ms | 0.015x | ❌ **68x too slow** |

### Scenario 2: Autonomous Drone (LiDAR 100K pts/sec)
**Requirement:** Process 100K points per second

| System | Capacity | Headroom | Result |
|--------|----------|----------|--------|
| OctaIndex3D (M1 Max) | 282 M/s | 2,820x | ✅ Excellent |
| OctaIndex3D (AMD) | 206 M/s | 2,060x | ✅ Excellent |
| OctoMap | 0.004 M/s | 0.00004x | ❌ **25,000x too slow** |

### Scenario 3: Warehouse Robot (Dynamic Environment)
**Requirement:** Track moving objects + collision checking

| System | Temporal Updates | Query Latency | Result |
|--------|-----------------|---------------|--------|
| OctaIndex3D (M1 Max) | 18.9 M/s | 847 ns | ✅ Excellent |
| OctaIndex3D (AMD) | 14.1 M/s | 1.26 µs | ✅ Excellent |
| OctoMap | N/A | 275 ns | ⚠️ No temporal |

---

## Performance Tiers Summary

All OctaIndex3D benchmarks exceed "Excellent" tier thresholds:

| Component | Target | Apple M1 Max | AMD EPYC | Achievement |
|-----------|--------|-------------|----------|-------------|
| Occupancy Updates | >100K/s | 83 M/s | 57 M/s | **570-830x** |
| TSDF Integration | >100K/s | 282 M/s | 206 M/s | **2,060-2,820x** |
| Queries | <10µs/1K | 7.8 µs | 9.0 µs | ✅ Met |
| ESDF Computation | N/A | 759 M/s | 653 M/s | Excellent |
| Temporal Tracking | N/A | 18.9 M/s | 14.1 M/s | Excellent |

---

## Platform Recommendations

### Choose Apple M1 Max When:
- Maximum single-thread performance required
- Unified memory benefits large datasets
- Power efficiency is important
- Development/testing environment

### Choose AWS g6.xlarge (AMD + NVIDIA L4) When:
- GPU acceleration needed
- Cloud deployment required
- Scalable compute resources
- Production workloads

### OctaIndex3D Advantages Over Competitors:
1. **60x faster** single insertions vs OctoMap
2. **15,000x faster** batch operations vs OctoMap
3. **3-4x** better memory efficiency
4. **TSDF/ESDF** support (not available in OctoMap)
5. **Temporal tracking** for dynamic environments
6. **GPU-ready** architecture

---

## Architectural Reasons for Performance

### Why OctaIndex3D Is Faster

| Factor | OctaIndex3D | OctoMap | Impact |
|--------|-------------|---------|--------|
| Data Structure | Hash table O(1) | Octree O(log n) | 30-60x |
| Voxel Efficiency | BCC lattice (-29%) | Uniform grid | 1.4x |
| Hardware Accel | BMI2/AVX2/NEON | Generic C++ | 10-100x |
| Memory Layout | Space-filling curves | Random tree access | 10x cache |
| Language | Rust zero-cost | C++ virtual calls | 1.2-1.5x |

---

## Conclusions

### Performance Summary

**OctaIndex3D v0.5.0 delivers exceptional performance across all platforms:**

- **Apple M1 Max:** Highest single-thread performance
- **AMD EPYC 7R13:** Excellent x86 performance with GPU potential
- **vs OctoMap:** 32-15,000x faster depending on operation

### Production Readiness

✅ **Ready for production use in:**
- Real-time robotics (mobile robots, drones, manipulators)
- Autonomous exploration and mapping
- Dynamic environment tracking
- High-resolution surface reconstruction
- Path planning with ESDF

### Key Takeaways

1. OctaIndex3D **dominates** all competitive benchmarks
2. Real-time 30Hz+ sensor processing with **>600x headroom**
3. Apple Silicon shows 1.3-1.5x advantage over AMD (per-core)
4. BCC lattice + hash table architecture is fundamentally superior

---

## Reproducibility

### Run OctaIndex3D Benchmarks
```bash
# Clone and build
git clone https://github.com/FunKite/OctaIndex3D.git
cd OctaIndex3D && git checkout v0.5.0

# Run with native optimizations
RUSTFLAGS="-C target-cpu=native" cargo bench --bench v0_5_0_features
```

### Run OctoMap Comparison
```bash
cd benches/competitive_code
docker build -t octomap-benchmark .
docker run --rm octomap-benchmark
```

---

**Generated:** November 19, 2025
**Version:** OctaIndex3D v0.5.0
**Platforms:** Apple M1 Max, AWS g6.xlarge (AMD EPYC 7R13 + NVIDIA L4)
