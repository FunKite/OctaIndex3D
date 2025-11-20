use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use octaindex3d::{lattice, morton, neighbors, Galactic128, Index64, Route64};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

// Benchmark Morton encoding/decoding
fn bench_morton_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton_encode");

    // Generate diverse random coordinates with fixed seed for reproducibility
    let mut rng = StdRng::seed_from_u64(42);
    let mut coords = Vec::new();

    // Include edge cases
    coords.push((0u16, 0u16, 0u16));
    coords.push((65535u16, 65535u16, 65535u16));

    // Add random samples covering the full range
    for _ in 0..10 {
        coords.push((
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
        ));
    }

    for (x, y, z) in coords {
        group.bench_with_input(
            BenchmarkId::new("encode", format!("{}_{}_{}", x, y, z)),
            &(x, y, z),
            |b, &(x, y, z)| {
                b.iter(|| morton::morton_encode(black_box(x), black_box(y), black_box(z)));
            },
        );
    }
    group.finish();
}

fn bench_morton_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("morton_decode");

    // Generate random Morton codes from random coordinates
    let mut rng = StdRng::seed_from_u64(43);
    let mut codes = Vec::new();

    // Edge cases
    codes.push(0u64);
    codes.push(morton::morton_encode(65535, 65535, 65535));

    // Random codes
    for _ in 0..10 {
        codes.push(morton::morton_encode(
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
        ));
    }

    for code in codes {
        group.bench_with_input(BenchmarkId::new("decode", code), &code, |b, &code| {
            b.iter(|| morton::morton_decode(black_box(code)));
        });
    }
    group.finish();
}

// Benchmark Index64 operations
fn bench_index64_creation(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(44);

    c.bench_function("index64_new", |b| {
        b.iter(|| {
            let lod = rng.random_range(0..16u8);
            let x = rng.random::<u16>();
            let y = rng.random::<u16>();
            let z = rng.random::<u16>();
            Index64::new(
                black_box(0),
                black_box(0),
                black_box(lod),
                black_box(x),
                black_box(y),
                black_box(z),
            )
        });
    });
}

fn bench_index64_extract(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(45);
    let x = rng.random::<u16>();
    let y = rng.random::<u16>();
    let z = rng.random::<u16>();
    let index = Index64::new(0, 0, 5, x, y, z).unwrap();

    c.bench_function("index64_extract_coords", |b| {
        b.iter(|| {
            let idx = black_box(index);
            (idx.lod(), idx.decode_coords())
        });
    });
}

// Benchmark Route64 operations
fn bench_route64_creation(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(46);

    c.bench_function("route64_new", |b| {
        b.iter(|| {
            // Generate random even coordinates (parity requirement)
            let x = (rng.random::<i32>() % 10000) * 2;
            let y = (rng.random::<i32>() % 10000) * 2;
            let z = (rng.random::<i32>() % 10000) * 2;
            Route64::new(black_box(0), black_box(x), black_box(y), black_box(z))
        });
    });
}

// Benchmark neighbor calculations
fn bench_neighbors_route64(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbors");
    let mut rng = StdRng::seed_from_u64(47);

    // Generate diverse random routes
    let mut routes = Vec::new();
    routes.push(Route64::new(0, 0, 0, 0).unwrap()); // Origin

    for _ in 0..10 {
        let x = (rng.random::<i32>() % 10000) * 2;
        let y = (rng.random::<i32>() % 10000) * 2;
        let z = (rng.random::<i32>() % 10000) * 2;
        routes.push(Route64::new(0, x, y, z).unwrap());
    }

    for route in routes {
        group.bench_with_input(
            BenchmarkId::new(
                "route64",
                format!("{}_{}_{}", route.x(), route.y(), route.z()),
            ),
            &route,
            |b, &route| {
                b.iter(|| neighbors::neighbors_route64(black_box(route)));
            },
        );
    }
    group.finish();
}

