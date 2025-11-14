# Glossary

This glossary collects core terms, acronyms, and symbols used throughout the book. It is meant to feel like an extended margin note: concise, practical, and biased toward the way terms appear in real code and system designs.

The entries below are a starting set; the plan is to expand this glossary as additional chapters are completed.

---

**BCC (Body‑Centered Cubic lattice)**  
A 3D lattice in which each cube has points at its corners and one additional point at its center. In this book, BCC is the default spatial discretization for indexing, storage, and queries.

**BCC octree**  
An octree‑like hierarchical structure whose leaf cells are BCC Voronoi cells (truncated octahedra) rather than axis‑aligned cubes. Supports 14‑neighbor connectivity and more isotropic refinements than classical octrees.

**Galactic128**  
A 128‑bit identifier format designed for globally unique addressing across datasets and deployments. Encodes a coordinate, level of detail, and additional routing or namespace information.

**Index64**  
A 64‑bit identifier encoding a BCC coordinate and level of detail using a Morton‑like space‑filling curve. Used for fast lookups, sorting, and range queries.

**Level of Detail (LOD)**  
An integer describing how coarse or fine a representation is. Lower LOD values represent large cells; higher values represent smaller, more detailed cells. Many APIs in OctaIndex3D take an explicit `lod` parameter.

**Morton encoding (Z‑order)**  
A space‑filling curve that interleaves the bits of coordinates into a single integer. Provides good locality for range scans, but less optimal than Hilbert curves for some workloads.

**Hilbert curve**  
A recursive space‑filling curve with excellent locality properties. More complex to implement than Morton encoding but often yields tighter cache behavior and fewer page faults in large scans.

**SIMD (Single Instruction, Multiple Data)**  
A hardware feature that allows the same operation to run on multiple data elements in parallel. OctaIndex3D uses SIMD instructions (such as NEON and AVX2) to accelerate Morton/Hilbert encoding, neighbor lookup, and aggregation.

**BMI2 (Bit Manipulation Instruction Set 2)**  
An extension to the x86‑64 instruction set that provides fast bit field extraction and deposit operations. Used to implement high‑performance Morton encoding and decoding on supported CPUs.

**Voronoi cell**  
For a given lattice point, the region of space closer to that point than to any other. In the BCC lattice this cell is a truncated octahedron, which is why the project is called OctaIndex3D.

**Truncated octahedron**  
The 14‑faced polyhedron that tiles 3D space as the Voronoi cell of the BCC lattice. It has 8 regular hexagonal faces and 6 square faces and is the “unit cell” for many of the constructions in this book.

