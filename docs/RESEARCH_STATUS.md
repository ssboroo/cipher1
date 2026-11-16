# CIPHER-1 NEXUS Research Status

Last updated: 2026-11-14

## Executive status

CIPHER-1 NEXUS is an experimental research workspace. It is not a production
security product, a published standard, or a demonstrated world-first
cryptographic algorithm.

## Implemented components

### C1NX framing

- Versioned binary frame with a 48-byte header.
- Unkeyed BLAKE3 digest for accidental-corruption detection.
- Bounded decoding and malformed-frame tests.
- The digest does not provide confidentiality or adversarial authentication.

### C1AE authenticated envelope

- Version-1 frame using the standardized XChaCha20-Poly1305 AEAD.
- 256-bit caller-managed key; helper generates key material using the OS CSPRNG.
- Fresh 192-bit random nonce per seal operation.
- Authenticated header and caller-supplied associated data.
- Configurable plaintext limit checked before decryption.
- Generated keys and decrypted compressed buffers use zeroizing wrappers.
- No key exchange, KDF, key rotation, sender identity, replay prevention, or
  independent audit.

### NARF reversible preprocessing

- Version 3 adaptive second-order residual transform over Z/256Z.
- Searches 64 coefficient pairs and uses deterministic tie-breaking.
- Extended unsigned-LEB128 repeat tokens for long runs.
- Raw fallback bounds format expansion to at most the 17-byte frame header.
- Version-3 decoder retains version-1 and version-2 read compatibility.
- Fixed-coefficient transform bijectivity is established by a lower-triangular
  matrix argument with unit diagonal.
- The recurrence family and token coding are engineering hypotheses, not a
  demonstrated novel compression method or a cryptographic primitive.

## Recent decoder hardening

- NARF decoding now uses fallible reservations for raw output, residual buffers, and reconstructed output. Capacity failures map to `AllocationFailed` rather than invoking infallible `Vec::with_capacity` on untrusted declared lengths.
- A regression test exercises an extreme declared output length with the caller limit explicitly raised to the platform maximum. This covers capacity-overflow handling; it does not replace process-level memory limits or fuzzing.

## Validation and automation

- Rust CI matrix: Ubuntu, Windows, and macOS.
- Declared runtime library MSRV: Rust 1.85.
- Cross-target compile checks: WebAssembly, Android ARM64, iOS ARM64, and
  Windows ARM64.
- CI checks known-compromised Rust crate versions in the resolved dependency
  graph.
- NARF benchmark compares identity, LZ4, Zstandard, and Brotli on five
  deterministic synthetic workloads. Raw CSV is archived by CI.
- Daily fuzz workflow exercises NARF and C1AE decoders with bounded outputs.
- Hourly autonomous R&D workflow also runs after source/documentation changes
  on main; it proposes one bounded patch on an isolated branch and is designed
  to open a review pull request. Automated changes are
  not merged without review.

## Evidence still required

- Published benchmark measurements from representative public corpora and
  multiple independent runs.
- Memory, streaming, energy, and cross-platform runtime measurements.
- Independent implementation and wire-format conformance vectors.
- External cryptographic review of C1AE integration and key-management design.
- Replay-protection protocol and sender-identity model for network use.
- Prior-art and patent review before any novelty claim.
- Independent cryptanalysis before any custom cryptographic primitive could be
  considered.

## Next engineering priorities

1. The exact token-size estimator is implemented and validated; the first
   post-optimization CI run showed a directional 26–29% encode-time reduction
   on four synthetic fixtures. Repeat the experiment on multiple runs and
   representative public corpora before accepting the improvement.
2. Prototype block-adaptive coefficient selection only if representative
   workloads justify its metadata and CPU cost.
3. Add stable cross-language conformance vectors for C1NX, C1AE, and NARF.
4. Keep C1AE based on reviewed primitives; focus research on protocol,
   interoperability, key lifecycle, and safe composition.
5. Preserve strict CI, fuzzing, dependency review, and human review gates.
