# CIPHER-1 Public Preview and Community Launch Plan

Last updated: 2026-11-27

## Product decision

The product direction is a public, open-source data engine intended to become usable by both everyday computer users and developers. The project should reach users in stages rather than presenting the current research workspace as a finished application.

CIPHER-1 is a data transformation and compression-engine research project. It is not an encryption product. NARF is reversible preprocessing, not encryption. The unkeyed BLAKE3 frame digest detects accidental corruption; it does not authenticate a sender. The optional C1AE integration has not been independently audited and must not be marketed as a production security feature.

## Current release classification

**Current state: research prototype — not yet a public user beta.**

The repository currently contains Rust libraries for experimental framing and NARF, a checksum-verified public-corpus benchmark runner, synthetic benchmark tests, fuzzing, and an in-repository Python reference decoder. It does not yet provide a supported end-user CLI or desktop application. Public-corpus benchmark results have not yet been published.

Do not distribute the current workspace as a stable consumer application, and do not claim production readiness, universal speed superiority, compression superiority, cryptographic security, or cross-platform runtime support based only on compile checks.

## Target product experience

The long-term product should serve two audiences through one shared, tested engine:

1. **Everyday users:** a simple desktop application for selecting files, compressing and decompressing them, inspecting output size and elapsed time, and restoring originals.
2. **Developers and integrators:** a stable CLI, documented library API, language bindings, reproducible benchmarks, and versioned format documentation.

The desktop application must call the same stable engine APIs as the CLI. Do not fork codec or framing logic into the GUI.

## Staged release plan

### Stage 0 — Research preview preparation

- Resolve and publish the project's open-source license before calling the repository open source or accepting external contributions under an assumed license.
- Keep the experimental status and security limitations visible in the README and release notes.
- Define supported input/output limits and a safe file-handling policy.
- Complete compatibility policy for NARF v3 and shared test vectors.
- Produce first checksum-verified public-corpus measurements with dataset provenance and license review.
- Add a contribution guide and a clear bug-report / feature-request process.

Exit criteria: license is explicit; current format behavior is documented; no known unresolved compatibility decision is hidden; reproducible baseline data is published.

### Stage 1 — Developer Preview CLI

Build a small Rust CLI on top of the existing library APIs. Initial command surface should be deliberately small:

- `cipher1 --version`
- `cipher1 inspect <file>`
- `cipher1 encode <input> <output>`
- `cipher1 decode <input> <output>`
- `cipher1 benchmark <file>` (clearly marked as a local measurement, not a general performance claim)

Required behavior:

- Never overwrite an existing output unless the user explicitly requests it.
- Write to a temporary file and atomically rename after successful completion where supported.
- Preserve source files; never modify input in place.
- Enforce configurable maximum decoded output size and safe defaults.
- Report errors without exposing partial output as a valid result.
- Support stdin/stdout only after binary-safe behavior and broken-pipe handling are tested.
- Include round-trip, corruption, truncation, output-limit, empty-file, large-file, and interrupted-write tests.
- Clearly label NARF as experimental and avoid implying that the current framing digest authenticates data.

Exit criteria: release binaries and checksums for tested targets; reproducible installation instructions; integration tests on each claimed runtime platform; known limitations documented.

### Stage 2 — Public Developer Preview

- Publish versioned preview releases and checksums.
- Provide copy-paste quick starts for Windows, macOS, and Linux only for targets that have passed runtime tests.
- Publish benchmark methodology, raw CSV, hardware/toolchain metadata, and exact commands.
- Recruit a small cohort of developers and technically capable users.
- Collect feedback through GitHub Issues and a short reproducible bug template.
- Triage crashes, data-loss risks, corruption handling, compatibility, and installation failures before feature requests.

Exit criteria: no unresolved data-loss issue; release artifacts can be reproduced; test cohort can install, encode, decode, and verify round trips using documented steps.

### Stage 3 — Consumer Desktop Beta

