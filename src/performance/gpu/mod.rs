//! GPU-accelerated batch operations
//!
//! This module provides GPU compute backends for massive batch operations:
//! - Metal (macOS/iOS)
//! - Vulkan via wgpu (cross-platform)
//!
//! GPU acceleration is most beneficial for very large batches (>10k items)
//! where the parallel processing power of GPUs significantly outweighs
//! the CPU-GPU transfer overhead.

#[cfg(all(feature = "gpu-metal", target_os = "macos"))]
pub mod metal;

#[cfg(all(feature = "gpu-vulkan", not(target_os = "windows")))]
pub mod wgpu_backend;

#[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
pub mod cuda;

#[cfg(feature = "gpu-rocm")]
pub mod rocm;

use crate::error::Result;
use crate::Route64;

/// GPU backend trait for batch operations
pub trait GpuBackend: Send + Sync {
    /// Check if this GPU backend is available
    fn is_available(&self) -> bool;

    /// Get the name of this GPU backend
    fn name(&self) -> &'static str;

    /// Calculate neighbors for a batch of routes on the GPU
    ///
    /// Returns a flat vector of all neighbors (14 per input route)
    fn batch_neighbors(&self, routes: &[Route64]) -> Result<Vec<Route64>>;

    /// Get recommended minimum batch size for GPU acceleration
    ///
    /// Below this size, CPU processing is likely faster due to transfer overhead
    fn min_batch_size(&self) -> usize {
        5000 // Conservative default
    }

    /// Get maximum batch size this backend can handle
    fn max_batch_size(&self) -> usize {
        1_000_000 // 1M routes = 14M neighbors
    }
}

/// High-level GPU batch processor that automatically selects backend
pub struct GpuBatchProcessor {
    backend: Box<dyn GpuBackend>,
}

impl GpuBatchProcessor {
    /// Create a new GPU batch processor with the best available backend
    pub fn new() -> Result<Self> {
        let backend = Self::best_backend()?;
        Ok(Self { backend })
    }

    /// Create a GPU batch processor with a specific backend
    #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
    pub fn with_metal() -> Result<Self> {
        Ok(Self {
            backend: Box::new(metal::MetalBackend::new()?),
        })
    }

    #[cfg(all(feature = "gpu-vulkan", not(target_os = "windows")))]
    pub fn with_vulkan() -> Result<Self> {
        Ok(Self {
            backend: Box::new(wgpu_backend::WgpuBackend::new()?),
        })
    }

    /// Get the best available GPU backend
    fn best_backend() -> Result<Box<dyn GpuBackend>> {
        // Try CUDA first (best for NVIDIA)
        #[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
        {
            // Catch panic from cudarc when CUDA isn't available
            if let Ok(Ok(backend)) = std::panic::catch_unwind(cuda::CudaBackend::new) {
                return Ok(Box::new(backend));
            }
        }

        // Try ROCm (best for AMD)
        #[cfg(feature = "gpu-rocm")]
        {
            if let Ok(backend) = rocm::RocmBackend::new() {
                return Ok(Box::new(backend));
            }
        }

        // Try Metal (best for Apple)
        #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
        {
            if let Ok(backend) = metal::MetalBackend::new() {
                return Ok(Box::new(backend));
            }
        }

        // Try Vulkan (cross-platform fallback)
        #[cfg(all(feature = "gpu-vulkan", not(target_os = "windows")))]
        {
            if let Ok(backend) = wgpu_backend::WgpuBackend::new() {
                return Ok(Box::new(backend));
            }
        }

        Err(crate::error::Error::InvalidFormat(
            "No GPU backend available".to_string(),
        ))
    }

    /// Get the name of the active backend
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Check if GPU acceleration should be used for this batch size
    pub fn should_use_gpu(&self, batch_size: usize) -> bool {
        batch_size >= self.backend.min_batch_size() && batch_size <= self.backend.max_batch_size()
    }

    /// Calculate neighbors for a batch of routes on the GPU
    pub fn batch_neighbors(&self, routes: &[Route64]) -> Result<Vec<Route64>> {
        if !self.should_use_gpu(routes.len()) {
            return Err(crate::error::Error::InvalidFormat(format!(
                "Batch size {} outside GPU optimal range [{}, {}]",
                routes.len(),
                self.backend.min_batch_size(),
                self.backend.max_batch_size()
            )));
        }

        self.backend.batch_neighbors(routes)
    }
}

/// Check if CUDA is available
#[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
pub fn is_cuda_available() -> bool {
    cuda::is_cuda_available()
}

#[cfg(not(all(feature = "gpu-cuda", not(target_os = "windows"))))]
pub fn is_cuda_available() -> bool {
    false
}

/// Check if Metal is available
#[cfg(all(feature = "gpu-metal", target_os = "macos"))]
pub fn is_metal_available() -> bool {
    metal::MetalBackend::new().is_ok()
}

#[cfg(not(all(feature = "gpu-metal", target_os = "macos")))]
pub fn is_metal_available() -> bool {
    false
}

/// Check if Vulkan is available
#[cfg(all(feature = "gpu-vulkan", not(target_os = "windows")))]
pub fn is_vulkan_available() -> bool {
    wgpu_backend::WgpuBackend::new().is_ok()
}

#[cfg(not(all(feature = "gpu-vulkan", not(target_os = "windows"))))]
pub fn is_vulkan_available() -> bool {
    false
}

/// Check if ROCm is available
#[cfg(feature = "gpu-rocm")]
pub fn is_rocm_available() -> bool {
    rocm::is_rocm_available()
}

#[cfg(not(feature = "gpu-rocm"))]
pub fn is_rocm_available() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_availability() {
        println!("CUDA available: {}", is_cuda_available());
        println!("ROCm available: {}", is_rocm_available());
        println!("Metal available: {}", is_metal_available());
        println!("Vulkan available: {}", is_vulkan_available());

        // At least one should be available on supported platforms
        #[cfg(any(
            feature = "gpu-cuda",
            feature = "gpu-rocm",
            feature = "gpu-metal",
            feature = "gpu-vulkan"
        ))]
        {
            let has_gpu = is_cuda_available()
                || is_rocm_available()
                || is_metal_available()
                || is_vulkan_available();
            if !has_gpu {
                println!("Warning: No GPU backend available");
            }
        }
    }

    #[test]
    #[cfg(any(feature = "gpu-metal", feature = "gpu-vulkan"))]
    fn test_gpu_processor_creation() {
        match GpuBatchProcessor::new() {
            Ok(processor) => {
                println!("Using GPU backend: {}", processor.backend_name());
                assert!(processor.backend.is_available());
            }
            Err(e) => {
                println!("Could not create GPU processor: {:?}", e);
            }
        }
    }
}
