//! OctaIndex3D Command-Line Interface
//!
//! This CLI provides:
//! - Interactive 3D octahedral maze game with Prim's algorithm
//! - Performance benchmarks
//! - Utility functions for spatial operations

use clap::{Parser, Subcommand};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

// Terminal control for single-key input
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{self, ClearType},
    ExecutableCommand,
};

// Re-use types from octaindex3d
use octaindex3d::{Index64, Result, Route64};

// ============================================================================
// Helper Functions
// ============================================================================

fn clear_screen() {
    use std::io::stdout;
    let mut stdout = stdout();
    let _ = stdout.execute(terminal::Clear(ClearType::All));
    let _ = stdout.execute(cursor::MoveTo(0, 0));
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn read_key_event() -> Result<KeyEvent> {
    loop {
        if let Event::Key(key_event) = event::read()? {
            return Ok(key_event);
        }
    }
}

fn wait_for_any_key() -> Result<()> {
    let _ = read_key_event()?;
    Ok(())
}

// ============================================================================
// CLI Structure
// ============================================================================

#[derive(Parser)]
#[command(name = "octaindex3d")]
#[command(about = "OctaIndex3D CLI - 3D Spatial Indexing & Interactive Maze Game", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Play an interactive 3D octahedral maze game
    Play {
        /// Maze size (default: 20x20x20)
        #[arg(short, long, default_value_t = 20)]
        size: u32,

        /// Random seed for maze generation
        #[arg(short = 'r', long)]
        seed: Option<u64>,

        /// Difficulty: easy (8x8x8), medium (20x20x20), hard (40x40x40)
        #[arg(short, long, value_parser = ["easy", "medium", "hard"])]
        difficulty: Option<String>,

        /// Game mode: astar (race A* on nodes explored) or bloodhound (survival).
        /// If omitted, an in-game menu lets you choose.
        #[arg(short, long, value_parser = ["astar", "bloodhound"])]
        mode: Option<String>,
    },

    /// View competitive statistics against A*
    Stats,

    /// Reset competitive statistics
    ResetStats,

    /// Run performance benchmarks
    Benchmark {
        /// Number of operations to benchmark
        #[arg(short, long, default_value_t = 100000)]
        iterations: usize,
    },

    /// Utility functions
    Utils {
        #[command(subcommand)]
        util_command: UtilCommands,
    },
}

#[derive(Subcommand)]
enum UtilCommands {
    /// Encode 3D coordinates to Index64
    Encode {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Z coordinate
        z: i32,
    },

    /// Decode Index64 to 3D coordinates
    Decode {
        /// Index64 value as hex or decimal
        value: String,
    },

    /// Calculate distance between two points
    Distance {
        /// First point (x,y,z)
        #[arg(value_parser = parse_coord)]
        from: (i32, i32, i32),
        /// Second point (x,y,z)
        #[arg(value_parser = parse_coord)]
        to: (i32, i32, i32),
    },

    /// Get 14 BCC neighbors for a coordinate
    Neighbors {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Z coordinate
        z: i32,
    },
}

fn parse_coord(s: &str) -> std::result::Result<(i32, i32, i32), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return Err("Coordinate must be in format x,y,z".to_string());
    }

    let x = parts[0]
        .trim()
        .parse::<i32>()
        .map_err(|_| "Invalid x coordinate")?;
    let y = parts[1]
        .trim()
        .parse::<i32>()
        .map_err(|_| "Invalid y coordinate")?;
    let z = parts[2]
        .trim()
        .parse::<i32>()
        .map_err(|_| "Invalid z coordinate")?;

    Ok((x, y, z))
}

// ============================================================================
// Competitive Statistics Tracking
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct CompetitiveStats {
    wins: u32,
    ties: u32,
    losses: u32,
    total_games: u32,
    best_efficiency: f64,
    worst_efficiency: f64,
    total_efficiency: f64,
}

impl Default for CompetitiveStats {
    fn default() -> Self {
        Self {
            wins: 0,
            ties: 0,
            losses: 0,
            total_games: 0,
            best_efficiency: 0.0,
            worst_efficiency: 100.0,
            total_efficiency: 0.0,
        }
    }
}

impl CompetitiveStats {
    fn stats_file() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".octaindex3d_stats.json")
    }

    fn load() -> Self {
        if let Ok(content) = fs::read_to_string(Self::stats_file()) {
            if let Ok(stats) = serde_json::from_str(&content) {
                return stats;
            }
        }
        Self::default()
    }

    fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(Self::stats_file(), json)
    }

    fn update(&mut self, player_moves: usize, optimal_moves: usize) {
        self.total_games += 1;
        let efficiency = (optimal_moves as f64 / player_moves as f64) * 100.0;

        #[allow(clippy::comparison_chain)]
        if player_moves < optimal_moves {
            self.wins += 1;
        } else if player_moves == optimal_moves {
            self.ties += 1;
        } else {
            self.losses += 1;
        }

        self.best_efficiency = self.best_efficiency.max(efficiency);
        self.worst_efficiency = self.worst_efficiency.min(efficiency);
        self.total_efficiency += efficiency;

        let _ = self.save();
    }

    fn win_rate(&self) -> f64 {
        if self.total_games == 0 {
            0.0
        } else {
            ((self.wins + self.ties) as f64 / self.total_games as f64) * 100.0
        }
    }

    fn average_efficiency(&self) -> f64 {
        if self.total_games == 0 {
            0.0
        } else {
            self.total_efficiency / self.total_games as f64
        }
    }
}

// ============================================================================
// Bloodhound Mode Statistics (persisted separately from A* stats)
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
struct BloodhoundStats {
    runs: u32,
    deaths: u32,
    best_level: u32,
    total_screams: u32,
}

impl BloodhoundStats {
    fn stats_file() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".octaindex3d_bloodhound_stats.json")
    }

    fn load() -> Self {
        if let Ok(content) = fs::read_to_string(Self::stats_file()) {
            if let Ok(stats) = serde_json::from_str(&content) {
                return stats;
            }
        }
        Self::default()
    }

    fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(Self::stats_file(), json)
    }

    /// Record the outcome of one full bloodhound run.
    fn record_run(&mut self, levels_cleared: u32, screams: u32, caught: bool) {
        self.runs += 1;
        if caught {
            self.deaths += 1;
        }
        self.best_level = self.best_level.max(levels_cleared);
        self.total_screams += screams;
        let _ = self.save();
    }
}

// ============================================================================
// Maze Generation using Prim's Algorithm
// ============================================================================

type Coord = (i32, i32, i32);

/// Check if a coordinate is valid in BCC (all even or all odd)
fn is_valid_bcc(c: Coord) -> bool {
    let parity_x = c.0.abs() % 2;
    let parity_y = c.1.abs() % 2;
    let parity_z = c.2.abs() % 2;
    parity_x == parity_y && parity_y == parity_z
}

/// BCC lattice neighbors (14-connectivity)
const BCC_NEIGHBORS: &[(i32, i32, i32)] = &[
    // Body diagonals
    (1, 1, 1),
    (1, 1, -1),
    (1, -1, 1),
    (1, -1, -1),
    (-1, 1, 1),
    (-1, 1, -1),
    (-1, -1, 1),
    (-1, -1, -1),
    // Axial double steps
    (2, 0, 0),
    (-2, 0, 0),
    (0, 2, 0),
    (0, -2, 0),
    (0, 0, 2),
    (0, 0, -2),
];

/// Convert 3D coordinate to linear index
#[allow(dead_code)]
fn coord_to_index(extent: (u32, u32, u32), c: Coord) -> Option<u32> {
    if c.0 < 0 || c.1 < 0 || c.2 < 0 {
        return None;
    }
    if c.0 >= extent.0 as i32 || c.1 >= extent.1 as i32 || c.2 >= extent.2 as i32 {
        return None;
    }
    Some(c.0 as u32 * extent.1 * extent.2 + c.1 as u32 * extent.2 + c.2 as u32)
}

/// Convert linear index to 3D coordinate
#[allow(dead_code)]
fn index_to_coord(extent: (u32, u32, u32), idx: u32) -> Coord {
    let z = idx % extent.2;
    let y = (idx / extent.2) % extent.1;
    let x = idx / (extent.1 * extent.2);
    (x as i32, y as i32, z as i32)
}

