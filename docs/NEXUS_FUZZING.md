# NEXUS Decoder Fuzzing

The `fuzz/` package is a separate cargo-fuzz workspace and is intentionally not included in the stable root Cargo workspace. It contains targets for arbitrary NARF decoder inputs and arbitrary C1AE authenticated-frame inputs.

## NARF decode target strategy

The `decode_arbitrary` target combines three complementary paths:

1. Feed arbitrary byte strings into the bounded decoder with a 1 MiB output limit. When a stream is accepted, check that a fresh encode/decode cycle preserves the decoded bytes.
2. For fuzz inputs up to 64 KiB, encode the input and verify the encoder's output round-trips through the bounded decoder.
3. Construct a deterministic valid version-3 frame and mutate it. The frame uses `alpha = beta = 0`, so its residual sequence is exactly the payload after the two seed bytes. Its token body contains two extended repeats, a four-byte short repeat, and a 128-byte alternating literal. The repeat length varies from 130 to 255 bytes, keeping the generated payload between 392 and 642 bytes.

The target mutates token bytes while preserving the valid header, directing mutations into token parsing, malformed run-length handling, and canonical unsigned-LEB128 parsing. It separately mutates the version, coefficients, declared length, and seed fields while preserving the magic. This supplements arbitrary-input fuzzing, which can otherwise reject most random inputs before reaching the token parser.

The current main-branch decoder does not enforce the stricter version-3 token-segmentation canonicality proposed separately in PR #4. This target therefore does not claim to test that rule. If PR #4 is accepted, its canonical-token validation should receive dedicated mutation cases against the merged implementation.

The structured frame is generated from each fuzz input; there is no checked-in seed corpus for this target. No coverage increase is claimed without coverage instrumentation. This strategy does not replace persisted real-world samples, property tests, or independent conformance testing.

## Run locally

Install the nightly Rust toolchain and cargo-fuzz, then run:

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run decode_arbitrary -- -max_total_time=600 -max_len=1048576
cargo +nightly fuzz run aead_open -- -max_total_time=600 -max_len=1048576
```

The target caps decoded output at 1 MiB to bound individual test-case allocations.

GitHub Actions runs each fuzz target for up to three minutes on pushes to `main`, pull requests targeting `main`, and the daily schedule. Crash artifacts are uploaded on failure. A clean fuzzing run is evidence only for the tested duration, inputs, and environment; it is not a proof of parser or cryptographic safety.
