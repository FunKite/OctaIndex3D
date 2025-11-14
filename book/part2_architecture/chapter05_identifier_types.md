# Chapter 5: Identifier Types and Encodings

## Learning Objectives

By the end of this chapter, you will be able to:

1. Explain why OctaIndex3D uses multiple identifier types instead of a single “universal key”.
2. Describe the structure and intended use cases of `Galactic128`, `Index64`, `Route64`, and `Hilbert64`.
3. Understand how Morton and Hilbert encodings linearize the BCC lattice.
4. Choose appropriate identifiers and encodings for different workloads.
5. Interpret and generate human-readable encodings for debugging and interoperability.

---

## 5.1 Multi-Scale Identification Requirements

Real systems rarely operate at a single scale. A robotics stack, for example, might:

- Use a **global frame** (Earth-centered) for long-range planning.
- Use a **local frame** (warehouse-relative) for obstacle avoidance.
- Use a **vehicle frame** for sensor fusion.

Each of these layers has different requirements:

- **Global planning** cares about large regions (kilometers) and robustness to floating-point rounding.
- **Local navigation** cares about fine detail (centimeters) and low-latency queries.
- **Internal algorithms** often work on compact, integer-based indices.

Trying to satisfy all of these constraints with a single identifier design leads to:

- Overly wide identifiers that hurt cache efficiency.
- Under-specified identifiers that silently mix frames or units.
- Heavy, error-prone conversion code scattered throughout the codebase.

OctaIndex3D instead adopts a **portfolio of identifier types**, each optimized for a specific role, with explicit conversions between them. Architecturally, this keeps each layer simple while preserving end-to-end correctness.

---

## 5.2 `Galactic128`: Global Addressing

`Galactic128` is designed for **global-scale indexing** where:

- The Earth (or another celestial body) is treated as a whole.
- Multiple frames must coexist (e.g., WGS84, ECEF, mission-specific frames).
- Long-term stability and reproducibility matter more than raw speed.

At a high level, a `Galactic128` identifier encodes:

- A **frame identifier** (which coordinate system we are in).
- A **level of detail** (coarse vs. fine resolution).
- A **BCC lattice coordinate** within that frame.

Conceptually, you can think of `Galactic128` as:

```rust
pub struct Galactic128 {
    hi: u64, // frame + hierarchy metadata
    lo: u64, // BCC index bits (Morton or Hilbert)
}
```

This is not the exact on-disk layout, but it captures the idea:

- The 128-bit width provides ample room for future expansion.
- The top half can encode frame and hierarchy metadata.
- The bottom half can hold a 64-bit spatial encoding.

**Intended use cases**:

- Long-term storage of “where” data in logging systems.
- Cross-service communication where frame context must not be lost.
- Cross-mission datasets where future readers may not have access to original application code.

`Galactic128` is deliberately **heavier** than other identifiers. It is not the best choice for inner loops, but it is the right choice for boundaries between subsystems.

---

## 5.3 `Index64`: Morton-Encoded Spatial Queries

`Index64` is the workhorse identifier for **fast spatial queries**. It is:

- Exactly 64 bits wide.
- Designed for dense storage and fast hashing.
- Encoded using a BCC-specific Morton (Z-order) scheme.

### 5.3.1 Structure and Semantics

At a conceptual level, an `Index64` consists of:

- A **level of detail** field.
- Interleaved bits for the BCC lattice coordinates `(x, y, z)`.

The details of BCC-specific Morton encoding are covered in Chapter 3; here we focus on architectural consequences:

- Neighboring cells in space tend to be close in Morton order.
- Identifiers can be compared as integers to approximate spatial locality.
- Range scans over `Index64` values can serve as a cheap spatial pre-filter.

OctaIndex3D exposes `Index64` as an opaque newtype:

```rust
pub struct Index64(u64);
```

with methods such as:

- `fn lod(&self) -> u8`
- `fn parent(&self) -> Option<Index64>`
- `fn children(&self) -> [Index64; 8]`
- `fn neighbors(&self) -> impl Iterator<Item = Index64>`

These operations use the bit-encoded hierarchy to run in constant time.

### 5.3.2 When to Use `Index64`

Choose `Index64` when:

- You need high-throughput spatial queries.
- Your workload is dominated by nearest-neighbor, range, or traversal operations.
- You are implementing containers and cache-friendly data structures.

Avoid using raw `u64` values; always wrap and unwrap through the provided API to preserve invariants and benefit from future improvements.

---

## 5.4 `Route64`: Local Routing Coordinates

Where `Index64` focuses on **static spatial indexing**, `Route64` is tuned for **local routing and traversal**.

