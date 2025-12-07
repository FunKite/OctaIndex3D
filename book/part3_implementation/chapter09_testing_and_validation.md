# Chapter 9: Testing and Validation

## Learning Objectives

By the end of this chapter, you will be able to:

1. Describe OctaIndex3D’s testing philosophy and its emphasis on correctness.
2. Understand how unit tests and property-based tests complement each other.
3. Design benchmarks that reflect realistic workloads and measure the right metrics.
4. Interpret benchmark results and distinguish noise from signal.
5. Integrate OctaIndex3D into continuous integration (CI) pipelines.

---

## 9.1 Testing Strategy

OctaIndex3D’s testing strategy is built on three pillars:

1. **Determinism**: given the same inputs and configuration, results must be reproducible.
2. **Invariants**: mathematical properties of the BCC lattice and encodings are encoded as testable conditions.
3. **Realism**: benchmarks and stress tests reflect real workloads, not synthetic microbenchmarks alone.

From these pillars come several concrete practices:

- Use **fixed random seeds** for randomized tests.
- Maintain a suite of **invariant checks** (e.g., parity, neighbor counts).
- Include **end-to-end tests** that exercise full pipelines (frames → identifiers → containers → queries).

---

## 9.2 Unit Testing

Unit tests focus on small, isolated pieces of functionality:

- Encoding and decoding of identifiers.
- Frame transformations between specific CRS pairs.
- Container operations like insertion, deletion, and iteration.

Tests are written to:

- Cover common cases and edge cases.
- Document expected behavior through examples.
- Guard against regressions when refactoring internals.

For example, tests for Morton encoding verify that:

- Known coordinates map to known identifiers.
- Encoding followed by decoding yields the original coordinates.
- Neighbor relationships are preserved at each level of detail.

In the Rust repository, this typically translates to:

- `#[test]` functions colocated with the code they exercise.
- A small number of **golden cases** (hand-checked values) for each major operation.
- Tests that stay at the level of public APIs, not internal helper functions, so that refactors are easier.

### 9.2.1 Example: Testing BCC Identifier Encoding

Here's a concrete example of a unit test for Index64 encoding:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index64_round_trip() {
        // Golden case: known BCC coordinates
        let coords = [(0, 0, 0), (2, 2, 0), (1, 1, 0), (3, 1, 0)];

        for (x, y, z) in coords {
            let lod = 5;
            let idx = Index64::encode(x, y, z, lod).expect("valid BCC coords");
            let (decoded_x, decoded_y, decoded_z, decoded_lod) = idx.decode();

            assert_eq!(decoded_x, x);
            assert_eq!(decoded_y, y);
            assert_eq!(decoded_z, z);
            assert_eq!(decoded_lod, lod);
        }
    }

    #[test]
    fn test_parity_violation_rejected() {
        // Invalid: (1, 0, 0) violates BCC parity constraint
        let result = Index64::encode(1, 0, 0, 5);
        assert!(result.is_err());
        assert!(matches!(result, Err(IndexError::ParityViolation)));
    }

    #[test]
    fn test_neighbor_relationships() {
        let origin = Index64::encode(0, 0, 0, 5).unwrap();
        let neighbors = origin.neighbors();

        // BCC lattice has exactly 14 nearest neighbors
        assert_eq!(neighbors.len(), 14);

        // All neighbors should be at the same LOD
        for neighbor in neighbors {
            assert_eq!(neighbor.lod(), 5);
        }
    }
}
```rust

This example demonstrates several best practices:

- **Golden cases**: hand-verified coordinates that cover different parts of the lattice.
- **Error path testing**: verifying that invalid inputs are properly rejected.
- **Invariant checking**: confirming that neighbor count and LOD properties hold.

### 9.2.2 Testing Container Operations

Container tests verify insertion, deletion, query, and iteration:

```rust
#[test]
fn test_container_insertion_and_query() {
    let mut container = Container::new();

    // Insert a cluster of cells
    let cells = vec![
        Index64::encode(0, 0, 0, 3).unwrap(),
        Index64::encode(2, 2, 0, 3).unwrap(),
        Index64::encode(4, 0, 0, 3).unwrap(),
    ];

    for cell in &cells {
        container.insert(*cell, 1.0); // occupancy value
    }

    // Query should find inserted cells
    for cell in &cells {
        assert!(container.contains(cell));
        assert_eq!(container.get(cell), Some(1.0));
    }

    // Query should not find non-existent cells
    let absent = Index64::encode(10, 10, 10, 3).unwrap();
    assert!(!container.contains(&absent));
}

