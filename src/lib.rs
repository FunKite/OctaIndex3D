//! # OctaIndex3D
//!
//! A 3D Spatial Indexing and Routing System based on Body-Centered Cubic (BCC) lattice
//! with truncated octahedral cells.
//!
//! This library provides efficient spatial analysis, indexing, and pathfinding in three
//! dimensions at multiple scales using a BCC lattice structure.
//!
//! ## Key Features
//!
//! - **High-Level Facade**: [`BccGrid`] for working in physical units (points to
//!   cells, neighbors, k-rings, A* pathfinding) without lattice details
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
//! use octaindex3d::{BccGrid, Index64, Result};
//!
//! # fn main() -> Result<()> {
//! // High-level API: a grid with 0.5-unit cells
//! let grid = BccGrid::new(0.5)?;
//! let cell = grid.cell_at(1.2, 3.4, 5.6)?;
//! assert_eq!(grid.neighbors(cell).len(), 14);
//!
//! let start = grid.cell_at(0.0, 0.0, 0.0)?;
//! let goal = grid.cell_at(3.0, 3.0, 3.0)?;
//! let path = grid.astar(start, goal)?;
//! assert_eq!(path.cells.last(), Some(&goal));
//!
//! // Lower-level API: a Morton-encoded storage key
//! let index = Index64::new(0, 0, 5, 100, 200, 300)?;
//! assert_eq!(index.decode_coords(), (100, 200, 300));
//! # Ok(())
//! # }
//! ```

pub mod compression;
pub mod container;
pub mod error;
pub mod frame;
pub mod grid;
pub mod ids;
pub mod lattice;
pub mod layers;
pub mod morton;
pub mod neighbors;
pub mod performance;

// v0.3.1 modules (feature-gated)
#[cfg(feature = "hilbert")]
pub mod hilbert;

#[cfg(feature = "container_v2")]
pub mod container_v2;

#[cfg(feature = "gis_geojson")]
pub mod geojson;

// Legacy v0.2 modules (deprecated, kept for compatibility)
pub mod id;
#[cfg(feature = "serde")]
pub mod io;
pub mod layer;
pub mod path;

// Re-export commonly used types
pub use crate::error::{Error, Result};
pub use crate::frame::{get_frame, list_frames, register_frame, FrameDescriptor};
pub use crate::grid::{BccGrid, GridPath};
pub use crate::ids::{FrameId, Galactic128, Index64, Route64};
pub use crate::lattice::{Lattice, LatticeCoord, Parity, BCC_NEIGHBORS_14};
pub use crate::layers::{
    export_mesh_obj, export_mesh_ply, export_mesh_stl, extract_mesh_from_tsdf, ESDFLayer,
    LayeredMap, Measurement, Mesh, OccupancyLayer, OccupancyState, OccupancyStats, TSDFLayer,
};

// Performance module re-exports
pub use crate::performance::{Backend, BatchIndexBuilder, BatchNeighborCalculator, BatchResult};

#[cfg(feature = "parallel")]
pub use crate::performance::{ParallelBatchIndexBuilder, ParallelBatchNeighborCalculator};

#[cfg(any(feature = "gpu-metal", feature = "gpu-vulkan"))]
pub use crate::performance::{GpuBackend, GpuBatchProcessor};

// v0.3.1 re-exports (feature-gated)
#[cfg(feature = "hilbert")]
pub use crate::hilbert::Hilbert64;

#[cfg(feature = "container_v2")]
pub use crate::container_v2::{ContainerWriterV2, HeaderV2, StreamConfig};

#[cfg(feature = "gis_geojson")]
pub use crate::geojson::{
    to_geojson_points, write_geojson_linestring, write_geojson_polygon, GeoJsonOptions,
};

// Legacy re-export (deprecated, kept for compatibility)
#[allow(deprecated)]
pub use crate::id::CellID;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Compile the README's Rust code blocks as doctests so the documentation
/// stays in sync with the implementation.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(VERSION, "0.5.6");
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
