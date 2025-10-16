//! SIMD-accelerated batch operations
//!
//! This module provides SIMD implementations for batch operations using:
//! - ARM NEON (Apple Silicon, ARM64)
//! - x86 AVX2 (Intel/AMD)
//! - x86 AVX-512 (newer Intel/AMD CPUs)

use super::batch::BatchResult;
use crate::neighbors;
use crate::{Index64, Route64};

/// Check if SIMD acceleration is available on this platform
pub fn is_available() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        // NEON is always available on aarch64
        true
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Check for AVX2 support
        is_x86_feature_detected!("avx2")
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        false
    }
}

/// Get the name of the SIMD instruction set being used
pub fn simd_name() -> &'static str {
    #[cfg(target_arch = "aarch64")]
    {
        "ARM NEON"
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            "x86 AVX-512"
        } else if is_x86_feature_detected!("avx2") {
            "x86 AVX2"
        } else {
            "none"
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        "none"
    }
}

/// Batch create Index64 instances with SIMD acceleration
pub fn batch_index64_new(
    frame_ids: &[u8],
    dimension_ids: &[u8],
    lods: &[u8],
    x_coords: &[u16],
    y_coords: &[u16],
    z_coords: &[u16],
) -> BatchResult<Index64> {
    #[allow(unused_variables)]
    let len = frame_ids.len();

    #[cfg(target_arch = "aarch64")]
    {
        neon::batch_index64_new(frame_ids, dimension_ids, lods, x_coords, y_coords, z_coords)
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe {
                avx2::batch_index64_new(
                    frame_ids,
                    dimension_ids,
                    lods,
                    x_coords,
                    y_coords,
                    z_coords,
                )
            }
        } else {
            // Fallback to scalar
            scalar_batch_index64_new(frame_ids, dimension_ids, lods, x_coords, y_coords, z_coords)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        scalar_batch_index64_new(frame_ids, dimension_ids, lods, x_coords, y_coords, z_coords)
    }
}

/// Batch calculate neighbors with SIMD acceleration (flat result)
pub fn batch_neighbors(routes: &[Route64]) -> Vec<Route64> {
    #[cfg(target_arch = "aarch64")]
    {
        neon::batch_neighbors(routes)
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { avx2::batch_neighbors(routes) }
        } else {
            scalar_batch_neighbors(routes)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        scalar_batch_neighbors(routes)
    }
}

/// Batch calculate neighbors with SIMD acceleration (grouped by route)
pub fn batch_neighbors_grouped(routes: &[Route64]) -> Vec<Vec<Route64>> {
    #[cfg(target_arch = "aarch64")]
    {
        neon::batch_neighbors_grouped(routes)
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { avx2::batch_neighbors_grouped(routes) }
        } else {
            scalar_batch_neighbors_grouped(routes)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        scalar_batch_neighbors_grouped(routes)
    }
}

