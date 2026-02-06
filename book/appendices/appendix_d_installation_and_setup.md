# Appendix D: Installation and Setup

This appendix provides practical guidance for installing and configuring OctaIndex3D.

Topics include:

- System requirements and supported platforms  
- Building from source with `cargo`  
- Enabling optional features (e.g., BMI2, SIMD, serialization)  
- Troubleshooting common issues  

---

## D.1 System Requirements

- A recent stable Rust toolchain (see below for tested versions).
- A 64‑bit CPU; optional optimizations are available for x86‑64 (BMI2, AVX2) and ARM (NEON).
- Enough memory to hold your working dataset plus index structures.

### D.1.1 Rust Version Compatibility

OctaIndex3D is tested across multiple Rust versions:

| Rust Version | Status      | Notes                                      |
|-------------|-------------|--------------------------------------------|
| **1.82.0**  | Recommended | Default via `rust-toolchain.toml` in repo |
| **1.77+**   | Supported   | Minimum Supported Rust Version (MSRV)      |
| \< 1.77     | Unsupported | Not covered by CI or book examples         |

- For the book and examples, use the pinned toolchain by running commands inside the repository root or `book/` directory so that `rust-toolchain.toml` takes effect.
- For your own projects, you can target Rust **1.77+**; CI in this repository checks both the current stable toolchain and the MSRV.

## D.2 Installation Instructions

For most users:

1. Install Rust via `rustup` if you have not already.
2. Add `octaindex3d = "0.5"` to your project's `Cargo.toml`.
3. Run `cargo build` to fetch and compile dependencies.

If you prefer to work from a local checkout, clone the repository and run `cargo test` to verify your environment.

## D.3 Feature Flags

OctaIndex3D exposes feature flags to control optional dependencies and CPU‑specific optimizations. Typical categories include:

- `bmi2`, `avx2`, or `neon` for accelerated bit‑manipulation paths.
- `serde` for serialization support.
- `rayon` or similar crates for parallel processing.

Consult the main crate documentation for the current list of flags and recommended combinations.

## D.4 Building from Source

To build directly from the repository:

```bash
git clone https://github.com/FunKite/OctaIndex3D.git
cd OctaIndex3D
cargo build --release
```

You can add CPU‑specific flags via `RUSTFLAGS` or a `.cargo/config.toml` file once you are comfortable with the baseline build.

## D.5 GPU Setup Instructions

OctaIndex3D supports GPU acceleration for encoding, neighbor search, and range queries on platforms with Metal (macOS), CUDA (NVIDIA), or Vulkan (cross-platform) support.

### D.5.1 Metal (macOS)

Metal support is enabled through the `gpu-metal` feature flag:

```toml
[dependencies]
octaindex3d = { version = "0.5", features = ["gpu-metal"] }
```

**Requirements:**
- macOS 10.15 or later
- Xcode Command Line Tools
- Metal-capable GPU (most Mac hardware from 2012 onwards)

**Verification:**

```bash
# Check Metal support
system_profiler SPDisplaysDataType | grep Metal
```

**Usage:**

```rust
use octaindex3d::performance::gpu::GpuBatchProcessor;

let gpu = GpuBatchProcessor::new()?;
println!("Using GPU backend: {}", gpu.backend_name());
```

### D.5.2 CUDA (NVIDIA)

CUDA support requires the NVIDIA CUDA Toolkit and compatible GPU:

```toml
[dependencies]
octaindex3d = { version = "0.5", features = ["gpu-cuda"] }
```

**Requirements:**
- NVIDIA GPU with compute capability 5.0+ (Maxwell or later)
- CUDA Toolkit 11.0 or later
- cuDNN 8.0+ (optional, for ML integration)

**Installation:**

```bash
# Ubuntu/Debian
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-0

# Verify installation
nvcc --version
nvidia-smi
```

**Environment Setup:**

```bash
export CUDA_HOME=/usr/local/cuda
export PATH=$CUDA_HOME/bin:$PATH
export LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH
```

**Usage:**

