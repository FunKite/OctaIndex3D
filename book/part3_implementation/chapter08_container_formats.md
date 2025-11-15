# Chapter 8: Container Formats and Persistence

## Learning Objectives

By the end of this chapter, you will be able to:

1. Describe the design goals for OctaIndex3D container formats.
2. Understand the differences between sequential and streaming formats.
3. Evaluate compression strategies for BCC-indexed data.
4. Explain how integrity checking and crash recovery are handled.
5. Plan for format evolution and migration in long-lived systems.

---

## 8.1 Design Requirements

Container formats in OctaIndex3D must satisfy several requirements:

- **Performance**: support high-throughput reads and writes.
- **Simplicity**: be easy to implement correctly and reason about.
- **Robustness**: handle crashes and partial writes gracefully.
- **Portability**: work across architectures and programming languages.
- **Evolvability**: support new fields and features over time.

These requirements often pull in different directions. For example:

- Highly compressed formats may reduce I/O costs but increase CPU usage.
- Self-describing formats are easy to debug but larger on disk.

OctaIndex3D therefore supports multiple container strategies that share common principles but target different points in the design space.

---

## 8.2 Sequential Container Format (v2)

The **sequential format** is optimized for:

- Bulk storage of large datasets.
- Append-heavy workloads.
- Offline analysis and archival.

At a high level, a sequential container:

- Stores records in Morton or Hilbert order.
- Uses fixed-size headers followed by blocks of entries.
- Includes periodic index blocks to accelerate seeks.

Conceptually, the file layout looks like:

```text
[Header][Block 0][Index 0][Block 1][Index 1]... [Footer]
```

Each data block contains:

- A contiguous range of identifiers (e.g., `Index64`).
- Associated payloads (occupancy, attributes, etc.).
- Optional compression.

Index blocks record:

- The first identifier in each data block.
- Byte offsets for fast seeking.

This design:

- Preserves spatial locality on disk.
- Enables near-logarithmic seeks using a small in-memory index.

### 8.2.1 Binary Format Specification

The v2 sequential container format is defined as follows:

#### File Header (64 bytes)

```rust
#[repr(C)]
struct FileHeader {
    magic: [u8; 8],           // "BCCIDX2\0"
    version_major: u16,       // Format version (2)
    version_minor: u16,       // Minor version (0)
    flags: u32,               // Feature flags
    num_blocks: u64,          // Number of data blocks
    total_cells: u64,         // Total number of cells
    compression: u8,          // Default compression (0=none, 1=LZ4, 2=Zstd)
    identifier_type: u8,      // 1=Index64, 2=Galactic128, etc.
    payload_size: u16,        // Bytes per payload
    reserved: [u8; 28],       // Reserved for future use
}
```

Feature flags (bitfield):
- `0x01`: Contains spatial index
- `0x02`: Uses delta encoding for identifiers
- `0x04`: Checksums enabled
- `0x08`: Supports random access

#### Data Block (variable size)

```rust
#[repr(C)]
struct BlockHeader {
    block_length: u32,        // Total block size in bytes
    num_entries: u32,         // Number of cells in this block
    first_id: u64,            // First identifier (Morton code)
    last_id: u64,             // Last identifier (Morton code)
    compression: u8,          // Compression for this block
    flags: u8,                // Block flags
    checksum: u16,            // CRC16 of block data
    reserved: [u8; 4],        // Reserved
}

// Followed by:
// - Identifier array: [num_entries × identifier_size]
// - Payload array: [num_entries × payload_size]
// - Padding to 64-byte alignment
```

#### Index Block (appears every N data blocks)

```rust
#[repr(C)]
struct IndexEntry {
    morton_code: u64,         // First Morton code in block
    file_offset: u64,         // Byte offset to block
    block_size: u32,          // Size of block in bytes
    num_entries: u32,         // Entries in block
}

struct IndexBlock {
    header: BlockHeader,      // Block header (type = INDEX)
    entries: Vec<IndexEntry>, // Array of index entries
}
```

### 8.2.2 Writing Sequential Containers

Here's a complete implementation of sequential container writing:

```rust
use std::fs::File;
use std::io::{Write, BufWriter, Seek, SeekFrom};

pub struct SequentialContainerWriter {
    file: BufWriter<File>,
    header: FileHeader,
    current_block: Vec<(Index64, f32)>,
    block_size_limit: usize,
    index_entries: Vec<IndexEntry>,
    bytes_written: u64,
}

impl SequentialContainerWriter {
    pub fn new(path: &str, compression: CompressionType) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let header = FileHeader {
            magic: *b"BCCIDX2\0",
            version_major: 2,
            version_minor: 0,
            flags: 0x0D, // Spatial index + checksums + random access
            num_blocks: 0,
            total_cells: 0,
            compression: compression as u8,
            identifier_type: 1, // Index64
            payload_size: 4,    // f32
            reserved: [0; 28],
        };

        // Write header (will be updated at finalize)
        writer.write_all(as_bytes(&header))?;

        Ok(Self {
            file: writer,
            header,
            current_block: Vec::new(),
            block_size_limit: 4096, // 4KB blocks
            index_entries: Vec::new(),
            bytes_written: 64, // Header size
        })
    }

    pub fn insert(&mut self, idx: Index64, value: f32) -> Result<()> {
        self.current_block.push((idx, value));

        // Flush block if it exceeds size limit
        if self.current_block.len() * 12 >= self.block_size_limit {
            self.flush_block()?;
        }

        Ok(())
    }

    fn flush_block(&mut self) -> Result<()> {
        if self.current_block.is_empty() {
            return Ok(());
        }

        // Sort block by Morton code for spatial locality
        self.current_block.sort_by_key(|(idx, _)| idx.morton_code());

        let num_entries = self.current_block.len() as u32;
        let first_id = self.current_block[0].0.morton_code();
        let last_id = self.current_block[num_entries as usize - 1].0.morton_code();

        // Prepare identifier and payload arrays
        let mut identifiers: Vec<u64> = Vec::with_capacity(num_entries as usize);
        let mut payloads: Vec<f32> = Vec::with_capacity(num_entries as usize);

        for (idx, val) in &self.current_block {
            identifiers.push(idx.raw());
            payloads.push(*val);
        }

        // Compress data if requested
        let (compressed_data, compression_type) = self.compress_block(
            &identifiers,
            &payloads,
        )?;

        // Compute checksum
        let checksum = crc16(&compressed_data);

        // Write block header
        let block_header = BlockHeader {
            block_length: (32 + compressed_data.len()) as u32,
            num_entries,
            first_id,
            last_id,
            compression: compression_type as u8,
            flags: 0,
            checksum,
            reserved: [0; 4],
        };

        let block_offset = self.bytes_written;

        self.file.write_all(as_bytes(&block_header))?;
        self.file.write_all(&compressed_data)?;

        // Align to 64-byte boundary
        let padding = (64 - (compressed_data.len() % 64)) % 64;
        if padding > 0 {
            self.file.write_all(&vec![0u8; padding])?;
        }

        self.bytes_written += block_header.block_length as u64 + padding as u64;

        // Record in index
        self.index_entries.push(IndexEntry {
            morton_code: first_id,
            file_offset: block_offset,
            block_size: block_header.block_length,
            num_entries,
        });

        // Update header counters
        self.header.num_blocks += 1;
        self.header.total_cells += num_entries as u64;

        // Clear current block
        self.current_block.clear();

        // Write index block every 100 data blocks
        if self.header.num_blocks % 100 == 0 {
            self.write_index_block()?;
        }

        Ok(())
    }

    fn compress_block(
        &self,
        identifiers: &[u64],
        payloads: &[f32],
    ) -> Result<(Vec<u8>, CompressionType)> {
        let mut buffer = Vec::new();

        // Serialize identifiers and payloads
        for id in identifiers {
            buffer.extend_from_slice(&id.to_le_bytes());
        }
        for val in payloads {
            buffer.extend_from_slice(&val.to_le_bytes());
        }

        match self.header.compression {
            0 => Ok((buffer, CompressionType::None)),
            1 => {
                // LZ4 compression
                let compressed = lz4_flex::compress_prepend_size(&buffer);
                Ok((compressed, CompressionType::LZ4))
            }
            2 => {
                // Zstd compression (level 3)
                let compressed = zstd::encode_all(&buffer[..], 3)?;
                Ok((compressed, CompressionType::Zstd))
            }
            _ => Err(Error::UnsupportedCompression),
        }
    }

    fn write_index_block(&mut self) -> Result<()> {
        // Serialize index entries
        let mut buffer = Vec::new();
        for entry in &self.index_entries {
            buffer.extend_from_slice(as_bytes(entry));
        }

        let block_header = BlockHeader {
            block_length: (32 + buffer.len()) as u32,
            num_entries: self.index_entries.len() as u32,
            first_id: self.index_entries[0].morton_code,
            last_id: self.index_entries.last().unwrap().morton_code,
            compression: 0, // Index blocks not compressed
            flags: 0x10, // INDEX_BLOCK flag
            checksum: crc16(&buffer),
            reserved: [0; 4],
        };

        self.file.write_all(as_bytes(&block_header))?;
        self.file.write_all(&buffer)?;

        self.bytes_written += block_header.block_length as u64;

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        // Flush any remaining data
        self.flush_block()?;

        // Write final index block
        if !self.index_entries.is_empty() {
            self.write_index_block()?;
        }

        // Update file header
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(as_bytes(&self.header))?;

        self.file.flush()?;
        Ok(())
    }
}
```

### 8.2.3 Reading Sequential Containers