/// Get valid BCC neighbors for a coordinate
fn get_neighbors(extent: (u32, u32, u32), coord: Coord) -> Vec<Coord> {
    let mut neighbors = Vec::with_capacity(14);
    for &(dx, dy, dz) in BCC_NEIGHBORS {
        let nx = coord.0 + dx;
        let ny = coord.1 + dy;
        let nz = coord.2 + dz;
        if nx >= 0
            && ny >= 0
            && nz >= 0
            && nx < extent.0 as i32
            && ny < extent.1 as i32
            && nz < extent.2 as i32
        {
            neighbors.push((nx, ny, nz));
        }
    }
    neighbors
}

/// Maze structure with BCC lattice
struct Maze {
    #[allow(dead_code)]
    extent: (u32, u32, u32),
    carved: HashSet<Coord>,
    connections: HashMap<Coord, Vec<Coord>>,
    start: Coord,
    goal: Coord,
}

impl Maze {
    /// Generate a new maze using randomized Prim's algorithm
    fn generate(extent: (u32, u32, u32), seed: u64) -> Self {
        use rand::rngs::StdRng;
        use rand::{RngExt, SeedableRng};

        let mut rng = StdRng::seed_from_u64(seed);
        let mut carved = HashSet::new();
        let mut connections: HashMap<Coord, Vec<Coord>> = HashMap::new();
        let mut frontier = Vec::new();
        let mut frontier_set = HashSet::new();

        // Start at origin (must be valid BCC)
        let start = (0, 0, 0);
        assert!(is_valid_bcc(start), "Start must be valid BCC");

        carved.insert(start);

        // Add neighbors to frontier
        for neighbor in get_neighbors(extent, start) {
            if is_valid_bcc(neighbor) && !frontier_set.contains(&neighbor) {
                frontier.push(neighbor);
                frontier_set.insert(neighbor);
            }
        }

        // Prim's algorithm
        while !frontier.is_empty() {
            // Pick random frontier cell
            let idx = rng.random_range(0..frontier.len());
            let current = frontier.swap_remove(idx);
            frontier_set.remove(&current);

            // Find carved neighbors
            let neighbors = get_neighbors(extent, current);
            let carved_neighbors: Vec<Coord> = neighbors
                .iter()
                .filter(|&&n| is_valid_bcc(n) && carved.contains(&n))
                .copied()
                .collect();

            if !carved_neighbors.is_empty() {
                // Connect to random carved neighbor
                let parent = carved_neighbors[rng.random_range(0..carved_neighbors.len())];
                carved.insert(current);

                // Create bidirectional connection
                connections.entry(current).or_default().push(parent);
                connections.entry(parent).or_default().push(current);

                // Add uncarved neighbors to frontier
                for neighbor in neighbors {
                    if is_valid_bcc(neighbor)
                        && !carved.contains(&neighbor)
                        && !frontier_set.contains(&neighbor)
                    {
                        frontier.push(neighbor);
                        frontier_set.insert(neighbor);
                    }
                }
            }
        }

        // Set goal to opposite corner (must be valid BCC)
        let mut goal = (
            (extent.0 - 1) as i32,
            (extent.1 - 1) as i32,
            (extent.2 - 1) as i32,
        );
        if !is_valid_bcc(goal) {
            goal = (goal.0 - 1, goal.1 - 1, goal.2 - 1);
        }

        Self {
            extent,
            carved,
            connections,
            start,
            goal,
        }
    }

    /// Check if a coordinate is carved (passable)
    #[allow(dead_code)]
    fn is_carved(&self, coord: Coord) -> bool {
        self.carved.contains(&coord)
    }

    /// Get connected neighbors for a coordinate
    fn get_connected_neighbors(&self, coord: Coord) -> Vec<Coord> {
        self.connections.get(&coord).cloned().unwrap_or_default()
    }

    /// Check if two coordinates are connected
    fn are_connected(&self, from: Coord, to: Coord) -> bool {
        if let Some(neighbors) = self.connections.get(&from) {
            neighbors.contains(&to)
        } else {
            false
        }
    }
}

// ============================================================================
// A* Pathfinding
// ============================================================================

#[derive(Clone, Eq, PartialEq)]
struct AstarNode {
    coord: Coord,
    g_cost: u32,
    f_cost: u32,
}

impl Ord for AstarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for AstarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn heuristic(from: Coord, to: Coord) -> u32 {
    let dx = (from.0 - to.0).abs() as f64;
    let dy = (from.1 - to.1).abs() as f64;
    let dz = (from.2 - to.2).abs() as f64;
    (dx * dx + dy * dy + dz * dz).sqrt() as u32
}

fn astar_pathfind(maze: &Maze, start: Coord, goal: Coord) -> Option<(Vec<Coord>, usize)> {
    let mut open_set = BinaryHeap::new();
    let mut came_from: HashMap<Coord, Coord> = HashMap::new();
    let mut g_score: HashMap<Coord, u32> = HashMap::new();
    let mut closed_set: HashSet<Coord> = HashSet::new();

    g_score.insert(start, 0);
    open_set.push(AstarNode {
        coord: start,
        g_cost: 0,
        f_cost: heuristic(start, goal),
    });

    while let Some(current) = open_set.pop() {
        if current.coord == goal {
            // Reconstruct path
            let mut path = vec![goal];
            let mut current_coord = goal;
            while let Some(&prev) = came_from.get(&current_coord) {
                path.push(prev);
                current_coord = prev;
            }
            path.reverse();
            // Include the goal node in the exploration count for fair comparison
            return Some((path, closed_set.len() + 1));
        }

        if closed_set.contains(&current.coord) {
            continue;
        }
        closed_set.insert(current.coord);

        // Explore neighbors (only connected ones in the maze)
        for neighbor in maze.get_connected_neighbors(current.coord) {
            if closed_set.contains(&neighbor) {
                continue;
            }

            let tentative_g = g_score
                .get(&current.coord)
                .unwrap_or(&u32::MAX)
                .saturating_add(1);

            if tentative_g < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                came_from.insert(neighbor, current.coord);
                g_score.insert(neighbor, tentative_g);

                let f = tentative_g.saturating_add(heuristic(neighbor, goal));
                open_set.push(AstarNode {
                    coord: neighbor,
                    g_cost: tentative_g,
                    f_cost: f,
                });
            }
        }
    }

    None
}

// ============================================================================
// Interactive Game
// ============================================================================

struct GameState {
    maze: Maze,
    current_pos: Coord,
    move_history: Vec<Coord>,
    visited: HashSet<Coord>,
    start_time: Instant,
    level: u32,
    hints_used: u32,
}

impl GameState {
    fn new(maze: Maze, level: u32) -> Self {
        let start_pos = maze.start;
        let mut visited = HashSet::new();
        visited.insert(start_pos);

        Self {
            maze,
            current_pos: start_pos,
            move_history: vec![start_pos],
            visited,
            start_time: Instant::now(),
            level,
            hints_used: 0,
        }
    }

    fn make_move(&mut self, key: char) -> std::result::Result<bool, String> {
        // Map single character to direction
        let next_pos = match key {
            'n' => (
                self.current_pos.0,
                self.current_pos.1 + 2,
                self.current_pos.2,
            ),
            's' => (
                self.current_pos.0,
                self.current_pos.1 - 2,
                self.current_pos.2,
            ),
            'e' => (
                self.current_pos.0 + 2,
                self.current_pos.1,
                self.current_pos.2,
            ),
            'w' => (
                self.current_pos.0 - 2,
                self.current_pos.1,
                self.current_pos.2,
            ),
            'u' => (
                self.current_pos.0,
                self.current_pos.1,
                self.current_pos.2 + 2,
            ),
            'd' => (
                self.current_pos.0,
                self.current_pos.1,
                self.current_pos.2 - 2,
            ),
            // Diagonal movements - numbered 1-8 for easier single-key input
            '1' => (
                self.current_pos.0 + 1,
                self.current_pos.1 + 1,
                self.current_pos.2 + 1,
            ), // neu
            '2' => (
                self.current_pos.0 + 1,
                self.current_pos.1 + 1,
                self.current_pos.2 - 1,
            ), // ned
            '3' => (
                self.current_pos.0 - 1,
                self.current_pos.1 + 1,
                self.current_pos.2 + 1,
            ), // nwu
            '4' => (
                self.current_pos.0 - 1,
                self.current_pos.1 + 1,
                self.current_pos.2 - 1,
            ), // nwd
            '5' => (
                self.current_pos.0 + 1,
                self.current_pos.1 - 1,
                self.current_pos.2 + 1,
            ), // seu
            '6' => (
                self.current_pos.0 + 1,
                self.current_pos.1 - 1,
                self.current_pos.2 - 1,
            ), // sed
            '7' => (
                self.current_pos.0 - 1,
                self.current_pos.1 - 1,
                self.current_pos.2 + 1,
            ), // swu
            '8' => (
                self.current_pos.0 - 1,
                self.current_pos.1 - 1,
                self.current_pos.2 - 1,
            ), // swd
            _ => return Err("Invalid direction".to_string()),
        };

        // Check if move is valid (connected in maze)
        if !self.maze.are_connected(self.current_pos, next_pos) {
            return Err("Cannot move in that direction (wall)".to_string());
        }

        self.current_pos = next_pos;
        self.move_history.push(next_pos);
        self.visited.insert(next_pos);

        Ok(self.current_pos == self.maze.goal)
    }

