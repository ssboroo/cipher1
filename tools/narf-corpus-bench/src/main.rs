#![forbid(unsafe_code)]

use brotli::{CompressorWriter, Decompressor};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use nexus_transform::{decode_with_limits, encode, DecodeLimits};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::hint::black_box;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

type AppResult<T> = Result<T, String>;

const MANIFEST_HEADER: &str = "sha256\tbytes\tpath";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANIFEST_ENTRIES: usize = 10_000;
const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const DEFAULT_WARMUPS: usize = 5;
const DEFAULT_SAMPLES: usize = 30;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const PAIR_RLE_MAGIC: &[u8; 4] = b"PRL1";
const USAGE: &str = "Usage:
  cargo run --release -p narf-corpus-bench -- \
    --corpus-id ID --corpus-dir DIR --manifest FILE --output-csv FILE \
    [--metadata-out FILE] [--max-file-bytes N] [--max-total-bytes N] \
    [--warmups N] [--samples N]

Manifest format: UTF-8 TSV with exact header sha256<TAB>bytes<TAB>path.";

const CODECS: [Codec; 6] = [
    Codec::Identity,
    Codec::PairRle,
    Codec::NarfV3,
    Codec::Lz4,
    Codec::Zstd,
    Codec::Brotli,
];

#[derive(Debug, Clone)]
struct Config {
    corpus_id: String,
    corpus_dir: PathBuf,
    manifest: PathBuf,
    output_csv: PathBuf,
    metadata_out: PathBuf,
    max_file_bytes: u64,
    max_total_bytes: u64,
    warmups: usize,
    samples: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestEntry {
    digest: [u8; 32],
    bytes: u64,
    path: String,
}

#[derive(Debug, Clone, Copy)]
enum Codec {
    Identity,
    PairRle,
    NarfV3,
    Lz4,
    Zstd,
    Brotli,
}

impl Codec {
    fn name(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::PairRle => "pair_rle",
            Self::NarfV3 => "narf_v3",
            Self::Lz4 => "lz4_prepend_size",
            Self::Zstd => "zstd_level_3",
            Self::Brotli => "brotli_quality_5",
        }
    }
}

#[derive(Debug, Clone)]
struct Measurement {
    p50: f64,
    p95: f64,
    p99: f64,
    raw_samples: Vec<f64>,
}

#[derive(Debug)]
struct FileResult {
    path: String,
    codec: Codec,
    input_bytes: u64,
    encoded_bytes: u64,
    encode: Measurement,
    decode: Measurement,
    samples: usize,
}

#[derive(Debug, Default)]
struct Aggregate {
    encoded_bytes: u64,
    ratios: Vec<f64>,
    file_count: usize,
    nonempty_file_count: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
}

