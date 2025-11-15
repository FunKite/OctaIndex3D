# Glossary

This glossary collects core terms, acronyms, and symbols used throughout the book. It is meant to feel like an extended margin note: concise, practical, and biased toward the way terms appear in real code and system designs.

---

## Core Concepts

**API (Application Programming Interface)**
A documented boundary for calling into OctaIndex3D from application code. In this book it usually refers to Rust modules, types, and functions exposed by the crate.

**BCC (Body-Centered Cubic lattice)**
A 3D lattice in which each cube has points at its corners and one additional point at its center. In this book, BCC is the default spatial discretization for indexing, storage, and queries.

**BCC octree**
An octree-like hierarchical structure whose leaf cells are BCC Voronoi cells (truncated octahedra) rather than axis-aligned cubes. Supports 14-neighbor connectivity and more isotropic refinements than classical octrees.

**Coordination number**
The number of nearest neighbors to a lattice point. BCC has 14 nearest neighbors, compared to 6 for simple cubic and 26 for the cubic grid with diagonals.

**Fundamental domain**
The volume of space "owned" by each lattice point. For BCC, this is a truncated octahedron with volume proportional to lattice spacing cubed.

**Level of Detail (LOD)**
An integer describing how coarse or fine a representation is. Lower LOD values represent large cells; higher values represent smaller, more detailed cells. Many APIs in OctaIndex3D take an explicit `lod` parameter.

**Parity constraint**
In BCC lattices, the sum $(x + y + z)$ must be even for a point to be a valid lattice point. This constraint is fundamental to BCC geometry and enforced throughout OctaIndex3D.

**Truncated octahedron**
The 14-faced polyhedron that tiles 3D space as the Voronoi cell of the BCC lattice. It has 8 regular hexagonal faces and 6 square faces and is the "unit cell" for many of the constructions in this book.

**Voronoi cell**
For a given lattice point, the region of space closer to that point than to any other. In the BCC lattice this cell is a truncated octahedron, which is why the project is called OctaIndex3D.

---

## Identifier Types

**Bech32m**
A checksum-protected, human-friendly text encoding commonly used in cryptographic systems. OctaIndex3D uses Bech32m to encode identifiers like `Galactic128` into strings that are easy to read, copy, and paste.

**Galactic128**
A 128-bit identifier format designed for globally unique addressing across datasets and deployments. Encodes a coordinate, level of detail, and additional routing or namespace information.

**Hilbert64**
A 64-bit identifier using Hilbert curve ordering instead of Morton encoding. Provides better cache locality for some query patterns.

**Index64**
A 64-bit identifier encoding a BCC coordinate and level of detail using a Morton-like space-filling curve. Used for fast lookups, sorting, and range queries.

**Route64**
A 64-bit identifier encoding the path from root to leaf in a BCC octree. Used for hierarchical traversal and parent-child relationships.

---

## Space-Filling Curves

**Hilbert curve**
A recursive space-filling curve with excellent locality properties. More complex to implement than Morton encoding but often yields tighter cache behavior and fewer page faults in large scans.

**Morton encoding (Z-order)**
A space-filling curve that interleaves the bits of coordinates into a single integer. Provides good locality for range scans, but less optimal than Hilbert curves for some workloads.

---

## Coordinate Systems

**ECEF (Earth-Centered, Earth-Fixed)**
A Cartesian coordinate system with origin at Earth's center, X-axis through the prime meridian at the equator, Z-axis through the north pole. Used as an intermediate frame for geodetic transformations.

**ENU (East-North-Up)**
A local tangent plane coordinate system with origin at a reference point on Earth's surface. X points east, Y points north, Z points up. Commonly used in robotics and navigation.

**Frame**
A coordinate system with a defined origin, orientation, and units. OctaIndex3D's Frame Registry manages transformations between frames.

**Frame Registry**
A system for managing multiple coordinate frames and computing transformation paths between them. Essential for multi-frame geospatial and robotics applications.

**WGS84 (World Geodetic System 1984)**
The standard geodetic reference system used by GPS. Defines Earth's shape as an ellipsoid with specific semi-major and semi-minor axes.

---

## Container Formats

**Container**
A file format for storing collections of spatial data indexed by BCC coordinates. OctaIndex3D supports sequential and streaming container formats.

**Delta encoding**
A compression technique that stores differences between consecutive values instead of absolute values. Achieves 10-15% compression on sorted BCC identifiers.

**Sequential container**
A container format optimized for bulk writes and full-file reads. Uses compression blocks and supports crash recovery.

**Streaming container**
A container format optimized for low-latency append-only writes. Used for logging and real-time data ingestion.

---

## Hardware and Performance

**AVX2 (Advanced Vector Extensions 2)**
A SIMD instruction set extension for x86-64 CPUs. Provides 256-bit vector operations used in OctaIndex3D for batch encoding and queries.

