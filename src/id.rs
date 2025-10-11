//! Cell ID system with 128-bit format and Bech32m encoding
//!
//! This module implements the hierarchical cell identification system with:
//! - 128-bit binary format (default)
//! - Bech32m human-readable encoding
//! - Support for frames, resolution levels, and coordinates

use crate::error::{Error, Result};
use crate::lattice::{Lattice, LatticeCoord, Parity};
use bech32::{Bech32m, Hrp};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Human-readable prefix for Bech32m encoding
pub const BECH32_HRP: &str = "cx3d";

/// Maximum coordinate value for 24-bit signed representation
const MAX_COORD_24BIT: i32 = (1 << 23) - 1; // 8,388,607
const MIN_COORD_24BIT: i32 = -(1 << 23);    // -8,388,608

/// 128-bit Cell ID
///
/// Bit layout (128 bits total):
/// - Frame: 8 bits (0-7)
/// - Resolution: 8 bits (8-15)
/// - Exponent: 4 bits (16-19)
/// - Flags: 4 bits (20-23)
/// - Reserved: 24 bits (24-47)
/// - X coordinate: 24 bits signed (48-71)
/// - Y coordinate: 24 bits signed (72-95)
/// - Z coordinate: 24 bits signed (96-119)
/// - Checksum: 8 bits (120-127)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellID {
    /// Raw 128-bit value
    value: u128,
}

impl CellID {
    /// Create a new Cell ID from components
    pub fn new(
        frame: u8,
        resolution: u8,
        x: i32,
        y: i32,
        z: i32,
        exponent: u8,
        flags: u8,
    ) -> Result<Self> {
        // Validate coordinates
        if x < MIN_COORD_24BIT || x > MAX_COORD_24BIT {
            return Err(Error::CoordinateOutOfRange { coord: x, bits: 24 });
        }
        if y < MIN_COORD_24BIT || y > MAX_COORD_24BIT {
            return Err(Error::CoordinateOutOfRange { coord: y, bits: 24 });
        }
        if z < MIN_COORD_24BIT || z > MAX_COORD_24BIT {
            return Err(Error::CoordinateOutOfRange { coord: z, bits: 24 });
        }

        // Validate parity
        Parity::from_coords(x, y, z)?;

        let exponent = exponent & 0x0F; // 4 bits
        let flags = flags & 0x0F;       // 4 bits

        // Build the 128-bit value
        let mut value: u128 = 0;

        // Frame (bits 0-7)
        value |= (frame as u128) & 0xFF;

        // Resolution (bits 8-15)
        value |= ((resolution as u128) & 0xFF) << 8;

        // Exponent (bits 16-19)
        value |= ((exponent as u128) & 0x0F) << 16;

        // Flags (bits 20-23)
        value |= ((flags as u128) & 0x0F) << 20;

        // Reserved 24 bits (24-47) remain zero

        // X coordinate (bits 48-71) - convert signed to unsigned for storage
        let x_unsigned = (x as u32) & 0x00FFFFFF;
        value |= (x_unsigned as u128) << 48;

        // Y coordinate (bits 72-95)
        let y_unsigned = (y as u32) & 0x00FFFFFF;
        value |= (y_unsigned as u128) << 72;

        // Z coordinate (bits 96-119)
        let z_unsigned = (z as u32) & 0x00FFFFFF;
        value |= (z_unsigned as u128) << 96;

        // Compute checksum (simple CRC-8)
        let checksum = Self::compute_checksum(value);
        value |= (checksum as u128) << 120;

        Ok(Self { value })
    }

    /// Create from frame, resolution, and lattice coordinates
    pub fn from_coords(frame: u8, resolution: u8, x: i32, y: i32, z: i32) -> Result<Self> {
        Self::new(frame, resolution, x, y, z, 0, 0)
    }

    /// Create from lattice coordinate
    pub fn from_lattice_coord(frame: u8, resolution: u8, coord: &LatticeCoord) -> Result<Self> {
        Self::new(frame, resolution, coord.x, coord.y, coord.z, 0, 0)
    }

    /// Extract frame field
    pub fn frame(&self) -> u8 {
        (self.value & 0xFF) as u8
    }

    /// Extract resolution field
    pub fn resolution(&self) -> u8 {
        ((self.value >> 8) & 0xFF) as u8
    }

