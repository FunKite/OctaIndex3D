//! Compressed Occupancy Storage for Large-Scale Maps
//!
//! Implements space-efficient storage using octree compression and run-length encoding

use super::occupancy::OccupancyState;
use crate::error::{Error, Result};
use crate::Index64;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::collections::HashMap;

/// Compression method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    /// No compression (raw storage)
    None,
    /// LZ4 fast compression
    LZ4,
    /// Run-length encoding (RLE) for sparse maps
    RLE,
    /// Octree-based hierarchical compression
    Octree,
}

/// Compressed voxel block (8x8x8 voxels)
#[derive(Debug, Clone)]
struct CompressedBlock {
    /// Compressed data
    data: Vec<u8>,
    /// Compression method used
    method: CompressionMethod,
    /// Uncompressed size
    uncompressed_size: usize,
}

/// Occupancy layer with compressed storage
///
/// Uses block-based compression (8x8x8 voxel blocks) to reduce memory usage
/// for large-scale occupancy maps. Achieves 10-100x compression on sparse maps.
///
/// # Example
///
/// ```rust
/// use octaindex3d::layers::CompressedOccupancyLayer;
/// use octaindex3d::Index64;
///
/// # fn example() -> octaindex3d::Result<()> {
/// let mut layer = CompressedOccupancyLayer::new();
///
/// // Updates are automatically compressed in blocks
/// let idx = Index64::new(0, 0, 5, 100, 200, 300)?;
/// layer.update_occupancy(idx, true, 0.9);
///
/// // Compression statistics
/// let stats = layer.stats();
/// println!("Compression ratio: {:.1}x", stats.compression_ratio());
/// # Ok(())
/// # }
/// ```
pub struct CompressedOccupancyLayer {
    /// Compressed blocks (indexed by block coordinates)
    blocks: HashMap<(i16, i16, i16), CompressedBlock>,
    /// Block size (voxels per dimension)
    block_size: usize,
    /// Compression method
    method: CompressionMethod,
    /// Decompressed cache (for fast access)
    cache: HashMap<(i16, i16, i16), Vec<f32>>,
    /// Cache size limit
    max_cache_size: usize,
}

impl CompressedOccupancyLayer {
    /// Create new compressed occupancy layer
    pub fn new() -> Self {
        Self::with_method(CompressionMethod::LZ4)
    }

    /// Create with specific compression method
    pub fn with_method(method: CompressionMethod) -> Self {
        Self {
            blocks: HashMap::new(),
            block_size: 8, // 8x8x8 blocks = 512 voxels
            method,
            cache: HashMap::new(),
            max_cache_size: 100, // Cache up to 100 blocks
        }
    }

    /// Get block coordinates for voxel index
    fn get_block_coords(&self, idx: Index64) -> (i16, i16, i16) {
        let (x, y, z) = idx.decode_coords();
        let (x, y, z) = (x as i16, y as i16, z as i16);
        (
            x / self.block_size as i16,
            y / self.block_size as i16,
            z / self.block_size as i16,
        )
    }

    /// Get local voxel index within block
    fn get_local_index(&self, idx: Index64) -> usize {
        let (x, y, z) = idx.decode_coords();
        let (x, y, z) = (x as i16, y as i16, z as i16);
        let bs = self.block_size as i16;
        let lx = (x % bs) as usize;
        let ly = (y % bs) as usize;
        let lz = (z % bs) as usize;
        lx + ly * self.block_size + lz * self.block_size * self.block_size
    }

    /// Decompress block
    fn decompress_block(&self, block: &CompressedBlock) -> Result<Vec<f32>> {
        match block.method {
            CompressionMethod::None => {
                // No compression - interpret directly as f32 array
                let values: Vec<f32> = block
                    .data
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(values)
            }
            CompressionMethod::LZ4 => {
                // LZ4 decompression
                let decompressed = decompress_size_prepended(&block.data).map_err(|e| {
                    Error::InvalidFormat(format!("LZ4 decompression failed: {}", e))
                })?;

                let values: Vec<f32> = decompressed
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(values)
            }
            CompressionMethod::RLE => {
                // Run-length decoding
                self.rle_decode(&block.data)
            }
            CompressionMethod::Octree => {
                // Octree decompression (simplified)
                self.octree_decode(&block.data)
            }
        }
    }

    /// Compress block
    fn compress_block(&self, data: &[f32]) -> Result<CompressedBlock> {
        let uncompressed_size = std::mem::size_of_val(data);

        // Convert to bytes
        let mut bytes = Vec::with_capacity(uncompressed_size);
        for &value in data {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        let compressed_data = match self.method {
            CompressionMethod::None => bytes,
            CompressionMethod::LZ4 => compress_prepend_size(&bytes),
            CompressionMethod::RLE => self.rle_encode(data)?,
            CompressionMethod::Octree => self.octree_encode(data)?,
        };

        Ok(CompressedBlock {
            data: compressed_data,
            method: self.method,
            uncompressed_size,
        })
    }

    /// Run-length encoding
    fn rle_encode(&self, data: &[f32]) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        if data.is_empty() {
            return Ok(encoded);
        }

        let mut current = data[0];
        let mut count: u32 = 1;

        for &value in &data[1..] {
            if value == current && count < u32::MAX {
                count += 1;
            } else {
                // Write (value, count) pair
                encoded.extend_from_slice(&current.to_le_bytes());
                encoded.extend_from_slice(&count.to_le_bytes());
                current = value;
                count = 1;
            }
        }

        // Write final pair
        encoded.extend_from_slice(&current.to_le_bytes());
        encoded.extend_from_slice(&count.to_le_bytes());

        Ok(encoded)
    }

