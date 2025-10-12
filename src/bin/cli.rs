//! OctaIndex3D CLI tool

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use octaindex3d::id::CellID;
use octaindex3d::io::export_cells_geojson;
use octaindex3d::path::{astar, k_ring, k_shell, trace_line, EuclideanCost};

#[derive(Parser)]
#[command(name = "octaindex3d")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "3D Spatial Indexing and Routing System based on BCC lattice", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert coordinates to cell ID
    IdFromCoord {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Z coordinate
        z: i32,
        /// Resolution level
        #[arg(short, long, default_value = "0")]
        resolution: u8,
        /// Frame ID
        #[arg(short, long, default_value = "0")]
        frame: u8,
    },

    /// Decode cell ID to coordinates
    IdToCoord {
        /// Cell ID in Bech32m format
        cell_id: String,
    },

    /// Get neighbors of a cell
    Neighbors {
        /// Cell ID in Bech32m format
        cell_id: String,
    },

    /// Get children of a cell
    Children {
        /// Cell ID in Bech32m format
        cell_id: String,
    },

    /// Get parent of a cell
    Parent {
        /// Cell ID in Bech32m format
        cell_id: String,
    },

    /// Compute k-ring around a cell
    KRing {
        /// Cell ID in Bech32m format
        cell_id: String,
        /// Ring distance
        #[arg(short, long)]
        k: usize,
        /// Output format (text, json, geojson)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Compute k-shell around a cell
    KShell {
        /// Cell ID in Bech32m format
        cell_id: String,
        /// Shell distance
        #[arg(short, long)]
        k: usize,
        /// Output format (text, json, geojson)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Find path between two cells using A*
    Route {
        /// Start coordinates (x,y,z)
        #[arg(long)]
        start: String,
        /// Goal coordinates (x,y,z)
        #[arg(long)]
        goal: String,
        /// Resolution level
        #[arg(short, long, default_value = "5")]
        resolution: u8,
        /// Frame ID
        #[arg(short, long, default_value = "0")]
        frame: u8,
    },

    /// Trace line between two cells
    TraceLine {
        /// Start coordinates (x,y,z)
        #[arg(long)]
        start: String,
        /// End coordinates (x,y,z)
        #[arg(long)]
        end: String,
        /// Resolution level
        #[arg(short, long, default_value = "5")]
        resolution: u8,
        /// Frame ID
        #[arg(short, long, default_value = "0")]
        frame: u8,
    },
}

fn parse_coords(s: &str) -> Result<(i32, i32, i32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        anyhow::bail!("Coordinates must be in format x,y,z");
    }

    let x = parts[0].trim().parse()?;
    let y = parts[1].trim().parse()?;
    let z = parts[2].trim().parse()?;

    Ok((x, y, z))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::IdFromCoord {
            x,
            y,
            z,
            resolution,
            frame,
        } => {
            let cell = CellID::from_coords(frame, resolution, x, y, z)
                .context("Failed to create cell ID")?;
            let bech32m = cell.to_bech32m().context("Failed to encode as Bech32m")?;

            println!("Cell ID: {}", cell);
            println!("Bech32m: {}", bech32m);
            println!("Frame: {}", cell.frame());
            println!("Resolution: {}", cell.resolution());
            println!("Coordinates: ({}, {}, {})", cell.x(), cell.y(), cell.z());
        }

        Commands::IdToCoord { cell_id } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;

            println!("Cell ID decoded successfully");
            println!("Frame: {}", cell.frame());
            println!("Resolution: {}", cell.resolution());
            println!("Coordinates: ({}, {}, {})", cell.x(), cell.y(), cell.z());
            println!("Raw value: {:#034x}", cell.raw_value());
        }

        Commands::Neighbors { cell_id } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;
            let neighbors = cell.neighbors();

            println!("Cell has {} neighbors:", neighbors.len());
            for (i, neighbor) in neighbors.iter().enumerate() {
                let bech32m = neighbor
                    .to_bech32m()
                    .unwrap_or_else(|_| "ERROR".to_string());
                println!(
                    "  {}: ({}, {}, {}) - {}",
                    i + 1,
                    neighbor.x(),
                    neighbor.y(),
                    neighbor.z(),
                    bech32m
                );
            }
        }

        Commands::Children { cell_id } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;
            let children = cell.children().context("Failed to get children")?;

            println!("Cell has {} children:", children.len());
            for (i, child) in children.iter().enumerate() {
                let bech32m = child.to_bech32m().unwrap_or_else(|_| "ERROR".to_string());
                println!(
                    "  {}: ({}, {}, {}) - {}",
                    i + 1,
                    child.x(),
                    child.y(),
                    child.z(),
                    bech32m
                );
            }
        }

        Commands::Parent { cell_id } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;
            let parent = cell.parent().context("Failed to get parent")?;
            let bech32m = parent.to_bech32m().unwrap_or_else(|_| "ERROR".to_string());

            println!("Parent cell:");
            println!("  Frame: {}", parent.frame());
            println!("  Resolution: {}", parent.resolution());
            println!(
                "  Coordinates: ({}, {}, {})",
                parent.x(),
                parent.y(),
                parent.z()
            );
            println!("  Bech32m: {}", bech32m);
        }

        Commands::KRing { cell_id, k, format } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;
            let ring = k_ring(cell, k);

            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&ring)?;
                    println!("{}", json);
                }
                "geojson" => {
                    let geojson = export_cells_geojson(&ring)?;
                    println!("{}", geojson);
                }
                _ => {
                    println!("K-ring (k={}) contains {} cells:", k, ring.len());
                    for (i, c) in ring.iter().enumerate() {
                        println!("  {}: ({}, {}, {})", i + 1, c.x(), c.y(), c.z());
                    }
                }
            }
        }

        Commands::KShell { cell_id, k, format } => {
            let cell = CellID::from_bech32m(&cell_id).context("Failed to decode cell ID")?;
            let shell = k_shell(cell, k);

            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&shell)?;
                    println!("{}", json);
                }
                "geojson" => {
                    let geojson = export_cells_geojson(&shell)?;
                    println!("{}", geojson);
                }
                _ => {
                    println!("K-shell (k={}) contains {} cells:", k, shell.len());
                    for (i, c) in shell.iter().enumerate() {
                        println!("  {}: ({}, {}, {})", i + 1, c.x(), c.y(), c.z());
                    }
                }
            }
        }

        Commands::Route {
            start,
            goal,
            resolution,
            frame,
        } => {
            let (sx, sy, sz) = parse_coords(&start)?;
            let (gx, gy, gz) = parse_coords(&goal)?;

            let start_cell = CellID::from_coords(frame, resolution, sx, sy, sz)?;
            let goal_cell = CellID::from_coords(frame, resolution, gx, gy, gz)?;

            println!("Finding path from {} to {}...", start_cell, goal_cell);

            let cost_fn = EuclideanCost;
            let path = astar(start_cell, goal_cell, &cost_fn)?;

            println!("\nPath found!");
            println!("  Length: {} cells", path.len());
            println!("  Cost: {:.2}", path.cost);
            println!("\nPath:");
            for (i, cell) in path.cells.iter().enumerate() {
                println!("  {}: ({}, {}, {})", i + 1, cell.x(), cell.y(), cell.z());
            }
        }

        Commands::TraceLine {
            start,
            end,
            resolution,
            frame,
        } => {
            let (sx, sy, sz) = parse_coords(&start)?;
            let (ex, ey, ez) = parse_coords(&end)?;

            let start_cell = CellID::from_coords(frame, resolution, sx, sy, sz)?;
            let end_cell = CellID::from_coords(frame, resolution, ex, ey, ez)?;

            println!("Tracing line from {} to {}...", start_cell, end_cell);

            let line = trace_line(start_cell, end_cell)?;

            println!("\nLine traverses {} cells:", line.len());
            for (i, cell) in line.iter().enumerate() {
                println!("  {}: ({}, {}, {})", i + 1, cell.x(), cell.y(), cell.z());
            }
        }
    }

    Ok(())
}