```rust
use octaindex3d::performance::gpu::GpuBatchProcessor;

let gpu = GpuBatchProcessor::new()?;
if gpu.should_use_gpu(routes.len()) {
    let neighbors = gpu.batch_neighbors(&routes)?;
    println!("computed {} neighbors", neighbors.len());
}
```

### D.5.3 Vulkan (Cross-Platform)

Vulkan provides cross-platform GPU acceleration:

```toml
[dependencies]
octaindex3d = { version = "0.5", features = ["gpu-vulkan"] }
```

**Requirements:**
- Vulkan 1.2+ compatible GPU
- Platform-specific Vulkan SDK:
  - **Windows:** Install from LunarG
  - **Linux:** `sudo apt-get install vulkan-tools libvulkan-dev`
  - **macOS:** MoltenVK (via Vulkan SDK)

**Verification:**

```bash
# Check Vulkan support
vulkaninfo --summary

# List available devices
vkcube --list-devices
```

**Usage:**

```rust
use octaindex3d::performance::gpu::GpuBatchProcessor;

let gpu = GpuBatchProcessor::new()?;
let neighbors = gpu.batch_neighbors(&routes)?;
```

### D.5.4 GPU Feature Comparison

| Feature | Metal | CUDA | Vulkan |
|---------|-------|------|--------|
| **Encoding** | ✓ | ✓ | ✓ |
| **Neighbor Search** | ✓ | ✓ | ✓ |
| **Range Queries** | ✓ | ✓ | ✓ |
| **Multi-GPU** | Limited | ✓ | ✓ |
| **Async Compute** | ✓ | ✓ | ✓ |
| **Memory Mapping** | ✓ | ✓ | ✓ |

For detailed performance comparisons, see Appendix C.

---

## D.6 Docker Deployment

OctaIndex3D can be containerized for reproducible deployments and cloud scaling.

### D.6.1 Basic Dockerfile

Create a `Dockerfile` for building OctaIndex3D applications:

```dockerfile
FROM rust:1.82-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary with optimizations
ENV RUSTFLAGS="-C target-cpu=native"
RUN cargo build --release --features "serde,rayon"

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/your-app /usr/local/bin/

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/your-app"]
```

### D.6.2 Multi-Architecture Builds

Build for multiple platforms using Docker Buildx:

```bash
# Enable buildx
docker buildx create --name multiarch --use

# Build for AMD64 and ARM64
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --tag octaindex3d-app:latest \
  --push \
  .
```

### D.6.3 Docker Compose for Development

Create a `docker-compose.yml` for local development:

```yaml
version: '3.8'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    volumes:
      - ./data:/data
      - ./config:/config
    environment:
      RUST_LOG: info
      OCTAINDEX_THREADS: 8
    ports:
      - "8080:8080"
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G

  # Optional: GPU-enabled container (NVIDIA)
  gpu-app:
    build:
      context: .
      dockerfile: Dockerfile.gpu
    runtime: nvidia
    environment:
      NVIDIA_VISIBLE_DEVICES: all
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
```

### D.6.4 GPU-Enabled Docker (NVIDIA)

Create `Dockerfile.gpu` for CUDA support:

```dockerfile
FROM nvidia/cuda:12.0-devel-ubuntu22.04 AS builder

# Install Rust
RUN apt-get update && apt-get install -y curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . .

# Build with CUDA support
RUN cargo build --release --features "cuda"

# Runtime stage with CUDA runtime
FROM nvidia/cuda:12.0-runtime-ubuntu22.04

COPY --from=builder /app/target/release/your-app /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/your-app"]
```

**Run with GPU:**

```bash
docker run --gpus all octaindex3d-app:gpu
```

---

## D.7 CI/CD Integration

Automate testing and deployment with continuous integration pipelines.

