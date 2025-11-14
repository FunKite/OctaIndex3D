# Chapter 7: Performance Optimization

## Learning Objectives

By the end of this chapter, you will be able to:

1. Describe the key hardware features that influence OctaIndex3D performance.
2. Understand how Morton encoding is implemented using BMI2 and similar instruction sets.
3. Recognize when SIMD and batching can provide substantial speedups.
4. Choose appropriate data layouts for cache-friendly processing.
5. Evaluate when GPU acceleration is beneficial for your workload.

---

## 7.1 Hardware Architecture Overview

Spatial indexing performance is dominated by three hardware factors:

- **Instruction throughput**: how many integer and bitwise operations per cycle.
- **Memory hierarchy**: cache sizes, latencies, and bandwidth.
- **Parallelism**: SIMD width and number of cores.

OctaIndex3D is designed with several assumptions:

- L1 and L2 caches are orders of magnitude faster than main memory.
- Sequential access patterns are favored by hardware prefetchers.
- Short, predictable branches are cheaper than unpredictable ones.

From these assumptions flow several design rules:

- Prefer **linear scans** over pointer chasing, when possible.
- Use **compact value types** to keep working sets in cache.
- Batch operations to amortize overhead and exploit SIMD.

---

## 7.2 BMI2 Morton Encoding

Chapter 3 introduced Morton (Z-order) encoding conceptually. In practice, OctaIndex3D uses **BMI2 instructions** (where available) to implement fast bit interleaving and de-interleaving.

### 7.2.1 The `pdep` and `pext` Instructions

On x86_64 with BMI2 support, two instructions are especially useful:

- `pdep` (parallel deposit): scatter bits from a source register into selected positions in a destination.
- `pext` (parallel extract): collect bits from selected positions into a compact representation.

Morton encoding can be framed as:

- Take the bits of `x`, `y`, and `z`.
- Use `pdep` with precomputed masks to place them in alternating bit positions.

The resulting sequence of `pdep` operations is far faster than manually interleaving bits with shifts and masks, especially for 64-bit indices.

### 7.2.2 Feature Detection and Fallbacks

OctaIndex3D does not assume BMI2 is always available. Instead:

- At **compile time**, feature flags control whether BMI2-optimized code is built.
- At **run time**, CPU feature detection decides which implementation to use.

Fallback paths use:

- Portable bit-manipulation routines.
- Possibly SIMD-friendly implementations that work across architectures.

This layered approach ensures:

- Excellent performance on modern servers and desktops.
- Correctness and reasonable speed on older or embedded systems.

---

## 7.3 SIMD and Batch Processing

Single queries are important, but many real workloads operate on **batches**:

- Robotic planners evaluating candidate paths.
- Simulation codes updating millions of cells per timestep.
- Query engines answering many nearest-neighbor requests at once.

OctaIndex3D provides batch-oriented APIs that:

- Take slices of identifiers or coordinates.
- Process them using vectorized loops.
- Minimize per-call overhead and avoid repeated bounds checks.

On architectures with SIMD (AVX2, NEON, etc.), the library can:

- Compute Morton or Hilbert encodings for multiple points in parallel.
- Perform range checks and masking in wide registers.

Even when explicit SIMD is not available, batching improves:

- Cache locality (data processed together is stored together).
- Branch predictability (loops are longer and more regular).

---

## 7.4 Cache-Friendly Data Layouts

Cache behavior often dominates raw arithmetic cost. To keep data hot in cache, OctaIndex3D favors:

- **Struct-of-arrays** layouts for numeric payloads.
- **Dense arrays** of identifiers.
- **Morton- or Hilbert-ordered** iteration to respect spatial locality.

Consider a container storing:

- An `Index64` identifier.
- An occupancy probability.
- A timestamp.

An array-of-structs layout might look like:

```text
[ (id0, occ0, t0), (id1, occ1, t1), ... ]
```

whereas a struct-of-arrays layout separates the fields:

```text
ids:    [id0, id1, id2, ...]
occ:    [occ0, occ1, occ2, ...]
times:  [t0, t1, t2, ...]
```

The latter is more amenable to:

- Vectorized operations on occupancy values.
- Scans over timestamps without touching identifiers.

OctaIndex3D does not mandate one layout; instead, it:

- Provides traits that both layouts can implement.
- Documents the trade-offs so that users can choose appropriately.

---

## 7.5 Cross-Architecture Considerations

While x86_64 with BMI2 and AVX2 is common, many applications run on:

- ARM64 (phones, tablets, some servers).
- Mixed-architecture clusters.

Designing for portability means:

- Avoiding tight coupling to a single instruction set.
- Isolating architecture-specific code in small, well-tested modules.
- Providing configuration options so users can pick the right trade-offs.

OctaIndex3D’s performance story is thus:

- **Best effort** on any hardware.
- **Near-optimal** on hardware with rich bit-manipulation and SIMD support.

---

## 7.6 GPU Acceleration

GPUs offer enormous parallel throughput but come with their own costs:

- Data transfer latency to and from device memory.
- Complex programming models.
- Limited flexibility for branch-heavy logic.

For many spatial indexing tasks, CPUs with good caches and SIMD are sufficient. However, GPU acceleration can be attractive when:

- You perform large, embarrassingly parallel computations (e.g., evaluating fields on a dense grid).
- The same kernel is applied to millions of points.
- Data can remain on the GPU for extended periods (e.g., in simulation pipelines).

From an architectural perspective, OctaIndex3D:

- Keeps its core data representations GPU-friendly (compact, POD-like types).
- Leaves the choice of GPU framework (CUDA, Vulkan, Metal) to the host application.
- Focuses its own complexity budget on high-quality CPU implementations.

---

## 7.7 Summary

In this chapter, we explored how OctaIndex3D turns the theoretical and architectural foundations of earlier parts into high-performance implementations:

- We examined the **hardware model** that informs design decisions.
- We saw how **BMI2 instructions** make Morton encoding extremely fast when available.
- We discussed the benefits of **SIMD and batch processing**.
- We explored **cache-friendly data layouts** and cross-architecture considerations.
- We considered where **GPU acceleration** fits into the broader picture.

The next chapter applies these performance principles to the design of concrete container formats and persistence mechanisms.

