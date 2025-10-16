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
    // Route64 format: [2 bits header | 20 bits x | 20 bits y | 20 bits z | 2 bits parity]
    // Coordinates are stored as signed 20-bit integers

    int x = int((value >> 42) & 0xFFFFF); // Extract 20 bits for x
    int y = int((value >> 22) & 0xFFFFF); // Extract 20 bits for y
    int z = int((value >> 2) & 0xFFFFF);  // Extract 20 bits for z

    // Sign extension for 20-bit signed integers
    if (x & 0x80000) x |= 0xFFF00000;
    if (y & 0x80000) y |= 0xFFF00000;
    if (z & 0x80000) z |= 0xFFF00000;

    return int3(x, y, z);
}

// Encode coordinates back to Route64 value
inline ulong encode_route64(int frame, int3 coords) {
    // Ensure parity (BCC lattice requires even parity)
    int parity = (coords.x + coords.y + coords.z) & 1;

    // Mask to 20 bits
    ulong x = ulong(coords.x) & 0xFFFFF;
    ulong y = ulong(coords.y) & 0xFFFFF;
    ulong z = ulong(coords.z) & 0xFFFFF;

    // Header bits (01 for Route64)
    ulong header = 0x01;

    // Combine: [2 bits header | 20 bits x | 20 bits y | 20 bits z | 2 bits parity]
    return (header << 62) | (x << 42) | (y << 22) | (z << 2) | ulong(parity);
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

    // Extract frame (assuming frame 0 for now, can be extended)
    int frame = 0;

    // Calculate 14 neighbors
    uint output_base = gid * 14;

    for (uint i = 0; i < 14; i++) {
        int3 neighbor_coords = coords + BCC_NEIGHBOR_OFFSETS[i].xyz;
        output_routes[output_base + i] = encode_route64(frame, neighbor_coords);
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
    int frame = 0;

    uint output_base = gid * 14;

    for (uint i = 0; i < 14; i++) {
        int3 neighbor_coords = coords + BCC_NEIGHBOR_OFFSETS[i].xyz;
        output_routes[output_base + i] = encode_route64(frame, neighbor_coords);
    }
}
