//! Mesh extraction from TSDF on BCC lattice
//!
//! Implements surface extraction using zero-crossing detection and
//! vertex interpolation optimized for BCC lattice structure.
//!
//! ## Algorithm
//!
//! 1. Find all zero-crossing edges (sign change between neighbors)
//! 2. Interpolate vertex positions along edges using linear interpolation
//! 3. Build triangles from connected vertices
//! 4. Compute normals from TSDF gradient
//!
//! ## BCC Lattice Advantages
//!
//! - 14 neighbors per voxel → more accurate surface representation
//! - Better isotropy → fewer triangle orientation artifacts
//! - Natural truncated octahedral cells

use super::TSDFLayer;
use crate::error::Result;
use crate::Index64;

/// 3D vertex with position and optional normal
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    /// Position in 3D space
    pub position: [f32; 3],
    /// Surface normal (unit vector)
    pub normal: Option<[f32; 3]>,
}

impl Vertex {
    /// Create vertex with position only
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            normal: None,
        }
    }

    /// Create vertex with position and normal
    pub fn with_normal(x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32) -> Self {
        Self {
            position: [x, y, z],
            normal: Some([nx, ny, nz]),
        }
    }
}

/// Triangle face (3 vertex indices)
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    /// Indices into vertex array
    pub indices: [usize; 3],
}

impl Triangle {
    /// Create new triangle
    pub fn new(i0: usize, i1: usize, i2: usize) -> Self {
        Self {
            indices: [i0, i1, i2],
        }
    }
}

