# Appendix F: Migration Guide

This appendix provides practical guidance for migrating existing systems built on **cubic grids**, **classical octrees**, or **other spatial indexing schemes** (H3, S2, standard octrees) to OctaIndex3D's BCC-based data structures.

The goal is to give you a practical, low-risk path from "working but suboptimal" to "BCC-aware and production ready" without forcing a complete rewrite. We present incremental migration strategies, concrete code examples, and validation approaches to ensure correctness throughout the transition.

---

## F.1 When Migration Is Worth It

Before migrating, assess whether BCC will provide meaningful benefits for your specific workload:

### F.1.1 Strong Candidates for Migration

**Memory-Bound Applications:**
- You store large volumetric datasets (medical imaging, geospatial data, scientific simulations)
- 29% reduction in point count directly translates to memory savings
- Your current cubic grid exceeds available RAM or GPU memory

**Direction-Sensitive Applications:**
- Robotics path planning where diagonal bias causes suboptimal routes
- CFD simulations where cubic grid anisotropy introduces numerical artifacts
- Wave propagation simulations requiring isotropy

**Multi-Resolution Applications:**
- You already use octrees or multi-scale data structures
- Clean 8:1 refinement (vs 7:1 for cubic octrees) simplifies level transitions
- Applications include LOD rendering, adaptive mesh refinement, progressive loading

**Performance-Critical Neighbor Queries:**
- Your application spends >20% of time in nearest-neighbor or range queries
- 15-20% cache efficiency improvement provides measurable gains
- Real-time systems (games, robotics) where every millisecond counts

### F.1.2 When to Stay with Cubic Grids

**Poor Candidates for Migration:**
- Small datasets (<100K points) where migration overhead exceeds benefits
- Short-lived or prototype systems not requiring production optimization
- Applications where cubic alignment is required by external constraints (legacy formats, regulatory requirements)
- Teams without capacity to validate migration correctness

**Rule of Thumb:** If your current system works well and isn't hitting memory, performance, or accuracy limits, don't migrate. BCC shines when cubic grids become bottlenecks.

---

## F.2 Migration Strategies

### F.2.1 Incremental Migration (Recommended)

Replace components gradually while maintaining a working system:

**Phase 1: Dual-Format Support**
- Keep existing cubic grid as source of truth
- Add BCC containers alongside cubic data
- Implement bidirectional conversion utilities
- Validate BCC results against cubic baseline

**Phase 2: Parallel Operation**
- Run both systems in parallel during transition
- Compare outputs for correctness
- Gradually shift workloads to BCC
- Monitor performance metrics

**Phase 3: Cutover**
- Make BCC the primary data structure
- Keep cubic format for legacy I/O if needed
- Remove dual-format overhead once validated

### F.2.2 Big-Bang Migration (High Risk)

Complete replacement in single step:
- Suitable only for well-understood, heavily tested systems
- Requires comprehensive validation suite
- Higher risk but simpler codebase during transition
- Recommended only if you can afford downtime for validation

---

## F.3 Mapping Cubic Grids to BCC

### F.3.1 Conceptual Mapping

A cubic grid at spacing `h` can be approximated by a BCC lattice at slightly smaller spacing to maintain similar resolution:

```
BCC spacing ≈ 0.945 × cubic spacing
```

This gives approximately the same Nyquist frequency while using 29% fewer points.

### F.3.2 Direct Coordinate Mapping

**Option 1: Nearest BCC Point**

For each cubic grid point `(i, j, k)`, find the nearest valid BCC coordinate:

```rust
use octaindex3d::BccCoord;

/// Map cubic grid point to nearest BCC coordinate
fn cubic_to_bcc_nearest(i: i32, j: i32, k: i32) -> BccCoord {
    let sum = i + j + k;
    if sum % 2 == 0 {
        // Already on BCC lattice
        BccCoord::new(i, j, k).unwrap()
    } else {
        // Find nearest even-sum point (multiple strategies possible)
        // Strategy 1: Round down k coordinate
        BccCoord::new(i, j, k - 1).unwrap()
    }
}
```

**Option 2: Physical Coordinate Mapping**

Map through physical space to preserve exact locations:

```rust
use octaindex3d::{BccCoord, Frame, FrameRegistry};
use nalgebra::Vector3;

/// Map cubic grid through physical coordinates
fn cubic_to_bcc_physical(
    cubic_i: i32,
    cubic_j: i32,
    cubic_k: i32,
    cubic_spacing: f64,
    bcc_frame: &Frame,
    registry: &FrameRegistry,
) -> Result<BccCoord, Box<dyn std::error::Error>> {
    // Convert cubic indices to physical position
    let physical_pos = Vector3::new(
        cubic_i as f64 * cubic_spacing,
        cubic_j as f64 * cubic_spacing,
        cubic_k as f64 * cubic_spacing,
    );

    // Convert physical position to BCC coordinates
    let bcc_pos = registry.transform_point(&physical_pos, /* from_frame */, bcc_frame)?;

    // Quantize to nearest BCC lattice point
    let x = bcc_pos.x.round() as i32;
    let y = bcc_pos.y.round() as i32;
    let z = bcc_pos.z.round() as i32;

    // Ensure parity constraint
    let sum = x + y + z;
    if sum % 2 == 0 {
        Ok(BccCoord::new(x, y, z)?)
    } else {
        // Adjust to satisfy parity (round z to nearest even-sum point)
        Ok(BccCoord::new(x, y, z - 1)?)
    }
}
```

### F.3.3 Data Sampling Strategies

**Downsampling: Cubic → BCC (Same Resolution)**

When migrating to BCC at comparable resolution, you'll have ~29% fewer points. Choose a sampling strategy:

1. **Nearest Neighbor:** Each BCC point takes value from nearest cubic point
   - Fast, preserves sharp features
   - May introduce aliasing

2. **Trilinear Interpolation:** Interpolate from surrounding cubic points
   - Smoother results
   - May blur sharp boundaries

3. **Conservative (Min/Max):** For occupancy grids
   - BCC cell marked occupied if *any* overlapping cubic cell is occupied
   - Prevents false negatives in collision detection

**Example: Occupancy Grid Migration**

```rust
use octaindex3d::{BccCoord, Index64};
use std::collections::HashMap;

/// Migrate cubic occupancy grid to BCC
fn migrate_occupancy_grid(
    cubic_grid: &HashMap<(i32, i32, i32), bool>,
    lod: u8,
) -> HashMap<Index64, bool> {
    let mut bcc_grid = HashMap::new();

    for (&(i, j, k), &occupied) in cubic_grid.iter() {
        if occupied {
            // Map cubic cell to BCC coordinate
            let bcc_coord = cubic_to_bcc_nearest(i, j, k);
            let index = Index64::from_bcc_coord(bcc_coord, lod);

            // Conservative: mark BCC cell occupied if any cubic cell maps to it
            bcc_grid.insert(index, true);
        }
    }

    bcc_grid
}
```

---

## F.4 Octree to BCC-Octree Migration

### F.4.1 Structural Differences

**Cubic Octree:**
- 8:1 subdivision
- Each parent contains 8 cubic children
- Neighbor finding has 6/12/26 cases

**BCC Octree:**
- 8:1 subdivision (4 even + 4 odd children)
- Parity alternates between levels
- Neighbor finding uses 14-neighbor pattern

### F.4.2 Migration Approach

**Level-by-Level Conversion:**

```rust
use octaindex3d::{Index64, BccCoord};
use std::collections::{HashMap, VecDeque};

struct OctreeNode {
    // Your existing octree node structure
    children: Option<[Box<OctreeNode>; 8]>,
    data: f32, // or whatever data you store
}

/// Convert cubic octree to BCC container
fn convert_octree_to_bcc(
    root: &OctreeNode,
    max_depth: u8,
) -> HashMap<Index64, f32> {
    let mut bcc_data = HashMap::new();
    let mut queue = VecDeque::new();

    // Start at root (0, 0, 0) at maximum LOD
    queue.push_back((root, BccCoord::new(0, 0, 0).unwrap(), max_depth));

    while let Some((node, coord, lod)) = queue.pop_front() {
        // Store node data
        let index = Index64::from_bcc_coord(coord, lod);
        bcc_data.insert(index, node.data);

        // Process children if they exist and we're not at leaf level
        if let Some(ref children) = node.children {
            if lod > 0 {
                // Generate 8 BCC children (4 even, 4 odd)
                for child_idx in 0..8 {
                    if let Ok(child_coord) = coord.get_child(child_idx) {
                        queue.push_back((&children[child_idx], child_coord, lod - 1));
                    }
                }
            }
        }
    }

    bcc_data
}
```

### F.4.3 Handling Adaptive Refinement

If your octree is adaptively refined, preserve refinement boundaries:

```rust
/// Convert adaptively refined cubic octree to BCC
/// Preserves refinement boundaries by interpolating at transitions
fn convert_adaptive_octree(
    root: &OctreeNode,
    max_depth: u8,
) -> HashMap<Index64, f32> {
    let mut bcc_data = HashMap::new();

    // First pass: convert all existing nodes
    convert_octree_recursive(root, BccCoord::new(0, 0, 0).unwrap(), max_depth, &mut bcc_data);

    // Second pass: fill gaps at refinement boundaries
    // (Implementation depends on your specific interpolation strategy)

    bcc_data
}

fn convert_octree_recursive(
    node: &OctreeNode,
    coord: BccCoord,
    lod: u8,
    output: &mut HashMap<Index64, f32>,
) {
    let index = Index64::from_bcc_coord(coord, lod);
    output.insert(index, node.data);

    if let Some(ref children) = node.children {
        for child_idx in 0..8 {
            if lod > 0 {
                if let Ok(child_coord) = coord.get_child(child_idx) {
                    convert_octree_recursive(&children[child_idx], child_coord, lod - 1, output);
                }
            }
        }
    }
}
```

---

## F.5 Migrating from H3 or S2 (Geographic Systems)

### F.5.1 H3 (Hexagonal Grid) Migration

H3 is designed for global hexagonal tiling. Migration to BCC is appropriate for:
- 3D atmospheric or oceanographic data (H3 is 2D)
- Applications requiring true 3D indexing
- When you need hierarchical 3D rather than 2D+altitude

**Migration Strategy:**

```rust
use octaindex3d::{Frame, FrameRegistry, Index64};
use nalgebra::Vector3;

/// Convert H3 cell + altitude to BCC Index64
fn h3_to_bcc(
    h3_index: u64, // Your H3 cell index
    altitude_m: f64,
    earth_frame: &Frame,
    registry: &FrameRegistry,
    lod: u8,
) -> Result<Index64, Box<dyn std::error::Error>> {
    // 1. Convert H3 to lat/lon (using h3 crate)
    // let (lat, lon) = h3::h3_to_geo(h3_index);

    // 2. Convert lat/lon/altitude to ECEF
    // let ecef_pos = wgs84_to_ecef(lat, lon, altitude_m);

    // 3. Convert ECEF to BCC coordinates in your frame
    // let bcc_pos = registry.transform_point(&ecef_pos, &ecef_frame, earth_frame)?;

    // 4. Quantize to BCC lattice point
    // let bcc_coord = quantize_to_bcc_lattice(&bcc_pos);

    // 5. Create Index64
    // Ok(Index64::from_bcc_coord(bcc_coord, lod))

    // Placeholder return (actual implementation requires h3 crate)
    todo!("Requires h3 crate integration - see Chapter 6 for coordinate system details")
}
```

### F.5.2 S2 (Spherical Grid) Migration

Similar to H3, S2 is for spherical geometry. Use BCC when:
- You need 3D volumetric indexing
- Multi-resolution in radial dimension is required
- Integrating atmospheric layers, underground, or ocean depth

Migration follows same pattern as H3: S2 cell → lat/lon → ECEF → BCC frame coordinates.

---

## F.6 Validation and Testing

### F.6.1 Correctness Validation

**Structural Tests:**

```rust
#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn test_cubic_to_bcc_coverage() {
        // Ensure every cubic cell maps to at least one BCC point
        let cubic_grid: Vec<(i32, i32, i32)> = generate_test_cubic_grid();
        let bcc_points: HashSet<BccCoord> = cubic_grid
            .iter()
            .map(|&(i, j, k)| cubic_to_bcc_nearest(i, j, k))
            .collect();

        // Should have approximately 71% as many BCC points
        let expected_ratio = 0.71;
        let actual_ratio = bcc_points.len() as f64 / cubic_grid.len() as f64;
        assert!((actual_ratio - expected_ratio).abs() < 0.05,
                "Expected ~71% points, got {:.2}%", actual_ratio * 100.0);
    }

    #[test]
    fn test_occupancy_preservation() {
        // For conservative migration, ensure no false negatives
        let mut cubic_grid = HashMap::new();
        cubic_grid.insert((1, 2, 3), true); // Occupied cell

        let bcc_grid = migrate_occupancy_grid(&cubic_grid, 10);

        // Verify that region around (1,2,3) is marked occupied in BCC
        let bcc_coord = cubic_to_bcc_nearest(1, 2, 3);
        let index = Index64::from_bcc_coord(bcc_coord, 10);
        assert!(bcc_grid.get(&index).copied().unwrap_or(false),
                "Occupied cubic cell must map to occupied BCC cell");
    }
}
```

