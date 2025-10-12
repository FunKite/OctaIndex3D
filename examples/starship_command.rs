//! Starship Command - Professional TUI Interface
//!
//! A polished terminal UI for deep space exploration using ratatui for
//! proper rendering, layout management, and Unicode handling.

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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::{HashSet, VecDeque};
use std::io;
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(250);

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
}

#[derive(Debug, Clone, PartialEq)]
enum ExplorationPhase {
    Idle,
    Scanning,
    Planning,
    Navigating,
    Analyzing,
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
        };

        app.add_log(LogType::Success, "U.S.S. NAVIGATOR systems online");
        app.add_log(LogType::Info, "All primary systems nominal");
        app.add_log(LogType::Info, "Beginning deep space exploration protocol");

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

    fn start_new_exploration(&mut self) -> OctaResult<()> {
        self.phase = ExplorationPhase::Scanning;
        let target = generate_target(&self.stats);

        self.add_log(
            LogType::Info,
            &format!("New target: {} - {}", target.target_type, target.name),
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

        self.current_position = (start.x(), start.y(), start.z());
        self.goal_position = (goal.x(), goal.y(), goal.z());

        // Scan for obstacles
        let scan_cells = k_ring(start, 3);
        self.add_log(
            LogType::Success,
            &format!("Sensor scan: {} cells analyzed", scan_cells.len()),
        );

        // Generate obstacles
        let mut obstacles_layer = Layer::new("obstacles");
        self.obstacles.clear();
        let mut obstacle_count = 0;

        for cell in &scan_cells {
            let hash = (cell.x().abs() * 73 + cell.y().abs() * 179 + cell.z().abs() * 283) % 100;
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
                &format!("{} spatial hazards detected", obstacle_count),
            );
        } else {
            self.add_log(LogType::Success, "Flight path clear");
        }

        self.phase = ExplorationPhase::Planning;
        self.add_log(LogType::Info, "Computing navigation path...");

        // Calculate path
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

        self.add_log(
            LogType::Success,
            &format!(
                "Path found: {} waypoints, {:.2} LY",
                self.path.len(),
                path_result.cost
            ),
        );
        self.stats.total_distance += path_result.cost;

        self.current_target = Some(target);
        self.phase = ExplorationPhase::Navigating;
        self.navigation_progress = 0.0;
        self.current_step = 0;

        Ok(())
    }

    fn update_navigation(&mut self) {
        if self.phase == ExplorationPhase::Navigating {
            if self.current_step < self.path.len() {
                self.current_position = self.path[self.current_step];
                self.current_step += 1;
                self.navigation_progress =
                    (self.current_step as f64 / self.path.len() as f64) * 100.0;
            } else {
                self.phase = ExplorationPhase::Analyzing;
                self.add_log(LogType::Success, "Destination reached");
            }
        }
    }

    fn analyze_discoveries(&mut self) -> OctaResult<()> {
        if let Some(target) = self.current_target.clone() {
            self.add_log(LogType::Info, "Performing spectral analysis...");

            let discoveries = make_discoveries(&target, &mut self.stats)?;

            for discovery in discoveries {
                match discovery {
                    Discovery::Galaxy { name, size } => {
                        self.stats.galaxies_scanned += 1;
                        self.add_log(LogType::Discovery, &format!("GALAXY: {} - {}", name, size));
                    }
                    Discovery::StarSystem { name, stars } => {
                        self.stats.star_systems_explored += 1;
                        self.add_log(
                            LogType::Discovery,
                            &format!("STAR SYSTEM: {} - {} star(s)", name, stars),
                        );
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
                                &format!("PLANET: {} - {} - HABITABLE!", name, planet_type),
                            );
                            self.stats.probes_deployed += 1;
                            self.stats.drones_active += 4;
                            self.add_log(LogType::Info, "  Probe deployed - 4 drones active");
                        } else {
                            self.add_log(
                                LogType::Discovery,
                                &format!("PLANET: {} - {}", name, planet_type),
                            );
                        }
                    }
                    Discovery::Anomaly { name, description } => {
                        self.stats.anomalies_detected += 1;
                        self.add_log(
                            LogType::Warning,
                            &format!("ANOMALY: {} - {}", name, description),
                        );
                    }
                }
            }

            self.add_log(LogType::Success, "Data transmitted to Starfleet");
            self.stats.jumps_completed += 1;
        }

        self.phase = ExplorationPhase::Complete;
        Ok(())
    }

    fn tick(&mut self) -> OctaResult<()> {
        match self.phase {
            ExplorationPhase::Idle | ExplorationPhase::Complete => {
                self.start_new_exploration()?;
            }
            ExplorationPhase::Navigating => {
                self.update_navigation();
            }
            ExplorationPhase::Analyzing => {
                self.analyze_discoveries()?;
            }
            _ => {}
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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
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

    // Show final stats
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

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(10), // Stats
            Constraint::Length(4),  // Target info
            Constraint::Min(15),    // Main content (3D view + telemetry)
            Constraint::Length(12), // Log
            Constraint::Length(1),  // Footer
        ])
        .split(size);

    // Header
    render_header(f, chunks[0], app);

    // Stats panels
    render_stats(f, chunks[1], app);

    // Target info
    render_target(f, chunks[2], app);

    // Main content area (3D view + telemetry)
    render_main_content(f, chunks[3], app);

    // Log
    render_log(f, chunks[4], app);

    // Footer
    render_footer(f, chunks[5]);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " U.S.S. NAVIGATOR - Mission Time: {} | Status: ACTIVE | Jumps: {} | Press 'q' to exit ",
        app.stats.elapsed(),
        app.stats.jumps_completed
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan))
        .title(title);

    f.render_widget(block, area);
}

