//! Euclidean Signed Distance Field (ESDF) layer
//!
//! Computes full (non-truncated) signed distance field from TSDF using
//! Fast Marching Method on BCC lattice.
//!
//! ## Algorithm
//!
//! 1. Initialize from TSDF zero-crossings (surface voxels)
//! 2. Propagate distances using BCC 14-neighbor connectivity
//! 3. Use priority queue for efficient wavefront expansion
//!
//! ## Advantages on BCC Lattice
//!
//! - **Better isotropy**: 14 neighbors vs 6/26 cubic → more accurate distances
//! - **Natural edge lengths**: BCC has 2 edge types (√3 and 2) vs cubic's 3 (1, √2, √3)
//! - **Fewer distance artifacts**: More uniform propagation in all directions

use super::{Layer, LayerType};
use crate::error::Result;
use crate::neighbors::neighbors_index64;
use crate::Index64;
use ordered_float::OrderedFloat;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Voxel data in ESDF layer
#[derive(Debug, Clone, Copy)]
struct ESDFVoxel {
    /// Signed distance value (no truncation)
    distance: f32,
    /// Whether this voxel has been processed
    fixed: bool,
}

impl Default for ESDFVoxel {
    fn default() -> Self {
        Self {
            distance: f32::MAX,
            fixed: false,
        }
    }
}

/// Euclidean Signed Distance Field layer
///
/// Stores full signed distance to nearest surface.
/// Computed from TSDF using Fast Marching Method on BCC lattice.
pub struct ESDFLayer {
    /// Voxel data indexed by Index64
    voxels: HashMap<Index64, ESDFVoxel>,

    /// Voxel size (meters per voxel)
    voxel_size: f32,

    /// Maximum distance to compute (meters)
    /// Beyond this, distance is clamped
    max_distance: f32,

    /// BCC edge lengths (precomputed for efficiency)
    /// - Diagonal edges (8 neighbors): √3
    /// - Axial edges (6 neighbors): 2
    edge_lengths: EdgeLengths,
}

/// Precomputed edge lengths for BCC lattice
#[derive(Debug, Clone, Copy)]
struct EdgeLengths {
    /// Diagonal edge length: √3 ≈ 1.732
    diagonal: f32,
    /// Axial edge length: 2.0
    axial: f32,
}

impl Default for EdgeLengths {
    fn default() -> Self {
        Self {
            diagonal: 3.0_f32.sqrt(),
            axial: 2.0,
        }
    }
}

impl ESDFLayer {
    /// Create new ESDF layer
    ///
    /// # Arguments
    /// * `voxel_size` - Size of each voxel in meters
    /// * `max_distance` - Maximum distance to compute (meters)
    ///
    /// # Example
    /// ```
    /// use octaindex3d::layers::ESDFLayer;
    ///
    /// let esdf = ESDFLayer::new(0.02, 5.0); // 2cm voxels, 5m max distance
    /// ```
    pub fn new(voxel_size: f32, max_distance: f32) -> Self {
        Self {
            voxels: HashMap::new(),
            voxel_size,
            max_distance,
            edge_lengths: EdgeLengths::default(),
        }
    }

