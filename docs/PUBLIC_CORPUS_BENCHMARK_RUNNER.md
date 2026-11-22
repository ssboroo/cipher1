# Public Corpus Benchmark Runner

## Research question

For a pinned, legally usable corpus and fixed codec settings, does the NARF v3 encoder produce a lower aggregate complete-frame byte count than identity, pair-RLE, LZ4, Zstandard, and Brotli, while keeping encode/decode cost within a declared budget?

The runner is an experiment harness, not evidence of a compression advantage. It does not download datasets and does not determine whether a dataset license permits use or redistribution.

## Build and run

Use a release build for measurements:

```bash
cargo test -p narf-corpus-bench
cargo run --release -p narf-corpus-bench -- \
  --corpus-id my-corpus-v1 \
  --corpus-dir /data/my-corpus \
  --manifest /data/my-corpus.manifest.tsv \
  --output-csv /tmp/my-corpus-results.csv
```

The default metadata file is the CSV path with `.metadata.txt` appended. Use `--metadata-out` to select another path. Output files must not already exist and must be outside the corpus directory.

Default resource and timing limits:

- Maximum file size: 64 MiB.
- Maximum total corpus size: 1 GiB.
- Warm-up operations per file/codec/operation: 5.
- Measured samples per file/codec/operation: 30.
- At least five measured samples are required. For independent replication, run the process at least five times on the same documented machine/configuration and retain every output pair.

The file and total-size limits can be changed with `--max-file-bytes` and `--max-total-bytes`. The pair-RLE baseline uses a 32-bit original-length field, so the per-file limit cannot exceed `u32::MAX`.

## Manifest format

The manifest is UTF-8 tab-separated text. Its header is exact:

```text
sha256<TAB>bytes<TAB>path
```

Each row contains:

1. A lowercase, 64-character SHA-256 digest.
2. The exact file length in canonical decimal notation (no leading zero except for zero).
3. A UTF-8 path relative to the corpus root, using forward slashes.

Rows must be sorted strictly by UTF-8 path bytes and paths must be unique. Empty files are allowed and must be listed. Absolute paths, parent traversal, empty path components, symlinks, non-regular files, and non-portable path characters are rejected. The runner does not recursively discover files; the manifest defines the exact experiment set.

Example (digests are illustrative only):

```text
sha256	bytes	path
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855	0	empty.bin
2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824	5	nested/hello.txt
```

Generate digests using a trusted SHA-256 implementation. SHA-256 is specified by NIST FIPS 180-4; this runner uses the `sha2` crate rather than implementing a hash function locally. A manifest checksum verifies bytes against the manifest, but it does not establish source authenticity, legal permission, or corpus provenance.

Before invoking any codec, the runner checks every listed file's size and SHA-256. It then re-reads and re-hashes each file immediately before benchmarking it. Keep the corpus directory unchanged during a run; standard portable filesystem APIs cannot eliminate every time-of-check/time-of-use race against concurrent hostile modification.

## Compared representations

Every codec is round-trip checked and its encoder is checked for deterministic output before timing.

- Identity: original bytes unchanged, without a wrapper.
- Pair-RLE: benchmark-only PRL1 frame with an 8-byte header and count/value pairs.
- NARF: complete output from the repository's current v3 writer, including its header and raw fallback.
- LZ4: size-prepended representation from `lz4_flex`.
- Zstandard: complete bulk frame at level 3.
- Brotli: complete stream at quality 5, lgwin 22, and a 4 KiB writer buffer.

The size metric is the complete encoded byte count. The aggregate ratio is `sum(encoded_bytes) / sum(input_bytes)`, not an unweighted mean of file ratios. The median of per-file ratios is also reported. Empty-file ratios are undefined and excluded from the median, but empty-file bytes and file counts remain in aggregate reporting.

## Output

The CSV contains one `file` row for every file/codec pair and one `aggregate` row for each codec. File rows include input bytes, encoded bytes, per-file ratio, encode/decode nearest-rank p50/p95/p99 nanoseconds per input byte, and sample count. Aggregate rows include file counts, total input/encoded bytes, aggregate ratio, and median non-empty-file ratio.

The metadata text records the manifest SHA-256, corpus identifier, size limits, sample settings, package version, target OS/architecture, available Rust/Cargo version strings, optional GitHub commit SHA, and fixed codec settings. CPU model is explicitly marked as not collected; record it separately with the experiment log. Keep the original manifest, metadata file, CSV, source license evidence, retrieval date, and repository commit together.

## Limitations

- The runner does not download, redistribute, license-review, or discover corpora.
- It does not measure peak resident memory, CPU cycles, energy, streaming throughput, or confidence intervals.
- Percentiles are descriptive. With 30 samples, p99 is the maximum observed sample.
- The runner benchmarks one file at a time, but it loads that file into memory for the codec comparison.
- The manifest is an experiment input, not a security boundary. Do not run it on untrusted directories or concurrently changing files.
- Codec library versions and exact compiler/build environment must be preserved with each experiment. The metadata captures requested dependency ranges and runtime version information, but not a complete dependency lockfile or CPU microarchitecture.
- A result on one corpus, one machine, or one configuration does not establish universal compression performance, novelty, security, or production readiness.
