# CIPHER-1 Threat Model

## Security objectives

Confidentiality, integrity, key-holder authenticity, replay resistance, side-channel resistance, and resource safety are separate properties and must be evaluated independently.

## Current components

### C1NX version-1 frame

The original frame provides structural validation, a configurable payload limit, and an unkeyed BLAKE3 digest. The digest detects accidental corruption but does not provide confidentiality, sender authentication, replay protection, or protection against an attacker who can replace both payload and digest.

### C1AE version-1 authenticated envelope

The optional envelope uses the standardized XChaCha20-Poly1305 AEAD through a maintained RustCrypto implementation. It uses a 256-bit caller-managed key, a fresh 192-bit nonce from the operating-system CSPRNG, and authenticates the complete envelope header plus caller-supplied associated data. The decoder enforces a plaintext-size limit before decryption.

This provides confidentiality and integrity for a correctly managed shared key, subject to the security of the underlying implementation and the surrounding system. It does not identify an individual sender when multiple parties share a key, prevent replay, provide forward secrecy, or solve key distribution, storage, rotation, revocation, or recovery.

The wrapper is new and has not received an independent security audit. The presence of a standard AEAD does not make the overall project production-ready.

## Out of scope

Custom cryptographic primitives, key derivation/password hashing, key lifecycle and KMS integration, sender identity, replay protection, side-channel resistance, post-quantum security, and secure deletion are not implemented.

## Future adversary model

The research should consider passive observers, active network attackers, malicious storage providers, chosen-plaintext and chosen-ciphertext attackers, nonce misuse, replay, malformed-input attackers, resource-exhaustion attackers, compromised endpoints, and stolen keys.

## Cryptographic policy

Do not introduce a custom production cryptographic primitive without extensive public cryptanalysis and independent review. Prefer established, reviewed algorithms through maintained libraries. Do not reuse a nonce with the same key. Generate keys with a cryptographically secure random source and store them in a dedicated secret manager. Any post-quantum design must identify the exact standardized algorithm and parameter set.

## Production-readiness requirements

- Independent security review of the envelope and its integration
- Public test vectors and at least one independent implementation
- Fuzz and property-based testing
- Differential implementation testing
- Side-channel and key-management review
- Replay and key-rotation protocol design where needed
- Reproducible benchmarks
- Documented vulnerability disclosure process

Until these requirements are met, CIPHER-1 is experimental research software only.
