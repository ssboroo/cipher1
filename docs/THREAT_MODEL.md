# CIPHER-1 Threat Model

## Security objectives

Future versions may target integrity, authenticity, confidentiality, resilience, and resource safety. These are separate properties and must be evaluated independently.

## Current guarantees

The initial prototype provides strict structural validation, an unkeyed BLAKE3 digest, configurable payload limits, and rejection of malformed or corrupted frames.

The digest does not provide sender authentication, confidentiality, replay protection, or protection against an attacker who can replace both the payload and digest.

## Out of scope

Encryption, key management, authentication, replay protection, side-channel resistance, post-quantum security, and secure deletion are not implemented.

## Future adversary model

The research should consider passive observers, active network attackers, malicious storage providers, chosen-plaintext and chosen-ciphertext attackers, malformed-input attackers, resource-exhaustion attackers, compromised endpoints, and stolen keys.

## Cryptographic policy

Do not introduce a custom production cryptographic primitive without extensive public cryptanalysis and independent review. Prefer established, reviewed algorithms through maintained libraries. Any post-quantum design must identify the exact standardized algorithm and parameter set.

## Production-readiness requirements

- Independent security review
- Public test vectors
- Fuzz and property-based testing
- Differential implementation testing
- Side-channel and key-management review
- Reproducible benchmarks
- Documented vulnerability disclosure process

Until these requirements are met, CIPHER-1 is experimental research software only.
