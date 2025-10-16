//! Profiling harness for identifying performance hotspots
//!
//! This exercises the hot paths to identify optimization opportunities.
//! Run with: cargo run --release --example profile_hotspots --features parallel

use octaindex3d::performance::*;
use octaindex3d::{morton, Index64, Route64};
use std::time::Instant;

fn main() {
    println!("=== OctaIndex3D Profiling Harness (Apple Silicon) ===\n");

    // Warm up
    warmup();

    // Profile each operation category
    profile_morton_operations();
    profile_index64_operations();
    profile_route64_operations();
    profile_neighbor_operations();
    profile_distance_operations();
    profile_spatial_queries();

    println!("\n=== Profiling Complete ===");
}

fn warmup() {
    println!("Warming up...");
    let routes: Vec<Route64> = (0..1000)
        .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
        .collect();

    let _ = batch_neighbors_auto(&routes);
    println!("Warmup complete\n");
}

fn profile_morton_operations() {
    println!("=== Profiling Morton Encode/Decode ===");

    // Generate test data
    let coords: Vec<(u16, u16, u16)> = (0..100_000)
        .map(|i| {
            (
                (i % 65536) as u16,
                ((i * 2) % 65536) as u16,
                ((i * 3) % 65536) as u16,
            )
        })
        .collect();

    // Profile encoding
    let start = Instant::now();
    let mut total = 0u64;
    for _ in 0..100 {
        let encoded = batch_morton_encode(&coords);
        total += encoded[0]; // Prevent optimization
    }
    let elapsed = start.elapsed();
    println!(
        "  Morton encode (100K x 100): {:?} ({} ops/sec)",
        elapsed,
        (100_000 * 100) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}", total);

    // Profile decoding
    let codes: Vec<u64> = coords
        .iter()
        .map(|&(x, y, z)| morton::morton_encode(x, y, z))
        .collect();

    let start = Instant::now();
    let mut total_x = 0u16;
    for _ in 0..100 {
        let decoded = batch_morton_decode(&codes);
        total_x = total_x.wrapping_add(decoded[0].0);
    }
    let elapsed = start.elapsed();
    println!(
        "  Morton decode (100K x 100): {:?} ({} ops/sec)",
        elapsed,
        (100_000 * 100) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}\n", total_x);
}

fn profile_index64_operations() {
    println!("=== Profiling Index64 Operations ===");

    let count = 50_000;
    let frame_ids = vec![0u8; count];
    let tiers = vec![0u8; count];
    let lods = vec![0u8; count];
    let coords: Vec<(u16, u16, u16)> = (0..count)
        .map(|i| {
            (
                (i % 1000) as u16,
                ((i * 2) % 1000) as u16,
                ((i * 3) % 1000) as u16,
            )
        })
        .collect();

    // Profile batch encoding
    let start = Instant::now();
    for _ in 0..200 {
        let _ = batch_index64_encode(&frame_ids, &tiers, &lods, &coords).unwrap();
    }
    let elapsed = start.elapsed();
    println!(
        "  Index64 batch encode (50K x 200): {:?} ({} ops/sec)",
        elapsed,
        (count * 200) as f64 / elapsed.as_secs_f64()
    );

    // Profile batch decoding
    let indices: Vec<Index64> = coords
        .iter()
        .map(|&(x, y, z)| Index64::new(0, 0, 0, x, y, z).unwrap())
        .collect();

    let start = Instant::now();
    let mut total = 0u16;
    for _ in 0..200 {
        let decoded = batch_index64_decode(&indices);
        total = total.wrapping_add(decoded[0].0);
    }
    let elapsed = start.elapsed();
    println!(
        "  Index64 batch decode (50K x 200): {:?} ({} ops/sec)",
        elapsed,
        (count * 200) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}\n", total);
}

fn profile_route64_operations() {
    println!("=== Profiling Route64 Operations ===");

    let count = 50_000;
    let routes: Vec<Route64> = (0..count)
        .map(|i| {
            let coord = ((i % 10000) * 2) as i32;
            Route64::new(0, coord, coord, coord).unwrap()
        })
        .collect();

    // Profile validation
    let start = Instant::now();
    for _ in 0..200 {
        let _ = batch_validate_routes(&routes);
    }
    let elapsed = start.elapsed();
    println!(
        "  Route validation (50K x 200): {:?} ({} ops/sec)",
        elapsed,
        (count * 200) as f64 / elapsed.as_secs_f64()
    );

    // Profile coordinate extraction
    let start = Instant::now();
    let mut total = 0i32;
    for _ in 0..200 {
        for route in &routes {
            total = total.wrapping_add(route.x());
            total = total.wrapping_add(route.y());
            total = total.wrapping_add(route.z());
        }
    }
    let elapsed = start.elapsed();
    println!(
        "  Coordinate extraction (50K x 200 x 3): {:?} ({} ops/sec)",
        elapsed,
        (count * 200 * 3) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}\n", total);
}

