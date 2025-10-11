//! Error types for OctaIndex3D

use thiserror::Error;

/// Result type alias for OctaIndex3D operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for OctaIndex3D
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Invalid parity for BCC lattice coordinates
    #[error("Invalid parity: coordinates ({x}, {y}, {z}) must have identical parity")]
    InvalidParity { x: i32, y: i32, z: i32 },

    /// Invalid resolution value
    #[error("Invalid resolution: {0} (must be 0-255)")]
    InvalidResolution(u16),

    /// Invalid frame value
    #[error("Invalid frame: {0} (must be 0-255)")]
    InvalidFrame(u16),

    /// Coordinate out of range for bit field
    #[error("Coordinate out of range: {coord} (must fit in {bits} bits)")]
    CoordinateOutOfRange { coord: i32, bits: u8 },

    /// Cell ID decoding error
    #[error("Failed to decode cell ID: {0}")]
    DecodingError(String),

    /// Cell ID encoding error
    #[error("Failed to encode cell ID: {0}")]
    EncodingError(String),

    /// Bech32m encoding/decoding error
    #[error("Bech32m error: {0}")]
    Bech32Error(String),

    /// No parent cell (already at resolution 0)
    #[error("No parent cell: already at resolution 0")]
    NoParent,

    /// No children cells (already at maximum resolution)
    #[error("No children cells: already at maximum resolution")]
    NoChildren,

    /// Invalid aggregation operation
    #[error("Invalid aggregation: {0}")]
    InvalidAggregation(String),

    /// Pathfinding error
    #[error("Pathfinding error: {0}")]
    PathfindingError(String),

    /// No path found between cells
    #[error("No path found from {start} to {goal}")]
    NoPathFound { start: String, goal: String },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Feature not implemented
    #[error("Feature not yet implemented: {0}")]
    NotImplemented(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err.to_string())
    }
}

impl From<bech32::DecodeError> for Error {
    fn from(err: bech32::DecodeError) -> Self {
        Error::Bech32Error(err.to_string())
    }
}

impl From<bech32::EncodeError> for Error {
    fn from(err: bech32::EncodeError) -> Self {
        Error::Bech32Error(err.to_string())
    }
}
