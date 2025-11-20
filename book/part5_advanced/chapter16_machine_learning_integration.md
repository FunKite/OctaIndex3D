# Chapter 16: Machine Learning Integration

## Learning Objectives

By the end of this chapter, you will be able to:

1. Represent BCC-indexed data in forms suitable for machine learning models.
2. Understand how graph neural networks (GNNs) can operate on BCC-based graphs.
3. Design feature extraction pipelines for 3D point clouds and volumes.
4. Integrate OctaIndex3D with frameworks like PyTorch and TensorFlow.

---

## 16.1 Graph Neural Networks on BCC Lattices

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

### 16.1.1 Building Graphs from Containers

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

### 16.1.2 Spatial Attention on BCC Graphs

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

## 16.2 Point Clouds and Feature Extraction

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

### 16.2.1 Voxelization Schemes

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

### 16.2.2 Multi-LOD Features

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

## 16.3 3D Object Detection and Trajectory Prediction

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

### 16.3.1 Label Projection and Consistency

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

### 16.3.2 Training Data Generation Pipelines

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

## 16.4 Framework Integration

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

### 16.4.1 PyTorch-Oriented Workflow

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

### 16.4.2 Serving and Online Inference

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

## 16.5 Data Pipelines and Training

Machine learning projects succeed or fail on their **data pipelines** at least as much as on model architectures. OctaIndex3D supports robust pipelines by:

- Providing a stable spatial index across experiments.
- Making it cheap to recompute features or add new ones.

### 16.5.1 Offline Training Pipelines

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

### 16.5.2 Online Learning and Feedback

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

---

## 16.6 PyTorch Integration in Depth

This section provides complete, production-ready examples of integrating OctaIndex3D with PyTorch.

### 16.6.1 Custom Dataset Class for BCC Containers

A PyTorch `Dataset` that reads from BCC-indexed containers:

```python
import torch
from torch.utils.data import Dataset
import numpy as np
from typing import Dict, List, Tuple
import pyoctaindex  # FFI bindings to OctaIndex3D

class BCCDataset(Dataset):
    """PyTorch Dataset for BCC-indexed 3D data."""

    def __init__(
        self,
        container_path: str,
        frame_id: str,
        lod: int,
        feature_keys: List[str],
        transform=None,
    ):
        """
        Args:
            container_path: Path to sequential container file
            frame_id: Frame identifier for coordinate system
            lod: Level of detail to use
            feature_keys: List of feature names to extract
            transform: Optional transform to apply to samples
        """
        self.container = pyoctaindex.SequentialContainer.open(container_path)
        self.frame_id = frame_id
        self.lod = lod
        self.feature_keys = feature_keys
        self.transform = transform

        # Build index of all cells
        self.cell_indices = []
        for idx, features in self.container.iter_at_lod(lod):
            self.cell_indices.append(idx)

    def __len__(self) -> int:
        return len(self.cell_indices)

    def __getitem__(self, idx: int) -> Dict[str, torch.Tensor]:
        """
        Returns a dictionary containing:
        - 'features': Tensor of shape [num_features]
        - 'position': Tensor of shape [3] (x, y, z)
        - 'neighbors': Tensor of shape [14, num_features] (14 BCC neighbors)
        - 'index': BCC index (for debugging/visualization)
        """
        cell_idx = self.cell_indices[idx]

        # Get cell features
        features = self.container.get_features(cell_idx, self.feature_keys)
        feature_tensor = torch.tensor(features, dtype=torch.float32)

        # Get cell position in world coordinates
        position = self.container.index_to_position(cell_idx, self.frame_id)
        position_tensor = torch.tensor(position, dtype=torch.float32)

        # Get neighbor features
        neighbors = self.container.get_neighbors(cell_idx, radius=1)
        neighbor_features = []
        for neighbor_idx in neighbors:
            nf = self.container.get_features(neighbor_idx, self.feature_keys)
            neighbor_features.append(nf)

        # Pad to exactly 14 neighbors
        while len(neighbor_features) < 14:
            neighbor_features.append(np.zeros(len(self.feature_keys)))

        neighbor_tensor = torch.tensor(
            np.array(neighbor_features[:14]),
            dtype=torch.float32
        )

        sample = {
            'features': feature_tensor,
            'position': position_tensor,
            'neighbors': neighbor_tensor,
            'index': cell_idx,
        }

        if self.transform:
            sample = self.transform(sample)

        return sample
```rust

