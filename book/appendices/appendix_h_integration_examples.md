# Appendix H: Integration Examples

> Status note: Several examples in this appendix are conceptual integration patterns. For compile-verified repo examples, use `/examples` and cross-check symbol names in `book/API_CONTRACT.md`.

This appendix provides **end-to-end integration examples** showing how OctaIndex3D works with popular tools, libraries, and frameworks across different domains.

Rather than exhaustive API coverage, these examples demonstrate practical integration patterns you can adapt to your projects. Each example includes working code, setup instructions, and common pitfalls.

---

## H.1 Rust Ecosystem Integration

### H.1.1 nalgebra for Transforms and Camera Models

**Use case:** Integrate BCC spatial indexing with nalgebra's linear algebra for robotics, computer vision, or rendering.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
nalgebra = "0.32"
```rust

**Example: Camera frustum culling with BCC**

```rust
use nalgebra::{Matrix4, Point3, Vector3};
use octaindex3d::{Index64, BccCoord, Frame, FrameRegistry};
use std::collections::HashSet;

/// Camera with perspective projection
struct Camera {
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
    view_projection: Matrix4<f32>,
}

impl Camera {
    fn new(eye: Point3<f32>, target: Point3<f32>, fov: f32, aspect: f32) -> Self {
        let view = Matrix4::look_at_rh(&eye, &target, &Vector3::y());
        let projection = Matrix4::new_perspective(aspect, fov, 0.1, 1000.0);
        let view_projection = projection * view;

        Camera {
            view,
            projection,
            view_projection,
        }
    }

    /// Check if BCC cell (in world space) is in frustum
    fn is_in_frustum(&self, center: Point3<f32>, radius: f32) -> bool {
        // Transform center to clip space
        let clip_pos = self.view_projection * center.to_homogeneous();

        // Perspective divide
        let w = clip_pos.w;
        if w <= 0.0 {
            return false; // Behind camera
        }

        let ndc_x = clip_pos.x / w;
        let ndc_y = clip_pos.y / w;
        let ndc_z = clip_pos.z / w;

        // Check against NDC bounds (with radius tolerance)
        let tolerance = radius / w;
        ndc_x.abs() <= 1.0 + tolerance
            && ndc_y.abs() <= 1.0 + tolerance
            && ndc_z >= -1.0 - tolerance
            && ndc_z <= 1.0 + tolerance
    }
}

/// Frustum cull BCC cells for rendering
fn frustum_cull_bcc(
    camera: &Camera,
    container: &octaindex3d::Container,
    lod: u8,
    cell_size: f32,
) -> Vec<Index64> {
    let mut visible = Vec::new();

    for index in container.iter_lod(lod) {
        let coord = index.to_bcc_coord();
        let (x, y, z) = coord.as_tuple();

        // Convert BCC coordinate to world position
        let world_pos = Point3::new(
            x as f32 * cell_size,
            y as f32 * cell_size,
            z as f32 * cell_size,
        );

        // Bounding sphere radius (approximate)
        let radius = cell_size * 0.866; // sqrt(3)/2 for truncated octahedron

        if camera.is_in_frustum(world_pos, radius) {
            visible.push(index);
        }
    }

    visible
}

// Usage
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let camera = Camera::new(
        Point3::new(10.0, 10.0, 10.0),
        Point3::new(0.0, 0.0, 0.0),
        std::f32::consts::FRAC_PI_4,
        16.0 / 9.0,
    );

    let container = octaindex3d::Container::load("scene.bcc")?;
    let visible_cells = frustum_cull_bcc(&camera, &container, 10, 1.0);

    println!("Visible cells: {}", visible_cells.len());
    Ok(())
}
```

### H.1.2 ndarray for Volume Data

**Use case:** Convert between dense ndarray volumes and sparse BCC containers.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
ndarray = "0.15"
```rust

**Example: Import volumetric data from ndarray**

