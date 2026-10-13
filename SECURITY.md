# Security Policy

## Project Status

**OctaIndex3D is no longer maintained.** Version 0.5.9 was the final release, and the repository is archived (read-only). No further releases or security patches will be published, and vulnerability reports are no longer monitored.

## Supported Versions

| Version | Supported |
| ------- | --------- |
| All     | :x:       |

No version receives security updates, including 0.5.9.

## Guidance for Users

- **Audit your dependency tree**: Run `cargo audit` or `cargo deny check advisories` regularly. OctaIndex3D's own dependencies will not be updated, so advisories against them must be handled in your project (for example, with `cargo update` for semver-compatible fixes, or by vendoring or forking).
- **Validate input**: When using OctaIndex3D with untrusted input, validate coordinates, indices, and container data before processing.
- **Fork if you need fixes**: The code is MIT-licensed. If you find a vulnerability and need a fix, fork the project and publish it under a new crate name.

## Security-Related Configuration

### Cargo Features

Some features may have different security implications:

- `parallel`: Uses Rayon for parallelism (vetted dependency)
- `simd`: Platform-specific optimizations (uses `unsafe` blocks, thoroughly tested)
- `serde`: Serialization support (consider input validation when deserializing)

### Unsafe Code

OctaIndex3D uses `unsafe` blocks for SIMD optimizations. All unsafe code:
- Is thoroughly tested with comprehensive test coverage
- Has detailed comments explaining safety invariants
- Is isolated to performance-critical sections
- Has safe fallback implementations
