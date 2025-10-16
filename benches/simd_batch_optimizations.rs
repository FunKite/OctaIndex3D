//! Benchmarks for SIMD batch operations and core optimizations
//!
//! Measures performance of:
//! - Batch Index64 encoding/decoding
//! - Batch route validation
//! - Batch distance calculations
//! - Batch bounding box queries
//! - Batch Morton encoding/decoding
//! - Batch Hilbert encoding/decoding (if feature enabled)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use octaindex3d::performance::*;
use octaindex3d::{Index64, Route64};

fn generate_coords(n: usize) -> Vec<(u16, u16, u16)> {
    (0..n)
        .map(|i| {
            let x = ((i * 123) % 65536) as u16;
            let y = ((i * 456) % 65536) as u16;
            let z = ((i * 789) % 65536) as u16;
            (x, y, z)
        })
        .collect()
}

fn generate_routes(n: usize) -> Vec<Route64> {
    (0..n)
        .map(|i| {
            let x = ((i * 2) % 10000) as i32;
            let y = ((i * 2) % 10000) as i32;
            let z = (((n - i) * 2) % 10000) as i32;
            Route64::new(0, x, y, z).unwrap()
        })
        .collect()
}

fn bench_batch_index64_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_index64_encode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);
        let frame_ids = vec![0u8; size];
        let tiers = vec![0u8; size];
        let lods = vec![0u8; size];

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_index64_encode(
                    black_box(&frame_ids),
                    black_box(&tiers),
                    black_box(&lods),
                    black_box(&coords),
                );
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_index64_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_index64_decode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);
        let indices: Vec<Index64> = coords
            .iter()
            .map(|&(x, y, z)| Index64::new(0, 0, 0, x, y, z).unwrap())
            .collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_index64_decode(black_box(&indices));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_validate_routes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_validate_routes");

    for size in [10, 100, 1000, 10000] {
        let routes = generate_routes(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_validate_routes(black_box(&routes));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_manhattan_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_manhattan_distance");

    let source = Route64::new(0, 5000, 5000, 5000).unwrap();

    for size in [10, 100, 1000, 10000] {
        let targets = generate_routes(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_manhattan_distance(black_box(source), black_box(&targets));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_euclidean_distance_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_euclidean_distance_squared");

    let source = Route64::new(0, 5000, 5000, 5000).unwrap();

    for size in [10, 100, 1000, 10000] {
        let targets = generate_routes(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_euclidean_distance_squared(black_box(source), black_box(&targets));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_bounding_box_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_bounding_box_query");

    for size in [100, 1000, 10000] {
        let routes = generate_routes(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_bounding_box_query(
                    black_box(&routes),
                    black_box(0),
                    black_box(5000),
                    black_box(0),
                    black_box(5000),
                    black_box(0),
                    black_box(5000),
                );
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_morton_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_morton_encode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_morton_encode(black_box(&coords));
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_batch_morton_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_morton_decode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);
        let codes: Vec<u64> = coords
            .iter()
            .map(|&(x, y, z)| octaindex3d::morton::morton_encode(x, y, z))
            .collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_morton_decode(black_box(&codes));
                black_box(result)
            })
        });
    }

    group.finish();
}

#[cfg(feature = "hilbert")]
fn bench_batch_hilbert_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_hilbert_encode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_hilbert_encode(black_box(&coords));
                black_box(result)
            })
        });
    }

    group.finish();
}

#[cfg(feature = "hilbert")]
fn bench_batch_hilbert_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_hilbert_decode");

    for size in [10, 100, 1000, 10000] {
        let coords = generate_coords(size);
        let codes: Vec<u64> = coords
            .iter()
            .map(|&(x, y, z)| octaindex3d::hilbert::hilbert3d_encode(x, y, z))
            .collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let result = batch_hilbert_decode(black_box(&codes));
                black_box(result)
            })
        });
    }

    group.finish();
}

// Comparison benchmark: SIMD batch vs scalar operations
fn bench_morton_scalar_vs_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton_scalar_vs_batch");

    let size = 1000;
    let coords = generate_coords(size);

    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("scalar", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(size);
            for &(x, y, z) in black_box(&coords) {
                results.push(octaindex3d::morton::morton_encode(x, y, z));
            }
            black_box(results)
        })
    });

    group.bench_function("batch", |b| {
        b.iter(|| {
            let result = batch_morton_encode(black_box(&coords));
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_index64_encode,
    bench_batch_index64_decode,
    bench_batch_validate_routes,
    bench_batch_manhattan_distance,
    bench_batch_euclidean_distance_squared,
    bench_batch_bounding_box_query,
    bench_batch_morton_encode,
    bench_batch_morton_decode,
    bench_morton_scalar_vs_batch,
);

#[cfg(feature = "hilbert")]
criterion_group!(
    hilbert_benches,
    bench_batch_hilbert_encode,
    bench_batch_hilbert_decode,
);

#[cfg(feature = "hilbert")]
criterion_main!(benches, hilbert_benches);

#[cfg(not(feature = "hilbert"))]
criterion_main!(benches);
