# CIPHER-1 NEXUS Research Status

Last updated: 2026-11-23

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
- Version-3 decoding validates the canonical left-to-right token segmentation
  without allocating a second token buffer.
- Token canonicality is relative to the frame's coefficient pair; the decoder
  does not independently verify global coefficient optimality.
- Raw fallback bounds format expansion to at most the 17-byte frame header.
- Version-3 decoder retains version-1 and version-2 read compatibility.
- Fixed-coefficient transform bijectivity is established by a lower-triangular
  matrix argument with unit diagonal.
- Decoder token buffers reserve the final output capacity and reconstruct
  residuals in place, avoiding a second output-sized allocation. Reservations
  remain fallible and failures map to `AllocationFailed`.
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
- The public-corpus runner verifies SHA-256 manifests and records complete
  encoded sizes, per-file results, aggregate ratios, timing samples, and
  experiment metadata. No public corpus has yet been admitted or measured.
- See [BENCHMARK_METHODOLOGY.md](BENCHMARK_METHODOLOGY.md) and
  [PUBLIC_CORPUS_BENCHMARK_RUNNER.md](PUBLIC_CORPUS_BENCHMARK_RUNNER.md) for
  framing rules, reproducibility details, and limitations.

## Recent merged work

- PR #1 merged as commit `bfdc377a0218d22912e02917f44adbeeca722d5b`.
  It adds fallible decoder capacity reservations, a defined allocation error,
  and an extreme-capacity regression test. Rust CI and decoder fuzz workflows
  passed for the PR head.
- PR #2 merged as commit `8a6312fc60806d3fdcf162e5b98966d8f7d457ef`.
  It adds complete-frame benchmark accounting, framed pair-RLE, per-operation
  timing samples, percentile reporting, and runner metadata. Its Rust CI and
  decoder fuzz workflows passed.
- PR #7 merged as commit `3e30e2cc6caa5a5ca80d23d7d04e9350624e2b69`.
  It adds the checksum-verified public-corpus benchmark runner, deterministic
  manifest generation, cross-platform Python tests, and documented usage.

## Current review findings

- NARF v3 decoding requires the canonical left-to-right token segmentation for
  the supplied alpha/beta coefficients. This rejects alternate literal/repeat
  segmentations while preserving the legacy version-1 and version-2 token
  acceptance rules.
- Canonical token validation does not prove that the supplied coefficient pair
  is the globally optimal selector result, nor does it enforce whole-frame
  canonicality for raw fallback.
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

1. Admit legally usable, versioned public corpora and publish reproducible
   benchmark results against identity, pair-RLE, LZ4, Zstandard, and Brotli.
2. Add stable cross-language conformance vectors for C1NX, C1AE, and NARF.
3. Measure peak memory, streaming behavior, energy, and platform runtime.
4. Keep C1AE based on reviewed primitives; focus research on protocol,
   interoperability, key lifecycle, and safe composition.
5. Preserve strict CI, fuzzing, dependency review, and human review gates.

## Release gates

No production release until the interoperable specification is frozen,
independent implementations pass conformance tests, benchmark results are
reproducible, platform CI is broad, external security review supports any
security claims, licensing/governance/disclosure policies exist, and
operational documentation is complete. Meeting only some gates is not
completion.