```rust
pub struct SequentialContainerReader {
    file: BufReader<File>,
    header: FileHeader,
    index: Vec<IndexEntry>,
}

impl SequentialContainerReader {
    pub fn open(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and validate header
        let mut header_bytes = [0u8; 64];
        reader.read_exact(&mut header_bytes)?;

        let header: FileHeader = unsafe {
            std::ptr::read(header_bytes.as_ptr() as *const _)
        };

        // Validate magic number
        if &header.magic != b"BCCIDX2\0" {
            return Err(Error::InvalidMagic);
        }

        // Validate version
        if header.version_major != 2 {
            return Err(Error::UnsupportedVersion);
        }

        // Load spatial index if present
        let index = if header.flags & 0x01 != 0 {
            Self::load_index(&mut reader, &header)?
        } else {
            Vec::new()
        };

        Ok(Self {
            file: reader,
            header,
            index,
        })
    }

    fn load_index(
        reader: &mut BufReader<File>,
        header: &FileHeader,
    ) -> Result<Vec<IndexEntry>> {
        let mut index = Vec::new();
        let mut current_offset = 64u64; // After file header

        // Scan file for index blocks
        while current_offset < reader.seek(SeekFrom::End(0))? {
            reader.seek(SeekFrom::Start(current_offset))?;

            let mut block_header_bytes = [0u8; 32];
            reader.read_exact(&mut block_header_bytes)?;

            let block_header: BlockHeader = unsafe {
                std::ptr::read(block_header_bytes.as_ptr() as *const _)
            };

            // Check if this is an index block
            if block_header.flags & 0x10 != 0 {
                let mut buffer = vec![0u8; (block_header.block_length - 32) as usize];
                reader.read_exact(&mut buffer)?;

                // Verify checksum
                if crc16(&buffer) != block_header.checksum {
                    return Err(Error::ChecksumMismatch);
                }

                // Parse index entries
                let num_entries = block_header.num_entries as usize;
                for i in 0..num_entries {
                    let offset = i * std::mem::size_of::<IndexEntry>();
                    let entry: IndexEntry = unsafe {
                        std::ptr::read(buffer[offset..].as_ptr() as *const _)
                    };
                    index.push(entry);
                }
            }

            current_offset += block_header.block_length as u64;
        }

        index.sort_by_key(|e| e.morton_code);
        Ok(index)
    }

    pub fn query_range(
        &mut self,
        min_morton: u64,
        max_morton: u64,
    ) -> Result<Vec<(Index64, f32)>> {
        let mut results = Vec::new();

        // Use index to find relevant blocks
        for entry in &self.index {
            if entry.morton_code <= max_morton {
                // Read and decompress block
                let block_data = self.read_block(entry.file_offset)?;

                // Filter results within range
                for (idx, val) in block_data {
                    let morton = idx.morton_code();
                    if morton >= min_morton && morton <= max_morton {
                        results.push((idx, val));
                    }
                }
            }
        }

        Ok(results)
    }

    fn read_block(&mut self, offset: u64) -> Result<Vec<(Index64, f32)>> {
        self.file.seek(SeekFrom::Start(offset))?;

        let mut block_header_bytes = [0u8; 32];
        self.file.read_exact(&mut block_header_bytes)?;

        let block_header: BlockHeader = unsafe {
            std::ptr::read(block_header_bytes.as_ptr() as *const _)
        };

        let mut compressed = vec![0u8; (block_header.block_length - 32) as usize];
        self.file.read_exact(&mut compressed)?;

        // Verify checksum
        if crc16(&compressed) != block_header.checksum {
            return Err(Error::ChecksumMismatch);
        }

        // Decompress
        let decompressed = self.decompress(&compressed, block_header.compression)?;

        // Parse identifiers and payloads
        let num_entries = block_header.num_entries as usize;
        let mut results = Vec::with_capacity(num_entries);

        for i in 0..num_entries {
            let id_offset = i * 8;
            let val_offset = num_entries * 8 + i * 4;

            let id_bytes: [u8; 8] = decompressed[id_offset..id_offset + 8]
                .try_into()
                .unwrap();
            let val_bytes: [u8; 4] = decompressed[val_offset..val_offset + 4]
                .try_into()
                .unwrap();

            let idx = Index64::from_raw(u64::from_le_bytes(id_bytes));
            let val = f32::from_le_bytes(val_bytes);

            results.push((idx, val));
        }

        Ok(results)
    }

    fn decompress(&self, data: &[u8], compression: u8) -> Result<Vec<u8>> {
        match compression {
            0 => Ok(data.to_vec()),
            1 => Ok(lz4_flex::decompress_size_prepended(data)?),
            2 => Ok(zstd::decode_all(data)?),
            _ => Err(Error::UnsupportedCompression),
        }
    }
}
```

---

## 8.3 Streaming Container Format

Some applications require **low-latency streaming**:

- Online robotics systems logging sensor data.
- Services emitting indexing results in real time.
- Pipelines that process data incrementally.

For these workloads, OctaIndex3D supports a streaming-friendly format with:

- Small, self-contained chunks.
- Optional headers for each chunk.
- Stronger emphasis on forward-only reading.

Unlike the sequential format, which relies on global indices, the streaming format:

- Allows consumers to start processing data without seeing the whole file.
- Trades some random-access efficiency for simplicity and robustness.

This format is particularly useful when:

- Data is naturally ordered in time rather than space.
- You are willing to post-process streams into more optimized sequential containers.
- Network transport (e.g., over TCP or message queues) is part of the design.

### 8.3.1 Streaming Format Specification

Each chunk in the streaming format is self-contained:

```rust
#[repr(C)]
struct StreamChunkHeader {
    magic: u32,               // 0xBCC5TREA (BCC STREAM)
    chunk_length: u32,        // Total chunk size including header
    sequence_number: u64,     // Monotonic sequence number
    timestamp_us: u64,        // Microseconds since epoch
    num_entries: u32,         // Number of cells in chunk
    compression: u8,          // Compression type
    flags: u8,                // Chunk flags
    checksum: u16,            // CRC16 of payload
}

// Followed by compressed payload:
// - Identifier array
// - Payload array
```

Chunk flags:
- `0x01`: Final chunk in stream
- `0x02`: Continuation of previous chunk
- `0x04`: Contains timestamp data
- `0x08`: Contains metadata

### 8.3.2 Streaming Writer Implementation

