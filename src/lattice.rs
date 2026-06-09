//! BCC Lattice geometry and mathematics v0.3.0
//!
//! This module implements the core Body-Centered Cubic (BCC) lattice structure
//! with truncated octahedral cells.

use crate::error::{Error, Result};

/// 14-neighbor connectivity for BCC lattice
/// 8 parity-flipping neighbors (diagonal, distance √3)
/// 6 parity-preserving neighbors (axis-aligned, distance 2)
pub const BCC_NEIGHBORS_14: &[(i32, i32, i32)] = &[
    // 8 parity-flipping (diagonal)
    (1, 1, 1),
    (1, 1, -1),
    (1, -1, 1),
    (1, -1, -1),
    (-1, 1, 1),
    (-1, 1, -1),
    (-1, -1, 1),
    (-1, -1, -1),
    // 6 parity-preserving (axis-aligned)
    (2, 0, 0),
    (-2, 0, 0),
    (0, 2, 0),
    (0, -2, 0),
    (0, 0, 2),
    (0, 0, -2),
];

/// Parity type for BCC lattice coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parity {
    /// Even parity (all coordinates even)
    Even,
    /// Odd parity (all coordinates odd)
    Odd,
}

impl Parity {
    /// Check if coordinates have valid identical parity
    #[inline]
    pub fn from_coords(x: i32, y: i32, z: i32) -> Result<Self> {
        let x_even = x % 2 == 0;
        let y_even = y % 2 == 0;
        let z_even = z % 2 == 0;

        if x_even == y_even && y_even == z_even {
            Ok(if x_even { Parity::Even } else { Parity::Odd })
        } else {
            Err(Self::invalid_parity_error(x, y, z))
        }
    }

    /// Cold path for parity error creation - kept out of hot instruction cache
    #[cold]
    #[inline(never)]
    fn invalid_parity_error(x: i32, y: i32, z: i32) -> Error {
        Error::InvalidParity { x, y, z }
    }

    /// Get the opposite parity
    pub fn opposite(self) -> Self {
        match self {
            Parity::Even => Parity::Odd,
            Parity::Odd => Parity::Even,
        }
    }
}

/// BCC Lattice coordinate system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LatticeCoord {
    /// X coordinate on the BCC lattice
    pub x: i32,
    /// Y coordinate on the BCC lattice
    pub y: i32,
    /// Z coordinate on the BCC lattice
    pub z: i32,
    /// Parity of the coordinates (all even or all odd)
    pub parity: Parity,
}

impl LatticeCoord {
    /// Create a new lattice coordinate, validating parity
    pub fn new(x: i32, y: i32, z: i32) -> Result<Self> {
        let parity = Parity::from_coords(x, y, z)?;
        Ok(Self { x, y, z, parity })
    }

    /// Create from coordinates without validation (use carefully!)
    pub fn new_unchecked(x: i32, y: i32, z: i32) -> Self {
        let parity = if x % 2 == 0 {
            Parity::Even
        } else {
            Parity::Odd
        };
        Self { x, y, z, parity }
    }

    /// Convert to physical 3D coordinates (center of cell)
    /// Assumes base unit length of 2 for lattice spacing
    pub fn to_physical(&self) -> (f64, f64, f64) {
        (self.x as f64, self.y as f64, self.z as f64)
    }

    /// Compute Euclidean distance to another lattice point
    pub fn distance_to(&self, other: &LatticeCoord) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        let dz = (self.z - other.z) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Compute Manhattan distance to another lattice point
    pub fn manhattan_distance_to(&self, other: &LatticeCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }

    /// Scale coordinates by a factor (for resolution changes)
    pub fn scale(&self, factor: i32) -> Self {
        Self::new_unchecked(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Divide coordinates by 2
    #[deprecated(
        since = "0.5.6",
        note = "component-wise halving does not produce valid BCC parents; use Lattice::get_parent"
    )]
    pub fn half(&self) -> Self {
        Self::new_unchecked(self.x / 2, self.y / 2, self.z / 2)
    }
}

/// Lattice utility functions
pub struct Lattice;

