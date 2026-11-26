from __future__ import annotations

import pathlib
import unittest

from reference_decoder import DecodeError, decode


ROOT = pathlib.Path(__file__).resolve().parents[2]
VECTOR_PATH = ROOT / "test-vectors" / "narf-frames.tsv"


def load_vectors():
    lines = VECTOR_PATH.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "id\tframe_hex\tdecoded_hex\trust_writer":
        raise AssertionError("unexpected conformance vector header")
    vectors = []
    for line in lines[1:]:
        if not line:
            continue
        identifier, frame_hex, decoded_hex, writer = line.split("\t")
        vectors.append((identifier, bytes.fromhex(frame_hex), bytes.fromhex(decoded_hex), writer))
    return vectors


def frame_v3(output_len: int, body: bytes, seed0: int = 0x41, seed1: int = 0x41) -> bytes:
    return (
        b"NXAR"
        + bytes((3, 0, 0))
        + output_len.to_bytes(8, "big")
        + bytes((seed0, seed1))
        + body
    )


class ReferenceDecoderTests(unittest.TestCase):
    def test_all_shared_vectors_decode(self):
        vectors = load_vectors()
        self.assertGreaterEqual(len(vectors), 12)
        for identifier, frame, expected, _writer in vectors:
            with self.subTest(vector=identifier):
                self.assertEqual(decode(frame), expected)

    def test_rejects_truncated_header_and_invalid_magic(self):
        with self.assertRaises(DecodeError):
            decode(b"NXAR")
        valid = load_vectors()[1][1]
        with self.assertRaises(DecodeError):
            decode(b"NOPE" + valid[4:])

    def test_rejects_unsupported_version_and_invalid_coefficients(self):
        valid = load_vectors()[1][1]
        with self.assertRaises(DecodeError):
            decode(valid[:4] + b"\x04" + valid[5:])
        invalid_coefficients = valid[:5] + b"\x04\x00" + valid[7:]
        with self.assertRaises(DecodeError):
            decode(invalid_coefficients)

    def test_rejects_output_limit_before_decoding(self):
        valid = load_vectors()[5][1]
        with self.assertRaises(DecodeError):
            decode(valid, max_output_len=16)

    def test_rejects_raw_mode_with_nonzero_seeds_or_wrong_body_length(self):
        valid_raw = load_vectors()[9][1]
        bad_seed = valid_raw[:15] + b"\x01" + valid_raw[16:]
        with self.assertRaises(DecodeError):
            decode(bad_seed)
        with self.assertRaises(DecodeError):
            decode(valid_raw[:-1])

    def test_rejects_v1_raw_mode(self):
        raw_v1 = b"NXAR" + bytes((1, 0xFF, 0xFF)) + (1).to_bytes(8, "big") + b"\x00\x00A"
        with self.assertRaises(DecodeError):
            decode(raw_v1)

    def test_rejects_short_v3_repeat_of_two(self):
        with self.assertRaises(DecodeError):
            decode(frame_v3(4, b"\x81A"))

    def test_rejects_extended_repeat_below_128(self):
        with self.assertRaises(DecodeError):
            decode(frame_v3(129, b"\xFF\x7FA"))

    def test_rejects_noncanonical_and_truncated_uleb128(self):
        with self.assertRaises(DecodeError):
            decode(frame_v3(130, b"\xFF\x80\x81\x00A"))
        with self.assertRaises(DecodeError):
            decode(frame_v3(130, b"\xFF\x80"))

    def test_rejects_truncated_literal_and_output_overrun(self):
        with self.assertRaises(DecodeError):
            decode(frame_v3(5, b"\x02AB"))
        with self.assertRaises(DecodeError):
            decode(frame_v3(4, b"\x82A\x00B"))

    def test_rejects_nonzero_unused_seed_and_trailing_output_mismatch(self):
        empty = b"NXAR" + bytes((3, 0, 0)) + (0).to_bytes(8, "big") + b"\x01\x00"
        with self.assertRaises(DecodeError):
            decode(empty)
        with self.assertRaises(DecodeError):
            decode(frame_v3(6, b"\x82A"))
        with self.assertRaises(DecodeError):
            decode(frame_v3(5, b"\x82A\x00B"))


if __name__ == "__main__":
    unittest.main()
