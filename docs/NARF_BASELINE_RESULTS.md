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


## Post-optimization screening run

After the baseline, the encoder was changed to estimate the exact token-stream
length for each coefficient pair without materializing 64 token buffers. It
then emits only the selected pair. The estimator is checked against the actual
packer across all 64 candidate pairs and run-boundary test lengths.

Date: 2026-11-14  
CI run: https://github.com/ssboroo/cipher1/actions/runs/43572417945  
Commit: d3f1918a (short SHA)  
Environment and workload settings: same GitHub-hosted ubuntu-latest harness.

### Encoded size

The byte counts are unchanged by this encoder-only optimization.

| Dataset | Input | NARF v3 | NARF ratio | LZ4 | Zstandard | Brotli |
|---|---:|---:|---:|---:|---:|---:|
| Repeated byte | 16,384 | 21 | 0.001282 | 80 | 19 | 13 |
| Period-four pattern | 16,384 | 16,399 | 1.000916 | 83 | 22 | 17 |
| Byte ramp | 16,384 | 21 | 0.001282 | 335 | 276 | 259 |
| Text-like sequence | 16,384 | 16,401 | 1.001038 | 124 | 65 | 47 |
| Deterministic random | 16,384 | 16,401 | 1.001038 | 16,454 | 16,394 | 16,388 |

### Encode and decode timing

Nanoseconds per input byte from this single run.

| Dataset | NARF encode | NARF decode | LZ4 encode | LZ4 decode | Zstd encode | Zstd decode | Brotli encode | Brotli decode |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Repeated byte | 60.729 | 3.794 | 0.058 | 0.021 | 0.322 | 0.134 | 3.232 | 1.996 |
| Period-four pattern | 214.198 | 5.391 | 0.060 | 0.641 | 0.324 | 0.137 | 3.691 | 1.117 |
| Byte ramp | 216.250 | 3.779 | 0.067 | 0.032 | 0.376 | 0.110 | 3.585 | 1.283 |
| Text-like sequence | 224.198 | 0.015 | 0.061 | 0.198 | 0.335 | 0.273 | 2.891 | 1.210 |
| Deterministic random | 225.816 | 0.015 | 0.111 | 0.023 | 0.741 | 0.103 | 9.205 | 3.145 |

### Measured change

Compared with the baseline run, NARF encode time per byte decreased by
approximately 4.8% on repeated bytes, 27.6% on the period-four fixture, 28.5%
on the ramp, 26.3% on text-like data, and 26.7% on deterministic random data.
Encoded sizes did not change.

This is a useful reduction in candidate-scoring overhead, but NARF remains
substantially slower than LZ4 and Zstandard in this harness. The two CI runs
were not controlled repetitions, so the percentage changes are directional
engineering evidence only, not statistically established speedups.

## Interpretation

1. The recurrence model is exceptionally effective by size on the repeated-byte
   and ramp fixtures, reducing 16 KiB to 21 bytes.
2. It does not model the period-four fixture well; the result is 15 bytes larger
   than the input, while all three general-purpose codecs compress it strongly.
3. Text-like and random inputs are effectively stored raw, with the fixed frame
   overhead. General-purpose compressors do substantially better on the
   text-like fixture.
4. The exact token-size estimator reduced observed NARF encode time by roughly
   26–29% on four of the five synthetic fixtures, without changing encoded
   size. This is a single-run comparison and needs repeated measurements.
5. NARF encoding remains substantially slower than LZ4 and Zstandard. The
   estimator removes 64 output-buffer constructions but does not reduce the
   64-candidate residual-scoring work.
6. The raw fallback bounds expansion to at most the 17-byte NARF header; it
   does not guarantee output smaller than the original unframed input.

## Next experiment

Collect multiple independent runs and representative public-corpus measurements.
Then evaluate whether block-adaptive coefficient selection or a cheaper
candidate-pruning method can improve the period-four and text-like fixtures
without making the encoder even slower.

No claim of world-first novelty, universal compression advantage, or
production readiness is supported by this experiment.
