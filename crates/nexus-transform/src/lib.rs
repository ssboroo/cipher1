//! Experimental NEXUS adaptive residual transform.
//!
//! This crate is a lossless preprocessing experiment. It is NOT encryption,
//! authentication, or a cryptographic primitive. Do not use it as a security
//! boundary. The candidate family and selector require independent research.

use thiserror::Error;

const MAGIC: &[u8; 4] = b"NXAR";
const VERSION: u8 = 3;
const LEGACY_VERSION: u8 = 1;
const LEGACY_VERSION_2: u8 = 2;
const RAW_MODE: u8 = 0xff;
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
    #[error("memory reservation failed while decoding")]
    AllocationFailed,
    #[error("run-length stream is malformed")]
    InvalidRunStream,
    #[error("version-3 token stream is not canonical")]
    NonCanonicalEncoding,
}

#[derive(Debug, Clone, Copy)]
pub struct DecodeLimits {
    pub max_output_len: usize,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self { max_output_len: 64 * 1024 * 1024 }
    }
}

/// Encode bytes using the smallest measured representation among 64 reversible
/// second-order modular residual transforms followed by a bounded PackBits-like
/// run/literal coder. The selector includes all fixed header bytes in its cost.
pub fn encode(input: &[u8]) -> Vec<u8> {
    // Score candidates without materializing 64 token buffers. Only the
    // selected coefficient pair is encoded into an output vector.
    let mut best: Option<(usize, u8, u8)> = None;

    for &alpha in &CANDIDATES {
        for &beta in &CANDIDATES {
            let total = HEADER_LEN + residual_token_len(input, alpha, beta);
            let replace = best.as_ref().is_none_or(|(best_size, best_a, best_b)| {
                total < *best_size
                    || (total == *best_size && (alpha, beta) < (*best_a, *best_b))
            });

            if replace {
                best = Some((total, alpha, beta));
            }
        }
    }

    let (_, selected_alpha, selected_beta) =
        best.expect("candidate set is non-empty");
    let packed = pack_residuals_v3(input, selected_alpha, selected_beta);
    debug_assert_eq!(
        packed.len(),
        residual_token_len(input, selected_alpha, selected_beta)
    );
    // Versions 2 and 3 use an explicit raw mode whenever token coding would not
    // reduce the payload. This bounds expansion to the fixed 17-byte header.
    let use_raw = packed.len() >= input.len();
    let (alpha, beta, seed0, seed1, body): (u8, u8, u8, u8, &[u8]) = if use_raw {
        (RAW_MODE, RAW_MODE, 0, 0, input)
    } else {
        (
            selected_alpha,
            selected_beta,
            input.first().copied().unwrap_or(0),
            input.get(1).copied().unwrap_or(0),
            &packed,
        )
    };

    let len = u64::try_from(input.len()).expect("slice length fits in u64 on supported targets");
    let mut output = Vec::with_capacity(HEADER_LEN + body.len());
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.push(alpha);
    output.push(beta);
    output.extend_from_slice(&len.to_be_bytes());
    output.push(seed0);
    output.push(seed1);
    output.extend_from_slice(body);
    output
}

fn try_vec_with_capacity(capacity: usize) -> Result<Vec<u8>, TransformError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| TransformError::AllocationFailed)?;
    Ok(output)
}

/// Decode an NEXUS residual stream, enforcing a caller-controlled allocation limit.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, TransformError> {
    decode_with_limits(input, DecodeLimits::default())
}

