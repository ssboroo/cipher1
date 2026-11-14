# CIPHER-1 NEXUS

CIPHER-1 NEXUS is an experimental research project exploring a new, modular data transformation and transport architecture. The goal is to investigate whether adaptive reversible mathematics, efficient coding, robust framing, optional error recovery, and cryptographic interoperability can form a useful cross-platform technology.

## Research principles

- Develop and test original mathematical hypotheses rather than relabel known algorithms.
- Publish precise specifications, proofs, test vectors, and reproducible benchmarks.
- Separate reversible encoding, compression, error correction, integrity, authentication, and encryption.
- Never claim cryptographic security, novelty, or performance superiority without evidence.
- Preserve interoperability and use established cryptographic standards for any security-sensitive deployment until a new primitive has survived independent review.

## Current experimental components

- `crates/cipher1-core`: versioned binary framing, unkeyed BLAKE3 integrity check, and an optional C1AE authenticated envelope using standard XChaCha20-Poly1305.
- `crates/nexus-transform`: NEXUS Adaptive Residual Transform (NARF) prototype. It tests 64 second-order modular recurrence candidates and applies a run/literal coder. Version 3 uses extended LEB128 run tokens and a raw fallback; the decoder retains version-1 and version-2 read compatibility.
- `docs/NEXUS_MATHEMATICAL_RESEARCH.md`: mathematical hypothesis, algebraic definition, and falsification criteria.
- `docs/NEXUS_ALGORITHM_SPEC.md`: experimental wire format and decoder requirements.
- `docs/NEXUS_VALIDATION_PLAN.md`: correctness, performance, platform, and security validation plan.
- `docs/THREAT_MODEL.md`: current security boundaries.

## Documentation

- [C1AE authenticated-encryption usage](docs/C1AE_USAGE.md)
- [Protected compression and encryption pipeline](docs/PIPELINE_USAGE.md)
- [NARF v3 algorithm specification](docs/NEXUS_ALGORITHM_SPEC.md)
- [Decoder fuzzing plan](docs/NEXUS_FUZZING.md)
- [Rust dependency supply-chain controls](docs/SUPPLY_CHAIN_SECURITY.md)
- [Current research status and evidence gates](docs/RESEARCH_STATUS.md)

## Important security notice

NARF is not encryption. The legacy C1NX digest is unkeyed and does not authenticate a sender. The optional C1AE module uses standard XChaCha20-Poly1305, but this wrapper has not been independently audited and does not provide key management, sender identity, or replay protection. Do not treat this research repository as a production security product.

## Build and test

Requirements: Rust 1.85 or newer.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Target platforms

The intended validation matrix includes Linux, Windows, macOS, Android, iOS, and WebAssembly. Support is not claimed until builds and conformance tests pass on each target.

## Project maturity

Research prototype. Experimental formats may change. No production release, formal standard, security certification, or novelty claim is currently made.
