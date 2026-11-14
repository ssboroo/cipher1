#![no_main]

use cipher1_core::aead::{open, seal, AeadDecodeLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let key = [0x42; 32];
    let limits = AeadDecodeLimits {
        max_plaintext_len: 1024 * 1024,
    };

    if let Ok(plaintext) = open(data, &key, b"nexus-fuzz-context", limits) {
        let frame = seal(&key, &plaintext, b"nexus-fuzz-context")
            .expect("bounded accepted plaintext should be sealable");
        let decoded = open(&frame, &key, b"nexus-fuzz-context", limits)
            .expect("the AEAD encoder must produce an authenticated frame");
        assert_eq!(decoded, plaintext);
    }
});
