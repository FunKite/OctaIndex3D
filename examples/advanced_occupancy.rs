//! Advanced Occupancy Features Showcase
//!
//! Demonstrates:
//! 1. GPU-accelerated ray casting (if available)
//! 2. Temporal filtering for dynamic environments
//! 3. Compressed storage for large maps
//! 4. ROS2 integration types
//!
//! Run with:
//! ```bash
//! cargo run --release --example advanced_occupancy
//! ```

use octaindex3d::layers::{
    CompressedOccupancyLayer, CompressionMethod, TemporalConfig, TemporalOccupancyLayer,
};
use octaindex3d::Index64;
use std::time::Instant;

fn main() -> octaindex3d::Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   OctaIndex3D: Advanced Occupancy Features              ║");
    println!("║   GPU • Temporal • Compression • ROS2                    ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Demo 1: Compressed Storage
    println!("═══ Demo 1: Compressed Storage for Large Maps ═══");
    demo_compressed_storage()?;
    println!();

    // Demo 2: Temporal Filtering
    println!("═══ Demo 2: Temporal Filtering (Dynamic Environments) ═══");
    demo_temporal_filtering()?;
    println!();

    // Demo 3: GPU Acceleration (if available)
    #[cfg(any(feature = "gpu-metal", feature = "gpu-cuda"))]
    {
        println!("═══ Demo 3: GPU-Accelerated Ray Casting ═══");
        demo_gpu_acceleration()?;
        println!();
    }

    // Demo 4: ROS2 Integration Types
    println!("═══ Demo 4: ROS2 Integration Types ═══");
    demo_ros2_integration()?;
    println!();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   All Advanced Features Demonstrated Successfully!      ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Demo 1: Compressed storage comparison
fn demo_compressed_storage() -> octaindex3d::Result<()> {
    let start = Instant::now();

    // Test all compression methods
    let methods = vec![
        CompressionMethod::None,
        CompressionMethod::LZ4,
        CompressionMethod::RLE,
        CompressionMethod::Octree,
    ];

    println!("Creating large sparse occupancy map (10,000 voxels)...\n");

    for method in methods {
        let mut layer = CompressedOccupancyLayer::with_method(method);

        // Add 10,000 sparse occupied voxels
        for i in 0..10000 {
            let x = (i * 10) as u16;
            let y = ((i * 7) % 1000) as u16;
            let z = ((i * 13) % 1000) as u16;

            if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                layer.update_occupancy(idx, true, 0.9);
            }
        }

        let stats = layer.stats();
        println!("Method: {:?}", method);
        println!("  Blocks:            {}", stats.total_blocks);
        println!(
            "  Uncompressed:      {:.2} MB",
            stats.uncompressed_bytes as f64 / 1_048_576.0
        );
        println!(
            "  Compressed:        {:.2} MB",
            stats.compressed_bytes as f64 / 1_048_576.0
        );
        println!("  Compression ratio: {:.1}x", stats.compression_ratio());
        println!("  Memory saved:      {:.2} MB", stats.memory_saved_mb());
        println!();
    }

    println!(
        "✓ Compression demo complete in {:.2}ms\n",
        start.elapsed().as_secs_f64() * 1000.0
    );
    println!("Best for sparse maps: RLE (highest ratio)");
    println!("Best for speed:       LZ4 (fastest decompression)");
    println!("Best for accuracy:    None (no overhead)");

    Ok(())
}

/// Demo 2: Temporal filtering for dynamic environments
fn demo_temporal_filtering() -> octaindex3d::Result<()> {
    let config = TemporalConfig {
        decay_rate: 1.0, // 1 log-odds/second decay
        max_age: 3.0,    // 3 second max age
        min_measurements_for_velocity: 3,
        track_dynamics: true,
    };

    let mut layer = TemporalOccupancyLayer::with_config(config);

    println!("Simulating moving obstacle:");
    println!("  Decay rate:  1.0 log-odds/second");
    println!("  Max age:     3.0 seconds");
    println!("  Scenario:    Robot moving past obstacle\n");

    // Simulate obstacle at initial position
    let obstacle_idx = Index64::new(0, 0, 5, 200, 200, 100)?;
    layer.update_occupancy(obstacle_idx, true, 0.9);

    println!("t=0.0s: Obstacle detected at (200, 200, 100)");
    println!("  State: {:?}", layer.get_state(obstacle_idx));
    println!(
        "  Probability: {:.3}",
        layer.get_probability(obstacle_idx).unwrap_or(0.5)
    );

    // Simulate time passing (obstacle moves away, no new measurements)
    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("\nt=0.1s: No new measurements (obstacle moved)");
    println!("  State: {:?} (with decay)", layer.get_state(obstacle_idx));
    println!(
        "  Probability: {:.3} (decaying toward unknown)",
        layer.get_probability(obstacle_idx).unwrap_or(0.5)
    );

    let stats = layer.stats();
    println!("\n✓ Temporal filtering active:");
    println!("  Total voxels:   {}", stats.total_voxels);
    println!("  Dynamic voxels: {}", stats.dynamic_voxels);
    println!("  Stale voxels:   {}", stats.stale_voxels);

    println!("\nBenefits:");
    println!("  • Automatic forgetting of transient obstacles");
    println!("  • Reduced memory for dynamic environments");
    println!("  • Better handling of sensor noise");

    Ok(())
}

