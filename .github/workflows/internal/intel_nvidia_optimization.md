# Intel/NVIDIA EC2 Optimization Workflow

**Instance Type:** g4dn.xlarge or similar (Intel Xeon + NVIDIA T4/A10)
**OS:** Ubuntu 22.04 LTS
**Purpose:** Profile and optimize for x86_64 + CUDA

## Prerequisites Check

```bash
# Check CPU features
lscpu | grep -E "Model name|Flags"

# Verify NVIDIA GPU
nvidia-smi

# Check CUDA version
nvcc --version

# Check Rust
rustc --version
```

## Step 1: System Setup

```bash
# Update system
sudo apt-get update
sudo apt-get upgrade -y

# Install build essentials
sudo apt-get install -y build-essential pkg-config libssl-dev git curl

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install CUDA toolkit (if not present)
# Check https://developer.nvidia.com/cuda-downloads for latest
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_1.1-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda-toolkit-12-3

# Set CUDA environment
export CUDA_HOME=/usr/local/cuda
export PATH=$CUDA_HOME/bin:$PATH
export LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH
echo 'export CUDA_HOME=/usr/local/cuda' >> ~/.bashrc
echo 'export PATH=$CUDA_HOME/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
```

## Step 2: Clone and Setup Project

```bash
# Clone repository
cd ~
git clone https://github.com/YOUR_USERNAME/octaindex3d.git
cd octaindex3d

# Create feature branch
git checkout -b intel-nvidia-optimizations
```

## Step 3: Baseline Profiling

```bash
# Build with Intel optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release --features parallel

# Run profiling harness
cargo run --release --example profile_hotspots --features parallel > /tmp/intel_baseline.txt

# Check what CPU features are being used
objdump -d target/release/examples/profile_hotspots | grep -E "vperm|vpadd|pdep|pext" | head -20

# Run benchmarks
cargo bench --features parallel -- --save-baseline intel_baseline
```

## Step 4: Enable and Test BMI2/AVX2

```bash
# Verify CPU supports BMI2 and AVX2
grep -E "bmi2|avx2" /proc/cpuinfo

# Build with explicit SIMD features
RUSTFLAGS="-C target-cpu=native -C target-feature=+bmi2,+avx2,+avx" \
  cargo build --release --features "simd,parallel"

# Profile Morton operations (should use BMI2)
cargo run --release --example profile_hotspots --features "simd,parallel" 2>&1 | grep -A 5 "Morton"

# Compare with baseline
echo "=== Baseline (no explicit SIMD) ==="
grep "Morton" /tmp/intel_baseline.txt
echo ""
echo "=== With BMI2/AVX2 ==="
cargo run --release --example profile_hotspots --features "simd,parallel" 2>&1 | grep "Morton"
```

## Step 5: CUDA Optimization

### 5.1: Install cudarc dependency

```bash
# Add to Cargo.toml (should already be there)
# cudarc = { version = "0.11", features = ["cuda-12030"] }
```

### 5.2: Create CUDA kernel for batch neighbors

Create file: `cuda/batch_neighbors.cu`

```cuda
// CUDA kernel for batch neighbor calculation
extern "C" __global__ void batch_neighbors_kernel(
    const int32_t* input_coords,  // [batch_size * 3] (x, y, z)
    const uint8_t* tiers,          // [batch_size]
    int32_t* output_coords,        // [batch_size * 14 * 3]
    int batch_size
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= batch_size) return;

    int x = input_coords[idx * 3 + 0];
    int y = input_coords[idx * 3 + 1];
    int z = input_coords[idx * 3 + 2];
    uint8_t tier = tiers[idx];

    int out_base = idx * 14 * 3;

    // 8 diagonal neighbors
    output_coords[out_base + 0] = x + 1; output_coords[out_base + 1] = y + 1; output_coords[out_base + 2] = z + 1;
    output_coords[out_base + 3] = x + 1; output_coords[out_base + 4] = y + 1; output_coords[out_base + 5] = z - 1;
    output_coords[out_base + 6] = x + 1; output_coords[out_base + 7] = y - 1; output_coords[out_base + 8] = z + 1;
    output_coords[out_base + 9] = x + 1; output_coords[out_base + 10] = y - 1; output_coords[out_base + 11] = z - 1;
    output_coords[out_base + 12] = x - 1; output_coords[out_base + 13] = y + 1; output_coords[out_base + 14] = z + 1;
    output_coords[out_base + 15] = x - 1; output_coords[out_base + 16] = y + 1; output_coords[out_base + 17] = z - 1;
    output_coords[out_base + 18] = x - 1; output_coords[out_base + 19] = y - 1; output_coords[out_base + 20] = z + 1;
    output_coords[out_base + 21] = x - 1; output_coords[out_base + 22] = y - 1; output_coords[out_base + 23] = z - 1;

    // 6 axis-aligned neighbors
    output_coords[out_base + 24] = x + 2; output_coords[out_base + 25] = y; output_coords[out_base + 26] = z;
    output_coords[out_base + 27] = x - 2; output_coords[out_base + 28] = y; output_coords[out_base + 29] = z;
    output_coords[out_base + 30] = x; output_coords[out_base + 31] = y + 2; output_coords[out_base + 32] = z;
    output_coords[out_base + 33] = x; output_coords[out_base + 34] = y - 2; output_coords[out_base + 35] = z;
    output_coords[out_base + 36] = x; output_coords[out_base + 37] = y; output_coords[out_base + 38] = z + 2;
    output_coords[out_base + 39] = x; output_coords[out_base + 40] = y; output_coords[out_base + 41] = z - 2;
}
```

### 5.3: Compile CUDA kernel

