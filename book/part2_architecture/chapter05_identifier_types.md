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

### 5.2.1 Bitfield Layout

The 128-bit structure is divided into logical fields:

```text
┌─────────────────────────────────────────────────────────────────┐
│  Bits 127-120  │  Bits 119-112  │  Bits 111-96  │  Bits 95-64   │
│   Version      │  Frame ID      │   Reserved    │   LOD + Flags │
├─────────────────────────────────────────────────────────────────┤
│                    Bits 63-0: Morton/Hilbert Code                │
└─────────────────────────────────────────────────────────────────┘
```

**Field descriptions:**

- **Version** (8 bits): Format version for future extensibility. Current version = 1.
- **Frame ID** (8 bits): Identifies the coordinate frame (WGS84, ECEF, custom frames).
- **Reserved** (16 bits): Reserved for future metadata (time stamps, provenance, etc.).
- **LOD + Flags** (32 bits): Level of detail (24 bits) + encoding flags (8 bits).
- **Morton/Hilbert Code** (64 bits): Spatial index within the frame at specified LOD.

### 5.2.2 Implementation

```rust
/// Global 128-bit identifier with frame and hierarchy metadata
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C, align(16))]
pub struct Galactic128 {
    hi: u64,  // Version, frame, reserved, LOD/flags
    lo: u64,  // Morton or Hilbert code
}

impl Galactic128 {
    /// Construct from frame, LOD, and spatial code
    pub fn new(frame: FrameId, lod: u32, code: u64) -> Self {
        let version = 1u64;
        let hi = (version << 56)
               | ((frame.0 as u64) << 48)
               | (lod as u64);

        Self { hi, lo: code }
    }

    /// Extract version number
    pub fn version(&self) -> u8 {
        (self.hi >> 56) as u8
    }

    /// Extract frame identifier
    pub fn frame(&self) -> FrameId {
        FrameId(((self.hi >> 48) & 0xFF) as u8)
    }

    /// Extract level of detail
    pub fn lod(&self) -> u32 {
        (self.hi & 0xFF_FFFF) as u32
    }

    /// Extract spatial code
    pub fn code(&self) -> u64 {
        self.lo
    }

    /// Create from Index64 with frame context
    pub fn from_index64(frame: FrameId, idx: Index64) -> Self {
        Self::new(frame, idx.lod() as u32, idx.to_morton())
    }

    /// Try to extract Index64 (loses frame information)
    pub fn to_index64(&self) -> Result<Index64, ConversionError> {
        let lod = self.lod();
        if lod > u8::MAX as u32 {
            return Err(ConversionError::LodOutOfRange);
        }

        Index64::from_morton(self.lo, lod as u8)
    }
}

/// Frame identifier
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameId(pub u8);

impl FrameId {
    pub const WGS84: FrameId = FrameId(1);
    pub const ECEF: FrameId = FrameId(2);
    pub const MISSION_A: FrameId = FrameId(64);  // User-defined start at 64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionError {
    LodOutOfRange,
    UnsupportedFrame,
    InvalidEncoding,
}
```

### 5.2.3 Binary Serialization

For stable, cross-platform storage:

```rust
use std::io::{Read, Write};

impl Galactic128 {
    /// Serialize to big-endian bytes (for stable wire format)
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&self.hi.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.lo.to_be_bytes());
        bytes
    }

    /// Deserialize from big-endian bytes
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        let hi = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let lo = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
        Self { hi, lo }
    }

    /// Write to a stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes)
    }

    /// Read from a stream
    pub fn read_from<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0u8; 16];
        reader.read_exact(&mut bytes)?;
        Ok(Self::from_bytes(bytes))
    }
}
```

### 5.2.4 Display and Debugging

```rust
use std::fmt;

impl fmt::Debug for Galactic128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Galactic128")
            .field("version", &self.version())
            .field("frame", &self.frame())
            .field("lod", &self.lod())
            .field("code", &format_args!("0x{:016x}", self.code()))
            .finish()
    }
}

impl fmt::Display for Galactic128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Galactic128(v{}, frame={:?}, lod={}, code=0x{:x})",
            self.version(),
            self.frame(),
            self.lod(),
            self.code()
        )
    }
}
```

**Intended use cases**:

- Long-term storage of "where" data in logging systems.
- Cross-service communication where frame context must not be lost.
- Cross-mission datasets where future readers may not have access to original application code.

`Galactic128` is deliberately **heavier** than other identifiers. It is not the best choice for inner loops, but it is the right choice for boundaries between subsystems.

### 5.2.5 Example Usage