### 16.6.2 Custom Collation Function

Batch BCC data efficiently:

```python
def bcc_collate_fn(batch: List[Dict[str, torch.Tensor]]) -> Dict[str, torch.Tensor]:
    """
    Collates a batch of BCC samples.

    Returns batched tensors with proper padding and masking.
    """
    batch_size = len(batch)

    # Stack features
    features = torch.stack([item['features'] for item in batch])
    positions = torch.stack([item['position'] for item in batch])
    neighbors = torch.stack([item['neighbors'] for item in batch])

    # Collect indices (useful for visualization/debugging)
    indices = [item['index'] for item in batch]

    return {
        'features': features,          # [B, num_features]
        'position': positions,          # [B, 3]
        'neighbors': neighbors,         # [B, 14, num_features]
        'indices': indices,             # List of length B
    }
```

### 16.6.3 DataLoader with BCC Containers

```python
from torch.utils.data import DataLoader

def create_bcc_dataloader(
    container_path: str,
    frame_id: str,
    lod: int,
    feature_keys: List[str],
    batch_size: int = 32,
    num_workers: int = 4,
    shuffle: bool = True,
) -> DataLoader:
    """
    Create a DataLoader for BCC-indexed data.
    """
    dataset = BCCDataset(
        container_path=container_path,
        frame_id=frame_id,
        lod=lod,
        feature_keys=feature_keys,
    )

    dataloader = DataLoader(
        dataset,
        batch_size=batch_size,
        shuffle=shuffle,
        num_workers=num_workers,
        collate_fn=bcc_collate_fn,
        pin_memory=True,  # For GPU transfer
        persistent_workers=True,  # Keep workers alive
    )

    return dataloader
```rust

### 16.6.4 GPU Data Transfer Optimizations

Minimize CPU-GPU transfer overhead:

```python
class BCCDataModule:
    """Manages efficient data loading and GPU transfer."""

    def __init__(
        self,
        train_container: str,
        val_container: str,
        frame_id: str,
        lod: int,
        feature_keys: List[str],
        batch_size: int = 32,
        device: str = 'cuda',
    ):
        self.device = device

        # Create dataloaders
        self.train_loader = create_bcc_dataloader(
            train_container, frame_id, lod, feature_keys,
            batch_size=batch_size, shuffle=True
        )

        self.val_loader = create_bcc_dataloader(
            val_container, frame_id, lod, feature_keys,
            batch_size=batch_size, shuffle=False
        )

        # Pre-allocate GPU buffers for reuse
        self.gpu_cache = {}

    def transfer_to_device(
        self,
        batch: Dict[str, torch.Tensor],
        non_blocking: bool = True,
    ) -> Dict[str, torch.Tensor]:
        """
        Transfer batch to GPU with optimal settings.
        """
        return {
            k: v.to(self.device, non_blocking=non_blocking)
            if isinstance(v, torch.Tensor) else v
            for k, v in batch.items()
        }
```

---

## 16.7 Complete Training Pipeline

### 16.7.1 Training Loop with Progress Tracking

