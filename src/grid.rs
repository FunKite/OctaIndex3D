//! High-level facade for working with the BCC lattice in physical units.
//!
//! [`BccGrid`] hides the lattice details (parity, scale tiers, signed 20-bit
//! coordinate ranges) behind a small API that works in physical units
//! (e.g. meters): convert points to cells, query neighbors and k-rings, and
//! run A* pathfinding — without touching the lower-level ID machinery.
//!
//! # Example
//!
//! ```
//! use octaindex3d::BccGrid;
//!
//! fn main() -> octaindex3d::Result<()> {
//!     // Cells whose centers are 0.5 physical units apart along each axis
//!     let grid = BccGrid::new(0.5)?;
//!
//!     // Convert a physical point to its cell
//!     let cell = grid.cell_at(1.2, 3.4, 5.6)?;
//!
//!     // Every interior cell has 14 neighbors
//!     assert_eq!(grid.neighbors(cell).len(), 14);
//!
//!     // All cells within 2 hops
//!     let nearby = grid.k_ring(cell, 2);
//!     assert!(nearby.len() > 14);
//!
//!     // Shortest path between two points
//!     let start = grid.cell_at(0.0, 0.0, 0.0)?;
//!     let goal = grid.cell_at(3.0, 3.0, 3.0)?;
//!     let path = grid.astar(start, goal)?;
//!     assert_eq!(path.cells.first(), Some(&start));
//!     assert_eq!(path.cells.last(), Some(&goal));
//!     Ok(())
//! }
//! ```

use crate::error::{Error, Result};
use crate::ids::Route64;
use crate::lattice::Lattice;
use crate::neighbors::neighbors_route64;
use ordered_float::OrderedFloat;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BinaryHeap, VecDeque};

/// Default limit on A* node expansions
const DEFAULT_MAX_EXPANSIONS: usize = 100_000;

/// A path found by [`BccGrid::astar`]
#[derive(Debug, Clone)]
pub struct GridPath {
    /// Sequence of cells from start to goal (inclusive)
    pub cells: Vec<Route64>,
    /// Total path length in physical units
    pub cost: f64,
}

impl GridPath {
    /// Number of cells in the path
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// A BCC lattice grid expressed in physical units.
///
/// `cell_size` is the distance between the centers of axially adjacent cells,
/// so the grid behaves like a cubic grid of that resolution while providing
/// 14-neighbor BCC connectivity. Point-to-cell snapping moves a point by at
/// most `≈ 0.56 × cell_size` (the BCC covering radius).
///
/// Cells are represented as [`Route64`] values at scale tier 0, which supports
/// lattice coordinates of roughly ±524k per axis — a volume of about
/// `±262k × cell_size` around the origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BccGrid {
    cell_size: f64,
}