#[test]
fn test_container_deletion() {
    let mut container = Container::new();
    let cell = Index64::encode(0, 0, 0, 3).unwrap();

    container.insert(cell, 1.0);
    assert!(container.contains(&cell));

    container.remove(&cell);
    assert!(!container.contains(&cell));
}
```

### 9.2.3 Testing Coordinate Frame Transformations

Frame transformation tests ensure that coordinate conversions are accurate:

```rust
#[test]
fn test_frame_transformation_round_trip() {
    let earth_frame = FrameRef::wgs84();
    let local_frame = FrameRef::local_meters(
        [37.7749, -122.4194, 0.0], // San Francisco
        1.0, // 1 meter per BCC unit
    );

    let point_wgs84 = [37.7750, -122.4195, 10.0];

    // Transform to local frame and back
    let point_local = earth_frame.transform_to(&local_frame, point_wgs84);
    let point_back = local_frame.transform_to(&earth_frame, point_local);

    // Should match within tolerance (1cm for this scale)
    for i in 0..3 {
        assert!((point_back[i] - point_wgs84[i]).abs() < 1e-5);
    }
}
```rust

---

## 9.3 Property-Based Testing

Some properties are better expressed as **general laws** than as specific examples. Property-based testing tools generate many random inputs and check that a property holds for all of them.

Examples of properties used in OctaIndex3D:

- **Parity preservation**: all encoded BCC indices satisfy $(x + y + z) \equiv 0 \pmod{2}$.
- **Round-trip symmetry**: for valid inputs, `decode(encode(x)) == x`.
- **Monotonicity**: increasing level of detail refines, but does not relocate, parent cells.
- **Frame consistency**: transforming a point from frame A to B and back yields the original point within a small tolerance.

These tests are particularly valuable because:

- They explore corner cases that hand-written examples might miss.
- They detect assumptions that only hold for narrow input ranges.

In practice, property tests in OctaIndex3D follow a few rules of thumb:

- Use **bounded domains** that reflect real inputs (e.g., LOD ranges, realistic coordinate bounds).
- Start from a small set of high-value properties rather than trying to cover everything.
- Keep failing cases reproducible by recording seeds and shrinking results into regression tests where appropriate.

### 9.3.1 Implementing Property Tests with `proptest`

OctaIndex3D uses the `proptest` crate for property-based testing. Here's how to implement the round-trip property:

```rust
use proptest::prelude::*;

// Strategy for generating valid BCC coordinates
fn bcc_coords() -> impl Strategy<Value = (i32, i32, i32)> {
    // Generate coordinates that satisfy parity constraint
    (-1000..1000i32, -1000..1000i32)
        .prop_map(|(x, y)| {
            // Ensure z satisfies (x + y + z) % 2 == 0
            let z_base = -1000 + ((x + y) % 2).abs();
            (x, y, z_base + ((-1000i32..1000).prop_map(|v| v * 2).sample_single()))
        })
        .prop_filter("valid parity", |(x, y, z)| (x + y + z) % 2 == 0)
}

