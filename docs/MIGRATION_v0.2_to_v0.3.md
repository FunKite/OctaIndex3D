# Migration Guide: v0.2.0 to v0.3.0+

This guide helps users migrate from the legacy v0.2.0 API to the modern v0.3.0+ ID system.

## Overview

Version 0.3.0 introduced a new ID system with three specialized types (`Galactic128`, `Index64`, `Route64`, and optionally `Hilbert64`) while maintaining backward compatibility with the legacy `CellID` API.

## API Comparison

### Creating Spatial IDs

**Old API (v0.2.0):**
```rust
use octaindex3d::CellID;

// Create a cell ID
let cell = CellID::from_coords(0, 5, 100, 200, 300)?;
```

**New API (v0.3.0+):**
```rust
use octaindex3d::{Galactic128, Index64};

// Create a global ID (128-bit with frame, tier, LOD, attributes)
let galactic = Galactic128::new(0, 5, 0, 0, 0, 100, 200, 300)?;

// Create a Morton-encoded index (64-bit, space-filling curve)
let index = Index64::new(0, 0, 5, 100, 200, 300)?;
```

### Working with Neighbors

**Old API (v0.2.0):**
```rust
let neighbors = cell.neighbors()?;
```

**New API (v0.3.0+):**
```rust
use octaindex3d::{Route64, neighbors::neighbors_route64};

let route = Route64::new(0, 100, 200, 300)?;
let neighbors = neighbors_route64(route);
```

### Pathfinding

**Old API (v0.2.0):**
```rust
use octaindex3d::{CellID, path::{astar, EuclideanCost}};

let start = CellID::from_coords(0, 5, 0, 0, 0)?;
let goal = CellID::from_coords(0, 5, 10, 10, 10)?;
let path = astar(start, goal, &EuclideanCost)?;
```

**New API (v0.3.0+):**
```rust
use octaindex3d::{Route64, path::{astar, EuclideanCost}};

let start = Route64::new(0, 0, 0, 0)?;
let goal = Route64::new(0, 10, 10, 10)?;
// Note: Pathfinding still uses legacy CellID internally
// Direct Route64 pathfinding coming in future release
```

## Key Differences

### ID Types Comparison

| Feature | CellID (v0.2.0) | Galactic128 | Index64 | Route64 |
|---------|----------------|-------------|---------|---------|
| **Size** | Varied | 128-bit | 64-bit | 64-bit |
| **Frame** | ✓ | ✓ | ✓ | ✗ |
| **Tier** | ✓ | ✓ | ✓ | ✗ |
| **LOD** | ✓ | ✓ | ✓ | ✗ |
| **Attributes** | ✗ | ✓ (24-bit) | ✗ | ✗ |
| **Space-Filling Curve** | ✗ | ✗ | ✓ (Morton) | ✗ |
| **Local Routing** | ✗ | ✗ | ✗ | ✓ |
| **Bech32m Encoding** | ✗ | ✓ | ✓ | ✓ |

### When to Use Each Type

- **`Galactic128`**: Global unique IDs with full metadata (frame, tier, LOD, attributes)
- **`Index64`**: Space-efficient Morton-encoded spatial indices with excellent cache locality
- **`Route64`**: Fast local routing and neighbor calculations
- **`Hilbert64`**: Better spatial locality than Morton (requires `hilbert` feature)

## Backward Compatibility

The legacy `CellID` API remains available for backward compatibility:

```rust
use octaindex3d::CellID;

// All v0.2.0 CellID methods still work
let cell = CellID::from_coords(0, 5, 100, 200, 300)?;
let neighbors = cell.neighbors()?;
```

However, **new projects should use the v0.3.0+ ID types** for better performance and functionality.

## Migration Strategy

### Option 1: Gradual Migration (Recommended)

Keep existing code using `CellID` and gradually introduce new types:

```rust
// Existing code continues to work
let legacy_cell = CellID::from_coords(0, 5, 100, 200, 300)?;

// New features use new types
let index = Index64::new(0, 0, 5, 100, 200, 300)?;
let morton_code = index.morton_code();
```

### Option 2: Full Migration

Replace all `CellID` usage with appropriate new types:

1. **Replace global IDs** → Use `Galactic128`
2. **Replace spatial indices** → Use `Index64` or `Hilbert64`
3. **Replace routing coordinates** → Use `Route64`
4. **Update pathfinding** → Convert to/from `CellID` as needed (temporary)

## Common Patterns

### Pattern 1: Storing Cell References

**Before:**
```rust
let cell = CellID::from_coords(0, 5, 100, 200, 300)?;
database.store(cell);
```

**After:**
```rust
let galactic = Galactic128::new(0, 5, 0, 0, 0, 100, 200, 300)?;
let bech32_id = galactic.to_bech32m()?;
database.store(&bech32_id); // Human-readable with checksum
```

### Pattern 2: Spatial Queries

**Before:**
```rust
let cell = CellID::from_coords(0, 5, 100, 200, 300)?;
let neighbors = cell.neighbors()?;
```

**After:**
```rust
let index = Index64::new(0, 0, 5, 100, 200, 300)?;
let route = Route64::from(index);
let neighbors = neighbors_route64(route);
```

### Pattern 3: Batch Processing

**Before:**
```rust
for coords in coordinate_list {
    let cell = CellID::from_coords(0, 5, coords.0, coords.1, coords.2)?;
    process(cell);
}
```

**After:**
```rust
// Much faster with batch encoding
let indices = Index64::encode_batch(&coordinate_list, 0, 0, 5)?;
for index in indices {
    process(index);
}
```

## Benefits of Migration

1. **Performance**: Batch operations are 10-50x faster
2. **Type Safety**: Specialized types prevent misuse
3. **Human-Readable IDs**: Bech32m encoding with checksums
4. **Space Efficiency**: 64-bit IDs where appropriate
5. **Better Locality**: Morton and Hilbert space-filling curves

## Timeline

- **v0.2.0**: Legacy `CellID` API
- **v0.3.0**: New ID system introduced, `CellID` remains available
- **v0.4.0**: Current release, both APIs supported
- **Future**: `CellID` will eventually be deprecated and removed

## Need Help?

- See [examples/](../examples/) for complete working examples
- Check [WHITEPAPER.md](WHITEPAPER.md) for architectural details
- Report issues at [GitHub Issues](https://github.com/FunKite/OctaIndex3D/issues)

---

*Last updated: 2025-10-16*
