//! Benchmarks for tier-1 architecture-specific optimizations
//!
//! Tests BMI2, cache optimizations, prefetching, and specialized kernels

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use octaindex3d::Route64;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[cfg(target_arch = "x86_64")]
use octaindex3d::performance::arch_optimized::{
    batch_morton_decode_bmi2, batch_morton_encode_bmi2, has_bmi2,
};

use octaindex3d::performance::fast_neighbors::{
    batch_neighbors_auto, batch_neighbors_fast, batch_neighbors_medium, neighbors_route64_fast,
};

use octaindex3d::neighbors;

/// Generate test routes
fn generate_routes(count: usize, seed: u64) -> Vec<Route64> {
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

// Benchmark Morton encoding: standard vs BMI2
#[cfg(target_arch = "x86_64")]
fn bench_morton_encoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton_encoding");

    for size in [100, 1000, 10000] {
        let coords = generate_coords(size, 1000);

        group.throughput(Throughput::Elements(size as u64));

        // Standard implementation
        group.bench_with_input(BenchmarkId::new("standard", size), &coords, |b, coords| {
            b.iter(|| {
                let result: Vec<u64> = coords
                    .iter()
                    .map(|&(x, y, z)| morton::morton_encode(x, y, z))
                    .collect();
                black_box(result)
            });
        });

        // BMI2 implementation (if available)
        if has_bmi2() {
            group.bench_with_input(BenchmarkId::new("bmi2", size), &coords, |b, coords| {
                b.iter(|| black_box(batch_morton_encode_bmi2(black_box(coords))));
            });
        }
    }

    group.finish();
}

// Benchmark Morton decoding: standard vs BMI2
#[cfg(target_arch = "x86_64")]
fn bench_morton_decoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton_decoding");

    for size in [100, 1000, 10000] {
        let coords = generate_coords(size, 2000);
        let codes: Vec<u64> = coords
            .iter()
            .map(|&(x, y, z)| morton::morton_encode(x, y, z))
            .collect();

        group.throughput(Throughput::Elements(size as u64));

        // Standard implementation
        group.bench_with_input(BenchmarkId::new("standard", size), &codes, |b, codes| {
            b.iter(|| {
                let result: Vec<(u16, u16, u16)> = codes
                    .iter()
                    .map(|&code| morton::morton_decode(code))
                    .collect();
                black_box(result)
            });
        });

        // BMI2 implementation (if available)
        if has_bmi2() {
            group.bench_with_input(BenchmarkId::new("bmi2", size), &codes, |b, codes| {
                b.iter(|| black_box(batch_morton_decode_bmi2(black_box(codes))));
            });
        }
    }

    group.finish();
}

// Benchmark neighbor calculations: standard vs fast kernels
fn bench_neighbor_kernels(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_kernels");

    for size in [10, 100, 1000, 10000] {
        let routes = generate_routes(size, 3000);

        group.throughput(Throughput::Elements(size as u64));

        // Standard implementation
        group.bench_with_input(BenchmarkId::new("standard", size), &routes, |b, routes| {
            b.iter(|| {
                let mut result = Vec::with_capacity(routes.len() * 14);
                for &route in routes {
                    result.extend(neighbors::neighbors_route64(black_box(route)));
                }
                black_box(result)
            });
        });

        // Fast kernel (unrolled)
        group.bench_with_input(
            BenchmarkId::new("fast_unrolled", size),
            &routes,
            |b, routes| {
                b.iter(|| {
                    let mut result = Vec::with_capacity(routes.len() * 14);
                    for &route in routes {
                        result.extend_from_slice(&neighbors_route64_fast(black_box(route)));
                    }
                    black_box(result)
                });
            },
        );

        // Batch with prefetching (tier-1 archs only)
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            group.bench_with_input(
                BenchmarkId::new("batch_prefetch", size),
                &routes,
                |b, routes| {
                    b.iter(|| black_box(batch_neighbors_fast(black_box(routes))));
                },
            );
        }

        // Auto selection (chooses best kernel)
        group.bench_with_input(
            BenchmarkId::new("auto_select", size),
            &routes,
            |b, routes| {
                b.iter(|| black_box(batch_neighbors_auto(black_box(routes))));
            },
        );
    }

    group.finish();
}

// Benchmark cache-optimized medium batches
fn bench_cache_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_optimization");

    let sizes = [500, 1000, 2000, 5000];

    for &size in &sizes {
        let routes = generate_routes(size, 4000);

        group.throughput(Throughput::Elements(size as u64));

        // Standard sequential processing
        group.bench_with_input(
            BenchmarkId::new("sequential", size),
            &routes,
            |b, routes| {
                b.iter(|| {
                    let mut result = Vec::with_capacity(routes.len() * 14);
                    for &route in routes {
                        result.extend_from_slice(&neighbors_route64_fast(route));
                    }
                    black_box(result)
                });
            },
        );

        // Cache-blocked processing
        group.bench_with_input(
            BenchmarkId::new("cache_blocked", size),
            &routes,
            |b, routes| {
                b.iter(|| black_box(batch_neighbors_medium(black_box(routes))));
            },
        );
    }

    group.finish();
}

// Benchmark single route neighbor calculation
fn bench_single_route_neighbors(c: &mut Criterion) {
    let route = Route64::new(0, 100, 200, 300).unwrap();

    c.benchmark_group("single_neighbor")
        .bench_function("standard", |b| {
            b.iter(|| black_box(neighbors::neighbors_route64(black_box(route))));
        })
        .bench_function("fast_unrolled", |b| {
            b.iter(|| black_box(neighbors_route64_fast(black_box(route))));
        });
}

// Platform-specific benches
#[cfg(target_arch = "x86_64")]
criterion_group!(
    tier1_benches,
    bench_morton_encoding_comparison,
    bench_morton_decoding_comparison,
    bench_neighbor_kernels,
    bench_cache_optimization,
    bench_single_route_neighbors,
);

#[cfg(not(target_arch = "x86_64"))]
criterion_group!(
    tier1_benches,
    bench_neighbor_kernels,
    bench_cache_optimization,
    bench_single_route_neighbors,
);

criterion_main!(tier1_benches);