```python
import torch
import torch.nn as nn
from torch.optim import AdamW
from torch.optim.lr_scheduler import CosineAnnealingLR
from tqdm import tqdm
import wandb  # For experiment tracking

class BCCTrainer:
    """Complete training harness for BCC-based models."""

    def __init__(
        self,
        model: nn.Module,
        train_loader: DataLoader,
        val_loader: DataLoader,
        device: str = 'cuda',
        learning_rate: float = 1e-3,
        weight_decay: float = 1e-4,
        max_epochs: int = 100,
        checkpoint_dir: str = './checkpoints',
    ):
        self.model = model.to(device)
        self.device = device
        self.train_loader = train_loader
        self.val_loader = val_loader
        self.max_epochs = max_epochs
        self.checkpoint_dir = checkpoint_dir

        # Optimizer and scheduler
        self.optimizer = AdamW(
            model.parameters(),
            lr=learning_rate,
            weight_decay=weight_decay,
        )

        self.scheduler = CosineAnnealingLR(
            self.optimizer,
            T_max=max_epochs,
        )

        # Loss function
        self.criterion = nn.MSELoss()

        # Tracking
        self.best_val_loss = float('inf')
        self.current_epoch = 0

    def train_epoch(self) -> Dict[str, float]:
        """Train for one epoch."""
        self.model.train()
        total_loss = 0.0
        num_batches = 0

        pbar = tqdm(self.train_loader, desc=f'Epoch {self.current_epoch}')
        for batch in pbar:
            # Transfer to GPU
            features = batch['features'].to(self.device)
            neighbors = batch['neighbors'].to(self.device)

            # Forward pass
            self.optimizer.zero_grad()
            predictions = self.model(features, neighbors)

            # Compute loss (example: predict next-timestep occupancy)
            targets = features  # Simplified example
            loss = self.criterion(predictions, targets)

            # Backward pass
            loss.backward()
            torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
            self.optimizer.step()

            # Track metrics
            total_loss += loss.item()
            num_batches += 1
            pbar.set_postfix({'loss': total_loss / num_batches})

        return {
            'train_loss': total_loss / num_batches,
        }

    def validate(self) -> Dict[str, float]:
        """Validate on validation set."""
        self.model.eval()
        total_loss = 0.0
        num_batches = 0

        with torch.no_grad():
            for batch in tqdm(self.val_loader, desc='Validation'):
                features = batch['features'].to(self.device)
                neighbors = batch['neighbors'].to(self.device)

                predictions = self.model(features, neighbors)
                targets = features
                loss = self.criterion(predictions, targets)

                total_loss += loss.item()
                num_batches += 1

        return {
            'val_loss': total_loss / num_batches,
        }

    def save_checkpoint(self, metrics: Dict[str, float], is_best: bool = False):
        """Save model checkpoint."""
        import os
        os.makedirs(self.checkpoint_dir, exist_ok=True)

        checkpoint = {
            'epoch': self.current_epoch,
            'model_state_dict': self.model.state_dict(),
            'optimizer_state_dict': self.optimizer.state_dict(),
            'scheduler_state_dict': self.scheduler.state_dict(),
            'metrics': metrics,
        }

        # Save regular checkpoint
        path = f'{self.checkpoint_dir}/checkpoint_epoch_{self.current_epoch}.pt'
        torch.save(checkpoint, path)

        # Save best checkpoint
        if is_best:
            best_path = f'{self.checkpoint_dir}/best_model.pt'
            torch.save(checkpoint, best_path)
            print(f'Saved best model with val_loss={metrics["val_loss"]:.4f}')

    def load_checkpoint(self, path: str):
        """Load checkpoint and resume training."""
        checkpoint = torch.load(path, map_location=self.device)
        self.model.load_state_dict(checkpoint['model_state_dict'])
        self.optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
        self.scheduler.load_state_dict(checkpoint['scheduler_state_dict'])
        self.current_epoch = checkpoint['epoch']
        print(f'Resumed from epoch {self.current_epoch}')

    def train(self):
        """Main training loop."""
        for epoch in range(self.max_epochs):
            self.current_epoch = epoch

            # Train
            train_metrics = self.train_epoch()

            # Validate
            val_metrics = self.validate()

            # Step scheduler
            self.scheduler.step()

            # Combine metrics
            metrics = {**train_metrics, **val_metrics}

            # Log to wandb
            wandb.log(metrics, step=epoch)

            # Checkpointing
            is_best = val_metrics['val_loss'] < self.best_val_loss
            if is_best:
                self.best_val_loss = val_metrics['val_loss']

            self.save_checkpoint(metrics, is_best=is_best)

            print(f"Epoch {epoch}: train_loss={train_metrics['train_loss']:.4f}, "
                  f"val_loss={val_metrics['val_loss']:.4f}")
```toml

