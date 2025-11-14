# Chapter 6: Coordinate Reference Systems

## Learning Objectives

By the end of this chapter, you will be able to:

1. Explain the role of coordinate reference systems (CRS) in OctaIndex3D.
2. Describe the design and purpose of the frame registry.
3. Distinguish between built-in frames and custom application-defined frames.
4. Understand how coordinate transformations are implemented and validated.
5. Integrate OctaIndex3D with GIS and other CRS-aware systems.

---

## 6.1 The Frame Registry

In Part I, we treated coordinates abstractly—as points in $\mathbb{R}^3$ that could be sampled on a BCC lattice. Real systems, however, must answer a more concrete question:

> “Which coordinate system are these numbers expressed in?”

Latitude/longitude, Earth-centered Cartesian coordinates, local ENU (East–North–Up), and game-world coordinates are all common examples. Confusing one for another can produce spectacularly wrong results.

To prevent such mistakes, OctaIndex3D centers its CRS design around a **frame registry**:

- A strongly-typed mapping from **frame identifiers** to **transformation functions** and **metadata**.
- A single, authoritative location where CRS-related information lives.
- A thread-safe resource that can be shared across the application.

Architecturally, the frame registry:

- Provides immutable frame definitions once constructed.
- Exposes handles that can be cheaply cloned and passed around.
- Avoids global mutable state; applications create and own registries explicitly.

---

## 6.2 Built-in Coordinate Systems

OctaIndex3D ships with a small set of **built-in frames** that cover common use cases:

- **ECEF (Earth-Centered, Earth-Fixed)**: a Cartesian frame with origin at the Earth’s center of mass.
- **WGS84 Geodetic**: latitude, longitude, and ellipsoidal height above the WGS84 ellipsoid.
- **ENU (East–North–Up)**: local tangent-plane frames attached to specific anchor points.
- **Generic Cartesian**: unit-agnostic, application-defined Cartesian frames (e.g., game worlds measured in meters or arbitrary units).

Each built-in frame:

- Has a stable identifier string (e.g., `"ECEF"`, `"WGS84"`).
- Provides forward and inverse transformations to a canonical internal Cartesian representation.
- Encodes any necessary metadata (ellipsoid parameters, epochs, geoid models).

### 6.2.1 WGS84 and ECEF in Practice

Most real-world geospatial data is expressed in WGS84 latitude/longitude and either ellipsoidal or orthometric height. OctaIndex3D’s built-in WGS84 frame wraps the familiar formulas:

- **Forward (WGS84 → ECEF)**: convert $(\varphi, \lambda, h)$ to $(x, y, z)$ using the WGS84 semi-major axis $a$, eccentricity $e$, and prime vertical radius of curvature $N(\varphi)$.
- **Inverse (ECEF → WGS84)**: recover geodetic coordinates from $(x, y, z)$ using an iterative or closed-form algorithm.

Conceptually:

```text
WGS84 (lat, lon, h)
    ⇄  ECEF (x, y, z)
    ⇄  Local ENU (e, n, u)
```

OctaIndex3D does not attempt to *replace* established geodesy libraries; instead, it:

- Encapsulates the formulas and constants behind a stable API.
- Provides hooks to integrate with external libraries when higher geodetic fidelity is required (e.g., time-varying plate motion models or alternate datums).

From the perspective of the spatial index:

- WGS84 is “just another frame” with a known transform to the canonical Cartesian representation.
- Higher-level systems (GIS, navigation, mapping) are free to treat the WGS84 frame as their primary interface, while OctaIndex3D handles the conversion to lattice indices.

Applications can:

- Use the built-in frames directly.
- Define local frames anchored in built-in frames.
- Mix built-in and custom frames in the same registry, as long as transformations are defined.

---

## 6.3 Custom Frame Definition

Many applications need their own frames:

- A **warehouse** with an origin at a loading dock.
- A **simulation** with an origin at a scenario-specific point.
- A **vehicle** with axes aligned to its body frame.

Defining a custom frame involves specifying:

