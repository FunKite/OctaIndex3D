//! Frontier Detection and Information Gain Primitives
//!
//! Provides building blocks for autonomous exploration:
//! - Frontier detection (boundaries between known and unknown)
//! - Information gain estimation from viewpoints
//! - Viewpoint candidate generation
//!
//! These primitives enable users to implement exploration strategies
//! like Next-Best-View (NBV) planning without prescribing a specific policy.

use super::occupancy::OccupancyLayer;
use crate::error::Result;
use crate::Index64;
use std::collections::{HashSet, VecDeque};

/// A frontier: boundary between explored and unexplored space
///
/// Frontiers represent regions where the robot has partial knowledge
/// and could gain information by observing from nearby viewpoints.
#[derive(Debug, Clone)]
pub struct Frontier {
    /// Center of mass of frontier voxels
    pub centroid: (f32, f32, f32),
    /// Voxel indices that make up this frontier
    pub voxels: Vec<Index64>,
    /// Estimated information gain (bits)
    pub information_gain: f32,
    /// Cluster size in voxels
    pub size: usize,
}

impl Frontier {
    /// Calculate bounding box of frontier
    pub fn bounding_box(&self, voxel_size: f32) -> ((f32, f32, f32), (f32, f32, f32)) {
        if self.voxels.is_empty() {
            return (self.centroid, self.centroid);
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for idx in &self.voxels {
            let (x, y, z) = idx.decode_coords();
            let (fx, fy, fz) = (
                x as f32 * voxel_size,
                y as f32 * voxel_size,
                z as f32 * voxel_size,
            );

            min_x = min_x.min(fx);
            min_y = min_y.min(fy);
            min_z = min_z.min(fz);
            max_x = max_x.max(fx);
            max_y = max_y.max(fy);
            max_z = max_z.max(fz);
        }

        ((min_x, min_y, min_z), (max_x, max_y, max_z))
    }

    /// Calculate frontier surface area (approximate)
    pub fn surface_area(&self, voxel_size: f32) -> f32 {
        self.size as f32 * voxel_size * voxel_size
    }
}

/// Viewpoint candidate for observation
#[derive(Debug, Clone)]
pub struct Viewpoint {
    /// Position (x, y, z) in meters
    pub position: (f32, f32, f32),
    /// Direction (normalized vector)
    pub direction: (f32, f32, f32),
    /// Expected information gain (bits)
    pub information_gain: f32,
    /// Associated frontier
    pub frontier_id: usize,
}

/// Configuration for frontier detection
#[derive(Debug, Clone)]
pub struct FrontierDetectionConfig {
    /// Minimum frontier cluster size (voxels)
    pub min_cluster_size: usize,
    /// Maximum search distance from known space (meters)
    pub max_distance: f32,
    /// Clustering distance threshold (meters)
    pub cluster_distance: f32,
}

impl Default for FrontierDetectionConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: 5,   // At least 5 voxels
            max_distance: 10.0,    // 10m max search
            cluster_distance: 0.5, // 0.5m clustering
        }
    }
}

/// Configuration for information gain calculation
#[derive(Debug, Clone)]
pub struct InformationGainConfig {
    /// Sensor range (meters)
    pub sensor_range: f32,
    /// Sensor field of view (radians)
    pub sensor_fov: f32,
    /// Ray casting resolution (degrees)
    pub ray_resolution: f32,
    /// Weight for unknown voxels
    pub unknown_weight: f32,
}

impl Default for InformationGainConfig {
    fn default() -> Self {
        Self {
            sensor_range: 5.0,                      // 5m depth camera
            sensor_fov: std::f32::consts::PI / 3.0, // 60° FOV
            ray_resolution: 5.0,                    // 5° between rays
            unknown_weight: 1.0,                    // 1 bit per unknown voxel
        }
    }
}