fn profile_neighbor_operations() {
    println!("=== Profiling Neighbor Calculations ===");

    // Small batch (fast kernel)
    let small_routes: Vec<Route64> = (0..100)
        .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
        .collect();

    let start = Instant::now();
    for _ in 0..10_000 {
        let _ = batch_neighbors_auto(&small_routes);
    }
    let elapsed = start.elapsed();
    println!(
        "  Small batch (100 routes x 10K): {:?} ({} routes/sec)",
        elapsed,
        (100 * 10_000) as f64 / elapsed.as_secs_f64()
    );

    // Medium batch (cache-blocked)
    let medium_routes: Vec<Route64> = (0..1_000)
        .map(|i| Route64::new(0, (i % 5000) * 2, (i % 5000) * 2, (i % 5000) * 2).unwrap())
        .collect();

    let start = Instant::now();
    for _ in 0..1_000 {
        let _ = batch_neighbors_auto(&medium_routes);
    }
    let elapsed = start.elapsed();
    println!(
        "  Medium batch (1K routes x 1K): {:?} ({} routes/sec)",
        elapsed,
        (1_000 * 1_000) as f64 / elapsed.as_secs_f64()
    );

    // Large batch (parallel)
    #[cfg(feature = "parallel")]
    {
        let large_routes: Vec<Route64> = (0..10_000)
            .map(|i| Route64::new(0, (i % 5000) * 2, (i % 5000) * 2, (i % 5000) * 2).unwrap())
            .collect();

        let start = Instant::now();
        for _ in 0..100 {
            let _ = batch_neighbors_auto(&large_routes);
        }
        let elapsed = start.elapsed();
        println!(
            "  Large batch (10K routes x 100): {:?} ({} routes/sec)",
            elapsed,
            (10_000 * 100) as f64 / elapsed.as_secs_f64()
        );
    }

    // Single route (fast unrolled)
    let single = Route64::new(0, 1000, 1000, 1000).unwrap();
    let start = Instant::now();
    let mut total = 0u64;
    for _ in 0..10_000_000 {
        let neighbors = neighbors_route64_fast(single);
        total = total.wrapping_add(neighbors[0].value());
    }
    let elapsed = start.elapsed();
    println!(
        "  Single route (10M iterations): {:?} ({} ops/sec)",
        elapsed,
        10_000_000.0 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}\n", total);
}

fn profile_distance_operations() {
    println!("=== Profiling Distance Calculations ===");

    let source = Route64::new(0, 5000, 5000, 5000).unwrap();
    let targets: Vec<Route64> = (0..50_000)
        .map(|i| {
            let coord = ((i % 10000) * 2) as i32;
            Route64::new(0, coord, coord, coord).unwrap()
        })
        .collect();

    // Manhattan distance
    let start = Instant::now();
    let mut total = 0i32;
    for _ in 0..200 {
        let distances = batch_manhattan_distance(source, &targets);
        total = total.wrapping_add(distances[0]);
    }
    let elapsed = start.elapsed();
    println!(
        "  Manhattan distance (50K x 200): {:?} ({} ops/sec)",
        elapsed,
        (50_000 * 200) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}", total);

    // Euclidean distance squared
    let start = Instant::now();
    let mut total = 0i64;
    for _ in 0..200 {
        let distances = batch_euclidean_distance_squared(source, &targets);
        total = total.wrapping_add(distances[0]);
    }
    let elapsed = start.elapsed();
    println!(
        "  Euclidean² distance (50K x 200): {:?} ({} ops/sec)",
        elapsed,
        (50_000 * 200) as f64 / elapsed.as_secs_f64()
    );
    println!("  Prevention: {}\n", total);
}

fn profile_spatial_queries() {
    println!("=== Profiling Spatial Queries ===");

    let routes: Vec<Route64> = (0..100_000)
        .map(|i| {
            let x = ((i % 200) * 100) as i32;
            let y = (((i / 200) % 200) * 100) as i32;
            let z = (((i / 40000) % 200) * 100) as i32;
            Route64::new(0, x, y, z).unwrap()
        })
        .collect();

    // Bounding box query
    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..100 {
        let results = batch_bounding_box_query(&routes, 0, 10000, 0, 10000, 0, 10000);
        total = total.wrapping_add(results.len());
    }
    let elapsed = start.elapsed();
    println!(
        "  Bounding box query (100K routes x 100): {:?} ({} queries/sec)",
        elapsed,
        100.0 / elapsed.as_secs_f64()
    );
    println!("  Average matches per query: {}", total / 100);
    println!("  Prevention: {}\n", total);
}
