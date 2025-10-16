//! ID types for OctaIndex3D v0.3.0
//!
//! Three interoperable ID types:
//! - Galactic128: 128-bit global IDs with frame and scale
//! - Index64: 64-bit Morton tile keys with tier+LOD
//! - Route64: 64-bit signed BCC coordinates for local pathfinding

use crate::error::{Error, Result};
use crate::lattice::{LatticeCoord, Parity};
use crate::morton;
use bech32::{Bech32m, Hrp};
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Bech32m HRP for Galactic128
pub const HRP_GALACTIC: &str = "g3d1";
/// Bech32m HRP for Index64
pub const HRP_INDEX: &str = "i3d1";
/// Bech32m HRP for Route64
pub const HRP_ROUTE: &str = "r3d1";

/// Frame ID type (8 bits)
pub type FrameId = u8;

// =============================================================================
// Galactic128
// =============================================================================

/// 128-bit global position/cell ID with frame and scale
///
/// Bit layout (MSB→LSB):
/// - Bits 127..120: ScaleMant (8 bits)
/// - Bits 119..118: ScaleTier (2 bits)
/// - Bits 117..112: LOD (6 bits)
/// - Bits 111..104: FrameID (8 bits)
/// - Bits 103..100: AttrInt (4 bits)
/// - Bits  99.. 96: AttrUsr (4 bits)
/// - Bits  95.. 64: X (32 bits signed)
/// - Bits  63.. 32: Y (32 bits signed)
/// - Bits  31..  0: Z (32 bits signed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Galactic128 {
    value: u128,
}

impl Galactic128 {
    /// Create new Galactic128 ID
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame: FrameId,
        scale_mant: u8,
        scale_tier: u8,
        lod: u8,
        attr_usr: u8,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<Self> {
        // Validate parity
        Parity::from_coords(x, y, z)?;

        // Validate ranges
        if scale_tier > 3 {
            return Err(Error::InvalidScaleTier(format!(
                "scale_tier must be 0-3, got {}",
                scale_tier
            )));
        }
        if lod > 63 {
            return Err(Error::InvalidLOD(format!("lod must be 0-63, got {}", lod)));
        }

        let mut value: u128 = 0;

        // Build from MSB to LSB
        value |= (scale_mant as u128) << 120;
        value |= ((scale_tier as u128) & 0x3) << 118;
        value |= ((lod as u128) & 0x3F) << 112;
        value |= (frame as u128) << 104;
        // AttrInt = 1 (version), bits 103..100
        value |= 1u128 << 100;
        // AttrUsr, bits 99..96
        value |= ((attr_usr as u128) & 0xF) << 96;
        // Coordinates
        value |= ((x as u32 as u128) & 0xFFFFFFFF) << 64;
        value |= ((y as u32 as u128) & 0xFFFFFFFF) << 32;
        value |= (z as u32 as u128) & 0xFFFFFFFF;

        Ok(Self { value })
    }

    /// Extract frame ID
    pub fn frame_id(&self) -> FrameId {
        ((self.value >> 104) & 0xFF) as u8
    }

    /// Extract scale mantissa
    pub fn scale_mant(&self) -> u8 {
        (self.value >> 120) as u8
    }

    /// Extract scale tier
    pub fn scale_tier(&self) -> u8 {
        ((self.value >> 118) & 0x3) as u8
    }

    /// Extract LOD
    pub fn lod(&self) -> u8 {
        ((self.value >> 112) & 0x3F) as u8
    }

    /// Extract user attributes
    pub fn attr_usr(&self) -> u8 {
        ((self.value >> 96) & 0xF) as u8
    }

    /// Extract X coordinate
    pub fn x(&self) -> i32 {
        ((self.value >> 64) & 0xFFFFFFFF) as u32 as i32
    }

    /// Extract Y coordinate
    pub fn y(&self) -> i32 {
        ((self.value >> 32) & 0xFFFFFFFF) as u32 as i32
    }

    /// Extract Z coordinate
    pub fn z(&self) -> i32 {
        (self.value & 0xFFFFFFFF) as u32 as i32
    }

    /// Get as lattice coordinate
    pub fn lattice_coord(&self) -> Result<LatticeCoord> {
        LatticeCoord::new(self.x(), self.y(), self.z())
    }

    /// Encode to Bech32m string
    pub fn to_bech32m(&self) -> Result<String> {
        let hrp = Hrp::parse(HRP_GALACTIC)?;
        let bytes = self.value.to_be_bytes();
        let encoded = bech32::encode::<Bech32m>(hrp, &bytes)?;
        Ok(encoded)
    }

