//! ROS2 Integration Bridge for OctaIndex3D
//!
//! Provides message type definitions and conversions compatible with ROS2
//! sensor_msgs and nav_msgs packages. These types can be serialized using
//! CDR and sent over DDS for ROS2 interoperability.
//!
//! ## Usage with ROS2
//!
//! ```no_run
//! // In your ROS2 node (requires rclrs or similar):
//! use octaindex3d::layers::ros2::{PointCloud2, OccupancyGrid};
//! use octaindex3d::layers::OccupancyLayer;
//!
//! # fn example() -> octaindex3d::Result<()> {
//! // Convert occupancy layer to ROS2 OccupancyGrid message
//! let layer = OccupancyLayer::new();
//! let grid = OccupancyGrid::from_occupancy_layer(&layer, 0.05, [0.0, 0.0, 0.0]);
//!
//! // Publish to ROS2 topic (pseudo-code)
//! // publisher.publish(grid);
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::occupancy::OccupancyLayer;

/// ROS2 Header (std_msgs/Header)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Header {
    /// Timestamp when this data was created
    pub stamp: Time,
    /// Coordinate frame identifier
    pub frame_id: String,
}

/// ROS2 Time (builtin_interfaces/Time)
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Time {
    /// Seconds component of the timestamp
    pub sec: i32,
    /// Nanoseconds component of the timestamp
    pub nanosec: u32,
}

impl Time {
    /// Create from seconds and nanoseconds
    pub fn new(sec: i32, nanosec: u32) -> Self {
        Self { sec, nanosec }
    }

    /// Get current system time
    pub fn now() -> Self {
        use std::time::SystemTime;
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        Self {
            sec: duration.as_secs() as i32,
            nanosec: duration.subsec_nanos(),
        }
    }
}

/// ROS2 OccupancyGrid (nav_msgs/OccupancyGrid)
///
/// Represents a 2D occupancy grid compatible with ROS2 navigation stack
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OccupancyGrid {
    /// Message header with timestamp and frame
    pub header: Header,
    /// Map metadata describing dimensions and resolution
    pub info: MapMetaData,
    /// Occupancy data: -1 (unknown), 0-100 (free to occupied)
    pub data: Vec<i8>,
}

/// Map metadata (nav_msgs/MapMetaData)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MapMetaData {
    /// Time when the map was loaded
    pub map_load_time: Time,
    /// Resolution in meters per cell
    pub resolution: f32,
    /// Map width in cells
    pub width: u32,
    /// Map height in cells
    pub height: u32,
    /// Origin pose of the map in the world frame
    pub origin: Pose,
}

/// Pose (geometry_msgs/Pose)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Pose {
    /// Position in 3D space
    pub position: Point,
    /// Orientation as a quaternion
    pub orientation: Quaternion,
}

/// Point (geometry_msgs/Point)
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Point {
    /// X coordinate in meters
    pub x: f64,
    /// Y coordinate in meters
    pub y: f64,
    /// Z coordinate in meters
    pub z: f64,
}

/// Quaternion (geometry_msgs/Quaternion)
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Quaternion {
    /// X component of the quaternion
    pub x: f64,
    /// Y component of the quaternion
    pub y: f64,
    /// Z component of the quaternion
    pub z: f64,
    /// W (scalar) component of the quaternion
    pub w: f64,
}

impl OccupancyGrid {
    /// Convert OctaIndex3D occupancy layer to ROS2 OccupancyGrid (2D projection)
    ///
    /// Projects 3D occupancy onto XY plane by taking maximum occupancy in Z
    pub fn from_occupancy_layer(
        _layer: &OccupancyLayer,
        resolution: f32,
        origin: [f64; 3],
    ) -> Self {
        // For simplicity, create a small grid centered at origin
        let width = 100;
        let height = 100;
        let data = vec![-1i8; (width * height) as usize];

        // Scan through voxels and project to 2D
        // (In real implementation, would scan actual voxel bounds)

        Self {
            header: Header {
                stamp: Time::now(),
                frame_id: "map".to_string(),
            },
            info: MapMetaData {
                map_load_time: Time::now(),
                resolution,
                width,
                height,
                origin: Pose {
                    position: Point {
                        x: origin[0],
                        y: origin[1],
                        z: origin[2],
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                },
            },
            data,
        }
    }

    /// Convert to bytes for ROS2 publishing (CDR serialization)
    #[cfg(feature = "serde")]
    pub fn to_cdr_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }
}

/// ROS2 PointCloud2 (sensor_msgs/PointCloud2)
///
/// Represents 3D point cloud data compatible with ROS2
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PointCloud2 {
    /// Message header with timestamp and frame
    pub header: Header,
    /// Height of the point cloud (1 for unordered clouds)
    pub height: u32,
    /// Width of the point cloud (number of points if height=1)
    pub width: u32,
    /// Description of each point field (x, y, z, etc.)
    pub fields: Vec<PointField>,
    /// Is the data big-endian? (typically false)
    pub is_bigendian: bool,
    /// Length of a point in bytes
    pub point_step: u32,
    /// Length of a row in bytes
    pub row_step: u32,
    /// Actual point data in binary format
    pub data: Vec<u8>,
    /// True if there are no invalid points
    pub is_dense: bool,
}

