//! Quick start: the modern OctaIndex3D API
//!
//! Demonstrates the `BccGrid` facade (physical units in, physical units out)
//! and the modern ID types. Run with:
//!
//! ```bash
//! cargo run --example quickstart
//! ```

use octaindex3d::{BccGrid, Index64, Result, Route64};

fn main() -> Result<()> {
    println!("=== OctaIndex3D Quick Start ===\n");

    // A grid with 0.5-unit cells (e.g. 0.5 m for robotics maps).
    // The grid handles BCC parity and coordinate snapping for you.
    let grid = BccGrid::new(0.5)?;

    // 1. Convert a physical point to a cell and back
    let cell = grid.cell_at(1.2, 3.4, 5.6)?;
    let (cx, cy, cz) = grid.center_of(cell);
    println!(
        "1. Point (1.2, 3.4, 5.6) -> cell {} centered at ({cx:.2}, {cy:.2}, {cz:.2})",
        cell
    );

    // 2. Neighbors: every interior BCC cell has 14
    let neighbors = grid.neighbors(cell);
    println!("\n2. Cell has {} neighbors", neighbors.len());

    // 3. K-ring: all cells within 2 hops
    let nearby = grid.k_ring(cell, 2);
    println!("\n3. {} cells within 2 hops", nearby.len());

    // 4. A* pathfinding between two points
    let start = grid.cell_at(0.0, 0.0, 0.0)?;
    let goal = grid.cell_at(5.0, 5.0, 5.0)?;
    let path = grid.astar(start, goal)?;
    println!(
        "\n4. Shortest path: {} cells, {:.2} physical units",
        path.len(),
        path.cost
    );

    // 5. A* that routes around obstacles
    let blocked = grid.k_ring(grid.cell_at(2.5, 2.5, 2.5)?, 1);
    let detour = grid.astar_where(start, goal, |c| !blocked.contains(&c))?;
    println!(
        "5. With an obstacle in the middle: {} cells, {:.2} physical units",
        detour.len(),
        detour.cost
    );

    // 6. Morton-encoded Index64 keys for storage and range queries
    let index = Index64::new(0, 0, 5, 100, 200, 300)?;
    println!(
        "\n6. Index64 key: {} (bech32m: {})",
        index,
        index.to_bech32m()?
    );

    // 7. Route64 IDs round-trip through human-readable bech32m strings
    let route = Route64::new(0, 100, 200, 300)?;
    let encoded = route.to_bech32m()?;
    assert_eq!(Route64::from_bech32m(&encoded)?, route);
    println!("7. Route64 {} <-> {}", route, encoded);

    Ok(())
}
