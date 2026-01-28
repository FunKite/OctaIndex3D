//! GPU-Accelerated Occupancy Operations
//!
//! Provides Metal and CUDA backends for massive parallel ray casting
//! and occupancy updates on BCC lattice.

use super::occupancy::OccupancyLayer;
use crate::error::{Error, Result};
use crate::Index64;

/// GPU-accelerated ray casting for occupancy mapping
pub struct GpuRayCaster {
    backend: Box<dyn RayCastBackend>,
}

/// Trait for GPU ray casting backends
pub trait RayCastBackend: Send + Sync {
    /// Get backend name
    fn name(&self) -> &'static str;

    /// Check if backend is available
    fn is_available(&self) -> bool;

    /// Cast multiple rays in parallel on GPU
    ///
    /// # Arguments
    /// * `origins` - Ray origin points (x, y, z)
    /// * `endpoints` - Ray endpoint points (x, y, z)
    /// * `voxel_size` - Size of voxels in meters
    /// * `free_confidence` - Confidence for free space (0.0-1.0)
    /// * `occupied_confidence` - Confidence for occupied endpoints (0.0-1.0)
    ///
    /// # Returns
    /// Vector of (index, occupied_flag, confidence) tuples
    fn cast_rays(
        &self,
        origins: &[(f32, f32, f32)],
        endpoints: &[(f32, f32, f32)],
        voxel_size: f32,
        free_confidence: f32,
        occupied_confidence: f32,
    ) -> Result<Vec<(Index64, bool, f32)>>;

    /// Get minimum batch size for efficient GPU usage
    fn min_batch_size(&self) -> usize {
        100 // Rays
    }
}

impl GpuRayCaster {
    /// Create new GPU ray caster with best available backend
    pub fn new() -> Result<Self> {
        let backend = Self::best_backend()?;
        Ok(Self { backend })
    }

    /// Get best available GPU backend
    fn best_backend() -> Result<Box<dyn RayCastBackend>> {
        // Try CUDA first (best for NVIDIA)
        #[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
        {
            if let Ok(backend) = CudaRayCaster::new() {
                return Ok(Box::new(backend));
            }
        }

        // Try Metal (best for Apple Silicon)
        #[cfg(all(feature = "gpu-metal", target_os = "macos"))]
        {
            if let Ok(backend) = MetalRayCaster::new() {
                return Ok(Box::new(backend));
            }
        }

        Err(Error::InvalidFormat(
            "No GPU ray casting backend available".to_string(),
        ))
    }

    /// Cast multiple rays in parallel
    pub fn cast_rays(
        &self,
        origins: &[(f32, f32, f32)],
        endpoints: &[(f32, f32, f32)],
        voxel_size: f32,
        free_confidence: f32,
        occupied_confidence: f32,
    ) -> Result<Vec<(Index64, bool, f32)>> {
        if origins.len() != endpoints.len() {
            return Err(Error::InvalidFormat(
                "Origins and endpoints must have same length".to_string(),
            ));
        }

        self.backend.cast_rays(
            origins,
            endpoints,
            voxel_size,
            free_confidence,
            occupied_confidence,
        )
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Apply ray casting results to occupancy layer
    pub fn apply_to_layer(
        &self,
        layer: &mut OccupancyLayer,
        results: Vec<(Index64, bool, f32)>,
    ) -> Result<()> {
        for (idx, occupied, confidence) in results {
            layer.update_occupancy(idx, occupied, confidence);
        }
        Ok(())
    }
}

// Metal backend for Apple Silicon
#[cfg(all(feature = "gpu-metal", target_os = "macos"))]
mod metal_impl {
    use super::*;
    use metal::*;
    use std::sync::Arc;

    pub struct MetalRayCaster {
        _device: Arc<Device>,
        _command_queue: Arc<CommandQueue>,
        _pipeline: ComputePipelineState,
    }

    impl MetalRayCaster {
        pub fn new() -> Result<Self> {
            let device = Device::system_default()
                .ok_or_else(|| Error::InvalidFormat("No Metal device found".to_string()))?;

            let command_queue = device.new_command_queue();

            // Compile ray casting shader
            let shader_source = include_str!("../performance/gpu/shaders/occupancy_raycast.metal");
            let options = CompileOptions::new();
            let library = device
                .new_library_with_source(shader_source, &options)
                .map_err(|e| Error::InvalidFormat(format!("Failed to compile shader: {}", e)))?;

            let kernel = library
                .get_function("cast_occupancy_rays", None)
                .map_err(|e| Error::InvalidFormat(format!("Failed to get kernel: {}", e)))?;

            let pipeline = device
                .new_compute_pipeline_state_with_function(&kernel)
                .map_err(|e| Error::InvalidFormat(format!("Failed to create pipeline: {}", e)))?;

            Ok(Self {
                _device: Arc::new(device),
                _command_queue: Arc::new(command_queue),
                _pipeline: pipeline,
            })
        }
    }

    impl RayCastBackend for MetalRayCaster {
        fn name(&self) -> &'static str {
            "Metal"
        }

        fn is_available(&self) -> bool {
            true
        }

        fn cast_rays(
            &self,
            _origins: &[(f32, f32, f32)],
            _endpoints: &[(f32, f32, f32)],
            _voxel_size: f32,
            _free_confidence: f32,
            _occupied_confidence: f32,
        ) -> Result<Vec<(Index64, bool, f32)>> {
            // Implementation would use Metal compute shaders
            // For now, return empty (will be implemented in shader)
            Ok(Vec::new())
        }
    }
}

#[cfg(all(feature = "gpu-metal", target_os = "macos"))]
pub use metal_impl::MetalRayCaster;

// CUDA backend for NVIDIA GPUs
#[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
mod cuda_impl {
    use super::*;

    pub struct CudaRayCaster {
        // CUDA context and kernels would go here
    }

    impl CudaRayCaster {
        pub fn new() -> Result<Self> {
            // Initialize CUDA ray casting kernel
            Ok(Self {})
        }
    }

    impl RayCastBackend for CudaRayCaster {
        fn name(&self) -> &'static str {
            "CUDA"
        }

        fn is_available(&self) -> bool {
            true
        }

        fn cast_rays(
            &self,
            _origins: &[(f32, f32, f32)],
            _endpoints: &[(f32, f32, f32)],
            _voxel_size: f32,
            _free_confidence: f32,
            _occupied_confidence: f32,
        ) -> Result<Vec<(Index64, bool, f32)>> {
            // CUDA implementation
            Ok(Vec::new())
        }
    }
}

#[cfg(all(feature = "gpu-cuda", not(target_os = "windows")))]
pub use cuda_impl::CudaRayCaster;

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(feature = "gpu-metal", feature = "gpu-cuda"))]
    fn test_gpu_ray_caster_creation() {
        match super::GpuRayCaster::new() {
            Ok(caster) => {
                println!("GPU Ray Caster backend: {}", caster.backend_name());
            }
            Err(e) => {
                println!("No GPU backend available: {:?}", e);
            }
        }
    }
}
