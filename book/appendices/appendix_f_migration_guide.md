# Appendix F: Migration Guide

This appendix helps you migrate existing systems built on **cubic grids** or **classical octrees** to OctaIndex3D’s BCC‑based data structures.

The goal is to give you a practical, low‑risk path from “working but suboptimal” to “BCC‑aware and production ready” without forcing a rewrite of everything at once.

---

## F.1 When a Migration Is Worth It

Use this section to decide whether BCC is a good fit for your workload:

- You are memory‑bound and want the 29% sampling efficiency of BCC.
- Your application is sensitive to directional bias (robots, CFD, wave propagation).
- You already maintain multi‑resolution data and want cleaner level‑of‑detail transitions.

For workloads that are small, short‑lived, or rarely queried, a full migration may not pay for itself. Part IV (Applications) provides more detailed guidance by domain.

---

## F.2 Mapping Cubic Grids to BCC Coordinates

At a high level, migration involves:

1. Choosing a target **level of detail (LOD)** that matches your current resolution.
2. Defining a mapping from your existing `(i, j, k)` grid indices into `BccCoord` values.
3. Gradually replacing internal uses of raw indices with `Index64` or `Galactic128` identifiers.

Future revisions of this appendix will include concrete code snippets and step‑by‑step recipes for:

- Sampling a cubic grid onto BCC cells
- Preserving physical units and boundary conditions
- Validating that error tolerances are maintained or improved

---

## F.3 Octree to BCC‑Octree Migration

If you already use an octree:

- Treat each existing leaf cube as a **source of truth**.
- Build a BCC‑octree at comparable resolution.
- Define a mapping policy (e.g., nearest neighbor, average, or conservative bounds) from cubic leaves to BCC cells.

Migration strategies and performance considerations will be expanded as Part III (Implementation) and Part IV (Applications) mature.

