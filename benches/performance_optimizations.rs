//! Benchmarks for performance optimizations (SIMD, parallel, GPU)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use octaindex3d::{BatchIndexBuilder, BatchNeighborCalculator, Route64};

#[cfg(feature = "parallel")]
use octaindex3d::{ParallelBatchIndexBuilder, ParallelBatchNeighborCalculator};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// Generate test routes with proper parity
fn generate_test_routes(count: usize, seed: u64) -> Vec<Route64> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .map(|_| {
            let x = (rng.gen::<i32>() % 10000) * 2;
            let y = (rng.gen::<i32>() % 10000) * 2;
            let z = (rng.gen::<i32>() % 10000) * 2;
            Route64::new(0, x, y, z).unwrap()
        })
        .collect()
}

// Benchmark batch neighbor calculations - single-threaded vs parallel
fn bench_batch_neighbors_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_neighbors_comparison");

    for size in [100, 1000, 10000] {
        let routes = generate_test_routes(size, 100);

        group.throughput(Throughput::Elements(size as u64));

        // Single-threaded baseline
        group.bench_with_input(
            BenchmarkId::new("single_threaded", size),
            &routes,
            |b, routes| {
                let calc = BatchNeighborCalculator::new();
                b.iter(|| black_box(calc.calculate(black_box(routes))));
            },
        );

        // Parallel with Rayon
        #[cfg(feature = "parallel")]
        {
            group.bench_with_input(BenchmarkId::new("parallel", size), &routes, |b, routes| {
                let calc = ParallelBatchNeighborCalculator::new();
                b.iter(|| black_box(calc.calculate(black_box(routes))));
            });
        }
    }

    group.finish();
}

// Benchmark batch index creation - single-threaded vs parallel
fn bench_batch_index_creation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_index_creation_comparison");

    for size in [100, 1000, 10000] {
        // Generate random coordinates
        let mut rng = StdRng::seed_from_u64(200);
        let frame_ids = vec![0u8; size];
        let dimension_ids = vec![0u8; size];
        let lods = vec![5u8; size];
        let x_coords: Vec<u16> = (0..size).map(|_| rng.gen()).collect();
        let y_coords: Vec<u16> = (0..size).map(|_| rng.gen()).collect();
        let z_coords: Vec<u16> = (0..size).map(|_| rng.gen()).collect();

        group.throughput(Throughput::Elements(size as u64));

        // Single-threaded baseline
        group.bench_with_input(BenchmarkId::new("single_threaded", size), &size, |b, _| {
            let builder = BatchIndexBuilder::new();
            b.iter(|| {
                black_box(builder.build(
                    black_box(&frame_ids),
                    black_box(&dimension_ids),
                    black_box(&lods),
                    black_box(&x_coords),
                    black_box(&y_coords),
                    black_box(&z_coords),
                ))
            });
        });

        // Parallel with Rayon
        #[cfg(feature = "parallel")]
        {
            group.bench_with_input(BenchmarkId::new("parallel", size), &size, |b, _| {
                let builder = ParallelBatchIndexBuilder::new();
                b.iter(|| {
                    black_box(builder.build(
                        black_box(&frame_ids),
                        black_box(&dimension_ids),
                        black_box(&lods),
                        black_box(&x_coords),
                        black_box(&y_coords),
                        black_box(&z_coords),
                    ))
                });
            });
        }
    }

    group.finish();
}

// Benchmark scaling characteristics
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    let sizes = [10, 100, 1000, 5000, 10000];

    for &size in &sizes {
        let routes = generate_test_routes(size, 300);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("neighbors_single", size),
            &routes,
            |b, routes| {
                let calc = BatchNeighborCalculator::new();
                b.iter(|| black_box(calc.calculate(black_box(routes))));
            },
        );

        #[cfg(feature = "parallel")]
        {
            if size >= 500 {
                // Only benchmark parallel for larger sizes
                group.bench_with_input(
                    BenchmarkId::new("neighbors_parallel", size),
                    &routes,
                    |b, routes| {
                        let calc = ParallelBatchNeighborCalculator::new();
                        b.iter(|| black_box(calc.calculate(black_box(routes))));
                    },
                );
            }
        }
    }

    group.finish();
}

// Memory efficiency benchmarks
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Test grouped vs flat neighbor output
    let size = 1000;
    let routes = generate_test_routes(size, 400);

    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("neighbors_flat", |b| {
        let calc = BatchNeighborCalculator::new();
        b.iter(|| black_box(calc.calculate(black_box(&routes))));
    });

    group.bench_function("neighbors_grouped", |b| {
        let calc = BatchNeighborCalculator::new();
        b.iter(|| black_box(calc.calculate_grouped(black_box(&routes))));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_neighbors_comparison,
    bench_batch_index_creation_comparison,
    bench_scaling,
    bench_memory_patterns,
);
criterion_main!(benches);
