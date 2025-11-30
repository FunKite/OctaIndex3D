//! Compression layer for OctaIndex3D container format
//!
//! Provides pluggable compression with LZ4 (default) and optional Zstd support.

use crate::error::{Error, Result};

/// LZ4 compression codec ID
pub const CODEC_LZ4: u8 = 0;
/// Zstandard compression codec ID
pub const CODEC_ZSTD: u8 = 1;
/// No compression codec ID
pub const CODEC_NONE: u8 = 3;

/// Compression trait
pub trait Compression: Send + Sync {
    /// Get codec ID
    fn codec_id(&self) -> u8;

    /// Compress data
    fn compress(&self, src: &[u8]) -> Result<Vec<u8>>;

    /// Decompress data
    fn decompress(&self, src: &[u8]) -> Result<Vec<u8>>;
}

/// LZ4 compression (always available)
#[derive(Debug, Clone, Copy)]
pub struct Lz4Compression;

impl Compression for Lz4Compression {
    fn codec_id(&self) -> u8 {
        CODEC_LZ4
    }

    fn compress(&self, src: &[u8]) -> Result<Vec<u8>> {
        Ok(lz4_flex::compress_prepend_size(src))
    }

    fn decompress(&self, src: &[u8]) -> Result<Vec<u8>> {
        lz4_flex::decompress_size_prepended(src)
            .map_err(|e| Error::Codec(format!("LZ4 decompression failed: {}", e)))
    }
}

/// Zstd compression (optional, requires 'zstd' feature)
#[cfg(feature = "zstd")]
#[derive(Debug, Clone, Copy)]
pub struct ZstdCompression {
    level: i32,
}

#[cfg(feature = "zstd")]
impl ZstdCompression {
    /// Create with default level (5)
    pub fn new() -> Self {
        Self { level: 5 }
    }

    /// Create with custom level (1-22)
    pub fn with_level(level: i32) -> Self {
        Self { level }
    }
}

#[cfg(feature = "zstd")]
impl Default for ZstdCompression {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "zstd")]
impl Compression for ZstdCompression {
    fn codec_id(&self) -> u8 {
        CODEC_ZSTD
    }

    fn compress(&self, src: &[u8]) -> Result<Vec<u8>> {
        zstd::bulk::compress(src, self.level)
            .map_err(|e| Error::Codec(format!("Zstd compression failed: {}", e)))
    }

    fn decompress(&self, src: &[u8]) -> Result<Vec<u8>> {
        // Use zstd::stream::decode_all which automatically handles buffer sizing
        // by reading the decompressed size from the frame header
        zstd::stream::decode_all(src)
            .map_err(|e| Error::Codec(format!("Zstd decompression failed: {}", e)))
    }
}

/// No compression (passthrough)
#[derive(Debug, Clone, Copy)]
pub struct NoCompression;

impl Compression for NoCompression {
    fn codec_id(&self) -> u8 {
        CODEC_NONE
    }

    fn compress(&self, src: &[u8]) -> Result<Vec<u8>> {
        Ok(src.to_vec())
    }

    fn decompress(&self, src: &[u8]) -> Result<Vec<u8>> {
        Ok(src.to_vec())
    }
}

/// Get default compression (LZ4)
pub fn default_compression() -> Box<dyn Compression> {
    Box::new(Lz4Compression)
}

/// Get compression by codec ID
pub fn get_compression(codec_id: u8) -> Result<Box<dyn Compression>> {
    match codec_id {
        CODEC_LZ4 => Ok(Box::new(Lz4Compression)),
        #[cfg(feature = "zstd")]
        CODEC_ZSTD => Ok(Box::new(ZstdCompression::new())),
        CODEC_NONE => Ok(Box::new(NoCompression)),
        _ => Err(Error::UnsupportedCodec(codec_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_roundtrip() {
        let codec = Lz4Compression;
        let data = b"Hello, world! This is a test of LZ4 compression.".repeat(100);

        let compressed = codec.compress(&data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = codec.decompress(&compressed).unwrap();
        assert_eq!(data, decompressed.as_slice());
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_roundtrip() {
        let codec = ZstdCompression::new();
        let data = b"Hello, world! This is a test of Zstd compression.".repeat(100);

        let compressed = codec.compress(&data).unwrap();
        assert!(compressed.len() < data.len());

        let decompressed = codec.decompress(&compressed).unwrap();
        assert_eq!(data, decompressed.as_slice());
    }

    #[test]
    fn test_no_compression() {
        let codec = NoCompression;
        let data = b"Hello, world!";

        let compressed = codec.compress(data).unwrap();
        assert_eq!(data, compressed.as_slice());

        let decompressed = codec.decompress(&compressed).unwrap();
        assert_eq!(data, decompressed.as_slice());
    }
}