    fn get_available_directions(&self) -> Vec<(char, String, bool)> {
        let connected = self.maze.get_connected_neighbors(self.current_pos);
        let mut directions = Vec::new();

        for neighbor in connected {
            let dx = neighbor.0 - self.current_pos.0;
            let dy = neighbor.1 - self.current_pos.1;
            let dz = neighbor.2 - self.current_pos.2;
            let visited = self.visited.contains(&neighbor);

            let (key, name) = match (dx, dy, dz) {
                (0, 2, 0) => ('n', "North (y+2)"),
                (0, -2, 0) => ('s', "South (y-2)"),
                (2, 0, 0) => ('e', "East (x+2)"),
                (-2, 0, 0) => ('w', "West (x-2)"),
                (0, 0, 2) => ('u', "Up (z+2)"),
                (0, 0, -2) => ('d', "Down (z-2)"),
                (1, 1, 1) => ('1', "NE-Up (+1,+1,+1)"),
                (1, 1, -1) => ('2', "NE-Down (+1,+1,-1)"),
                (-1, 1, 1) => ('3', "NW-Up (-1,+1,+1)"),
                (-1, 1, -1) => ('4', "NW-Down (-1,+1,-1)"),
                (1, -1, 1) => ('5', "SE-Up (+1,-1,+1)"),
                (1, -1, -1) => ('6', "SE-Down (+1,-1,-1)"),
                (-1, -1, 1) => ('7', "SW-Up (-1,-1,+1)"),
                (-1, -1, -1) => ('8', "SW-Down (-1,-1,-1)"),
                _ => continue,
            };

            directions.push((key, name.to_string(), visited));
        }

        directions
    }

    fn display_status(&self) {
        use std::io::{stdout, Write};

        print!("\r\n🎮 3D OCTAHEDRAL MAZE GAME - Level {}\r\n", self.level);
        print!(
            "Position: {:?}  →  Goal: {:?}\r\n",
            self.current_pos, self.maze.goal
        );
        print!(
            "Moves: {}  |  Time: {:.1}s  |  Visited: {}/{}\r\n",
            self.move_history.len() - 1,
            self.start_time.elapsed().as_secs_f64(),
            self.visited.len(),
            self.maze.carved.len()
        );

        let distance = heuristic(self.current_pos, self.maze.goal);
        print!("Distance to goal: {} (straight line)\r\n\r\n", distance);

        print!("Available Directions (press key to move):\r\n");
        let directions = self.get_available_directions();
        for (key, name, visited) in directions {
            let status = if visited { "●" } else { "○" }; // ● = visited, ○ = unvisited
            print!("  [{:>1}] {} {}\r\n", key, status, name);
        }

        print!("\r\nCommands: [h]int | [q]uit\r\n");
        let _ = stdout().flush();
    }
}

