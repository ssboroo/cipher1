# NEXUS Adaptive Residual Transform (NARF) — Experimental Specification

## 1. Scope

NARF v0.2 is a lossless adaptive preprocessing codec. It is not an encryption algorithm. The decoder also accepts legacy v0.1/version-1 compressed streams.

## 2. Stream format

All integers are unsigned and big-endian unless stated otherwise.

| Offset | Size | Field | Value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `NXAR` |
| 4 | 1 | Version | 2 for new encodings; decoder also accepts version 1 |
| 5 | 1 | Alpha | Selected recurrence coefficient, or `0xff` in raw mode |
| 6 | 1 | Beta | Selected recurrence coefficient, or `0xff` in raw mode |
| 7 | 8 | Original length | Unsigned 64-bit |
| 15 | 1 | Seed 0 | First input byte, or zero in raw mode |
| 16 | 1 | Seed 1 | Second input byte, or zero in raw mode |
| 17 | variable | Body | Residual tokens, or original bytes in raw mode |

The fixed header length is 17 bytes for both versions. Version 2 raw mode is selected only when the token stream is not shorter than the original input. It sets both coefficient bytes to `0xff`, both seed bytes to zero, and stores the original bytes verbatim after the header. The decoder rejects a raw body whose length differs from the declared original length. Version 1 does not define raw mode.

## 3. Candidate transform

Candidate coefficients are selected independently from:

`C = [0, 1, 2, 3, 5, 8, 13, 21]`

For each pair (a,b), preserve the first two input bytes and calculate:

`r[i] = (x[i] - a*x[i-1] - b*x[i-2]) mod 256`

All intermediate arithmetic must be widened before reduction modulo 256.

## 4. Token coder

Each token begins with one control byte.

- If the high bit is zero, the low seven bits encode literal_count - 1, followed by 1..128 literal bytes.
- If the high bit is one, the low seven bits encode repeat_count - 1, followed by one repeated byte.
- The encoder emits a repeat token for runs of at least three equal bytes.
- Literal tokens stop before a run of at least three or at 128 bytes.
- The decoder must reject truncated tokens, output overflow, and a final decoded residual length that differs from original_length - min(original_length, 2).

## 5. Selection

Evaluate all 64 coefficient pairs. For the transformed representation, cost equals 17 plus the complete token stream length. Select minimum cost; break ties by ascending alpha then ascending beta. Do not select using residual size alone. Version 2 compares the selected token stream length with the original input length. If the token stream is not strictly shorter, encode raw mode instead. Thus encoded size is at most original_length + 17 bytes (subject to representable input length), rather than suffering unbounded PackBits expansion.

## 6. Decoder requirements

- Validate magic and version before allocation.
- Accept versions 1 and 2; raw mode is valid only in version 2.
- For raw mode, require both coefficients to equal `0xff`, both seed bytes to be zero, and body length to equal the declared output length.
- Parse length with checked conversion to the platform size.
- Enforce caller-supplied maximum output size.
- Check token boundaries and output growth with checked arithmetic.
- Reconstruct using the recurrence in modulo-256 arithmetic.
- Reject malformed or trailing token bytes that do not produce exactly the declared residual count.

## 7. Compatibility

Versions 1 and 2 are experimental and may change before a stable format is declared. Version 2 writers are not readable by version-1-only decoders. Version-2 decoders retain read compatibility with valid version-1 streams. A future stable format requires a published specification, conformance vectors, fuzzing corpus, and independent implementation.

## 8. Security

The format is public and unkeyed. Do not use it to hide data, authenticate senders, or verify adversarially modified content.