**BMI2 (Bit Manipulation Instruction Set 2)**
An extension to the x86-64 instruction set that provides fast bit field extraction and deposit operations. Used to implement high-performance Morton encoding and decoding on supported CPUs.

**CUDA (Compute Unified Device Architecture)**
NVIDIA's GPU programming platform. OctaIndex3D includes experimental CUDA kernels for bulk spatial queries.

**GPU (Graphics Processing Unit)**
A specialized processor for parallel computation. OctaIndex3D supports GPU acceleration for batch operations on Metal, CUDA, and Vulkan.

**Metal**
Apple's GPU programming framework. OctaIndex3D uses Metal for GPU acceleration on macOS and iOS.

**NEON**
ARM's SIMD instruction set extension. Provides 128-bit vector operations used for batch encoding on ARM processors.

**SIMD (Single Instruction, Multiple Data)**
A hardware feature that allows the same operation to run on multiple data elements in parallel. OctaIndex3D uses SIMD instructions (such as NEON and AVX2) to accelerate Morton/Hilbert encoding, neighbor lookup, and aggregation.

**Vulkan**
A cross-platform GPU API. OctaIndex3D supports Vulkan for GPU-accelerated spatial queries on Linux and Windows.

---

## Testing and Development

**cargo-fuzz**
A Rust fuzzing tool that generates random inputs to find bugs. Used in OctaIndex3D to test encoding/decoding and container parsing.

**Criterion**
A Rust benchmarking library that provides statistical analysis of performance. All OctaIndex3D benchmarks use Criterion.

**proptest**
A property-based testing framework for Rust. Used to verify mathematical invariants (e.g., encode-decode round trips) across large input spaces.

---

## Geospatial and GIS

**GDAL (Geospatial Data Abstraction Library)**
An open-source library for reading and writing raster and vector geospatial data formats. OctaIndex3D integrates with GDAL for importing GIS data.

**GeoJSON**
A JSON-based format for encoding geographic data structures. OctaIndex3D can export BCC cells and queries to GeoJSON for visualization in QGIS and web maps.

**GIS (Geographic Information System)**
Software systems for capturing, storing, analyzing, and managing spatial and geographic data. OctaIndex3D integrates with GIS tools like QGIS and PostGIS.

**H3**
Uber's hexagonal hierarchical geospatial indexing system. Alternative to OctaIndex3D for 2D/spherical use cases.

**LiDAR (Light Detection and Ranging)**
A remote sensing method using laser pulses to measure distances. Generates point clouds commonly indexed with OctaIndex3D.

**PostGIS**
A spatial database extension for PostgreSQL. OctaIndex3D can export to PostGIS for integration with existing GIS workflows.

**QGIS (Quantum GIS)**
An open-source desktop GIS application. OctaIndex3D exports to formats compatible with QGIS for visualization and analysis.

**S2**
Google's spherical geometry library for geographic data. Alternative to OctaIndex3D for spherical/planetary use cases.

---

## Robotics and Scientific Computing

**NetCDF (Network Common Data Form)**
A file format for storing array-oriented scientific data. Commonly used in climate modeling; OctaIndex3D can import NetCDF grids.

**Rayon**
A Rust library for data parallelism. OctaIndex3D uses Rayon for parallel batch operations and multi-threaded queries.

**SLAM (Simultaneous Localization and Mapping)**
A technique for building maps while tracking position. Robotics applications use OctaIndex3D for occupancy grid storage in SLAM systems.

**UAV (Unmanned Aerial Vehicle)**
Autonomous drones. Example use case throughout the book for path planning and spatial indexing.

---

## Integration and Ecosystem

**Bevy**
A data-driven game engine for Rust. OctaIndex3D integrates with Bevy for voxel worlds and spatial queries in games.

**Godot**
An open-source game engine. OctaIndex3D provides GDNative bindings for use with Godot's GDScript.

**PyO3**
A Rust library for creating Python bindings. OctaIndex3D uses PyO3 to expose its API to Python for scientific computing workflows.

**WASM (WebAssembly)**
A binary instruction format for web browsers. OctaIndex3D compiles to WASM for browser-based visualization and spatial queries.

---

## Data Structures

**Octree**
A hierarchical tree structure where each internal node has exactly eight children. Classical octrees use cubic cells; OctaIndex3D uses truncated octahedra.

**R-tree**
A tree data structure for indexing spatial information. Alternative to octrees for bounding-box queries.

---

## Comparison Systems

The following terms refer to alternative spatial indexing systems that are compared with OctaIndex3D throughout the book:

**Cubic grid**
A simple spatial discretization using axis-aligned cubes. Simpler than BCC but suffers from directional bias (6 vs 26-neighbor ambiguity).

**FCC (Face-Centered Cubic lattice)**
An alternative space-filling lattice with 12 nearest neighbors. More isotropic than simple cubic but not as efficient as BCC for many use cases.

**SC (Simple Cubic lattice)**
The simplest 3D lattice with points at integer coordinates. Has only 6 nearest neighbors and exhibits strong directional bias.
