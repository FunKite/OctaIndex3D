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
```

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

## 11.5 Summary

In this chapter, we applied OctaIndex3D to geospatial analysis:

- **Atmospheric and environmental models** benefit from efficient, isotropic volumetric sampling.
- **Hierarchical refinement** manages scale and complexity through explicit refinement policies and multi-resolution analysis patterns.
- **GIS integration** (via WGS84 and GeoJSON) enables visualization and interoperability with existing tools such as QGIS.
- **Urban-scale models** link BCC-indexed data to practical decision-making contexts, from planning to real-time digital twins.

The next chapter turns to scientific computing applications, where BCC lattices intersect with physics and chemistry simulations.
