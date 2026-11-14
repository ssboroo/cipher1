//! Authenticated encryption envelope using the standardized XChaCha20-Poly1305 AEAD.
//!
//! Keys are supplied and managed by the caller. Nonces are generated from the
//! operating system CSPRNG for each seal operation. This is a standard-primitive
//! integration, not a novel CIPHER-1 cipher or a key-management system.

use chacha20poly1305::{
    aead::{Aead, Payload},
    KeyInit, XChaCha20Poly1305, XNonce,
};
use thiserror::Error;
use zeroize::Zeroizing;

const MAGIC: &[u8; 4] = b"C1AE";
const VERSION: u8 = 1;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const HEADER_LEN: usize = 4 + 1 + NONCE_LEN + 8;
const KEY_LEN: usize = 32;
const DEFAULT_MAX_PLAINTEXT: usize = 64 * 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AeadFrameError {
    #[error("input is shorter than the authenticated frame header and tag")]
    Truncated,
    #[error("invalid authenticated frame magic")]
    InvalidMagic,
    #[error("unsupported authenticated frame version: {0}")]
    UnsupportedVersion(u8),
    #[error("declared plaintext exceeds the configured limit")]
    PayloadTooLarge,
    #[error("declared plaintext length does not match the frame length")]
    LengthMismatch,
    #[error("frame authentication failed")]
    AuthenticationFailed,
    #[error("payload could not be encrypted with the selected AEAD limits")]
    EncryptionFailed,
    #[error("operating-system random number generation failed")]
    RandomnessUnavailable,
    #[error("payload length cannot be represented on this platform")]
    LengthNotRepresentable,
}

#[derive(Debug, Clone, Copy)]
pub struct AeadDecodeLimits {
    pub max_plaintext_len: usize,
}

impl Default for AeadDecodeLimits {
    fn default() -> Self {
        Self {
            max_plaintext_len: DEFAULT_MAX_PLAINTEXT,
        }
    }
}

/// Generate a 256-bit key using the operating system cryptographic RNG.
///
/// Store the returned key in a secure key manager; never commit or log it.
pub fn generate_key() -> Result<Zeroizing<[u8; KEY_LEN]>, AeadFrameError> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    getrandom::getrandom(&mut *key).map_err(|_| AeadFrameError::RandomnessUnavailable)?;
    Ok(key)
}

/// Encrypt and authenticate a payload.
///
/// The 24-byte nonce is generated for every call. Associated data is
/// authenticated but not encrypted and must be supplied unchanged to open.
pub fn seal(
    key: &[u8; KEY_LEN],
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, AeadFrameError> {
    let plaintext_len =
        u64::try_from(plaintext.len()).map_err(|_| AeadFrameError::LengthNotRepresentable)?;
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).map_err(|_| AeadFrameError::RandomnessUnavailable)?;

    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    header.extend_from_slice(&nonce);
    header.extend_from_slice(&plaintext_len.to_be_bytes());

    let aad_capacity = HEADER_LEN
        .checked_add(associated_data.len())
        .ok_or(AeadFrameError::LengthNotRepresentable)?;
    let mut authenticated_data = Vec::with_capacity(aad_capacity);
    authenticated_data.extend_from_slice(&header);
    authenticated_data.extend_from_slice(associated_data);

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .expect("the fixed-size key always has the required length");
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &authenticated_data,
            },
        )
        .map_err(|_| AeadFrameError::EncryptionFailed)?;

    let output_capacity = HEADER_LEN
        .checked_add(ciphertext.len())
        .ok_or(AeadFrameError::LengthNotRepresentable)?;
    let mut output = Vec::with_capacity(output_capacity);
    output.extend_from_slice(&header);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Authenticate and decrypt a frame with a caller-controlled output limit.