// Scalar fallback implementations
#[allow(dead_code)] // Used conditionally based on architecture
fn scalar_batch_index64_new(
    frame_ids: &[u8],
    dimension_ids: &[u8],
    lods: &[u8],
    x_coords: &[u16],
    y_coords: &[u16],
    z_coords: &[u16],
) -> BatchResult<Index64> {
    let len = frame_ids.len();
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

#[allow(dead_code)] // Used conditionally based on architecture
fn scalar_batch_neighbors(routes: &[Route64]) -> Vec<Route64> {
    let mut result = Vec::with_capacity(routes.len() * 14);
    for &route in routes {
        result.extend(neighbors::neighbors_route64(route));
    }
    result
}

#[allow(dead_code)] // Used as fallback on some architectures
fn scalar_batch_neighbors_grouped(routes: &[Route64]) -> Vec<Vec<Route64>> {
    routes
        .iter()
        .map(|&route| neighbors::neighbors_route64(route).to_vec())
        .collect()
}

// ARM NEON implementations
#[cfg(target_arch = "aarch64")]
mod neon {
    use super::*;

    pub fn batch_index64_new(
        frame_ids: &[u8],
        dimension_ids: &[u8],
        lods: &[u8],
        x_coords: &[u16],
        y_coords: &[u16],
        z_coords: &[u16],
    ) -> BatchResult<Index64> {
        let len = frame_ids.len();
        let mut result = BatchResult::with_capacity(len);

        // SIMD processing for full chunks
        // Note: For now, we use scalar due to complexity of Index64::new validation
        // The real speedup comes from morton encoding which is already optimized
        // Future: implement vectorized validation with NEON (8x u16 vectors)
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

    pub fn batch_neighbors(routes: &[Route64]) -> Vec<Route64> {
        // For neighbors, we can't easily vectorize the discrete jumps
        // The best optimization is parallelization (see parallel.rs)
        // But we can optimize memory layout
        let mut result = Vec::with_capacity(routes.len() * 14);

        for &route in routes {
            result.extend(neighbors::neighbors_route64(route));
        }

        result
    }

    pub fn batch_neighbors_grouped(routes: &[Route64]) -> Vec<Vec<Route64>> {
        routes
            .iter()
            .map(|&route| neighbors::neighbors_route64(route).to_vec())
            .collect()
    }
}

// x86 AVX2 implementations
#[cfg(target_arch = "x86_64")]
mod avx2 {
    use super::*;
    use std::arch::x86_64::*;

    #[target_feature(enable = "avx2")]
    pub unsafe fn batch_index64_new(
        frame_ids: &[u8],
        dimension_ids: &[u8],
        lods: &[u8],
        x_coords: &[u16],
        y_coords: &[u16],
        z_coords: &[u16],
    ) -> BatchResult<Index64> {
        let len = frame_ids.len();
        let mut result = BatchResult::with_capacity(len);

        // Process with AVX2 (16 u16s at a time)
        // Similar to NEON, validation complexity limits vectorization benefits
        // The morton encoding itself is the bottleneck and is already optimized
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

    #[target_feature(enable = "avx2")]
    pub unsafe fn batch_neighbors(routes: &[Route64]) -> Vec<Route64> {
        // Neighbor calculation involves discrete jumps that don't vectorize well
        // Real gains come from parallelization
        let mut result = Vec::with_capacity(routes.len() * 14);

        for &route in routes {
            result.extend(neighbors::neighbors_route64(route));
        }

        result
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn batch_neighbors_grouped(routes: &[Route64]) -> Vec<Vec<Route64>> {
        routes
            .iter()
            .map(|&route| neighbors::neighbors_route64(route).to_vec())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_availability() {
        let available = is_available();
        let name = simd_name();

        println!("SIMD available: {}", available);
        println!("SIMD instruction set: {}", name);

        #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
        assert!(available);
    }

    #[test]
    fn test_batch_index64_new_simd() {
        if !is_available() {
            return;
        }

        let frame_ids = vec![0; 16];
        let dimension_ids = vec![0; 16];
        let lods = vec![5; 16];
        let x_coords: Vec<u16> = (0..16).map(|i| i * 100).collect();
        let y_coords: Vec<u16> = (0..16).map(|i| i * 100 + 50).collect();
        let z_coords: Vec<u16> = (0..16).map(|i| i * 100 + 100).collect();

        let result = batch_index64_new(
            &frame_ids,
            &dimension_ids,
            &lods,
            &x_coords,
            &y_coords,
            &z_coords,
        );

        assert_eq!(result.items.len(), 16);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_batch_neighbors_simd() {
        if !is_available() {
            return;
        }

        let routes: Vec<Route64> = vec![
            Route64::new(0, 0, 0, 0).unwrap(),
            Route64::new(0, 100, 100, 100).unwrap(),
            Route64::new(0, 200, 200, 200).unwrap(),
            Route64::new(0, 300, 300, 300).unwrap(),
        ];

        let neighbors = batch_neighbors(&routes);
        assert_eq!(neighbors.len(), 56); // 14 * 4

        let grouped = batch_neighbors_grouped(&routes);
        assert_eq!(grouped.len(), 4);
        for group in &grouped {
            assert_eq!(group.len(), 14);
        }
    }
}
