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
- **WGS84 Geodetic**: latitude, longitude, and height above the WGS84 ellipsoid.
- **ENU (East–North–Up)**: local tangent-plane frames attached to specific anchor points.
- **Generic Cartesian**: unit-agnostic, application-defined Cartesian frames (e.g., game worlds measured in meters or arbitrary units).

Each built-in frame:

- Has a stable identifier string (e.g., `"ECEF"`, `"WGS84"`).
- Provides forward and inverse transformations to a canonical internal Cartesian representation.
- Encodes any necessary metadata (ellipsoid parameters, epochs, geoid models).

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

---

## 6.7 Summary

In this chapter, we examined how OctaIndex3D models coordinate reference systems:

- The **frame registry** provides a central, strongly-typed repository of CRS definitions.
- **Built-in frames** cover common scenarios such as ECEF, WGS84, ENU, and generic Cartesian coordinates.
- **Custom frames** allow applications to model domain-specific coordinate systems with explicit transformations.
- **Coordinate transformations** are composed and cached carefully, with attention to precision and reproducibility.
- **GIS integration** and **thread safety** ensure that OctaIndex3D plays well with existing ecosystems and high-concurrency workloads.

With frames and identifiers in place, Part II has now established the architectural context needed to understand the implementation details of Part III.

