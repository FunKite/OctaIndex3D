//! Mesh export formats (PLY, OBJ)
//!
//! Implements standard mesh file format writers with no proprietary dependencies.
//! All formats are documented open standards.

use super::mesh::Mesh;
use crate::error::Result;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Export mesh to PLY format (Stanford Polygon File Format)
///
/// PLY is a simple, open format for storing 3D mesh data.
/// Specification: <http://paulbourke.net/dataformats/ply/>
///
/// # Arguments
/// * `mesh` - Mesh to export
/// * `path` - Output file path
/// * `binary` - If true, write binary PLY; if false, write ASCII PLY
///
/// # Example
/// ```no_run
/// use octaindex3d::layers::{Mesh, export_mesh_ply};
/// # use octaindex3d::Result;
///
/// # fn example() -> Result<()> {
/// let mesh = Mesh::new();
/// export_mesh_ply(&mesh, "output.ply", false)?; // ASCII format
/// # Ok(())
/// # }
/// ```
pub fn export_mesh_ply(mesh: &Mesh, path: impl AsRef<Path>, binary: bool) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // Write PLY header
    writeln!(writer, "ply")?;

    if binary {
        // Binary format (little endian)
        writeln!(writer, "format binary_little_endian 1.0")?;
    } else {
        // ASCII format
        writeln!(writer, "format ascii 1.0")?;
    }

    writeln!(writer, "comment Exported from OctaIndex3D")?;
    writeln!(writer, "element vertex {}", mesh.vertices.len())?;
    writeln!(writer, "property float x")?;
    writeln!(writer, "property float y")?;
    writeln!(writer, "property float z")?;

    // Add normal properties if mesh has normals
    let has_normals = mesh.vertices.iter().any(|v| v.normal.is_some());
    if has_normals {
        writeln!(writer, "property float nx")?;
        writeln!(writer, "property float ny")?;
        writeln!(writer, "property float nz")?;
    }

    writeln!(writer, "element face {}", mesh.triangles.len())?;
    writeln!(writer, "property list uchar int vertex_indices")?;
    writeln!(writer, "end_header")?;

    if binary {
        // Write binary data
        write_ply_binary(&mut writer, mesh, has_normals)?;
    } else {
        // Write ASCII data
        write_ply_ascii(&mut writer, mesh, has_normals)?;
    }

    Ok(())
}

/// Write PLY vertices and faces in ASCII format
fn write_ply_ascii(writer: &mut BufWriter<File>, mesh: &Mesh, has_normals: bool) -> Result<()> {
    // Write vertices
    for vertex in &mesh.vertices {
        let pos = vertex.position;

        if has_normals {
            if let Some(normal) = vertex.normal {
                writeln!(
                    writer,
                    "{} {} {} {} {} {}",
                    pos[0], pos[1], pos[2], normal[0], normal[1], normal[2]
                )?;
            } else {
                // No normal for this vertex, use default (0, 0, 1)
                writeln!(writer, "{} {} {} 0.0 0.0 1.0", pos[0], pos[1], pos[2])?;
            }
        } else {
            writeln!(writer, "{} {} {}", pos[0], pos[1], pos[2])?;
        }
    }

    // Write faces
    for triangle in &mesh.triangles {
        let idx = triangle.indices;
        writeln!(writer, "3 {} {} {}", idx[0], idx[1], idx[2])?;
    }

    Ok(())
}

/// Write PLY vertices and faces in binary format
fn write_ply_binary(writer: &mut BufWriter<File>, mesh: &Mesh, has_normals: bool) -> Result<()> {
    // Write vertices
    for vertex in &mesh.vertices {
        let pos = vertex.position;

        // Write position (3 floats, little endian)
        writer.write_all(&pos[0].to_le_bytes())?;
        writer.write_all(&pos[1].to_le_bytes())?;
        writer.write_all(&pos[2].to_le_bytes())?;

        if has_normals {
            if let Some(normal) = vertex.normal {
                writer.write_all(&normal[0].to_le_bytes())?;
                writer.write_all(&normal[1].to_le_bytes())?;
                writer.write_all(&normal[2].to_le_bytes())?;
            } else {
                // Default normal (0, 0, 1)
                writer.write_all(&0.0f32.to_le_bytes())?;
                writer.write_all(&0.0f32.to_le_bytes())?;
                writer.write_all(&1.0f32.to_le_bytes())?;
            }
        }
    }

    // Write faces
    for triangle in &mesh.triangles {
        let idx = triangle.indices;

        // Count (1 byte)
        writer.write_all(&[3u8])?;

        // Indices (3 * 4 bytes, little endian)
        writer.write_all(&(idx[0] as i32).to_le_bytes())?;
        writer.write_all(&(idx[1] as i32).to_le_bytes())?;
        writer.write_all(&(idx[2] as i32).to_le_bytes())?;
    }

    Ok(())
}