```rust
use ndarray::Array3;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};

/// Convert ndarray volume to BCC container
/// Only stores non-zero values (sparse representation)
fn ndarray_to_bcc<T: Copy + Default + PartialEq>(
    volume: &Array3<T>,
    lod: u8,
    threshold: T,
) -> InMemoryContainer<T> {
    let mut container = InMemoryContainer::new();
    let (nx, ny, nz) = volume.dim();

    for i in 0..nx {
        for j in 0..ny {
            for k in 0..nz {
                let value = volume[[i, j, k]];

                // Only store values above threshold
                if value != threshold {
                    // Map array indices to BCC coordinates
                    if let Ok(coord) = BccCoord::new(i as i32, j as i32, k as i32) {
                        let index = Index64::from_bcc_coord(coord, lod);
                        container.insert(index, value);
                    }
                }
            }
        }
    }

    container
}

/// Convert BCC container back to dense ndarray volume
fn bcc_to_ndarray<T: Copy + Default>(
    container: &InMemoryContainer<T>,
    shape: (usize, usize, usize),
    lod: u8,
) -> Array3<T> {
    let mut volume = Array3::default(shape);

    for (index, value) in container.iter() {
        let coord = index.to_bcc_coord();
        let (x, y, z) = coord.as_tuple();

        // Check bounds
        if x >= 0 && y >= 0 && z >= 0 {
            let (i, j, k) = (x as usize, y as usize, z as usize);
            if i < shape.0 && j < shape.1 && k < shape.2 {
                volume[[i, j, k]] = *value;
            }
        }
    }

    volume
}

// Example: CT scan processing
fn main() {
    // Load medical imaging data (e.g., from DICOM)
    let ct_scan = Array3::<f32>::zeros((512, 512, 300));
    // ... populate ct_scan from actual data ...

    // Convert to sparse BCC (only store tissue, not air)
    let threshold = 0.1; // Hounsfield unit threshold
    let bcc_scan = ndarray_to_bcc(&ct_scan, 12, threshold);

    println!("Original: {} voxels", ct_scan.len());
    println!("BCC sparse: {} occupied cells", bcc_scan.len());
    println!("Compression ratio: {:.1}×", ct_scan.len() as f64 / bcc_scan.len() as f64);
}
```

### H.1.3 Rayon for Parallel Processing

**Use case:** Parallelize BCC operations across CPU cores.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
rayon = "1.7"
```rust

**Example: Parallel neighbor aggregation**

```rust
use octaindex3d::{Index64, BccCoord, InMemoryContainer};
use rayon::prelude::*;
use std::collections::HashMap;

/// Compute average of neighbors in parallel
fn parallel_neighbor_average(
    container: &InMemoryContainer<f32>,
) -> HashMap<Index64, f32> {
    container
        .indices()
        .par_iter() // Rayon parallel iterator
        .map(|&index| {
            let coord = index.to_bcc_coord();
            let neighbors = coord.get_14_neighbors();

            // Gather neighbor values
            let values: Vec<f32> = neighbors
                .iter()
                .filter_map(|&n| {
                    let n_idx = Index64::from_bcc_coord(n, index.lod());
                    container.get(n_idx).copied()
                })
                .collect();

            // Compute average
            let avg = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f32>() / values.len() as f32
            };

            (index, avg)
        })
        .collect()
}

// Example: Smoothing filter
fn main() {
    let mut container = InMemoryContainer::new();
    // ... populate with noisy data ...

    // Apply 3 iterations of smoothing
    for iteration in 0..3 {
        let smoothed = parallel_neighbor_average(&container);

        // Update container with smoothed values
        for (index, value) in smoothed {
            container.insert(index, value);
        }

        println!("Iteration {} complete", iteration + 1);
    }
}
```

### H.1.4 Tokio for Async Streaming

**Use case:** Stream BCC data asynchronously for real-time applications.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```rust

**Example: Async sensor data ingestion**