- Build a cross-platform GUI over the stable CLI/library interface.
- Show source size, output size, elapsed time, and whether the result is larger or smaller.
- Include progress, cancellation, clear failure recovery, and safe destination selection.
- Offer a non-destructive verify operation and never delete originals automatically.
- Add accessible UI, localization, installer signing, update strategy, and platform-specific runtime tests.
- Do not include encryption or key-management claims in the first consumer beta.

Exit criteria: usability testing, accessibility review, signed installers where available, recovery testing, and documented support policy.

### Stage 4 — Stable 1.0

- Freeze the public frame specification and compatibility rules.
- Pass independent implementation and conformance testing.
- Publish repeated benchmark results on representative, licensed public corpora, including complete framing overhead and negative results.
- Validate peak memory, streaming, platform runtime behavior, and parallel scaling.
- Complete external security review before making any security claims.
- Publish support, vulnerability disclosure, governance, and deprecation policies.

## Performance and correctness acceptance

Every published benchmark must identify:

- CPU, RAM, operating system, compiler, build profile, and dependency versions;
- exact dataset source, version, license, retrieval date, and SHA-256 manifest;
- input bytes and complete output bytes including all frame overhead;
- encode and decode throughput and latency distributions across repeated runs;
- checksums proving decoded output equals the source;
- raw per-file samples and commands needed to reproduce the run;
- memory and streaming limitations where measured.

Compare against identity, LZ4, Zstandard, and Brotli under equivalent settings. Do not select only favorable files. Separate compression ratio from speed. Do not use synthetic CI smoke tests as public performance evidence.

## Initial community and marketing plan

### Positioning

Use the factual description:

> CIPHER-1 is an experimental, open-source data-engine project exploring reversible preprocessing, framed data formats, and adaptive codec selection. We are building a transparent, benchmark-driven toolchain and inviting developers to review the code and help test early releases.

Use the phrase **open-source** only after an explicit license has been selected and committed.

### Audience sequence

1. Rust and systems developers for code review and early CLI testing.
2. Compression and storage engineers for benchmark methodology and workload feedback.
3. Technical power users for the desktop beta.
4. General users after installation, recovery, and data-integrity workflows are dependable.

### Launch assets

- A concise GitHub README and quick-start page.
- A 60–90 second demo showing a real encode/decode round trip and checksum verification.
- A benchmark page with methodology and raw data, not unsupported superlatives.
- A release page with checksums, supported platforms, known limitations, and rollback guidance.
- Issue templates for bug reports, compatibility reports, and feature requests.

### First 30 days after a usable CLI exists

- Week 1: publish the preview release, installation guide, and demo.
- Week 2: invite 10–20 technically capable testers; prioritize reproducible reports.
- Week 3: fix installation, correctness, compatibility, and data-loss issues; rerun tests.
- Week 4: publish a transparent progress report with adoption, issue counts, benchmark results, and remaining blockers.

Do not advertise a desktop download before a signed or otherwise verifiable desktop build exists. Do not buy broad advertising before the first users can successfully install and complete round trips.

## License decision — owner action required

The current workspace declares `license = "UNLICENSED"` and the repository has no root `LICENSE` file. A public GitHub repository is not automatically open source, and the project must not claim that others may use, modify, or redistribute it until the owner chooses and adds a license.

A common permissive option for a Rust project is dual licensing under **MIT OR Apache-2.0**. MIT is simpler; Apache-2.0 includes express patent-grant and additional notice requirements. This is a project-owner/legal decision, not something to infer from the word "open". Before changing package metadata, confirm that the owner has rights to all project contributions and dependencies are reviewed for license compatibility.

## Release communication rules

- State the exact version, commit, supported platforms, and maturity level.
- Link benchmark data and test results.
- Report failures and known limitations.
- Never claim "world's fastest", "unbreakable", "secure encryption", or production readiness without independent evidence.
- Keep experimental NARF and C1AE functionality clearly separated from stable user-facing features.
