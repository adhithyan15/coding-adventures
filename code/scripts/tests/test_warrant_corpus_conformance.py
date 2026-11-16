"""Tests for the warrant-corpus-conformance-v1 checker (VER04i 9.1, WD-100)."""
from __future__ import annotations

import importlib.util
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "warrant_corpus_conformance.py"
spec = importlib.util.spec_from_file_location("warrant_corpus_conformance", SCRIPT)
conf = importlib.util.module_from_spec(spec)
sys.modules["warrant_corpus_conformance"] = conf
spec.loader.exec_module(conf)


class ReferenceReader(unittest.TestCase):
    def reject(self, data: bytes, reason: str) -> None:
        with self.assertRaises(conf.Reject) as caught:
            conf.read_json(data)
        self.assertEqual(caught.exception.reason, reason)

    def test_the_known_runner_bugs_are_refused(self) -> None:
        self.reject(b'{"a":1,"a":2}', "duplicate-key")
        self.reject(b"9007199254740993", "unsafe-integer")
        self.reject(b'"\\ud800"', "unpaired-surrogate")
        self.reject(b"1e2", "number-form")
        self.reject(b"\xef\xbb\xbf{}", "bom")
        self.reject(b"\xc0\x80", "utf8")
        self.reject(b"[" * 257 + b"]" * 257, "depth")

    def test_canonical_writer_rules(self) -> None:
        value = conf.read_json(b'{"10":1,"2":2,"\\ud83d\\ude00":3,"\\uffff":4,"d":"\x7f"}')
        text = conf.canonical(value)
        self.assertLess(text.index('"10"'), text.index('"2"'))
        self.assertLess(text.index("\\uffff"), text.index("\\ud83d\\ude00"))
        self.assertIn('"\\u007f"', text)
        self.assertTrue(text.endswith("}\n"))

    def test_containment_traps(self) -> None:
        self.assertFalse(conf.contains("win32", "D:\\repo", "C:\\tmp\\x")["inside"])
        self.assertTrue(conf.contains("win32", "D:\\repo", "D:\\repo\\..foo")["inside"])
        self.assertFalse(conf.contains("posix", "/r", "/rr")["inside"])

    def test_label_digest_binds_name_not_version(self) -> None:
        base = {"toolchain": {"name": "python", "version": "1"}}
        self.assertEqual(conf.label_digest(base), conf.label_digest({"toolchain": {"name": "python", "version": "2"}}))
        self.assertNotEqual(conf.label_digest(base), conf.label_digest({"toolchain": {"name": "node", "version": "1"}}))


class ReviewFindings(unittest.TestCase):
    def test_huge_integers_reject_cleanly(self) -> None:
        for digits in (17, 4301, 100_000):
            with self.assertRaises(conf.Reject) as caught:
                conf.read_json(b"9" * digits)
            self.assertEqual(caught.exception.reason, "unsafe-integer")

    def test_device_and_double_slash_paths_are_refused(self) -> None:
        for flavor, root, child in (
            ("win32", "D:\\r", "\\/?\\D:\\r"),
            ("win32", "D:\\r", "/\\?/D:/r"),
            ("win32", "D:\\r", "\\\\?/D:\\r"),
            ("win32", "D:\\r", "\\\\./D:\\r"),
            ("win32", "\\\\?\\D:\\r", "D:\\r\\x"),
            ("posix", "/", "//x"),
            ("posix", "//", "/r"),
        ):
            self.assertEqual(conf.contains(flavor, root, child), {"inside": False, "same": False})
            self.assertEqual(conf.alt_contains(flavor, root, child), {"inside": False, "same": False})
        self.assertTrue(conf.contains("posix", "/", "///x")["inside"])

    def test_the_second_opinion_is_a_different_algorithm_that_agrees(self) -> None:
        for _slug, _s, flavor, root, child, inside, same in conf.CONTAINMENT_CASES:
            self.assertEqual(conf.alt_contains(flavor, root, child), {"inside": inside, "same": same})

    def test_list_and_option_reference_encoder(self) -> None:
        u8 = lambda n: {"type": "U8", "value": str(n)}  # noqa: E731
        self.assertEqual(conf.encode_extended({"type": "List<U8>", "value": [u8(1), u8(2)]}, "List<U8>").hex(), "3002000000" + "10" + "01" + "00" * 7 + "10" + "02" + "00" * 7)
        self.assertEqual(conf.encode_extended({"type": "Option<U8>", "value": []}, "Option<U8>"), b"\x40")
        self.assertEqual(conf.encode_extended({"type": "Option<U8>", "value": [u8(0)]}, "Option<U8>").hex(), "41" + "10" + "00" * 8)
        with self.assertRaises(conf.ValueReject):
            conf.encode_extended({"type": "Option<U8>", "value": [u8(0), u8(1)]}, "Option<U8>")
        with self.assertRaises(conf.TypeReject):
            conf.decode_and_encode({"type": "List<U8>", "value": []}, "List<U8>", {})


class Fixture(unittest.TestCase):
    def test_checked_in_fixture_matches_the_reference(self) -> None:
        self.assertEqual(conf.main([]), 0)

    def test_tampered_fixture_is_caught(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp) / "fixture"
            shutil.copytree(conf.DEFAULT_FIXTURE, copy)
            path = copy / "json.json"
            path.write_bytes(path.read_bytes().replace(b'"reject"', b'"accept"', 1))
            self.assertEqual(conf.main(["--fixture", str(copy)]), 1)

    def test_extra_file_is_caught(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            copy = Path(tmp) / "fixture"
            shutil.copytree(conf.DEFAULT_FIXTURE, copy)
            (copy / "stray.json").write_text("{}\n")
            self.assertEqual(conf.main(["--fixture", str(copy)]), 1)

    def test_every_category_is_indexed(self) -> None:
        files = conf.build_files()
        index = conf.read_json(files["index.json"].encode())
        self.assertEqual({r["category"] for r in index["files"]}, {"json", "tagged", "containment", "label-digest"})


if __name__ == "__main__":
    unittest.main()