impl OccupancyLayer {
    /// Detect frontier voxels (boundaries between free and unknown space)
    ///
    /// A voxel is a frontier if it is:
    /// 1. In unknown state
    /// 2. Adjacent to at least one free voxel
    ///
    /// Returns clustered frontiers sorted by size (largest first)
    pub fn detect_frontiers(&self, config: &FrontierDetectionConfig) -> Result<Vec<Frontier>> {
        let frontier_voxels = Vec::new();

        // First pass: find all frontier candidates
        // (In real implementation, would iterate through actual voxels)
        // For now, this is a placeholder that demonstrates the API

        // Cluster frontier voxels
        let clusters = self.cluster_frontiers(&frontier_voxels, config)?;

        // Filter by minimum size and calculate centroids
        let mut frontiers: Vec<Frontier> = clusters
            .into_iter()
            .filter(|cluster| cluster.len() >= config.min_cluster_size)
            .map(|voxels| {
                let centroid = Self::calculate_centroid(&voxels);
                Frontier {
                    centroid,
                    size: voxels.len(),
                    voxels,
                    information_gain: 0.0, // Calculated separately
                }
            })
            .collect();

        // Sort by size (largest first)
        frontiers.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(frontiers)
    }

    /// Calculate information gain from a viewpoint
    ///
    /// Estimates how many bits of information would be gained by
    /// observing from the given position and direction.
    ///
    /// Uses ray casting to simulate sensor coverage and counts
    /// the number of unknown voxels that would be observed.
    pub fn information_gain_from(
        &self,
        viewpoint: (f32, f32, f32),
        direction: (f32, f32, f32),
        config: &InformationGainConfig,
    ) -> f32 {
        // Normalize direction
        let dir_len =
            (direction.0 * direction.0 + direction.1 * direction.1 + direction.2 * direction.2)
                .sqrt();

        if dir_len < 0.001 {
            return 0.0;
        }

        let dir = (
            direction.0 / dir_len,
            direction.1 / dir_len,
            direction.2 / dir_len,
        );

        let mut gain = 0.0;
        let mut observed = HashSet::new();

        // Cast rays in sensor FOV
        let ray_count = (config.sensor_fov.to_degrees() / config.ray_resolution).ceil() as i32;

        for i in -ray_count..=ray_count {
            for j in -ray_count..=ray_count {
                let angle_h = i as f32 * config.ray_resolution.to_radians();
                let angle_v = j as f32 * config.ray_resolution.to_radians();

                // Skip rays outside FOV
                if angle_h.abs() > config.sensor_fov / 2.0
                    || angle_v.abs() > config.sensor_fov / 2.0
                {
                    continue;
                }

                // Calculate ray direction (simplified rotation)
                let ray_dir = Self::rotate_direction(dir, angle_h, angle_v);

                // Cast ray and count unknown voxels
                let endpoint = (
                    viewpoint.0 + ray_dir.0 * config.sensor_range,
                    viewpoint.1 + ray_dir.1 * config.sensor_range,
                    viewpoint.2 + ray_dir.2 * config.sensor_range,
                );

                gain += self.ray_information_gain(viewpoint, endpoint, &mut observed, config);
            }
        }

        gain
    }

