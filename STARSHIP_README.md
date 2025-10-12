# 🚀 Starship Command - Professional TUI Interface

A polished, professional terminal UI for deep space exploration using **ratatui** for perfect rendering and layout management.

## ✨ What's Fixed

### No More Formatting Issues!
- ✅ **Perfect borders** - All borders align correctly
- ✅ **Unicode handled properly** - Emojis and special chars don't break layout
- ✅ **Responsive panels** - Adapts to your terminal size
- ✅ **Clean rendering** - Uses proper TUI framework (ratatui)
- ✅ **Color support** - Beautiful color-coded displays

### Professional Layout
```
┌─────────────────────────────────────────────────────────────┐
│ U.S.S. NAVIGATOR - Mission Time: 00:15:32 | Jumps: 47      │
├─────────────────┬─────────────────┬─────────────────────────┤
│ Exploration     │ Ship Systems    │ Deployment              │
│ Stats           │                 │                         │
│ Galaxies:    15 │ Power:   [###…] │ Probes:    23           │
│ Systems:    127 │ Shields: [###…] │ Drones:    92           │
│ Planets:    342 │ Energy:  [###…] │ Missions:  23           │
├─────────────────┴─────────────────┴─────────────────────────┤
│ Current Target: Star System - Alpha Centauri-442            │
│ Distance: 12.5 LY | Coords: (1234, 5678, 9012)             │
├──────────────────────┬──────────────────────────────────────┤
│ 3D Holographic       │ Telemetry                            │
│ Display              │                                      │
│                      │ ┌─ Navigation Progress ──────────┐  │
│ FAR  Layer [Z=45]:   │ │ [########............] 42.5%   │  │
│   # . . . * . . . #  │ └────────────────────────────────┘  │
│   . . . . . . . .    │                                      │
│                      │ Position: (1234, 5678, 9012)        │
│ MID  Layer [Z=48]:   │ Waypoint: 17 / 40                   │
│   . . . . . . . . #  │ Velocity: 147x Light Speed           │
│   # . . . . . . . .  │                                      │
├──────────────────────┴──────────────────────────────────────┤
│ Mission Log                                                 │
│ [00:15:30] + Sensor scan: 175 cells analyzed                │
│ [00:15:31] ! 18 spatial hazards detected                    │
│ [00:15:32] + Path found: 40 waypoints, 86.60 LY            │
│ [00:15:33] * PLANET: Kepler-442-B - Rocky - HABITABLE!     │
└─────────────────────────────────────────────────────────────┘
OctaIndex3D v0.2.0 | BCC Lattice Navigation
```

## 🚀 Running It

```bash
cargo run --release --example starship_command
```

**Controls:**
- Press **'q'** to exit (no need for Enter!)
- Interface updates automatically every 250ms
- Continuous exploration until you quit

## 📊 Features

### Perfect Rendering with Ratatui
- **Professional TUI framework** - No manual border calculations
- **Automatic layout** - Handles terminal resizing
- **Unicode aware** - Proper width calculations for all characters
- **Color coded** - Different colors for different data types
- **Clean code** - Widget-based architecture

### Real-Time 3D Visualization
```
FAR  Layer [Z=45]:
  # . . . . . . . # . . .
  . . . . . . . . . . . .

MID  Layer [Z=48]:
  . . . . * . . . # . . .    <-- * = Your ship
  # . . . . . . . . . . .

NEAR Layer [Z=51]:
  . . . . . . . . . . . #
  . . # . . . . . . . . .

Legend: * = Ship  # = Obstacle  . = Path
```

### Navigation Progress
- **Live progress bar** showing journey completion
- **Real-time coordinates** updating as you move
- **Waypoint counter** (e.g., "17 / 40")
- **Velocity display** (147× Speed of Light = Warp 9.2)
- **Phase indicator** (Scanning, Planning, Navigating, Analyzing)

### Mission Log with Colors
- **Green** = Success messages (+)
- **Yellow** = Warnings (!)
- **White** = Information (i)
- **Magenta Bold** = Discoveries (*)
- **Timestamps** for every entry

### Discovery System
- **Galaxies** - Dwarf, Spiral, Elliptical, Irregular
- **Star Systems** - 1-3 stars per system
- **Planets** - Rocky, Gas Giant, Ice World, Super-Earth
- **Habitable Planets** → Auto-deploys probes!
- **Anomalies** - Rare special events