pub fn open(
    input: &[u8],
    key: &[u8; KEY_LEN],
    associated_data: &[u8],
    limits: AeadDecodeLimits,
) -> Result<Vec<u8>, AeadFrameError> {
    if input.len() < HEADER_LEN + TAG_LEN {
        return Err(AeadFrameError::Truncated);
    }
    if &input[..4] != MAGIC {
        return Err(AeadFrameError::InvalidMagic);
    }
    if input[4] != VERSION {
        return Err(AeadFrameError::UnsupportedVersion(input[4]));
    }

    let mut length_bytes = [0u8; 8];
    length_bytes.copy_from_slice(&input[5 + NONCE_LEN..HEADER_LEN]);
    let declared = u64::from_be_bytes(length_bytes);
    let max_len = u64::try_from(limits.max_plaintext_len).unwrap_or(u64::MAX);
    if declared > max_len {
        return Err(AeadFrameError::PayloadTooLarge);
    }

    let plaintext_len =
        usize::try_from(declared).map_err(|_| AeadFrameError::LengthNotRepresentable)?;
    let expected_total = HEADER_LEN
        .checked_add(plaintext_len)
        .and_then(|length| length.checked_add(TAG_LEN))
        .ok_or(AeadFrameError::LengthNotRepresentable)?;
    if input.len() != expected_total {
        return Err(AeadFrameError::LengthMismatch);
    }

    let aad_capacity = HEADER_LEN
        .checked_add(associated_data.len())
        .ok_or(AeadFrameError::LengthNotRepresentable)?;
    let mut authenticated_data = Vec::with_capacity(aad_capacity);
    authenticated_data.extend_from_slice(&input[..HEADER_LEN]);
    authenticated_data.extend_from_slice(associated_data);

    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .expect("the fixed-size key always has the required length");
    cipher
        .decrypt(
            XNonce::from_slice(&input[5..5 + NONCE_LEN]),
            Payload {
                msg: &input[HEADER_LEN..],
                aad: &authenticated_data,
            },
        )
        .map_err(|_| AeadFrameError::AuthenticationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_generation_uses_fresh_random_material() {
        assert_ne!(*generate_key().unwrap(), *generate_key().unwrap());
    }

    #[test]
    fn round_trips_empty_and_binary_payloads() {
        let key = [0x42; KEY_LEN];
        for payload in [&[][..], &[0, 1, 2, 0, 255, 128, 42][..]] {
            let frame = seal(&key, payload, b"tenant=alpha").unwrap();
            let decoded = open(
                &frame,
                &key,
                b"tenant=alpha",
                AeadDecodeLimits::default(),
            )
            .unwrap();
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn associated_data_and_key_are_authenticated() {
        let key = [0x11; KEY_LEN];
        let wrong_key = [0x12; KEY_LEN];
        let frame = seal(&key, b"secret payload", b"account:17").unwrap();

        assert_eq!(
            open(&frame, &key, b"account:18", AeadDecodeLimits::default()),
            Err(AeadFrameError::AuthenticationFailed)
        );
        assert_eq!(
            open(&frame, &wrong_key, b"account:17", AeadDecodeLimits::default()),
            Err(AeadFrameError::AuthenticationFailed)
        );
    }

    #[test]
    fn nonce_is_fresh_for_repeated_seals() {
        let key = [0x33; KEY_LEN];
        let first = seal(&key, b"same plaintext", b"").unwrap();
        let second = seal(&key, b"same plaintext", b"").unwrap();
        assert_ne!(first, second);
        assert_eq!(
            open(&first, &key, b"", AeadDecodeLimits::default()).unwrap(),
            b"same plaintext"
        );
        assert_eq!(
            open(&second, &key, b"", AeadDecodeLimits::default()).unwrap(),
            b"same plaintext"
        );
    }

    #[test]
    fn rejects_tampering_and_trailing_bytes() {
        let key = [0x77; KEY_LEN];
        let mut frame = seal(&key, b"authenticated", b"context").unwrap();
        *frame.last_mut().unwrap() ^= 1;
        assert_eq!(
            open(&frame, &key, b"context", AeadDecodeLimits::default()),
            Err(AeadFrameError::AuthenticationFailed)
        );

        let mut nonce_tampered = seal(&key, b"authenticated", b"context").unwrap();
        nonce_tampered[5] ^= 1;
        assert_eq!(
            open(&nonce_tampered, &key, b"context", AeadDecodeLimits::default()),
            Err(AeadFrameError::AuthenticationFailed)
        );

        let mut trailing = seal(&key, b"authenticated", b"context").unwrap();
        trailing.push(0);
        assert_eq!(
            open(&trailing, &key, b"context", AeadDecodeLimits::default()),
            Err(AeadFrameError::LengthMismatch)
        );
    }

    #[test]
    fn enforces_plaintext_limit_before_decryption() {
        let key = [0x55; KEY_LEN];
        let frame = seal(&key, b"12345", b"").unwrap();
        assert_eq!(
            open(
                &frame,
                &key,
                b"",
                AeadDecodeLimits {
                    max_plaintext_len: 4,
                },
            ),
            Err(AeadFrameError::PayloadTooLarge)
        );
    }

    #[test]
    fn rejects_invalid_magic_and_version() {
        let key = [0x88; KEY_LEN];
        let frame = seal(&key, b"x", b"").unwrap();

        let mut bad_magic = frame.clone();
        bad_magic[0] ^= 1;
        assert_eq!(
            open(&bad_magic, &key, b"", AeadDecodeLimits::default()),
            Err(AeadFrameError::InvalidMagic)
        );

        let mut bad_version = frame;
        bad_version[4] = 2;
        assert_eq!(
            open(&bad_version, &key, b"", AeadDecodeLimits::default()),
            Err(AeadFrameError::UnsupportedVersion(2))
        );
    }
}
