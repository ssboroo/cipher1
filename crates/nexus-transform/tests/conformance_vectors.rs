use nexus_transform::{decode_with_limits, encode, DecodeLimits};

const VECTORS: &str = include_str!("../../../test-vectors/narf-frames.tsv");

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hex value has an odd length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).expect("valid hex digit");
            let low = (pair[1] as char).to_digit(16).expect("valid hex digit");
            ((high << 4) | low) as u8
        })
        .collect()
}

#[test]
fn rust_decoder_matches_shared_narf_frame_vectors() {
    let mut lines = VECTORS.lines();
    assert_eq!(
        lines.next(),
        Some("id\tframe_hex\tdecoded_hex\trust_writer")
    );

    let mut vector_count = 0usize;
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "malformed vector row: {line}");

        let frame = decode_hex(fields[1]);
        let expected = decode_hex(fields[2]);
        let decoded = decode_with_limits(
            &frame,
            DecodeLimits {
                max_output_len: expected.len(),
            },
        )
        .unwrap_or_else(|error| panic!("{} failed to decode: {error}", fields[0]));

        assert_eq!(decoded, expected, "decoder mismatch for {}", fields[0]);

        if fields[3] == "writer" {
            assert_eq!(
                encode(&expected),
                frame,
                "writer bytes differ for {}",
                fields[0]
            );
        } else {
            assert_eq!(fields[3], "decoder", "unknown vector role");
        }

        vector_count += 1;
    }

    assert_eq!(vector_count, 13, "unexpected conformance vector count");
}