fn render_stats(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // Exploration stats
    let exploration = vec![
        format!("Galaxies:        {:>6}", app.stats.galaxies_scanned),
        format!("Star Systems:    {:>6}", app.stats.star_systems_explored),
        format!("Planets:         {:>6}", app.stats.planets_discovered),
        format!("  Habitable:     {:>6}", app.stats.habitable_planets),
        format!("Anomalies:       {:>6}", app.stats.anomalies_detected),
        format!("Obstacles:       {:>6}", app.stats.obstacles_avoided),
        format!("Distance:    {:>8.2} LY", app.stats.total_distance),
    ];

    let exploration_text = Text::from(exploration.join("\n"));
    let exploration_widget = Paragraph::new(exploration_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Exploration Stats "),
        )
        .style(Style::default().fg(Color::Green));
    f.render_widget(exploration_widget, chunks[0]);

    // Ship systems
    let systems = vec![
        "Power:       [##########] 100%",
        "Shields:     [##########] 100%",
        "Energy:      [##########] 100%",
        "Sensors:     [##########] 100%",
        "Navigation:  [##########] 100%",
        "Holographics:[##########] 100%",
        "",
    ];

    let systems_text = Text::from(systems.join("\n"));
    let systems_widget = Paragraph::new(systems_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Ship Systems "),
        )
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(systems_widget, chunks[1]);

    // Deployment
    let deployment = vec![
        format!("Probes:       {:>6}", app.stats.probes_deployed),
        format!("Drones:       {:>6}", app.stats.drones_active),
        format!("Missions:     {:>6}", app.stats.habitable_planets),
        format!("Data Packets: {:>6}", app.stats.jumps_completed * 3),
        String::new(),
        String::new(),
        String::new(),
    ];

    let deployment_text = Text::from(deployment.join("\n"));
    let deployment_widget = Paragraph::new(deployment_text)
        .block(Block::default().borders(Borders::ALL).title(" Deployment "))
        .style(Style::default().fg(Color::Magenta));
    f.render_widget(deployment_widget, chunks[2]);
}