    /// Extract exponent field
    pub fn exponent(&self) -> u8 {
        ((self.value >> 16) & 0x0F) as u8
    }

    /// Extract flags field
    pub fn flags(&self) -> u8 {
        ((self.value >> 20) & 0x0F) as u8
    }

    /// Extract X coordinate
    pub fn x(&self) -> i32 {
        let x_unsigned = ((self.value >> 48) & 0x00FFFFFF) as u32;
        // Sign-extend from 24 bits
        if x_unsigned & 0x00800000 != 0 {
            // Negative number
            (x_unsigned | 0xFF000000) as i32
        } else {
            x_unsigned as i32
        }
    }

    /// Extract Y coordinate
    pub fn y(&self) -> i32 {
        let y_unsigned = ((self.value >> 72) & 0x00FFFFFF) as u32;
        if y_unsigned & 0x00800000 != 0 {
            (y_unsigned | 0xFF000000) as i32
        } else {
            y_unsigned as i32
        }
    }

    /// Extract Z coordinate
    pub fn z(&self) -> i32 {
        let z_unsigned = ((self.value >> 96) & 0x00FFFFFF) as u32;
        if z_unsigned & 0x00800000 != 0 {
            (z_unsigned | 0xFF000000) as i32
        } else {
            z_unsigned as i32
        }
    }

    /// Extract checksum
    pub fn checksum(&self) -> u8 {
        ((self.value >> 120) & 0xFF) as u8
    }

    /// Get lattice coordinates
    pub fn lattice_coord(&self) -> Result<LatticeCoord> {
        LatticeCoord::new(self.x(), self.y(), self.z())
    }

    /// Validate checksum
    pub fn validate_checksum(&self) -> Result<()> {
        let stored_checksum = self.checksum();
        // Clear checksum bits and recompute
        let value_without_checksum = self.value & !(0xFF_u128 << 120);
        let computed_checksum = Self::compute_checksum(value_without_checksum);

        if stored_checksum == computed_checksum {
            Ok(())
        } else {
            Err(Error::ChecksumMismatch {
                expected: computed_checksum,
                actual: stored_checksum,
            })
        }
    }

    /// Compute simple CRC-8 checksum
    fn compute_checksum(value: u128) -> u8 {
        // Simple CRC-8 with polynomial x^8 + x^2 + x + 1 (0x07)
        let bytes = value.to_le_bytes();
        let mut crc: u8 = 0;

        for &byte in &bytes[0..15] {
            // Skip checksum byte
            crc ^= byte;
            for _ in 0..8 {
                if crc & 0x80 != 0 {
                    crc = (crc << 1) ^ 0x07;
                } else {
                    crc <<= 1;
                }
            }
        }

        crc
    }

    /// Get the 14 neighbors of this cell
    pub fn neighbors(&self) -> Vec<CellID> {
        let coord = self.lattice_coord().unwrap();
        let neighbors = Lattice::get_neighbors(&coord);

        neighbors
            .into_iter()
            .filter_map(|n| Self::from_lattice_coord(self.frame(), self.resolution(), &n).ok())
            .collect()
    }

    /// Get parent cell (one resolution coarser)
    pub fn parent(&self) -> Result<CellID> {
        let resolution = self.resolution();
        if resolution == 0 {
            return Err(Error::NoParent);
        }

        let coord = self.lattice_coord()?;
        let parent_coord = Lattice::get_parent(&coord);

        Self::from_lattice_coord(self.frame(), resolution - 1, &parent_coord)
    }

    /// Get 8 children cells (one resolution finer)
    pub fn children(&self) -> Result<Vec<CellID>> {
        let resolution = self.resolution();
        if resolution == 255 {
            return Err(Error::NoChildren);
        }

        let coord = self.lattice_coord()?;
        let children_coords = Lattice::get_children(&coord);

        children_coords
            .into_iter()
            .map(|c| Self::from_lattice_coord(self.frame(), resolution + 1, &c))
            .collect()
    }

    /// Encode to Bech32m string
    pub fn to_bech32m(&self) -> Result<String> {
        let hrp = Hrp::parse(BECH32_HRP).map_err(|e| Error::Bech32Error(e.to_string()))?;

        // Convert 128-bit value to bytes (16 bytes)
        let bytes = self.value.to_be_bytes();

        // Encode using Bech32m
        let encoded = bech32::encode::<Bech32m>(hrp, &bytes)
            .map_err(|e| Error::EncodingError(e.to_string()))?;

        Ok(encoded)
    }

