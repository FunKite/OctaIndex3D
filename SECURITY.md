# Security Policy

## Supported Versions

We release patches for security vulnerabilities for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.4.x   | :white_check_mark: |
| < 0.4   | :x:                |

## Reporting a Vulnerability

We take the security of OctaIndex3D seriously. If you discover a security vulnerability, please follow these steps:

### Where to Report

**Please DO NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

1. **GitHub Security Advisories** (Preferred)
   - Go to https://github.com/FunKite/OctaIndex3D/security/advisories
   - Click "Report a vulnerability"
   - Fill out the advisory form with details

2. **Email**
   - Send an email to: FunKite@users.noreply.github.com
   - Include "SECURITY" in the subject line
   - Provide detailed information about the vulnerability

### What to Include

Please include the following information in your report:

- Type of vulnerability (e.g., buffer overflow, injection, etc.)
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability, including how an attacker might exploit it

### What to Expect

- **Initial Response**: You'll receive acknowledgment of your report as soon as possible
- **Status Updates**: We'll keep you informed of our progress
- **Validation**: We'll work to validate the vulnerability and determine its impact
- **Fix Development**: Once confirmed, we'll develop and test a fix
- **Disclosure**: After the fix is released, we'll publish a security advisory
- **Credit**: You'll be credited in the advisory (unless you prefer to remain anonymous)

### Security Update Process

1. Security patch is developed privately
2. New version is released with security fix
3. Security advisory is published
4. Affected users are notified via GitHub

## Security Best Practices for Users

When using OctaIndex3D:

1. **Keep Updated**: Always use the latest version to benefit from security patches
2. **Review Dependencies**: Regularly run `cargo audit` to check for vulnerable dependencies
3. **Validate Input**: When using OctaIndex3D with user input, validate coordinates and indices
4. **Monitor Advisories**: Watch the repository for security advisories

## Automated Security Measures

This repository employs several automated security measures:

- **Dependabot**: Automatic dependency updates for Cargo and GitHub Actions
- **Cargo Audit**: CI pipeline checks for known vulnerabilities in dependencies
- **Cargo Deny**: Enhanced security scanning for licenses, advisories, and supply chain issues
- **Clippy**: Linting to catch potential security issues during development

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

## Contact

For general security questions or concerns that don't constitute a vulnerability, please open a regular GitHub issue with the "security" label.

---

**Thank you for helping keep OctaIndex3D and its users safe!**