/// Export mesh to OBJ format (Wavefront OBJ)
///
/// OBJ is a widely-supported open format for 3D geometry.
/// Specification: <http://www.martinreddy.net/gfx/3d/OBJ.spec>
///
/// # Arguments
/// * `mesh` - Mesh to export
/// * `path` - Output file path
///
/// # Example
/// ```no_run
/// use octaindex3d::layers::{Mesh, export_mesh_obj};
/// # use octaindex3d::Result;
///
/// # fn example() -> Result<()> {
/// let mesh = Mesh::new();
/// export_mesh_obj(&mesh, "output.obj")?;
/// # Ok(())
/// # }
/// ```
pub fn export_mesh_obj(mesh: &Mesh, path: impl AsRef<Path>) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // Write OBJ header
    writeln!(writer, "# Exported from OctaIndex3D")?;
    writeln!(writer, "# Vertices: {}", mesh.vertices.len())?;
    writeln!(writer, "# Faces: {}", mesh.triangles.len())?;
    writeln!(writer)?;

    // Write vertices
    for vertex in &mesh.vertices {
        let pos = vertex.position;
        writeln!(writer, "v {} {} {}", pos[0], pos[1], pos[2])?;
    }

    writeln!(writer)?;

    // Write normals if available
    let has_normals = mesh.vertices.iter().any(|v| v.normal.is_some());
    if has_normals {
        for vertex in &mesh.vertices {
            if let Some(normal) = vertex.normal {
                writeln!(writer, "vn {} {} {}", normal[0], normal[1], normal[2])?;
            } else {
                writeln!(writer, "vn 0.0 0.0 1.0")?; // Default normal
            }
        }
        writeln!(writer)?;
    }

    // Write faces
    // OBJ indices are 1-based
    for triangle in &mesh.triangles {
        let idx = triangle.indices;

        if has_normals {
            // Format: f v1//vn1 v2//vn2 v3//vn3
            writeln!(
                writer,
                "f {}//{} {}//{} {}//{}",
                idx[0] + 1,
                idx[0] + 1,
                idx[1] + 1,
                idx[1] + 1,
                idx[2] + 1,
                idx[2] + 1
            )?;
        } else {
            // Format: f v1 v2 v3
            writeln!(writer, "f {} {} {}", idx[0] + 1, idx[1] + 1, idx[2] + 1)?;
        }
    }

    Ok(())
}

/// Export mesh to STL format (Stereolithography)
///
/// STL is a simple format used for 3D printing.
/// Specification: <https://en.wikipedia.org/wiki/STL_(file_format)>
///
/// # Arguments
/// * `mesh` - Mesh to export
/// * `path` - Output file path
/// * `binary` - If true, write binary STL; if false, write ASCII STL
pub fn export_mesh_stl(mesh: &Mesh, path: impl AsRef<Path>, binary: bool) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    if binary {
        write_stl_binary(&mut writer, mesh)?;
    } else {
        write_stl_ascii(&mut writer, mesh)?;
    }

    Ok(())
}

