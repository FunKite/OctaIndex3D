//! Specialized neighbor calculation kernels optimized for hot paths
//!
//! These implementations are highly optimized for the most common
//! neighbor calculation patterns in 3D spatial indexing.

use crate::Route64;

const ROUTE64_COORD_MIN: i32 = -(1 << 19);
const ROUTE64_COORD_MAX: i32 = (1 << 19) - 1;

#[inline(always)]
fn append_neighbor_if_valid(
    result: &mut Vec<Route64>,
    tier: u8,
    origin: (i32, i32, i32),
    offset: (i32, i32, i32),
) {
    let (x, y, z) = origin;
    let (dx, dy, dz) = offset;

    let Some(nx) = x.checked_add(dx) else {
        return;
    };
    let Some(ny) = y.checked_add(dy) else {
        return;
    };
    let Some(nz) = z.checked_add(dz) else {
        return;
    };

    if !(ROUTE64_COORD_MIN..=ROUTE64_COORD_MAX).contains(&nx)
        || !(ROUTE64_COORD_MIN..=ROUTE64_COORD_MAX).contains(&ny)
        || !(ROUTE64_COORD_MIN..=ROUTE64_COORD_MAX).contains(&nz)
    {
        return;
    }

    // Safety: the source route is valid and BCC neighbor offsets preserve parity.
    unsafe {
        result.push(Route64::new_unchecked(tier, nx, ny, nz));
    }
}

pub(crate) fn append_neighbors_route64_fast(route: Route64, result: &mut Vec<Route64>) {
    let x = route.x();
    let y = route.y();
    let z = route.z();
    let tier = route.scale_tier();
    let origin = (x, y, z);

    append_neighbor_if_valid(result, tier, origin, (1, 1, 1));
    append_neighbor_if_valid(result, tier, origin, (1, 1, -1));
    append_neighbor_if_valid(result, tier, origin, (1, -1, 1));
    append_neighbor_if_valid(result, tier, origin, (1, -1, -1));
    append_neighbor_if_valid(result, tier, origin, (-1, 1, 1));
    append_neighbor_if_valid(result, tier, origin, (-1, 1, -1));
    append_neighbor_if_valid(result, tier, origin, (-1, -1, 1));
    append_neighbor_if_valid(result, tier, origin, (-1, -1, -1));
    append_neighbor_if_valid(result, tier, origin, (2, 0, 0));
    append_neighbor_if_valid(result, tier, origin, (-2, 0, 0));
    append_neighbor_if_valid(result, tier, origin, (0, 2, 0));
    append_neighbor_if_valid(result, tier, origin, (0, -2, 0));
    append_neighbor_if_valid(result, tier, origin, (0, 0, 2));
    append_neighbor_if_valid(result, tier, origin, (0, 0, -2));
}

/// Fast neighbor calculation with manual loop unrolling
///
/// This version unrolls the neighbor loop and uses inline assembly hints
/// for better instruction scheduling on modern CPUs.
///
/// # Safety
/// This function uses unsafe code (`new_unchecked`) internally for performance,
/// but preserves `Route64` range semantics by filtering neighbors that would
/// leave the signed 20-bit coordinate domain.
#[must_use]
#[inline(always)]
pub fn neighbors_route64_fast(route: Route64) -> Vec<Route64> {
    let mut result = Vec::with_capacity(14);
    append_neighbors_route64_fast(route, &mut result);
    result
}

/// Batch neighbor calculation with cache-optimized memory access
///
/// Processes neighbors in a cache-friendly pattern with prefetching.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn batch_neighbors_fast(routes: &[Route64]) -> Vec<Route64> {
    use super::arch_optimized::{prefetch_read, prefetch_write};

    let output_count = routes.len() * 14;
    let mut result: Vec<Route64> = Vec::with_capacity(output_count);

    /// Number of routes to prefetch ahead
    const PREFETCH_DISTANCE: usize = 4;

    for i in 0..routes.len() {
        // Prefetch upcoming routes
        if i + PREFETCH_DISTANCE < routes.len() {
            prefetch_read(&routes[i + PREFETCH_DISTANCE] as *const Route64);
        }

        // Calculate neighbors for current route
        let before = result.len();
        append_neighbors_route64_fast(routes[i], &mut result);

        // Reserve space and prefetch write location
        if before + 14 <= result.capacity() {
            let write_ptr = unsafe { result.as_mut_ptr().add(before) };
            prefetch_write(write_ptr);
        }
    }

    result
}

/// Fallback for non-tier1 architectures
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn batch_neighbors_fast(routes: &[Route64]) -> Vec<Route64> {
    let mut result = Vec::with_capacity(routes.len() * 14);
    for &route in routes {
        append_neighbors_route64_fast(route, &mut result);
    }
    result
}

/// Specialized kernel for small batches (< 100 routes)
///
/// Uses stack allocation and direct copying for minimal overhead.
#[inline]
pub fn batch_neighbors_small<const N: usize>(routes: &[Route64; N]) -> Vec<Route64> {
    let mut result = Vec::with_capacity(N * 14);

    for &route in routes {
        append_neighbors_route64_fast(route, &mut result);
    }

    result
}

/// Specialized kernel for medium batches (100-1000 routes)
///
/// Optimizes for L1/L2 cache efficiency with blocking.
pub fn batch_neighbors_medium(routes: &[Route64]) -> Vec<Route64> {
    /// Block size optimized for L1 cache
    const BLOCK_SIZE: usize = 64; // Fits in L1 cache

    let mut result = Vec::with_capacity(routes.len() * 14);

    for chunk in routes.chunks(BLOCK_SIZE) {
        for &route in chunk {
            append_neighbors_route64_fast(route, &mut result);
        }
    }

    result
}

