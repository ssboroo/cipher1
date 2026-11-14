use nexus_transform::{decode, encode};
use std::hint::black_box;
use std::time::Instant;

const INPUT_LEN: usize = 16 * 1024;
const ITERATIONS: usize = 5;

fn datasets() -> Vec<(&'static str, Vec<u8>)> {
    let repeated = vec![0x41; INPUT_LEN];
    let periodic = (0..INPUT_LEN).map(|i| [0x10, 0x20, 0x30, 0x40][i % 4]).collect();
    let ramp = (0..INPUT_LEN).map(|i| (i & 0xff) as u8).collect();
    let text_like = (0..INPUT_LEN)
        .map(|i| {
            let alphabet = b"the quick brown fox jumps over the lazy dog ";
            alphabet[i % alphabet.len()]
        })
        .collect();
    let mut state = 0x9e37_79b9_u32;
    let random = (0..INPUT_LEN)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect();

    vec![
        ("repeated", repeated),
        ("periodic", periodic),
        ("ramp", ramp),
        ("text_like", text_like),
        ("deterministic_random", random),
    ]
}

// Deliberately simple reference baseline: encode each run as (count, byte),
// splitting runs at 255. This is a size baseline, not a production compressor.
fn pair_rle_size(input: &[u8]) -> usize {
    let mut encoded = 0usize;
    let mut i = 0usize;
    while i < input.len() {
        let value = input[i];
        let mut count = 1usize;
        while i + count < input.len() && input[i + count] == value && count < 255 {
            count += 1;
        }
        encoded += 2;
        i += count;
    }
    encoded
}

fn main() {
    println!("dataset,input_bytes,narf_bytes,identity_bytes,pair_rle_bytes,narf_ratio,encode_ns_per_byte,decode_ns_per_byte");
    for (name, input) in datasets() {
        let encoded = encode(&input);
        assert_eq!(decode(&encoded).expect("NARF decode"), input);

        let encode_start = Instant::now();
        for _ in 0..ITERATIONS {
            black_box(encode(black_box(&input)));
        }
        let encode_ns = encode_start.elapsed().as_nanos() as f64
            / (ITERATIONS * input.len()) as f64;

        let decode_start = Instant::now();
        for _ in 0..ITERATIONS {
            black_box(decode(black_box(&encoded)).expect("NARF decode"));
        }
        let decode_ns = decode_start.elapsed().as_nanos() as f64
            / (ITERATIONS * input.len()) as f64;

        let ratio = encoded.len() as f64 / input.len().max(1) as f64;
        println!(
            "{name},{},{},{},{},{ratio:.6},{encode_ns:.3},{decode_ns:.3}",
            input.len(),
            encoded.len(),
            input.len(),
            pair_rle_size(&input),
        );
    }
}
