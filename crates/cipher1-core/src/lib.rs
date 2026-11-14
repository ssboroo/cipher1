//! CIPHER-1 version 1 binary frame codec.
//!
//! This module provides framing and an unkeyed integrity digest. The optional
//! authenticated-encryption envelope is exposed separately through the aead module.

pub mod aead;

use thiserror::Error;

const MAGIC: [u8; 4] = *b"C1NX";
const VERSION: u8 = 1;
const FLAGS: u8 = 0;
const RESERVED: [u8; 2] = [0, 0];
const DIGEST_LEN: usize = 32;
const HEADER_LEN: usize = 4 + 1 + 1 + 2 + 8 + DIGEST_LEN;
const DIGEST_DOMAIN: &[u8] = b"CIPHER-1::FRAME::V1\0";

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_payload_len: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self { max_payload_len: 64 * 1024 * 1024 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub payload: Vec<u8>,
    pub version: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("input is shorter than the CIPHER-1 header")]
    TruncatedHeader,
    #[error("invalid CIPHER-1 magic bytes")]
    InvalidMagic,
    #[error("unsupported frame version: {0}")]
    UnsupportedVersion(u8),
    #[error("unsupported frame flags: {0:#04x}")]
    UnsupportedFlags(u8),
    #[error("reserved header bytes must be zero")]
    NonZeroReserved,
    #[error("payload length {declared} exceeds configured limit {limit}")]
    PayloadTooLarge { declared: u64, limit: usize },
    #[error("declared payload length does not match input length")]
    LengthMismatch,
    #[error("payload integrity verification failed")]
    IntegrityMismatch,
    #[error("payload length cannot be represented on this platform")]
    LengthNotRepresentable,
}

pub fn encode(payload: &[u8]) -> Vec<u8> {
    let payload_len = payload.len() as u64;
    let digest = digest_payload(VERSION, FLAGS, payload_len, payload);

    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(&MAGIC);
    output.push(VERSION);
    output.push(FLAGS);
    output.extend_from_slice(&RESERVED);
    output.extend_from_slice(&payload_len.to_be_bytes());
    output.extend_from_slice(&digest);
    output.extend_from_slice(payload);
    output
}

pub fn decode(input: &[u8]) -> Result<DecodedFrame, FrameError> {
    decode_with_limits(input, DecodeLimits::default())
}

pub fn decode_with_limits(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<DecodedFrame, FrameError> {
    if input.len() < HEADER_LEN {
        return Err(FrameError::TruncatedHeader);
    }
    if input[0..4] != MAGIC {
        return Err(FrameError::InvalidMagic);
    }

    let version = input[4];
    if version != VERSION {
        return Err(FrameError::UnsupportedVersion(version));
    }

    let flags = input[5];
    if flags != FLAGS {
        return Err(FrameError::UnsupportedFlags(flags));
    }
    if input[6..8] != RESERVED {
        return Err(FrameError::NonZeroReserved);
    }

    let mut length_bytes = [0_u8; 8];
    length_bytes.copy_from_slice(&input[8..16]);
    let declared_len = u64::from_be_bytes(length_bytes);

    if declared_len > limits.max_payload_len as u64 {
        return Err(FrameError::PayloadTooLarge {
            declared: declared_len,
            limit: limits.max_payload_len,
        });
    }

    let payload_len =
        usize::try_from(declared_len).map_err(|_| FrameError::LengthNotRepresentable)?;
    let expected_total = HEADER_LEN
        .checked_add(payload_len)
        .ok_or(FrameError::LengthNotRepresentable)?;

    if input.len() != expected_total {
        return Err(FrameError::LengthMismatch);
    }

    let expected_digest = &input[16..HEADER_LEN];
    let payload = &input[HEADER_LEN..];
    let actual_digest = digest_payload(version, flags, declared_len, payload);

    if !constant_time_eq(expected_digest, &actual_digest) {
        return Err(FrameError::IntegrityMismatch);
    }

    Ok(DecodedFrame {
        payload: payload.to_vec(),
        version,
    })
}

fn digest_payload(version: u8, flags: u8, payload_len: u64, payload: &[u8]) -> [u8; DIGEST_LEN] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(DIGEST_DOMAIN);
    hasher.update(&[version, flags]);
    hasher.update(&payload_len.to_be_bytes());
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_payload() {
        let decoded = decode(&encode(&[])).expect("empty frame should decode");
        assert!(decoded.payload.is_empty());
        assert_eq!(decoded.version, VERSION);
    }

    #[test]
    fn round_trip_binary_payload() {
        let payload = [0, 1, 2, 0, 255, 128, 42];
        let decoded = decode(&encode(&payload)).expect("binary frame should decode");
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn encoding_is_deterministic() {
        assert_eq!(encode(b"repeatable input"), encode(b"repeatable input"));
    }

    #[test]
    fn rejects_truncated_header() {
        assert_eq!(decode(b"C1NX"), Err(FrameError::TruncatedHeader));
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut frame = encode(b"payload");
        frame[0] = b'X';
        assert_eq!(decode(&frame), Err(FrameError::InvalidMagic));
    }

    #[test]
    fn rejects_modified_payload() {
        let mut frame = encode(b"payload");
        *frame.last_mut().expect("frame has payload") ^= 1;
        assert_eq!(decode(&frame), Err(FrameError::IntegrityMismatch));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut frame = encode(b"payload");
        frame.push(0);
        assert_eq!(decode(&frame), Err(FrameError::LengthMismatch));
    }

    #[test]
    fn enforces_payload_limit() {
        let result = decode_with_limits(
            &encode(b"payload"),
            DecodeLimits { max_payload_len: 3 },
        );
        assert_eq!(
            result,
            Err(FrameError::PayloadTooLarge { declared: 7, limit: 3 })
        );
    }
}