fn run() -> AppResult<()> {
    let Some(config) = parse_args(env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(());
    };

    let root = fs::canonicalize(&config.corpus_dir)
        .map_err(|error| format!("cannot resolve corpus directory: {error}"))?;
    if !root.is_dir() {
        return Err("corpus directory is not a directory".to_owned());
    }

    let manifest_metadata = fs::metadata(&config.manifest)
        .map_err(|error| format!("cannot inspect manifest: {error}"))?;
    if manifest_metadata.len() > MAX_MANIFEST_BYTES {
        return Err(format!("manifest exceeds the {MAX_MANIFEST_BYTES}-byte limit"));
    }
    let manifest_file = File::open(&config.manifest)
        .map_err(|error| format!("cannot open manifest: {error}"))?;
    let mut manifest_bytes = Vec::new();
    manifest_bytes
        .try_reserve_exact(manifest_metadata.len() as usize)
        .map_err(|error| format!("cannot reserve manifest buffer: {error}"))?;
    manifest_file
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut manifest_bytes)
        .map_err(|error| format!("cannot read manifest: {error}"))?;
    if manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(format!("manifest exceeds the {MAX_MANIFEST_BYTES}-byte limit"));
    }
    let manifest_text = std::str::from_utf8(&manifest_bytes)
        .map_err(|_| "manifest is not valid UTF-8".to_owned())?;
    let entries = parse_manifest(manifest_text)?;
    let manifest_digest = sha256(&manifest_bytes);
    let manifest_path = fs::canonicalize(&config.manifest)
        .map_err(|error| format!("cannot resolve manifest path: {error}"))?;

    let csv_target = prepare_output_target(&config.output_csv, &root, &manifest_path)?;
    let metadata_target = prepare_output_target(&config.metadata_out, &root, &manifest_path)?;
    if csv_target == metadata_target {
        return Err("CSV and metadata output paths must be different".to_owned());
    }

    // Verify every file before invoking any codec. Re-hash each file when it is
    // loaded for benchmarking to detect changes after this preflight pass.
    let total_input_bytes = preflight(
        &root,
        &entries,
        config.max_file_bytes,
        config.max_total_bytes,
    )?;

    let mut csv = String::from(
        "record_type,corpus_id,file_path,codec,file_count,nonempty_file_count,input_bytes,encoded_bytes,ratio,aggregate_ratio,median_file_ratio,encode_p50_ns_per_byte,encode_p95_ns_per_byte,encode_p99_ns_per_byte,decode_p50_ns_per_byte,decode_p95_ns_per_byte,decode_p99_ns_per_byte,sample_count,encode_raw_samples_ns_per_byte,decode_raw_samples_ns_per_byte\n",
    );
    let mut aggregates = BTreeMap::<&'static str, Aggregate>::new();
    for codec in CODECS {
        aggregates.insert(codec.name(), Aggregate::default());
    }

    for entry in &entries {
        let input = read_verified_file(&root, entry)?;
        for codec in CODECS {
            let result = benchmark_file(
                &entry.path,
                codec,
                &input,
                config.warmups,
                config.samples,
            )?;
            append_file_row(&mut csv, &config.corpus_id, &result);

            let aggregate = aggregates
                .get_mut(codec.name())
                .ok_or_else(|| "internal aggregate entry is missing".to_owned())?;
            aggregate.file_count += 1;
            aggregate.encoded_bytes = aggregate
                .encoded_bytes
                .checked_add(result.encoded_bytes)
                .ok_or_else(|| "aggregate encoded byte count overflow".to_owned())?;
            if result.input_bytes > 0 {
                aggregate.nonempty_file_count += 1;
                aggregate
                    .ratios
                    .push(result.encoded_bytes as f64 / result.input_bytes as f64);
            }
        }
    }

    for codec in CODECS {
        let aggregate = aggregates
            .get(codec.name())
            .ok_or_else(|| "internal aggregate entry is missing".to_owned())?;
        append_aggregate_row(
            &mut csv,
            &config.corpus_id,
            codec,
            aggregate,
            total_input_bytes,
        );
    }

    let metadata = build_metadata(
        &config,
        &entries,
        total_input_bytes,
        &manifest_digest,
    );
    write_new_file(&csv_target, csv.as_bytes())?;
    write_new_file(&metadata_target, metadata.as_bytes())?;
    println!("Wrote {} and {}", csv_target.display(), metadata_target.display());
    Ok(())
}

fn parse_args<I>(args: I) -> AppResult<Option<Config>>
where
    I: IntoIterator<Item = String>,
{
    let mut values = BTreeMap::<String, String>::new();
    let allowed = [
        "--corpus-id", "--corpus-dir", "--manifest", "--output-csv",
        "--metadata-out", "--max-file-bytes", "--max-total-bytes",
        "--warmups", "--samples",
    ];
    let mut args = args.into_iter();
    while let Some(key) = args.next() {
        if key == "--help" || key == "-h" {
            return Ok(None);
        }
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown argument: {key}"));
        }
        let value = args.next().ok_or_else(|| format!("missing value for {key}"))?;
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate argument: {key}"));
        }
    }

    let required = |key: &str| -> AppResult<String> {
        values.get(key).cloned().ok_or_else(|| format!("missing required argument: {key}"))
    };
    let parse_u64 = |key: &str, default: u64| -> AppResult<u64> {
        match values.get(key) {
            Some(value) => value.parse::<u64>().map_err(|_| format!("invalid integer for {key}")),
            None => Ok(default),
        }
    };
    let parse_usize = |key: &str, default: usize| -> AppResult<usize> {
        match values.get(key) {
            Some(value) => value.parse::<usize>().map_err(|_| format!("invalid integer for {key}")),
            None => Ok(default),
        }
    };

    let corpus_id = required("--corpus-id")?;
    if corpus_id.is_empty()
        || !corpus_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("corpus ID must contain only ASCII letters, digits, dot, underscore, or hyphen".to_owned());
    }

    let max_file_bytes = parse_u64("--max-file-bytes", DEFAULT_MAX_FILE_BYTES)?;
    if max_file_bytes == 0 || max_file_bytes > u32::MAX as u64 {
        return Err("max-file-bytes must be in 1..=u32::MAX for the pair-RLE baseline".to_owned());
    }
    let max_total_bytes = parse_u64("--max-total-bytes", DEFAULT_MAX_TOTAL_BYTES)?;
    if max_total_bytes == 0 {
        return Err("max-total-bytes must be greater than zero".to_owned());
    }
    let warmups = parse_usize("--warmups", DEFAULT_WARMUPS)?;
    let samples = parse_usize("--samples", DEFAULT_SAMPLES)?;
    if samples < 5 {
        return Err("samples must be at least 5 for percentile summaries".to_owned());
    }
    if warmups > 10_000 || samples > 100_000 {
        return Err("warmups and samples exceed the configured safety limits".to_owned());
    }

    let output_csv = PathBuf::from(required("--output-csv")?);
    let metadata_out = values
        .get("--metadata-out")
        .map(|value| PathBuf::from(value.as_str()))
        .unwrap_or_else(|| {
            let mut path = output_csv.as_os_str().to_os_string();
            path.push(".metadata.txt");
            PathBuf::from(path)
        });

    Ok(Some(Config {
        corpus_id,
        corpus_dir: PathBuf::from(required("--corpus-dir")?),
        manifest: PathBuf::from(required("--manifest")?),
        output_csv,
        metadata_out,
        max_file_bytes,
        max_total_bytes,
        warmups,
        samples,
    }))
}