proptest! {
    #[test]
    fn test_index64_round_trip_property(
        (x, y, z) in bcc_coords(),
        lod in 0u8..20u8
    ) {
        let idx = Index64::encode(x, y, z, lod)?;
        let (dx, dy, dz, dl) = idx.decode();

        prop_assert_eq!(dx, x);
        prop_assert_eq!(dy, y);
        prop_assert_eq!(dz, z);
        prop_assert_eq!(dl, lod);
    }

    #[test]
    fn test_parent_child_relationship(
        (x, y, z) in bcc_coords(),
        lod in 1u8..20u8
    ) {
        let child = Index64::encode(x, y, z, lod)?;
        let parent = child.parent();

        // Parent should be at LOD-1
        prop_assert_eq!(parent.lod(), lod - 1);

        // Child should be one of parent's 8 children
        let children = parent.children();
        prop_assert!(children.contains(&child));
    }
}
```

This example demonstrates:

- **Custom strategies**: `bcc_coords()` generates only valid BCC coordinates.
- **Bounded inputs**: coordinates and LODs are constrained to realistic ranges.
- **Relationship testing**: verifying parent-child hierarchical relationships.

### 9.3.2 Key Properties to Test

Beyond basic round-trips, consider these advanced properties:

#### Neighbor Symmetry

If A is a neighbor of B, then B must be a neighbor of A:

```rust
proptest! {
    #[test]
    fn test_neighbor_symmetry(
        (x, y, z) in bcc_coords(),
        lod in 0u8..15u8
    ) {
        let cell = Index64::encode(x, y, z, lod)?;
        let neighbors = cell.neighbors();

        for neighbor in neighbors {
            let reverse_neighbors = neighbor.neighbors();
            prop_assert!(
                reverse_neighbors.contains(&cell),
                "neighbor relationship must be symmetric"
            );
        }
    }
}
```rust

#### Morton Code Ordering

Morton codes should preserve spatial locality:

```rust
proptest! {
    #[test]
    fn test_morton_preserves_proximity(
        (x1, y1, z1) in bcc_coords(),
        (x2, y2, z2) in bcc_coords(),
        lod in 0u8..15u8
    ) {
        let dist = ((x2 - x1).pow(2) + (y2 - y1).pow(2) + (z2 - z1).pow(2)) as f64;
        dist = dist.sqrt();

        // Only test nearby points
        prop_assume!(dist < 10.0);

        let idx1 = Index64::encode(x1, y1, z1, lod)?;
        let idx2 = Index64::encode(x2, y2, z2, lod)?;

        let morton1 = idx1.morton_code();
        let morton2 = idx2.morton_code();
        let morton_dist = (morton1 as i64 - morton2 as i64).abs();

        // Nearby cells should have similar Morton codes
        // (this is a weak form of locality preservation)
        prop_assert!(
            morton_dist < 1000,
            "nearby cells should have nearby Morton codes"
        );
    }
}
```

#### Frame Transformation Consistency

Frame transformations should be invertible:

```rust
proptest! {
    #[test]
    fn test_frame_transformation_invertible(
        x in -180.0f64..180.0,
        y in -90.0f64..90.0,
        z in -1000.0f64..10000.0
    ) {
        let earth_frame = FrameRef::wgs84();
        let local_frame = FrameRef::local_meters([0.0, 0.0, 0.0], 1.0);

        let point_wgs84 = [x, y, z];
        let point_local = earth_frame.transform_to(&local_frame, point_wgs84);
        let point_back = local_frame.transform_to(&earth_frame, point_local);

        for i in 0..3 {
            let diff = (point_back[i] - point_wgs84[i]).abs();
            prop_assert!(
                diff < 1e-4,
                "round-trip error {} exceeds tolerance at index {}", diff, i
            );
        }
    }
}
```rust

### 9.3.3 Shrinking and Debugging Failures

When a property test fails, `proptest` automatically **shrinks** the failing input to find a minimal example. For instance, if a test fails at coordinates `(8472, -3291, 5181)`, shrinking might reduce it to `(2, 0, 0)` — making the bug much easier to diagnose.

To aid debugging:

- Run failed tests with `PROPTEST_MAX_SHRINK_ITERS=10000` for more thorough shrinking.
- Capture shrunk failures as regression tests:

```rust
#[test]
fn test_regression_issue_42() {
    // Minimal failing case found by proptest shrinking
    let cell = Index64::encode(2, 0, 0, 15).unwrap();
    let parent = cell.parent();

    // Bug was: parent.children() didn't include cell
    assert!(parent.children().contains(&cell));
}
```

---

## 9.4 Benchmark Design

Performance benchmarks in OctaIndex3D aim to answer:

> "How fast is this operation under realistic conditions on concrete hardware?"

To that end, benchmarks:

- Use representative data distributions (e.g., clustered vs. uniform points).
- Measure batch operations, not only single calls.
- Report both **throughput** (operations per second) and **latency** distributions.

Examples include:

- Encoding/decoding large arrays of coordinates at various LODs.
- Running nearest-neighbor queries on containers of different sizes.
- Measuring container construction and iteration times.

Benchmarks are accompanied by:

- Documentation of hardware and compiler settings.
- Scripts that allow others to reproduce results.

For day-to-day development:

- Maintain a **small, fast benchmark suite** that can be run locally when working on hot-path code.
- Reserve heavier, long-running benchmarks for scheduled runs (nightly/weekly) or for release candidates.

### 9.4.1 Implementing Benchmarks with Criterion

OctaIndex3D uses the `criterion` crate for rigorous benchmarking. Here's a complete example:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use octaindex3d::{Index64, Container};

fn benchmark_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding");

    // Benchmark different LOD levels
    for lod in [5, 10, 15, 20] {
        group.bench_with_input(
            BenchmarkId::new("Index64::encode", lod),
            &lod,
            |b, &lod| {
                b.iter(|| {
                    // Use black_box to prevent compiler optimization
                    Index64::encode(
                        black_box(100),
                        black_box(200),
                        black_box(300),
                        black_box(lod)
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_neighbor_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_queries");

    for size in [100, 1000, 10_000] {
        let mut container = Container::new();

        // Populate container with clustered data
        for i in 0..size {
            let x = (i % 100) * 2;
            let y = (i / 100) * 2;
            let z = 0;
            let idx = Index64::encode(x, y, z, 10).unwrap();
            container.insert(idx, 1.0);
        }

        group.bench_with_input(
            BenchmarkId::new("find_neighbors", size),
            &container,
            |b, container| {
                let query = Index64::encode(50, 50, 0, 10).unwrap();
                b.iter(|| {
                    black_box(container.find_neighbors(black_box(&query), 14))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_encoding, benchmark_neighbor_queries);
criterion_main!(benches);
```bash

