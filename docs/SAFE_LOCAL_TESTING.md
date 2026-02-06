# Safer Local Testing

Use this workflow when validating dependency updates or untrusted branches on your local machine.

## Quick start

```bash
./scripts/safe_local_test.sh
```

Default behavior:

- Runs `cargo test --locked --offline`.
- Skips long-running `layers::esdf::tests::test_esdf_from_tsdf`.
- Runs `cargo deny check advisories` when `cargo-deny` is installed.
- Unsets common credential environment variables before test execution.
- Warns when `Cargo.toml` or `Cargo.lock` changed locally.

## Options

```bash
# Include the long-running ESDF test
./scripts/safe_local_test.sh --include-slow

# Allow network access for missing crates/index updates
./scripts/safe_local_test.sh --allow-network

# Skip advisory checks
./scripts/safe_local_test.sh --skip-advisories
```

## Notes

- This reduces risk; it does not fully sandbox Rust builds/tests.
- `cargo test` still executes build scripts, proc macros, and test code with user-level permissions.
- For unknown code, prefer a VM/container in addition to this script.