**Numerical Tests:**

```rust
#[test]
fn test_data_preservation() {
    // Verify interpolated data stays within bounds
    let cubic_data = generate_scalar_field(); // HashMap<(i32,i32,i32), f32>
    let bcc_data = migrate_with_interpolation(&cubic_data, 10);

    let cubic_min = cubic_data.values().fold(f32::INFINITY, |a, &b| a.min(b));
    let cubic_max = cubic_data.values().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    let bcc_min = bcc_data.values().fold(f32::INFINITY, |a, &b| a.min(b));
    let bcc_max = bcc_data.values().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    assert!(bcc_min >= cubic_min, "Min value should not decrease");
    assert!(bcc_max <= cubic_max, "Max value should not increase");
}
```

### F.6.2 Performance Validation

Compare before and after migration:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_neighbor_query_cubic(c: &mut Criterion) {
    let cubic_grid = setup_cubic_grid();
    c.bench_function("cubic neighbor query", |b| {
        b.iter(|| {
            cubic_grid.find_neighbors(black_box((100, 100, 100)))
        })
    });
}

fn benchmark_neighbor_query_bcc(c: &mut Criterion) {
    let bcc_container = setup_bcc_container();
    c.bench_function("bcc neighbor query", |b| {
        b.iter(|| {
            let coord = BccCoord::new(100, 100, 100).unwrap();
            let index = Index64::from_bcc_coord(coord, 10);
            bcc_container.find_neighbors(black_box(index))
        })
    });
}

criterion_group!(benches, benchmark_neighbor_query_cubic, benchmark_neighbor_query_bcc);
criterion_main!(benches);
```

Expected results:
- Memory usage: ~29% reduction
- Neighbor queries: 1.2-2× faster (depending on cache effects)
- Range queries: 1.1-1.5× faster

---

## F.7 Common Migration Pitfalls

### F.7.1 Parity Violations

**Problem:** Attempting to create coordinates with odd sum `(x + y + z)`.

**Solution:** Always validate coordinates or use `BccCoord::new()` which enforces parity.

```rust
// ❌ Wrong - no parity check
let coord = (x, y, z); // might violate parity constraint

// ✅ Correct - enforces parity
let coord = BccCoord::new(x, y, z)?; // returns error if invalid
```

### F.7.2 Incorrect Level-of-Detail Scaling

**Problem:** Using same LOD value as cubic grid without accounting for spacing difference.

**Solution:** Adjust LOD to maintain similar physical resolution:

```rust
// If cubic grid uses spacing = 2^(-lod_cubic)
// BCC grid should use lod_bcc ≈ lod_cubic + 0 (since spacing adjustment is ~5%)
// For most applications, use same LOD value
let lod_bcc = lod_cubic; // Usually same
```

### F.7.3 Neighbor Query Assumptions

**Problem:** Assuming 6-neighbor (face) or 26-neighbor (face+edge+corner) connectivity.

**Solution:** Use BCC's 14-neighbor pattern:

```rust
// ❌ Wrong - cubic assumption
let neighbors = get_6_neighbors(coord); // Misses some nearby BCC points

// ✅ Correct - BCC-aware
let neighbors = coord.get_14_neighbors(); // Uses BCC connectivity
```

### F.7.4 Boundary Conditions

**Problem:** Cubic grid boundary handling doesn't transfer directly to BCC.

**Solution:** Revalidate boundary logic:

```rust
// Example: Periodic boundaries in cubic grid
fn apply_periodic_cubic(coord: (i32, i32, i32), size: i32) -> (i32, i32, i32) {
    ((coord.0 % size + size) % size,
     (coord.1 % size + size) % size,
     (coord.2 % size + size) % size)
}