```rust
use octaindex3d::{StreamingContainer, Index64, BccCoord};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Simulated sensor reading
struct SensorReading {
    position: (i32, i32, i32),
    value: f32,
    timestamp: u64,
}

/// Async sensor simulator
async fn sensor_stream(tx: mpsc::Sender<SensorReading>) {
    let mut tick = interval(Duration::from_millis(10));
    let mut count = 0u64;

    loop {
        tick.tick().await;

        let reading = SensorReading {
            position: (
                (count % 100) as i32,
                ((count / 100) % 100) as i32,
                ((count / 10000) % 100) as i32,
            ),
            value: (count as f32).sin(),
            timestamp: count,
        };

        if tx.send(reading).await.is_err() {
            break; // Receiver dropped
        }

        count += 1;
    }
}

/// Async BCC ingestor
async fn bcc_ingestor(mut rx: mpsc::Receiver<SensorReading>) {
    let mut container = StreamingContainer::new("live_data.bcc.stream")
        .expect("Failed to create streaming container");

    while let Some(reading) = rx.recv().await {
        let (x, y, z) = reading.position;

        if let Ok(coord) = BccCoord::new(x, y, z) {
            let index = Index64::from_bcc_coord(coord, 10);
            container.append(index, reading.value)
                .expect("Failed to append");
        }

        // Periodic flush
        if reading.timestamp % 1000 == 0 {
            container.flush().expect("Failed to flush");
            println!("Flushed at timestamp {}", reading.timestamp);
        }
    }

    container.close().expect("Failed to close container");
}

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel(100);

    // Spawn sensor and ingestor tasks
    let sensor_task = tokio::spawn(sensor_stream(tx));
    let ingestor_task = tokio::spawn(bcc_ingestor(rx));

    // Run for 10 seconds
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Cleanup (in real code, use graceful shutdown)
    sensor_task.abort();
    ingestor_task.await.expect("Ingestor task failed");
}
```

---

## H.2 Geospatial Tools Integration

### H.2.1 GeoJSON Export for QGIS

**Use case:** Visualize BCC data in QGIS by exporting to GeoJSON.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
geojson = "0.24"
serde_json = "1.0"
```rust

**Example: Export BCC points to GeoJSON**

```rust
use octaindex3d::{Container, Index64, Frame, FrameRegistry};
use geojson::{Feature, FeatureCollection, Geometry, Value};
use std::fs::File;
use std::io::Write;

/// Convert BCC container to GeoJSON FeatureCollection
fn bcc_to_geojson(
    container: &Container,
    frame: &Frame,
    registry: &FrameRegistry,
    lod: u8,
) -> FeatureCollection {
    let mut features = Vec::new();

    for (index, value) in container.iter_lod(lod) {
        let coord = index.to_bcc_coord();

        // Convert BCC coordinate to physical position
        let physical = frame.bcc_to_physical(coord, lod);

        // Transform to WGS84 (lat/lon)
        let wgs84_pos = registry.transform_to_wgs84(&physical, frame)
            .expect("Failed to transform to WGS84");

        // Create GeoJSON Point
        let geometry = Geometry::new(Value::Point(vec![
            wgs84_pos.longitude,
            wgs84_pos.latitude,
            wgs84_pos.altitude,
        ]));

        // Create Feature with properties
        let mut properties = serde_json::Map::new();
        properties.insert("value".to_string(), json!(value));
        properties.insert("lod".to_string(), json!(lod));
        properties.insert("index".to_string(), json!(index.as_u64()));

        let feature = Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        features.push(feature);
    }

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let container = Container::load("urban_temp.bcc")?;
    let registry = FrameRegistry::new();
    let frame = registry.get_frame("earth")?;

    let geojson = bcc_to_geojson(&container, frame, &registry, 12);

    // Write to file
    let json = serde_json::to_string_pretty(&geojson)?;
    let mut file = File::create("urban_temp.geojson")?;
    file.write_all(json.as_bytes())?;

    println!("Exported {} features to urban_temp.geojson", geojson.features.len());
    println!("Open in QGIS: Layer → Add Layer → Add Vector Layer");

    Ok(())
}
```

### H.2.2 GDAL Integration

**Use case:** Read raster data with GDAL and index in BCC.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
gdal = "0.16"
```rust

**Example: Import elevation data from GeoTIFF**

