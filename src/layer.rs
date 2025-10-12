//! Data layer storage and aggregation
//!
//! This module provides attribute storage for cells and aggregation operations.

use crate::error::{Error, Result};
use crate::id::CellID;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregation function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregation {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    Median,
}

/// Generic data layer for storing cell attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer<T> {
    name: String,
    data: FxHashMap<CellID, T>,
}

impl<T> Layer<T> {
    /// Create a new empty layer
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: FxHashMap::default(),
        }
    }

    /// Get layer name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Insert a value for a cell
    pub fn set(&mut self, cell: CellID, value: T) {
        self.data.insert(cell, value);
    }

    /// Get a value for a cell
    pub fn get(&self, cell: &CellID) -> Option<&T> {
        self.data.get(cell)
    }

    /// Remove a cell's value
    pub fn remove(&mut self, cell: &CellID) -> Option<T> {
        self.data.remove(cell)
    }

    /// Check if a cell has a value
    pub fn contains(&self, cell: &CellID) -> bool {
        self.data.contains_key(cell)
    }

    /// Get number of cells with data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if layer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate over all cell-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&CellID, &T)> {
        self.data.iter()
    }

    /// Get all cell IDs
    pub fn cell_ids(&self) -> impl Iterator<Item = &CellID> {
        self.data.keys()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl Layer<f64> {
    /// Aggregate values from a set of cells
    pub fn aggregate(&self, cells: &[CellID], agg: Aggregation) -> Result<f64> {
        let values: Vec<f64> = cells
            .iter()
            .filter_map(|cell| self.get(cell))
            .copied()
            .collect();

        if values.is_empty() {
            return Err(Error::InvalidAggregation("No values to aggregate".into()));
        }

        match agg {
            Aggregation::Count => Ok(values.len() as f64),
            Aggregation::Sum => Ok(values.iter().sum()),
            Aggregation::Mean => Ok(values.iter().sum::<f64>() / values.len() as f64),
            Aggregation::Min => values
                .iter()
                .copied()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .ok_or_else(|| Error::InvalidAggregation("No minimum found".into())),
            Aggregation::Max => values
                .iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .ok_or_else(|| Error::InvalidAggregation("No maximum found".into())),
            Aggregation::Median => {
                let mut sorted = values.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted.len() / 2;
                #[allow(clippy::manual_is_multiple_of)]
                if sorted.len() % 2 == 0 {
                    Ok((sorted[mid - 1] + sorted[mid]) / 2.0)
                } else {
                    Ok(sorted[mid])
                }
            }
        }
    }

    /// Roll up (aggregate) fine cells to coarse parent cells
    pub fn rollup(&self, agg: Aggregation) -> Result<Layer<f64>> {
        let mut coarse_layer = Layer::new(format!("{}_rollup", self.name));

        // Group cells by their parent
        let mut parent_groups: HashMap<CellID, Vec<f64>> = HashMap::new();

        for (cell, value) in self.iter() {
            if let Ok(parent) = cell.parent() {
                parent_groups.entry(parent).or_default().push(*value);
            }
        }

        // Aggregate each parent group
        for (parent, values) in parent_groups {
            let agg_value = match agg {
                Aggregation::Count => values.len() as f64,
                Aggregation::Sum => values.iter().sum(),
                Aggregation::Mean => values.iter().sum::<f64>() / values.len() as f64,
                Aggregation::Min => values
                    .iter()
                    .copied()
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap(),
                Aggregation::Max => values
                    .iter()
                    .copied()
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap(),
                Aggregation::Median => {
                    let mut sorted = values.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let mid = sorted.len() / 2;
                    if sorted.len() % 2 == 0 {
                        (sorted[mid - 1] + sorted[mid]) / 2.0
                    } else {
                        sorted[mid]
                    }
                }
            };
            coarse_layer.set(parent, agg_value);
        }

        Ok(coarse_layer)
    }
}

/// Flag bits for common cell properties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellFlags {
    bits: u32,
}

impl CellFlags {
    pub const NONE: u32 = 0;
    pub const BLOCKED: u32 = 1 << 0;
    pub const NO_FLY: u32 = 1 << 1;
    pub const WATER: u32 = 1 << 2;
    pub const BOUNDARY: u32 = 1 << 3;

    pub fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn has_flag(&self, flag: u32) -> bool {
        self.bits & flag != 0
    }

    pub fn set_flag(&mut self, flag: u32) {
        self.bits |= flag;
    }

    pub fn clear_flag(&mut self, flag: u32) {
        self.bits &= !flag;
    }

    pub fn is_blocked(&self) -> bool {
        self.has_flag(Self::BLOCKED)
    }

    pub fn bits(&self) -> u32 {
        self.bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_basic() {
        let mut layer = Layer::new("test");
        let cell = CellID::from_coords(0, 5, 0, 0, 0).unwrap();

        layer.set(cell, 42.0);
        assert_eq!(layer.get(&cell), Some(&42.0));
        assert_eq!(layer.len(), 1);
    }

    #[test]
    fn test_aggregation() {
        let mut layer = Layer::new("test");
        let cell1 = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let cell2 = CellID::from_coords(0, 5, 2, 2, 2).unwrap();
        let cell3 = CellID::from_coords(0, 5, 4, 4, 4).unwrap();

        layer.set(cell1, 10.0);
        layer.set(cell2, 20.0);
        layer.set(cell3, 30.0);

        let cells = vec![cell1, cell2, cell3];

        assert_eq!(layer.aggregate(&cells, Aggregation::Sum).unwrap(), 60.0);
        assert_eq!(layer.aggregate(&cells, Aggregation::Mean).unwrap(), 20.0);
        assert_eq!(layer.aggregate(&cells, Aggregation::Min).unwrap(), 10.0);
        assert_eq!(layer.aggregate(&cells, Aggregation::Max).unwrap(), 30.0);
    }

    #[test]
    fn test_flags() {
        let mut flags = CellFlags::empty();
        assert!(!flags.is_blocked());

        flags.set_flag(CellFlags::BLOCKED);
        assert!(flags.is_blocked());

        flags.clear_flag(CellFlags::BLOCKED);
        assert!(!flags.is_blocked());
    }
}
