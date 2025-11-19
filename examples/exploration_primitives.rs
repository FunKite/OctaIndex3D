//! Exploration Primitives for Autonomous Navigation
//!
//! Demonstrates frontier detection and information gain calculation
//! as building blocks for active exploration strategies.
//!
//! Run with:
//! ```bash
//! cargo run --release --example exploration_primitives
//! ```

use octaindex3d::layers::{
    Frontier, FrontierDetectionConfig, InformationGainConfig, OccupancyLayer,
};
use octaindex3d::Index64;
use std::time::Instant;

fn main() -> octaindex3d::Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   OctaIndex3D: Exploration Primitives                   ║");
    println!("║   Frontier Detection • Information Gain • NBV Planning   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Demo 1: Frontier Detection
    println!("═══ Demo 1: Frontier Detection ═══");
    demo_frontier_detection()?;
    println!();

    // Demo 2: Information Gain Calculation
    println!("═══ Demo 2: Information Gain from Viewpoints ═══");
    demo_information_gain()?;
    println!();

    // Demo 3: Viewpoint Candidate Generation
    println!("═══ Demo 3: Next-Best-View Candidate Generation ═══");
    demo_viewpoint_candidates()?;
    println!();

    // Demo 4: Building a Simple Exploration Strategy
    println!("═══ Demo 4: Simple Exploration Strategy ═══");
    demo_exploration_strategy()?;
    println!();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   Exploration Primitives Ready for Use!                 ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Demo 1: Frontier detection from partially explored map
fn demo_frontier_detection() -> octaindex3d::Result<()> {
    let mut layer = OccupancyLayer::new();
    let start = Instant::now();

    println!("Creating partially explored environment:");
    println!("  • Central explored region (5x5x2m)");
    println!("  • Surrounding unknown space");
    println!();

    // Simulate explored region (robot has scanned the center)
    for x in 0..100 {
        for y in 0..100 {
            for z in 0..40 {
                if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                    // Mark as free (explored)
                    layer.update_occupancy(idx, false, 0.8);
                }
            }
        }
    }

    // Add some obstacles
    for x in 40..60 {
        for y in 40..60 {
            for z in 0..40 {
                if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                    layer.update_occupancy(idx, true, 0.9);
                }
            }
        }
    }

    let stats = layer.stats();
    println!("Map statistics:");
    println!(
        "  Free voxels:     {} ({:.1}%)",
        stats.free_count,
        100.0 * stats.free_count as f32 / stats.total_voxels as f32
    );
    println!(
        "  Occupied voxels: {} ({:.1}%)",
        stats.occupied_count,
        100.0 * stats.occupied_count as f32 / stats.total_voxels as f32
    );
    println!(
        "  Unknown voxels:  {} ({:.1}%)",
        stats.unknown_count,
        100.0 * stats.unknown_count as f32 / stats.total_voxels as f32
    );
    println!();

    // Detect frontiers
    let frontier_config = FrontierDetectionConfig {
        min_cluster_size: 10,
        max_distance: 10.0,
        cluster_distance: 0.3,
    };

    let frontiers = layer.detect_frontiers(&frontier_config)?;
    let elapsed = start.elapsed();

    println!(
        "✓ Frontier detection complete in {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!("  Found {} frontier clusters", frontiers.len());

    if !frontiers.is_empty() {
        println!("\n  Top 3 largest frontiers:");
        for (i, frontier) in frontiers.iter().take(3).enumerate() {
            println!(
                "    {}. Size: {} voxels, Centroid: ({:.2}, {:.2}, {:.2})",
                i + 1,
                frontier.size,
                frontier.centroid.0,
                frontier.centroid.1,
                frontier.centroid.2
            );
        }
    }

    println!("\nInterpretation:");
    println!("  • Frontiers mark the boundary between known and unknown");
    println!("  • Largest frontiers typically represent unexplored corridors");
    println!("  • Smaller frontiers may be occluded areas behind obstacles");

    Ok(())
}

/// Demo 2: Information gain calculation from different viewpoints
fn demo_information_gain() -> octaindex3d::Result<()> {
    let layer = OccupancyLayer::new();

    let ig_config = InformationGainConfig {
        sensor_range: 5.0,
        sensor_fov: std::f32::consts::PI / 3.0, // 60°
        ray_resolution: 5.0,
        unknown_weight: 1.0,
    };

    println!("Testing viewpoints around frontier at (2.5, 2.5, 1.0):");
    println!("  Sensor: 5m range, 60° FOV, 5° ray resolution\n");

    let frontier_pos = (2.5, 2.5, 1.0);
    let test_positions = [
        ((1.0, 2.5, 1.0), "Close, direct"),
        ((0.0, 2.5, 1.0), "Far, direct"),
        ((1.0, 3.5, 1.0), "Close, oblique"),
        ((3.5, 3.5, 1.0), "Side view"),
    ];

    for (pos, description) in &test_positions {
        let direction = (
            frontier_pos.0 - pos.0,
            frontier_pos.1 - pos.1,
            frontier_pos.2 - pos.2,
        );

        let ig = layer.information_gain_from(*pos, direction, &ig_config);

        println!("  {}: IG = {:.2} bits", description, ig);
    }

    println!("\nKey insights:");
    println!("  • Closer viewpoints typically have higher information gain");
    println!("  • Direct views capture more unknown voxels than oblique");
    println!("  • Multiple viewpoints can cover occluded regions");

    Ok(())
}

