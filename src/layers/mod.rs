//! Multi-layer spatial data on BCC lattice
//!
//! This module provides a flexible multi-layer architecture for storing different
//! types of spatial data (TSDF, ESDF, Occupancy, Color, etc.) on the same BCC lattice.
//!
//! ## Architecture
//!
//! - **Layer**: Trait for different data types (TSDF, ESDF, Occupancy, etc.)
//! - **LayeredMap**: Container for multiple layers sharing the same spatial index
//! - **Measurement**: Sensor observations (depth, RGB, intensity, etc.)
//!
//! ## Example
//!
//! ```rust
//! use octaindex3d::layers::{LayeredMap, TSDFLayer, Measurement};
//! use octaindex3d::Index64;
//!
//! # fn main() -> octaindex3d::Result<()> {
//! // Create a multi-layer map
//! let mut map = LayeredMap::new();
//!
//! // Add a TSDF layer for surface reconstruction
//! let tsdf = TSDFLayer::new(0.1); // 10cm truncation distance
//! map.add_tsdf_layer(tsdf);
//!
//! // Simulate depth sensor measurement
//! let idx = Index64::new(0, 0, 5, 100, 200, 300)?;
//! let measurement = Measurement::depth(2.5, 1.0); // 2.5m distance, confidence 1.0
//!
//! // Update TSDF
//! map.update_tsdf(idx, &measurement)?;
//! # Ok(())
//! # }
//! ```

pub mod bcc_utils;
pub mod esdf;
pub mod exploration;
pub mod export;
pub mod measurement;
pub mod mesh;
pub mod occupancy;
pub mod occupancy_compressed;
pub mod occupancy_gpu;
pub mod occupancy_temporal;
pub mod ros2_bridge;
pub mod tsdf;

pub use bcc_utils::{is_valid_bcc, physical_to_bcc_voxel, snap_to_nearest_bcc};
pub use esdf::ESDFLayer;
pub use exploration::{Frontier, FrontierDetectionConfig, InformationGainConfig, Viewpoint};
pub use export::{export_mesh_obj, export_mesh_ply, export_mesh_stl};
pub use measurement::{Measurement, MeasurementType};
pub use mesh::{extract_mesh_from_tsdf, Mesh, MeshStats, Triangle, Vertex};
pub use occupancy::{OccupancyLayer, OccupancyState, OccupancyStats};
pub use occupancy_compressed::{CompressedOccupancyLayer, CompressionMethod, CompressionStats};
pub use occupancy_temporal::{TemporalConfig, TemporalOccupancyLayer, TemporalStats};
pub use tsdf::TSDFLayer;

// Re-export ROS2 types
pub mod ros2 {
    pub use super::ros2_bridge::*;
}

// Re-export GPU types (conditionally)
#[cfg(any(feature = "gpu-metal", feature = "gpu-cuda"))]
pub mod gpu {
    pub use super::occupancy_gpu::*;
}

use crate::error::{Error, Result};
use crate::Index64;
use std::collections::HashMap;

/// Layer type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerType {
    /// Truncated Signed Distance Field (surface reconstruction)
    TSDF,
    /// Euclidean Signed Distance Field (path planning)
    ESDF,
    /// Probabilistic occupancy (sensor fusion)
    Occupancy,
    /// RGB color information
    Color,
    /// Intensity (LiDAR)
    Intensity,
}

impl LayerType {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            LayerType::TSDF => "TSDF",
            LayerType::ESDF => "ESDF",
            LayerType::Occupancy => "Occupancy",
            LayerType::Color => "Color",
            LayerType::Intensity => "Intensity",
        }
    }
}

/// Generic layer trait for spatial data
pub trait Layer: Send + Sync {
    /// Get the layer type
    fn layer_type(&self) -> LayerType;

    /// Update a voxel from sensor measurement
    fn update(&mut self, idx: Index64, measurement: &Measurement) -> Result<()>;

    /// Query voxel value (returns None if voxel not observed)
    fn query(&self, idx: Index64) -> Option<f32>;

    /// Check if voxel has been observed
    fn contains(&self, idx: Index64) -> bool {
        self.query(idx).is_some()
    }

    /// Get number of voxels in this layer
    fn voxel_count(&self) -> usize;

    /// Clear all data
    fn clear(&mut self);

    /// Get memory usage in bytes (approximate)
    fn memory_usage(&self) -> usize;
}

/// Multi-layer spatial map on BCC lattice
///
/// Stores multiple data layers (TSDF, ESDF, Occupancy, etc.) on the same
/// BCC lattice structure, sharing the spatial indexing infrastructure.
#[derive(Default)]
pub struct LayeredMap {
    /// Active layers
    layers: HashMap<LayerType, Box<dyn Layer>>,
}

