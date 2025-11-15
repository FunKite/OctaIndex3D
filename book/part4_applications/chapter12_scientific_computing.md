# Chapter 12: Scientific Computing

## Learning Objectives

By the end of this chapter, you will be able to:

1. Explain how BCC lattices can be used in molecular dynamics and crystallography.
2. Understand how OctaIndex3D supports computational fluid dynamics (CFD) and volumetric data analysis.
3. Design neighbor search patterns for particle simulations.
4. Evaluate trade-offs between BCC and domain-specific meshes.

---

## 12.1 Molecular Dynamics and Crystallography

In molecular dynamics and crystallography:

- Atoms are arranged in crystal lattices, often including BCC structures.
- Efficient neighbor search is critical for computing forces and energies.

OctaIndex3D can:

- Represent atomic positions in frames aligned with crystal axes.
- Use BCC indexing to accelerate spatial queries.

Because BCC lattices arise naturally in some crystal structures, using a BCC-based index:

- Aligns the computational grid with the physical structure.
- Reduces artifacts when computing properties that depend on local environment.

In a simple workflow, you might:

1. Choose a frame aligned with the crystal axes (for example, with basis vectors matching the unit cell).
2. Map atomic positions into that frame and discretize them to BCC cells at an appropriate LOD.
3. Use OctaIndex3D containers to store per-cell aggregates (such as local density or potential energy).
4. Run neighbor queries over the BCC grid to compute short-range interactions or identify defect structures.

Because the indexing structure is separate from the physics code, you can experiment with different cutoffs, interaction models, or coarsening strategies without changing how data is stored.

---

### 12.1.1 Crystallography Case Study

Consider a crystallography workload that:

- Analyzes a large supercell to detect defects and dislocations.
- Needs fast neighborhood queries around each atom.
- Produces statistics on local coordination environments.

An OctaIndex3D-based approach might:

1. Define a frame whose basis vectors match the crystal lattice vectors.
2. Map each atom’s position into this frame and assign it to a BCC cell.
3. Maintain containers keyed by `Index64` that:
   - Store atom IDs or small atom lists per cell.
   - Store derived quantities such as local density or average bond length.
4. For each atom, query:
   - Neighboring cells within a cutoff radius.
   - Atoms in those cells for exact distance checks.

Because the frame is aligned with the lattice, identifying slip planes, grain boundaries, or vacancy clusters becomes a matter of:

- Selecting ranges of indices corresponding to specific crystallographic directions.
- Aggregating statistics over those ranges.

This separates the indexing and neighbor search concerns from the underlying physics and visualization tools.

### 12.1.2 Molecular Modeling on BCC Grids

Molecular modeling workflows often combine:

- Discrete atoms and bonds.
- Continuous fields (electrostatic potential, electron density).

OctaIndex3D can host the **continuous fields**:

- Scalar fields sampled on BCC cells (e.g., potential, density).
- Vector fields (e.g., gradient of potential) approximated using neighbor stencils.

A typical workflow:

1. Generate or import a continuous field on a conventional grid.
2. Resample it onto a BCC lattice at one or more LODs.
3. Use BCC neighbor queries to:
   - Compute gradients.
   - Integrate along field lines.
   - Identify regions where thresholds are crossed.

This opens the door to:

- Multi-resolution representations of binding pockets or channels.
- Fast queries such as "find all regions where potential is below a threshold within a given distance of the protein surface".

#### Code Example: Molecular Dynamics Neighbor Search

Here's a practical example of using OctaIndex3D for neighbor search in molecular dynamics:

```rust
use octaindex3d::{Index64, Frame, Container};
use std::collections::HashMap;

/// Atom data structure
#[derive(Clone, Debug)]
struct Atom {
    id: u32,
    position: [f64; 3],
    atom_type: String,
    charge: f64,
}

/// Molecular dynamics neighbor search using BCC indexing
struct MDNeighborSearch {
    frame: Frame,
    lod: u8,
    cutoff_radius: f64,
    atom_bins: HashMap<Index64, Vec<Atom>>,
}

impl MDNeighborSearch {
    /// Create a new neighbor search structure
    fn new(frame: Frame, lod: u8, cutoff_radius: f64) -> Self {
        Self {
            frame,
            lod,
            cutoff_radius,
            atom_bins: HashMap::new(),
        }
    }

    /// Insert an atom into the spatial index
    fn insert_atom(&mut self, atom: Atom) {
        let idx = self.frame.coords_to_index(&atom.position, self.lod)
            .expect("Failed to convert coordinates to index");

        self.atom_bins.entry(idx)
            .or_insert_with(Vec::new)
            .push(atom);
    }

    /// Find all atoms within cutoff radius of a target position
    fn find_neighbors(&self, target_pos: [f64; 3]) -> Vec<&Atom> {
        let target_idx = self.frame.coords_to_index(&target_pos, self.lod)
            .expect("Failed to convert target coordinates");

        let mut neighbors = Vec::new();
        let cutoff_sq = self.cutoff_radius * self.cutoff_radius;

        // Query the target cell and all neighboring cells
        let candidate_cells = self.get_neighbor_cells(target_idx);

        for cell_idx in candidate_cells {
            if let Some(atoms) = self.atom_bins.get(&cell_idx) {
                for atom in atoms {
                    let dist_sq = distance_squared(&target_pos, &atom.position);
                    if dist_sq <= cutoff_sq {
                        neighbors.push(atom);
                    }
                }
            }
        }

        neighbors
    }

    /// Get neighboring cells within the cutoff radius
    fn get_neighbor_cells(&self, center: Index64) -> Vec<Index64> {
        let mut cells = vec![center];

        // Add 14 nearest BCC neighbors
        for neighbor in center.neighbors_14() {
            cells.push(neighbor);
        }

        // For larger cutoffs, add second shell of neighbors
        if self.needs_second_shell() {
            for &first_neighbor in &cells.clone() {
                for second_neighbor in first_neighbor.neighbors_14() {
                    if !cells.contains(&second_neighbor) {
                        cells.push(second_neighbor);
                    }
                }
            }
        }

        cells
    }

    fn needs_second_shell(&self) -> bool {
        // Heuristic: need second shell if cutoff is larger than cell size
        let cell_size = self.frame.cell_size_at_lod(self.lod);
        self.cutoff_radius > cell_size * 1.5
    }

    /// Compute interaction energy for a configuration
    fn compute_total_energy(&self) -> f64 {
        let mut total_energy = 0.0;

        for atoms in self.atom_bins.values() {
            for atom in atoms {
                let neighbors = self.find_neighbors(atom.position);
                for neighbor in neighbors {
                    if neighbor.id != atom.id {
                        total_energy += lennard_jones_potential(
                            distance(&atom.position, &neighbor.position),
                            atom.atom_type.as_str(),
                            neighbor.atom_type.as_str(),
                        );
                    }
                }
            }
        }

        // Divide by 2 because each pair is counted twice
        total_energy / 2.0
    }
}

/// Compute squared Euclidean distance
fn distance_squared(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

/// Compute Euclidean distance
fn distance(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    distance_squared(a, b).sqrt()
}

/// Lennard-Jones potential (simplified)
fn lennard_jones_potential(r: f64, type_a: &str, type_b: &str) -> f64 {
    let epsilon = get_epsilon(type_a, type_b);
    let sigma = get_sigma(type_a, type_b);

    let r6 = (sigma / r).powi(6);
    let r12 = r6 * r6;

    4.0 * epsilon * (r12 - r6)
}

fn get_epsilon(_type_a: &str, _type_b: &str) -> f64 {
    // Simplified: return constant
    1.0
}

fn get_sigma(_type_a: &str, _type_b: &str) -> f64 {
    // Simplified: return constant
    3.4
}
```

### 12.1.3 Crystal Structure Analysis

BCC lattices are particularly well-suited for crystallographic applications because many metal crystals naturally form BCC structures (e.g., iron, chromium, tungsten at room temperature). This alignment between the computational grid and physical structure provides several advantages:

**Reduced computational artifacts**: When the computational lattice matches the crystal lattice, numerical artifacts from discretization are minimized.

**Efficient defect detection**: Vacancies, interstitials, and dislocations can be identified by comparing local coordination numbers against ideal BCC values.

**Grain boundary analysis**: The consistent neighbor relationships make it easier to identify and characterize grain boundaries.

#### Example: Defect Detection Workflow

```rust
use octaindex3d::{Index64, Container};

/// Represents a crystallographic defect
#[derive(Debug, Clone, Copy, PartialEq)]
enum DefectType {
    Vacancy,
    Interstitial,
    Substitutional,
    Dislocation,
    None,
}

/// Analyze crystal structure for defects
struct CrystalDefectAnalyzer {
    occupancy: Container<Index64, bool>,
    coordination_numbers: Container<Index64, u8>,
    lod: u8,
}

impl CrystalDefectAnalyzer {
    fn new(lod: u8) -> Self {
        Self {
            occupancy: Container::new(),
            coordination_numbers: Container::new(),
            lod,
        }
    }

    /// Compute coordination number for each occupied site
    fn compute_coordination_numbers(&mut self) {
        for (&idx, &occupied) in self.occupancy.iter() {
            if occupied {
                let coord_num = self.count_neighbors(idx);
                self.coordination_numbers.insert(idx, coord_num);
            }
        }
    }

    /// Count occupied neighbors for a given site
    fn count_neighbors(&self, idx: Index64) -> u8 {
        let mut count = 0;
        for neighbor in idx.neighbors_14() {
            if self.occupancy.get(&neighbor).copied().unwrap_or(false) {
                count += 1;
            }
        }
        count
    }

    /// Identify defect type at a given site
    fn classify_defect(&self, idx: Index64) -> DefectType {
        let occupied = self.occupancy.get(&idx).copied().unwrap_or(false);
        let coord_num = self.coordination_numbers.get(&idx).copied().unwrap_or(0);

        // In ideal BCC, each atom has 8 nearest neighbors in the first shell
        // and 6 in the second shell (14 total in our BCC lattice)
        const IDEAL_COORDINATION: u8 = 14;

        match (occupied, coord_num) {
            (false, _) => DefectType::Vacancy,
            (true, n) if n > IDEAL_COORDINATION => DefectType::Interstitial,
            (true, n) if n < IDEAL_COORDINATION - 2 => DefectType::Dislocation,
            (true, IDEAL_COORDINATION) => DefectType::None,
            _ => DefectType::Substitutional,
        }
    }

    /// Generate defect statistics report
    fn defect_statistics(&self) -> DefectStatistics {
        let mut stats = DefectStatistics::default();

        for &idx in self.occupancy.keys() {
            match self.classify_defect(idx) {
                DefectType::Vacancy => stats.vacancies += 1,
                DefectType::Interstitial => stats.interstitials += 1,
                DefectType::Substitutional => stats.substitutionals += 1,
                DefectType::Dislocation => stats.dislocations += 1,
                DefectType::None => stats.perfect_sites += 1,
            }
        }

        stats
    }
}

#[derive(Default, Debug)]
struct DefectStatistics {
    vacancies: usize,
    interstitials: usize,
    substitutionals: usize,
    dislocations: usize,
    perfect_sites: usize,
}

impl DefectStatistics {
    fn total_defects(&self) -> usize {
        self.vacancies + self.interstitials + self.substitutionals + self.dislocations
    }

    fn defect_density(&self, total_sites: usize) -> f64 {
        self.total_defects() as f64 / total_sites as f64
    }
}
```

