//! Morton (Z-order) encoding and decoding for 3D coordinates
//!
//! Implements efficient interleaving of 3D coordinates into a single 64-bit value.
//! Uses BMI2 instructions (pdep/pext) on x86_64 when available, with LUT fallback.

/// Morton encode three 16-bit coordinates into a 48-bit value
#[must_use]
#[inline]
pub fn morton_encode(x: u16, y: u16, z: u16) -> u64 {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if is_x86_feature_detected!("bmi2") {
            return unsafe { morton_encode_bmi2(x, y, z) };
        }
    }
    morton_encode_lut(x, y, z)
}

/// Morton decode a 48-bit value into three 16-bit coordinates
#[must_use]
#[inline]
pub fn morton_decode(morton: u64) -> (u16, u16, u16) {
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    {
        if is_x86_feature_detected!("bmi2") {
            return unsafe { morton_decode_bmi2(morton) };
        }
    }
    morton_decode_lut(morton)
}

// BMI2 implementation (x86_64 only)
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "bmi2")]
unsafe fn morton_encode_bmi2(x: u16, y: u16, z: u16) -> u64 {
    use std::arch::x86_64::_pdep_u64;

    // Spread bits using pdep: every third bit position
    let mx = _pdep_u64(x as u64, 0x9249249249249249); // bits 0,3,6,9,...
    let my = _pdep_u64(y as u64, 0x2492492492492492); // bits 1,4,7,10,...
    let mz = _pdep_u64(z as u64, 0x4924924924924924); // bits 2,5,8,11,...

    mx | my | mz
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
#[target_feature(enable = "bmi2")]
unsafe fn morton_decode_bmi2(morton: u64) -> (u16, u16, u16) {
    use std::arch::x86_64::_pext_u64;

    // Extract bits using pext: every third bit position
    let x = _pext_u64(morton, 0x9249249249249249) as u16;
    let y = _pext_u64(morton, 0x2492492492492492) as u16;
    let z = _pext_u64(morton, 0x4924924924924924) as u16;

    (x, y, z)
}

// LUT-based implementation (fallback and non-x86)
fn morton_encode_lut(x: u16, y: u16, z: u16) -> u64 {
    let mut result = 0u64;

    // Process 8 bits at a time using lookup table
    for i in 0..2 {
        let shift = i * 8;
        let xb = ((x >> shift) & 0xFF) as usize;
        let yb = ((y >> shift) & 0xFF) as usize;
        let zb = ((z >> shift) & 0xFF) as usize;

        result |= MORTON_ENCODE_TABLE[xb] << (shift * 3);
        result |= MORTON_ENCODE_TABLE[yb] << (shift * 3 + 1);
        result |= MORTON_ENCODE_TABLE[zb] << (shift * 3 + 2);
    }

    result
}

fn morton_decode_lut(morton: u64) -> (u16, u16, u16) {
    let mut x = 0u16;
    let mut y = 0u16;
    let mut z = 0u16;

    // Process 24 bits at a time, split into three 8-bit chunks
    // Use byte-specific lookup tables since bit positions shift after >> 8 and >> 16
    for i in 0..2 {
        let shift = i * 24; // Process 24 bits (8 bits per axis)
        let bits = ((morton >> shift) & 0xFFFFFF) as u32;

        // Extract bytes and use appropriate lookup table for each byte position
        let byte0 = (bits & 0xFF) as usize;
        let byte1 = ((bits >> 8) & 0xFF) as usize;
        let byte2 = ((bits >> 16) & 0xFF) as usize;

        // Combine results from three bytes, each contributing 2-3 bits
        // The shift amounts differ per axis because each byte contributes a different number of bits:
        // X: 3 bits from byte0, 3 bits from byte1, 2 bits from byte2 → shifts: 0, 3, 6
        // Y: 3 bits from byte0, 2 bits from byte1, 3 bits from byte2 → shifts: 0, 3, 5
        // Z: 2 bits from byte0, 3 bits from byte1, 3 bits from byte2 → shifts: 0, 2, 5
        let xb = (MORTON_DECODE_X_TABLE_B0[byte0] as u16)
            | ((MORTON_DECODE_X_TABLE_B1[byte1] as u16) << 3)
            | ((MORTON_DECODE_X_TABLE_B2[byte2] as u16) << 6);

        let yb = (MORTON_DECODE_Y_TABLE_B0[byte0] as u16)
            | ((MORTON_DECODE_Y_TABLE_B1[byte1] as u16) << 3)
            | ((MORTON_DECODE_Y_TABLE_B2[byte2] as u16) << 5);

        let zb = (MORTON_DECODE_Z_TABLE_B0[byte0] as u16)
            | ((MORTON_DECODE_Z_TABLE_B1[byte1] as u16) << 2)
            | ((MORTON_DECODE_Z_TABLE_B2[byte2] as u16) << 5);

        x |= xb << (i * 8);
        y |= yb << (i * 8);
        z |= zb << (i * 8);
    }

    (x, y, z)
}

// Helper function for generating decode tables (kept for clarity)
#[allow(dead_code)]
#[inline]
fn extract_every_third(bits: u64, offset: u32) -> u8 {
    let mut result = 0u8;
    for i in 0..8 {
        let bit = (bits >> (offset + i * 3)) & 1;
        result |= (bit as u8) << i;
    }
    result
}

// Morton encode lookup table for 8-bit values
// Each entry spreads 8 bits across every third bit position
const MORTON_ENCODE_TABLE: [u64; 256] = generate_morton_encode_lut();

const fn generate_morton_encode_lut() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        let mut result = 0u64;
        let mut j = 0;
        while j < 8 {
            if (i & (1 << j)) != 0 {
                result |= 1u64 << (j * 3);
            }
            j += 1;
        }
        table[i] = result;
        i += 1;
    }
    table
}

