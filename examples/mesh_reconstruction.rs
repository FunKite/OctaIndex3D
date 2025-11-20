//! Complete 3D Mesh Reconstruction Pipeline
//!
//! Demonstrates the full pipeline:
//! 1. Build TSDF from simulated depth sensor
//! 2. Extract mesh from TSDF zero-crossings
//! 3. Export to PLY, OBJ, and STL formats
//! 4. Show mesh statistics and quality metrics
//!
//! Run with:
//! ```bash
//! cargo run --release --example mesh_reconstruction
//! ```


use octaindex3d::{
    export_mesh_obj, export_mesh_ply, export_mesh_stl, extract_mesh_from_tsdf, Index64,
    Result, TSDFLayer,
};
use std::time::Instant;

fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║   OctaIndex3D: Complete 3D Reconstruction Pipeline       ║");
    println!("║   Sensor → TSDF → Mesh → Export (PLY/OBJ/STL)            ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Configuration
    let truncation = 0.08; // 8cm truncation
    let voxel_size = 0.01; // 1cm voxels (high resolution!)
    let sphere_radius = 0.5; // 50cm radius sphere
    let sphere_center = (0.0, 0.0, 2.0); // 2m away from sensor

    println!("Configuration:");
    println!("  TSDF truncation:     {:.2} cm", truncation * 100.0);
    println!(
        "  Voxel size:          {:.2} cm (high resolution!)",
        voxel_size * 100.0
    );
    println!(
        "  Scene:               Sphere (r={:.2}cm) at ({:.1}, {:.1}, {:.1})m",
        sphere_radius * 100.0,
        sphere_center.0,
        sphere_center.1,
        sphere_center.2
    );
    println!();

    // Step 1: Build TSDF from simulated depth camera
    println!("Step 1: Building TSDF from simulated depth measurements...");
    let start = Instant::now();

    let mut tsdf = TSDFLayer::with_params(truncation, 100.0, voxel_size);

    // Simulate depth camera scanning the sphere
    let scan_resolution = 80; // 80x80 = 6400 rays
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
                // Update voxels along ray
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

    let tsdf_stats = tsdf.stats();
    let tsdf_time = start.elapsed();

    println!("✓ TSDF Complete!");
    println!(
        "  Build time:          {:.2} ms",
        tsdf_time.as_secs_f64() * 1000.0
    );
    println!(
        "  Measurements:        {} depth readings",
        measurement_count
    );
    println!("  Total voxels:        {}", tsdf_stats.voxel_count);
    println!(
        "  Surface voxels:      {} ({:.1}%)",
        tsdf_stats.surface_voxel_count,
        100.0 * tsdf_stats.surface_voxel_count as f32 / tsdf_stats.voxel_count.max(1) as f32
    );
    println!(
        "  Distance range:      [{:.3}, {:.3}] m",
        tsdf_stats.min_distance, tsdf_stats.max_distance
    );
    println!();

    // Step 2: Extract mesh from TSDF
    println!("Step 2: Extracting mesh from TSDF zero-crossings...");
    let start = Instant::now();

    let mesh = extract_mesh_from_tsdf(&tsdf)?;

    let mesh_stats = mesh.stats();
    let mesh_time = start.elapsed();

    println!("✓ Mesh Extraction Complete!");
    println!(
        "  Extraction time:     {:.2} ms",
        mesh_time.as_secs_f64() * 1000.0
    );
    println!("  Vertices:            {}", mesh_stats.vertex_count);
    println!("  Triangles:           {}", mesh_stats.triangle_count);
    println!(
        "  Has normals:         {}",
        if mesh_stats.has_normals { "Yes" } else { "No" }
    );

    if mesh_stats.vertex_count > 0 {
        let surface_area = mesh.surface_area();
        println!("  Surface area:        {:.4} m²", surface_area);

        // Compare to theoretical sphere surface area: 4πr²
        let theoretical_area = 4.0 * std::f32::consts::PI * sphere_radius * sphere_radius;
        let area_error = ((surface_area - theoretical_area) / theoretical_area * 100.0).abs();
        println!("  Theoretical area:    {:.4} m² (sphere)", theoretical_area);
        println!("  Area accuracy:       {:.1}% error", area_error);

        if let Some((min, max)) = mesh.bounding_box() {
            println!("  Bounding box:");
            println!("    Min: ({:.3}, {:.3}, {:.3})", min[0], min[1], min[2]);
            println!("    Max: ({:.3}, {:.3}, {:.3})", max[0], max[1], max[2]);
        }
    }
    println!();

    // Step 3: Export mesh to various formats
    println!("Step 3: Exporting mesh to file formats...");

    if mesh_stats.vertex_count > 0 {
        // Export PLY (ASCII)
        let ply_path = "/tmp/reconstruction.ply";
        export_mesh_ply(&mesh, ply_path, false)?;
        println!("  ✓ Exported to PLY (ASCII):    {}", ply_path);

        // Export PLY (Binary)
        let ply_bin_path = "/tmp/reconstruction_binary.ply";
        export_mesh_ply(&mesh, ply_bin_path, true)?;
        println!("  ✓ Exported to PLY (Binary):   {}", ply_bin_path);

        // Export OBJ
        let obj_path = "/tmp/reconstruction.obj";
        export_mesh_obj(&mesh, obj_path)?;
        println!("  ✓ Exported to OBJ:            {}", obj_path);

        // Export STL (ASCII)
        let stl_path = "/tmp/reconstruction.stl";
        export_mesh_stl(&mesh, stl_path, false)?;
        println!("  ✓ Exported to STL (ASCII):    {}", stl_path);

        // Export STL (Binary)
        let stl_bin_path = "/tmp/reconstruction_binary.stl";
        export_mesh_stl(&mesh, stl_bin_path, true)?;
        println!("  ✓ Exported to STL (Binary):   {}", stl_bin_path);

        println!();
        println!("Files exported to /tmp/ - open in your favorite 3D viewer!");
        println!("  MeshLab:     meshlab {}", ply_path);
        println!("  Blender:     blender {}", obj_path);
        println!("  CloudCompare: CloudCompare {}", ply_path);
    } else {
        println!("  ⚠ No mesh extracted (no surface found)");
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
        "║  Mesh Extraction:    {:6.2} ms                           ║",
        mesh_time.as_secs_f64() * 1000.0
    );
    println!(
        "║  Total Pipeline:     {:6.2} ms                           ║",
        (tsdf_time + mesh_time).as_secs_f64() * 1000.0
    );
    println!("║                                                           ║");
    println!("║  Quality Metrics:                                         ║");
    println!(
        "║  • {:6} vertices, {:6} triangles                       ║",
        mesh_stats.vertex_count, mesh_stats.triangle_count
    );
    if mesh_stats.vertex_count > 0 {
        let surface_area = mesh.surface_area();
        let theoretical_area = 4.0 * std::f32::consts::PI * sphere_radius * sphere_radius;
        let area_error = ((surface_area - theoretical_area) / theoretical_area * 100.0).abs();
        println!(
            "║  • Surface area accuracy: {:.1}% error                  ║",
            area_error
        );
    }
    println!("║                                                           ║");
    println!("║  BCC Lattice Advantages:                                  ║");
    println!("║  • 14-neighbor connectivity → smoother surfaces           ║");
    println!("║  • Better isotropy → fewer mesh artifacts                 ║");
    println!("║  • Efficient zero-crossing detection                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    Ok(())
}

/// Ray-sphere intersection (returns distance to hit point)
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

/// Update TSDF along a ray
fn update_ray(
    tsdf: &mut TSDFLayer,
    ray_origin: (f32, f32, f32),
    ray_dir: (f32, f32, f32),
    depth: f32,
    voxel_size: f32,
    truncation: f32,
) -> Result<()> {
    use octaindex3d::layers::snap_to_nearest_bcc;

    let start_dist = (depth - truncation).max(0.1);
    let end_dist = depth + truncation;
    let step_size = voxel_size * 0.5;

    let mut dist = start_dist;
    while dist <= end_dist {
        let pos = (
            ray_origin.0 + ray_dir.0 * dist,
            ray_origin.1 + ray_dir.1 * dist,
            ray_origin.2 + ray_dir.2 * dist,
        );

        let voxel_x = (pos.0 / voxel_size).round() as i32;
        let voxel_y = (pos.1 / voxel_size).round() as i32;
        let voxel_z = (pos.2 / voxel_size).round() as i32;

        let (vx, vy, vz) = snap_to_nearest_bcc(voxel_x, voxel_y, voxel_z);

        if vx >= 0
            && vy >= 0
            && vz >= 0
            && vx <= u16::MAX as i32
            && vy <= u16::MAX as i32
            && vz <= u16::MAX as i32
        {
            if let Ok(idx) = Index64::new(0, 0, 5, vx as u16, vy as u16, vz as u16) {
                tsdf.update_from_depth_ray(idx, ray_origin, depth, 1.0)?;
            }
        }

        dist += step_size;
    }

    Ok(())
}
