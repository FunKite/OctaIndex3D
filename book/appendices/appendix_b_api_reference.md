# Appendix B: Complete API Reference

This appendix provides a high-level overview of the OctaIndex3D API surface. It is not a replacement for generated documentation, but serves as a conceptual map of the most important modules and types.

Topics include:

- B.1 Core Identifier Types
- B.2 Frame Registry and Coordinate Systems
- B.3 Container Types and Interfaces
- B.4 Query Operations
- B.5 Error Handling

---

## B.1 Core Identifier Types

### B.1.1 Index64

**Description**: Basic 64-bit BCC cell identifier using Morton encoding.

**Representation**:
```rust
pub struct Index64 {
    raw: u64,  // Packed Morton code
}
```rust

**Key Methods**:

```rust
impl Index64 {
    /// Create from raw Morton code
    pub fn from_raw(raw: u64) -> Self;

    /// Create from lattice coordinates (i, j, k) and LOD
    pub fn from_coords(i: i32, j: i32, k: i32, lod: u8) -> Self;

    /// Decode to lattice coordinates
    pub fn to_coords(self) -> (i32, i32, k32, u8);

    /// Get Morton code
    pub fn morton_code(self) -> u64;

    /// Get LOD (level of detail)
    pub fn lod(self) -> u8;

    /// Get 14 BCC neighbors
    pub fn neighbors(self) -> [Index64; 14];

    /// Get parent cell (one LOD coarser)
    pub fn parent(self) -> Index64;

    /// Get children cells (one LOD finer)
    pub fn children(self) -> Vec<Index64>;
}
```

**Example**:
```rust
let idx = Index64::from_coords(10, 20, 30, 3);
let morton = idx.morton_code();
let (i, j, k, lod) = idx.to_coords();
let neighbors = idx.neighbors();
```rust

### B.1.2 Galactic128

**Description**: 128-bit identifier supporting galactic-scale coordinates and deeper LOD hierarchies.

**Representation**:
```rust
pub struct Galactic128 {
    hi: u64,  // High 64 bits
    lo: u64,  // Low 64 bits
}
```

**Key Methods**:
```rust
impl Galactic128 {
    pub fn from_coords(i: i64, j: i64, k: i64, lod: u16) -> Self;
    pub fn to_coords(self) -> (i64, i64, i64, u16);
    pub fn morton_code(self) -> u128;
    pub fn neighbors(self) -> [Galactic128; 14];
}
```rust

### B.1.3 Hilbert64

**Description**: 64-bit identifier using Hilbert curve encoding for improved spatial locality.

**Key Methods**:
```rust
impl Hilbert64 {
    pub fn from_coords(i: i32, j: i32, k: i32, lod: u8) -> Self;
    pub fn to_coords(self) -> (i32, i32, i32, u8);
    pub fn hilbert_index(self) -> u64;
    pub fn neighbors(self) -> [Hilbert64; 14];
}
```

### B.1.4 Route64

**Description**: Route-based identifier optimized for network and distributed systems.

**Key Methods**:
```rust
impl Route64 {
    pub fn from_coords(i: i32, j: i32, k: i32, lod: u8) -> Self;
    pub fn encode_route(path: &[Direction]) -> Self;
    pub fn decode_route(self) -> Vec<Direction>;
}
```rust

---

## B.2 Frame Registry and Coordinate Systems

### B.2.1 FrameRegistry

**Description**: Manages coordinate reference systems and transformations.

**Core Type**:
```rust
pub struct FrameRegistry {
    frames: HashMap<FrameId, Frame>,
}

pub struct Frame {
    pub id: FrameId,
    pub origin: Vec3,
    pub orientation: Quat,
    pub parent: Option<FrameId>,
}
```

