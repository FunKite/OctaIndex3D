//! Truncated Signed Distance Field (TSDF) layer
//!
//! Implements incremental TSDF integration following Curless & Levoy 1996
//! with optimizations for BCC lattice structure.
//!
//! ## Algorithm
//!
//! The TSDF stores:
//! - **Distance**: Signed distance to nearest surface (negative inside, positive outside)
//! - **Weight**: Cumulative confidence for incremental averaging
//!
//! Each depth measurement updates nearby voxels using running weighted average:
//! ```text
//! new_distance = (old_distance * old_weight + sdf_value * measurement_weight) / (old_weight + measurement_weight)
//! new_weight = old_weight + measurement_weight
//! ```
//!
//! ## Advantages on BCC Lattice
//!
//! 1. **Better isotropy**: 14-neighbor connectivity gives more uniform distance field
//! 2. **Efficient storage**: Morton encoding enables fast spatial queries
//! 3. **Natural hierarchy**: Parent-child relationships for multi-resolution

use super::{Layer, LayerType, Measurement, MeasurementType};
use crate::error::{Error, Result};
use crate::Index64;
use std::collections::HashMap;

/// Voxel data in TSDF layer
#[derive(Debug, Clone, Copy)]
struct TSDFVoxel {
    /// Signed distance value (truncated)
    distance: f32,
    /// Cumulative weight for averaging
    weight: f32,
}

impl Default for TSDFVoxel {
    fn default() -> Self {
        Self {
            distance: 0.0,
            weight: 0.0,
        }
    }
}

/// Truncated Signed Distance Field layer
///
/// Stores signed distance to nearest surface with incremental updates.
/// Optimized for BCC lattice with 14-neighbor connectivity.
pub struct TSDFLayer {
    /// Voxel data indexed by Index64
    voxels: HashMap<Index64, TSDFVoxel>,

    /// Truncation distance (meters)
    /// Voxels farther than this from surface are not updated
    truncation_distance: f32,

    /// Maximum weight per voxel (prevents old data from dominating)
    max_weight: f32,

    /// Voxel size (meters per voxel)
    voxel_size: f32,
}

impl TSDFLayer {
    /// Create a new TSDF layer
    ///
    /// # Arguments
    /// * `truncation_distance` - Maximum distance from surface to integrate (meters)
    ///
    /// # Example
    /// ```
    /// use octaindex3d::layers::TSDFLayer;
    ///
    /// let tsdf = TSDFLayer::new(0.1); // 10cm truncation
    /// ```
    pub fn new(truncation_distance: f32) -> Self {
        Self {
            voxels: HashMap::new(),
            truncation_distance,
            max_weight: 100.0,
            voxel_size: 0.02, // Default 2cm voxels
        }
    }

    /// Create TSDF layer with custom parameters
    pub fn with_params(truncation_distance: f32, max_weight: f32, voxel_size: f32) -> Self {
        Self {
            voxels: HashMap::new(),
            truncation_distance,
            max_weight,
            voxel_size,
        }
    }

    /// Set voxel size
    pub fn set_voxel_size(&mut self, size: f32) {
        self.voxel_size = size;
    }

    /// Get voxel size
    pub fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    /// Get truncation distance
    pub fn truncation_distance(&self) -> f32 {
        self.truncation_distance
    }

    /// Update TSDF from depth measurement
    ///
    /// Implements Curless & Levoy volumetric integration.
    /// The depth measurement provides a signed distance to the surface.
    /// For simple cases, this is the direct SDF value. For camera-based updates,
    /// use `update_from_depth_ray` instead.
    fn update_from_depth(&mut self, idx: Index64, sdf_value: f32, confidence: f32) -> Result<()> {
        // Only update if within truncation distance
        if sdf_value.abs() > self.truncation_distance {
            return Ok(());
        }

        // Truncate SDF value
        let truncated_sdf = sdf_value.clamp(-self.truncation_distance, self.truncation_distance);

        // Get or create voxel
        let voxel = self.voxels.entry(idx).or_default();

        // Incremental weighted average (Curless & Levoy 1996)
        let new_weight = (voxel.weight + confidence).min(self.max_weight);
        let new_distance = if voxel.weight > 0.0 {
            (voxel.distance * voxel.weight + truncated_sdf * confidence) / new_weight
        } else {
            truncated_sdf
        };

        voxel.distance = new_distance;
        voxel.weight = new_weight;

        Ok(())
    }

