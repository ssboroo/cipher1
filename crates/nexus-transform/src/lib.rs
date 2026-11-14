//! Experimental NEXUS adaptive residual transform.
//!
//! This crate is a lossless preprocessing experiment. It is NOT encryption,
//! authentication, or a cryptographic primitive. Do not use it as a security
//! boundary. The candidate family and selector require independent research.

use thiserror::Error;

const MAGIC: &[u8; 4] = b"NXAR";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 1 + 1 + 8 + 2;
const CANDIDATES: [u8; 8] = [0, 1, 2, 3, 5, 8, 13, 21];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransformError {
    #[error("input is shorter than the NEXUS header")]
    Truncated,
    #[error("invalid NEXUS magic")]
    InvalidMagic,
    #[error("unsupported NEXUS transform version: {0}")]
    UnsupportedVersion(u8),
    #[error("declared output length exceeds the configured limit")]
    OutputTooLarge,
    #[error("encoded stream has an invalid length")]
    InvalidLength,
    #[error("transform parameters or unused seed bytes are non-canonical")]
    InvalidParameters,
    #[error("run-length stream is malformed")]
    InvalidRunStream,
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_output_len: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_output_len: 64 * 1024 * 1024,
        }
    }
}

/// Encode bytes using the smallest measured representation among 64 reversible
/// second-order modular residual transforms followed by a bounded PackBits-like
/// run/literal coder. The selector includes all fixed header bytes in its cost.
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut best: Option<(usize, u8, u8, Vec<u8>)> = None;

    for &alpha in &CANDIDATES {
        for &beta in &CANDIDATES {
            let residuals = forward_residual(input, alpha, beta);
            let packed = pack_runs(&residuals);
            let total = HEADER_LEN + packed.len();

            let replace = best
                .as_ref()
                .map_or(true, |(best_size, best_a, best_b, _)| {
                    total < *best_size
                        || (total == *best_size && (alpha, beta) < (*best_a, *best_b))
                });

            if replace {
                best = Some((total, alpha, beta, packed));
            }
        }
    }

    let (_, alpha, beta, packed) = best.expect("candidate set is non-empty");
    let len = u64::try_from(input.len()).expect("slice length fits in u64 on supported targets");
    let mut output = Vec::with_capacity(HEADER_LEN + packed.len());
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.push(alpha);
    output.push(beta);
    output.extend_from_slice(&len.to_be_bytes());
    output.push(input.first().copied().unwrap_or(0));
    output.push(input.get(1).copied().unwrap_or(0));
    output.extend_from_slice(&packed);
    output
}

/// Decode an NEXUS residual stream, enforcing a caller-controlled allocation limit.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, TransformError> {
    decode_with_limits(input, DecodeLimits::default())
}

pub fn decode_with_limits(input: &[u8], limits: DecodeLimits) -> Result<Vec<u8>, TransformError> {
    if input.len() < HEADER_LEN {
        return Err(TransformError::Truncated);
    }
    if &input[..4] != MAGIC {
        return Err(TransformError::InvalidMagic);
    }
    if input[4] != VERSION {
        return Err(TransformError::UnsupportedVersion(input[4]));
    }

    let alpha = input[5];
    let beta = input[6];
    if !CANDIDATES.contains(&alpha) || !CANDIDATES.contains(&beta) {
        return Err(TransformError::InvalidParameters);
    }
    let mut len_bytes = [0_u8; 8];
    len_bytes.copy_from_slice(&input[7..15]);
    let declared = u64::from_be_bytes(len_bytes);
    let output_len = usize::try_from(declared).map_err(|_| TransformError::OutputTooLarge)?;
    if output_len > limits.max_output_len {
        return Err(TransformError::OutputTooLarge);
    }

    let seed0 = input[15];
    let seed1 = input[16];
    if (output_len == 0 && (seed0 != 0 || seed1 != 0)) || (output_len == 1 && seed1 != 0) {
        return Err(TransformError::InvalidParameters);
    }
    let residual_count = output_len.saturating_sub(2);
    let residuals = unpack_runs(&input[HEADER_LEN..], residual_count)?;
    let mut output = Vec::with_capacity(output_len);

    if output_len >= 1 {
        output.push(seed0);
    }
    if output_len >= 2 {
        output.push(seed1);
    }
    for residual in residuals {
        let i = output.len();
        let previous = output[i - 1] as u16;
        let previous2 = output[i - 2] as u16;
        let value =
            (residual as u16 + (alpha as u16 * previous) + (beta as u16 * previous2)) & 0xff;
        output.push(value as u8);
    }
    Ok(output)
}

