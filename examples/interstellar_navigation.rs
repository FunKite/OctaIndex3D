//! Interstellar Navigation Demo
//!
//! An alien mothership uses OctaIndex3D for multi-scale navigation from intergalactic
//! space down to Earth's surface, showcasing hierarchical spatial indexing, obstacle
//! avoidance, and sensor scanning across astronomical scales.

use octaindex3d::layer::{CellFlags, Layer};
use octaindex3d::path::{astar, k_ring, AvoidBlockedCost, EuclideanCost};
use octaindex3d::{CellID, Result};
use std::thread;
use std::time::Duration;

/// Navigation phase with corresponding scale and description
#[derive(Debug, Clone)]
struct NavigationPhase {
    name: String,
    resolution: u8,
    start_coords: (i32, i32, i32),
    goal_coords: (i32, i32, i32),
    scale_description: String,
    scale_units: String,
    obstacle_density: f64,
}

/// Statistics for the navigation system
struct NavStats {
    cells_explored: usize,
    obstacles_detected: usize,
    _path_length: usize,
    _total_distance: f64,
}

fn main() -> Result<()> {
    clear_screen();

    println!("\n{}", "═".repeat(80));
    println!("{:^80}", "⚡ INTERSTELLAR NAVIGATION SYSTEM ⚡");
    println!("{:^80}", "Powered by OctaIndex3D Technology");
    println!("{}", "═".repeat(80));

    sleep(100);

    // Define navigation phases from intergalactic to planetary
    let phases = vec![
        NavigationPhase {
            name: "INTERGALACTIC APPROACH".to_string(),
            resolution: 3,
            start_coords: (-40, -40, -40),
            goal_coords: (0, 0, 0),
            scale_description: "Approaching Local Group from ~25 million light-years".to_string(),
            scale_units: "1 cell ≈ 6.25 million light-years".to_string(),
            obstacle_density: 0.05,
        },
        NavigationPhase {
            name: "GALACTIC APPROACH".to_string(),
            resolution: 13,
            start_coords: (0, 0, 0),
            goal_coords: (50, 60, 40),
            scale_description: "Navigating through Milky Way to Solar System".to_string(),
            scale_units: "1 cell ≈ 6,100 light-years".to_string(),
            obstacle_density: 0.12,
        },
        NavigationPhase {
            name: "SOLAR SYSTEM ENTRY".to_string(),
            resolution: 23,
            start_coords: (400, 480, 320),
            goal_coords: (500, 520, 380),
            scale_description: "Navigating asteroid belt to inner system".to_string(),
            scale_units: "1 cell ≈ 5.96 light-years".to_string(),
            obstacle_density: 0.20,
        },
        NavigationPhase {
            name: "EARTH APPROACH".to_string(),
            resolution: 28,
            start_coords: (4000, 4160, 3040),
            goal_coords: (4100, 4200, 3080),
            scale_description: "Final approach to Earth orbit".to_string(),
            scale_units: "1 cell ≈ 186,000 km".to_string(),
            obstacle_density: 0.15,
        },
    ];

    let mut total_cells_explored = 0;
    let mut total_obstacles = 0;

    // Execute each navigation phase
    for (phase_num, phase) in phases.iter().enumerate() {
        let stats = execute_phase(phase_num + 1, phase)?;
        total_cells_explored += stats.cells_explored;
        total_obstacles += stats.obstacles_detected;

        sleep(50);
    }

    // Probe deployment sequence
    deploy_probes()?;

    // Final summary
    print_final_summary(total_cells_explored, total_obstacles)?;

    Ok(())
}

