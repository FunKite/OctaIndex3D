//! wgpu/Vulkan GPU backend for cross-platform acceleration
//!
//! This module provides GPU acceleration via wgpu, which supports:
//! - Vulkan (Linux, Windows, Android)
//! - Metal (macOS, iOS) - as fallback
//! - DirectX 12 (Windows)
//! - WebGPU (browsers)

use super::GpuBackend;
use crate::error::{Error, Result};
use crate::Route64;

#[cfg(feature = "gpu-vulkan")]
use wgpu;

#[cfg(feature = "gpu-vulkan")]
use std::sync::Arc;

/// wgpu GPU backend implementation
#[cfg(feature = "gpu-vulkan")]
pub struct WgpuBackend {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::ComputePipeline,
}

#[cfg(feature = "gpu-vulkan")]
impl WgpuBackend {
    /// Create a new wgpu backend
    pub fn new() -> Result<Self> {
        use pollster;

        // Initialize wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|e| Error::InvalidFormat(format!("No suitable GPU adapter found: {}", e)))?;

        // Request device and queue
        // Note: SHADER_INT64 is required for u64 operations but not widely supported
        // This will fail gracefully on GPUs without 64-bit integer support
        let features = adapter.features();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("OctaIndex3D Compute Device"),
                required_features: features & wgpu::Features::SHADER_INT64,
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::default(),
            },
        ))
        .map_err(|e| {
            Error::InvalidFormat(format!(
                "Failed to create device (SHADER_INT64 may not be supported): {}",
                e
            ))
        })?;

        // Load and compile compute shader
        let shader_source = include_str!("shaders/neighbors.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neighbor Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Neighbor Compute Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("batch_neighbors"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            pipeline,
        })
    }

    /// Get the wgpu device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get the wgpu queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[cfg(feature = "gpu-vulkan")]
impl GpuBackend for WgpuBackend {
    fn is_available(&self) -> bool {
        true // If we got here, wgpu is available
    }

    fn name(&self) -> &'static str {
        "wgpu (Vulkan/Metal/DX12)"
    }

    fn batch_neighbors(&self, routes: &[Route64]) -> Result<Vec<Route64>> {
        let input_count = routes.len();
        let output_count = input_count * 14;

        // Convert routes to u64 for GPU processing
        let input_data: Vec<u64> = routes.iter().map(|r| r.value()).collect();
        let input_bytes = bytemuck::cast_slice(&input_data);

        // Create GPU buffers
        let input_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input Routes Buffer"),
            size: input_bytes.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Neighbors Buffer"),
            size: (output_count * std::mem::size_of::<u64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (output_count * std::mem::size_of::<u64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Upload input data
        self.queue.write_buffer(&input_buffer, 0, input_bytes);

        // Create bind group
        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Create and submit command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Neighbor Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroup_count = input_count.div_ceil(256) as u32;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // Copy output to staging buffer
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (output_count * std::mem::size_of::<u64>()) as u64,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|e| Error::InvalidFormat(format!("Failed to poll device: {}", e)))?;

        receiver
            .recv()
            .map_err(|e| Error::InvalidFormat(format!("Failed to receive buffer mapping: {}", e)))?
            .map_err(|e| Error::InvalidFormat(format!("Failed to map buffer: {}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let output_data: Vec<u64> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        // Convert back to Route64
        let mut results = Vec::with_capacity(output_count);
        for &value in &output_data {
            results.push(Route64::from_value(value)?);
        }

        Ok(results)
    }

    fn min_batch_size(&self) -> usize {
        2000 // wgpu has moderate overhead
    }

    fn max_batch_size(&self) -> usize {
        1_000_000
    }
}

#[cfg(not(feature = "gpu-vulkan"))]
pub struct WgpuBackend;

#[cfg(not(feature = "gpu-vulkan"))]
impl WgpuBackend {
    pub fn new() -> Result<Self> {
        Err(Error::InvalidFormat("wgpu feature not enabled".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "gpu-vulkan")]
    fn test_wgpu_backend_creation() {
        match WgpuBackend::new() {
            Ok(backend) => {
                assert!(backend.is_available());
                assert_eq!(backend.name(), "wgpu (Vulkan/Metal/DX12)");
                println!("wgpu backend initialized");
            }
            Err(e) => {
                println!("wgpu not available: {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(feature = "gpu-vulkan")]
    fn test_wgpu_batch_neighbors() {
        let backend = match WgpuBackend::new() {
            Ok(b) => b,
            Err(_) => return, // Skip if wgpu not available
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
                println!("wgpu batch neighbors: {} results", neighbors.len());
            }
            Err(e) => {
                panic!("wgpu batch neighbors failed: {:?}", e);
            }
        }
    }
}
