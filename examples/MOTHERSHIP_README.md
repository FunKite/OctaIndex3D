# 🛸 Mothership Dashboard - Continuous Interstellar Exploration

An immersive Star Trek-style command center interface demonstrating OctaIndex3D's multi-scale spatial navigation capabilities through continuous exploration of the universe.

## Features

### 🎮 Interactive Dashboard
- **Mission Clock**: Real-time mission duration tracking
- **Ship Systems**: Power, shields, energy, sensors, and navigation status displays
- **Running Statistics**: Live tallies of discoveries and exploration progress
- **Current Target**: Real-time display of exploration objectives
- **Sensor Array**: Live feed of sensor scans and navigation computations

### 🌌 Exploration System
The mothership continuously explores:
- **Galaxies** (Resolution 5) - Million light-year scale
- **Star Systems** (Resolution 15) - Tens of light-years scale
- **Planets** (Resolution 25) - Single light-year scale

### 📊 Discovery Types
- **Galaxies**: Dwarf, Spiral, Elliptical, Irregular formations
- **Star Systems**: Single, binary, and trinary star systems
- **Planets**: Rocky, Gas Giants, Ice Worlds, Super-Earths, Desert worlds
- **Anomalies**: Subspace distortions, nebulae, dark matter, ancient signals, wormholes

### 🛸 Autonomous Operations
- **Obstacle Avoidance**: Real-time A* pathfinding with spatial hazard detection
- **Probe Deployment**: Automatic deployment to habitable planets
- **Drone Operations**: Surface reconnaissance on promising worlds
- **Data Transmission**: Results sent to "Federation Science Division"

## Running the Demo

### Standard Run (10 explorations)
```bash
cargo run --release --example mothership_dashboard
```

### Continuous Mode (Runs Forever)
Edit `examples/mothership_dashboard.rs` and comment out the exploration limit:

```rust
// Line 96-99: Comment these lines out
// if stats.exploration_count >= 10 {
//     show_shutdown_sequence(&stats)?;
//     break;
// }
```

Then run:
```bash
cargo run --release --example mothership_dashboard
```

Press **Ctrl+C** to exit and see final mission statistics.

## What You'll See

### Boot Sequence
```
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║              🛸  MOTHERSHIP COMMAND & CONTROL SYSTEM  🛸                 ║
║                                                                           ║
║                        OctaIndex3D Navigation Core                        ║
║                              Version 0.2.0                                ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝

  ┌─ PRIMARY SYSTEMS
  │  ⚡ Quantum Drive ........................ ✓ ONLINE
  │  🔋 Power Core ........................ ✓ ONLINE
  │  🛡️  Shield Generator ........................ ✓ ONLINE
  │  📡 Long Range Sensors ........................ ✓ ONLINE
```

### Main Dashboard
```
╔══════════════════════════════════════════════════════════════════════════╗
║      🛸 U.S.S. NAVIGATOR - DEEP SPACE EXPLORATION COMMAND 🛸             ║
╠══════════════════════════════════════════════════════════════════════════╣
║  Mission Time: 00:05:42  │  Status: ACTIVE  │  FTL Drive: ENGAGED       ║
╠──────────────────────────────────────────────────────────────────────────╣
║                                                                          ║
║  ┌─ EXPLORATION STATISTICS ─────┐  ┌─ SHIP SYSTEMS ──────────────────┐ ║
║  │                               │  │                                  │ ║
║  │  🌌 Galaxies Scanned:      15 │  │  ⚡ Power:    [████████] 100%   │ ║
║  │  ⭐ Star Systems:         127 │  │  🛡️  Shields:  [████████] 100%   │ ║
║  │  🪐 Planets Discovered:   342 │  │  🔋 Energy:   [████████] 100%   │ ║
║  │  ❓ Anomalies Detected:    23 │  │  🤖 Drones:          45          │ ║
║  │  🚧 Obstacles Avoided:    198 │  │  🛸 Probes:          18          │ ║
```

### FTL Travel Animation
```
🚀 ENGAGING FTL DRIVE...
═ [████████████████░░░░░░░░░░░░░░]  54.2% │ Pos: (12451, -8832, 5621)
```

### Discoveries
```
🔬 ANALYSIS RESULTS:
🌌 Galaxy detected: Andromeda - Size: Spiral
⭐ Star system mapped: Alpha Centauri - 3 star(s)
🪐 Planet catalogued: Kepler-442-B - Rocky - HABITABLE ✓
   🛸 Deploying survey probe...
   ✓ Probe deployed - 3 drones active on surface
❓ Anomaly identified: Ancient Signal - Non-natural EM signature detected
```

## OctaIndex3D Technology Showcase

This demo demonstrates:
- ✅ **Multi-scale Navigation**: Resolutions 5 → 35 (millions of light-years to meters)
- ✅ **Hierarchical Spatial Indexing**: 8:1 refinement across scales
- ✅ **Real-time Obstacle Avoidance**: A* pathfinding with blocked cell detection
- ✅ **K-ring Sensor Scanning**: Efficient spatial queries around current position
- ✅ **BCC Lattice 14-Neighbor Connectivity**: Optimal 3D sampling for navigation
- ✅ **Dynamic Coordinate Systems**: Handling astronomical coordinate ranges

## Customization

### Adjust Exploration Speed
Modify sleep timings in `mothership_dashboard.rs`:
- Boot sequence: Lines 119-146
- Sensor scans: Lines 253-272
- Travel animation: Line 389
- Discovery display: Lines 313-336

### Change Discovery Rates
Modify `make_discoveries()` function (lines 395-457):
- Anomaly chance: Line 445 (`seed % 7 == 0`)
- Habitable planet chance: Line 428 (`seed % 3 == 0`)
- Planet types: Lines 421-422

### Add New Target Types
Edit `generate_exploration_target()` function (lines 460-536) to add:
- Asteroid fields
- Comets
- Space stations
- Black holes
- etc.

## Performance

The demo uses:
- **OctaIndex3D v0.2.0**: High-performance BCC lattice implementation
- **A* Pathfinding**: ~1M cell expansions/second
- **K-ring Queries**: ~500ns for 211 cells
- **Real-time Processing**: No perceptible lag during navigation

## Use Case

This demonstrates OctaIndex3D's suitability for:
- Space simulation games
- Astronomical data visualization
- Multi-scale route planning
- Procedural universe generation
- Scientific space mission planning

---

**Live long and prosper.** 🖖