pub fn decode_with_limits(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<Vec<u8>, TransformError> {
    if input.len() < HEADER_LEN {
        return Err(TransformError::Truncated);
    }
    if &input[..4] != MAGIC {
        return Err(TransformError::InvalidMagic);
    }
    let version = input[4];
    if version != VERSION && version != LEGACY_VERSION && version != LEGACY_VERSION_2 {
        return Err(TransformError::UnsupportedVersion(version));
    }

    let alpha = input[5];
    let beta = input[6];
    let mut len_bytes = [0_u8; 8];
    len_bytes.copy_from_slice(&input[7..15]);
    let declared = u64::from_be_bytes(len_bytes);
    let output_len = usize::try_from(declared).map_err(|_| TransformError::OutputTooLarge)?;
    if output_len > limits.max_output_len {
        return Err(TransformError::OutputTooLarge);
    }

    let seed0 = input[15];
    let seed1 = input[16];
    if alpha == RAW_MODE && beta == RAW_MODE {
        if version == LEGACY_VERSION || seed0 != 0 || seed1 != 0 {
            return Err(TransformError::InvalidParameters);
        }
        let raw = &input[HEADER_LEN..];
        if raw.len() != output_len {
            return Err(TransformError::InvalidLength);
        }
        let mut output = try_vec_with_capacity(output_len)?;
        output.extend_from_slice(raw);
        return Ok(output);
    }
    if alpha == RAW_MODE || beta == RAW_MODE
        || !CANDIDATES.contains(&alpha)
        || !CANDIDATES.contains(&beta)
    {
        return Err(TransformError::InvalidParameters);
    }
    if (output_len == 0 && (seed0 != 0 || seed1 != 0))
        || (output_len == 1 && seed1 != 0)
    {
        return Err(TransformError::InvalidParameters);
    }
    let residual_count = output_len.saturating_sub(2);
    let body = &input[HEADER_LEN..];
    let residuals = if version == VERSION {
        unpack_runs_v3(body, residual_count)?
    } else {
        unpack_runs(body, residual_count)?
    };
    if version == VERSION && !is_canonical_v3(body, &residuals) {
        return Err(TransformError::NonCanonicalEncoding);
    }
    let mut output = try_vec_with_capacity(output_len)?;

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
        let value = (residual as u16
            + (alpha as u16 * previous)
            + (beta as u16 * previous2)) & 0xff;
        output.push(value as u8);
    }
    Ok(output)
}

#[cfg(test)]
fn forward_residual(input: &[u8], alpha: u8, beta: u8) -> Vec<u8> {
    let mut residuals = Vec::with_capacity(input.len().saturating_sub(2));
    for i in 2..input.len() {
        let prediction = (alpha as u16 * input[i - 1] as u16
            + beta as u16 * input[i - 2] as u16) & 0xff;
        residuals.push((input[i] as u16).wrapping_sub(prediction) as u8);
    }
    residuals
}

/// Token format: 0..127 means 1..128 literal bytes; 128..255 means
/// 1..128 repetitions of the following byte.
#[cfg(test)]
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


/// Encode residuals directly from the source bytes. This avoids allocating a
/// full residual buffer for each of the 64 candidate coefficient pairs.
fn pack_residuals_v3(input: &[u8], alpha: u8, beta: u8) -> Vec<u8> {
    let residual_count = input.len().saturating_sub(2);
    let mut out = Vec::with_capacity(residual_count);
    let mut i = 0usize;

    while i < residual_count {
        let value = residual_at(input, alpha, beta, i);
        let mut run = 1usize;
        while i + run < residual_count
            && residual_at(input, alpha, beta, i + run) == value
        {
            run += 1;
        }

        if run >= 128 {
            out.push(0xff);
            write_uleb128(run, &mut out);
            out.push(value);
            i += run;
        } else if run >= 3 {
            out.push(0x80 | ((run - 1) as u8));
            out.push(value);
            i += run;
        } else {
            let start = i;
            let mut literals = [0u8; 128];
            while i < residual_count && i - start < literals.len() {
                let next_value = residual_at(input, alpha, beta, i);
                let mut next_run = 1usize;
                while next_run < 3
                    && i + next_run < residual_count
                    && residual_at(input, alpha, beta, i + next_run) == next_value
                {
                    next_run += 1;
                }
                if next_run >= 3 {
                    break;
                }
                let take = next_run.min(literals.len() - (i - start));
                for offset in 0..take {
                    literals[i - start + offset] =
                        residual_at(input, alpha, beta, i + offset);
                }
                i += take;
            }
            let count = i - start;
            out.push((count - 1) as u8);
            out.extend_from_slice(&literals[..count]);
        }
    }
    out
}

