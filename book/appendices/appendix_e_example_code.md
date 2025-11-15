# Appendix E: Example Code

This appendix gathers self-contained, runnable examples that illustrate how to:

- Initialize a frame registry and define custom frames
- Construct identifiers from coordinates and vice versa
- Build and query containers for common workloads
- Integrate OctaIndex3D with external systems (e.g., GIS, ML frameworks)
- Apply common patterns and avoid anti-patterns

Each example is complete and can be run with minimal setup. Reference the corresponding chapters for deeper explanations.

---

## E.1 Quick Start: First Steps with BCC

**Goal:** Create BCC indices, perform neighbor queries, and understand basic operations.

**File:** `examples/e1_quick_start.rs`

```rust
use octaindex3d::{Index64, BccCoord, FrameRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OctaIndex3D Quick Start ===\n");

    // Step 1: Create a frame registry
    let mut registry = FrameRegistry::new();
    let frame_id = registry.register_frame(
        "world",
        [0.0, 0.0, 0.0], // origin
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], // identity orientation
    )?;

    println!("Registered frame 'world' with ID: {}", frame_id);

    // Step 2: Create BCC coordinates
    // BCC lattice requires parity: (x + y + z) must be even
    let coord1 = BccCoord::new(0, 0, 0, 0)?; // LOD=0, x=0, y=0, z=0 (parity: even ✓)
    let coord2 = BccCoord::new(0, 1, 1, 0)?; // LOD=0, x=1, y=1, z=0 (parity: even ✓)

    println!("\nCreated BCC coordinates:");
    println!("  coord1: {:?}", coord1);
    println!("  coord2: {:?}", coord2);

    // Step 3: Convert to Index64 identifiers
    let idx1 = Index64::from_bcc_coord(frame_id, coord1)?;
    let idx2 = Index64::from_bcc_coord(frame_id, coord2)?;

    println!("\nIndex64 identifiers:");
    println!("  idx1: {}", idx1);
    println!("  idx2: {}", idx2);

    // Step 4: Find neighbors
    let neighbors = idx1.get_14_neighbors()?;

    println!("\ncoord1 has {} neighbors:", neighbors.len());
    for (i, neighbor) in neighbors.iter().take(5).enumerate() {
        println!("  [{}]: {}", i, neighbor);
    }
    println!("  ... ({} more)", neighbors.len() - 5);

    // Step 5: Check if coord2 is a neighbor of coord1
    let is_neighbor = neighbors.contains(&idx2);
    println!("\nIs coord2 a neighbor of coord1? {}", is_neighbor);

    // Step 6: Compute Euclidean distance
    let pos1 = idx1.to_position()?;
    let pos2 = idx2.to_position()?;
    let distance = ((pos1[0] - pos2[0]).powi(2)
                  + (pos1[1] - pos2[1]).powi(2)
                  + (pos1[2] - pos2[2]).powi(2)).sqrt();

    println!("\nPositions:");
    println!("  pos1: [{:.2}, {:.2}, {:.2}]", pos1[0], pos1[1], pos1[2]);
    println!("  pos2: [{:.2}, {:.2}, {:.2}]", pos2[0], pos2[1], pos2[2]);
    println!("  Distance: {:.2}", distance);

    Ok(())
}
```

**Run:**
```bash
cargo run --example e1_quick_start
```

**See:** Chapter 1 (Introduction), Chapter 2 (Mathematical Foundations), Chapter 5 (Identifier Types)

---

## E.2 Building and Querying Containers

**Goal:** Create a container, insert spatial data, and perform range queries.

**File:** `examples/e2_container_usage.rs`

