// WebGPU compute shader for batch neighbor calculations
//
// This shader calculates the 14 BCC lattice neighbors for each input route
// in parallel on the GPU using WGSL (WebGPU Shading Language)

// BCC lattice neighbor offsets (14 nearest neighbors)
const BCC_NEIGHBOR_OFFSETS: array<vec3<i32>, 14> = array<vec3<i32>, 14>(
    // Diagonal neighbors (8)
    vec3<i32>( 1,  1,  1),
    vec3<i32>( 1,  1, -1),
    vec3<i32>( 1, -1,  1),
    vec3<i32>( 1, -1, -1),
    vec3<i32>(-1,  1,  1),
    vec3<i32>(-1,  1, -1),
    vec3<i32>(-1, -1,  1),
    vec3<i32>(-1, -1, -1),
    // Axis-aligned neighbors (6)
    vec3<i32>( 2,  0,  0),
    vec3<i32>(-2,  0,  0),
    vec3<i32>( 0,  2,  0),
    vec3<i32>( 0, -2,  0),
    vec3<i32>( 0,  0,  2),
    vec3<i32>( 0,  0, -2),
);

// Storage buffers
@group(0) @binding(0) var<storage, read> input_routes: array<u64>;
@group(0) @binding(1) var<storage, read_write> output_routes: array<u64>;

// Extract coordinates from Route64 value
fn decode_route64(value: u64) -> vec3<i32> {
    // Route64 format: [2 bits header | 2 bits tier | 20 bits x | 20 bits y | 20 bits z]
    // Bits 63-62: header (0b01)
    // Bits 61-60: tier (0-3)
    // Bits 59-40: x (20 bits, signed)
    // Bits 39-20: y (20 bits, signed)
    // Bits 19-0:  z (20 bits, signed)

    // WGSL doesn't support bitwise ops on u64, so we work with u32 parts
    let high = u32(value >> 32u);
    let low = u32(value); // Truncation automatically masks to lower 32 bits

    // high = bits 63-32, low = bits 31-0
    // x needs bits 59-40: high[27:8]
    var x = i32((high >> 8u) & 0xFFFFFu);
    // y needs bits 39-20: high[7:0] + low[31:20]
    var y = i32(((high & 0xFFu) << 12u) | (low >> 20u));
    // z needs bits 19-0: low[19:0]
    var z = i32(low & 0xFFFFFu);

    // Sign extension for 20-bit signed integers
    if ((x & 0x80000) != 0) {
        x = x | i32(0xFFF00000u);
    }
    if ((y & 0x80000) != 0) {
        y = y | i32(0xFFF00000u);
    }
    if ((z & 0x80000) != 0) {
        z = z | i32(0xFFF00000u);
    }

    return vec3<i32>(x, y, z);
}

// Encode coordinates back to Route64 value with tier
fn encode_route64(tier: u32, coords: vec3<i32>) -> u64 {
    // Mask to 20 bits using u32 operations
    let x = u32(coords.x) & 0xFFFFFu;
    let y = u32(coords.y) & 0xFFFFFu;
    let z = u32(coords.z) & 0xFFFFFu;

    // Header bits (0b01 for Route64)
    let header = 0x01u;
    let tier_bits = tier & 0x3u;

    // Combine using u32 parts:
    // Bits 63-62: header (2 bits)
    // Bits 61-60: tier (2 bits)
    // Bits 59-40: x (20 bits)
    // Bits 39-20: y (20 bits)
    // Bits 19-0:  z (20 bits)

    // high word (bits 63-32): header[1:0] + tier[1:0] + x[19:0] + y[19:12]
    let high = (header << 30u) | (tier_bits << 28u) | (x << 8u) | (y >> 12u);
    // low word (bits 31-0): y[11:0] + z[19:0]
    let low = ((y & 0xFFFu) << 20u) | z;

    return (u64(high) << 32u) | u64(low);
}

// Main compute kernel for batch neighbor calculation
@compute @workgroup_size(256, 1, 1)
fn batch_neighbors(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;

    // Check bounds
    if (gid >= arrayLength(&input_routes)) {
        return;
    }

    // Each thread processes one input route and generates 14 neighbors
    let route_value = input_routes[gid];

    // Decode route to coordinates
    let coords = decode_route64(route_value);

    // Extract tier from input route (bits 61-60)
    let high = u32(route_value >> 32u);
    let tier = (high >> 28u) & 0x3u;

    // Calculate 14 neighbors - unroll loop since WGSL requires constant array indices
    let output_base = gid * 14u;

    output_routes[output_base + 0u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[0]);
    output_routes[output_base + 1u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[1]);
    output_routes[output_base + 2u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[2]);
    output_routes[output_base + 3u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[3]);
    output_routes[output_base + 4u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[4]);
    output_routes[output_base + 5u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[5]);
    output_routes[output_base + 6u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[6]);
    output_routes[output_base + 7u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[7]);
    output_routes[output_base + 8u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[8]);
    output_routes[output_base + 9u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[9]);
    output_routes[output_base + 10u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[10]);
    output_routes[output_base + 11u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[11]);
    output_routes[output_base + 12u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[12]);
    output_routes[output_base + 13u] = encode_route64(tier, coords + BCC_NEIGHBOR_OFFSETS[13]);
}
