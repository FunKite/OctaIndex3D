//! Container v2 - Append-friendly container format
//!
//! Provides streaming container format with:
//! - Append without rewrite
//! - Fast open via footer + TOC
//! - Crash recovery with checkpoints
//! - Optional SHA-256 integrity

use crate::compression::Compression;
use crate::error::{Error, Result};
use crc32fast::Hasher;
use std::io::{Seek, Write};

#[cfg(feature = "container_v2")]
use sha2::{Digest, Sha256};

const MAGIC_V2: &[u8; 8] = b"OCTA3D2\0";
const FORMAT_VERSION_V2: u8 = 2;

/// Stream configuration for Container v2
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Checkpoint every N frames (default: 1000)
    pub checkpoint_frames: usize,
    /// Checkpoint every N bytes (default: 64MB)
    pub checkpoint_bytes: usize,
    /// Enable SHA-256 hashing (default: false)
    pub enable_sha256: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            checkpoint_frames: 1000,
            checkpoint_bytes: 64 * 1024 * 1024,
            enable_sha256: false,
        }
    }
}

/// Container v2 header (32 bytes)
#[derive(Debug, Clone)]
pub struct HeaderV2 {
    /// Container format version (currently 2).
    pub format_version: u8,
    /// Feature flags; bit 0 indicates per-frame SHA-256 hashes are present.
    pub flags: u8,
    /// Unique stream identifier, derived from the creation timestamp.
    pub stream_id: u64,
    /// Byte offset of the first frame (immediately after the 32-byte header).
    pub first_frame_offset: u64,
}

impl HeaderV2 {
    /// Creates a header for a new stream, generating a fresh `stream_id`
    /// and setting the SHA-256 flag if `enable_sha256` is true.
    pub fn new(enable_sha256: bool) -> Self {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};

        let flags = if enable_sha256 { 0x01 } else { 0x00 };
        let stream_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos() as u64;

        Self {
            format_version: FORMAT_VERSION_V2,
            flags,
            stream_id,
            first_frame_offset: 32,
        }
    }

    /// Returns true if frames in this stream carry SHA-256 integrity hashes.
    pub fn has_sha256(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Serializes the header to its fixed 32-byte on-disk representation.
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(MAGIC_V2);
        bytes[8] = self.format_version;
        bytes[9] = self.flags;
        // bytes[10..14] reserved = 0
        bytes[16..24].copy_from_slice(&self.stream_id.to_be_bytes());
        bytes[24..32].copy_from_slice(&self.first_frame_offset.to_be_bytes());
        bytes
    }

    /// Parses a header from its 32-byte on-disk representation.
    ///
    /// Returns [`Error::InvalidFormat`] if the magic number does not match.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        if &bytes[0..8] != MAGIC_V2 {
            return Err(Error::InvalidFormat("Invalid magic number".to_string()));
        }

        Ok(Self {
            format_version: bytes[8],
            flags: bytes[9],
            stream_id: u64::from_be_bytes(
                bytes[16..24]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
            first_frame_offset: u64::from_be_bytes(
                bytes[24..32]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
        })
    }
}

/// TOC entry (32 bytes)
#[derive(Debug, Clone)]
pub struct TocEntry {
    /// Byte offset of the frame within the container.
    pub offset: u64,
    /// Size of the frame payload before compression, in bytes.
    pub uncompressed_len: u32,
    /// Size of the frame payload after compression, in bytes.
    pub compressed_len: u32,
    /// Compression codec identifier used for this frame.
    pub codec: u8,
    /// Graph identifier the frame belongs to.
    pub graph: u8,
    /// Level-of-detail tag for the frame.
    pub lod: u8,
    /// Storage tier tag for the frame.
    pub tier: u8,
    /// Monotonically increasing sequence number of the frame.
    pub seq: u64,
}

