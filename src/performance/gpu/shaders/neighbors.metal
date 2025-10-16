//! Metal compute shader for batch neighbor calculations
//!
//! This shader calculates the 14 BCC lattice neighbors for each input route
//! in parallel on the GPU.

#include <metal_stdlib>
using namespace metal;

// BCC lattice neighbor offsets (14 nearest neighbors)
// Format: {x, y, z, w} where w is padding
constant int4 BCC_NEIGHBOR_OFFSETS[14] = {
    int4( 1,  1,  1, 0),  // Diagonal neighbors (8)
    int4( 1,  1, -1, 0),
    int4( 1, -1,  1, 0),
    int4( 1, -1, -1, 0),
    int4(-1,  1,  1, 0),
    int4(-1,  1, -1, 0),
    int4(-1, -1,  1, 0),
    int4(-1, -1, -1, 0),
    int4( 2,  0,  0, 0),  // Axis-aligned neighbors (6)
    int4(-2,  0,  0, 0),
    int4( 0,  2,  0, 0),
    int4( 0, -2,  0, 0),
    int4( 0,  0,  2, 0),
    int4( 0,  0, -2, 0),
};

// Extract coordinates from Route64 value
inline int3 decode_route64(ulong value) {
    // Route64 format: [2 bits header | 2 bits tier | 20 bits x | 20 bits y | 20 bits z]
    // Bits 63-62: header (0b01)
    // Bits 61-60: tier (0-3)
    // Bits 59-40: x (20 bits, signed)
    // Bits 39-20: y (20 bits, signed)
    // Bits 19-0:  z (20 bits, signed)

    int x = int((value >> 40) & 0xFFFFF); // Extract 20 bits for x
    int y = int((value >> 20) & 0xFFFFF); // Extract 20 bits for y
    int z = int(value & 0xFFFFF);         // Extract 20 bits for z

    // Sign extension for 20-bit signed integers
    if (x & 0x80000) x |= 0xFFF00000;
    if (y & 0x80000) y |= 0xFFF00000;
    if (z & 0x80000) z |= 0xFFF00000;

    return int3(x, y, z);
}

// Encode coordinates back to Route64 value
inline ulong encode_route64(int tier, int3 coords) {
    // Validate parity (BCC lattice requires even parity)
    // Note: We don't error here, just encode as-is. The Rust side will validate.

    // Mask to 20 bits
    ulong x = ulong(coords.x) & 0xFFFFF;
    ulong y = ulong(coords.y) & 0xFFFFF;
    ulong z = ulong(coords.z) & 0xFFFFF;

    // Header bits (0b01 for Route64)
    ulong header = 0x01;
    ulong tier_bits = ulong(tier) & 0x3;

    // Combine: [2 bits header | 2 bits tier | 20 bits x | 20 bits y | 20 bits z]
    return (header << 62) | (tier_bits << 60) | (x << 40) | (y << 20) | z;
}

// Main compute kernel for batch neighbor calculation
kernel void batch_neighbors(
    device const ulong* input_routes [[buffer(0)]],
    device ulong* output_routes [[buffer(1)]],
    uint gid [[thread_position_in_grid]])
{
    // Each thread processes one input route and generates 14 neighbors
    ulong route_value = input_routes[gid];

    // Decode route to coordinates
    int3 coords = decode_route64(route_value);

    // Extract tier from input route (bits 61-60)
    int tier = int((route_value >> 60) & 0x3);

    // Calculate 14 neighbors
    uint output_base = gid * 14;

    for (uint i = 0; i < 14; i++) {
        int3 neighbor_coords = coords + BCC_NEIGHBOR_OFFSETS[i].xyz;
        output_routes[output_base + i] = encode_route64(tier, neighbor_coords);
    }
}

// Alternative kernel for grouped output (14 neighbors per group)
kernel void batch_neighbors_grouped(
    device const ulong* input_routes [[buffer(0)]],
    device ulong* output_routes [[buffer(1)]],
    device const uint* input_count [[buffer(2)]],
    uint gid [[thread_position_in_grid]])
{
    if (gid >= *input_count) {
        return;
    }

    ulong route_value = input_routes[gid];
    int3 coords = decode_route64(route_value);

    // Extract tier from input route (bits 61-60)
    int tier = int((route_value >> 60) & 0x3);

    uint output_base = gid * 14;

    for (uint i = 0; i < 14; i++) {
        int3 neighbor_coords = coords + BCC_NEIGHBOR_OFFSETS[i].xyz;
        output_routes[output_base + i] = encode_route64(tier, neighbor_coords);
    }
}
