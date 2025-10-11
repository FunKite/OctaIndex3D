//! Pathfinding and routing algorithms
//!
//! This module provides A* pathfinding, k-rings, and other routing operations.

use crate::error::{Error, Result};
use crate::id::CellID;
use crate::layer::{CellFlags, Layer};
use crate::lattice::LatticeCoord;
use ordered_float::OrderedFloat;
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// Cost function trait for pathfinding
pub trait CostFn {
    /// Compute cost of moving from current cell to neighbor
    fn cost(&self, current: CellID, neighbor: CellID) -> f64;

    /// Heuristic estimate of cost from current to goal
    fn heuristic(&self, current: CellID, goal: CellID) -> f64;
}

/// Simple Euclidean distance cost function
#[derive(Debug, Clone, Copy)]
pub struct EuclideanCost;

impl CostFn for EuclideanCost {
    fn cost(&self, current: CellID, neighbor: CellID) -> f64 {
        let c1 = current.lattice_coord().unwrap();
        let c2 = neighbor.lattice_coord().unwrap();
        c1.distance_to(&c2)
    }

    fn heuristic(&self, current: CellID, goal: CellID) -> f64 {
        let c1 = current.lattice_coord().unwrap();
        let c2 = goal.lattice_coord().unwrap();
        c1.distance_to(&c2)
    }
}

/// Cost function that avoids blocked cells
pub struct AvoidBlockedCost {
    flags: Layer<CellFlags>,
    blocked_penalty: f64,
}

impl AvoidBlockedCost {
    pub fn new(flags: Layer<CellFlags>, blocked_penalty: f64) -> Self {
        Self {
            flags,
            blocked_penalty,
        }
    }
}

impl CostFn for AvoidBlockedCost {
    fn cost(&self, current: CellID, neighbor: CellID) -> f64 {
        let base_cost = {
            let c1 = current.lattice_coord().unwrap();
            let c2 = neighbor.lattice_coord().unwrap();
            c1.distance_to(&c2)
        };

        if let Some(flags) = self.flags.get(&neighbor) {
            if flags.is_blocked() {
                return f64::INFINITY; // Cannot traverse
            }
        }

        base_cost
    }

    fn heuristic(&self, current: CellID, goal: CellID) -> f64 {
        let c1 = current.lattice_coord().unwrap();
        let c2 = goal.lattice_coord().unwrap();
        c1.distance_to(&c2)
    }
}

/// A* pathfinding result
#[derive(Debug, Clone)]
pub struct Path {
    /// Sequence of cells from start to goal
    pub cells: Vec<CellID>,
    /// Total cost of the path
    pub cost: f64,
}

impl Path {
    /// Get path length (number of cells)
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// A* pathfinding state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AStarNode {
    cell: CellID,
    f_score: OrderedFloat<f64>, // f = g + h
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min-heap
        other.f_score.cmp(&self.f_score)
    }
}

/// A* pathfinding algorithm
pub fn astar<C: CostFn>(start: CellID, goal: CellID, cost_fn: &C) -> Result<Path> {
    if start == goal {
        return Ok(Path {
            cells: vec![start],
            cost: 0.0,
        });
    }

    let mut open_set = BinaryHeap::new();
    let mut came_from: FxHashMap<CellID, CellID> = FxHashMap::default();
    let mut g_score: FxHashMap<CellID, f64> = FxHashMap::default();

    g_score.insert(start, 0.0);
    let h_start = cost_fn.heuristic(start, goal);
    open_set.push(AStarNode {
        cell: start,
        f_score: OrderedFloat(h_start),
    });

    while let Some(AStarNode { cell: current, .. }) = open_set.pop() {
        if current == goal {
            // Reconstruct path
            let mut path = vec![current];
            let mut current = current;
            while let Some(&prev) = came_from.get(&current) {
                path.push(prev);
                current = prev;
            }
            path.reverse();

            let cost = *g_score.get(&goal).unwrap();
            return Ok(Path { cells: path, cost });
        }

        let current_g = *g_score.get(&current).unwrap_or(&f64::INFINITY);

        for neighbor in current.neighbors() {
            let edge_cost = cost_fn.cost(current, neighbor);
            if edge_cost.is_infinite() {
                continue; // Skip blocked cells
            }

            let tentative_g = current_g + edge_cost;
            let neighbor_g = *g_score.get(&neighbor).unwrap_or(&f64::INFINITY);

            if tentative_g < neighbor_g {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative_g);
                let h = cost_fn.heuristic(neighbor, goal);
                let f = tentative_g + h;

                open_set.push(AStarNode {
                    cell: neighbor,
                    f_score: OrderedFloat(f),
                });
            }
        }
    }

    Err(Error::NoPathFound {
        start: format!("{}", start),
        goal: format!("{}", goal),
    })
}