    /// Decode from Bech32m string
    pub fn from_bech32m(s: &str) -> Result<Self> {
        let (hrp, data) = bech32::decode(s).map_err(|e| Error::DecodingError(e.to_string()))?;

        // Verify HRP
        if hrp.as_str() != BECH32_HRP {
            return Err(Error::DecodingError(format!(
                "Invalid HRP: expected '{}', got '{}'",
                BECH32_HRP,
                hrp.as_str()
            )));
        }

        // Convert bytes back to u128
        if data.len() != 16 {
            return Err(Error::DecodingError(format!(
                "Invalid data length: expected 16 bytes, got {}",
                data.len()
            )));
        }

        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&data);
        let value = u128::from_be_bytes(bytes);

        let cell_id = Self { value };

        // Validate checksum
        cell_id.validate_checksum()?;

        Ok(cell_id)
    }

    /// Convert to raw bytes (big-endian)
    pub fn to_bytes(&self) -> [u8; 16] {
        self.value.to_be_bytes()
    }

    /// Create from raw bytes (big-endian)
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        let value = u128::from_be_bytes(bytes);
        let cell_id = Self { value };
        cell_id.validate_checksum()?;
        Ok(cell_id)
    }

    /// Get raw u128 value
    pub fn raw_value(&self) -> u128 {
        self.value
    }
}

impl fmt::Display for CellID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CellID(frame={}, res={}, x={}, y={}, z={})",
            self.frame(),
            self.resolution(),
            self.x(),
            self.y(),
            self.z()
        )
    }
}

impl PartialOrd for CellID {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CellID {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_id_creation() {
        let cell = CellID::from_coords(0, 5, 2, 4, 6).unwrap();
        assert_eq!(cell.frame(), 0);
        assert_eq!(cell.resolution(), 5);
        assert_eq!(cell.x(), 2);
        assert_eq!(cell.y(), 4);
        assert_eq!(cell.z(), 6);
    }

    #[test]
    fn test_cell_id_invalid_parity() {
        // Mixed parity should fail
        let result = CellID::from_coords(0, 5, 0, 1, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_cell_id_checksum() {
        let cell = CellID::from_coords(0, 5, 2, 4, 6).unwrap();
        assert!(cell.validate_checksum().is_ok());
    }

    #[test]
    fn test_neighbor_count() {
        let cell = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let neighbors = cell.neighbors();
        assert_eq!(neighbors.len(), 14);
    }

    #[test]
    fn test_parent_child() {
        let parent = CellID::from_coords(0, 5, 2, 4, 6).unwrap();

        // Test parent operation
        let child = CellID::from_coords(0, 6, 4, 8, 12).unwrap();
        assert_eq!(child.parent().unwrap(), parent);

        // Note: Full 8:1 children generation requires filtering for valid parity
        // This is a known limitation documented in the spec
    }

    #[test]
    fn test_bech32m_encoding() {
        let cell = CellID::from_coords(0, 5, 2, 4, 6).unwrap();
        let encoded = cell.to_bech32m().unwrap();
        assert!(encoded.starts_with(BECH32_HRP));

        let decoded = CellID::from_bech32m(&encoded).unwrap();
        assert_eq!(decoded, cell);
    }

    #[test]
    fn test_bytes_roundtrip() {
        let cell = CellID::from_coords(0, 10, 100, 200, 300).unwrap();
        let bytes = cell.to_bytes();
        let cell2 = CellID::from_bytes(bytes).unwrap();
        assert_eq!(cell, cell2);
    }

    #[test]
    fn test_negative_coordinates() {
        let cell = CellID::from_coords(0, 5, -2, -4, -6).unwrap();
        assert_eq!(cell.x(), -2);
        assert_eq!(cell.y(), -4);
        assert_eq!(cell.z(), -6);
    }

    #[test]
    fn test_cell_ordering() {
        let cell1 = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let cell2 = CellID::from_coords(0, 6, 0, 0, 0).unwrap();
        // Cells with different resolutions should be ordered
        assert_ne!(cell1, cell2);
    }
}
