"""Falsify the data/transport gate independently of a Warrant parser.

Expected syntax verdicts and subtree witnesses are authored data. This gate
must not ask the parser under test to invent those answers for itself.
"""

from __future__ import annotations

import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest import mock

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import tfv
import warrant_syntax_corpus as corpus


def baseline() -> dict:
    cases = []
    for feature in sorted(corpus.FEATURES):
        for accepted in (False, True):
            cases.append({
                "id": f"{feature}/{'pass' if accepted else 'reject'}",
                "feature": feature,
                "source": "warrant values\n  example simple:\n    true\n",
                "accept": accepted,
                "witnesses": ["literal=true"] if accepted else [],
            })
    return {"format": "warrant-syntax-v1", "cases": cases}


class CorpusTests(unittest.TestCase):
    def check_bad(self, mutate, message: str) -> None:
        document = baseline()
        mutate(document)
        self.assertTrue(any(message in p for p in corpus.validate_document(document)))

    def test_baseline_and_exact_transport_round_trip(self) -> None:
        document = baseline()
        self.assertEqual(corpus.validate_document(document), [])
        records = tfv.parse_file(corpus.generate_tfv(document))
        self.assertEqual(len(records), len(document["cases"]))
        for (record_id, fields), case in zip(records, document["cases"]):
            self.assertEqual(record_id, case["id"])
            self.assertEqual(fields, [
                ("source", ("text", case["source"])),
                ("accept", ("bool", case["accept"])),
                ("witnesses", ("list", [("text", s) for s in case["witnesses"]])),
            ])

    def test_closed_root_case_and_format(self) -> None:
        self.check_bad(lambda d: d.update(extra=True), "root fields")
        self.check_bad(lambda d: d.update(format="warrant-syntax-v2"), "format")
        self.check_bad(lambda d: d["cases"][0].update(extra=True), "case fields")
        self.check_bad(lambda d: d["cases"][0].pop("source"), "case fields")
        self.assertTrue(corpus.validate_document([]))

    def test_cases_need_a_bounded_nonempty_list(self) -> None:
        for invalid in ([], None, {}, "cases", baseline()["cases"] * 10):
            self.assertTrue(corpus.validate_document({"format": "warrant-syntax-v1", "cases": invalid}))

    def test_ids_are_exact_unique_and_bounded(self) -> None:
        for value in ("", "bad id", "x\ny", "é", "x" * 129, 4):
            self.check_bad(lambda d, v=value: d["cases"][0].update(id=v), "id")
        self.check_bad(lambda d: d["cases"][1].update(id=d["cases"][0]["id"]), "duplicate id")

    def test_feature_and_positive_negative_coverage(self) -> None:
        self.check_bad(lambda d: d["cases"][0].update(feature="invented"), "feature")
        self.check_bad(lambda d: d["cases"].pop(0), "baseline coverage")
        self.check_bad(lambda d: d.update(cases=[c for c in d["cases"] if c["feature"] != "values"]), "baseline coverage")

    def test_accept_is_bool_not_a_number_or_string(self) -> None:
        for value in (0, 1, "false", None, []):
            self.check_bad(lambda d, v=value: d["cases"][0].update(accept=v), "accept")

    def test_source_is_utf8_text_with_byte_limit(self) -> None:
        for value in (None, 1, "", "a" * 4097, "é" * 2049, "\ud800"):
            self.check_bad(lambda d, v=value: d["cases"][0].update(source=v), "source")
        document = baseline()
        document["cases"][0]["source"] = "é" * 2048
        self.assertEqual(corpus.validate_document(document), [])

    def test_witnesses_are_nonempty_for_acceptance_and_empty_for_rejection(self) -> None:
        self.check_bad(lambda d: d["cases"][0].update(witnesses=["literal=true"]), "rejected case")
        self.check_bad(lambda d: d["cases"][1].update(witnesses=[]), "accepted case")
        for value in (None, "literal=true", [True], ["bad"], ["literal="], ["literal=true\n"], ["literal=\ud800"]):
            self.check_bad(lambda d, v=value: d["cases"][1].update(witnesses=v), "witness")
        self.check_bad(lambda d: d["cases"][1].update(witnesses=["literal=true"] * 2), "duplicate witness")
        self.check_bad(lambda d: d["cases"][1].update(witnesses=[f"literal={i}" for i in range(33)]), "witness")

    def test_loader_rejects_duplicate_keys_and_nonfinite_json(self) -> None:
        for text in ('{"format":1,"format":2}', '{"x":NaN}', '{"x":Infinity}', '[' * 1500):
            with self.assertRaises(corpus.CorpusError):
                corpus.loads_document(text)

    def test_invalid_document_cannot_generate_transport(self) -> None:
        document = baseline()
        document["cases"][1]["accept"] = 1
        with self.assertRaises(corpus.CorpusError):
            corpus.generate_tfv(document)

    def test_cli_writes_lf_bytes_and_detects_byte_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "cases.json"
            target = Path(directory) / "cases.tfv"
            source.write_text(json.dumps(baseline()), encoding="utf-8")
            with mock.patch.object(corpus, "JSON_PATH", source), mock.patch.object(corpus, "TFV_PATH", target), redirect_stdout(io.StringIO()):
                self.assertEqual(corpus.main(["--write"]), 0)
                expected = corpus.generate_tfv(baseline()).encode("utf-8")
                self.assertEqual(target.read_bytes(), expected)
                self.assertNotIn(b"\r", expected)
                self.assertEqual(corpus.main([]), 0)
                target.write_bytes(expected.replace(b"\n", b"\r\n"))
                self.assertEqual(corpus.main([]), 1)
                source.write_bytes(b"x" * (corpus.MAX_JSON_BYTES + 1))
                self.assertEqual(corpus.main([]), 1)


if __name__ == "__main__":
    unittest.main()
