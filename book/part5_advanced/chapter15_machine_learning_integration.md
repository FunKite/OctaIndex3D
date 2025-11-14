# Chapter 15: Machine Learning Integration

## Learning Objectives

By the end of this chapter, you will be able to:

1. Represent BCC-indexed data in forms suitable for machine learning models.
2. Understand how graph neural networks (GNNs) can operate on BCC-based graphs.
3. Design feature extraction pipelines for 3D point clouds and volumes.
4. Integrate OctaIndex3D with frameworks like PyTorch and TensorFlow.

---

## 15.1 Graph Neural Networks on BCC Lattices

Graph neural networks operate on graphs where:

- Nodes carry feature vectors.
- Edges represent relationships between nodes.

BCC-indexed grids naturally define such graphs:

- Cells become nodes.
- Neighbor relationships define edges.

Compared to cubic grids, BCC-based graphs:

- Have more isotropic connectivity.
- Can represent volumetric fields with fewer nodes.

OctaIndex3D can:

- Export adjacency information for BCC graphs.
- Provide node features derived from scalar or vector fields.

GNN architectures (e.g., message-passing networks) can then:

- Learn representations of spatial fields.
- Support tasks like segmentation, anomaly detection, or forecasting.

In a typical setup:

1. OctaIndex3D containers hold physical fields (for example, occupancy, velocity, or semantic labels) keyed by BCC identifiers.
2. A preprocessing step converts these containers into tensors: one tensor for node features, and one or more sparse tensors or index arrays describing edges.
3. A GNN library (such as PyTorch Geometric or DGL) consumes these tensors, applying message-passing layers over the BCC graph.

Because connectivity and neighbor counts are uniform, model designers can reason about receptive fields and effective resolution more easily than with irregular, ad hoc graphs.

---

### 15.1.1 Building Graphs from Containers

To feed GNNs, you need:

- A node feature matrix.
- An edge list (or adjacency structure).

With OctaIndex3D:

1. Iterate over a container to collect:
   - Identifiers for selected cells (nodes).
   - Per-cell features (scalars, vectors, categorical values).
2. For each node, use neighbor queries to:
   - Enumerate neighbors within a given radius or LOD band.
   - Emit directed or undirected edges as pairs of integer node indices.
3. Convert identifiers to consecutive node indices via a mapping table.

This process yields:

- `X`: a dense tensor of shape `[num_nodes, num_features]`.
- `E`: an edge index tensor (e.g., shape `[2, num_edges]` in PyTorch Geometric).

Because BCC neighbors are consistent, you can:

- Control the effective receptive field by stacking GNN layers.
- Reason about how many LODs and neighbor rings a model “sees”.

### 15.1.2 Spatial Attention on BCC Graphs

Attention mechanisms (as in transformers or graph attention networks) require:

- Well-defined neighborhoods.
- Relative position encodings.

On BCC graphs, relative positions can be:

- Derived from lattice coordinates associated with each identifier.
- Encoded as small integer offsets or learned embeddings.

Typical patterns:

- **Graph attention networks (GATs)** where attention weights depend on:
  - Feature similarity between neighboring cells.
  - Encoded relative offsets (e.g., “neighbor in +x direction”).
- **Transformer-style blocks** on local BCC patches:
  - Use BCC cells as tokens.
  - Add positional encodings based on lattice coordinates and LOD.

This lets models learn anisotropic behavior when appropriate, while starting from an isotropic underlying graph.

## 15.2 Point Clouds and Feature Extraction

Many machine learning tasks involve point clouds:

- LiDAR scans.
- 3D reconstructions from multi-view images.

OctaIndex3D can:

- Bin points into BCC cells.
- Aggregate features (e.g., mean intensity, point density) per cell.

These aggregated features:

- Form inputs to downstream models (CNNs, GNNs, transformers).
- Reduce raw data volume while preserving structure.

Feature extraction pipelines typically:

- Use frames to align point clouds in a common CRS.
- Construct containers keyed by BCC identifiers.
- Export tensors or arrays compatible with ML frameworks.

For example, a LiDAR perception stack might:

