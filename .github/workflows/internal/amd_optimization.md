# AMD EC2 Optimization Workflow

**Instance Type:** c5a.xlarge or similar (AMD EPYC)
**OS:** Ubuntu 22.04 LTS
**Purpose:** Profile and optimize for AMD Zen architecture

## Prerequisites Check

```bash
# Check CPU model (should show AMD EPYC)
lscpu | grep -E "Model name|Flags"

# Check for AMD-specific features
grep -E "bmi2|avx2|zen" /proc/cpuinfo

# Check Rust
rustc --version

# Verify we're on AMD
cat /proc/cpuinfo | grep vendor_id | head -1
# Should show: vendor_id : AuthenticAMD
```

## Step 1: System Setup

```bash
# Update system
sudo apt-get update
sudo apt-get upgrade -y

# Install build essentials
sudo apt-get install -y build-essential pkg-config libssl-dev git curl

# Install AMD-specific profiling tools
sudo apt-get install -y linux-tools-common linux-tools-generic linux-tools-$(uname -r)

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Check AMD CPU generation
lscpu | grep "Model name"
# Zen 1 (EPYC 7xxx): Released 2017
# Zen 2 (EPYC 7xx2): Released 2019, better BMI2
# Zen 3 (EPYC 7xx3): Released 2021, best performance
# Zen 4 (EPYC 9xx4): Released 2022, AVX-512
```

## Step 2: Clone and Setup Project

```bash
# Clone repository (if not already done)
cd ~
git clone https://github.com/YOUR_USERNAME/octaindex3d.git
cd octaindex3d

# Create feature branch
git checkout -b amd-optimizations
```

## Step 3: Baseline Profiling

```bash
# Build with AMD optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release --features parallel

# Run profiling harness
cargo run --release --example profile_hotspots --features parallel > /tmp/amd_baseline.txt

# Check generated assembly for AMD-specific instructions
objdump -d target/release/examples/profile_hotspots | grep -E "vperm|vpadd|pdep|pext" | head -20

# Run benchmarks
cargo bench --features parallel -- --save-baseline amd_baseline
```

## Step 4: AMD Zen-Specific Optimizations

### 4.1: Check BMI2 Performance

**Important:** AMD Zen 1 and early Zen 2 have slow BMI2 (emulated in microcode)!

```bash
# Test BMI2 performance
cat > /tmp/test_bmi2.rs << 'EOF'
use std::time::Instant;
use std::arch::x86_64::*;

fn main() {
    let iterations = 100_000_000;

    // Test PDEP performance
    let start = Instant::now();
    let mut sum = 0u64;
    unsafe {
        for i in 0..iterations {
            sum = sum.wrapping_add(_pdep_u64(i, 0x5555555555555555));
        }
    }
    let pdep_time = start.elapsed();

    println!("PDEP: {:?} ({} ops/sec)", pdep_time, iterations as f64 / pdep_time.as_secs_f64());
    println!("Dummy: {}", sum);
}
EOF

rustc -C opt-level=3 -C target-cpu=native /tmp/test_bmi2.rs -o /tmp/test_bmi2
/tmp/test_bmi2

# If < 100M ops/sec, BMI2 is slow on this CPU
# If > 500M ops/sec, BMI2 is fast (Zen 3+)
```

### 4.2: Conditional BMI2 Usage

If BMI2 is slow, we need to use LUT fallback:

Edit `src/morton.rs`:
```rust
// Add AMD BMI2 detection
#[cfg(target_arch = "x86_64")]
pub fn should_use_bmi2() -> bool {
    if !is_x86_feature_detected!("bmi2") {
        return false;
    }

    // Check if we're on AMD with slow BMI2 (Zen 1/2)
    // On these CPUs, LUT is actually faster!
    #[cfg(target_feature = "bmi2")]
    {
        // TODO: Benchmark at runtime and cache result
        // For now, always use BMI2 on x86_64
        true
    }

    #[cfg(not(target_feature = "bmi2"))]
    false
}
```

### 4.3: Test Both Approaches