Run benchmarks with:

```bash
cargo bench --bench index_benchmarks
```

Criterion produces detailed statistical analysis, including:

- **Mean and standard deviation** of execution times
- **Outlier detection** to identify unstable benchmarks
- **Comparison with baseline** for regression detection

### 9.4.2 Interpreting Benchmark Results

Criterion output looks like this:

```toml
encoding/Index64::encode/5
                        time:   [12.453 ns 12.512 ns 12.578 ns]
                        change: [-2.1134% -0.8421% +0.4012%] (p = 0.19 > 0.05)
                        No change in performance detected.
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe
```

Key metrics to watch:

- **Time**: Look at the median (middle value) rather than the mean.
- **Change**: Percentage change from baseline; changes > ±5% warrant investigation.
- **Outliers**: High outlier counts suggest measurement instability (background processes, CPU throttling).
- **p-value**: p < 0.05 indicates statistically significant change.

Best practices for stable benchmarks:

- Run on a quiet system (close browsers, disable background tasks).
- Use `nice -n -20` or `taskset` to reduce interference.
- Take multiple samples and compare medians.
- Establish baselines and track trends over time.

### 9.4.3 Performance Regression Testing

To catch performance regressions in CI:

```rust
#[test]
fn test_encoding_performance_threshold() {
    use std::time::Instant;

    let iterations = 100_000;
    let start = Instant::now();

    for i in 0..iterations {
        let _ = Index64::encode(i * 2, i * 2, 0, 10);
    }

    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() / iterations;

    // Fail if encoding takes longer than 50ns per operation
    assert!(
        ns_per_op < 50,
        "Encoding performance regressed: {}ns per operation (threshold: 50ns)",
        ns_per_op
    );
}
```bash

This approach is less rigorous than criterion but useful for catching major regressions in CI pipelines where statistical analysis would be too slow.

---

## 9.5 Fuzzing Strategies

Fuzzing generates random or malformed inputs to uncover edge cases, crashes, and undefined behavior. OctaIndex3D uses both general-purpose and domain-specific fuzzing.

### 9.5.1 Cargo Fuzz Integration

`cargo-fuzz` leverages LLVM's LibFuzzer for coverage-guided fuzzing:

```bash
cargo install cargo-fuzz
cargo fuzz init
```

