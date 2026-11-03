"""Falsify the CT01 transport guard independently of the Rust adapter."""

from __future__ import annotations

import copy
import importlib.util
import pathlib
import sys
import unittest

SCRIPTS = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import ct_compare_conformance as oracle
import tfv

SPEC = importlib.util.spec_from_file_location(
    "ct_compare_tfv", SCRIPTS / "ct_compare_tfv.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CtCompareTransportTests(unittest.TestCase):
    def setUp(self) -> None:
        self.document = oracle.parse_json(
            oracle.DEFAULT_CORPUS.read_text(encoding="utf-8")
        )

    def test_transport_keeps_every_identity_and_exact_u64(self) -> None:
        transport, dispatch = MODULE.render(self.document)
        records = tfv.parse_file(transport)
        self.assertEqual(
            [name for name, _ in records], [c["id"] for c in self.document["cases"]]
        )
        maximum = dict(
            next(fields for name, fields in records if name == "u64-max-equal")
        )
        self.assertEqual(maximum["left_u64"], ("int", 2**64 - 1))
        self.assertIn("fixed::<0>", dispatch)
        self.assertIn("fixed::<16>", dispatch)
        self.assertIn('Err("fixed width has no generated arm"', dispatch)
        MODULE.check_generated(self.document, transport, dispatch)

    def test_transport_and_dispatch_drift_fail_separately(self) -> None:
        transport, dispatch = MODULE.render(self.document)
        with self.assertRaisesRegex(ValueError, "cases.tfv"):
            MODULE.check_generated(
                self.document, transport.replace("I64", "I63", 1), dispatch
            )
        with self.assertRaisesRegex(ValueError, "fixed_widths.rs"):
            MODULE.check_generated(
                self.document, transport, dispatch.replace("fixed::<16>", "fixed::<15>")
            )

    def test_wrong_functional_answer_and_work_never_generate(self) -> None:
        for field in ("expected", "expected_work"):
            changed = copy.deepcopy(self.document)
            if field == "expected":
                changed["cases"][1][field]["equal"] = False
            else:
                changed["cases"][1][field]["byte_reads"] = 0
            with self.subTest(field=field), self.assertRaises(ValueError):
                MODULE.render(changed)

    def test_new_fixed_width_gets_a_real_const_generic_arm(self) -> None:
        changed = copy.deepcopy(self.document)
        row = next(c for c in changed["cases"] if c["id"] == "fixed-16-equal")
        row.update(fixed_size_bytes=3, left_hex="00aaff", right_hex="00aaff")
        row["expected_work"].update(byte_positions=3, byte_reads=6)
        _, dispatch = MODULE.render(changed)
        self.assertIn("3 => fixed::<3>", dispatch)
        self.assertIn("16 => fixed::<16>", dispatch)

    def test_canonical_bytes_and_repeatability(self) -> None:
        first = MODULE.render(self.document)
        self.assertEqual(first, MODULE.render(self.document))
        self.assertTrue(all(text.endswith("\n") and "\r" not in text for text in first))

    def test_duplicate_and_unknown_cases_reject(self) -> None:
        changed = copy.deepcopy(self.document)
        changed["cases"].append(copy.deepcopy(changed["cases"][0]))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            MODULE.render(changed)
        changed["cases"][-1]["operation"] = "not-an-operation"
        with self.assertRaisesRegex(ValueError, "unknown operation"):
            MODULE.render(changed)

    def test_checked_in_artifacts_are_current(self) -> None:
        MODULE.check_generated(
            self.document,
            MODULE.TRANSPORT.read_text(encoding="utf-8"),
            MODULE.DISPATCH.read_text(encoding="utf-8"),
        )


if __name__ == "__main__":
    unittest.main()
