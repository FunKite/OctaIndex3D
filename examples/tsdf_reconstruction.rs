//! TSDF Reconstruction Example
//!
//! Demonstrates truncated signed distance field (TSDF) reconstruction
//! on BCC lattice using simulated depth camera measurements.
//!
//! This example:
//! 1. Creates a TSDF layer
//! 2. Simulates depth sensor scanning a simple scene (sphere)
//! 3. Integrates measurements incrementally
//! 4. Extracts surface voxels and zero-crossings
//! 5. Shows statistics and performance metrics
//!
//! Run with:
//! ```bash
//! cargo run --release --example tsdf_reconstruction
//! ```

use octaindex3d::layers::Layer;
use octaindex3d::{Index64, LayeredMap, Result, TSDFLayer};
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║   OctaIndex3D TSDF Reconstruction Demo                   ║");
    println!("║   BCC Lattice Surface Reconstruction                     ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Configuration
    let truncation = 0.1; // 10cm truncation distance
    let voxel_size = 0.02; // 2cm voxels
    let sphere_radius = 1.0; // 1m radius sphere
    let sphere_center = (0.0, 0.0, 3.0); // 3m away from sensor

    println!("Configuration:");
    println!("  Truncation distance: {:.2} m", truncation);
    println!("  Voxel size:          {:.2} cm", voxel_size * 100.0);
    println!(
        "  Scene:               Sphere (r={:.2}m) at ({:.1}, {:.1}, {:.1})",
        sphere_radius, sphere_center.0, sphere_center.1, sphere_center.2
    );
    println!();

    // Create layered map with TSDF
    let mut map = LayeredMap::new();
    let mut tsdf = TSDFLayer::with_params(truncation, 100.0, voxel_size);

    println!("Building TSDF from simulated depth measurements...");
    let start = Instant::now();

    // Simulate depth camera: scan a grid of rays
    let scan_resolution = 50; // 50x50 depth image
    let fov = std::f32::consts::PI / 3.0; // 60 degree FOV
    let mut measurement_count = 0;

    for u in 0..scan_resolution {
        for v in 0..scan_resolution {
            // Convert pixel to ray direction
            let u_norm = (u as f32 / scan_resolution as f32 - 0.5) * 2.0;
            let v_norm = (v as f32 / scan_resolution as f32 - 0.5) * 2.0;
            let angle_u = u_norm * fov / 2.0;
            let angle_v = v_norm * fov / 2.0;

            let ray_dir = (
                angle_u.sin(),
                angle_v.sin(),
                (1.0 - angle_u.powi(2) - angle_v.powi(2)).max(0.0).sqrt(),
            );

            // Ray-sphere intersection
            if let Some(depth) =
                ray_sphere_intersection((0.0, 0.0, 0.0), ray_dir, sphere_center, sphere_radius)
            {
                // Cast ray and update voxels along it
                update_ray(
                    &mut tsdf,
                    (0.0, 0.0, 0.0),
                    ray_dir,
                    depth,
                    voxel_size,
                    truncation,
                )?;
                measurement_count += 1;
            }
        }
    }

    let build_time = start.elapsed();

    // Get statistics before adding to map
    let stats = tsdf.stats();
    let surface_voxels = tsdf.get_surface_voxels(voxel_size * 2.0);
    let zero_crossings = tsdf.get_zero_crossing_edges();
    let mem_usage = tsdf.memory_usage();

    // Add TSDF to map
    map.add_tsdf_layer(tsdf);

    println!("\n✓ TSDF Construction Complete!");
    println!("\nPerformance:");
    println!(
        "  Build time:          {:.2} ms",
        build_time.as_secs_f64() * 1000.0
    );
    println!(
        "  Measurements:        {} depth readings",
        measurement_count
    );
    println!(
        "  Throughput:          {:.0} measurements/sec",
        measurement_count as f64 / build_time.as_secs_f64()
    );

    println!("\nTSDF Statistics:");
    println!("  Total voxels:        {}", stats.voxel_count);
    println!(
        "  Surface voxels:      {} ({:.1}%)",
        stats.surface_voxel_count,
        100.0 * stats.surface_voxel_count as f32 / stats.voxel_count as f32
    );
    println!(
        "  Distance range:      [{:.3}, {:.3}] m",
        stats.min_distance, stats.max_distance
    );
    println!("  Average weight:      {:.1}", stats.average_weight);
    println!("  Zero crossings:      {} edges", zero_crossings.len());
    println!("  Memory usage:        {:.2} KB", mem_usage as f64 / 1024.0);

    println!("\nSurface Voxel Samples (first 5):");
    for (i, &idx) in surface_voxels.iter().take(5).enumerate() {
        let dist = map.query_tsdf(idx).unwrap();
        let (x, y, z) = idx.decode_coords();
        println!("  [{}] Index64({}, {}, {}) → dist={:.4}m", i, x, y, z, dist);
    }

    println!("\nZero Crossing Samples (first 5):");
    for (i, &(idx1, idx2)) in zero_crossings.iter().take(5).enumerate() {
        let d1 = map.query_tsdf(idx1).unwrap();
        let d2 = map.query_tsdf(idx2).unwrap();
        let (x1, y1, z1) = idx1.decode_coords();
        let (x2, y2, z2) = idx2.decode_coords();
        println!(
            "  [{}] ({}, {}, {}) <-> ({}, {}, {}) | dist: {:.4} <-> {:.4}",
            i, x1, y1, z1, x2, y2, z2, d1, d2
        );
    }

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║   Next Steps                                              ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  • Marching cubes for mesh extraction                     ║");
    println!("║  • GPU-accelerated batch integration                      ║");
    println!("║  • Multi-view fusion with camera poses                    ║");
    println!("║  • ESDF computation from TSDF                             ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    Ok(())
}

/// Ray-sphere intersection
/// Returns distance to intersection point, or None if no hit
fn ray_sphere_intersection(
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    sphere_center: (f32, f32, f32),
    sphere_radius: f32,
) -> Option<f32> {
    let oc = (
        ray_origin.0 - sphere_center.0,
        ray_origin.1 - sphere_center.1,
        ray_origin.2 - sphere_center.2,
    );

    let a = ray_dir.0 * ray_dir.0 + ray_dir.1 * ray_dir.1 + ray_dir.2 * ray_dir.2;
    let b = 2.0 * (oc.0 * ray_dir.0 + oc.1 * ray_dir.1 + oc.2 * ray_dir.2);
    let c = oc.0 * oc.0 + oc.1 * oc.1 + oc.2 * oc.2 - sphere_radius * sphere_radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        None
    } else {
        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        if t > 0.0 {
            Some(t)
        } else {
            None
        }
    }
}

