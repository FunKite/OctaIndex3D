# Appendix I: BCC Quick Reference Card

*This appendix is designed to be printed on a single sheet and kept at your desk.*

---

## The Parity Formula

A point $(x, y, z)$ is in the BCC lattice if and only if:

$$
(x + y + z) \mod 2 = 0
$$

**In code:**
```rust
fn is_bcc_valid(x: i32, y: i32, z: i32) -> bool {
    (x + y + z) & 1 == 0
}
```

---

## 14-Neighbor Offsets

### Opposite-Parity Neighbors (8) — Distance $\sqrt{3} \approx 1.732$

```rust
const OPPOSITE_PARITY: [(i32, i32, i32); 8] = [
    ( 1,  1,  1), (-1, -1, -1),
    ( 1, -1, -1), (-1,  1,  1),
    ( 1,  1, -1), (-1, -1,  1),
    ( 1, -1,  1), (-1,  1, -1),
];
```

### Same-Parity Neighbors (6) — Distance $2.0$

```rust
const SAME_PARITY: [(i32, i32, i32); 6] = [
    ( 2,  0,  0), (-2,  0,  0),
    ( 0,  2,  0), ( 0, -2,  0),
    ( 0,  0,  2), ( 0,  0, -2),
];
```

---

## Morton Encode/Decode (Portable)

```rust
// Spread bits for Morton encoding
fn split_by_3(mut x: u64) -> u64 {
    x &= 0x1F_FFFF;
    x = (x | x << 32) & 0x1F00_0000_FFFF;
    x = (x | x << 16) & 0x1F_0000_FF00_00FF;
    x = (x | x <<  8) & 0x100F_00F0_0F00_F00F;
    x = (x | x <<  4) & 0x10C3_0C30_C30C_30C3;
    x = (x | x <<  2) & 0x1249_2492_4924_9249;
    x
}

fn morton_encode(x: u32, y: u32, z: u32) -> u64 {
    split_by_3(x as u64) |
    (split_by_3(y as u64) << 1) |
    (split_by_3(z as u64) << 2)
}
```

---

## Key Numbers

| Metric | BCC | Cubic | Improvement |
|--------|-----|-------|-------------|
| Neighbor distance CV | 0.073 | 0.211 | **3× lower** |
| Samples for same quality | 71% | 100% | **29% fewer** |
| Path error vs Euclidean | ±5% | ±41% | **8× better** |
| Neighbor count | 14 | 6 or 26 | Consistent |

---

## Hierarchical Navigation

**Parent** (LOD $l \to l-1$):
```rust
fn parent(x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    (x >> 1, y >> 1, z >> 1)
}
```

**Children** (LOD $l \to l+1$):
```rust
fn children(x: i32, y: i32, z: i32) -> Vec<(i32, i32, i32)> {
    let parent_parity = (x + y + z) & 1;
    let mut result = Vec::with_capacity(8);
    for dx in 0..=1 {
        for dy in 0..=1 {
            for dz in 0..=1 {
                if (dx + dy + dz) & 1 == parent_parity {
                    result.push((2*x + dx, 2*y + dy, 2*z + dz));
                }
            }
        }
    }
    result
}
```

---

## Performance Ballparks

| Operation | BMI2 (x86) | NEON (ARM) | Generic |
|-----------|------------|------------|---------|
| Morton encode | 3-4 cycles | 5-7 cycles | 25-35 cycles |
| Morton decode | 3-4 cycles | 5-7 cycles | 25-35 cycles |
| BCC validation | 1 cycle | 1 cycle | 1 cycle |

**Break-even for GPU**: ~100,000 coordinates

---

## Common Imports

```rust
use octaindex3d::{
    // Identifiers
    Index64, Route64, Galactic128, Hilbert64,

    // Frames
    FrameDescriptor, get_frame, list_frames, register_frame,
};

use octaindex3d::neighbors::{neighbors_index64, neighbors_route64};
use octaindex3d::layers::{LayeredMap, OccupancyLayer, TSDFLayer, ESDFLayer};
```

---

## Quick Checks

**Is my point valid?**
```rust
assert!((x + y + z) % 2 == 0, "Invalid BCC point");
```

**Am I using the right scale?**
- `Galactic128`: Global/interplanetary (32-bit coords, 1km+ steps)
- `Index64`: Planet-wide (48-bit Morton, 100m steps typical)
- `Route64`: Local navigation (20-bit coords, cm-m steps)

**Is my workload GPU-worthy?**
- Fewer than 100K points → CPU is faster
- Latency-sensitive → CPU is better
- Streaming/irregular → CPU is simpler

---

*"When in doubt, profile it."*

---

## Emergency Fixes

**Paths are zigzagging:**
- Check you're using 14-neighbor connectivity, not 6 or 26

**Memory is 29% higher than expected:**
- Check you're actually using BCC, not cubic grid

**Morton decode is slow:**
- Enable BMI2: `RUSTFLAGS="-C target-cpu=native" cargo build --release`

**Can't convert between frames:**
- Register both frames in the FrameRegistry first

---

*OctaIndex3D v0.5.x — https://github.com/FunKite/OctaIndex3D*