fn parse_manifest(text: &str) -> AppResult<Vec<ManifestEntry>> {
    let mut lines = text.lines();
    if lines.next() != Some(MANIFEST_HEADER) {
        return Err(format!("manifest header must be exactly: {MANIFEST_HEADER}"));
    }

    let mut entries = Vec::new();
    let mut previous_path: Option<String> = None;
    for (line_index, line) in lines.enumerate() {
        if line.is_empty() {
            return Err(format!("manifest line {} is empty", line_index + 2));
        }
        let mut fields = line.splitn(3, '\t');
        let digest_text = fields.next().ok_or_else(|| "missing digest".to_owned())?;
        let bytes_text = fields.next().ok_or_else(|| "missing byte length".to_owned())?;
        let path = fields.next().ok_or_else(|| "missing path".to_owned())?;

        let digest = parse_digest(digest_text)
            .map_err(|error| format!("manifest line {}: {error}", line_index + 2))?;
        if bytes_text.is_empty()
            || !bytes_text.bytes().all(|byte| byte.is_ascii_digit())
            || (bytes_text.len() > 1 && bytes_text.starts_with('0'))
        {
            return Err(format!("manifest line {} has a non-canonical decimal byte length", line_index + 2));
        }
        let bytes = bytes_text
            .parse::<u64>()
            .map_err(|_| format!("manifest line {} has an invalid byte length", line_index + 2))?;
        validate_relative_path(path)
            .map_err(|error| format!("manifest line {}: {error}", line_index + 2))?;

        if let Some(previous) = &previous_path {
            if previous.as_bytes() >= path.as_bytes() {
                return Err(format!(
                    "manifest paths must be unique and sorted by UTF-8 byte order (line {})",
                    line_index + 2
                ));
            }
        }
        previous_path = Some(path.to_owned());
        entries.push(ManifestEntry { digest, bytes, path: path.to_owned() });
        if entries.len() > MAX_MANIFEST_ENTRIES {
            return Err(format!("manifest exceeds the {MAX_MANIFEST_ENTRIES}-entry limit"));
        }
    }

    if entries.is_empty() {
        return Err("manifest must contain at least one file entry".to_owned());
    }
    Ok(entries)
}

fn validate_relative_path(path: &str) -> AppResult<()> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\t')
        || path.contains('\n')
        || path.contains('\r')
        || path.contains('\0')
    {
        return Err("path must be a portable relative path using forward slashes".to_owned());
    }

    for component in path.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with('.')
            || component.ends_with(' ')
            || component.chars().any(|character| {
                character.is_control() || matches!(character, '<' | '>' | '"' | '|' | '?' | '*')
            })
        {
            return Err(format!("path contains a non-portable component: {component:?}"));
        }
        let stem = component.split('.').next().unwrap_or_default().to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if reserved {
            return Err(format!("path uses a reserved Windows name: {component:?}"));
        }
    }
    Ok(())
}

