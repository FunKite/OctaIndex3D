//! I/O operations for various file formats
//!
//! This module provides import/export functionality for different formats.

use crate::error::{Error, Result};
use crate::id::CellID;
use crate::layer::Layer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Cell data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    pub frame: u8,
    pub resolution: u8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub value: f64,
}

impl CellData {
    pub fn from_cell_value(cell: CellID, value: f64) -> Self {
        Self {
            frame: cell.frame(),
            resolution: cell.resolution(),
            x: cell.x(),
            y: cell.y(),
            z: cell.z(),
            value,
        }
    }

    pub fn to_cell(&self) -> Result<CellID> {
        CellID::from_coords(self.frame, self.resolution, self.x, self.y, self.z)
    }
}

/// Dataset container for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub cells: Vec<CellData>,
}

impl Dataset {
    pub fn from_layer(layer: &Layer<f64>) -> Self {
        let cells: Vec<CellData> = layer
            .iter()
            .map(|(cell, value)| CellData::from_cell_value(*cell, *value))
            .collect();

        Self {
            name: layer.name().to_string(),
            cells,
        }
    }

    pub fn to_layer(&self) -> Result<Layer<f64>> {
        let mut layer = Layer::new(&self.name);

        for cell_data in &self.cells {
            let cell = cell_data.to_cell()?;
            layer.set(cell, cell_data.value);
        }

        Ok(layer)
    }

    /// Save to JSON file
    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).map_err(|e| Error::IoError(e.to_string()))?;
        Ok(())
    }

    /// Load from JSON file
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| Error::IoError(e.to_string()))
    }

    /// Save to CBOR file
    pub fn save_cbor<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_cbor::to_writer(writer, self).map_err(|e| Error::IoError(e.to_string()))?;
        Ok(())
    }

    /// Load from CBOR file
    pub fn load_cbor<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        serde_cbor::from_reader(reader).map_err(|e| Error::IoError(e.to_string()))
    }
}

/// GeoJSON point feature (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoJsonPoint {
    #[serde(rename = "type")]
    pub type_: String,
    pub coordinates: [f64; 3],
    pub properties: Option<serde_json::Value>,
}

/// Export cells as GeoJSON points
pub fn export_cells_geojson(cells: &[CellID]) -> Result<String> {
    let features: Vec<serde_json::Value> = cells
        .iter()
        .map(|cell| {
            let coord = cell.lattice_coord().unwrap();
            let (x, y, z) = coord.to_physical();

            serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [x, y, z]
                },
                "properties": {
                    "cell_id": cell.to_bech32m().ok(),
                    "frame": cell.frame(),
                    "resolution": cell.resolution()
                }
            })
        })
        .collect();

    let geojson = serde_json::json!({
        "type": "FeatureCollection",
        "features": features
    });

    serde_json::to_string_pretty(&geojson).map_err(|e| Error::IoError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_layer_roundtrip() {
        let mut layer = Layer::new("test");
        let cell1 = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let cell2 = CellID::from_coords(0, 5, 2, 2, 2).unwrap();

        layer.set(cell1, 42.0);
        layer.set(cell2, 84.0);

        let dataset = Dataset::from_layer(&layer);
        assert_eq!(dataset.cells.len(), 2);

        let layer2 = dataset.to_layer().unwrap();
        assert_eq!(layer2.get(&cell1), Some(&42.0));
        assert_eq!(layer2.get(&cell2), Some(&84.0));
    }

    #[test]
    fn test_geojson_export() {
        let cell1 = CellID::from_coords(0, 5, 0, 0, 0).unwrap();
        let cell2 = CellID::from_coords(0, 5, 2, 2, 2).unwrap();

        let cells = vec![cell1, cell2];
        let geojson = export_cells_geojson(&cells).unwrap();

        assert!(geojson.contains("FeatureCollection"));
        assert!(geojson.contains("Point"));
    }
}