```rust
use gdal::Dataset;
use gdal::raster::RasterBand;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};

/// Import elevation raster to BCC
fn import_elevation(
    path: &str,
    lod: u8,
) -> Result<InMemoryContainer<f32>, Box<dyn std::error::Error>> {
    let dataset = Dataset::open(path)?;
    let rasterband: RasterBand = dataset.rasterband(1)?;

    let (width, height) = rasterband.size();
    let mut container = InMemoryContainer::new();

    // Read elevation data
    let buffer = rasterband.read_as::<f32>((0, 0), (width, height), (width, height), None)?;

    // Index in BCC
    for y in 0..height {
        for x in 0..width {
            let elevation = buffer.data[(y * width + x) as usize];

            // Skip no-data values
            if elevation > -9999.0 {
                // Map x,y,elevation to BCC coordinates
                let z = (elevation / 10.0) as i32; // Scale elevation

                if let Ok(coord) = BccCoord::new(x as i32, y as i32, z) {
                    let index = Index64::from_bcc_coord(coord, lod);
                    container.insert(index, elevation);
                }
            }
        }
    }

    Ok(container)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let elevation = import_elevation("terrain.tif", 12)?;
    println!("Imported {} elevation points", elevation.len());

    // Query elevation at specific location
    let query_coord = BccCoord::new(1000, 2000, 50)?;
    let query_index = Index64::from_bcc_coord(query_coord, 12);

    if let Some(&elev) = elevation.get(query_index) {
        println!("Elevation at ({}, {}, {}): {:.2}m",
                 1000, 2000, 50, elev);
    }

    Ok(())
}
```

### H.2.3 PostGIS Integration

**Use case:** Store BCC indices in PostgreSQL/PostGIS for spatial queries.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
postgres = "0.19"
```sql

**Example: Store and query BCC data in PostGIS**

```sql
-- SQL schema
CREATE TABLE bcc_points (
    id SERIAL PRIMARY KEY,
    bcc_index BIGINT NOT NULL,
    lod SMALLINT NOT NULL,
    value REAL NOT NULL,
    geometry GEOMETRY(PointZ, 4326), -- WGS84 3D point
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_bcc_index ON bcc_points(bcc_index);
CREATE INDEX idx_geometry ON bcc_points USING GIST(geometry);
```

```rust
use octaindex3d::{Index64, BccCoord, Frame, FrameRegistry};
use postgres::{Client, NoTls};

/// Insert BCC points into PostGIS
fn insert_bcc_points(
    client: &mut Client,
    container: &octaindex3d::Container,
    frame: &Frame,
    registry: &FrameRegistry,
    lod: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    for (index, value) in container.iter_lod(lod) {
        let coord = index.to_bcc_coord();

        // Convert to WGS84
        let physical = frame.bcc_to_physical(coord, lod);
        let wgs84 = registry.transform_to_wgs84(&physical, frame)?;

        // Insert into PostGIS
        client.execute(
            "INSERT INTO bcc_points (bcc_index, lod, value, geometry)
             VALUES ($1, $2, $3, ST_SetSRID(ST_MakePoint($4, $5, $6), 4326))",
            &[
                &(index.as_u64() as i64),
                &(lod as i16),
                &value,
                &wgs84.longitude,
                &wgs84.latitude,
                &wgs84.altitude,
            ],
        )?;
    }

    Ok(())
}

/// Query BCC points within bounding box
fn query_bbox(
    client: &mut Client,
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
) -> Result<Vec<(i64, f32)>, Box<dyn std::error::Error>> {
    let rows = client.query(
        "SELECT bcc_index, value
         FROM bcc_points
         WHERE ST_Intersects(
             geometry,
             ST_MakeEnvelope($1, $2, $3, $4, 4326)
         )",
        &[&min_lon, &min_lat, &max_lon, &max_lat],
    )?;

    let results = rows
        .iter()
        .map(|row| {
            let index: i64 = row.get(0);
            let value: f32 = row.get(1);
            (index, value)
        })
        .collect();

    Ok(results)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect(
        "host=localhost user=postgres dbname=spatial",
        NoTls,
    )?;

    // Query urban area
    let results = query_bbox(&mut client, -122.5, 37.7, -122.3, 37.8)?;
    println!("Found {} points in San Francisco area", results.len());

    Ok(())
}
```toml

---

## H.3 Game Engine Integration

### H.3.1 Bevy Integration

**Use case:** Use BCC for voxel worlds in Bevy game engine.

**Setup:**
```toml
[dependencies]
octaindex3d = "0.1"
bevy = "0.12"
```

**Example: BCC voxel world in Bevy**

```rust
use bevy::prelude::*;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};

#[derive(Resource)]
struct VoxelWorld {
    container: InMemoryContainer<VoxelType>,
    lod: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum VoxelType {
    Air,
    Dirt,
    Stone,
    Grass,
}

impl VoxelWorld {
    fn new(lod: u8) -> Self {
        VoxelWorld {
            container: InMemoryContainer::new(),
            lod,
        }
    }