fn play_game(custom_size: Option<u32>, seed: u64) -> Result<()> {
    use std::io::{stdout, Write};

    let _raw_mode_guard = RawModeGuard::new()?;

    // Load competitive stats
    let mut stats = CompetitiveStats::load();

    // Progressive levels: start at 2x2x2, then 3x3x3, 4x4x4, etc.
    let mut current_level = 1u32;
    let mut current_size = custom_size.unwrap_or(2); // Start at 2x2x2 if no custom size
    let use_progressive = custom_size.is_none();

    'game_loop: loop {
        // Clear screen and show level intro
        clear_screen();
        print!(
            "\r\n🎮 LEVEL {} - Generating {}x{}x{} Maze...\r\n",
            current_level, current_size, current_size, current_size
        );
        let _ = stdout().flush();

        let extent = (current_size, current_size, current_size);
        let level_seed = seed.wrapping_add(current_level as u64); // Different seed per level
        let maze = Maze::generate(extent, level_seed);

        print!("✓ Maze generated!\r\n");
        print!("✓ Carved nodes: {}\r\n", maze.carved.len());
        print!("✓ Start: {:?} → Goal: {:?}\r\n", maze.start, maze.goal);
        print!("\r\nPress any key to begin...\r\n");
        let _ = stdout().flush();

        // Wait for key to start
        wait_for_any_key()?;

        let mut game = GameState::new(maze, current_level);

        'level_loop: loop {
            // Clear screen and display game state
            clear_screen();
            game.display_status();

            // Read single key
            let KeyEvent { code, .. } = read_key_event()?;
            match code {
                KeyCode::Char('q') => {
                    clear_screen();
                    print!("\r\nThanks for playing!\r\n");
                    print!("Final stats:\r\n");
                    print!("  Level reached: {}\r\n", current_level);
                    print!(
                        "  Total time: {:.1}s\r\n",
                        game.start_time.elapsed().as_secs_f64()
                    );
                    let _ = stdout().flush();
                    break 'game_loop;
                }
                KeyCode::Char('h') => {
                    // Show hint and increment counter
                    game.hints_used += 1;
                    clear_screen();
                    game.display_status();
                    print!("\r\n💡 Computing optimal path...\r\n");
                    if let Some((path, nodes_visited)) =
                        astar_pathfind(&game.maze, game.current_pos, game.maze.goal)
                    {
                        print!("Optimal path has {} moves\r\n", path.len() - 1);
                        print!("A* nodes explored: {}\r\n", nodes_visited);
                        if path.len() > 1 {
                            let next = path[1];
                            let dx = next.0 - game.current_pos.0;
                            let dy = next.1 - game.current_pos.1;
                            let dz = next.2 - game.current_pos.2;
                            let hint_key = match (dx, dy, dz) {
                                (0, 2, 0) => 'n',
                                (0, -2, 0) => 's',
                                (2, 0, 0) => 'e',
                                (-2, 0, 0) => 'w',
                                (0, 0, 2) => 'u',
                                (0, 0, -2) => 'd',
                                (1, 1, 1) => '1',
                                (1, 1, -1) => '2',
                                (-1, 1, 1) => '3',
                                (-1, 1, -1) => '4',
                                (1, -1, 1) => '5',
                                (1, -1, -1) => '6',
                                (-1, -1, 1) => '7',
                                (-1, -1, -1) => '8',
                                _ => '?',
                            };
                            print!("Next optimal move: press [{}]\r\n", hint_key);
                        }
                    } else {
                        print!("No path found!\r\n");
                    }
                    print!("\r\nPress any key to continue...\r\n");
                    let _ = stdout().flush();
                    wait_for_any_key()?;
                }
                KeyCode::Char(c) => {
                    match game.make_move(c) {
                        Ok(reached_goal) => {
                            if reached_goal {
                                // Level complete!
                                clear_screen();
                                let elapsed = game.start_time.elapsed().as_secs_f64();
                                print!("\r\n🎉 LEVEL {} COMPLETE!\r\n", current_level);
                                print!("═══════════════════════════════════════\r\n");
                                print!("Total moves: {}\r\n", game.move_history.len() - 1);
                                print!("Time taken: {:.1}s\r\n", elapsed);
                                print!(
                                    "Nodes visited: {}/{}\r\n",
                                    game.visited.len(),
                                    game.maze.carved.len()
                                );
                                print!("Hints used: {}\r\n", game.hints_used);

                                // Run A* for comparison
                                print!("\r\n🤖 Computing optimal solution...\r\n");
                                if let Some((optimal_path, astar_nodes_visited)) =
                                    astar_pathfind(&game.maze, game.maze.start, game.maze.goal)
                                {
                                    let optimal_moves = optimal_path.len() - 1;
                                    let player_moves = game.move_history.len() - 1;
                                    let player_visited = game.visited.len();

                                    print!("\r\n📊 Performance Comparison:\r\n");
                                    print!("───────────────────────────────────────\r\n");

                                    // Path efficiency comparison (still showing moves for reference)
                                    let move_efficiency =
                                        (optimal_moves as f64) / (player_moves as f64) * 100.0;
                                    let move_diff = player_moves as i32 - optimal_moves as i32;

                                    if optimal_moves == player_moves {
                                        print!("🎯 Optimal path was {} moves, your path was {} moves (100% efficiency - PERFECT!)\r\n",
                                                optimal_moves, player_moves);
                                    } else {
                                        let pct_more =
                                            ((player_moves as f64 / optimal_moves as f64) - 1.0)
                                                * 100.0;
                                        print!("🎯 Optimal path was {} moves, your path was {} moves ({:.0}% efficiency)\r\n",
                                                optimal_moves, player_moves, move_efficiency);
                                        print!("   └─ You used {} extra move{} ({:.0}% more than optimal)\r\n",
                                                move_diff, if move_diff == 1 { "" } else { "s" }, pct_more);
                                    }

                                    // Nodes explored efficiency (primary metric)
                                    let nodes_efficiency = (astar_nodes_visited as f64)
                                        / (player_visited as f64)
                                        * 100.0;

                                    // Exploration efficiency comparison
                                    print!("\r\n🔍 Exploration Comparison:\r\n");
                                    #[allow(clippy::comparison_chain)]
                                    if player_visited < astar_nodes_visited {
                                        let pct_fewer = ((astar_nodes_visited as f64
                                            / player_visited as f64)
                                            - 1.0)
                                            * 100.0;
                                        print!(
                                            "   A* explored {} nodes, you explored {} nodes\r\n",
                                            astar_nodes_visited, player_visited
                                        );
                                        print!("   └─ You explored {} fewer nodes ({:.0}% fewer than A*)!\r\n",
                                                astar_nodes_visited - player_visited, pct_fewer);
                                    } else if player_visited == astar_nodes_visited {
                                        print!("   A* explored {} nodes, you explored {} nodes (same as A*!)\r\n",
                                                astar_nodes_visited, player_visited);
                                    } else {
                                        let pct_more = ((player_visited as f64
                                            / astar_nodes_visited as f64)
                                            - 1.0)
                                            * 100.0;
                                        print!(
                                            "   A* explored {} nodes, you explored {} nodes\r\n",
                                            astar_nodes_visited, player_visited
                                        );
                                        print!("   └─ You explored {} more nodes ({:.0}% more than A*)\r\n",
                                                player_visited - astar_nodes_visited, pct_more);
                                    }

                                    // Competitive result (based on nodes explored)
                                    print!("\r\n🎮 COMPETITIVE RESULT (Fewest Nodes Explored Wins):\r\n");
                                    print!("───────────────────────────────────────\r\n");
                                    #[allow(clippy::comparison_chain)]
                                    if player_visited < astar_nodes_visited {
                                        let nodes_saved = astar_nodes_visited - player_visited;
                                        print!(
                                            "🏆 YOU WIN! You explored {} fewer nodes than A*!\r\n",
                                            nodes_saved
                                        );
                                        stats.update(player_visited, astar_nodes_visited);
                                    } else if player_visited == astar_nodes_visited {
                                        print!("🤝 TIE! You explored the same number of nodes as A*!\r\n");
                                        stats.update(player_visited, astar_nodes_visited);
                                    } else {
                                        let extra_nodes = player_visited - astar_nodes_visited;
                                        print!(
                                            "🤖 A* WINS. You explored {} more nodes than A*.\r\n",
                                            extra_nodes
                                        );
                                        stats.update(player_visited, astar_nodes_visited);
                                    }

                                    // Display updated stats
                                    print!("\r\n📈 YOUR LIFETIME STATS:\r\n");
                                    print!(
                                        "   Wins: {} | Ties: {} | Losses: {}\r\n",
                                        stats.wins, stats.ties, stats.losses
                                    );
                                    print!("   Win Rate: {:.1}%\r\n", stats.win_rate());
                                    print!(
                                        "   Avg Efficiency: {:.1}%\r\n",
                                        stats.average_efficiency()
                                    );

                                    if player_visited == astar_nodes_visited {
                                        print!("\r\n🏆 PERFECT! You explored exactly as many nodes as A*!\r\n");
                                    } else if nodes_efficiency >= 95.0 {
                                        print!("\r\n🏆 Nearly perfect! You explored only slightly more nodes than A*!\r\n");
                                    } else if nodes_efficiency >= 80.0 {
                                        print!("\r\n⭐ Great job! You were efficient in your exploration!\r\n");
                                    } else if nodes_efficiency >= 60.0 {
                                        print!("\r\n👍 Good effort! There's room for improvement in your exploration.\r\n");
                                    } else {
                                        print!("\r\n💪 Keep practicing! Try the hint command to explore more efficiently.\r\n");
                                    }
                                }

                                if use_progressive {
                                    print!("\r\n🎯 Ready for Level {}?\r\n", current_level + 1);
                                    print!("Press [Enter] to continue or [q] to quit...\r\n");
                                    let _ = stdout().flush();

                                    // Wait for decision
                                    loop {
                                        let KeyEvent { code, .. } = read_key_event()?;
                                        match code {
                                            KeyCode::Enter => {
                                                current_level += 1;
                                                current_size += 1;
                                                break 'level_loop; // Go to next level
                                            }
                                            KeyCode::Char('q') => {
                                                break 'game_loop; // Quit game
                                            }
                                            _ => {}
                                        }
                                    }
                                } else {
                                    print!("\r\nPress any key to exit...\r\n");
                                    let _ = stdout().flush();
                                    wait_for_any_key()?;
                                    break 'game_loop;
                                }
                            }
                        }
                        Err(_e) => {
                            // Invalid move - just redraw (error will be silent in single-key mode)
                        }
                    }
                }
                _ => {} // Ignore other keys
            }
        }
    }

    println!();
    Ok(())
}

// ============================================================================
// Bloodhound Survival Mode
// ============================================================================

/// Map a movement key to its BCC coordinate delta.
fn dir_delta(key: char) -> Option<Coord> {
    Some(match key {
        'n' => (0, 2, 0),
        's' => (0, -2, 0),
        'e' => (2, 0, 0),
        'w' => (-2, 0, 0),
        'u' => (0, 0, 2),
        'd' => (0, 0, -2),
        '1' => (1, 1, 1),
        '2' => (1, 1, -1),
        '3' => (-1, 1, 1),
        '4' => (-1, 1, -1),
        '5' => (1, -1, 1),
        '6' => (1, -1, -1),
        '7' => (-1, -1, 1),
        '8' => (-1, -1, -1),
        _ => return None,
    })
}

/// Map a BCC coordinate delta to its movement key and label.
fn delta_to_dir(d: Coord) -> Option<(char, &'static str)> {
    Some(match d {
        (0, 2, 0) => ('n', "North (y+2)"),
        (0, -2, 0) => ('s', "South (y-2)"),
        (2, 0, 0) => ('e', "East (x+2)"),
        (-2, 0, 0) => ('w', "West (x-2)"),
        (0, 0, 2) => ('u', "Up (z+2)"),
        (0, 0, -2) => ('d', "Down (z-2)"),
        (1, 1, 1) => ('1', "NE-Up (+1,+1,+1)"),
        (1, 1, -1) => ('2', "NE-Down (+1,+1,-1)"),
        (-1, 1, 1) => ('3', "NW-Up (-1,+1,+1)"),
        (-1, 1, -1) => ('4', "NW-Down (-1,+1,-1)"),
        (1, -1, 1) => ('5', "SE-Up (+1,-1,+1)"),
        (1, -1, -1) => ('6', "SE-Down (+1,-1,-1)"),
        (-1, -1, 1) => ('7', "SW-Up (-1,-1,+1)"),
        (-1, -1, -1) => ('8', "SW-Down (-1,-1,-1)"),
        _ => return None,
    })
}

/// Squared Euclidean distance between two coordinates (exact, no truncation).
fn dist_sq(a: Coord, b: Coord) -> i64 {
    let dx = (a.0 - b.0) as i64;
    let dy = (a.1 - b.1) as i64;
    let dz = (a.2 - b.2) as i64;
    dx * dx + dy * dy + dz * dz
}

/// Roll spikes into ~`prob_percent`% of carved cells (start and goal excluded).
fn generate_spikes(maze: &Maze, seed: u64, prob_percent: u32) -> HashSet<Coord> {
    use rand::rngs::StdRng;
    use rand::{RngExt, SeedableRng};

    let mut rng = StdRng::seed_from_u64(seed);
    let mut spikes = HashSet::new();
    for &cell in &maze.carved {
        if cell == maze.start || cell == maze.goal {
            continue;
        }
        if rng.random_range(0..100) < prob_percent {
            spikes.insert(cell);
        }
    }
    spikes
}

