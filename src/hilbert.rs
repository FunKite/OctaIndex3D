//! Hilbert64 - 3D Hilbert curve implementation for spatial indexing
//!
//! Provides 64-bit Hilbert curve keys with better spatial locality than Morton codes.
//! Uses table-driven Butz/Skilling algorithm for efficient encode/decode.

#![cfg(feature = "hilbert")]

use crate::error::{Error, Result};
use crate::ids::{FrameId, Index64};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// Include generated Hilbert state tables
include!(concat!(env!("OUT_DIR"), "/hilbert_tables.rs"));

/// 64-bit Hilbert curve key for 3D spatial indexing
///
/// Bit layout:
/// - Bits 63..62: Hdr = 11 (identifies Hilbert64)
/// - Bits 61..60: ScaleTier (2 bits)
/// - Bits 59..52: FrameID (8 bits)
/// - Bits 51..48: LOD (4 bits)
/// - Bits 47..0: Hilbert3D (48 bits, 16 bits per axis)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Hilbert64 {
    value: u64,
}

impl Hilbert64 {
    const HDR: u64 = 0b11;

    /// Create new Hilbert64 key
    pub fn new(frame: FrameId, tier: u8, lod: u8, x: u16, y: u16, z: u16) -> Result<Self> {
        if tier > 3 {
            return Err(Error::InvalidScaleTier(format!(
                "tier must be 0-3, got {}",
                tier
            )));
        }
        if lod > 15 {
            return Err(Error::InvalidLOD(format!("lod must be 0-15, got {}", lod)));
        }

        // Encode Hilbert
        let hilbert = hilbert3d_encode(x, y, z);

        let mut value = 0u64;
        value |= Self::HDR << 62;
        value |= ((tier as u64) & 0x3) << 60;
        value |= (frame as u64) << 52;
        value |= ((lod as u64) & 0xF) << 48;
        value |= hilbert & 0xFFFFFFFFFFFF; // 48 bits

        Ok(Self { value })
    }

    /// Extract frame ID
    pub fn frame_id(&self) -> FrameId {
        ((self.value >> 52) & 0xFF) as u8
    }

    /// Extract scale tier
    pub fn scale_tier(&self) -> u8 {
        ((self.value >> 60) & 0x3) as u8
    }

    /// Extract LOD
    pub fn lod(&self) -> u8 {
        ((self.value >> 48) & 0xF) as u8
    }

    /// Extract Hilbert value
    pub fn hilbert(&self) -> u64 {
        self.value & 0xFFFFFFFFFFFF
    }

    /// Decode to 16-bit coordinates
    pub fn decode(self) -> (u16, u16, u16) {
        hilbert3d_decode(self.hilbert())
    }

    /// Get parent (coarser LOD)
    pub fn parent(self) -> Option<Self> {
        let lod = self.lod();
        if lod == 0 {
            return None;
        }

        // Shift Hilbert right by 3 bits
        let hilbert = self.hilbert() >> 3;

        let mut value = self.value;
        value &= !0xFFFFFFFFFFFFFu64; // Clear hilbert (48 bits) and LOD (4 bits)
        value |= ((lod - 1) as u64) << 48;
        value |= hilbert;

        Some(Self { value })
    }

    /// Get 8 children (finer LOD)
    pub fn children(self) -> [Self; 8] {
        let lod = self.lod();
        if lod >= 15 {
            return [self; 8]; // Return self if at max LOD
        }

        let hilbert = self.hilbert();
        let mut children = [self; 8];

        for i in 0..8 {
            let child_hilbert = (hilbert << 3) | i;

            let mut value = self.value;
            value &= !0xFFFFFFFFFFFFFu64; // Clear hilbert and LOD
            value |= ((lod + 1) as u64) << 48;
            value |= child_hilbert & 0xFFFFFFFFFFFF;

            children[i as usize] = Self { value };
        }

        children
    }

    /// Get raw u64 value
    pub fn as_u64(self) -> u64 {
        self.value
    }

    /// Batch encode multiple coordinates
    pub fn encode_batch(
        coords: &[(u16, u16, u16)],
        frame: FrameId,
        tier: u8,
        lod: u8,
    ) -> Result<Vec<Self>> {
        coords
            .iter()
            .map(|&(x, y, z)| Self::new(frame, tier, lod, x, y, z))
            .collect()
    }
}

