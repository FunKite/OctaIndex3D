# Chapter 11: Geospatial Analysis

## Learning Objectives

By the end of this chapter, you will be able to:

1. Describe how BCC lattices support large-scale geospatial modeling.
2. Understand how hierarchical refinement helps manage massive atmospheric and environmental datasets.
3. Integrate OctaIndex3D containers with GIS tools such as QGIS and GeoJSON.
4. Design storage and query patterns for urban-scale 3D models.

---

## 11.1 Atmospheric and Environmental Modeling

Climate and environmental models often:

- Cover the entire globe.
- Use multiple vertical levels (altitude or pressure).
- Require adaptive resolution in regions of interest.

BCC-based indexing offers a natural fit for these problems:

- Efficient sampling of volumetric fields.
- Isotropic neighbor relationships that reduce directional artifacts.
- 29% fewer samples for equivalent fidelity compared to cubic grids.

OctaIndex3D containers can store:

- Scalar fields (temperature, humidity, pollutant concentration).
- Vector fields (wind velocity).

Frame definitions encode:

- The mapping between model coordinates and ECEF or other CRS.
- Any vertical coordinate conventions (e.g., height vs. pressure).

This combination lets you treat a global atmosphere or ocean as a single, coherent data structure instead of a patchwork of tiles and ad hoc grids. A typical workflow might:

1. Define a frame aligned with the model’s native coordinates (for example, a latitude–longitude–pressure grid).
2. Register a transformation from that frame to an Earth-centered Cartesian frame.
3. Sample model outputs into BCC cells at multiple LODs, storing scalar and vector fields in containers.
4. Run diagnostic or visualization queries directly on the BCC containers, rather than repeatedly reprojecting data to other grids.

Because the indexing is independent of any particular solver, you can compare and combine data from multiple models, regridding them into a common BCC representation for downstream analysis.

---

### 11.1.1 Choosing Frames for Earth Systems

Most atmospheric and oceanic models use one of a few standard coordinate choices:

- Latitude/longitude with some vertical coordinate (height, pressure, sigma).
- Earth-centered, Earth-fixed (ECEF) Cartesian coordinates.
- Local tangent frames (e.g., ENU) anchored to regions of interest.

In OctaIndex3D, these become **frames**:

- A **model frame** aligned with the solver’s internal grid.
- An **Earth frame** (ECEF with a chosen ellipsoid, often WGS84).
- Optional **regional frames** for high-resolution areas (e.g., a metropolitan region).

The frame registry then stores:

- Transformations between model, Earth, and regional frames.
- Metadata such as CRS identifiers, vertical datum, and time dependence (e.g., tectonic corrections).

This explicit structure pays off when:

- Comparing multiple models that use different grids or projections.
- Ingesting observations from satellites and ground stations with their own CRSs.
- Exporting subsets of the field to GIS tools that expect WGS84 lat/long.

### 11.1.2 Atmospheric Modeling Case Study

Consider a regional air-quality model that:

- Covers a continent with moderate resolution.
- Uses nested, higher-resolution grids over major cities.
- Outputs hourly fields for several pollutants and meteorological variables.

An OctaIndex3D-based workflow might:

1. Define a **model frame** matching the solver grid.
2. Register transformations between the model frame and WGS84-ECEF.
3. For each output time step:
   - Sample the model field into BCC cells at a **global LOD** that roughly matches the coarse grid spacing.
   - Refine around selected urban areas into **finer LODs**, using city-specific regional frames.
4. Store the resulting scalar fields (e.g., PM2.5, NO₂, O₃) in containers keyed by `Galactic128` or `Index64`.
5. Run diagnostics directly on the containers:
   - Max/mean concentration over regions.
   - Exceedance counts above regulatory thresholds.
   - Trajectories of plumes along prevailing winds.

Because all variables share the same BCC lattice and frames, multi-field queries—such as “regions where high PM2.5 coincides with low wind speed and temperature inversion”—become simple container operations instead of multi-grid joins.

## 11.2 Hierarchical Adaptive Refinement

Hierarchical refinement allows:

- Coarse resolution in regions of low interest.
- Fine resolution where dynamics are complex or observations are dense.

Using OctaIndex3D:

- Coarse cells are represented at low LOD.
- Fine cells subdivide parents at higher LOD.
- Containers track which cells at each LOD are populated.

Queries can:

- Traverse from coarse to fine cells as needed.
- Aggregate statistics (e.g., averages) up the hierarchy.
- Support multi-resolution visualization and analysis.

For example, a hurricane-tracking application might:

1. Maintain a coarse global grid that covers the entire planet.
2. Identify storm regions using coarse cells whose wind speeds exceed a threshold.
3. Spawn finer LODs only within those regions, capturing the internal structure of the storm.
4. Use ancestor–descendant relationships between BCC cells to roll up fine-scale statistics (like maximum wind speed or rainfall) back into the coarse grid for global dashboards.

Because parent and child cells share a compact index relationship, these roll-ups and refinements require no expensive spatial joins or geometric predicates. They are simple operations on identifiers.

---

### 11.2.1 Refinement Policies

In practice, you rarely refine everywhere. Instead, you define **policies** such as:

- Refine where gradients exceed a threshold (e.g., steep temperature or pressure changes).
- Refine where observations are dense (e.g., near major cities, shipping lanes).
- Refine along fronts or storm tracks identified from coarse diagnostics.

These policies operate on BCC containers:

1. Run a coarse diagnostic pass over the global field.
2. Mark candidate parent cells for refinement based on policy thresholds.
3. Allocate children at one or more finer LODs only for marked parents.
4. Resample or interpolate model outputs into the new fine cells.

De-refinement (coarsening) follows the reverse pattern:

- When a region is quiescent, fine cells can be merged back into their parent cell.
- Aggregation operators (mean, max, percentiles) roll up values.
- Containers track which cells at each LOD remain active.

The identifiers encode these parent–child relationships, so refinement is an **index manipulation problem**, not a geometry problem.

### 11.2.2 Multi-Resolution Analysis Patterns

Once hierarchy is in place, several common analysis patterns emerge:

- **Drill-down:** start with global aggregates, then zoom into high-LOD regions.
- **Roll-up:** compute global or regional statistics by aggregating over children.
- **Multi-scale correlation:** compare fields at different LODs (e.g., coarse wind vs. fine pollutant fields).

In OctaIndex3D, these patterns map to:

- Iterators over parents and their children.
- Aggregation helpers that accept user-defined reduction functions.
- Queries that restrict attention to a chosen LOD range.

This makes it straightforward to implement workflows like:

- “Show me all regions where city-scale pollution problems are not visible in the coarse global model.”
- “Rank storm systems by the maximum fine-scale wind speed found in their refined cores.”

## 11.3 GIS Integration and Visualization

To integrate with GIS tools:

- OctaIndex3D frames are tagged with CRS identifiers (e.g., EPSG codes).
- Export functions generate **GeoJSON** or similar formats representing:
  - Cell centers or vertices.
  - Aggregated values or metadata.

Visualization tools like QGIS can:

- Render slices or iso-surfaces of BCC-indexed data.
- Overlay results on maps and satellite imagery.

Because BCC cells are truncated octahedra, visual representation often uses:

- Approximate polyhedral meshes.
- Or sampling of the field onto more conventional grids for display only.

In practice, most teams adopt a hybrid approach:

- Use OctaIndex3D and BCC containers as the “truth” for computation and storage.
- On demand, resample relevant portions of the field onto regular grids or meshes that visualization tools understand well.

This avoids “fighting” existing GIS ecosystems while still capturing the benefits of BCC-based indexing. When better BCC-native visualization tools become available, the underlying containers are already prepared.

---

### 11.3.1 Exporting to WGS84 and GeoJSON

Most GIS workflows assume:

- Coordinates in WGS84 (EPSG:4326) as latitude/longitude (and optional height).
- Vector formats like GeoJSON, Shapefile, or GeoPackage.

An OctaIndex3D export pipeline typically:

1. Selects a set of BCC cells (for example, all cells above a threshold).
2. Converts their identifiers to positions in an **Earth frame**.
3. Transforms those positions into WGS84 lat/long via the frame registry.
4. Emits GeoJSON features with:
   - Point geometries (cell centers) or polygons (cell footprints).
   - Properties for scalar or vector fields (values, units, timestamps).

Pseudo-code for an export helper might look like:

```rust
fn export_cells_to_geojson(
    cells: impl Iterator<Item = (Index64, FieldValue)>,
    frames: &FrameRegistry,
) -> GeoJson {
    let mut features = Vec::new();
    for (id, value) in cells {
        let pos_ecef = frames.index_to_ecef(id);
        let (lat, lon, h) = frames.ecef_to_wgs84(pos_ecef);
        features.push(make_feature(lat, lon, h, value));
    }
    GeoJson::FeatureCollection(features)
}
```rust

The exact APIs will differ, but the pattern remains:

- Indices → Earth frame → WGS84 → GIS format.

### 11.3.2 Working with QGIS and Other Tools

Once data is in GeoJSON (or other supported formats), tools like QGIS can:

- Render static maps (contours, heatmaps, point clouds).
- Animate time series using time-aware layers.
- Combine BCC-derived layers with traditional datasets (roads, land use, demographics).

Practical tips include:

- Exporting at multiple levels of detail, so users can switch between coarse overview and fine detail.
- Including metadata in layer names or attributes (LOD, timestamp, model version).
- Providing pre-built QGIS style files (QML) that apply consistent color scales and symbology.

This keeps the **OctaIndex3D side** focused on robust indexing and aggregation, while leaving cartographic decisions to GIS specialists.

## 11.4 Urban-Scale 3D Models

Urban models include:

- Buildings and infrastructure.
- Vegetation and terrain.
- Dynamic entities (traffic, pedestrians).

OctaIndex3D can represent:

- Static geometry indexed by BCC cells.
- Dynamic attributes (traffic density, air quality) attached to cells.

Containers can be:

- Sharded by region (e.g., city blocks).
- Stored in sequential formats for efficient server-side queries.

Applications include:

- Urban planning and scenario analysis.
- Disaster response simulations.
- Real-time digital twins for monitoring and control.

As an example, imagine a city-scale digital twin:

1. Static infrastructure (buildings, bridges, tunnels) is voxelized into BCC cells at several LODs.
2. Sensors throughout the city publish observations—traffic counts, air quality readings, energy usage—which are aggregated into those cells.
3. Planning tools query the BCC containers to answer questions like:
   - “Where will air quality exceed safe thresholds under this weather forecast?”
   - “Which evacuation routes remain viable if a particular bridge is closed?”
4. Results are exported as GeoJSON or 3D tiles and consumed by web-based dashboards and GIS tools.

Here again, OctaIndex3D is not replacing the entire GIS stack; it is providing a powerful, isotropic core for spatial indexing and multi-resolution aggregation that other tools can build on.

---

### 11.4.1 Storage and Sharding Strategies

City-scale datasets quickly grow large. OctaIndex3D containers can be:

- **Spatially sharded** (e.g., by borough, tile, or administrative boundary).
- **Vertically partitioned** by theme (transport, energy, air quality, etc.).
- Stored in streaming container formats for efficient server-side range queries.

Typical patterns include:

- One container per `(region, LOD)` pair.
- Metadata indices mapping higher-level concepts (e.g., “downtown core”) to container sets.
- Background jobs that maintain derived layers (e.g., daily averages, peak-hour traffic).

Sharding is invisible to most application code:

- Requests express queries in frame coordinates.
- A routing layer maps queries to containers based on spatial coverage.
- Results are merged before being returned.

### 11.4.2 Large-Scale Data Processing

Urban digital twins often rely on distributed processing frameworks. OctaIndex3D fits into these systems by:

- Treating containers as chunked, immutable datasets in object stores.
- Using identifier ranges as natural partitions for parallel processing.
- Allowing local workers to operate on subsets of the BCC lattice.

Examples include:

- Batch jobs computing exposure metrics (e.g., population-weighted pollution) over many years of data.
- Near-real-time pipelines aggregating sensor streams into rolling windows.
- Simulation workflows that write results directly into BCC containers for later querying.

Because identifiers are compact and sortable, they work well as keys in columnar formats and key–value stores, which are common in big-data ecosystems.

## 11.5 Practical Case Studies

### 11.5.1 Global Climate Model Post-Processing

Consider a workflow that post-processes output from a global climate model:

**Problem:** A climate model outputs temperature, precipitation, and wind fields on a lat/lon grid with 0.25° resolution and 50 vertical levels. Researchers need to:
- Extract regional subsets for specific continents
- Compute long-term averages and anomalies
- Export high-resolution data for impact studies

**OctaIndex3D Solution:**

```rust
struct ClimateField {
    variable_name: String,
    time: DateTime<Utc>,
    data: HashMap<Index64, f32>,
}

fn ingest_climate_model_output(
    netcdf_path: &Path,
    frame_registry: &FrameRegistry,
) -> Vec<ClimateField> {
    let dataset = NetCdf::open(netcdf_path)?;

    // Read lat/lon/level coordinates
    let lats = dataset.variable("lat").unwrap().values::<f32>()?;
    let lons = dataset.variable("lon").unwrap().values::<f32>()?;
    let levels = dataset.variable("level").unwrap().values::<f32>()?;

    // Read temperature field
    let temp_data = dataset.variable("temperature")
        .unwrap()
        .values::<f32>()?;  // Shape: [time, level, lat, lon]

    let mut fields = Vec::new();

    // Convert to BCC indices
    for (t_idx, time) in dataset.times().enumerate() {
        let mut bcc_data = HashMap::new();

        for (k, &level_hpa) in levels.iter().enumerate() {
            for (j, &lat) in lats.iter().enumerate() {
                for (i, &lon) in lons.iter().enumerate() {
                    let idx = t_idx * levels.len() * lats.len() * lons.len()
                        + k * lats.len() * lons.len()
                        + j * lons.len()
                        + i;

                    let temp = temp_data[idx];

                    // Convert lat/lon/pressure to ECEF
                    let altitude_m = pressure_to_altitude(level_hpa);
                    let ecef = wgs84_to_ecef(lat, lon, altitude_m);

                    // Map to BCC index at appropriate LOD
                    let lod = select_lod_for_resolution(0.25);
                    let index = frame_registry.ecef_to_index(ecef, lod);

                    bcc_data.insert(index, temp);
                }
            }
        }

        fields.push(ClimateField {
            variable_name: "temperature".to_string(),
            time,
            data: bcc_data,
        });
    }

    fields
}
```

**Regional Extraction:**

```rust
fn extract_europe_region(
    fields: &[ClimateField],
    frame_registry: &FrameRegistry,
) -> Vec<ClimateField> {
    // Define Europe bounding box
    let lat_min = 35.0;
    let lat_max = 71.0;
    let lon_min = -10.0;
    let lon_max = 40.0;
    let alt_min = 0.0;
    let alt_max = 20000.0;  // Up to 20km altitude

    // Convert corners to BCC index ranges
    let sw_corner = wgs84_to_ecef(lat_min, lon_min, alt_min);
    let ne_corner = wgs84_to_ecef(lat_max, lon_max, alt_max);

    let lod = 5;  // Appropriate for 0.25° resolution
    let sw_index = frame_registry.ecef_to_index(sw_corner, lod);
    let ne_index = frame_registry.ecef_to_index(ne_corner, lod);

    // Extract cells within bounding box
    fields.iter().map(|field| {
        let filtered_data: HashMap<_, _> = field.data.iter()
            .filter(|(&index, _)| {
                index.is_within_box(sw_index, ne_index)
            })
            .map(|(&k, &v)| (k, v))
            .collect();

        ClimateField {
            variable_name: field.variable_name.clone(),
            time: field.time,
            data: filtered_data,
        }
    }).collect()
}
```rust

**Computing Anomalies:**

```rust
fn compute_climatology(
    fields: &[ClimateField],
) -> HashMap<Index64, f32> {
    // Compute long-term mean at each BCC cell
    let mut sums: HashMap<Index64, f32> = HashMap::new();
    let mut counts: HashMap<Index64, u32> = HashMap::new();

    for field in fields {
        for (&index, &value) in &field.data {
            *sums.entry(index).or_insert(0.0) += value;
            *counts.entry(index).or_insert(0) += 1;
        }
    }

    sums.into_iter()
        .map(|(index, sum)| {
            let count = counts[&index];
            (index, sum / count as f32)
        })
        .collect()
}

fn compute_anomaly(
    field: &ClimateField,
    climatology: &HashMap<Index64, f32>,
) -> ClimateField {
    let anomaly_data = field.data.iter()
        .filter_map(|(&index, &value)| {
            climatology.get(&index).map(|&clim| {
                (index, value - clim)
            })
        })
        .collect();

    ClimateField {
        variable_name: format!("{}_anomaly", field.variable_name),
        time: field.time,
        data: anomaly_data,
    }
}
```