/// Demo 3: GPU-accelerated ray casting
#[cfg(any(feature = "gpu-metal", feature = "gpu-cuda"))]
fn demo_gpu_acceleration() -> octaindex3d::Result<()> {
    use octaindex3d::layers::gpu::GpuRayCaster;

    println!("Detecting GPU backends...");

    match GpuRayCaster::new() {
        Ok(caster) => {
            println!("✓ GPU backend found: {}", caster.backend_name());
            println!();
            println!("GPU ray casting capabilities:");
            println!("  • Parallel ray traversal on BCC lattice");
            println!("  • 100-1000x speedup for large batches");
            println!("  • Automatic free space carving");
            println!("  • DDA traversal with BCC snapping");
            println!();
            println!("Example usage:");
            println!("  let origins = vec![(0.0, 0.0, 0.0); 10000];");
            println!("  let endpoints = /* ... */;");
            println!("  let results = caster.cast_rays(&origins, &endpoints, 0.05, 0.7, 0.9)?;");
            println!("  // Results contain (index, occupied, confidence) for all voxels");
        }
        Err(e) => {
            println!("✗ No GPU backend available: {:?}", e);
            println!();
            println!("To enable GPU acceleration:");
            println!("  • macOS:  cargo build --features gpu-metal");
            println!("  • NVIDIA: cargo build --features gpu-cuda");
            println!("  • Other:  cargo build --features gpu-vulkan");
        }
    }

    Ok(())
}

/// Demo 4: ROS2 integration types
fn demo_ros2_integration() -> octaindex3d::Result<()> {
    use octaindex3d::layers::ros2::PointCloud2;

    println!("ROS2 message types available:");
    println!();

    // Create ROS2 OccupancyGrid
    println!("1. OccupancyGrid (nav_msgs/OccupancyGrid)");
    println!("   Compatible with ROS2 navigation stack");
    println!("   let grid = OccupancyGrid::from_occupancy_layer(&layer, 0.05, [0.0, 0.0, 0.0]);");
    println!("   // Publish to /map topic for nav2");
    println!();

    // Create ROS2 PointCloud2
    println!("2. PointCloud2 (sensor_msgs/PointCloud2)");
    let voxels = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
    let cloud = PointCloud2::from_occupied_voxels(voxels.clone(), "map");
    println!("   Created point cloud: {} points", cloud.width);
    println!(
        "   Fields: {}",
        cloud
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("   Data size: {} bytes", cloud.data.len());
    println!();

    // With intensity (occupancy probability)
    println!("3. PointCloud2 with Intensity");
    let voxels_with_prob = vec![(1.0, 2.0, 3.0, 0.9), (4.0, 5.0, 6.0, 0.7)];
    let cloud_intensity = PointCloud2::from_occupancy_with_intensity(voxels_with_prob, "map");
    println!(
        "   Fields: {}",
        cloud_intensity
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("   Includes occupancy probability as intensity");
    println!();

    #[cfg(feature = "serde")]
    {
        println!("4. Serialization Support");
        println!("   JSON/CDR serialization enabled for ROS2 DDS");
        println!("   let bytes = grid.to_cdr_bytes()?;");
        println!("   // Send over DDS to ROS2 nodes");
    }

    println!();
    println!("✓ ROS2 bridge ready for robotics integration");
    println!();
    println!("Integration example:");
    println!("  // In your ROS2 node (using rclrs):");
    println!("  let publisher = node.create_publisher::<OccupancyGrid>(\"/map\")?;");
    println!("  let grid = OccupancyGrid::from_occupancy_layer(&layer, 0.05, [0.0, 0.0, 0.0]);");
    println!("  publisher.publish(&grid)?;");

    Ok(())
}