/// Pick the carved cell nearest the bounding-box corner that is furthest
/// (max Euclidean distance) from `from`. Used to release a new bloodhound.
fn furthest_corner_spawn(maze: &Maze, from: Coord) -> Coord {
    let (ex, ey, ez) = maze.extent;
    let xs = [0i32, ex as i32 - 1];
    let ys = [0i32, ey as i32 - 1];
    let zs = [0i32, ez as i32 - 1];

    let mut best_corner = from;
    let mut best_corner_d = -1i64;
    for &x in &xs {
        for &y in &ys {
            for &z in &zs {
                let corner = (x, y, z);
                let d = dist_sq(corner, from);
                if d > best_corner_d {
                    best_corner_d = d;
                    best_corner = corner;
                }
            }
        }
    }

    // Snap to the nearest actually-carved cell to that corner.
    let mut spawn = maze.start;
    let mut best_d = i64::MAX;
    for &cell in &maze.carved {
        let d = dist_sq(cell, best_corner);
        if d < best_d {
            best_d = d;
            spawn = cell;
        }
    }
    spawn
}

/// A single pursuing bloodhound. It chases the freshest evidence it knows of,
/// and roams to search when its trail goes cold.
struct Bloodhound {
    pos: Coord,
    /// Cell it is currently walking toward (a scent/blood location, or a search step).
    target: Coord,
    /// Last-known player location (scent source) used to bias the search.
    anchor: Coord,
    /// Turn number of the freshest scream/blood evidence this hound is tracking.
    evidence_turn: u64,
    /// Per-cell visit counts, so the search spreads out instead of oscillating.
    visits: HashMap<Coord, u32>,
}

/// Result of a player's attempted move in bloodhound mode.
enum MoveOutcome {
    /// Wall or invalid key — no turn was taken.
    Blocked,
    /// Moved into an empty cell.
    Moved,
    /// Moved onto the goal — level cleared.
    ReachedGoal,
    /// Walked straight into a bloodhound — the player is killed.
    IntoBloodhound,
}

struct BloodhoundGame {
    maze: Maze,
    current_pos: Coord,
    turn: u64,
    spikes: HashSet<Coord>,
    /// Cell -> creation turn (age) of the blood trace left there.
    blood: HashMap<Coord, u64>,
    hounds: Vec<Bloodhound>,
    bleed_turns_remaining: u8,
    move_history: Vec<Coord>,
    visited: HashSet<Coord>,
    start_time: Instant,
    level: u32,
    hints_used: u32,
    screams: u32,
}

impl BloodhoundGame {
    fn new(maze: Maze, spikes: HashSet<Coord>, level: u32) -> Self {
        let start_pos = maze.start;
        let mut visited = HashSet::new();
        visited.insert(start_pos);
        Self {
            maze,
            current_pos: start_pos,
            turn: 0,
            spikes,
            blood: HashMap::new(),
            hounds: Vec::new(),
            bleed_turns_remaining: 0,
            move_history: vec![start_pos],
            visited,
            start_time: Instant::now(),
            level,
            hints_used: 0,
            screams: 0,
        }
    }

    fn hound_at(&self, c: Coord) -> bool {
        self.hounds.iter().any(|h| h.pos == c)
    }

    /// Number of reachable (no-wall) neighbor cells that contain a spike.
    fn spike_detector_count(&self) -> usize {
        self.maze
            .get_connected_neighbors(self.current_pos)
            .into_iter()
            .filter(|n| self.spikes.contains(n))
            .count()
    }

    fn nearest_hound_distance(&self) -> Option<f64> {
        self.hounds
            .iter()
            .map(|h| (dist_sq(h.pos, self.current_pos) as f64).sqrt())
            .fold(None, |acc, d| match acc {
                Some(m) if m <= d => Some(m),
                _ => Some(d),
            })
    }

    fn is_caught(&self) -> bool {
        self.hound_at(self.current_pos)
    }

    /// True if any bloodhound is one cell from the player with no wall between
    /// them (i.e. it could step straight onto the player).
    fn any_hound_can_lunge(&self) -> bool {
        self.hounds
            .iter()
            .any(|h| self.maze.are_connected(h.pos, self.current_pos))
    }

    /// Resolve the bloodhounds' turn. A hound one open cell away lunges and
    /// catches the player; otherwise every hound advances one step (and may
    /// step onto the player). Returns true if the player is caught.
    fn resolve_hounds(&mut self) -> bool {
        // Lunge: the player moved first this turn, so a hound still one open
        // cell away means they failed to break contact — it pounces.
        if self.is_caught() || self.any_hound_can_lunge() {
            return true;
        }
        self.move_bloodhounds();
        self.is_caught()
    }

    /// True if the player has at least one legal move (a connected neighbor
    /// not occupied by a bloodhound). When false, the player is blocked in.
    fn has_available_move(&self) -> bool {
        self.maze
            .get_connected_neighbors(self.current_pos)
            .into_iter()
            .any(|n| !self.hound_at(n))
    }

    /// Attempt a move. A wall or invalid key is `Blocked` (no turn taken);
    /// walking into a bloodhound's cell is fatal (`IntoBloodhound`).
    fn make_move(&mut self, key: char) -> MoveOutcome {
        let Some(delta) = dir_delta(key) else {
            return MoveOutcome::Blocked;
        };
        let next_pos = (
            self.current_pos.0 + delta.0,
            self.current_pos.1 + delta.1,
            self.current_pos.2 + delta.2,
        );

        if !self.maze.are_connected(self.current_pos, next_pos) {
            return MoveOutcome::Blocked;
        }
        if self.hound_at(next_pos) {
            // Run straight into the bloodhound's jaws.
            return MoveOutcome::IntoBloodhound;
        }

        self.current_pos = next_pos;
        self.turn += 1;
        self.move_history.push(next_pos);
        self.visited.insert(next_pos);

        // Leave a blood trace while bleeding.
        if self.bleed_turns_remaining > 0 {
            self.blood.insert(next_pos, self.turn);
            self.bleed_turns_remaining -= 1;
        }

        // Step on a spike -> scream.
        if self.spikes.contains(&next_pos) {
            self.trigger_scream();
        }

        if self.current_pos == self.maze.goal {
            MoveOutcome::ReachedGoal
        } else {
            MoveOutcome::Moved
        }
    }

    /// Stepping on a spike: release a hound from the far corner, inform every
    /// hound of the player's current location, and start bleeding for 3 turns.
    fn trigger_scream(&mut self) {
        self.screams += 1;
        let spawn = furthest_corner_spawn(&self.maze, self.current_pos);
        let mut visits = HashMap::new();
        visits.insert(spawn, 1);
        self.hounds.push(Bloodhound {
            pos: spawn,
            target: self.current_pos,
            anchor: self.current_pos,
            evidence_turn: self.turn,
            visits,
        });
        // Broadcast the exact player position to all hounds (including the new one).
        for h in &mut self.hounds {
            h.target = self.current_pos;
            h.anchor = self.current_pos;
            h.evidence_turn = self.turn;
        }
        self.bleed_turns_remaining = 3;
        self.blood.insert(self.current_pos, self.turn);
    }

    /// Pick a search step when the trail is cold: move to the connected
    /// neighbor visited least often (so the hound spreads out rather than
    /// oscillating), breaking ties toward its last-known scent location.
    /// Never returns the current cell unless the hound is truly isolated.
    fn search_step(&self, i: usize) -> Coord {
        let h = &self.hounds[i];
        let mut best: Option<(u32, u32, Coord)> = None;
        for nb in self.maze.get_connected_neighbors(h.pos) {
            let visit_count = *h.visits.get(&nb).unwrap_or(&0);
            let toward_scent = heuristic(nb, h.anchor);
            let key = (visit_count, toward_scent);
            let better = match best {
                None => true,
                Some((bv, bt, _)) => key < (bv, bt),
            };
            if better {
                best = Some((visit_count, toward_scent, nb));
            }
        }
        best.map(|(_, _, c)| c).unwrap_or(h.pos)
    }

    /// Advance every bloodhound one cell. A hound walks the unique tree path
    /// toward its freshest scream/blood evidence; once it reaches that cell with
    /// no fresher lead it searches outward — it never sits still. Crossing a
    /// fresher blood trace re-points it at the player.
    fn move_bloodhounds(&mut self) {
        let n = self.hounds.len();
        for i in 0..n {
            let pos = self.hounds[i].pos;
            let next = if pos != self.hounds[i].target {
                // Head toward the freshest known evidence.
                astar_pathfind(&self.maze, pos, self.hounds[i].target)
                    .and_then(|(path, _)| path.get(1).copied())
                    .unwrap_or_else(|| self.search_step(i))
            } else {
                // Arrived at the last-known location: roam to hunt for the trail.
                let step = self.search_step(i);
                self.hounds[i].target = step; // commit so the search keeps spreading
                step
            };

            self.hounds[i].pos = next;
            *self.hounds[i].visits.entry(next).or_insert(0) += 1;

            // Detect blood by age; follow fresher trails toward the player.
            if let Some(&age) = self.blood.get(&next) {
                if age > self.hounds[i].evidence_turn {
                    self.hounds[i].target = next;
                    self.hounds[i].anchor = next;
                    self.hounds[i].evidence_turn = age;
                }
            }
        }
    }

