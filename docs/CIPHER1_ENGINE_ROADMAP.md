# CIPHER-1 Performance-First Engine Roadmap

## 1. Product objective

CIPHER-1 is being developed as a modular, performance-first data engine. Its primary value must come from measured improvements in end-to-end data processing for explicitly identified workloads: throughput, latency, memory use, and predictable resource consumption.

CIPHER-1 is not required to use one proprietary transform for every input. The engine should select an appropriate processing path and retain established codecs when they are faster or more effective.

This roadmap is an engineering plan, not a claim that the current repository is production-ready or faster than established alternatives.

## 2. Design principles

1. **Measure before optimizing.** Profile representative workloads and preserve raw measurements.
2. **End-to-end cost matters.** Include framing, metadata, planner overhead, allocations, copies, and decode cost.
3. **No universal-winner assumption.** A backend may be selected only for workloads where evidence supports it.
4. **Correctness is non-negotiable.** Every transform and codec must round-trip exactly and reject malformed inputs safely.
5. **Bound resource use.** Decoders must enforce caller limits and avoid unchecked size arithmetic and infallible allocation based on untrusted lengths.
6. **Keep layers separate.** Reversible transforms, compression, framing, integrity, authentication, and encryption have distinct contracts.
7. **Portable first, specialized second.** Optimized SIMD and architecture-specific paths must retain a tested portable fallback.
8. **Compatibility is explicit.** Experimental formats may evolve, but acceptance changes and migration policy must be documented.
9. **Negative results are useful.** Retain and publish results that show a candidate is slower, larger, or unsuitable.
10. **No unsupported claims.** Do not claim novelty, cryptographic security, quantum resistance, standards compliance, or production readiness without the required evidence.

## 3. Target architecture

The intended architecture is modular. Components should be introduced incrementally and only where their interfaces and costs can be tested.

### 3.1 Public API

A stable Rust API should expose bounded, explicit operations such as:

- encode and decode complete buffers;
- encode and decode streams with bounded working memory;
- process independent blocks;
- provide caller-owned output buffers where practical;
- configure output limits, codec policy, and resource budgets;
- report structured errors and operation metadata.

The API must not silently change the wire format or security properties based on hidden defaults.

### 3.2 Execution planner

The planner chooses a processing path from a documented policy using available information such as input size, workload class, latency budget, memory budget, and requested compatibility level.

The initial planner should be deliberately simple:

- an identity fast path with minimal overhead;
- explicit caller-selected backend mode for deterministic use;
- optional measured heuristics only after representative training/evaluation data exists;
- no expensive input scan unless its cost is justified by the expected benefit;
- deterministic fallback when analysis is unavailable or inconclusive.

Planner time and memory are part of every end-to-end benchmark. The planner must not make a slower path appear faster by excluding selection overhead.

### 3.3 Transform interface

Reversible transforms are optional preprocessing stages. NARF remains an experimental transform and must not be treated as a general-purpose compressor or cryptographic primitive.

A transform backend must define:

- input and output contracts;
- maximum expansion and framing overhead;
- required scratch memory;
- deterministic behavior;
- decoder resource limits;
- compatibility/version rules;
- benchmark evidence by workload class.

A transform should be selected only when the total pipeline is beneficial.

### 3.4 Codec interface

The engine should compare and, where appropriate, expose established backends such as:

- identity;
- simple RLE as a baseline;
- LZ4;
- Zstandard;
- Brotli;
- experimental NARF preprocessing followed by a codec only when experiments justify the composition.

Codec versions and settings must be pinned or recorded. Complete encoded representations must be compared, not payload-only estimates.

### 3.5 Framing and streaming

Framing should carry versioned metadata and explicit lengths with checked arithmetic. Block processing should allow independent bounded units, making streaming and parallel execution possible without requiring an entire file in memory.

A streaming API must document:

- maximum block size;
- maximum buffered bytes;
- backpressure behavior;
- error recovery behavior;
- whether a partially emitted stream can be resumed;
- how truncation and trailing data are handled.

### 3.6 Integrity and security