Create a fuzz target in `fuzz/fuzz_targets/index64_decode.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use octaindex3d::Index64;

fuzz_target!(|data: &[u8]| {
    if data.len() >= 8 {
        // Construct an Index64 from raw bytes
        let raw = u64::from_le_bytes(data[..8].try_into().unwrap());
        let idx = Index64::from_raw(raw);

        // Decode should never panic
        let (x, y, z, lod) = idx.decode();

        // Invariant: re-encoding valid coordinates should succeed
        if (x + y + z) % 2 == 0 && lod < 21 {
            let _ = Index64::encode(x, y, z, lod);
        }
    }
});
```bash

Run the fuzzer:

```bash
cargo fuzz run index64_decode -- -max_total_time=300
```

This runs for 5 minutes, exploring inputs that maximize code coverage.

### 9.5.2 Structured Fuzzing with `arbitrary`

For more structured inputs, use the `arbitrary` crate:

```rust
use arbitrary::Arbitrary;

#[derive(Arbitrary, Debug)]
struct FuzzQuery {
    x: i32,
    y: i32,
    z: i32,
    lod: u8,
    query_radius: u8,
}

fuzz_target!(|query: FuzzQuery| {
    // Ensure BCC parity constraint
    if (query.x + query.y + query.z) % 2 != 0 {
        return;
    }

    if let Ok(idx) = Index64::encode(query.x, query.y, query.z, query.lod) {
        // Neighbor query should never panic
        let _ = idx.neighbors();

        // Container query should never panic
        let mut container = Container::new();
        container.insert(idx, 1.0);
        let _ = container.range_query(idx, query.query_radius);
    }
});
```bash

### 9.5.3 Reproducing Fuzz Failures

When fuzzing finds a crash, it saves the input to `fuzz/artifacts/`:

```bash
# Reproduce the crash
cargo fuzz run index64_decode fuzz/artifacts/index64_decode/crash-abc123

# Minimize the crashing input
cargo fuzz cmin index64_decode
```

Convert crashes into regression tests:

```rust
#[test]
fn test_fuzz_crash_abc123() {
    // Input that caused crash: [0xFF, 0xFF, 0xFF, 0xFF, 0x1F, 0x00, 0x00, 0x00]
    let raw = 0x1F_FF_FF_FF_FF_u64;
    let idx = Index64::from_raw(raw);

    // Previously panicked here; now handles gracefully
    let (x, y, z, lod) = idx.decode();
    assert!(lod < 21, "LOD overflow should be clamped");
}
```rust

---

## 9.6 Cross-Platform Validation

Floating-point behavior and instruction sets vary across platforms. To ensure correctness:

- Test suites run on multiple architectures (x86_64, ARM64 where available).
- Feature-flag combinations (with and without BMI2, SIMD, etc.) are exercised.
- Numerical tolerance thresholds are chosen conservatively.

When platform-specific bugs are found:

- Regression tests are added to prevent recurrence.
- Workarounds are clearly documented.

As a practical checklist when changing low-level code:

- Run tests at multiple optimization levels (debug vs. release).
- Exercise both "fast path" and "fallback" implementations (e.g., with and without BMI2).
- Confirm that tolerances and invariants behave as expected on at least one non-x86_64 platform.

### 9.6.1 Testing Platform-Specific Code

Use conditional compilation to test feature-flagged code:

```rust
#[test]
fn test_morton_encoding_consistency() {
    let coords = (100, 200, 300);
    let lod = 10;

    #[cfg(target_feature = "bmi2")]
    let result_bmi2 = {
        let idx = Index64::encode(coords.0, coords.1, coords.2, lod).unwrap();
        idx.morton_code()
    };

    #[cfg(not(target_feature = "bmi2"))]
    let result_fallback = {
        let idx = Index64::encode(coords.0, coords.1, coords.2, lod).unwrap();
        idx.morton_code()
    };

    // Both paths should produce identical results
    #[cfg(all(target_feature = "bmi2", not(target_feature = "bmi2")))]
    assert_eq!(result_bmi2, result_fallback);
}
```

Run tests with different feature flags:

```bash
# Test with BMI2 enabled
RUSTFLAGS="-C target-cpu=native" cargo test

# Test fallback path (disable BMI2)
RUSTFLAGS="-C target-feature=-bmi2" cargo test

# Test on ARM (cross-compilation or CI)
cargo test --target aarch64-unknown-linux-gnu
```bash