    /// Decode from Bech32m string
    pub fn from_bech32m(s: &str) -> Result<Self> {
        let (hrp, data) = bech32::decode(s)?;
        if hrp.as_str() != HRP_GALACTIC {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong HRP: expected {}, got {}", HRP_GALACTIC, hrp),
            });
        }
        if data.len() != 16 {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong length: expected 16 bytes, got {}", data.len()),
            });
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&data);
        Ok(Self {
            value: u128::from_be_bytes(bytes),
        })
    }

    /// Get raw value
    pub fn raw(&self) -> u128 {
        self.value
    }
}

impl fmt::Display for Galactic128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "G128(f={}, t={}:{}, lod={}, {},{},{})",
            self.frame_id(),
            self.scale_tier(),
            self.scale_mant(),
            self.lod(),
            self.x(),
            self.y(),
            self.z()
        )
    }
}

// =============================================================================
// Index64
// =============================================================================

/// 64-bit database key using Morton (Z-order) encoding
///
/// Bit layout:
/// - Bits 63..62: Hdr = 10 (identifies Index64)
/// - Bits 61..60: ScaleTier (2 bits)
/// - Bits 59..52: FrameID (8 bits)
/// - Bits 51..48: LOD (4 bits)
/// - Bits 47.. 0: Morton3D (48 bits, 16 bits per axis)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Index64 {
    value: u64,
}

impl Index64 {
    const HDR: u64 = 0b10;

    /// Create new Index64
    pub fn new(frame: FrameId, tier: u8, lod: u8, x16: u16, y16: u16, z16: u16) -> Result<Self> {
        if tier > 3 {
            return Err(Error::InvalidScaleTier(format!(
                "tier must be 0-3, got {}",
                tier
            )));
        }
        if lod > 15 {
            return Err(Error::InvalidLOD(format!("lod must be 0-15, got {}", lod)));
        }

        // Encode Morton
        let morton = morton::morton_encode(x16, y16, z16);

        let mut value = 0u64;
        value |= Self::HDR << 62;
        value |= ((tier as u64) & 0x3) << 60;
        value |= (frame as u64) << 52;
        value |= ((lod as u64) & 0xF) << 48;
        value |= morton & 0xFFFFFFFFFFFF; // 48 bits

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

    /// Extract Morton value
    pub fn morton(&self) -> u64 {
        self.value & 0xFFFFFFFFFFFF
    }

    /// Decode to 16-bit coordinates
    pub fn decode_coords(&self) -> (u16, u16, u16) {
        morton::morton_decode(self.morton())
    }

    /// Get parent (coarser LOD)
    pub fn parent(&self) -> Option<Self> {
        let lod = self.lod();
        if lod == 0 {
            return None;
        }

        // Shift Morton right by 3 bits
        let morton = self.morton() >> 3;

        let mut value = self.value;
        value &= !0xFFFFFFFFFFFFFu64; // Clear morton (48 bits) and LOD (4 bits) = 52 bits total
        value |= ((lod - 1) as u64) << 48;
        value |= morton;

        Some(Self { value })
    }

    /// Get 8 children (finer LOD)
    pub fn children(&self) -> Vec<Self> {
        let lod = self.lod();
        if lod >= 15 {
            return vec![];
        }

        let morton = self.morton();
        let mut children = Vec::with_capacity(8);

        for i in 0..8 {
            let child_morton = (morton << 3) | i;

            let mut value = self.value;
            value &= !0xFFFFFFFFFFFFFu64; // Clear morton (48 bits) and LOD (4 bits) = 52 bits total
            value |= ((lod + 1) as u64) << 48;
            value |= child_morton & 0xFFFFFFFFFFFF;

            children.push(Self { value });
        }

        children
    }

    /// Encode to Bech32m string
    pub fn to_bech32m(&self) -> Result<String> {
        let hrp = Hrp::parse(HRP_INDEX)?;
        let bytes = self.value.to_be_bytes();
        let encoded = bech32::encode::<Bech32m>(hrp, &bytes)?;
        Ok(encoded)
    }

    /// Decode from Bech32m string
    pub fn from_bech32m(s: &str) -> Result<Self> {
        let (hrp, data) = bech32::decode(s)?;
        if hrp.as_str() != HRP_INDEX {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong HRP: expected {}, got {}", HRP_INDEX, hrp),
            });
        }
        if data.len() != 8 {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong length: expected 8 bytes, got {}", data.len()),
            });
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data);
        Ok(Self {
            value: u64::from_be_bytes(bytes),
        })
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.value
    }
}

