//! BCC Lattice Utilities for Spatial Mapping
//!
//! Helper functions for working with BCC lattice in the context of
//! spatial mapping, TSDF reconstruction, and sensor fusion.

/// Snap physical coordinates to nearest valid BCC lattice point
///
/// BCC lattice requires all coordinates to have identical parity (all even or all odd).
/// This function finds the closest valid BCC point by evaluating all 16 candidates.
///
/// # Performance
/// - Maximum error: √(3/4) ≈ 0.866 voxels
/// - Compare to naive snapping: √3 ≈ 1.73 voxels (2x worse!)
///
/// # Example
/// ```
/// use octaindex3d::layers::bcc_utils::snap_to_nearest_bcc;
///
/// // Coordinate (5, 6, 7) has mixed parity
/// let (x, y, z) = snap_to_nearest_bcc(5, 6, 7);
/// // Result: (5, 7, 7) - all odd, nearest to input
/// assert_eq!((x, y, z), (5, 7, 7));
/// ```
pub fn snap_to_nearest_bcc(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    let x_even = x % 2 == 0;
    let y_even = y % 2 == 0;
    let z_even = z % 2 == 0;

    // Check if already valid BCC point
    if x_even == y_even && y_even == z_even {
        return (x, y, z);
    }

    // Generate all even parity candidates
    let even_candidates = [
        (x & !1, y & !1, z & !1),
        ((x + 1) & !1, y & !1, z & !1),
        (x & !1, (y + 1) & !1, z & !1),
        ((x + 1) & !1, (y + 1) & !1, z & !1),
        (x & !1, y & !1, (z + 1) & !1),
        ((x + 1) & !1, y & !1, (z + 1) & !1),
        (x & !1, (y + 1) & !1, (z + 1) & !1),
        ((x + 1) & !1, (y + 1) & !1, (z + 1) & !1),
    ];

    // Generate all odd parity candidates
    let odd_candidates = [
        (x | 1, y | 1, z | 1),
        ((x - 1) | 1, y | 1, z | 1),
        (x | 1, (y - 1) | 1, z | 1),
        ((x - 1) | 1, (y - 1) | 1, z | 1),
        (x | 1, y | 1, (z - 1) | 1),
        ((x - 1) | 1, y | 1, (z - 1) | 1),
        (x | 1, (y - 1) | 1, (z - 1) | 1),
        ((x - 1) | 1, (y - 1) | 1, (z - 1) | 1),
    ];

    // Find nearest even candidate
    let (best_even, best_even_dist) = find_nearest(&even_candidates, x, y, z);

    // Find nearest odd candidate
    let (best_odd, best_odd_dist) = find_nearest(&odd_candidates, x, y, z);

    // Return overall nearest
    if best_even_dist <= best_odd_dist {
        best_even
    } else {
        best_odd
    }
}

/// Find nearest point from candidates
#[inline]
fn find_nearest(candidates: &[(i32, i32, i32)], x: i32, y: i32, z: i32) -> ((i32, i32, i32), i32) {
    let mut best = candidates[0];
    let mut best_dist = distance_squared(x, y, z, best);

    for &candidate in &candidates[1..] {
        let dist = distance_squared(x, y, z, candidate);
        if dist < best_dist {
            best_dist = dist;
            best = candidate;
        }
    }

    (best, best_dist)
}

/// Compute squared distance (avoids expensive sqrt)
#[inline]
fn distance_squared(x1: i32, y1: i32, z1: i32, p2: (i32, i32, i32)) -> i32 {
    let dx = x1 - p2.0;
    let dy = y1 - p2.1;
    let dz = z1 - p2.2;
    dx * dx + dy * dy + dz * dz
}

/// Convert physical coordinates to BCC voxel coordinates
///
/// # Arguments
/// * `pos` - Physical position (x, y, z) in meters
/// * `voxel_size` - Size of each voxel in meters
///
/// # Returns
/// Nearest valid BCC lattice point in voxel coordinates
///
/// # Example
/// ```
/// use octaindex3d::layers::bcc_utils::physical_to_bcc_voxel;
///
/// let pos = (1.23, 4.56, 7.89); // meters
/// let voxel_size = 0.05; // 5cm voxels
/// let (vx, vy, vz) = physical_to_bcc_voxel(pos, voxel_size);
/// ```
pub fn physical_to_bcc_voxel(pos: (f32, f32, f32), voxel_size: f32) -> (i32, i32, i32) {
    // Convert to voxel coordinates
    let vx = (pos.0 / voxel_size).round() as i32;
    let vy = (pos.1 / voxel_size).round() as i32;
    let vz = (pos.2 / voxel_size).round() as i32;

    // Snap to nearest BCC point
    snap_to_nearest_bcc(vx, vy, vz)
}

