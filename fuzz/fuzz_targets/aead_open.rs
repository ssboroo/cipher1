#![no_main]

use cipher1_core::aead::{open, seal, AeadDecodeLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let key = [0x42; 32];
    let limits = AeadDecodeLimits {
        max_plaintext_len: 1024 * 1024,
    };

    let sealed = seal(&key, data, b"nexus-fuzz-context")
        .expect("bounded fuzz input should be sealable");
    let round_trip = open(&sealed, &key, b"nexus-fuzz-context", limits)
        .expect("freshly sealed fuzz input must authenticate");
    assert_eq!(round_trip.as_slice(), data);

    if let Ok(plaintext) = open(data, &key, b"nexus-fuzz-context", limits) {
        let frame = seal(&key, plaintext.as_slice(), b"nexus-fuzz-context")
            .expect("bounded accepted plaintext should be sealable");
        let decoded = open(&frame, &key, b"nexus-fuzz-context", limits)
            .expect("the AEAD encoder must produce an authenticated frame");
        assert_eq!(decoded.as_slice(), plaintext.as_slice());
    }
});
