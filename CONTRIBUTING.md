# Contributing to OctaIndex3D

Thank you for your interest in contributing to OctaIndex3D! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Performance Considerations](#performance-considerations)
- [Documentation](#documentation)

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and professional in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/OctaIndex3D
   cd octaindex3d
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/FunKite/OctaIndex3D
   ```

## Development Setup

### Prerequisites

- **Rust 1.77+** (MSRV - Minimum Supported Rust Version)
- **Cargo** (comes with Rust)
- **Git**

### Build and Test

```bash
# Build with all features
cargo build --all-features

# Run all tests
cargo test --all-features

# Run benchmarks (requires nightly for some features)
cargo bench --features parallel

# Check code formatting
cargo fmt -- --check

# Run clippy for lints
cargo clippy --all-features -- -D warnings
```

### Recommended Build Flags

For maximum performance during development:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Feature Flags

- `parallel` - Multi-threaded batch operations (recommended)
- `simd` - SIMD-accelerated operations
- `hilbert` - Hilbert curve support
- `container_v2` - Streaming container format
- `gis_geojson` - GeoJSON export functionality
- `zstd_compression` - Zstd compression support

## How to Contribute

### Reporting Bugs

Before creating a bug report, please:
1. Check if the issue already exists in [Issues](https://github.com/FunKite/OctaIndex3D/issues)
2. Verify you're using the latest version
3. Test with minimal reproduction steps

Use the **Bug Report** template when filing issues.

### Suggesting Enhancements

Feature requests are welcome! Use the **Feature Request** template and include:
- Clear description of the proposed functionality
- Use cases and motivation
- Expected behavior
- Alternative solutions considered

### Pull Requests

We actively welcome your pull requests! See [Pull Request Process](#pull-request-process) below.

## Pull Request Process

1. **Create a feature branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Write clear, concise commit messages
   - Follow the coding standards
   - Add tests for new functionality
   - Update documentation as needed

3. **Test thoroughly**:
   ```bash
   # Run all tests
   cargo test --all-features

   # Check formatting
   cargo fmt -- --check

   # Run clippy
   cargo clippy --all-features -- -D warnings

   # Run benchmarks if performance-related
   cargo bench
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Open a Pull Request**:
   - Fill out the PR template completely
   - Link any related issues
   - Provide clear description of changes
   - Include benchmark results if performance-related

6. **Address review feedback**:
   - Respond to comments promptly
   - Make requested changes
   - Push additional commits as needed

7. **Merge requirements**:
   - All tests must pass (CI/CD checks)
   - At least one maintainer approval
   - No unresolved review comments
   - Branch up to date with main

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` with default settings
- Address all `cargo clippy` warnings
- Prefer explicit over implicit where it aids clarity
- Use meaningful variable and function names

### Code Organization

```rust
// 1. External crate imports
use std::collections::HashMap;
use rayon::prelude::*;

// 2. Internal crate imports
use crate::morton::*;
use crate::types::*;

// 3. Type aliases
type Result<T> = std::result::Result<T, Error>;

// 4. Constants
const MAX_COORD: i32 = 524_287;

// 5. Implementation
impl MyType {
    // Public methods first
    pub fn new() -> Self { ... }

    // Private methods last
    fn internal_helper(&self) { ... }
}
```

### Documentation

All public APIs must have documentation:

```rust
/// Encodes 3D coordinates into a Morton code (Z-order curve).
///
/// # Arguments
///
/// * `x` - X coordinate (0..=65535)
/// * `y` - Y coordinate (0..=65535)
/// * `z` - Z coordinate (0..=65535)
///
/// # Returns
///
/// 48-bit Morton code as `u64`
///
/// # Examples
///
/// ```
/// use octaindex3d::morton::encode;
/// let code = encode(100, 200, 300);
/// assert_eq!(code, 0x0000_1234_5678_9abc);
/// ```
pub fn encode(x: u16, y: u16, z: u16) -> u64 {
    // Implementation
}
```

## Testing Guidelines

### Test Coverage

- **Unit tests**: Test individual functions and methods
- **Integration tests**: Test module interactions
- **Documentation tests**: Ensure examples in docs work
- **Property tests**: Use `proptest` for fuzz testing (if applicable)

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let coords = (100, 200, 300);
        let code = encode(coords.0, coords.1, coords.2);
        let decoded = decode(code);
        assert_eq!(coords, decoded);
    }

    #[test]
    #[should_panic(expected = "coordinate out of range")]
    fn test_invalid_coordinate() {
        encode(100_000, 0, 0); // Should panic
    }
}
```

### Benchmark Tests

For performance-critical changes, include benchmarks:

```rust
#[cfg(test)]
mod benches {
    use criterion::{black_box, Criterion};

    pub fn bench_encode(c: &mut Criterion) {
        c.bench_function("morton_encode", |b| {
            b.iter(|| encode(black_box(100), black_box(200), black_box(300)))
        });
    }
}
```

## Performance Considerations

OctaIndex3D is a high-performance library. For performance-related changes:

1. **Benchmark before and after**:
   ```bash
   cargo bench > baseline.txt
   # Make your changes
   cargo bench > optimized.txt
   ```

2. **Profile hotspots** (if needed):
   ```bash
   cargo run --release --example profile_hotspots
   ```

3. **Consider platform-specific optimizations**:
   - BMI2 (Intel/AMD x86_64)
   - NEON (ARM/Apple Silicon)
   - AVX2 (x86_64)

4. **Include benchmark results in PR**:
   - Before/after comparison
   - Test platform details (CPU, OS)
   - Speedup percentage

## Documentation

### Code Documentation

- Use `///` for public API documentation
- Use `//!` for module-level documentation
- Include examples in doc comments
- Document panics, errors, and safety considerations

### User-Facing Documentation

Update relevant files when adding features:
- `README.md` - Overview and quick start
- `WHITEPAPER.md` - Technical deep dives
- `PERFORMANCE.md` - Performance guidelines
- `examples/` - Working code examples

### Changelog

For significant changes, add an entry to the "Unreleased" section (we'll create one if needed):

```markdown
## [Unreleased]

### Added
- New feature X for Y use case

### Changed
- Improved performance of Z by 30%

### Fixed
- Bug in W causing incorrect results
```

## Questions?

If you have questions or need help:
- Open an issue with the "question" label
- Check existing issues and discussions
- Reach out to maintainers

Thank you for contributing to OctaIndex3D!
