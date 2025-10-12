//! Mothership Dashboard - Continuous Interstellar Exploration
//!
//! An immersive Star Trek-style command center interface showing real-time
//! exploration across the universe using OctaIndex3D technology.

use octaindex3d::layer::{CellFlags, Layer};
use octaindex3d::path::{astar, k_ring, AvoidBlockedCost, EuclideanCost};
use octaindex3d::{CellID, Result};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

/// Ship systems and exploration stats
#[derive(Debug, Clone)]
struct MissionStats {
    start_time: Instant,
    galaxies_scanned: u32,
    star_systems_explored: u32,
    planets_discovered: u32,
    anomalies_detected: u32,
    obstacles_avoided: u32,
    total_distance: f64,
    probes_deployed: u32,
    drones_active: u32,
    exploration_count: u32,
}

impl MissionStats {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            galaxies_scanned: 0,
            star_systems_explored: 0,
            planets_discovered: 0,
            anomalies_detected: 0,
            obstacles_avoided: 0,
            total_distance: 0.0,
            probes_deployed: 0,
            drones_active: 0,
            exploration_count: 0,
        }
    }

    fn elapsed(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs();
        let hours = elapsed / 3600;
        let minutes = (elapsed % 3600) / 60;
        let seconds = elapsed % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

/// Current exploration target
#[derive(Debug, Clone)]
struct ExplorationTarget {
    name: String,
    target_type: String,
    distance: f64,
    resolution: u8,
    coordinates: (i32, i32, i32),
}

/// Discovery types
#[derive(Debug, Clone)]
enum Discovery {
    Galaxy {
        name: String,
        size: String,
    },
    StarSystem {
        name: String,
        stars: u32,
    },
    Planet {
        name: String,
        planet_type: String,
        habitable: bool,
    },
    Anomaly {
        name: String,
        description: String,
    },
}

fn main() -> Result<()> {
    let mut stats = MissionStats::new();

    // Initialize display
    clear_screen();
    show_boot_sequence()?;

    // Main exploration loop - run continuously
    // Press Ctrl+C to exit
    loop {
        clear_screen();

        // Generate new exploration target
        let target = generate_exploration_target(&stats);

        // Draw dashboard
        draw_dashboard(&stats, &target)?;

        // Execute exploration
        explore_target(&mut stats, &target)?;

        stats.exploration_count += 1;

        // Limit explorations for demo - remove this to run continuously
        if stats.exploration_count >= 10 {
            show_shutdown_sequence(&stats)?;
            break;
        }
    }

    Ok(())
}

fn show_boot_sequence() -> Result<()> {
    let boot_messages = vec![
        "╔═══════════════════════════════════════════════════════════════════════════╗",
        "║                                                                           ║",
        "║              🛸  MOTHERSHIP COMMAND & CONTROL SYSTEM  🛸                 ║",
        "║                                                                           ║",
        "║                        OctaIndex3D Navigation Core                        ║",
        "║                              Version 0.2.0                                ║",
        "║                                                                           ║",
        "╚═══════════════════════════════════════════════════════════════════════════╝",
    ];

    for line in boot_messages {
        println!("{}", line);
        sleep(100);
    }

    println!();
    sleep(300);

    let systems = vec![
        (
            "PRIMARY SYSTEMS",
            vec![
                "⚡ Quantum Drive",
                "🔋 Power Core",
                "🛡️  Shield Generator",
                "📡 Long Range Sensors",
            ],
        ),
        (
            "NAVIGATION",
            vec![
                "🧭 OctaIndex3D Engine",
                "🗺️  Galactic Mapper",
                "⭐ Star Chart Database",
                "🎯 Targeting Computer",
            ],
        ),
        (
            "EXPLORATION",
            vec![
                "🛸 Probe Launch System",
                "🤖 Drone Control",
                "🔬 Science Lab",
                "💾 Data Banks",
            ],
        ),
    ];

    for (category, items) in systems {
        println!("  ┌─ {} ", category);
        for item in items {
            print!("  │  {} ", item);
            stdout().flush().ok();
            sleep(150);
            println!("........................ ✓ ONLINE");
        }
        println!("  └─");
        sleep(100);
    }

    println!();
    println!("  🚀 ALL SYSTEMS NOMINAL");
    println!("  🌌 BEGINNING EXPLORATION PROTOCOL");
    println!();
    sleep(800);

    println!("  Press Ctrl+C to exit...");
    println!();
    sleep(1500);

    Ok(())
}

fn draw_dashboard(stats: &MissionStats, target: &ExplorationTarget) -> Result<()> {
    let width = 100;

    // Top border
    println!("╔{}╗", "═".repeat(width - 2));
    println!(
        "║{:^98}║",
        "🛸 U.S.S. NAVIGATOR - DEEP SPACE EXPLORATION COMMAND 🛸"
    );
    println!("╠{}╣", "═".repeat(width - 2));

    // Mission clock and status
    println!(
        "║  Mission Time: {}  │  Status: ACTIVE  │  FTL Drive: ENGAGED  {:24}║",
        stats.elapsed(),
        ""
    );
    println!("╠{}╣", "─".repeat(width - 2));

    // Main stats panel
    println!("║                                                                                                  ║");
    println!("║  ┌─ EXPLORATION STATISTICS ────────────────────────┐  ┌─ SHIP SYSTEMS ──────────────────────┐  ║");
    println!("║  │                                                  │  │                                      │  ║");
    println!("║  │  🌌 Galaxies Scanned:        {:>6}            │  │  ⚡ Power:          [████████] 100% │  ║",
        stats.galaxies_scanned);
    println!("║  │  ⭐ Star Systems Explored:   {:>6}            │  │  🛡️  Shields:        [████████] 100% │  ║",
        stats.star_systems_explored);
    println!("║  │  🪐 Planets Discovered:      {:>6}            │  │  🔋 Energy:         [████████] 100% │  ║",
        stats.planets_discovered);
    println!("║  │  ❓ Anomalies Detected:      {:>6}            │  │  📡 Sensors:        [████████] 100% │  ║",
        stats.anomalies_detected);
    println!("║  │  🚧 Obstacles Avoided:       {:>6}            │  │  🧭 Navigation:     [████████] 100% │  ║",
        stats.obstacles_avoided);
    println!("║  │  📏 Distance Traveled:  {:>10.2} LY        │  │  🤖 Drones Active:  {:>4}         │  ║",
        stats.total_distance, stats.drones_active);
    println!("║  │                                                  │  │  🛸 Probes Deployed: {:>4}         │  ║",
        stats.probes_deployed);
    println!("║  │                                                  │  │                                      │  ║");
    println!("║  └──────────────────────────────────────────────────┘  └──────────────────────────────────────┘  ║");

    println!("║                                                                                                  ║");
    println!("╠{}╣", "─".repeat(width - 2));

    // Current target
    println!("║  ┌─ CURRENT TARGET ─────────────────────────────────────────────────────────────────────────────┐  ║");
    println!("║  │                                                                                              │  ║");
    println!(
        "║  │  Target:     {} - {}                                         ",
        target.target_type, target.name
    );
    println!(
        "║  │  Distance:   {:.2} light-years                                                     ",
        target.distance
    );
    println!(
        "║  │  Coords:     ({:>8}, {:>8}, {:>8})                                                ",
        target.coordinates.0, target.coordinates.1, target.coordinates.2
    );
    println!("║  │  Resolution: Level {}                                                                      ",
        target.resolution);
    println!("║  │                                                                                              │  ║");
    println!("║  └──────────────────────────────────────────────────────────────────────────────────────────────┘  ║");

    println!("║                                                                                                  ║");
    println!("╠{}╣", "─".repeat(width - 2));

    // Sensor display header
    println!("║  ┌─ SENSOR ARRAY ───────────────────────────────────────────────────────────────────────────────┐  ║");
    println!("║  │                                                                                              │  ║");

    Ok(())
}

fn draw_dashboard_footer() -> Result<()> {
    let width = 100;

    println!("║  │                                                                                              │  ║");
    println!("║  └──────────────────────────────────────────────────────────────────────────────────────────────┘  ║");
    println!("╠{}╣", "═".repeat(width - 2));
    println!("║  [OCTAINDEX3D v0.2.0] Multi-scale spatial navigation │ BCC Lattice │ 14-neighbor connectivity      ║");
    println!("╚{}╝", "═".repeat(width - 2));
    stdout().flush().ok();

    Ok(())
}

fn explore_target(stats: &mut MissionStats, target: &ExplorationTarget) -> Result<()> {
    // Create cells for navigation
    let start = CellID::from_coords(
        0,
        target.resolution,
        target.coordinates.0 - 50,
        target.coordinates.1 - 50,
        target.coordinates.2 - 50,
    )?;
    let goal = CellID::from_coords(
        0,
        target.resolution,
        target.coordinates.0,
        target.coordinates.1,
        target.coordinates.2,
    )?;

    // Sensor scan
    print_sensor_message("  │  📡 Initiating long-range sensor scan...")?;
    sleep(600);

    let scan_radius = 2;
    let scan_cells = k_ring(start, scan_radius);
    print_sensor_message(&format!(
        "  │  ✓ Scan complete: {} cells detected",
        scan_cells.len()
    ))?;
    sleep(400);

    // Obstacle detection
    print_sensor_message("  │  🔍 Analyzing spatial hazards...")?;
    sleep(500);

    let mut obstacles = Layer::new("obstacles");
    let mut obstacle_count = 0;

    for cell in &scan_cells {
        let hash = (cell.x().abs() * 73 + cell.y().abs() * 179 + cell.z().abs() * 283) % 100;
        if (hash as f64) < 15.0 {
            let mut flags = CellFlags::empty();
            flags.set_flag(CellFlags::BLOCKED);
            obstacles.set(*cell, flags);
            obstacle_count += 1;
        }
    }

    stats.obstacles_avoided += obstacle_count;

    if obstacle_count > 0 {
        print_sensor_message(&format!(
            "  │  ⚠️  {} spatial hazards detected - plotting safe course",
            obstacle_count
        ))?;
    } else {
        print_sensor_message("  │  ✓ Sector clear - direct approach authorized")?;
    }
    sleep(500);

    // Navigation
    print_sensor_message("  │  🧭 Computing optimal navigation path...")?;
    sleep(600);

    let path = if obstacle_count > 0 {
        astar(start, goal, &AvoidBlockedCost::new(obstacles, 1000.0))?
    } else {
        astar(start, goal, &EuclideanCost)?
    };

    print_sensor_message(&format!(
        "  │  ✓ Course plotted: {} waypoints, distance {:.2} LY",
        path.cells.len(),
        path.cost
    ))?;
    stats.total_distance += path.cost;
    sleep(500);

    // Travel animation
    print_sensor_message("  │")?;
    print_sensor_message("  │  🚀 ENGAGING FTL DRIVE...")?;
    sleep(400);

    animate_travel(&path)?;

    print_sensor_message("  │  ✓ Destination reached")?;
    sleep(400);

    // Make discoveries
    let discoveries = make_discoveries(target, stats)?;

    print_sensor_message("  │")?;
    print_sensor_message("  │  🔬 ANALYSIS RESULTS:")?;
    sleep(300);

    for discovery in discoveries {
        match discovery {
            Discovery::Galaxy { name, size } => {
                stats.galaxies_scanned += 1;
                print_sensor_message(&format!(
                    "  │  🌌 Galaxy detected: {} - Size: {}",
                    name, size
                ))?;
            }
            Discovery::StarSystem { name, stars } => {
                stats.star_systems_explored += 1;
                print_sensor_message(&format!(
                    "  │  ⭐ Star system mapped: {} - {} star(s)",
                    name, stars
                ))?;
            }
            Discovery::Planet {
                name,
                planet_type,
                habitable,
            } => {
                stats.planets_discovered += 1;
                let hab = if habitable {
                    "HABITABLE ✓"
                } else {
                    "Non-habitable"
                };
                print_sensor_message(&format!(
                    "  │  🪐 Planet catalogued: {} - {} - {}",
                    name, planet_type, hab
                ))?;

                // Deploy probe to habitable planets
                if habitable {
                    sleep(300);
                    print_sensor_message("  │     🛸 Deploying survey probe...")?;
                    stats.probes_deployed += 1;
                    stats.drones_active += 3;
                    sleep(400);
                    print_sensor_message("  │     ✓ Probe deployed - 3 drones active on surface")?;
                }
            }
            Discovery::Anomaly { name, description } => {
                stats.anomalies_detected += 1;
                print_sensor_message(&format!(
                    "  │  ❓ Anomaly identified: {} - {}",
                    name, description
                ))?;
            }
        }
        sleep(400);
    }

    print_sensor_message("  │")?;
    print_sensor_message("  │  💾 Data transmitted to Federation Science Division")?;
    sleep(600);

    draw_dashboard_footer()?;

    sleep(1500);

    Ok(())
}

fn animate_travel(path: &octaindex3d::path::Path) -> Result<()> {
    let steps = 20;
    let step_size = if path.cells.len() > steps {
        path.cells.len() / steps
    } else {
        1
    };

    for i in 0..steps {
        let idx = i * step_size;
        if idx >= path.cells.len() {
            break;
        }

        let cell = &path.cells[idx];
        let progress = (idx as f64 / path.cells.len() as f64) * 100.0;

        let bar_width = 30;
        let filled = ((progress / 100.0) * bar_width as f64) as usize;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        let warp_chars = ["═", "≡", "━", "─"];
        let warp = warp_chars[i % warp_chars.len()];

        print!(
            "\r  │  {} [{}] {:>5.1}% │ Pos: ({:>8}, {:>8}, {:>8})  ",
            warp,
            bar,
            progress,
            cell.x(),
            cell.y(),
            cell.z()
        );
        stdout().flush().ok();
        sleep(100);
    }

    // Complete
    let bar = "█".repeat(30);
    println!(
        "\r  │  ✓ [{}] 100.0% │ TARGET ACQUIRED                              ",
        bar
    );

    Ok(())
}

fn make_discoveries(target: &ExplorationTarget, _stats: &MissionStats) -> Result<Vec<Discovery>> {
    let mut discoveries = Vec::new();

    // Use coordinates to pseudo-randomly determine what we find
    let seed = (target.coordinates.0.abs()
        + target.coordinates.1.abs()
        + target.coordinates.2.abs()) as usize;

    match target.target_type.as_str() {
        "Galaxy" => {
            discoveries.push(Discovery::Galaxy {
                name: target.name.clone(),
                size: vec!["Dwarf", "Spiral", "Elliptical", "Irregular"][seed % 4].to_string(),
            });
            // Also find some star systems in the galaxy
            discoveries.push(Discovery::StarSystem {
                name: format!("{} Sector {}", target.name, seed % 100),
                stars: 1 + (seed % 3) as u32,
            });
        }
        "Star System" => {
            discoveries.push(Discovery::StarSystem {
                name: target.name.clone(),
                stars: 1 + (seed % 3) as u32,
            });
            // Find planets
            let num_planets = 1 + (seed % 4);
            for i in 0..num_planets {
                let planet_types = ["Rocky", "Gas Giant", "Ice World", "Super-Earth", "Desert"];
                let planet_type = planet_types[(seed + i) % planet_types.len()];
                let habitable = planet_type == "Rocky" || planet_type == "Super-Earth";

                discoveries.push(Discovery::Planet {
                    name: format!("{}-{}", target.name, (b'A' + i as u8) as char),
                    planet_type: planet_type.to_string(),
                    habitable: habitable && (seed + i) % 5 == 0,
                });
            }
        }
        "Planet" => {
            let planet_types = ["Rocky", "Gas Giant", "Ice World", "Super-Earth"];
            let planet_type = planet_types[seed % planet_types.len()];
            let habitable =
                (planet_type == "Rocky" || planet_type == "Super-Earth") && seed % 3 == 0;

            discoveries.push(Discovery::Planet {
                name: target.name.clone(),
                planet_type: planet_type.to_string(),
                habitable,
            });
        }
        _ => {}
    }

    // Chance of anomaly
    if seed % 7 == 0 {
        let anomalies = [
            (
                "Subspace Distortion",
                "Unusual quantum fluctuations detected",
            ),
            ("Nebula Formation", "Dense stellar nursery identified"),
            (
                "Dark Matter Concentration",
                "Gravitational anomaly confirmed",
            ),
            ("Ancient Signal", "Non-natural EM signature detected"),
            (
                "Wormhole Signature",
                "Spacetime curvature exceeds normal parameters",
            ),
        ];
        let anomaly = &anomalies[seed % anomalies.len()];
        discoveries.push(Discovery::Anomaly {
            name: anomaly.0.to_string(),
            description: anomaly.1.to_string(),
        });
    }

    Ok(discoveries)
}

fn generate_exploration_target(stats: &MissionStats) -> ExplorationTarget {
    let time_seed = stats.start_time.elapsed().as_secs() as usize;

    let galaxy_names = [
        "Andromeda",
        "Triangulum",
        "Whirlpool",
        "Sombrero",
        "Pinwheel",
        "Cartwheel",
        "Tadpole",
        "Sunflower",
        "Messier 87",
        "Centaurus A",
    ];

    let star_names = [
        "Alpha Centauri",
        "Proxima",
        "Sirius",
        "Vega",
        "Arcturus",
        "Betelgeuse",
        "Rigel",
        "Procyon",
        "Altair",
        "Deneb",
    ];

    let planet_prefixes = [
        "Kepler",
        "TRAPPIST",
        "Gliese",
        "Ross",
        "Wolf",
        "Luyten",
        "Lacaille",
        "Tau Ceti",
        "Epsilon Eridani",
        "HD",
    ];

    // Cycle through different target types
    let cycle = (time_seed / 3) % 15;

    if cycle < 3 {
        // Galaxy
        let name = galaxy_names[time_seed % galaxy_names.len()].to_string();
        ExplorationTarget {
            name: name.clone(),
            target_type: "Galaxy".to_string(),
            distance: 1_000_000.0 + (time_seed as f64 * 100_000.0),
            resolution: 5,
            coordinates: (
                (time_seed as i32 * 137) % 1000 - 500,
                (time_seed as i32 * 241) % 1000 - 500,
                (time_seed as i32 * 163) % 1000 - 500,
            ),
        }
    } else if cycle < 10 {
        // Star System
        let name = format!(
            "{}-{}",
            star_names[time_seed % star_names.len()],
            time_seed % 100
        );
        ExplorationTarget {
            name,
            target_type: "Star System".to_string(),
            distance: 10.0 + (time_seed as f64 * 5.0) % 1000.0,
            resolution: 15,
            coordinates: (
                (time_seed as i32 * 157) % 10000 - 5000,
                (time_seed as i32 * 271) % 10000 - 5000,
                (time_seed as i32 * 193) % 10000 - 5000,
            ),
        }
    } else {
        // Planet
        let name = format!(
            "{}-{}",
            planet_prefixes[time_seed % planet_prefixes.len()],
            100 + time_seed % 900
        );
        ExplorationTarget {
            name,
            target_type: "Planet".to_string(),
            distance: 1.0 + (time_seed as f64 * 0.5) % 50.0,
            resolution: 25,
            coordinates: (
                (time_seed as i32 * 173) % 100000 - 50000,
                (time_seed as i32 * 281) % 100000 - 50000,
                (time_seed as i32 * 211) % 100000 - 50000,
            ),
        }
    }
}

fn print_sensor_message(msg: &str) -> Result<()> {
    println!("{:<98}║", msg);
    stdout().flush().ok();
    Ok(())
}

fn show_shutdown_sequence(stats: &MissionStats) -> Result<()> {
    clear_screen();

    println!("\n\n");
    println!("  ╔═══════════════════════════════════════════════════════════════════════╗");
    println!("  ║                                                                       ║");
    println!("  ║                    🛸  MISSION DEACTIVATION  🛸                       ║");
    println!("  ║                                                                       ║");
    println!("  ╚═══════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Mission Duration: {}", stats.elapsed());
    println!();
    println!("  Final Mission Statistics:");
    println!("  ─────────────────────────");
    println!("    🌌 Galaxies Scanned:        {}", stats.galaxies_scanned);
    println!(
        "    ⭐ Star Systems Explored:   {}",
        stats.star_systems_explored
    );
    println!(
        "    🪐 Planets Discovered:      {}",
        stats.planets_discovered
    );
    println!(
        "    ❓ Anomalies Detected:      {}",
        stats.anomalies_detected
    );
    println!(
        "    🚧 Obstacles Avoided:       {}",
        stats.obstacles_avoided
    );
    println!(
        "    📏 Distance Traveled:       {:.2} LY",
        stats.total_distance
    );
    println!("    🛸 Probes Deployed:         {}", stats.probes_deployed);
    println!();
    println!("  Safe travels, Captain. 🖖");
    println!();

    Ok(())
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    stdout().flush().ok();
}

fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

fn stdout() -> io::Stdout {
    io::stdout()
}
