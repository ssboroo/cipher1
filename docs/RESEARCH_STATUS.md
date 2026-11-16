# CIPHER-1 NEXUS Research Status

Last updated: 2026-11-16

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

- Version-1 frame using XChaCha20-Poly1305.
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
- Decoder-owned output and residual buffers use fallible capacity reservations;
  allocation reservation failures map to `AllocationFailed`.
- The caller-controlled output limit remains the primary resource policy.
- The recurrence family and token coding are engineering hypotheses, not a
  demonstrated novel compression method or a cryptographic primitive.

## Validation and automation

- Rust CI workspace tests run on Ubuntu, Windows, and macOS.
- Declared runtime library MSRV: Rust 1.85.
- Linux CI additionally runs Clippy with `-D warnings`, dependency supply-chain
  checks, cross-target compilation for WebAssembly, Android ARM64, iOS ARM64,
  and Windows ARM64, plus the benchmark harness.
- Decoder fuzz workflow exercises NARF and C1AE decoders with bounded outputs.
- The benchmark compares identity, framed pair-RLE, NARF v3, LZ4, Zstandard,
  and Brotli on five deterministic synthetic 16 KiB workloads. It reports full
  encoded sizes and 30 individually timed encode/decode samples after five
  warm-ups, with nearest-rank p50/p95/p99.
- Benchmark metadata records Rust/Cargo versions and Linux runner, kernel, and
  CPU information when available.
- See [BENCHMARK_METHODOLOGY.md](BENCHMARK_METHODOLOGY.md) for framing rules,
  reproducibility details, and limitations.

## Recent merged work

- PR #1 merged as commit `bfdc377a0218d22912e02917f44adbeeca722d5b`.
  It adds fallible decoder capacity reservations, a defined allocation error,
  and an extreme-capacity regression test. Rust CI and decoder fuzz workflows
  passed for the PR head.
- PR #2 merged as commit `8a6312fc60806d3fdcf162e5b98966d8f7d457ef`.
  It adds complete-frame benchmark accounting, framed pair-RLE, per-operation
  timing samples, percentile reporting, and runner metadata. Its Rust CI and
  decoder fuzz workflows passed.

## Current review findings

- NARF v3 encoding is deterministic, but the decoder accepts more than one
  valid token segmentation for some residual sequences. Canonical
  unsigned-LEB128 lengths are enforced; whole-stream token canonicality is
  not currently a decoder requirement. Any strict-canonical mode requires an
  explicit compatibility decision and conformance tests.
- Fallible reservation does not guarantee process liveness under OS memory
  overcommit or physical-memory exhaustion.
- The benchmark-only pair-RLE decoder is for trusted harness data, not an
  exposed untrusted-input API.

## Evidence still required

- Published benchmark measurements from representative public corpora and
  multiple independent runs.
- Memory, streaming, energy, and cross-platform runtime measurements.
- Independent implementation and wire-format conformance vectors.
- External cryptographic review of C1AE integration and key-management design.
- Replay-protection protocol and sender-identity model for network use.
- Prior-art and patent review before any novelty claim.
- Independent cryptanalysis before any custom cryptographic primitive could
  be considered.

## Next engineering priorities

1. Establish NARF token canonicality with explicit accepted/rejected vectors and
   a backward-compatibility decision.
2. Expand the benchmark to versioned public corpora with recorded licenses,
   checksums, and preprocessing; repeat measurements on comparable hardware.
3. Add stable cross-language conformance vectors for C1NX, C1AE, and NARF.
4. Measure peak memory, streaming behavior, and platform runtime behavior.
5. Keep C1AE based on reviewed primitives; focus research on protocol,
   interoperability, key lifecycle, and safe composition.
6. Preserve strict CI, fuzzing, dependency review, and human review gates.

## Release gates

No production release until the interoperable specification is frozen,
independent implementations pass conformance tests, benchmark results are
reproducible, platform CI is broad, external security review supports any
security claims, licensing/governance/disclosure policies exist, and
operational documentation is complete. Meeting only some gates is not
completion.
