use brotli::{CompressorWriter, Decompressor};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use nexus_transform::{decode, encode};
use std::hint::black_box;
use std::io::{Cursor, Read, Write};
use std::time::Instant;

const INPUT_LEN: usize = 16 * 1024;
const WARMUP: usize = 2;
const ITERATIONS: usize = 10;

fn datasets() -> Vec<(&'static str, Vec<u8>)> {
    let repeated = vec![0x41; INPUT_LEN];
    let periodic = (0..INPUT_LEN)
        .map(|i| [0x10, 0x20, 0x30, 0x40][i % 4])
        .collect();
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

fn brotli_compress(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    {
        let mut writer = CompressorWriter::new(&mut output, 4096, 5, 22);
        writer.write_all(input).expect("Brotli compression");
    }
    output
}

fn brotli_decompress(input: &[u8]) -> Vec<u8> {
    let mut decoder = Decompressor::new(Cursor::new(input), 4096);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .expect("Brotli decompression");
    output
}

fn measure_encode<F>(input: &[u8], mut operation: F) -> f64
where
    F: FnMut(&[u8]) -> Vec<u8>,
{
    for _ in 0..WARMUP {
        black_box(operation(black_box(input)));
    }
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(operation(black_box(input)));
    }
    start.elapsed().as_nanos() as f64 / (ITERATIONS * input.len().max(1)) as f64
}

fn measure_decode<F>(encoded: &[u8], input_len: usize, mut operation: F) -> f64
where
    F: FnMut(&[u8]) -> Vec<u8>,
{
    for _ in 0..WARMUP {
        black_box(operation(black_box(encoded)));
    }
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(operation(black_box(encoded)));
    }
    start.elapsed().as_nanos() as f64 / (ITERATIONS * input_len.max(1)) as f64
}

fn main() {
    println!(
        "dataset,input_bytes,identity_bytes,narf_bytes,narf_ratio,lz4_bytes,zstd_bytes,brotli_bytes,pair_rle_bytes,narf_encode_ns_per_byte,narf_decode_ns_per_byte,lz4_encode_ns_per_byte,lz4_decode_ns_per_byte,zstd_encode_ns_per_byte,zstd_decode_ns_per_byte,brotli_encode_ns_per_byte,brotli_decode_ns_per_byte"
    );

    for (name, input) in datasets() {
        let narf = encode(&input);
        let lz4 = compress_prepend_size(&input);
        let zstd = zstd::bulk::compress(&input, 3).expect("Zstandard compression");
        let brotli = brotli_compress(&input);

        assert_eq!(decode(&narf).expect("NARF decode"), input);
        assert_eq!(decompress_size_prepended(&lz4).expect("LZ4 decode"), input);
        assert_eq!(
            zstd::bulk::decompress(&zstd, input.len()).expect("Zstandard decode"),
            input
        );
        assert_eq!(brotli_decompress(&brotli), input);

        let narf_encode = measure_encode(&input, encode);
        let narf_decode = measure_decode(&narf, input.len(), |bytes| {
            decode(bytes).expect("NARF decode")
        });
        let lz4_encode = measure_encode(&input, compress_prepend_size);
        let lz4_decode = measure_decode(&lz4, input.len(), |bytes| {
            decompress_size_prepended(bytes).expect("LZ4 decode")
        });
        let zstd_encode = measure_encode(&input, |bytes| {
            zstd::bulk::compress(bytes, 3).expect("Zstandard compression")
        });
        let zstd_decode = measure_decode(&zstd, input.len(), |bytes| {
            zstd::bulk::decompress(bytes, input.len()).expect("Zstandard decode")
        });
        let brotli_encode = measure_encode(&input, brotli_compress);
        let brotli_decode = measure_decode(&brotli, input.len(), brotli_decompress);

        println!(
            "{name},{},{},{},{:.6},{},{},{},{},{narf_encode:.3},{narf_decode:.3},{lz4_encode:.3},{lz4_decode:.3},{zstd_encode:.3},{zstd_decode:.3},{brotli_encode:.3},{brotli_decode:.3}",
            input.len(),
            input.len(),
            narf.len(),
            narf.len() as f64 / input.len() as f64,
            lz4.len(),
            zstd.len(),
            brotli.len(),
            pair_rle_size(&input),
        );
    }
}