## 12.2 Computational Fluid Dynamics

CFD simulations solve partial differential equations on discretized domains. BCC grids offer:

- More isotropic stencils than cubic grids.
- Improved sampling efficiency for volumetric fields.

OctaIndex3D can:

- Index cells used in finite-difference or finite-volume schemes.
- Store scalar and vector fields (pressure, velocity, temperature).

Neighbor queries support:

- Stencil operations (e.g., 14-point stencils on BCC).
- Adaptive refinement in regions with sharp gradients.

To integrate OctaIndex3D into an existing CFD code, you might:

1. Replace or complement the existing grid structure with a BCC-indexed container.
2. Implement stencil operations using neighbor queries that traverse the 14 neighbors of each cell.
3. Use LODs to refine boundary layers or shock regions while keeping the bulk of the domain coarse.
4. Export fields to conventional meshes when needed for coupling with other solvers or visualization.

Early experiments in this style typically keep the core numerical scheme unchanged while letting BCC indexing handle adaptive refinement and data management. Over time, more deeply BCC-specific discretizations can be explored.

---

### 12.2.1 Stencils and Neighbor Patterns

On a BCC lattice, the **natural neighbor set** differs from a cubic grid:

- There are 14 nearest neighbors at nearly equal distances.
- Second- and third-ring neighbors form symmetric shells around each cell.

OctaIndex3D exposes these neighbor relationships via queries that:

- Enumerate neighbors in deterministic order.
- Provide approximate distances for use in discretizations.

Finite-difference or finite-volume schemes can then:

- Use 14-point stencils for diffusion-like operators.
- Use larger stencils (including second-ring neighbors) for higher-order accuracy.

Because the neighbor geometry is more isotropic, directional artifacts (e.g., grid-aligned diffusion) are reduced compared to cubic grids with 6- or 7-point stencils.

### 12.2.2 Adaptive Regions and Boundary Layers

Boundary layers and shocks demand higher resolution than the surrounding flow. With OctaIndex3D:

1. Coarse LOD cells cover the full domain.
2. Diagnostics identify regions where:
   - Gradients exceed thresholds.
   - Turbulence measures or vorticity is high.
3. Only those regions are refined into finer LODs.

The resulting structure:

- Keeps the majority of the domain cheap to update.
- Concentrates computation where it pays off most.

The same identifiers then support:

- Visualization of refinement regions.
- Post-processing of quantities integrated over refined cells.

#### Code Example: CFD Solver with Adaptive Refinement