    /// Update TSDF from depth measurement with camera ray
    ///
    /// Computes SDF based on voxel position relative to sensor and measured depth.
    pub fn update_from_depth_ray(
        &mut self,
        idx: Index64,
        sensor_pos: (f32, f32, f32),
        ray_depth: f32,
        confidence: f32,
    ) -> Result<()> {
        // Get voxel position in world space
        let (x, y, z) = idx.decode_coords();
        let voxel_pos = (
            x as f32 * self.voxel_size,
            y as f32 * self.voxel_size,
            z as f32 * self.voxel_size,
        );

        // Compute distance from voxel to sensor
        let dx = voxel_pos.0 - sensor_pos.0;
        let dy = voxel_pos.1 - sensor_pos.1;
        let dz = voxel_pos.2 - sensor_pos.2;
        let voxel_distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Compute signed distance: positive in front of surface, negative behind
        let sdf_value = ray_depth - voxel_distance;

        // Update using standard method
        self.update_from_depth(idx, sdf_value, confidence)
    }

    /// Get distance value for a voxel
    pub fn get_distance(&self, idx: Index64) -> Option<f32> {
        self.voxels.get(&idx).map(|v| v.distance)
    }

    /// Get weight for a voxel
    pub fn get_weight(&self, idx: Index64) -> Option<f32> {
        self.voxels.get(&idx).map(|v| v.weight)
    }

    /// Check if voxel is near surface (distance close to zero)
    pub fn is_surface_voxel(&self, idx: Index64, threshold: f32) -> bool {
        self.get_distance(idx)
            .map(|d| d.abs() < threshold)
            .unwrap_or(false)
    }

    /// Get all voxels near surface (for mesh extraction)
    pub fn get_surface_voxels(&self, threshold: f32) -> Vec<Index64> {
        self.voxels
            .iter()
            .filter(|(_, v)| v.distance.abs() < threshold && v.weight > 0.0)
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Get all zero-crossing edges (where sign changes between neighbors)
    ///
    /// Returns pairs of (voxel1, voxel2) where distance changes sign.
    /// These edges intersect the surface.
    pub fn get_zero_crossing_edges(&self) -> Vec<(Index64, Index64)> {
        use crate::neighbors::neighbors_index64;

        let mut edges = Vec::new();

        for (&idx, voxel) in &self.voxels {
            if voxel.weight == 0.0 {
                continue;
            }

            // Check all 14 neighbors
            let neighbors = neighbors_index64(idx);
            for neighbor_idx in neighbors {
                if let Some(neighbor_voxel) = self.voxels.get(&neighbor_idx) {
                    if neighbor_voxel.weight == 0.0 {
                        continue;
                    }

                    // Check for sign change (zero crossing)
                    if voxel.distance * neighbor_voxel.distance < 0.0 {
                        // Only add each edge once (idx < neighbor to avoid duplicates)
                        if idx.raw() < neighbor_idx.raw() {
                            edges.push((idx, neighbor_idx));
                        }
                    }
                }
            }
        }

        edges
    }

    /// Batch update from multiple depth measurements
    ///
    /// More efficient than individual updates when processing large point clouds.
    pub fn batch_update(&mut self, updates: &[(Index64, f32, f32)]) -> Result<()> {
        for &(idx, distance, confidence) in updates {
            self.update_from_depth(idx, distance, confidence)?;
        }
        Ok(())
    }

    /// Get statistics about the TSDF
    pub fn stats(&self) -> TSDFStats {
        let mut min_distance = f32::MAX;
        let mut max_distance = f32::MIN;
        let mut total_weight = 0.0;
        let mut surface_voxels = 0;

        for voxel in self.voxels.values() {
            if voxel.weight > 0.0 {
                min_distance = min_distance.min(voxel.distance);
                max_distance = max_distance.max(voxel.distance);
                total_weight += voxel.weight;

                if voxel.distance.abs() < self.voxel_size {
                    surface_voxels += 1;
                }
            }
        }

        TSDFStats {
            voxel_count: self.voxels.len(),
            surface_voxel_count: surface_voxels,
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
            average_weight: if self.voxels.is_empty() {
                0.0
            } else {
                total_weight / self.voxels.len() as f32
            },
        }
    }
}

/// TSDF statistics
#[derive(Debug, Clone)]
pub struct TSDFStats {
    /// Total number of voxels
    pub voxel_count: usize,
    /// Number of voxels near surface
    pub surface_voxel_count: usize,
    /// Minimum distance value
    pub min_distance: f32,
    /// Maximum distance value
    pub max_distance: f32,
    /// Average weight across all voxels
    pub average_weight: f32,
}

impl Layer for TSDFLayer {
    fn layer_type(&self) -> LayerType {
        LayerType::TSDF
    }