### 16.7.2 Hyperparameter Tuning with Ray Tune

```python
from ray import tune
from ray.tune.schedulers import ASHAScheduler

def train_bcc_model(config: Dict):
    """Training function for hyperparameter tuning."""
    # Create model with config
    model = BCCModel(
        hidden_dim=config['hidden_dim'],
        num_layers=config['num_layers'],
        dropout=config['dropout'],
    )

    # Create dataloaders
    train_loader = create_bcc_dataloader(
        'train.bcc', 'world', lod=3,
        feature_keys=['occupancy', 'intensity'],
        batch_size=config['batch_size'],
    )

    val_loader = create_bcc_dataloader(
        'val.bcc', 'world', lod=3,
        feature_keys=['occupancy', 'intensity'],
        batch_size=config['batch_size'],
    )

    # Train
    trainer = BCCTrainer(
        model=model,
        train_loader=train_loader,
        val_loader=val_loader,
        learning_rate=config['lr'],
        weight_decay=config['weight_decay'],
        max_epochs=50,
    )

    for epoch in range(50):
        train_metrics = trainer.train_epoch()
        val_metrics = trainer.validate()

        # Report to Ray Tune
        tune.report(
            loss=val_metrics['val_loss'],
            train_loss=train_metrics['train_loss'],
        )

# Run hyperparameter search
analysis = tune.run(
    train_bcc_model,
    config={
        'hidden_dim': tune.choice([64, 128, 256]),
        'num_layers': tune.choice([2, 3, 4]),
        'dropout': tune.uniform(0.0, 0.5),
        'lr': tune.loguniform(1e-4, 1e-2),
        'weight_decay': tune.loguniform(1e-5, 1e-3),
        'batch_size': tune.choice([16, 32, 64]),
    },
    num_samples=20,
    scheduler=ASHAScheduler(metric='loss', mode='min'),
    resources_per_trial={'gpu': 1},
)

best_config = analysis.best_config
print(f'Best config: {best_config}')
```

### 15.7.3 Multi-GPU Training with DistributedDataParallel

```python
import torch.distributed as dist
import torch.multiprocessing as mp
from torch.nn.parallel import DistributedDataParallel as DDP

def setup_distributed(rank: int, world_size: int):
    """Initialize distributed training."""
    os.environ['MASTER_ADDR'] = 'localhost'
    os.environ['MASTER_PORT'] = '12355'
    dist.init_process_group('nccl', rank=rank, world_size=world_size)

def cleanup_distributed():
    """Cleanup distributed training."""
    dist.destroy_process_group()

def train_ddp(rank: int, world_size: int, config: Dict):
    """Training function for each GPU."""
    setup_distributed(rank, world_size)

    # Create model and wrap with DDP
    model = BCCModel(**config['model_params'])
    model = model.to(rank)
    ddp_model = DDP(model, device_ids=[rank])

    # Create distributed sampler
    from torch.utils.data.distributed import DistributedSampler

    dataset = BCCDataset(**config['dataset_params'])
    sampler = DistributedSampler(
        dataset,
        num_replicas=world_size,
        rank=rank,
        shuffle=True,
    )

    train_loader = DataLoader(
        dataset,
        batch_size=config['batch_size'],
        sampler=sampler,
        num_workers=4,
        pin_memory=True,
    )

    # Training loop
    optimizer = AdamW(ddp_model.parameters(), lr=config['lr'])

    for epoch in range(config['max_epochs']):
        sampler.set_epoch(epoch)  # Shuffle differently each epoch

        ddp_model.train()
        for batch in train_loader:
            features = batch['features'].to(rank)
            neighbors = batch['neighbors'].to(rank)

            optimizer.zero_grad()
            predictions = ddp_model(features, neighbors)
            loss = nn.MSELoss()(predictions, features)
            loss.backward()
            optimizer.step()

        # Synchronize metrics across GPUs
        if rank == 0:
            print(f'Epoch {epoch} completed')

    cleanup_distributed()

# Launch multi-GPU training
def main():
    world_size = torch.cuda.device_count()
    config = {
        'model_params': {'hidden_dim': 128, 'num_layers': 3},
        'dataset_params': {'container_path': 'train.bcc', 'frame_id': 'world', 'lod': 3},
        'batch_size': 32,
        'lr': 1e-3,
        'max_epochs': 100,
    }

    mp.spawn(
        train_ddp,
        args=(world_size, config),
        nprocs=world_size,
        join=True,
    )

if __name__ == '__main__':
    main()
```rust