```bash
# Compile kernel
nvcc -ptx -O3 --gpu-architecture=sm_75 cuda/batch_neighbors.cu -o cuda/batch_neighbors.ptx

# Verify PTX output
ls -lh cuda/batch_neighbors.ptx
```

### 5.4: Test CUDA backend

```bash
# Build with CUDA support
cargo build --release --features "cuda,parallel"

# Test CUDA neighbor calculation
cargo test --release --features cuda cuda::tests -- --nocapture

# Benchmark CPU vs GPU
cargo run --release --example compare_backends --features "cuda,parallel"
```

## Step 6: Create Comparison Benchmark

Create `examples/intel_nvidia_comparison.rs`:

```rust
//! Compare CPU (Intel) vs GPU (NVIDIA) performance

use octaindex3d::{Route64, performance::*};
use std::time::Instant;

fn main() {
    println!("=== Intel Xeon + NVIDIA GPU Comparison ===\n");

    let sizes = vec![1_000, 10_000, 100_000, 1_000_000];

    for size in sizes {
        println!("Batch size: {}", size);

        let routes: Vec<Route64> = (0..size)
            .map(|i| Route64::new(0, (i % 10000) * 2, (i % 10000) * 2, (i % 10000) * 2).unwrap())
            .collect();

        // CPU benchmark
        let start = Instant::now();
        let cpu_result = batch_neighbors_auto(&routes);
        let cpu_time = start.elapsed();
        println!("  CPU: {:?} ({} routes/sec)", cpu_time, size as f64 / cpu_time.as_secs_f64());

        // GPU benchmark (if available)
        #[cfg(feature = "cuda")]
        {
            let start = Instant::now();
            let gpu_result = cuda_batch_neighbors(&routes);
            let gpu_time = start.elapsed();
            println!("  GPU: {:?} ({} routes/sec)", gpu_time, size as f64 / gpu_time.as_secs_f64());
            println!("  Speedup: {:.2}x\n", cpu_time.as_secs_f64() / gpu_time.as_secs_f64());
        }
    }
}
```

Run it:
```bash
cargo run --release --example intel_nvidia_comparison --features "cuda,parallel"
```

## Step 7: Profile with perf

```bash
# Install perf
sudo apt-get install -y linux-tools-common linux-tools-generic linux-tools-$(uname -r)

# Profile hotspots
sudo perf record -g cargo run --release --example profile_hotspots --features parallel
sudo perf report

# Check for BMI2 usage
sudo perf stat -e instructions,cycles,branches,branch-misses \
  cargo run --release --example profile_hotspots --features "simd,parallel"

# Generate flamegraph
cargo install flamegraph
sudo cargo flamegraph --example profile_hotspots --features parallel
```

## Step 8: Optimize Based on Results

### Expected bottlenecks:
1. **Morton operations** - Should be 3-5x faster with BMI2 than Apple Silicon LUT
2. **Batch neighbors** - GPU should win for batches >100K
3. **Memory bandwidth** - Intel typically has higher bandwidth than Apple Silicon

### Optimization targets:
```bash
# If Morton decode is still slow, check BMI2 usage:
objdump -d target/release/examples/profile_hotspots | grep pdep -A 5

# If neighbors are slow, check vectorization:
cargo rustc --release --example profile_hotspots -- --emit asm
grep "vpadd\|vperm" target/release/examples/profile_hotspots.s

# GPU transfer overhead check:
# Create benchmark that measures just GPU transfer vs computation
```

## Step 9: Document Results

Create results file:
```bash
cat > /tmp/intel_nvidia_results.txt << 'EOF'
# Intel/NVIDIA Optimization Results

## Hardware
- CPU: $(lscpu | grep "Model name" | cut -d: -f2 | xargs)
- CPU Features: $(grep flags /proc/cpuinfo | head -1 | grep -o "bmi2\|avx2\|avx512")
- GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader)
- GPU Memory: $(nvidia-smi --query-gpu=memory.total --format=csv,noheader)

## Results
$(cat /tmp/intel_baseline.txt)

EOF

# Copy to repo
cp /tmp/intel_nvidia_results.txt .github/workflows/internal/
```

## Step 10: Commit and Push

```bash
# Add optimizations
git add .

# Commit
git commit -m "Intel/NVIDIA optimizations: BMI2 + CUDA kernels

- Enable BMI2 for ultra-fast Morton encoding (PDEP/PEXT)
- Implement CUDA kernels for batch neighbor calculation
- Profile results show X speedup on CPU, Y speedup on GPU
- Benchmarks included for future comparison"

# Push to branch
git push origin intel-nvidia-optimizations
```

## Expected Performance

**Intel Xeon (BMI2):**
- Morton encode: 800M+ ops/sec (vs 462M on Apple Silicon)
- Morton decode: 800M+ ops/sec (vs 157M on Apple Silicon)
- BMI2 is 3-5x faster than LUT approach

**NVIDIA GPU (for large batches):**
- Small (<10K): CPU faster due to transfer overhead
- Medium (10K-100K): Break-even point
- Large (>100K): GPU 10-50x faster
- Massive (>1M): GPU 50-100x faster

## Troubleshooting

**BMI2 not detected:**
```bash
# Force enable (risky - only if you're sure CPU supports it)
RUSTFLAGS="-C target-feature=+bmi2" cargo build --release

# Or check if running in VM without CPU pass-through
lscpu | grep Hypervisor
```

**CUDA errors:**
```bash
# Check CUDA installation
nvidia-smi
nvcc --version

# Check library paths
ldconfig -p | grep cuda

# Verify GPU is accessible
nvidia-smi -L
```

**Performance worse than expected:**
```bash
# Check CPU governor (should be performance, not powersave)
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor

# Set to performance mode
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable frequency scaling
sudo cpupower frequency-set -g performance
```

---

**Next:** Switch to AMD instance and run `amd_optimization.md` workflow
