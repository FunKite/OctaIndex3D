# OctaIndex3D CLI Usage Guide

The OctaIndex3D command-line interface provides three main features:
- **Interactive 3D Maze Game** - Navigate through octahedral mazes generated with Prim's algorithm
- **Performance Benchmarks** - Test the speed of core operations
- **Utility Functions** - Encode/decode coordinates, calculate distances, find neighbors

## Building the CLI

```bash
cargo build --release --features cli
```

The binary will be located at `./target/release/octaindex3d`.

## 1. Interactive 3D Maze Game

Play an interactive text-based game where you navigate through a 3D octahedral maze built on a BCC (Body-Centered Cubic) lattice using Prim's algorithm.

### 🆕 New Features

- **Single-key navigation** - Just press a key to move, no Enter needed!
- **Visited/unvisited indicators** - See which nodes you've explored (● = visited, ○ = unvisited)
- **Progressive levels** - Start with 2×2×2, beat it to unlock 3×3×3, then 4×4×4, and beyond!
- **Real-time stats** - Track moves, time, and exploration percentage

### Basic Usage

```bash
# Progressive mode (default) - starts at 2x2x2, then 3x3x3, etc.
./target/release/octaindex3d play

# Play with different difficulty levels (single maze)
./target/release/octaindex3d play --difficulty easy    # 8x8x8
./target/release/octaindex3d play --difficulty medium  # 20x20x20
./target/release/octaindex3d play --difficulty hard    # 40x40x40

# Play with custom size (single maze)
./target/release/octaindex3d play --size 15

# Use a specific seed for reproducibility
./target/release/octaindex3d play --seed 42
```

### Game Controls (Single-Key Input!)

**Axial moves (2-step):**
- `n` - North (y+2)
- `s` - South (y-2)
- `e` - East (x+2)
- `w` - West (x-2)
- `u` - Up (z+2)
- `d` - Down (z-2)

**Diagonal moves (1-step in each axis):**
- `1` - NE-Up (+1,+1,+1)
- `2` - NE-Down (+1,+1,-1)
- `3` - NW-Up (-1,+1,+1)
- `4` - NW-Down (-1,+1,-1)
- `5` - SE-Up (+1,-1,+1)
- `6` - SE-Down (+1,-1,-1)
- `7` - SW-Up (-1,-1,+1)
- `8` - SW-Down (-1,-1,-1)

**Special commands:**
- `h` - Hint: show the next optimal move using A* pathfinding
- `q` - Quit the game

### Game Features

- **Real-time tracking**: See your current position, moves made, and time elapsed
- **Available directions**: Only valid moves are shown (no walls!)
- **Hints**: Get help from A* algorithm to find the optimal path
- **Performance comparison**: After completing the maze, see how your path compares to the A* optimal solution
- **Efficiency rating**: Get scored on how close you were to the optimal path

### Example Session

```
🎮 LEVEL 1 - Generating 2x2x2 Maze...
✓ Maze generated!
✓ Carved nodes: 2
✓ Start: (0, 0, 0) → Goal: (1, 1, 1)

Press any key to begin...

╔═══════════════════════════════════════════════════════════╗
║          3D OCTAHEDRAL MAZE GAME - Level 1               ║
╚═══════════════════════════════════════════════════════════╝
Position: (0, 0, 0)  →  Goal: (1, 1, 1)
Moves: 0  |  Time: 0.0s  |  Visited: 1/2
Distance to goal: 1 (straight line)

Available Directions (press key to move):
  [1] ○ NE-Up (+1,+1,+1)

Commands: [h]int | [q]uit

(User presses '1')

🎉 LEVEL 1 COMPLETE!
═══════════════════════════════════════════
Total moves: 1
Time taken: 2.3s
Nodes visited: 2/2

🤖 Computing optimal solution...
Optimal path: 1 moves
Your path: 1 moves
Efficiency: 100.0%

🏆 Nearly perfect! You found a near-optimal path!

🎯 Ready for Level 2?
Press [Enter] to continue or [q] to quit...
```

## 2. Performance Benchmarks

Run benchmarks to test the performance of core OctaIndex3D operations.

### Usage

```bash
# Run with default 100,000 iterations
./target/release/octaindex3d benchmark

# Run with custom iteration count
./target/release/octaindex3d benchmark --iterations 1000000
```

### Benchmarked Operations

1. **Morton Encoding** - Index64::new() creation
2. **Route64 Creation** - Local routing coordinate creation
3. **BCC Neighbor Calculations** - 14-neighbor connectivity
4. **BCC Validity Check** - Parity verification
5. **Maze Generation** - Prim's algorithm on 20x20x20 lattice
6. **A* Pathfinding** - Optimal path search on generated maze

### Example Output

```
╔═══════════════════════════════════════════════════════════╗
║            OCTAINDEX3D PERFORMANCE BENCHMARKS             ║
╚═══════════════════════════════════════════════════════════╝

Running 100000 iterations for each benchmark...

1. Morton Encoding (Index64::new)
   Time: 0.002s
   Rate: 45.23M ops/sec

2. Route64 Creation
   Time: 0.001s
   Rate: 89.45M ops/sec

[... more benchmarks ...]
```

## 3. Utility Functions

