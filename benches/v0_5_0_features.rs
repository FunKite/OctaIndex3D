//! Benchmarks for OctaIndex3D v0.5.0 New Features
//!
//! This benchmark suite measures performance of the complete autonomous
//! mapping stack introduced in v0.5.0, including:
//! - Probabilistic occupancy mapping (OccupancyLayer)
//! - TSDF surface reconstruction (TSDFLayer)
//! - ESDF distance fields (ESDFLayer)
//! - Exploration primitives (frontier detection, information gain)
//! - Temporal occupancy with time decay
//!
//! Run with:
//! ```bash
//! cargo bench --bench v0_5_0_features
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use octaindex3d::layers::{
    ESDFLayer, FrontierDetectionConfig, InformationGainConfig, Layer, Measurement,
    OccupancyLayer, TSDFLayer, TemporalConfig, TemporalOccupancyLayer,
};
use octaindex3d::Index64;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

// ============================================================================
// Occupancy Layer Benchmarks
// ============================================================================

/// Benchmark single occupancy update (Bayesian log-odds)
fn bench_occupancy_single_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("occupancy_single_update");

    // Test different confidence levels
    for confidence in [0.5, 0.7, 0.9, 0.95, 0.99] {
        let mut layer = OccupancyLayer::new();
        let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

        group.bench_with_input(
            BenchmarkId::new("confidence", format!("{:.2}", confidence)),
            &confidence,
            |b, &conf| {
                b.iter(|| {
                    layer.update_occupancy(black_box(idx), black_box(true), black_box(conf));
                });
            },
        );
    }
    group.finish();
}