1. Use frames to transform each scan into a consistent vehicle-centric or world-centric coordinate system.
2. Assign returns to BCC cells at one or more LODs, maintaining per-cell aggregates such as:
   - Count of points.
   - Mean and variance of intensity.
   - Local surface normals estimated from neighboring cells.
3. Export these aggregates as dense or sparse tensors to feed into a 3D CNN, transformer, or GNN.

Because the binning step is deterministic and reversible, labels produced by downstream models can be mapped back to raw point clouds for visualization and debugging.

---

### 15.2.1 Voxelization Schemes

Point clouds can be voxelized onto BCC lattices in several ways:

- **Static voxelization**:
  - Choose a fixed LOD and spatial extent.
  - Bin all points into that grid.
  - Use it for batch training or evaluation.
- **Dynamic voxelization**:
  - Center grids around regions of interest (e.g., around a vehicle).
  - Rebuild or update grids per frame.

OctaIndex3D helps by:

- Providing frame-aware coordinate transforms.
- Offering consistent binning across frames and sensors.

Design choices include:

- Whether to keep empty cells (dense tensors) or omit them (sparse tensors).
- How to normalize features (per-cell counts, log counts, min–max scaling).

### 15.2.2 Multi-LOD Features

Multi-resolution representations often improve robustness:

- Coarse LOD captures context.
- Fine LOD captures detail.

With BCC containers:

- Maintain separate containers for multiple LODs.
- Extract features from each and concatenate:
  - Coarse features: aggregated statistics over larger cells.
  - Fine features: local detail around objects or regions.

Models can then:

- Attend to coarse context while focusing on fine details when necessary.
- Generalize across sensor resolutions by relying on shared structure across LODs.

## 15.3 3D Object Detection and Trajectory Prediction

In domains like autonomous driving:

- Models must detect objects and predict their trajectories in 3D.

BCC-indexed data supports:

- Occupancy-based representations (free vs. occupied space).
- Multi-scale features that capture context at different resolutions.

Trajectories can be:

- Quantized to BCC cells for coarse prediction.
- Refined later in continuous space using regression models.

OctaIndex3D helps by:

- Providing efficient queries for neighborhood features.
- Supporting batch extraction of input tensors.

One design pattern is to use OctaIndex3D primarily as a **feature engine**:

1. For each timestep, construct BCC-indexed containers representing occupancy, semantics, or other signals in the scene.
2. For each object (vehicle, pedestrian, drone), query a fixed-radius neighborhood around its current position and aggregate those features into a fixed-size vector.
3. Feed sequences of such vectors into a recurrent or transformer-based model that predicts future motion.

Here, BCC indexing ensures that the notion of “local neighborhood” is isotropic and resolution-aware, which improves the stability of learned models across different environments and sensor configurations.

---

### 15.3.1 Label Projection and Consistency

Supervised learning requires labels that align with inputs. With BCC-indexed inputs:

- 3D bounding boxes, instance masks, or semantic labels:
  - Can be projected into the same frame as the BCC lattice.
  - Can be converted to sets of BCC cells (e.g., “all cells intersecting this box”).

This enables:

- Cell-level labels (classification per cell).
- Object-level labels with cell-level support (e.g., instance IDs assigned to cells).

Maintaining consistency:

- Use the same frame registry for both labels and data.
- Store label information in containers keyed by the same identifiers as features.

### 15.3.2 Training Data Generation Pipelines

Training pipelines often:

- Start from raw logs (sensor data, simulation outputs).
- Produce curated datasets for ML frameworks.

An OctaIndex3D-centric pipeline might:

1. Ingest raw data and labels into BCC containers using frames and identifiers.
2. Run feature extraction passes to compute per-cell and per-object features.
3. Snapshot containers and export:
   - Features as tensors or Arrow/Parquet tables.
   - Labels as aligned tensors or columns using the same identifiers.
4. Use lightweight Python loaders that:
   - Read exported data.
   - Construct batches for training.

Because identifiers are stable, you can:

- Recompute features with new algorithms while preserving label alignment.
- Merge additional modalities or annotations into existing datasets.

