# Internal Optimization Workflows

**PRIVATE** - Not for public documentation

These workflows document the optimization process for different CPU/GPU architectures.

## Files

1. **`intel_nvidia_optimization.md`** - Intel Xeon + NVIDIA GPU workflow
   - BMI2 optimization (PDEP/PEXT for Morton encoding)
   - CUDA kernels for batch operations
   - g4dn.xlarge or similar EC2 instances

2. **`amd_optimization.md`** - AMD EPYC workflow
   - Zen architecture-specific optimizations
   - Conditional BMI2 usage (Zen 1/2 slow, Zen 3+ fast)
   - Cache blocking tuned for AMD
   - c5a.xlarge or similar EC2 instances

## Workflow

### Phase 1: Intel/NVIDIA (Current)
```bash
# On EC2: g4dn.xlarge (Intel + NVIDIA T4)
cd ~/octaindex3d
cat .github/workflows/internal/intel_nvidia_optimization.md
# Follow instructions...
```

### Phase 2: AMD
```bash
# Switch to: c5a.xlarge (AMD EPYC)
cd ~/octaindex3d
cat .github/workflows/internal/amd_optimization.md
# Follow instructions...
```

### Phase 3: Compare Results
- Merge branches
- Create comparison document
- Update main optimization guide with findings

## Expected Timeline

- Intel/NVIDIA: 2-3 hours
- AMD: 2-3 hours
- Comparison/Documentation: 1 hour

## Results Location

All results saved to:
- `.github/workflows/internal/intel_nvidia_results.txt`
- `.github/workflows/internal/amd_results.txt`
- `.github/workflows/internal/amd_vs_intel.md`

## Why Hidden?

These workflows contain:
- Work-in-progress optimization strategies
- Incomplete benchmarks
- Internal decision-making process
- Cost considerations for EC2 instances

Public docs will be polished and placed in `docs/` when complete.

## Access

To find these files:
```bash
cd .github/workflows/internal
ls -la
```

Or on GitHub:
`https://github.com/YOUR_REPO/tree/main/.github/workflows/internal`

(Requires repository access)
