//! Metal compute shader for GPU-accelerated occupancy ray casting
//!
//! Performs parallel ray tracing on BCC lattice for occupancy mapping

#include <metal_stdlib>
using namespace metal;

/// Ray casting parameters
struct RayParams {
    float voxel_size;
    float free_confidence;
    float occupied_confidence;
    uint ray_count;
};

/// Convert world coordinates to BCC voxel coordinates
inline int3 world_to_voxel(float3 pos, float voxel_size) {
    return int3(floor(pos.x / voxel_size),
                floor(pos.y / voxel_size),
                floor(pos.z / voxel_size));
}

/// Check if voxel coordinates are valid BCC
inline bool is_valid_bcc(int3 voxel) {
    return ((voxel.x + voxel.y + voxel.z) % 2) == 0;
}

/// Snap to nearest valid BCC voxel
inline int3 snap_to_bcc(int3 voxel) {
    int sum = voxel.x + voxel.y + voxel.z;
    if (sum % 2 == 0) {
        return voxel; // Already valid
    }

    // Try all 6 neighbors and pick closest
    int3 candidates[6] = {
        int3(voxel.x + 1, voxel.y, voxel.z),
        int3(voxel.x - 1, voxel.y, voxel.z),
        int3(voxel.x, voxel.y + 1, voxel.z),
        int3(voxel.x, voxel.y - 1, voxel.z),
        int3(voxel.x, voxel.y, voxel.z + 1),
        int3(voxel.x, voxel.y, voxel.z - 1)
    };

    int3 best = candidates[0];
    int best_dist = 999999;

    for (int i = 0; i < 6; i++) {
        int3 c = candidates[i];
        if (is_valid_bcc(c)) {
            int3 diff = c - voxel;
            int dist = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
            if (dist < best_dist) {
                best_dist = dist;
                best = c;
            }
        }
    }

    return best;
}

/// DDA ray traversal on BCC lattice
///
/// Outputs voxels along ray from origin to endpoint
kernel void cast_occupancy_rays(
    constant float3* origins [[buffer(0)]],
    constant float3* endpoints [[buffer(1)]],
    constant RayParams& params [[buffer(2)]],
    device int3* free_voxels [[buffer(3)]],      // Free space voxels
    device int3* occupied_voxels [[buffer(4)]],  // Occupied endpoint voxels
    device uint* free_count [[buffer(5)]],       // Number of free voxels
    device uint* occupied_count [[buffer(6)]],   // Number of occupied voxels
    uint gid [[thread_position_in_grid]])
{
    if (gid >= params.ray_count) return;

    float3 origin = origins[gid];
    float3 endpoint = endpoints[gid];

    // Convert to voxel coordinates
    int3 start_voxel = snap_to_bcc(world_to_voxel(origin, params.voxel_size));
    int3 end_voxel = snap_to_bcc(world_to_voxel(endpoint, params.voxel_size));

    // Ray direction
    float3 dir = endpoint - origin;
    float length = sqrt(dir.x * dir.x + dir.y * dir.y + dir.z * dir.z);
    if (length < 0.001) return;

    dir = dir / length; // Normalize

    // DDA setup
    int3 voxel = start_voxel;
    int3 step = int3(dir.x >= 0 ? 1 : -1,
                     dir.y >= 0 ? 1 : -1,
                     dir.z >= 0 ? 1 : -1);

    float3 delta = float3(
        abs(dir.x) > 0.001 ? params.voxel_size / abs(dir.x) : 1e10,
        abs(dir.y) > 0.001 ? params.voxel_size / abs(dir.y) : 1e10,
        abs(dir.z) > 0.001 ? params.voxel_size / abs(dir.z) : 1e10
    );

    float3 t_max = float3(
        abs(dir.x) > 0.001 ? ((voxel.x + (step.x > 0 ? 1 : 0)) * params.voxel_size - origin.x) / dir.x : 1e10,
        abs(dir.y) > 0.001 ? ((voxel.y + (step.y > 0 ? 1 : 0)) * params.voxel_size - origin.y) / dir.y : 1e10,
        abs(dir.z) > 0.001 ? ((voxel.z + (step.z > 0 ? 1 : 0)) * params.voxel_size - origin.z) / dir.z : 1e10
    );

    // Traverse ray (max 1000 voxels per ray)
    for (int i = 0; i < 1000; i++) {
        // Check if we've reached endpoint
        if (voxel.x == end_voxel.x &&
            voxel.y == end_voxel.y &&
            voxel.z == end_voxel.z) {
            break;
        }

        // Mark current voxel as free (if not the endpoint)
        if (is_valid_bcc(voxel)) {
            uint idx = atomic_fetch_add_explicit(free_count, 1, memory_order_relaxed);
            if (idx < 1000000) { // Safety limit
                free_voxels[idx] = voxel;
            }
        }

        // Step to next voxel (DDA)
        if (t_max.x < t_max.y) {
            if (t_max.x < t_max.z) {
                voxel.x += step.x;
                t_max.x += delta.x;
            } else {
                voxel.z += step.z;
                t_max.z += delta.z;
            }
        } else {
            if (t_max.y < t_max.z) {
                voxel.y += step.y;
                t_max.y += delta.y;
            } else {
                voxel.z += step.z;
                t_max.z += delta.z;
            }
        }

        // Safety check: don't go too far
        int3 diff = voxel - start_voxel;
        int dist_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
        if (dist_sq > 1000000) break;
    }

    // Mark endpoint as occupied
    if (is_valid_bcc(end_voxel)) {
        uint idx = atomic_fetch_add_explicit(occupied_count, 1, memory_order_relaxed);
        if (idx < 100000) { // Safety limit
            occupied_voxels[idx] = end_voxel;
        }
    }
}

/// Batch update log-odds values for occupancy voxels
kernel void batch_update_log_odds(
    device float* log_odds [[buffer(0)]],     // Current log-odds values
    constant int3* voxels [[buffer(1)]],      // Voxel coordinates
    constant float* updates [[buffer(2)]],    // Log-odds updates
    constant uint& count [[buffer(3)]],       // Number of updates
    uint gid [[thread_position_in_grid]])
{
    if (gid >= count) return;

    int3 voxel = voxels[gid];
    float update = updates[gid];

    // Simple hash for voxel lookup (in real impl would use proper hash map)
    uint hash = ((uint)voxel.x * 73856093) ^
                ((uint)voxel.y * 19349663) ^
                ((uint)voxel.z * 83492791);
    uint idx = hash % 1000000; // Safety limit

    // Atomic add to log-odds
    atomic_fetch_add_explicit((device atomic_float*)&log_odds[idx],
                             update,
                             memory_order_relaxed);
}
