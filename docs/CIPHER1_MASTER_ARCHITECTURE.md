# CIPHER-1 Master Architecture and Unified Delivery Plan

**Assessment date:** 2026-11-27  
**Product direction:** one open data engine for developers and everyday users  
**Engineering priority:** correctness, safe usability, measurable performance, portability

## 1. Product definition

CIPHER-1 is a modular, lossless data engine. Its purpose is to inspect, transform, compress, frame, verify, and optionally protect data through clearly separated components.

The product is one shared engine exposed through multiple interfaces:

- **CLI:** first usable product surface and automation interface.
- **Library/API:** stable integration surface for applications.
- **Desktop app:** accessible file-oriented experience for everyday users, built on the same engine API.
- **Web/WASM:** optional browser interface after bounded-memory and streaming behavior are stable.

The CLI, library, desktop application, and browser build must not contain separate codec implementations. They call the same core APIs and share the same format and conformance tests.

CIPHER-1 is not currently a finished consumer product, a stable standard, or a production security product. Release maturity must be stated accurately.

## 2. Current repository baseline

The default branch currently contains:

- `cipher1-core`: versioned binary framing, unkeyed BLAKE3 corruption detection, and an optional C1AE XChaCha20-Poly1305 envelope.
- `nexus-transform`: experimental NARF reversible preprocessing, with versioned frames and bounded decoding.
- A benchmark harness comparing identity, benchmark-only pair-RLE, NARF, LZ4, Zstandard, and Brotli.
- A checksum-verified public-corpus benchmark runner and manifest generator.
- Shared NARF frame and malformed-input vectors, plus an in-repository Python reference decoder.
- Rust CI, cross-target compile checks, and decoder fuzzing workflows.

Important boundaries:

- LZ4, Zstandard, and Brotli are currently benchmark/dev comparison backends, not yet a stable user-facing codec-planner API.
- The existing synthetic benchmark is a smoke test, not publication-quality evidence.
- The public-corpus runner exists, but no public-corpus results have yet been published.
- The Python decoder is an in-repository cross-language check, not an independently maintained external implementation.
- NARF v3 remains experimental. Its token-segmentation compatibility question in PR #4 must be resolved before freezing the format.
- C1AE uses a standard AEAD primitive, but the integration has not been independently audited and does not supply key lifecycle, sender identity, replay protection, or key distribution.
- The default branch still declares `UNLICENSED`. The owner-approved MIT OR Apache-2.0 change is pending in PR #15 and must not be described as effective until merged.

## 3. Unified layered architecture

The architecture separates data processing from security and from user interfaces.

```text
CLI / Desktop / Web-WASM / Embedded Library
                    |
             Stable Engine API
                    |
       Input Policy + Resource Limits
                    |
          Execution Planner
          /       |        \
   Transform   Codec     Framing
   Plugins     Backends  + Metadata
       \         |        /
        ---- Verified Output ----
                    |
          Integrity Verification
                    |
      Optional Protection Envelope
      (separate, reviewed crypto layer)
```

### Layer responsibilities

1. **Public API:** stable operations, explicit options, typed errors, cancellation/progress hooks where appropriate.
2. **Input and resource policy:** maximum input/output sizes, block sizes, memory budgets, concurrency limits, and safe path handling.
3. **Planner:** selects only available, compatible implementations using declared capabilities and measured workload behavior. A deterministic safe fallback must always exist.
4. **Reversible transforms:** optional preprocessing such as NARF. A transform is used only when its complete representation is beneficial and its format version is supported.
5. **Codec backends:** mature established codecs first. Each adapter declares format/version, streaming capability, memory behavior, supported levels, and licensing.
6. **Framing:** versioned stream/block metadata, lengths, codec identifiers, transform identifiers, and compatibility rules.
7. **Integrity:** checks accidental corruption and verifies decoded output. Unkeyed hashes must never be described as sender authentication.
8. **Protection:** optional AEAD wrapping, performed after compression. Decryption/authentication must complete before decompression. Keep keys, identities, replay protection, and key lifecycle outside the compression layer.
9. **Interfaces:** CLI and GUI are thin clients of the same API. No UI-specific codec or wire-format behavior.

### Core invariants