fn forward_residual(input: &[u8], alpha: u8, beta: u8) -> Vec<u8> {
    let mut residuals = Vec::with_capacity(input.len().saturating_sub(2));
    for i in 2..input.len() {
        let prediction =
            (alpha as u16 * input[i - 1] as u16 + beta as u16 * input[i - 2] as u16) & 0xff;
        residuals.push((input[i] as u16).wrapping_sub(prediction) as u8);
    }
    residuals
}

/// Token format: 0..127 means 1..128 literal bytes; 128..255 means
/// 1..128 repetitions of the following byte.
fn pack_runs(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < input.len() {
        let run = run_length(input, i);
        if run >= 3 {
            let count = run.min(128);
            out.push(0x80 | ((count - 1) as u8));
            out.push(input[i]);
            i += count;
        } else {
            let start = i;
            i += run;
            while i < input.len() && i - start < 128 {
                let next_run = run_length(input, i);
                if next_run >= 3 {
                    break;
                }
                let take = next_run.min(128 - (i - start));
                i += take;
            }
            let count = i - start;
            out.push((count - 1) as u8);
            out.extend_from_slice(&input[start..i]);
        }
    }
    out
}

fn run_length(input: &[u8], start: usize) -> usize {
    let mut end = start + 1;
    while end < input.len() && input[end] == input[start] && end - start < 128 {
        end += 1;
    }
    end - start
}