- A unique **name** (e.g., `"warehouse_A"`, `"sim:scenario_12"`).
- A transformation from the custom frame to a chosen parent frame.
- The inverse transformation from the parent frame back to the custom frame.
- Optional metadata such as units, descriptions, and validity periods.

In Rust, the API might look conceptually like:

```rust
let frame = registry.register_frame(FrameDefinition::builder("warehouse_A")
    .parent("ECEF")
    .forward(|local: Vec3| -> Vec3 { /* ... */ })
    .inverse(|ecef: Vec3| -> Vec3 { /* ... */ })
    .build()?);
```

Internally, the registry:

- Validates that parent frames exist.
- Checks that forward and inverse transformations are well-typed.
- Ensures that frame names are unique.

Once registered, frames become immutable—any changes must go through explicit migration paths, keeping behavior reproducible.

### 6.3.1 Local ENU Frames

A common pattern is to define **local ENU frames** anchored at specific locations in WGS84:

- Choose an anchor point $(\varphi_0, \lambda_0, h_0)$ in WGS84.
- Convert it to ECEF coordinates $(x_0, y_0, z_0)$.
- Define a local orthonormal basis $(\hat{e}, \hat{n}, \hat{u})$ at that point.

The forward transform ENU → ECEF then becomes:

```text
P_ecef = P_anchor + e * \hat{e} + n * \hat{n} + u * \hat{u}
```

and the inverse ECEF → ENU subtracts the anchor and projects onto the basis vectors.

OctaIndex3D’s frame definition API captures this structure explicitly:

- Frames are tagged as “local” or “global”.
- Metadata records the anchor geodetic point and parent frame.
- Transformations are guaranteed to be continuous and invertible in a neighborhood of the anchor.

This allows applications to:

- Attach multiple ENU frames to different sites (warehouses, depots, landing pads).
- Convert between them via the shared parent (e.g., WGS84/ECEF).

---

## 6.4 Coordinate Transformations

Coordinate transformations are at the heart of the frame system. Given:

- A source frame `F_src`.
- A destination frame `F_dst`.
- A point `p_src` expressed in `F_src`.

We want to compute `p_dst` expressed in `F_dst`. Architecturally, this is handled by:

1. **Finding a transformation path** through the frame graph (often via a canonical frame such as ECEF).
2. **Composing** the forward and inverse transformations along that path.
3. **Applying** the resulting transformation to the input point.

The frame registry caches these compositions so that:

- Common transformations (e.g., `WGS84` → `ECEF` → local ENU) are efficient.
- Applications do not pay repeated overhead for path discovery.

### 6.4.1 Precision Considerations

At high levels of detail (LOD), small numerical errors can move a point across BCC cell boundaries. To mitigate this:

- Transformations use double-precision floating-point arithmetic by default.
- Critical code paths are tested with adversarial inputs (e.g., points near frame boundaries).
- Documentation highlights the expected precision and failure modes.

When transforming into lattice indices:

- The library rounds consistently according to a documented convention (e.g., round-to-nearest with ties broken toward even).
- Any ambiguity is resolved before identifiers are constructed, so that invariants like parity are preserved.

### 6.4.2 From Coordinates to Lattice Indices

The final step from a continuous coordinate to a lattice identifier is where the CRS machinery meets the BCC theory from Part I:

1. Start with a point expressed in some frame (e.g., WGS84, local ENU, game world).  
2. Transform it into the canonical Cartesian frame associated with the BCC lattice.  
3. Apply the lattice quantization rules (Section 2.x) to find the nearest BCC lattice point.  
4. Construct an appropriate identifier (`Index64`, `Hilbert64`, or `Galactic128`) using smart constructors.  

Architecturally, this step is centralized in a small set of functions:

- `frame.to_index64(point, lod) -> Result<Index64, IndexError>`
- `frame.to_galactic(point, lod) -> Result<Galactic128, IndexError>`

These functions:

- Apply frame transformations.
- Enforce LOD bounds.
- Handle edge cases near cell boundaries deterministically.

By keeping this logic in one place, OctaIndex3D:

- Ensures consistent behavior across all higher-level APIs.
- Makes it straightforward to test precision and parity invariants in isolation.

---

## 6.5 GIS Integration

Many users of OctaIndex3D interact with Geographic Information Systems (GIS) such as QGIS, PostGIS, or GDAL-based tools. To integrate smoothly:

- Frames can be tagged with **EPSG codes** or Well-Known Text (WKT) representations.
- Import and export functions map between OctaIndex3D frames and GIS CRS definitions.
- GeoJSON and similar formats can be generated from indexed data.

Architecturally, this means:

- The frame registry can serve as a bridge between OctaIndex3D and external CRS catalogs.
- Application code can treat OctaIndex3D frames as first-class CRS objects rather than opaque IDs.

Care is taken to:

- Surface mismatches between CRS definitions as explicit errors.
- Avoid silent unit conversions (e.g., meters vs. feet).
- Preserve metadata necessary for round-trip fidelity where possible.

### 6.5.1 Case Study: WGS84 Tiles to OctaIndex3D

Consider a workflow where:

1. You receive terrain tiles as GeoTIFFs in WGS84 (EPSG:4326).  
2. You want to index them in OctaIndex3D for fast 3D queries.  

An integration pipeline might:

- Use GDAL (or a similar library) to read GeoTIFFs and obtain georeferenced coordinates.
- Map the GeoTIFF’s CRS to the corresponding OctaIndex3D frame (e.g., `"EPSG:4326"` → `WGS84`).
- For each sample point or pixel center:
  - Convert WGS84 → ECEF → canonical Cartesian.
  - Quantize to a BCC lattice point at the desired LOD.
  - Construct an `Index64` and store associated elevation or intensity values in a container.

On export:

- Reverse the process: identifiers → canonical Cartesian → WGS84 → GeoTIFF or GeoJSON.
- Preserve CRS metadata and any tiling scheme identifiers (e.g., TMS, XYZ) alongside the container.

From the application’s perspective:

- GIS tools “see” familiar WGS84 data.
- OctaIndex3D “sees” only Cartesian coordinates and lattice indices.
- The frame registry sits in the middle, ensuring that conversions are consistent and explicit.

---

## 6.6 Thread Safety and Concurrency

Coordinate transformations are often invoked in performance-critical, multi-threaded contexts:

- Batch queries over large datasets.
- Real-time robotics pipelines.
- Server-side APIs handling concurrent requests.

To support these scenarios:

- Frame registries are **immutable** once constructed; they can be shared across threads without locking.
- Transformation caches are implemented using thread-safe primitives or are built eagerly to avoid contention.
- Per-thread scratch space is used where necessary to avoid heap allocations.

From an architectural standpoint, this means:

- Applications can safely clone handles to the registry and use them freely in parallel code.
- The cost of CRS handling remains predictable under load.

### 6.6.1 Caching Strategies

Caching deserves special care in concurrent systems:

- **Path caches** store precomputed transformation chains between frequently-used frame pairs (e.g., `WGS84` ↔ `ECEF`, `ECEF` ↔ `local_warehouse_ENU`).
- **Parameter caches** hold derived constants for expensive geodetic formulas.

OctaIndex3D’s architecture favors:

- Read-only caches shared across threads once constructed.
- Construction-time initialization where feasible, so that no synchronization is needed in hot paths.
- Clear documentation about which caches are per-registry and which are per-process, so that applications can reason about memory usage.

---

## 6.7 Summary

In this chapter, we examined how OctaIndex3D models coordinate reference systems:

- The **frame registry** provides a central, strongly-typed repository of CRS definitions.
- **Built-in frames** cover common scenarios such as ECEF, WGS84, ENU, and generic Cartesian coordinates.
- **Custom frames** allow applications to model domain-specific coordinate systems with explicit transformations.
- **Coordinate transformations** are composed and cached carefully, with attention to precision and reproducibility.
- **GIS integration** and **thread safety** ensure that OctaIndex3D plays well with existing ecosystems and high-concurrency workloads.

With frames and identifiers in place, Part II has now established the architectural context needed to understand the implementation details of Part III.
