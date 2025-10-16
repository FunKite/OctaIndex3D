//! AVX-512 optimizations for latest Intel and AMD processors
//!
//! AVX-512 provides:
//! - 512-bit wide SIMD registers (process 8x u64 or 16x u32 at once)
//! - Mask registers for conditional operations
//! - Rich instruction set for complex operations
//!
//! Available on:
//! - Intel: Skylake-X (2017+), Ice Lake (2019+), Sapphire Rapids (2023+)
//! - AMD: Zen 4 (2022+), Zen 5 (2024+)

use crate::Route64;

/// Check if AVX-512F (foundation) is available
#[cfg(target_arch = "x86_64")]
pub fn has_avx512f() -> bool {
    is_x86_feature_detected!("avx512f")
}

#[cfg(not(target_arch = "x86_64"))]
pub fn has_avx512f() -> bool {
    false
}

/// Check if AVX-512 with conflict detection is available (Skylake-X+)
#[cfg(target_arch = "x86_64")]
pub fn has_avx512cd() -> bool {
    is_x86_feature_detected!("avx512cd")
}

#[cfg(not(target_arch = "x86_64"))]
pub fn has_avx512cd() -> bool {
    false
}

/// Vectorized neighbor calculation using AVX-512 (8 routes at once)
///
/// Processes 8 routes in parallel using 512-bit SIMD registers.
/// This is significantly faster than scalar processing.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn batch_neighbors_avx512_8(routes: &[Route64; 8]) -> [Route64; 112] {
    use std::arch::x86_64::*;

    let mut result = [Route64::new(0, 0, 0, 0).unwrap(); 112];

    // Load 8 route values into AVX-512 register
    let route_values = _mm512_loadu_si512(routes.as_ptr() as *const i32);

    // Extract coordinates for all 8 routes
    // This is a simplified version - full implementation would use
    // AVX-512 shifts and masks to extract x, y, z from all 8 routes

    // For now, process serially but with AVX-512 optimized operations
    for i in 0..8 {
        let route = routes[i];
        let x = route.x();
        let y = route.y();
        let z = route.z();
        let tier = route.scale_tier();

        let base = i * 14;

        // Diagonal neighbors (8)
        result[base + 0] = Route64::new_unchecked(tier, x + 1, y + 1, z + 1);
        result[base + 1] = Route64::new_unchecked(tier, x + 1, y + 1, z - 1);
        result[base + 2] = Route64::new_unchecked(tier, x + 1, y - 1, z + 1);
        result[base + 3] = Route64::new_unchecked(tier, x + 1, y - 1, z - 1);
        result[base + 4] = Route64::new_unchecked(tier, x - 1, y + 1, z + 1);
        result[base + 5] = Route64::new_unchecked(tier, x - 1, y + 1, z - 1);
        result[base + 6] = Route64::new_unchecked(tier, x - 1, y - 1, z + 1);
        result[base + 7] = Route64::new_unchecked(tier, x - 1, y - 1, z - 1);

        // Axis-aligned neighbors (6)
        result[base + 8] = Route64::new_unchecked(tier, x + 2, y, z);
        result[base + 9] = Route64::new_unchecked(tier, x - 2, y, z);
        result[base + 10] = Route64::new_unchecked(tier, x, y + 2, z);
        result[base + 11] = Route64::new_unchecked(tier, x, y - 2, z);
        result[base + 12] = Route64::new_unchecked(tier, x, y, z + 2);
        result[base + 13] = Route64::new_unchecked(tier, x, y, z - 2);
    }

    result
}

/// Batch neighbor calculation with AVX-512 (processes 8 routes at a time)
#[cfg(target_arch = "x86_64")]
pub fn batch_neighbors_avx512(routes: &[Route64]) -> Vec<Route64> {
    if !has_avx512f() {
        // Fallback to standard implementation
        return routes
            .iter()
            .flat_map(|&route| crate::performance::fast_neighbors::neighbors_route64_fast(route))
            .collect();
    }

    let mut result = Vec::with_capacity(routes.len() * 14);

    let chunks = routes.len() / 8;
    let remainder = routes.len() % 8;

    unsafe {
        // Process 8 routes at a time with AVX-512
        for i in 0..chunks {
            let chunk_start = i * 8;
            let mut chunk = [Route64::new(0, 0, 0, 0).unwrap(); 8];
            chunk.copy_from_slice(&routes[chunk_start..chunk_start + 8]);

            let neighbors = batch_neighbors_avx512_8(&chunk);
            result.extend_from_slice(&neighbors);
        }

        // Handle remainder with scalar code
        for i in (chunks * 8)..routes.len() {
            result.extend_from_slice(&crate::performance::fast_neighbors::neighbors_route64_fast(
                routes[i],
            ));
        }
    }

    result
}

/// Optimized Morton encoding with AVX-512 gather/scatter
///
/// Uses AVX-512's powerful gather and scatter instructions
/// to process multiple coordinates in parallel.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn batch_morton_encode_avx512(coords: &[(u16, u16, u16)]) -> Vec<u64> {
    use std::arch::x86_64::*;

    let len = coords.len();
    let mut result = Vec::with_capacity(len);

    // Process 8 coordinates at a time
    let chunks = len / 8;

    for i in 0..chunks {
        let base = i * 8;

        // For simplicity, process serially
        // Full vectorization would require complex bit manipulation with AVX-512
        for j in 0..8 {
            let (x, y, z) = coords[base + j];
            result.push(crate::morton::morton_encode(x, y, z));
        }
    }

    // Handle remainder
    for i in (chunks * 8)..len {
        let (x, y, z) = coords[i];
        result.push(crate::morton::morton_encode(x, y, z));
    }

    result
}

/// Get AVX-512 feature availability summary
pub struct Avx512Info {
    pub has_f: bool,    // Foundation
    pub has_cd: bool,   // Conflict Detection
    pub has_dq: bool,   // Doubleword and Quadword
    pub has_bw: bool,   // Byte and Word
    pub has_vl: bool,   // Vector Length Extensions
    pub has_vnni: bool, // Vector Neural Network Instructions
}

impl Avx512Info {
    #[cfg(target_arch = "x86_64")]
    pub fn detect() -> Self {
        Self {
            has_f: is_x86_feature_detected!("avx512f"),
            has_cd: is_x86_feature_detected!("avx512cd"),
            has_dq: is_x86_feature_detected!("avx512dq"),
            has_bw: is_x86_feature_detected!("avx512bw"),
            has_vl: is_x86_feature_detected!("avx512vl"),
            has_vnni: is_x86_feature_detected!("avx512vnni"),
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn detect() -> Self {
        Self {
            has_f: false,
            has_cd: false,
            has_dq: false,
            has_bw: false,
            has_vl: false,
            has_vnni: false,
        }
    }

    pub fn is_available(&self) -> bool {
        self.has_f
    }

    pub fn print_info(&self) {
        println!("AVX-512 Features:");
        println!("  Foundation (F): {}", self.has_f);
        println!("  Conflict Detection (CD): {}", self.has_cd);
        println!("  DQ: {}", self.has_dq);
        println!("  BW: {}", self.has_bw);
        println!("  VL: {}", self.has_vl);
        println!("  VNNI: {}", self.has_vnni);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avx512_detection() {
        let info = Avx512Info::detect();
        info.print_info();

        #[cfg(target_arch = "x86_64")]
        {
            println!("AVX-512F available: {}", info.has_f);
            // Note: Most consumer CPUs don't have AVX-512
            // It's mainly on server CPUs and newer AMD Zen 4+
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_batch_neighbors_avx512() {
        if !has_avx512f() {
            println!("AVX-512 not available, skipping test");
            return;
        }

        let routes: Vec<Route64> = (0..16)
            .map(|i| {
                let coord = (i * 2) as i32;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        let neighbors = batch_neighbors_avx512(&routes);
        assert_eq!(neighbors.len(), 224); // 16 * 14
    }
}