// Morton decode lookup tables for extracting X, Y, Z from interleaved bits
// We need separate tables for each byte position because bit positions shift after >> 8 and >> 16
//
// For byte 0 (bits 0-7):   X at 0,3,6   Y at 1,4,7   Z at 2,5
// For byte 1 (bits 8-15):  X at 1,4,7   Y at 2,5     Z at 0,3,6  (after >> 8)
// For byte 2 (bits 16-23): X at 2,5     Y at 0,3,6   Z at 1,4,7  (after >> 16)

// Byte 0 tables
const MORTON_DECODE_X_TABLE_B0: [u8; 256] = generate_morton_decode_lut(0);
const MORTON_DECODE_Y_TABLE_B0: [u8; 256] = generate_morton_decode_lut(1);
const MORTON_DECODE_Z_TABLE_B0: [u8; 256] = generate_morton_decode_lut(2);

// Byte 1 tables (after >> 8, bit positions are shifted by 1)
const MORTON_DECODE_X_TABLE_B1: [u8; 256] = generate_morton_decode_lut(1);
const MORTON_DECODE_Y_TABLE_B1: [u8; 256] = generate_morton_decode_lut(2);
const MORTON_DECODE_Z_TABLE_B1: [u8; 256] = generate_morton_decode_lut(0);

// Byte 2 tables (after >> 16, bit positions are shifted by 2)
const MORTON_DECODE_X_TABLE_B2: [u8; 256] = generate_morton_decode_lut(2);
const MORTON_DECODE_Y_TABLE_B2: [u8; 256] = generate_morton_decode_lut(0);
const MORTON_DECODE_Z_TABLE_B2: [u8; 256] = generate_morton_decode_lut(1);

const fn generate_morton_decode_lut(offset: u32) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let mut result = 0u8;
        let mut j = 0;
        while j < 3 {
            // Extract up to 3 bits at stride 3
            if (i & (1 << (offset + j * 3))) != 0 {
                result |= 1u8 << j;
            }
            j += 1;
        }
        table[i] = result;
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morton_identity() {
        let coords = [
            (0, 0, 0),
            (1, 2, 3),
            (255, 255, 255),
            (65535, 65535, 65535),
            (12345, 54321, 32145),
        ];

        for (x, y, z) in coords {
            let encoded = morton_encode(x, y, z);
            let (dx, dy, dz) = morton_decode(encoded);
            assert_eq!((x, y, z), (dx, dy, dz), "Morton roundtrip failed");
        }
    }

    #[test]
    fn test_morton_ordering() {
        // Morton order should preserve spatial locality
        let a = morton_encode(0, 0, 0);
        let b = morton_encode(1, 0, 0);
        let c = morton_encode(2, 0, 0);

        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn test_morton_lut() {
        // Test LUT implementation directly
        let (x, y, z) = (12345, 54321, 32145);
        let encoded = morton_encode_lut(x, y, z);
        let (dx, dy, dz) = morton_decode_lut(encoded);
        assert_eq!((x, y, z), (dx, dy, dz));
    }

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    #[test]
    fn test_morton_bmi2() {
        if is_x86_feature_detected!("bmi2") {
            let (x, y, z) = (12345, 54321, 32145);
            let encoded = unsafe { morton_encode_bmi2(x, y, z) };
            let (dx, dy, dz) = unsafe { morton_decode_bmi2(encoded) };
            assert_eq!((x, y, z), (dx, dy, dz));
        }
    }
}