---

## 15.8 Model Serving and Inference

### 15.8.1 Production Serving Architecture

```python
from fastapi import FastAPI
from pydantic import BaseModel
import uvicorn
import torch

app = FastAPI()

class InferenceRequest(BaseModel):
    indices: List[int]
    frame_id: str
    lod: int

class InferenceResponse(BaseModel):
    predictions: List[float]
    latency_ms: float

class BCCModelServer:
    """Production model serving for BCC-indexed data."""

    def __init__(self, model_path: str, container_path: str, device: str = 'cuda'):
        # Load model
        checkpoint = torch.load(model_path, map_location=device)
        self.model = BCCModel(**checkpoint['model_config'])
        self.model.load_state_dict(checkpoint['model_state_dict'])
        self.model = self.model.to(device)
        self.model.eval()

        # Load container
        self.container = pyoctaindex.SequentialContainer.open(container_path)
        self.device = device

        # Compile model for faster inference (PyTorch 2.0+)
        self.model = torch.compile(self.model)

    @torch.no_grad()
    def predict(self, indices: List[int], frame_id: str, lod: int) -> np.ndarray:
        """Run inference on BCC indices."""
        # Extract features
        features = []
        neighbors = []

        for idx in indices:
            f = self.container.get_features(idx, ['occupancy', 'intensity'])
            n = self.container.get_neighbors(idx, radius=1)

            features.append(f)
            neighbors.append(n)

        # Convert to tensors
        features_t = torch.tensor(features, dtype=torch.float32, device=self.device)
        neighbors_t = torch.tensor(neighbors, dtype=torch.float32, device=self.device)

        # Inference
        predictions = self.model(features_t, neighbors_t)

        return predictions.cpu().numpy()

# Global server instance
server = BCCModelServer(
    model_path='checkpoints/best_model.pt',
    container_path='production_data.bcc',
)

@app.post('/predict', response_model=InferenceResponse)
async def predict(request: InferenceRequest):
    import time
    start = time.time()

    predictions = server.predict(request.indices, request.frame_id, request.lod)

    latency_ms = (time.time() - start) * 1000

    return InferenceResponse(
        predictions=predictions.tolist(),
        latency_ms=latency_ms,
    )

@app.get('/health')
async def health():
    return {'status': 'healthy'}

if __name__ == '__main__':
    uvicorn.run(app, host='0.0.0.0', port=8000)
```

### 15.8.2 Batch Inference Optimization