impl TocEntry {
    /// Serializes the entry to its fixed 32-byte on-disk representation.
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.offset.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.uncompressed_len.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.compressed_len.to_be_bytes());
        bytes[16] = self.codec;
        bytes[17] = self.graph;
        bytes[18] = self.lod;
        bytes[19] = self.tier;
        bytes[20..28].copy_from_slice(&self.seq.to_be_bytes());
        bytes
    }

    /// Parses an entry from its 32-byte on-disk representation.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            offset: u64::from_be_bytes(
                bytes[0..8]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
            uncompressed_len: u32::from_be_bytes(
                bytes[8..12]
                    .try_into()
                    .expect("slice is guaranteed to be 4 bytes"),
            ),
            compressed_len: u32::from_be_bytes(
                bytes[12..16]
                    .try_into()
                    .expect("slice is guaranteed to be 4 bytes"),
            ),
            codec: bytes[16],
            graph: bytes[17],
            lod: bytes[18],
            tier: bytes[19],
            seq: u64::from_be_bytes(
                bytes[20..28]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
        }
    }
}

/// Footer (32 bytes)
#[derive(Debug, Clone)]
pub struct Footer {
    /// Byte offset where the table of contents begins.
    pub toc_offset: u64,
    /// Length of the table of contents, in bytes.
    pub toc_len: u64,
    /// Number of TOC entries (frames) recorded.
    pub entry_count: u64,
    /// Copy of the header flags, for recovery without re-reading the header.
    pub flags_copy: u64,
}

impl Footer {
    /// Serializes the footer to its fixed 32-byte on-disk representation.
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.toc_offset.to_be_bytes());
        bytes[8..16].copy_from_slice(&self.toc_len.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.entry_count.to_be_bytes());
        bytes[24..32].copy_from_slice(&self.flags_copy.to_be_bytes());
        bytes
    }

    /// Parses a footer from its 32-byte on-disk representation.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            toc_offset: u64::from_be_bytes(
                bytes[0..8]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
            toc_len: u64::from_be_bytes(
                bytes[8..16]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
            entry_count: u64::from_be_bytes(
                bytes[16..24]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
            flags_copy: u64::from_be_bytes(
                bytes[24..32]
                    .try_into()
                    .expect("slice is guaranteed to be 8 bytes"),
            ),
        }
    }
}

/// Container v2 writer
pub struct ContainerWriterV2<W: Write + Seek> {
    writer: W,
    config: StreamConfig,
    header: HeaderV2,
    compression: Box<dyn Compression>,
    toc_entries: Vec<TocEntry>,
    bytes_since_checkpoint: usize,
    next_seq: u64,
}

impl<W: Write + Seek> ContainerWriterV2<W> {
    /// Creates a writer over `writer`, immediately writing the stream header.
    ///
    /// Frames are LZ4-compressed by default; see [`Self::with_compression`].
    pub fn new(mut writer: W, config: StreamConfig) -> Result<Self> {
        let header = HeaderV2::new(config.enable_sha256);

        // Write header
        writer.write_all(&header.to_bytes())?;

        Ok(Self {
            writer,
            config,
            header,
            compression: Box::new(crate::compression::Lz4Compression),
            toc_entries: Vec::new(),
            bytes_since_checkpoint: 0,
            next_seq: 0,
        })
    }

    /// Replaces the default LZ4 codec with a custom [`Compression`] implementation.
    pub fn with_compression(mut self, compression: Box<dyn Compression>) -> Result<Self> {
        self.compression = compression;
        Ok(self)
    }

    /// Appends one frame of data, compressing it and recording a TOC entry.
    ///
    /// A checkpoint (TOC + footer) is flushed automatically once the configured
    /// frame-count or byte thresholds in [`StreamConfig`] are reached.
    pub fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        let uncompressed_len = data.len() as u32;
        let offset = self.writer.stream_position()?;

        // Compute SHA-256 if enabled
        #[cfg(feature = "container_v2")]
        let sha256_hash = if self.header.has_sha256() {
            let mut hasher = Sha256::new();
            hasher.update(data);
            Some(hasher.finalize())
        } else {
            None
        };

        // Compress
        let compressed = self.compression.compress(data)?;
        let compressed_len = compressed.len() as u32;