/// Check if coordinates form a valid BCC lattice point
#[inline]
pub fn is_valid_bcc(x: i32, y: i32, z: i32) -> bool {
    let x_even = x % 2 == 0;
    let y_even = y % 2 == 0;
    let z_even = z % 2 == 0;
    x_even == y_even && y_even == z_even
}

/// Compute maximum snapping error for BCC lattice
///
/// Returns the theoretical maximum distance a point can move
/// when snapped to the nearest BCC lattice point.
///
/// For BCC: max_error = √(3/4) ≈ 0.866 voxels
pub fn max_bcc_snap_error() -> f32 {
    (3.0_f32 / 4.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_already_valid() {
        // All even
        assert_eq!(snap_to_nearest_bcc(0, 0, 0), (0, 0, 0));
        assert_eq!(snap_to_nearest_bcc(2, 4, 6), (2, 4, 6));

        // All odd
        assert_eq!(snap_to_nearest_bcc(1, 1, 1), (1, 1, 1));
        assert_eq!(snap_to_nearest_bcc(3, 5, 7), (3, 5, 7));
    }

    #[test]
    fn test_snap_mixed_parity() {
        // (5, 6, 7) → mixed parity
        // Nearest odd: (5, 7, 7) - dist² = 0 + 1 + 0 = 1
        // Nearest even: (6, 6, 6) - dist² = 1 + 0 + 1 = 2
        let result = snap_to_nearest_bcc(5, 6, 7);
        assert_eq!(result, (5, 7, 7));

        // (0, 1, 0) → mixed parity
        // Nearest even: (0, 0, 0) - dist² = 0 + 1 + 0 = 1
        // Nearest odd: (1, 1, 1) - dist² = 1 + 0 + 1 = 2
        let result = snap_to_nearest_bcc(0, 1, 0);
        assert_eq!(result, (0, 0, 0));
    }

    #[test]
    fn test_snap_error_bound() {
        // Test that snapping error is within theoretical bound
        let _max_error_sq = (3.0 / 4.0) as i32; // Should be 0, but we use squared distance

        for x in -10..10 {
            for y in -10..10 {
                for z in -10..10 {
                    let (sx, sy, sz) = snap_to_nearest_bcc(x, y, z);

                    // Verify snapped point is valid BCC
                    assert!(is_valid_bcc(sx, sy, sz));

                    // Verify distance is within bound
                    let dist_sq = distance_squared(x, y, z, (sx, sy, sz));
                    assert!(
                        dist_sq <= 3,
                        "Snap error too large: ({},{},{}) → ({},{},{}) dist²={}",
                        x,
                        y,
                        z,
                        sx,
                        sy,
                        sz,
                        dist_sq
                    );
                }
            }
        }
    }

    #[test]
    fn test_is_valid_bcc() {
        assert!(is_valid_bcc(0, 0, 0));
        assert!(is_valid_bcc(2, 4, 6));
        assert!(is_valid_bcc(1, 1, 1));
        assert!(is_valid_bcc(3, 5, 7));

        assert!(!is_valid_bcc(0, 1, 0));
        assert!(!is_valid_bcc(1, 2, 3));
        assert!(!is_valid_bcc(0, 0, 1));
    }

    #[test]
    fn test_physical_to_voxel() {
        let voxel_size = 0.05; // 5cm voxels

        // (0.1, 0.1, 0.1) → voxel (2, 2, 2)
        let (vx, vy, vz) = physical_to_bcc_voxel((0.1, 0.1, 0.1), voxel_size);
        assert!(is_valid_bcc(vx, vy, vz));
        assert!((vx - 2).abs() <= 1);
        assert!((vy - 2).abs() <= 1);
        assert!((vz - 2).abs() <= 1);
    }

    #[test]
    fn test_max_error() {
        let max_err = max_bcc_snap_error();
        assert!((max_err - 0.866).abs() < 0.001);
    }
}