```python
class BatchInferenceEngine:
    """Optimized batch inference for large-scale processing."""

    def __init__(
        self,
        model_path: str,
        device: str = 'cuda',
        batch_size: int = 1024,
    ):
        self.model = self.load_model(model_path, device)
        self.device = device
        self.batch_size = batch_size

    def load_model(self, path: str, device: str):
        checkpoint = torch.load(path, map_location=device)
        model = BCCModel(**checkpoint['model_config'])
        model.load_state_dict(checkpoint['model_state_dict'])
        model = model.to(device)
        model.eval()
        return torch.compile(model)  # TorchScript or torch.compile

    @torch.no_grad()
    def predict_container(
        self,
        input_container: str,
        output_container: str,
        lod: int,
    ):
        """Process entire container with batched inference."""
        reader = pyoctaindex.SequentialContainer.open(input_container)
        writer = pyoctaindex.SequentialContainer.create(output_container)

        batch_indices = []
        batch_features = []

        for idx, features in reader.iter_at_lod(lod):
            batch_indices.append(idx)
            batch_features.append(features)

            if len(batch_indices) >= self.batch_size:
                # Process batch
                predictions = self._process_batch(batch_features)

                # Write results
                for i, pred in zip(batch_indices, predictions):
                    writer.insert(i, pred)

                # Clear batch
                batch_indices.clear()
                batch_features.clear()

        # Process remaining
        if batch_indices:
            predictions = self._process_batch(batch_features)
            for i, pred in zip(batch_indices, predictions):
                writer.insert(i, pred)

        writer.finalize()

    def _process_batch(self, features: List[np.ndarray]) -> np.ndarray:
        """Process a batch of features."""
        features_t = torch.tensor(features, dtype=torch.float32, device=self.device)
        predictions = self.model(features_t)
        return predictions.cpu().numpy()
```rust

### 15.8.3 Model Versioning and A/B Testing

```python
class ModelRegistry:
    """Manage multiple model versions for A/B testing."""

    def __init__(self):
        self.models = {}
        self.weights = {}

    def register_model(self, name: str, path: str, weight: float = 1.0):
        """Register a model version."""
        model = BCCModelServer(path)
        self.models[name] = model
        self.weights[name] = weight

    def predict_ab(self, indices: List[int], user_id: str) -> Tuple[np.ndarray, str]:
        """Route prediction to model based on A/B test."""
        import hashlib

        # Deterministic routing based on user_id
        hash_val = int(hashlib.md5(user_id.encode()).hexdigest(), 16)
        total_weight = sum(self.weights.values())

        cumulative = 0
        threshold = (hash_val % 100) / 100.0 * total_weight

        for name, weight in self.weights.items():
            cumulative += weight
            if threshold < cumulative:
                model = self.models[name]
                predictions = model.predict(indices, 'world', 3)
                return predictions, name

        # Default to first model
        name = list(self.models.keys())[0]
        predictions = self.models[name].predict(indices, 'world', 3)
        return predictions, name

# Usage
registry = ModelRegistry()
registry.register_model('baseline', 'models/v1.pt', weight=0.5)
registry.register_model('experimental', 'models/v2.pt', weight=0.5)

predictions, model_version = registry.predict_ab(indices, user_id='user123')
```

---

## 15.9 Performance Optimization

### 15.9.1 Mixed-Precision Training

```python
from torch.cuda.amp import autocast, GradScaler

class MixedPrecisionTrainer(BCCTrainer):
    """Training with automatic mixed precision."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.scaler = GradScaler()

    def train_epoch(self) -> Dict[str, float]:
        self.model.train()
        total_loss = 0.0
        num_batches = 0

        for batch in tqdm(self.train_loader):
            features = batch['features'].to(self.device)
            neighbors = batch['neighbors'].to(self.device)

            self.optimizer.zero_grad()

            # Mixed precision forward pass
            with autocast():
                predictions = self.model(features, neighbors)
                loss = self.criterion(predictions, features)

            # Scaled backward pass
            self.scaler.scale(loss).backward()
            self.scaler.unscale_(self.optimizer)
            torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
            self.scaler.step(self.optimizer)
            self.scaler.update()

            total_loss += loss.item()
            num_batches += 1

        return {'train_loss': total_loss / num_batches}
```toml

### 15.9.2 Gradient Accumulation

