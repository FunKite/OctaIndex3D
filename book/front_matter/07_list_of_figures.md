# List of Figures

## Part I: Foundations

### Chapter 1
- Figure 1.1: Cubic grid vs. BCC lattice comparison
- Figure 1.2: Directional bias in cubic grids
- Figure 1.3: Voronoi cell comparison (cube vs. truncated octahedron)
- Figure 1.4: Application domains overview
- Figure 1.5: Book organization diagram

### Chapter 2
- Figure 2.1: Simple cubic, FCC, and BCC lattice structures
- Figure 2.2: BCC lattice parity constraint visualization
- Figure 2.3: Truncated octahedron construction
- Figure 2.4: 14-neighbor connectivity diagram
- Figure 2.5: Hierarchical refinement (8:1 subdivision)
- Figure 2.6: Isotropy coefficient comparison
- Figure 2.7: Nyquist sampling in 3D space
- Figure 2.8: Voronoi tessellation of BCC lattice

### Chapter 3
- Figure 3.1: Classical octree structure
- Figure 3.2: BCC octree node relationships
- Figure 3.3: Parent-child mapping in BCC hierarchies
- Figure 3.4: Neighbor finding algorithm flowchart
- Figure 3.5: Z-order (Morton) curve in 2D and 3D
- Figure 3.6: Hilbert curve progression (orders 1-4)
- Figure 3.7: Cache locality comparison (Morton vs. Hilbert)
- Figure 3.8: Space-filling curve bit interleaving

## Part II: Architecture and Design

### Chapter 4
- Figure 4.1: OctaIndex3D system architecture overview
- Figure 4.2: Type hierarchy diagram
- Figure 4.3: Memory layout of core types
- Figure 4.4: Data flow through the system
- Figure 4.5: Error handling decision tree

### Chapter 5
- Figure 5.1: Galactic128 bit layout
- Figure 5.2: Index64 bit layout
- Figure 5.3: Route64 bit layout
- Figure 5.4: Hilbert64 bit layout
- Figure 5.5: Identifier type conversion matrix
- Figure 5.6: Bech32m encoding process
- Figure 5.7: Scale comparison across identifier types
- Figure 5.8: Use case decision diagram

### Chapter 6
- Figure 6.1: Frame registry architecture
- Figure 6.2: ECEF coordinate system
- Figure 6.3: Coordinate transformation pipeline
- Figure 6.4: WGS84 to local frame conversion
- Figure 6.5: Thread-safe registry access pattern

## Part III: Implementation

### Chapter 7
- Figure 7.1: Modern CPU architecture overview
- Figure 7.2: BMI2 PDEP instruction visualization
- Figure 7.3: SIMD lane processing diagram
- Figure 7.4: NEON register layout
- Figure 7.5: AVX2 vectorization example
- Figure 7.6: Cache hierarchy and access patterns
- Figure 7.7: Batch size vs. throughput curve
- Figure 7.8: GPU kernel execution model
- Figure 7.9: Performance scaling across hardware
- Figure 7.10: Benchmark statistical analysis

### Chapter 8
- Figure 8.1: Container v1 format layout
- Figure 8.2: Container v2 streaming format
- Figure 8.3: Frame structure with compression
- Figure 8.4: TOC and footer organization
- Figure 8.5: Checkpoint mechanism visualization
- Figure 8.6: Crash recovery process
- Figure 8.7: Compression ratio vs. codec comparison
- Figure 8.8: Format migration workflow

### Chapter 9
- Figure 9.1: Test pyramid for OctaIndex3D
- Figure 9.2: Property-based testing example
- Figure 9.3: Criterion.rs output interpretation
- Figure 9.4: Performance regression dashboard
- Figure 9.5: CI/CD pipeline architecture

## Part IV: Applications

### Chapter 10
- Figure 10.1: 3D occupancy grid representation
- Figure 10.2: Sensor fusion pipeline
- Figure 10.3: A* search visualization on BCC lattice
- Figure 10.4: UAV trajectory comparison (cubic vs. BCC)
- Figure 10.5: Real-time performance timeline

### Chapter 11
- Figure 11.1: Atmospheric model grid refinement
- Figure 11.2: Hierarchical LOD selection
- Figure 11.3: GeoJSON export workflow
- Figure 11.4: QGIS visualization of BCC data
- Figure 11.5: Urban 3D model example
- Figure 11.6: Multi-scale environmental dataset

### Chapter 12
- Figure 12.1: FCC to BCC lattice mapping
- Figure 12.2: Molecular dynamics neighbor search
- Figure 12.3: CFD grid comparison
- Figure 12.4: Volumetric rendering with BCC voxels
- Figure 12.5: Particle simulation spatial hashing

### Chapter 13
- Figure 13.1: Voxel terrain representation
- Figure 13.2: Procedural maze generation
- Figure 13.3: NPC pathfinding comparison
- Figure 13.4: Spatial partitioning for collision detection
- Figure 13.5: LOD transition visualization
- Figure 13.6: 3D maze game screenshot

## Part V: Advanced Topics

### Chapter 14
- Figure 14.1: Spatial partitioning strategy
- Figure 14.2: Apache Arrow memory layout
- Figure 14.3: Distributed A* communication pattern
- Figure 14.4: Map-reduce on spatial data
- Figure 14.5: Scalability benchmarks

### Chapter 15
- Figure 15.1: GNN architecture on BCC graph
- Figure 15.2: Point cloud processing pipeline
- Figure 15.3: 3D object detection bounding boxes
- Figure 15.4: Trajectory prediction neural network
- Figure 15.5: PyTorch integration diagram

### Chapter 16
- Figure 16.1: Research roadmap timeline
- Figure 16.2: Hilbert state machine optimization
- Figure 16.3: Compression-aware query plan
- Figure 16.4: BCC-native ray marching
- Figure 16.5: Future hardware trends

## Appendices

### Appendix A
- Figure A.1: Voronoi cell geometric proof
- Figure A.2: Sampling efficiency diagram
- Figure A.3: Isotropy measurement visualization

### Appendix C
- Figure C.1: Cross-platform benchmark comparison
- Figure C.2: Performance vs. competitors
- Figure C.3: Scaling analysis plots