```bash
# Force LUT (no BMI2)
RUSTFLAGS="-C target-cpu=native -C target-feature=-bmi2" \
  cargo run --release --example profile_hotspots > /tmp/amd_no_bmi2.txt

# Use BMI2
RUSTFLAGS="-C target-cpu=native -C target-feature=+bmi2" \
  cargo run --release --example profile_hotspots > /tmp/amd_with_bmi2.txt

# Compare Morton performance
echo "=== WITHOUT BMI2 (LUT) ==="
grep "Morton" /tmp/amd_no_bmi2.txt
echo ""
echo "=== WITH BMI2 ==="
grep "Morton" /tmp/amd_with_bmi2.txt
```

## Step 5: AVX2 Optimization

AMD has excellent AVX2 performance (unlike AVX-512 on some Intel chips):

```bash
# Build with AVX2 focus
RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+fma" \
  cargo build --release --features "simd,parallel"

# Test batch operations (should benefit from AVX2)
cargo run --release --example profile_hotspots --features "simd,parallel" 2>&1 | \
  grep -A 5 "Batch\|Distance"
```

## Step 6: AMD-Specific Cache Optimization

AMD Zen has different cache hierarchy than Intel:

**Zen 1/2/3:**
- L1: 32KB per core
- L2: 512KB per core
- L3: 16MB per CCX (shared by 4-8 cores)

**Optimization:** Adjust cache blocking sizes

Edit `src/performance/fast_neighbors.rs`:
```rust
// Adjust BLOCK_SIZE for AMD cache
pub fn batch_neighbors_medium(routes: &[Route64]) -> Vec<Route64> {
    #[cfg(target_vendor = "amd")]
    const BLOCK_SIZE: usize = 32; // Smaller for AMD L1

    #[cfg(not(target_vendor = "amd"))]
    const BLOCK_SIZE: usize = 64;

    // ... rest of implementation
}
```

Test:
```bash
# Test different block sizes
for size in 16 32 64 128; do
  echo "Testing block size: $size"
  # Manually edit BLOCK_SIZE and rebuild
  # cargo run --release --example profile_hotspots | grep "Medium batch"
done
```

## Step 7: Profile with AMD uProf (Optional)

```bash
# Download AMD uProf
wget https://developer.amd.com/wordpress/media/files/AMDuProf_Linux_x64_4.1.424.tar.bz2
tar -xjf AMDuProf_Linux_x64_4.1.424.tar.bz2
cd AMDuProf_Linux_x64_4.1.424/bin

# Profile application
./AMDuProfCLI collect --config tbp \
  --output-dir /tmp/uprof_output \
  cargo run --release --example profile_hotspots

# Generate report
./AMDuProfCLI report --input /tmp/uprof_output
```

## Step 8: ROCm/HIP Support (for AMD GPUs)

If your instance has AMD GPU (rare on EC2, but possible):

```bash
# Install ROCm
wget https://repo.radeon.com/amdgpu-install/latest/ubuntu/jammy/amdgpu-install_*.deb
sudo dpkg -i amdgpu-install_*.deb
sudo amdgpu-install --usecase=rocm --no-dkms

# Verify ROCm
rocm-smi

# Build with HIP support
cargo build --release --features "hip,parallel"

# Test
cargo test --features hip -- --nocapture
```

## Step 9: Compare Intel vs AMD

Create comparison document:
```bash
cat > /tmp/amd_vs_intel.md << 'EOF'
# AMD vs Intel Comparison

## CPU Details
**AMD:**
$(lscpu | grep "Model name" | cut -d: -f2 | xargs)
$(lscpu | grep "CPU MHz" | cut -d: -f2 | xargs)

**Intel (from previous run):**
[Paste from intel_nvidia_results.txt]

## Performance Results

### Morton Encoding
| CPU | Encode (ops/sec) | Decode (ops/sec) |
|-----|------------------|------------------|
| AMD | X | Y |
| Intel | A | B |

### Batch Neighbors
| CPU | Small | Medium | Large |
|-----|-------|--------|-------|
| AMD | X | Y | Z |
| Intel | A | B | C |

## Analysis

**AMD Strengths:**
- [To be filled based on results]

**Intel Strengths:**
- [To be filled based on results]

**Winner by Category:**
- Morton ops: [AMD/Intel]
- Batch operations: [AMD/Intel]
- Overall: [AMD/Intel]
EOF
```

## Step 10: AMD-Specific Optimizations to Try

### 10.1: Prefetch Distance
AMD benefits from different prefetch distances:
```rust
// In arch_optimized.rs
#[cfg(target_vendor = "amd")]
const PREFETCH_DISTANCE: usize = 12; // AMD likes longer distance

#[cfg(not(target_vendor = "amd"))]
const PREFETCH_DISTANCE: usize = 8;
```