```python
def train_with_gradient_accumulation(
    model,
    dataloader,
    optimizer,
    accumulation_steps: int = 4,
):
    """Train with gradient accumulation for large effective batch size."""
    model.train()
    optimizer.zero_grad()

    for i, batch in enumerate(dataloader):
        features = batch['features'].to('cuda')
        neighbors = batch['neighbors'].to('cuda')

        # Forward pass
        predictions = model(features, neighbors)
        loss = nn.MSELoss()(predictions, features)

        # Normalize loss by accumulation steps
        loss = loss / accumulation_steps
        loss.backward()

        # Update weights every accumulation_steps
        if (i + 1) % accumulation_steps == 0:
            optimizer.step()
            optimizer.zero_grad()
```

### 15.9.3 Memory Profiling

```python
import torch.profiler as profiler

def profile_training_step(model, sample_batch):
    """Profile memory and compute for a training step."""
    with profiler.profile(
        activities=[
            profiler.ProfilerActivity.CPU,
            profiler.ProfilerActivity.CUDA,
        ],
        record_shapes=True,
        profile_memory=True,
        with_stack=True,
    ) as prof:
        features = sample_batch['features'].to('cuda')
        neighbors = sample_batch['neighbors'].to('cuda')

        predictions = model(features, neighbors)
        loss = nn.MSELoss()(predictions, features)
        loss.backward()

    # Print memory usage
    print(prof.key_averages().table(sort_by='cuda_memory_usage', row_limit=10))

    # Export Chrome trace
    prof.export_chrome_trace('trace.json')
```python

---

## 15.10 Troubleshooting Common Issues

### 15.10.1 Out of Memory Errors

**Problem**: GPU runs out of memory during training.

**Solutions**:
1. Reduce batch size
2. Enable gradient checkpointing
3. Use mixed-precision training
4. Clear cache: `torch.cuda.empty_cache()`

### 15.10.2 Slow Data Loading

**Problem**: GPU is underutilized due to slow data loading.

**Solutions**:
1. Increase `num_workers` in DataLoader
2. Use `pin_memory=True`
3. Pre-process and cache features
4. Use `persistent_workers=True`

### 15.10.3 Poor Model Convergence

**Problem**: Model loss doesn't decrease.

**Solutions**:
1. Check learning rate (try 1e-4 to 1e-2)
2. Verify data normalization
3. Check for NaN/Inf values
4. Reduce model complexity

---

## 15.11 Further Reading

**Machine Learning**:
- *Deep Learning* by Goodfellow, Bengio, and Courville
- *Dive into Deep Learning* (d2l.ai)

**Graph Neural Networks**:
- Kipf & Welling (2016). "Semi-Supervised Classification with Graph Convolutional Networks"
- Veličković et al. (2017). "Graph Attention Networks"

**3D Deep Learning**:
- Qi et al. (2017). "PointNet: Deep Learning on Point Sets for 3D Classification and Segmentation"
- Thomas et al. (2019). "KPConv: Flexible and Deformable Convolution for Point Clouds"

**PyTorch**:
- PyTorch documentation (pytorch.org)
- PyTorch Geometric documentation (pytorch-geometric.readthedocs.io)

---

## 15.12 Summary

In this chapter, we saw how OctaIndex3D connects with machine learning:

- **GNNs** operate on BCC-based graphs with isotropic connectivity.
- **Point cloud processing** uses BCC binning for efficient feature extraction.
- **3D object detection and trajectory prediction** benefit from multi-scale BCC representations.
- **PyTorch integration** includes custom Dataset classes, collation functions, and DataLoaders (§15.6).
- **Complete training pipeline** with progress tracking, validation, checkpointing, and hyperparameter tuning (§15.7).
- **Multi-GPU training** using DistributedDataParallel for scaling (§15.7.3).
- **Model serving** with FastAPI, batch inference optimization, and A/B testing (§15.8).
- **Performance optimization** through mixed-precision training, gradient accumulation, and memory profiling (§15.9).
- **Troubleshooting guidance** for common issues like OOM errors and slow data loading (§15.10).

The next chapter looks ahead to future research directions and potential evolutions of the OctaIndex3D ecosystem.