```rust
use octaindex3d::{Index64, BccCoord, FrameRegistry, SequentialContainer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Container Building and Querying ===\n");

    // Initialize frame
    let mut registry = FrameRegistry::new();
    let frame_id = registry.register_frame("sensor_frame", [0.0, 0.0, 0.0],
                                           [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])?;

    // Create container
    let mut container = SequentialContainer::new();

    println!("Inserting 1000 random points into BCC grid...");

    // Insert points in a 10×10×10 BCC grid (LOD=0)
    for x in -5..5 {
        for y in -5..5 {
            for z in -5..5 {
                // Ensure parity constraint
                if (x + y + z) % 2 == 0 {
                    let coord = BccCoord::new(0, x, y, z)?;
                    let idx = Index64::from_bcc_coord(frame_id, coord)?;

                    // Insert with metadata (e.g., occupancy value)
                    let occupancy: f32 = (x * y * z) as f32 / 125.0;
                    container.insert(idx, &occupancy.to_le_bytes())?;
                }
            }
        }
    }

    println!("Inserted {} entries\n", container.len());

    // Query 1: Range query (all points within a bounding box)
    let min_coord = BccCoord::new(0, -2, -2, -2)?;
    let max_coord = BccCoord::new(0, 2, 2, 2)?;
    let min_idx = Index64::from_bcc_coord(frame_id, min_coord)?;
    let max_idx = Index64::from_bcc_coord(frame_id, max_coord)?;

    println!("Range query: bounding box [-2,-2,-2] to [2,2,2]");
    let range_results = container.range_query(min_idx, max_idx)?;
    println!("  Found {} points in range\n", range_results.len());

    // Query 2: Neighbor query (all occupied neighbors of origin)
    let origin = Index64::from_bcc_coord(frame_id, BccCoord::new(0, 0, 0, 0)?)?;
    let neighbors = origin.get_14_neighbors()?;

    let mut occupied_neighbors = Vec::new();
    for neighbor in neighbors {
        if container.contains(&neighbor) {
            occupied_neighbors.push(neighbor);
        }
    }

    println!("Neighbor query: origin (0,0,0)");
    println!("  {} of 14 neighbors are occupied\n", occupied_neighbors.len());

    // Query 3: K-nearest neighbors (simplified: radius search)
    let query_point = Index64::from_bcc_coord(frame_id, BccCoord::new(0, 0, 0, 0)?)?;
    let radius = 2; // LOD levels

    println!("Radius query: {} LOD levels from origin", radius);
    let mut nearby = Vec::new();

    // BFS-style expansion (simplified)
    let mut queue = vec![query_point];
    let mut visited = std::collections::HashSet::new();

    for _ in 0..radius {
        let mut next_queue = Vec::new();
        for idx in queue {
            if visited.insert(idx) && container.contains(&idx) {
                nearby.push(idx);
            }
            next_queue.extend(idx.get_14_neighbors()?);
        }
        queue = next_queue;
    }

    println!("  Found {} points within {} steps\n", nearby.len(), radius);

    // Serialize container to file
    let path = "/tmp/bcc_container.bin";
    container.save(path)?;
    println!("Saved container to {}", path);

    // Load container from file
    let loaded = SequentialContainer::load(path)?;
    println!("Loaded container: {} entries", loaded.len());

    Ok(())
}
```

**Run:**
```bash
cargo run --example e2_container_usage
```

**See:** Chapter 8 (Container Formats), Chapter 3 (Hierarchical Structures)

---

## E.3 Multi-Resolution Hierarchical Queries

**Goal:** Create a multi-LOD grid and perform hierarchical refinement.

**File:** `examples/e3_multi_resolution.rs`

