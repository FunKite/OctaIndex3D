//! Probabilistic Occupancy Layer with Bayesian Fusion
//!
//! Implements probabilistic occupancy mapping using log-odds representation
//! for efficient Bayesian updates on the BCC lattice.
//!
//! ## Algorithm
//!
//! Uses log-odds representation: L(x) = log(p(x) / (1 - p(x)))
//!
//! Benefits:
//! - Faster updates: addition instead of multiplication
//! - No numerical instability near 0 or 1
//! - Easy to implement with confidence weighting
//!
//! ## States
//!
//! - **Unknown**: No measurements yet (log-odds = 0)
//! - **Free**: Likely unoccupied (log-odds < threshold)
//! - **Occupied**: Likely occupied (log-odds > threshold)
//!
//! ## References
//!
//! - Hornung et al., "OctoMap: An Efficient Probabilistic 3D Mapping Framework" (2013)
//! - Moravec & Elfes, "High Resolution Maps from Wide Angle Sonar" (1985)

use super::measurement::MeasurementData;
use super::{Layer, LayerType, Measurement};
use crate::error::Result;
use crate::Index64;
use std::collections::HashMap;

/// Occupancy state classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OccupancyState {
    /// No measurements yet
    Unknown,
    /// Likely unoccupied (log-odds < -threshold)
    Free,
    /// Likely occupied (log-odds > threshold)
    Occupied,
}

/// Voxel occupancy data
#[derive(Debug, Clone, Copy)]
struct OccupancyVoxel {
    /// Log-odds value: log(p / (1-p))
    log_odds: f32,
    /// Number of measurements
    measurement_count: u32,
}

impl Default for OccupancyVoxel {
    fn default() -> Self {
        Self {
            log_odds: 0.0, // Unknown (p = 0.5)
            measurement_count: 0,
        }
    }
}

/// Probabilistic occupancy layer using log-odds Bayesian updates
///
/// This layer maintains probabilistic occupancy estimates for each voxel
/// using efficient log-odds representation. It's optimized for sensor fusion
/// from multiple noisy observations.
///
/// # Example
///
/// ```rust
/// use octaindex3d::layers::{OccupancyLayer, Layer, Measurement};
/// use octaindex3d::Index64;
///
/// # fn example() -> octaindex3d::Result<()> {
/// // Create occupancy layer
/// let mut occupancy = OccupancyLayer::new();
///
/// // Simulate sensor measurement (occupied)
/// let idx = Index64::new(0, 0, 5, 100, 200, 300)?;
/// let measurement = Measurement::occupied(0.9); // 90% confident occupied
///
/// // Update with Bayesian fusion
/// occupancy.update(idx, &measurement)?;
///
/// // Query state
/// let state = occupancy.get_state(idx);
/// let prob = occupancy.get_probability(idx);
/// # Ok(())
/// # }
/// ```
pub struct OccupancyLayer {
    /// Voxel data (sparse storage)
    voxels: HashMap<Index64, OccupancyVoxel>,

    /// Log-odds threshold for occupied classification
    /// Default: 0.85 ≈ log(5.67) (p ≈ 0.85)
    occupied_threshold: f32,

    /// Log-odds threshold for free classification
    /// Default: -0.85 ≈ log(0.176) (p ≈ 0.15)
    free_threshold: f32,

    /// Clamping limits to prevent saturation
    /// Default: ±3.5 (p ≈ 0.97 / 0.03)
    max_log_odds: f32,
    min_log_odds: f32,
}

impl OccupancyLayer {
    /// Create new occupancy layer with default parameters
    ///
    /// Default thresholds:
    /// - Occupied: p > 0.7 (log-odds > 0.85)
    /// - Free: p < 0.3 (log-odds < -0.85)
    /// - Clamping: p ∈ [0.03, 0.97] (log-odds ∈ [-3.5, 3.5])
    pub fn new() -> Self {
        Self {
            voxels: HashMap::new(),
            occupied_threshold: 0.85,
            free_threshold: -0.85,
            max_log_odds: 3.5,
            min_log_odds: -3.5,
        }
    }