impl fmt::Display for Index64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (x, y, z) = self.decode_coords();
        write!(
            f,
            "I64(f={}, t={}, lod={}, morton={:012x}, {},{},{})",
            self.frame_id(),
            self.scale_tier(),
            self.lod(),
            self.morton(),
            x,
            y,
            z
        )
    }
}

// =============================================================================
// Route64
// =============================================================================

/// 64-bit local routing coordinate (signed 20-bit BCC)
///
/// Bit layout:
/// - Bits 63..62: Hdr = 01 (identifies Route64)
/// - Bits 61..60: ScaleTier (2 bits)
/// - Bits 59..40: X (20 bits signed)
/// - Bits 39..20: Y (20 bits signed)
/// - Bits 19.. 0: Z (20 bits signed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Route64 {
    value: u64,
}

impl Route64 {
    const HDR: u64 = 0b01;
    #[allow(dead_code)] // Reserved for future bit manipulation utilities
    const COORD_BITS: u32 = 20;
    const COORD_MAX: i32 = (1 << 19) - 1; // 524287
    const COORD_MIN: i32 = -(1 << 19); // -524288

    /// Create new Route64
    pub fn new(tier: u8, x: i32, y: i32, z: i32) -> Result<Self> {
        // Validate parity
        Parity::from_coords(x, y, z)?;

        // Validate range
        if tier > 3 {
            return Err(Error::InvalidScaleTier(format!(
                "tier must be 0-3, got {}",
                tier
            )));
        }
        if !(Self::COORD_MIN..=Self::COORD_MAX).contains(&x) {
            return Err(Error::OutOfRange(format!("x={} out of 20-bit range", x)));
        }
        if !(Self::COORD_MIN..=Self::COORD_MAX).contains(&y) {
            return Err(Error::OutOfRange(format!("y={} out of 20-bit range", y)));
        }
        if !(Self::COORD_MIN..=Self::COORD_MAX).contains(&z) {
            return Err(Error::OutOfRange(format!("z={} out of 20-bit range", z)));
        }

        let mut value = 0u64;
        value |= Self::HDR << 62;
        value |= ((tier as u64) & 0x3) << 60;
        value |= ((x as u32 as u64) & 0xFFFFF) << 40;
        value |= ((y as u32 as u64) & 0xFFFFF) << 20;
        value |= (z as u32 as u64) & 0xFFFFF;

        Ok(Self { value })
    }

    /// Extract scale tier
    pub fn scale_tier(&self) -> u8 {
        ((self.value >> 60) & 0x3) as u8
    }

    /// Extract X coordinate (signed 20-bit)
    pub fn x(&self) -> i32 {
        let raw = ((self.value >> 40) & 0xFFFFF) as u32;
        sign_extend_20(raw)
    }

    /// Extract Y coordinate (signed 20-bit)
    pub fn y(&self) -> i32 {
        let raw = ((self.value >> 20) & 0xFFFFF) as u32;
        sign_extend_20(raw)
    }

    /// Extract Z coordinate (signed 20-bit)
    pub fn z(&self) -> i32 {
        let raw = (self.value & 0xFFFFF) as u32;
        sign_extend_20(raw)
    }

    /// Get as lattice coordinate
    pub fn lattice_coord(&self) -> Result<LatticeCoord> {
        LatticeCoord::new(self.x(), self.y(), self.z())
    }

    /// Encode to Bech32m string
    pub fn to_bech32m(&self) -> Result<String> {
        let hrp = Hrp::parse(HRP_ROUTE)?;
        let bytes = self.value.to_be_bytes();
        let encoded = bech32::encode::<Bech32m>(hrp, &bytes)?;
        Ok(encoded)
    }

    /// Decode from Bech32m string
    pub fn from_bech32m(s: &str) -> Result<Self> {
        let (hrp, data) = bech32::decode(s)?;
        if hrp.as_str() != HRP_ROUTE {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong HRP: expected {}, got {}", HRP_ROUTE, hrp),
            });
        }
        if data.len() != 8 {
            return Err(Error::InvalidBech32 {
                kind: format!("Wrong length: expected 8 bytes, got {}", data.len()),
            });
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data);
        Ok(Self {
            value: u64::from_be_bytes(bytes),
        })
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.value
    }

    /// Alias for `raw()` - returns the underlying u64 value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Create Route64 from raw u64 value (unsafe - does not validate)
    ///
    /// This is primarily used for GPU/SIMD operations where validation
    /// has already been performed. Use `new()` for general construction.
    pub fn from_value(value: u64) -> Result<Self> {
        // Basic header validation
        let header = (value >> 62) & 0x3;
        if header != Self::HDR {
            return Err(Error::DecodingError(format!(
                "Invalid Route64 header: expected 0x01, got 0x{:02x}",
                header
            )));
        }

        // Extract and validate coordinates
        let route = Self { value };
        let (x, y, z) = (route.x(), route.y(), route.z());

        // Validate parity
        Parity::from_coords(x, y, z)?;

        Ok(route)
    }

    /// Create new Route64 without validation (unsafe, for hot paths only)
    ///
    /// # Safety
    /// Caller must ensure:
    /// - tier is in range 0-3
    /// - coordinates are within 20-bit signed range
    /// - coordinates have valid parity (all even or all odd)
    #[inline(always)]
    pub unsafe fn new_unchecked(tier: u8, x: i32, y: i32, z: i32) -> Self {
        let mut value = 0u64;
        value |= Self::HDR << 62;
        value |= ((tier as u64) & 0x3) << 60;
        value |= ((x as u32 as u64) & 0xFFFFF) << 40;
        value |= ((y as u32 as u64) & 0xFFFFF) << 20;
        value |= (z as u32 as u64) & 0xFFFFF;
        Self { value }
    }
}

