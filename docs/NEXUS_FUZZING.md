# NEXUS Decoder Fuzzing

The `fuzz/` package is a separate cargo-fuzz workspace and is intentionally not included in the stable root Cargo workspace.

## Run locally

Install the nightly Rust toolchain and cargo-fuzz, then run:

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz run decode_arbitrary -- -max_total_time=600 -max_len=1048576
```

The target feeds arbitrary byte strings into the bounded decoder. Whenever a stream is accepted, it checks that the decoded bytes survive a fresh encode/decode cycle. The target caps decoded output at 1 MiB to bound individual test-case allocations.

GitHub Actions runs a three-minute fuzzing session daily and uploads crash artifacts on failure. A clean fuzzing run is evidence only for the tested duration, corpus, and environment; it is not a proof of parser safety.
