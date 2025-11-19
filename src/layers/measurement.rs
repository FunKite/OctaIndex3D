//! Sensor measurement types for spatial mapping
//!
//! This module defines different types of sensor measurements that can be
//! integrated into spatial layers (TSDF, Occupancy, etc.).

use crate::error::{Error, Result};

/// Type of sensor measurement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementType {
    /// Depth measurement (e.g., depth camera, LiDAR)
    Depth,
    /// Occupancy observation (occupied/free)
    Occupancy,
    /// RGB color
    Color,
    /// Intensity (LiDAR reflectivity)
    Intensity,
}

/// Sensor measurement for spatial updates
#[derive(Debug, Clone)]
pub struct Measurement {
    /// Type of measurement
    pub measurement_type: MeasurementType,

    /// Measurement-specific data
    pub data: MeasurementData,

    /// Confidence/weight (0.0 to 1.0)
    pub confidence: f32,
}

/// Measurement data variants
#[derive(Debug, Clone)]
pub enum MeasurementData {
    /// Depth measurement
    Depth {
        /// Distance to surface (meters)
        distance: f32,
        /// Surface normal (optional)
        normal: Option<[f32; 3]>,
    },

    /// Occupancy observation
    Occupancy {
        /// True if occupied, false if free
        occupied: bool,
    },

    /// RGB color
    Color {
        /// Red channel [0, 255]
        r: u8,
        /// Green channel [0, 255]
        g: u8,
        /// Blue channel [0, 255]
        b: u8,
    },

    /// Intensity
    Intensity {
        /// Intensity value [0.0, 1.0]
        value: f32,
    },
}

impl Measurement {
    /// Create a depth measurement
    ///
    /// # Arguments
    /// * `distance` - Distance to surface in meters (positive)
    /// * `confidence` - Measurement confidence [0.0, 1.0]
    ///
    /// # Example
    /// ```
    /// use octaindex3d::layers::Measurement;
    ///
    /// let measurement = Measurement::depth(2.5, 1.0); // 2.5m, full confidence
    /// ```
    pub fn depth(distance: f32, confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Depth,
            data: MeasurementData::Depth {
                distance,
                normal: None,
            },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create a depth measurement with surface normal
    pub fn depth_with_normal(distance: f32, normal: [f32; 3], confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Depth,
            data: MeasurementData::Depth {
                distance,
                normal: Some(normal),
            },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create an occupancy measurement (occupied)
    pub fn occupied(confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Occupancy,
            data: MeasurementData::Occupancy { occupied: true },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create an occupancy measurement (free)
    pub fn free(confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Occupancy,
            data: MeasurementData::Occupancy { occupied: false },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create a color measurement
    pub fn color(r: u8, g: u8, b: u8, confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Color,
            data: MeasurementData::Color { r, g, b },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Create an intensity measurement
    pub fn intensity(value: f32, confidence: f32) -> Self {
        Self {
            measurement_type: MeasurementType::Intensity,
            data: MeasurementData::Intensity { value },
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Get depth value if this is a depth measurement
    pub fn as_depth(&self) -> Result<f32> {
        match &self.data {
            MeasurementData::Depth { distance, .. } => Ok(*distance),
            _ => Err(Error::InvalidFormat("Not a depth measurement".to_string())),
        }
    }

    /// Get surface normal if available
    pub fn surface_normal(&self) -> Option<[f32; 3]> {
        match &self.data {
            MeasurementData::Depth { normal, .. } => *normal,
            _ => None,
        }
    }

    /// Check if this is an occupied measurement
    pub fn is_occupied(&self) -> Result<bool> {
        match &self.data {
            MeasurementData::Occupancy { occupied } => Ok(*occupied),
            _ => Err(Error::InvalidFormat(
                "Not an occupancy measurement".to_string(),
            )),
        }
    }

    /// Get RGB color if this is a color measurement
    pub fn as_rgb(&self) -> Result<(u8, u8, u8)> {
        match &self.data {
            MeasurementData::Color { r, g, b } => Ok((*r, *g, *b)),
            _ => Err(Error::InvalidFormat("Not a color measurement".to_string())),
        }
    }

    /// Get intensity value if this is an intensity measurement
    pub fn as_intensity(&self) -> Result<f32> {
        match &self.data {
            MeasurementData::Intensity { value } => Ok(*value),
            _ => Err(Error::InvalidFormat(
                "Not an intensity measurement".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_measurement() {
        let m = Measurement::depth(2.5, 1.0);
        assert_eq!(m.measurement_type, MeasurementType::Depth);
        assert_eq!(m.as_depth().unwrap(), 2.5);
        assert_eq!(m.confidence, 1.0);
        assert!(m.surface_normal().is_none());
    }

    #[test]
    fn test_depth_with_normal() {
        let normal = [0.0, 0.0, 1.0];
        let m = Measurement::depth_with_normal(1.5, normal, 0.9);
        assert_eq!(m.as_depth().unwrap(), 1.5);
        assert_eq!(m.surface_normal().unwrap(), normal);
    }

    #[test]
    fn test_occupancy_measurement() {
        let occupied = Measurement::occupied(0.8);
        assert!(occupied.is_occupied().unwrap());
        assert_eq!(occupied.confidence, 0.8);

        let free = Measurement::free(0.9);
        assert!(!free.is_occupied().unwrap());
    }

    #[test]
    fn test_color_measurement() {
        let m = Measurement::color(255, 128, 64, 1.0);
        let (r, g, b) = m.as_rgb().unwrap();
        assert_eq!(r, 255);
        assert_eq!(g, 128);
        assert_eq!(b, 64);
    }

    #[test]
    fn test_confidence_clamping() {
        let m1 = Measurement::depth(1.0, 1.5); // Over 1.0
        assert_eq!(m1.confidence, 1.0);

        let m2 = Measurement::depth(1.0, -0.5); // Under 0.0
        assert_eq!(m2.confidence, 0.0);
    }
}