    fn set_voxel(&mut self, x: i32, y: i32, z: i32, voxel: VoxelType) {
        if let Ok(coord) = BccCoord::new(x, y, z) {
            let index = Index64::from_bcc_coord(coord, self.lod);
            self.container.insert(index, voxel);
        }
    }

    fn get_voxel(&self, x: i32, y: i32, z: i32) -> VoxelType {
        BccCoord::new(x, y, z)
            .ok()
            .and_then(|coord| {
                let index = Index64::from_bcc_coord(coord, self.lod);
                self.container.get(index).copied()
            })
            .unwrap_or(VoxelType::Air)
    }
}

/// Generate terrain
fn generate_terrain(mut world: ResMut<VoxelWorld>) {
    for x in -50..50 {
        for z in -50..50 {
            // Simple height map
            let height = (10.0 + 5.0 * (x as f32 * 0.1).sin() * (z as f32 * 0.1).cos()) as i32;

            for y in 0..height {
                let voxel = if y == height - 1 {
                    VoxelType::Grass
                } else if y > height - 4 {
                    VoxelType::Dirt
                } else {
                    VoxelType::Stone
                };

                world.set_voxel(x, y, z, voxel);
            }
        }
    }
}

/// Render voxels as cubes (simplified)
fn render_voxels(
    world: Res<VoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    for (index, voxel_type) in world.container.iter() {
        let coord = index.to_bcc_coord();
        let (x, y, z) = coord.as_tuple();

        let color = match voxel_type {
            VoxelType::Grass => Color::rgb(0.2, 0.8, 0.2),
            VoxelType::Dirt => Color::rgb(0.6, 0.4, 0.2),
            VoxelType::Stone => Color::rgb(0.5, 0.5, 0.5),
            VoxelType::Air => continue,
        };

        commands.spawn(PbrBundle {
            mesh: cube_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color: color,
                ..default()
            }),
            transform: Transform::from_xyz(x as f32, y as f32, z as f32),
            ..default()
        });
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(VoxelWorld::new(10))
        .add_systems(Startup, (generate_terrain, render_voxels).chain())
        .run();
}
```toml

### H.3.2 Godot Integration (via GDNative)

**Use case:** Expose BCC spatial indexing to Godot via GDNative bindings.

**Setup:**
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
octaindex3d = "0.1"
gdnative = "0.11"
```

**Example: Godot BCC plugin**

```rust
use gdnative::prelude::*;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};

#[derive(NativeClass)]
#[inherit(Reference)]
struct BccSpatialIndex {
    container: InMemoryContainer<Variant>,
    lod: u8,
}

#[methods]
impl BccSpatialIndex {
    fn new(_base: &Reference) -> Self {
        BccSpatialIndex {
            container: InMemoryContainer::new(),
            lod: 10,
        }
    }

    #[method]
    fn set_value(&mut self, x: i32, y: i32, z: i32, value: Variant) {
        if let Ok(coord) = BccCoord::new(x, y, z) {
            let index = Index64::from_bcc_coord(coord, self.lod);
            self.container.insert(index, value);
        }
    }

    #[method]
    fn get_value(&self, x: i32, y: i32, z: i32) -> Variant {
        BccCoord::new(x, y, z)
            .ok()
            .and_then(|coord| {
                let index = Index64::from_bcc_coord(coord, self.lod);
                self.container.get(index).cloned()
            })
            .unwrap_or_else(Variant::new)
    }

    #[method]
    fn find_neighbors(&self, x: i32, y: i32, z: i32) -> VariantArray {
        let array = VariantArray::new();

        if let Ok(coord) = BccCoord::new(x, y, z) {
            for neighbor in coord.get_14_neighbors() {
                let index = Index64::from_bcc_coord(neighbor, self.lod);
                if let Some(value) = self.container.get(index) {
                    let (nx, ny, nz) = neighbor.as_tuple();
                    let dict = Dictionary::new();
                    dict.insert("x", nx);
                    dict.insert("y", ny);
                    dict.insert("z", nz);
                    dict.insert("value", value.clone());
                    array.push(dict);
                }
            }
        }

        array.into_shared()
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<BccSpatialIndex>();
}

godot_init!(init);
```python