## 🎮 What You'll See

### Boot Phase
The interface loads with all systems showing nominal status.

### Exploration Phases
1. **Idle/Complete** → Generates new target
2. **Scanning** → Sensor sweep of target area
3. **Planning** → A* pathfinding with obstacle avoidance
4. **Navigating** → Real-time movement through space
5. **Analyzing** → Discovery of celestial objects

### 3D View
- Shows **3 depth layers** (FAR, MID, NEAR)
- **Ship position** marked with `*`
- **Obstacles** shown as `#`
- **Flight path** shown as `.`
- Updates in real-time during navigation

### Statistics
All stats accumulate and update live:
- Galaxies scanned
- Star systems explored
- Planets discovered (with habitable count)
- Anomalies detected
- Obstacles avoided
- Total distance in light-years
- Probes deployed
- Drones active

## 🎨 Color Scheme

- **Cyan** - Headers and target info
- **Green** - Exploration stats and success messages
- **Yellow** - Ship systems and warnings
- **Magenta** - Deployment stats and discoveries
- **Blue** - 3D holographic display
- **White** - General text and info
- **Dark Gray** - Footer

## ⚙️ Technical Details

### Dependencies
- **ratatui** - Terminal UI framework
- **crossterm** - Cross-platform terminal manipulation
- **octaindex3d** - The spatial navigation engine

### Performance
- **250ms tick rate** - Smooth updates without flickering
- **Efficient rendering** - Only redraws what changed
- **No lag** - Optimized for continuous operation

### OctaIndex3D Integration
Every exploration demonstrates:
- **K-ring sensor scans** - 3-ring radius (175 cells)
- **A* pathfinding** - Optimal route calculation
- **Obstacle avoidance** - AvoidBlockedCost with 1000.0 penalty
- **Multi-scale navigation** - Resolutions 5, 15, 25
- **BCC lattice** - 14-neighbor connectivity

## 🆚 vs. Previous Versions

| Feature | mothership_bridge.rs | starship_command.rs (NEW) |
|---------|---------------------|---------------------------|
| Border Issues | ❌ Misaligned | ✅ Perfect |
| Unicode Handling | ❌ Manual calc | ✅ Automatic |
| Layout | ❌ Hard-coded | ✅ Responsive |
| Colors | ❌ ANSI codes | ✅ TUI framework |
| Resizing | ❌ Breaks | ✅ Adapts |
| Code Quality | ⚠️  Complex | ✅ Widget-based |
| Exit Method | 'q' + Enter | 'q' only |

## 🛠️ Customization

### Adjust Tick Rate
Line 26:
```rust
const TICK_RATE: Duration = Duration::from_millis(250);
// Lower = faster updates, Higher = slower
```

### Change Colors
Edit the `Style::default().fg(Color::...)` calls in the render functions (lines 400-650)

### Modify Discovery Rates
Edit `make_discoveries()` function (line 707):
- Anomaly chance: `seed % 6 == 0` → Lower number = more frequent
- Habitable chance: `seed % 4 == 0` → Lower number = more frequent

### Adjust 3D View Size
Line 531 (in `render_3d_view`):
```rust
for dy in -3..=3 {  // Vertical range
    for dx in -20..=20 {  // Horizontal range
```

## 📈 What Makes This Better

1. **Professional Framework** - Uses industry-standard ratatui
2. **Automatic Layout** - No manual border calculations
3. **Proper Unicode** - Handles all characters correctly
4. **Responsive** - Adapts to terminal size
5. **Color Support** - Beautiful, color-coded interface
6. **Widget Architecture** - Clean, maintainable code
7. **Event Handling** - Proper keyboard input
8. **No Flickering** - Smooth, efficient rendering

## 🎯 Perfect For

- Demonstrating OctaIndex3D capabilities
- Learning ratatui TUI development
- Understanding spatial navigation
- Showcasing BCC lattice pathfinding
- Building terminal-based applications
- Presenting to audiences (looks professional!)

---

**To run: `cargo run --release --example starship_command`**

Press 'q' to exit. Enjoy the exploration, Captain! 🖖
