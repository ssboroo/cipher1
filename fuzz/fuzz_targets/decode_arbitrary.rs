#![no_main]

use libfuzzer_sys::fuzz_target;
use nexus_transform::{decode_with_limits, encode, DecodeLimits};

fuzz_target!(|data: &[u8]| {
    let limits = DecodeLimits {
        max_output_len: 1024 * 1024,
    };

    if let Ok(decoded) = decode_with_limits(data, limits) {
        let reencoded = encode(&decoded);
        let decoded_again = decode_with_limits(&reencoded, limits)
            .expect("the encoder must produce a valid stream");
        assert_eq!(decoded_again, decoded);
    }
});