```rust
use octaindex3d::{Index64, BccCoord, FrameRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Resolution Hierarchical Queries ===\n");

    let mut registry = FrameRegistry::new();
    let frame_id = registry.register_frame("world", [0.0, 0.0, 0.0],
                                           [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])?;

    // Create coarse grid (LOD=0)
    let coarse_coord = BccCoord::new(0, 0, 0, 0)?;
    let coarse_idx = Index64::from_bcc_coord(frame_id, coarse_coord)?;

    println!("Coarse cell at LOD=0: {}", coarse_idx);

    // Refine to LOD=1 (8 children)
    println!("\nRefining to LOD=1 (8 children):");
    let children_lod1 = coarse_idx.get_children()?;

    for (i, child) in children_lod1.iter().enumerate() {
        let coord = child.to_bcc_coord()?;
        println!("  Child {}: {} -> {:?}", i, child, coord);
    }

    // Refine one child to LOD=2
    println!("\nRefining child 0 to LOD=2 (8 grandchildren):");
    let grandchildren = children_lod1[0].get_children()?;

    for (i, grandchild) in grandchildren.iter().take(4).enumerate() {
        let coord = grandchild.to_bcc_coord()?;
        println!("  Grandchild {}: {} -> {:?}", i, grandchild, coord);
    }
    println!("  ... (4 more grandchildren)");

    // Navigate upward (child → parent)
    println!("\nNavigating upward:");
    let child = children_lod1[0];
    let parent = child.get_parent()?;

    println!("  Child:  {}", child);
    println!("  Parent: {}", parent);
    println!("  Parent matches coarse? {}", parent == coarse_idx);

    // Find cross-LOD neighbors
    println!("\nCross-LOD neighbors (LOD=1 child with LOD=0 neighbors):");
    let child_neighbors = children_lod1[0].get_14_neighbors()?;

    for neighbor in child_neighbors.iter().take(3) {
        let neighbor_coord = neighbor.to_bcc_coord()?;
        println!("  Neighbor: {} (LOD={})", neighbor, neighbor_coord.lod());
    }
    println!("  ... ({} total neighbors)", child_neighbors.len());

    // Adaptive refinement example
    println!("\nAdaptive refinement (refine only if occupancy > threshold):");

    struct Cell {
        idx: Index64,
        occupancy: f32,
    }

    let mut cells = vec![Cell { idx: coarse_idx, occupancy: 0.8 }];
    let threshold = 0.5;

    // Refine cells with high occupancy
    let mut refined_cells = Vec::new();
    for cell in cells {
        if cell.occupancy > threshold {
            println!("  Refining cell {} (occupancy={:.2})", cell.idx, cell.occupancy);
            let children = cell.idx.get_children()?;
            for child in children {
                // Simulate occupancy for children
                let child_occupancy = cell.occupancy * 0.7;
                refined_cells.push(Cell { idx: child, occupancy: child_occupancy });
            }
        } else {
            println!("  Keeping cell {} (occupancy={:.2})", cell.idx, cell.occupancy);
            refined_cells.push(cell);
        }
    }

    println!("\nRefined grid has {} cells", refined_cells.len());

    Ok(())
}
```

**Run:**
```bash
cargo run --example e3_multi_resolution
```

**See:** Chapter 3 (Hierarchical Structures), Chapter 10 (Robotics - Occupancy Grids)

---

## E.4 Coordinate System Transformations

**Goal:** Transform points between different coordinate frames.

**File:** `examples/e4_coordinate_transforms.rs`