/// Update TSDF along a ray from sensor to measured depth
fn update_ray(
    tsdf: &mut TSDFLayer,
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    depth: f32,
    voxel_size: f32,
    truncation: f32,
) -> Result<()> {
    // Sample points along ray from (depth - truncation) to (depth + truncation)
    let start_dist = (depth - truncation).max(0.1);
    let end_dist = depth + truncation;
    let step_size = voxel_size * 0.5; // Sample at half voxel resolution

    let mut dist = start_dist;
    while dist <= end_dist {
        // Compute 3D position
        let pos = (
            ray_origin.0 + ray_dir.0 * dist,
            ray_origin.1 + ray_dir.1 * dist,
            ray_origin.2 + ray_dir.2 * dist,
        );

        // Convert to voxel coordinates
        let voxel_x = (pos.0 / voxel_size).round() as i32;
        let voxel_y = (pos.1 / voxel_size).round() as i32;
        let voxel_z = (pos.2 / voxel_size).round() as i32;

        // Snap to valid BCC lattice point (ensure identical parity)
        let (vx, vy, vz) = snap_to_bcc(voxel_x, voxel_y, voxel_z);

        // Create Index64 (convert to u16)
        if vx >= 0
            && vy >= 0
            && vz >= 0
            && vx <= u16::MAX as i32
            && vy <= u16::MAX as i32
            && vz <= u16::MAX as i32
        {
            if let Ok(idx) = Index64::new(0, 0, 5, vx as u16, vy as u16, vz as u16) {
                // Update TSDF with camera ray (more accurate than direct depth)
                tsdf.update_from_depth_ray(idx, ray_origin, depth, 1.0)?;
            }
        }

        dist += step_size;
    }

    Ok(())
}