fn parse_digest(value: &str) -> AppResult<[u8; 32]> {
    if value.len() != 64 {
        return Err("SHA-256 digest must contain exactly 64 lowercase hexadecimal characters".to_owned());
    }
    let bytes = value.as_bytes();
    let mut digest = [0u8; 32];
    for index in 0..32 {
        let high = hex_nibble(bytes[index * 2])
            .ok_or_else(|| "SHA-256 digest contains a non-lowercase-hexadecimal character".to_owned())?;
        let low = hex_nibble(bytes[index * 2 + 1])
            .ok_or_else(|| "SHA-256 digest contains a non-lowercase-hexadecimal character".to_owned())?;
        digest[index] = (high << 4) | low;
    }
    Ok(digest)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn format_digest(digest: &[u8; 32]) -> String {
    use std::fmt::Write as FmtWrite;
    let mut output = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn sha256_file(path: &Path, expected_bytes: u64) -> AppResult<[u8; 32]> {
    let file = File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    let mut total = 0u64;
    let mut reader = file.take(expected_bytes.saturating_add(1));
    loop {
        let count = reader.read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64)
            .ok_or_else(|| format!("file size overflow while hashing {}", path.display()))?;
        hasher.update(&buffer[..count]);
    }
    if total != expected_bytes {
        return Err(format!("file size changed while hashing {}", path.display()));
    }
    Ok(hasher.finalize().into())
}

fn preflight(
    root: &Path,
    entries: &[ManifestEntry],
    max_file_bytes: u64,
    max_total_bytes: u64,
) -> AppResult<u64> {
    let mut total = 0u64;
    for entry in entries {
        if entry.bytes > max_file_bytes {
            return Err(format!("{} exceeds max-file-bytes", entry.path));
        }
        total = total.checked_add(entry.bytes)
            .ok_or_else(|| "total input byte count overflow".to_owned())?;
        if total > max_total_bytes {
            return Err(format!("corpus exceeds max-total-bytes: {total} > {max_total_bytes}"));
        }
        let path = resolve_entry(root, &entry.path)?;
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("cannot inspect {}: {error}", entry.path))?;
        if metadata.len() != entry.bytes {
            return Err(format!(
                "size mismatch for {}: manifest={}, actual={}",
                entry.path, entry.bytes, metadata.len()
            ));
        }
        if sha256_file(&path, entry.bytes)? != entry.digest {
            return Err(format!("SHA-256 mismatch for {}", entry.path));
        }
    }
    Ok(total)
}

fn resolve_entry(root: &Path, relative_path: &str) -> AppResult<PathBuf> {
    validate_relative_path(relative_path)?;
    let components: Vec<&str> = relative_path.split('/').collect();
    let mut current = root.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("cannot inspect {}: {error}", relative_path))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlink paths are not allowed: {relative_path}"));
        }
        if index + 1 == components.len() {
            if !metadata.is_file() {
                return Err(format!("manifest entry is not a regular file: {relative_path}"));
            }
        } else if !metadata.is_dir() {
            return Err(format!("path parent is not a directory: {relative_path}"));
        }
    }
    Ok(current)
}

fn read_verified_file(root: &Path, entry: &ManifestEntry) -> AppResult<Vec<u8>> {
    let path = resolve_entry(root, &entry.path)?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("cannot inspect {}: {error}", entry.path))?;
    if metadata.len() != entry.bytes {
        return Err(format!("file changed after preflight: {}", entry.path));
    }
    let capacity = usize::try_from(entry.bytes)
        .map_err(|_| format!("file size is not representable: {}", entry.path))?;
    let file = File::open(&path).map_err(|error| format!("cannot open {}: {error}", entry.path))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity)
        .map_err(|error| format!("cannot reserve input buffer for {}: {error}", entry.path))?;
    file.take(entry.bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", entry.path))?;
    if bytes.len() as u64 != entry.bytes || sha256(&bytes) != entry.digest {
        return Err(format!("file changed after preflight: {}", entry.path));
    }
    Ok(bytes)
}

fn prepare_output_target(path: &Path, root: &Path, manifest: &Path) -> AppResult<PathBuf> {
    let file_name = path.file_name()
        .ok_or_else(|| format!("output path has no file name: {}", path.display()))?;
    let parent = path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve output directory {}: {error}", parent.display()))?;
    let target = canonical_parent.join(file_name);
    if target.starts_with(root) {
        return Err(format!("output must be outside the corpus directory: {}", target.display()));
    }
    if target.as_path() == manifest {
        return Err("output path must not overwrite the manifest".to_owned());
    }
    if fs::symlink_metadata(&target).is_ok() {
        return Err(format!("refusing to overwrite an existing output: {}", target.display()));
    }
    Ok(target)
}