/// Return the exact v3 token-stream length without allocating or emitting
/// token bytes. This scores all candidates before encoding only the winner.
fn residual_token_len(input: &[u8], alpha: u8, beta: u8) -> usize {
    let residual_count = input.len().saturating_sub(2);
    let mut total = 0usize;
    let mut i = 0usize;

    while i < residual_count {
        let value = residual_at(input, alpha, beta, i);
        let mut run = 1usize;
        while i + run < residual_count
            && residual_at(input, alpha, beta, i + run) == value
        {
            run += 1;
        }

        if run >= 128 {
            total += 2 + uleb128_len(run);
            i += run;
        } else if run >= 3 {
            total += 2;
            i += run;
        } else {
            let start = i;
            while i < residual_count && i - start < 128 {
                let next_value = residual_at(input, alpha, beta, i);
                let mut next_run = 1usize;
                while next_run < 3
                    && i + next_run < residual_count
                    && residual_at(input, alpha, beta, i + next_run) == next_value
                {
                    next_run += 1;
                }
                if next_run >= 3 {
                    break;
                }
                i += next_run.min(128 - (i - start));
            }
            total += 1 + (i - start);
        }
    }
    total
}

fn uleb128_len(mut value: usize) -> usize {
    let mut length = 1usize;
    while value >= 0x80 {
        value >>= 7;
        length += 1;
    }
    length
}

fn residual_at(input: &[u8], alpha: u8, beta: u8, residual_index: usize) -> u8 {
    let i = residual_index + 2;
    let prediction = (alpha as u16 * input[i - 1] as u16
        + beta as u16 * input[i - 2] as u16)
        & 0xff;
    (input[i] as u16).wrapping_sub(prediction) as u8
}