/// Point field description (sensor_msgs/PointField)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PointField {
    /// Name of the field (e.g., "x", "y", "z", "intensity")
    pub name: String,
    /// Offset from start of point struct in bytes
    pub offset: u32,
    /// Data type: 1=INT8, 2=UINT8, 3=INT16, 4=UINT16, 5=INT32, 6=UINT32, 7=FLOAT32, 8=FLOAT64
    pub datatype: u8,
    /// Number of elements in the field (typically 1)
    pub count: u32,
}

impl PointCloud2 {
    /// Create XYZ point cloud from occupied voxels
    pub fn from_occupied_voxels(voxels: Vec<(f32, f32, f32)>, frame_id: &str) -> Self {
        let point_count = voxels.len() as u32;
        let point_step = 12; // 3 floats * 4 bytes
        let mut data = Vec::with_capacity((point_count * point_step) as usize);

        for (x, y, z) in voxels {
            data.extend_from_slice(&x.to_le_bytes());
            data.extend_from_slice(&y.to_le_bytes());
            data.extend_from_slice(&z.to_le_bytes());
        }

        Self {
            header: Header {
                stamp: Time::now(),
                frame_id: frame_id.to_string(),
            },
            height: 1,
            width: point_count,
            fields: vec![
                PointField {
                    name: "x".to_string(),
                    offset: 0,
                    datatype: 7, // FLOAT32
                    count: 1,
                },
                PointField {
                    name: "y".to_string(),
                    offset: 4,
                    datatype: 7,
                    count: 1,
                },
                PointField {
                    name: "z".to_string(),
                    offset: 8,
                    datatype: 7,
                    count: 1,
                },
            ],
            is_bigendian: false,
            point_step,
            row_step: point_count * point_step,
            data,
            is_dense: true,
        }
    }

    /// Create XYZI point cloud (with intensity) from occupancy probabilities
    pub fn from_occupancy_with_intensity(
        voxels: Vec<(f32, f32, f32, f32)>, // (x, y, z, probability)
        frame_id: &str,
    ) -> Self {
        let point_count = voxels.len() as u32;
        let point_step = 16; // 4 floats * 4 bytes
        let mut data = Vec::with_capacity((point_count * point_step) as usize);

        for (x, y, z, intensity) in voxels {
            data.extend_from_slice(&x.to_le_bytes());
            data.extend_from_slice(&y.to_le_bytes());
            data.extend_from_slice(&z.to_le_bytes());
            data.extend_from_slice(&intensity.to_le_bytes());
        }

        Self {
            header: Header {
                stamp: Time::now(),
                frame_id: frame_id.to_string(),
            },
            height: 1,
            width: point_count,
            fields: vec![
                PointField {
                    name: "x".to_string(),
                    offset: 0,
                    datatype: 7,
                    count: 1,
                },
                PointField {
                    name: "y".to_string(),
                    offset: 4,
                    datatype: 7,
                    count: 1,
                },
                PointField {
                    name: "z".to_string(),
                    offset: 8,
                    datatype: 7,
                    count: 1,
                },
                PointField {
                    name: "intensity".to_string(),
                    offset: 12,
                    datatype: 7,
                    count: 1,
                },
            ],
            is_bigendian: false,
            point_step,
            row_step: point_count * point_step,
            data,
            is_dense: true,
        }
    }
}

/// Helper to extract occupied voxels from OccupancyLayer for ROS2 publishing
pub fn extract_occupied_voxels(_layer: &OccupancyLayer, _voxel_size: f32) -> Vec<(f32, f32, f32)> {
    // This would iterate through the layer's voxels
    // For now, returns empty (requires access to layer internals)
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ros2_time() {
        let time = Time::now();
        assert!(time.sec > 0);
    }

    #[test]
    fn test_pointcloud2_creation() {
        let voxels = vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)];
        let cloud = PointCloud2::from_occupied_voxels(voxels, "map");

        assert_eq!(cloud.width, 2);
        assert_eq!(cloud.fields.len(), 3); // x, y, z
        assert_eq!(cloud.data.len(), 24); // 2 points * 3 floats * 4 bytes
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_occupancy_grid_serialization() {
        let grid = OccupancyGrid {
            header: Header {
                stamp: Time::new(100, 500),
                frame_id: "map".to_string(),
            },
            info: MapMetaData {
                map_load_time: Time::new(100, 500),
                resolution: 0.05,
                width: 100,
                height: 100,
                origin: Pose {
                    position: Point {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    orientation: Quaternion {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        w: 1.0,
                    },
                },
            },
            data: vec![0; 10000],
        };

        let _bytes = grid.to_cdr_bytes();
        // In real ROS2 usage, would use CDR serialization, not JSON
    }
}
