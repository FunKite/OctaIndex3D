//! ESDF Path Planning Example
//!
//! Demonstrates:
//! 1. TSDF reconstruction from depth measurements
//! 2. ESDF computation from TSDF using Fast Marching on BCC lattice
//! 3. Path planning using ESDF gradient descent
//!
//! This showcases the full pipeline: Sensor → TSDF → ESDF → Navigation
//!
//! Run with:
//! ```bash
//! cargo run --release --example esdf_path_planning
//! ```

use octaindex3d::layers::Layer;
use octaindex3d::{ESDFLayer, Index64, Measurement, Result, TSDFLayer};
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║   OctaIndex3D ESDF Path Planning Demo                    ║");
    println!("║   TSDF → ESDF → Navigation on BCC Lattice                ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Configuration
    let truncation = 0.1; // 10cm TSDF truncation
    let voxel_size = 0.02; // 2cm voxels
    let max_esdf_dist = 2.0; // 2m max ESDF distance

    println!("Configuration:");
    println!("  TSDF truncation:     {:.2} m", truncation);
    println!("  Voxel size:          {:.2} cm", voxel_size * 100.0);
    println!("  ESDF max distance:   {:.2} m", max_esdf_dist);
    println!();

    // Step 1: Build TSDF from simulated environment
    println!("Step 1: Building TSDF from simulated environment...");
    let start = Instant::now();

    let mut tsdf = TSDFLayer::with_params(truncation, 100.0, voxel_size);

    // Simulate two obstacles (cylinders)
    add_cylinder_obstacle(&mut tsdf, (1.5, 0.0, 0.0), 0.5, 1.0)?;
    add_cylinder_obstacle(&mut tsdf, (-1.5, 0.0, 0.0), 0.5, 1.0)?;

    let tsdf_stats = tsdf.stats();
    let tsdf_time = start.elapsed();

    println!("✓ TSDF Complete!");
    println!(
        "  Build time:          {:.2} ms",
        tsdf_time.as_secs_f64() * 1000.0
    );
    println!("  Total voxels:        {}", tsdf_stats.voxel_count);
    println!("  Surface voxels:      {}", tsdf_stats.surface_voxel_count);
    println!();

    // Step 2: Compute ESDF from TSDF
    println!("Step 2: Computing ESDF using Fast Marching on BCC lattice...");
    let start = Instant::now();

    let mut esdf = ESDFLayer::new(voxel_size, max_esdf_dist);
    esdf.compute_from_tsdf(&tsdf, voxel_size * 2.0)?;

    let esdf_stats = esdf.stats();
    let esdf_time = start.elapsed();

    println!("✓ ESDF Complete!");
    println!(
        "  Computation time:    {:.2} ms",
        esdf_time.as_secs_f64() * 1000.0
    );
    println!("  Total voxels:        {}", esdf_stats.voxel_count);
    println!("  Free space voxels:   {}", esdf_stats.free_voxel_count);
    println!("  Obstacle voxels:     {}", esdf_stats.obstacle_voxel_count);
    println!(
        "  Distance range:      [{:.3}, {:.3}] m",
        esdf_stats.min_distance, esdf_stats.max_distance
    );
    println!();

    // Step 3: Test collision checking
    println!("Step 3: Collision checking using ESDF...");

    let safety_margin = 0.1; // 10cm safety margin
    let test_points = [
        (0.0, 0.0, 0.0, "Origin"),
        (1.0, 0.0, 0.0, "Near obstacle 1"),
        (2.0, 0.0, 0.0, "Inside obstacle 1"),
        (-1.0, 0.0, 0.0, "Near obstacle 2"),
        (0.0, 1.0, 0.0, "Away from obstacles"),
    ];

    for &(x, y, z, label) in &test_points {
        // Convert to voxel coordinates
        let vx = (x / voxel_size).round() as i16;
        let vy = (y / voxel_size).round() as i16;
        let vz = (z / voxel_size).round() as i16;

        // Ensure in u16 range and BCC parity
        if let Ok((bx, by, bz)) = snap_and_convert(vx, vy, vz) {
            if let Ok(idx) = Index64::new(0, 0, 5, bx, by, bz) {
                if let Some(dist) = esdf.get_distance(idx) {
                    let is_safe = esdf.is_free_space(idx, safety_margin);
                    let status = if is_safe { "✓ SAFE" } else { "✗ UNSAFE" };

                    println!("  {:25} → dist={:6.3}m {}", label, dist, status);
                } else {
                    println!("  {:25} → [not in ESDF]", label);
                }
            }
        }
    }
    println!();

    // Step 4: Show ESDF gradient for navigation
    println!("Step 4: Computing navigation gradients...");

    let nav_points = [(0.0, 0.0, 0.0), (0.5, 0.5, 0.0)];

    for &(x, y, z) in &nav_points {
        let vx = (x / voxel_size).round() as i16;
        let vy = (y / voxel_size).round() as i16;
        let vz = (z / voxel_size).round() as i16;

        if let Ok((bx, by, bz)) = snap_and_convert(vx, vy, vz) {
            if let Ok(idx) = Index64::new(0, 0, 5, bx, by, bz) {
                if let Some((gx, gy, gz)) = esdf.get_gradient(idx) {
                    println!(
                        "  Point ({:.2}, {:.2}, {:.2}) → gradient=({:.3}, {:.3}, {:.3})",
                        x, y, z, gx, gy, gz
                    );
                }
            }
        }
    }
    println!();

    // Summary
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║   Performance Summary                                     ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!(
        "║  TSDF Build:         {:6.2} ms                           ║",
        tsdf_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  ESDF Computation:   {:6.2} ms                           ║",
        esdf_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  Total Pipeline:     {:6.2} ms                           ║",
        (tsdf_time + esdf_time).as_secs_f64() * 1000.0
    );
    println!("║                                                           ║");
    println!("║  BCC Lattice Advantage:                                   ║");
    println!("║  • 14-neighbor connectivity → better isotropy             ║");
    println!("║  • More accurate distance fields than cubic grids         ║");
    println!("║  • Fewer propagation artifacts                            ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("Next Steps:");
    println!("  • A* path planning with ESDF costs");
    println!("  • Dynamic obstacle updates");
    println!("  • GPU-accelerated ESDF computation");
    println!("  • Multi-resolution ESDF pyramid");
    println!();

    Ok(())
}

/// Add a cylindrical obstacle to TSDF
fn add_cylinder_obstacle(
    tsdf: &mut TSDFLayer,
    center: (f32, f32, f32),
    radius: f32,
    height: f32,
) -> Result<()> {
    let voxel_size = tsdf.voxel_size();
    let truncation = tsdf.truncation_distance();

    // Sample points around cylinder surface
    let num_angular = 32;
    let num_height = 10;

    for h in 0..num_height {
        let z = center.2 + (h as f32 / num_height as f32 - 0.5) * height;

        for a in 0..num_angular {
            let angle = 2.0 * std::f32::consts::PI * a as f32 / num_angular as f32;

            // Sample voxels in and around the cylinder
            for r in -2..3 {
                let sample_r = radius + r as f32 * voxel_size * 2.0;
                let sx = center.0 + sample_r * angle.cos();
                let sy = center.1 + sample_r * angle.sin();

                // Convert to voxel coordinates
                let vx = (sx / voxel_size).round() as i16;
                let vy = (sy / voxel_size).round() as i16;
                let vz = (z / voxel_size).round() as i16;

                if let Ok((bx, by, bz)) = snap_and_convert(vx, vy, vz) {
                    if let Ok(idx) = Index64::new(0, 0, 5, bx, by, bz) {
                        // Compute signed distance to cylinder surface
                        let dx = sx - center.0;
                        let dy = sy - center.1;
                        let dist_to_axis = (dx * dx + dy * dy).sqrt();
                        let sdf = radius - dist_to_axis; // Negative outside, positive inside

                        if sdf.abs() <= truncation {
                            tsdf.update(idx, &Measurement::depth(sdf, 1.0))?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Snap to BCC and convert to u16 range
fn snap_and_convert(x: i16, y: i16, z: i16) -> Result<(u16, u16, u16)> {
    let (bx, by, bz) = snap_to_bcc(x as i32, y as i32, z as i32);

    // Check if in u16 range
    if bx >= 0
        && by >= 0
        && bz >= 0
        && bx <= u16::MAX as i32
        && by <= u16::MAX as i32
        && bz <= u16::MAX as i32
    {
        Ok((bx as u16, by as u16, bz as u16))
    } else {
        Err(octaindex3d::Error::InvalidFormat(
            "Coordinates out of range".to_string(),
        ))
    }
}

/// Snap to nearest BCC point (simplified version for example)
fn snap_to_bcc(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    use octaindex3d::layers::snap_to_nearest_bcc;
    snap_to_nearest_bcc(x, y, z)
}