fn execute_phase(phase_num: usize, phase: &NavigationPhase) -> Result<NavStats> {
    println!("\n{}", "─".repeat(80));
    println!("📡 PHASE {}: {}", phase_num, phase.name);
    println!("{}", "─".repeat(80));
    println!("🔭 {}", phase.scale_description);
    println!("📏 Scale: {}", phase.scale_units);
    println!("⚙️  Resolution Level: {}", phase.resolution);
    println!();

    sleep(200);

    // Create start and goal cells
    let start = CellID::from_coords(
        0,
        phase.resolution,
        phase.start_coords.0,
        phase.start_coords.1,
        phase.start_coords.2,
    )?;
    let goal = CellID::from_coords(
        0,
        phase.resolution,
        phase.goal_coords.0,
        phase.goal_coords.1,
        phase.goal_coords.2,
    )?;

    println!(
        "📍 Current Position: ({}, {}, {})",
        start.x(),
        start.y(),
        start.z()
    );
    println!(
        "🎯 Target Position:  ({}, {}, {})",
        goal.x(),
        goal.y(),
        goal.z()
    );
    println!();

    // Scan surroundings
    println!("🔍 Initiating sensor scan...");
    sleep(150);
    let scan_radius = 3;
    let scan_cells = k_ring(start, scan_radius);
    println!(
        "   ✓ Scanned {} cells in {}-ring radius",
        scan_cells.len(),
        scan_radius
    );
    sleep(80);

    // Generate obstacle field
    println!("⚠️  Generating obstacle field...");
    sleep(100);
    let mut obstacles = Layer::new("obstacles");
    let mut obstacle_count = 0;

    // Create obstacles along potential path
    for cell in &scan_cells {
        // Use pseudo-random based on coords
        let hash = (cell.x().abs() * 73 + cell.y().abs() * 179 + cell.z().abs() * 283) % 100;
        if (hash as f64) < (phase.obstacle_density * 100.0) {
            let mut flags = CellFlags::empty();
            flags.set_flag(CellFlags::BLOCKED);
            obstacles.set(*cell, flags);
            obstacle_count += 1;
        }
    }

    println!("   ⚠️  Detected {} obstacles in scan range", obstacle_count);
    sleep(80);

    // Calculate navigation path
    println!("🧭 Computing optimal navigation path...");
    println!("   Algorithm: A* with obstacle avoidance");
    sleep(200);

    let cost_function = if obstacle_count > 0 {
        // Use obstacle-avoiding cost function
        let path = astar(
            start,
            goal,
            &AvoidBlockedCost::new(obstacles.clone(), 1000.0),
        )?;
        println!(
            "   ✓ Path calculated: {} cells, cost: {:.2}",
            path.cells.len(),
            path.cost
        );
        path
    } else {
        // Use simple Euclidean cost
        let path = astar(start, goal, &EuclideanCost)?;
        println!(
            "   ✓ Path calculated: {} cells, cost: {:.2}",
            path.cells.len(),
            path.cost
        );
        path
    };

    sleep(100);

    // Visualize navigation
    println!("\n🚀 NAVIGATING...");
    sleep(80);
    visualize_navigation(&cost_function.cells, obstacle_count)?;

    println!("\n✅ Phase {} complete!", phase_num);
    println!("   • Cells explored: {}", scan_cells.len());
    println!("   • Path length: {} cells", cost_function.cells.len());
    println!("   • Distance traveled: {:.2} units", cost_function.cost);

    Ok(NavStats {
        cells_explored: scan_cells.len(),
        obstacles_detected: obstacle_count,
        _path_length: cost_function.cells.len(),
        _total_distance: cost_function.cost,
    })
}