---

## 9.7 Continuous Integration and Release Validation

OctaIndex3D uses continuous integration (CI) pipelines to:

- Run the full test suite on each change.
- Execute benchmarks on representative hardware (where feasible).
- Validate container compatibility across versions.

### 9.7.1 GitHub Actions Configuration

Here's a complete GitHub Actions workflow for OctaIndex3D:

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    name: Test Suite
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta, nightly]
        exclude:
          # Reduce matrix size for faster CI
          - os: windows-latest
            rust: beta

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --all-features --workspace

      - name: Run tests (no default features)
        run: cargo test --no-default-features --workspace

      - name: Run doc tests
        run: cargo test --doc --all-features

  cross-platform:
    name: Cross-Platform Tests
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - aarch64-unknown-linux-gnu
          - x86_64-unknown-linux-musl

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cross
        run: cargo install cross

      - name: Test ${{ matrix.target }}
        run: cross test --target ${{ matrix.target }}

  clippy:
    name: Clippy Lints
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Run clippy
        run: cargo clippy --all-features --workspace -- -D warnings

  fmt:
    name: Formatting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - name: Check formatting
        run: cargo fmt --all -- --check

  benchmarks:
    name: Performance Benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable

      - name: Run benchmarks
        run: cargo bench --no-fail-fast

      - name: Store benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/index.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true

  miri:
    name: Miri (Undefined Behavior Detection)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: miri

      - name: Run Miri
        run: cargo miri test
```

### 9.7.2 Release Validation Checklist

Before releasing a new version:

1. **Run extended test suite:**

   ```bash
   # Long-running integration tests
   cargo test --release --all-features -- --ignored

   # Property tests with high iteration counts
   PROPTEST_CASES=100000 cargo test
```rust

2. **Verify container format compatibility:**

   ```rust
   #[test]
   #[ignore] // Run manually before release
   fn test_container_backward_compatibility() {
       let test_files = [
           "tests/fixtures/v1.0.0_container.bcc",
           "tests/fixtures/v1.1.0_container.bcc",
           "tests/fixtures/v1.2.0_container.bcc",
       ];

       for file in test_files {
           let container = Container::load(file)
               .expect(&format!("Failed to load {}", file));

           // Verify data integrity
           assert!(container.len() > 0);
           assert!(container.validate().is_ok());
       }
   }
   ```

3. **Run soak tests:**

   ```rust
   #[test]
   #[ignore]
   fn test_soak_large_dataset() {
       let mut container = Container::new();

       // Insert 10 million cells
       for i in 0..10_000_000 {
           let x = (i % 1000) * 2;
           let y = ((i / 1000) % 1000) * 2;
           let z = ((i / 1_000_000) % 10) * 2;
           let idx = Index64::encode(x, y, z, 10).unwrap();
           container.insert(idx, i as f32);
       }

       // Verify no memory leaks or corruption
       assert_eq!(container.len(), 10_000_000);

       // Run queries
       for _ in 0..1000 {
           let query = Index64::encode(500, 500, 0, 10).unwrap();
           let _ = container.range_query(query, 10);
       }
   }
