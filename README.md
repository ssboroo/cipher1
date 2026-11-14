# CIPHER-1

CIPHER-1 is an experimental, versioned data-encoding and integrity framework written in Rust.

## Research direction

The long-term research direction is a modular pipeline combining canonical framing, integrity verification, optional compression, optional error-correction coding, and authenticated encryption through established cryptographic algorithms.

## Current status

The first milestone is a binary framing and integrity prototype. It is not an encryption algorithm and does not provide confidentiality, sender authentication, replay protection, or quantum resistance. It must not be used to protect sensitive production data.

## Design principles

- Strict, versioned binary format.
- Bounded resource use when decoding untrusted input.
- Deterministic encoding.
- Modular and independently testable components.
- Established cryptographic primitives rather than unreviewed custom cryptography.
- Security and performance claims require reproducible evidence and independent review.

## Repository layout

- `crates/cipher1-core`: core frame codec.
- `docs/ARCHITECTURE.md`: format and architecture.
- `docs/THREAT_MODEL.md`: security scope and assumptions.

## Quick start

Requirements: Rust stable toolchain.

```bash
cargo test --workspace
```

## Security status

Research software only. No production-security claim is made.

## License

No license has been selected. All rights are reserved unless and until a license is added.