/// Demo 3: Generate and rank viewpoint candidates
fn demo_viewpoint_candidates() -> octaindex3d::Result<()> {
    let layer = OccupancyLayer::new();
    let start = Instant::now();

    // Create mock frontiers
    let frontiers = vec![
        Frontier {
            centroid: (5.0, 0.0, 1.0),
            voxels: Vec::new(),
            information_gain: 0.0,
            size: 50,
        },
        Frontier {
            centroid: (0.0, 5.0, 1.0),
            voxels: Vec::new(),
            information_gain: 0.0,
            size: 30,
        },
        Frontier {
            centroid: (-3.0, -3.0, 1.0),
            voxels: Vec::new(),
            information_gain: 0.0,
            size: 20,
        },
    ];

    let ig_config = InformationGainConfig::default();

    println!(
        "Generating viewpoint candidates for {} frontiers:",
        frontiers.len()
    );
    println!("  • 3 distances: 1m, 2m, 3m");
    println!("  • 8 angles around each frontier");
    println!("  • = 72 total candidates\n");

    let candidates = layer.generate_viewpoint_candidates(&frontiers, &ig_config);
    let elapsed = start.elapsed();

    println!(
        "✓ Generated {} candidates in {:.2}ms\n",
        candidates.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    println!("Top 5 candidates by information gain:");
    for (i, candidate) in candidates.iter().take(5).enumerate() {
        println!(
            "  {}. Pos: ({:.2}, {:.2}, {:.2}), IG: {:.2} bits, Frontier: {}",
            i + 1,
            candidate.position.0,
            candidate.position.1,
            candidate.position.2,
            candidate.information_gain,
            candidate.frontier_id
        );
    }

    println!("\nUsage in your planner:");
    println!("  1. Filter candidates by reachability/collision-free");
    println!("  2. Consider path cost to each candidate");
    println!("  3. Select optimal: max(IG - λ × cost)");
    println!("  4. Execute motion and update map");
    println!("  5. Repeat until exploration complete");

    Ok(())
}

/// Demo 4: Simple greedy exploration strategy
fn demo_exploration_strategy() -> octaindex3d::Result<()> {
    println!("Example: Greedy Next-Best-View planner\n");
    println!("```rust");
    println!("fn next_best_view(");
    println!("    layer: &OccupancyLayer,");
    println!("    robot_pos: (f32, f32, f32),");
    println!(") -> Option<Viewpoint> {{");
    println!("    // 1. Detect frontiers");
    println!("    let frontier_config = FrontierDetectionConfig::default();");
    println!("    let frontiers = layer.detect_frontiers(&frontier_config)?;");
    println!("    ");
    println!("    if frontiers.is_empty() {{");
    println!("        return None; // Exploration complete!");
    println!("    }}");
    println!("    ");
    println!("    // 2. Generate viewpoint candidates");
    println!("    let ig_config = InformationGainConfig::default();");
    println!("    let mut candidates = layer.generate_viewpoint_candidates(");
    println!("        &frontiers, &ig_config");
    println!("    );");
    println!("    ");
    println!("    // 3. Score by: IG - λ × distance");
    println!("    let lambda = 0.1; // Cost weight");
    println!("    candidates.iter_mut().for_each(|c| {{");
    println!("        let dist = distance(robot_pos, c.position);");
    println!("        c.information_gain -= lambda * dist;");
    println!("    }});");
    println!("    ");
    println!("    // 4. Return best candidate");
    println!("    candidates.sort_by(|a, b|");
    println!("        b.information_gain.partial_cmp(&a.information_gain).unwrap()");
    println!("    );");
    println!("    candidates.first().cloned()");
    println!("}}");
    println!("```\n");

    println!("This is just one strategy! You can implement:");
    println!("  • Frontier-based exploration (visit nearest frontier)");
    println!("  • Coverage-based (maximize sensor coverage)");
    println!("  • Semantic-aware (prioritize regions of interest)");
    println!("  • Multi-robot coordination (divide frontiers)");
    println!("  • Uncertainty-aware (balance exploration vs mapping quality)");

    println!("\nKey advantages of these primitives:");
    println!("  ✓ No prescribed policy - you control the strategy");
    println!("  ✓ Efficient: frontier detection runs in O(n log n)");
    println!("  ✓ Flexible: works with any sensor model");
    println!("  ✓ Composable: combine with path planning, SLAM, etc.");

    Ok(())
}
