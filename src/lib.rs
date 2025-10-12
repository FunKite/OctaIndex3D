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
//! - **Uniform 3D Tiling**: Truncated octahedron cells tile 3D space without gaps
//! - **14-Neighbor Connectivity**: More isotropic than cubic grids
//! - **Hierarchical 8:1 Refinement**: Multiresolution support
//! - **Robust Cell IDs**: 128-bit format with Bech32m encoding
//! - **First-Class Routing**: A* pathfinding with closed-set optimization and configurable limits
//! - **Data Aggregation**: Efficient spatial queries and roll-ups
//!
//! ## Example
//!
//! ```rust
//! use octaindex3d::CellID;
//!
//! // Create a cell at coordinates (0, 0, 0) at resolution 5
//! // Coordinates must have identical parity (all even or all odd)
//! let cell = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
//!
//! // Get its 14 neighbors
//! let neighbors = cell.neighbors();
//! assert_eq!(neighbors.len(), 14);
//!
//! // Get parent cell (one resolution coarser)
//! let parent = cell.parent().unwrap();
//! assert_eq!(parent.resolution(), 4);
//! ```

pub mod error;
pub mod id;
pub mod io;
pub mod lattice;
pub mod layer;
pub mod path;

// Re-export commonly used types
pub use crate::error::{Error, Result};
pub use crate::id::CellID;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // Verify version string is in expected format (e.g., "0.2.1")
        assert!(VERSION.contains('.'));
        assert!(VERSION.chars().any(|c| c.is_ascii_digit()));
    }
}
