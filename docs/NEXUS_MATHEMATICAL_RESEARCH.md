# CIPHER-1 NEXUS Mathematical Research Program

## 1. Research status

This document defines an experimental research direction. It does not claim that the proposed transform family is mathematically novel, cryptographically secure, or superior to established compression methods. A novelty claim requires a systematic literature and patent review. A security claim requires formal analysis and independent cryptanalysis.

## 2. Research objective

Investigate whether a data-dependent selection over reversible finite-ring recurrences can reduce the encoded size of structured data while preserving exact reconstruction, bounded resource use, and deterministic output.

The initial candidate family is called the NEXUS Adaptive Residual Family (NARF).

## 3. Algebraic domain

Let R = Z/256Z. For an input sequence x in R^n, choose coefficients (a,b) from a finite candidate set C x C, where C = {0,1,2,3,5,8,13,21}.

The transform retains x_0 and x_1 as seeds and computes, for i >= 2:

r_i = x_i - a*x_(i-1) - b*x_(i-2) (mod 256).

The inverse recurrence is:

x_i = r_i + a*x_(i-1) + b*x_(i-2) (mod 256).

Because the coefficient of x_i is one, reconstruction is uniquely defined for every a,b in R. The transform is therefore bijective when the two seeds and coefficient pair are retained.

## 3.1 Matrix formulation and invertibility proof

For fixed coefficients a and b, define the linear map T from R^n to R^n by retaining the first two coordinates and replacing each later coordinate with its residual. Relative to the standard basis, T is represented by a lower-triangular n-by-n matrix. Its first two rows are identity rows. For every i >= 2, row i has diagonal entry 1, entry -a in column i-1, entry -b in column i-2, and zero entries elsewhere.

Therefore every diagonal entry is 1 and det(T) = 1 in R = Z/256Z. The element 1 is a unit in this ring, so T is invertible. Equivalently, the inverse can be constructed sequentially because each x_i appears with coefficient 1 and depends only on already reconstructed coordinates.

This establishes bijectivity for each fixed coefficient pair and each finite input length. It does not establish compression efficiency, computational novelty, cryptographic security, or optimality of the coefficient search.

For n < 2, the transform is the identity on the available coordinates; the same conclusion holds.

## 4. Adaptive objective

For each candidate (a,b), compute a residual sequence r(a,b), then apply the specified run/literal coder P. Select:

(a*,b*) = argmin_(a,b in C x C) [H + |P(r(a,b))|],

where H is the fixed header size. Ties are broken lexicographically by (a,b), making encoding deterministic.

This objective is a local empirical cost function, not an entropy bound. It does not imply that the method beats general-purpose compressors.

## 5. Claims to test

- Exact invertibility for every byte sequence and every candidate pair.
- Deterministic output across supported architectures.
- No decompression allocation above a caller-specified limit.
- Compression gains on selected structured datasets.
- Bounded overhead and graceful expansion on incompressible data.
- Runtime and memory costs versus identity, RLE, LZ4, Zstandard, and Brotli baselines.

## 6. Mathematical work still required

1. Prove the recurrence is a bijection over R^n with fixed seeds and coefficients.
2. Characterize residual distributions for constant, affine, periodic, Markov-like, and adversarial inputs.
3. Determine whether the candidate search has useful expected gains after including selector and framing overhead.
4. Analyze worst-case expansion and establish a hard output bound.
5. Study blockwise coefficient selection, change-point detection, and streaming adaptation.
6. Explore generalization to Z/(2^w)Z and vector-valued finite modules.
7. Search the literature for equivalent linear prediction, finite-ring transforms, adaptive residual coding, and related patents before asserting novelty.

## 7. Cryptography boundary

NARF is a public, deterministic, reversible preprocessing transform. It has no secret key and provides no confidentiality, authentication, integrity, or resistance to chosen-plaintext analysis. It must never be treated as encryption or as a cryptographic primitive.

Any future cryptographic layer must begin with a separate threat model, use independently reviewed primitives during the interoperability phase, and undergo formal proof attempts and external cryptanalysis before any custom primitive is considered for deployment.

## 8. Falsification criteria

The research hypothesis should be rejected or narrowed if:
- gains disappear when headers and metadata are counted;
- performance is consistently inferior to established compressors on the intended workload;
- the search cost exceeds the value of the gains;
- decoding cannot be made robust against malformed inputs;
- prior art demonstrates that the claimed novelty is not novel.

## 9. Reproducibility

Every experiment must record dataset provenance, input sizes, compiler version, target architecture, CPU model, build flags, random seeds, repetitions, median and tail latency, throughput, peak memory, and exact output sizes. Raw results and scripts must be committed.