    /// Get voxel size
    pub fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    /// Get maximum distance
    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }

    /// Get distance value for a voxel
    pub fn get_distance(&self, idx: Index64) -> Option<f32> {
        self.voxels.get(&idx).map(|v| v.distance)
    }

    /// Compute ESDF from TSDF using Fast Marching Method
    ///
    /// This is the main entry point for ESDF computation.
    ///
    /// # Algorithm
    /// 1. Find TSDF zero-crossings (surface voxels)
    /// 2. Initialize ESDF at these points with distance ≈ 0
    /// 3. Propagate using priority queue (Dijkstra-like)
    /// 4. Use BCC 14-neighbor connectivity for better isotropy
    ///
    /// # Arguments
    /// * `tsdf` - Source TSDF layer
    /// * `surface_threshold` - Distance threshold for surface detection (meters)
    pub fn compute_from_tsdf(
        &mut self,
        tsdf: &super::TSDFLayer,
        surface_threshold: f32,
    ) -> Result<()> {
        // Clear existing data
        self.voxels.clear();

        // Get surface voxels from TSDF (zero-crossings)
        let surface_voxels = tsdf.get_surface_voxels(surface_threshold);

        if surface_voxels.is_empty() {
            return Ok(()); // No surface found
        }

        // Initialize priority queue for Fast Marching
        // Use min-heap ordered by distance (smallest distance processed first)
        let mut open: BinaryHeap<Reverse<(OrderedFloat<f32>, Index64)>> = BinaryHeap::new();

        // Initialize surface voxels
        for &idx in &surface_voxels {
            let dist = tsdf.get_distance(idx).unwrap_or(0.0);

            self.voxels.insert(
                idx,
                ESDFVoxel {
                    distance: dist,
                    fixed: true,
                },
            );

            // Add neighbors to open list
            let neighbors = neighbors_index64(idx);
            for neighbor_idx in neighbors {
                if !self.voxels.contains_key(&neighbor_idx) {
                    open.push(Reverse((OrderedFloat(dist.abs()), neighbor_idx)));
                }
            }
        }

        // Fast marching: propagate distances
        while let Some(Reverse((_, current_idx))) = open.pop() {
            // Skip if already processed
            if self
                .voxels
                .get(&current_idx)
                .map(|v| v.fixed)
                .unwrap_or(false)
            {
                continue;
            }

            // Compute distance from neighbors
            let new_distance = self.compute_distance_from_neighbors(current_idx);

            // Clamp to max distance
            let clamped_distance = new_distance.clamp(-self.max_distance, self.max_distance);

            // Mark as fixed
            self.voxels.insert(
                current_idx,
                ESDFVoxel {
                    distance: clamped_distance,
                    fixed: true,
                },
            );

            // Add neighbors to open list
            let neighbors = neighbors_index64(current_idx);
            for neighbor_idx in neighbors {
                if !self
                    .voxels
                    .get(&neighbor_idx)
                    .map(|v| v.fixed)
                    .unwrap_or(false)
                {
                    let estimated_dist =
                        clamped_distance.abs() + self.edge_lengths.diagonal * self.voxel_size;

                    if estimated_dist <= self.max_distance {
                        open.push(Reverse((OrderedFloat(estimated_dist), neighbor_idx)));
                    }
                }
            }
        }

        Ok(())
    }

    /// Compute distance for a voxel from its neighbors
    ///
    /// Uses minimum distance + edge length across all 14 BCC neighbors
    fn compute_distance_from_neighbors(&self, idx: Index64) -> f32 {
        let neighbors = neighbors_index64(idx);
        let mut min_distance = f32::MAX;

        for (i, neighbor_idx) in neighbors.iter().enumerate() {
            if let Some(neighbor) = self.voxels.get(neighbor_idx) {
                if neighbor.fixed {
                    // Determine edge length based on neighbor type
                    // First 8 neighbors are diagonal (√3), next 6 are axial (2)
                    let edge_length = if i < 8 {
                        self.edge_lengths.diagonal
                    } else {
                        self.edge_lengths.axial
                    };

                    // Distance = neighbor distance + edge length
                    let candidate_dist = neighbor.distance.signum()
                        * (neighbor.distance.abs() + edge_length * self.voxel_size);

                    if candidate_dist.abs() < min_distance.abs() {
                        min_distance = candidate_dist;
                    }
                }
            }
        }

        min_distance
    }

    /// Get all voxels within a distance threshold
    ///
    /// Useful for path planning collision checking
    pub fn get_voxels_within_distance(&self, threshold: f32) -> Vec<(Index64, f32)> {
        self.voxels
            .iter()
            .filter(|(_, v)| v.distance.abs() <= threshold)
            .map(|(&idx, v)| (idx, v.distance))
            .collect()
    }

    /// Check if a voxel is in free space (positive distance, above threshold)
    pub fn is_free_space(&self, idx: Index64, threshold: f32) -> bool {
        self.voxels
            .get(&idx)
            .map(|v| v.distance > threshold)
            .unwrap_or(false)
    }

    /// Get gradient at voxel (for path planning)
    ///
    /// Returns normalized direction to nearest obstacle
    pub fn get_gradient(&self, idx: Index64) -> Option<(f32, f32, f32)> {
        let dist = self.get_distance(idx)?;
        let neighbors = neighbors_index64(idx);

        // Compute gradient using central differences
        let mut grad_x = 0.0;
        let mut grad_y = 0.0;
        let mut grad_z = 0.0;
        let mut count = 0;

        for neighbor_idx in neighbors {
            if let Some(neighbor_dist) = self.get_distance(neighbor_idx) {
                let (x1, y1, z1) = idx.decode_coords();
                let (x2, y2, z2) = neighbor_idx.decode_coords();

                let dx = (x2 as i32 - x1 as i32) as f32;
                let dy = (y2 as i32 - y1 as i32) as f32;
                let dz = (z2 as i32 - z1 as i32) as f32;

                let dd = neighbor_dist - dist;

                grad_x += dd * dx;
                grad_y += dd * dy;
                grad_z += dd * dz;
                count += 1;
            }
        }

        if count > 0 {
            grad_x /= count as f32;
            grad_y /= count as f32;
            grad_z /= count as f32;

            // Normalize
            let mag = (grad_x * grad_x + grad_y * grad_y + grad_z * grad_z).sqrt();
            if mag > 1e-6 {
                Some((grad_x / mag, grad_y / mag, grad_z / mag))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get ESDF statistics
    pub fn stats(&self) -> ESDFStats {
        let mut min_distance = f32::MAX;
        let mut max_distance = f32::MIN;
        let mut obstacle_voxels = 0;
        let mut free_voxels = 0;

        for voxel in self.voxels.values() {
            min_distance = min_distance.min(voxel.distance);
            max_distance = max_distance.max(voxel.distance);

            if voxel.distance < 0.0 {
                obstacle_voxels += 1;
            } else {
                free_voxels += 1;
            }
        }

        ESDFStats {
            voxel_count: self.voxels.len(),
            obstacle_voxel_count: obstacle_voxels,
            free_voxel_count: free_voxels,
            min_distance: if min_distance == f32::MAX {
                0.0
            } else {
                min_distance
            },
            max_distance: if max_distance == f32::MIN {
                0.0
            } else {
                max_distance
            },
        }
    }
}

/// ESDF statistics
#[derive(Debug, Clone)]
pub struct ESDFStats {
    /// Total number of voxels
    pub voxel_count: usize,
    /// Number of voxels inside obstacles (dist < 0)
    pub obstacle_voxel_count: usize,
    /// Number of voxels in free space (dist > 0)
    pub free_voxel_count: usize,
    /// Minimum distance value
    pub min_distance: f32,
    /// Maximum distance value
    pub max_distance: f32,
}

impl Layer for ESDFLayer {
    fn layer_type(&self) -> LayerType {
        LayerType::ESDF
    }

    fn update(&mut self, _idx: Index64, _measurement: &super::Measurement) -> Result<()> {
        // ESDF is typically computed from TSDF, not updated directly
        // But we could support incremental updates in the future
        Ok(())
    }

    fn query(&self, idx: Index64) -> Option<f32> {
        self.get_distance(idx)
    }

    fn voxel_count(&self) -> usize {
        self.voxels.len()
    }

    fn clear(&mut self) {
        self.voxels.clear();
    }

    fn memory_usage(&self) -> usize {
        // HashMap overhead + voxel data
        self.voxels.len() * 40
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::{Measurement, TSDFLayer};

    #[test]
    fn test_esdf_creation() {
        let esdf = ESDFLayer::new(0.02, 5.0);
        assert_eq!(esdf.voxel_count(), 0);
        assert_eq!(esdf.voxel_size(), 0.02);
        assert_eq!(esdf.max_distance(), 5.0);
    }

    #[test]
    fn test_esdf_from_tsdf() -> Result<()> {
        // Create simple TSDF with a surface
        let mut tsdf = TSDFLayer::new(0.1);

        // Add some surface voxels
        for i in 0..5 {
            let idx = Index64::new(0, 0, 5, 100 + i, 100, 100)?;
            tsdf.update(idx, &Measurement::depth(0.01, 1.0))?;
        }

        // Compute ESDF
        let mut esdf = ESDFLayer::new(0.02, 5.0);
        esdf.compute_from_tsdf(&tsdf, 0.05)?;

        // Should have some voxels
        assert!(esdf.voxel_count() > 0);

        let stats = esdf.stats();
        assert!(stats.voxel_count > 0);

        Ok(())
    }

    #[test]
    fn test_edge_lengths() {
        let edge_lengths = EdgeLengths::default();
        // Diagonal: √3 ≈ 1.732
        assert!((edge_lengths.diagonal - 1.732).abs() < 0.001);
        // Axial: 2.0
        assert_eq!(edge_lengths.axial, 2.0);
    }
}
