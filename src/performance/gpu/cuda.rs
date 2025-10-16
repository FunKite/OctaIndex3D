//! CUDA backend for NVIDIA GPUs
//!
//! Leverages NVIDIA CUDA for massive parallel processing on NVIDIA GPUs.
//! This is the fastest option for NVIDIA hardware.
//!
//! Supported GPUs:
//! - RTX 40-series (Ada Lovelace) - Best
//! - RTX 30-series (Ampere)
//! - RTX 20-series (Turing)
//! - GTX 16-series
//! - Tesla/Quadro data center GPUs

use crate::Route64;
use crate::error::{Result, Error};
use super::GpuBackend;

#[cfg(feature = "gpu-cuda")]
use cudarc::driver::CudaDevice;
#[cfg(feature = "gpu-cuda")]
use std::sync::Arc;

/// CUDA GPU backend implementation
#[cfg(feature = "gpu-cuda")]
pub struct CudaBackend {
    #[allow(dead_code)]  // Will be used when kernel execution is implemented
    device: Arc<CudaDevice>,
}

#[cfg(feature = "gpu-cuda")]
impl CudaBackend {
    /// Create a new CUDA backend
    pub fn new() -> Result<Self> {
        // Initialize CUDA
        let device = CudaDevice::new(0)
            .map_err(|e| Error::InvalidFormat(format!("Failed to initialize CUDA device: {:?}", e)))?;

        Ok(Self { device })
    }

    /// Get device name
    pub fn device_name(&self) -> String {
        // Simplified version - would query actual device properties in production
        "CUDA Device".to_string()
    }
}

#[cfg(feature = "gpu-cuda")]
impl GpuBackend for CudaBackend {
    fn is_available(&self) -> bool {
        true // If we got here, CUDA is available
    }

    fn name(&self) -> &'static str {
        "CUDA (NVIDIA)"
    }

    fn batch_neighbors(&self, routes: &[Route64]) -> Result<Vec<Route64>> {
        // TODO: Implement actual CUDA kernel execution
        // For now, fall back to CPU implementation
        // This requires compiling the PTX kernel and using cudarc's launch API

        let output_count = routes.len() * 14;
        let mut results = Vec::with_capacity(output_count);

        for &route in routes {
            let neighbors = crate::performance::fast_neighbors::neighbors_route64_fast(route);
            for neighbor in &neighbors {
                results.push(*neighbor);
            }
        }

        Ok(results)
    }

    fn min_batch_size(&self) -> usize {
        1000 // CUDA is efficient even at moderate sizes
    }

    fn max_batch_size(&self) -> usize {
        10_000_000 // CUDA can handle very large batches
    }
}

#[cfg(not(feature = "gpu-cuda"))]
pub struct CudaBackend;

#[cfg(not(feature = "gpu-cuda"))]
impl CudaBackend {
    pub fn new() -> Result<Self> {
        Err(Error::InvalidFormat("CUDA feature not enabled".to_string()))
    }
}

/// Check if CUDA is available
#[cfg(feature = "gpu-cuda")]
pub fn is_cuda_available() -> bool {
    CudaDevice::new(0).is_ok()
}

#[cfg(not(feature = "gpu-cuda"))]
pub fn is_cuda_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "gpu-cuda")]
    fn test_cuda_backend_creation() {
        match CudaBackend::new() {
            Ok(backend) => {
                assert!(backend.is_available());
                println!("CUDA device: {}", backend.device_name());
            }
            Err(e) => {
                println!("CUDA not available: {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(feature = "gpu-cuda")]
    fn test_cuda_batch_neighbors() {
        let backend = match CudaBackend::new() {
            Ok(b) => b,
            Err(_) => return, // Skip if CUDA not available
        };

        // Create test routes
        let routes: Vec<Route64> = (0..100)
            .map(|i| {
                let coord = (i * 2) as i32;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        match backend.batch_neighbors(&routes) {
            Ok(neighbors) => {
                assert_eq!(neighbors.len(), 1400); // 14 * 100
                println!("CUDA batch neighbors: {} results", neighbors.len());
            }
            Err(e) => {
                panic!("CUDA batch neighbors failed: {:?}", e);
            }
        }
    }
}