    /// Create occupancy layer with custom thresholds
    ///
    /// # Arguments
    /// * `occupied_prob` - Probability threshold for occupied (0.5 - 1.0)
    /// * `free_prob` - Probability threshold for free (0.0 - 0.5)
    /// * `clamp_prob` - Clamping probability (prevents saturation)
    pub fn with_thresholds(occupied_prob: f32, free_prob: f32, clamp_prob: f32) -> Self {
        let occupied_threshold = prob_to_log_odds(occupied_prob);
        let free_threshold = prob_to_log_odds(free_prob);
        let max_log_odds = prob_to_log_odds(clamp_prob);
        let min_log_odds = prob_to_log_odds(1.0 - clamp_prob);

        Self {
            voxels: HashMap::new(),
            occupied_threshold,
            free_threshold,
            max_log_odds,
            min_log_odds,
        }
    }

    /// Update occupancy from sensor measurement using Bayesian fusion
    ///
    /// Log-odds update: L_new = L_old + L_measurement
    ///
    /// # Arguments
    /// * `idx` - Voxel index
    /// * `occupied` - True if sensor detected obstacle
    /// * `confidence` - Sensor confidence (0.5 - 1.0)
    pub fn update_occupancy(&mut self, idx: Index64, occupied: bool, confidence: f32) {
        let voxel = self.voxels.entry(idx).or_default();

        // Convert measurement to log-odds
        let measurement_log_odds = if occupied {
            prob_to_log_odds(confidence)
        } else {
            prob_to_log_odds(1.0 - confidence)
        };

        // Bayesian update (addition in log-odds space)
        let new_log_odds = voxel.log_odds + measurement_log_odds;

        // Clamp to prevent saturation
        voxel.log_odds = new_log_odds.clamp(self.min_log_odds, self.max_log_odds);
        voxel.measurement_count += 1;
    }

    /// Get occupancy state classification
    pub fn get_state(&self, idx: Index64) -> OccupancyState {
        match self.voxels.get(&idx) {
            None => OccupancyState::Unknown,
            Some(voxel) => {
                if voxel.log_odds > self.occupied_threshold {
                    OccupancyState::Occupied
                } else if voxel.log_odds < self.free_threshold {
                    OccupancyState::Free
                } else {
                    OccupancyState::Unknown
                }
            }
        }
    }

    /// Get occupancy probability (0.0 - 1.0)
    pub fn get_probability(&self, idx: Index64) -> Option<f32> {
        self.voxels
            .get(&idx)
            .map(|voxel| log_odds_to_prob(voxel.log_odds))
    }

    /// Get log-odds value directly
    pub fn get_log_odds(&self, idx: Index64) -> Option<f32> {
        self.voxels.get(&idx).map(|v| v.log_odds)
    }

    /// Get number of measurements for a voxel
    pub fn get_measurement_count(&self, idx: Index64) -> u32 {
        self.voxels
            .get(&idx)
            .map(|v| v.measurement_count)
            .unwrap_or(0)
    }