```rust
use octaindex3d::{Index64, FrameRegistry, BccCoord};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Coordinate System Transformations ===\n");

    let mut registry = FrameRegistry::new();

    // Define world frame (ECEF-like, meters)
    let world_frame = registry.register_frame(
        "world",
        [0.0, 0.0, 0.0],
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    )?;

    // Define robot frame (offset and rotated 90° around Z)
    let robot_frame = registry.register_frame(
        "robot",
        [10.0, 5.0, 0.0], // origin offset
        [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], // 90° CCW around Z
    )?;

    // Define sensor frame (offset from robot)
    let sensor_frame = registry.register_frame_with_parent(
        "lidar",
        robot_frame,
        [0.5, 0.0, 1.2], // 0.5m forward, 1.2m up from robot
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    )?;

    println!("Registered frames:");
    println!("  World:  {}", world_frame);
    println!("  Robot:  {}", robot_frame);
    println!("  Sensor: {}", sensor_frame);

    // Create point in sensor frame
    let sensor_coord = BccCoord::new(0, 2, 0, 0)?; // 2 units in +X (forward)
    let sensor_idx = Index64::from_bcc_coord(sensor_frame, sensor_coord)?;

    println!("\nPoint in sensor frame:");
    println!("  Index: {}", sensor_idx);
    let sensor_pos = sensor_idx.to_position()?;
    println!("  Position (sensor): [{:.2}, {:.2}, {:.2}]",
             sensor_pos[0], sensor_pos[1], sensor_pos[2]);

    // Transform to robot frame
    let robot_idx = registry.transform(sensor_idx, robot_frame)?;
    let robot_pos = robot_idx.to_position()?;

    println!("\nTransformed to robot frame:");
    println!("  Index: {}", robot_idx);
    println!("  Position (robot): [{:.2}, {:.2}, {:.2}]",
             robot_pos[0], robot_pos[1], robot_pos[2]);

    // Transform to world frame
    let world_idx = registry.transform(sensor_idx, world_frame)?;
    let world_pos = world_idx.to_position()?;

    println!("\nTransformed to world frame:");
    println!("  Index: {}", world_idx);
    println!("  Position (world): [{:.2}, {:.2}, {:.2}]",
             world_pos[0], world_pos[1], world_pos[2]);

    // Batch transformation example
    println!("\n--- Batch Transformation ---");

    let sensor_points = vec![
        BccCoord::new(0, 0, 0, 0)?,
        BccCoord::new(0, 2, 0, 0)?,
        BccCoord::new(0, 0, 2, 0)?,
        BccCoord::new(0, 2, 2, 0)?,
    ];

    println!("Transforming {} points from sensor → world:", sensor_points.len());

    for (i, coord) in sensor_points.iter().enumerate() {
        let sensor_idx = Index64::from_bcc_coord(sensor_frame, *coord)?;
        let world_idx = registry.transform(sensor_idx, world_frame)?;
        let world_pos = world_idx.to_position()?;

        println!("  Point {}: sensor {} → world [{:.2}, {:.2}, {:.2}]",
                 i, sensor_idx, world_pos[0], world_pos[1], world_pos[2]);
    }

    Ok(())
}
```

**Run:**
```bash
cargo run --example e4_coordinate_transforms
```

**See:** Chapter 6 (Coordinate Systems), Chapter 10 (Robotics - ROS Integration)

---

## E.5 Streaming Container for Real-Time Logging

**Goal:** Use streaming containers for high-throughput sensor data logging.

**File:** `examples/e5_streaming_container.rs`

```rust
use octaindex3d::{Index64, BccCoord, FrameRegistry, StreamingContainer};
use std::time::{Instant, Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Streaming Container for Real-Time Logging ===\n");

    let mut registry = FrameRegistry::new();
    let frame_id = registry.register_frame("lidar_frame", [0.0, 0.0, 0.0],
                                           [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])?;

    // Create streaming container (write-optimized, append-only)
    let mut stream = StreamingContainer::new("/tmp/lidar_stream.bcc")?;

    println!("Simulating LiDAR sensor at 10Hz with 1000 points/scan...\n");

    let scan_rate = Duration::from_millis(100); // 10Hz
    let points_per_scan = 1000;
    let num_scans = 10;

    let start_time = Instant::now();

    for scan_id in 0..num_scans {
        let scan_start = Instant::now();

        // Simulate LiDAR scan (1000 random points)
        for point_id in 0..points_per_scan {
            // Generate pseudo-random BCC coordinates
            let x = ((point_id * 17 + scan_id * 31) % 20) as i32 - 10;
            let y = ((point_id * 13 + scan_id * 23) % 20) as i32 - 10;
            let z = ((point_id * 11 + scan_id * 19) % 20) as i32 - 10;

            // Ensure parity
            let z = if (x + y + z) % 2 == 0 { z } else { z + 1 };

            let coord = BccCoord::new(0, x, y, z)?;
            let idx = Index64::from_bcc_coord(frame_id, coord)?;

            // Metadata: intensity value
            let intensity: u16 = ((point_id * 257) % 65536) as u16;

            // Append to stream (lock-free, low-latency write)
            stream.append(idx, &intensity.to_le_bytes())?;
        }

        let scan_duration = scan_start.elapsed();
        println!("Scan {:2}: {} points in {:4}ms",
                 scan_id, points_per_scan, scan_duration.as_millis());

        // Wait for next scan (if faster than real-time)
        let remaining = scan_rate.saturating_sub(scan_duration);
        std::thread::sleep(remaining);
    }

    let total_duration = start_time.elapsed();
    let total_points = num_scans * points_per_scan;

    println!("\n=== Performance Summary ===");
    println!("Total points:  {}", total_points);
    println!("Total time:    {:.2}s", total_duration.as_secs_f64());
    println!("Throughput:    {:.0} points/sec",
             total_points as f64 / total_duration.as_secs_f64());
    println!("Average write: {:.2}µs/point",
             total_duration.as_micros() as f64 / total_points as f64);

    // Flush and close
    stream.flush()?;
    println!("\nStream flushed and closed");

    // Convert to sequential container for querying
    println!("\nConverting stream → sequential container for queries...");
    let sequential = stream.to_sequential()?;
    println!("Sequential container: {} entries", sequential.len());

    // Now can perform queries
    let origin = Index64::from_bcc_coord(frame_id, BccCoord::new(0, 0, 0, 0)?)?;
    let neighbors = origin.get_14_neighbors()?;

    let mut occupied = 0;
    for neighbor in neighbors {
        if sequential.contains(&neighbor) {
            occupied += 1;
        }
    }

    println!("Neighbor query at origin: {} of 14 occupied", occupied);

    Ok(())
}
```