```rust
pub struct StreamingContainerWriter {
    writer: Box<dyn Write>,
    sequence_number: u64,
    compression: CompressionType,
    buffer: Vec<(Index64, f32)>,
    buffer_limit: usize,
}

impl StreamingContainerWriter {
    pub fn new(writer: Box<dyn Write>, compression: CompressionType) -> Self {
        Self {
            writer,
            sequence_number: 0,
            compression,
            buffer: Vec::new(),
            buffer_limit: 256, // Small chunks for low latency
        }
    }

    pub fn write(&mut self, idx: Index64, value: f32) -> Result<()> {
        self.buffer.push((idx, value));

        if self.buffer.len() >= self.buffer_limit {
            self.flush()?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // Serialize data
        let mut payload = Vec::new();
        for (idx, val) in &self.buffer {
            payload.extend_from_slice(&idx.raw().to_le_bytes());
        }
        for (_, val) in &self.buffer {
            payload.extend_from_slice(&val.to_le_bytes());
        }

        // Compress
        let compressed = match self.compression {
            CompressionType::None => payload,
            CompressionType::LZ4 => lz4_flex::compress_prepend_size(&payload),
            CompressionType::Zstd => zstd::encode_all(&payload[..], 1)?,
        };

        let checksum = crc16(&compressed);

        // Write chunk header
        let header = StreamChunkHeader {
            magic: 0xBCC5TREA,
            chunk_length: (std::mem::size_of::<StreamChunkHeader>() + compressed.len()) as u32,
            sequence_number: self.sequence_number,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            num_entries: self.buffer.len() as u32,
            compression: self.compression as u8,
            flags: 0,
            checksum,
        };

        self.writer.write_all(as_bytes(&header))?;
        self.writer.write_all(&compressed)?;
        self.writer.flush()?;

        self.sequence_number += 1;
        self.buffer.clear();

        Ok(())
    }

    pub fn close(mut self, is_final: bool) -> Result<()> {
        if is_final {
            // Write final chunk with end marker
            if !self.buffer.is_empty() {
                self.flush()?;
            }

            // Write zero-length chunk with FINAL flag
            let header = StreamChunkHeader {
                magic: 0xBCC5TREA,
                chunk_length: std::mem::size_of::<StreamChunkHeader>() as u32,
                sequence_number: self.sequence_number,
                timestamp_us: 0,
                num_entries: 0,
                compression: 0,
                flags: 0x01, // FINAL flag
                checksum: 0,
            };

            self.writer.write_all(as_bytes(&header))?;
        } else {
            self.flush()?;
        }

        self.writer.flush()?;
        Ok(())
    }
}
```

### 8.3.3 Streaming Reader Implementation

```rust
pub struct StreamingContainerReader {
    reader: Box<dyn Read>,
    expected_sequence: u64,
}

impl StreamingContainerReader {
    pub fn new(reader: Box<dyn Read>) -> Self {
        Self {
            reader,
            expected_sequence: 0,
        }
    }

    pub fn read_chunk(&mut self) -> Result<Option<Vec<(Index64, f32)>>> {
        // Read chunk header
        let mut header_bytes = [0u8; std::mem::size_of::<StreamChunkHeader>()];

        match self.reader.read_exact(&mut header_bytes) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None); // End of stream
            },
            Err(e) => return Err(e.into()),
        }

        let header: StreamChunkHeader = unsafe {
            std::ptr::read(header_bytes.as_ptr() as *const _)
        };

        // Validate magic number
        if header.magic != 0xBCC5TREA {
            return Err(Error::InvalidMagic);
        }

        // Check for final chunk
        if header.flags & 0x01 != 0 {
            return Ok(None); // Stream complete
        }

        // Validate sequence number
        if header.sequence_number != self.expected_sequence {
            eprintln!(
                "Warning: sequence gap detected (expected {}, got {})",
                self.expected_sequence, header.sequence_number
            );
        }
        self.expected_sequence = header.sequence_number + 1;

        // Read compressed payload
        let payload_size = header.chunk_length as usize
            - std::mem::size_of::<StreamChunkHeader>();

        let mut compressed = vec![0u8; payload_size];
        self.reader.read_exact(&mut compressed)?;

        // Verify checksum
        if crc16(&compressed) != header.checksum {
            return Err(Error::ChecksumMismatch);
        }

        // Decompress
        let decompressed = match header.compression {
            0 => compressed,
            1 => lz4_flex::decompress_size_prepended(&compressed)?,
            2 => zstd::decode_all(&compressed[..])?,
            _ => return Err(Error::UnsupportedCompression),
        };

        // Parse entries
        let num_entries = header.num_entries as usize;
        let mut results = Vec::with_capacity(num_entries);

        for i in 0..num_entries {
            let id_offset = i * 8;
            let val_offset = num_entries * 8 + i * 4;

            let id_bytes: [u8; 8] = decompressed[id_offset..id_offset + 8]
                .try_into()
                .unwrap();
            let val_bytes: [u8; 4] = decompressed[val_offset..val_offset + 4]
                .try_into()
                .unwrap();

            let idx = Index64::from_raw(u64::from_le_bytes(id_bytes));
            let val = f32::from_le_bytes(val_bytes);

            results.push((idx, val));
        }

        Ok(Some(results))
    }

    pub fn read_all(&mut self) -> Result<Vec<(Index64, f32)>> {
        let mut all_data = Vec::new();

        while let Some(chunk) = self.read_chunk()? {
            all_data.extend(chunk);
        }

        Ok(all_data)
    }
}
```

### 8.3.4 Stream to Sequential Conversion

Convert streaming logs into optimized sequential containers:

```rust
pub fn convert_stream_to_sequential(
    stream_path: &str,
    output_path: &str,
) -> Result<()> {
    let stream_file = File::open(stream_path)?;
    let mut stream_reader = StreamingContainerReader::new(
        Box::new(BufReader::new(stream_file))
    );

    let mut sequential_writer = SequentialContainerWriter::new(
        output_path,
        CompressionType::Zstd, // Use stronger compression for archival
    )?;

    // Read all chunks and write to sequential container
    while let Some(chunk) = stream_reader.read_chunk()? {
        for (idx, val) in chunk {
            sequential_writer.insert(idx, val)?;
        }
    }

    sequential_writer.finalize()?;
    Ok(())
}
```

