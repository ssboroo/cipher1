# NEXUS Adaptive Residual Transform (NARF) — Experimental Specification

## 1. Scope

NARF v0.3 is a lossless adaptive preprocessing codec. It is not an encryption algorithm. The decoder accepts legacy version-1 and version-2 streams.

## 2. Stream format

All integers are unsigned and big-endian unless stated otherwise.

| Offset | Size | Field | Value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `NXAR` |
| 4 | 1 | Version | 3 for new encodings; decoder also accepts versions 1 and 2 |
| 5 | 1 | Alpha | Selected recurrence coefficient, or `0xff` in raw mode |
| 6 | 1 | Beta | Selected recurrence coefficient, or `0xff` in raw mode |
| 7 | 8 | Original length | Unsigned 64-bit |
| 15 | 1 | Seed 0 | First input byte, or zero in raw mode |
| 16 | 1 | Seed 1 | Second input byte, or zero in raw mode |
| 17 | variable | Body | Residual tokens, or original bytes in raw mode |

The fixed header length is 17 bytes for all versions. Versions 2 and 3 support raw mode: it is selected only when the encoded token stream is not shorter than the original input. Raw mode sets both coefficient bytes to `0xff`, both seed bytes to zero, and stores the original bytes verbatim after the header. The decoder rejects a raw body whose length differs from the declared original length. Version 1 does not define raw mode.

## 3. Candidate transform

Candidate coefficients are selected independently from:

`C = [0, 1, 2, 3, 5, 8, 13, 21]`

For each pair (a,b), preserve the first two input bytes and calculate:

`r[i] = (x[i] - a*x[i-1] - b*x[i-2]) mod 256`

All intermediate arithmetic must be widened before reduction modulo 256.

## 4. Token coder

Versions 1 and 2 use the original token grammar: literal controls encode 1..128 bytes; high-bit controls encode 1..128 repeats followed by the repeated byte. Their encoder emits repeat tokens for runs of at least three equal bytes.

Version 3 uses an extended grammar:

- Controls `0x00..0x7f` encode 1..128 literal bytes.
- Controls `0x80..0xfe` encode 1..127 repeated bytes followed by the repeated byte. The encoder uses these for runs of 3..127.
- Control `0xff` introduces an extended repeat: an unsigned LEB128 run length of at least 128, followed by the repeated byte.
- The encoder uses extended repeats for runs of 128 or more, avoiding repeated 128-byte tokens on long constant residual sequences.
- The decoder rejects non-canonical/overflowing LEB128 lengths, extended lengths below 128, truncated tokens, output overflow, and a final decoded residual length that differs from original_length - min(original_length, 2).

## 5. Selection

Evaluate all 64 coefficient pairs. For the transformed representation, cost equals 17 plus the complete token stream length. Select minimum cost; break ties by ascending alpha then ascending beta. Do not select using residual size alone. The current implementation computes the exact token-stream length without allocating candidate token buffers, then encodes only the winning coefficient pair; this is an implementation optimization and does not alter the wire format. Versions 2 and 3 compare the selected token stream length with the original input length. If the token stream is not strictly shorter, encode raw mode instead. Thus encoded size is at most original_length + 17 bytes (subject to representable input length), rather than suffering unbounded token expansion.

## 6. Decoder requirements

- Validate magic and version before allocation.
- Accept versions 1, 2, and 3; raw mode is valid only in versions 2 and 3.
- Parse version-3 extended repeat lengths as canonical unsigned LEB128 values.
- For raw mode, require both coefficients to equal `0xff`, both seed bytes to be zero, and body length to equal the declared output length.
- Parse length with checked conversion to the platform size.
- Enforce caller-supplied maximum output size.
- Check token boundaries and output growth with checked arithmetic.
- Reconstruct using the recurrence in modulo-256 arithmetic.
- Reject malformed or trailing token bytes that do not produce exactly the declared residual count.

## 7. Compatibility

Versions 1, 2, and 3 are experimental and may change before a stable format is declared. Version-3 writers are not readable by version-1- or version-2-only decoders. Version-3 decoders retain read compatibility with valid version-1 and version-2 streams. A future stable format requires a published specification, conformance vectors, fuzzing corpus, and independent implementation.

## 8. Security

The format is public and unkeyed. Do not use it to hide data, authenticate senders, or verify adversarially modified content.