Integrity, authentication, and encryption remain optional and separate from transforms and compression.

Use established, reviewed primitives and formats for security-sensitive behavior. The current C1AE integration has not received an independent audit and lacks key management, sender identity, and replay protection; do not market it as a production security solution.

## 4. Performance research priorities

### Priority A — Establish trustworthy baselines

- Verify current main-branch CI and repository health.
- Resolve the NARF v3 canonical-token compatibility policy before changing decoder acceptance.
- Run the checksum-verified public-corpus harness on datasets whose provenance and license have been reviewed.
- Retain corpus manifests, source/version/retrieval records, license evidence, raw CSV samples, metadata, and commit SHA.
- Perform at least five independent process runs on the same documented machine/configuration.
- Include identity, pair-RLE, NARF v3, LZ4, Zstandard, and Brotli with complete encoded sizes.

### Priority B — Profile the NARF encoder

The current NARF v3 encoder evaluates 64 coefficient pairs. Prior screening results show that this candidate-selection work is costly relative to established fast codecs on the tested synthetic fixtures.

Investigate, as separate falsifiable experiments:

- cheaper candidate pruning;
- block-adaptive coefficient selection;
- early abandonment with a proven lower bound;
- vectorized residual scoring;
- reduced passes and memory traffic;
- alternate candidate sets evaluated on held-out data.

Do not adopt an optimization solely because it improves one synthetic fixture. Validate exact round-trip behavior, deterministic output policy, encoded size, and representative corpus performance.

### Priority C — Build the engine fast path

- Introduce a minimal identity path and measure planner/API overhead.
- Define a codec trait/interface without imposing dynamic dispatch on hot paths unless benchmarks justify it.
- Measure buffer reuse and caller-provided output buffers.
- Keep single-thread execution efficient for small inputs.

### Priority D — Streaming and parallelism

- Implement bounded-memory block processing.
- Measure block sizes and scheduling overhead.
- Parallelize independent blocks while preserving deterministic frame order and clear failure semantics.
- Report scaling at 1, 2, 4, and 8 workers on documented hardware.

### Priority E — Architecture-specific optimization

- Profile x86_64 and ARM64 independently.
- Evaluate AVX2/AVX-512 and NEON only where profiling identifies suitable kernels.
- Keep feature detection safe and portable fallbacks tested.
- Do not infer runtime support from cross-compilation alone.

## 5. Benchmark protocol

Every performance report should include:

- exact repository commit and dirty/clean source state;
- dataset identifiers, versions, SHA-256 manifests, provenance, and license evidence;
- hardware model, CPU microarchitecture, RAM, OS/kernel, compiler, dependency lockfile, build profile, and codec settings;
- complete encoded byte counts, including all framing and metadata;
- encode and decode throughput;
- latency distributions with enough samples for meaningful tail estimates;
- peak resident memory and allocation counts where available;
- CPU cycles per input byte where hardware counters are available;
- streaming buffer/memory bounds;
- parallel scaling and thread count;
- raw samples and independent process repetitions.

Do not use the current 30-sample synthetic CI smoke test as publication-quality evidence. With 30 samples, nearest-rank p99 is the maximum observed sample. Do not make stable tail-latency claims from it.

Performance claims must specify the workload, hardware, codec settings, and baseline. A result on one corpus or one machine does not establish universal superiority.

## 6. Phased roadmap and exit criteria

### Phase 1 — Repository correctness and measurement baseline

Work:
- verify latest main CI and workflows;
- settle experimental NARF v3 canonicality/compatibility policy;
- audit the corpus runner and corpus licensing/provenance;
- produce reproducible public-corpus baseline results.

Exit criteria:
- all reported checks are linked to actual workflow runs;
- compatibility decision and test vectors are documented;
- raw benchmark artifacts and metadata are retained;
- results include negative findings and limitations.

### Phase 2 — Core performance experiments

Work:
- profile NARF candidate scoring;
- implement one bounded optimization per experiment;
- compare block-adaptive and candidate-pruning hypotheses;
- add memory/allocation profiling.

