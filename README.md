# CIPHER-1 NEXUS

CIPHER-1 NEXUS is an experimental research project exploring a modular data transformation and transport architecture. The long-term research objective is to determine whether reversible preprocessing, established compression, robust framing, optional error recovery, integrity, authentication, and encryption can be composed into a useful cross-platform technology.

No claim of novelty, cryptographic security, quantum resistance, performance superiority, standards compliance, or production readiness is made without independent evidence.

## Research principles

- Formulate falsifiable hypotheses and check prior art before making novelty claims.
- Define mathematical assumptions and publish precise specifications, proofs, and test vectors.
- Keep reversible transforms, compression, error correction, integrity, authentication, and encryption as separate layers.
- Use reviewed cryptographic standards for security-sensitive functions; do not deploy experimental primitives.
- Publish reproducible measurements, including negative results and framing overhead.
- Preserve backward compatibility deliberately and document format changes.

## Current experimental components

- `crates/cipher1-core`: versioned binary framing, an unkeyed BLAKE3 integrity check, and an optional C1AE authenticated envelope using XChaCha20-Poly1305.
- `crates/nexus-transform`: NEXUS Adaptive Residual Transform (NARF) prototype. It evaluates 64 second-order modular recurrence candidates and applies run/literal coding. Version 3 uses extended unsigned-LEB128 run tokens and raw fallback; the decoder retains version-1 and version-2 read compatibility.
- NARF decoder capacity reservations use fallible allocation and return a defined allocation error when reservation fails. Caller-supplied output limits remain essential; this is not a guarantee against OS-level memory exhaustion.
- The benchmark harness compares identity, a benchmark-only framed pair-RLE baseline, NARF v3, LZ4, Zstandard, and Brotli.

## Benchmark status

The current CI harness uses five deterministic synthetic inputs of 16 KiB each. It includes complete encoded stream sizes and separately measures encode/decode operations after warm-up, reporting nearest-rank p50/p95/p99 samples. CI records toolchain and Linux runner metadata.

This is an engineering smoke test, not a publication-quality compression study. Thirty samples make p99 equal to the maximum observed sample. Public corpora, independent repeated runs, peak memory, streaming behavior, energy, and broad runtime comparisons remain outstanding.

See [benchmark methodology](docs/BENCHMARK_METHODOLOGY.md), [validation plan](docs/NEXUS_VALIDATION_PLAN.md), and [research status](docs/RESEARCH_STATUS.md).

## Documentation

- [C1AE authenticated-encryption usage](docs/C1AE_USAGE.md)
- [Protected compression and encryption pipeline](docs/PIPELINE_USAGE.md)
- [NARF v3 algorithm specification](docs/NEXUS_ALGORITHM_SPEC.md)
- [NARF preliminary benchmark results](docs/NARF_BASELINE_RESULTS.md)
- [Benchmark methodology and limitations](docs/BENCHMARK_METHODOLOGY.md)
- [Mathematical research and falsification criteria](docs/NEXUS_MATHEMATICAL_RESEARCH.md)
- [Decoder fuzzing plan](docs/NEXUS_FUZZING.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Rust dependency supply-chain controls](docs/SUPPLY_CHAIN_SECURITY.md)
- [Validation plan](docs/NEXUS_VALIDATION_PLAN.md)
- [Current research status and release gates](docs/RESEARCH_STATUS.md)

## License

CIPHER-1 is dual-licensed under **MIT OR Apache-2.0**, at your option. See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), and [licensing notes](docs/LICENSING.md). Third-party dependencies and datasets retain their own licenses.

## Security notice

NARF is reversible preprocessing, not encryption. The legacy C1NX digest is unkeyed and does not authenticate a sender. The optional C1AE module uses XChaCha20-Poly1305, but this repository's integration has not been independently audited and does not provide key management, sender identity, or replay protection. Do not treat this research repository as a production security product.

## Build and test

Requirements: Rust 1.85 or newer.

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The benchmark can be run with:

```bash
cargo bench -p nexus-transform --bench narf_baseline
```

The benchmark currently uses deterministic synthetic fixtures and should not be used alone to make general performance claims. An opt-in, checksum-verified public-corpus runner is available in `tools/narf-corpus-bench`; see [the runner guide](docs/PUBLIC_CORPUS_BENCHMARK_RUNNER.md).

## Reproducible public-corpus workflow

The optional host benchmark compares complete encoded representations from identity, benchmark-only pair-RLE, NARF v3, LZ4, Zstandard, and Brotli. It verifies each corpus file against a SHA-256 manifest before benchmarking and records per-file plus aggregate CSV results.

Generate the manifest with the Python 3 standard-library helper:

```bash
python3 tools/narf-corpus-bench/scripts/create_manifest.py \
  --corpus-dir /data/my-corpus \
  --output /data/my-corpus.manifest.tsv
```

Then run the Rust benchmark in release mode:

```bash
cargo run --release -p narf-corpus-bench --features host-benchmark -- \
  --corpus-id my-corpus-v1 \
  --corpus-dir /data/my-corpus \
  --manifest /data/my-corpus.manifest.tsv \
  --output-csv /tmp/my-corpus-results.csv
```

See [benchmark runner methodology](docs/PUBLIC_CORPUS_BENCHMARK_RUNNER.md) and [manifest generator documentation](docs/CORPUS_MANIFEST_GENERATOR.md). The generator and runner do not establish dataset provenance or licensing permission. Review and retain each dataset's source, version, retrieval date, and license evidence. Keep corpus files immutable during a run. Synthetic CI fixtures are smoke tests, not public-corpus performance evidence.

Run the generator tests with:

```bash
python3 -m unittest discover -s tools/narf-corpus-bench/scripts -p 'test_*.py' -v
```

## Platform validation

The CI matrix runs workspace tests on Linux, Windows, and macOS. Linux CI additionally checks the declared Rust 1.85 library MSRV, Clippy, dependency supply-chain rules, and compile targets for WebAssembly, Android ARM64, iOS ARM64, and Windows ARM64. These compile checks do not constitute full runtime or conformance validation on every target.

## Project maturity and release gates

Research prototype. Experimental formats may change. No production release, formal standard, security certification, or novelty claim is currently made.

A release requires, at minimum:

- a frozen interoperable specification and stable test vectors;
- independent implementations and conformance evidence;
- reproducible benchmark results on representative public corpora;
- broad platform CI and runtime validation;
- external security review before making security claims;
- license, governance, and vulnerability-disclosure policies;
- clear operational, key-management, and limitations documentation.

Passing some checks does not mean the project has met all release gates.
