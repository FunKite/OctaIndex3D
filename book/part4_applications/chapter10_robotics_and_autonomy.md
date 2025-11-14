# Chapter 10: Robotics and Autonomous Systems

## Learning Objectives

By the end of this chapter, you will be able to:

1. Describe how OctaIndex3D supports 3D occupancy mapping for robotic systems.
2. Understand how sensor data (LiDAR, RGB-D, radar) is integrated into BCC-indexed grids.
3. Design path-planning queries that exploit BCC isotropy.
4. Manage real-time constraints using incremental updates and multi-resolution planning.
5. Apply these ideas in a UAV navigation case study.

---

## 10.1 3D Occupancy Grids

Robotics systems often represent the environment as a **3D occupancy grid**:

- Each cell encodes the probability that space is occupied.
- Planning and collision checking operate directly on this grid.

Using BCC lattices instead of cubic grids yields:

- More isotropic neighbor relationships (less bias in pathfinding).
- Fewer cells for equivalent spatial resolution (memory savings).

In OctaIndex3D:

- Occupancy grids are modeled as containers keyed by `Index64` or `Hilbert64`.
- Frames represent coordinate choices (e.g., local ENU for the robot’s world).
- Queries map continuous robot poses into lattice coordinates via the frame registry.

You can think of this as replacing a “stack of 2D images” with a single coherent 3D structure. Each BCC cell is large enough to be relevant for collision checking, but small enough that you can resolve shelves, pillars, and other clutter. Because the same indexing scheme works from the floor to the ceiling, the motion planner does not need special cases for ramps, stairwells, or multi-level environments: it just moves through cells.

From an implementation perspective, most robotics stacks already maintain something like:

- A pose estimate of the robot in a world frame.
- A stream of depth or range measurements.
- A map representation, frequently a voxel grid or TSDF.

OctaIndex3D replaces the ad hoc voxel grid with:

- A well-typed container whose keys are BCC indices.
- A frame-aware interface that makes units and origins explicit.
- Query primitives that understand neighbor relationships and refinement levels.

Once the grid is expressed in these terms, path planners, collision checkers, and map visualizers all speak a common language.

---

### 10.1.1 Representing Occupancy Values

A practical occupancy grid needs to decide how to encode belief about each cell. Two common options are:

- **Probabilities** in `[0.0, 1.0]`
- **Log-odds** values in `(-∞, +∞)`

Log-odds are usually preferable in robotics:

- Additive updates (a hit adds `+α`, a miss adds `-β`)
- Numerically stable over thousands of observations
- Easy to clamp to configurable confidence bounds

A minimal value type might look like:

```rust
/// Log-odds occupancy value.
#[derive(Clone, Copy)]
pub struct LogOdds(f32);

impl LogOdds {
    pub fn update(self, delta: f32, min: f32, max: f32) -> Self {
        let v = (self.0 + delta).clamp(min, max);
        Self(v)
    }

    pub fn to_probability(self) -> f32 {
        let e = (self.0 as f64).exp();
        (e / (1.0 + e)) as f32
    }
}
```

OctaIndex3D containers then store `LogOdds` keyed by `Index64` or `Hilbert64`:

- Sparse maps (e.g., `HashMap<Index64, LogOdds>`) for large, mostly empty spaces
- Dense arrays for small local volumes around the robot
- Hybrid layouts for multi-resolution grids

The key point is that **indexing is decoupled from storage**: the same index type can back a hash map, a flat array, or a block-compressed structure, depending on your performance needs.

### 10.1.2 Minimal Log-Odds Grid Example

A skeletal log-odds grid using a sparse container might look like:

```rust
use std::collections::HashMap;

struct OccupancyGrid {
    cells: HashMap<Index64, LogOdds>,
    hit_delta: f32,
    miss_delta: f32,
    min_lo: f32,
    max_lo: f32,
}
```

Core methods include:

- `integrate_hit(index)` – apply `+hit_delta`
- `integrate_miss(index)` – apply `-miss_delta`
- `is_occupied(index)` – compare probability to a threshold