/// Sign-extend 20-bit value to 32-bit signed
#[inline]
fn sign_extend_20(val: u32) -> i32 {
    let sign_bit = (val >> 19) & 1;
    if sign_bit == 1 {
        // Negative: extend with 1s
        (val | 0xFFF00000) as i32
    } else {
        // Positive
        val as i32
    }
}

impl fmt::Display for Route64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "R64(t={}, {},{},{})",
            self.scale_tier(),
            self.x(),
            self.y(),
            self.z()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_galactic128_creation() {
        let g = Galactic128::new(0, 5, 1, 10, 3, 2, 4, 6).unwrap();
        assert_eq!(g.frame_id(), 0);
        assert_eq!(g.scale_mant(), 5);
        assert_eq!(g.scale_tier(), 1);
        assert_eq!(g.lod(), 10);
        assert_eq!(g.attr_usr(), 3);
        assert_eq!((g.x(), g.y(), g.z()), (2, 4, 6));
    }

    #[test]
    fn test_galactic128_parity() {
        // Valid even parity
        assert!(Galactic128::new(0, 0, 0, 0, 0, 0, 0, 0).is_ok());
        // Valid odd parity
        assert!(Galactic128::new(0, 0, 0, 0, 0, 1, 1, 1).is_ok());
        // Invalid mixed parity
        assert!(Galactic128::new(0, 0, 0, 0, 0, 0, 1, 0).is_err());
    }

    #[test]
    fn test_index64_morton() {
        let idx = Index64::new(0, 0, 5, 100, 200, 300).unwrap();
        assert_eq!(idx.frame_id(), 0);
        assert_eq!(idx.lod(), 5);
        let (x, y, z) = idx.decode_coords();
        assert_eq!((x, y, z), (100, 200, 300));
    }

    #[test]
    fn test_index64_hierarchy() {
        let parent = Index64::new(0, 0, 5, 8, 8, 8).unwrap();
        let children = parent.children();
        assert_eq!(children.len(), 8);

        // All children should have parent as ancestor
        for child in &children {
            assert_eq!(child.parent().unwrap(), parent);
        }
    }

    #[test]
    fn test_route64_signed() {
        // Positive coordinates
        let r = Route64::new(0, 100, 200, 300).unwrap();
        assert_eq!((r.x(), r.y(), r.z()), (100, 200, 300));

        // Negative coordinates
        let r = Route64::new(0, -100, -200, -300).unwrap();
        assert_eq!((r.x(), r.y(), r.z()), (-100, -200, -300));
    }

    #[test]
    fn test_route64_parity() {
        // Valid even
        assert!(Route64::new(0, 0, 0, 0).is_ok());
        // Valid odd
        assert!(Route64::new(0, 1, 1, 1).is_ok());
        // Invalid
        assert!(Route64::new(0, 0, 1, 0).is_err());
    }

    #[test]
    fn test_bech32m_roundtrip() {
        let g = Galactic128::new(0, 5, 1, 10, 3, 2, 4, 6).unwrap();
        let encoded = g.to_bech32m().unwrap();
        let decoded = Galactic128::from_bech32m(&encoded).unwrap();
        assert_eq!(g, decoded);

        let idx = Index64::new(0, 0, 5, 100, 200, 300).unwrap();
        let encoded = idx.to_bech32m().unwrap();
        let decoded = Index64::from_bech32m(&encoded).unwrap();
        assert_eq!(idx, decoded);

        let r = Route64::new(0, 100, 200, 300).unwrap();
        let encoded = r.to_bech32m().unwrap();
        let decoded = Route64::from_bech32m(&encoded).unwrap();
        assert_eq!(r, decoded);
    }
}