fn render_target(f: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(target) = &app.current_target {
        format!(
            "Type: {} | Name: {} | Distance: {:.2} LY | Coords: ({}, {}, {}) | Resolution: {}",
            target.target_type,
            target.name,
            target.distance,
            target.coordinates.0,
            target.coordinates.1,
            target.coordinates.2,
            target.resolution
        )
    } else {
        "Awaiting target assignment...".to_string()
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Current Target "),
        )
        .style(Style::default().fg(Color::Cyan))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_main_content(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // 3D visualization
    render_3d_view(f, chunks[0], app);

    // Telemetry
    render_telemetry(f, chunks[1], app);
}

fn render_3d_view(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    if app.phase == ExplorationPhase::Navigating || app.phase == ExplorationPhase::Analyzing {
        let (cx, cy, cz) = app.current_position;

        // Draw 3 z-layers
        for layer in 0..3 {
            let z_offset = layer as i32 - 1; // -1, 0, 1
            let z = cz + z_offset * 3;

            let layer_name = match z_offset {
                -1 => "FAR ",
                0 => "MID ",
                _ => "NEAR",
            };

            lines.push(format!("{} Layer [Z={}]:", layer_name, z));

            // Draw horizontal slice
            for dy in -3..=3 {
                let mut row = String::from("  ");
                for dx in -20..=20 {
                    let x = cx + dx;
                    let y = cy + dy;
                    let pos = (x, y, z);

                    if x == cx && y == cy && z == cz {
                        row.push('*'); // Ship
                    } else if app.obstacles.contains(&pos) {
                        row.push('#'); // Obstacle
                    } else if app.path.contains(&pos) {
                        row.push('.'); // Path
                    } else {
                        row.push(' '); // Empty
                    }
                }
                lines.push(row);
            }
            lines.push(String::new());
        }

        lines.push("Legend: * = Ship  # = Obstacle  . = Path".to_string());
    } else {
        lines.push("Awaiting navigation data...".to_string());
    }

    let text = Text::from(lines.join("\n"));
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 3D Holographic Display "),
        )
        .style(Style::default().fg(Color::Blue));

    f.render_widget(paragraph, area);
}

fn render_telemetry(f: &mut Frame, area: Rect, app: &App) {
    let inner_area = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Progress gauge
    let gauge_area = Rect {
        x: inner_area.x,
        y: inner_area.y,
        width: inner_area.width,
        height: 3,
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Navigation Progress ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent(app.navigation_progress as u16)
        .label(format!("{:.1}%", app.navigation_progress));

    f.render_widget(gauge, gauge_area);

    // Position info
    let info_area = Rect {
        x: inner_area.x,
        y: inner_area.y + 3,
        width: inner_area.width,
        height: inner_area.height - 3,
    };

    let (cx, cy, cz) = app.current_position;
    let (gx, gy, gz) = app.goal_position;

    let info = vec![
        format!("Position: ({:>8}, {:>8}, {:>8})", cx, cy, cz),
        format!("Goal:     ({:>8}, {:>8}, {:>8})", gx, gy, gz),
        format!("Waypoint: {} / {}", app.current_step, app.path.len()),
        format!("Velocity: 147x Light Speed"),
        format!("Status:   {:?}", app.phase),
    ];

    let text = Text::from(info.join("\n"));
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Telemetry "))
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(paragraph, info_area);
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let log_items: Vec<ListItem> = app
        .log_entries
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|entry| {
            let style = match entry.log_type {
                LogType::Info => Style::default().fg(Color::White),
                LogType::Success => Style::default().fg(Color::Green),
                LogType::Warning => Style::default().fg(Color::Yellow),
                LogType::Discovery => Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            };

            let icon = match entry.log_type {
                LogType::Info => "i",
                LogType::Success => "+",
                LogType::Warning => "!",
                LogType::Discovery => "*",
            };

            ListItem::new(format!("[{}] {} {}", entry.timestamp, icon, entry.message)).style(style)
        })
        .collect();

    let log_list = List::new(log_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Mission Log "),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(log_list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let text = " OctaIndex3D v0.2.0 | BCC Lattice Navigation | A* Pathfinding | Multi-Resolution Indexing ";
    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
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
    let star_names = ["Alpha Centauri", "Proxima", "Sirius", "Vega", "Arcturus"];
    let planet_prefixes = ["Kepler", "TRAPPIST", "Gliese", "Ross", "Wolf"];

    let cycle = (time_seed / 2) % 20;

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
    } else if cycle < 14 {
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
            ("Subspace Distortion", "Quantum fluctuations detected"),
            ("Nebula Formation", "Active stellar nursery"),
            ("Dark Matter", "Gravitational anomaly confirmed"),
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
    println!("║        MISSION DEACTIVATION - FINAL STATS         ║");
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
        "║  Obstacles:         {:>6}                        ║",
        stats.obstacles_avoided
    );
    println!(
        "║  Distance:      {:>10.2} LY                   ║",
        stats.total_distance
    );
    println!(
        "║  Jumps:             {:>6}                        ║",
        stats.jumps_completed
    );
    println!("║                                                    ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!("\nSafe travels, Captain. 🖖\n");
}