**Usage in GDScript:**
```gdscript
# Godot script
extends Node

var bcc_index

func _ready():
    bcc_index = BccSpatialIndex.new()

    # Set some voxels
    bcc_index.set_value(0, 0, 0, "origin")
    bcc_index.set_value(2, 0, 0, "east")
    bcc_index.set_value(0, 2, 0, "north")

    # Query
    var value = bcc_index.get_value(0, 0, 0)
    print("Value at origin: ", value)

    # Find neighbors
    var neighbors = bcc_index.find_neighbors(0, 0, 0)
    print("Neighbors: ", neighbors.size())
```

---

## H.4 Scientific Computing Integration

### H.4.1 Python Bindings (PyO3)

**Use case:** Use BCC from Python for scientific computing and ML.

**Setup:**
```toml
[lib]
name = "octaindex3d_py"
crate-type = ["cdylib"]

[dependencies]
octaindex3d = "0.1"
pyo3 = { version = "0.20", features = ["extension-module"] }
numpy = "0.20"
```rust

**Example: Python bindings**

```rust
use pyo3::prelude::*;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};
use numpy::{PyArray3, PyReadonlyArray3};

#[pyclass]
struct BccContainer {
    inner: InMemoryContainer<f64>,
    lod: u8,
}

#[pymethods]
impl BccContainer {
    #[new]
    fn new(lod: u8) -> Self {
        BccContainer {
            inner: InMemoryContainer::new(),
            lod,
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, value: f64) -> PyResult<()> {
        let coord = BccCoord::new(x, y, z)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        let index = Index64::from_bcc_coord(coord, self.lod);
        self.inner.insert(index, value);
        Ok(())
    }

    fn get(&self, x: i32, y: i32, z: i32) -> PyResult<Option<f64>> {
        let coord = BccCoord::new(x, y, z)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        let index = Index64::from_bcc_coord(coord, self.lod);
        Ok(self.inner.get(index).copied())
    }

    fn from_numpy<'py>(
        &mut self,
        py: Python<'py>,
        array: PyReadonlyArray3<f64>,
    ) -> PyResult<()> {
        let arr = array.as_array();
        let (nx, ny, nz) = arr.dim();

        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let value = arr[[i, j, k]];
                    if value != 0.0 {
                        if let Ok(coord) = BccCoord::new(i as i32, j as i32, k as i32) {
                            let index = Index64::from_bcc_coord(coord, self.lod);
                            self.inner.insert(index, value);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

#[pymodule]
fn octaindex3d_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<BccContainer>()?;
    Ok(())
}
```

**Usage in Python:**
```python
import numpy as np
import octaindex3d_py as bcc

# Create container
container = bcc.BccContainer(lod=10)

# Set values
container.set(0, 0, 0, 1.5)
container.set(2, 0, 0, 2.5)

# Get value
value = container.get(0, 0, 0)
print(f"Value: {value}")  # Output: Value: 1.5

# Import from NumPy
volume = np.random.rand(100, 100, 100)
volume[volume < 0.5] = 0  # Sparsify

container.from_numpy(volume)
print(f"Imported {len(container)} non-zero voxels")
```rust

---

## H.5 Database Integration

### H.5.1 SQLite Integration

**Use case:** Store BCC indices in SQLite for portable spatial database.

**Example:**