impl BccGrid {
    /// Create a grid with the given physical cell size.
    ///
    /// Returns an error if `cell_size` is not a positive, finite number.
    pub fn new(cell_size: f64) -> Result<Self> {
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err(Error::OutOfRange(format!(
                "cell_size must be positive and finite, got {}",
                cell_size
            )));
        }
        Ok(Self { cell_size })
    }

    /// The physical distance between axially adjacent cell centers
    pub fn cell_size(&self) -> f64 {
        self.cell_size
    }

    /// Physical length of one lattice coordinate unit (half the cell size)
    #[inline]
    fn unit(&self) -> f64 {
        self.cell_size / 2.0
    }

    /// Get the cell containing (nearest to) a physical point
    pub fn cell_at(&self, x: f64, y: f64, z: f64) -> Result<Route64> {
        let u = self.unit();
        let coord = Lattice::physical_to_lattice(x / u, y / u, z / u, 0)?;
        Route64::new(0, coord.x, coord.y, coord.z)
    }

    /// Get the physical center of a cell
    pub fn center_of(&self, cell: Route64) -> (f64, f64, f64) {
        let u = self.unit();
        (
            cell.x() as f64 * u,
            cell.y() as f64 * u,
            cell.z() as f64 * u,
        )
    }

    /// Get the neighbors of a cell (14 for interior cells, fewer at the
    /// edge of the coordinate range)
    pub fn neighbors(&self, cell: Route64) -> Vec<Route64> {
        neighbors_route64(cell)
    }

    /// Physical Euclidean distance between two cell centers
    pub fn distance(&self, a: Route64, b: Route64) -> f64 {
        lattice_distance(a, b) * self.unit()
    }

    /// All cells within `k` hops of `center` (graph distance), including `center`
    pub fn k_ring(&self, center: Route64, k: usize) -> Vec<Route64> {
        self.bfs_collect(center, k, false)
    }

    /// All cells at exactly `k` hops from `center` (graph distance)
    pub fn k_shell(&self, center: Route64, k: usize) -> Vec<Route64> {
        self.bfs_collect(center, k, true)
    }

    fn bfs_collect(&self, center: Route64, k: usize, shell_only: bool) -> Vec<Route64> {
        if k == 0 {
            return vec![center];
        }

        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();
        visited.insert(center);
        queue.push_back((center, 0usize));

        let mut result = vec![];
        while let Some((cell, dist)) = queue.pop_front() {
            if !shell_only || dist == k {
                result.push(cell);
            }
            if dist < k {
                for neighbor in neighbors_route64(cell) {
                    if visited.insert(neighbor) {
                        queue.push_back((neighbor, dist + 1));
                    }
                }
            }
        }
        result
    }

    /// Find the shortest path between two cells with A*
    ///
    /// All cells are considered traversable; see [`BccGrid::astar_where`] to
    /// route around obstacles.
    pub fn astar(&self, start: Route64, goal: Route64) -> Result<GridPath> {
        self.astar_where(start, goal, |_| true)
    }

    /// Find the shortest path between two cells with A*, restricted to cells
    /// for which `traversable` returns `true`
    ///
    /// The search expands at most 100,000 nodes; use
    /// [`BccGrid::astar_with_limit`] to override.
    pub fn astar_where<F>(&self, start: Route64, goal: Route64, traversable: F) -> Result<GridPath>
    where
        F: Fn(Route64) -> bool,
    {
        self.astar_with_limit(start, goal, traversable, DEFAULT_MAX_EXPANSIONS)
    }

    /// A* with a configurable limit on node expansions
    pub fn astar_with_limit<F>(
        &self,
        start: Route64,
        goal: Route64,
        traversable: F,
        max_expansions: usize,
    ) -> Result<GridPath>
    where
        F: Fn(Route64) -> bool,
    {
        if start == goal {
            return Ok(GridPath {
                cells: vec![start],
                cost: 0.0,
            });
        }

        #[derive(PartialEq, Eq)]
        struct Node {
            cell: Route64,
            f_score: OrderedFloat<f64>,
        }
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Node {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                // Reversed for min-heap
                other.f_score.cmp(&self.f_score)
            }
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set: FxHashSet<Route64> = FxHashSet::default();
        let mut came_from: FxHashMap<Route64, Route64> = FxHashMap::default();
        let mut g_score: FxHashMap<Route64, f64> = FxHashMap::default();
        let mut expansions = 0;

        g_score.insert(start, 0.0);
        open_set.push(Node {
            cell: start,
            f_score: OrderedFloat(lattice_distance(start, goal)),
        });

        while let Some(Node { cell: current, .. }) = open_set.pop() {
            if !closed_set.insert(current) {
                continue;
            }

            expansions += 1;
            if expansions > max_expansions {
                return Err(Error::SearchLimitExceeded {
                    expansions,
                    limit: max_expansions,
                });
            }

            if current == goal {
                let mut cells = vec![current];
                let mut cursor = current;
                while let Some(&prev) = came_from.get(&cursor) {
                    cells.push(prev);
                    cursor = prev;
                }
                cells.reverse();
                let cost = g_score[&goal] * self.unit();
                return Ok(GridPath { cells, cost });
            }

            let current_g = g_score[&current];

            for neighbor in neighbors_route64(current) {
                if closed_set.contains(&neighbor) || !traversable(neighbor) {
                    continue;
                }

                let tentative_g = current_g + lattice_distance(current, neighbor);
                let neighbor_g = *g_score.get(&neighbor).unwrap_or(&f64::INFINITY);

                if tentative_g < neighbor_g {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g);
                    open_set.push(Node {
                        cell: neighbor,
                        f_score: OrderedFloat(tentative_g + lattice_distance(neighbor, goal)),
                    });
                }
            }
        }

        Err(Error::NoPathFound {
            start: format!("{}", start),
            goal: format!("{}", goal),
        })
    }
}