/// Write STL in ASCII format
fn write_stl_ascii(writer: &mut BufWriter<File>, mesh: &Mesh) -> Result<()> {
    writeln!(writer, "solid OctaIndex3D_Mesh")?;

    for triangle in &mesh.triangles {
        let v0 = mesh.vertices[triangle.indices[0]].position;
        let v1 = mesh.vertices[triangle.indices[1]].position;
        let v2 = mesh.vertices[triangle.indices[2]].position;

        // Compute face normal
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let normal = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];

        let mag = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let n = if mag > 1e-6 {
            [normal[0] / mag, normal[1] / mag, normal[2] / mag]
        } else {
            [0.0, 0.0, 1.0]
        };

        writeln!(writer, "  facet normal {} {} {}", n[0], n[1], n[2])?;
        writeln!(writer, "    outer loop")?;
        writeln!(writer, "      vertex {} {} {}", v0[0], v0[1], v0[2])?;
        writeln!(writer, "      vertex {} {} {}", v1[0], v1[1], v1[2])?;
        writeln!(writer, "      vertex {} {} {}", v2[0], v2[1], v2[2])?;
        writeln!(writer, "    endloop")?;
        writeln!(writer, "  endfacet")?;
    }

    writeln!(writer, "endsolid OctaIndex3D_Mesh")?;

    Ok(())
}

/// Write STL in binary format
fn write_stl_binary(writer: &mut BufWriter<File>, mesh: &Mesh) -> Result<()> {
    // Header (80 bytes)
    let header =
        b"OctaIndex3D Binary STL                                                          ";
    writer.write_all(&header[..80])?;

    // Number of triangles (4 bytes, little endian)
    writer.write_all(&(mesh.triangles.len() as u32).to_le_bytes())?;

    // Write triangles
    for triangle in &mesh.triangles {
        let v0 = mesh.vertices[triangle.indices[0]].position;
        let v1 = mesh.vertices[triangle.indices[1]].position;
        let v2 = mesh.vertices[triangle.indices[2]].position;

        // Compute face normal
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let normal = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];

        let mag = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let n = if mag > 1e-6 {
            [normal[0] / mag, normal[1] / mag, normal[2] / mag]
        } else {
            [0.0, 0.0, 1.0]
        };

        // Normal (3 floats)
        writer.write_all(&n[0].to_le_bytes())?;
        writer.write_all(&n[1].to_le_bytes())?;
        writer.write_all(&n[2].to_le_bytes())?;

        // Vertices (3 * 3 floats)
        writer.write_all(&v0[0].to_le_bytes())?;
        writer.write_all(&v0[1].to_le_bytes())?;
        writer.write_all(&v0[2].to_le_bytes())?;

        writer.write_all(&v1[0].to_le_bytes())?;
        writer.write_all(&v1[1].to_le_bytes())?;
        writer.write_all(&v1[2].to_le_bytes())?;

        writer.write_all(&v2[0].to_le_bytes())?;
        writer.write_all(&v2[1].to_le_bytes())?;
        writer.write_all(&v2[2].to_le_bytes())?;

        // Attribute byte count (2 bytes, usually 0)
        writer.write_all(&0u16.to_le_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::mesh::{Mesh, Triangle, Vertex};
    use super::*;
    use std::io::Read;

    #[test]
    fn test_ply_ascii_export() -> Result<()> {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Vertex::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Vertex::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Vertex::new(0.0, 1.0, 0.0));
        mesh.add_triangle(Triangle::new(v0, v1, v2));

        let temp_path = std::env::temp_dir().join("test_mesh.ply");
        export_mesh_ply(&mesh, &temp_path, false)?;

        // Verify file exists and has content
        let mut file = File::open(&temp_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert!(contents.contains("ply"));
        assert!(contents.contains("element vertex 3"));
        assert!(contents.contains("element face 1"));

        std::fs::remove_file(&temp_path).ok();

        Ok(())
    }

    #[test]
    fn test_obj_export() -> Result<()> {
        let mut mesh = Mesh::new();
        let v0 = mesh.add_vertex(Vertex::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Vertex::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Vertex::new(0.0, 1.0, 0.0));
        mesh.add_triangle(Triangle::new(v0, v1, v2));

        let temp_path = std::env::temp_dir().join("test_mesh.obj");
        export_mesh_obj(&mesh, &temp_path)?;

        // Verify file exists and has content
        let mut file = File::open(&temp_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        assert!(contents.contains("v 0"));
        assert!(contents.contains("v 1"));
        assert!(contents.contains("f 1 2 3"));

        std::fs::remove_file(&temp_path).ok();

        Ok(())
    }
}
