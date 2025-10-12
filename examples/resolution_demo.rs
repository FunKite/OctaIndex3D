//! Demonstration of resolution levels and hierarchical refinement

use octaindex3d::{CellID, Result};

fn main() -> Result<()> {
    println!("=== Resolution Demonstration ===\n");

    // Same physical location at different resolutions
    println!("1. Same Physical Location at Different Resolutions:");
    println!("   (Coordinates scale by 2× per resolution level)\n");

    let res0 = CellID::from_coords(0, 0, 1, 1, 1)?;
    let res1 = CellID::from_coords(0, 1, 2, 2, 2)?;
    let res2 = CellID::from_coords(0, 2, 4, 4, 4)?;
    let res3 = CellID::from_coords(0, 3, 8, 8, 8)?;
    let res4 = CellID::from_coords(0, 4, 16, 16, 16)?;

    println!(
        "   Resolution 0: coords ({}, {}, {}) - Base scale (1×)",
        res0.x(),
        res0.y(),
        res0.z()
    );
    println!(
        "   Resolution 1: coords ({}, {}, {}) - 2× finer (2^1)",
        res1.x(),
        res1.y(),
        res1.z()
    );
    println!(
        "   Resolution 2: coords ({}, {}, {}) - 4× finer (2^2)",
        res2.x(),
        res2.y(),
        res2.z()
    );
    println!(
        "   Resolution 3: coords ({}, {}, {}) - 8× finer (2^3)",
        res3.x(),
        res3.y(),
        res3.z()
    );
    println!(
        "   Resolution 4: coords ({}, {}, {}) - 16× finer (2^4)",
        res4.x(),
        res4.y(),
        res4.z()
    );

    // Parent-child relationships
    println!("\n2. Parent-Child Hierarchical Navigation:");
    println!("   (Child resolution = Parent resolution + 1)\n");

    let parent = CellID::from_coords(0, 5, 10, 10, 10)?;
    println!(
        "   Parent cell: res={}, coords=({}, {}, {})",
        parent.resolution(),
        parent.x(),
        parent.y(),
        parent.z()
    );

    let children = parent.children()?;
    println!(
        "   Children count: {} (BCC lattice has ~4 valid children)",
        children.len()
    );
    for (i, child) in children.iter().enumerate() {
        println!(
            "     Child {}: res={}, coords=({}, {}, {})",
            i + 1,
            child.resolution(),
            child.x(),
            child.y(),
            child.z()
        );
    }

    // Verify parent relationship
    let child = &children[0];
    let reconstructed_parent = child.parent()?;
    println!(
        "\n   Verify: child.parent() == original parent? {}",
        reconstructed_parent == parent
    );

    // Resolution refinement scale
    println!("\n3. Refinement Scale per Resolution:");
    println!("   (Shows 2^R multiplier)\n");

    for res in [0, 5, 10, 15, 20, 25, 30] {
        let refinement = 2_u64.pow(res as u32);
        println!(
            "   Resolution {:2}: 2^{:2} = {:>15} × refinement",
            res, res, refinement
        );
    }

    // Practical example: Earth mapping
    println!("\n4. Practical Example: Mapping Earth Surface");
    println!("   (Assuming 1 unit at res 0 = 5000 km)\n");

    let scales = [
        (0, 5000.0, "continent"),
        (5, 156.25, "region"),
        (10, 4.88, "city"),
        (15, 0.153, "building"),
        (20, 0.0048, "room"),
        (25, 0.00015, "object"),
        (30, 0.0000047, "microscopic"),
    ];

    for (res, km_per_unit, description) in scales {
        let m_per_unit = km_per_unit * 1000.0;
        if m_per_unit >= 1000.0 {
            println!(
                "   Res {:2}: ~{:>10.1} km  per unit ({})",
                res, km_per_unit, description
            );
        } else if m_per_unit >= 1.0 {
            println!(
                "   Res {:2}: ~{:>10.1} m   per unit ({})",
                res, m_per_unit, description
            );
        } else {
            let mm_per_unit = m_per_unit * 1000.0;
            println!(
                "   Res {:2}: ~{:>10.3} mm  per unit ({})",
                res, mm_per_unit, description
            );
        }
    }

    // Coordinate range implications
    println!("\n5. Coordinate Range at Different Resolutions:");
    println!("   (32-bit coords: ±2.1 billion range)\n");

    let max_coord = 2_147_483_648_i64; // 2^31

    for res in [0, 10, 20, 30] {
        let refinement = 2_u64.pow(res as u32);
        let physical_range_units = max_coord / refinement as i64;

        println!(
            "   Res {:2}: ±{:>12} physical units per axis",
            res, physical_range_units
        );
        if res == 20 {
            println!(
                "          (e.g., if 1 unit = 1m, range = ±{} km)",
                physical_range_units / 1000
            );
        }
    }

    // Resolution limits
    println!("\n6. Practical Resolution Limits:");
    println!("   Most applications should use resolutions 0-30\n");

    println!("   ✅ Res 0-10:  Macro scale (continents → buildings)");
    println!("   ✅ Res 10-20: Human scale (buildings → millimeters)");
    println!("   ✅ Res 20-30: Micro scale (millimeters → micrometers)");
    println!("   ⚠️  Res 30+:   Impractical (atomic/quantum scale)");
    println!("   ❌ Res 100+:  Physically meaningless");

    println!("\n=== Demonstration Complete ===");
    Ok(())
}