/// Euclidean distance between two cells in lattice units
#[inline]
fn lattice_distance(a: Route64, b: Route64) -> f64 {
    let dx = (a.x() - b.x()) as f64;
    let dy = (a.y() - b.y()) as f64;
    let dz = (a.z() - b.z()) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::Parity;

    #[test]
    fn test_grid_creation() {
        assert!(BccGrid::new(0.5).is_ok());
        assert!(BccGrid::new(0.0).is_err());
        assert!(BccGrid::new(-1.0).is_err());
        assert!(BccGrid::new(f64::NAN).is_err());
        assert!(BccGrid::new(f64::INFINITY).is_err());
    }

    #[test]
    fn test_cell_at_and_center_round_trip() {
        let grid = BccGrid::new(0.5).unwrap();
        // Max snap distance is the covering radius: sqrt(5)/2 lattice units
        let max_err = (5.0_f64).sqrt() / 2.0 * grid.cell_size() / 2.0 + 1e-9;

        for &(x, y, z) in &[
            (0.0, 0.0, 0.0),
            (1.2, 3.4, 5.6),
            (-0.7, 2.3, -9.1),
            (0.24, 0.26, 0.25),
        ] {
            let cell = grid.cell_at(x, y, z).unwrap();
            let (cx, cy, cz) = grid.center_of(cell);
            let d = ((cx - x).powi(2) + (cy - y).powi(2) + (cz - z).powi(2)).sqrt();
            assert!(
                d <= max_err,
                "({}, {}, {}) snapped {} away (max {})",
                x,
                y,
                z,
                d,
                max_err
            );
            assert!(Parity::from_coords(cell.x(), cell.y(), cell.z()).is_ok());
        }
    }

    #[test]
    fn test_neighbors_and_k_ring() {
        let grid = BccGrid::new(1.0).unwrap();
        let cell = grid.cell_at(0.0, 0.0, 0.0).unwrap();

        assert_eq!(grid.neighbors(cell).len(), 14);
        assert_eq!(grid.k_ring(cell, 0), vec![cell]);
        assert_eq!(grid.k_ring(cell, 1).len(), 15); // center + 14
        assert_eq!(grid.k_shell(cell, 1).len(), 14);
        assert!(grid.k_ring(cell, 2).len() > 15);
    }

    #[test]
    fn test_distance() {
        let grid = BccGrid::new(0.5).unwrap();
        let a = grid.cell_at(0.0, 0.0, 0.0).unwrap();
        // Axially adjacent cells are exactly cell_size apart
        let b = Route64::new(0, 2, 0, 0).unwrap();
        assert!((grid.distance(a, b) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn test_astar_straight_line() {
        let grid = BccGrid::new(1.0).unwrap();
        let start = grid.cell_at(0.0, 0.0, 0.0).unwrap();
        let goal = grid.cell_at(5.0, 5.0, 5.0).unwrap();

        let path = grid.astar(start, goal).unwrap();
        assert_eq!(path.cells.first(), Some(&start));
        assert_eq!(path.cells.last(), Some(&goal));
        assert!(path.cost > 0.0);

        // Diagonal moves make this optimal: 10 hops of (1,1,1)
        assert_eq!(path.cells.len(), 11);
    }

    #[test]
    fn test_astar_routes_around_obstacles() {
        let grid = BccGrid::new(1.0).unwrap();
        let start = grid.cell_at(0.0, 0.0, 0.0).unwrap();
        let goal = grid.cell_at(3.0, 3.0, 3.0).unwrap();

        let unobstructed = grid.astar(start, goal).unwrap();

        // Block a shell of cells around the midpoint of the direct route
        let mid = Route64::new(0, 3, 3, 3).unwrap();
        let blocked: FxHashSet<Route64> = grid.k_ring(mid, 1).into_iter().collect();
        let path = grid
            .astar_where(start, goal, |c| !blocked.contains(&c) || c == goal)
            .unwrap();

        assert_eq!(path.cells.first(), Some(&start));
        assert_eq!(path.cells.last(), Some(&goal));
        for cell in &path.cells[..path.cells.len() - 1] {
            assert!(!blocked.contains(cell));
        }
        assert!(path.cost >= unobstructed.cost);
    }

    #[test]
    fn test_astar_expansion_limit() {
        let grid = BccGrid::new(1.0).unwrap();
        let start = grid.cell_at(0.0, 0.0, 0.0).unwrap();
        let goal = grid.cell_at(50.0, 50.0, 50.0).unwrap();

        let result = grid.astar_with_limit(start, goal, |_| true, 5);
        assert!(matches!(
            result,
            Err(Error::SearchLimitExceeded { limit: 5, .. })
        ));
    }
}
