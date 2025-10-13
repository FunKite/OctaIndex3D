//! Frame registry for coordinate reference systems

use crate::error::{Error, Result};
use crate::ids::FrameId;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Frame descriptor with coordinate system information
#[derive(Debug, Clone, PartialEq)]
pub struct FrameDescriptor {
    /// Frame name (e.g., "ECEF", "ENU")
    pub name: String,
    /// Datum (e.g., "WGS-84")
    pub datum: String,
    /// Description
    pub description: String,
    /// Right-handed coordinate system
    pub right_handed: bool,
    /// Base unit scale at tier 0 (meters)
    pub base_unit: f64,
}

impl FrameDescriptor {
    /// Create a new frame descriptor
    pub fn new(
        name: impl Into<String>,
        datum: impl Into<String>,
        description: impl Into<String>,
        right_handed: bool,
        base_unit: f64,
    ) -> Self {
        Self {
            name: name.into(),
            datum: datum.into(),
            description: description.into(),
            right_handed,
            base_unit,
        }
    }

    /// Compute hash for conflict detection
    fn compute_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.datum.hash(&mut hasher);
        self.right_handed.hash(&mut hasher);
        self.base_unit.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}

/// Global frame registry
static FRAME_REGISTRY: Lazy<RwLock<FrameRegistry>> =
    Lazy::new(|| RwLock::new(FrameRegistry::new()));

/// Frame registry implementation
struct FrameRegistry {
    frames: HashMap<FrameId, (Arc<FrameDescriptor>, u64)>,
}

impl FrameRegistry {
    fn new() -> Self {
        let mut registry = Self {
            frames: HashMap::new(),
        };

        // Register default frames
        let ecef = FrameDescriptor::new(
            "ECEF",
            "WGS-84",
            "Earth-Centered Earth-Fixed",
            true,
            1.0,
        );
        registry
            .frames
            .insert(0, (Arc::new(ecef.clone()), ecef.compute_hash()));

        registry
    }

    fn register(&mut self, id: FrameId, desc: FrameDescriptor) -> Result<()> {
        let hash = desc.compute_hash();

        if let Some((existing, existing_hash)) = self.frames.get(&id) {
            if *existing_hash == hash {
                // Idempotent registration with identical descriptor
                return Ok(());
            } else {
                // Conflict: different descriptor for same ID
                return Err(Error::FrameConflict(id));
            }
        }

        self.frames.insert(id, (Arc::new(desc), hash));
        Ok(())
    }

    fn get(&self, id: FrameId) -> Result<Arc<FrameDescriptor>> {
        self.frames
            .get(&id)
            .map(|(desc, _)| Arc::clone(desc))
            .ok_or(Error::InvalidFrameID(id))
    }

    fn list(&self) -> Vec<(FrameId, Arc<FrameDescriptor>)> {
        self.frames
            .iter()
            .map(|(id, (desc, _))| (*id, Arc::clone(desc)))
            .collect()
    }
}

/// Register a new frame
pub fn register_frame(id: FrameId, desc: FrameDescriptor) -> Result<()> {
    FRAME_REGISTRY.write().register(id, desc)
}

/// Get a frame descriptor
pub fn get_frame(id: FrameId) -> Result<Arc<FrameDescriptor>> {
    FRAME_REGISTRY.read().get(id)
}

/// List all registered frames
pub fn list_frames() -> Vec<(FrameId, Arc<FrameDescriptor>)> {
    FRAME_REGISTRY.read().list()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_registration() {
        let desc = FrameDescriptor::new("TEST", "WGS-84", "Test frame", true, 1.0);
        register_frame(100, desc.clone()).unwrap();

        let retrieved = get_frame(100).unwrap();
        assert_eq!(retrieved.name, "TEST");
    }

    #[test]
    fn test_frame_conflict() {
        let desc1 = FrameDescriptor::new("TEST1", "WGS-84", "Test frame 1", true, 1.0);
        let desc2 = FrameDescriptor::new("TEST2", "WGS-84", "Test frame 2", true, 2.0);

        register_frame(101, desc1).unwrap();

        // Different descriptor should conflict
        let result = register_frame(101, desc2);
        assert!(result.is_err());
    }

    #[test]
    fn test_idempotent_registration() {
        let desc = FrameDescriptor::new("TEST3", "WGS-84", "Test frame 3", true, 1.0);

        register_frame(102, desc.clone()).unwrap();
        // Identical registration should succeed
        register_frame(102, desc).unwrap();
    }

    #[test]
    fn test_default_frame() {
        // Frame 0 (ECEF) should be registered by default
        let frame = get_frame(0).unwrap();
        assert_eq!(frame.name, "ECEF");
    }
}
