# Appendix G: Performance Tuning Cookbook

This appendix is a **decision guide** for making OctaIndex3D fast on your hardware. Instead of re‑deriving theory, it focuses on “if you see X, try Y” style recipes.

It is meant to complement Chapter 7 (Performance Optimization) and Appendix C (Benchmarks).

---

## G.1 Choosing CPU Features

Start with a simple baseline:

- Build without CPU‑specific features, confirm correctness and basic performance.
- Enable `bmi2` and `avx2` (on x86‑64) or `neon` (on ARM) once tests pass.

When in doubt:

- Prefer feature flags that match your deployment fleet.
- Use `RUSTFLAGS` or `target-cpu` only after you understand the impact on portability.

---

## G.2 Memory vs. Speed Trade‑offs

Common levers include:

- **Chunk size**: Larger chunks improve sequential access but may increase latency for small queries.
- **Caching policies**: Keep hot regions at high LOD, demote cold regions to coarser levels.
- **Container choice**: Use streaming containers for long‑running ingestion, simpler formats for offline analysis.

Future revisions of this appendix will include concrete parameter tables derived from Appendix C’s benchmark data.

---

## G.3 Profiling Checklist

Before tuning:

1. Enable debug logging only where needed.
2. Run representative workloads (not microbenchmarks alone).
3. Capture flamegraphs or profile traces on your target hardware.

After each change, re‑run the same workload and record:

- Runtime
- Peak memory usage
- Cache misses (if measured)

This iterative loop is the core of performance‑driven development described in Chapter 7.