**Key Methods**:
```rust
impl FrameRegistry {
    /// Create new registry
    pub fn new() -> Self;

    /// Register a new frame
    pub fn register_frame(
        &mut self,
        id: FrameId,
        origin: Vec3,
        orientation: Quat,
        parent: Option<FrameId>,
    ) -> Result<()>;

    /// Transform position between frames
    pub fn transform(
        &self,
        pos: Vec3,
        from: &FrameId,
        to: &FrameId,
    ) -> Result<Vec3>;

    /// Convert world position to BCC index
    pub fn world_to_index(
        &self,
        pos: Vec3,
        frame: &FrameId,
        lod: u8,
    ) -> Result<Index64>;

    /// Convert BCC index to world position
    pub fn index_to_world(
        &self,
        idx: Index64,
        frame: &FrameId,
    ) -> Result<Vec3>;
}
```rust

**Example**:
```rust
let mut registry = FrameRegistry::new();

registry.register_frame(
    "world",
    Vec3::zero(),
    Quat::identity(),
    None,
)?;

registry.register_frame(
    "robot",
    Vec3::new(10.0, 0.0, 0.0),
    Quat::from_euler(0.0, 0.0, PI/4.0),
    Some("world"),
)?;

let world_pos = registry.transform(
    Vec3::new(1.0, 0.0, 0.0),
    &"robot",
    &"world",
)?;
```

---

## B.3 Container Types and Interfaces

### B.3.1 Container Trait

**Description**: Common interface for all BCC containers.

```rust
pub trait Container<V> {
    type Index: Identifier;

    /// Insert value at index
    fn insert(&mut self, idx: Self::Index, value: V) -> Result<()>;

    /// Get value at index
    fn get(&self, idx: Self::Index) -> Option<&V>;

    /// Remove value at index
    fn remove(&mut self, idx: Self::Index) -> Option<V>;

    /// Iterate over all entries
    fn iter(&self) -> impl Iterator<Item = (Self::Index, &V)>;

    /// Query indices in range
    fn query_range(
        &self,
        min: Self::Index,
        max: Self::Index,
    ) -> Vec<Self::Index>;
}
```rust

### B.3.2 SequentialContainer

**Description**: Optimized for bulk storage with spatial locality.

```rust
pub struct SequentialContainer<V> {
    blocks: Vec<Block<V>>,
    index: BTreeMap<u64, usize>,  // Morton code -> block index
}

impl<V> SequentialContainer<V> {
    pub fn new() -> Self;
    pub fn open(path: &str) -> Result<Self>;
    pub fn save(&self, path: &str) -> Result<()>;

    /// Query cells within sphere
    pub fn query_sphere(
        &self,
        center: Vec3,
        radius: f32,
        lod: u8,
    ) -> Vec<Index64>;

    /// Query cells within box
    pub fn query_box(
        &self,
        min: Vec3,
        max: Vec3,
        lod: u8,
    ) -> Vec<Index64>;
}
```

### B.3.3 StreamingContainer

**Description**: Low-latency streaming for real-time applications.

```rust
pub struct StreamingContainer<V> {
    buffer: VecDeque<(Index64, V)>,
    capacity: usize,
}

impl<V> StreamingContainer<V> {
    pub fn new(capacity: usize) -> Self;
    pub fn push(&mut self, idx: Index64, value: V);
    pub fn flush(&mut self) -> Vec<(Index64, V)>;
}
```rust

### B.3.4 HashContainer

**Description**: Fast random access using hash map.

```rust
pub struct HashContainer<V> {
    cells: HashMap<Index64, V>,
}

impl<V> HashContainer<V> {
    pub fn new() -> Self;
    pub fn with_capacity(capacity: usize) -> Self;
}
```

---

## B.4 Query Operations

### B.4.1 Neighbor Queries

```rust
/// Get immediate neighbors (14 for BCC)
pub fn get_neighbors<V>(
    container: &impl Container<V>,
    idx: Index64,
) -> Vec<(Index64, &V)>;

/// Get neighbors within radius
pub fn get_neighbors_in_radius<V>(
    container: &impl Container<V>,
    idx: Index64,
    radius: f32,
) -> Vec<(Index64, &V)>;

/// Get neighbors at specific LOD
pub fn get_neighbors_at_lod<V>(
    container: &impl Container<V>,
    idx: Index64,
    target_lod: u8,
) -> Vec<(Index64, &V)>;
```rust