fn write_new_file(path: &Path, contents: &[u8]) -> AppResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(contents).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    file.sync_all().map_err(|error| format!("cannot sync {}: {error}", path.display()))?;
    Ok(())
}

fn benchmark_file(
    path: &str,
    codec: Codec,
    input: &[u8],
    warmups: usize,
    samples: usize,
) -> AppResult<FileResult> {
    let encoded = encode_codec(codec, input)?;
    if encode_codec(codec, input)? != encoded {
        return Err(format!("{} encoder is not deterministic for {path}", codec.name()));
    }
    if decode_codec(codec, &encoded, input.len())? != input {
        return Err(format!("{} round trip failed for {path}", codec.name()));
    }
    let encode = measure(input.len(), warmups, samples, || encode_codec(codec, input))?;
    let decode = measure(input.len(), warmups, samples, || {
        decode_codec(codec, &encoded, input.len())
    })?;
    Ok(FileResult {
        path: path.to_owned(),
        codec,
        input_bytes: input.len() as u64,
        encoded_bytes: encoded.len() as u64,
        encode,
        decode,
        samples,
    })
}

fn encode_codec(codec: Codec, input: &[u8]) -> AppResult<Vec<u8>> {
    match codec {
        Codec::Identity => Ok(input.to_vec()),
        Codec::PairRle => pair_rle_encode(input),
        Codec::NarfV3 => Ok(encode(input)),
        Codec::Lz4 => Ok(compress_prepend_size(input)),
        Codec::Zstd => zstd::bulk::compress(input, 3)
            .map_err(|error| format!("Zstandard encode failed: {error}")),
        Codec::Brotli => {
            let mut output = Vec::new();
            {
                let mut writer = CompressorWriter::new(&mut output, 4096, 5, 22);
                writer.write_all(input)
                    .map_err(|error| format!("Brotli encode failed: {error}"))?;
            }
            Ok(output)
        }
    }
}

fn decode_codec(codec: Codec, encoded: &[u8], expected: usize) -> AppResult<Vec<u8>> {
    match codec {
        Codec::Identity => {
            if encoded.len() != expected {
                return Err("identity length mismatch".to_owned());
            }
            Ok(encoded.to_vec())
        }
        Codec::PairRle => pair_rle_decode(encoded, expected),
        Codec::NarfV3 => decode_with_limits(
            encoded,
            DecodeLimits { max_output_len: expected },
        )
        .map_err(|error| format!("NARF decode failed: {error}")),
        Codec::Lz4 => {
            if encoded.len() < 4 {
                return Err("LZ4 frame is truncated".to_owned());
            }
            let declared = u32::from_le_bytes(
                encoded[..4].try_into().map_err(|_| "LZ4 length prefix is invalid".to_owned())?,
            ) as usize;
            if declared != expected {
                return Err(format!("LZ4 length mismatch: declared={declared}, expected={expected}"));
            }
            decompress_size_prepended(encoded).map_err(|error| format!("LZ4 decode failed: {error}"))
        }
        Codec::Zstd => zstd::bulk::decompress(encoded, expected)
            .map_err(|error| format!("Zstandard decode failed: {error}")),
        Codec::Brotli => {
            let mut decoder = Decompressor::new(Cursor::new(encoded), 4096);
            let mut output = Vec::new();
            output.try_reserve_exact(expected)
                .map_err(|error| format!("cannot reserve Brotli output: {error}"))?;
            decoder.read_to_end(&mut output)
                .map_err(|error| format!("Brotli decode failed: {error}"))?;
            if output.len() != expected {
                return Err(format!("Brotli length mismatch: decoded={}, expected={expected}", output.len()));
            }
            Ok(output)
        }
    }
}

fn pair_rle_encode(input: &[u8]) -> AppResult<Vec<u8>> {
    let original_len = u32::try_from(input.len())
        .map_err(|_| "pair-RLE input exceeds its u32 frame limit".to_owned())?;
    let capacity = input.len().checked_mul(2).and_then(|value| value.checked_add(8))
        .ok_or_else(|| "pair-RLE output capacity overflow".to_owned())?;
    let mut output = Vec::new();
    output.try_reserve_exact(capacity)
        .map_err(|error| format!("cannot reserve pair-RLE output: {error}"))?;
    output.extend_from_slice(PAIR_RLE_MAGIC);
    output.extend_from_slice(&original_len.to_be_bytes());

    let mut index = 0usize;
    while index < input.len() {
        let value = input[index];
        let mut count = 1usize;
        while index + count < input.len()
            && input[index + count] == value
            && count < u8::MAX as usize
        {
            count += 1;
        }
        output.push(count as u8);
        output.push(value);
        index += count;
    }
    Ok(output)
}

