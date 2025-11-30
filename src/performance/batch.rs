//! Batch operations for efficient processing of multiple indices and routes
//!
//! This module provides optimized batch processing capabilities with automatic
//! SIMD vectorization where available.

use crate::neighbors;
use crate::{Index64, Route64};

#[cfg(feature = "simd")]
use super::simd;

/// Result container for batch operations
#[derive(Debug, Clone)]
pub struct BatchResult<T> {
    /// Successfully processed items
    pub items: Vec<T>,
    /// Errors encountered during processing (index, error)
    pub errors: Vec<(usize, crate::error::Error)>,
}

impl<T> BatchResult<T> {
    /// Create a new batch result with items and no errors
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            errors: Vec::new(),
        }
    }

    /// Create a new batch result with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            errors: Vec::new(),
        }
    }

    /// Get the number of successfully processed items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if there are no successfully processed items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if any errors occurred during processing
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Batch builder for creating multiple Index64 instances efficiently
pub struct BatchIndexBuilder {
    /// Enable SIMD acceleration if available
    use_simd: bool,
}

impl BatchIndexBuilder {
    /// Create a new batch index builder with auto-detected SIMD support
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "simd")]
            use_simd: simd::is_available(),
            #[cfg(not(feature = "simd"))]
            use_simd: false,
        }
    }

    /// Configure SIMD usage (only effective if SIMD feature is enabled)
    pub fn with_simd(mut self, enabled: bool) -> Self {
        self.use_simd = enabled && cfg!(feature = "simd");
        self
    }

    /// Build multiple Index64 instances from coordinate arrays
    ///
    /// # Arguments
    /// * `frame_ids` - Frame IDs (one per index)
    /// * `dimension_ids` - Dimension IDs (one per index)
    /// * `lods` - Level of detail values (one per index)
    /// * `x_coords` - X coordinates
    /// * `y_coords` - Y coordinates
    /// * `z_coords` - Z coordinates
    pub fn build(
        &self,
        frame_ids: &[u8],
        dimension_ids: &[u8],
        lods: &[u8],
        x_coords: &[u16],
        y_coords: &[u16],
        z_coords: &[u16],
    ) -> BatchResult<Index64> {
        let len = frame_ids.len();
        assert_eq!(dimension_ids.len(), len);
        assert_eq!(lods.len(), len);
        assert_eq!(x_coords.len(), len);
        assert_eq!(y_coords.len(), len);
        assert_eq!(z_coords.len(), len);

        #[cfg(feature = "simd")]
        if self.use_simd && simd::is_available() {
            return simd::batch_index64_new(
                frame_ids,
                dimension_ids,
                lods,
                x_coords,
                y_coords,
                z_coords,
            );
        }

        // Fallback: scalar implementation
        let mut result = BatchResult::with_capacity(len);

        for i in 0..len {
            match Index64::new(
                frame_ids[i],
                dimension_ids[i],
                lods[i],
                x_coords[i],
                y_coords[i],
                z_coords[i],
            ) {
                Ok(idx) => result.items.push(idx),
                Err(e) => result.errors.push((i, e)),
            }
        }

        result
    }
}

impl Default for BatchIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch calculator for computing neighbors of multiple routes
pub struct BatchNeighborCalculator {
    /// Enable SIMD acceleration if available
    use_simd: bool,
}

impl BatchNeighborCalculator {
    /// Create a new batch neighbor calculator with auto-detected SIMD support
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "simd")]
            use_simd: simd::is_available(),
            #[cfg(not(feature = "simd"))]
            use_simd: false,
        }
    }

    /// Configure SIMD usage (only effective if SIMD feature is enabled)
    pub fn with_simd(mut self, enabled: bool) -> Self {
        self.use_simd = enabled && cfg!(feature = "simd");
        self
    }

    /// Calculate neighbors for a batch of routes
    ///
    /// Returns a flat vector containing all neighbors (14 per route)
    pub fn calculate(&self, routes: &[Route64]) -> Vec<Route64> {
        #[cfg(feature = "simd")]
        if self.use_simd && simd::is_available() {
            return simd::batch_neighbors(routes);
        }

        // Fallback: scalar implementation
        let mut result = Vec::with_capacity(routes.len() * 14);

        for &route in routes {
            result.extend(neighbors::neighbors_route64(route));
        }

        result
    }

    /// Calculate neighbors and group by input route
    ///
    /// Returns a vector of vectors, where each inner vector contains
    /// the 14 neighbors of the corresponding input route
    pub fn calculate_grouped(&self, routes: &[Route64]) -> Vec<Vec<Route64>> {
        #[cfg(feature = "simd")]
        if self.use_simd && simd::is_available() {
            return simd::batch_neighbors_grouped(routes);
        }

        // Fallback: scalar implementation
        routes
            .iter()
            .map(|&route| neighbors::neighbors_route64(route).to_vec())
            .collect()
    }
}

impl Default for BatchNeighborCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_index_builder() {
        let builder = BatchIndexBuilder::new();

        let frame_ids = vec![0, 0, 0];
        let dimension_ids = vec![0, 0, 0];
        let lods = vec![5, 5, 5];
        let x_coords = vec![100, 200, 300];
        let y_coords = vec![150, 250, 350];
        let z_coords = vec![200, 300, 400];

        let result = builder.build(
            &frame_ids,
            &dimension_ids,
            &lods,
            &x_coords,
            &y_coords,
            &z_coords,
        );

        assert_eq!(result.len(), 3);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_batch_neighbor_calculator() {
        let calc = BatchNeighborCalculator::new();

        let routes = vec![
            Route64::new(0, 0, 0, 0).unwrap(),
            Route64::new(0, 100, 100, 100).unwrap(),
        ];

        let neighbors = calc.calculate(&routes);
        assert_eq!(neighbors.len(), 28); // 14 neighbors * 2 routes

        let grouped = calc.calculate_grouped(&routes);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].len(), 14);
        assert_eq!(grouped[1].len(), 14);
    }
}