    fn display_status(&self) {
        use std::io::{stdout, Write};

        print!("\r\n🩸 BLOODHOUND SURVIVAL - Level {}\r\n", self.level);
        print!(
            "Position: {:?}  →  Goal: {:?}\r\n",
            self.current_pos, self.maze.goal
        );
        print!(
            "Moves: {}  |  Time: {:.1}s  |  Visited: {}/{}\r\n",
            self.move_history.len() - 1,
            self.start_time.elapsed().as_secs_f64(),
            self.visited.len(),
            self.maze.carved.len()
        );

        // Spike detector reading.
        let spikes_near = self.spike_detector_count();
        if spikes_near == 0 {
            print!("🧭 Spike detector: clear (0 adjacent)\r\n");
        } else {
            print!(
                "🧭 Spike detector: ⚠ {} adjacent cell(s) contain spikes!\r\n",
                spikes_near
            );
        }

        // Bloodhound threat.
        match self.nearest_hound_distance() {
            None => print!("🐕 Bloodhounds: none released (stay quiet!)\r\n"),
            Some(d) => print!(
                "🐕 Bloodhounds: {} loose — nearest {:.1} away (straight line)!\r\n",
                self.hounds.len(),
                d
            ),
        }
        if self.any_hound_can_lunge() {
            print!("‼️  A bloodhound is one step away with a clear path — MOVE or it lunges!\r\n");
        }

        if self.bleed_turns_remaining > 0 {
            print!(
                "🩸 Bleeding: {} turn(s) of blood trail left\r\n",
                self.bleed_turns_remaining
            );
        }

        let dist = heuristic(self.current_pos, self.maze.goal);
        print!("\r\nDistance to goal: {} (straight line)\r\n\r\n", dist);

        print!("Available Directions (press key to move):\r\n");
        for neighbor in self.maze.get_connected_neighbors(self.current_pos) {
            let delta = (
                neighbor.0 - self.current_pos.0,
                neighbor.1 - self.current_pos.1,
                neighbor.2 - self.current_pos.2,
            );
            if let Some((key, name)) = delta_to_dir(delta) {
                if self.hound_at(neighbor) {
                    print!(
                        "  [{:>1}] 💀 {} (BLOODHOUND - moving here is death!)\r\n",
                        key, name
                    );
                } else {
                    let mark = if self.visited.contains(&neighbor) {
                        "●"
                    } else {
                        "○"
                    };
                    print!("  [{:>1}] {} {}\r\n", key, mark, name);
                }
            }
        }

        print!("\r\nCommands: [h]int (path to goal) | [q]uit\r\n");
        let _ = stdout().flush();
    }
}

/// Render the "caught by a bloodhound" end-of-run screen.
fn print_caught_screen(game: &BloodhoundGame, current_level: u32, levels_cleared: u32) {
    use std::io::{stdout, Write};
    clear_screen();
    print!("\r\n💀 CAUGHT! A bloodhound sank its teeth into you.\r\n");
    print!("═══════════════════════════════════════\r\n");
    print!("Levels cleared this run: {}\r\n", levels_cleared);
    print!("Reached level: {}\r\n", current_level);
    print!("Screams this level: {}\r\n", game.screams);
    print!("Total moves: {}\r\n", game.move_history.len() - 1);
    print!(
        "Time survived: {:.1}s\r\n",
        game.start_time.elapsed().as_secs_f64()
    );
    let _ = stdout().flush();
}

