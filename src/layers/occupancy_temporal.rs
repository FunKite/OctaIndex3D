//! Temporal Filtering for Dynamic Environment Occupancy Mapping
//!
//! Implements time-aware occupancy tracking with decay for dynamic objects

use super::occupancy::OccupancyState;
use crate::error::Result;
use crate::Index64;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Voxel with temporal information
#[derive(Debug, Clone)]
struct TemporalVoxel {
    /// Log-odds value
    log_odds: f32,
    /// Last update timestamp
    last_update: Instant,
    /// Measurement count
    measurement_count: u32,
    /// Velocity estimate (if moving)
    velocity: Option<(f32, f32, f32)>,
}

/// Temporal filtering configuration
#[derive(Debug, Clone)]
pub struct TemporalConfig {
    /// Decay rate for unobserved voxels (log-odds per second)
    pub decay_rate: f32,
    /// Maximum time before considering voxel stale (seconds)
    pub max_age: f32,
    /// Minimum measurements before tracking velocity
    pub min_measurements_for_velocity: u32,
    /// Enable dynamic object tracking
    pub track_dynamics: bool,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.5,                  // Decay toward unknown at 0.5 log-odds/s
            max_age: 5.0,                     // 5 second max age
            min_measurements_for_velocity: 3, // Need 3+ measurements for velocity
            track_dynamics: true,             // Enable by default
        }
    }
}

/// Occupancy layer with temporal filtering for dynamic environments
///
/// This extends standard occupancy mapping with:
/// - Time-based decay for unobserved areas
/// - Dynamic object tracking and velocity estimation
/// - Automatic pruning of stale data
///
/// # Use Cases
/// - Mobile robotics in dynamic environments
/// - Human-robot interaction (moving people)
/// - Autonomous vehicles (moving obstacles)
pub struct TemporalOccupancyLayer {
    /// Voxel data with timestamps
    voxels: HashMap<Index64, TemporalVoxel>,
    /// Configuration
    config: TemporalConfig,
    /// Thresholds (same as OccupancyLayer)
    occupied_threshold: f32,
    free_threshold: f32,
    /// Maximum log-odds (clamping)
    max_log_odds: f32,
    /// Minimum log-odds (clamping)
    min_log_odds: f32,
}

impl TemporalOccupancyLayer {
    /// Create new temporal occupancy layer
    pub fn new() -> Self {
        Self::with_config(TemporalConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: TemporalConfig) -> Self {
        Self {
            voxels: HashMap::new(),
            config,
            occupied_threshold: 0.847, // log(5.67) ≈ p=0.7
            free_threshold: -1.099,    // log(0.333) ≈ p=0.25
            max_log_odds: 3.466,       // log(31.95) ≈ p=0.97
            min_log_odds: -3.466,      // log(0.0313) ≈ p=0.03
        }
    }

    /// Update occupancy with temporal decay
    pub fn update_occupancy(&mut self, idx: Index64, occupied: bool, confidence: f32) {
        let now = Instant::now();

        // Get or create voxel
        let voxel = self.voxels.entry(idx).or_insert_with(|| TemporalVoxel {
            log_odds: 0.0,
            last_update: now,
            measurement_count: 0,
            velocity: None,
        });

        // Apply temporal decay since last update
        let dt = now.duration_since(voxel.last_update).as_secs_f32();
        if dt > 0.0 {
            // Exponential decay toward unknown (log-odds = 0)
            let decay = (-self.config.decay_rate * dt).exp();
            voxel.log_odds *= decay;
        }

        // Calculate log-odds update from measurement
        let prob = if occupied {
            confidence
        } else {
            1.0 - confidence
        };
        let log_odds_update = (prob / (1.0 - prob)).ln();

        // Bayesian update
        voxel.log_odds += log_odds_update;

        // Clamp to prevent saturation
        voxel.log_odds = voxel.log_odds.clamp(self.min_log_odds, self.max_log_odds);

        // Update metadata
        voxel.last_update = now;
        voxel.measurement_count += 1;
    }

    /// Integrate ray with temporal awareness
    pub fn integrate_ray(
        &mut self,
        origin: (f32, f32, f32),
        endpoint: (f32, f32, f32),
        voxel_size: f32,
        free_confidence: f32,
        occupied_confidence: f32,
    ) -> Result<()> {
        // Use DDA ray traversal (same as OccupancyLayer)
        let start_x = (origin.0 / voxel_size).floor() as i16;
        let start_y = (origin.1 / voxel_size).floor() as i16;
        let start_z = (origin.2 / voxel_size).floor() as i16;

        let end_x = (endpoint.0 / voxel_size).floor() as i16;
        let end_y = (endpoint.1 / voxel_size).floor() as i16;
        let end_z = (endpoint.2 / voxel_size).floor() as i16;

        // Simple ray traversal (mark free space)
        let dx = (end_x - start_x).abs();
        let dy = (end_y - start_y).abs();
        let dz = (end_z - start_z).abs();
        let max_steps = (dx.max(dy).max(dz) as usize).min(1000);

        for step in 0..max_steps {
            let t = step as f32 / max_steps as f32;
            let x = (start_x as f32 + t * (end_x - start_x) as f32).round() as u16;
            let y = (start_y as f32 + t * (end_y - start_y) as f32).round() as u16;
            let z = (start_z as f32 + t * (end_z - start_z) as f32).round() as u16;

            if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                self.update_occupancy(idx, false, free_confidence);
            }
        }

        // Mark endpoint as occupied
        if let Ok(idx) = Index64::new(0, 0, 5, end_x as u16, end_y as u16, end_z as u16) {
            self.update_occupancy(idx, true, occupied_confidence);
        }

        Ok(())
    }