```rust
use octaindex3d::{Index64, BccCoord};
use rusqlite::{Connection, Result};

fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS bcc_data (
            bcc_index INTEGER PRIMARY KEY,
            lod INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            z INTEGER NOT NULL,
            value REAL NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_coords ON bcc_data(x, y, z)",
        [],
    )?;

    Ok(())
}

fn insert_bcc(
    conn: &Connection,
    coord: BccCoord,
    lod: u8,
    value: f32,
) -> Result<()> {
    let index = Index64::from_bcc_coord(coord, lod);
    let (x, y, z) = coord.as_tuple();

    conn.execute(
        "INSERT OR REPLACE INTO bcc_data (bcc_index, lod, x, y, z, value)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        &[
            &(index.as_u64() as i64),
            &(lod as i64),
            &(x as i64),
            &(y as i64),
            &(z as i64),
            &(value as f64),
        ],
    )?;

    Ok(())
}

fn query_range(
    conn: &Connection,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
) -> Result<Vec<(BccCoord, f32)>> {
    let mut stmt = conn.prepare(
        "SELECT x, y, z, value FROM bcc_data
         WHERE x BETWEEN ?1 AND ?2
           AND y BETWEEN ?3 AND ?4
           AND z BETWEEN ?5 AND ?6",
    )?;

    let results = stmt.query_map(
        &[&min_x, &max_x, &min_y, &max_y, &min_z, &max_z],
        |row| {
            let x: i32 = row.get(0)?;
            let y: i32 = row.get(1)?;
            let z: i32 = row.get(2)?;
            let value: f64 = row.get(3)?;

            let coord = BccCoord::new(x, y, z).unwrap();
            Ok((coord, value as f32))
        },
    )?;

    results.collect()
}

fn main() -> Result<()> {
    let conn = Connection::open("bcc_data.db")?;
    create_schema(&conn)?;

    // Insert data
    for x in 0..100 {
        for y in 0..100 {
            for z in 0..10 {
                if let Ok(coord) = BccCoord::new(x, y, z) {
                    insert_bcc(&conn, coord, 10, (x + y + z) as f32)?;
                }
            }
        }
    }

    // Query
    let results = query_range(&conn, 10, 20, 10, 20, 0, 10)?;
    println!("Found {} points in range", results.len());

    Ok(())
}
```

---

## H.6 Web and Visualization

### H.6.1 WebAssembly (WASM) for Browser Visualization

**Use case:** Run BCC in the browser for interactive 3D visualization.

**Setup:**
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
octaindex3d = "0.1"
wasm-bindgen = "0.2"
web-sys = "0.3"
```rust

**Example: WASM bindings**

```rust
use wasm_bindgen::prelude::*;
use octaindex3d::{Index64, BccCoord, InMemoryContainer};

#[wasm_bindgen]
pub struct BccVisualization {
    container: InMemoryContainer<f32>,
    lod: u8,
}

#[wasm_bindgen]
impl BccVisualization {
    #[wasm_bindgen(constructor)]
    pub fn new(lod: u8) -> Self {
        BccVisualization {
            container: InMemoryContainer::new(),
            lod,
        }
    }

    #[wasm_bindgen]
    pub fn set_value(&mut self, x: i32, y: i32, z: i32, value: f32) -> Result<(), JsValue> {
        let coord = BccCoord::new(x, y, z)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        let index = Index64::from_bcc_coord(coord, self.lod);
        self.container.insert(index, value);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_all_points(&self) -> Vec<f32> {
        let mut points = Vec::new();

        for (index, value) in self.container.iter() {
            let coord = index.to_bcc_coord();
            let (x, y, z) = coord.as_tuple();

            points.push(x as f32);
            points.push(y as f32);
            points.push(z as f32);
            points.push(*value);
        }

        points
    }
}
```

**JavaScript usage:**
```javascript
import init, { BccVisualization } from './pkg/octaindex3d_wasm.js';

async function main() {
    await init();

    const bcc = new BccVisualization(10);

    // Add points
    for (let x = -10; x <= 10; x += 2) {
        for (let y = -10; y <= 10; y += 2) {
            for (let z = -10; z <= 10; z += 2) {
                const value = Math.sin(x * 0.1) + Math.cos(y * 0.1);
                bcc.set_value(x, y, z, value);
            }
        }
    }

    // Get all points for Three.js rendering
    const points = bcc.get_all_points();
    console.log(`Rendering ${points.length / 4} points`);

    // ... render with Three.js or WebGL ...
}

main();
```python

---

## H.7 Further Reading

**Rust Ecosystem:**
- nalgebra documentation: https://nalgebra.org
- ndarray guide: https://docs.rs/ndarray
- Rayon parallel programming: https://docs.rs/rayon

**Geospatial:**
- GDAL/OGR tutorial: https://gdal.org/tutorials/
- PostGIS documentation: https://postgis.net/docs/
- GeoJSON specification: https://geojson.org

**Game Development:**
- Bevy book: https://bevyengine.org/learn/book/
- GDNative guide: https://godot-rust.github.io/book/

**Python Scientific Computing:**
- PyO3 guide: https://pyo3.rs
- NumPy integration: https://docs.rs/numpy

**Web:**
- wasm-bindgen book: https://rustwasm.github.io/wasm-bindgen/

---

**End of Appendix H**