impl Lattice {
    /// Convert physical coordinates to the nearest valid BCC lattice point.
    ///
    /// Resolution 0 = base lattice; each higher resolution doubles the lattice
    /// density (physical coordinates are scaled by `2^resolution` before snapping).
    ///
    /// A valid BCC point has all coordinates even or all coordinates odd. The
    /// nearest such point is found by comparing the best all-even candidate
    /// (each axis rounded to the nearest even integer) against the best all-odd
    /// candidate (each axis rounded to the nearest odd integer). The snapping
    /// error is at most the BCC covering radius, `sqrt(5)/2 ≈ 1.118` lattice units.
    pub fn physical_to_lattice(x: f64, y: f64, z: f64, resolution: u8) -> Result<LatticeCoord> {
        let scale = 2.0_f64.powi(resolution as i32);
        let (sx, sy, sz) = (x * scale, y * scale, z * scale);

        for v in [sx, sy, sz] {
            if !v.is_finite() || v.abs() > i32::MAX as f64 - 1.0 {
                return Err(Error::OutOfRange(format!(
                    "physical coordinate {} out of lattice range at resolution {}",
                    v, resolution
                )));
            }
        }

        let even = (round_to_even(sx), round_to_even(sy), round_to_even(sz));
        let odd = (round_to_odd(sx), round_to_odd(sy), round_to_odd(sz));

        let dist2 = |(cx, cy, cz): (i32, i32, i32)| {
            (cx as f64 - sx).powi(2) + (cy as f64 - sy).powi(2) + (cz as f64 - sz).powi(2)
        };

        let best = if dist2(even) <= dist2(odd) { even } else { odd };
        LatticeCoord::new(best.0, best.1, best.2)
    }

    /// Check if coordinates represent a valid BCC lattice point
    pub fn is_valid_lattice_point(x: i32, y: i32, z: i32) -> bool {
        Parity::from_coords(x, y, z).is_ok()
    }

    /// Get all 14 neighbors of a lattice coordinate
    pub fn get_neighbors(coord: &LatticeCoord) -> Vec<LatticeCoord> {
        BCC_NEIGHBORS_14
            .iter()
            .map(|(dx, dy, dz)| {
                LatticeCoord::new_unchecked(coord.x + dx, coord.y + dy, coord.z + dz)
            })
            .collect()
    }

    /// Compute the 8 children coordinates for hierarchical refinement.
    ///
    /// Each parent at resolution R owns exactly 8 children at resolution R+1
    /// (coordinates double between resolutions). The children of parent `p` are:
    ///
    /// - `2p` plus the three positive axial offsets `(2,0,0)`, `(0,2,0)`, `(0,0,2)`
    /// - the four positive-x body diagonals `2p + (1, ±1, ±1)`
    ///
    /// Diagonal and axial child points lie exactly on the boundary between two
    /// parent Voronoi cells; the positive-direction convention above assigns each
    /// to exactly one parent, so every BCC point has a unique parent and all
    /// children are valid BCC lattice points (see [`Lattice::get_parent`]).
    pub fn get_children(coord: &LatticeCoord) -> Vec<LatticeCoord> {
        const CHILD_OFFSETS: [(i32, i32, i32); 8] = [
            (0, 0, 0),
            (2, 0, 0),
            (0, 2, 0),
            (0, 0, 2),
            (1, 1, 1),
            (1, 1, -1),
            (1, -1, 1),
            (1, -1, -1),
        ];

        CHILD_OFFSETS
            .iter()
            .map(|(dx, dy, dz)| {
                LatticeCoord::new_unchecked(2 * coord.x + dx, 2 * coord.y + dy, 2 * coord.z + dz)
            })
            .collect()
    }

    /// Compute parent coordinate for hierarchical coarsening.
    ///
    /// Exact inverse of [`Lattice::get_children`]: for every parent `p` and each
    /// child `c` in `get_children(p)`, `get_parent(c) == p`, and every valid BCC
    /// point has exactly one parent.
    pub fn get_parent(coord: &LatticeCoord) -> LatticeCoord {
        match coord.parity {
            Parity::Even => {
                // Child was 2p or 2p + an even axial offset: halve, then if the
                // result has mixed parity, exactly one axis is in the minority —
                // subtracting 1 from it recovers the parent.
                let (qx, qy, qz) = (coord.x / 2, coord.y / 2, coord.z / 2);
                let odd = [qx & 1 != 0, qy & 1 != 0, qz & 1 != 0];
                let odd_count = odd.iter().filter(|&&o| o).count();
                match odd_count {
                    0 | 3 => LatticeCoord::new_unchecked(qx, qy, qz),
                    1 => {
                        // Minority axis is the odd one
                        let (dx, dy, dz) = (odd[0] as i32, odd[1] as i32, odd[2] as i32);
                        LatticeCoord::new_unchecked(qx - dx, qy - dy, qz - dz)
                    }
                    _ => {
                        // Minority axis is the even one
                        let (dx, dy, dz) = (!odd[0] as i32, !odd[1] as i32, !odd[2] as i32);
                        LatticeCoord::new_unchecked(qx - dx, qy - dy, qz - dz)
                    }
                }
            }
            Parity::Odd => {
                // Child was 2p + (1, ±1, ±1): x determines the parent's x exactly;
                // for y and z the two halving candidates differ by 1, so exactly
                // one matches the parity of parent x.
                let px = (coord.x - 1) / 2;
                let target = px & 1;
                let pick = |c: i32| {
                    let lo = (c - 1) / 2;
                    if lo & 1 == target {
                        lo
                    } else {
                        lo + 1
                    }
                };
                LatticeCoord::new_unchecked(px, pick(coord.y), pick(coord.z))
            }
        }
    }
}