### D.7.1 GitHub Actions

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Suite
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
        components: rustfmt, clippy

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache target directory
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-target-${{ matrix.rust }}-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cargo fmt -- --check

    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Run tests
      run: cargo test --all-features

    - name: Run tests (no default features)
      run: cargo test --no-default-features

    - name: Build documentation
      run: cargo doc --no-deps --all-features

  bench:
    name: Benchmarks
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Run benchmarks
      run: cargo bench --all-features -- --output-format bencher | tee bench_output.txt

    - name: Store benchmark result
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: bench_output.txt
        github-token: ${{ secrets.GITHUB_TOKEN }}
        auto-push: true

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Install tarpaulin
      run: cargo install cargo-tarpaulin

    - name: Generate coverage
      run: cargo tarpaulin --all-features --workspace --timeout 300 --out Xml

    - name: Upload to codecov.io
      uses: codecov/codecov-action@v3
      with:
        files: ./cobertura.xml
        fail_ci_if_error: true

  cross-compile:
    name: Cross-compilation
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - aarch64-unknown-linux-gnu
          - x86_64-apple-darwin
          - aarch64-apple-darwin

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Install cross
      run: cargo install cross

    - name: Cross-compile
      run: cross build --release --target ${{ matrix.target }}
```

### D.7.2 GitLab CI

Create `.gitlab-ci.yml`:

```yaml
stages:
  - test
  - build
  - deploy

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo

cache:
  paths:
    - .cargo/
    - target/

test:
  stage: test
  image: rust:1.82
  script:
    - cargo test --all-features
    - cargo clippy --all-targets -- -D warnings
    - cargo fmt -- --check
  parallel:
    matrix:
      - RUST_VERSION: ['stable', 'nightly']

build:
  stage: build
  image: rust:1.82
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
    expire_in: 1 week

deploy:docker:
  stage: deploy
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -t $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA .
    - docker push $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA
  only:
    - main
```

### D.7.3 Jenkins Pipeline

Create `Jenkinsfile`:

```groovy
pipeline {
    agent any

    environment {
        CARGO_HOME = "${WORKSPACE}/.cargo"
    }

    stages {
        stage('Setup') {
            steps {
                sh 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
            }
        }

        stage('Build') {
            steps {
                sh 'cargo build --release'
            }
        }

        stage('Test') {
            parallel {
                stage('Unit Tests') {
                    steps {
                        sh 'cargo test'
                    }
                }
                stage('Integration Tests') {
                    steps {
                        sh 'cargo test --test "*" -- --ignored'
                    }
                }
            }
        }

        stage('Benchmark') {
            steps {
                sh 'cargo bench'
            }
        }

        stage('Deploy') {
            when {
                branch 'main'
            }
            steps {
                sh 'docker build -t octaindex3d:latest .'
                sh 'docker push octaindex3d:latest'
            }
        }
    }

    post {
        always {
            junit 'target/test-results/**/*.xml'
            archiveArtifacts artifacts: 'target/release/*', fingerprint: true
        }
    }
}
```

---

## D.8 Platform-Specific Notes

### D.8.1 Windows

**Visual Studio Build Tools:**
Some dependencies may require Visual Studio Build Tools:

```powershell
# Install via chocolatey
choco install visualstudio2022buildtools

# Or download from Microsoft
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
```

**BMI2 Support:**
Ensure you're compiling with appropriate target features:

```powershell
$env:RUSTFLAGS="-C target-feature=+bmi2,+avx2"
cargo build --release
```

### D.8.2 Linux

**Distribution-Specific Packages:**

```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev

# Fedora/RHEL
sudo dnf install gcc make openssl-devel

# Arch Linux
sudo pacman -S base-devel openssl
```

**Kernel Modules for GPU:**

```bash
# NVIDIA drivers
sudo ubuntu-drivers autoinstall

# AMD ROCm
sudo apt-get install rocm-dkms
```

### D.8.3 macOS

**Xcode Command Line Tools:**

```bash
xcode-select --install
```

**Homebrew Dependencies:**

```bash
brew install pkg-config openssl
```

**Apple Silicon (M1/M2/M3):**
NEON SIMD is automatically available on ARM64 macOS. To build universal binaries:

```bash
# Build for both architectures
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Create universal binary
lipo -create \
  target/aarch64-apple-darwin/release/your-app \
  target/x86_64-apple-darwin/release/your-app \
  -output your-app-universal