/// Compute k-ring: all cells within k steps (graph distance)
pub fn k_ring(center: CellID, k: usize) -> Vec<CellID> {
    if k == 0 {
        return vec![center];
    }

    let mut visited = FxHashSet::default();
    let mut queue = VecDeque::new();

    visited.insert(center);
    queue.push_back((center, 0));

    let mut result = vec![];

    while let Some((cell, dist)) = queue.pop_front() {
        result.push(cell);

        if dist < k {
            for neighbor in cell.neighbors() {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }
    }

    result
}

/// Compute k-shell: all cells at exactly k steps (graph distance)
pub fn k_shell(center: CellID, k: usize) -> Vec<CellID> {
    if k == 0 {
        return vec![center];
    }

    let mut visited = FxHashSet::default();
    let mut queue = VecDeque::new();

    visited.insert(center);
    queue.push_back((center, 0));

    let mut result = vec![];

    while let Some((cell, dist)) = queue.pop_front() {
        if dist == k {
            result.push(cell);
        }

        if dist < k {
            for neighbor in cell.neighbors() {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }
    }

    result
}

/// Compute cells along a line between two cells (3D Bresenham-like)
pub fn trace_line(start: CellID, end: CellID) -> Result<Vec<CellID>> {
    if start == end {
        return Ok(vec![start]);
    }

    let coord_start = start.lattice_coord()?;
    let coord_end = end.lattice_coord()?;

    let mut cells = vec![start];

    // Simple sampling approach: sample along line and find unique cells
    let dx = (coord_end.x - coord_start.x) as f64;
    let dy = (coord_end.y - coord_start.y) as f64;
    let dz = (coord_end.z - coord_start.z) as f64;

    let max_steps = (dx.abs().max(dy.abs()).max(dz.abs()) * 2.0) as usize;

    let mut prev_cell = start;

    for i in 1..=max_steps {
        let t = i as f64 / max_steps as f64;
        let x = coord_start.x as f64 + t * dx;
        let y = coord_start.y as f64 + t * dy;
        let z = coord_start.z as f64 + t * dz;

        if let Ok(coord) = crate::lattice::Lattice::physical_to_lattice(x, y, z, start.resolution())
        {
            if let Ok(cell) = CellID::from_lattice_coord(start.frame(), start.resolution(), &coord)
            {
                if cell != prev_cell {
                    cells.push(cell);
                    prev_cell = cell;
                }
            }
        }
    }

    // Ensure end cell is included
    if cells.last() != Some(&end) {
        cells.push(end);
    }

    Ok(cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_astar_simple() {
        let start = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let goal = CellID::from_coords(0, 5, 10, 10, 10).unwrap();

        let cost_fn = EuclideanCost;
        let path = astar(start, goal, &cost_fn).unwrap();

        assert!(!path.is_empty());
        assert_eq!(path.cells.first(), Some(&start));
        assert_eq!(path.cells.last(), Some(&goal));
    }

    #[test]
    fn test_k_ring() {
        let center = CellID::from_coords(0, 5, 0, 0, 0).unwrap();

        // k=0 should return just the center
        let ring0 = k_ring(center, 0);
        assert_eq!(ring0.len(), 1);
        assert_eq!(ring0[0], center);

        // k=1 should return center + 14 neighbors
        let ring1 = k_ring(center, 1);
        assert_eq!(ring1.len(), 15); // center + 14 neighbors

        // k=2 should return more cells
        let ring2 = k_ring(center, 2);
        assert!(ring2.len() > ring1.len());
    }

    #[test]
    fn test_k_shell() {
        let center = CellID::from_coords(0, 5, 0, 0, 0).unwrap();

        // k=0 should return just the center
        let shell0 = k_shell(center, 0);
        assert_eq!(shell0.len(), 1);

        // k=1 should return exactly 14 neighbors
        let shell1 = k_shell(center, 1);
        assert_eq!(shell1.len(), 14);
    }

    #[test]
    fn test_trace_line() {
        let start = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let end = CellID::from_coords(0, 5, 10, 0, 0).unwrap();

        let line = trace_line(start, end).unwrap();
        assert!(!line.is_empty());
        assert_eq!(line.first(), Some(&start));
        assert_eq!(line.last(), Some(&end));
    }
}