fn write_uleb128(mut value: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn read_uleb128(input: &[u8], start: usize) -> Result<(usize, usize), TransformError> {
    let mut value = 0usize;
    let mut shift = 0u32;
    for offset in 0..10usize {
        let byte = *input
            .get(start.checked_add(offset).ok_or(TransformError::InvalidRunStream)?)
            .ok_or(TransformError::InvalidRunStream)?;
        let part = (byte & 0x7f) as usize;
        if shift >= usize::BITS || part > (usize::MAX >> shift) {
            return Err(TransformError::InvalidRunStream);
        }
        value |= part << shift;
        if byte & 0x80 == 0 {
            if offset > 0 && part == 0 {
                return Err(TransformError::InvalidRunStream);
            }
            return Ok((value, offset + 1));
        }
        shift = shift
            .checked_add(7)
            .ok_or(TransformError::InvalidRunStream)?;
    }
    Err(TransformError::InvalidRunStream)
}

fn unpack_runs_v3(input: &[u8], expected: usize) -> Result<Vec<u8>, TransformError> {
    let mut out = try_vec_with_capacity(expected)?;
    let mut i = 0usize;
    while i < input.len() {
        let control = input[i];
        i += 1;
        if control == 0xff {
            let (count, consumed) = read_uleb128(input, i)?;
            if count < 128 {
                return Err(TransformError::InvalidRunStream);
            }
            i = i
                .checked_add(consumed)
                .ok_or(TransformError::InvalidRunStream)?;
            if out.len().checked_add(count).is_none_or(|n| n > expected) {
                return Err(TransformError::InvalidRunStream);
            }
            let value = *input.get(i).ok_or(TransformError::InvalidRunStream)?;
            i += 1;
            out.resize(out.len() + count, value);
        } else if control & 0x80 != 0 {
            let count = (control as usize & 0x7f) + 1;
            if count < 3 || out.len().checked_add(count).is_none_or(|n| n > expected) {
                return Err(TransformError::InvalidRunStream);
            }
            let value = *input.get(i).ok_or(TransformError::InvalidRunStream)?;
            i += 1;
            out.resize(out.len() + count, value);
        } else {
            let count = (control as usize & 0x7f) + 1;
            if out.len().checked_add(count).is_none_or(|n| n > expected) {
                return Err(TransformError::InvalidRunStream);
            }
            let end = i.checked_add(count).ok_or(TransformError::InvalidRunStream)?;
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

/// Check the deterministic version-3 token segmentation without allocating.
///
/// Canonicality is evaluated for the residual sequence selected by the frame's
/// alpha and beta values. It does not verify that those coefficients are the
/// globally optimal pair chosen by the encoder.
fn is_canonical_v3(encoded: &[u8], residuals: &[u8]) -> bool {
    let mut encoded_index = 0usize;
    let mut residual_index = 0usize;

    while residual_index < residuals.len() {
        let value = residuals[residual_index];
        let mut run = 1usize;
        while residual_index + run < residuals.len()
            && residuals[residual_index + run] == value
        {
            run += 1;
        }

        if run >= 128 {
            if !consume_expected_byte(encoded, &mut encoded_index, 0xff) {
                return false;
            }
            let mut remaining = run;
            loop {
                let mut byte = (remaining & 0x7f) as u8;
                remaining >>= 7;
                if remaining != 0 {
                    byte |= 0x80;
                }
                if !consume_expected_byte(encoded, &mut encoded_index, byte) {
                    return false;
                }
                if remaining == 0 {
                    break;
                }
            }
            if !consume_expected_byte(encoded, &mut encoded_index, value) {
                return false;
            }
            residual_index += run;
        } else if run >= 3 {
            let control = 0x80 | ((run - 1) as u8);
            if !consume_expected_byte(encoded, &mut encoded_index, control)
                || !consume_expected_byte(encoded, &mut encoded_index, value)
            {
                return false;
            }
            residual_index += run;
        } else {
            let start = residual_index;
            while residual_index < residuals.len() && residual_index - start < 128 {
                let next_value = residuals[residual_index];
                let mut next_run = 1usize;
                while next_run < 3
                    && residual_index + next_run < residuals.len()
                    && residuals[residual_index + next_run] == next_value
                {
                    next_run += 1;
                }
                if next_run >= 3 {
                    break;
                }
                residual_index += next_run.min(128 - (residual_index - start));
            }

            let count = residual_index - start;
            if count == 0
                || !consume_expected_byte(encoded, &mut encoded_index, (count - 1) as u8)
            {
                return false;
            }
            let Some(end) = encoded_index.checked_add(count) else {
                return false;
            };
            let Some(actual) = encoded.get(encoded_index..end) else {
                return false;
            };
            if actual != &residuals[start..residual_index] {
                return false;
            }
            encoded_index = end;
        }
    }

    encoded_index == encoded.len()
}

fn consume_expected_byte(encoded: &[u8], cursor: &mut usize, expected: u8) -> bool {
    if encoded.get(*cursor).copied() != Some(expected) {
        return false;
    }
    *cursor += 1;
    true
}

#[cfg(test)]
fn run_length(input: &[u8], start: usize) -> usize {
    let mut end = start + 1;
    while end < input.len() && input[end] == input[start] && end - start < 128 {
        end += 1;
    }
    end - start
}

fn unpack_runs(input: &[u8], expected: usize) -> Result<Vec<u8>, TransformError> {
    let mut out = try_vec_with_capacity(expected)?;
    let mut i = 0;
    while i < input.len() {
        let control = input[i];
        i += 1;
        let count = (control as usize & 0x7f) + 1;
        if out.len().checked_add(count).is_none_or(|n| n > expected) {
            return Err(TransformError::InvalidRunStream);
        }
        if control & 0x80 != 0 {
            let value = *input.get(i).ok_or(TransformError::InvalidRunStream)?;
            i += 1;
            out.resize(out.len() + count, value);
        } else {
            let end = i.checked_add(count).ok_or(TransformError::InvalidRunStream)?;
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

    fn frame_with_tokens(version: u8, residual_count: usize, tokens: &[u8]) -> Vec<u8> {
        let original_len = residual_count + 2;
        let mut frame = Vec::with_capacity(HEADER_LEN + tokens.len());
        frame.extend_from_slice(MAGIC);
        frame.push(version);
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&(original_len as u64).to_be_bytes());
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(tokens);
        frame
    }

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
        let input: Vec<u8> = (0..8192).map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        }).collect();
        assert_eq!(decode(&encode(&input)).unwrap(), input);
    }

    #[test]
    fn decoder_reports_failed_reservations_instead_of_panicking() {
        assert_eq!(
            try_vec_with_capacity(usize::MAX),
            Err(TransformError::AllocationFailed)
        );

        let mut frame = Vec::new();
        frame.extend_from_slice(MAGIC);
        frame.push(VERSION);
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&u64::MAX.to_be_bytes());
        frame.extend_from_slice(&[0, 0]);

        let result = decode_with_limits(
            &frame,
            DecodeLimits {
                max_output_len: usize::MAX,
            },
        );
        if usize::BITS < 64 {
            assert_eq!(result, Err(TransformError::OutputTooLarge));
        } else {
            assert_eq!(result, Err(TransformError::AllocationFailed));
        }
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
        encoded[5] = 0;
        encoded[6] = 0;
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
                    let packed = pack_residuals_v3(&input, alpha, beta);
                    assert_eq!(
                        residual_token_len(&input, alpha, beta),
                        packed.len(),
                        "token size mismatch for alpha={alpha}, beta={beta}, len={len}"
                    );
                    let mut reconstructed = input.iter().take(2).copied().collect::<Vec<_>>();
                    for residual in residuals {
                        let n = reconstructed.len();
                        let value = (residual as u16
                            + alpha as u16 * reconstructed[n - 1] as u16
                            + beta as u16 * reconstructed[n - 2] as u16)
                            & 0xff;
                        reconstructed.push(value as u8);
                    }
                    assert_eq!(reconstructed, input, "alpha={alpha}, beta={beta}, len={len}");
                }
            }
        }
    }


    #[test]
    fn incompressible_input_uses_raw_mode_without_token_expansion() {
        let mut state = 0x243f_6a88_u32;
        let mut saw_raw_mode = false;
        for len in 0..=4096 {
            let input: Vec<u8> = (0..len)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    state as u8
                })
                .collect();
            let encoded = encode(&input);
            assert!(encoded.len() <= HEADER_LEN + input.len(), "length={len}");
            assert_eq!(decode(&encoded).unwrap(), input);
            if encoded[5] == RAW_MODE {
                saw_raw_mode = true;
                assert_eq!(&encoded[5..7], &[RAW_MODE, RAW_MODE]);
                assert_eq!(&encoded[15..17], &[0, 0]);
                assert_eq!(&encoded[HEADER_LEN..], input.as_slice());
            }
        }
        assert!(saw_raw_mode, "deterministic incompressible corpus should exercise raw mode");
    }

    #[test]
    fn decoder_accepts_legacy_version_one_and_two_streams() {
        let input = vec![0x5a; 512];
        let residuals = forward_residual(&input, 0, 0);
        let old_tokens = pack_runs(&residuals);
        let mut legacy_v1 = Vec::with_capacity(HEADER_LEN + old_tokens.len());
        legacy_v1.extend_from_slice(MAGIC);
        legacy_v1.push(LEGACY_VERSION);
        legacy_v1.extend_from_slice(&[0, 0]);
        legacy_v1.extend_from_slice(&(input.len() as u64).to_be_bytes());
        legacy_v1.extend_from_slice(&[input[0], input[1]]);
        legacy_v1.extend_from_slice(&old_tokens);
        assert_eq!(decode(&legacy_v1).unwrap(), input);

        let mut legacy_v2 = legacy_v1.clone();
        legacy_v2[4] = LEGACY_VERSION_2;
        assert_eq!(decode(&legacy_v2).unwrap(), input);
    }

    #[test]
    fn token_size_estimator_matches_extended_run_boundaries() {
        for len in 0..=512 {
            let mut input = vec![0, 0];
            input.extend(std::iter::repeat_n(0x5a, len));
            let packed = pack_residuals_v3(&input, 0, 0);
            assert_eq!(residual_token_len(&input, 0, 0), packed.len(), "length={len}");
        }
    }

    #[test]
    fn version_three_encodes_long_runs_compactly() {
        for input in [vec![0x41; 16_384], (0..16_384).map(|i| (i % 2) as u8).collect()] {
            let encoded = encode(&input);
            assert_eq!(encoded[4], VERSION);
            assert_eq!(decode(&encoded).unwrap(), input);
            assert!(encoded.len() < 64, "encoded size was {}", encoded.len());
        }
    }

    #[test]
    fn version_three_rejects_malformed_extended_runs() {
        let mut frame = Vec::new();
        frame.extend_from_slice(MAGIC);
        frame.push(VERSION);
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&129_u64.to_be_bytes());
        frame.extend_from_slice(&[0, 0]);
        frame.extend_from_slice(&[0xff, 0x81, 0x01, 0x41]);
        assert_eq!(decode(&frame), Err(TransformError::InvalidRunStream));

        let mut short_extended = frame.clone();
        short_extended[7..15].copy_from_slice(&127_u64.to_be_bytes());
        short_extended[18] = 0x7f;
        short_extended[19] = 0x41;
        assert_eq!(decode(&short_extended), Err(TransformError::InvalidRunStream));

        let mut overlong = frame;
        overlong[18] = 0x81;
        overlong[19] = 0x00;
        assert_eq!(decode(&overlong), Err(TransformError::InvalidRunStream));
    }

    #[test]
    fn version_three_accepts_canonical_literal_and_repeat_tokens() {
        let literal = frame_with_tokens(VERSION, 4, &[0x03, 1, 2, 3, 4]);
        assert_eq!(decode(&literal).unwrap(), [0, 0, 1, 2, 3, 4]);

        let short_run = frame_with_tokens(VERSION, 3, &[0x82, 0x41]);
        assert_eq!(decode(&short_run).unwrap(), [0, 0, 0x41, 0x41, 0x41]);

        let long_run = frame_with_tokens(VERSION, 130, &[0xff, 0x82, 0x01, 0x41]);
        assert_eq!(decode(&long_run).unwrap(), [vec![0, 0], vec![0x41; 130]].concat());
    }

    #[test]
    fn version_three_rejects_noncanonical_token_segmentations() {
        let cases = [
            (
                vec![1, 2, 3, 4],
                vec![0x01, 1, 2, 0x01, 3, 4],
            ),
            (
                vec![0x41; 3],
                vec![0x02, 0x41, 0x41, 0x41],
            ),
            (
                vec![0x41; 130],
                vec![0xfe, 0x41, 0x82, 0x41],
            ),
            (
                vec![0x10, 0x41, 0x41, 0x41, 0x41],
                vec![0x01, 0x10, 0x41, 0x82, 0x41],
            ),
            (
                vec![0x41; 131],
                vec![0xff, 0x80, 0x01, 0x41, 0x82, 0x41],
            ),
        ];

        for (residuals, tokens) in cases {
            let frame = frame_with_tokens(VERSION, residuals.len(), &tokens);
            assert_eq!(
                decode(&frame),
                Err(TransformError::NonCanonicalEncoding),
                "tokens={tokens:?}"
            );
        }
    }

    #[test]
    fn legacy_versions_keep_accepting_noncanonical_literal_segmentation() {
        let tokens = [0x01, 1, 2, 0x01, 3, 4];
        for version in [LEGACY_VERSION, LEGACY_VERSION_2] {
            let frame = frame_with_tokens(version, 4, &tokens);
            assert_eq!(decode(&frame).unwrap(), [0, 0, 1, 2, 3, 4]);
        }
    }

    #[test]
    fn raw_mode_rejects_length_mismatch_and_noncanonical_seeds() {
        let input: Vec<u8> = (0_u16..256)
            .map(|i| (i.wrapping_mul(73).wrapping_add(i.wrapping_mul(i).wrapping_mul(19))) as u8)
            .collect();
        let mut encoded = Vec::with_capacity(HEADER_LEN + input.len());
        encoded.extend_from_slice(MAGIC);
        encoded.push(VERSION);
        encoded.extend_from_slice(&[RAW_MODE, RAW_MODE]);
        encoded.extend_from_slice(&(input.len() as u64).to_be_bytes());
        encoded.extend_from_slice(&[0, 0]);
        encoded.extend_from_slice(&input);
        assert_eq!(decode(&encoded).unwrap(), input);

        let mut trailing_payload = encoded.clone();
        trailing_payload.push(0);
        assert_eq!(
            decode(&trailing_payload),
            Err(TransformError::InvalidLength)
        );

        let mut bad_seed = encoded.clone();
        bad_seed[15] = 1;
        assert_eq!(decode(&bad_seed), Err(TransformError::InvalidParameters));

        let mut partial_marker = encoded.clone();
        partial_marker[6] = 0;
        assert_eq!(decode(&partial_marker), Err(TransformError::InvalidParameters));

        let mut legacy_raw = encoded;
        legacy_raw[4] = LEGACY_VERSION_2;
        assert_eq!(decode(&legacy_raw).unwrap(), input);

        let mut forbidden_v1_raw = legacy_raw;
        forbidden_v1_raw[4] = LEGACY_VERSION;
        assert_eq!(
            decode(&forbidden_v1_raw),
            Err(TransformError::InvalidParameters)
        );
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
        encoded[5] = 0;
        encoded[6] = 0;
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
