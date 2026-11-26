"""Independent, standard-library NARF frame decoder used by conformance tests."""

from __future__ import annotations

import struct

MAGIC = b"NXAR"
HEADER_SIZE = 17
RAW_MODE = 0xFF
CANDIDATES = {0, 1, 2, 3, 5, 8, 13, 21}
USIZE_BITS = struct.calcsize("P") * 8
USIZE_MAX = (1 << USIZE_BITS) - 1
DEFAULT_MAX_OUTPUT = 64 * 1024 * 1024


class DecodeError(ValueError):
    """Raised when a NARF frame is malformed or exceeds its output limit."""


def _read_uleb128(body: bytes, offset: int) -> tuple[int, int]:
    value = 0
    for index in range(10):
        position = offset + index
        if position >= len(body):
            raise DecodeError("truncated extended repeat length")
        byte = body[position]
        part = byte & 0x7F
        shift = index * 7
        if shift >= USIZE_BITS or part > (USIZE_MAX >> shift):
            raise DecodeError("extended repeat length overflows usize")
        value |= part << shift
        if byte & 0x80 == 0:
            if index > 0 and part == 0:
                raise DecodeError("non-canonical unsigned LEB128")
            return value, index + 1
    raise DecodeError("extended repeat length is too long")


def _decode_residuals(body: bytes, expected: int, version: int) -> bytes:
    output = bytearray()
    cursor = 0

    while cursor < len(body):
        control = body[cursor]
        cursor += 1

        if version == 3 and control == 0xFF:
            count, consumed = _read_uleb128(body, cursor)
            if count < 128:
                raise DecodeError("extended repeat length is below 128")
            cursor += consumed
            if count > expected - len(output):
                raise DecodeError("extended repeat exceeds declared output")
            if cursor >= len(body):
                raise DecodeError("missing extended repeat value")
            output.extend(bytes((body[cursor],)) * count)
            cursor += 1
            continue

        count = (control & 0x7F) + 1
        if control & 0x80:
            if version == 3 and count < 3:
                raise DecodeError("version-3 short repeat is shorter than three")
            if count > expected - len(output):
                raise DecodeError("repeat exceeds declared output")
            if cursor >= len(body):
                raise DecodeError("missing repeat value")
            output.extend(bytes((body[cursor],)) * count)
            cursor += 1
        else:
            if count > expected - len(output):
                raise DecodeError("literal exceeds declared output")
            end = cursor + count
            if end > len(body):
                raise DecodeError("truncated literal")
            output.extend(body[cursor:end])
            cursor = end

    if len(output) != expected:
        raise DecodeError("decoded residual length does not match declaration")
    return bytes(output)


def decode(frame: bytes, max_output_len: int = DEFAULT_MAX_OUTPUT) -> bytes:
    """Decode a NARF v1/v2/v3 frame without depending on the Rust crate."""
    if len(frame) < HEADER_SIZE:
        raise DecodeError("truncated header")
    if frame[:4] != MAGIC:
        raise DecodeError("invalid magic")

    version = frame[4]
    if version not in (1, 2, 3):
        raise DecodeError("unsupported version")

    alpha, beta = frame[5], frame[6]
    declared = int.from_bytes(frame[7:15], "big")
    if declared > USIZE_MAX:
        raise DecodeError("declared length does not fit usize")
    if declared > max_output_len:
        raise DecodeError("declared length exceeds output limit")

    seed0, seed1 = frame[15], frame[16]
    body = frame[HEADER_SIZE:]

    if alpha == RAW_MODE and beta == RAW_MODE:
        if version == 1 or seed0 != 0 or seed1 != 0:
            raise DecodeError("invalid raw-mode parameters")
        if len(body) != declared:
            raise DecodeError("raw body length does not match declaration")
        return body

    if alpha == RAW_MODE or beta == RAW_MODE:
        raise DecodeError("only one raw-mode coefficient is set")
    if alpha not in CANDIDATES or beta not in CANDIDATES:
        raise DecodeError("coefficient is outside the candidate set")
    if (declared == 0 and (seed0 != 0 or seed1 != 0)) or (
        declared == 1 and seed1 != 0
    ):
        raise DecodeError("unused seed bytes must be zero")

    residual_count = max(declared - 2, 0)
    residuals = _decode_residuals(body, residual_count, version)

    if declared == 0:
        return b""
    if declared == 1:
        return bytes((seed0,))

    output = bytearray((seed0, seed1))
    for residual in residuals:
        value = (residual + alpha * output[-1] + beta * output[-2]) & 0xFF
        output.append(value)
    return bytes(output)
