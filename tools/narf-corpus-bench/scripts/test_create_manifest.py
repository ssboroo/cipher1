import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("create_manifest.py")
SPEC = importlib.util.spec_from_file_location("create_manifest", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ManifestGeneratorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.root = self.base / "corpus"
        self.root.mkdir()
        self.output = self.base / "corpus.manifest.tsv"

    def tearDown(self):
        self.temp.cleanup()

    def test_generates_sorted_manifest_with_empty_files_and_known_hash(self):
        (self.root / "z.txt").write_bytes(b"hello")
        (self.root / "empty.bin").write_bytes(b"")
        nested = self.root / "nested"
        nested.mkdir()
        (nested / "a.txt").write_bytes(b"abc")

        count = MODULE.create_manifest(self.root, self.output)
        self.assertEqual(count, 3)
        rows = self.output.read_text(encoding="utf-8").splitlines()
        self.assertEqual(rows[0], MODULE.HEADER)
        self.assertEqual([row.split("\t", 2)[2] for row in rows[1:]],
                         ["empty.bin", "nested/a.txt", "z.txt"])
        self.assertIn(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\t3\tnested/a.txt",
            rows,
        )

    def test_refuses_output_inside_corpus_and_existing_output(self):
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.root / "manifest.tsv")
        self.output.write_text("existing", encoding="utf-8")
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.output)

    def test_rejects_empty_corpus_and_size_limits(self):
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.output)
        (self.root / "large.bin").write_bytes(b"12345")
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.output, max_file_bytes=4)
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.output, max_total_bytes=4)

    def test_rejects_nonportable_and_reserved_paths(self):
        for value in ["../secret", "a//b", "CON.txt", "a\\b", "trailing."]:
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    MODULE.validate_relative_path(value)

    def test_rejects_symlink_entries_when_supported(self):
        target = self.base / "outside.txt"
        target.write_text("outside", encoding="utf-8")
        link = self.root / "linked.txt"
        try:
            link.symlink_to(target)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks are unavailable on this platform")
        with self.assertRaises(ValueError):
            MODULE.create_manifest(self.root, self.output)


if __name__ == "__main__":
    unittest.main()