**Run:**
```bash
cargo run --example e5_streaming_container
```

**See:** Chapter 8 (Container Formats - Streaming), Chapter 10 (Robotics Applications)

---

## E.6 GIS Integration: WGS84 and GeoJSON Export

**Goal:** Integrate with GIS workflows using WGS84 coordinates and GeoJSON.

**File:** `examples/e6_gis_integration.rs`

```rust
use octaindex3d::{Index64, BccCoord, FrameRegistry};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== GIS Integration: WGS84 and GeoJSON ===\n");

    let mut registry = FrameRegistry::new();

    // Define Earth frame (ECEF coordinates, meters)
    let earth_frame = registry.register_frame_wgs84("earth")?;

    println!("Registered Earth frame (WGS84/ECEF)\n");

    // Example: San Francisco coordinates
    let lat = 37.7749;  // degrees North
    let lon = -122.4194; // degrees West
    let alt = 10.0;      // meters above sea level

    println!("Input: San Francisco");
    println!("  Lat: {:.4}°", lat);
    println!("  Lon: {:.4}°", lon);
    println!("  Alt: {:.1}m\n", alt);

    // Convert WGS84 → ECEF
    let (x, y, z) = wgs84_to_ecef(lat, lon, alt);
    println!("ECEF coordinates:");
    println!("  X: {:.2}m", x);
    println!("  Y: {:.2}m", y);
    println!("  Z: {:.2}m\n", z);

    // Snap to BCC grid (LOD=10 for ~1m resolution at Earth scale)
    let lod = 10;
    let bcc_x = (x / (1 << lod) as f64).round() as i32;
    let bcc_y = (y / (1 << lod) as f64).round() as i32;
    let bcc_z = (z / (1 << lod) as f64).round() as i32;

    // Ensure parity
    let bcc_z = if (bcc_x + bcc_y + bcc_z) % 2 == 0 { bcc_z } else { bcc_z + 1 };

    let coord = BccCoord::new(lod as u8, bcc_x, bcc_y, bcc_z)?;
    let idx = Index64::from_bcc_coord(earth_frame, coord)?;

    println!("BCC index (LOD={}):", lod);
    println!("  Index: {}", idx);
    println!("  Coord: {:?}\n", coord);

    // Convert back to position and then to WGS84
    let pos = idx.to_position()?;
    let (lat2, lon2, alt2) = ecef_to_wgs84(pos[0], pos[1], pos[2]);

    println!("Roundtrip (BCC → ECEF → WGS84):");
    println!("  Lat: {:.4}° (error: {:.6}°)", lat2, lat2 - lat);
    println!("  Lon: {:.4}° (error: {:.6}°)", lon2, lon2 - lon);
    println!("  Alt: {:.1}m (error: {:.2}m)\n", alt2, alt2 - alt);

    // Export to GeoJSON
    println!("Exporting to GeoJSON...");

    let geojson = json!({
        "type": "Feature",
        "geometry": {
            "type": "Point",
            "coordinates": [lon2, lat2, alt2]
        },
        "properties": {
            "bcc_index": idx.to_string(),
            "lod": lod,
            "ecef_x": pos[0],
            "ecef_y": pos[1],
            "ecef_z": pos[2],
        }
    });

    println!("{}", serde_json::to_string_pretty(&geojson)?);

    // Save GeoJSON to file
    let path = "/tmp/bcc_point.geojson";
    std::fs::write(path, serde_json::to_string_pretty(&geojson)?)?;
    println!("\nSaved GeoJSON to {}", path);

    Ok(())
}

// WGS84 → ECEF conversion (simplified, assumes WGS84 ellipsoid)
fn wgs84_to_ecef(lat: f64, lon: f64, alt: f64) -> (f64, f64, f64) {
    let a = 6378137.0; // WGS84 semi-major axis (meters)
    let e2 = 0.00669437999014; // WGS84 eccentricity squared

    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();

    let n = a / (1.0 - e2 * lat_rad.sin().powi(2)).sqrt();

    let x = (n + alt) * lat_rad.cos() * lon_rad.cos();
    let y = (n + alt) * lat_rad.cos() * lon_rad.sin();
    let z = (n * (1.0 - e2) + alt) * lat_rad.sin();

    (x, y, z)
}

// ECEF → WGS84 conversion (Bowring method, simplified)
fn ecef_to_wgs84(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let a = 6378137.0;
    let e2 = 0.00669437999014;

    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);

    let lat = (z / p).atan();
    // Iterative refinement (simplified, 3 iterations)
    for _ in 0..3 {
        let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
        let lat_new = (z + e2 * n * lat.sin()).atan2(p);
        if (lat_new - lat).abs() < 1e-12 {
            break;
        }
        lat = lat_new;
    }

    let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
    let alt = p / lat.cos() - n;

    (lat.to_degrees(), lon.to_degrees(), alt)
}
```