- Lossless paths must round-trip byte-for-byte.
- Decoders must validate headers and lengths before allocating or processing declared output.
- Every untrusted decode path must enforce caller-visible output and resource limits.
- Never overwrite an existing destination by default.
- Never expose partial output as a successful file.
- Preserve original input files; never delete them automatically.
- Unknown versions, algorithms, flags, and trailing bytes are rejected unless a versioned specification explicitly permits them.
- Scalar/reference implementations remain correct when optional SIMD or parallel paths are unavailable.
- Format changes require versioning, vectors, compatibility tests, and migration notes.

## 4. Performance strategy

Performance is an end-to-end property, not an isolated algorithm score.

### Backend policy

- Keep identity as the guaranteed fallback.
- Integrate mature codecs through narrow adapters, beginning with LZ4 and Zstandard where workload and licensing review support them; evaluate Brotli for relevant workloads.
- Treat NARF as an experimental transform candidate, not the default.
- Keep codec dependencies optional where practical so minimal builds stay small.
- Do not add a dependency because it is new or fashionable.

### Planner policy

The planner should use low-cost, explainable decisions:

1. Validate the input and policy limits.
2. Use cheap sampling or known metadata only when it has a measured net benefit.
3. Avoid expensive candidate exploration for tiny inputs.
4. Select from codecs supported by the target build and requested format.
5. Include framing, metadata, and transform overhead in the size estimate.
6. Fall back to identity when the estimated total representation is not beneficial.
7. Record the chosen path and reason in inspect/benchmark diagnostics.

Do not make a costly 64-candidate NARF search the default for all data. Profile first; consider bounded sampling, block adaptation, pruning, and early exit only as hypotheses, with a reference implementation and differential tests.

### Benchmark requirements

Every benchmark report must include:

- exact input corpus, version, provenance, license, and SHA-256 manifest;
- hardware, OS, kernel, compiler, dependency versions, build flags, and codec settings;
- full encoded bytes including all framing and metadata;
- compression ratio, encode/decode throughput, and latency distributions;
- raw samples, repeat count, warm-up policy, and reproducible commands;
- output checksum equality;
- peak memory and streaming measurements when available;
- negative results and workloads where a backend loses.

Use synthetic CI fixtures for regression smoke tests only. Use licensed representative public corpora and multiple independent runs for public performance claims. Never claim universal superiority from a small or selected workload.

## 5. Safe user-facing product

### First CLI preview

Initial commands:

- `cipher1 --version`
- `cipher1 inspect <file>`
- `cipher1 encode <input> <output>`
- `cipher1 decode <input> <output>`
- `cipher1 verify <file>` once verification semantics are defined
- `cipher1 benchmark <file>` as a local measurement, not a universal claim

File safety contract:

- Refuse to overwrite unless an explicit `--force` option is supplied.
- Write to a unique temporary file in the destination directory.
- Flush and close successfully before atomic rename where supported.
- Remove temporary output after failure; never replace a valid destination with partial data.
- Detect input/output path aliasing and reject it.
- Preserve permissions and metadata only where the platform behavior is explicit and tested.
- Apply a conservative decoded-size limit, with an explicit user override.
- Support cancellation without leaving a valid-looking partial output.
- Test empty, binary, huge, malformed, truncated, corrupted, and interrupted files.

### Desktop beta

Only after the CLI and API semantics are stable:

- drag-and-drop file selection;
- clear Compress / Decompress / Verify actions;
- before/after size and elapsed time;
- visible progress, cancel, and safe recovery;
- explicit output destination;
- no automatic deletion of originals;
- clear format/version and experimental-feature labels;
- accessibility, localization, signed installers where available, and update/rollback policy.

### Web/WASM

Only after the core API has bounded memory and tested streaming semantics. Browser support must account for browser memory limits, file-size restrictions, cancellation, and target-specific codec availability.

## 6. Security and trust boundaries

- NARF is reversible preprocessing, not encryption.
- The unkeyed BLAKE3 digest detects accidental corruption; it does not authenticate a sender.
- C1AE uses XChaCha20-Poly1305 but the repository integration has not been independently audited.
- Compression must occur before encryption; authenticated decryption must succeed before decompression.
- Apply separate limits to encrypted plaintext/frame size and decompressed output size.
- Compression can leak information through ciphertext length when secrets and attacker-controlled input share a compression context.
- Do not create custom cryptographic primitives.
- Do not claim post-quantum security without a specific standardized algorithm, parameter set, implementation review, and interoperability evidence.
- Publish a threat model, vulnerability reporting channel, dependency policy, and security response process before production security claims.