### 11.5.2 Urban Digital Twin for Air Quality

**Problem:** A city wants to build a real-time digital twin that:
- Ingests sensor data from hundreds of air quality monitors
- Interpolates concentrations across the urban area
- Alerts residents when thresholds are exceeded
- Supports what-if scenarios for policy planning

**Architecture:**

```rust
struct AirQualityTwin {
    // Static city geometry
    buildings: HashMap<Index64, BuildingInfo>,

    // Dynamic sensor readings
    sensors: Vec<SensorStation>,
    current_fields: HashMap<String, HashMap<Index64, f32>>,

    // Metadata
    city_frame: FrameId,
    lod_coarse: u8,  // 100m resolution for city-wide view
    lod_fine: u8,    // 10m resolution near sensors
}

struct SensorStation {
    id: String,
    location: Vec3,  // in city frame
    pollutants: Vec<String>,
}

impl AirQualityTwin {
    fn ingest_sensor_reading(
        &mut self,
        sensor_id: &str,
        timestamp: DateTime<Utc>,
        pollutant: &str,
        value_ugm3: f32,
    ) {
        let sensor = self.sensors.iter()
            .find(|s| s.id == sensor_id)
            .expect("Unknown sensor");

        // Map sensor location to BCC indices at multiple LODs
        let index_coarse = self.city_frame.to_index(
            sensor.location,
            self.lod_coarse,
        );
        let index_fine = self.city_frame.to_index(
            sensor.location,
            self.lod_fine,
        );

        // Update fields at both LODs
        self.current_fields
            .entry(pollutant.to_string())
            .or_default()
            .insert(index_fine, value_ugm3);

        // Also update coarse level (aggregate if multiple sensors)
        self.update_coarse_field(pollutant, index_coarse, value_ugm3);
    }

    fn interpolate_field(
        &self,
        pollutant: &str,
        lod: u8,
    ) -> HashMap<Index64, f32> {
        let sensor_values = self.current_fields
            .get(pollutant)
            .expect("Unknown pollutant");

        // Use inverse-distance weighting to interpolate
        let mut interpolated = HashMap::new();

        // Get all cells in city bounds at this LOD
        for cell_index in self.get_city_cells(lod) {
            let cell_pos = cell_index.to_position(&self.city_frame);

            // Find nearby sensor readings
            let nearby: Vec<_> = sensor_values.iter()
                .filter_map(|(&sensor_idx, &value)| {
                    let sensor_pos = sensor_idx.to_position(&self.city_frame);
                    let dist = cell_pos.distance_to(sensor_pos);

                    if dist < 500.0 {  // Within 500m
                        Some((dist, value))
                    } else {
                        None
                    }
                })
                .collect();

            if nearby.is_empty() {
                continue;
            }

            // Inverse distance weighting
            let (weighted_sum, weight_sum) = nearby.iter()
                .fold((0.0, 0.0), |(ws, w), &(dist, value)| {
                    let weight = 1.0 / (dist + 1.0);  // +1 to avoid divide by zero
                    (ws + weight * value, w + weight)
                });

            interpolated.insert(cell_index, weighted_sum / weight_sum);
        }

        interpolated
    }

    fn check_thresholds(&self, pollutant: &str, threshold_ugm3: f32) -> Vec<Index64> {
        self.current_fields
            .get(pollutant)
            .map(|field| {
                field.iter()
                    .filter(|(_, &value)| value > threshold_ugm3)
                    .map(|(&index, _)| index)
                    .collect()
            })
            .unwrap_or_default()
    }
}
```rust

**Visualization Export:**

