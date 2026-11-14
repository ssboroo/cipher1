# NARF v3 Preliminary Benchmark Results

Date: 2026-11-14  
CI run: https://github.com/ssboroo/cipher1/actions/runs/43571940660  
Commit: 05afd334 (short SHA)  
Environment: GitHub-hosted ubuntu-latest; 16 KiB deterministic synthetic inputs;
2 warm-up iterations and 10 measured iterations per codec.

These are preliminary screening measurements from one CI run. The runner CPU
model was not recorded, and no confidence intervals or independent process
repetitions are available. Do not treat the timing values as portable or as
proof of superiority.

## Encoded size

All sizes are bytes. NARF includes its 17-byte frame header. LZ4, Zstandard,
and Brotli sizes include the framing produced by their benchmark APIs.
Pair-RLE is a payload-only reference.

| Dataset | Input | NARF v3 | NARF ratio | LZ4 | Zstandard | Brotli |
|---|---:|---:|---:|---:|---:|---:|
| Repeated byte | 16,384 | 21 | 0.001282 | 80 | 19 | 13 |
| Period-four pattern | 16,384 | 16,399 | 1.000916 | 83 | 22 | 17 |
| Byte ramp | 16,384 | 21 | 0.001282 | 335 | 276 | 259 |
| Text-like sequence | 16,384 | 16,401 | 1.001038 | 124 | 65 | 47 |
| Deterministic random | 16,384 | 16,401 | 1.001038 | 16,454 | 16,394 | 16,388 |

## Encode and decode timing

Nanoseconds per input byte, measured on the same CI runner. Lower is faster.

| Dataset | NARF encode | NARF decode | LZ4 encode | LZ4 decode | Zstd encode | Zstd decode | Brotli encode | Brotli decode |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Repeated byte | 63.814 | 3.354 | 0.103 | 0.018 | 0.576 | 0.317 | 4.346 | 2.205 |
| Period-four pattern | 295.878 | 4.914 | 0.062 | 0.525 | 0.485 | 0.306 | 3.773 | 1.468 |
| Byte ramp | 302.299 | 3.247 | 0.077 | 0.038 | 0.545 | 0.120 | 4.066 | 1.670 |
| Text-like sequence | 304.286 | 0.012 | 0.075 | 0.214 | 0.493 | 0.296 | 3.623 | 1.546 |
| Deterministic random | 308.208 | 0.011 | 0.132 | 0.019 | 0.984 | 0.102 | 11.577 | 10.183 |

## Interpretation

1. The recurrence model is exceptionally effective by size on the repeated-byte
   and ramp fixtures, reducing 16 KiB to 21 bytes.
2. It does not model the period-four fixture well; the result is 15 bytes larger
   than the input, while all three general-purpose codecs compress it strongly.
3. Text-like and random inputs are effectively stored raw, with the fixed frame
   overhead. General-purpose compressors do substantially better on the
   text-like fixture.
4. NARF encoding is much slower than the established codecs in this harness.
   Its current 64-candidate search constructs a token stream for every
   coefficient pair, which is an obvious optimization target.
5. The raw fallback bounds expansion to at most the 17-byte NARF header; it
   does not guarantee output smaller than the original unframed input.

## Next experiment

Replace per-candidate token-buffer construction with an exact token-size
estimator. Score all 64 coefficient pairs without allocating output buffers,
then encode only the winning pair. Verify estimator/encoder size equality for
all candidate pairs and boundary-length test vectors. Re-run this same benchmark
before attempting block-adaptive coefficient selection.

No claim of world-first novelty, universal compression advantage, or
production readiness is supported by this experiment.
