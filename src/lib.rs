//! # OctaIndex3D v0.3.0
//!
//! A 3D Spatial Indexing and Routing System based on Body-Centered Cubic (BCC) lattice
//! with truncated octahedral cells.
//!
//! This library provides efficient spatial analysis, indexing, and pathfinding in three
//! dimensions at multiple scales using a BCC lattice structure.
//!
//! ## Key Features
//!
//! - **Three ID Types**: Galactic128 (global), Index64 (Morton), Route64 (local routing)
//! - **14-Neighbor Connectivity**: More isotropic than cubic grids
//! - **Hierarchical Refinement**: Multi-resolution support
//! - **Bech32m Encoding**: Human-readable text encoding with checksums
//! - **Compression**: LZ4 (default) and optional Zstd support
//! - **Frame Registry**: Coordinate reference system management
//! - **Container Format**: Compressed spatial data storage
//!
//! ## Example
//!
//! ```rust
//! use octaindex3d::{Galactic128, Index64, Route64, Result};
//!
//! # fn main() -> Result<()> {
//! // Create a global ID
//! let galactic = Galactic128::new(0, 5, 1, 10, 0, 2, 4, 6)?;
//!
//! // Create a Morton-encoded index
//! let index = Index64::new(0, 0, 5, 100, 200, 300)?;
//!
//! // Create a local routing coordinate
//! let route = Route64::new(0, 100, 200, 300)?;
//!
//! // Get neighbors
//! let neighbors = octaindex3d::neighbors::neighbors_route64(route);
//! assert_eq!(neighbors.len(), 14);
//! # Ok(())
//! # }
//! ```

pub mod compression;
pub mod container;
pub mod error;
pub mod frame;
pub mod ids;
pub mod lattice;
pub mod morton;
pub mod neighbors;

// v0.3.1 modules (feature-gated)
#[cfg(feature = "hilbert")]
pub mod hilbert;

#[cfg(feature = "container_v2")]
pub mod container_v2;

#[cfg(feature = "gis_geojson")]
pub mod geojson;

// Legacy modules (for compatibility)
pub mod id;
pub mod io;
pub mod layer;
pub mod path;

// Re-export commonly used types
pub use crate::error::{Error, Result};
pub use crate::ids::{Galactic128, Index64, Route64, FrameId};
pub use crate::lattice::{Lattice, LatticeCoord, Parity, BCC_NEIGHBORS_14};
pub use crate::frame::{FrameDescriptor, register_frame, get_frame, list_frames};

// v0.3.1 re-exports (feature-gated)
#[cfg(feature = "hilbert")]
pub use crate::hilbert::Hilbert64;

#[cfg(feature = "container_v2")]
pub use crate::container_v2::{ContainerWriterV2, StreamConfig, HeaderV2};

#[cfg(feature = "gis_geojson")]
pub use crate::geojson::{to_geojson_points, write_geojson_linestring, write_geojson_polygon, GeoJsonOptions};

// Legacy re-export
pub use crate::id::CellID;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.3.1");
    }

    #[test]
    fn test_basic_id_creation() {
        // Test Galactic128
        let g = Galactic128::new(0, 5, 1, 10, 0, 2, 4, 6).unwrap();
        assert_eq!(g.frame_id(), 0);

        // Test Index64
        let i = Index64::new(0, 0, 5, 100, 200, 300).unwrap();
        assert_eq!(i.lod(), 5);

        // Test Route64
        let r = Route64::new(0, 100, 200, 300).unwrap();
        assert_eq!((r.x(), r.y(), r.z()), (100, 200, 300));
    }

    #[test]
    fn test_bcc_neighbors() {
        assert_eq!(BCC_NEIGHBORS_14.len(), 14);
    }
}
