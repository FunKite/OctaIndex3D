//! Metal GPU backend for macOS and iOS
//!
//! This module provides Metal compute shader acceleration for batch operations.
//! Metal is Apple's high-performance graphics and compute API.

use super::GpuBackend;
use crate::error::{Error, Result};
use crate::Route64;

#[cfg(feature = "gpu-metal")]
use metal::*;

#[cfg(feature = "gpu-metal")]
use std::sync::Arc;

/// Metal GPU backend implementation
#[cfg(feature = "gpu-metal")]
pub struct MetalBackend {
    device: Arc<Device>,
    command_queue: Arc<CommandQueue>,
    neighbor_pipeline: ComputePipelineState,
}

#[cfg(feature = "gpu-metal")]
impl MetalBackend {
    /// Create a new Metal backend
    pub fn new() -> Result<Self> {
        // Get the default Metal device
        let device = Device::system_default()
            .ok_or_else(|| Error::InvalidFormat("No Metal device found".to_string()))?;

        // Create command queue
        let command_queue = device.new_command_queue();

        // Compile the compute shader
        let library_source = include_str!("shaders/neighbors.metal");
        let options = CompileOptions::new();
        let library = device
            .new_library_with_source(library_source, &options)
            .map_err(|e| Error::InvalidFormat(format!("Failed to compile Metal shader: {}", e)))?;

        // Get the kernel function
        let kernel_function = library
            .get_function("batch_neighbors", None)
            .map_err(|e| Error::InvalidFormat(format!("Failed to get kernel function: {}", e)))?;

        // Create compute pipeline
        let neighbor_pipeline = device
            .new_compute_pipeline_state_with_function(&kernel_function)
            .map_err(|e| Error::InvalidFormat(format!("Failed to create pipeline: {}", e)))?;

        Ok(Self {
            device: Arc::new(device),
            command_queue: Arc::new(command_queue),
            neighbor_pipeline,
        })
    }

    /// Get the Metal device
    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(feature = "gpu-metal")]
impl GpuBackend for MetalBackend {
    fn is_available(&self) -> bool {
        true // If we got here, Metal is available
    }

    fn name(&self) -> &'static str {
        "Metal"
    }

    fn batch_neighbors(&self, routes: &[Route64]) -> Result<Vec<Route64>> {
        let input_count = routes.len();
        let output_count = input_count * 14; // 14 neighbors per route

        // Convert routes to u64 for GPU processing
        let input_data: Vec<u64> = routes.iter().map(|r| r.value()).collect();

        // Create Metal buffers
        let input_buffer = self.device.new_buffer_with_data(
            input_data.as_ptr() as *const std::ffi::c_void,
            (input_data.len() * std::mem::size_of::<u64>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        let output_buffer = self.device.new_buffer(
            (output_count * std::mem::size_of::<u64>()) as u64,
            MTLResourceOptions::StorageModeShared,
        );

        // Create command buffer and encoder
        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();

        // Set pipeline and buffers
        encoder.set_compute_pipeline_state(&self.neighbor_pipeline);
        encoder.set_buffer(0, Some(&input_buffer), 0);
        encoder.set_buffer(1, Some(&output_buffer), 0);

        // Calculate thread group sizes
        let thread_group_size = MTLSize {
            width: 256,
            height: 1,
            depth: 1,
        };

        let thread_groups = MTLSize {
            width: input_count.div_ceil(256) as u64,
            height: 1,
            depth: 1,
        };

        // Dispatch compute kernel
        encoder.dispatch_thread_groups(thread_groups, thread_group_size);
        encoder.end_encoding();

        // Execute and wait
        command_buffer.commit();
        command_buffer.wait_until_completed();

        // Read results back
        let output_ptr = output_buffer.contents() as *const u64;
        let output_slice = unsafe { std::slice::from_raw_parts(output_ptr, output_count) };

        // Convert back to Route64
        let mut results = Vec::with_capacity(output_count);
        for &value in output_slice {
            results.push(Route64::from_value(value)?);
        }

        Ok(results)
    }

    fn min_batch_size(&self) -> usize {
        1000 // Metal has lower overhead than Vulkan
    }

    fn max_batch_size(&self) -> usize {
        2_000_000 // Metal can handle very large batches
    }
}

#[cfg(not(feature = "gpu-metal"))]
pub struct MetalBackend;

#[cfg(not(feature = "gpu-metal"))]
impl MetalBackend {
    pub fn new() -> Result<Self> {
        Err(Error::InvalidFormat(
            "Metal feature not enabled".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "gpu-metal")]
    fn test_metal_backend_creation() {
        match MetalBackend::new() {
            Ok(backend) => {
                assert!(backend.is_available());
                assert_eq!(backend.name(), "Metal");
                println!("Metal device: {}", backend.device().name());
            }
            Err(e) => {
                println!("Metal not available: {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(feature = "gpu-metal")]
    fn test_metal_batch_neighbors() {
        let backend = match MetalBackend::new() {
            Ok(b) => b,
            Err(_) => return, // Skip if Metal not available
        };

        // Create test routes
        let routes: Vec<Route64> = (0..100)
            .map(|i| {
                let coord = i * 2;
                Route64::new(0, coord, coord, coord).unwrap()
            })
            .collect();

        match backend.batch_neighbors(&routes) {
            Ok(neighbors) => {
                assert_eq!(neighbors.len(), 1400); // 14 * 100
                println!("Metal batch neighbors: {} results", neighbors.len());
            }
            Err(e) => {
                panic!("Metal batch neighbors failed: {:?}", e);
            }
        }
    }
}
