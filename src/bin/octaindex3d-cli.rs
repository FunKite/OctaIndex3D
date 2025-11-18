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
        use rand::{Rng, SeedableRng};

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

fn play_game(custom_size: Option<u32>, seed: u64) {
    use std::io::{stdout, Write};

    // Enable raw mode for single-key input
    terminal::enable_raw_mode().expect("Failed to enable raw mode");

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
        loop {
            if let Event::Key(_) = event::read().expect("Failed to read event") {
                break;
            }
        }

        let mut game = GameState::new(maze, current_level);

        'level_loop: loop {
            // Clear screen and display game state
            clear_screen();
            game.display_status();

            // Read single key
            if let Event::Key(KeyEvent { code, .. }) = event::read().expect("Failed to read event")
            {
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
                        event::read().expect("Failed to read event");
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
                                            let pct_more = ((player_moves as f64
                                                / optimal_moves as f64)
                                                - 1.0)
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
                                            print!("   A* explored {} nodes, you explored {} nodes\r\n",
                                                astar_nodes_visited, player_visited);
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
                                            print!("   A* explored {} nodes, you explored {} nodes\r\n",
                                                astar_nodes_visited, player_visited);
                                            print!("   └─ You explored {} more nodes ({:.0}% more than A*)\r\n",
                                                player_visited - astar_nodes_visited, pct_more);
                                        }

                                        // Competitive result (based on nodes explored)
                                        print!("\r\n🎮 COMPETITIVE RESULT (Fewest Nodes Explored Wins):\r\n");
                                        print!("───────────────────────────────────────\r\n");
                                        #[allow(clippy::comparison_chain)]
                                        if player_visited < astar_nodes_visited {
                                            let nodes_saved = astar_nodes_visited - player_visited;
                                            print!("🏆 YOU WIN! You explored {} fewer nodes than A*!\r\n", nodes_saved);
                                            stats.update(player_visited, astar_nodes_visited);
                                        } else if player_visited == astar_nodes_visited {
                                            print!("🤝 TIE! You explored the same number of nodes as A*!\r\n");
                                            stats.update(player_visited, astar_nodes_visited);
                                        } else {
                                            let extra_nodes = player_visited - astar_nodes_visited;
                                            print!("🤖 A* WINS. You explored {} more nodes than A*.\r\n", extra_nodes);
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
                                            if let Event::Key(KeyEvent { code, .. }) =
                                                event::read().expect("Failed to read event")
                                            {
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
                                        }
                                    } else {
                                        print!("\r\nPress any key to exit...\r\n");
                                        let _ = stdout().flush();
                                        event::read().expect("Failed to read event");
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
    }

    // Disable raw mode before exiting
    terminal::disable_raw_mode().expect("Failed to disable raw mode");
    println!();
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Play {
            size,
            seed,
            difficulty,
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

            play_game(custom_size, actual_seed);
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
