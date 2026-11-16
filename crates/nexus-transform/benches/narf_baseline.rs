use brotli::{CompressorWriter, Decompressor};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use nexus_transform::{decode, encode};
use std::hint::black_box;
use std::io::{Cursor, Read, Write};
use std::time::Instant;

const INPUT_LEN: usize = 16 * 1024;
const WARMUP: usize = 5;
const ITERATIONS: usize = 30;
const PAIR_RLE_MAGIC: &[u8; 4] = b"PRL1";

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

/// A deliberately simple benchmark-only pair-RLE frame.
/// The 8-byte header is magic (4 bytes) plus original length (u32 BE).
/// Each payload pair is a non-zero u8 count followed by its byte value.
fn pair_rle_encode(input: &[u8]) -> Vec<u8> {
    let original_len = u32::try_from(input.len()).expect("benchmark input fits in u32");
    let mut output = Vec::new();
    output.extend_from_slice(PAIR_RLE_MAGIC);
    output.extend_from_slice(&original_len.to_be_bytes());

    let mut i = 0usize;
    while i < input.len() {
        let value = input[i];
        let mut count = 1usize;
        while i + count < input.len() && input[i + count] == value && count < u8::MAX as usize {
            count += 1;
        }
        output.push(count as u8);
        output.push(value);
        i += count;
    }
    output
}

fn pair_rle_decode(input: &[u8]) -> Vec<u8> {
    assert!(input.len() >= 8, "truncated pair-RLE frame");
    assert_eq!(&input[..4], PAIR_RLE_MAGIC, "invalid pair-RLE magic");
    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&input[4..8]);
    let expected = u32::from_be_bytes(length_bytes) as usize;
    let payload = &input[8..];
    assert_eq!(payload.len() % 2, 0, "truncated pair-RLE pair");

    let mut output = Vec::with_capacity(expected);
    for pair in payload.chunks_exact(2) {
        let count = pair[0] as usize;
        assert_ne!(count, 0, "zero-length pair-RLE run");
        let new_len = output
            .len()
            .checked_add(count)
            .expect("pair-RLE output length overflow");
        assert!(new_len <= expected, "pair-RLE output exceeds declared length");
        output.resize(new_len, pair[1]);
    }
    assert_eq!(output.len(), expected, "pair-RLE output length mismatch");
    output
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
    decoder.read_to_end(&mut output).expect("Brotli decompression");
    output
}

#[derive(Clone, Copy)]
struct Percentiles {
    p50: f64,
    p95: f64,
    p99: f64,
}

fn nearest_rank(samples: &[f64], percentile: f64) -> f64 {
    let rank = (percentile * samples.len() as f64).ceil() as usize;
    samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

fn measure<F>(input_len: usize, mut operation: F) -> Percentiles
where
    F: FnMut() -> Vec<u8>,
{
    for _ in 0..WARMUP {
        black_box(operation());
    }

    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let output = black_box(operation());
        let elapsed_ns = start.elapsed().as_nanos() as f64;
        black_box(&output);
        samples.push(elapsed_ns / input_len.max(1) as f64);
    }
    samples.sort_by(|left, right| left.total_cmp(right));

    Percentiles {
        p50: nearest_rank(&samples, 0.50),
        p95: nearest_rank(&samples, 0.95),
        p99: nearest_rank(&samples, 0.99),
    }
}

fn report_codec<E, D>(
    dataset: &str,
    codec: &str,
    input: &[u8],
    encoded: &[u8],
    mut encode_operation: E,
    mut decode_operation: D,
) where
    E: FnMut(&[u8]) -> Vec<u8>,
    D: FnMut(&[u8]) -> Vec<u8>,
{
    assert_eq!(encode_operation(input), encoded, "{codec} output is not deterministic");
    assert_eq!(decode_operation(encoded), input, "{codec} round trip failed");

    let encode = measure(input.len(), || encode_operation(input));
    let decode = measure(input.len(), || decode_operation(encoded));
    let ratio = encoded.len() as f64 / input.len().max(1) as f64;

    println!(
        "{dataset},{codec},{},{},{ratio:.6},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
        input.len(),
        encoded.len(),
        encode.p50,
        encode.p95,
        encode.p99,
        decode.p50,
        decode.p95,
        decode.p99,
    );
}

fn main() {
    println!("# benchmark_schema=2");
    println!("# architecture={}", std::env::consts::ARCH);
    println!("# operating_system={}", std::env::consts::OS);
    println!("# input_bytes_per_dataset={INPUT_LEN}");
    println!("# warmup_iterations={WARMUP}");
    println!("# measured_iterations={ITERATIONS}");
    println!(
        "dataset,codec,input_bytes,encoded_bytes,total_ratio,encode_p50_ns_per_byte,encode_p95_ns_per_byte,encode_p99_ns_per_byte,decode_p50_ns_per_byte,decode_p95_ns_per_byte,decode_p99_ns_per_byte"
    );

    for (name, input) in datasets() {
        let identity = input.clone();
        let pair_rle = pair_rle_encode(&input);
        let narf = encode(&input);
        let lz4 = compress_prepend_size(&input);
        let zstd = zstd::bulk::compress(&input, 3).expect("Zstandard compression");
        let brotli = brotli_compress(&input);

        report_codec(name, "identity", &input, &identity, |bytes| bytes.to_vec(), |bytes| {
            bytes.to_vec()
        });
        report_codec(
            name,
            "pair_rle",
            &input,
            &pair_rle,
            pair_rle_encode,
            pair_rle_decode,
        );
        report_codec(name, "narf_v3", &input, &narf, encode, |bytes| {
            decode(bytes).expect("NARF decode")
        });
        report_codec(
            name,
            "lz4_prepend_size",
            &input,
            &lz4,
            compress_prepend_size,
            |bytes| decompress_size_prepended(bytes).expect("LZ4 decode"),
        );
        report_codec(
            name,
            "zstd_level_3",
            &input,
            &zstd,
            |bytes| zstd::bulk::compress(bytes, 3).expect("Zstandard compression"),
            |bytes| {
                zstd::bulk::decompress(bytes, input.len()).expect("Zstandard decompression")
            },
        );
        report_codec(
            name,
            "brotli_quality_5",
            &input,
            &brotli,
            brotli_compress,
            brotli_decompress,
        );
    }
}