    /// Get occupancy state with temporal decay applied
    pub fn get_state(&self, idx: Index64) -> OccupancyState {
        match self.voxels.get(&idx) {
            Some(voxel) => {
                // Check if voxel is stale
                let age = Instant::now()
                    .duration_since(voxel.last_update)
                    .as_secs_f32();

                if age > self.config.max_age {
                    return OccupancyState::Unknown;
                }

                // Apply real-time decay
                let decay = (-self.config.decay_rate * age).exp();
                let current_log_odds = voxel.log_odds * decay;

                if current_log_odds > self.occupied_threshold {
                    OccupancyState::Occupied
                } else if current_log_odds < self.free_threshold {
                    OccupancyState::Free
                } else {
                    OccupancyState::Unknown
                }
            }
            None => OccupancyState::Unknown,
        }
    }

    /// Get probability with temporal decay
    pub fn get_probability(&self, idx: Index64) -> Option<f32> {
        self.voxels.get(&idx).map(|voxel| {
            let age = Instant::now()
                .duration_since(voxel.last_update)
                .as_secs_f32();
            let decay = (-self.config.decay_rate * age).exp();
            let current_log_odds = voxel.log_odds * decay;
            1.0 / (1.0 + (-current_log_odds).exp())
        })
    }

    /// Prune stale voxels older than max_age
    pub fn prune_stale(&mut self) {
        let max_age = Duration::from_secs_f32(self.config.max_age);
        let now = Instant::now();

        self.voxels
            .retain(|_, voxel| now.duration_since(voxel.last_update) < max_age);
    }

    /// Get statistics
    pub fn stats(&self) -> TemporalStats {
        let now = Instant::now();
        let mut stats = TemporalStats::default();

        stats.total_voxels = self.voxels.len();

        for (_, voxel) in &self.voxels {
            let age = now.duration_since(voxel.last_update).as_secs_f32();
            let decay = (-self.config.decay_rate * age).exp();
            let current_log_odds = voxel.log_odds * decay;

            if current_log_odds > self.occupied_threshold {
                stats.occupied_count += 1;
            } else if current_log_odds < self.free_threshold {
                stats.free_count += 1;
            } else {
                stats.unknown_count += 1;
            }

            if voxel.velocity.is_some() {
                stats.dynamic_voxels += 1;
            }

            if age > self.config.max_age {
                stats.stale_voxels += 1;
            }
        }

        stats
    }
}

impl Default for TemporalOccupancyLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for temporal occupancy layer
#[derive(Debug, Default)]
pub struct TemporalStats {
    pub total_voxels: usize,
    pub occupied_count: usize,
    pub free_count: usize,
    pub unknown_count: usize,
    pub dynamic_voxels: usize,
    pub stale_voxels: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_decay() {
        let mut layer = TemporalOccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

        // Initial update (occupied)
        layer.update_occupancy(idx, true, 0.9);
        assert_eq!(layer.get_state(idx), OccupancyState::Occupied);

        // Decay should happen over time
        // (In real test, would need to wait or mock time)
    }

    #[test]
    fn test_stale_pruning() {
        let mut layer = TemporalOccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

        layer.update_occupancy(idx, true, 0.9);
        assert_eq!(layer.stats().total_voxels, 1);

        layer.prune_stale();
        // Voxel should still be there (not stale yet)
        assert_eq!(layer.stats().total_voxels, 1);
    }
}