/// Snap coordinates to NEAREST valid BCC lattice point
/// BCC requires all coordinates to have identical parity (all even or all odd)
///
/// This algorithm finds the closest valid BCC point by considering all 8 candidates
/// (formed by rounding each axis up or down while maintaining parity).
/// Maximum error: √(3/4) ≈ 0.866 voxels (vs naive snapping: √3 ≈ 1.73 voxels)
fn snap_to_bcc(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    let x_even = x % 2 == 0;
    let y_even = y % 2 == 0;
    let z_even = z % 2 == 0;

    // Check if already valid
    if x_even == y_even && y_even == z_even {
        return (x, y, z);
    }

    // Generate 8 candidate points by rounding each axis up/down while maintaining parity
    // We need 4 "all even" candidates and 4 "all odd" candidates
    let candidates = [
        // All even candidates (clear bit 0)
        (x & !1, y & !1, z & !1),                   // (floor, floor, floor)
        ((x + 1) & !1, y & !1, z & !1),             // (ceil, floor, floor)
        (x & !1, (y + 1) & !1, z & !1),             // (floor, ceil, floor)
        ((x + 1) & !1, (y + 1) & !1, z & !1),       // (ceil, ceil, floor)
        (x & !1, y & !1, (z + 1) & !1),             // (floor, floor, ceil)
        ((x + 1) & !1, y & !1, (z + 1) & !1),       // (ceil, floor, ceil)
        (x & !1, (y + 1) & !1, (z + 1) & !1),       // (floor, ceil, ceil)
        ((x + 1) & !1, (y + 1) & !1, (z + 1) & !1), // (ceil, ceil, ceil)
    ];

    // Find nearest even candidate
    let mut best_even = candidates[0];
    let mut best_even_dist = distance_squared(x, y, z, candidates[0]);

    for &candidate in &candidates[1..] {
        let dist = distance_squared(x, y, z, candidate);
        if dist < best_even_dist {
            best_even_dist = dist;
            best_even = candidate;
        }
    }

    // Generate all odd candidates (set bit 0)
    let odd_candidates = [
        (x | 1, y | 1, z | 1),                   // (ceil_odd, ceil_odd, ceil_odd)
        ((x - 1) | 1, y | 1, z | 1),             // (floor_odd, ceil_odd, ceil_odd)
        (x | 1, (y - 1) | 1, z | 1),             // (ceil_odd, floor_odd, ceil_odd)
        ((x - 1) | 1, (y - 1) | 1, z | 1),       // (floor_odd, floor_odd, ceil_odd)
        (x | 1, y | 1, (z - 1) | 1),             // (ceil_odd, ceil_odd, floor_odd)
        ((x - 1) | 1, y | 1, (z - 1) | 1),       // (floor_odd, ceil_odd, floor_odd)
        (x | 1, (y - 1) | 1, (z - 1) | 1),       // (ceil_odd, floor_odd, floor_odd)
        ((x - 1) | 1, (y - 1) | 1, (z - 1) | 1), // (floor_odd, floor_odd, floor_odd)
    ];

    // Find nearest odd candidate
    let mut best_odd = odd_candidates[0];
    let mut best_odd_dist = distance_squared(x, y, z, odd_candidates[0]);

    for &candidate in &odd_candidates[1..] {
        let dist = distance_squared(x, y, z, candidate);
        if dist < best_odd_dist {
            best_odd_dist = dist;
            best_odd = candidate;
        }
    }

    // Return whichever is closer: best even or best odd
    if best_even_dist <= best_odd_dist {
        best_even
    } else {
        best_odd
    }
}

/// Compute squared distance between two points (avoids sqrt for comparison)
#[inline]
fn distance_squared(x1: i32, y1: i32, z1: i32, p2: (i32, i32, i32)) -> i32 {
    let dx = x1 - p2.0;
    let dy = y1 - p2.1;
    let dz = z1 - p2.2;
    dx * dx + dy * dy + dz * dz
}
