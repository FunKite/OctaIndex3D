## Description

<!-- Provide a clear and concise description of your changes -->

## Related Issue

<!-- Link to the issue this PR addresses (if applicable) -->
Closes #

## Type of Change

<!-- Mark the relevant option with an [x] -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Code cleanup / refactoring
- [ ] CI/CD improvement
- [ ] Other (please describe):

## Changes Made

<!-- Detailed list of changes -->

-
-
-

## Motivation and Context

<!-- Why is this change required? What problem does it solve? -->

## Testing

### Test Coverage

- [ ] All existing tests pass
- [ ] New tests added for new functionality
- [ ] Manual testing performed
- [ ] Edge cases considered and tested

### Test Commands Run

```bash
# List the commands you ran to test your changes
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt -- --check
```

<details>
<summary>Test Output</summary>

```
<!-- Paste relevant test output here -->
```

</details>

## Performance Impact

<!-- Required for performance-related changes -->

- [ ] No performance impact
- [ ] Performance improvement (include benchmarks below)
- [ ] Potential performance regression (explain why acceptable)
- [ ] Not applicable

<details>
<summary>Benchmark Results</summary>

**Platform:** <!-- e.g., Apple M1 Max, Intel Xeon, AMD EPYC -->
**OS:** <!-- e.g., macOS 14.0, Ubuntu 22.04 -->
**Rust Version:** <!-- e.g., 1.82.0 -->

**Before:**
```
<!-- Paste baseline benchmark results -->
```

**After:**
```
<!-- Paste optimized benchmark results -->
```

**Summary:**
- Speedup: X% improvement in Y operation
- Memory: Z MB reduction/increase

</details>

## Breaking Changes

<!-- If this introduces breaking changes, describe them and the migration path -->

- [ ] This PR introduces breaking changes

<details>
<summary>Breaking Change Details</summary>

**What breaks:**
-

**Migration guide:**
```rust
// Old way
let old = ...;

// New way
let new = ...;
```

</details>

## Documentation

- [ ] Code comments added/updated
- [ ] API documentation added/updated (doc comments)
- [ ] README.md updated (if needed)
- [ ] WHITEPAPER.md updated (if needed)
- [ ] Examples added/updated (if needed)
- [ ] Inline code examples in docs tested

## Code Quality

- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] No unnecessary dependencies added
- [ ] Error handling is appropriate
- [ ] Edge cases handled

## Checklist

<!-- Mark items as complete with [x] -->

- [ ] My code follows the [contribution guidelines](../CONTRIBUTING.md)
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new compiler warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] Any dependent changes have been merged and published

## Platform Testing

<!-- Check all platforms where you've tested this change -->

- [ ] Linux x86_64
- [ ] macOS x86_64 (Intel)
- [ ] macOS ARM64 (Apple Silicon)
- [ ] Windows x86_64
- [ ] Not applicable / tested on CI

## Feature Flags

<!-- If your changes affect specific features -->

Tested with the following feature combinations:

- [ ] Default features only
- [ ] `--all-features`
- [ ] `--features parallel`
- [ ] `--features simd`
- [ ] `--features hilbert`
- [ ] `--features container_v2`
- [ ] `--features gis_geojson`
- [ ] Other combination (specify):

## Additional Notes

<!-- Any additional information reviewers should know -->

## Screenshots / Output

<!-- If applicable, add screenshots or terminal output showing the changes -->

```
<!-- Paste relevant output here -->
```

## Reviewer Notes

<!-- Specific areas where you'd like reviewer attention -->

Please pay special attention to:
-
-

## Post-Merge Tasks

<!-- List any tasks that need to happen after merging (if applicable) -->

- [ ] Update crates.io (if version bump)
- [ ] Update CLAUDE.md development log
- [ ] Create release notes
- [ ] Update dependent examples
- [ ] Other:
