// OctoMap Competitive Benchmark
// Measures performance for comparison with OctaIndex3D

#include <octomap/octomap.h>
#include <octomap/OcTree.h>
#include <chrono>
#include <iostream>
#include <vector>
#include <random>
#include <iomanip>

using namespace octomap;
using namespace std;
using namespace std::chrono;

class BenchmarkRunner {
private:
    OcTree tree;
    mt19937 rng;

public:
    BenchmarkRunner(double resolution) : tree(resolution), rng(12345) {}

    // Benchmark 1: Single point insertion
    void bench_single_insertions(int count) {
        uniform_real_distribution<double> dist(-10.0, 10.0);

        auto start = high_resolution_clock::now();
        for (int i = 0; i < count; i++) {
            point3d p(dist(rng), dist(rng), dist(rng));
            tree.updateNode(p, true);
        }
        auto end = high_resolution_clock::now();

        auto duration = duration_cast<nanoseconds>(end - start).count();
        double per_insert = (double)duration / count;
        double throughput = 1e9 / per_insert;

        cout << "Single Insertions (" << count << "):" << endl;
        cout << "  Total time: " << duration / 1e6 << " ms" << endl;
        cout << "  Per insert: " << per_insert << " ns" << endl;
        cout << "  Throughput: " << fixed << setprecision(2) << throughput / 1e6 << " M ops/sec" << endl;
        cout << endl;
    }

    // Benchmark 2: Batch insertions
    void bench_batch_insertions(int batch_size) {
        uniform_real_distribution<double> dist(-10.0, 10.0);

        // Generate points
        Pointcloud cloud;
        for (int i = 0; i < batch_size; i++) {
            cloud.push_back(point3d(dist(rng), dist(rng), dist(rng)));
        }

        auto start = high_resolution_clock::now();
        point3d sensor_origin(0, 0, 0);
        tree.insertPointCloud(cloud, sensor_origin);
        auto end = high_resolution_clock::now();

        auto duration = duration_cast<nanoseconds>(end - start).count();
        double per_point = (double)duration / batch_size;
        double throughput = 1e9 / per_point;

        cout << "Batch Insertion (" << batch_size << " points):" << endl;
        cout << "  Total time: " << duration / 1e6 << " ms" << endl;
        cout << "  Per point: " << per_point << " ns" << endl;
        cout << "  Throughput: " << fixed << setprecision(2) << throughput / 1e6 << " M points/sec" << endl;
        cout << endl;
    }

    // Benchmark 3: Ray insertion (simulating depth camera)
    void bench_ray_insertions(int ray_count) {
        uniform_real_distribution<double> angle(-M_PI, M_PI);
        uniform_real_distribution<double> distance(0.5, 10.0);
        point3d sensor_origin(0, 0, 0);

        auto start = high_resolution_clock::now();
        for (int i = 0; i < ray_count; i++) {
            double theta = angle(rng);
            double phi = angle(rng);
            double r = distance(rng);

            point3d end(
                r * sin(phi) * cos(theta),
                r * sin(phi) * sin(theta),
                r * cos(phi)
            );

            tree.insertRay(sensor_origin, end);
        }
        auto end = high_resolution_clock::now();

        auto duration = duration_cast<nanoseconds>(end - start).count();
        double per_ray = (double)duration / ray_count;
        double throughput = 1e9 / per_ray;

        cout << "Ray Insertion (" << ray_count << " rays):" << endl;
        cout << "  Total time: " << duration / 1e6 << " ms" << endl;
        cout << "  Per ray: " << per_ray << " ns" << endl;
        cout << "  Throughput: " << fixed << setprecision(2) << throughput / 1e6 << " M rays/sec" << endl;
        cout << endl;
    }

    // Benchmark 4: Point queries
    void bench_queries(int query_count) {
        // First, insert some points
        uniform_real_distribution<double> dist(-10.0, 10.0);
        for (int i = 0; i < 10000; i++) {
            point3d p(dist(rng), dist(rng), dist(rng));
            tree.updateNode(p, i % 2 == 0); // 50% occupied
        }

        // Now query
        auto start = high_resolution_clock::now();
        int occupied_count = 0;
        for (int i = 0; i < query_count; i++) {
            point3d p(dist(rng), dist(rng), dist(rng));
            OcTreeNode* node = tree.search(p);
            if (node && tree.isNodeOccupied(node)) {
                occupied_count++;
            }
        }
        auto end = high_resolution_clock::now();

        auto duration = duration_cast<nanoseconds>(end - start).count();
        double per_query = (double)duration / query_count;
        double throughput = 1e9 / per_query;

        cout << "Point Queries (" << query_count << "):" << endl;
        cout << "  Total time: " << duration / 1e6 << " ms" << endl;
        cout << "  Per query: " << per_query << " ns" << endl;
        cout << "  Throughput: " << fixed << setprecision(2) << throughput / 1e6 << " M queries/sec" << endl;
        cout << "  Occupied: " << occupied_count << " / " << query_count << endl;
        cout << endl;
    }

    // Get memory usage
    void report_memory() {
        size_t node_count = tree.size();
        size_t memory_usage = tree.memoryUsage();
        double bytes_per_node = (double)memory_usage / node_count;

        cout << "Memory Usage:" << endl;
        cout << "  Nodes: " << node_count << endl;
        cout << "  Total memory: " << memory_usage / 1024.0 / 1024.0 << " MB" << endl;
        cout << "  Bytes per node: " << bytes_per_node << endl;
        cout << endl;
    }
};

int main() {
    cout << "====================================" << endl;
    cout << "OctoMap Competitive Benchmark" << endl;
    cout << "====================================" << endl;
    cout << endl;

    // Use 5cm resolution to match OctaIndex3D benchmarks
    BenchmarkRunner bench(0.05);

    cout << "Configuration:" << endl;
    cout << "  Resolution: 0.05m (5cm)" << endl;
    cout << "  Seed: 12345" << endl;
    cout << endl;

    // Run benchmarks
    bench.bench_single_insertions(1000);
    bench.bench_single_insertions(10000);

    bench.bench_batch_insertions(100);
    bench.bench_batch_insertions(1000);
    bench.bench_batch_insertions(10000);

    bench.bench_ray_insertions(100);
    bench.bench_ray_insertions(1000);

    bench.bench_queries(1000);
    bench.bench_queries(10000);

    bench.report_memory();

    return 0;
}
