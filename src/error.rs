//! Error types for OctaIndex3D v0.3.0

use thiserror::Error;

/// Result type alias for OctaIndex3D operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for OctaIndex3D
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Invalid parity for BCC lattice coordinates
    #[error("Invalid parity: coordinates ({x}, {y}, {z}) must all have the same parity")]
    InvalidParity {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Z coordinate
        z: i32,
    },

    /// Coordinate value out of valid range
    #[error("Coordinate out of range: {0}")]
    OutOfRange(String),

    /// Coordinate overflow during arithmetic
    #[error("Coordinate overflow during operation")]
    CoordinateOverflow,

    /// Invalid frame ID
    #[error("Invalid frame ID: {0}")]
    InvalidFrameID(u8),

    /// Frame conflict - attempting to register different frame with same ID
    #[error("Frame conflict: frame {0} already registered with different descriptor")]
    FrameConflict(u8),

    /// Invalid LOD (level of detail) value
    #[error("Invalid LOD: {0}")]
    InvalidLOD(String),

    /// Invalid scale tier value
    #[error("Invalid scale tier: {0}")]
    InvalidScaleTier(String),

    /// Bech32 encoding/decoding error
    #[error("Bech32 error: {kind}")]
    InvalidBech32 {
        /// Error description
        kind: String,
    },

    /// Unsupported compression codec
    #[error("Unsupported codec: {0}")]
    UnsupportedCodec(u8),

    /// Compression/decompression error
    #[error("Codec error: {0}")]
    Codec(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(String),

    /// Invalid container format
    #[error("Invalid container format: {0}")]
    InvalidFormat(String),

    /// CRC checksum mismatch
    #[error("CRC mismatch: expected {expected:08x}, got {actual:08x}")]
    CrcMismatch {
        /// Expected CRC value
        expected: u32,
        /// Actual CRC value
        actual: u32,
    },

    /// No parent cell available
    #[error("No parent cell available")]
    NoParent,

    /// No children cells available
    #[error("No children cells available")]
    NoChildren,

    /// Invalid Morton encoding
    #[error("Invalid Morton encoding: {0}")]
    InvalidMorton(String),

    /// Pathfinding error
    #[error("Pathfinding error: {0}")]
    Pathfinding(String),

    /// No path found between cells
    #[error("No path found from {start} to {goal}")]
    NoPathFound {
        /// Start cell identifier
        start: String,
        /// Goal cell identifier
        goal: String,
    },

    /// Search limit exceeded during pathfinding
    #[error("Search limit exceeded: expanded {expansions} nodes (limit: {limit})")]
    SearchLimitExceeded {
        /// Number of nodes expanded
        expansions: usize,
        /// Maximum allowed expansions
        limit: usize,
    },

    // Legacy error variants for compatibility
    /// Invalid aggregation operation
    #[error("Invalid aggregation: {0}")]
    InvalidAggregation(String),

    /// IO error (legacy)
    #[error("IO error: {0}")]
    IoError(String),

    /// Bech32m encoding/decoding error (legacy)
    #[error("Bech32m error: {0}")]
    Bech32Error(String),

    /// Cell ID decoding error (legacy)
    #[error("Failed to decode cell ID: {0}")]
    DecodingError(String),

    /// Cell ID encoding error (legacy)
    #[error("Failed to encode cell ID: {0}")]
    EncodingError(String),

    // v0.3.1 error variants
    /// Invalid container version
    #[error("Invalid container version: {0}")]
    InvalidContainerVersion(u8),

    /// Footer not found during container recovery
    #[error("Footer not found in container")]
    FooterNotFound,

    /// TOC (table of contents) corruption
    #[error("TOC corrupt: {0}")]
    TocCorrupt(String),

    /// SHA-256 hash mismatch
    #[error("SHA-256 mismatch")]
    Sha256Mismatch,
}

impl From<std::io::Error> for Error {
    /// Convert from std::io::Error
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<bech32::DecodeError> for Error {
    /// Convert from Bech32 decode error
    fn from(err: bech32::DecodeError) -> Self {
        Error::InvalidBech32 {
            kind: err.to_string(),
        }
    }
}

impl From<bech32::EncodeError> for Error {
    /// Convert from Bech32 encode error
    fn from(err: bech32::EncodeError) -> Self {
        Error::InvalidBech32 {
            kind: err.to_string(),
        }
    }
}

impl From<bech32::primitives::hrp::Error> for Error {
    /// Convert from Bech32 HRP error
    fn from(err: bech32::primitives::hrp::Error) -> Self {
        Error::InvalidBech32 {
            kind: err.to_string(),
        }
    }
}

#[cfg(feature = "gis_geojson")]
impl From<serde_json::Error> for Error {
    /// Convert from serde_json error
    fn from(err: serde_json::Error) -> Self {
        Error::Io(err.to_string())
    }
}