```rust
use octaindex3d::{Index64, Frame, Container};
use std::collections::HashSet;

/// CFD field data at a cell
#[derive(Clone, Copy, Debug)]
struct FluidCell {
    pressure: f64,
    velocity: [f64; 3],
    temperature: f64,
    density: f64,
}

impl FluidCell {
    fn zero() -> Self {
        Self {
            pressure: 0.0,
            velocity: [0.0, 0.0, 0.0],
            temperature: 0.0,
            density: 0.0,
        }
    }
}

/// Adaptive CFD solver using BCC lattice
struct AdaptiveCFDSolver {
    frame: Frame,
    base_lod: u8,
    max_lod: u8,
    /// Cells at various LODs
    cells: HashMap<Index64, FluidCell>,
    /// Cells marked for refinement
    refined_cells: HashSet<Index64>,
    /// Gradient threshold for refinement
    gradient_threshold: f64,
}

impl AdaptiveCFDSolver {
    fn new(frame: Frame, base_lod: u8, max_lod: u8, gradient_threshold: f64) -> Self {
        Self {
            frame,
            base_lod,
            max_lod,
            cells: HashMap::new(),
            refined_cells: HashSet::new(),
            gradient_threshold,
        }
    }

    /// Initialize domain with base LOD
    fn initialize_domain(&mut self, bounds: BoundingBox) {
        // Create uniform grid at base LOD
        for x in bounds.x_range() {
            for y in bounds.y_range() {
                for z in bounds.z_range() {
                    let pos = [x, y, z];
                    if let Ok(idx) = self.frame.coords_to_index(&pos, self.base_lod) {
                        self.cells.insert(idx, FluidCell::zero());
                    }
                }
            }
        }
    }

    /// Compute pressure gradient at a cell using 14-neighbor stencil
    fn compute_pressure_gradient(&self, idx: Index64) -> Option<[f64; 3]> {
        let cell = self.cells.get(&idx)?;
        let mut grad = [0.0, 0.0, 0.0];
        let mut count = 0;

        for neighbor_idx in idx.neighbors_14() {
            if let Some(neighbor) = self.cells.get(&neighbor_idx) {
                // Get direction vector to neighbor
                let dir = self.approximate_direction(idx, neighbor_idx);
                let dp = neighbor.pressure - cell.pressure;

                grad[0] += dp * dir[0];
                grad[1] += dp * dir[1];
                grad[2] += dp * dir[2];
                count += 1;
            }
        }

        if count > 0 {
            let norm = 1.0 / (count as f64);
            grad[0] *= norm;
            grad[1] *= norm;
            grad[2] *= norm;
            Some(grad)
        } else {
            None
        }
    }

    /// Approximate normalized direction from one cell to neighbor
    fn approximate_direction(&self, from: Index64, to: Index64) -> [f64; 3] {
        // In practice, use actual coordinates
        // This is a simplified placeholder
        let from_coords = self.frame.index_to_coords(from).unwrap();
        let to_coords = self.frame.index_to_coords(to).unwrap();

        let dx = to_coords[0] - from_coords[0];
        let dy = to_coords[1] - from_coords[1];
        let dz = to_coords[2] - from_coords[2];

        let mag = (dx*dx + dy*dy + dz*dz).sqrt();
        if mag > 0.0 {
            [dx/mag, dy/mag, dz/mag]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Identify cells needing refinement based on gradient criterion
    fn mark_refinement_candidates(&mut self) {
        let mut candidates = HashSet::new();

        for (&idx, _) in &self.cells {
            if idx.lod() >= self.max_lod {
                continue; // Already at max refinement
            }

            if let Some(grad) = self.compute_pressure_gradient(idx) {
                let grad_mag = (grad[0]*grad[0] + grad[1]*grad[1] + grad[2]*grad[2]).sqrt();

                if grad_mag > self.gradient_threshold {
                    candidates.insert(idx);
                }
            }
        }

        self.refined_cells = candidates;
    }

    /// Refine marked cells to next LOD
    fn refine_cells(&mut self) {
        let mut new_cells = HashMap::new();

        for &parent_idx in &self.refined_cells {
            let parent_cell = match self.cells.get(&parent_idx) {
                Some(c) => *c,
                None => continue,
            };

            // Create child cells at next LOD
            let child_lod = parent_idx.lod() + 1;
            if child_lod > self.max_lod {
                continue;
            }

            // Get children indices (BCC refinement produces 8 children)
            for child_idx in parent_idx.children() {
                // Initialize child with interpolated values
                new_cells.insert(child_idx, parent_cell);
            }

            // Remove parent from active cells
            self.cells.remove(&parent_idx);
        }

        // Add new refined cells
        self.cells.extend(new_cells);
        self.refined_cells.clear();
    }

    /// Perform one time step of simulation
    fn time_step(&mut self, dt: f64) {
        // 1. Mark cells for refinement
        self.mark_refinement_candidates();

        // 2. Refine if needed
        if !self.refined_cells.is_empty() {
            self.refine_cells();
        }

        // 3. Update flow fields using BCC stencils
        self.update_flow_fields(dt);
    }

    /// Update flow fields using finite-volume method on BCC lattice
    fn update_flow_fields(&mut self, dt: f64) {
        let mut updates = HashMap::new();

        for (&idx, &cell) in &self.cells {
            // Compute fluxes through 14 neighbors
            let mut pressure_flux = 0.0;
            let mut velocity_flux = [0.0, 0.0, 0.0];

            for neighbor_idx in idx.neighbors_14() {
                if let Some(&neighbor) = self.cells.get(&neighbor_idx) {
                    // Simplified flux computation
                    pressure_flux += (neighbor.pressure - cell.pressure) * 0.1;

                    for i in 0..3 {
                        velocity_flux[i] += (neighbor.velocity[i] - cell.velocity[i]) * 0.1;
                    }
                }
            }

            // Update cell values
            let mut new_cell = cell;
            new_cell.pressure += pressure_flux * dt;
            for i in 0..3 {
                new_cell.velocity[i] += velocity_flux[i] * dt;
            }

            updates.insert(idx, new_cell);
        }

        // Apply updates
        self.cells.extend(updates);
    }

    /// Export results for visualization
    fn export_vtk(&self, filename: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(filename)?;

        writeln!(file, "# vtk DataFile Version 3.0")?;
        writeln!(file, "BCC CFD Results")?;
        writeln!(file, "ASCII")?;
        writeln!(file, "DATASET UNSTRUCTURED_GRID")?;
        writeln!(file, "POINTS {} float", self.cells.len())?;

        // Write point coordinates
        for &idx in self.cells.keys() {
            let coords = self.frame.index_to_coords(idx).unwrap();
            writeln!(file, "{} {} {}", coords[0], coords[1], coords[2])?;
        }

        // Write cell data
        writeln!(file, "\nPOINT_DATA {}", self.cells.len())?;
        writeln!(file, "SCALARS pressure float 1")?;
        writeln!(file, "LOOKUP_TABLE default")?;

        for cell in self.cells.values() {
            writeln!(file, "{}", cell.pressure)?;
        }

        Ok(())
    }
}

struct BoundingBox {
    min: [f64; 3],
    max: [f64; 3],
    step: f64,
}

impl BoundingBox {
    fn x_range(&self) -> impl Iterator<Item = f64> {
        let min = self.min[0];
        let max = self.max[0];
        let step = self.step;
        let n = ((max - min) / step) as usize;
        (0..n).map(move |i| min + i as f64 * step)
    }

    fn y_range(&self) -> impl Iterator<Item = f64> {
        let min = self.min[1];
        let max = self.max[1];
        let step = self.step;
        let n = ((max - min) / step) as usize;
        (0..n).map(move |i| min + i as f64 * step)
    }

    fn z_range(&self) -> impl Iterator<Item = f64> {
        let min = self.min[2];
        let max = self.max[2];
        let step = self.step;
        let n = ((max - min) / step) as usize;
        (0..n).map(move |i| min + i as f64 * step)
    }
}
```

## 12.3 Volumetric Data Analysis

Volumetric datasets arise from:

- Medical imaging (CT, MRI).
- 3D microscopy.
- Simulation outputs.

OctaIndex3D supports:

- Resampling data onto BCC grids for analysis.
- Aggregating statistics across scales.
- Efficient extraction of isosurfaces and subvolumes using range queries.