## 7. Open-source and supply-chain policy

The owner's selected direction is MIT OR Apache-2.0 dual licensing. PR #15 contains the proposed license texts and metadata update; until it is merged, the default branch remains `UNLICENSED`.

Before merging and publishing releases:

- confirm rights to all existing project contributions;
- verify the complete canonical license texts and copyright notices;
- review third-party dependencies, datasets, benchmarks, and bundled assets separately;
- document contribution terms and provenance;
- use dependency auditing and an SBOM;
- pin release inputs and publish checksums;
- add artifact signing only when the process is implemented and verified.

The project license does not relicense third-party code, data, or separately owned contributions.

## 8. Unified delivery roadmap

### Milestone A — Establish a clean project foundation

- Review and merge the owner-approved license PR after complete license-text and ownership review.
- Resolve duplicate/stale roadmap documents into this master plan.
- Decide the NARF v3 token canonicality and compatibility policy; keep PR #4 unmerged until approved.
- Keep main-branch status and experimental limitations accurate.

**Exit:** license is effective on main; format compatibility decision is explicit; documentation has one authoritative architecture plan.

### Milestone B — Safe CLI and stable API

- Define the minimal engine API and typed error model.
- Add CLI commands for version, inspect, encode, and decode.
- Implement no-overwrite, temporary output, atomic commit, path-alias checks, and bounded decode.
- Add end-to-end tests for success and failure paths.
- Test actual runtime behavior on each claimed OS.

**Exit:** a tester can safely encode and decode files and prove exact round-trip equality using documented commands.

### Milestone C — Codec integration and measured planner

- Add narrow codec adapters with explicit feature and format identifiers.
- Establish identical test vectors and full-frame comparisons.
- Implement deterministic planner selection with identity fallback.
- Publish the first public-corpus benchmark with raw results and reproducible metadata.
- Optimize only measured bottlenecks.

**Exit:** planner decisions are explainable and benchmarked; no unsupported performance claim.

### Milestone D — Public developer preview

- Publish versioned binaries only for runtime-tested targets.
- Publish checksums, installation instructions, known limitations, and release notes.
- Recruit a small technical tester cohort.
- Prioritize data-loss, corruption, compatibility, and install issues.

**Exit:** independent testers can install, use, and verify round trips without data-loss defects.

### Milestone E — Consumer desktop and web

- Build the desktop UI over the same stable API.
- Add progress, cancellation, accessibility, localization, and recovery.
- Add WASM/browser only after memory and streaming validation.
- Establish support, telemetry privacy, and update policies.

**Exit:** usability and platform validation are complete for each advertised target.

### Milestone F — Stable 1.0

- Freeze a versioned format specification.
- Obtain independently maintained implementation and conformance evidence.
- Publish reproducible benchmark reports on representative datasets.
- Complete external security review before security claims.
- Publish governance, support, vulnerability disclosure, deprecation, and migration policies.

**Exit:** all release gates pass; a successful unit test or CI run alone is not sufficient.

## 9. Immediate priority order

1. Finish the license review and merge PR #15 only after verifying rights and full license text.
2. Resolve NARF v3 compatibility policy without breaking existing readers silently.
3. Build the safe CLI on existing core APIs; this is the highest-value user-facing milestone.
4. Integrate mature codecs through a clean adapter interface and benchmark full representations.
5. Produce the first licensed public-corpus report.
6. Stabilize API and format before desktop/WASM work.
7. Optimize through profiling, not assumptions.

## 10. Engineering iteration contract

Every iteration must inspect the live repository, open PRs/issues, and actual CI; choose one bounded task; write a falsifiable hypothesis; implement on a dedicated branch; add regression tests; run available checks; inspect actual CI; and report the exact commit, results, and limitations.

Never claim a test passed unless its result is visible. Never merge around a failing or absent required check. Never make a release, security, novelty, or speed claim beyond the available evidence.

A schedule is a review cadence, not proof of continuous background engineering. The repository state and completed commits are the source of truth.
