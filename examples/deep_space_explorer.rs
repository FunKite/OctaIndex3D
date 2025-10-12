//! Deep Space Explorer - Cinematic Exploration Experience
//!
//! A cinematic, educational interface showcasing OctaGrid/OctaIndex3D BCC lattice
//! navigation with proper pacing, probe/drone perspectives, and detailed
//! visualization of the octahedral cell structure.

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use octaindex3d::layer::{CellFlags, Layer};
use octaindex3d::path::{astar, k_ring, AvoidBlockedCost, EuclideanCost};
use octaindex3d::{CellID, Result as OctaResult};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::{HashSet, VecDeque};
use std::io;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(100);
const SCAN_DELAY_MS: u64 = 1500;
const DISCOVERY_DELAY_MS: u64 = 2000;
const PROBE_DEPLOY_DELAY_MS: u64 = 3000;

#[derive(Debug, Clone, PartialEq)]
enum ViewContext {
    Mothership,
    Probe { id: usize, target: String },
    Drone { id: usize, location: String },
}

struct App {
    stats: MissionStats,
    current_target: Option<ExplorationTarget>,
    log_entries: VecDeque<LogEntry>,
    phase: ExplorationPhase,
    navigation_progress: f64,
    current_position: (i32, i32, i32),
    goal_position: (i32, i32, i32),
    obstacles: HashSet<(i32, i32, i32)>,
    path: Vec<(i32, i32, i32)>,
    current_step: usize,
    should_quit: bool,
    view_context: ViewContext,
    delay_counter: u64,
    delay_target: u64,
    current_resolution: u8,
    cell_neighbors: Vec<(i32, i32, i32)>,
    travel_speed: usize, // cells per tick
    pending_discoveries: Vec<Discovery>,
    discovery_index: usize,
    drone_scan_step: usize,
    probes_to_retrieve: u32,
    drones_to_retrieve: u32,
    asset_recovery_step: usize,
    active_drone_ids: Vec<usize>,
    active_probe_id: Option<usize>,
    time_compression: f64,
    time_compression_stage: u8,
}

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
    cells_traversed: u32,
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
            cells_traversed: 0,
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

#[derive(Debug, Clone)]
struct ExplorationTarget {
    name: String,
    target_type: String,
    distance: f64,
    resolution: u8,
    coordinates: (i32, i32, i32),
}

#[derive(Debug, Clone)]
struct LogEntry {
    message: String,
    log_type: LogType,
    timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
enum LogType {
    Info,
    Success,
    Warning,
    Discovery,
    Tech,
}

#[derive(Debug, Clone, PartialEq)]
enum ExplorationPhase {
    Idle,
    InitializingScan,
    Scanning,
    AnalyzingTopology,
    ComputingPath,
    PreparingNavigation,
    Traveling,
    Arriving,
    DeployingProbes,
    ProbeTransit,
    DroneScan,
    RetrievingAssets,
    AnalyzingDiscoveries,
    Complete,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            stats: MissionStats::new(),
            current_target: None,
            log_entries: VecDeque::new(),
            phase: ExplorationPhase::Idle,
            navigation_progress: 0.0,
            current_position: (0, 0, 0),
            goal_position: (0, 0, 0),
            obstacles: HashSet::new(),
            path: Vec::new(),
            current_step: 0,
            should_quit: false,
            view_context: ViewContext::Mothership,
            delay_counter: 0,
            delay_target: 0,
            current_resolution: 10,
            cell_neighbors: Vec::new(),
            travel_speed: 1,
            pending_discoveries: Vec::new(),
            discovery_index: 0,
            drone_scan_step: 0,
            probes_to_retrieve: 0,
            drones_to_retrieve: 0,
            asset_recovery_step: 0,
            active_drone_ids: Vec::new(),
            active_probe_id: None,
            time_compression: 1.0,
            time_compression_stage: 0,
        };

        app.add_log(
            LogType::Success,
            "U.S.S. NAVIGATOR - All systems operational",
        );
        app.add_log(
            LogType::Tech,
            "OctaGrid navigation core (OctaIndex3D engine) initialized",
        );
        app.add_log(
            LogType::Tech,
            "BCC Lattice engine: 14-neighbor connectivity active",
        );
        app.add_log(LogType::Info, "Beginning continuous exploration protocol");

