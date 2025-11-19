//! Neighbor operations for spatial IDs

use crate::ids::{Galactic128, Index64, Route64};
use crate::lattice::BCC_NEIGHBORS_14;

/// Get 14 neighbors of a Route64 coordinate
#[must_use]
pub fn neighbors_route64(route: Route64) -> Vec<Route64> {
    let tier = route.scale_tier();
    let (x, y, z) = (route.x(), route.y(), route.z());

    BCC_NEIGHBORS_14
        .iter()
        .filter_map(|(dx, dy, dz)| {
            let nx = x.checked_add(*dx)?;
            let ny = y.checked_add(*dy)?;
            let nz = z.checked_add(*dz)?;
            Route64::new(tier, nx, ny, nz).ok()
        })
        .collect()
}

/// Get 14 neighbors of an Index64 coordinate
#[must_use]
pub fn neighbors_index64(index: Index64) -> Vec<Index64> {
    let (x, y, z) = index.decode_coords();
    let frame = index.frame_id();
    let tier = index.scale_tier();
    let lod = index.lod();

    BCC_NEIGHBORS_14
        .iter()
        .filter_map(|(dx, dy, dz)| {
            // Convert to signed and add offset
            let nx = (x as i32).checked_add(*dx)?;
            let ny = (y as i32).checked_add(*dy)?;
            let nz = (z as i32).checked_add(*dz)?;

            // Check if still in u16 range
            if nx < 0
                || ny < 0
                || nz < 0
                || nx > u16::MAX as i32
                || ny > u16::MAX as i32
                || nz > u16::MAX as i32
            {
                return None;
            }

            Index64::new(frame, tier, lod, nx as u16, ny as u16, nz as u16).ok()
        })
        .collect()
}

/// Get 14 neighbors of a Galactic128 coordinate
#[must_use]
pub fn neighbors_galactic128(galactic: Galactic128) -> Vec<Galactic128> {
    let (x, y, z) = (galactic.x(), galactic.y(), galactic.z());
    let frame = galactic.frame_id();
    let scale_mant = galactic.scale_mant();
    let scale_tier = galactic.scale_tier();
    let lod = galactic.lod();
    let attr_usr = galactic.attr_usr();

    BCC_NEIGHBORS_14
        .iter()
        .filter_map(|(dx, dy, dz)| {
            let nx = x.checked_add(*dx)?;
            let ny = y.checked_add(*dy)?;
            let nz = z.checked_add(*dz)?;
            Galactic128::new(frame, scale_mant, scale_tier, lod, attr_usr, nx, ny, nz).ok()
        })
        .collect()
}

/// Compute Euclidean distance between two Route64 cells
#[must_use]
pub fn distance_route64(a: Route64, b: Route64) -> f64 {
    let dx = (a.x() - b.x()) as f64;
    let dy = (a.y() - b.y()) as f64;
    let dz = (a.z() - b.z()) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Compute Manhattan distance between two Route64 cells
#[must_use]
pub fn manhattan_distance_route64(a: Route64, b: Route64) -> i32 {
    (a.x() - b.x()).abs() + (a.y() - b.y()).abs() + (a.z() - b.z()).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route64_neighbors() {
        let route = Route64::new(0, 0, 0, 0).unwrap();
        let neighbors = neighbors_route64(route);
        assert_eq!(neighbors.len(), 14);

        // Check that all neighbors have opposite parity for diagonal moves
        // or same parity for axis-aligned moves
    }

    #[test]
    fn test_index64_neighbors() {
        // Use center coordinates that won't underflow (14 away from edges)
        let index = Index64::new(0, 0, 5, 100, 100, 100).unwrap();
        let neighbors = neighbors_index64(index);
        // Should get all 14 neighbors when not at boundary
        assert_eq!(neighbors.len(), 14);

        // Test at boundary - should get fewer neighbors
        let edge_index = Index64::new(0, 0, 5, 0, 0, 0).unwrap();
        let edge_neighbors = neighbors_index64(edge_index);
        // At corner, can only move in positive directions
        // 8 parity-flipping neighbors: (1,1,1), (1,1,-1)X, (1,-1,1)X, (1,-1,-1)X,
        //                              (-1,1,1)X, (-1,1,-1)X, (-1,-1,1)X, (-1,-1,-1)X
        // Only (1,1,1) valid
        // 6 parity-preserving: (2,0,0), (-2,0,0)X, (0,2,0), (0,-2,0)X, (0,0,2), (0,0,-2)X
        // 4 neighbors total
        assert_eq!(edge_neighbors.len(), 4);
    }

    #[test]
    fn test_galactic128_neighbors() {
        let galactic = Galactic128::new(0, 0, 0, 0, 0, 0, 0, 0).unwrap();
        let neighbors = neighbors_galactic128(galactic);
        assert_eq!(neighbors.len(), 14);
    }

    #[test]
    fn test_distance() {
        let a = Route64::new(0, 0, 0, 0).unwrap();
        let b = Route64::new(0, 2, 0, 0).unwrap();

        let dist = distance_route64(a, b);
        assert!((dist - 2.0).abs() < 1e-10);

        let manhattan = manhattan_distance_route64(a, b);
        assert_eq!(manhattan, 2);
    }
}