Various utility commands for working with OctaIndex3D coordinates and spatial operations.

### Encode Coordinates

Convert 3D coordinates to Index64 format:

```bash
./target/release/octaindex3d utils encode 100 200 300
```

Output:
```
Coordinate: (100, 200, 300)
Index64: Index64 { value: 9224779411810585920 }
Hex: 0x80050000044e8d40
Bech32m: i3d11sqzsqqqyf6x5qvpj6d3
```

**Note**: Coordinates must be in range 0..65535 (u16).

### Decode Index64

Decode a Bech32m-encoded Index64 back to coordinates:

```bash
./target/release/octaindex3d utils decode i3d11sqzsqqqyf6x5qvpj6d3
```

Output:
```
Bech32m: i3d11sqzsqqqyf6x5qvpj6d3
Index64: Index64 { value: 9224779411810585920 }
Hex: 0x80050000044e8d40
Coordinates: (100, 200, 300)
Frame: 0
Tier: 0
LOD: 5
```

### Calculate Distance

Calculate various distance metrics between two 3D points:

```bash
./target/release/octaindex3d utils distance 0,0,0 10,10,10
```

Output:
```
From: (0, 0, 0)
To: (10, 10, 10)
Euclidean Distance: 17.32
Manhattan Distance: 30
Chebyshev Distance (BCC minimum): 10
```

**Note**: Coordinates must be in format `x,y,z` with no spaces.

### Get Neighbors

Get all 14 BCC neighbors for a coordinate:

```bash
./target/release/octaindex3d utils neighbors 0 0 0
```

Output:
```
Coordinate: (0, 0, 0)
BCC Valid: Yes

14 BCC Neighbors:
  1. (1, 1, 1) ✓
  2. (1, 1, -1) ✓
  3. (1, -1, 1) ✓
  4. (1, -1, -1) ✓
  5. (-1, 1, 1) ✓
  6. (-1, 1, -1) ✓
  7. (-1, -1, 1) ✓
  8. (-1, -1, -1) ✓
  9. (2, 0, 0) ✓
  10. (-2, 0, 0) ✓
  11. (0, 2, 0) ✓
  12. (0, -2, 0) ✓
  13. (0, 0, 2) ✓
  14. (0, 0, -2) ✓
```

The ✓ indicates the neighbor is valid in the BCC lattice (all coordinates have same parity).

## Advanced Tips

### Maze Game Strategy

1. **Start with progressive mode** - The default progressive mode starts at 2×2×2 and gradually increases difficulty
2. **Watch for visited indicators** - ● means you've been there, ○ means it's unexplored
3. **Use hints sparingly** - Press `h` to see the optimal next move, but try to solve it yourself first!
4. **Master diagonal moves** - The 8 body-diagonal moves (keys 1-8) are often more efficient than axial moves
5. **Aim for 95%+ efficiency** - After completing, you'll see how close you were to optimal
6. **Try different seeds** - Each seed produces a different maze topology

### Performance Tuning

For maximum performance when running benchmarks:

```bash
# Build with native CPU optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release --features cli

# Run with high iteration count for more accurate measurements
./target/release/octaindex3d benchmark --iterations 1000000
```

### Integration with Scripts

The CLI can be integrated into shell scripts:

```bash
#!/bin/bash
# Example: Encode a batch of coordinates

while IFS=, read -r x y z; do
    ./target/release/octaindex3d utils encode $x $y $z
done < coordinates.csv
```

## Technical Details

### BCC Lattice Structure

The maze game uses a Body-Centered Cubic (BCC) lattice with 14-neighbor connectivity:
- **8 body-diagonal neighbors**: (±1, ±1, ±1) - Move diagonally through the cube
- **6 axial neighbors**: (±2, 0, 0), (0, ±2, 0), (0, 0, ±2) - Move along axes

This creates a more isotropic (uniform in all directions) spatial structure than standard cubic grids.

### Prim's Algorithm

The maze generation uses **randomized Prim's algorithm**:
1. Start with a single carved cell
2. Maintain a frontier of uncarved cells adjacent to carved cells
3. Randomly select a frontier cell and connect it to a random carved neighbor
4. Repeat until all valid BCC cells are carved

This creates a perfect maze (spanning tree) with no loops and exactly one path between any two points.

### A* Pathfinding

The optimal path calculation uses **A* search** with Euclidean distance heuristic:
- Explores the maze following the spanning tree edges
- Uses straight-line distance to goal as the heuristic
- Guaranteed to find the shortest path in the maze

## Troubleshooting

### Build Errors

If you encounter build errors:

```bash
# Make sure the cli feature is enabled
cargo build --release --features cli

# Clean and rebuild if needed
cargo clean
cargo build --release --features cli
```

### Game Input Issues

If directional commands aren't working:
- Make sure to use lowercase (e.g., `neu` not `NEU`)
- Only moves shown in "Available Directions" are valid
- Use `hint` to verify the next valid move

### Coordinate Range Errors

If you get "out of range" errors with encode:
- Index64 coordinates must be 0..65535 (u16 range)
- For larger coordinates, use Route64 instead (supports i32 range)

## See Also

- [Main README](README.md) - Project overview
- [Examples](examples/) - More example code
- [API Documentation](https://docs.rs/octaindex3d) - Full library documentation