/// Benchmark batch occupancy updates
fn bench_occupancy_batch_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("occupancy_batch_updates");

    for size in [100, 1000, 10000] {
        let mut rng = StdRng::seed_from_u64(101 + size as u64);
        let mut layer = OccupancyLayer::new();

        // Generate random voxel indices
        let indices: Vec<Index64> = (0..size)
            .map(|_| {
                Index64::new(
                    0,
                    0,
                    5,
                    rng.random_range(0..1000),
                    rng.random_range(0..1000),
                    rng.random_range(0..1000),
                )
                .unwrap()
            })
            .collect();

        let measurements: Vec<bool> = (0..size).map(|_| rng.random_bool(0.3)).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("updates", size), &size, |b, _| {
            b.iter(|| {
                for (idx, &occupied) in indices.iter().zip(measurements.iter()) {
                    layer.update_occupancy(black_box(*idx), black_box(occupied), black_box(0.8));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark ray integration (depth camera simulation)
fn bench_occupancy_ray_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("occupancy_ray_integration");
    let mut layer = OccupancyLayer::new();

    // Simulate ray from origin to target
    for ray_length in [1.0, 5.0, 10.0] {
        let origin = (0.0, 0.0, 0.0);
        let direction = (1.0, 0.0, 0.0); // X-axis
        let hit_distance = ray_length;

        group.bench_with_input(
            BenchmarkId::new("ray_meters", format!("{:.1}", ray_length)),
            &ray_length,
            |b, _| {
                b.iter(|| {
                    layer.integrate_ray(
                        black_box(origin),
                        black_box(direction),
                        black_box(hit_distance),
                        black_box(0.05), // 5cm voxels
                        black_box(0.8),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark occupancy state queries
fn bench_occupancy_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("occupancy_queries");
    let mut rng = StdRng::seed_from_u64(102);

    // Build a populated occupancy map
    let mut layer = OccupancyLayer::new();
    for _ in 0..10000 {
        let idx = Index64::new(
            0,
            0,
            5,
            rng.random_range(0..200),
            rng.random_range(0..200),
            rng.random_range(0..200),
        )
        .unwrap();
        layer.update_occupancy(idx, rng.random_bool(0.3), 0.8);
    }

    // Query random voxels
    let query_indices: Vec<Index64> = (0..1000)
        .map(|_| {
            Index64::new(
                0,
                0,
                5,
                rng.random_range(0..200),
                rng.random_range(0..200),
                rng.random_range(0..200),
            )
            .unwrap()
        })
        .collect();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("get_state_batch", |b| {
        b.iter(|| {
            let mut states = Vec::with_capacity(query_indices.len());
            for &idx in &query_indices {
                states.push(layer.get_state(black_box(idx)));
            }
            black_box(states)
        });
    });

    group.bench_function("get_probability_batch", |b| {
        b.iter(|| {
            let mut probs = Vec::with_capacity(query_indices.len());
            for &idx in &query_indices {
                probs.push(layer.get_probability(black_box(idx)));
            }
            black_box(probs)
        });
    });

    group.finish();
}

// ============================================================================
// TSDF Layer Benchmarks
// ============================================================================

/// Benchmark TSDF single measurement integration
fn bench_tsdf_single_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("tsdf_single_update");
    let mut layer = TSDFLayer::new(0.1); // 10cm truncation

    let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

    for distance in [0.01, 0.05, 0.1, 0.2] {
        let measurement = Measurement::depth(distance, 1.0);

        group.bench_with_input(
            BenchmarkId::new("distance_m", format!("{:.2}", distance)),
            &measurement,
            |b, meas| {
                b.iter(|| {
                    layer.update(black_box(idx), black_box(meas)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark TSDF batch integration (simulating depth frame)
fn bench_tsdf_depth_frame_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("tsdf_depth_frame");

    // Simulate different depth camera resolutions
    for num_points in [1000, 10000, 50000] {
        // 1K, 10K, 50K points (typical depth cameras)
        let mut rng = StdRng::seed_from_u64(103 + num_points as u64);
        let mut layer = TSDFLayer::new(0.1);

        // Generate simulated depth points
        let points: Vec<(Index64, f32)> = (0..num_points)
            .map(|_| {
                let idx = Index64::new(
                    0,
                    0,
                    5,
                    rng.random_range(0..500),
                    rng.random_range(0..500),
                    rng.random_range(0..500),
                )
                .unwrap();
                let distance = rng.random_range(0.01..5.0); // 1cm to 5m
                (idx, distance)
            })
            .collect();

        group.throughput(Throughput::Elements(num_points as u64));
        group.bench_with_input(
            BenchmarkId::new("points", num_points),
            &points,
            |b, points| {
                b.iter(|| {
                    for &(idx, distance) in points {
                        let measurement = Measurement::depth(distance, 1.0);
                        layer.update(black_box(idx), black_box(&measurement)).unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark TSDF distance queries
fn bench_tsdf_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("tsdf_queries");
    let mut rng = StdRng::seed_from_u64(104);

    // Build populated TSDF
    let mut layer = TSDFLayer::new(0.1);
    for _ in 0..10000 {
        let idx = Index64::new(
            0,
            0,
            5,
            rng.random_range(0..200),
            rng.random_range(0..200),
            rng.random_range(0..200),
        )
        .unwrap();
        let measurement = Measurement::depth(rng.random_range(0.01..1.0), 1.0);
        layer.update(idx, &measurement).unwrap();
    }

    let query_indices: Vec<Index64> = (0..1000)
        .map(|_| {
            Index64::new(
                0,
                0,
                5,
                rng.random_range(0..200),
                rng.random_range(0..200),
                rng.random_range(0..200),
            )
            .unwrap()
        })
        .collect();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("get_distance_batch", |b| {
        b.iter(|| {
            let mut distances = Vec::with_capacity(query_indices.len());
            for &idx in &query_indices {
                distances.push(layer.get_distance(black_box(idx)));
            }
            black_box(distances)
        });
    });

    group.finish();
}

// ============================================================================
// ESDF Layer Benchmarks
// ============================================================================

/// Benchmark ESDF computation from TSDF
fn bench_esdf_from_tsdf(c: &mut Criterion) {
    let mut group = c.benchmark_group("esdf_from_tsdf");
    let mut rng = StdRng::seed_from_u64(105);

    // Create TSDF layers of different sizes
    for num_voxels in [1000, 5000, 10000] {
        let mut tsdf = TSDFLayer::new(0.1);

        // Populate TSDF with surface data
        for _ in 0..num_voxels {
            let idx = Index64::new(
                0,
                0,
                5,
                rng.random_range(0..100),
                rng.random_range(0..100),
                rng.random_range(0..100),
            )
            .unwrap();
            let measurement = Measurement::depth(rng.random_range(0.01..0.1), 1.0);
            tsdf.update(idx, &measurement).unwrap();
        }

        group.throughput(Throughput::Elements(num_voxels as u64));
        group.bench_with_input(
            BenchmarkId::new("voxels", num_voxels),
            &tsdf,
            |b, tsdf_layer| {
                b.iter(|| {
                    let mut esdf = ESDFLayer::new(0.02, 5.0);
                    esdf.compute_from_tsdf(black_box(tsdf_layer), black_box(0.01))
                        .unwrap();
                    black_box(esdf)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark ESDF distance queries
fn bench_esdf_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("esdf_queries");
    let mut rng = StdRng::seed_from_u64(106);

    // Build ESDF from TSDF
    let mut tsdf = TSDFLayer::new(0.1);
    for _ in 0..5000 {
        let idx = Index64::new(
            0,
            0,
            5,
            rng.random_range(0..100),
            rng.random_range(0..100),
            rng.random_range(0..100),
        )
        .unwrap();
        let measurement = Measurement::depth(rng.random_range(0.01..0.1), 1.0);
        tsdf.update(idx, &measurement).unwrap();
    }

    let mut esdf = ESDFLayer::new(0.02, 5.0);
    esdf.compute_from_tsdf(&tsdf, 0.01).unwrap();

    let query_indices: Vec<Index64> = (0..1000)
        .map(|_| {
            Index64::new(
                0,
                0,
                5,
                rng.random_range(0..100),
                rng.random_range(0..100),
                rng.random_range(0..100),
            )
            .unwrap()
        })
        .collect();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("get_distance_batch", |b| {
        b.iter(|| {
            let mut distances = Vec::with_capacity(query_indices.len());
            for &idx in &query_indices {
                distances.push(esdf.get_distance(black_box(idx)));
            }
            black_box(distances)
        });
    });

    group.finish();
}

// ============================================================================
// Exploration Primitives Benchmarks
// ============================================================================

/// Benchmark frontier detection
fn bench_frontier_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("frontier_detection");
    let mut rng = StdRng::seed_from_u64(107);

    // Create occupancy maps of different sizes
    for map_size in [50, 100, 200] {
        let mut layer = OccupancyLayer::new();

        // Create partially explored environment
        for x in 0..map_size {
            for y in 0..map_size {
                for z in 0..40 {
                    if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                        // Central region is explored (free)
                        if x > 10 && x < map_size - 10 && y > 10 && y < map_size - 10 {
                            layer.update_occupancy(idx, false, 0.8);
                        }

                        // Add some obstacles
                        if x > map_size / 3
                            && x < 2 * map_size / 3
                            && y > map_size / 3
                            && y < 2 * map_size / 3
                            && rng.random_bool(0.1)
                        {
                            layer.update_occupancy(idx, true, 0.9);
                        }
                    }
                }
            }
        }

        let config = FrontierDetectionConfig::default();

        group.bench_with_input(
            BenchmarkId::new("map_size", map_size),
            &map_size,
            |b, _| {
                b.iter(|| {
                    let frontiers = layer.detect_frontiers(black_box(&config));
                    black_box(frontiers)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark information gain calculation
fn bench_information_gain(c: &mut Criterion) {
    let mut group = c.benchmark_group("information_gain");
    let mut rng = StdRng::seed_from_u64(108);

    // Create occupancy map with frontiers
    let mut layer = OccupancyLayer::new();
    for x in 0..100 {
        for y in 0..100 {
            for z in 0..40 {
                if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                    if x > 20 && x < 80 && y > 20 && y < 80 {
                        layer.update_occupancy(idx, false, 0.8);
                    }
                    if x > 40 && x < 60 && y > 40 && y < 60 && rng.random_bool(0.1) {
                        layer.update_occupancy(idx, true, 0.9);
                    }
                }
            }
        }
    }

    let config = InformationGainConfig::default();
    let viewpoint = (2.5, 2.5, 1.0); // 2.5m, 2.5m, 1m
    let direction = (1.0, 0.0, 0.0); // Looking in +X direction

    group.bench_function("single_viewpoint", |b| {
        b.iter(|| {
            let gain = layer.information_gain_from(
                black_box(viewpoint),
                black_box(direction),
                black_box(&config),
            );
            black_box(gain)
        });
    });

    group.finish();
}

/// Benchmark viewpoint candidate generation
fn bench_viewpoint_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewpoint_generation");
    let mut rng = StdRng::seed_from_u64(109);

    // Create occupancy map with frontiers
    let mut layer = OccupancyLayer::new();
    for x in 0..100 {
        for y in 0..100 {
            for z in 0..40 {
                if let Ok(idx) = Index64::new(0, 0, 5, x, y, z) {
                    if x > 20 && x < 80 && y > 20 && y < 80 {
                        layer.update_occupancy(idx, false, 0.8);
                    }
                }
            }
        }
    }

    // Detect frontiers
    let frontier_config = FrontierDetectionConfig::default();
    if let Ok(frontiers) = layer.detect_frontiers(&frontier_config) {
        if !frontiers.is_empty() {
            let ig_config = InformationGainConfig::default();

            group.bench_function("generate_candidates", |b| {
                b.iter(|| {
                    let viewpoints = layer.generate_viewpoint_candidates(
                        black_box(&frontiers),
                        black_box(&ig_config),
                    );
                    black_box(viewpoints)
                });
            });
        }
    }

    group.finish();
}

// ============================================================================
// Temporal Occupancy Benchmarks
// ============================================================================

/// Benchmark temporal occupancy updates with time decay
fn bench_temporal_occupancy_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("temporal_occupancy");
    let config = TemporalConfig {
        decay_rate: 0.1,
        max_age: 5.0,
        min_measurements_for_velocity: 3,
        track_dynamics: true,
    };
    let mut layer = TemporalOccupancyLayer::with_config(config);

    let idx = Index64::new(0, 0, 5, 100, 100, 100).unwrap();

    group.bench_function("single_update", |b| {
        b.iter(|| {
            layer.update_occupancy(black_box(idx), black_box(true), black_box(0.8));
        });
    });

    group.finish();
}

/// Benchmark temporal batch updates
fn bench_temporal_batch_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("temporal_batch");
    let mut rng = StdRng::seed_from_u64(110);

    for num_voxels in [1000, 5000, 10000] {
        let config = TemporalConfig {
            decay_rate: 0.1,
            max_age: 5.0,
            min_measurements_for_velocity: 3,
            track_dynamics: true,
        };
        let mut layer = TemporalOccupancyLayer::with_config(config);

        // Populate with voxels
        let indices: Vec<Index64> = (0..num_voxels)
            .filter_map(|_| {
                Index64::new(
                    0,
                    0,
                    5,
                    rng.random_range(0..200),
                    rng.random_range(0..200),
                    rng.random_range(0..200),
                )
                .ok()
            })
            .collect();

        group.throughput(Throughput::Elements(num_voxels as u64));
        group.bench_with_input(
            BenchmarkId::new("voxels", num_voxels),
            &num_voxels,
            |b, _| {
                b.iter(|| {
                    for &idx in &indices {
                        layer.update_occupancy(idx, rng.random_bool(0.3), 0.8);
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    occupancy_benches,
    bench_occupancy_single_update,
    bench_occupancy_batch_updates,
    bench_occupancy_ray_integration,
    bench_occupancy_queries,
);

criterion_group!(
    tsdf_benches,
    bench_tsdf_single_update,
    bench_tsdf_depth_frame_integration,
    bench_tsdf_queries,
);

criterion_group!(esdf_benches, bench_esdf_from_tsdf, bench_esdf_queries,);

criterion_group!(
    exploration_benches,
    bench_frontier_detection,
    bench_information_gain,
    bench_viewpoint_generation,
);

criterion_group!(
    temporal_benches,
    bench_temporal_occupancy_updates,
    bench_temporal_batch_updates,
);

criterion_main!(
    occupancy_benches,
    tsdf_benches,
    esdf_benches,
    exploration_benches,
    temporal_benches,
);