## 15.4 Framework Integration

While OctaIndex3D is implemented in Rust, many ML workflows use:

- Python with PyTorch or TensorFlow.

Integration patterns include:

- **FFI bindings** that expose core operations to Python.
- **Arrow-based interchange** for zero-copy data sharing.
- Exporting containers as NumPy arrays or PyTorch tensors.

Design considerations:

- Keep boundary surfaces small; do heavy computation in Rust where possible.
- Ensure that identifier and frame semantics are preserved across the boundary.

In practice, many teams adopt a layered architecture:

- Use Rust and OctaIndex3D for performance-critical indexing, neighbor search, and aggregation.
- Expose compact, well-documented FFI bindings that operate on Arrow arrays or raw tensors.
- Keep Python code focused on experiment orchestration, model definition, and training loops.

This division of labor respects the strengths of each ecosystem while keeping BCC-specific complexity contained in one place.

---

### 15.4.1 PyTorch-Oriented Workflow

In a typical PyTorch setup:

1. Rust/OctaIndex3D code:
   - Builds containers from raw data.
   - Exports Arrow arrays or flat buffers of features and identifiers.
2. A thin binding layer:
   - Converts Arrow arrays or buffers into `torch.Tensor` objects.
   - Handles device placement (CPU/GPU).
3. PyTorch models:
   - Consume tensors as usual.
   - Treat identifiers either as:
     - Implicit (ordering in tensors encodes position).
     - Explicit (separate tensor of indices or lattice coordinates).

This keeps:

- Heavy numeric work (neighbor queries, aggregation) in Rust.
- Model experimentation and training loops in Python.

### 15.4.2 Serving and Online Inference

For online inference (production serving):

- Rust-based services using OctaIndex3D:
  - Maintain live containers keyed by BCC indices.
  - Extract features for the current request (e.g., around a vehicle or region).
- A model runtime:
  - Receives feature tensors over FFI or RPC.
  - Produces predictions (e.g., occupancy probabilities, trajectories).

Because both training and serving use the same indexing and feature extraction logic:

- Training/serving skew is reduced.
- Debugging mispredictions is easier (you can inspect the exact BCC cells used).

---

## 15.5 Data Pipelines and Training

Machine learning projects succeed or fail on their **data pipelines** at least as much as on model architectures. OctaIndex3D supports robust pipelines by:

- Providing a stable spatial index across experiments.
- Making it cheap to recompute features or add new ones.

### 15.5.1 Offline Training Pipelines

An offline pipeline might:

1. Periodically run feature extraction jobs over large BCC containers.
2. Export features and labels as:
   - Parquet files partitioned by time, region, or LOD.
   - Arrow streams for direct ingestion by training clusters.
3. Use distributed training frameworks (PyTorch DDP, Horovod, etc.) to:
   - Read partitions in parallel.
   - Train models on shared schemas.

Because identifiers are unchanged across runs:

- You can add new labels or features without reindexing.
- Experiments remain comparable even as feature sets evolve.

### 15.5.2 Online Learning and Feedback

Some systems incorporate:

- Online learning.
- Active learning and human-in-the-loop annotation.

OctaIndex3D helps by:

- Allowing you to log:
   - The identifiers of cells involved in each prediction.
   - The features used.
   - The model outputs and eventual outcomes (labels).
- Providing an easy path to:
   - Pull those logged identifiers.
   - Reconstruct their feature vectors from historical containers.

This supports:

- Targeted retraining on difficult regions or edge cases.
- Spatial analyses of where models perform poorly.

## 15.6 Summary

In this chapter, we saw how OctaIndex3D connects with machine learning:

- **GNNs** operate on BCC-based graphs.
- **Point cloud processing** uses BCC binning for feature extraction.
- **3D object detection and trajectory prediction** benefit from multi-scale BCC representations.
- Integration with **ML frameworks** leverages FFI, Arrow, and tensor exports.
- Robust **data pipelines and training workflows** build on stable BCC identifiers and containers.

The final chapter looks ahead to future research directions and potential evolutions of the OctaIndex3D ecosystem.
