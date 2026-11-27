# NEXUS Validation and Experimental Plan

## Phase 0 — Correctness and safety

- Unit-test all candidate coefficients and input lengths from 0 through 512.
- Property-test random byte arrays with fixed and generated seeds.
- Fuzz the decoder with arbitrary byte streams.
- Test length overflow, truncated headers, invalid versions, truncated literals, truncated runs, excessive output declarations, and trailing data.
- Exercise allocation-capacity overflow with caller limits raised to the platform maximum; decoding must return a defined error rather than panic.
- Verify version-2/3 raw fallback for incompressible inputs and assert encoded_size <= input_size + 17 for all generated inputs.
- Verify version-3 extended runs, canonical ULEB128 parsing, overflow rejection, and version-1/version-2 compatibility.
- Verify version-1 decode compatibility and reject non-canonical raw-mode markers, seeds, and payload lengths.
- Verify the exact token-size estimator equals the emitted token-stream length for all 64 coefficient pairs and extended-run boundary lengths.
- Cross-check fixed NARF frame vectors with the Rust implementation and an in-repository Python standard-library reference decoder. This is a second-language consistency check, not an external independent implementation.
- Run Miri where supported and sanitizing/fuzzing tools on native targets.
- Require deterministic output across x86_64, AArch64, and WebAssembly.

Exit criteria: exact round-trip for all generated inputs; no panic, unbounded allocation, or undefined behavior on malformed input; version-2/3 encoded-size bound holds; legacy version-1 and version-2 fixtures decode exactly.

## Phase 1 — Baselines

Compare against:
- Identity (no compression)
- Simple RLE
- LZ4
- Zstandard
- Brotli

Use public corpora and legally distributable samples: Canterbury, Silesia, Calgary, enwik8 where licensing permits, synthetic periodic data, repeated records, JSON, CSV, source code, images, audio, and encrypted/random data.

Report total bytes including every header and metadata field. The CI pair-RLE baseline uses an explicit 8-byte frame header; LZ4 uses its size-prepended frame; Zstandard and Brotli report their complete streams. Identity is raw bytes without an added wrapper because it is a lower-bound reference when the caller already knows the input length. Record compiler, runner, kernel, and CPU metadata with each CI artifact. See [BENCHMARK_METHODOLOGY.md](BENCHMARK_METHODOLOGY.md) for the current synthetic harness and its limitations.

## Phase 2 — Performance

Measure:
- compression ratio;
- encode and decode throughput;
- p50/p95/p99 latency;
- peak resident memory;
- CPU cycles per input byte;
- energy per GiB where hardware measurement is available.

Run at least 30 repetitions after warm-up, publish raw measurements, and avoid comparing unlike hardware or compiler settings.

## Phase 3 — Mathematical characterization

Evaluate residual histograms, empirical entropy, autocorrelation, conditional entropy, and coefficient-selection stability across data classes. Use held-out datasets to prevent tuning only to the benchmark corpus.

## Phase 4 — Platform matrix

Targets:
- Linux x86_64 and AArch64
- Windows x86_64 and ARM64
- macOS Intel and Apple Silicon
- Android ARM64
- iOS ARM64
- WebAssembly/WASI

Require byte-for-byte conformance vectors on every target. Run the Rust vector integration test and Python reference decoder in the supported CI matrix. These in-repository checks do not replace an independently maintained third-party implementation.

## Phase 5 — Security research

NARF is not cryptography. Test parser robustness and side-channel behavior as engineering concerns, but do not describe these as cryptographic security. Evaluate future encryption separately against established AEAD and post-quantum standards.

## Phase 6 — Release gates

No production release until:
- a stable format version is frozen;
- independent implementations pass conformance tests;
- fuzzing and security review are published;
- benchmark claims are independently reproducible;
- all known limitations are documented;
- a license, governance process, and vulnerability disclosure policy exist.