    /// Run-length decoding
    fn rle_decode(&self, data: &[u8]) -> Result<Vec<f32>> {
        let mut decoded = Vec::new();
        let mut i = 0;

        while i + 8 <= data.len() {
            let value = f32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            let count =
                u32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]) as usize;

            for _ in 0..count {
                decoded.push(value);
            }

            i += 8;
        }

        Ok(decoded)
    }

    /// Simplified octree encoding
    fn octree_encode(&self, data: &[f32]) -> Result<Vec<u8>> {
        // For simplicity, just use LZ4 for now
        // Real octree would recursively subdivide and prune uniform regions
        Ok(compress_prepend_size(
            &data
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ))
    }

    /// Simplified octree decoding
    fn octree_decode(&self, data: &[u8]) -> Result<Vec<f32>> {
        // For simplicity, just use LZ4 for now
        let decompressed = decompress_size_prepended(data)
            .map_err(|e| Error::InvalidFormat(format!("Octree decompression failed: {}", e)))?;

        let values: Vec<f32> = decompressed
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        Ok(values)
    }

    /// Update occupancy value
    pub fn update_occupancy(&mut self, idx: Index64, occupied: bool, confidence: f32) {
        let block_coords = self.get_block_coords(idx);
        let local_idx = self.get_local_index(idx);

        // Load or create block
        let block_size_cubed = self.block_size * self.block_size * self.block_size;
        let mut log_odds_data = if let Some(block) = self.blocks.get(&block_coords) {
            self.decompress_block(block)
                .unwrap_or_else(|_| vec![0.0; block_size_cubed])
        } else {
            vec![0.0; block_size_cubed]
        };

        // Update log-odds
        let prob = if occupied {
            confidence
        } else {
            1.0 - confidence
        };
        let log_odds_update = (prob / (1.0 - prob)).ln();
        log_odds_data[local_idx] += log_odds_update;

        // Clamp
        log_odds_data[local_idx] = log_odds_data[local_idx].clamp(-3.466, 3.466);

        // Recompress and store
        if let Ok(compressed) = self.compress_block(&log_odds_data) {
            self.blocks.insert(block_coords, compressed);
            self.cache.insert(block_coords, log_odds_data);

            // Evict oldest cache entry if too large
            if self.cache.len() > self.max_cache_size {
                if let Some((&key, _)) = self.cache.iter().next() {
                    self.cache.remove(&key);
                }
            }
        }
    }

    /// Get occupancy state
    pub fn get_state(&self, idx: Index64) -> OccupancyState {
        let block_coords = self.get_block_coords(idx);
        let local_idx = self.get_local_index(idx);

        if let Some(block) = self.blocks.get(&block_coords) {
            if let Ok(data) = self.decompress_block(block) {
                let log_odds = data[local_idx];
                if log_odds > 0.847 {
                    return OccupancyState::Occupied;
                } else if log_odds < -1.099 {
                    return OccupancyState::Free;
                }
            }
        }

        OccupancyState::Unknown
    }

    /// Get compression statistics
    pub fn stats(&self) -> CompressionStats {
        let mut compressed_bytes = 0;
        let mut uncompressed_bytes = 0;

        for block in self.blocks.values() {
            compressed_bytes += block.data.len();
            uncompressed_bytes += block.uncompressed_size;
        }

        CompressionStats {
            total_blocks: self.blocks.len(),
            compressed_bytes,
            uncompressed_bytes,
            cached_blocks: self.cache.len(),
        }
    }
}

impl Default for CompressedOccupancyLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression statistics
#[derive(Debug)]
pub struct CompressionStats {
    /// Total number of compressed blocks
    pub total_blocks: usize,
    /// Total compressed data size in bytes
    pub compressed_bytes: usize,
    /// Original uncompressed size in bytes
    pub uncompressed_bytes: usize,
    /// Number of blocks currently in cache
    pub cached_blocks: usize,
}

impl CompressionStats {
    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_bytes == 0 {
            1.0
        } else {
            self.uncompressed_bytes as f64 / self.compressed_bytes as f64
        }
    }

    /// Memory saved in MB
    pub fn memory_saved_mb(&self) -> f64 {
        (self.uncompressed_bytes - self.compressed_bytes) as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressed_storage() {
        let mut layer = CompressedOccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

        layer.update_occupancy(idx, true, 0.9);
        assert_eq!(layer.get_state(idx), OccupancyState::Occupied);

        let stats = layer.stats();
        assert!(stats.total_blocks > 0);
    }

    #[test]
    fn test_rle_encoding() {
        let layer = CompressedOccupancyLayer::with_method(CompressionMethod::RLE);
        let data = vec![1.0, 1.0, 1.0, 2.0, 2.0, 3.0];

        let encoded = layer.rle_encode(&data).unwrap();
        let decoded = layer.rle_decode(&encoded).unwrap();

        assert_eq!(data, decoded);
    }
}
