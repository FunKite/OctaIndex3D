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
    pub fn from_coords(x: i32, y: i32, z: i32) -> Result<Self> {
        let x_even = x % 2 == 0;
        let y_even = y % 2 == 0;
        let z_even = z % 2 == 0;

        if x_even == y_even && y_even == z_even {
            Ok(if x_even { Parity::Even } else { Parity::Odd })
        } else {
            Err(Error::InvalidParity { x, y, z })
        }
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

    /// Divide coordinates by 2 (for parent lookup)
    pub fn half(&self) -> Self {
        Self::new_unchecked(self.x / 2, self.y / 2, self.z / 2)
    }
}

/// Lattice utility functions
pub struct Lattice;

impl Lattice {
    /// Convert physical coordinates to nearest lattice point at given resolution
    /// Resolution 0 = base lattice, higher resolutions = finer grid (2^res scaling)
    pub fn physical_to_lattice(x: f64, y: f64, z: f64, _resolution: u8) -> Result<LatticeCoord> {
        let _scale = 1.0; // At resolution 0, lattice spacing is 1 unit

        // Round to nearest integer coordinates
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        let zi = z.round() as i32;

        // Ensure parity (if mixed, find nearest valid point)
        let sum_parity = (xi + yi + zi) % 2;

        if sum_parity == 0 {
            // All even or all odd - check which
            LatticeCoord::new(xi, yi, zi)
        } else {
            // Mixed parity - need to adjust to nearest valid point
            // Try rounding each dimension differently to find nearest valid point
            let candidates = vec![
                (xi + 1, yi, zi),
                (xi - 1, yi, zi),
                (xi, yi + 1, zi),
                (xi, yi - 1, zi),
                (xi, yi, zi + 1),
                (xi, yi, zi - 1),
            ];

            let mut best = candidates[0];
            let mut best_dist = f64::MAX;

            for (cx, cy, cz) in candidates {
                if Parity::from_coords(cx, cy, cz).is_ok() {
                    let dist = ((cx as f64 - x).powi(2)
                        + (cy as f64 - y).powi(2)
                        + (cz as f64 - z).powi(2))
                    .sqrt();
                    if dist < best_dist {
                        best_dist = dist;
                        best = (cx, cy, cz);
                    }
                }
            }

            LatticeCoord::new(best.0, best.1, best.2)
        }
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

    /// Compute the 8 children coordinates for hierarchical refinement
    /// Each parent at resolution R has 8 children at resolution R+1
    pub fn get_children(coord: &LatticeCoord) -> Vec<LatticeCoord> {
        let mut children = Vec::with_capacity(8);

        // Children are at 2*parent ± (0 or 1) for each axis
        for dx in [0, 1] {
            for dy in [0, 1] {
                for dz in [0, 1] {
                    children.push(LatticeCoord::new_unchecked(
                        2 * coord.x + dx,
                        2 * coord.y + dy,
                        2 * coord.z + dz,
                    ));
                }
            }
        }

        children
    }

    /// Compute parent coordinate for hierarchical coarsening
    pub fn get_parent(coord: &LatticeCoord) -> LatticeCoord {
        coord.half()
    }
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
    fn test_parent_child_relationship() {
        let parent = LatticeCoord::new(2, 4, 6).unwrap();
        let children = Lattice::get_children(&parent);

        // All children should have the same parent when we compute their parent
        for child in &children {
            let child_parent = Lattice::get_parent(child);
            assert_eq!(child_parent, parent);
        }
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
