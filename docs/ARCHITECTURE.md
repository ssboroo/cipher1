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

The current header size is 48 bytes.

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
- crypto: adapters for established AEAD algorithms
- stream: bounded streaming encoder and decoder
- vectors: public interoperability test vectors