These methods remain agnostic to frames. Calling code is responsible for:

1. Converting continuous positions to indices via the frame registry.
2. Choosing the LOD appropriate for the robot’s current task.
3. Applying the updates in batches for better performance.

This separation mirrors the overall OctaIndex3D architecture: **frames decide where**, indices decide **which cell**, and containers decide **how values are stored and updated**.

## 10.2 Sensor Fusion

Modern robots combine data from multiple sensors:

- **LiDAR** provides sparse but accurate range measurements.
- **RGB-D cameras** provide dense but noisier depth data.
- **Radar** provides long-range, lower-resolution detections.

OctaIndex3D supports sensor fusion by:

- Using frames to express each sensor’s pose in a common coordinate system.
- Projecting rays or depth pixels into BCC cells.
- Updating occupancy probabilities using Bayesian or log-odds filters.

In a typical pipeline:

1. Sensor drivers publish measurements in the sensor’s native frame.
2. The localization system estimates the robot pose in a world or map frame.
3. OctaIndex3D transforms each measurement into that map frame via the frame registry.
4. Rays are traced through the BCC lattice, marking traversed cells as free and endpoints as occupied.

Because neighbor relationships are uniform in all directions, the ray-tracing logic does not have “easy” directions and “hard” directions the way a cubic grid does. This reduces subtle bias in how obstacles appear in the map, especially when sensors are mounted at odd angles or the robot frequently rotates.

Batch APIs allow:

- Efficient accumulation of thousands of measurements per frame.
- Use of SIMD-friendly kernels for occupancy updates.

Practically, this means you can accumulate full LiDAR sweeps or depth camera images with a handful of calls, instead of iterating cell-by-cell in application code. The heavy lifting—index conversion, ray traversal, and occupancy update—is handled inside the library, where it can be tuned and tested thoroughly.

---

### 10.2.1 Implementing a Ray Update Kernel

A typical integration kernel takes:

- A sensor pose in the map frame
- A batch of range or depth measurements
- Tuning parameters (hit/miss log-odds, max range, step size)

At a high level:

```rust
fn integrate_scan(
    grid: &mut OccupancyGrid,
    frame: &FrameId,
    sensor_pose: Pose3,
    returns: &[RangeReturn],
) {
    for r in returns {
        let ray = frame.project_range(sensor_pose, r);
        for cell in ray.traversed_cells() {
            grid.integrate_miss(cell.index);
        }
        if let Some(hit_cell) = ray.hit_cell() {
            grid.integrate_hit(hit_cell.index);
        }
    }
}
```

The details of `project_range` and `traversed_cells` depend on your chosen API, but the structure is stable:

- A **geometric layer** converts raw measurements into BCC cell indices.
- An **update layer** applies log-odds deltas to those cells.

Batching by scan or by LOD lets the implementation use SIMD-friendly kernels internally without complicating application code.

### 10.2.2 SLAM Integration Patterns

Most SLAM systems separate:

- **State estimation** (poses, velocities, landmarks)
- **Mapping** (occupancy grids, signed distance fields, feature maps)

OctaIndex3D fits naturally as the mapping component. Two common patterns are:

1. **SLAM owns the map, OctaIndex3D wraps it**
   - The SLAM backend maintains its internal map representation.
   - OctaIndex3D is used as a read-only index for queries (e.g., collision checks, visibility tests).
   - A synchronization layer periodically exports the map into BCC cells.

2. **OctaIndex3D owns the map, SLAM consumes it**
   - The occupancy grid lives primarily in an OctaIndex3D container.
   - SLAM optimization (pose graph, factor graph) treats the grid as an external field.
   - Cost functions query occupancy along candidate trajectories via neighbor and ray-cast APIs.

In both cases, the integration hinges on **frames**:

- Robot poses are expressed in the map frame managed by the frame registry.
- Landmark positions and loop-closure constraints are evaluated in that same frame.
- OctaIndex3D containers are keyed by identifiers derived from that frame.