/// Specialized kernel for large batches (> 50K routes)
///
/// Uses parallel processing with optimal chunk sizes.
/// Only beneficial for very large batches where parallelization overhead is amortized.
#[cfg(feature = "parallel")]
pub fn batch_neighbors_large(routes: &[Route64]) -> Vec<Route64> {
    use rayon::prelude::*;

    // Increased chunk size to amortize Rayon overhead
    // Neighbor calculation is extremely fast, so we need large chunks to benefit from parallelism
    routes
        .par_chunks(2048) // Larger chunks reduce parallel overhead
        .flat_map(|chunk| {
            let mut local_result = Vec::with_capacity(chunk.len() * 14);
            for &route in chunk {
                append_neighbors_route64_fast(route, &mut local_result);
            }
            local_result
        })
        .collect()
}

/// Automatic batch size selection with optimal kernel
///
/// Thresholds optimized for Apple Silicon M-series chips:
/// - Small (≤100): Fast kernel with prefetching
/// - Medium (≤50K): Cache-blocked processing (L2/L3 optimized)
/// - Large (>50K): Parallel processing to utilize all cores
pub fn batch_neighbors_auto(routes: &[Route64]) -> Vec<Route64> {
    match routes.len() {
        0..=100 => batch_neighbors_fast(routes),
        #[cfg(feature = "parallel")]
        101..=50000 => batch_neighbors_medium(routes),
        #[cfg(not(feature = "parallel"))]
        101.. => batch_neighbors_medium(routes),
        #[cfg(feature = "parallel")]
        _ => batch_neighbors_large(routes),
    }
}

/// Streaming neighbor calculation for very large datasets
///
/// Processes routes in chunks and yields results incrementally,
/// minimizing peak memory usage.
pub struct NeighborStream<'a> {
    /// Input routes to process
    routes: &'a [Route64],
    /// Current position in the routes array
    position: usize,
    /// Number of routes to process per chunk
    chunk_size: usize,
}

impl<'a> NeighborStream<'a> {
    /// Create new neighbor stream with default chunk size
    pub fn new(routes: &'a [Route64]) -> Self {
        Self {
            routes,
            position: 0,
            chunk_size: 256,
        }
    }

    /// Set custom chunk size for the stream
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}

impl<'a> Iterator for NeighborStream<'a> {
    type Item = Vec<Route64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.routes.len() {
            return None;
        }

        let end = (self.position + self.chunk_size).min(self.routes.len());
        let chunk = &self.routes[self.position..end];

        self.position = end;

        Some(batch_neighbors_auto(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_neighbors() {
        let route = Route64::new(0, 0, 0, 0).unwrap();
        let neighbors = neighbors_route64_fast(route);

        assert_eq!(neighbors.len(), 14);

        // Verify all neighbors are valid
        for neighbor in &neighbors {
            assert!(neighbor.x() >= -2 && neighbor.x() <= 2);
            assert!(neighbor.y() >= -2 && neighbor.y() <= 2);
            assert!(neighbor.z() >= -2 && neighbor.z() <= 2);
        }
    }

    #[test]
    fn test_fast_neighbors_respect_route64_bounds() {
        let route = Route64::new(0, 524286, 524286, 524286).unwrap();
        let neighbors = neighbors_route64_fast(route);

        assert_eq!(neighbors.len(), 11);
        assert!(neighbors
            .iter()
            .all(|neighbor| neighbor.x() >= ROUTE64_COORD_MIN
                && neighbor.x() <= ROUTE64_COORD_MAX
                && neighbor.y() >= ROUTE64_COORD_MIN
                && neighbor.y() <= ROUTE64_COORD_MAX
                && neighbor.z() >= ROUTE64_COORD_MIN
                && neighbor.z() <= ROUTE64_COORD_MAX));
    }

    #[test]
    fn test_batch_neighbors_fast() {
        let routes: Vec<Route64> = (0..10)
            .map(|i| {
                let coord = i * 2;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        let neighbors = batch_neighbors_fast(&routes);
        assert_eq!(neighbors.len(), 140); // 10 * 14
    }

    #[test]
    fn test_batch_neighbors_small() {
        let routes = [
            Route64::new(0, 0, 0, 0).unwrap(),
            Route64::new(0, 2, 2, 2).unwrap(),
            Route64::new(0, 4, 4, 4).unwrap(),
        ];

        let neighbors = batch_neighbors_small(&routes);
        assert_eq!(neighbors.len(), 42); // 3 * 14
    }

    #[test]
    fn test_batch_neighbors_medium() {
        let routes: Vec<Route64> = (0..500)
            .map(|i| {
                let coord = i * 2;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        let neighbors = batch_neighbors_medium(&routes);
        assert_eq!(neighbors.len(), 7000); // 500 * 14
    }

    #[test]
    fn test_batch_neighbors_auto() {
        // Test small
        let small: Vec<Route64> = (0..50)
            .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
            .collect();
        let result = batch_neighbors_auto(&small);
        assert_eq!(result.len(), 700);

        // Test medium
        let medium: Vec<Route64> = (0..500)
            .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
            .collect();
        let result = batch_neighbors_auto(&medium);
        assert_eq!(result.len(), 7000);
    }

    #[test]
    fn test_neighbor_stream() {
        let routes: Vec<Route64> = (0..1000)
            .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
            .collect();

        let stream = NeighborStream::new(&routes).with_chunk_size(100);

        let mut total_neighbors = 0;
        for chunk_neighbors in stream {
            total_neighbors += chunk_neighbors.len();
        }

        assert_eq!(total_neighbors, 14000); // 1000 * 14
    }
}
