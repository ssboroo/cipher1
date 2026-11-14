# CIPHER-1 Protected Data Pipeline

This pipeline composes the experimental NARF v3 reversible transform with the
standard C1AE XChaCha20-Poly1305 authenticated envelope. NARF provides
preprocessing only; C1AE provides encryption and authentication.

## Processing order

1. Encode the input with NARF.
2. Encrypt and authenticate the NARF frame with C1AE.
3. On receipt, authenticate and decrypt with C1AE.
4. Decode NARF with a strict maximum output length.

Compression must happen before encryption. Decompression must happen only
after successful AEAD authentication.

## Rust example

```rust
use cipher1_core::aead::{
    generate_key, open, seal, AeadDecodeLimits,
};
use nexus_transform::{decode_with_limits, encode, DecodeLimits};

fn protect(
    message: &[u8],
    key: &[u8; 32],
    context: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let compressed = encode(message);
    let protected = seal(key, &compressed, context)?;
    Ok(protected)
}

fn recover(
    protected: &[u8],
    key: &[u8; 32],
    context: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let compressed = open(
        protected,
        key,
        context,
        AeadDecodeLimits {
            max_plaintext_len: 8 * 1024 * 1024,
        },
    )?;

    let plaintext = decode_with_limits(
        compressed.as_slice(),
        DecodeLimits {
            max_output_len: 64 * 1024 * 1024,
        },
    )?;
    Ok(plaintext)
}

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let key = generate_key()?;
    let context = b"tenant:42;object:backup-2026-11";
    let protected = protect(b"example payload", &key, context)?;
    let recovered = recover(&protected, &key, context)?;
    assert_eq!(recovered, b"example payload");
    Ok(())
}
```

## Security boundaries

- C1AE authenticates its header and caller-supplied context. The context is
  public and must be supplied unchanged when decrypting.
- The C1AE decoder limits the decrypted compressed frame; the NARF decoder
  separately limits decompressed output. Both limits are required.
- This pipeline does not provide sender identity, replay prevention, key
  exchange, key rotation, or post-quantum protection.
- Do not combine secrets and attacker-controlled input in the same compressed
  message when an attacker can observe ciphertext lengths. Compression can
  create length side channels. Use separate frames or disable compression for
  such protocols.
- The C1AE wrapper has not received an independent security audit. NARF is
  experimental and is not a security primitive.