/// 3D mesh representation
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Vertex array
    pub vertices: Vec<Vertex>,
    /// Triangle array
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    /// Create empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }

    /// Add vertex, returns index
    pub fn add_vertex(&mut self, vertex: Vertex) -> usize {
        let idx = self.vertices.len();
        self.vertices.push(vertex);
        idx
    }

    /// Add triangle
    pub fn add_triangle(&mut self, triangle: Triangle) {
        self.triangles.push(triangle);
    }

    /// Get mesh statistics
    pub fn stats(&self) -> MeshStats {
        MeshStats {
            vertex_count: self.vertices.len(),
            triangle_count: self.triangles.len(),
            has_normals: self.vertices.iter().any(|v| v.normal.is_some()),
        }
    }

    /// Compute approximate surface area
    pub fn surface_area(&self) -> f32 {
        let mut area = 0.0;

        for tri in &self.triangles {
            let v0 = self.vertices[tri.indices[0]].position;
            let v1 = self.vertices[tri.indices[1]].position;
            let v2 = self.vertices[tri.indices[2]].position;

            // Triangle area = 0.5 * |cross(v1-v0, v2-v0)|
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            let mag = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            area += 0.5 * mag;
        }

        area
    }

    /// Compute bounding box
    pub fn bounding_box(&self) -> Option<([f32; 3], [f32; 3])> {
        if self.vertices.is_empty() {
            return None;
        }

        let mut min = self.vertices[0].position;
        let mut max = self.vertices[0].position;

        for vertex in &self.vertices {
            let p = vertex.position;
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }

        Some((min, max))
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Mesh statistics
#[derive(Debug, Clone)]
pub struct MeshStats {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub has_normals: bool,
}

/// Extract mesh from TSDF using zero-crossing interpolation
///
/// This is a simplified BCC-optimized extraction that:
/// 1. Finds zero-crossing edges from TSDF
/// 2. Interpolates vertex positions
/// 3. Creates triangles from edge connectivity
///
/// # Arguments
/// * `tsdf` - Source TSDF layer
///
/// # Returns
/// Mesh with vertices and triangles
pub fn extract_mesh_from_tsdf(tsdf: &TSDFLayer) -> Result<Mesh> {
    let mut mesh = Mesh::new();
    let voxel_size = tsdf.voxel_size();

    // Get zero-crossing edges
    let edges = tsdf.get_zero_crossing_edges();

    if edges.is_empty() {
        return Ok(mesh); // No surface found
    }

    // For each edge, create an interpolated vertex
    // Store mapping from edge → vertex index
    use std::collections::HashMap;
    let mut edge_to_vertex: HashMap<(Index64, Index64), usize> = HashMap::new();

    for &(idx1, idx2) in &edges {
        // Get distances at endpoints
        let d1 = tsdf.get_distance(idx1).unwrap_or(0.0);
        let d2 = tsdf.get_distance(idx2).unwrap_or(0.0);

        // Linear interpolation parameter: t where d1 + t*(d2-d1) = 0
        let t = if (d2 - d1).abs() > 1e-6 {
            -d1 / (d2 - d1)
        } else {
            0.5 // Fallback to midpoint
        }
        .clamp(0.0, 1.0);

        // Get positions
        let (x1, y1, z1) = idx1.decode_coords();
        let (x2, y2, z2) = idx2.decode_coords();

        let p1 = [
            x1 as f32 * voxel_size,
            y1 as f32 * voxel_size,
            z1 as f32 * voxel_size,
        ];
        let p2 = [
            x2 as f32 * voxel_size,
            y2 as f32 * voxel_size,
            z2 as f32 * voxel_size,
        ];

        // Interpolated position
        let pos = [
            p1[0] + t * (p2[0] - p1[0]),
            p1[1] + t * (p2[1] - p1[1]),
            p1[2] + t * (p2[2] - p1[2]),
        ];

        // Compute normal from TSDF gradient at interpolated position
        // For simplicity, use finite differences at idx1
        let normal = compute_normal(tsdf, idx1, voxel_size);

        let vertex = if let Some(n) = normal {
            Vertex::with_normal(pos[0], pos[1], pos[2], n[0], n[1], n[2])
        } else {
            Vertex::new(pos[0], pos[1], pos[2])
        };

        let v_idx = mesh.add_vertex(vertex);

        // Store both directions (edge is undirected)
        edge_to_vertex.insert((idx1, idx2), v_idx);
        edge_to_vertex.insert((idx2, idx1), v_idx);
    }

    // Build triangles using naive fan triangulation
    // Group vertices by proximity and create triangles
    // This is a simplified approach - production code would use proper mesh topology
    build_triangles_naive(&mut mesh, &edges, &edge_to_vertex);

    Ok(mesh)
}

/// Compute normal at voxel using finite differences
fn compute_normal(tsdf: &TSDFLayer, idx: Index64, voxel_size: f32) -> Option<[f32; 3]> {
    use crate::neighbors::neighbors_index64;

    let d = tsdf.get_distance(idx)?;
    let neighbors = neighbors_index64(idx);

    let mut grad = [0.0, 0.0, 0.0];
    let mut count = 0;

    for neighbor_idx in neighbors {
        if let Some(dn) = tsdf.get_distance(neighbor_idx) {
            let (x1, y1, z1) = idx.decode_coords();
            let (x2, y2, z2) = neighbor_idx.decode_coords();

            let dx = (x2 as f32 - x1 as f32) * voxel_size;
            let dy = (y2 as f32 - y1 as f32) * voxel_size;
            let dz = (z2 as f32 - z1 as f32) * voxel_size;

            let dd = dn - d;

            grad[0] += dd * dx;
            grad[1] += dd * dy;
            grad[2] += dd * dz;
            count += 1;
        }
    }

    if count > 0 {
        grad[0] /= count as f32;
        grad[1] /= count as f32;
        grad[2] /= count as f32;

        // Normalize
        let mag = (grad[0] * grad[0] + grad[1] * grad[1] + grad[2] * grad[2]).sqrt();
        if mag > 1e-6 {
            Some([grad[0] / mag, grad[1] / mag, grad[2] / mag])
        } else {
            None
        }
    } else {
        None
    }
}

/// Build triangles using naive approach
/// For BCC lattice, we need to handle the 14-neighbor connectivity properly
/// This is a simplified version - production would use proper Delaunay or advancing front
fn build_triangles_naive(
    mesh: &mut Mesh,
    edges: &[(Index64, Index64)],
    edge_to_vertex: &std::collections::HashMap<(Index64, Index64), usize>,
) {
    use std::collections::HashSet;

    // Group edges by their voxels
    let mut voxel_edges: std::collections::HashMap<Index64, Vec<usize>> =
        std::collections::HashMap::new();

    for (i, &(idx1, idx2)) in edges.iter().enumerate() {
        voxel_edges.entry(idx1).or_default().push(i);
        voxel_edges.entry(idx2).or_default().push(i);
    }

    // For each voxel with 3+ edges, try to form triangles
    let mut used_triangles: HashSet<[usize; 3]> = HashSet::new();

    for edge_indices in voxel_edges.values() {
        if edge_indices.len() < 3 {
            continue;
        }

        // Try all combinations of 3 edges
        for i in 0..edge_indices.len() {
            for j in (i + 1)..edge_indices.len() {
                for k in (j + 1)..edge_indices.len() {
                    let e1 = &edges[edge_indices[i]];
                    let e2 = &edges[edge_indices[j]];
                    let e3 = &edges[edge_indices[k]];

                    // Check if these edges form a triangle
                    if let Some(tri_indices) = try_form_triangle(e1, e2, e3, edge_to_vertex) {
                        let mut sorted = tri_indices;
                        sorted.sort_unstable();

                        if used_triangles.insert(sorted) {
                            mesh.add_triangle(Triangle::new(
                                tri_indices[0],
                                tri_indices[1],
                                tri_indices[2],
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Try to form a triangle from 3 edges
fn try_form_triangle(
    e1: &(Index64, Index64),
    e2: &(Index64, Index64),
    e3: &(Index64, Index64),
    edge_to_vertex: &std::collections::HashMap<(Index64, Index64), usize>,
) -> Option<[usize; 3]> {
    let v1 = edge_to_vertex.get(e1)?;
    let v2 = edge_to_vertex.get(e2)?;
    let v3 = edge_to_vertex.get(e3)?;

    // Check if all three are different
    if v1 != v2 && v2 != v3 && v1 != v3 {
        Some([*v1, *v2, *v3])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::{Layer, Measurement};

    #[test]
    fn test_mesh_creation() {
        let mesh = Mesh::new();
        assert_eq!(mesh.vertices.len(), 0);
        assert_eq!(mesh.triangles.len(), 0);
    }

    #[test]
    fn test_add_vertex() {
        let mut mesh = Mesh::new();
        let v = Vertex::new(1.0, 2.0, 3.0);
        let idx = mesh.add_vertex(v);
        assert_eq!(idx, 0);
        assert_eq!(mesh.vertices.len(), 1);
    }

    #[test]
    fn test_mesh_extraction() -> Result<()> {
        // Create simple TSDF with surface
        let mut tsdf = TSDFLayer::new(0.1);

        for i in 0..5 {
            let idx = Index64::new(0, 0, 5, 100 + i, 100, 100)?;
            tsdf.update(idx, &Measurement::depth(0.01, 1.0))?;
        }

        // Extract mesh
        let mesh = extract_mesh_from_tsdf(&tsdf)?;

        // Should have some vertices (from zero crossings)
        let stats = mesh.stats();
        assert!(stats.vertex_count > 0); // May have no triangles if not enough connectivity

        Ok(())
    }

    #[test]
    fn test_surface_area() {
        let mut mesh = Mesh::new();

        // Create a simple triangle (area = 0.5)
        let v0 = mesh.add_vertex(Vertex::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Vertex::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Vertex::new(0.0, 1.0, 0.0));
        mesh.add_triangle(Triangle::new(v0, v1, v2));

        let area = mesh.surface_area();
        assert!((area - 0.5).abs() < 1e-5);
    }
}
