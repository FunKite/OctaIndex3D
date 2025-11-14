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

## 8.2 Sequential Container Format

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

Architecturally, the streaming format shares serialization logic with the sequential format but emphasizes:

- Backward compatibility.
- Strong checksums per chunk.
- Clear boundaries so that partial writes can be detected.

---

## 8.4 Compression Strategies

BCC-indexed data often exhibits strong spatial correlation:

- Neighboring cells may have similar occupancy values.
- Many regions may be empty or sparse.

Containers can exploit this structure using:

- **Run-length encoding** for long stretches of identical values.
- **Block-based compression** (e.g., LZ4, Zstd) applied to groups of records.
- **Delta encoding** for identifiers or scalar fields.

OctaIndex3D’s container design keeps compression modular:

- Compression is applied to data blocks, not the entire file.
- Each block records its compression scheme in a small header.
- Applications can mix compressed and uncompressed blocks.

This enables:

- Fast random access (only the relevant blocks need decompression).
- Experimentation with different compression schemes without changing the format.

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

On disk, this means:

- Headers include a `format_version` field.
- Deprecated fields remain readable but may be ignored by newer code.
- New optional fields are added in backward-compatible ways.

Applications can:

- Continue to read old files while gradually migrating.
- Detect and reject files written by incompatible future versions.

---

## 8.7 Summary

In this chapter, we explored how OctaIndex3D persists BCC-indexed data:

- The **sequential container format** optimizes for bulk storage and spatial locality on disk.
- The **streaming container format** supports low-latency, forward-only workloads.
- Modular **compression strategies** exploit spatial correlation without sacrificing random access.
- Robust **crash recovery and integrity** mechanisms make failures detectable and recoverable.
- Careful **versioning and migration** practices allow formats to evolve with the library.

With containers in place, we now turn to testing and validation, ensuring that the implementation behaves as intended across platforms and over time.