fn visualize_navigation(path: &[CellID], _obstacles: usize) -> Result<()> {
    let steps = path.len().min(20);
    let step_size = if path.len() > 20 { path.len() / 20 } else { 1 };

    for i in 0..steps {
        let idx = i * step_size;
        if idx >= path.len() {
            break;
        }

        let cell = &path[idx];
        let progress = (idx as f64 / path.len() as f64) * 100.0;

        // Progress bar
        let bar_width = 40;
        let filled = ((progress / 100.0) * bar_width as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        print!(
            "\r   [{}] {:5.1}%  Position: ({:>8}, {:>8}, {:>8})",
            bar,
            progress,
            cell.x(),
            cell.y(),
            cell.z()
        );

        std::io::Write::flush(&mut std::io::stdout()).ok();
        sleep(30);
    }

    // Complete the bar
    let bar: String = "█".repeat(40);
    println!("\r   [{}] 100.0%  🎯 DESTINATION REACHED", bar);

    Ok(())
}

fn deploy_probes() -> Result<()> {
    println!("\n{}", "═".repeat(80));
    println!("{:^80}", "🛸 PROBE DEPLOYMENT SEQUENCE");
    println!("{}", "═".repeat(80));

    sleep(1000);

    // Orbital deployment
    println!("\n📡 STAGE 1: Orbital Probe Deployment");
    println!("   Resolution Level: 30 (Near-Earth orbit)");
    sleep(150);

    let orbit_cell = CellID::from_coords(0, 30, 32800, 33600, 24640)?;
    println!(
        "   Mothership orbit position: ({}, {}, {})",
        orbit_cell.x(),
        orbit_cell.y(),
        orbit_cell.z()
    );

    sleep(100);

    // Deploy probes using children cells
    println!("\n   Deploying reconnaissance probes...");
    sleep(80);

    let probe_positions = orbit_cell.children()?;
    for (i, probe) in probe_positions.iter().enumerate() {
        println!(
            "   🛸 Probe {} deployed at ({}, {}, {})",
            i + 1,
            probe.x(),
            probe.y(),
            probe.z()
        );
        sleep(50);
    }

    println!(
        "\n   ✓ {} probes active in orbital formation",
        probe_positions.len()
    );
    sleep(200);

    // Atmospheric entry
    println!("\n📡 STAGE 2: Atmospheric Entry");
    println!("   Resolution Level: 32 (Atmospheric approach)");
    sleep(150);

    // Select one probe for landing
    let landing_probe = &probe_positions[0];
    let entry_cell = CellID::from_coords(
        0,
        32,
        landing_probe.x() * 4,
        landing_probe.y() * 4,
        landing_probe.z() * 4,
    )?;

    println!("   🛸 Probe #1 entering atmosphere...");
    sleep(100);

    // Navigate through atmosphere
    let surface_target = CellID::from_coords(
        0,
        32,
        entry_cell.x() + 100,
        entry_cell.y() + 80,
        entry_cell.z() - 200,
    )?;

    println!("   🧭 Computing descent trajectory...");
    sleep(150);

    let descent_path = astar(entry_cell, surface_target, &EuclideanCost)?;
    println!(
        "   ✓ Trajectory calculated: {} waypoints",
        descent_path.cells.len()
    );

    println!("\n   🔥 DESCENDING...");
    sleep(80);

    for i in 0..10 {
        let progress = (i + 1) * 10;
        let altitude = 100 - (i * 10);
        print!("\r   Altitude: {:>3} km  |  ", altitude);
        for j in 0..10 {
            if j < i {
                print!("▼");
            } else {
                print!("·");
            }
        }
        print!("  {}%", progress);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        sleep(50);
    }
    println!("\r   Altitude:   0 km  |  ▼▼▼▼▼▼▼▼▼▼  100% - TOUCHDOWN!");

    sleep(200);

    // Drone deployment
    println!("\n📡 STAGE 3: Surface Drone Deployment");
    println!("   Resolution Level: 35 (Surface level)");
    sleep(150);

    let surface_cell = CellID::from_coords(
        0,
        35,
        surface_target.x() * 8,
        surface_target.y() * 8,
        surface_target.z() * 8,
    )?;

    println!(
        "   Landing site: ({}, {}, {})",
        surface_cell.x(),
        surface_cell.y(),
        surface_cell.z()
    );
    sleep(100);

    println!("\n   Deploying reconnaissance drones...");
    sleep(80);

    // Deploy drones in a pattern
    let drone_scan = k_ring(surface_cell, 2);
    let drones: Vec<_> = drone_scan.iter().take(8).collect();

    for (i, drone_pos) in drones.iter().enumerate() {
        let _angle = (i as f64 * 45.0).to_radians();
        let direction = match i {
            0 => "North",
            1 => "Northeast",
            2 => "East",
            3 => "Southeast",
            4 => "South",
            5 => "Southwest",
            6 => "West",
            _ => "Northwest",
        };

        println!(
            "   🤖 Drone {} deployed - Direction: {} - Position: ({}, {}, {})",
            i + 1,
            direction,
            drone_pos.x(),
            drone_pos.y(),
            drone_pos.z()
        );
        sleep(50);
    }

    println!("\n   ✓ All drones operational");
    println!("   ✓ Beginning surface reconnaissance...");

    sleep(1000);

    // Drone scanning animation
    println!("\n   🔍 Scanning terrain...");
    sleep(80);

    for cycle in 0..3 {
        for i in 0..8 {
            let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            print!(
                "\r   {} Scan cycle {}/3 - Analyzing sector {}/8",
                chars[i % chars.len()],
                cycle + 1,
                i + 1
            );
            std::io::Write::flush(&mut std::io::stdout()).ok();
            sleep(40);
        }
    }
    println!("\r   ✓ Scan complete - Data transmitted to mothership");

    Ok(())
}

fn print_final_summary(total_cells: usize, total_obstacles: usize) -> Result<()> {
    sleep(300);

    println!("\n{}", "═".repeat(80));
    println!("{:^80}", "🌟 MISSION COMPLETE 🌟");
    println!("{}", "═".repeat(80));

    sleep(100);

    println!("\n📊 NAVIGATION STATISTICS:");
    println!("   • Total cells explored: {}", total_cells);
    println!("   • Total obstacles avoided: {}", total_obstacles);
    println!("   • Navigation phases completed: 4");
    println!("   • Probes deployed: 4 (orbital reconnaissance)");
    println!("   • Drones deployed: 8 (surface reconnaissance)");
    println!("   • Distance traversed: ~25 million light-years → 0 km");

    println!("\n🎯 OCTAINDEX3D TECHNOLOGY DEMONSTRATION:");
    println!("   ✓ Multi-scale navigation (resolution 3 → 35)");
    println!("   ✓ Hierarchical spatial indexing (8:1 refinement)");
    println!("   ✓ Real-time obstacle detection and avoidance");
    println!("   ✓ A* pathfinding across 10^9 scale range");
    println!("   ✓ K-ring sensor scanning");
    println!("   ✓ BCC lattice 14-neighbor connectivity");

    println!("\n🚀 MISSION STATUS: SUCCESS");
    println!("   Earth reconnaissance complete. Data transmission in progress...");

    println!("\n{}", "═".repeat(80));
    println!(
        "{:^80}",
        "OctaIndex3D - Navigating the Universe, One Cell at a Time"
    );
    println!("{}", "═".repeat(80));
    println!();

    Ok(())
}

// Helper functions
fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}
