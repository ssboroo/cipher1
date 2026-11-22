# Corpus Manifest Generator

The Python standard-library helper creates the sorted SHA-256 TSV manifest required by the Rust benchmark runner. It does not download datasets, validate licenses, or establish corpus provenance.

## Generate a manifest

Run from the repository root:

```bash
python3 tools/narf-corpus-bench/scripts/create_manifest.py \
  --corpus-dir /data/my-corpus \
  --output /data/my-corpus.manifest.tsv
```

The output is created exclusively and must not already exist. It must be outside the corpus directory. The generator recursively enumerates regular files, rejects symlink entries and non-portable paths, sorts paths by UTF-8 byte order, and writes the exact `sha256<TAB>bytes<TAB>path` header and rows expected by the Rust runner.

Defaults match the runner: 64 MiB per file, 1 GiB total, and 10,000 entries. Override size limits explicitly when the corresponding runner invocation uses matching limits.

## Tests

```bash
python3 -m unittest discover -s tools/narf-corpus-bench/scripts -p 'test_*.py' -v
```

## Limitations

The generator is a convenience tool, not a security boundary. It checks symlinks before reading, but ordinary portable filesystem APIs cannot eliminate all concurrent filesystem mutation races. Keep the corpus immutable while generating and benchmarking. Review the source, version, retrieval date, and license of every corpus before use; a checksum proves byte consistency against the manifest, not authenticity or permission to use or redistribute the data.
