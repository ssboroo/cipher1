# C1AE Authenticated Encryption Usage

C1AE version 1 is an optional authenticated envelope implemented with the
standard XChaCha20-Poly1305 AEAD. It is not a custom cipher.

## Example

```rust
use cipher1_core::aead::{generate_key, open, seal, AeadDecodeLimits};

fn example() -> Result<(), Box<dyn std::error::Error>> {
    // Generate once, then store and distribute the key using a secure
    // key-management system. Do not generate a new key for every message.
    let key = generate_key()?;

    // Associated data is public but authenticated. The receiver must supply
    // the exact same context when opening the frame.
    let context = b"tenant:42;purpose:backup";
    let frame = seal(&key, b"example plaintext", context)?;

    let plaintext = open(
        &frame,
        &key,
        context,
        AeadDecodeLimits::default(),
    )?;
    assert_eq!(plaintext.as_slice(), b"example plaintext");
    Ok(())
}
```

## Wire and operational properties

- 32-byte caller-managed key.
- Fresh 24-byte nonce generated from the operating-system CSPRNG per call.
- 37-byte authenticated header followed by ciphertext and a 16-byte tag.
- Total framing overhead is 53 bytes per frame, excluding caller-supplied
  associated data (which is authenticated but not stored in the frame).
- AAD binds external context such as a tenant ID, object type, or protocol
  version. It is not encrypted.
- The decoder checks declared plaintext length and exact frame size before
  decryption.

## Security limitations

- Keep keys in a secret manager or platform key store. Never commit, log, or
  embed production keys in binaries. The zeroizing wrappers clear their own
  buffers on drop, but copies or serialized forms are not automatically cleared.
- The API does not implement password-based key derivation, key exchange,
  rotation, revocation, recovery, or sender identity.
- All holders of the same symmetric key can create valid frames. AEAD alone
  does not prevent replay; applications must bind sequence numbers or unique
  object versions into authenticated context and enforce freshness separately.
- Do not reuse a key across unrelated protocols without a key-separation plan.
- This wrapper has not received an independent security audit. Passing tests
  and fuzzing does not establish production readiness.
- NARF preprocessing is not encryption and must not be treated as a security
  layer.