```rust
fn export_air_quality_to_geojson(
    twin: &AirQualityTwin,
    pollutant: &str,
    lod: u8,
) -> GeoJson {
    let field = twin.interpolate_field(pollutant, lod);

    let features: Vec<_> = field.iter()
        .map(|(&index, &value)| {
            let ecef = twin.city_frame.index_to_ecef(index);
            let (lat, lon, h) = ecef_to_wgs84(ecef);

            Feature {
                geometry: Geometry::Point(Point::new(lon, lat)),
                properties: json!({
                    "pollutant": pollutant,
                    "value_ugm3": value,
                    "altitude_m": h,
                    "index": index.to_string(),
                }),
            }
        })
        .collect();

    GeoJson::FeatureCollection(FeatureCollection { features })
}
```

## 11.6 Performance and Scalability

### 11.6.1 Memory-Mapped Containers for Large Datasets

For continent or planet-scale data, keep containers on disk and memory-map them:

```rust
use memmap2::MmapOptions;

struct MmappedContainer {
    mmap: Mmap,
    header: ContainerHeader,
}

impl MmappedContainer {
    fn open(path: &Path) -> Result<Self, Error> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Read header
        let header = ContainerHeader::deserialize(&mmap[0..256])?;

        Ok(Self { mmap, header })
    }

    fn query_range(&self, start_index: Index64, end_index: Index64) -> Vec<(Index64, Value)> {
        // Binary search in memory-mapped data
        let data_start = self.header.data_offset;
        let entry_size = self.header.entry_size;

        // ... implementation details
        vec![]
    }
}
```rust

### 11.6.2 Parallel Processing with Rayon

Process large geospatial datasets in parallel:

```rust
use rayon::prelude::*;

fn parallel_refinement_analysis(
    global_field: &HashMap<Index64, f32>,
    threshold: f32,
) -> Vec<Index64> {
    // Find all cells exceeding threshold in parallel
    global_field.par_iter()
        .filter_map(|(&index, &value)| {
            if value > threshold {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn parallel_export_to_geojson(
    fields: &[ClimateField],
    frame_registry: &FrameRegistry,
) -> Vec<GeoJson> {
    fields.par_iter()
        .map(|field| {
            export_field_to_geojson(field, frame_registry)
        })
        .collect()
}
```

### 11.6.3 Caching and Pre-computation

For interactive applications, pre-compute common queries:

```rust
struct GeospatialCache {
    // Pre-computed aggregates at different LODs
    lod_summaries: HashMap<u8, HashMap<Index64, Summary>>,

    // Hot cells for fast lookup
    hot_cells: LruCache<Index64, CellData>,
}

struct Summary {
    mean: f32,
    std_dev: f32,
    min: f32,
    max: f32,
    count: u32,
}

impl GeospatialCache {
    fn precompute_lod_summaries(
        &mut self,
        fine_field: &HashMap<Index64, f32>,
        target_lods: &[u8],
    ) {
        for &lod in target_lods {
            let mut lod_summary = HashMap::new();

            // Group fine cells by their parent at this LOD
            let mut groups: HashMap<Index64, Vec<f32>> = HashMap::new();

            for (&fine_index, &value) in fine_field {
                let parent_index = fine_index.parent_at_lod(lod);
                groups.entry(parent_index).or_default().push(value);
            }

            // Compute summary statistics
            for (parent_index, values) in groups {
                let summary = compute_summary(&values);
                lod_summary.insert(parent_index, summary);
            }

            self.lod_summaries.insert(lod, lod_summary);
        }
    }
}

fn compute_summary(values: &[f32]) -> Summary {
    let count = values.len() as u32;
    let mean = values.iter().sum::<f32>() / count as f32;
    let variance = values.iter()
        .map(|&v| (v - mean).powi(2))
        .sum::<f32>() / count as f32;

    Summary {
        mean,
        std_dev: variance.sqrt(),
        min: values.iter().copied().fold(f32::INFINITY, f32::min),
        max: values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        count,
    }
}
```rust

## 11.7 Integration with Geospatial Tools

### 11.7.1 GDAL Integration

Export BCC containers to formats that GDAL can read:

```rust
fn export_to_geotiff(
    field: &HashMap<Index64, f32>,
    frame_registry: &FrameRegistry,
    output_path: &Path,
    bbox: BoundingBox,
    resolution_m: f32,
) -> Result<(), Error> {
    // Determine grid dimensions
    let width = ((bbox.max_lon - bbox.min_lon) * 111_000.0 / resolution_m) as usize;
    let height = ((bbox.max_lat - bbox.min_lat) * 111_000.0 / resolution_m) as usize;

    let mut raster = vec![f32::NAN; width * height];

    // Resample BCC data onto regular grid
    for y in 0..height {
        for x in 0..width {
            let lon = bbox.min_lon + (x as f32 / width as f32) * (bbox.max_lon - bbox.min_lon);
            let lat = bbox.min_lat + (y as f32 / height as f32) * (bbox.max_lat - bbox.min_lat);

            let ecef = wgs84_to_ecef(lat, lon, 0.0);
            let index = frame_registry.ecef_to_index(ecef, 5);

            if let Some(&value) = field.get(&index) {
                raster[y * width + x] = value;
            }
        }
    }

    // Write GeoTIFF using gdal crate
    // ... GDAL-specific code
    Ok(())
}
```

### 11.7.2 PostGIS Integration

Store BCC identifiers in PostGIS for spatial queries:

```sql
-- Create table for BCC-indexed geospatial data
CREATE TABLE bcc_climate_data (
    id SERIAL PRIMARY KEY,
    bcc_index BIGINT NOT NULL,
    location GEOMETRY(PointZ, 4326),
    temperature_k REAL,
    pressure_hpa REAL,
    timestamp TIMESTAMPTZ,
    lod SMALLINT
);

-- Create spatial index
CREATE INDEX idx_bcc_location ON bcc_climate_data USING GIST(location);

-- Create index on BCC identifier for range queries
CREATE INDEX idx_bcc_index ON bcc_climate_data(bcc_index);

-- Query: Find all measurements within a region
SELECT bcc_index, temperature_k, timestamp
FROM bcc_climate_data
WHERE ST_Contains(
    ST_MakeEnvelope(-10, 35, 40, 71, 4326),  -- Europe bbox
    location
)
AND timestamp > NOW() - INTERVAL '24 hours'
ORDER BY temperature_k DESC
LIMIT 100;
```bash

## 11.8 Further Reading

**Geospatial Fundamentals:**
- Longley, P.A., et al. (2015). *Geographic Information Science & Systems* (4th ed.). Wiley.
- Snyder, J.P. (1987). *Map Projections: A Working Manual*. USGS Professional Paper 1395.

**Climate and Atmospheric Modeling:**
- Jacobson, M.Z. (2005). *Fundamentals of Atmospheric Modeling* (2nd ed.). Cambridge University Press.
- Stull, R.B. (2017). *Practical Meteorology: An Algebra-based Survey of Atmospheric Science*. UBC.

**GIS Software and Standards:**
- QGIS Project. (2023). *QGIS User Guide*. https://qgis.org/
- Open Geospatial Consortium. (2023). *OGC Standards*. https://www.ogc.org/standards

**Large-Scale Data Processing:**
- Baumann, P., et al. (2018). "Array databases: Concepts, standards, implementations." *Journal of Big Data*, 5(1), 1-61.
- Li, Z., et al. (2020). "Geospatial big data handling theory and methods: A review and research challenges." *ISPRS Journal of Photogrammetry and Remote Sensing*, 162, 47-60.

**Digital Twins:**
- Batty, M. (2018). "Digital twins." *Environment and Planning B: Urban Analytics and City Science*, 45(5), 817-820.
- Shahat, E., et al. (2021). "City digital twin potentials: A review and research agenda." *Sustainability*, 13(6), 3386.

## 11.9 Summary

In this chapter, we applied OctaIndex3D to geospatial analysis:

- **Atmospheric and environmental models** benefit from efficient, isotropic volumetric sampling.
- **Hierarchical refinement** manages scale and complexity through explicit refinement policies and multi-resolution analysis patterns.
- **GIS integration** (via WGS84 and GeoJSON) enables visualization and interoperability with existing tools such as QGIS.
- **Urban-scale models** link BCC-indexed data to practical decision-making contexts, from planning to real-time digital twins.
- **Practical case studies** demonstrate climate model post-processing and urban air quality monitoring.
- **Performance optimizations** including memory-mapping, parallel processing, and caching enable handling of large-scale datasets.
- **Tool integration** patterns connect OctaIndex3D with GDAL, PostGIS, and other geospatial software.

The next chapter turns to scientific computing applications, where BCC lattices intersect with physics and chemistry simulations.