Because many analysis operations are local, BCC’s improved isotropy reduces directional artifacts and bias.

For example, in medical imaging:

- CT or MRI volumes can be resampled onto a BCC grid at multiple LODs.
- Range queries extract subvolumes around a lesion or region of interest.
- Aggregate statistics (mean intensity, texture measures) can be computed at coarse LODs, while fine LODs capture detailed structure.

Researchers can then compare measurements across scans and patients in a consistent index space, even when source data originates from different scanners or acquisition protocols.

---

### 12.3.1 Isosurfaces and Subvolume Extraction

Volumetric analysis often needs:

- Isosurfaces (e.g., bone density thresholds in CT).
- Subvolumes around regions of interest.

Using OctaIndex3D:

1. Scalars are stored in BCC containers keyed by `Index64` or `Hilbert64`.
2. A query selects all cells where values cross a threshold.
3. Isosurface extraction algorithms (e.g., marching cubes variants) operate on:
   - Local neighborhoods of BCC cells.
   - Interpolated values along cell boundaries.

Subvolume extraction becomes:

- A range query over identifiers corresponding to a spatial region.
- Optional regridding to cubic voxels if downstream tools require it.

### 12.3.2 Multi-Resolution Statistics

With multiple LODs, you can compute:

- Coarse aggregates (mean, variance) over large regions.
- Fine-detail statistics only where needed.

Typical workflows:

- Compute patient-level or experiment-level summaries at coarse LODs.
- Zoom into regions flagged as unusual and recompute metrics at fine LODs.

Because parent–child relationships are encoded in identifiers, rolling up or drilling down is:

- A matter of iterating over child indices.
- Applying user-defined aggregation functions.

## 12.4 Particle Simulations and Neighbor Search

Particle methods (e.g., smoothed particle hydrodynamics) require repeated neighbor searches:

- For each particle, find nearby particles within a kernel radius.

OctaIndex3D can:

- Bin particles into BCC cells.
- Use neighbor queries to identify candidate neighbors.
- Reduce the number of pairwise distance checks.

Compared to cubic binning, BCC binning:

- Achieves more uniform neighbor counts.
- Reduces worst-case density variations for isotropic distributions.

In practice, a particle simulation might:

1. Assign each particle to a BCC cell based on its position.
2. Maintain a container mapping cell indices to short lists of particle IDs.
3. For each particle, query neighboring cells and check only particles in those cells for interaction.

Because the volume of a BCC cell is lower for a given resolution, neighbor lists tend to be more uniform, which improves load balancing in parallel codes and reduces the variance of per-particle work.

---

### 12.4.1 Implementing a BCC Binning Structure

A minimal binning structure might:

- Maintain a map from cell indices to lists of particle IDs.
- Rebuild or update that map every few time steps.

Conceptually:

1. Clear all bins.
2. For each particle:
   - Convert its position to a BCC index using a frame and LOD.
   - Append its ID to the bin for that index.
3. For each particle, to find neighbors:
   - Enumerate its cell’s neighbors via OctaIndex3D.
   - For each neighboring cell, iterate over its particle list.

This structure is:

- Simple to implement.
- Easy to parallelize over particles or bins.
- Decoupled from the underlying physics integrator.

### 12.4.2 Particle-Based CFD and SPH

In smoothed particle hydrodynamics and related particle-based CFD methods:

- Each particle interacts with neighbors within a kernel radius.
- Kernel functions often assume isotropic neighbor distributions.

BCC binning supports these assumptions better than cubic grids:

- Neighbor counts per bin are more uniform.
- Angular bias in neighbor distribution is reduced.

This leads to:

- Smoother fields reconstructed from particle samples.
- More stable time-stepping when coupled with adaptive time-step controllers.

The same indexing design can also be reused for:

- Collision detection between particles and boundaries.
- Coupling particles with Eulerian fields stored in BCC containers.

---

## 12.5 High-Performance and Parallel Computing

Scientific computing workloads are typically:

- Parallelized across many cores or nodes.
- Limited by memory bandwidth and cache behavior.

OctaIndex3D helps by:

- Using compact, cache-friendly identifiers.
- Supporting partitioning by identifier ranges or spatial regions.
- Allowing containers to be sharded across processes or threads.

### 12.5.1 Domain Decomposition

When distributing work across processes:

- Each process owns one or more spatial subdomains.
- Subdomains map naturally to ranges of BCC cells.

With OctaIndex3D:

1. Choose a decomposition strategy (e.g., by geographic region, height bands, or index ranges).
2. Assign each subdomain’s identifier ranges to a process.
3. Use halo or ghost layers where subdomains touch.

Because identifiers are sortable, communication patterns can be expressed as:

- Exchanging contiguous ranges of indices and associated values.
- Using collective operations keyed by identifier ranges.

### 12.5.2 GPU and Accelerator Integration

Many scientific codes offload computation to GPUs or other accelerators. OctaIndex3D can support this by:

- Packing identifiers and values into flat arrays.
- Providing deterministic neighbor lists that can be precomputed or generated on-device.

On the device side:

- Kernels operate on arrays of cell data.
- Neighbor lists are represented as index offsets or compressed sparse row (CSR)-like structures.

This keeps:

- The high-level mapping between physics and indices in host code.
- The heavy numerical work in accelerator-friendly data structures.
---

## 12.6 Integration with Scientific Computing Libraries

OctaIndex3D is designed to integrate seamlessly with established scientific computing ecosystems, particularly Python-based workflows using NumPy, SciPy, and HPC libraries.

### 12.6.1 Python/NumPy Interoperability

Python bindings enable data exchange between OctaIndex3D and NumPy arrays:

```python
import numpy as np
import octaindex3d as oi3d

# Create a frame and container in Rust
frame = oi3d.Frame.cartesian()
container = oi3d.Container()

# Populate from NumPy arrays
positions = np.random.rand(10000, 3) * 100.0  # Random positions
values = np.random.rand(10000)  # Associated scalar values

# Efficient bulk insert
container.insert_from_arrays(
    frame=frame,
    positions=positions,
    values=values,
    lod=10
)

# Query and export to NumPy
query_region = np.array([[0, 0, 0], [50, 50, 50]])
results = container.query_region(frame, query_region, lod=10)

# Returns structured array compatible with NumPy
indices = results['indices']  # uint64 array
retrieved_values = results['values']  # float64 array
coords = results['coords']  # (N, 3) float64 array

# Perform analysis with SciPy
from scipy.spatial import cKDTree
from scipy.ndimage import gaussian_filter

# Build KD-tree for comparison
tree = cKDTree(coords)

# Apply Gaussian filter to gridded data
if results['is_regular']:
    grid_shape = results['grid_shape']
    gridded = retrieved_values.reshape(grid_shape)
    smoothed = gaussian_filter(gridded, sigma=2.0)
```

### 12.6.2 Integration with Parallel Computing Frameworks

#### MPI Integration for Distributed Simulations

```rust
use mpi::traits::*;
use octaindex3d::{Index64, Container};

fn distributed_md_simulation() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let rank = world.rank();
    let size = world.size();

    // Each rank owns a spatial partition
    let partition = compute_partition(rank, size);

    // Local container for this partition
    let mut local_atoms = Container::<Index64, Atom>::new();

    // Simulation loop
    for step in 0..num_steps {
        // 1. Local force computation
        compute_forces(&local_atoms);

        // 2. Identify boundary atoms that need ghost exchange
        let boundary_atoms = identify_boundary_atoms(&local_atoms, &partition);

        // 3. Exchange ghost atoms with neighbors
        exchange_ghost_atoms(&world, &boundary_atoms);

        // 4. Update positions
        integrate_equations_of_motion(&mut local_atoms, dt);

        // 5. Migrate atoms that crossed partition boundaries
        migrate_atoms(&world, &mut local_atoms, &partition);

        // 6. Synchronize for analysis/output
        if step % output_frequency == 0 {
            gather_statistics(&world, &local_atoms);
        }
    }
}

fn compute_partition(rank: i32, size: i32) -> SpatialPartition {
    // Simple slab decomposition
    let z_min = (rank as f64 / size as f64) * domain_size;
    let z_max = ((rank + 1) as f64 / size as f64) * domain_size;

    SpatialPartition {
        bounds: BoundingBox {
            min: [0.0, 0.0, z_min],
            max: [domain_size, domain_size, z_max],
        },
        ghost_width: cutoff_radius * 1.5,
    }
}

fn exchange_ghost_atoms(world: &impl Communicator, boundary_atoms: &[Atom]) {
    let rank = world.rank();
    let size = world.size();

    // Send to upper neighbor
    if rank < size - 1 {
        world.process_at_rank(rank + 1)
            .send(&boundary_atoms[..]);
    }

    // Receive from lower neighbor
    if rank > 0 {
        let (ghost_atoms, _) = world.process_at_rank(rank - 1)
            .receive_vec::<Atom>();
        // Add ghost atoms to local container
    }

    // Repeat for lower/upper directions
}

struct SpatialPartition {
    bounds: BoundingBox,
    ghost_width: f64,
}
```

#### Rayon for Shared-Memory Parallelism

```rust
use rayon::prelude::*;
use octaindex3d::{Index64, Container};

/// Parallel particle force computation
fn compute_forces_parallel(
    atoms: &Container<Index64, Atom>,
    forces: &mut Container<Index64, [f64; 3]>,
) {
    // Convert container to parallel iterator
    let atom_pairs: Vec<_> = atoms.iter().collect();

    // Parallel force computation
    let force_updates: Vec<_> = atom_pairs
        .par_iter()
        .map(|(&idx, &atom)| {
            let mut force = [0.0, 0.0, 0.0];

            // Find neighbors efficiently using BCC indexing
            for neighbor_idx in idx.neighbors_14() {
                if let Some(&neighbor_atom) = atoms.get(&neighbor_idx) {
                    if neighbor_atom.id != atom.id {
                        let f = compute_pairwise_force(&atom, &neighbor_atom);
                        force[0] += f[0];
                        force[1] += f[1];
                        force[2] += f[2];
                    }
                }
            }

            (idx, force)
        })
        .collect();

    // Update forces container
    for (idx, force) in force_updates {
        forces.insert(idx, force);
    }
}

fn compute_pairwise_force(atom1: &Atom, atom2: &Atom) -> [f64; 3] {
    let r = distance(&atom1.position, &atom2.position);
    let r_vec = [
        atom2.position[0] - atom1.position[0],
        atom2.position[1] - atom1.position[1],
        atom2.position[2] - atom1.position[2],
    ];

    // Lennard-Jones force
    let sigma = 3.4;
    let epsilon = 1.0;
    let r6 = (sigma / r).powi(6);
    let r12 = r6 * r6;

    let f_mag = 24.0 * epsilon * (2.0 * r12 - r6) / r;

    [
        f_mag * r_vec[0] / r,
        f_mag * r_vec[1] / r,
        f_mag * r_vec[2] / r,
    ]
}
```

### 12.6.3 GPU Acceleration with CUDA/ROCm

For GPU-accelerated scientific computing, OctaIndex3D containers can be transferred to device memory:

```rust
use cuda_runtime_api::*;

/// GPU-accelerated neighbor search kernel
#[cfg(feature = "cuda")]
mod gpu {
    use super::*;

    pub struct GPUAcceleratedContainer {
        // Device pointers
        d_indices: *mut u64,
        d_positions: *mut f32,
        d_values: *mut f32,
        count: usize,
    }

    impl GPUAcceleratedContainer {
        /// Transfer container data to GPU
        pub fn from_container(container: &Container<Index64, f64>) -> CudaResult<Self> {
            let count = container.len();

            // Allocate device memory
            let mut d_indices = std::ptr::null_mut();
            let mut d_positions = std::ptr::null_mut();
            let mut d_values = std::ptr::null_mut();

            unsafe {
                cuda_malloc(&mut d_indices, count * std::mem::size_of::<u64>())?;
                cuda_malloc(&mut d_positions, count * 3 * std::mem::size_of::<f32>())?;
                cuda_malloc(&mut d_values, count * std::mem::size_of::<f32>())?;

                // Copy data from host to device
                // (implementation details omitted for brevity)
            }

            Ok(Self {
                d_indices,
                d_positions,
                d_values,
                count,
            })
        }

        /// Launch neighbor search kernel
        pub fn parallel_neighbor_search(&self, cutoff: f32) -> CudaResult<NeighborList> {
            // Launch CUDA kernel for parallel neighbor finding
            // Each thread processes one atom
            let threads_per_block = 256;
            let num_blocks = (self.count + threads_per_block - 1) / threads_per_block;

            unsafe {
                // Kernel launch (pseudo-code)
                // neighbor_search_kernel<<<num_blocks, threads_per_block>>>(
                //     self.d_indices,
                //     self.d_positions,
                //     self.d_values,
                //     self.count,
                //     cutoff
                // );
            }

            // Copy results back and return
            todo!()
        }
    }
}
```

## 12.7 Performance Optimization Techniques

### 12.7.1 Cache-Friendly Data Layouts

BCC indices naturally support cache-efficient access patterns:

```rust
/// Struct-of-Arrays (SoA) layout for better cache performance
struct ParticleSystemSoA {
    // Parallel arrays indexed by particle ID
    indices: Vec<Index64>,
    positions_x: Vec<f64>,
    positions_y: Vec<f64>,
    positions_z: Vec<f64>,
    velocities_x: Vec<f64>,
    velocities_y: Vec<f64>,
    velocities_z: Vec<f64>,
    masses: Vec<f64>,
}

impl ParticleSystemSoA {
    /// Compute forces with better cache locality
    fn compute_forces_soa(&self) -> Vec<[f64; 3]> {
        let mut forces = vec![[0.0; 3]; self.indices.len()];

        // Sequential access patterns benefit from cache prefetching
        for i in 0..self.indices.len() {
            let idx = self.indices[i];
            let pos_i = [self.positions_x[i], self.positions_y[i], self.positions_z[i]];

            for neighbor_idx in idx.neighbors_14() {
                // Find neighbor in our arrays (using spatial hash or sort)
                if let Some(j) = self.find_particle_index(neighbor_idx) {
                    let pos_j = [
                        self.positions_x[j],
                        self.positions_y[j],
                        self.positions_z[j]
                    ];

                    let force = compute_pairwise_force_simple(&pos_i, &pos_j, self.masses[i], self.masses[j]);
                    forces[i][0] += force[0];
                    forces[i][1] += force[1];
                    forces[i][2] += force[2];
                }
            }
        }

        forces
    }

    fn find_particle_index(&self, idx: Index64) -> Option<usize> {
        // Binary search or hash lookup
        self.indices.binary_search(&idx).ok()
    }
}

fn compute_pairwise_force_simple(pos1: &[f64; 3], pos2: &[f64; 3], m1: f64, m2: f64) -> [f64; 3] {
    // Simplified gravitational force
    let dx = pos2[0] - pos1[0];
    let dy = pos2[1] - pos1[1];
    let dz = pos2[2] - pos1[2];
    let r2 = dx*dx + dy*dy + dz*dz;
    let r = r2.sqrt();

    if r < 1e-10 {
        return [0.0; 0.0; 0.0];
    }

    let f_mag = m1 * m2 / r2;
    [f_mag * dx / r, f_mag * dy / r, f_mag * dz / r]
}
```

### 12.7.2 SIMD Vectorization

Modern CPUs support SIMD instructions that can be leveraged for BCC computations:

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Vectorized distance computations for multiple neighbors
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn compute_distances_simd(
    center: &[f32; 3],
    neighbors: &[[f32; 3]],
) -> Vec<f32> {
    let mut distances = Vec::with_capacity(neighbors.len());

    // Load center position into SIMD registers
    let cx = _mm256_set1_ps(center[0]);
    let cy = _mm256_set1_ps(center[1]);
    let cz = _mm256_set1_ps(center[2]);

    // Process 8 neighbors at a time with AVX2
    for chunk in neighbors.chunks(8) {
        // Load neighbor positions (8 at a time)
        let mut nx = [0.0f32; 8];
        let mut ny = [0.0f32; 8];
        let mut nz = [0.0f32; 8];

        for (i, pos) in chunk.iter().enumerate() {
            nx[i] = pos[0];
            ny[i] = pos[1];
            nz[i] = pos[2];
        }

        let nx_vec = _mm256_loadu_ps(nx.as_ptr());
        let ny_vec = _mm256_loadu_ps(ny.as_ptr());
        let nz_vec = _mm256_loadu_ps(nz.as_ptr());

        // Compute differences
        let dx = _mm256_sub_ps(nx_vec, cx);
        let dy = _mm256_sub_ps(ny_vec, cy);
        let dz = _mm256_sub_ps(nz_vec, cz);

        // Compute squared distances
        let dx2 = _mm256_mul_ps(dx, dx);
        let dy2 = _mm256_mul_ps(dy, dy);
        let dz2 = _mm256_mul_ps(dz, dz);

        let r2 = _mm256_add_ps(_mm256_add_ps(dx2, dy2), dz2);

        // Square root for distance
        let r = _mm256_sqrt_ps(r2);

        // Store results
        let mut result = [0.0f32; 8];
        _mm256_storeu_ps(result.as_mut_ptr(), r);

        distances.extend_from_slice(&result[..chunk.len()]);
    }

    distances
}
```

### 12.7.3 Memory Pool Allocation

For high-performance simulations, custom allocators reduce allocation overhead:

```rust
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