### B.4.2 Range Queries

```rust
/// Query by bounding box
pub fn query_aabb<V>(
    container: &impl Container<V>,
    min: Vec3,
    max: Vec3,
    lod: u8,
) -> Vec<(Index64, &V)>;

/// Query by sphere
pub fn query_sphere<V>(
    container: &impl Container<V>,
    center: Vec3,
    radius: f32,
    lod: u8,
) -> Vec<(Index64, &V)>;

/// Query by frustum (for rendering)
pub fn query_frustum<V>(
    container: &impl Container<V>,
    frustum: &Frustum,
    lod: u8,
) -> Vec<(Index64, &V)>;
```

### B.4.3 Aggregation Operations

```rust
/// Sum values in region
pub fn sum_region<V: Add<Output = V> + Copy>(
    container: &impl Container<V>,
    region: &Region,
) -> V;

/// Average values in region
pub fn avg_region(
    container: &impl Container<f32>,
    region: &Region,
) -> f32;

/// Max value in region
pub fn max_region<V: Ord + Copy>(
    container: &impl Container<V>,
    region: &Region,
) -> Option<V>;
```rust

---

## B.5 Error Handling

### B.5.1 Error Types

```rust
#[derive(Debug, Error)]
pub enum OctaIndexError {
    #[error("Invalid coordinates: ({0}, {1}, {2})")]
    InvalidCoordinates(i32, i32, i32),

    #[error("Invalid LOD: {0} (must be 0-63)")]
    InvalidLOD(u8),

    #[error("Frame not found: {0}")]
    FrameNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Container corrupted: {0}")]
    CorruptedContainer(String),
}

pub type Result<T> = std::result::Result<T, OctaIndexError>;
```

### B.5.2 Error Recovery

```rust
/// Validate container integrity
pub fn validate_container(path: &str) -> Result<ValidationReport>;

/// Attempt recovery from corrupted container
pub fn recover_container(
    damaged: &str,
    output: &str,
) -> Result<RecoveryStats>;
```rust

---

## B.6 Common Usage Patterns

### B.6.1 Basic Workflow

```rust
use octaindex3d::*;

fn main() -> Result<()> {
    // Create frame registry
    let mut frames = FrameRegistry::new();
    frames.register_frame("world", Vec3::zero(), Quat::identity(), None)?;

    // Create container
    let mut container = SequentialContainer::<f32>::new();

    // Insert data
    for i in 0..100 {
        for j in 0..100 {
            for k in 0..100 {
                let idx = Index64::from_coords(i, j, k, 0);
                container.insert(idx, (i + j + k) as f32)?;
            }
        }
    }

    // Query
    let results = container.query_sphere(
        Vec3::new(50.0, 50.0, 50.0),
        10.0,
        0,
    );

    println!("Found {} cells in sphere", results.len());

    // Save
    container.save("data.bcc")?;

    Ok(())
}
```

### B.6.2 Multi-Resolution Pattern

```rust
// Maintain multiple LODs
let mut coarse = SequentialContainer::new();  // LOD 5
let mut fine = SequentialContainer::new();     // LOD 2

// Query coarse first, refine selectively
let coarse_hits = coarse.query_box(min, max, 5);
for coarse_idx in coarse_hits {
    if should_refine(coarse_idx) {
        // Query children at finer LOD
        for child in coarse_idx.children() {
            if let Some(value) = fine.get(child) {
                process(child, value);
            }
        }
    }
}
```bash

---

## B.7 Further Documentation

For complete, auto-generated API documentation:
- **Rust docs**: Run `cargo doc --open` in the OctaIndex3D repository
- **Online docs**: https://docs.rs/octaindex3d
- **Examples**: https://github.com/octaindex3d/octaindex3d/tree/main/examples