This ensures that SLAM optimization, mapping, and planning stay consistent, even when the global frame drifts or is re-anchored (for example, after a loop closure).

## 10.3 Path Planning with A*

Path planning algorithms like A* operate on graphs:

- Nodes represent lattice cells.
- Edges connect neighboring cells.
- Edge weights encode movement cost.

In a cubic grid, movement costs and neighbor counts depend heavily on direction. BCC lattices mitigate this by:

- Providing 14 neighbors at nearly equal distances.
- Reducing directional bias in computed paths.

OctaIndex3D exposes neighbor queries that:

- Return neighbors in deterministic order.
- Provide distances or approximate costs for use in A* heuristics.

Planners can layer multiple resolutions together:

- A coarse LOD grid captures the global topology of the environment (rooms, aisles, ramps).
- Finer LODs near the robot and around obstacles capture detailed geometry.

A typical planning loop might:

1. Run A* on a coarse grid to find a route several dozen meters ahead.
2. Project that route down to a finer LOD in the local neighborhood.
3. Re-run A* or a local planner using the finer grid to avoid small obstacles and dynamic agents.
4. Feed the resulting path to a controller that operates in continuous space.

Because refinement preserves parent–child relationships in the BCC lattice, each coarse cell along the global path maps cleanly to a small set of finer cells. There is no need for awkward regridding or interpolation steps: the identifiers carry the hierarchy information for you.

---

### 10.3.1 Global vs. Local Planners

Most autonomous systems use at least two planning layers:

- A **global planner** that reasons about the entire environment.
- A **local planner** that focuses on a short horizon around the robot.

OctaIndex3D supports this split naturally:

- Global planning uses coarse LOD containers and cheaper cost models.
- Local planning uses finer LOD containers with higher-fidelity occupancy.

For example:

- Global: A* or D* Lite over a few thousand coarse cells.
- Local: A* or hybrid A* over tens of thousands of fine cells near the robot.

Because indices encode LOD, the local planner can quickly restrict itself to:

- Children of cells along the global route
- Cells within a fixed radius of the current pose

### 10.3.2 Heuristics on a BCC Lattice

On a regular grid, Manhattan or Euclidean distance is typically used as an A* heuristic. On a BCC lattice:

- The underlying geometry is still Euclidean.
- Neighbor distances are more uniform than on a cubic grid.

This suggests a simple heuristic:

- Convert each candidate cell’s index back to lattice coordinates.
- Compute a scaled Euclidean distance to the goal cell.

Because neighbor distances are nearly identical, this heuristic is **admissible** and **consistent** for most practical purposes. It also reduces the “stair-stepping” artifacts that appear when Manhattan distance is used on a cubic grid.

### 10.3.3 Motion Planning Algorithms Beyond A*

While A* is a natural first choice, many robots combine or replace it with:

- **D* / D* Lite / Anytime D*** for dynamic environments where obstacles move.
- **RRT / RRT\*** for high-dimensional configuration spaces (e.g., arms).
- **Model Predictive Control (MPC)** for systems with strong dynamic constraints.

OctaIndex3D supports these algorithms by providing:

- Fast collision checks along line segments or short trajectories.
- Neighborhood queries for sampling-based planners (e.g., “nearby free cells”).
- Multi-resolution collision checks (coarse pre-check, fine verification).

For example, an RRT node expansion step might:

1. Sample a target pose in continuous space.
2. Discretize the straight-line path into BCC cells along the segment.
3. Reject the sample if any cells exceed an occupancy threshold.
4. Otherwise, add the node and edge to the tree.

The same pattern works for lattice-based MPC: each candidate control sequence is mapped to a short trajectory, which is evaluated against the occupancy containers.

## 10.4 Real-Time Constraints

Robotic systems face strict real-time constraints:

- Control loops may run at tens or hundreds of Hertz.
- Sensor data arrives continuously and must be integrated quickly.

OctaIndex3D addresses these constraints by:

- Supporting incremental container updates (insert/erase/update operations).
- Allowing planners to work on **snapshots** of the grid while updates continue in the background.
- Providing batch query APIs that minimize per-call overhead.

One common pattern is a double-buffered map:

- A background thread continually integrates new sensor data into a “live” container.
- Periodically (for example, at 10 Hz), the live container is cloned or checkpointed into a read-only snapshot.
- Planning and collision checking operate exclusively on the snapshot until the next update.

Because containers are built on compact value types, cloning or snapshotting can be surprisingly cheap—often much cheaper than re-running the entire mapping pipeline inside the planner. This decouples the timing of sensing, mapping, and planning, making it easier to respect real-time budgets.

Trade-offs include:

- Choosing update frequencies for different LODs.
- Balancing memory usage against planning horizon and resolution.

---

### 10.4.1 Budgeting and Profiling

Real-time performance starts with a **budget**. For a 20 Hz planning loop (50 ms per cycle), a typical breakdown might be:

- Sensor fusion and mapping: 15–20 ms
- Global planning (coarse grid): 5–10 ms (not every cycle)
- Local planning (fine grid): 10–15 ms
- Control and actuation: 5–10 ms

OctaIndex3D helps you stay within these budgets by:

- Exposing batched operations (`integrate_scan`, multi-cell queries).
- Allowing you to tune LOD and map extents independently.
- Making it cheap to skip or throttle expensive operations when needed.

Basic profiling strategies include:

- Timing each stage (mapping, global plan, local plan) separately.
- Recording LOD and container sizes alongside timings.
- Logging worst-case and percentile latencies over long runs.

These measurements quickly reveal whether you should:

- Reduce map radius at fine LODs.
- Increase step size in ray casting.
- Run global planning less often.

### 10.4.2 Degradation Strategies

When a robot is overloaded—too many obstacles, too much sensor data, or an underpowered CPU—it needs a **graceful degradation** strategy. OctaIndex3D’s multi-resolution design enables several options:

- Temporarily disable updates to the coarsest or finest LOD.
- Shrink the local fine-grid radius around the robot.
- Increase downsampling of input point clouds or depth images.
- Use cheaper heuristics or shorter horizons in planners.

Crucially, these strategies preserve the **core abstraction**:

- Frames, identifiers, and containers remain unchanged.
- Only the density and extent of occupied cells varies.

As a result, you can implement performance fallbacks without rewriting algorithms or changing APIs. Configuration changes—LOD thresholds, map radii, downsampling factors—are enough.

## 10.5 UAV Navigation Case Study

Consider a UAV navigating a warehouse:

- The environment is represented in a local ENU frame anchored at the warehouse origin.
- Sensor data from LiDAR and cameras is fused into a BCC occupancy grid.
- Global planning uses a coarse LOD; local obstacle avoidance uses a finer LOD.

Using OctaIndex3D, the end-to-end system might look like this:

1. The frame registry defines the warehouse frame and a body frame attached to the UAV. Sensor frames (LiDAR, cameras) are registered relative to the body frame.
2. As the UAV flies, the state estimator publishes its pose in the warehouse frame. Each sensor measurement is transformed from its sensor frame, through the body frame, into the warehouse frame.
3. Measurements are discretized to BCC cells at two LODs: a coarse grid for long-range awareness and a fine grid within, say, 10 meters of the UAV.
4. Occupancy containers store log-odds values keyed by `Index64` or `Hilbert64`. Batch update calls integrate hundreds or thousands of measurements per cycle.
5. A global planner runs A* on the coarse grid, computing a path from the current location to a goal waypoint (for example, a loading bay).
6. A local planner refines segments of that path on the fine grid, reacting to newly observed obstacles like moving forklifts or workers.
7. Finally, the refined path is converted back into smooth continuous waypoints, respecting the UAV’s dynamics and no-fly zones.

Even in this relatively simple example, you can see all of the major ideas of the book in play:

- Frames keep coordinate systems straight.
- BCC indices provide compact, isotropic sampling of space.
- Containers maintain multi-resolution occupancy maps.
- Queries support both global and local planning without changing data structures.