fn unpack_runs(input: &[u8], expected: usize) -> Result<Vec<u8>, TransformError> {
    let mut out = Vec::with_capacity(expected);
    let mut i = 0;
    while i < input.len() {
        let control = input[i];
        i += 1;
        let count = (control as usize & 0x7f) + 1;
        if out.len().checked_add(count).map_or(true, |n| n > expected) {
            return Err(TransformError::InvalidRunStream);
        }
        if control & 0x80 != 0 {
            let value = *input.get(i).ok_or(TransformError::InvalidRunStream)?;
            i += 1;
            out.resize(out.len() + count, value);
        } else {
            let end = i
                .checked_add(count)
                .ok_or(TransformError::InvalidRunStream)?;
            let literal = input.get(i..end).ok_or(TransformError::InvalidRunStream)?;
            out.extend_from_slice(literal);
            i = end;
        }
    }
    if out.len() != expected {
        return Err(TransformError::InvalidRunStream);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_and_short_inputs() {
        for input in [&[][..], &[42][..], &[1, 2][..]] {
            assert_eq!(decode(&encode(input)).unwrap(), input);
        }
    }

    #[test]
    fn round_trip_structured_data() {
        let input: Vec<u8> = (0..4096).map(|i| ((i / 16) % 8) as u8).collect();
        assert_eq!(decode(&encode(&input)).unwrap(), input);
    }

    #[test]
    fn round_trip_all_byte_values() {
        let input: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&input)).unwrap(), input);
    }

    #[test]
    fn round_trip_deterministic_pseudorandom_data() {
        let mut state = 0x1234_5678_u32;
        let input: Vec<u8> = (0..8192)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        assert_eq!(decode(&encode(&input)).unwrap(), input);
    }

    #[test]
    fn rejects_output_over_limit() {
        let encoded = encode(b"too large");
        assert_eq!(
            decode_with_limits(&encoded, DecodeLimits { max_output_len: 2 }),
            Err(TransformError::OutputTooLarge)
        );
    }

    #[test]
    fn rejects_corrupt_run_stream() {
        let mut encoded = encode(b"");
        encoded.push(0x80);
        encoded.push(0);
        assert_eq!(decode(&encoded), Err(TransformError::InvalidRunStream));
    }

    #[test]
    fn selector_never_claims_compression_without_measuring_header() {
        let input = vec![7_u8; 1024];
        let encoded = encode(&input);
        assert!(encoded.len() < input.len());
        assert_eq!(decode(&encoded).unwrap(), input);
    }

    #[test]
    fn round_trip_all_single_byte_values_across_boundary_lengths() {
        for value in 0_u8..=u8::MAX {
            for len in 0..=260 {
                let input = vec![value; len];
                assert_eq!(decode(&encode(&input)).unwrap(), input);
            }
        }
    }

    #[test]
    fn round_trip_deterministic_inputs_for_every_length_through_2048() {
        let mut state = 0x9e37_79b9_u32;
        for len in 0..=2048 {
            let input: Vec<u8> = (0..len)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    state as u8
                })
                .collect();
            assert_eq!(decode(&encode(&input)).unwrap(), input, "length={len}");
        }
    }

    #[test]
    fn every_candidate_pair_is_reversible_on_deterministic_inputs() {
        let mut state = 0x6a09_e667_u32;
        for &alpha in &CANDIDATES {
            for &beta in &CANDIDATES {
                for len in 0..=512 {
                    let input: Vec<u8> = (0..len)
                        .map(|_| {
                            state ^= state << 13;
                            state ^= state >> 17;
                            state ^= state << 5;
                            state as u8
                        })
                        .collect();
                    let residuals = forward_residual(&input, alpha, beta);
                    let mut reconstructed = input.iter().take(2).copied().collect::<Vec<_>>();
                    for residual in residuals {
                        let n = reconstructed.len();
                        let value = (residual as u16
                            + alpha as u16 * reconstructed[n - 1] as u16
                            + beta as u16 * reconstructed[n - 2] as u16)
                            & 0xff;
                        reconstructed.push(value as u8);
                    }
                    assert_eq!(
                        reconstructed, input,
                        "alpha={alpha}, beta={beta}, len={len}"
                    );
                }
            }
        }
    }

    #[test]
    fn rejects_non_candidate_coefficients() {
        let mut encoded = encode(b"parameter validation");
        encoded[5] = 4;
        assert_eq!(decode(&encoded), Err(TransformError::InvalidParameters));
    }

    #[test]
    fn rejects_nonzero_unused_seeds() {
        let mut empty = encode(b"");
        empty[15] = 1;
        assert_eq!(decode(&empty), Err(TransformError::InvalidParameters));

        let mut single = encode(b"x");
        single[16] = 1;
        assert_eq!(decode(&single), Err(TransformError::InvalidParameters));
    }

    #[test]
    fn rejects_every_truncated_header_length() {
        let encoded = encode(b"header truncation");
        for end in 0..HEADER_LEN {
            assert_eq!(decode(&encoded[..end]), Err(TransformError::Truncated));
        }
    }

    #[test]
    fn rejects_trailing_encoded_tokens() {
        let mut encoded = encode(b"");
        encoded.push(0);
        encoded.push(0);
        assert_eq!(decode(&encoded), Err(TransformError::InvalidRunStream));
    }

    #[test]
    fn packer_round_trips_token_boundary_lengths() {
        for len in [1, 2, 3, 127, 128, 129, 255, 256, 257, 511, 512] {
            let literal: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            assert_eq!(unpack_runs(&pack_runs(&literal), len).unwrap(), literal);

            let repeated = vec![0xa5; len];
            assert_eq!(unpack_runs(&pack_runs(&repeated), len).unwrap(), repeated);
        }
    }
}