### 10.2: Branch Prediction
AMD Zen has excellent branch predictor, leverage it:
```rust
// Use likely/unlikely hints aggressively on AMD
#[cfg(target_vendor = "amd")]
if likely(condition) { ... }
```

### 10.3: Memory Ordering
AMD has relaxed memory ordering, can be faster:
```rust
use std::sync::atomic::Ordering;

// AMD can use Relaxed more often
#[cfg(target_vendor = "amd")]
const LOAD_ORDER: Ordering = Ordering::Relaxed;

#[cfg(not(target_vendor = "amd"))]
const LOAD_ORDER: Ordering = Ordering::Acquire;
```

## Step 11: Document and Commit

```bash
# Save results
cargo run --release --example profile_hotspots --features parallel > \
  .github/workflows/internal/amd_results.txt

# Compare benchmarks
cargo bench --features parallel -- --baseline amd_baseline

# Add changes
git add .

# Commit
git commit -m "AMD EPYC optimizations

- Conditional BMI2 usage (slow on Zen 1/2)
- Optimized cache blocking for AMD L1/L2/L3
- AVX2 + FMA optimizations
- Prefetch tuning for AMD
- Performance results vs Intel included"

# Push
git push origin amd-optimizations
```

## Expected Performance Characteristics

### AMD Zen 3 (EPYC 7xx3) vs Intel Xeon vs Apple M-series

| Operation | Apple M2 | Intel Xeon | AMD Zen 3 |
|-----------|----------|------------|-----------|
| Morton Encode | 462M/s | 800M/s (BMI2) | 600M/s (BMI2*) |
| Morton Decode | 157M/s (LUT) | 800M/s (BMI2) | 400M/s (mixed) |
| Batch Neighbors | 50M/s | 45M/s | 48M/s |
| AVX2 Operations | N/A (NEON) | Good | Excellent |

*AMD Zen 3+ has fast BMI2, Zen 1/2 should use LUT

### Why Choose AMD:
1. **Cost:** Often 20-40% cheaper than equivalent Intel on EC2
2. **Core Count:** More cores per dollar
3. **AVX2:** Excellent performance (better than some Intel)
4. **Consistent:** Less variation between SKUs than Intel

### Why Choose Intel:
1. **BMI2:** Always fast (AMD Zen 1/2 is slow)
2. **Single-thread:** Slightly higher clock speeds
3. **Ecosystem:** More optimization tools available

## Troubleshooting

**BMI2 slower than expected:**
```bash
# Check CPU generation
dmidecode -t processor | grep Version
# Zen 1/2 → Use LUT
# Zen 3+ → Use BMI2

# Force LUT
RUSTFLAGS="-C target-feature=-bmi2" cargo build --release
```

**AVX2 not being used:**
```bash
# Check compiler output
cargo rustc --release -- --emit asm
grep "vpadd\|vpmul" target/release/*.s

# Force AVX2
RUSTFLAGS="-C target-feature=+avx2" cargo build --release
```

**Performance inconsistent:**
```bash
# AMD EPYC can have NUMA issues
numactl --hardware

# Pin to single NUMA node
numactl --cpunodebind=0 --membind=0 cargo run --release --example profile_hotspots

# Check for frequency throttling
sudo cpupower frequency-info
```

**Cache thrashing:**
```bash
# Monitor cache misses
sudo perf stat -e cache-references,cache-misses,L1-dcache-load-misses \
  cargo run --release --example profile_hotspots

# Tune BLOCK_SIZE in fast_neighbors.rs based on results
```

## Final Validation

```bash
# Run full test suite
cargo test --release --features parallel

# Run all benchmarks
cargo bench --features parallel

# Final profiling run
cargo run --release --example profile_hotspots --features parallel

# Copy results to repo
cp /tmp/amd_baseline.txt .github/workflows/internal/
cp /tmp/amd_no_bmi2.txt .github/workflows/internal/
cp /tmp/amd_with_bmi2.txt .github/workflows/internal/
```

---

**Next Steps:**
1. Merge Intel/NVIDIA and AMD branches
2. Add runtime CPU detection
3. Create unified optimization guide
4. Document which CPU to choose for which workload