```bash

4. **Update benchmark baselines:**

   ```bash
   cargo bench --all-features
   git add target/criterion/*/base
   git commit -m "Update benchmark baselines for v1.3.0"
   ```

5. **Verify documentation builds:**

   ```bash
   cargo doc --all-features --no-deps --open
   ```

### 9.7.3 CI Performance Budgets

To prevent performance regressions, enforce performance budgets in CI:

```rust
// tests/performance_budgets.rs
use std::time::Instant;

macro_rules! performance_budget {
    ($name:expr, $budget_ns:expr, $iterations:expr, $code:expr) => {
        let start = Instant::now();
        for _ in 0..$iterations {
            $code;
        }
        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() / $iterations;

        assert!(
            ns_per_op < $budget_ns,
            "{} exceeded budget: {}ns (budget: {}ns)",
            $name, ns_per_op, $budget_ns
        );
    };
}

#[test]
fn test_performance_budgets() {
    use octaindex3d::Index64;

    // Encoding should complete in < 50ns
    performance_budget!("Index64::encode", 50, 100_000, {
        let _ = Index64::encode(100, 200, 300, 10);
    });

    // Decoding should complete in < 30ns
    performance_budget!("Index64::decode", 30, 100_000, {
        let idx = Index64::encode(100, 200, 300, 10).unwrap();
        let _ = idx.decode();
    });

    // Neighbor query should complete in < 200ns
    performance_budget!("neighbors", 200, 10_000, {
        let idx = Index64::encode(100, 200, 300, 10).unwrap();
        let _ = idx.neighbors();
    });
}
```

For teams integrating OctaIndex3D into their own systems, a minimal CI setup usually includes:

- `cargo test` over the full workspace on every change.
- A subset of OctaIndex3D's benchmarks focused on the parts you exercise most.
- Compatibility checks that load a small corpus of existing containers to catch accidental format regressions early.

---

## 9.8 Validation Suites and Test Organization

As the codebase grows, organizing tests into coherent suites becomes essential. OctaIndex3D uses several test organization strategies:

### 9.8.1 Test Module Structure

```rust
// src/index64.rs
pub struct Index64 { /* ... */ }

impl Index64 {
    pub fn encode(x: i32, y: i32, z: i32, lod: u8) -> Result<Self> { /* ... */ }
    pub fn decode(&self) -> (i32, i32, i32, u8) { /* ... */ }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod encoding {
        use super::*;

        #[test]
        fn test_round_trip() { /* ... */ }

        #[test]
        fn test_parity_check() { /* ... */ }
    }

    mod decoding {
        use super::*;

        #[test]
        fn test_bit_layout() { /* ... */ }
    }

    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Property tests grouped separately
    }
}
```

### 9.8.2 Integration Test Suites

```rust
// tests/integration/pathfinding.rs
use octaindex3d::*;

#[test]
fn test_astar_pathfinding_warehouse() {
    // End-to-end test: frame → container → A* query
    let frame = FrameRef::local_meters([0.0, 0.0, 0.0], 0.1);
    let mut container = Container::new();

    // Build environment
    // ... populate container ...

    // Run pathfinding
    let start = Index64::encode(0, 0, 0, 5).unwrap();
    let goal = Index64::encode(100, 100, 0, 5).unwrap();

    let path = astar(&container, start, goal);
    assert!(path.is_some());
    assert!(path.unwrap().len() > 0);
}
```

### 9.8.3 Validation Test Suite

Create a comprehensive validation suite that runs all invariants:

```rust
// tests/validation.rs
use octaindex3d::*;

/// Comprehensive validation of a container
fn validate_container(container: &Container) -> Result<(), String> {
    // Check 1: All identifiers have valid parity
    for (idx, _) in container.iter() {
        let (x, y, z, _) = idx.decode();
        if (x + y + z) % 2 != 0 {
            return Err(format!("Parity violation: {:?}", idx));
        }
    }

    // Check 2: All LODs are within valid range
    for (idx, _) in container.iter() {
        if idx.lod() > 20 {
            return Err(format!("Invalid LOD: {}", idx.lod()));
        }
    }

    // Check 3: Parent-child relationships are consistent
    for (idx, _) in container.iter() {
        if idx.lod() > 0 {
            let parent = idx.parent();
            let siblings = parent.children();
            if !siblings.contains(&idx) {
                return Err(format!("Inconsistent parent-child: {:?}", idx));
            }
        }
    }

    Ok(())
}

#[test]
fn test_validate_all_fixtures() {
    let fixtures = std::fs::read_dir("tests/fixtures")
        .expect("fixtures directory");

    for entry in fixtures {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "bcc" {
            let container = Container::load(&path)
                .expect(&format!("Failed to load {:?}", path));

            validate_container(&container)
                .expect(&format!("Validation failed for {:?}", path));
        }
    }
}
```rust

---

## 9.9 Troubleshooting Common Testing Issues

### Flaky Tests

**Problem:** Tests pass most of the time but occasionally fail.

**Solutions:**
- Use fixed random seeds: `proptest_config!(ProptestConfig { rng_seed: Some([0u8; 32]) })`
- Avoid timing-dependent tests; use mock time if necessary
- Disable parallel test execution: `cargo test -- --test-threads=1`

### Slow Test Suites

**Problem:** Test suite takes too long to run frequently.

**Solutions:**
- Split tests into fast (`cargo test`) and slow (`cargo test -- --ignored`)
- Use `#[ignore]` for integration tests
- Run property tests with fewer cases in CI: `PROPTEST_CASES=100 cargo test`

