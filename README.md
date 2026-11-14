# CIPHER-1 NEXUS

CIPHER-1 NEXUS is an experimental research project exploring a new, modular data transformation and transport architecture. The goal is to investigate whether adaptive reversible mathematics, efficient coding, robust framing, optional error recovery, and cryptographic interoperability can form a useful cross-platform technology.

## Research principles

- Develop and test original mathematical hypotheses rather than relabel known algorithms.
- Publish precise specifications, proofs, test vectors, and reproducible benchmarks.
- Separate reversible encoding, compression, error correction, integrity, authentication, and encryption.
- Never claim cryptographic security, novelty, or performance superiority without evidence.
- Preserve interoperability and use established cryptographic standards for any security-sensitive deployment until a new primitive has survived independent review.

## Current experimental components

- `crates/cipher1-core`: versioned binary framing and unkeyed BLAKE3 integrity check.
- `crates/nexus-transform`: NEXUS Adaptive Residual Transform (NARF) prototype. It tests 64 second-order modular recurrence candidates and applies a run/literal coder. Version 3 uses extended LEB128 run tokens and a raw fallback; the decoder retains version-1 and version-2 read compatibility.
- `docs/NEXUS_MATHEMATICAL_RESEARCH.md`: mathematical hypothesis, algebraic definition, and falsification criteria.
- `docs/NEXUS_ALGORITHM_SPEC.md`: experimental wire format and decoder requirements.
- `docs/NEXUS_VALIDATION_PLAN.md`: correctness, performance, platform, and security validation plan.
- `docs/THREAT_MODEL.md`: current security boundaries.

## Important security notice

NARF is not encryption and has no secret key. The existing frame digest is unkeyed and does not authenticate a sender. Neither component protects sensitive production data. Do not use this repository as a security product.

## Build and test

Requirements: Rust stable toolchain.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Target platforms

The intended validation matrix includes Linux, Windows, macOS, Android, iOS, and WebAssembly. Support is not claimed until builds and conformance tests pass on each target.

## Project maturity

Research prototype. Experimental formats may change. No production release, formal standard, security certification, or novelty claim is currently made.
