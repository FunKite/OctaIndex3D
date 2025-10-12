//! Basic usage examples for OctaIndex3D

use octaindex3d::layer::{Aggregation, Layer};
use octaindex3d::path::{astar, k_ring, k_shell, EuclideanCost};
use octaindex3d::{CellID, Result};

fn main() -> Result<()> {
    println!("=== OctaIndex3D v0.2.0 Examples ===\n");

    // 1. Create cells
    println!("1. Creating cells:");
    let cell = CellID::from_coords(0, 5, 0, 0, 0)?;
    println!("   Cell: {}", cell);
    println!(
        "   Frame: {}, Resolution: {}",
        cell.frame(),
        cell.resolution()
    );

    // 2. Bech32m encoding
    println!("\n2. Bech32m encoding:");
    let encoded = cell.to_bech32m()?;
    println!("   Encoded: {}", encoded);
    let decoded = CellID::from_bech32m(&encoded)?;
    println!("   Decoded matches: {}", decoded == cell);

    // 3. Neighbors
    println!("\n3. Neighbors (14 in BCC lattice):");
    let neighbors = cell.neighbors();
    println!("   Count: {}", neighbors.len());
    println!("   First 3: {:?}", &neighbors[..3]);

    // 4. Hierarchy
    println!("\n4. Hierarchical navigation:");
    let parent = cell.parent()?;
    println!("   Parent resolution: {}", parent.resolution());
    let children = cell.children()?;
    println!(
        "   Children count: {} (BCC lattice constraint)",
        children.len()
    );

    // 5. K-ring
    println!("\n5. K-ring (all cells within k steps):");
    let ring = k_ring(cell, 2);
    println!("   K-ring(2): {} cells", ring.len());

    // 6. K-shell
    println!("\n6. K-shell (cells at exactly k steps):");
    let shell = k_shell(cell, 2);
    println!("   K-shell(2): {} cells", shell.len());

    // 7. Pathfinding
    println!("\n7. A* pathfinding:");
    let start = CellID::from_coords(0, 5, 0, 0, 0)?;
    let goal = CellID::from_coords(0, 5, 10, 10, 10)?;
    let path = astar(start, goal, &EuclideanCost)?;
    println!("   Path length: {} cells", path.cells.len());
    println!("   Path cost: {:.2}", path.cost);
    println!(
        "   Path: {:?}",
        path.cells
            .iter()
            .map(|c| (c.x(), c.y(), c.z()))
            .collect::<Vec<_>>()
    );

    // 8. Data layers
    println!("\n8. Data layers:");
    let mut temp_layer = Layer::new("temperature");

    // Add temperature data
    for c in k_ring(cell, 1) {
        temp_layer.set(c, 20.0 + (c.x() as f64) * 0.5);
    }

    println!("   Layer size: {} cells", temp_layer.len());
    if let Some(temp) = temp_layer.get(&cell) {
        println!("   Temperature at origin: {:.1}°C", temp);
    }

    // Aggregate
    let cells = k_ring(cell, 1);
    let avg = temp_layer.aggregate(&cells, Aggregation::Mean)?;
    println!("   Average temperature: {:.1}°C", avg);

    // 9. Coordinate range demo
    println!("\n9. Coordinate range (v0.2.0 - 32-bit):");
    let large_cell = CellID::from_coords(0, 10, 1_000_000, 2_000_000, -500_000)?;
    println!(
        "   Large coordinates: ({}, {}, {})",
        large_cell.x(),
        large_cell.y(),
        large_cell.z()
    );
    println!("   Range: ±2.1 billion per axis!");

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}