Imagine a path-planning algorithm that:

- Operates within a fixed local region (e.g., a warehouse).
- Frequently steps between neighboring cells along a candidate path.
- Needs to maintain compact, incremental representations of routes.

`Route64` encodes:

- A base `Index64`-compatible location.
- Additional bits that capture local routing state (e.g., preferred direction, branch index).

This allows:

- Efficient representation of partial paths as sequences of `Route64` values.
- Branch-and-bound algorithms that store frontiers compactly.
- Integration with higher-level planners that work in continuous coordinates.

Architecturally, `Route64`:

- Shares many operations with `Index64` (like neighbor enumeration).
- Adds methods for path extension and cost accumulation.
- Remains 64 bits to preserve cache characteristics.

---

## 5.5 `Hilbert64`: Enhanced Locality

Morton encoding is simple and fast, but its locality is not optimal. As discussed in Chapter 3, **Hilbert curves** provide better locality at the cost of more complex bit manipulations.

`Hilbert64` is an identifier type that:

- Encodes BCC lattice coordinates using a 3D Hilbert curve.
- Preserves better spatial locality than Morton for many workloads.
- Trades a small amount of CPU time for improved cache behavior.

### 5.5.1 When Hilbert Beats Morton

Hilbert ordering is particularly advantageous when:

- You perform long, sequential scans over regions of space.
- The cost of cache misses dominates arithmetic cost.
- Your hardware has strong prefetching that benefits from smoother access patterns.

Morton ordering is often preferable when:

- You need raw speed for single-point operations.
- You are heavily constrained by instruction throughput rather than memory.

OctaIndex3D therefore does not pick a single “winner”. Instead, it offers both:

- `Index64` (Morton) for simple, fast indexing.
- `Hilbert64` for workloads that benefit from stronger locality.

Conversions between the two are explicit, type-checked operations.

---

## 5.6 Conversions and Interoperability

Having multiple identifier types is only useful if conversions are:

- Well-defined.
- Efficient.
- Easy to use correctly.

OctaIndex3D provides a small set of conversion functions such as:

- `Galactic128::from_index64(frame, index64) -> Galactic128`
- `Index64::from_galactic128(id: Galactic128) -> Result<Index64, IndexError>`
- `Hilbert64::from_index64(index64: Index64) -> Hilbert64`
- `Index64::from_hilbert64(id: Hilbert64) -> Index64`

Architecturally, conversions follow these rules:

- **Frame changes are explicit**: converting between frames always goes through a continuous coordinate representation and may lose precision at extremely high LODs.
- **Encoding changes preserve semantics**: Morton ↔ Hilbert conversions keep the underlying lattice location and LOD constant.
- **Errors are surfaced**: conversions that cannot be represented (e.g., downsampling beyond supported LODs) return `Result` types.

This approach prevents subtle bugs where identifiers appear to be interchangeable but actually represent different frames or resolutions.

---

## 5.7 Human-Readable Encodings

Binary identifiers are ideal for machines but inconvenient for:

- Log files.
- Manual debugging.
- Copy–paste reproduction of issues.

To bridge this gap, OctaIndex3D supports human-readable encodings inspired by schemes like **Bech32m**:

- A short, lowercase human-readable prefix indicating the identifier type and frame (e.g., `oi1`, `oi1-gal`, `oi1-loc`).
- A checksummed payload encoding the bits of the identifier.

For example:

```text
oi1-gal1q9a5h7k3...
oi1-idx1qqp8z4u...
```

The exact format is specified in the library documentation, but the architectural goals are clear:

- **Self-describing**: you can tell at a glance what kind of identifier you are looking at.
- **Robust to transcription errors**: the checksum detects common mistakes.
- **Stable over time**: changes to the internal binary layout preserve the external textual format whenever possible.

Applications can freely log and exchange these encodings, then parse them back into strongly-typed identifiers when needed.

---

## 5.8 Summary

In this chapter, we examined the portfolio of identifier types that OctaIndex3D uses to represent locations in BCC lattices:

- `Galactic128` provides **global, frame-aware addressing** suitable for long-term storage and cross-system integration.
- `Index64` serves as the **fast, Morton-encoded workhorse** for spatial queries and containers.
- `Route64` augments local indices with **routing context**, enabling compact representation of paths and frontiers.
- `Hilbert64` offers a **Hilbert-encoded alternative** with improved locality for scan-heavy workloads.

We also saw how:

- Explicit **conversions** maintain correctness across frames and encodings.
- **Human-readable encodings** make debugging and interoperability practical without sacrificing type safety.

Together, these identifier types form the connective tissue between the architectural concepts of Part II and the concrete implementation techniques explored in Part III.