    fn update(&mut self, idx: Index64, measurement: &Measurement) -> Result<()> {
        match measurement.measurement_type {
            MeasurementType::Depth => {
                let distance = measurement.as_depth()?;
                self.update_from_depth(idx, distance, measurement.confidence)
            }
            _ => Err(Error::InvalidFormat(
                "TSDF layer requires depth measurements".to_string(),
            )),
        }
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
        // Rough estimate: 24 bytes per entry (HashMap overhead) + 8 bytes (Index64) + 8 bytes (TSDFVoxel)
        self.voxels.len() * 40
    }
}

// Helper function for neighbors - we need to add this to neighbors module
#[cfg(not(test))]
mod neighbors_ext {
    use crate::Index64;

    #[allow(dead_code)]
    pub fn neighbors_index64(_idx: Index64) -> Vec<Index64> {
        // Placeholder - will be implemented in src/neighbors.rs
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Measurement;

    #[test]
    fn test_tsdf_creation() {
        let tsdf = TSDFLayer::new(0.1);
        assert_eq!(tsdf.voxel_count(), 0);
        assert_eq!(tsdf.truncation_distance(), 0.1);
    }

    #[test]
    fn test_tsdf_update() -> Result<()> {
        let mut tsdf = TSDFLayer::new(0.1);
        let idx = Index64::new(0, 0, 5, 0, 0, 0)?;
        // Use SDF value within truncation range
        let measurement = Measurement::depth(0.05, 1.0);

        tsdf.update(idx, &measurement)?;

        // Voxel should exist now
        assert!(tsdf.contains(idx));
        assert_eq!(tsdf.voxel_count(), 1);

        Ok(())
    }

    #[test]
    fn test_tsdf_incremental_update() -> Result<()> {
        let mut tsdf = TSDFLayer::new(0.1);
        let idx = Index64::new(0, 0, 5, 0, 0, 0)?;

        // First measurement (SDF value within truncation)
        tsdf.update_from_depth(idx, 0.05, 1.0)?;
        let dist1 = tsdf.get_distance(idx).unwrap();
        let weight1 = tsdf.get_weight(idx).unwrap();

        // Second measurement (slightly different distance)
        tsdf.update_from_depth(idx, 0.03, 1.0)?;
        let dist2 = tsdf.get_distance(idx).unwrap();
        let weight2 = tsdf.get_weight(idx).unwrap();

        // Weight should increase
        assert!(weight2 > weight1);
        // Distance should be averaged
        assert!(dist2 != dist1);
        assert!((dist2 - 0.04).abs() < 0.01); // Should average to ~0.04

        Ok(())
    }

    #[test]
    fn test_tsdf_stats() -> Result<()> {
        let mut tsdf = TSDFLayer::new(0.1);

        // Add some voxels with SDF values within truncation
        for i in 0..10 {
            let idx = Index64::new(0, 0, 5, i, 0, 0)?;
            tsdf.update_from_depth(idx, 0.05, 1.0)?;
        }

        let stats = tsdf.stats();
        assert_eq!(stats.voxel_count, 10);
        assert!(stats.average_weight > 0.0);

        Ok(())
    }

    #[test]
    fn test_max_weight_clamping() -> Result<()> {
        let mut tsdf = TSDFLayer::with_params(0.1, 5.0, 0.02);
        let idx = Index64::new(0, 0, 5, 0, 0, 0)?;

        // Add many measurements to exceed max_weight
        for _ in 0..20 {
            tsdf.update_from_depth(idx, 0.05, 1.0)?;
        }

        let weight = tsdf.get_weight(idx).unwrap();
        assert!(weight <= 5.0, "Weight should be clamped to max_weight");

        Ok(())
    }

    #[test]
    fn test_wrong_measurement_type() -> Result<()> {
        let mut tsdf = TSDFLayer::new(0.1);
        let idx = Index64::new(0, 0, 5, 0, 0, 0)?;
        let wrong_measurement = Measurement::occupied(1.0);

        let result = tsdf.update(idx, &wrong_measurement);
        assert!(result.is_err());

        Ok(())
    }
}