    /// Get all occupied voxels
    pub fn get_occupied_voxels(&self) -> Vec<Index64> {
        self.voxels
            .iter()
            .filter(|(_, v)| v.log_odds > self.occupied_threshold)
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Get all free voxels
    pub fn get_free_voxels(&self) -> Vec<Index64> {
        self.voxels
            .iter()
            .filter(|(_, v)| v.log_odds < self.free_threshold)
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Get all unknown voxels (observed but uncertain)
    pub fn get_unknown_voxels(&self) -> Vec<Index64> {
        self.voxels
            .iter()
            .filter(|(_, v)| {
                v.log_odds >= self.free_threshold && v.log_odds <= self.occupied_threshold
            })
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Get statistics about the occupancy layer
    pub fn stats(&self) -> OccupancyStats {
        let mut occupied_count = 0;
        let mut free_count = 0;
        let mut unknown_count = 0;
        let mut total_measurements = 0;

        for voxel in self.voxels.values() {
            total_measurements += voxel.measurement_count;

            if voxel.log_odds > self.occupied_threshold {
                occupied_count += 1;
            } else if voxel.log_odds < self.free_threshold {
                free_count += 1;
            } else {
                unknown_count += 1;
            }
        }

        OccupancyStats {
            total_voxels: self.voxels.len(),
            occupied_count,
            free_count,
            unknown_count,
            total_measurements,
        }
    }

    /// Ray casting for free space propagation
    ///
    /// Marks voxels along ray as free up to endpoint
    /// Used for integrating depth sensor measurements
    pub fn integrate_ray(
        &mut self,
        origin: (f32, f32, f32),
        endpoint: (f32, f32, f32),
        voxel_size: f32,
        free_confidence: f32,
        occupied_confidence: f32,
    ) -> Result<()> {
        use super::snap_to_nearest_bcc;

        // Ray direction and length
        let dx = endpoint.0 - origin.0;
        let dy = endpoint.1 - origin.1;
        let dz = endpoint.2 - origin.2;
        let ray_length = (dx * dx + dy * dy + dz * dz).sqrt();

        if ray_length < 1e-6 {
            return Ok(());
        }

        // Normalized direction
        let dir = (dx / ray_length, dy / ray_length, dz / ray_length);

        // Step size (half voxel for good coverage)
        let step_size = voxel_size * 0.5;
        let num_steps = (ray_length / step_size) as usize;

        // Mark free space along ray
        for i in 0..num_steps {
            let t = i as f32 * step_size;
            let pos = (
                origin.0 + dir.0 * t,
                origin.1 + dir.1 * t,
                origin.2 + dir.2 * t,
            );

            // Convert to BCC voxel coordinates
            let voxel_x = (pos.0 / voxel_size).round() as i32;
            let voxel_y = (pos.1 / voxel_size).round() as i32;
            let voxel_z = (pos.2 / voxel_size).round() as i32;

            let (vx, vy, vz) = snap_to_nearest_bcc(voxel_x, voxel_y, voxel_z);

            // Create index if valid
            if vx >= 0
                && vy >= 0
                && vz >= 0
                && vx <= u16::MAX as i32
                && vy <= u16::MAX as i32
                && vz <= u16::MAX as i32
            {
                if let Ok(idx) = Index64::new(0, 0, 5, vx as u16, vy as u16, vz as u16) {
                    self.update_occupancy(idx, false, free_confidence);
                }
            }
        }

        // Mark endpoint as occupied
        let end_voxel_x = (endpoint.0 / voxel_size).round() as i32;
        let end_voxel_y = (endpoint.1 / voxel_size).round() as i32;
        let end_voxel_z = (endpoint.2 / voxel_size).round() as i32;

        let (vx, vy, vz) = snap_to_nearest_bcc(end_voxel_x, end_voxel_y, end_voxel_z);

        if vx >= 0
            && vy >= 0
            && vz >= 0
            && vx <= u16::MAX as i32
            && vy <= u16::MAX as i32
            && vz <= u16::MAX as i32
        {
            if let Ok(idx) = Index64::new(0, 0, 5, vx as u16, vy as u16, vz as u16) {
                self.update_occupancy(idx, true, occupied_confidence);
            }
        }

        Ok(())
    }
}

impl Default for OccupancyLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for OccupancyLayer {
    fn layer_type(&self) -> LayerType {
        LayerType::Occupancy
    }

    fn update(&mut self, idx: Index64, measurement: &Measurement) -> Result<()> {
        match &measurement.data {
            MeasurementData::Occupancy { occupied } => {
                self.update_occupancy(idx, *occupied, measurement.confidence);
                Ok(())
            }
            _ => Ok(()), // Ignore non-occupancy measurements
        }
    }

    fn query(&self, idx: Index64) -> Option<f32> {
        self.get_probability(idx)
    }

    fn voxel_count(&self) -> usize {
        self.voxels.len()
    }

    fn clear(&mut self) {
        self.voxels.clear();
    }

    fn memory_usage(&self) -> usize {
        // Each entry: Index64 (8 bytes) + OccupancyVoxel (8 bytes) + HashMap overhead (~24 bytes)
        self.voxels.len() * 40
    }
}

/// Statistics about occupancy layer
#[derive(Debug, Clone)]
pub struct OccupancyStats {
    pub total_voxels: usize,
    pub occupied_count: usize,
    pub free_count: usize,
    pub unknown_count: usize,
    pub total_measurements: u32,
}

/// Convert probability to log-odds
///
/// L = log(p / (1-p))
#[inline]
fn prob_to_log_odds(prob: f32) -> f32 {
    let p = prob.clamp(0.001, 0.999); // Prevent division by zero
    (p / (1.0 - p)).ln()
}

/// Convert log-odds to probability
///
/// p = 1 / (1 + exp(-L))
#[inline]
fn log_odds_to_prob(log_odds: f32) -> f32 {
    1.0 / (1.0 + (-log_odds).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_odds_conversion() {
        // Test known values
        assert!((prob_to_log_odds(0.5) - 0.0).abs() < 1e-5); // p=0.5 → L=0
        assert!((log_odds_to_prob(0.0) - 0.5).abs() < 1e-5); // L=0 → p=0.5

        // Round trip
        let p = 0.7;
        let l = prob_to_log_odds(p);
        let p2 = log_odds_to_prob(l);
        assert!((p - p2).abs() < 1e-5);
    }

    #[test]
    fn test_occupancy_layer_creation() {
        let layer = OccupancyLayer::new();
        assert_eq!(layer.voxel_count(), 0);

        let stats = layer.stats();
        assert_eq!(stats.total_voxels, 0);
        assert_eq!(stats.occupied_count, 0);
        assert_eq!(stats.free_count, 0);
    }

    #[test]
    fn test_occupancy_update() -> Result<()> {
        let mut layer = OccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 200, 300)?;

        // Initial state: unknown
        assert_eq!(layer.get_state(idx), OccupancyState::Unknown);

        // Update as occupied with high confidence
        layer.update_occupancy(idx, true, 0.9);
        assert_eq!(layer.get_state(idx), OccupancyState::Occupied);

        // Check probability increased
        let prob = layer.get_probability(idx).unwrap();
        assert!(prob > 0.5);

        Ok(())
    }

    #[test]
    fn test_bayesian_fusion() -> Result<()> {
        let mut layer = OccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 200, 300)?;

        // Multiple weak occupied measurements
        for _ in 0..5 {
            layer.update_occupancy(idx, true, 0.6); // Weak confidence
        }

        // Should converge to occupied
        assert_eq!(layer.get_state(idx), OccupancyState::Occupied);

        // Counter with free measurements
        for _ in 0..10 {
            layer.update_occupancy(idx, false, 0.6);
        }

        // Should flip to free
        assert_eq!(layer.get_state(idx), OccupancyState::Free);

        Ok(())
    }

    #[test]
    fn test_measurement_trait() -> Result<()> {
        use crate::layers::Layer;

        let mut layer = OccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 200, 300)?;

        let measurement = Measurement::occupied(0.9);
        layer.update(idx, &measurement)?;

        assert_eq!(layer.get_state(idx), OccupancyState::Occupied);

        Ok(())
    }

    #[test]
    fn test_state_counts() -> Result<()> {
        let mut layer = OccupancyLayer::new();

        // Create occupied voxels
        for i in 0..10 {
            let idx = Index64::new(0, 0, 5, i * 10, 100, 100)?;
            layer.update_occupancy(idx, true, 0.9);
        }

        // Create free voxels
        for i in 0..5 {
            let idx = Index64::new(0, 0, 5, i * 10, 200, 100)?;
            layer.update_occupancy(idx, false, 0.9);
        }

        let stats = layer.stats();
        assert_eq!(stats.occupied_count, 10);
        assert_eq!(stats.free_count, 5);

        Ok(())
    }

    #[test]
    fn test_ray_integration() -> Result<()> {
        let mut layer = OccupancyLayer::new();
        let voxel_size = 0.1;

        // Cast ray from origin to point
        let origin = (0.0, 0.0, 0.0);
        let endpoint = (1.0, 1.0, 1.0);

        layer.integrate_ray(origin, endpoint, voxel_size, 0.7, 0.9)?;

        // Should have some free voxels along ray
        let stats = layer.stats();
        assert!(stats.free_count > 0);

        // Endpoint should be occupied
        assert!(stats.occupied_count > 0);

        Ok(())
    }
}
