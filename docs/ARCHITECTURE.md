# CIPHER-1 Architecture

## Scope

CIPHER-1 is a modular data pipeline. The first milestone defines a stable binary envelope. Future stages must be specified independently and composed through explicit, versioned metadata.

## Proposed pipeline

```text
Application Data
      |
      v
Canonical Serialization
      |
      v
Optional Compression
      |
      v
Optional Error-Correction Coding
      |
      v
Authenticated Encryption
      |
      v
CIPHER-1 Framing
      |
      v
Transport or Storage
```

Decoding performs the inverse operations in a strictly validated order.

## Version 1 frame layout

All integer fields use big-endian byte order.

| Offset | Size | Field | Version 1 value |
|---:|---:|---|---|
| 0 | 4 | Magic | C1NX |
| 4 | 1 | Version | 0x01 |
| 5 | 1 | Flags | 0x00 |
| 6 | 2 | Reserved | 0x0000 |
| 8 | 8 | Payload length | Unsigned 64-bit |
| 16 | 32 | Integrity digest | BLAKE3 |
| 48 | N | Payload | Opaque bytes |

The current C1NX version-1 header size is 48 bytes.

## Integrity construction

The digest covers the domain separator, version, flags, payload length, and payload:

```text
"CIPHER-1::FRAME::V1\0"
|| version
|| flags
|| payload_length_be
|| payload
```

This unkeyed digest can detect corruption, but it does not authenticate the sender. An attacker who can replace a frame can recompute the digest.

## C1AE version-1 authenticated envelope

The optional authenticated-encryption module uses XChaCha20-Poly1305. Its wire layout is:

| Offset | Size | Field | Value |
|---:|---:|---|---|
| 0 | 4 | Magic | C1AE |
| 4 | 1 | Version | 0x01 |
| 5 | 24 | Nonce | Fresh OS-CSPRNG output |
| 29 | 8 | Plaintext length | Unsigned 64-bit, big-endian |
| 37 | N + 16 | Ciphertext and tag | XChaCha20-Poly1305 output |

The authenticated associated data is the complete 37-byte header concatenated with caller-supplied associated data. The key is a caller-managed 32-byte value. The decoder checks the declared plaintext limit and exact frame length before decrypting. Key IDs, KDFs, key rotation, sender identity, and replay protection are not part of this format.

This is an integration of an established AEAD primitive, not a new CIPHER-1 cipher. The wrapper has not been independently audited.

## Compatibility rules

- Reject unknown versions and flags.
- Reserved bytes must be zero in version 1.
- Enforce payload limits before allocating memory.
- Require the input length to match the declared payload length exactly.
- Use a new version for incompatible format changes.

## Planned modules

- frame: binary envelope and parsing
- integrity: digest and authentication interfaces
- compression: optional compression adapters
- fec: error detection and forward-error-correction adapters
- crypto: adapters for established AEAD algorithms (C1AE currently uses XChaCha20-Poly1305)
- stream: bounded streaming encoder and decoder
- vectors: public interoperability test vectors