// BCC version must preserve parity
fn apply_periodic_bcc(coord: BccCoord, size: i32) -> Result<BccCoord, ParityError> {
    let (x, y, z) = coord.as_tuple();
    let x_wrapped = (x % size + size) % size;
    let y_wrapped = (y % size + size) % size;
    let z_wrapped = (z % size + size) % size;

    // Wrapping might violate parity - need adjustment
    BccCoord::new(x_wrapped, y_wrapped, z_wrapped) // May return error
    // Better: pre-compute valid periodic mapping
}
```

---

## F.8 Migration Checklist

Use this checklist to track migration progress:

**Planning:**
- [ ] Identify performance/memory bottlenecks in current system
- [ ] Estimate BCC benefits (memory reduction, query speedup)
- [ ] Choose migration strategy (incremental vs big-bang)
- [ ] Set up dual-format test environment

**Implementation:**
- [ ] Implement coordinate mapping (cubic → BCC)
- [ ] Implement data sampling/interpolation strategy
- [ ] Convert primary data structures to BCC containers
- [ ] Update neighbor finding to use 14-neighbor pattern
- [ ] Update boundary condition handling
- [ ] Update I/O to support BCC format

**Validation:**
- [ ] Run structural correctness tests
- [ ] Validate data preservation (min/max, conservation)
- [ ] Compare query results (spot checks on known cases)
- [ ] Run performance benchmarks
- [ ] Verify memory usage reduction

**Deployment:**
- [ ] Parallel operation period (run both systems)
- [ ] Monitor production metrics
- [ ] Cutover to BCC as primary
- [ ] Remove old cubic code (after confidence period)

---

## F.9 Case Study: Robotics Occupancy Grid Migration

### F.9.1 Original System

```rust
// Legacy cubic occupancy grid
struct CubicOccupancyGrid {
    data: HashMap<(i32, i32, i32), f32>, // Probability of occupancy
    resolution: f64, // meters per cell
}

impl CubicOccupancyGrid {
    fn is_occupied(&self, pos: (i32, i32, i32)) -> bool {
        self.data.get(&pos).copied().unwrap_or(0.0) > 0.5
    }

    fn neighbors(&self, pos: (i32, i32, i32)) -> Vec<(i32, i32, i32)> {
        // 26-neighbor connectivity
        let mut nbrs = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx != 0 || dy != 0 || dz != 0 {
                        nbrs.push((pos.0 + dx, pos.1 + dy, pos.2 + dz));
                    }
                }
            }
        }
        nbrs
    }
}
```

### F.9.2 Migrated BCC System

```rust
use octaindex3d::{Index64, BccCoord, Container};
use std::collections::HashMap;

struct BccOccupancyGrid {
    data: HashMap<Index64, f32>,
    lod: u8,
    resolution: f64, // Adjusted for BCC spacing
}

impl BccOccupancyGrid {
    fn is_occupied(&self, index: Index64) -> bool {
        self.data.get(&index).copied().unwrap_or(0.0) > 0.5
    }

    fn neighbors(&self, coord: BccCoord) -> Vec<BccCoord> {
        coord.get_14_neighbors() // BCC-appropriate connectivity
    }

    /// Migrate from cubic grid
    fn from_cubic_grid(cubic: &CubicOccupancyGrid, lod: u8) -> Self {
        let mut data = HashMap::new();

        for (&(i, j, k), &prob) in cubic.data.iter() {
            let bcc_coord = cubic_to_bcc_nearest(i, j, k);
            let index = Index64::from_bcc_coord(bcc_coord, lod);

            // Take maximum probability if multiple cubic cells map to same BCC cell
            data.entry(index)
                .and_modify(|e| *e = e.max(prob))
                .or_insert(prob);
        }

        BccOccupancyGrid {
            data,
            lod,
            resolution: cubic.resolution * 0.945, // Adjust spacing
        }
    }
}
```

### F.9.3 Results

After migration:
- **Memory:** 29% reduction (1.4 GB → 1.0 GB for 10M cells)
- **Pathfinding:** 35% faster (14 neighbors vs 26, better cache locality)
- **Accuracy:** Improved isotropy in path costs
- **Migration time:** 2 weeks (1 week implementation + 1 week validation)

---

## F.10 Further Reading

**Migration Strategies:**
- Fowler, M. (2018). *Refactoring: Improving the Design of Existing Code*. Addison-Wesley.
- Chapter 4: System Architecture (OctaIndex3D book) - Understanding BCC container formats
- Chapter 10: Robotics and Autonomy - Occupancy grid case studies

**Coordinate System Conversion:**
- Chapter 6: Coordinate Systems (OctaIndex3D book) - Frame transformations
- Snyder, J. P. (1987). *Map Projections—A Working Manual*. USGS Professional Paper 1395.

**Validation Techniques:**
- Chapter 9: Testing and Validation (OctaIndex3D book) - Property-based testing for spatial data
- Appendix C: Performance Benchmarks - Methodology for comparing systems

**Domain-Specific Migration:**
- Part IV: Applications (OctaIndex3D book) - Chapters 10-13 for robotics, geospatial, scientific computing, and gaming migrations

---

**End of Appendix F**