Exit criteria:
- optimizations have regression tests and repeatable measurements;
- no correctness or compatibility regression;
- gains are reproduced on held-out workloads or explicitly limited to a named workload class.

### Phase 3 — Engine core and planner

Work:
- define stable internal backend interfaces;
- implement identity fast path;
- add explicit codec selection and a minimal planner;
- measure planner and dispatch overhead.

Exit criteria:
- end-to-end tests cover each backend;
- planner decisions are deterministic under documented policy;
- no hidden automatic behavior changes the format or security properties.

### Phase 4 — Streaming and parallel runtime

Work:
- bounded block/stream API;
- backpressure and error semantics;
- parallel independent-block processing;
- memory and scaling benchmarks.

Exit criteria:
- bounded-memory behavior is tested on large inputs;
- partial/truncated streams are handled deterministically;
- parallel output conforms byte-for-byte to the documented format;
- throughput gains include scheduling and framing overhead.

### Phase 5 — Cross-platform SDK

Work:
- Linux, Windows, and macOS runtime validation;
- x86_64 and ARM64 performance measurements;
- Python bindings and, if justified, C ABI/WASM bindings;
- independently maintained decoder/conformance implementation.

Exit criteria:
- supported targets pass conformance and runtime tests;
- ABI/API versioning and support policy are documented;
- third-party implementation results are available.

### Phase 6 — Production candidate

Work:
- freeze interoperable specifications;
- external code and security review appropriate to the product claims;
- dependency, license, governance, vulnerability-disclosure, and release processes;
- installation, operations, migration, backup/recovery, and troubleshooting documentation.

Exit criteria:
- every release gate is met and independently evidenced;
- unresolved limitations are disclosed;
- production claims are limited to the tested configurations and workloads.

## 7. Release gates

A production release is blocked until all applicable conditions are met:

- stable, frozen, versioned wire-format specifications;
- independent implementation and conformance evidence;
- reproducible public-corpus benchmarks with raw data;
- broad platform runtime CI, not compile checks alone;
- fuzzing, property tests, malformed-input tests, and resource-limit tests;
- external security review before any security claims;
- license and ownership clarity, governance, and vulnerability-disclosure policy;
- operational, compatibility, migration, and limitations documentation.

Passing some gates is not completion. A scheduled development process does not guarantee that the project will meet these gates.

## 8. Commercialization hypothesis

The initial product should be a developer-focused engine SDK, not a cloud service by default.

Potential offerings, subject to license review and customer validation:

- Community core: free SDK/CLI and documented baseline backends;
- Commercial developer support: optimized builds, integration help, and priority support;
- Business/enterprise: deployment assistance, performance tuning, long-term support, and custom integration;
- Hosted API only if customer discovery shows that managed processing is more valuable than an embeddable library.

Pricing must be validated through customer interviews, pilot deployments, support-cost analysis, and comparison with existing codec/library licensing. Do not forecast revenue or valuation before evidence of demand.

## 9. Current known limitations

- NARF is experimental; its v3 token-segmentation compatibility policy is not frozen.
- The current NARF encoder is substantially slower than LZ4 and Zstandard on previously recorded synthetic fixtures.
- Public-corpus benchmark tooling exists, but publication-quality public-corpus results have not yet been established.
- The in-repository Python decoder is a second-language consistency check, not an independently maintained third-party implementation.
- C1AE integration has not been independently audited and does not provide key lifecycle, sender identity, or replay protection.
- Existing cross-compilation checks do not prove full runtime correctness or performance on every target.
- Repository licensing and governance require explicit review before commercial distribution.

## 10. Daily engineering loop

Each development iteration should:

1. inspect current repository state, open PRs/issues, and actual CI;
2. select one highest-value bounded task;
3. state a falsifiable hypothesis and expected regression risks;
4. implement the smallest coherent change on a dedicated branch;
5. add tests and run available checks;
6. inspect actual CI results;
7. open or update a PR when appropriate, without unsafe merges;
8. record measured results, failed experiments, limitations, and the next task.

A day with no safe, evidence-backed code change may produce a research note or a clear blocker report instead of a speculative commit.
