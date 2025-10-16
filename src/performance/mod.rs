//! High-performance batch operations with SIMD, parallel, and GPU acceleration
//!
//! This module provides optimized implementations for batch operations on OctaIndex3D
//! data structures, leveraging:
//! - SIMD instructions (ARM NEON, x86 AVX2/AVX-512)
//! - Multi-threading (Rayon)
//! - GPU acceleration (Metal, Vulkan via wgpu)

pub mod arch_optimized;
pub mod batch;
pub mod fast_neighbors;
pub mod memory;
pub mod morton_batch;
pub mod simd;
pub mod simd_batch;

#[cfg(feature = "hilbert")]
pub mod hilbert_batch;

#[cfg(target_arch = "x86_64")]
pub mod avx512;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(any(
    feature = "gpu-metal",
    feature = "gpu-vulkan",
    feature = "gpu-cuda",
    feature = "gpu-rocm"
))]
pub mod gpu;

// Re-export commonly used items
pub use arch_optimized::{has_bmi2, ArchInfo};
pub use batch::{BatchIndexBuilder, BatchNeighborCalculator, BatchResult};
pub use fast_neighbors::{batch_neighbors_auto, neighbors_route64_fast, NeighborStream};
pub use memory::{AlignedBatchProcessor, AlignedVec, NumaInfo, CACHE_LINE_SIZE};
pub use morton_batch::{batch_morton_decode, batch_morton_encode};
pub use simd_batch::{
    batch_bounding_box_query, batch_euclidean_distance_squared, batch_index64_decode,
    batch_index64_encode, batch_manhattan_distance, batch_validate_routes,
};

#[cfg(feature = "hilbert")]
pub use hilbert_batch::{batch_hilbert_decode, batch_hilbert_encode};

#[cfg(target_arch = "x86_64")]
pub use avx512::{batch_neighbors_avx512, has_avx512f, Avx512Info};

#[cfg(feature = "parallel")]
pub use parallel::{ParallelBatchIndexBuilder, ParallelBatchNeighborCalculator};

#[cfg(any(feature = "gpu-metal", feature = "gpu-vulkan"))]
pub use gpu::{GpuBackend, GpuBatchProcessor};

/// Backend selection for batch operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Single-threaded CPU with SIMD
    CpuSingleThreaded,
    /// Multi-threaded CPU with SIMD and Rayon
    #[cfg(feature = "parallel")]
    CpuParallel,
    /// GPU acceleration via CUDA (NVIDIA)
    #[cfg(feature = "gpu-cuda")]
    GpuCuda,
    /// GPU acceleration via ROCm (AMD)
    #[cfg(feature = "gpu-rocm")]
    GpuRocm,
    /// GPU acceleration via Metal (macOS/iOS)
    #[cfg(feature = "gpu-metal")]
    GpuMetal,
    /// GPU acceleration via Vulkan (cross-platform)
    #[cfg(feature = "gpu-vulkan")]
    GpuVulkan,
}

impl Backend {
    /// Get the best available backend for the current platform
    pub fn best_available() -> Self {
        // Prefer GPU for large batches, with priority: CUDA > ROCm > Metal > Vulkan
        // Then fall back to CPU parallel, then single-threaded

        #[cfg(feature = "gpu-cuda")]
        {
            if gpu::is_cuda_available() {
                return Backend::GpuCuda;
            }
        }

        #[cfg(feature = "gpu-rocm")]
        {
            if gpu::is_rocm_available() {
                return Backend::GpuRocm;
            }
        }

        #[cfg(all(target_os = "macos", feature = "gpu-metal"))]
        {
            if gpu::is_metal_available() {
                return Backend::GpuMetal;
            }
        }

        #[cfg(feature = "gpu-vulkan")]
        {
            if gpu::is_vulkan_available() {
                return Backend::GpuVulkan;
            }
        }

        #[cfg(feature = "parallel")]
        return Backend::CpuParallel;

        #[cfg(not(feature = "parallel"))]
        Backend::CpuSingleThreaded
    }

    /// Check if this backend is available on the current system
    pub fn is_available(&self) -> bool {
        match self {
            Backend::CpuSingleThreaded => true,
            #[cfg(feature = "parallel")]
            Backend::CpuParallel => true,
            #[cfg(feature = "gpu-cuda")]
            Backend::GpuCuda => gpu::is_cuda_available(),
            #[cfg(feature = "gpu-rocm")]
            Backend::GpuRocm => gpu::is_rocm_available(),
            #[cfg(feature = "gpu-metal")]
            Backend::GpuMetal => gpu::is_metal_available(),
            #[cfg(feature = "gpu-vulkan")]
            Backend::GpuVulkan => gpu::is_vulkan_available(),
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::best_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection() {
        let backend = Backend::best_available();
        assert!(backend.is_available());
    }
}
