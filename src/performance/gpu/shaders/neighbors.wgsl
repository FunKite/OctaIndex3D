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
    // Route64 format: [2 bits header | 20 bits x | 20 bits y | 20 bits z | 2 bits parity]
    var x = i32((value >> 42u) & 0xFFFFFu);
    var y = i32((value >> 22u) & 0xFFFFFu);
    var z = i32((value >> 2u) & 0xFFFFFu);

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

// Encode coordinates back to Route64 value
fn encode_route64(coords: vec3<i32>) -> u64 {
    // Calculate parity (BCC lattice requires even parity)
    let parity = u64((coords.x + coords.y + coords.z) & 1);

    // Mask to 20 bits
    let x = u64(coords.x) & 0xFFFFFu;
    let y = u64(coords.y) & 0xFFFFFu;
    let z = u64(coords.z) & 0xFFFFFu;

    // Header bits (01 for Route64)
    let header = 0x01u;

    // Combine: [2 bits header | 20 bits x | 20 bits y | 20 bits z | 2 bits parity]
    return (header << 62u) | (x << 42u) | (y << 22u) | (z << 2u) | parity;
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

    // Calculate 14 neighbors
    let output_base = gid * 14u;

    for (var i = 0u; i < 14u; i = i + 1u) {
        let neighbor_coords = coords + BCC_NEIGHBOR_OFFSETS[i];
        output_routes[output_base + i] = encode_route64(neighbor_coords);
    }
}
