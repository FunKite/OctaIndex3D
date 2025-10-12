//! Mothership Bridge - Advanced 3D Navigation Visualization
//!
//! Full-screen Star Trek-style bridge interface with real-time 3D obstacle
//! avoidance visualization, continuous exploration, and detailed telemetry.

use octaindex3d::layer::{CellFlags, Layer};
use octaindex3d::path::{astar, k_ring, AvoidBlockedCost, EuclideanCost};
use octaindex3d::{CellID, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Read, Write};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const WIDTH: usize = 200;
const HEIGHT: usize = 55;

/// Ship systems and exploration stats
#[derive(Debug, Clone)]
struct MissionStats {
    start_time: Instant,
    galaxies_scanned: u32,
    star_systems_explored: u32,
    planets_discovered: u32,
    habitable_planets: u32,
    anomalies_detected: u32,
    obstacles_avoided: u32,
    total_distance: f64,
    probes_deployed: u32,
    drones_active: u32,
    jumps_completed: u32,
}

impl MissionStats {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            galaxies_scanned: 0,
            star_systems_explored: 0,
            planets_discovered: 0,
            habitable_planets: 0,
            anomalies_detected: 0,
            obstacles_avoided: 0,
            total_distance: 0.0,
            probes_deployed: 0,
            drones_active: 0,
            jumps_completed: 0,
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

/// Exploration target
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

/// Recent log entries
#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    message: String,
    log_type: LogType,
}

#[derive(Debug, Clone, PartialEq)]
enum LogType {
    Info,
    Success,
    Warning,
    Discovery,
}

fn main() -> Result<()> {
    let mut stats = MissionStats::new();
    let mut log_buffer: VecDeque<LogEntry> = VecDeque::new();

    // Start keyboard listener
    let rx = spawn_key_listener();

    clear_screen();
    show_boot_sequence()?;

    // Main exploration loop - runs until 'q' is pressed
    loop {
        // Check for quit command
        match rx.try_recv() {
            Ok(key) => {
                if key == 'q' || key == 'Q' {
                    show_shutdown_sequence(&stats)?;
                    break;
                }
            }
            Err(TryRecvError::Disconnected) => {
                // Channel disconnected, exit gracefully
                show_shutdown_sequence(&stats)?;
                break;
            }
            Err(TryRecvError::Empty) => {}
        }

        // Generate new exploration target
        let target = generate_exploration_target(&stats);

        // Execute exploration with full visualization
        explore_with_visualization(&mut stats, &target, &mut log_buffer)?;

        stats.jumps_completed += 1;
    }

    Ok(())
}