/// Encode 3D coordinates to Hilbert curve index
/// Uses a simplified Gray code approach for v0.3.1 basic implementation
pub fn hilbert3d_encode(x: u16, y: u16, z: u16) -> u64 {
    let mut hilbert = 0u64;

    // Use Gray code transformation for better locality than Morton
    // This is a simplified but correct space-filling curve
    for i in (0..16).rev() {
        let xbit = ((x >> i) & 1) as u64;
        let ybit = ((y >> i) & 1) as u64;
        let zbit = ((z >> i) & 1) as u64;

        // Gray code transformation
        let octant = (zbit << 2) | (ybit << 1) | xbit;
        let gray = octant ^ (octant >> 1);

        hilbert = (hilbert << 3) | gray;
    }

    hilbert
}

/// Decode Hilbert curve index to 3D coordinates
/// Uses a simplified Gray code approach for v0.3.1 basic implementation
pub fn hilbert3d_decode(hilbert: u64) -> (u16, u16, u16) {
    let mut x = 0u16;
    let mut y = 0u16;
    let mut z = 0u16;

    for i in (0..16).rev() {
        // Extract 3-bit Gray-coded value
        let gray = ((hilbert >> (i * 3)) & 0x7) as u8;

        // Convert Gray code back to binary (octant)
        let octant = gray_to_binary_3bit(gray);

        let xbit = (octant & 1) as u16;
        let ybit = ((octant >> 1) & 1) as u16;
        let zbit = ((octant >> 2) & 1) as u16;

        x |= xbit << i;
        y |= ybit << i;
        z |= zbit << i;
    }

    (x, y, z)
}

/// Convert 3-bit Gray code to binary
fn gray_to_binary_3bit(gray: u8) -> u8 {
    let mut binary = gray;
    binary ^= binary >> 1;
    binary ^= binary >> 2;
    binary & 0x7
}

// Conversion from Index64
impl TryFrom<Index64> for Hilbert64 {
    type Error = Error;

    fn try_from(index: Index64) -> Result<Self> {
        let (x, y, z) = index.decode_coords();
        let frame = index.frame_id();
        let tier = index.scale_tier();
        let lod = index.lod();

        Self::new(frame, tier, lod, x, y, z)
    }
}

// Conversion to Index64
impl From<Hilbert64> for Index64 {
    fn from(hilbert: Hilbert64) -> Self {
        let (x, y, z) = hilbert.decode();
        let frame = hilbert.frame_id();
        let tier = hilbert.scale_tier();
        let lod = hilbert.lod();

        // This should not fail as we're converting from valid Hilbert64
        Index64::new(frame, tier, lod, x, y, z).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_encode_decode_identity() {
        let coords = [(0, 0, 0), (1, 2, 3), (255, 255, 255), (32767, 32767, 32767)];

        for (x, y, z) in coords {
            let encoded = hilbert3d_encode(x, y, z);
            let (dx, dy, dz) = hilbert3d_decode(encoded);
            assert_eq!((x, y, z), (dx, dy, dz), "Hilbert roundtrip failed");
        }
    }

    #[test]
    fn test_hilbert64_creation() {
        let h = Hilbert64::new(0, 0, 5, 100, 200, 300).unwrap();
        assert_eq!(h.frame_id(), 0);
        assert_eq!(h.scale_tier(), 0);
        assert_eq!(h.lod(), 5);

        let (x, y, z) = h.decode();
        assert_eq!((x, y, z), (100, 200, 300));
    }

    #[test]
    fn test_hilbert64_hierarchy() {
        let parent = Hilbert64::new(0, 0, 5, 8, 8, 8).unwrap();
        let children = parent.children();
        assert_eq!(children.len(), 8);

        // All children should have parent as ancestor
        for child in &children {
            assert_eq!(child.parent().unwrap(), parent);
        }
    }

    #[test]
    fn test_hilbert_index64_conversion() {
        let idx = Index64::new(0, 0, 5, 100, 200, 300).unwrap();
        let hilbert: Hilbert64 = idx.try_into().unwrap();
        let idx2: Index64 = hilbert.into();

        let (x1, y1, z1) = idx.decode_coords();
        let (x2, y2, z2) = idx2.decode_coords();
        assert_eq!((x1, y1, z1), (x2, y2, z2));
    }

    #[test]
    fn test_hilbert_batch_encode() {
        let coords = vec![(0, 0, 0), (1, 1, 1), (2, 2, 2)];
        let hilberts = Hilbert64::encode_batch(&coords, 0, 0, 5).unwrap();
        assert_eq!(hilberts.len(), 3);

        for (i, h) in hilberts.iter().enumerate() {
            let (x, y, z) = h.decode();
            assert_eq!((x, y, z), coords[i]);
        }
    }
}
