//! Parallel batch operations using Rayon
//!
//! This module provides multi-threaded implementations of batch operations,
//! leveraging Rayon for data parallelism. This is especially effective for
//! large batches (>1000 items) where the workload can be distributed across
//! multiple CPU cores.

use crate::{Index64, Route64};
use crate::neighbors;
use super::batch::BatchResult;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Parallel batch builder for Index64 instances
pub struct ParallelBatchIndexBuilder {
    chunk_size: usize,
}

impl ParallelBatchIndexBuilder {
    /// Create a new parallel batch builder
    pub fn new() -> Self {
        Self {
            chunk_size: 1024, // Optimal chunk size for most workloads
        }
    }

    /// Set the chunk size for parallel processing
    ///
    /// Smaller chunks = more parallelism but more overhead
    /// Larger chunks = less overhead but less parallelism
    /// Recommended: 512-2048 for most workloads
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Build multiple Index64 instances in parallel
    #[cfg(feature = "parallel")]
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

        // Parallel processing using Rayon
        let results: Vec<_> = (0..len)
            .into_par_iter()
            .map(|i| {
                Index64::new(
                    frame_ids[i],
                    dimension_ids[i],
                    lods[i],
                    x_coords[i],
                    y_coords[i],
                    z_coords[i],
                )
            })
            .collect();

        // Separate successes and errors
        let mut batch_result = BatchResult::with_capacity(len);
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(idx) => batch_result.items.push(idx),
                Err(e) => batch_result.errors.push((i, e)),
            }
        }

        batch_result
    }

    #[cfg(not(feature = "parallel"))]
    pub fn build(
        &self,
        _frame_ids: &[u8],
        _dimension_ids: &[u8],
        _lods: &[u8],
        _x_coords: &[u16],
        _y_coords: &[u16],
        _z_coords: &[u16],
    ) -> BatchResult<Index64> {
        panic!("Parallel feature not enabled");
    }
}

impl Default for ParallelBatchIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel batch calculator for neighbor operations
pub struct ParallelBatchNeighborCalculator {
    chunk_size: usize,
}

impl ParallelBatchNeighborCalculator {
    /// Create a new parallel neighbor calculator
    pub fn new() -> Self {
        Self {
            chunk_size: 256, // Smaller chunks for neighbor calc (more work per item)
        }
    }

    /// Set the chunk size for parallel processing
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Calculate neighbors for a batch of routes in parallel
    ///
    /// Returns a flat vector containing all neighbors (14 per route)
    #[cfg(feature = "parallel")]
    pub fn calculate(&self, routes: &[Route64]) -> Vec<Route64> {
        routes
            .par_chunks(self.chunk_size)
            .flat_map(|chunk| {
                let mut result = Vec::with_capacity(chunk.len() * 14);
                for &route in chunk {
                    result.extend(neighbors::neighbors_route64(route));
                }
                result
            })
            .collect()
    }

    /// Calculate neighbors and group by input route (parallel)
    #[cfg(feature = "parallel")]
    pub fn calculate_grouped(&self, routes: &[Route64]) -> Vec<Vec<Route64>> {
        routes
            .par_iter()
            .map(|&route| neighbors::neighbors_route64(route).to_vec())
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    pub fn calculate(&self, _routes: &[Route64]) -> Vec<Route64> {
        panic!("Parallel feature not enabled");
    }

    #[cfg(not(feature = "parallel"))]
    pub fn calculate_grouped(&self, _routes: &[Route64]) -> Vec<Vec<Route64>> {
        panic!("Parallel feature not enabled");
    }
}

impl Default for ParallelBatchNeighborCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to determine if parallel processing is beneficial
pub fn should_use_parallel(item_count: usize) -> bool {
    // Parallel overhead is worth it for batches > ~500 items
    item_count >= 500
}

/// Get the number of threads Rayon will use
#[cfg(feature = "parallel")]
pub fn thread_count() -> usize {
    rayon::current_num_threads()
}

#[cfg(not(feature = "parallel"))]
pub fn thread_count() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "parallel")]
    fn test_parallel_index_builder() {
        let builder = ParallelBatchIndexBuilder::new();

        let n = 1000;
        let frame_ids = vec![0; n];
        let dimension_ids = vec![0; n];
        let lods = vec![5; n];
        let x_coords: Vec<u16> = (0..n).map(|i| (i % 65536) as u16).collect();
        let y_coords: Vec<u16> = (0..n).map(|i| ((i + 100) % 65536) as u16).collect();
        let z_coords: Vec<u16> = (0..n).map(|i| ((i + 200) % 65536) as u16).collect();

        let result = builder.build(
            &frame_ids, &dimension_ids, &lods,
            &x_coords, &y_coords, &z_coords
        );

        assert_eq!(result.items.len(), n);
        assert!(!result.has_errors());
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_parallel_neighbor_calculator() {
        let calc = ParallelBatchNeighborCalculator::new();

        let routes: Vec<Route64> = (0..100)
            .map(|i| {
                let coord = (i * 2) as i32;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        let neighbors = calc.calculate(&routes);
        assert_eq!(neighbors.len(), 1400); // 14 * 100

        let grouped = calc.calculate_grouped(&routes);
        assert_eq!(grouped.len(), 100);
        for group in &grouped {
            assert_eq!(group.len(), 14);
        }
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_thread_count() {
        let count = thread_count();
        assert!(count > 0);
        println!("Rayon thread count: {}", count);
    }

    #[test]
    fn test_should_use_parallel() {
        assert!(!should_use_parallel(10));
        assert!(!should_use_parallel(100));
        assert!(should_use_parallel(1000));
        assert!(should_use_parallel(10000));
    }
}
