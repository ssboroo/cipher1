# Public Corpus Benchmark Protocol

## Status and purpose

This document defines the admission and reporting rules for a future public-corpus comparison of NARF v3, identity, pair-RLE, LZ4, Zstandard, and Brotli. It is a protocol, not a report of completed measurements. No public-corpus results are claimed by this document.

The falsifiable research question is:

> On a declared, legally usable corpus and fixed codec settings, does NARF v3 produce a lower aggregate complete-frame byte count than each selected baseline, without unacceptable encode/decode cost or memory use?

The result may be positive, mixed, or negative. A result on one corpus does not establish a universal advantage.

## Corpus admission requirements

Every corpus release and every included file must have a recorded source, version, license or terms-of-use evidence, and cryptographic checksum before measurement.

Do not assume that a dataset is redistributable merely because it is publicly downloadable. Review the dataset's own license and the license or terms of each included component. If redistribution is not permitted or the terms are unclear, record the source URL and retrieval instructions but do not commit the dataset to this repository.

The experiment manifest must record:

- Corpus name, release/version, canonical source URL, retrieval date, and license/terms reference.
- File-relative path, byte length, SHA-256, and inclusion/exclusion decision for every candidate file.
- Any exclusions and the objective reason (unsupported format, duplicate, corrupt file, unavailable license evidence, or configured size bound).
- Exact preprocessing, if any. The primary comparison must use original file bytes without normalization, recompression, line-ending conversion, or content-aware filtering.
- Toolchain, codec crate/library versions, codec settings, target triple, build profile, CPU model, operating system, and repository commit SHA.

Candidate sources for evaluation include Canterbury, Calgary, and Silesia. Their availability does not imply permission to redistribute their contents. Confirm the applicable terms for each dataset release before use. Additional real-world corpora should be added only with documented provenance and licensing.

## Initial candidate source screening (2026-11-18)

This is a provenance and licensing screen, not legal advice and not a grant of permission. The reviewed pages establish benchmark purpose and file provenance, but they do not establish a blanket redistribution license for any of the three candidate corpora.

| Candidate | Evidence reviewed | Admission decision |
|---|---|---|
| Canterbury | The [official corpus site](https://www.corpus.canterbury.ac.nz/) describes it as a lossless-compression research benchmark. The reviewed landing page did not expose a corpus-wide license or complete per-file rights statement. | Not yet admitted. Obtain and review the exact release's README/terms and the rights for each included source file. |
| Calgary | The [corpus reference page](https://www.data-compression.info/Corpora/CalgaryCorpus/) describes the 18-file collection and links to a University of Calgary FTP source. The page's own copyright notice is not a license for the corpus contents. No corpus-wide redistribution grant was verified in the reviewed material. | Not yet admitted. Retrieve the exact archive's accompanying README/index and review component-level rights; do not infer rights from the FTP location. |
| Silesia | The [original author's corpus page](https://sun.aei.polsl.pl/~sdeor/index.php?page=silesia) identifies 12 files and their different sources, including Project Gutenberg, Mozilla, a hospital image, NCI, OpenOffice, OSDB, the Virtual Library of Polish Literature, Samba, Harvard astronomical catalogs, and XMLPPM. The reviewed page provides provenance and MD5 identifiers but no blanket corpus redistribution license. | Not yet admitted. Review the rights for every component and the exact archive. A single corpus-level assumption is insufficient. |

The Silesia source page reports a 12-file corpus totaling 211,938,580 bytes and explicitly describes heterogeneous source material. That makes it a useful candidate for a later large-file experiment, but also makes per-file rights review essential. Its published MD5 values are historical identifiers only; generate SHA-256 checksums locally for the experiment manifest.

The first implementation should therefore accept user-supplied corpus directories and manifests without downloading or redistributing corpus bytes. A corpus may be used in a published experiment only after its exact release and component-level terms have been recorded. If the rights remain unclear, exclude that file and document the exclusion; do not silently substitute another copy.

## File selection and reproducibility

1. Pin a specific corpus release or immutable source snapshot. A moving download URL is not a version.
2. Store a sorted manifest of relative file paths and SHA-256 checksums. Sort paths lexicographically using UTF-8 byte order.
3. Verify every checksum before running any codec. Abort the run on a mismatch; do not silently skip changed files.
4. Preserve empty files and small files unless the protocol explicitly defines a minimum size. Report file count and total input bytes.
5. Do not sample files after seeing codec results. If a subset is necessary, define the deterministic selection rule before the experiment and publish the complete candidate manifest.
6. Keep corpus bytes outside Git unless their licenses explicitly permit repository redistribution.

## Encoded-size accounting

Measure the complete byte sequence required to store and decode each representation:

- Identity: exact original bytes. The known original length is external benchmark context; do not invent a wrapper for identity.
- Pair-RLE: include its complete frame header and payload.
- NARF v3: include its complete frame header, metadata, and selected body, including raw fallback when selected.
- LZ4, Zstandard, and Brotli: include the complete emitted frame or stream and any length prefix required by the selected API.

Record codec names, exact versions, and all compression parameters. Do not compare a payload-only size for one codec with a complete frame for another.

For each file, report input bytes, encoded bytes, and encoded/input ratio. For aggregate corpus size, calculate:

`aggregate_ratio = sum(encoded_bytes) / sum(input_bytes)`

Also report per-file ratios and their median. Do not substitute an unweighted mean of per-file ratios for the aggregate ratio. Report files that expand as well as files that compress.

## Timing and resource measurements

- Verify a full round trip before collecting timing samples.
- Time encode and decode separately; do not include file I/O in codec timing.
- Use identical input bytes and declared codec settings for all codecs.
- Run at least five independent process invocations per machine/configuration. Record raw samples, not only summary statistics.
- Report median and nearest-rank p95/p99 only with sample count. Treat p99 from small samples as descriptive, not a reliable tail estimate.
- Record peak resident memory where a reproducible platform-specific measurement is available. State when it is unavailable.
- Do not compare measurements from unlike hardware as controlled head-to-head performance results.
- Preserve raw CSV/JSON output and metadata for every run.

A separate streaming experiment is required before making streaming-performance claims. A separate energy-measurement protocol is required before making energy claims.

## Required result tables

Publish at minimum:

1. Corpus manifest summary: release, license evidence, file count, total bytes, and checksum manifest digest.
2. Per-file size results for every codec.
3. Aggregate encoded bytes and aggregate ratio for every codec.
4. Encode/decode timing distributions and independent-run counts.
5. Peak memory results or an explicit unavailable marker.
6. Failed files, excluded files, and all deviations from this protocol.

Include negative results and regressions. Do not remove poorly performing files from the published result set.

## Interpretation and limitations

A lower aggregate byte count is evidence only for the pinned corpus and settings. It does not prove better performance on arbitrary data. A timing advantage on one machine is not a cross-platform performance claim. A corpus benchmark does not establish security, novelty, standards compliance, or production readiness.

Independent replication should use the same manifest and settings on separately provisioned hardware. Cross-implementation conformance is a separate requirement from compression benchmarking.