```

---

## D.9 Troubleshooting

Common issues and remedies:

### D.9.1 Build Issues

- **Build fails with unknown CPU features**
  Disable the corresponding feature flags (for example, `bmi2` or `avx2`) or ensure you are compiling for a compatible target.

- **Linker errors on Windows**
  Install Visual Studio Build Tools and ensure the MSVC toolchain is selected:
  ```bash
  rustup default stable-msvc
  ```

- **Missing OpenSSL on Linux**
  Install development packages:
  ```bash
  sudo apt-get install libssl-dev pkg-config
  ```

- **`link.exe` or `lld` not found**
  - **Windows:** Ensure the MSVC toolchain is installed and selected (`rustup show active-toolchain` should include `msvc`).
  - **Linux/macOS:** Install the platform toolchain (`build-essential` on Debian/Ubuntu, `xcode-select --install` on macOS).

- **Rust version mismatches between CI and local**
  - Run `rustc --version` locally and compare against the pinned `rust-toolchain.toml` and MSRV noted in §D.1.1.
  - When upgrading Rust, update `rust-toolchain.toml`, run `cargo check` with `1.77` (MSRV) and stable, and only then rely on newer language features.

### D.9.2 Runtime Issues

- **Examples or benchmarks run slowly on laptop hardware**
  Start with smaller datasets and lower levels of detail (LOD). Use the Performance Tuning Cookbook (Appendix G) to identify safe optimizations.

- **Parity assertion failures or "not in BCC lattice" errors**
  Check that you are constructing coordinates through the provided types (such as `BccCoord`) rather than hand‑crafting indices. Chapter 2 explains the parity constraint in detail.

- **GPU initialization fails**
  Verify driver installation and compatibility:
  ```bash
  # NVIDIA
  nvidia-smi

  # Vulkan
  vulkaninfo

  # Metal (macOS)
  system_profiler SPDisplaysDataType | grep Metal
  ```

- **Platform-specific GPU issues**
  - **Windows:** Ensure the NVIDIA or vendor driver is installed and matches the CUDA/Vulkan runtime versions; WSL2 users may need to enable GPU passthrough explicitly.
  - **Linux:** Confirm that kernel modules are loaded (`nvidia-smi` or `lsmod | grep amdgpu`) and that you are not inside a container without GPU access.
  - **macOS:** Verify that you are on a Metal-capable GPU and that Xcode Command Line Tools are installed; older Intel Macs without full Metal support may fall back to CPU paths.

### D.9.3 Docker Issues

- **Container build fails on ARM64**
  Ensure you're using multi-architecture base images:
  ```dockerfile
  FROM --platform=$BUILDPLATFORM rust:1.82-slim
  ```

- **GPU not accessible in container**
  Install NVIDIA Container Toolkit:
  ```bash
  distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
  curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
  curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
    sudo tee /etc/apt/sources.list.d/nvidia-docker.list
  sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
  sudo systemctl restart docker
  ```

### D.9.4 CI/CD Issues

- **Tests timeout in CI**
  Increase timeout limits and reduce test parallelism:
  ```yaml
  - run: cargo test -- --test-threads=1 --timeout=300
  ```

- **Caching not working**
  Verify cache keys include Cargo.lock:
  ```yaml
  key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
  ```

- **Cross-compilation fails**
  Use `cross` tool instead of bare `cargo`:
  ```bash
  cargo install cross
  cross build --target aarch64-unknown-linux-gnu
  ```

---

## D.10 Further Reading

- **Rust Installation Guide:** https://www.rust-lang.org/tools/install
- **Docker Documentation:** https://docs.docker.com/
- **GitHub Actions for Rust:** https://github.com/actions-rs
- **CUDA Installation Guide:** https://docs.nvidia.com/cuda/cuda-installation-guide-linux/
- **Vulkan Getting Started:** https://vulkan-tutorial.com/
- **Metal Programming Guide:** https://developer.apple.com/metal/

As the book and codebase evolve, this appendix will grow to cover additional platforms and integration scenarios reported by readers.