/// Memory pool for particle data
struct ParticlePool {
    memory: NonNull<u8>,
    layout: Layout,
    capacity: usize,
    used: usize,
}

impl ParticlePool {
    fn new(capacity: usize) -> Self {
        let size = capacity * std::mem::size_of::<Atom>();
        let align = std::mem::align_of::<Atom>();
        let layout = Layout::from_size_align(size, align).unwrap();

        let memory = unsafe {
            let ptr = alloc(layout);
            NonNull::new(ptr).expect("Allocation failed")
        };

        Self {
            memory,
            layout,
            capacity,
            used: 0,
        }
    }

    fn allocate_atom(&mut self) -> Option<&mut Atom> {
        if self.used >= self.capacity {
            return None;
        }

        unsafe {
            let offset = self.used * std::mem::size_of::<Atom>();
            let ptr = self.memory.as_ptr().add(offset) as *mut Atom;
            self.used += 1;
            Some(&mut *ptr)
        }
    }

    fn reset(&mut self) {
        self.used = 0;
    }
}

impl Drop for ParticlePool {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.memory.as_ptr(), self.layout);
        }
    }
}
```

## 12.8 Troubleshooting Common Issues

### 12.8.1 Numerical Instabilities

**Problem**: Force calculations diverge or produce NaN values.

**Solutions**:
- Add softening parameters to prevent division by zero:
  ```rust
  let r_safe = (r2 + epsilon*epsilon).sqrt();  // epsilon = 1e-6
  ```
- Clamp forces to maximum values:
  ```rust
  let force_mag = force_mag.min(max_force);
  ```
- Use double precision for accumulation:
  ```rust
  let force_sum: f64 = forces.iter().map(|f| *f as f64).sum();
  ```

### 12.8.2 Performance Bottlenecks

**Problem**: Neighbor searches dominate runtime.

**Solutions**:
- Use appropriate LOD for cell size vs. cutoff radius
- Implement Verlet neighbor lists that update less frequently
- Consider cell list reuse across multiple timesteps
- Profile to identify hot spots:
  ```rust
  use std::time::Instant;

  let start = Instant::now();
  compute_forces(&atoms);
  println!("Force computation: {:?}", start.elapsed());
  ```

### 12.8.3 Memory Usage

**Problem**: Containers consume excessive memory.

**Solutions**:
- Use sparse containers for low-density systems
- Implement compression for inactive regions
- Stream data from disk for very large systems
- Use LOD hierarchy to store only active fine cells

## 12.9 Further Reading

### Books and Monographs

1. **"Computer Simulation of Liquids"** by Allen & Tildesley (2017)
   - Classic reference for molecular dynamics algorithms
   - Chapter 5 covers neighbor search algorithms

2. **"Numerical Simulation in Molecular Dynamics"** by Griebel, Knapek & Zumbusch (2007)
   - Modern perspective on MD algorithms
   - Discusses spatial decomposition strategies

3. **"Computational Fluid Dynamics: Principles and Applications"** by Blazek (2015)
   - Comprehensive CFD textbook
   - Chapter 3 discusses spatial discretization schemes

### Research Papers

1. Petersen & Middleton (1955). "Sampling and Reconstruction of Wave-Number-Limited Functions in N-Dimensional Euclidean Spaces"
   - Original work on BCC sampling efficiency

2. Van De Geijn et al. (2001). "Fast Parallel Algorithms for Neighbor Finding on the BCC Lattice"
   - Algorithms for efficient BCC neighbor enumeration

3. Yoon & Hovland (2004). "Space-Filling Curves for Improved Cache Performance in Scientific Computing"
   - Discussion of cache-friendly spatial indexing

### Online Resources

- **OctaIndex3D Documentation**: https://docs.octaindex3d.org/scientific-computing
- **BCC Lattice Theory**: https://en.wikipedia.org/wiki/Cubic_crystal_system#Body-centered_cubic
- **LAMMPS MD Package**: https://www.lammps.org (for comparison with traditional MD codes)

---

## 12.10 Summary

In this chapter, we examined scientific computing applications:

- **Molecular dynamics and crystallography** benefit from grid/structure alignment, with detailed code examples for neighbor search and defect detection.
- **CFD** uses BCC grids for more isotropic stencils and sampling, with adaptive refinement strategies.
- **Volumetric data analysis** leverages BCC indexing for efficient queries.
- **Particle simulations** exploit BCC-based binning for neighbor search.
- **Integration with scientific libraries** enables workflows with NumPy, MPI, Rayon, and GPU acceleration.
- **Performance optimization** through cache-friendly layouts, SIMD vectorization, and memory pooling.
- **Troubleshooting guide** addresses common numerical and performance issues.

The next chapter turns to gaming and virtual worlds, where similar indexing techniques support interactive, latency-sensitive applications.