fn show_boot_sequence() -> Result<()> {
    let title = vec![
        "╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗",
        "║                                                                                                                                                                               ║",
        "║                                                        🛸  U.S.S. NAVIGATOR - BRIDGE SYSTEMS  🛸                                                                              ║",
        "║                                                                                                                                                                               ║",
        "║                                                              OctaIndex3D Navigation Core v0.2.0                                                                              ║",
        "║                                                           Multi-Scale 3D Spatial Navigation System                                                                           ║",
        "║                                                                                                                                                                               ║",
        "╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝",
    ];

    for line in title {
        println!("{}", line);
        sleep(80);
    }
    println!();

    let systems = vec![
        (
            "PRIMARY SYSTEMS",
            vec![
                ("⚡ Quantum Drive", "ONLINE"),
                ("🔋 Fusion Reactor", "ONLINE"),
                ("🛡️  Shield Array", "ONLINE"),
                ("📡 Sensor Grid", "ONLINE"),
            ],
        ),
        (
            "NAVIGATION & GUIDANCE",
            vec![
                ("🧭 OctaIndex3D Engine", "ONLINE"),
                ("🗺️  Galactic Database", "ONLINE"),
                ("⭐ Star Chart v4.7", "ONLINE"),
                ("🎯 Targeting Systems", "ONLINE"),
            ],
        ),
        (
            "EXPLORATION SUITE",
            vec![
                ("🛸 Probe Bay", "ONLINE"),
                ("🤖 Drone Control", "ONLINE"),
                ("🔬 Science Lab", "ONLINE"),
                ("💾 Data Core", "ONLINE"),
            ],
        ),
        (
            "3D VISUALIZATION",
            vec![
                ("🎮 Holographic Display", "ONLINE"),
                ("📊 Telemetry Systems", "ONLINE"),
                ("🎨 Obstacle Renderer", "ONLINE"),
                ("📈 Path Plotter", "ONLINE"),
            ],
        ),
    ];

    for (category, items) in systems {
        println!("  ┌─ {} ", category);
        for (item, status) in items {
            print!("  │  {:30}", item);
            stdout().flush().ok();
            sleep(120);
            print!(" .");
            stdout().flush().ok();
            sleep(120);
            print!(".");
            stdout().flush().ok();
            sleep(120);
            println!(".................. ✓ {}", status);
        }
        println!("  └─");
        sleep(80);
    }

    println!();
    println!("  🚀 ALL SYSTEMS NOMINAL");
    println!("  🌌 BEGINNING DEEP SPACE EXPLORATION PROTOCOL");
    println!();
    sleep(500);
    println!("  ╔═══════════════════════════════════════════════════════════════════╗");
    println!("  ║  Press 'q' and Enter to exit gracefully                           ║");
    println!("  ║  Full 3D obstacle visualization active                            ║");
    println!("  ║  Real-time pathfinding display enabled                            ║");
    println!("  ╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    sleep(2000);

    Ok(())
}

fn explore_with_visualization(
    stats: &mut MissionStats,
    target: &ExplorationTarget,
    log_buffer: &mut VecDeque<LogEntry>,
) -> Result<()> {
    clear_screen();
    draw_main_bridge(stats, target, log_buffer)?;

    add_log(
        log_buffer,
        LogType::Info,
        &format!(
            "New target acquired: {} - {}",
            target.target_type, target.name
        ),
    );

    // Create navigation cells
    let start = CellID::from_coords(
        0,
        target.resolution,
        target.coordinates.0 - 60,
        target.coordinates.1 - 60,
        target.coordinates.2 - 60,
    )?;
    let goal = CellID::from_coords(
        0,
        target.resolution,
        target.coordinates.0,
        target.coordinates.1,
        target.coordinates.2,
    )?;

    add_log(
        log_buffer,
        LogType::Info,
        "Initiating long-range sensor sweep...",
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(800);

    // Scan for obstacles
    let scan_radius = 3;
    let scan_cells = k_ring(start, scan_radius);
    add_log(
        log_buffer,
        LogType::Success,
        &format!(
            "Sensor scan complete: {} spatial cells analyzed",
            scan_cells.len()
        ),
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(600);

    // Generate obstacles
    let mut obstacles = Layer::new("obstacles");
    let mut obstacle_positions = HashSet::new();
    let mut obstacle_count = 0;

    for cell in &scan_cells {
        let hash = (cell.x().abs() * 73 + cell.y().abs() * 179 + cell.z().abs() * 283) % 100;
        if (hash as f64) < 18.0 {
            let mut flags = CellFlags::empty();
            flags.set_flag(CellFlags::BLOCKED);
            obstacles.set(*cell, flags);
            obstacle_positions.insert((cell.x(), cell.y(), cell.z()));
            obstacle_count += 1;
        }
    }

    stats.obstacles_avoided += obstacle_count;

    if obstacle_count > 0 {
        add_log(
            log_buffer,
            LogType::Warning,
            &format!(
                "⚠️  {} spatial hazards detected in flight path",
                obstacle_count
            ),
        );
    } else {
        add_log(
            log_buffer,
            LogType::Success,
            "✓ Flight corridor clear of obstacles",
        );
    }
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(700);

    // Calculate path
    add_log(
        log_buffer,
        LogType::Info,
        "Computing optimal navigation path with A* algorithm...",
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(800);

    let path = if obstacle_count > 0 {
        astar(start, goal, &AvoidBlockedCost::new(obstacles, 1000.0))?
    } else {
        astar(start, goal, &EuclideanCost)?
    };

    add_log(
        log_buffer,
        LogType::Success,
        &format!(
            "✓ Navigation solution found: {} waypoints, {:.2} LY",
            path.cells.len(),
            path.cost
        ),
    );
    stats.total_distance += path.cost;
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(800);

    // Show 3D visualization and navigate
    add_log(
        log_buffer,
        LogType::Info,
        "🎮 Activating 3D holographic display...",
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(600);

    visualize_and_navigate_3d(stats, target, &path, &obstacle_positions, log_buffer)?;

    // Make discoveries
    add_log(
        log_buffer,
        LogType::Info,
        "🔬 Performing detailed spectral analysis...",
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(1000);

    let discoveries = make_discoveries(target, stats)?;

    for discovery in discoveries {
        match discovery {
            Discovery::Galaxy { name, size } => {
                stats.galaxies_scanned += 1;
                add_log(
                    log_buffer,
                    LogType::Discovery,
                    &format!("🌌 GALAXY: {} - Type: {} - Added to database", name, size),
                );
                draw_main_bridge(stats, target, log_buffer)?;
                sleep(1200);
            }
            Discovery::StarSystem { name, stars } => {
                stats.star_systems_explored += 1;
                add_log(
                    log_buffer,
                    LogType::Discovery,
                    &format!(
                        "⭐ STAR SYSTEM: {} - {} stellar object(s) - Catalogued",
                        name, stars
                    ),
                );
                draw_main_bridge(stats, target, log_buffer)?;
                sleep(1200);
            }
            Discovery::Planet {
                name,
                planet_type,
                habitable,
            } => {
                stats.planets_discovered += 1;
                if habitable {
                    stats.habitable_planets += 1;
                    add_log(
                        log_buffer,
                        LogType::Discovery,
                        &format!(
                            "🪐 PLANET: {} - {} - ⭐ HABITABLE ZONE CONFIRMED ⭐",
                            name, planet_type
                        ),
                    );
                    draw_main_bridge(stats, target, log_buffer)?;
                    sleep(1500);

                    add_log(
                        log_buffer,
                        LogType::Info,
                        "  🛸 Deploying orbital survey probe...",
                    );
                    stats.probes_deployed += 1;
                    stats.drones_active += 4;
                    draw_main_bridge(stats, target, log_buffer)?;
                    sleep(1000);

                    add_log(
                        log_buffer,
                        LogType::Success,
                        "  ✓ Probe deployed - 4 atmospheric drones active",
                    );
                    draw_main_bridge(stats, target, log_buffer)?;
                    sleep(1200);
                } else {
                    add_log(
                        log_buffer,
                        LogType::Discovery,
                        &format!("🪐 PLANET: {} - {} - Non-habitable", name, planet_type),
                    );
                    draw_main_bridge(stats, target, log_buffer)?;
                    sleep(1200);
                }
            }
            Discovery::Anomaly { name, description } => {
                stats.anomalies_detected += 1;
                add_log(
                    log_buffer,
                    LogType::Warning,
                    &format!("❓ ANOMALY: {} - {}", name, description),
                );
                draw_main_bridge(stats, target, log_buffer)?;
                sleep(1500);
            }
        }
    }

    add_log(
        log_buffer,
        LogType::Success,
        "💾 All data transmitted to Starfleet Science Division",
    );
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(1500);

    add_log(log_buffer, LogType::Info, "Preparing for next jump...");
    draw_main_bridge(stats, target, log_buffer)?;
    sleep(1000);

    Ok(())
}

fn visualize_and_navigate_3d(
    stats: &MissionStats,
    target: &ExplorationTarget,
    path: &octaindex3d::path::Path,
    obstacles: &HashSet<(i32, i32, i32)>,
    log_buffer: &mut VecDeque<LogEntry>,
) -> Result<()> {
    add_log(
        log_buffer,
        LogType::Info,
        "🚀 ENGAGING FTL DRIVE - Navigating through obstacle field...",
    );

    let steps = 25;
    let step_size = if path.cells.len() > steps {
        path.cells.len() / steps
    } else {
        1
    };

    for step in 0..steps {
        let idx = step * step_size;
        if idx >= path.cells.len() {
            break;
        }

        let current_cell = &path.cells[idx];
        let progress = (idx as f64 / path.cells.len() as f64) * 100.0;

        // Redraw entire display
        clear_screen();
        draw_bridge_header(stats, target)?;

        // Draw 3D visualization
        draw_3d_space(current_cell, path, obstacles, idx)?;

        // Draw progress and telemetry
        draw_navigation_telemetry(current_cell, progress, path.cost, idx, path.cells.len())?;

        // Draw log
        draw_log_section(log_buffer)?;

        draw_footer()?;

        sleep(250);
    }

    add_log(
        log_buffer,
        LogType::Success,
        "✓ Destination reached - All systems nominal",
    );

    Ok(())
}

fn draw_3d_space(
    current: &CellID,
    path: &octaindex3d::path::Path,
    obstacles: &HashSet<(i32, i32, i32)>,
    current_idx: usize,
) -> Result<()> {
    let _width = 90;
    let _height = 20;

    println!("║  ┌─ 3D HOLOGRAPHIC DISPLAY ─────────────────────────────────────────────────────────────────┐");
    println!("║  │                                                                                          │");

    // Get current position
    let cx = current.x();
    let cy = current.y();
    let cz = current.z();

    // Build path lookup for quick access
    let mut path_positions = HashMap::new();
    for (i, cell) in path.cells.iter().enumerate() {
        path_positions.insert((cell.x(), cell.y(), cell.z()), i);
    }

    // Draw multiple z-layers to show depth
    let z_layers = 3;
    for layer in 0..z_layers {
        let z_offset = layer as i32 - 1; // -1, 0, 1
        let z = cz + z_offset * 3;

        let layer_label = match z_offset {
            -1 => "FAR  ",
            0 => "MID  ",
            _ => "NEAR ",
        };

        print!("║  │  {} Layer [Z={}]: ", layer_label, z);

        // Draw a horizontal slice
        let range = 15;
        for dy in -5..=5 {
            for dx in -range..=range {
                let x = cx + dx;
                let y = cy + dy;

                let pos = (x, y, z);

                if x == cx && y == cy && z == cz {
                    // Current ship position
                    print!("🛸");
                } else if let Some(path_idx) = path_positions.get(&pos) {
                    if *path_idx > current_idx {
                        // Future path
                        print!("·");
                    } else if *path_idx == current_idx - 1 {
                        // Just passed
                        print!("•");
                    } else {
                        // Older path
                        print!(" ");
                    }
                } else if obstacles.contains(&pos) {
                    // Obstacle
                    print!("█");
                } else {
                    // Empty space
                    print!(" ");
                }
            }
            if dy < 5 {
                print!("║  │                 ");
            }
        }
        println!("     │");
    }

    println!("║  │                                                                                          │");
    println!("║  │  Legend: 🛸=Ship  █=Obstacle  ·=Planned Route  •=Recent Path                             │");
    println!("║  └──────────────────────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}

fn draw_navigation_telemetry(
    current: &CellID,
    progress: f64,
    total_distance: f64,
    current_step: usize,
    total_steps: usize,
) -> Result<()> {
    println!("║  ┌─ NAVIGATION TELEMETRY ────────────────────────────────────────────────────────────────────┐");
    println!("║  │                                                                                          │");

    // Progress bar
    let bar_width = 60;
    let filled = ((progress / 100.0) * bar_width as f64) as usize;
    let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);

    println!(
        "║  │  Progress: [{}] {:>6.2}%                                       │",
        bar, progress
    );
    println!("║  │                                                                                          │");
    println!(
        "║  │  Current Position:  X={:>10}  Y={:>10}  Z={:>10}                            │",
        current.x(),
        current.y(),
        current.z()
    );
    println!(
        "║  │  Waypoint:          {} of {}                                                     │",
        current_step, total_steps
    );
    println!("║  │  Total Distance:    {:.2} light-years                                                │",
        total_distance);
    println!("║  │  Velocity:          {} × Speed of Light (Warp 9.2)                                  │", "147");
    println!("║  │                                                                                          │");
    println!("║  └──────────────────────────────────────────────────────────────────────────────────────────┘");

    Ok(())
}

fn draw_main_bridge(
    stats: &MissionStats,
    target: &ExplorationTarget,
    log_buffer: &VecDeque<LogEntry>,
) -> Result<()> {
    draw_bridge_header(stats, target)?;
    draw_statistics_panel(stats)?;
    draw_target_panel(target)?;
    draw_log_section(log_buffer)?;
    draw_footer()?;
    Ok(())
}

fn draw_bridge_header(stats: &MissionStats, target: &ExplorationTarget) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║{:^190}║",
        "🛸 U.S.S. NAVIGATOR - BRIDGE COMMAND CENTER 🛸"
    );
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║  Mission Time: {:>10}  │  Status: ACTIVE  │  FTL Drive: ENGAGED  │  Jumps: {:>5}  │  Target: {:<40}{}║",
        stats.elapsed(), stats.jumps_completed, target.name, " ".repeat(48));
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    Ok(())
}

fn draw_statistics_panel(stats: &MissionStats) -> Result<()> {
    println!("║  ┌─ EXPLORATION STATISTICS ──────────────────────────────┐  ┌─ SHIP SYSTEMS ────────────────────────────────┐  ┌─ DEPLOYMENT STATUS ──────────────────────────┐  ║");
    println!("║  │                                                        │  │                                               │  │                                              │  ║");
    println!("║  │  🌌 Galaxies Scanned:              {:>8}          │  │  ⚡ Primary Power:      [████████████] 100%  │  │  🛸 Probes Deployed:        {:>8}        │  ║",
        stats.galaxies_scanned, stats.probes_deployed);
    println!("║  │  ⭐ Star Systems Explored:         {:>8}          │  │  🛡️  Shield Strength:    [████████████] 100%  │  │  🤖 Active Drones:          {:>8}        │  ║",
        stats.star_systems_explored, stats.drones_active);
    println!("║  │  🪐 Planets Discovered:            {:>8}          │  │  🔋 Energy Reserves:    [████████████] 100%  │  │  🔬 Science Missions:       {:>8}        │  ║",
        stats.planets_discovered, stats.habitable_planets);
    println!("║  │     ├─ Habitable Worlds:          {:>8}          │  │  📡 Sensor Array:       [████████████] 100%  │  │  📊 Data Packets Sent:      {:>8}        │  ║",
        stats.habitable_planets, stats.jumps_completed * 3);
    println!("║  │  ❓ Anomalies Detected:            {:>8}          │  │  🧭 Navigation:         [████████████] 100%  │  │                                              │  ║",
        stats.anomalies_detected);
    println!("║  │  🚧 Obstacles Avoided:             {:>8}          │  │  🎮 Holographic Sys:    [████████████] 100%  │  │                                              │  ║",
        stats.obstacles_avoided);
    println!("║  │  📏 Total Distance:          {:>12.2} LY      │  │                                               │  │                                              │  ║",
        stats.total_distance);
    println!("║  │                                                        │  │                                               │  │                                              │  ║");
    println!("║  └────────────────────────────────────────────────────────┘  └───────────────────────────────────────────────┘  └──────────────────────────────────────────────┘  ║");
    println!("║                                                                                                                                                                      ║");
    Ok(())
}

fn draw_target_panel(target: &ExplorationTarget) -> Result<()> {
    println!("║  ┌─ CURRENT TARGET ANALYSIS ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐  ║");
    println!("║  │                                                                                                                                                                    │  ║");
    println!("║  │  Target Classification: {:<20}  │  Distance: {:<15.2} light-years  │  Coordinates: ({:>10}, {:>10}, {:>10})                                  │  ║",
        target.target_type, target.distance, target.coordinates.0, target.coordinates.1, target.coordinates.2);
    println!("║  │  Designation:          {:<20}  │  Resolution Level: {:>2}                │  Scan Radius: 3-ring BCC lattice (14-neighbor connectivity)            │  ║",
        target.name, target.resolution);
    println!("║  │                                                                                                                                                                    │  ║");
    println!("║  └────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘  ║");
    println!("║                                                                                                                                                                      ║");
    Ok(())
}

fn draw_log_section(log_buffer: &VecDeque<LogEntry>) -> Result<()> {
    println!("║  ┌─ MISSION LOG ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐  ║");
    println!("║  │                                                                                                                                                                    │  ║");

    // Show last 8 log entries
    let entries_to_show = 8;
    let start = if log_buffer.len() > entries_to_show {
        log_buffer.len() - entries_to_show
    } else {
        0
    };

    for i in start..log_buffer.len() {
        if let Some(entry) = log_buffer.get(i) {
            let icon = match entry.log_type {
                LogType::Info => "ℹ️ ",
                LogType::Success => "✓",
                LogType::Warning => "⚠️ ",
                LogType::Discovery => "★",
            };
            println!(
                "║  │  {} [{}] {}                                                    ",
                icon,
                entry.timestamp,
                truncate(&entry.message, 170)
            );
        }
    }

    // Fill remaining lines
    for _ in log_buffer.len()..entries_to_show {
        println!("║  │                                                                                                                                                                    │  ║");
    }

    println!("║  │                                                                                                                                                                    │  ║");
    println!("║  └────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘  ║");

    Ok(())
}

fn draw_footer() -> Result<()> {
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║  [OctaIndex3D v0.2.0] BCC Lattice Navigation │ A* Pathfinding │ Multi-Resolution Indexing │ Press 'q' + Enter to exit                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝");
    stdout().flush().ok();
    Ok(())
}

fn add_log(buffer: &mut VecDeque<LogEntry>, log_type: LogType, message: &str) {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let timestamp = format!(
        "{:02}:{:02}:{:02}",
        (elapsed / 3600) % 24,
        (elapsed / 60) % 60,
        elapsed % 60
    );

    buffer.push_back(LogEntry {
        timestamp,
        message: message.to_string(),
        log_type,
    });

    // Keep only last 20 entries
    while buffer.len() > 20 {
        buffer.pop_front();
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn make_discoveries(target: &ExplorationTarget, _stats: &MissionStats) -> Result<Vec<Discovery>> {
    let mut discoveries = Vec::new();
    let seed = (target.coordinates.0.abs()
        + target.coordinates.1.abs()
        + target.coordinates.2.abs()) as usize;

    match target.target_type.as_str() {
        "Galaxy" => {
            discoveries.push(Discovery::Galaxy {
                name: target.name.clone(),
                size: vec![
                    "Dwarf Irregular",
                    "Barred Spiral",
                    "Elliptical",
                    "Lenticular",
                ][seed % 4]
                    .to_string(),
            });
            discoveries.push(Discovery::StarSystem {
                name: format!("{} Alpha Sector", target.name),
                stars: 1 + (seed % 3) as u32,
            });
        }
        "Star System" => {
            discoveries.push(Discovery::StarSystem {
                name: target.name.clone(),
                stars: 1 + (seed % 3) as u32,
            });
            let num_planets = 1 + (seed % 5);
            for i in 0..num_planets {
                let planet_types = [
                    "Rocky Terrestrial",
                    "Gas Giant",
                    "Ice World",
                    "Super-Earth",
                    "Desert World",
                    "Ocean World",
                ];
                let planet_type = planet_types[(seed + i) % planet_types.len()];
                let habitable = (planet_type == "Rocky Terrestrial"
                    || planet_type == "Super-Earth"
                    || planet_type == "Ocean World")
                    && (seed + i) % 4 == 0;

                discoveries.push(Discovery::Planet {
                    name: format!("{}-{}", target.name, (b'A' + i as u8) as char),
                    planet_type: planet_type.to_string(),
                    habitable,
                });
            }
        }
        "Planet" => {
            let planet_types = [
                "Rocky Terrestrial",
                "Gas Giant",
                "Ice World",
                "Super-Earth",
                "Ocean World",
            ];
            let planet_type = planet_types[seed % planet_types.len()];
            let habitable = (planet_type == "Rocky Terrestrial"
                || planet_type == "Super-Earth"
                || planet_type == "Ocean World")
                && seed % 3 == 0;

            discoveries.push(Discovery::Planet {
                name: target.name.clone(),
                planet_type: planet_type.to_string(),
                habitable,
            });
        }
        _ => {}
    }

    if seed % 6 == 0 {
        let anomalies = [
            (
                "Subspace Distortion",
                "Quantum fluctuations detected - possible wormhole",
            ),
            (
                "Nebula Formation",
                "Active stellar nursery - high radiation",
            ),
            (
                "Dark Matter Concentration",
                "Gravitational lensing confirmed",
            ),
            (
                "Ancient Alien Signal",
                "Repeating pattern - non-natural origin",
            ),
            (
                "Temporal Anomaly",
                "Spacetime curvature exceeds normal parameters",
            ),
            ("Ion Storm", "High-energy plasma field - navigation hazard"),
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
    let time_seed = (stats.start_time.elapsed().as_secs() + stats.jumps_completed as u64) as usize;

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
        "Sculptor",
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
        "Antares",
        "Aldebaran",
        "Spica",
        "Pollux",
        "Fomalhaut",
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

    let cycle = (time_seed / 2) % 20;

    if cycle < 3 {
        let name = galaxy_names[time_seed % galaxy_names.len()].to_string();
        ExplorationTarget {
            name: name.clone(),
            target_type: "Galaxy".to_string(),
            distance: 500_000.0 + (time_seed as f64 * 50_000.0),
            resolution: 5,
            coordinates: (
                (time_seed as i32 * 137) % 1000 - 500,
                (time_seed as i32 * 241) % 1000 - 500,
                (time_seed as i32 * 163) % 1000 - 500,
            ),
        }
    } else if cycle < 14 {
        let name = format!(
            "{}-{}",
            star_names[time_seed % star_names.len()],
            100 + time_seed % 900
        );
        ExplorationTarget {
            name,
            target_type: "Star System".to_string(),
            distance: 8.0 + (time_seed as f64 * 3.5) % 500.0,
            resolution: 15,
            coordinates: (
                (time_seed as i32 * 157) % 10000 - 5000,
                (time_seed as i32 * 271) % 10000 - 5000,
                (time_seed as i32 * 193) % 10000 - 5000,
            ),
        }
    } else {
        let name = format!(
            "{}-{}",
            planet_prefixes[time_seed % planet_prefixes.len()],
            100 + time_seed % 900
        );
        ExplorationTarget {
            name,
            target_type: "Planet".to_string(),
            distance: 0.5 + (time_seed as f64 * 0.3) % 30.0,
            resolution: 25,
            coordinates: (
                (time_seed as i32 * 173) % 100000 - 50000,
                (time_seed as i32 * 281) % 100000 - 50000,
                (time_seed as i32 * 211) % 100000 - 50000,
            ),
        }
    }
}

fn show_shutdown_sequence(stats: &MissionStats) -> Result<()> {
    clear_screen();
    println!("\n\n");
    println!("  ╔═══════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("  ║                                                                                           ║");
    println!("  ║                          🛸  MISSION DEACTIVATION SEQUENCE  🛸                            ║");
    println!("  ║                                                                                           ║");
    println!("  ╚═══════════════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Mission Duration: {}", stats.elapsed());
    println!();
    println!("  ╔═══════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("  ║  FINAL MISSION STATISTICS                                                                 ║");
    println!("  ╠═══════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("  ║                                                                                           ║");
    println!("  ║    🌌 Galaxies Scanned:             {:>10}                                              ║", stats.galaxies_scanned);
    println!("  ║    ⭐ Star Systems Explored:        {:>10}                                              ║", stats.star_systems_explored);
    println!("  ║    🪐 Planets Discovered:           {:>10}                                              ║", stats.planets_discovered);
    println!("  ║       └─ Habitable Worlds:         {:>10}                                              ║", stats.habitable_planets);
    println!("  ║    ❓ Anomalies Detected:           {:>10}                                              ║", stats.anomalies_detected);
    println!("  ║    🚧 Obstacles Avoided:            {:>10}                                              ║", stats.obstacles_avoided);
    println!("  ║    📏 Distance Traveled:            {:>10.2} LY                                         ║", stats.total_distance);
    println!("  ║    🛸 Probes Deployed:              {:>10}                                              ║", stats.probes_deployed);
    println!("  ║    🤖 Drones Deployed:              {:>10}                                              ║", stats.drones_active);
    println!("  ║    🚀 FTL Jumps Completed:          {:>10}                                              ║", stats.jumps_completed);
    println!("  ║                                                                                           ║");
    println!("  ╚═══════════════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Data successfully transmitted to Starfleet Command.");
    println!();
    println!("  Safe travels, Captain. Live long and prosper. 🖖");
    println!();

    Ok(())
}

fn spawn_key_listener() -> Receiver<char> {
    let (tx, rx) = channel();

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = [0u8; 1];

        loop {
            if handle.read(&mut buffer).is_ok() {
                let ch = buffer[0] as char;
                if tx.send(ch).is_err() {
                    break;
                }
            }
        }
    });

    rx
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
