#![no_main]

use libfuzzer_sys::fuzz_target;
use nexus_transform::{decode_with_limits, encode, DecodeLimits};

const HEADER_LEN: usize = 17;

fn mutate_region(frame: &mut [u8], operations: &[u8], start: usize, end: usize) {
    let end = end.min(frame.len());
    if start >= end {
        return;
    }

    let region_len = end - start;
    for operation in operations.chunks_exact(2) {
        let offset = start + usize::from(operation[0]) % region_len;
        frame[offset] ^= operation[1];
    }
}

fn structured_payload(value: u8, repeat_len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2 * repeat_len + 132);
    payload.resize(repeat_len, value);
    payload.extend_from_slice(&[value.wrapping_add(1); 4]);
    for index in 0..128 {
        payload.push(if index % 2 == 0 { 0x55 } else { 0xaa });
    }
    payload.resize(payload.len() + repeat_len, value.wrapping_add(2));
    payload
}

fuzz_target!(|data: &[u8]| {
    let limits = DecodeLimits {
        max_output_len: 1024 * 1024,
    };

    if data.len() <= 64 * 1024 {
        let encoded = encode(data);
        let decoded = decode_with_limits(&encoded, limits)
            .expect("the encoder must produce a valid stream");
        assert_eq!(decoded, data);
    }

    if let Ok(decoded) = decode_with_limits(data, limits) {
        let reencoded = encode(&decoded);
        let decoded_again = decode_with_limits(&reencoded, limits)
            .expect("the encoder must produce a valid stream");
        assert_eq!(decoded_again, decoded);
    }

    // Start from a valid, non-raw v3 frame so mutations reach token parsing
    // and canonicality checks instead of failing only at the magic header.
    let value = data.first().copied().unwrap_or(0x42);
    let repeat_len = 130 + usize::from(data.get(1).copied().unwrap_or(0)) % 126;
    let payload = structured_payload(value, repeat_len);
    let valid_frame = encode(&payload);
    assert_eq!(valid_frame[4], 3);
    assert_ne!(&valid_frame[5..7], &[0xff, 0xff]);
    assert!(valid_frame.len() > HEADER_LEN);

    let operations = data.get(2..).unwrap_or(&[]);

    // Preserve the header while mutating token bytes. The seed includes
    // extended repeats, a short repeat, and a literal region.
    let mut body_mutated = valid_frame.clone();
    let body_end = body_mutated.len();
    mutate_region(&mut body_mutated, operations, HEADER_LEN, body_end);
    let _ = decode_with_limits(&body_mutated, limits);

    // Mutate version, coefficients, declared length, and seed fields while
    // preserving the magic so header validation and bounded allocation paths
    // are exercised independently of arbitrary-input decoding.
    let mut header_mutated = valid_frame;
    mutate_region(&mut header_mutated, operations, 4, HEADER_LEN);
    let _ = decode_with_limits(&header_mutated, limits);
});