```rust
use octaindex::{Galactic128, Index64, FrameId};

// Create from continuous coordinates in WGS84 frame
let (lat, lon, alt) = (37.7749, -122.4194, 100.0);  // San Francisco
let idx = Index64::from_wgs84(lat, lon, alt, 15)?;
let gal = Galactic128::from_index64(FrameId::WGS84, idx);

// Serialize for storage
let bytes = gal.to_bytes();
std::fs::write("location.bin", &bytes)?;

// Deserialize
let bytes = std::fs::read("location.bin")?;
let loaded = Galactic128::from_bytes(bytes.try_into().unwrap());

assert_eq!(gal, loaded);
assert_eq!(loaded.frame(), FrameId::WGS84);
assert_eq!(loaded.lod(), 15);
```

---

## 5.3 `Index64`: Morton-Encoded Spatial Queries

`Index64` is the workhorse identifier for **fast spatial queries**. It is:

- Exactly 64 bits wide.
- Designed for dense storage and fast hashing.
- Encoded using a BCC-specific Morton (Z-order) scheme.

### 5.3.1 Structure and Semantics

At a conceptual level, an `Index64` consists of:

- A **Level of Detail (LOD)** field (5 bits, supporting LODs 0-31).
- Interleaved bits for the BCC lattice coordinates `(x, y, z)` (59 bits total).

**Bitfield layout:**

```text
┌──────────────────────────────────────────────────────────────┐
│  Bits 63-59  │             Bits 58-0                         │
│     LOD      │      Morton-encoded (x, y, z)                 │
└──────────────────────────────────────────────────────────────┘
```

The details of BCC-specific Morton encoding are covered in Chapter 3; here we focus on architectural consequences:

- Neighboring cells in space tend to be close in Morton order.
- Identifiers can be compared as integers to approximate spatial locality.
- Range scans over `Index64` values can serve as a cheap spatial pre-filter.

### 5.3.2 Implementation

OctaIndex3D exposes `Index64` as an opaque newtype:

```rust
/// 64-bit Morton-encoded BCC lattice identifier
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Index64(u64);

impl Index64 {
    const LOD_BITS: u32 = 5;
    const LOD_MASK: u64 = 0x1F;
    const LOD_SHIFT: u32 = 59;
    const CODE_MASK: u64 = (1u64 << 59) - 1;

    /// Encode BCC coordinates at given LOD
    pub fn encode(x: i32, y: i32, z: i32, lod: u8) -> Result<Self, EncodingError> {
        // Validate BCC parity
        if (x + y + z) % 2 != 0 {
            return Err(EncodingError::InvalidParity);
        }

        // Validate LOD range
        if lod > 31 {
            return Err(EncodingError::LodOutOfRange);
        }

        // Convert to unsigned for Morton encoding
        let morton = morton_encode(x as u32, y as u32, z as u32);

        // Pack LOD in upper bits
        let packed = ((lod as u64) << Self::LOD_SHIFT) | (morton & Self::CODE_MASK);

        Ok(Self(packed))
    }

    /// Decode to BCC coordinates and LOD
    pub fn decode(&self) -> (i32, i32, i32, u8) {
        let lod = self.lod();
        let morton = self.0 & Self::CODE_MASK;
        let (x, y, z) = morton_decode(morton);
        (x as i32, y as i32, z as i32, lod)
    }

    /// Extract level of detail
    pub fn lod(&self) -> u8 {
        ((self.0 >> Self::LOD_SHIFT) & Self::LOD_MASK) as u8
    }

    /// Get raw Morton code (without LOD)
    pub fn to_morton(&self) -> u64 {
        self.0 & Self::CODE_MASK
    }

    /// Construct from Morton code and LOD
    pub fn from_morton(morton: u64, lod: u8) -> Result<Self, EncodingError> {
        if lod > 31 {
            return Err(EncodingError::LodOutOfRange);
        }

        let packed = ((lod as u64) << Self::LOD_SHIFT) | (morton & Self::CODE_MASK);
        Ok(Self(packed))
    }

    /// Get parent cell (one LOD coarser)
    pub fn parent(&self) -> Option<Self> {
        let lod = self.lod();
        if lod == 0 {
            return None;
        }

        // Parent is at LOD-1 with coordinates divided by 2
        let (x, y, z, _) = self.decode();
        Self::encode(x / 2, y / 2, z / 2, lod - 1).ok()
    }

    /// Get 8 child cells (one LOD finer)
    pub fn children(&self) -> Result<[Self; 8], EncodingError> {
        let lod = self.lod();
        if lod >= 31 {
            return Err(EncodingError::LodOutOfRange);
        }

        let (x, y, z, _) = self.decode();
        let child_lod = lod + 1;

        // 8 children at 2x coordinates + offsets
        let base_x = x * 2;
        let base_y = y * 2;
        let base_z = z * 2;

        Ok([
            Self::encode(base_x, base_y, base_z, child_lod)?,
            Self::encode(base_x + 2, base_y, base_z, child_lod)?,
            Self::encode(base_x, base_y + 2, base_z, child_lod)?,
            Self::encode(base_x + 2, base_y + 2, base_z, child_lod)?,
            Self::encode(base_x, base_y, base_z + 2, child_lod)?,
            Self::encode(base_x + 2, base_y, base_z + 2, child_lod)?,
            Self::encode(base_x, base_y + 2, base_z + 2, child_lod)?,
            Self::encode(base_x + 2, base_y + 2, base_z + 2, child_lod)?,
        ])
    }

    /// Get 12 face-adjacent neighbors (BCC lattice)
    pub fn neighbors_12(&self) -> Result<[Self; 12], EncodingError> {
        let (x, y, z, lod) = self.decode();

        // BCC has 12 face neighbors at distance sqrt(2)
        const OFFSETS: [(i32, i32, i32); 12] = [
            (2, 0, 0), (-2, 0, 0),
            (0, 2, 0), (0, -2, 0),
            (0, 0, 2), (0, 0, -2),
            (1, 1, 0), (-1, -1, 0),
            (1, 0, 1), (-1, 0, -1),
            (0, 1, 1), (0, -1, -1),
        ];

        let mut neighbors = [Self(0); 12];
        for (i, &(dx, dy, dz)) in OFFSETS.iter().enumerate() {
            neighbors[i] = Self::encode(x + dx, y + dy, z + dz, lod)?;
        }

        Ok(neighbors)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingError {
    InvalidParity,
    LodOutOfRange,
    CoordinateOverflow,
}

// Morton encoding helpers (from Chapter 7)
fn morton_encode(x: u32, y: u32, z: u32) -> u64 {
    // Implementation from Chapter 7.2
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if std::arch::is_x86_feature_detected!("bmi2") {
            morton_encode_bmi2(x, y, z)
        } else {
            morton_encode_fallback(x, y, z)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    morton_encode_fallback(x, y, z)
}

fn morton_decode(morton: u64) -> (u32, u32, u32) {
    // Implementation from Chapter 7.2
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if std::arch::is_x86_feature_detected!("bmi2") {
            morton_decode_bmi2(morton)
        } else {
            morton_decode_fallback(morton)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    morton_decode_fallback(morton)
}
```

### 5.3.3 Ord impl for Spatial Locality

The `Ord` implementation leverages Morton ordering:

```rust
impl Ord for Index64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Index64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
```

This allows:
- Sorting identifiers to cluster spatially nearby cells
- Binary search on sorted arrays
- B-tree indexing that respects spatial locality

### 5.3.4 Example Usage

```rust
use octaindex::Index64;

// Encode a BCC lattice point
let idx = Index64::encode(100, 100, 0, 10)?;
assert_eq!(idx.lod(), 10);

// Navigate hierarchy
let parent = idx.parent().unwrap();
assert_eq!(parent.lod(), 9);

let children = idx.children()?;
assert_eq!(children.len(), 8);
assert!(children.iter().all(|c| c.lod() == 11));

// Get neighbors
let neighbors = idx.neighbors_12()?;
assert_eq!(neighbors.len(), 12);

// Decode back to coordinates
let (x, y, z, lod) = idx.decode();
assert_eq!((x, y, z, lod), (100, 100, 0, 10));
```

### 5.3.5 When to Use `Index64`

Choose `Index64` when:

- You need high-throughput spatial queries.
- Your workload is dominated by nearest-neighbor, range, or traversal operations.
- You are implementing containers and cache-friendly data structures.
- Performance is more important than frame metadata.

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

## 5.8 Performance Characteristics and Trade-offs

Choosing an identifier type is ultimately a performance and ergonomics decision. While Chapter 7 will dive into microbenchmarks, it is helpful to have a high-level mental model here.

At a coarse level, you can think in terms of three metrics:

- **Key width**: how many bits/bytes does the identifier occupy?
- **CPU cost**: how expensive are common operations (comparisons, neighbor lookups, conversions)?
- **Locality / scan quality**: how well does the ordering preserve spatial locality when stored in arrays or B-trees?

The table below summarizes typical trade-offs:

| Identifier  | Width | Encoding    | CPU Cost (per op) | Locality for Scans | Typical Use Case                          |
|------------|-------|------------|--------------------|--------------------|-------------------------------------------|
| `Index64`  | 64b   | Morton BCC | Very low           | Good               | General-purpose spatial indexing          |
| `Route64`  | 64b   | Morton + route bits | Low      | Good               | Path planning, local routing              |
| `Hilbert64`| 64b   | Hilbert BCC| Moderate           | Very good          | Scan-heavy analytics, cache-sensitive R/W |
| `Galactic128` | 128b | Frame + 64b index | Higher  | Good               | Global storage, cross-system integration  |

Notes:

- “CPU Cost” is relative and assumes well-optimized bit-twiddling implementations on modern CPUs.
- “Locality for Scans” is a qualitative measure; exact behavior depends on data distributions and access patterns.
- Actual performance numbers are workload- and hardware-dependent; always validate with benchmarks.

Architecturally, the recommended pattern is:

1. Use `Galactic128` at **system boundaries** (storage, logs, cross-service APIs).  
2. Convert to `Index64` or `Hilbert64` at **computation boundaries** (query engines, in-memory containers).  
3. Use `Route64` in **algorithm-specific internals** where routing context matters.  

This separation allows the codebase to evolve in each dimension without forcing a one-size-fits-all identifier.

---

## 5.9 Summary

In this chapter, we examined the portfolio of identifier types that OctaIndex3D uses to represent locations in BCC lattices:

- `Galactic128` provides **global, frame-aware addressing** suitable for long-term storage and cross-system integration.
- `Index64` serves as the **fast, Morton-encoded workhorse** for spatial queries and containers.
- `Route64` augments local indices with **routing context**, enabling compact representation of paths and frontiers.
- `Hilbert64` offers a **Hilbert-encoded alternative** with improved locality for scan-heavy workloads.

We also saw how:

- Explicit **conversions** maintain correctness across frames and encodings.
- **Human-readable encodings** make debugging and interoperability practical without sacrificing type safety.

Together, these identifier types form the connective tissue between the architectural concepts of Part II and the concrete implementation techniques explored in Part III.

---

## Further Reading

### Morton and Hilbert Encoding

- **"An Inventory of Three-Dimensional Hilbert Space-Filling Curves"**
  Jinjun Chen et al., 2007
  Technical analysis of 3D Hilbert curve variants.

- **"Z-order (curve)"** — Wikipedia
  <https://en.wikipedia.org/wiki/Z-order_curve>
  Overview of Morton ordering and applications.

- **"Fast Generation of Morton/Hilbert Codes"**
  Various authors
  Bit-manipulation techniques for space-filling curves.

### Spatial Index Design

- **"R-trees: A Dynamic Index Structure for Spatial Searching"**
  Antonin Guttman, 1984
  Classic spatial index structure for comparison.

- **"The Ubiquitous B-Tree"**
  Douglas Comer, ACM Computing Surveys, 1979
  Foundation for understanding tree-based spatial indices.

- **"Multidimensional Binary Search Trees Used for Associative Searching"**
  Jon Louis Bentley, 1975
  k-d trees and spatial partitioning strategies.

### Geospatial Standards

- **"WGS84 Coordinate System"**
  National Geospatial-Intelligence Agency
  <https://earth-info.nga.mil/index.php?dir=wgs84&action=wgs84>
  Official specification for global geodetic reference.

- **"OGC Standards"** — Open Geospatial Consortium
  <https://www.ogc.org/standards>
  Industry standards for geospatial interoperability.

- **"Bech32m Address Format"** (Bitcoin BIP 173/350)
  Inspiration for human-readable checksummed encodings.

### Implementation Techniques

- **"Bit Twiddling Hacks"**
  Sean Eron Anderson, Stanford
  <https://graphics.stanford.edu/~seander/bithacks.html>
  Bit manipulation techniques used in encoding/decoding.

- **"The Rust Performance Book"**
  <https://nnethercote.github.io/perf-book/>
  Performance optimization for Rust code (relevant for Chapter 7 techniques).

### Case Studies

- **"S2 Geometry"** — Google
  <https://s2geometry.io/>
  Hierarchical spherical indexing system for Earth-scale data.

- **"H3: Uber's Hexagonal Hierarchical Spatial Index"**
  <https://h3geo.org/>
  Alternative grid system with hexagonal cells.

- **"Geohash"**
  <https://en.wikipedia.org/wiki/Geohash>
  String-based hierarchical spatial index for comparison.