### Platform-Specific Failures

**Problem:** Tests pass locally but fail in CI on different platforms.

**Solutions:**
- Use `f64::EPSILON` for floating-point comparisons instead of exact equality
- Test on multiple platforms locally using Docker or cross-compilation
- Add platform-specific test annotations:
  ```rust
  #[test]
  #[cfg(target_arch = "x86_64")]
  fn test_bmi2_specific() { /* ... */ }
  ```

### Memory Leaks in Tests

**Problem:** Tests consume increasing memory or fail with OOM.

**Solutions:**
- Use `cargo miri test` to detect memory safety issues
- Run tests under Valgrind: `valgrind cargo test`
- Profile tests with `heaptrack` or `massif`

---

## 9.10 Summary

In this chapter, we explored comprehensive testing and validation strategies for OctaIndex3D:

- **Unit tests** verify individual components with golden cases and edge cases (§9.2).
- **Property-based testing** with `proptest` explores large input spaces and uncovers subtle bugs (§9.3).
- **Benchmarking** with `criterion` measures performance rigorously and detects regressions (§9.4).
- **Fuzzing** with `cargo-fuzz` discovers crashes and undefined behavior through random input generation (§9.5).
- **Cross-platform validation** ensures correctness across different architectures and feature flags (§9.6).
- **Continuous integration** automates testing, catches regressions early, and enforces quality standards (§9.7).
- **Validation suites** organize tests and provide comprehensive correctness checks (§9.8).

### Key Takeaways

1. **Test at multiple levels:** unit, integration, property-based, and fuzz testing each catch different classes of bugs.

2. **Make tests deterministic:** use fixed seeds for random tests to ensure reproducibility.

3. **Benchmark rigorously:** use proper statistical methods (criterion) rather than ad-hoc timing.

4. **Automate everything:** CI should run tests, benchmarks, linters, and format checks on every change.

5. **Validate invariants:** BCC parity constraints, parent-child relationships, and coordinate bounds should be tested systematically.

6. **Performance is a feature:** treat performance regressions as bugs by enforcing performance budgets in CI.

With Part III complete, we have traversed the full path from mathematical theory (Part I) through system architecture (Part II) to concrete, tested implementation. The remaining parts focus on real-world applications (Part IV) and advanced topics (Part V) built on this foundation.

---

## Further Reading

### Testing and Property-Based Testing
- *Property-Based Testing with PropEr, Erlang, and Elixir* by Fred Hebert (applies to Rust/proptest)
- [Rust proptest documentation](https://docs.rs/proptest/)
- [QuickCheck paper](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf) by Koen Claessen and John Hughes

### Benchmarking
- [Criterion.rs user guide](https://bheisler.github.io/criterion.rs/book/)
- *Systems Performance* by Brendan Gregg (profiling and performance analysis)
- ["Benchmarking Crimes"](https://www.cse.unsw.edu.au/~gernot/benchmarking-crimes.html) by Gernot Heiser

### Fuzzing
- [The Fuzzing Book](https://www.fuzzingbook.org/) by Andreas Zeller et al.
- [LibFuzzer tutorial](https://llvm.org/docs/LibFuzzer.html)
- [cargo-fuzz guide](https://rust-fuzz.github.io/book/)

### Continuous Integration
- [GitHub Actions documentation](https://docs.github.com/en/actions)
- *Continuous Delivery* by Jez Humble and David Farley
- [Rust CI best practices](https://www.infinyon.com/blog/2021/04/github-actions-best-practices/)

### Spatial Index Testing
- Conway, D.A. & Sloane, N.J.A., "Sphere Packings, Lattices and Groups" — includes validation strategies for lattice algorithms
- Testing strategies from H3 and S2 libraries (open-source geospatial indexing systems)

---

*"Testing shows the presence, not the absence, of bugs."*
— Edsger W. Dijkstra

*"But property-based testing gets you closer to the absence than you'd think."*
— Chapter 9 Summary