        app
    }

    fn add_log(&mut self, log_type: LogType, message: &str) {
        let timestamp = format!("{}", self.stats.elapsed());
        self.log_entries.push_back(LogEntry {
            message: message.to_string(),
            log_type,
            timestamp,
        });
        if self.log_entries.len() > 100 {
            self.log_entries.pop_front();
        }
    }

    fn set_delay(&mut self, ms: u64) {
        self.delay_target = ms / 100; // Convert to ticks
        self.delay_counter = 0;
    }

    fn is_delayed(&mut self) -> bool {
        if self.delay_counter < self.delay_target {
            self.delay_counter += 1;
            true
        } else {
            // Reset delay system after completion
            self.delay_target = 0;
            self.delay_counter = 0;
            false
        }
    }

    fn start_new_exploration(&mut self) -> OctaResult<()> {
        self.view_context = ViewContext::Mothership;
        self.phase = ExplorationPhase::InitializingScan;
        self.set_delay(1000);

        let target = generate_target(&self.stats);
        self.current_resolution = target.resolution;

        self.add_log(LogType::Info, &format!("═══ NEW EXPLORATION MISSION ═══"));
        self.add_log(
            LogType::Info,
            &format!("Target: {} - {}", target.target_type, target.name),
        );
        self.add_log(
            LogType::Tech,
            &format!("Distance: {:.2} light-years", target.distance),
        );
        self.add_log(
            LogType::Tech,
            &format!("OctaGrid Resolution Level: {}", target.resolution),
        );

        match target.target_type.as_str() {
            "Nebula" => {
                self.add_log(
                    LogType::Warning,
                    "Mission Profile: Nebula survey - ionized gas reduces visibility",
                );
                self.add_log(
                    LogType::Tech,
                    "Adaptive plasma filters engaged for magneto-hydrodynamic turbulence",
                );
            }
            "Binary System" => {
                self.add_log(
                    LogType::Tech,
                    "Mission Profile: Binary grav-lens analysis and planetary census",
                );
                self.add_log(
                    LogType::Warning,
                    "Caution: Dual-star radiation flares predicted",
                );
            }
            "Rogue Planet" => {
                self.add_log(
                    LogType::Info,
                    "Mission Profile: Rogue world reconnaissance in interstellar dark space",
                );
                self.add_log(
                    LogType::Warning,
                    "Thermal imaging prioritized - no parent star detected",
                );
            }
            "Galaxy" => {
                self.add_log(
                    LogType::Info,
                    "Mission Profile: Deep field galactic structure mapping",
                );
            }
            "Star System" => {
                self.add_log(
                    LogType::Info,
                    "Mission Profile: Frontier system survey and resource cataloguing",
                );
            }
            "Planet" => {
                self.add_log(
                    LogType::Info,
                    "Mission Profile: Detailed planetary biosignature sweep",
                );
            }
            _ => {}
        }

        self.current_target = Some(target);
        self.time_compression = 1.0;
        self.time_compression_stage = 0;
        Ok(())
    }

    fn process_scan(&mut self) -> OctaResult<()> {
        if let Some(target) = self.current_target.clone() {
            self.phase = ExplorationPhase::Scanning;
            self.add_log(LogType::Info, "Initiating long-range sensor array...");

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

            self.current_position = (start.x(), start.y(), start.z());
            self.goal_position = (goal.x(), goal.y(), goal.z());

            // Get neighbors to show BCC lattice
            let neighbors = start.neighbors();
            self.cell_neighbors = neighbors.iter().map(|c| (c.x(), c.y(), c.z())).collect();

            self.add_log(
                LogType::Tech,
                &format!(
                    "BCC Lattice: Scanning {} neighbor cells (14-connectivity)",
                    neighbors.len()
                ),
            );
            self.add_log(
                LogType::Tech,
                "OctaGrid neighbor catalog cached for real-time routing",
            );

            // Scan for obstacles
            let scan_cells = k_ring(start, 3);
            self.add_log(
                LogType::Success,
                &format!(
                    "K-ring scan complete: {} spatial cells analyzed",
                    scan_cells.len()
                ),
            );
            self.add_log(
                LogType::Tech,
                "Truncated octahedron cells tile 3D space perfectly",
            );

            // Generate obstacles
            let mut obstacles_layer = Layer::new("obstacles");
            self.obstacles.clear();
            let mut obstacle_count = 0;

            for cell in &scan_cells {
                let hash =
                    (cell.x().abs() * 73 + cell.y().abs() * 179 + cell.z().abs() * 283) % 100;
                if (hash as f64) < 18.0 {
                    let mut flags = CellFlags::empty();
                    flags.set_flag(CellFlags::BLOCKED);
                    obstacles_layer.set(*cell, flags);
                    self.obstacles.insert((cell.x(), cell.y(), cell.z()));
                    obstacle_count += 1;
                }
            }

            self.stats.obstacles_avoided += obstacle_count;

            if obstacle_count > 0 {
                self.add_log(
                    LogType::Warning,
                    &format!(
                        "Spatial hazards detected: {} obstacles in flight corridor",
                        obstacle_count
                    ),
                );
            } else {
                self.add_log(
                    LogType::Success,
                    "Navigation corridor clear - optimal conditions",
                );
            }

            // Calculate path now (during scanning phase)
            let path_result = if obstacle_count > 0 {
                astar(start, goal, &AvoidBlockedCost::new(obstacles_layer, 1000.0))?
            } else {
                astar(start, goal, &EuclideanCost)?
            };

            self.path = path_result
                .cells
                .iter()
                .map(|c| (c.x(), c.y(), c.z()))
                .collect();
            self.stats.total_distance += path_result.cost;

            // Move to next phase
            self.phase = ExplorationPhase::AnalyzingTopology;
            self.set_delay(SCAN_DELAY_MS);
        }
        Ok(())
    }

    fn analyze_topology(&mut self) {
        self.add_log(LogType::Tech, "Analyzing BCC lattice topology...");
        self.add_log(
            LogType::Tech,
            "Truncated octahedron cells provide optimal space filling",
        );
        self.add_log(
            LogType::Tech,
            "OctaGrid adjacency map validated against obstacle layer",
        );
        self.phase = ExplorationPhase::ComputingPath;
        self.set_delay(1000);
    }

    fn compute_path(&mut self) {
        self.add_log(
            LogType::Info,
            "Computing optimal navigation path with A* algorithm...",
        );
        self.add_log(
            LogType::Tech,
            "Cost function: Euclidean distance + obstacle avoidance",
        );
        self.add_log(
            LogType::Tech,
            "OctaGrid cost map resolved for deterministic route planning",
        );
        self.add_log(
            LogType::Success,
            &format!("Navigation solution found: {} waypoints", self.path.len()),
        );
        if !self.path.is_empty() {
            let (gx, gy, gz) = self.goal_position;
            let (sx, sy, sz) = self.path[0];
            let distance = (((gx - sx).pow(2) + (gy - sy).pow(2) + (gz - sz).pow(2)) as f64).sqrt();
            self.add_log(
                LogType::Tech,
                &format!("Path length: {:.2} cells", distance),
            );
        }
        self.add_log(
            LogType::Tech,
            &format!("Traversing {} octahedral cells", self.path.len()),
        );
        self.phase = ExplorationPhase::PreparingNavigation;
        self.set_delay(1200);
    }

    fn start_navigation(&mut self) {
        self.phase = ExplorationPhase::Traveling;
        self.navigation_progress = 0.0;
        self.current_step = 0;
        self.travel_speed = 1; // Start slow
        self.time_compression = 8.0;
        self.time_compression_stage = 0;
        self.add_log(
            LogType::Info,
            "Initiating sustained propulsion burn across OctaGrid corridor...",
        );
        self.add_log(
            LogType::Tech,
            "OctaGrid lattice locked; executing cell-by-cell traversal",
        );
        self.add_log(
            LogType::Tech,
            &format!(
                "Time compression engaged at {:.1}× to accelerate cruise monitoring",
                self.time_compression
            ),
        );
    }

    fn update_travel(&mut self) {
        if self.phase == ExplorationPhase::Traveling {
            if self.current_step < self.path.len() {
                // Gradually increase speed
                if self.current_step > 10 && self.travel_speed < 3 {
                    self.travel_speed = 2;
                }
                if self.current_step > 20 && self.travel_speed < 4 {
                    self.travel_speed = 3;
                }

                // Move through cells
                for _ in 0..self.travel_speed {
                    if self.current_step < self.path.len() {
                        self.current_position = self.path[self.current_step];
                        self.current_step += 1;
                        self.stats.cells_traversed += 1;
                    }
                }

                self.navigation_progress =
                    (self.current_step as f64 / self.path.len() as f64) * 100.0;

                // Dynamic time compression adjustments
                let progress_ratio = self.navigation_progress / 100.0;
                let stage = if progress_ratio < 0.25 {
                    0usize
                } else if progress_ratio < 0.5 {
                    1
                } else if progress_ratio < 0.75 {
                    2
                } else {
                    3
                };
                let compression_schedule = [8.0, 14.0, 22.0, 32.0];
                if stage > self.time_compression_stage as usize {
                    self.time_compression_stage = stage as u8;
                    self.time_compression = compression_schedule[stage];
                    self.add_log(
                        LogType::Info,
                        &format!(
                            "Time compression adjusted to {:.1}× for cruise segment {}",
                            self.time_compression,
                            stage + 1
                        ),
                    );
                }

                // Log progress at milestones
                if self.current_step == self.path.len() / 4 {
                    self.add_log(
                        LogType::Info,
                        "25% complete - Maintaining optimal trajectory",
                    );
                } else if self.current_step == self.path.len() / 2 {
                    self.add_log(LogType::Info, "50% complete - Halfway to destination");
                } else if self.current_step == 3 * self.path.len() / 4 {
                    self.add_log(
                        LogType::Info,
                        "75% complete - Beginning deceleration sequence",
                    );
                }
            } else {
                self.time_compression = 1.0;
                self.time_compression_stage = 0;
                self.phase = ExplorationPhase::Arriving;
                self.add_log(
                    LogType::Success,
                    "Destination reached - All systems nominal",
                );
                self.add_log(
                    LogType::Tech,
                    &format!("Total cells traversed: {}", self.path.len()),
                );
                self.add_log(
                    LogType::Tech,
                    "Time compression released to real-time for arrival procedures",
                );
                self.set_delay(1500);
            }
        }
    }

    fn analyze_discoveries(&mut self) -> OctaResult<()> {
        // First time entering this phase - load discoveries
        if self.pending_discoveries.is_empty() && self.discovery_index == 0 {
            self.phase = ExplorationPhase::AnalyzingDiscoveries;
            if let Some(target) = self.current_target.clone() {
                self.add_log(LogType::Info, "Performing detailed spectral analysis...");
                self.pending_discoveries = make_discoveries(&target, &mut self.stats)?;
                self.set_delay(1000);
                return Ok(());
            }
        }

        // Process one discovery at a time
        if self.discovery_index < self.pending_discoveries.len() {
            let discovery = self.pending_discoveries[self.discovery_index].clone();

            match discovery {
                Discovery::Galaxy { name, size } => {
                    self.stats.galaxies_scanned += 1;
                    self.add_log(
                        LogType::Discovery,
                        &format!("GALAXY IDENTIFIED: {} - Classification: {}", name, size),
                    );
                    self.set_delay(DISCOVERY_DELAY_MS);
                }
                Discovery::StarSystem { name, stars } => {
                    self.stats.star_systems_explored += 1;
                    self.add_log(
                        LogType::Discovery,
                        &format!("STAR SYSTEM: {} - Stellar objects: {}", name, stars),
                    );
                    self.set_delay(DISCOVERY_DELAY_MS);
                }
                Discovery::Planet {
                    name,
                    planet_type,
                    habitable,
                } => {
                    self.stats.planets_discovered += 1;
                    if habitable {
                        self.stats.habitable_planets += 1;
                        self.add_log(
                            LogType::Discovery,
                            &format!(
                                "HABITABLE PLANET: {} - Type: {} - LIFE POTENTIAL!",
                                name, planet_type
                            ),
                        );
                        self.set_delay(DISCOVERY_DELAY_MS + 500);

                        // Clear discovery state and trigger probe deployment
                        self.pending_discoveries.clear();
                        self.discovery_index = 0;
                        self.phase = ExplorationPhase::DeployingProbes;
                        return Ok(());
                    } else {
                        self.add_log(
                            LogType::Discovery,
                            &format!("PLANET: {} - Type: {}", name, planet_type),
                        );
                        self.set_delay(DISCOVERY_DELAY_MS);
                    }
                }
                Discovery::Anomaly { name, description } => {
                    self.stats.anomalies_detected += 1;
                    self.add_log(
                        LogType::Warning,
                        &format!("ANOMALY: {} - {}", name, description),
                    );
                    self.set_delay(DISCOVERY_DELAY_MS + 500);
                }
            }

            self.discovery_index += 1;
        } else {
            // All discoveries processed
            self.add_log(
                LogType::Success,
                "All data catalogued and transmitted to Starfleet",
            );
            self.stats.jumps_completed += 1;
            self.pending_discoveries.clear();
            self.discovery_index = 0;
            self.phase = ExplorationPhase::Complete;
            self.set_delay(2000);
        }

        Ok(())
    }

    fn deploy_probe(&mut self) -> OctaResult<()> {
        if let Some(target) = self.current_target.clone() {
            self.stats.probes_deployed += 1;
            let probe_id = self.stats.probes_deployed as usize;
            self.probes_to_retrieve += 1;
            self.active_probe_id = Some(probe_id);

            self.add_log(
                LogType::Info,
                &format!("═══ DEPLOYING PROBE {} ═══", probe_id),
            );
            self.add_log(
                LogType::Tech,
                "Probe equipped with miniaturized OctaGrid guidance (OctaIndex3D engine)",
            );
            self.set_delay(PROBE_DEPLOY_DELAY_MS);

            self.add_log(LogType::Info, "Probe separating from mothership...");
            self.set_delay(1500);

            // Switch to probe view
            self.view_context = ViewContext::Probe {
                id: probe_id,
                target: target.name.clone(),
            };
            self.add_log(
                LogType::Success,
                &format!("═══ PROBE {} AUTONOMOUS CONTROL ═══", probe_id),
            );
            self.add_log(LogType::Tech, "Increasing resolution for detailed survey");

            // Probe navigates at higher resolution
            self.current_resolution += 3;
            self.add_log(
                LogType::Tech,
                &format!(
                    "Resolution increased: {} -> {}",
                    self.current_resolution - 3,
                    self.current_resolution
                ),
            );

            self.phase = ExplorationPhase::ProbeTransit;
            self.set_delay(2000);
            self.time_compression = 4.0;
            self.time_compression_stage = 0;
            self.add_log(
                LogType::Tech,
                &format!(
                    "Time compression adjusted to {:.1}× for probe descent telemetry",
                    self.time_compression
                ),
            );

            // Create short probe path (closer to surface)
            let probe_start = CellID::from_coords(
                0,
                self.current_resolution,
                self.goal_position.0 * 8,
                self.goal_position.1 * 8,
                self.goal_position.2 * 8,
            )?;
            let probe_goal = CellID::from_coords(
                0,
                self.current_resolution,
                self.goal_position.0 * 8 + 50,
                self.goal_position.1 * 8 + 50,
                self.goal_position.2 * 8 - 100,
            )?;

            self.current_position = (probe_start.x(), probe_start.y(), probe_start.z());
            self.goal_position = (probe_goal.x(), probe_goal.y(), probe_goal.z());

            let probe_path = astar(probe_start, probe_goal, &EuclideanCost)?;
            self.path = probe_path
                .cells
                .iter()
                .map(|c| (c.x(), c.y(), c.z()))
                .collect();

            self.add_log(LogType::Info, "Probe descending to survey altitude...");
            self.add_log(
                LogType::Tech,
                &format!(
                    "Probe path: {} waypoints at resolution {}",
                    self.path.len(),
                    self.current_resolution
                ),
            );
            self.add_log(
                LogType::Tech,
                "OctaGrid lattice refined for close-approach probe corridor",
            );

            self.current_step = 0;
            self.navigation_progress = 0.0;
            self.travel_speed = 1;
            self.set_delay(1000);
        }
        Ok(())
    }

    fn process_drone_scan(&mut self) -> OctaResult<()> {
        if let Some(target) = self.current_target.clone() {
            self.phase = ExplorationPhase::DroneScan;

            // Use step counter to process one step at a time
            match self.drone_scan_step {
                0 => {
                    // Deploy drones
                    self.active_drone_ids.clear();
                    let starting_id = self.stats.drones_active + 1;
                    self.stats.drones_active += 4;
                    for i in 0..4 {
                        self.active_drone_ids.push((starting_id + i) as usize);
                    }
                    self.drones_to_retrieve += 4;
                    let drone_id = self
                        .active_drone_ids
                        .first()
                        .copied()
                        .unwrap_or(starting_id as usize);
                    self.view_context = ViewContext::Drone {
                        id: drone_id,
                        location: target.name.clone(),
                    };
                    self.add_log(LogType::Success, "═══ DEPLOYING 4 SPECIALIZED DRONES ═══");
                    self.add_log(
                        LogType::Tech,
                        "Atmospheric | Jungle | Surface | Biological survey teams launched",
                    );
                    self.time_compression = 1.0;
                    self.time_compression_stage = 0;
                    self.add_log(
                        LogType::Tech,
                        "Time compression set to 1.0× for real-time drone supervision",
                    );
                    self.current_resolution += 2;
                    self.set_delay(2000);
                }
                1 => {
                    self.add_log(LogType::Tech, "═══ ATMOSPHERIC VOLUMETRIC ANALYSIS ═══");
                    self.add_log(
                        LogType::Tech,
                        &format!(
                            "OctaGrid 3D atmospheric mapping: Res {}",
                            self.current_resolution
                        ),
                    );
                    self.set_delay(2500);
                }
                2 => {
                    self.add_log(LogType::Info, "→ Layer 1 (0-1km): Troposphere base mapped");
                    self.add_log(
                        LogType::Tech,
                        "  BCC lattice: 2,847 cells | O2: 21.3%, N2: 77.8%",
                    );
                    self.set_delay(2200);
                }
                3 => {
                    self.add_log(LogType::Info, "→ Layer 2 (1-5km): Mid troposphere");
                    self.add_log(
                        LogType::Tech,
                        "  Octahedral cells: 4,532 | Temp gradient: -6.5°C/km",
                    );
                    self.set_delay(2200);
                }
                4 => {
                    self.add_log(LogType::Info, "→ Layer 3 (5-12km): Upper troposphere");
                    self.add_log(
                        LogType::Success,
                        "  K-ring scan complete: Clouds & convection mapped",
                    );
                    self.add_log(
                        LogType::Tech,
                        "✓ 3D atmospheric model: 8,403 volumetric cells",
                    );
                    self.set_delay(2500);
                }
                5 => {
                    self.add_log(LogType::Tech, "═══ JUNGLE VOLUMETRIC ANALYSIS ═══");
                    self.add_log(
                        LogType::Tech,
                        "OctaGrid 3D forest mapping: Surface to canopy",
                    );
                    self.set_delay(2500);
                }
                6 => {
                    self.add_log(LogType::Info, "→ Forest floor (0-2m): Undergrowth layer");
                    self.add_log(
                        LogType::Tech,
                        "  BCC 14-neighbor: 1,847 cells | Density: 73%",
                    );
                    self.set_delay(2200);
                }
                7 => {
                    self.add_log(LogType::Info, "→ Understory (2-10m): Sapling layer");
                    self.add_log(
                        LogType::Tech,
                        "  Octahedral scan: 3,421 cells | Biomass: 127 kg/m³",
                    );
                    self.set_delay(2200);
                }
                8 => {
                    self.add_log(LogType::Info, "→ Mid-canopy (10-25m): Main canopy");
                    self.add_log(
                        LogType::Success,
                        "  A* pathfinding: Branch network | Coverage: 87%",
                    );
                    self.set_delay(2200);
                }
                9 => {
                    self.add_log(LogType::Info, "→ Emergent layer (25-45m): Canopy tops");
                    self.add_log(
                        LogType::Success,
                        "  Volumetric: 2,156 cells | Max height: 42m",
                    );
                    self.add_log(LogType::Tech, "✓ 3D jungle model: 7,424 volumetric cells");
                    self.set_delay(2500);
                }
                10 => {
                    self.add_log(
                        LogType::Success,
                        "Surface composition & biological markers analyzed",
                    );
                    self.add_log(
                        LogType::Success,
                        "Complete 3D biosphere data transmitted to probe",
                    );
                    self.set_delay(2500);
                }
                _ => {
                    // Return to mothership and start asset collection
                    self.view_context = ViewContext::Mothership;
                    self.add_log(LogType::Info, "═══ RETRIEVING DEPLOYED ASSETS ═══");
                    self.drone_scan_step = 0;
                    self.asset_recovery_step = 0;
                    self.phase = ExplorationPhase::RetrievingAssets;
                    self.set_delay(1200);
                    return Ok(());
                }
            }

            self.drone_scan_step += 1;
        }
        Ok(())
    }

    fn process_asset_retrieval(&mut self) -> OctaResult<()> {
        match self.asset_recovery_step {
            0 => {
                self.add_log(LogType::Info, "Coordinating recovery of deployed assets...");
                self.time_compression = 1.0;
                self.time_compression_stage = 0;
                self.asset_recovery_step = 1;
                self.set_delay(1200);
            }
            1 => {
                if self.drones_to_retrieve > 0 {
                    let drone_id = self.active_drone_ids.pop();
                    if let Some(id) = drone_id {
                        self.add_log(
                            LogType::Info,
                            &format!("Drone {} docking with hangar bay...", id),
                        );
                    } else {
                        self.add_log(LogType::Info, "Drone returning to hangar bay...");
                    }
                    if self.drones_to_retrieve > 0 {
                        self.drones_to_retrieve -= 1;
                    }
                    if self.stats.drones_active > 0 {
                        self.stats.drones_active -= 1;
                    }
                    self.set_delay(1200);
                } else {
                    self.add_log(
                        LogType::Success,
                        "All surface drones secured aboard mothership",
                    );
                    self.asset_recovery_step = 2;
                    self.set_delay(1500);
                }
            }
            2 => {
                if self.probes_to_retrieve > 0 {
                    if let Some(probe_id) = self.active_probe_id.take() {
                        self.add_log(
                            LogType::Info,
                            &format!("Probe {} docking with primary bay...", probe_id),
                        );
                    } else {
                        self.add_log(LogType::Info, "Probe docking with primary bay...");
                    }
                    self.probes_to_retrieve -= 1;
                    self.set_delay(1800);
                } else {
                    self.add_log(
                        LogType::Success,
                        "All probes secured and telemetry archived",
                    );
                    self.asset_recovery_step = 3;
                    self.set_delay(1000);
                }
            }
            3 => {
                self.add_log(
                    LogType::Tech,
                    "Stowing assets and resetting survey instrumentation",
                );
                self.asset_recovery_step = 4;
                self.set_delay(1200);
            }
            4 => {
                self.add_log(
                    LogType::Success,
                    "Mission assets recovered - preparing for next jump",
                );
                self.asset_recovery_step = 0;
                self.drones_to_retrieve = 0;
                self.active_drone_ids.clear();
                self.probes_to_retrieve = 0;
                self.active_probe_id = None;
                self.view_context = ViewContext::Mothership;
                self.phase = ExplorationPhase::Complete;
                self.current_resolution = self.current_resolution.saturating_sub(5);
                self.stats.jumps_completed += 1;
                self.set_delay(2000);
            }
            _ => {}
        }

        Ok(())
    }

    fn tick(&mut self) -> OctaResult<()> {
        // Handle delays
        if self.delay_target > 0 && self.is_delayed() {
            return Ok(());
        }

        match self.phase {
            ExplorationPhase::Idle => {
                self.start_new_exploration()?;
            }
            ExplorationPhase::InitializingScan => {
                self.process_scan()?;
            }
            ExplorationPhase::Scanning => {
                // Scanning is now handled immediately in process_scan()
                // This state is only hit if there's a delay
            }
            ExplorationPhase::AnalyzingTopology => {
                self.analyze_topology();
            }
            ExplorationPhase::ComputingPath => {
                self.compute_path();
            }
            ExplorationPhase::PreparingNavigation => {
                self.start_navigation();
            }
            ExplorationPhase::Traveling => {
                self.update_travel();
            }
            ExplorationPhase::Arriving => {
                self.analyze_discoveries()?;
            }
            ExplorationPhase::AnalyzingDiscoveries => {
                // Process one discovery at a time
                self.analyze_discoveries()?;
            }
            ExplorationPhase::DeployingProbes => {
                self.deploy_probe()?;
            }
            ExplorationPhase::ProbeTransit => {
                if self.current_step < self.path.len() {
                    if let Some(position) = self.path.get(self.current_step) {
                        self.current_position = *position;
                    }
                    self.current_step += 1;
                    let total_steps = if self.path.is_empty() {
                        1
                    } else {
                        self.path.len()
                    };
                    self.navigation_progress =
                        (self.current_step as f64 / total_steps as f64) * 100.0;
                    if self.current_step == self.path.len() / 2 {
                        self.add_log(LogType::Info, "Probe at mid-course correction point");
                    }
                    self.set_delay(150);
                } else {
                    self.add_log(LogType::Success, "Probe reached survey position");
                    self.process_drone_scan()?;
                }
            }
            ExplorationPhase::DroneScan => {
                self.process_drone_scan()?;
            }
            ExplorationPhase::RetrievingAssets => {
                self.process_asset_retrieval()?;
            }
            ExplorationPhase::Complete => {
                self.start_new_exploration()?;
            }
        }
        Ok(())
    }
}

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    show_final_stats(&app.stats);

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            if let Err(e) = app.tick() {
                app.add_log(LogType::Warning, &format!("Error: {:?}", e));
            }
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let frame = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(size);

    render_header(f, frame[0], app);
    render_footer(f, frame[2], app);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(frame[1]);

    render_log(f, body[0], app);

    let right_column = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)])
        .split(body[1]);

    render_stats(f, right_column[0], app);

    let detail = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(right_column[1]);

    render_navigation_overview(f, detail[0], app);

    let lower = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(detail[1]);

    render_telemetry(f, lower[0], app);
    render_octagrid_panel(f, lower[1], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let context_str = match &app.view_context {
        ViewContext::Mothership => "U.S.S. NAVIGATOR - MOTHERSHIP".to_string(),
        ViewContext::Probe { id, target } => format!("PROBE {} - Surveying {}", id, target),
        ViewContext::Drone { id, location } => {
            format!("DRONE {} - Surface Operations on {}", id, location)
        }
    };

    let title = format!(
        " {} | Mission Time: {} | Jumps: {} | Time Compression: {:.1}× | Press 'q' to exit ",
        context_str,
        app.stats.elapsed(),
        app.stats.jumps_completed,
        app.time_compression
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(title);

    f.render_widget(block, area);
}

fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Exploration stats
    let exploration_lines = vec![
        Line::from(vec![
            Span::styled("Galaxies:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.galaxies_scanned),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Star Systems: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.star_systems_explored),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Planets:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.planets_discovered),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Habitable:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.habitable_planets),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Anomalies:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.anomalies_detected),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Distance:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>8.1} LY", app.stats.total_distance),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];

    let exploration_widget = Paragraph::new(exploration_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Discoveries "),
        )
        .style(Style::default());
    f.render_widget(exploration_widget, chunks[0]);

    // OctaGrid Tech
    let tech_lines = vec![
        Line::from(vec![
            Span::styled("Resolution:     ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>3}", app.current_resolution),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cells Traversed:", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.cells_traversed),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Obstacles:      ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6}", app.stats.obstacles_avoided),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled("Lattice:        ", Style::default().fg(Color::Gray)),
            Span::styled("BCC 14-nbr", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Cells:          ", Style::default().fg(Color::Gray)),
            Span::styled("Octahedral", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Algorithm:      ", Style::default().fg(Color::Gray)),
            Span::styled("A* + K-ring", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Time Compression:", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>6.1}×", app.time_compression),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let tech_widget = Paragraph::new(tech_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" OctaGrid Technology "),
        )
        .style(Style::default());
    f.render_widget(tech_widget, chunks[1]);

    // Mission assets
    let assets_lines = vec![
        Line::from(vec![
            Span::styled("Probes:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>4}", app.stats.probes_deployed),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Drones:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>4}", app.stats.drones_active),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(Span::styled("", Style::default())),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:?}", app.phase), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("View:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                match &app.view_context {
                    ViewContext::Mothership => "Mothership",
                    ViewContext::Probe { .. } => "Probe",
                    ViewContext::Drone { .. } => "Drone",
                },
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let assets_widget = Paragraph::new(assets_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mission Assets "),
        )
        .style(Style::default());
    f.render_widget(assets_widget, chunks[2]);
}

fn render_navigation_overview(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    let phase_summary = match app.phase {
        ExplorationPhase::Idle => "Standing by for next assignment...",
        ExplorationPhase::InitializingScan => {
            "Initializing sensor array and coarse OctaGrid sweep..."
        }
        ExplorationPhase::Scanning => "Executing K-ring scan around coordinates...",
        ExplorationPhase::AnalyzingTopology => "Computing BCC lattice topology for safe routing...",
        ExplorationPhase::ComputingPath => "Running A* search across OctaGrid lattice...",
        ExplorationPhase::PreparingNavigation => {
            "Aligning thrusters and synchronizing lattice checkpoints..."
        }
        ExplorationPhase::Traveling => "Cruise phase underway - monitoring OctaGrid corridor...",
        ExplorationPhase::Arriving => "Arrival checks - compressing data and braking burn...",
        ExplorationPhase::DeployingProbes => "Preparing probe release sequence...",
        ExplorationPhase::ProbeTransit => "Probe descending through mapped corridor...",
        ExplorationPhase::DroneScan => "Surface drones conducting layered surveys...",
        ExplorationPhase::RetrievingAssets => "Collecting probes and drones for departure...",
        ExplorationPhase::AnalyzingDiscoveries => "Cataloguing discoveries for transmission...",
        ExplorationPhase::Complete => "Mission cycle complete - requesting next target...",
    };

    lines.push(Line::from(Span::styled(
        phase_summary,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::ITALIC),
    )));
    lines.push(Line::from(""));

    if !app.path.is_empty() {
        let bar_width = (area.width as usize).saturating_sub(16);
        let filled = ((app.navigation_progress / 100.0) * bar_width as f64) as usize;
        let progress_bar = format!(
            "[{}{}] {:>5.1}%",
            "=".repeat(filled),
            " ".repeat(bar_width.saturating_sub(filled)),
            app.navigation_progress
        );
        lines.push(Line::from(Span::styled(
            progress_bar,
            Style::default().fg(Color::Green),
        )));

        lines.push(Line::from(vec![
            Span::styled("Waypoint: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{}/{}",
                    app.current_step.min(app.path.len()),
                    app.path.len()
                ),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    if matches!(
        app.phase,
        ExplorationPhase::Traveling
            | ExplorationPhase::ProbeTransit
            | ExplorationPhase::Arriving
            | ExplorationPhase::PreparingNavigation
    ) {
        lines.push(Line::from(vec![
            Span::styled("Time Compression: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>4.1}× realtime", app.time_compression),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Cruise Speed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} lattice cells/tick", app.travel_speed),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    if let Some(target) = &app.current_target {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Gray)),
            Span::styled(&target.name, Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("OctaGrid Resolution: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", target.resolution),
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Navigation Overview ")
                .border_type(BorderType::Rounded),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_octagrid_panel(f: &mut Frame, area: Rect, app: &App) {
    let active_neighbors = app.cell_neighbors.len();
    let pending = app
        .pending_discoveries
        .len()
        .saturating_sub(app.discovery_index);

    let view_label = match &app.view_context {
        ViewContext::Mothership => "Mothership control deck".to_string(),
        ViewContext::Probe { target, .. } => format!("Probe feed over {}", target),
        ViewContext::Drone { location, .. } => format!("Drone survey above {}", location),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("OctaGrid Corridor: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} active neighbor cells", active_neighbors),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("Route Integrity: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if app.obstacles.is_empty() {
                    "Clear"
                } else {
                    "Obstacle avoidance engaged"
                },
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Telemetry Cache: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} pending discovery reports", pending),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Asset Recovery: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{} drones | {} probes awaiting retrieval",
                    app.drones_to_retrieve, app.probes_to_retrieve
                ),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Current View: ", Style::default().fg(Color::Gray)),
            Span::styled(view_label, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("OctaGrid Utilization: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} cells traversed this mission", app.stats.cells_traversed),
                Style::default().fg(Color::Green),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" OctaGrid Operations ")
                .border_type(BorderType::Rounded),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_telemetry(f: &mut Frame, area: Rect, app: &App) {
    let (cx, cy, cz) = app.current_position;
    let (gx, gy, gz) = app.goal_position;

    let distance_remaining =
        (((gx - cx).pow(2) + (gy - cy).pow(2) + (gz - cz).pow(2)) as f64).sqrt();

    let mut lines = vec![Line::from(vec![Span::styled(
        "╔═ PROXIMITY RADAR ═╗",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )])];

    // Radar display (circular, 11x11 character grid)
    let radar_radius = 5;
    for row in 0..11 {
        let mut radar_row = vec![Span::raw("  ")];
        for col in 0..11 {
            let dx = col as i32 - radar_radius;
            let dy = row as i32 - radar_radius;
            let dist = ((dx * dx + dy * dy) as f64).sqrt();

            // Center is current position
            if dx == 0 && dy == 0 {
                radar_row.push(Span::styled(
                    "@",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if dist <= radar_radius as f64 {
                // Calculate relative position to goal
                let rel_gx = gx - cx;
                let rel_gy = gy - cy;
                let goal_dist = ((rel_gx * rel_gx + rel_gy * rel_gy) as f64).sqrt();

                // Scale goal position to radar
                let scaled_gx = if goal_dist > 0.0 {
                    ((rel_gx as f64 / goal_dist) * radar_radius as f64) as i32
                } else {
                    0
                };
                let scaled_gy = if goal_dist > 0.0 {
                    ((rel_gy as f64 / goal_dist) * radar_radius as f64) as i32
                } else {
                    0
                };

                if dx == scaled_gx && dy == scaled_gy {
                    radar_row.push(Span::styled(
                        "★",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    // Check for obstacles in radar range
                    let check_pos = (cx + dx * 10, cy + dy * 10, cz);
                    if app.obstacles.contains(&check_pos) {
                        radar_row.push(Span::styled("●", Style::default().fg(Color::Red)));
                    } else if dist > radar_radius as f64 - 0.5 && dist < radar_radius as f64 + 0.5 {
                        // Range ring
                        radar_row.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
                    } else {
                        radar_row.push(Span::raw(" "));
                    }
                }
            } else {
                radar_row.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(radar_row));
    }

    lines.extend(vec![
        Line::from(vec![Span::styled(
            "╚═══════════════════╝",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Range: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.1} cells", distance_remaining),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Position: ",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("({}, {}, {})", cx, cy, cz),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Destination: ",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("({}, {}, {})", gx, gy, gz),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Resolution: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.current_resolution),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ]);

    if let Some(target) = &app.current_target {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Target: ", Style::default().fg(Color::Gray)),
            Span::styled(&target.name, Style::default().fg(Color::Yellow)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Navigation Radar ")
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let max_entries = area.height.saturating_sub(2) as usize;
    let capacity = max_entries.max(1);
    let log_lines: Vec<Line> = app
        .log_entries
        .iter()
        .rev()
        .take(capacity)
        .map(|entry| {
            let style = match entry.log_type {
                LogType::Info => Style::default().fg(Color::White),
                LogType::Success => Style::default().fg(Color::Green),
                LogType::Warning => Style::default().fg(Color::Yellow),
                LogType::Discovery => Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
                LogType::Tech => Style::default().fg(Color::Cyan),
            };

            let icon = match entry.log_type {
                LogType::Info => ">",
                LogType::Success => "+",
                LogType::Warning => "!",
                LogType::Discovery => "*",
                LogType::Tech => "~",
            };

            Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(format!("{} ", icon), style),
                Span::styled(&entry.message, style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(log_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mission Log — Latest First ")
                .border_type(BorderType::Rounded),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let text = format!(
        " OctaGrid v0.2.0 (OctaIndex3D engine) | BCC Lattice: {} cells | Resolution: {} | 14-Neighbor Connectivity | Truncated Octahedron Tiling ",
        app.stats.cells_traversed,
        app.current_resolution
    );

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn generate_target(stats: &MissionStats) -> ExplorationTarget {
    let time_seed = (stats.start_time.elapsed().as_secs() + stats.jumps_completed as u64) as usize;

    let galaxy_names = [
        "Andromeda",
        "Triangulum",
        "Whirlpool",
        "Sombrero",
        "Pinwheel",
    ];
    let nebula_names = [
        "Orion Veil",
        "Ghost Cloud",
        "Cygnus Drift",
        "Crimson Shroud",
        "Aurora Veil",
    ];
    let binary_designations = [
        "Sigma Aurigae",
        "Theta Draconis",
        "Lambda Serpentis",
        "Omega Lyrae",
        "Delta Eridani",
    ];
    let rogue_names = ["Nomad", "Straylight", "Wanderer", "Outrider", "Echo"];
    let star_names = ["Alpha Centauri", "Proxima", "Sirius", "Vega", "Arcturus"];
    let planet_prefixes = ["Kepler", "TRAPPIST", "Gliese", "Ross", "Wolf"];

    let cycle = (time_seed / 2) % 24;

    if cycle < 3 {
        ExplorationTarget {
            name: galaxy_names[time_seed % galaxy_names.len()].to_string(),
            target_type: "Galaxy".to_string(),
            distance: 500_000.0 + (time_seed as f64 * 50_000.0),
            resolution: 5,
            coordinates: (
                (time_seed as i32 * 137) % 1000 - 500,
                (time_seed as i32 * 241) % 1000 - 500,
                (time_seed as i32 * 163) % 1000 - 500,
            ),
        }
    } else if cycle < 7 {
        ExplorationTarget {
            name: nebula_names[time_seed % nebula_names.len()].to_string(),
            target_type: "Nebula".to_string(),
            distance: 1_200.0 + (time_seed as f64 * 90.0) % 3_500.0,
            resolution: 12,
            coordinates: (
                (time_seed as i32 * 191) % 6_000 - 3_000,
                (time_seed as i32 * 223) % 6_000 - 3_000,
                (time_seed as i32 * 167) % 6_000 - 3_000,
            ),
        }
    } else if cycle < 12 {
        ExplorationTarget {
            name: format!(
                "{} Binary",
                binary_designations[time_seed % binary_designations.len()]
            ),
            target_type: "Binary System".to_string(),
            distance: 18.0 + (time_seed as f64 * 2.3) % 140.0,
            resolution: 18,
            coordinates: (
                (time_seed as i32 * 157) % 12_000 - 6_000,
                (time_seed as i32 * 271) % 12_000 - 6_000,
                (time_seed as i32 * 193) % 12_000 - 6_000,
            ),
        }
    } else if cycle < 16 {
        ExplorationTarget {
            name: format!(
                "{}-{}",
                rogue_names[time_seed % rogue_names.len()],
                300 + (time_seed % 700)
            ),
            target_type: "Rogue Planet".to_string(),
            distance: 0.4 + (time_seed as f64 * 0.12) % 15.0,
            resolution: 22,
            coordinates: (
                (time_seed as i32 * 199) % 40_000 - 20_000,
                (time_seed as i32 * 281) % 40_000 - 20_000,
                (time_seed as i32 * 211) % 40_000 - 20_000,
            ),
        }
    } else if cycle < 20 {
        ExplorationTarget {
            name: format!(
                "{}-{}",
                star_names[time_seed % star_names.len()],
                100 + time_seed % 900
            ),
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
        ExplorationTarget {
            name: format!(
                "{}-{}",
                planet_prefixes[time_seed % planet_prefixes.len()],
                100 + time_seed % 900
            ),
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

fn make_discoveries(
    target: &ExplorationTarget,
    _stats: &mut MissionStats,
) -> OctaResult<Vec<Discovery>> {
    let mut discoveries = Vec::new();
    let seed = (target.coordinates.0.abs()
        + target.coordinates.1.abs()
        + target.coordinates.2.abs()) as usize;

    match target.target_type.as_str() {
        "Galaxy" => {
            discoveries.push(Discovery::Galaxy {
                name: target.name.clone(),
                size: vec!["Dwarf", "Spiral", "Elliptical", "Irregular"][seed % 4].to_string(),
            });
        }
        "Nebula" => {
            let anomaly_profiles = [
                (
                    "Ionized Shock Front",
                    "Charged particle density exceeds 130%",
                ),
                (
                    "Protostar Nursery",
                    "Dense molecular clouds collapsing into proto-stars",
                ),
                (
                    "Magnetic Flux Filament",
                    "Filamentary magnetic structure steering charged dust",
                ),
                (
                    "Cryogenic Fog Pockets",
                    "Super-cooled gas pillars forming crystalline lattices",
                ),
            ];
            let anomaly = &anomaly_profiles[seed % anomaly_profiles.len()];
            discoveries.push(Discovery::Anomaly {
                name: anomaly.0.to_string(),
                description: anomaly.1.to_string(),
            });

            if seed % 3 == 0 {
                discoveries.push(Discovery::StarSystem {
                    name: format!("{} Protocluster", target.name),
                    stars: 1,
                });
            }
        }
        "Binary System" => {
            discoveries.push(Discovery::StarSystem {
                name: target.name.clone(),
                stars: 2,
            });
            let num_planets = 2 + (seed % 4);
            let planet_types = ["Rocky", "Ocean World", "Desert", "Gas Giant", "Super-Earth"];
            for i in 0..num_planets {
                let planet_type = planet_types[(seed + i) % planet_types.len()];
                let habitable = matches!(planet_type, "Rocky" | "Super-Earth" | "Ocean World")
                    && (seed + i) % 5 == 0;
                discoveries.push(Discovery::Planet {
                    name: format!("{}-{}", target.name, (b'B' + i as u8) as char),
                    planet_type: planet_type.to_string(),
                    habitable,
                });
            }

            if seed % 4 == 0 {
                discoveries.push(Discovery::Anomaly {
                    name: "Lagrange Flux".to_string(),
                    description: "Stable plasma bridge detected between binary stars".to_string(),
                });
            }
        }
        "Rogue Planet" => {
            let planet_types = ["Ice World", "Super-Earth", "Carbon World", "Silicate Core"];
            let planet_type = planet_types[seed % planet_types.len()];
            let habitable = (planet_type == "Super-Earth") && seed % 5 == 0;
            discoveries.push(Discovery::Planet {
                name: target.name.clone(),
                planet_type: planet_type.to_string(),
                habitable,
            });

            let rogue_findings = [
                (
                    "Thermal Plume",
                    "Subsurface ocean vents emitting geothermal energy",
                ),
                (
                    "Polar Cavern",
                    "Radar sounding indicates hollow structure beneath polar cap",
                ),
                (
                    "Mass Wake",
                    "Gravitational signature suggests unseen companion object",
                ),
            ];
            let finding = &rogue_findings[seed % rogue_findings.len()];
            discoveries.push(Discovery::Anomaly {
                name: finding.0.to_string(),
                description: finding.1.to_string(),
            });
        }
        "Star System" => {
            discoveries.push(Discovery::StarSystem {
                name: target.name.clone(),
                stars: 1 + (seed % 3) as u32,
            });
            let num_planets = 1 + (seed % 4);
            for i in 0..num_planets {
                let planet_types = ["Rocky", "Gas Giant", "Ice World", "Super-Earth"];
                let planet_type = planet_types[(seed + i) % planet_types.len()];
                let habitable =
                    (planet_type == "Rocky" || planet_type == "Super-Earth") && (seed + i) % 4 == 0;

                discoveries.push(Discovery::Planet {
                    name: format!("{}-{}", target.name, (b'A' + i as u8) as char),
                    planet_type: planet_type.to_string(),
                    habitable,
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

    if seed % 6 == 0 {
        let anomalies = [
            (
                "Gravity Shear",
                "Tidal gradients affecting orbital stability",
            ),
            (
                "Nebula Accretion Zone",
                "Dense molecular gas forming protostars",
            ),
            (
                "Radio Burst Source",
                "Broadband emissions traced to magnetar activity",
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

fn show_final_stats(stats: &MissionStats) {
    println!("\n╔════════════════════════════════════════════════════╗");
    println!("║     DEEP SPACE EXPLORER - MISSION COMPLETE        ║");
    println!("╠════════════════════════════════════════════════════╣");
    println!("║                                                    ║");
    println!(
        "║  Mission Duration:  {}                      ║",
        stats.elapsed()
    );
    println!(
        "║  Galaxies:          {:>6}                        ║",
        stats.galaxies_scanned
    );
    println!(
        "║  Star Systems:      {:>6}                        ║",
        stats.star_systems_explored
    );
    println!(
        "║  Planets:           {:>6}                        ║",
        stats.planets_discovered
    );
    println!(
        "║  Habitable Worlds:  {:>6}                        ║",
        stats.habitable_planets
    );
    println!(
        "║  Anomalies:         {:>6}                        ║",
        stats.anomalies_detected
    );
    println!(
        "║  Distance:      {:>10.2} LY                   ║",
        stats.total_distance
    );
    println!(
        "║  Cells Traversed:   {:>6}                        ║",
        stats.cells_traversed
    );
    println!(
        "║  Probes Deployed:   {:>6}                        ║",
        stats.probes_deployed
    );
    println!(
        "║  Drones Active:     {:>6}                        ║",
        stats.drones_active
    );
    println!("║                                                    ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!("\nThank you for exploring the universe with OctaGrid technology! 🚀\n");
}
