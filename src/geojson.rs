//! GeoJSON adapter for exporting spatial IDs
//!
//! Converts Galactic128 IDs to GeoJSON format with WGS84 coordinates.

#![cfg(feature = "gis_geojson")]

use crate::error::Result;
use crate::frame::get_frame;
use crate::ids::Galactic128;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// GeoJSON export options
#[derive(Debug, Clone)]
pub struct GeoJsonOptions {
    /// Include properties (frame, tier, LOD, etc.)
    pub include_properties: bool,
    /// Coordinate precision (decimal places, default 7 ≈ 1 cm)
    pub precision: u8,
}

impl Default for GeoJsonOptions {
    fn default() -> Self {
        Self {
            include_properties: true,
            precision: 7,
        }
    }
}

/// Convert Galactic128 IDs to GeoJSON Point features
pub fn to_geojson_points(ids: &[Galactic128], opts: &GeoJsonOptions) -> Value {
    let features: Vec<Value> = ids
        .iter()
        .filter_map(|id| id_to_geojson_point(id, opts).ok())
        .collect();

    json!({
        "type": "FeatureCollection",
        "features": features
    })
}

/// Write GeoJSON LineString to file
pub fn write_geojson_linestring(
    path: &Path,
    ids: &[Galactic128],
    opts: &GeoJsonOptions,
) -> Result<()> {
    let coordinates: Result<Vec<Vec<f64>>> = ids
        .iter()
        .map(|id| {
            let (lon, lat, alt) = id_to_wgs84(id)?;
            Ok(vec![
                round_coordinate(lon, opts.precision),
                round_coordinate(lat, opts.precision),
                round_coordinate(alt, opts.precision),
            ])
        })
        .collect();

    let linestring = json!({
        "type": "Feature",
        "geometry": {
            "type": "LineString",
            "coordinates": coordinates?
        },
        "properties": if opts.include_properties {
            json!({"count": ids.len()})
        } else {
            json!({})
        }
    });

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&linestring)?.as_bytes())?;
    Ok(())
}

/// Write GeoJSON Polygon to file
pub fn write_geojson_polygon(
    path: &Path,
    rings: &[Vec<Galactic128>],
    opts: &GeoJsonOptions,
) -> Result<()> {
    let polygon_coords: Result<Vec<Vec<Vec<f64>>>> = rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|id| {
                    let (lon, lat, alt) = id_to_wgs84(id)?;
                    Ok(vec![
                        round_coordinate(lon, opts.precision),
                        round_coordinate(lat, opts.precision),
                        round_coordinate(alt, opts.precision),
                    ])
                })
                .collect()
        })
        .collect();

    let polygon = json!({
        "type": "Feature",
        "geometry": {
            "type": "Polygon",
            "coordinates": polygon_coords?
        },
        "properties": if opts.include_properties {
            json!({"rings": rings.len()})
        } else {
            json!({})
        }
    });

    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(&polygon)?.as_bytes())?;
    Ok(())
}

// Internal helpers

fn id_to_geojson_point(id: &Galactic128, opts: &GeoJsonOptions) -> Result<Value> {
    let (lon, lat, alt) = id_to_wgs84(id)?;

    let mut properties = json!({});
    if opts.include_properties {
        properties = json!({
            "frame": id.frame_id(),
            "tier": id.scale_tier(),
            "lod": id.lod(),
            "attr_usr": id.attr_usr(),
            "bech32m": id.to_bech32m()?
        });
    }

    Ok(json!({
        "type": "Feature",
        "geometry": {
            "type": "Point",
            "coordinates": [
                round_coordinate(lon, opts.precision),
                round_coordinate(lat, opts.precision),
                round_coordinate(alt, opts.precision)
            ]
        },
        "properties": properties
    }))
}

/// Convert Galactic128 coordinates to WGS84 (lon, lat, alt)
fn id_to_wgs84(id: &Galactic128) -> Result<(f64, f64, f64)> {
    // Get frame descriptor
    let frame = get_frame(id.frame_id())?;

    // Get coordinates
    let x = id.x() as f64;
    let y = id.y() as f64;
    let z = id.z() as f64;

    // Simple conversion for frame 0 (ECEF-like)
    // For v0.3.1, we assume a simple ENU-to-WGS84 approximation
    // More complex CRS transforms deferred to later versions

    if frame.name == "ECEF" {
        // Simplified ECEF to WGS84 conversion
        // In real implementation, this would use proper geodetic conversion
        let lon = (x / 111320.0).to_radians(); // approximate meters to degrees
        let lat = (y / 110540.0).to_radians();
        let alt = z;
        Ok((lon.to_degrees(), lat.to_degrees(), alt))
    } else {
        // For other frames, assume local ENU coordinates
        // Scale by base unit
        let scale = frame.base_unit;
        Ok((x * scale, y * scale, z * scale))
    }
}

fn round_coordinate(value: f64, precision: u8) -> f64 {
    let multiplier = 10f64.powi(precision as i32);
    (value * multiplier).round() / multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geojson_points() {
        let ids = vec![
            Galactic128::new(0, 0, 0, 0, 0, 0, 0, 0).unwrap(),
            Galactic128::new(0, 0, 0, 0, 0, 1000, 1000, 0).unwrap(),
        ];

        let opts = GeoJsonOptions::default();
        let geojson = to_geojson_points(&ids, &opts);

        assert_eq!(geojson["type"], "FeatureCollection");
        assert!(geojson["features"].is_array());
        assert_eq!(geojson["features"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_coordinate_precision() {
        let rounded = round_coordinate(123.456789, 2);
        assert_eq!(rounded, 123.46);

        let rounded = round_coordinate(123.456789, 5);
        assert_eq!(rounded, 123.45679);
    }

    #[test]
    fn test_geojson_properties() {
        let id = Galactic128::new(0, 5, 1, 10, 3, 100, 200, 300).unwrap();
        let opts = GeoJsonOptions {
            include_properties: true,
            precision: 7,
        };

        let point = id_to_geojson_point(&id, &opts).unwrap();
        assert!(point["properties"]["frame"].is_number());
        assert!(point["properties"]["bech32m"].is_string());
    }
}
