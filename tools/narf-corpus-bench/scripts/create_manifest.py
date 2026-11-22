#!/usr/bin/env python3
"""Create a deterministic SHA-256 TSV manifest for narf-corpus-bench."""

from __future__ import annotations

import argparse
import hashlib
import os
import stat
import sys
from pathlib import Path

HEADER = "sha256\tbytes\tpath"
HASH_BUFFER_BYTES = 64 * 1024
DEFAULT_MAX_FILE_BYTES = 64 * 1024 * 1024
DEFAULT_MAX_TOTAL_BYTES = 1024 * 1024 * 1024
MAX_ENTRIES = 10_000


def validate_relative_path(value: str) -> None:
    if (
        not value
        or value.startswith("/")
        or "\\" in value
        or ":" in value
        or any(character in value for character in "\t\n\r\0")
    ):
        raise ValueError(f"not a portable relative path: {value!r}")

    for component in value.split("/"):
        if (
            not component
            or component in {".", ".."}
            or component.endswith((".", " "))
            or any(ord(character) < 32 or character in '<>"|?*' for character in component)
        ):
            raise ValueError(f"non-portable path component: {component!r}")
        stem = component.split(".", 1)[0].upper()
        reserved = stem in {"CON", "PRN", "AUX", "NUL"} or (
            len(stem) == 4 and stem[:3] in {"COM", "LPT"} and stem[3] in "123456789"
        )
        if reserved:
            raise ValueError(f"reserved Windows path component: {component!r}")


def hash_file(path: Path, expected_size: int, max_file_bytes: int) -> str:
    if expected_size > max_file_bytes:
        raise ValueError(f"file exceeds max-file-bytes: {path}")
    digest = hashlib.sha256()
    total = 0
    with path.open("rb") as source:
        while True:
            block = source.read(HASH_BUFFER_BYTES)
            if not block:
                break
            total += len(block)
            if total > max_file_bytes:
                raise ValueError(f"file grew beyond max-file-bytes while hashing: {path}")
            digest.update(block)
    if total != expected_size:
        raise ValueError(f"file changed while hashing: {path}")
    return digest.hexdigest()


def collect_entries(root: Path, max_file_bytes: int, max_total_bytes: int) -> list[tuple[str, int, str]]:
    entries: list[tuple[str, int, str]] = []
    total_bytes = 0

    for current, directories, files in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        directories.sort(key=lambda name: name.encode("utf-8"))
        files.sort(key=lambda name: name.encode("utf-8"))

        for name in list(directories):
            candidate = current_path / name
            mode = candidate.lstat().st_mode
            if stat.S_ISLNK(mode):
                raise ValueError(f"symlink directory is not allowed: {candidate}")
            if not stat.S_ISDIR(mode):
                raise ValueError(f"non-directory entry encountered: {candidate}")

        for name in files:
            candidate = current_path / name
            mode = candidate.lstat().st_mode
            if stat.S_ISLNK(mode):
                raise ValueError(f"symlink file is not allowed: {candidate}")
            if not stat.S_ISREG(mode):
                raise ValueError(f"non-regular file is not allowed: {candidate}")

            relative = candidate.relative_to(root).as_posix()
            validate_relative_path(relative)
            size = candidate.stat().st_size
            if size > max_file_bytes:
                raise ValueError(f"file exceeds max-file-bytes: {relative}")
            total_bytes += size
            if total_bytes > max_total_bytes:
                raise ValueError("corpus exceeds max-total-bytes")
            digest = hash_file(candidate, size, max_file_bytes)
            entries.append((relative, size, digest))
            if len(entries) > MAX_ENTRIES:
                raise ValueError(f"corpus exceeds {MAX_ENTRIES} entries")

    entries.sort(key=lambda entry: entry[0].encode("utf-8"))
    return entries


def create_manifest(
    corpus_dir: Path,
    output: Path,
    max_file_bytes: int = DEFAULT_MAX_FILE_BYTES,
    max_total_bytes: int = DEFAULT_MAX_TOTAL_BYTES,
) -> int:
    if corpus_dir.is_symlink():
        raise ValueError("corpus root must not be a symlink")
    root = corpus_dir.resolve(strict=True)
    if not root.is_dir():
        raise ValueError("corpus root is not a directory")
    if max_file_bytes < 0 or max_total_bytes < 0:
        raise ValueError("size limits must be non-negative")

    output_parent = output.parent if str(output.parent) else Path(".")
    parent = output_parent.resolve(strict=True)
    target = parent / output.name
    if target == root or root in target.parents:
        raise ValueError("manifest output must be outside the corpus directory")
    if target.exists() or target.is_symlink():
        raise ValueError("refusing to overwrite an existing output")
    if target == root:
        raise ValueError("manifest output must not replace the corpus root")

    entries = collect_entries(root, max_file_bytes, max_total_bytes)
    if not entries:
        raise ValueError("corpus must contain at least one regular file")

    lines = [HEADER]
    lines.extend(f"{digest}\t{size}\t{relative}" for relative, size, digest in entries)
    payload = ("\n".join(lines) + "\n").encode("utf-8")

    # Exclusive creation avoids silently replacing an existing manifest.
    with target.open("xb") as destination:
        destination.write(payload)
        destination.flush()
        os.fsync(destination.fileno())
    return len(entries)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--max-file-bytes", type=int, default=DEFAULT_MAX_FILE_BYTES)
    parser.add_argument("--max-total-bytes", type=int, default=DEFAULT_MAX_TOTAL_BYTES)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        count = create_manifest(
            args.corpus_dir, args.output, args.max_file_bytes, args.max_total_bytes
        )
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    print(f"wrote {count} entries to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