// Benchmark lattice coordinate operations
fn bench_lattice_coord_creation(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(48);

    c.bench_function("lattice_coord_new", |b| {
        b.iter(|| {
            // Generate even coordinates to satisfy parity requirements
            let x = (rng.random_range(-5000..5000)) * 2;
            let y = (rng.random_range(-5000..5000)) * 2;
            let z = (rng.random_range(-5000..5000)) * 2;
            lattice::LatticeCoord::new(black_box(x), black_box(y), black_box(z))
        });
    });
}

fn bench_lattice_distance(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(49);
    // Generate even coordinates to satisfy parity requirements
    let coord1 = lattice::LatticeCoord::new(
        (rng.random_range(-2500..2500)) * 2,
        (rng.random_range(-2500..2500)) * 2,
        (rng.random_range(-2500..2500)) * 2,
    )
    .unwrap();
    let coord2 = lattice::LatticeCoord::new(
        (rng.random_range(-2500..2500)) * 2,
        (rng.random_range(-2500..2500)) * 2,
        (rng.random_range(-2500..2500)) * 2,
    )
    .unwrap();

    c.bench_function("lattice_distance", |b| {
        b.iter(|| black_box(coord1).distance_to(&black_box(coord2)));
    });
}

// Benchmark batch operations
fn bench_batch_index_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    for size in [100, 1000, 10000] {
        // Pre-generate random coordinates for this batch size
        let mut rng = StdRng::seed_from_u64(50 + size as u64);
        let coords: Vec<(u16, u16, u16)> = (0..size)
            .map(|_| (rng.random(), rng.random(), rng.random()))
            .collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("index64_batch", size),
            &coords,
            |b, coords| {
                b.iter(|| {
                    let mut results = Vec::with_capacity(coords.len());
                    for &(x, y, z) in coords {
                        results.push(Index64::new(0, 0, 5, x, y, z).unwrap());
                    }
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

fn bench_batch_neighbors(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_neighbors");

    for size in [100, 1000, 10000] {
        // Generate truly random routes with proper parity
        let mut rng = StdRng::seed_from_u64(60 + size as u64);
        let routes: Vec<_> = (0..size)
            .map(|_| {
                let x = (rng.random::<i32>() % 10000) * 2;
                let y = (rng.random::<i32>() % 10000) * 2;
                let z = (rng.random::<i32>() % 10000) * 2;
                Route64::new(0, x, y, z).unwrap()
            })
            .collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("neighbors_batch", size),
            &routes,
            |b, routes| {
                b.iter(|| {
                    let mut results = Vec::with_capacity(routes.len() * 14);
                    for &route in routes {
                        results.extend(neighbors::neighbors_route64(black_box(route)));
                    }
                    black_box(results)
                });
            },
        );
    }

    group.finish();
}

// Benchmark Galactic128 operations
fn bench_galactic128_creation(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(70);

    c.bench_function("galactic128_new", |b| {
        b.iter(|| {
            let frame = rng.random::<u8>();
            let scale_mant = rng.random_range(0..16u8);
            let scale_tier = rng.random_range(0..4u8);
            let lod = rng.random_range(0..16u8);
            let attr_usr = rng.random::<u8>();
            // Generate random even coordinates (parity requirement)
            let x = (rng.random::<i32>() % 100000) * 2;
            let y = (rng.random::<i32>() % 100000) * 2;
            let z = (rng.random::<i32>() % 100000) * 2;
            Galactic128::new(
                black_box(frame),
                black_box(scale_mant),
                black_box(scale_tier),
                black_box(lod),
                black_box(attr_usr),
                black_box(x),
                black_box(y),
                black_box(z),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_morton_encode,
    bench_morton_decode,
    bench_index64_creation,
    bench_index64_extract,
    bench_route64_creation,
    bench_neighbors_route64,
    bench_lattice_coord_creation,
    bench_lattice_distance,
    bench_batch_index_creation,
    bench_batch_neighbors,
    bench_galactic128_creation,
);
criterion_main!(benches);