**Run:**
```bash
cargo run --example e6_gis_integration
```

**See:** Chapter 6 (Coordinate Systems - WGS84), Chapter 11 (Geospatial Analysis)

---

## E.7 Common Patterns and Best Practices

### Pattern 1: Lazy Container Loading

**Problem:** Loading large containers into memory is slow and memory-intensive.

**Solution:** Use memory-mapped files or streaming containers for on-demand access.

```rust
use octaindex3d::{SequentialContainer, MemoryMappedContainer};

// ❌ BAD: Load entire container into memory
let container = SequentialContainer::load("/large/dataset.bcc")?;

// ✅ GOOD: Memory-map for lazy loading
let container = MemoryMappedContainer::open("/large/dataset.bcc")?;
```

### Pattern 2: Batch Operations

**Problem:** Repeated single-point queries have high overhead.

**Solution:** Batch queries together for better cache performance.

```rust
// ❌ BAD: Query neighbors one at a time
for idx in indices {
    let neighbors = idx.get_14_neighbors()?;
    process(neighbors);
}

// ✅ GOOD: Batch neighbor queries
let all_neighbors = Index64::batch_get_neighbors(&indices)?;
process_batch(all_neighbors);
```

### Pattern 3: Pre-allocate Containers

**Problem:** Growing containers dynamically causes frequent reallocations.

**Solution:** Pre-allocate capacity if size is known.

```rust
// ❌ BAD: Let container grow dynamically
let mut container = SequentialContainer::new();
for idx in large_dataset {
    container.insert(idx, data)?;
}

// ✅ GOOD: Pre-allocate capacity
let mut container = SequentialContainer::with_capacity(expected_size);
for idx in large_dataset {
    container.insert(idx, data)?;
}
```

### Pattern 4: Error Handling in Production

**Problem:** Using `.unwrap()` causes panics in production.

**Solution:** Propagate errors with `?` operator or handle gracefully.

```rust
// ❌ BAD: Panic on invalid input
let coord = BccCoord::new(0, x, y, z).unwrap();

// ✅ GOOD: Propagate or handle errors
let coord = BccCoord::new(0, x, y, z)?;

// ✅ ALSO GOOD: Validate and fix parity
let z = if (x + y + z) % 2 == 0 { z } else { z + 1 };
let coord = BccCoord::new(0, x, y, z)?;
```

