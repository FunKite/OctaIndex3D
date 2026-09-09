# Release checklist

crates.io versions are immutable. Prepare changes on a short-lived branch, merge through a reviewed PR with passing checks, and publish from the merged commit. Never tag before successful publication.

1. Confirm the proposed version is absent from the crates.io index and GitHub tags/releases. Review open PRs, dependency changes, and security alerts.
2. Update `Cargo.toml`, the root package in `Cargo.lock`, the version assertion in `src/lib.rs`, README highlights and versioned links, the documentation index, and book release guidance. Move changelog entries into a dated release section and update comparison links.
3. Run `./scripts/safe_local_test.sh` (use `--allow-network` if the locked dependencies are not cached), formatting, all-feature checking, Clippy, and feature-matrix CI. Review any skipped hardware/slow tests explicitly.
4. Run `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --locked --all-features --no-deps` and the equivalent no-default-feature check. Run doctests, including the README examples. CI enforces the all-feature documentation gate.
5. With an existing nightly toolchain, measure item coverage using `cargo +nightly rustdoc --locked --all-features --lib -- -Z unstable-options --show-coverage`. Require 100% documented items; this metric does not mean every item has a runnable example. docs.rs builds the published package with all features, on nightly, including platform-specific target variants.
6. Inspect `cargo package --locked --list`: source, build script, README, changelog, and license must be present; credentials, local settings, and internal artifacts must be absent. Repository links in the README must be absolute and release-versioned because registry rendering has a different base URL.
7. After merging and synchronizing `main`, run `cargo publish --locked --dry-run` without `--allow-dirty` or `--no-verify`. Inspect and test documentation from the extracted `target/package/octaindex3d-VERSION` package, not just the checkout. Record the commit and package checksum.
8. Run `cargo publish --locked` from that same commit. If upload status is ambiguous, check the registry before retrying. Verify the version and checksum in the crates.io index.
9. Create an annotated `vVERSION` tag at the published commit and push it. The release workflow creates the GitHub Release; update its notes with the version's changelog and validation results.
10. Verify the crates.io version, GitHub tag/release, and successful docs.rs build with 100% documented-item coverage. A queued docs.rs build is pending verification, not a confirmed success.

No new tools are required for the stable documentation gate. The release process must preserve the locked dependency set and must not bypass protected-branch checks.