fn pair_rle_decode(input: &[u8], expected: usize) -> AppResult<Vec<u8>> {
    if input.len() < 8 || &input[..4] != PAIR_RLE_MAGIC {
        return Err("pair-RLE header is invalid or truncated".to_owned());
    }
    let declared = u32::from_be_bytes(
        input[4..8].try_into().map_err(|_| "pair-RLE length is invalid".to_owned())?,
    ) as usize;
    if declared != expected {
        return Err(format!("pair-RLE length mismatch: declared={declared}, expected={expected}"));
    }
    let payload = &input[8..];
    if payload.len() % 2 != 0 {
        return Err("pair-RLE payload has a truncated pair".to_owned());
    }
    let mut output = Vec::new();
    output.try_reserve_exact(expected)
        .map_err(|error| format!("cannot reserve pair-RLE output: {error}"))?;
    for pair in payload.chunks_exact(2) {
        let count = pair[0] as usize;
        if count == 0 {
            return Err("pair-RLE contains a zero-length run".to_owned());
        }
        let new_len = output.len().checked_add(count)
            .ok_or_else(|| "pair-RLE output length overflow".to_owned())?;
        if new_len > expected {
            return Err("pair-RLE output exceeds the expected length".to_owned());
        }
        output.resize(new_len, pair[1]);
    }
    if output.len() != expected {
        return Err("pair-RLE output length mismatch".to_owned());
    }
    Ok(output)
}

fn measure<F>(input_len: usize, warmups: usize, samples: usize, mut operation: F) -> AppResult<Measurement>
where
    F: FnMut() -> AppResult<Vec<u8>>,
{
    for _ in 0..warmups {
        black_box(operation()?);
    }
    let mut timings = Vec::new();
    timings.try_reserve_exact(samples)
        .map_err(|error| format!("cannot reserve timing samples: {error}"))?;
    for _ in 0..samples {
        let start = Instant::now();
        let output = black_box(operation()?);
        let elapsed_ns = start.elapsed().as_nanos() as f64;
        black_box(&output);
        timings.push(elapsed_ns / input_len.max(1) as f64);
    }
    let mut sorted = timings.clone();
    sorted.sort_by(f64::total_cmp);
    Ok(Measurement {
        p50: nearest_rank(&sorted, 0.50),
        p95: nearest_rank(&sorted, 0.95),
        p99: nearest_rank(&sorted, 0.99),
        raw_samples: timings,
    })
}

fn nearest_rank(sorted: &[f64], percentile: f64) -> f64 {
    let rank = (percentile * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Some(values[middle])
    }
}

fn append_file_row(output: &mut String, corpus_id: &str, result: &FileResult) {
    let ratio = if result.input_bytes == 0 {
        String::new()
    } else {
        format!("{:.9}", result.encoded_bytes as f64 / result.input_bytes as f64)
    };
    let fields = vec![
        "file".to_owned(), corpus_id.to_owned(), result.path.clone(), result.codec.name().to_owned(),
        "1".to_owned(), (if result.input_bytes > 0 { "1" } else { "0" }).to_owned(),
        result.input_bytes.to_string(), result.encoded_bytes.to_string(), ratio,
        String::new(), String::new(),
        format!("{:.3}", result.encode.p50), format!("{:.3}", result.encode.p95),
        format!("{:.3}", result.encode.p99), format!("{:.3}", result.decode.p50),
        format!("{:.3}", result.decode.p95), format!("{:.3}", result.decode.p99),
        result.samples.to_string(),
        sample_list(&result.encode.raw_samples),
        sample_list(&result.decode.raw_samples),
    ];
    append_csv_row(output, &fields);
}

fn append_aggregate_row(
    output: &mut String,
    corpus_id: &str,
    codec: Codec,
    aggregate: &Aggregate,
    total_input_bytes: u64,
) {
    let aggregate_ratio = if total_input_bytes == 0 {
        String::new()
    } else {
        format!("{:.9}", aggregate.encoded_bytes as f64 / total_input_bytes as f64)
    };
    let median_ratio = median(&mut aggregate.ratios.clone())
        .map(|value| format!("{value:.9}"))
        .unwrap_or_default();
    let fields = vec![
        "aggregate".to_owned(), corpus_id.to_owned(), String::new(), codec.name().to_owned(),
        aggregate.file_count.to_string(), aggregate.nonempty_file_count.to_string(),
        total_input_bytes.to_string(), aggregate.encoded_bytes.to_string(), String::new(),
        aggregate_ratio, median_ratio,
        String::new(), String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
    ];
    append_csv_row(output, &fields);
}