        // Compute CRC32
        let mut crc_hasher = Hasher::new();
        crc_hasher.update(&compressed);
        let crc32 = crc_hasher.finalize();

        // Write frame header (16 bytes)
        let mut frame_header = [0u8; 16];
        frame_header[0] = self.compression.codec_id();
        frame_header[1] = 0; // codec_vers
        frame_header[2] = 0; // graph_id
        frame_header[3] = 0; // pad
        frame_header[4..8].copy_from_slice(&uncompressed_len.to_be_bytes());
        frame_header[8..12].copy_from_slice(&compressed_len.to_be_bytes());
        frame_header[12..16].copy_from_slice(&crc32.to_be_bytes());
        self.writer.write_all(&frame_header)?;

        // Write compressed data
        self.writer.write_all(&compressed)?;

        // Write SHA-256 if enabled
        #[cfg(feature = "container_v2")]
        if let Some(hash) = sha256_hash {
            self.writer.write_all(&hash)?;
        }

        // Add TOC entry
        self.toc_entries.push(TocEntry {
            offset,
            uncompressed_len,
            compressed_len,
            codec: self.compression.codec_id(),
            graph: 0,
            lod: 0,
            tier: 0,
            seq: self.next_seq,
        });

        self.next_seq += 1;
        self.bytes_since_checkpoint += compressed_len as usize;

        // Check if we should checkpoint
        if self.toc_entries.len() >= self.config.checkpoint_frames
            || self.bytes_since_checkpoint >= self.config.checkpoint_bytes
        {
            self.write_checkpoint()?;
        }

        Ok(())
    }

    fn write_checkpoint(&mut self) -> Result<()> {
        let toc_offset = self.writer.stream_position()?;

        // Write TOC entries
        for entry in &self.toc_entries {
            self.writer.write_all(&entry.to_bytes())?;
        }

        let toc_len = (self.toc_entries.len() * 32) as u64;
        let entry_count = self.toc_entries.len() as u64;

        // Write footer
        let footer = Footer {
            toc_offset,
            toc_len,
            entry_count,
            flags_copy: self.header.flags as u64,
        };
        self.writer.write_all(&footer.to_bytes())?;
        self.writer.flush()?;

        self.bytes_since_checkpoint = 0;
        Ok(())
    }

    /// Finalizes the container, writing the last checkpoint (TOC + footer).
    ///
    /// Must be called for the container to be readable; dropping the writer
    /// without calling `finish` leaves only data up to the last checkpoint.
    pub fn finish(mut self) -> Result<()> {
        // Write final checkpoint
        if !self.toc_entries.is_empty() {
            self.write_checkpoint()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_v2_roundtrip() {
        let header = HeaderV2::new(true);
        assert!(header.has_sha256());

        let bytes = header.to_bytes();
        let header2 = HeaderV2::from_bytes(&bytes).unwrap();

        assert_eq!(header.format_version, header2.format_version);
        assert_eq!(header.flags, header2.flags);
        assert_eq!(header.stream_id, header2.stream_id);
    }

    #[test]
    fn test_toc_entry_roundtrip() {
        let entry = TocEntry {
            offset: 1000,
            uncompressed_len: 500,
            compressed_len: 300,
            codec: 0,
            graph: 0,
            lod: 5,
            tier: 1,
            seq: 42,
        };

        let bytes = entry.to_bytes();
        let entry2 = TocEntry::from_bytes(&bytes);

        assert_eq!(entry.offset, entry2.offset);
        assert_eq!(entry.seq, entry2.seq);
        assert_eq!(entry.lod, entry2.lod);
    }

    #[test]
    fn test_container_v2_write() {
        let mut buffer = Vec::new();
        let config = StreamConfig::default();

        {
            let mut writer = ContainerWriterV2::new(Cursor::new(&mut buffer), config).unwrap();
            writer.write_frame(b"Hello, world!").unwrap();
            writer.write_frame(b"Frame 2").unwrap();
            writer.finish().unwrap();
        }

        // Verify header magic
        assert_eq!(&buffer[0..8], b"OCTA3D2\0");
    }
}