    /// Generate viewpoint candidates for frontiers
    ///
    /// For each frontier, generates candidate observation positions
    /// at various distances and angles.
    pub fn generate_viewpoint_candidates(
        &self,
        frontiers: &[Frontier],
        ig_config: &InformationGainConfig,
    ) -> Vec<Viewpoint> {
        let mut candidates = Vec::new();

        for (frontier_id, frontier) in frontiers.iter().enumerate() {
            // Generate candidates around frontier centroid
            let distances = [1.0, 2.0, 3.0]; // meters from frontier
            let angles = 8; // 8 positions around circle

            for &distance in &distances {
                for i in 0..angles {
                    let angle = 2.0 * std::f32::consts::PI * i as f32 / angles as f32;

                    let position = (
                        frontier.centroid.0 + distance * angle.cos(),
                        frontier.centroid.1 + distance * angle.sin(),
                        frontier.centroid.2, // Same height
                    );

                    // Direction points toward frontier
                    let dir = (
                        frontier.centroid.0 - position.0,
                        frontier.centroid.1 - position.1,
                        frontier.centroid.2 - position.2,
                    );

                    // Calculate information gain
                    let ig = self.information_gain_from(position, dir, ig_config);

                    candidates.push(Viewpoint {
                        position,
                        direction: Self::normalize(dir),
                        information_gain: ig,
                        frontier_id,
                    });
                }
            }
        }

        // Sort by information gain (highest first)
        candidates.sort_by(|a, b| {
            b.information_gain
                .partial_cmp(&a.information_gain)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    // Helper methods

    /// Cluster frontier voxels using connected components
    fn cluster_frontiers(
        &self,
        voxels: &[Index64],
        config: &FrontierDetectionConfig,
    ) -> Result<Vec<Vec<Index64>>> {
        if voxels.is_empty() {
            return Ok(Vec::new());
        }

        let mut visited = HashSet::new();
        let mut clusters = Vec::new();

        for &voxel in voxels {
            if visited.contains(&voxel) {
                continue;
            }

            // BFS to find connected component
            let cluster = self.bfs_cluster(voxel, voxels, &mut visited, config)?;
            if !cluster.is_empty() {
                clusters.push(cluster);
            }
        }

        Ok(clusters)
    }

    /// BFS to find connected component
    fn bfs_cluster(
        &self,
        start: Index64,
        _all_voxels: &[Index64],
        visited: &mut HashSet<Index64>,
        _config: &FrontierDetectionConfig,
    ) -> Result<Vec<Index64>> {
        let mut cluster = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            cluster.push(current);

            // Check neighbors (simplified - would use actual BCC neighbors)
            // For now, just demonstrates the clustering logic
        }

        Ok(cluster)
    }

    /// Calculate centroid of voxels
    fn calculate_centroid(voxels: &[Index64]) -> (f32, f32, f32) {
        if voxels.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;

        for voxel in voxels {
            let (x, y, z) = voxel.decode_coords();
            sum_x += x as f32;
            sum_y += y as f32;
            sum_z += z as f32;
        }

        let count = voxels.len() as f32;
        (sum_x / count, sum_y / count, sum_z / count)
    }

    /// Calculate information gain along a ray
    fn ray_information_gain(
        &self,
        _origin: (f32, f32, f32),
        _endpoint: (f32, f32, f32),
        _observed: &mut HashSet<Index64>,
        config: &InformationGainConfig,
    ) -> f32 {
        // Would cast ray and count unknown voxels
        // For now, placeholder
        let unknown_count = 0;
        unknown_count as f32 * config.unknown_weight
    }

    /// Rotate direction vector
    fn rotate_direction(dir: (f32, f32, f32), angle_h: f32, angle_v: f32) -> (f32, f32, f32) {
        // Simplified rotation (in real impl would use proper rotation matrices)
        let cos_h = angle_h.cos();
        let sin_h = angle_h.sin();
        let cos_v = angle_v.cos();
        let sin_v = angle_v.sin();

        (
            dir.0 * cos_h - dir.1 * sin_h,
            dir.0 * sin_h + dir.1 * cos_h,
            dir.2 * cos_v + (dir.0 * dir.0 + dir.1 * dir.1).sqrt() * sin_v,
        )
    }

    /// Normalize vector
    fn normalize(v: (f32, f32, f32)) -> (f32, f32, f32) {
        let len = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt();
        if len < 0.001 {
            return (0.0, 0.0, 1.0);
        }
        (v.0 / len, v.1 / len, v.2 / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontier_detection_config() {
        let config = FrontierDetectionConfig::default();
        assert_eq!(config.min_cluster_size, 5);
        assert_eq!(config.max_distance, 10.0);
    }

    #[test]
    fn test_information_gain_config() {
        let config = InformationGainConfig::default();
        assert_eq!(config.sensor_range, 5.0);
        assert!(config.sensor_fov > 0.0);
    }

    #[test]
    fn test_frontier_bounding_box() {
        let frontier = Frontier {
            centroid: (0.0, 0.0, 0.0),
            voxels: Vec::new(),
            information_gain: 0.0,
            size: 0,
        };

        let (min, max) = frontier.bounding_box(0.05);
        assert_eq!(min, (0.0, 0.0, 0.0));
        assert_eq!(max, (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_normalize() {
        let v = (3.0, 4.0, 0.0);
        let normalized = OccupancyLayer::normalize(v);
        let len = (normalized.0 * normalized.0
            + normalized.1 * normalized.1
            + normalized.2 * normalized.2)
            .sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }
}