In practice, teams adopting this style of mapping and planning report not only quantitative improvements (shorter paths, lower collision rates) but also qualitative ones: the robot’s behavior feels less “grid-aligned” and more natural, especially when navigating diagonally through cluttered environments.

---

### 10.5.1 Mapping Loop

The UAV’s mapping loop can be written as a tight, repeatable sequence:

1. Read the latest pose estimate in the warehouse frame.
2. Fetch sensor measurements (LiDAR sweep, stereo depth, radar returns).
3. Transform measurements into the warehouse frame using the frame registry.
4. Discretize rays into BCC cells at coarse and fine LODs.
5. Apply log-odds updates to the corresponding occupancy containers.

In code-like pseudocode:

```rust
fn mapping_step(world: &mut WorldState) {
    let pose = world.state_estimator.pose_in(&world.frames.warehouse);
    let scans = world.sensors.read_all();

    for scan in scans {
        let frame = world.frames.for_sensor(&scan.sensor_id);
        let returns = frame.to_map_returns(pose, &scan);
        integrate_scan(&mut world.grid, &world.frames.warehouse, pose, &returns);
    }
}
```

Here, `WorldState` owns both:

- The **frame registry** mapping sensor IDs to frames.
- The **occupancy grid** containers that back global and local planners.

### 10.5.2 Planning Loop

The planning loop then operates on snapshots of the grid:

1. Clone or checkpoint the current occupancy containers.
2. Run global A* (or D* Lite) on the coarse grid toward the goal.
3. Project a corridor around the global path into the fine grid.
4. Run local planning (A* or MPC) within that corridor.
5. Output a time-parameterized trajectory to the controller.

In pseudocode:

```rust
fn planning_step(world: &mut WorldState, goal: Pose3) {
    let snapshot = world.grid.snapshot();

    let coarse_path = global_plan(&snapshot.coarse, world.pose, goal);
    let corridor = corridor_from_path(&coarse_path);

    let fine_path = local_plan(&snapshot.fine, corridor, world.pose, goal);
    world.controller.follow(fine_path);
}
```

This separation of mapping and planning:

- Keeps heavy sensor fusion work off the critical control path.
- Allows planners to work with consistent, read-only views of the world.
- Makes it straightforward to simulate or replay missions using recorded snapshots.

### 10.5.3 Failure Modes and Resilience

Real warehouses are messy: GPS is unavailable, pallets move, and humans walk through aisles. OctaIndex3D does not solve these problems by itself, but it helps structure the response:

- **Localization loss:** fall back to a conservative mode that:
  - Freezes global planning.
  - Shrinks the local map radius.
  - Increases occupancy thresholds before entering unknown space.
- **Sensor dropout:** maintain a time-decayed occupancy grid so stale observations gradually revert toward “unknown” rather than “free” or “occupied”.
- **Unexpected obstacles:** treat clusters of newly occupied cells as dynamic; avoid baking them into long-lived maps until they are observed consistently.

All of these behaviors can be implemented as policies over:

- How occupancy values are updated and decayed.
- Which LODs are used for which decisions.
- When to accept, reject, or delay planned motions.

The underlying frame and container structure remains unchanged, reducing complexity in the higher-level autonomy stack.

## 10.6 Summary

In this chapter, we saw how OctaIndex3D applies to robotics and autonomous systems:

- **Occupancy grids** benefit from BCC isotropy and memory efficiency.
- **Sensor fusion** is simplified by the frame registry and batch update APIs.
- **Path planning** algorithms like A* and its relatives operate on neighbor queries with reduced directional bias.
- **SLAM integration** uses frames to keep mapping and state estimation aligned.
- **Real-time performance analysis** and degradation strategies help robots stay within strict timing budgets.
- A **UAV navigation case study** illustrates these ideas in a realistic setting, from mapping through planning and execution.

The next chapter turns to geospatial analysis, where similar principles apply at much larger spatial scales.