---

## E.8 Anti-Patterns to Avoid

### Anti-Pattern 1: Ignoring Parity Constraint

**Problem:** Creating coordinates without checking parity causes errors.

```rust
// ❌ BAD: Assume all (x, y, z) are valid
let coord = BccCoord::new(0, 1, 1, 1)?; // Error: parity is odd!
```

**Solution:** Always ensure `(x + y + z) % 2 == 0` or use helper methods.

```rust
// ✅ GOOD: Validate parity before construction
let z = if (x + y + z) % 2 == 0 { z } else { z + 1 };
let coord = BccCoord::new(0, x, y, z)?;
```

### Anti-Pattern 2: Mixing Frame IDs

**Problem:** Performing operations on indices from different frames.

```rust
// ❌ BAD: Mix indices from different frames
let idx1 = Index64::from_bcc_coord(frame_a, coord1)?;
let idx2 = Index64::from_bcc_coord(frame_b, coord2)?;
let distance = idx1.distance_to(idx2); // Meaningless!
```

**Solution:** Transform to common frame before comparison.

```rust
// ✅ GOOD: Transform to common frame
let idx2_in_frame_a = registry.transform(idx2, frame_a)?;
let distance = idx1.distance_to(idx2_in_frame_a);
```

### Anti-Pattern 3: Sequential Writes to Streaming Container

**Problem:** Calling `flush()` after every write negates streaming benefits.

```rust
// ❌ BAD: Flush after every write
for point in points {
    stream.append(idx, data)?;
    stream.flush()?; // Slow!
}
```

**Solution:** Batch writes and flush periodically or at end.

```rust
// ✅ GOOD: Batch writes
for point in points {
    stream.append(idx, data)?;
}
stream.flush()?; // Flush once at end
```

### Anti-Pattern 4: Over-Refining LOD

**Problem:** Refining to unnecessarily high LOD wastes memory.

```rust
// ❌ BAD: Refine everything to LOD=15
let lod = 15; // 2^15 = 32768× resolution!
```

**Solution:** Use adaptive refinement based on occupancy or error metrics.

```rust
// ✅ GOOD: Adaptive refinement
if occupancy > threshold && lod < max_lod {
    refine_cell(cell, lod + 1);
}
```

---

## E.9 Integration Examples

### Example: Bevy Game Engine Integration

```rust
use bevy::prelude::*;
use octaindex3d::{Index64, SequentialContainer};

#[derive(Component)]
struct BccVoxel {
    idx: Index64,
}

fn spawn_voxels(mut commands: Commands, container: Res<SequentialContainer>) {
    for (idx, _data) in container.iter() {
        let pos = idx.to_position().unwrap();

        commands.spawn((
            BccVoxel { idx },
            TransformBundle::from(Transform::from_xyz(pos[0], pos[1], pos[2])),
        ));
    }
}
```

**See:** Chapter 13 (Gaming and Virtual Worlds)

### Example: PyTorch Integration (via PyO3)

```rust
use pyo3::prelude::*;
use octaindex3d::SequentialContainer;

#[pyfunction]
fn load_bcc_dataset(path: String) -> PyResult<Vec<Vec<f32>>> {
    let container = SequentialContainer::load(&path)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))?;

    let mut features = Vec::new();
    for (idx, data) in container.iter() {
        let pos = idx.to_position().unwrap();
        features.push(vec![pos[0], pos[1], pos[2], /* ...data... */]);
    }

    Ok(features)
}

#[pymodule]
fn octaindex3d_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_bcc_dataset, m)?)?;
    Ok(())
}
```

**See:** Chapter 15 (Machine Learning Integration)

---

## E.10 Further Reading

For additional examples and patterns:

- **Source Repository:** https://github.com/FunKite/OctaIndex3D/tree/main/examples
- **API Documentation:** https://docs.rs/octaindex3d
- **Integration Cookbook:** Appendix H
- **Performance Tuning:** Appendix G

Each chapter in the main book includes dedicated examples. Refer to the chapter summaries for topic-specific code samples.