fn play_bloodhound_game(seed: u64) -> Result<()> {
    use std::io::{stdout, Write};

    let _raw_mode_guard = RawModeGuard::new()?;
    let mut stats = BloodhoundStats::load();

    // Progressive sizing: start at 8x8x8, grow by one each cleared level.
    let mut current_level = 1u32;
    let mut current_size = 8u32;
    let mut levels_cleared = 0u32;
    let mut total_screams = 0u32;
    let mut caught = false;

    'game_loop: loop {
        clear_screen();
        print!(
            "\r\n🩸 BLOODHOUND SURVIVAL - LEVEL {} ({}x{}x{})\r\n",
            current_level, current_size, current_size, current_size
        );
        print!("Reach the goal before the bloodhounds reach you.\r\n");
        let _ = stdout().flush();

        let extent = (current_size, current_size, current_size);
        let level_seed = seed.wrapping_add(current_level as u64);
        let maze = Maze::generate(extent, level_seed);
        // Derive a distinct (but deterministic) seed for spike placement.
        let spikes = generate_spikes(&maze, level_seed ^ 0x9E37_79B9_7F4A_7C15, 10);

        print!(
            "\r\n✓ Maze generated! Carved nodes: {}\r\n",
            maze.carved.len()
        );
        print!("✓ Start: {:?} → Goal: {:?}\r\n", maze.start, maze.goal);
        print!("\r\n10% of cells hide an invisible metal spike. Step on one and\r\n");
        print!("you scream, releasing a bloodhound from the far corner...\r\n");
        print!("\r\nPress any key to begin...\r\n");
        let _ = stdout().flush();
        wait_for_any_key()?;

        let mut game = BloodhoundGame::new(maze, spikes, current_level);

        'level_loop: loop {
            clear_screen();
            game.display_status();

            // Blocked in (no legal move): the player forfeits the turn and the
            // hounds advance anyway.
            if !game.has_available_move() {
                print!("\r\n🚧 You're blocked in — no escape! You forfeit this turn.\r\n");
                print!("Press any key to wait it out (or [q] to quit)...\r\n");
                let _ = stdout().flush();
                let KeyEvent { code, .. } = read_key_event()?;
                if let KeyCode::Char('q') = code {
                    total_screams += game.screams;
                    clear_screen();
                    print!("\r\nYou abandon the maze. The hounds win by default.\r\n");
                    print!("Levels cleared this run: {}\r\n", levels_cleared);
                    let _ = stdout().flush();
                    break 'game_loop;
                }
                if game.resolve_hounds() {
                    caught = true;
                    total_screams += game.screams;
                    print_caught_screen(&game, current_level, levels_cleared);
                    break 'game_loop;
                }
                continue 'level_loop;
            }

            let KeyEvent { code, .. } = read_key_event()?;
            match code {
                KeyCode::Char('q') => {
                    total_screams += game.screams;
                    clear_screen();
                    print!("\r\nYou abandon the maze. The hounds win by default.\r\n");
                    print!("Levels cleared this run: {}\r\n", levels_cleared);
                    let _ = stdout().flush();
                    break 'game_loop;
                }
                KeyCode::Char('h') => {
                    game.hints_used += 1;
                    clear_screen();
                    game.display_status();
                    print!("\r\n💡 Computing path to goal...\r\n");
                    if let Some((path, _)) =
                        astar_pathfind(&game.maze, game.current_pos, game.maze.goal)
                    {
                        if path.len() > 1 {
                            let next = path[1];
                            let delta = (
                                next.0 - game.current_pos.0,
                                next.1 - game.current_pos.1,
                                next.2 - game.current_pos.2,
                            );
                            let key = delta_to_dir(delta).map(|(k, _)| k).unwrap_or('?');
                            print!(
                                "Optimal path: {} moves. Next move: press [{}]\r\n",
                                path.len() - 1,
                                key
                            );
                        } else {
                            print!("You're already at the goal!\r\n");
                        }
                    } else {
                        print!("No path found!\r\n");
                    }
                    print!("\r\nPress any key to continue...\r\n");
                    let _ = stdout().flush();
                    wait_for_any_key()?;
                }
                KeyCode::Char(c) => {
                    match game.make_move(c) {
                        MoveOutcome::ReachedGoal => {
                            // Reached the goal — level cleared.
                            levels_cleared += 1;
                            total_screams += game.screams;
                            clear_screen();
                            print!("\r\n🎉 LEVEL {} CLEARED — you escaped!\r\n", current_level);
                            print!("═══════════════════════════════════════\r\n");
                            print!("Moves: {}\r\n", game.move_history.len() - 1);
                            print!("Time: {:.1}s\r\n", game.start_time.elapsed().as_secs_f64());
                            print!("Screams this level: {}\r\n", game.screams);
                            print!("Bloodhounds loose at the end: {}\r\n", game.hounds.len());
                            print!(
                                "\r\n🎯 Ready for Level {} ({}x{}x{})?\r\n",
                                current_level + 1,
                                current_size + 1,
                                current_size + 1,
                                current_size + 1
                            );
                            print!("Press [Enter] to continue or [q] to quit...\r\n");
                            let _ = stdout().flush();
                            loop {
                                let KeyEvent { code, .. } = read_key_event()?;
                                match code {
                                    KeyCode::Enter => {
                                        current_level += 1;
                                        current_size += 1;
                                        break 'level_loop;
                                    }
                                    KeyCode::Char('q') => break 'game_loop,
                                    _ => {}
                                }
                            }
                        }
                        MoveOutcome::Moved => {
                            // Player moved — now the hounds hunt (and may lunge).
                            if game.resolve_hounds() {
                                caught = true;
                                total_screams += game.screams;
                                print_caught_screen(&game, current_level, levels_cleared);
                                break 'game_loop;
                            }
                        }
                        MoveOutcome::IntoBloodhound => {
                            // Walked into a bloodhound — instant death.
                            caught = true;
                            total_screams += game.screams;
                            print_caught_screen(&game, current_level, levels_cleared);
                            break 'game_loop;
                        }
                        MoveOutcome::Blocked => {
                            // Wall or invalid key — silently redraw.
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Record the run once it ends (death or quit).
    stats.record_run(levels_cleared, total_screams, caught);

    print!("\r\n📈 BLOODHOUND LIFETIME STATS:\r\n");
    print!("   Runs: {} | Deaths: {}\r\n", stats.runs, stats.deaths);
    print!("   Best level reached: {}\r\n", stats.best_level);
    print!("   Total screams: {}\r\n", stats.total_screams);
    print!("\r\nPress any key to exit...\r\n");
    let _ = stdout().flush();
    wait_for_any_key()?;

    println!();
    Ok(())
}

// ============================================================================
// Benchmarks
// ============================================================================

fn run_benchmarks(iterations: usize) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║            OCTAINDEX3D PERFORMANCE BENCHMARKS             ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("Running {} iterations for each benchmark...\n", iterations);

    // Benchmark 1: Morton Encoding
    println!("1. Morton Encoding (Index64::new)");
    let start = Instant::now();
    for i in 0..iterations {
        let x = (i % 1000) as u16;
        let y = ((i / 1000) % 1000) as u16;
        let z = ((i / 1000000) % 1000) as u16;
        let _ = Index64::new(0, 0, 5, x, y, z);
    }
    let elapsed = start.elapsed();
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!(
        "   Rate: {:.2}M ops/sec\n",
        iterations as f64 / elapsed.as_secs_f64() / 1_000_000.0
    );

    // Benchmark 2: Route64 Creation
    println!("2. Route64 Creation");
    let start = Instant::now();
    for i in 0..iterations {
        let x = (i % 1000) as i32;
        let y = ((i / 1000) % 1000) as i32;
        let z = ((i / 1000000) % 1000) as i32;
        let _ = Route64::new(0, x, y, z);
    }
    let elapsed = start.elapsed();
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!(
        "   Rate: {:.2}M ops/sec\n",
        iterations as f64 / elapsed.as_secs_f64() / 1_000_000.0
    );

    // Benchmark 3: Neighbor Calculations
    println!("3. BCC Neighbor Calculations");
    let coord = (100, 100, 100);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = get_neighbors((200, 200, 200), coord);
    }
    let elapsed = start.elapsed();
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!(
        "   Rate: {:.2}M ops/sec\n",
        iterations as f64 / elapsed.as_secs_f64() / 1_000_000.0
    );

    // Benchmark 4: BCC Validity Check
    println!("4. BCC Validity Check");
    let start = Instant::now();
    for i in 0..iterations {
        let x = (i % 1000) as i32;
        let y = ((i / 1000) % 1000) as i32;
        let z = ((i / 1000000) % 1000) as i32;
        let _ = is_valid_bcc((x, y, z));
    }
    let elapsed = start.elapsed();
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!(
        "   Rate: {:.2}M ops/sec\n",
        iterations as f64 / elapsed.as_secs_f64() / 1_000_000.0
    );

    // Benchmark 5: Maze Generation
    println!("5. Maze Generation (20x20x20)");
    let start = Instant::now();
    let maze = Maze::generate((20, 20, 20), 42);
    let elapsed = start.elapsed();
    println!("   Time: {:.3}s", elapsed.as_secs_f64());
    println!("   Carved nodes: {}", maze.carved.len());
    println!(
        "   Rate: {:.2}K nodes/sec\n",
        maze.carved.len() as f64 / elapsed.as_secs_f64() / 1000.0
    );

    // Benchmark 6: A* Pathfinding
    println!("6. A* Pathfinding (on generated maze)");
    let start = Instant::now();
    let path_result = astar_pathfind(&maze, maze.start, maze.goal);
    let elapsed = start.elapsed();
    if let Some((p, nodes_visited)) = path_result {
        println!("   Time: {:.3}s", elapsed.as_secs_f64());
        println!("   Path length: {}", p.len());
        println!("   Nodes explored: {}", nodes_visited);
        println!(
            "   Search rate: {:.2}K nodes/sec\n",
            nodes_visited as f64 / elapsed.as_secs_f64() / 1000.0
        );
    } else {
        println!("   No path found\n");
    }

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                  BENCHMARKS COMPLETE                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
}

// ============================================================================
// Utility Functions
// ============================================================================

fn run_encode(x: i32, y: i32, z: i32) -> Result<()> {
    // Convert to u16, checking bounds
    if x < 0 || y < 0 || z < 0 || x > u16::MAX as i32 || y > u16::MAX as i32 || z > u16::MAX as i32
    {
        return Err(octaindex3d::Error::OutOfRange(format!(
            "Coordinates must be in range 0..{}",
            u16::MAX
        )));
    }

    let index = Index64::new(0, 0, 5, x as u16, y as u16, z as u16)?;
    println!("\nCoordinate: ({}, {}, {})", x, y, z);
    println!("Index64: {:?}", index);
    println!("Hex: {:#018x}", index.raw());
    println!("Bech32m: {}", index.to_bech32m()?);
    Ok(())
}

fn run_decode(value: String) -> Result<()> {
    // Try to decode as Bech32m first (starts with i3d1)
    if value.starts_with("i3d1") {
        let index = Index64::from_bech32m(&value)?;
        println!("\nBech32m: {}", value);
        println!("Index64: {:?}", index);
        println!("Hex: {:#018x}", index.raw());
        let (x, y, z) = index.decode_coords();
        println!("Coordinates: ({}, {}, {})", x, y, z);
        println!("Frame: {}", index.frame_id());
        println!("Tier: {}", index.scale_tier());
        println!("LOD: {}", index.lod());
    } else {
        // Otherwise interpret as hex or decimal
        let val = if let Some(stripped) = value.strip_prefix("0x") {
            u64::from_str_radix(stripped, 16)
                .map_err(|_| octaindex3d::Error::OutOfRange("Invalid hex value".to_string()))?
        } else {
            value
                .parse::<u64>()
                .map_err(|_| octaindex3d::Error::OutOfRange("Invalid decimal value".to_string()))?
        };

        println!("\nRaw value: {:#018x}", val);
        println!("Note: Use Index64::from_bech32m() to decode properly, or encode coordinates to see structure");
    }
    Ok(())
}

fn run_distance(from: (i32, i32, i32), to: (i32, i32, i32)) {
    let euclidean = {
        let dx = (from.0 - to.0) as f64;
        let dy = (from.1 - to.1) as f64;
        let dz = (from.2 - to.2) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    };

    let manhattan = { (from.0 - to.0).abs() + (from.1 - to.1).abs() + (from.2 - to.2).abs() };

    let chebyshev = {
        (from.0 - to.0)
            .abs()
            .max((from.1 - to.1).abs())
            .max((from.2 - to.2).abs())
    };

    println!("\nFrom: {:?}", from);
    println!("To: {:?}", to);
    println!("Euclidean Distance: {:.2}", euclidean);
    println!("Manhattan Distance: {}", manhattan);
    println!("Chebyshev Distance (BCC minimum): {}", chebyshev);
}

fn run_neighbors(x: i32, y: i32, z: i32) {
    let coord = (x, y, z);

    println!("\nCoordinate: {:?}", coord);
    println!(
        "BCC Valid: {}",
        if is_valid_bcc(coord) { "Yes" } else { "No" }
    );
    println!("\n14 BCC Neighbors:");

    let neighbors: Vec<_> = BCC_NEIGHBORS
        .iter()
        .map(|&(dx, dy, dz)| (x + dx, y + dy, z + dz))
        .collect();

    for (i, n) in neighbors.iter().enumerate() {
        let valid = is_valid_bcc(*n);
        println!("  {}. {:?} {}", i + 1, n, if valid { "✓" } else { "✗" });
    }
}

// ============================================================================
// Main
// ============================================================================

fn display_stats() {
    let stats = CompetitiveStats::load();

    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║        COMPETITIVE STATS - Can You Beat A*?              ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    if stats.total_games == 0 {
        println!("No games played yet. Start playing with: octaindex3d play\n");
        return;
    }

    println!("📊 OVERALL RECORD:\n");
    println!("   Total Games:        {}", stats.total_games);
    println!("   Wins:               {} (beat A*)", stats.wins);
    println!("   Ties:               {} (matched A*)", stats.ties);
    println!("   Losses:             {} (A* was better)", stats.losses);

    println!("\n📈 PERFORMANCE METRICS:\n");
    println!("   Win Rate (W+T):     {:.1}%", stats.win_rate());
    println!("   Avg Efficiency:     {:.1}%", stats.average_efficiency());
    println!("   Best Efficiency:    {:.1}%", stats.best_efficiency);
    println!("   Worst Efficiency:   {:.1}%", stats.worst_efficiency);

    println!("\n💡 INTERPRETATION:\n");
    if stats.win_rate() >= 90.0 {
        println!("   🏆 You're a pathfinding master!");
    } else if stats.win_rate() >= 70.0 {
        println!("   ⭐ You're beating A* consistently!");
    } else if stats.win_rate() >= 50.0 {
        println!("   👍 You're competitive with A*!");
    } else {
        println!("   💪 Keep practicing to improve!");
    }

    let avg = stats.average_efficiency();
    if avg >= 95.0 {
        println!("   Your solutions are nearly optimal!");
    } else if avg >= 80.0 {
        println!("   Your solutions are quite efficient!");
    } else if avg >= 60.0 {
        println!("   There's room to optimize your paths.");
    } else {
        println!("   Try using the hint system to improve!");
    }

    println!(
        "\n📁 Stats file: {}\n",
        CompetitiveStats::stats_file().display()
    );
}

fn reset_stats() {
    match fs::remove_file(CompetitiveStats::stats_file()) {
        Ok(_) => {
            println!("\n✓ Stats reset successfully!");
            println!("Your competitive record has been cleared.\n");
        }
        Err(e) => {
            println!("\n✗ Could not reset stats: {}\n", e);
        }
    }
}

/// Interactive mode picker. Returns `Some(false)` for A* race, `Some(true)`
/// for bloodhound survival, or `None` if the player quits.
fn select_mode_menu() -> Result<Option<bool>> {
    use std::io::{stdout, Write};

    let _raw_mode_guard = RawModeGuard::new()?;
    loop {
        clear_screen();
        print!("\r\n🎮 3D OCTAHEDRAL MAZE GAME\r\n");
        print!("═══════════════════════════════════════\r\n\r\n");
        print!("Choose your mode:\r\n\r\n");
        print!("  [1] A* Race        — beat A* on nodes explored (classic)\r\n");
        print!("  [2] Bloodhounds    — reach the goal before the hounds catch you\r\n\r\n");
        print!("  [q] Quit\r\n");
        let _ = stdout().flush();

        let KeyEvent { code, .. } = read_key_event()?;
        match code {
            KeyCode::Char('1') => return Ok(Some(false)),
            KeyCode::Char('2') => return Ok(Some(true)),
            KeyCode::Char('q') => return Ok(None),
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Play {
            size,
            seed,
            difficulty,
            mode,
        } => {
            // Determine if we use custom size or progressive mode
            let custom_size = if let Some(diff) = difficulty {
                Some(match diff.as_str() {
                    "easy" => 8,
                    "medium" => 20,
                    "hard" => 40,
                    _ => size,
                })
            } else if size != 20 {
                // If user specified non-default size
                Some(size)
            } else {
                None // Use progressive mode starting at 2x2x2
            };

            let actual_seed = seed.unwrap_or_else(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(42)
            });

            // Resolve the mode: explicit --mode flag, else an interactive menu.
            let chosen = match mode.as_deref() {
                Some("astar") => Some(false),
                Some("bloodhound") => Some(true),
                _ => select_mode_menu()?,
            };

            match chosen {
                Some(false) => play_game(custom_size, actual_seed)?,
                Some(true) => play_bloodhound_game(actual_seed)?,
                None => {} // User quit at the menu.
            }
        }

        Commands::Stats => {
            display_stats();
        }

        Commands::ResetStats => {
            reset_stats();
        }

        Commands::Benchmark { iterations } => {
            run_benchmarks(iterations);
        }

        Commands::Utils { util_command } => match util_command {
            UtilCommands::Encode { x, y, z } => {
                run_encode(x, y, z)?;
            }
            UtilCommands::Decode { value } => {
                run_decode(value)?;
            }
            UtilCommands::Distance { from, to } => {
                run_distance(from, to);
            }
            UtilCommands::Neighbors { x, y, z } => {
                run_neighbors(x, y, z);
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod bloodhound_tests {
    use super::*;

    fn make_hound(pos: Coord) -> Bloodhound {
        let mut visits = HashMap::new();
        visits.insert(pos, 1);
        Bloodhound {
            pos,
            target: pos,
            anchor: pos,
            evidence_turn: 0,
            visits,
        }
    }

    /// A hound one open (connected) cell away lunges and catches the player.
    #[test]
    fn connected_hound_lunges_and_catches() {
        let maze = Maze::generate((6, 6, 6), 7);
        let start = maze.start;
        let hp = maze.get_connected_neighbors(start)[0]; // an open neighbor
        let mut game = BloodhoundGame::new(maze, HashSet::new(), 1);
        game.hounds.push(make_hound(hp));
        assert!(game.any_hound_can_lunge());
        assert!(game.resolve_hounds(), "open-adjacent hound must catch");
    }

    /// A hound one cell away but separated by a wall cannot lunge.
    #[test]
    fn walled_neighbor_hound_cannot_lunge() {
        let maze = Maze::generate((6, 6, 6), 7);
        let start = maze.start;
        let connected: HashSet<Coord> = maze.get_connected_neighbors(start).into_iter().collect();
        let walled = BCC_NEIGHBORS
            .iter()
            .map(|d| (start.0 + d.0, start.1 + d.1, start.2 + d.2))
            .find(|c| maze.carved.contains(c) && !connected.contains(c));
        let mut game = BloodhoundGame::new(maze, HashSet::new(), 1);
        match walled {
            Some(wc) => {
                game.hounds.push(make_hound(wc));
                assert!(!game.any_hound_can_lunge(), "a wall must block the lunge");
            }
            None => {
                game.hounds.push(make_hound((5, 5, 5)));
                assert!(!game.any_hound_can_lunge());
            }
        }
    }

    /// Moving into a bloodhound's cell is fatal, not silently blocked.
    #[test]
    fn walking_into_hound_is_fatal() {
        let maze = Maze::generate((6, 6, 6), 7);
        let start = maze.start;
        let nb = maze.get_connected_neighbors(start)[0];
        let d = (nb.0 - start.0, nb.1 - start.1, nb.2 - start.2);
        let (key, _) = delta_to_dir(d).expect("neighbor delta maps to a move key");
        let mut game = BloodhoundGame::new(maze, HashSet::new(), 1);
        game.hounds.push(make_hound(nb));
        assert!(matches!(game.make_move(key), MoveOutcome::IntoBloodhound));
    }

    /// A hound sitting on its target with no fresher lead must still move
    /// (search) rather than camping in place.
    #[test]
    fn hound_searches_instead_of_sitting() {
        let maze = Maze::generate((6, 6, 6), 7);
        let start = maze.start;
        let hp = maze
            .carved
            .iter()
            .copied()
            .find(|&c| c != start && !maze.get_connected_neighbors(c).is_empty())
            .expect("a carved cell with a neighbor");
        let mut game = BloodhoundGame::new(maze, HashSet::new(), 1);
        game.hounds.push(make_hound(hp)); // target == pos: it has "arrived"
        game.move_bloodhounds();
        assert_ne!(game.hounds[0].pos, hp, "hound must not sit on its cell");
    }
}
