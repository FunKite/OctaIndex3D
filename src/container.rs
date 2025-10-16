//! Container format for compressed spatial data

use crate::compression::{get_compression, Compression};
use crate::error::{Error, Result};
use crc32fast::Hasher;
use std::io::{Read, Write};

const MAGIC: &[u8; 8] = b"OCTA3D\0\0";
const FORMAT_VERSION: u8 = 1;

/// Frame metadata
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    pub codec_id: u8,
    pub codec_vers: u8,
    pub graph_id: u8,
    pub uncompressed_len: u32,
    pub compressed_len: u32,
    pub crc32c: u32,
}

/// Container writer
pub struct ContainerWriter<W: Write> {
    frames: Vec<(FrameMetadata, Vec<u8>)>,
    compression: Box<dyn Compression>,
    writer: Option<W>,
}

impl<W: Write> ContainerWriter<W> {
    /// Create new container writer with default compression (LZ4)
    pub fn new(writer: W) -> Result<Self> {
        Self::with_compression(writer, Box::new(crate::compression::Lz4Compression))
    }

    /// Create with custom compression
    pub fn with_compression(writer: W, compression: Box<dyn Compression>) -> Result<Self> {
        Ok(Self {
            frames: Vec::new(),
            compression,
            writer: Some(writer),
        })
    }

    /// Write a frame of data
    pub fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        let uncompressed_len = data.len() as u32;

        // Compress data
        let compressed = self.compression.compress(data)?;
        let compressed_len = compressed.len() as u32;

        // Compute CRC32C of compressed data
        let mut hasher = Hasher::new();
        hasher.update(&compressed);
        let crc32c = hasher.finalize();

        // Store metadata and compressed data
        self.frames.push((
            FrameMetadata {
                codec_id: self.compression.codec_id(),
                codec_vers: 0,
                graph_id: 0,
                uncompressed_len,
                compressed_len,
                crc32c,
            },
            compressed,
        ));

        Ok(())
    }

    /// Finish writing and flush headers
    pub fn finish(mut self) -> Result<()> {
        let mut writer = self.writer.take().unwrap();
        let frame_count = self.frames.len() as u32;

        // Write file header (16 bytes)
        writer.write_all(MAGIC)?;
        writer.write_all(&[FORMAT_VERSION])?;
        writer.write_all(&[0])?; // flags
        writer.write_all(&frame_count.to_be_bytes())?;
        writer.write_all(&[0, 0])?; // reserved

        // Write frame headers
        for (meta, _) in &self.frames {
            let mut header = [0u8; 16];
            header[0] = meta.codec_id;
            header[1] = meta.codec_vers;
            header[2] = meta.graph_id;
            header[3] = 0; // pad
            header[4..8].copy_from_slice(&meta.uncompressed_len.to_be_bytes());
            header[8..12].copy_from_slice(&meta.compressed_len.to_be_bytes());
            header[12..16].copy_from_slice(&meta.crc32c.to_be_bytes());
            writer.write_all(&header)?;
        }

        // Write compressed data
        for (_, compressed) in &self.frames {
            writer.write_all(compressed)?;
        }

        Ok(())
    }
}

/// Container reader
pub struct ContainerReader<R: Read> {
    reader: R,
    frame_count: u32,
    current_frame: u32,
    frames: Vec<FrameMetadata>,
}

impl<R: Read> ContainerReader<R> {
    /// Open a container for reading
    pub fn open(mut reader: R) -> Result<Self> {
        // Read file header (16 bytes)
        let mut magic_buf = [0u8; 8];
        reader.read_exact(&mut magic_buf)?;
        if &magic_buf != MAGIC {
            return Err(Error::InvalidFormat("Invalid magic number".to_string()));
        }

        let mut format_version_buf = [0u8; 1];
        reader.read_exact(&mut format_version_buf)?;
        let format_version = format_version_buf[0];
        if format_version != FORMAT_VERSION {
            return Err(Error::InvalidFormat(format!(
                "Unsupported format version: {}",
                format_version
            )));
        }

        let mut flags_buf = [0u8; 1];
        reader.read_exact(&mut flags_buf)?;

        let mut frame_count_buf = [0u8; 4];
        reader.read_exact(&mut frame_count_buf)?;
        let frame_count = u32::from_be_bytes(frame_count_buf);

        let mut reserved_buf = [0u8; 2];
        reader.read_exact(&mut reserved_buf)?;

        // Read frame headers
        let mut frames = Vec::with_capacity(frame_count as usize);
        for _ in 0..frame_count {
            let mut frame_header = [0u8; 16];
            reader.read_exact(&mut frame_header)?;

            frames.push(FrameMetadata {
                codec_id: frame_header[0],
                codec_vers: frame_header[1],
                graph_id: frame_header[2],
                uncompressed_len: u32::from_be_bytes([
                    frame_header[4],
                    frame_header[5],
                    frame_header[6],
                    frame_header[7],
                ]),
                compressed_len: u32::from_be_bytes([
                    frame_header[8],
                    frame_header[9],
                    frame_header[10],
                    frame_header[11],
                ]),
                crc32c: u32::from_be_bytes([
                    frame_header[12],
                    frame_header[13],
                    frame_header[14],
                    frame_header[15],
                ]),
            });
        }

        Ok(Self {
            reader,
            frame_count,
            current_frame: 0,
            frames,
        })
    }

    /// Read next frame
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>> {
        if self.current_frame >= self.frame_count {
            return Ok(None);
        }

        let frame_meta = &self.frames[self.current_frame as usize];

        // Read compressed data
        let mut compressed = vec![0u8; frame_meta.compressed_len as usize];
        self.reader.read_exact(&mut compressed)?;

        // Verify CRC
        let mut hasher = Hasher::new();
        hasher.update(&compressed);
        let computed_crc = hasher.finalize();
        if computed_crc != frame_meta.crc32c {
            return Err(Error::CrcMismatch {
                expected: frame_meta.crc32c,
                actual: computed_crc,
            });
        }

        // Decompress
        let compression = get_compression(frame_meta.codec_id)?;
        let decompressed = compression.decompress(&compressed)?;

        self.current_frame += 1;
        Ok(Some(decompressed))
    }

    /// Get total frame count
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_container_write_read() {
        let data1 = b"Hello, world!".repeat(100);
        let data2 = b"Another frame of data".repeat(50);

        // Write container
        let mut buffer = Vec::new();
        {
            let mut writer = ContainerWriter::new(Cursor::new(&mut buffer)).unwrap();
            writer.write_frame(&data1).unwrap();
            writer.write_frame(&data2).unwrap();
            writer.finish().unwrap();
        }

        // Read container
        let mut reader = ContainerReader::open(Cursor::new(&buffer)).unwrap();
        assert_eq!(reader.frame_count(), 2);

        let frame1 = reader.next_frame().unwrap().unwrap();
        assert_eq!(frame1, data1);

        let frame2 = reader.next_frame().unwrap().unwrap();
        assert_eq!(frame2, data2);

        let frame3 = reader.next_frame().unwrap();
        assert!(frame3.is_none());
    }
}