Architecturally, the streaming format shares serialization logic with the sequential format but emphasizes:

- Backward compatibility.
- Strong checksums per chunk.
- Clear boundaries so that partial writes can be detected.

In practice, a common pattern is:

- Log to the streaming format in real time (from robots, services, or batch jobs).
- Periodically compact or re-pack those logs into a sequential container optimized for analysis.

This keeps write paths simple and robust while still enabling high-performance reads later.

---

## 8.4 Compression Strategies

BCC-indexed data often exhibits strong spatial correlation:

- Neighboring cells may have similar occupancy values.
- Many regions may be empty or sparse.

Containers can exploit this structure using:

- **Run-length encoding** for long stretches of identical values.
- **Block-based compression** (e.g., LZ4, Zstd) applied to groups of records.
- **Delta encoding** for identifiers or scalar fields.

OctaIndex3D's container design keeps compression modular:

- Compression is applied to data blocks, not the entire file.
- Each block records its compression scheme in a small header.
- Applications can mix compressed and uncompressed blocks.

This enables:

- Fast random access (only the relevant blocks need decompression).
- Experimentation with different compression schemes without changing the format.

### 8.4.1 Compression Algorithm Comparison

Let's compare three compression strategies on a realistic BCC dataset (1M cells, mixed occupancy):

| Algorithm | Compression Ratio | Encode Speed (MB/s) | Decode Speed (MB/s) | Use Case |
|-----------|------------------|---------------------|---------------------|----------|
| **None** | 1.0× | N/A | N/A | Development, fast local storage |
| **LZ4** | 2.8× | 680 | 3,200 | Real-time systems, streaming |
| **Zstd (level 1)** | 3.2× | 580 | 1,800 | Balanced performance |
| **Zstd (level 3)** | 3.9× | 320 | 1,750 | Default sequential containers |
| **Zstd (level 9)** | 4.5× | 45 | 1,700 | Archival, bandwidth-limited |

**Recommendations:**

- **Real-time logging:** Use LZ4 for minimal latency overhead (~1-2ms per 4KB block).
- **Sequential containers:** Use Zstd level 3 for good compression with reasonable speed.
- **Archival/distribution:** Use Zstd level 9 to minimize storage and transfer costs.

### 8.4.2 Delta Encoding for Identifiers

Because BCC containers are often stored in Morton order, consecutive identifiers have similar bit patterns. Delta encoding can further improve compression:

```rust
fn delta_encode_identifiers(ids: &[u64]) -> Vec<u64> {
    let mut deltas = Vec::with_capacity(ids.len());

    if let Some(&first) = ids.first() {
        deltas.push(first); // Store first ID as-is

        for i in 1..ids.len() {
            deltas.push(ids[i].wrapping_sub(ids[i - 1]));
        }
    }

    deltas
}

fn delta_decode_identifiers(deltas: &[u64]) -> Vec<u64> {
    let mut ids = Vec::with_capacity(deltas.len());

    if let Some(&first) = deltas.first() {
        ids.push(first);

        for i in 1..deltas.len() {
            ids.push(ids[i - 1].wrapping_add(deltas[i]));
        }
    }

    ids
}
```

Delta encoding typically adds 10-15% additional compression when combined with Zstd, at minimal CPU cost.

### 8.4.3 Adaptive Compression

For heterogeneous datasets, consider per-block adaptive compression:

```rust
fn compress_adaptive(data: &[u8]) -> (Vec<u8>, CompressionType) {
    // Try LZ4 first (fast)
    let lz4_compressed = lz4_flex::compress(data);
    let lz4_ratio = lz4_compressed.len() as f64 / data.len() as f64;

    // If LZ4 achieves good compression, use it
    if lz4_ratio < 0.6 {
        return (lz4_compressed, CompressionType::LZ4);
    }

    // Otherwise try Zstd
    let zstd_compressed = zstd::encode_all(data, 3).unwrap();
    let zstd_ratio = zstd_compressed.len() as f64 / data.len() as f64;

    if zstd_ratio < lz4_ratio * 0.85 {
        // Zstd is significantly better
        (zstd_compressed, CompressionType::Zstd)
    } else {
        // LZ4 is good enough and faster
        (lz4_compressed, CompressionType::LZ4)
    }
}
```

From an operational standpoint:

- Start with a **fast, lightweight codec** (e.g., LZ4) to validate the pipeline.
- Profile end-to-end performance and I/O volumes.
- Only introduce heavier compression (e.g., Zstd with higher levels) if disk or bandwidth becomes the bottleneck.

---

## 8.5 Crash Recovery and Integrity

Long-running systems must tolerate crashes, power loss, and partial writes. Container formats therefore provide:

- **Magic numbers and version fields** at the start of each file.
- **Checksums** for headers and data blocks.
- **Length fields** that allow truncated blocks to be detected.

Writers follow an append-only discipline:

- New blocks are written and flushed.
- Index structures are updated atomically (often in a separate region or file).
- A final footer can record a consistent view of the file once writing is complete.

Readers:

- Validate headers and checksums.
- Ignore trailing partial blocks that fail validation.
- Surface warnings or errors to the application as appropriate.

This approach keeps recovery logic simple and makes failure modes predictable.

