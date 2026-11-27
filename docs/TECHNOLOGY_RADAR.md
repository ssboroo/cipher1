# CIPHER-1 Technology Radar

**Assessment date:** 2026-11-27  
**Project direction:** open-source intent, all-user product, performance-first data engine  
**Status:** research and evaluation guidance; not an adoption or compatibility guarantee

This document records technology candidates and decision rules. A technology is not selected merely because it is new. Adoption requires a reproducible benchmark, correctness tests, platform validation, maintenance review, and license compatibility.

## Current verified baseline and candidates

| Area | Candidate / current observation | CIPHER-1 decision |
|---|---|---|
| Systems language | Rust 1.100.0 was announced as stable on 2026-11-12. | Keep the engine in stable Rust. Evaluate the newest stable compiler in CI, but preserve the declared library MSRV until an explicit compatibility review and release decision. |
| Compression | The upstream Zstandard project documents stable framing and published reference benchmarks; a Zstd 1.6.0 tag/release has been reported in late-November 2026 coverage. | Verify the exact upstream tag, release notes, source and crate bindings before pinning. Benchmark against LZ4, Zstd, Brotli and identity using complete frame overhead. |
| SIMD | Google Highway advertises portable, length-agnostic SIMD and runtime dispatch across x86, Arm, RISC-V and WebAssembly targets. It is a C++ library. | Evaluate architecture-specific stable Rust intrinsics and maintained Rust SIMD abstractions first. Treat Highway as an FFI candidate; include call overhead, build complexity and license review. Always retain a scalar fallback. |
| CPU targets | AVX2/AVX-512, ARM NEON/SVE2, RISC-V Vector and WebAssembly SIMD are relevant optimization targets. | Runtime feature detection, safe dispatch and per-architecture benchmarks. Never compile the baseline binary with an instruction set that excludes supported CPUs. |
| Web | WebAssembly can provide a browser demo without requiring a separate algorithm implementation. | Consider WASM only after a stable, bounded engine API exists; test memory limits, streaming behavior and browser compatibility. |
| Desktop | A maintained Rust-compatible desktop shell such as Tauri is a candidate. | Build a desktop shell only after the CLI/API and file-safety semantics are stable. Reuse the same core; do not fork codec logic. |
| Validation | cargo-fuzz/libFuzzer, Miri, sanitizers, property tests and cross-platform CI. | Use each where applicable; fuzz untrusted decoders and keep deterministic regression corpora. |
| Supply chain | cargo-audit/cargo-deny, SBOM generation, pinned release inputs and signed artifacts. | Add incrementally with documented policy; avoid claiming reproducible or signed releases until independently verified. |

## Decision rules

1. **Correctness first:** every optimized path must match the scalar/reference implementation byte-for-byte and pass shared conformance vectors.
2. **Measure end-to-end:** include framing, planner overhead, allocations, memory peaks, encode/decode latency and throughput.
3. **Representative workloads:** public corpora with provenance plus text, JSON, source code, media, archives, random/encrypted-like data, small files and streaming workloads.
4. **Portable by default:** optimized CPU paths are optional accelerators; scalar fallback remains correct and supported.
5. **No novelty tax:** do not replace a mature dependency or stable API without a measurable benefit and migration plan.
6. **Open-source compatibility:** review exact SPDX license, transitive dependencies, notices and patent terms before adoption. The repository currently declares `UNLICENSED`; open-source intent is not an effective license.
7. **Security separation:** compression and reversible transforms are not encryption. Use established, reviewed cryptographic implementations for security-sensitive functions.
8. **Evidence labels:** distinguish upstream claims, local measurements, CI smoke tests and independent third-party results.

## Immediate engineering sequence

1. Resolve current PR review and NARF v3 compatibility questions before changing format behavior.
2. Resolve the owner's exact open-source license choice; do not infer a license from the word "open".
3. Implement a safe CLI on the existing stable APIs, with no-overwrite defaults, bounded decode output, temporary files and atomic replacement where supported.
4. Establish the first public-corpus baseline for identity, LZ4, Zstandard, Brotli and NARF, including total frame size and raw results.
5. Profile the 64-candidate NARF selector. Test pruning, early exit and block adaptation as hypotheses; preserve a reference path and reject changes that reduce correctness or worsen end-to-end performance.
6. Add runtime SIMD dispatch only after profiling identifies a stable hotspot and cross-architecture tests are available.
7. Build the consumer desktop and browser/WASM experiences on the same stable engine API.

## Sources checked

- Rust 1.100.0 stable announcement (2026-11-12): https://blog.rust-lang.org/releases/latest/
- Rust official release notes: https://doc.rust-lang.org/releases.html
- Zstandard upstream project and benchmark methodology: https://facebook.github.io/zstd/
- Zstandard upstream releases: https://github.com/facebook/zstd/releases
- Google Highway SIMD project: https://github.com/google/highway
- 2026 Rust SIMD ecosystem survey (secondary research; verify crate status and license before adoption): https://shnatsel.github.io/state-of-simd-rust-2026/
- Zstandard 1.6.0 late-November report (secondary reporting; confirm exact upstream tag before pinning): https://www.phoronix.com/news/Zstandard-Zstd-1.6

## Review cadence

Re-check official release notes and upstream project status before a dependency or architecture change and at least once per monthly roadmap review. Record the date and the exact versions checked. Do not let a moving "latest" version silently change released CIPHER-1 behavior.