impl LayeredMap {
    /// Create a new empty layered map
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
        }
    }

    /// Add a TSDF layer for surface reconstruction
    pub fn add_tsdf_layer(&mut self, layer: TSDFLayer) {
        self.layers.insert(LayerType::TSDF, Box::new(layer));
    }

    /// Add an ESDF layer for path planning
    pub fn add_esdf_layer(&mut self, layer: ESDFLayer) {
        self.layers.insert(LayerType::ESDF, Box::new(layer));
    }

    /// Add an Occupancy layer for probabilistic sensor fusion
    pub fn add_occupancy_layer(&mut self, layer: OccupancyLayer) {
        self.layers.insert(LayerType::Occupancy, Box::new(layer));
    }

    /// Get reference to TSDF layer
    ///
    /// Note: This returns None if the layer doesn't exist.
    /// Access to TSDF-specific methods requires getting the layer separately.
    pub fn has_tsdf_layer(&self) -> bool {
        self.layers.contains_key(&LayerType::TSDF)
    }

    /// Get TSDF layer (consumes the map, returns layer)
    /// For most use cases, use update_tsdf and query_tsdf instead
    pub fn take_tsdf_layer(&mut self) -> Option<TSDFLayer> {
        self.layers.remove(&LayerType::TSDF).map(|boxed| {
            // SAFETY: We know this is a TSDFLayer because LayerType::TSDF
            // can only be inserted via add_tsdf_layer
            let any = Box::into_raw(boxed) as *mut TSDFLayer;
            unsafe { *Box::from_raw(any) }
        })
    }

    /// Update TSDF layer with measurement
    pub fn update_tsdf(&mut self, idx: Index64, measurement: &Measurement) -> Result<()> {
        match self.layers.get_mut(&LayerType::TSDF) {
            Some(layer) => layer.update(idx, measurement),
            None => Err(Error::InvalidFormat(
                "TSDF layer not initialized".to_string(),
            )),
        }
    }

    /// Query TSDF distance value
    pub fn query_tsdf(&self, idx: Index64) -> Option<f32> {
        self.layers
            .get(&LayerType::TSDF)
            .and_then(|layer| layer.query(idx))
    }

    /// Query ESDF distance value
    pub fn query_esdf(&self, idx: Index64) -> Option<f32> {
        self.layers
            .get(&LayerType::ESDF)
            .and_then(|layer| layer.query(idx))
    }

    /// Update Occupancy layer with measurement
    pub fn update_occupancy(&mut self, idx: Index64, measurement: &Measurement) -> Result<()> {
        match self.layers.get_mut(&LayerType::Occupancy) {
            Some(layer) => layer.update(idx, measurement),
            None => Err(Error::InvalidFormat(
                "Occupancy layer not initialized".to_string(),
            )),
        }
    }

    /// Query Occupancy probability value
    pub fn query_occupancy(&self, idx: Index64) -> Option<f32> {
        self.layers
            .get(&LayerType::Occupancy)
            .and_then(|layer| layer.query(idx))
    }

    /// Check if a layer exists
    pub fn has_layer(&self, layer_type: LayerType) -> bool {
        self.layers.contains_key(&layer_type)
    }

    /// Remove a layer
    pub fn remove_layer(&mut self, layer_type: LayerType) -> Option<Box<dyn Layer>> {
        self.layers.remove(&layer_type)
    }

    /// Get all active layer types
    pub fn layer_types(&self) -> Vec<LayerType> {
        self.layers.keys().copied().collect()
    }

    /// Get total voxel count across all layers
    pub fn total_voxels(&self) -> usize {
        self.layers.values().map(|l| l.voxel_count()).sum()
    }

    /// Get total memory usage across all layers (bytes)
    pub fn total_memory_usage(&self) -> usize {
        self.layers.values().map(|l| l.memory_usage()).sum()
    }

    /// Clear all layers
    pub fn clear(&mut self) {
        for layer in self.layers.values_mut() {
            layer.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layered_map_creation() {
        let map = LayeredMap::new();
        assert_eq!(map.layer_types().len(), 0);
        assert_eq!(map.total_voxels(), 0);
    }

    #[test]
    fn test_layer_type_names() {
        assert_eq!(LayerType::TSDF.name(), "TSDF");
        assert_eq!(LayerType::ESDF.name(), "ESDF");
        assert_eq!(LayerType::Occupancy.name(), "Occupancy");
    }
}