/// Round to the nearest even integer
#[inline]
fn round_to_even(v: f64) -> i32 {
    ((v / 2.0).round() * 2.0) as i32
}

/// Round to the nearest odd integer
#[inline]
fn round_to_odd(v: f64) -> i32 {
    (((v - 1.0) / 2.0).round() * 2.0) as i32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_validation() {
        // Valid even parity
        assert!(Parity::from_coords(0, 0, 0).is_ok());
        assert!(Parity::from_coords(2, 4, 6).is_ok());

        // Valid odd parity
        assert!(Parity::from_coords(1, 1, 1).is_ok());
        assert!(Parity::from_coords(3, 5, 7).is_ok());

        // Invalid mixed parity
        assert!(Parity::from_coords(0, 1, 0).is_err());
        assert!(Parity::from_coords(1, 2, 3).is_err());
    }

    #[test]
    fn test_lattice_coord_creation() {
        let coord = LatticeCoord::new(2, 4, 6).unwrap();
        assert_eq!(coord.x, 2);
        assert_eq!(coord.y, 4);
        assert_eq!(coord.z, 6);
        assert_eq!(coord.parity, Parity::Even);

        let coord = LatticeCoord::new(1, 3, 5).unwrap();
        assert_eq!(coord.parity, Parity::Odd);

        // Invalid should fail
        assert!(LatticeCoord::new(0, 1, 2).is_err());
    }

    #[test]
    fn test_neighbor_count() {
        let coord = LatticeCoord::new(0, 0, 0).unwrap();
        let neighbors = Lattice::get_neighbors(&coord);
        assert_eq!(neighbors.len(), 14, "BCC lattice should have 14 neighbors");
    }

    #[test]
    fn test_neighbor_parity() {
        let coord = LatticeCoord::new(0, 0, 0).unwrap(); // Even parity
        let neighbors = Lattice::get_neighbors(&coord);

        // 8 neighbors should be opposite parity (odd)
        let opposite_parity_count = neighbors.iter().filter(|n| n.parity == Parity::Odd).count();
        assert_eq!(opposite_parity_count, 8);

        // 6 neighbors should be same parity (even)
        let same_parity_count = neighbors
            .iter()
            .filter(|n| n.parity == Parity::Even)
            .count();
        assert_eq!(same_parity_count, 6);
    }

    #[test]
    fn test_children_count() {
        let coord = LatticeCoord::new(2, 4, 6).unwrap();
        let children = Lattice::get_children(&coord);
        assert_eq!(children.len(), 8, "Each cell should have 8 children");
    }

    #[test]
    fn test_children_are_valid_bcc_points() {
        for &(px, py, pz) in &[(2, 4, 6), (1, 3, 5), (0, 0, 0), (-3, 5, -7), (-2, -4, 6)] {
            let parent = LatticeCoord::new(px, py, pz).unwrap();
            for child in Lattice::get_children(&parent) {
                assert!(
                    Parity::from_coords(child.x, child.y, child.z).is_ok(),
                    "child ({},{},{}) of ({},{},{}) is not a valid BCC point",
                    child.x,
                    child.y,
                    child.z,
                    px,
                    py,
                    pz
                );
            }
        }
    }

    #[test]
    fn test_parent_child_relationship() {
        // Round-trip for even, odd, and negative parents
        for &(px, py, pz) in &[(2, 4, 6), (1, 3, 5), (0, 0, 0), (-3, 5, -7), (-2, -4, 6)] {
            let parent = LatticeCoord::new(px, py, pz).unwrap();
            let children = Lattice::get_children(&parent);
            assert_eq!(children.len(), 8);

            for child in &children {
                let child_parent = Lattice::get_parent(child);
                assert_eq!(
                    child_parent, parent,
                    "child ({},{},{}) should have parent ({},{},{})",
                    child.x, child.y, child.z, px, py, pz
                );
            }
        }
    }

    #[test]
    fn test_children_partition_lattice() {
        // Every valid BCC point in a region must appear in exactly one parent's
        // children set (the 8:1 refinement is a partition).
        use std::collections::HashMap;

        let mut owner: HashMap<(i32, i32, i32), (i32, i32, i32)> = HashMap::new();
        for px in -4..=4 {
            for py in -4..=4 {
                for pz in -4..=4 {
                    let Ok(parent) = LatticeCoord::new(px, py, pz) else {
                        continue;
                    };
                    for child in Lattice::get_children(&parent) {
                        let prev = owner.insert((child.x, child.y, child.z), (px, py, pz));
                        assert!(
                            prev.is_none(),
                            "child ({},{},{}) claimed by both {:?} and ({},{},{})",
                            child.x,
                            child.y,
                            child.z,
                            prev,
                            px,
                            py,
                            pz
                        );
                    }
                }
            }
        }

        // Interior child-level points (away from the sampled boundary) must all be covered
        for cx in -6..=6 {
            for cy in -6..=6 {
                for cz in -6..=6 {
                    if Parity::from_coords(cx, cy, cz).is_ok() {
                        assert!(
                            owner.contains_key(&(cx, cy, cz)),
                            "BCC point ({},{},{}) has no parent",
                            cx,
                            cy,
                            cz
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_physical_to_lattice_valid_points() {
        // Exact all-odd point must map to itself (regression: previously errored)
        let c = Lattice::physical_to_lattice(1.0, 1.0, 1.0, 0).unwrap();
        assert_eq!((c.x, c.y, c.z), (1, 1, 1));

        // Exact all-even point maps to itself
        let c = Lattice::physical_to_lattice(2.0, 4.0, 6.0, 0).unwrap();
        assert_eq!((c.x, c.y, c.z), (2, 4, 6));

        // Mixed-parity integer input snaps to a nearby valid point
        // (regression: (2,1,1) previously errored)
        let c = Lattice::physical_to_lattice(2.0, 1.0, 1.0, 0).unwrap();
        assert!(Parity::from_coords(c.x, c.y, c.z).is_ok());
    }

    #[test]
    fn test_physical_to_lattice_always_succeeds_and_is_near() {
        // Property: any finite in-range input snaps to a valid point within the
        // BCC covering radius sqrt(5)/2 (dist² ≤ 1.25)
        let max_dist2 = 1.25 + 1e-9;
        for i in 0..1000 {
            // Deterministic pseudo-random sample points
            let x = ((i * 7919) % 200) as f64 / 10.0 - 10.0 + 0.137;
            let y = ((i * 104729) % 200) as f64 / 10.0 - 10.0 + 0.421;
            let z = ((i * 1299709) % 200) as f64 / 10.0 - 10.0 + 0.733;

            let c = Lattice::physical_to_lattice(x, y, z, 0)
                .unwrap_or_else(|e| panic!("({}, {}, {}) failed: {}", x, y, z, e));
            let d2 = (c.x as f64 - x).powi(2) + (c.y as f64 - y).powi(2) + (c.z as f64 - z).powi(2);
            assert!(
                d2 <= max_dist2,
                "({}, {}, {}) snapped to ({},{},{}), dist² = {}",
                x,
                y,
                z,
                c.x,
                c.y,
                c.z,
                d2
            );
        }
    }

    #[test]
    fn test_physical_to_lattice_resolution_scaling() {
        // At resolution 1, coordinates are scaled by 2 before snapping
        let c = Lattice::physical_to_lattice(1.0, 2.0, 3.0, 1).unwrap();
        assert_eq!((c.x, c.y, c.z), (2, 4, 6));

        // Out-of-range input errors instead of overflowing
        assert!(Lattice::physical_to_lattice(1e300, 0.0, 0.0, 0).is_err());
        assert!(Lattice::physical_to_lattice(f64::NAN, 0.0, 0.0, 0).is_err());
    }

    #[test]
    fn test_distance() {
        let a = LatticeCoord::new(0, 0, 0).unwrap();
        let b = LatticeCoord::new(2, 0, 0).unwrap();
        assert_eq!(a.distance_to(&b), 2.0);

        let c = LatticeCoord::new(1, 1, 1).unwrap();
        let dist = a.distance_to(&c);
        assert!((dist - 3.0_f64.sqrt()).abs() < 1e-10);
    }
}