fn sample_list(samples: &[f64]) -> String {
    samples.iter().map(|sample| sample.to_string()).collect::<Vec<_>>().join(";")
}

fn append_csv_row(output: &mut String, fields: &[String]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&csv_escape(field));
    }
    output.push('\n');
}

fn csv_escape(value: &str) -> String {
    if value.chars().any(|character| matches!(character, ',' | '"' | '\n' | '\r')) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn build_metadata(
    config: &Config,
    entries: &[ManifestEntry],
    total_input_bytes: u64,
    manifest_digest: &[u8; 32],
) -> String {
    let empty_files = entries.iter().filter(|entry| entry.bytes == 0).count();
    let commit = env::var("GITHUB_SHA")
        .or_else(|_| env::var("CIPHER1_COMMIT_SHA"))
        .unwrap_or_else(|_| "not-provided".to_owned());
    format!(
        "schema_version=1\ncorpus_id={}\nmanifest_sha256={}\nfile_count={}\nempty_file_count={}\ntotal_input_bytes={}\nmax_file_bytes={}\nmax_total_bytes={}\nwarmups={}\nsamples={}\npackage_version={}\ntarget_os={}\ntarget_arch={}\nrustc_version={}\ncargo_version={}\nrepository_commit={}\ncpu_model=not_collected_by_runner\ndependency_version_requirements=brotli:7.0,lz4_flex:0.11,sha2:0.10.9,zstd:0.13,nexus-transform:workspace\ncodec_pair_rle=PRL1_u32_be_count_value\ncodec_narf=nexus_transform_current_v3_writer\ncodec_lz4=lz4_flex_size_prepended\ncodec_zstd=zstd_bulk_level_3\ncodec_brotli=quality_5_lgwin_22_buffer_4096\n",
        config.corpus_id, format_digest(manifest_digest), entries.len(), empty_files,
        total_input_bytes, config.max_file_bytes, config.max_total_bytes,
        config.warmups, config.samples, env!("CARGO_PKG_VERSION"),
        env::consts::OS, env::consts::ARCH,
        command_version("rustc"), command_version("cargo"),
        sanitize_metadata_value(&commit),
    )
}

fn command_version(program: &str) -> String {
    match Command::new(program).arg("--version").output() {
        Ok(output) if output.status.success() => sanitize_metadata_value(
            String::from_utf8_lossy(&output.stdout).trim(),
        ),
        _ => "unavailable".to_owned(),
    }
}

fn sanitize_metadata_value(value: &str) -> String {
    value.chars().map(|character| {
        if character.is_control() { ' ' } else { character }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_ID: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("narf-corpus-bench-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temporary directory");
        path
    }

    fn manifest_line(bytes: &[u8], path: &str) -> String {
        format!("{}\t{}\t{path}", format_digest(&sha256(bytes)), bytes.len())
    }

    #[test]
    fn sha256_matches_the_standard_abc_test_vector() {
        assert_eq!(
            format_digest(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn manifest_accepts_sorted_entries_and_empty_files() {
        let text = format!(
            "{MANIFEST_HEADER}\n{}\n{}\n",
            manifest_line(b"", "empty.bin"),
            manifest_line(b"payload", "nested/payload.bin")
        );
        let entries = parse_manifest(&text).expect("valid manifest");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].bytes, 0);
        assert_eq!(entries[1].path, "nested/payload.bin");
    }

    #[test]
    fn manifest_rejects_bad_header_digest_and_uppercase_hex() {
        assert!(parse_manifest("sha256,bytes,path\n").is_err());
        assert!(parse_manifest(&format!("{MANIFEST_HEADER}\n{}\t1\ta", "00")).is_err());
        assert!(parse_manifest(&format!("{MANIFEST_HEADER}\n{}\t1\ta", "A".repeat(64))).is_err());
    }

    #[test]
    fn manifest_rejects_noncanonical_sizes_unsorted_paths_and_duplicates() {
        let digest = format_digest(&sha256(b"x"));
        assert!(parse_manifest(&format!("{MANIFEST_HEADER}\n{digest}\t01\ta")).is_err());
        assert!(parse_manifest(&format!(
            "{MANIFEST_HEADER}\n{digest}\t1\tb\n{digest}\t1\ta"
        )).is_err());
        assert!(parse_manifest(&format!(
            "{MANIFEST_HEADER}\n{digest}\t1\ta\n{digest}\t1\ta"
        )).is_err());
    }

    #[test]
    fn manifest_rejects_traversal_absolute_and_nonportable_paths() {
        let digest = format_digest(&sha256(b"x"));
        for path in ["../secret", "a/../secret", "/absolute", "C:/drive", "a\\b", "a//b", "CON.txt"] {
            let text = format!("{MANIFEST_HEADER}\n{digest}\t1\t{path}");
            assert!(parse_manifest(&text).is_err(), "accepted {path:?}");
        }
    }

    #[test]
    fn preflight_checks_sizes_and_hashes() {
        let root = temp_dir();
        fs::write(root.join("a.bin"), b"abc").expect("write test file");
        let entry = ManifestEntry { digest: sha256(b"abc"), bytes: 3, path: "a.bin".to_owned() };
        assert_eq!(preflight(&root, std::slice::from_ref(&entry), 10, 10).unwrap(), 3);
        let wrong_digest = ManifestEntry { digest: sha256(b"bad"), ..entry.clone() };
        assert!(preflight(&root, &[wrong_digest], 10, 10).is_err());
        let wrong_size = ManifestEntry { bytes: 4, ..entry };
        assert!(preflight(&root, &[wrong_size], 10, 10).is_err());
        fs::remove_dir_all(root).expect("remove temporary directory");
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_symlink_components() {
        use std::os::unix::fs::symlink;
        let root = temp_dir();
        let outside = root.with_extension("outside");
        fs::create_dir_all(&outside).expect("create outside directory");
        fs::write(outside.join("payload.bin"), b"x").expect("write outside file");
        symlink(&outside, root.join("linked")).expect("create symlink");
        assert!(resolve_entry(&root, "linked/payload.bin").is_err());
        fs::remove_dir_all(root).expect("remove root");
        fs::remove_dir_all(outside).expect("remove outside");
    }

    #[test]
    fn output_target_rejects_corpus_paths_and_existing_files() {
        let root = temp_dir();
        let manifest = root.join("manifest.tsv");
        fs::write(&manifest, "manifest").expect("write manifest");
        let canonical_manifest = fs::canonicalize(&manifest).unwrap();
        assert!(prepare_output_target(&root.join("inside.csv"), &root, &canonical_manifest).is_err());

        let outside = root.with_extension("out");
        fs::create_dir_all(&outside).expect("create output directory");
        fs::write(outside.join("existing.csv"), "old").expect("write existing output");
        assert!(prepare_output_target(&outside.join("existing.csv"), &root, &canonical_manifest).is_err());
        fs::remove_dir_all(root).expect("remove root");
        fs::remove_dir_all(outside).expect("remove output directory");
    }

    #[test]
    fn all_codecs_round_trip_empty_and_structured_inputs() {
        for input in [
            Vec::new(),
            vec![0x41; 1024],
            (0..4096).map(|index| (index & 0xff) as u8).collect(),
        ] {
            for codec in CODECS {
                let encoded = encode_codec(codec, &input).expect("encode");
                let decoded = decode_codec(codec, &encoded, input.len()).expect("decode");
                assert_eq!(decoded, input, "codec={}", codec.name());
                assert_eq!(encode_codec(codec, &input).expect("deterministic encode"), encoded);
            }
        }
    }

    #[test]
    fn csv_escaping_quotes_commas_and_line_breaks() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn percentile_and_median_use_order_statistics() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(nearest_rank(&values, 0.50), 3.0);
        assert_eq!(nearest_rank(&values, 0.95), 5.0);
        assert_eq!(nearest_rank(&values, 0.99), 5.0);
        let mut even = [4.0, 1.0, 3.0, 2.0];
        assert_eq!(median(&mut even), Some(2.5));
    }

    #[test]
    fn argument_parser_enforces_sample_and_file_size_bounds() {
        let base = [
            "--corpus-id", "sample",
            "--corpus-dir", "corpus",
            "--manifest", "manifest.tsv",
            "--output-csv", "results.csv",
        ];
        let args = base.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        assert!(parse_args(args).unwrap().is_some());

        let mut too_few = base.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        too_few.extend(["--samples".to_owned(), "4".to_owned()]);
        assert!(parse_args(too_few).is_err());

        let mut too_large = base.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        too_large.extend(["--max-file-bytes".to_owned(), (u32::MAX as u64 + 1).to_string()]);
        assert!(parse_args(too_large).is_err());
    }
}