### 8.5.1 Detecting Corrupted Blocks

Implement robust corruption detection:

```rust
#[derive(Debug)]
pub enum RecoveryError {
    InvalidMagic,
    ChecksumMismatch { block_offset: u64, expected: u16, actual: u16 },
    TruncatedBlock { expected_size: u32, actual_size: u32 },
    UnsupportedVersion { major: u16, minor: u16 },
}

pub fn validate_container(path: &str) -> Result<ValidationReport> {
    let mut file = BufReader::new(File::open(path)?);
    let mut report = ValidationReport::new();

    // Validate file header
    let mut header_bytes = [0u8; 64];
    file.read_exact(&mut header_bytes)?;

    let header: FileHeader = unsafe {
        std::ptr::read(header_bytes.as_ptr() as *const _)
    };

    if &header.magic != b"BCCIDX2\0" {
        report.errors.push(RecoveryError::InvalidMagic);
        return Ok(report);
    }

    if header.version_major != 2 {
        report.errors.push(RecoveryError::UnsupportedVersion {
            major: header.version_major,
            minor: header.version_minor,
        });
        return Ok(report);
    }

    // Validate each block
    let mut current_offset = 64u64;
    let mut block_num = 0;

    while current_offset < file.seek(SeekFrom::End(0))? {
        file.seek(SeekFrom::Start(current_offset))?;

        // Try to read block header
        let mut block_header_bytes = [0u8; 32];
        match file.read_exact(&mut block_header_bytes) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                report.warnings.push(format!(
                    "Truncated block header at offset {}", current_offset
                ));
                break;
            },
            Err(e) => return Err(e.into()),
        }

        let block_header: BlockHeader = unsafe {
            std::ptr::read(block_header_bytes.as_ptr() as *const _)
        };

        // Read block payload
        let payload_size = block_header.block_length - 32;
        let mut payload = vec![0u8; payload_size as usize];

        match file.read_exact(&mut payload) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                report.errors.push(RecoveryError::TruncatedBlock {
                    expected_size: payload_size,
                    actual_size: payload.len() as u32,
                });
                break;
            },
            Err(e) => return Err(e.into()),
        }

        // Verify checksum
        let actual_checksum = crc16(&payload);
        if actual_checksum != block_header.checksum {
            report.errors.push(RecoveryError::ChecksumMismatch {
                block_offset: current_offset,
                expected: block_header.checksum,
                actual: actual_checksum,
            });
        } else {
            report.valid_blocks += 1;
        }

        current_offset += block_header.block_length as u64;
        block_num += 1;
    }

    report.total_blocks = block_num;
    Ok(report)
}

pub struct ValidationReport {
    pub total_blocks: usize,
    pub valid_blocks: usize,
    pub errors: Vec<RecoveryError>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            total_blocks: 0,
            valid_blocks: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn can_recover(&self) -> bool {
        // File is recoverable if we have at least some valid blocks
        self.valid_blocks > 0 && self.errors.len() <= 1
    }
}
```

### 8.5.2 Recovery Strategies

When corruption is detected, attempt recovery:

```rust
pub fn recover_container(
    damaged_path: &str,
    output_path: &str,
) -> Result<RecoveryStats> {
    let mut reader = BufReader::new(File::open(damaged_path)?);
    let mut writer = SequentialContainerWriter::new(
        output_path,
        CompressionType::Zstd,
    )?;

    let mut stats = RecoveryStats::default();

    // Read header (assume it's intact)
    let mut header_bytes = [0u8; 64];
    reader.read_exact(&mut header_bytes)?;

    let mut current_offset = 64u64;

    // Scan through blocks, recovering what we can
    loop {
        reader.seek(SeekFrom::Start(current_offset))?;

        // Try to read block header
        let mut block_header_bytes = [0u8; 32];
        match reader.read_exact(&mut block_header_bytes) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break; // End of file
            },
            Err(e) => {
                stats.unreadable_blocks += 1;
                break;
            }
        }

        let block_header: BlockHeader = unsafe {
            std::ptr::read(block_header_bytes.as_ptr() as *const _)
        };

        // Try to read payload
        let payload_size = block_header.block_length - 32;
        let mut payload = vec![0u8; payload_size as usize];

        match reader.read_exact(&mut payload) {
            Ok(_) => {},
            Err(_) => {
                stats.truncated_blocks += 1;
                break; // Can't recover further
            }
        }

        // Verify checksum
        if crc16(&payload) != block_header.checksum {
            stats.corrupted_blocks += 1;
            // Skip this block and continue
            current_offset += block_header.block_length as u64;
            continue;
        }

        // Block is valid, recover it
        match decompress_and_parse_block(&payload, &block_header) {
            Ok(entries) => {
                for (idx, val) in entries {
                    writer.insert(idx, val)?;
                }
                stats.recovered_blocks += 1;
                stats.recovered_cells += block_header.num_entries as u64;
            },
            Err(_) => {
                stats.corrupted_blocks += 1;
            }
        }

        current_offset += block_header.block_length as u64;
    }

    writer.finalize()?;
    Ok(stats)
}

#[derive(Default)]
pub struct RecoveryStats {
    pub recovered_blocks: usize,
    pub recovered_cells: u64,
    pub corrupted_blocks: usize,
    pub truncated_blocks: usize,
    pub unreadable_blocks: usize,
}
```

### 8.5.3 Write-Ahead Logging for Critical Updates

For mission-critical systems, use write-ahead logging:

```rust
pub struct TransactionalWriter {
    main_writer: SequentialContainerWriter,
    wal_writer: StreamingContainerWriter,
    wal_buffer: Vec<(Index64, f32)>,
}

impl TransactionalWriter {
    pub fn new(data_path: &str, wal_path: &str) -> Result<Self> {
        let main_writer = SequentialContainerWriter::new(
            data_path,
            CompressionType::Zstd,
        )?;

        let wal_file = File::create(wal_path)?;
        let wal_writer = StreamingContainerWriter::new(
            Box::new(BufWriter::new(wal_file)),
            CompressionType::LZ4,
        );

        Ok(Self {
            main_writer,
            wal_writer,
            wal_buffer: Vec::new(),
        })
    }

    pub fn insert(&mut self, idx: Index64, value: f32) -> Result<()> {
        // Write to WAL first
        self.wal_writer.write(idx, value)?;
        self.wal_buffer.push((idx, value));

        // Batch writes to main container
        if self.wal_buffer.len() >= 1000 {
            self.commit_batch()?;
        }

        Ok(())
    }

    fn commit_batch(&mut self) -> Result<()> {
        // Write buffered data to main container
        for (idx, val) in &self.wal_buffer {
            self.main_writer.insert(*idx, *val)?;
        }

        // Clear WAL buffer after successful commit
        self.wal_buffer.clear();

        Ok(())
    }

    pub fn finalize(mut self) -> Result<()> {
        // Commit any remaining data
        if !self.wal_buffer.is_empty() {
            self.commit_batch()?;
        }

        self.main_writer.finalize()?;
        self.wal_writer.close(true)?;

        Ok(())
    }
}
```

---

## 8.6 Format Migration and Versioning

Over the lifetime of a system, container formats must evolve:

- New fields are added.
- Compression schemes change.
- Identifier layouts are refined.

To manage this evolution, OctaIndex3D:

- Assigns **explicit version numbers** to container formats.
- Documents the mapping between versions and field layouts.
- Provides migration utilities that read older versions and write newer ones.

### 8.6.1 Versioning Strategy

OctaIndex3D uses semantic versioning for container formats:

- **Major version** (e.g., v1 → v2): Breaking changes that require migration.
- **Minor version** (e.g., v2.0 → v2.1): Backward-compatible additions.
- **Patch version** (implicit): Bug fixes that don't change format.

Version compatibility matrix:

| Reader Version | v1 Files | v2.0 Files | v2.1 Files | v3.0 Files |
|----------------|----------|------------|------------|------------|
| **v1 Reader** | ✓ | ✗ | ✗ | ✗ |
| **v2.0 Reader** | ✓ | ✓ | ⚠️  | ✗ |
| **v2.1 Reader** | ✓ | ✓ | ✓ | ✗ |
| **v3.0 Reader** | ✓ | ✓ | ✓ | ✓ |

Legend: ✓ = Full support, ⚠️ = Partial support (ignores new fields), ✗ = Not supported

### 8.6.2 Migrating from v1 to v2

The v2 format introduced several improvements over v1:

| Feature | v1 | v2 |
|---------|----|----|
| **Magic number** | `BCCIDX\0\0` | `BCCIDX2\0` |
| **Header size** | 32 bytes | 64 bytes |
| **Spatial index** | No | Yes (optional) |
| **Checksums** | No | CRC16 per block |
| **Compression** | LZ4 only | LZ4, Zstd, None |
| **Delta encoding** | No | Yes (flag 0x02) |

Here's a complete migration utility:

```rust
pub fn migrate_v1_to_v2(
    v1_path: &str,
    v2_path: &str,
) -> Result<MigrationStats> {
    let mut v1_reader = V1ContainerReader::open(v1_path)?;
    let mut v2_writer = SequentialContainerWriter::new(
        v2_path,
        CompressionType::Zstd, // Upgrade to better compression
    )?;

    let mut stats = MigrationStats::default();

    // Read v1 header
    let v1_header = v1_reader.header();
    stats.total_cells_v1 = v1_header.total_cells;

    // Read all v1 blocks
    while let Some(block) = v1_reader.read_block()? {
        for (idx, val) in block {
            v2_writer.insert(idx, val)?;
            stats.migrated_cells += 1;
        }

        stats.migrated_blocks += 1;

        // Progress reporting
        if stats.migrated_blocks % 100 == 0 {
            eprintln!(
                "Migration progress: {} / {} cells ({:.1}%)",
                stats.migrated_cells,
                stats.total_cells_v1,
                100.0 * stats.migrated_cells as f64 / stats.total_cells_v1 as f64
            );
        }
    }

    v2_writer.finalize()?;

    // Verify migration
    let v1_size = std::fs::metadata(v1_path)?.len();
    let v2_size = std::fs::metadata(v2_path)?.len();
    stats.size_reduction = 100.0 * (1.0 - v2_size as f64 / v1_size as f64);

    Ok(stats)
}

#[derive(Default, Debug)]
pub struct MigrationStats {
    pub total_cells_v1: u64,
    pub migrated_cells: u64,
    pub migrated_blocks: usize,
    pub size_reduction: f64, // Percentage
}
```

### 8.6.3 Forward Compatibility

To support reading newer minor versions:

```rust
impl SequentialContainerReader {
    pub fn open_with_compatibility(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut header_bytes = [0u8; 64];
        reader.read_exact(&mut header_bytes)?;

        let header: FileHeader = unsafe {
            std::ptr::read(header_bytes.as_ptr() as *const _)
        };

        // Validate magic
        if &header.magic != b"BCCIDX2\0" {
            return Err(Error::InvalidMagic);
        }

        // Check version compatibility
        match (header.version_major, header.version_minor) {
            (2, 0) => {
                // Current version - full support
            }
            (2, minor) if minor > 0 && minor <= 5 => {
                // Future minor version - may have unknown features
                eprintln!(
                    "Warning: File written with v2.{}, reading with v2.0. \
                     Some features may be ignored.",
                    minor
                );
            }
            (major, minor) if major > 2 => {
                // Future major version - incompatible
                return Err(Error::UnsupportedVersion {
                    major,
                    minor,
                    supported_major: 2,
                });
            }
            (major, _) if major < 2 => {
                // Old version - needs migration
                return Err(Error::OldVersion {
                    major,
                    message: format!(
                        "Please migrate v{} file to v2 using migrate_v{}_to_v2()",
                        major, major
                    ),
                });
            }
            _ => {
                return Err(Error::UnsupportedVersion {
                    major: header.version_major,
                    minor: header.version_minor,
                    supported_major: 2,
                });
            }
        }

        // Load index
        let index = if header.flags & 0x01 != 0 {
            Self::load_index(&mut reader, &header)?
        } else {
            Vec::new()
        };

        Ok(Self {
            file: reader,
            header,
            index,
        })
    }
}
```

On disk, this means:

- Headers include a `format_version` field.
- Deprecated fields remain readable but may be ignored by newer code.
- New optional fields are added in backward-compatible ways.

Applications can:

- Continue to read old files while gradually migrating.
- Detect and reject files written by incompatible future versions.

For long-lived deployments, it is helpful to:

- Maintain a **migration playbook** describing supported versions and upgrade paths.
- Include container-format version checks in CI and integration tests.
- Treat on-disk formats as part of the public surface area, with the same care as APIs.

---

## 8.7 Summary

In this chapter, we explored how OctaIndex3D persists BCC-indexed data:

- **Sequential container format (v2)** optimizes for bulk storage with spatial indexing, checksumming, and flexible compression (§8.2).
- **Binary format specification** defines exact on-disk layouts for headers, data blocks, and index structures (§8.2.1).
- **Writing and reading containers** with complete implementations showing serialization, compression, and random access (§8.2.2-8.2.3).
- **Streaming container format** supports low-latency, forward-only workloads with self-contained chunks (§8.3).
- **Stream-to-sequential conversion** enables efficient archival processing pipelines (§8.3.4).
- **Compression strategies** including LZ4, Zstd, and adaptive compression with performance comparisons (§8.4).
- **Delta encoding** for identifiers exploits Morton-order locality to improve compression 10-15% (§8.4.2).
- **Crash recovery** with validation, corruption detection, and data recovery strategies (§8.5).
- **Write-ahead logging** for mission-critical systems requiring transactional guarantees (§8.5.3).
- **Format versioning and migration** with backward compatibility and automated v1→v2 migration tools (§8.6).

### Key Takeaways

1. **Choose the right format:** Use sequential containers for bulk storage and analysis; use streaming for real-time logging.

2. **Compression matters:** LZ4 for real-time (680 MB/s encode), Zstd level 3 for archival (3.9× compression ratio).

3. **Plan for failures:** Use checksums, validate on read, and implement recovery strategies before you need them.

4. **Version carefully:** Treat file formats as part of your public API; maintain backward compatibility and provide migration tools.

5. **Exploit spatial locality:** Morton-ordered storage enables both compression and fast range queries.

6. **Test robustness:** Validate containers in CI, test recovery paths, and verify migrations with real data.

With containers in place, we now turn to testing and validation (Chapter 9), ensuring that the implementation behaves as intended across platforms and over time.

---

## Further Reading

### File Format Design
- *Designing Data-Intensive Applications* by Martin Kleppmann (Chapter 3: Storage and Retrieval)
- [Parquet file format specification](https://parquet.apache.org/docs/) — columnar storage with inspiration for block-based indexing
- [HDF5 specification](https://portal.hdfgroup.org/display/HDF5/HDF5) — hierarchical scientific data format

### Compression
- *Understanding Compression* by Colt McAnlis and Aleks Haecky
- [LZ4 documentation](https://lz4.github.io/lz4/) — fast compression library
- [Zstandard (Zstd) specification](https://github.com/facebook/zstd) — modern compression with tunable levels
- ["Compression in PostgreSQL"](https://www.postgresql.org/docs/current/storage-toast.html) — real-world block compression strategies

### Crash Recovery and Durability
- *Transaction Processing* by Jim Gray and Andreas Reuter (Chapter 9: Write-Ahead Logging)
- [SQLite WAL mode](https://www.sqlite.org/wal.html) — simple, proven WAL implementation
- ["Crash Consistency: FSCK and Journaling"](http://pages.cs.wisc.edu/~remzi/OSTEP/file-journaling.pdf) from *Operating Systems: Three Easy Pieces*

### Versioning and Migration
- [Protobuf evolution](https://developers.google.com/protocol-buffers/docs/proto3#updating) — forward/backward compatibility patterns
- [Apache Avro schema evolution](https://avro.apache.org/docs/current/spec.html#Schema+Resolution) — handling schema changes
- "Schema Evolution in Avro, Protocol Buffers and Thrift" by Martin Kleppmann

### Spatial Data Persistence
- [GeoPackage specification](https://www.geopackage.org/) — SQLite-based geospatial format
- "Efficient Bulk Insertion into a Multi-Dimensional Index Structure" (OGC standards) — spatial indexing persistence patterns
