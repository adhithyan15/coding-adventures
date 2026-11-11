"""Falsification tests for the independent WD-003 Python oracle."""
from __future__ import annotations

import contextlib
import copy
import io
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/warrant_corpus_oracle.py"
_spec = importlib.util.spec_from_file_location("warrant_corpus_oracle", SCRIPT)
assert _spec and _spec.loader
oracle = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(oracle)

SPEC_TEXT = """warrant ct_compare
  error SelectError = CT_SELECT_LENGTH_MISMATCH
  model same(a: Bytes, b: Bytes) -> Bool = a == b
  model same_u64(a: U64, b: U64) -> Bool = a == b
  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool
  example fixed_zero:
    eq_fixed<0>(b"", b"") == true
  claim eq_fixed_agrees_w0:
    forall a: Bytes<0>, b: Bytes<0> | len(a) == len(b). eq_fixed<0>(a, b) == same(a, b)
"""


def valid_document():
    ops = oracle._source_ops(SPEC_TEXT)
    models = oracle._models_from_spec(SPEC_TEXT)
    claims = oracle._claim_definitions(SPEC_TEXT, ops, models)
    examples = oracle._source_examples(SPEC_TEXT, ops, models)
    claim = claims["eq_fixed_agrees_w0"]
    domains = [oracle._domain_spec(row[1])[0] for row in claim["binders"]]
    types = [row[1]["type"] for row in claim["binders"]]
    prefix = oracle._stream_prefix(claim["id"], types, claim["result_type"])
    inputs = hashlib.sha256(prefix)
    expected = hashlib.sha256(prefix)
    count = 0
    for values in __import__("itertools").product(*domains):
        env = dict(zip((row[0] for row in claim["binders"]), values))
        if not oracle._eval(claim["admit"], env, models):
            continue
        for value, ty in zip(values, types):
            inputs.update(oracle._encode(value, ty))
        expected.update(oracle._encode(oracle._eval(claim["rhs"], env, models), claim["result_type"]))
        count += 1
    suite = {
        **claim,
        "kind": "exhaustive",
        "cases": count,
        "discarded": 0,
        "expected_calls": count,
        "inputs_sha256": inputs.hexdigest(),
        "expected_sha256": expected.hexdigest(),
    }
    faults = oracle._fault_rows(ops)
    return {
        "schema_version": 1,
        "profile": "warrant-corpus-v1",
        "module": "ct_compare",
        "spec": "code/specs/ct-compare.warrant",
        "spec_sha256": hashlib.sha256(SPEC_TEXT.encode()).hexdigest(),
        "generator": {"name": "warrant-grammar", "version": "0.4.0"},
        "types": oracle._types_from_spec(SPEC_TEXT),
        "ops": ops,
        "models": models,
        "examples": examples,
        "suites": [suite],
        "faults": faults,
        "controls": oracle._control_rows(faults, claims),
    }


class WarrantCorpusOracleTests(unittest.TestCase):
    def validate(self, doc=None):
        return oracle.validate_document(copy.deepcopy(doc or valid_document()), SPEC_TEXT)

    def test_stream_framing_golden_empty_one_and_multiple(self):
        prefix = oracle._stream_prefix("warrant/x", ["Bool"], "Bool")
        self.assertEqual(prefix.hex(), "77617272616e742d73747265616d2d7631000900000077617272616e742f780100000004000000426f6f6c04000000426f6f6c")
        empty = hashlib.sha256(prefix).hexdigest()
        one = hashlib.sha256(prefix + oracle._encode(False, "Bool")).hexdigest()
        multiple = hashlib.sha256(prefix + oracle._encode(False, "Bool") + oracle._encode(True, "Bool")).hexdigest()
        self.assertEqual(empty, "c3edd4858a709d95b479823f02b56445e996bed4652838ddf697ff20921a0d35")
        self.assertEqual(one, "efc37a58bf04b82ca409416f5b4482500d94b5cc0c253d452c615ac781a424bb")
        self.assertEqual(multiple, "33196c746aa4c043a7782c18afc11a6b875a78327be77436be22aa5fd2b0ddda")
        self.assertNotEqual(prefix, oracle._stream_prefix("warrant/y", ["Bool"], "Bool"))
        self.assertNotEqual(prefix, oracle._stream_prefix("warrant/x", ["Bytes"], "Bool"))
        self.assertNotEqual(prefix, oracle._stream_prefix("warrant/x", ["Bool", "Bool"], "Bool"))

    def test_minimal_independent_profile_validates(self):
        self.assertEqual(self.validate(), {"eq_fixed_agrees_w0": 1})

    def test_wrong_subject_output_cannot_change_expected_model_stream(self):
        doc = valid_document()
        # A subject-side change is validated structurally; expected values and
        # digest remain derived exclusively from the pure `same` model.
        doc["suites"][0]["lhs"] = ["subject_op", "eq", [["var", "a"], ["var", "b"]]]
        self.assertEqual(doc["suites"][0]["expected_sha256"], valid_document()["suites"][0]["expected_sha256"])
        with self.assertRaises(oracle.CorpusError) as caught:
            self.validate(doc)
        self.assertEqual(caught.exception.code, "suite-source")

    def test_domain_order_count_and_digests_are_recomputed(self):
        for mutate, code in (
            (lambda d: d["suites"][0]["binders"][0][1].__setitem__("domain", "bytes_n(1)"), "suite-source"),
            (lambda d: d["suites"][0].__setitem__("cases", 2), "suite-count"),
            (lambda d: d["suites"][0].__setitem__("inputs_sha256", "0" * 64), "input-digest"),
            (lambda d: d["suites"][0].__setitem__("expected_sha256", "0" * 64), "expected-digest"),
        ):
            with self.subTest(code=code):
                doc = valid_document(); mutate(doc)
                with self.assertRaises(oracle.CorpusError) as caught:
                    self.validate(doc)
                self.assertEqual(caught.exception.code, code)

    def test_enumeration_order_change_breaks_input_digest(self):
        doc = valid_document()
        original = oracle.domain
        def reversed_domain(name, parameter=None):
            values = original(name, parameter)
            return [b"\x00"] if name == "bytes_n" and parameter == 0 else values
        with mock.patch.object(oracle, "domain", side_effect=reversed_domain):
            with self.assertRaises(oracle.CorpusError) as caught:
                self.validate(doc)
        self.assertEqual(caught.exception.code, "input-digest")

    def test_model_rhs_mutation_is_bound_to_source_claim(self):
        doc = valid_document()
        doc["suites"][0]["rhs"] = ["not", ["var", "a"]]
        with self.assertRaises(oracle.CorpusError) as caught:
            self.validate(doc)
        self.assertEqual(caught.exception.code, "suite-source")

    def test_unknown_subject_nodes_and_forbidden_model_calls_fail_closed(self):
        doc = valid_document(); doc["suites"][0]["rhs"] = ["mystery", 1]
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "suite-source")
        doc = valid_document(); doc["suites"][0]["rhs"] = ["op", "eq", []]
        with self.assertRaises(oracle.CorpusError): self.validate(doc)
        doc = valid_document(); doc["suites"][0]["lhs"] = ["op", "eq", []]
        with self.assertRaises(oracle.CorpusError): self.validate(doc)

    def test_sharding_and_warrant_v1_forms_are_rejected(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            oracle.main(["--shard", "0/2"])
        for tag in ("quantifier", "lambda", "match"):
            doc = valid_document(); doc["suites"][0]["rhs"] = [tag]
            with self.subTest(tag=tag), self.assertRaises(oracle.CorpusError):
                self.validate(doc)

    def test_examples_are_exactly_bound_to_source_declarations(self):
        for mutate in (
            lambda d: d["examples"][0].__setitem__("id", "warrant/ct_compare/example/changed"),
            lambda d: d["examples"][0].__setitem__("op", "eq"),
            lambda d: d["examples"][0]["args"][0].__setitem__("value", "01"),
            lambda d: d["examples"][0].__setitem__("expected", False),
        ):
            doc = valid_document(); mutate(doc)
            with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
            self.assertEqual(caught.exception.code, "example-source")
        doc = valid_document(); doc["examples"].clear()
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "example-source")

    def test_unsupported_fixed_width_is_rejected_before_domain_materialization(self):
        for operation in (
            lambda: oracle._domain_name_for_type("Bytes<4096>"),
            lambda: oracle._domain_spec({"domain": "bytes_n(4096)", "type": "Bytes<4096>"}),
        ):
            with self.assertRaises(oracle.CorpusError): operation()

    def test_faulted_operation_without_claim_target_is_rejected(self):
        orphan_spec = SPEC_TEXT.replace(
            "  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool",
            "  op eq(a: Bytes, b: Bytes) -> Bool\n  op eq_fixed<N: Nat>(a: Bytes<N>, b: Bytes<N>) -> Bool",
        )
        ops = oracle._source_ops(orphan_spec)
        faults = oracle._fault_rows(ops)
        claims = oracle._claim_definitions(orphan_spec, ops, oracle._models_from_spec(orphan_spec))
        with self.assertRaises(oracle.CorpusError) as caught:
            oracle._control_rows(faults, claims)
        self.assertEqual(caught.exception.code, "faults")

    def test_faults_and_controls_are_complete_and_claim_bound(self):
        doc = valid_document(); doc["faults"].pop()
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "faults")
        doc = valid_document(); doc["faults"].append(dict(doc["faults"][0]))
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "faults")
        doc = valid_document(); doc["controls"].pop()
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "controls")
        doc = valid_document(); doc["controls"][0]["required_claims"] = []
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "controls")
        doc = valid_document(); doc["controls"].append(dict(doc["controls"][0]))
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "controls")

    def test_suite_source_binding_catches_claim_mutations(self):
        mutations = (
            lambda s: s.__setitem__("lhs", ["subject_op", "eq", [["var", "a"], ["var", "b"]]]),
            lambda s: s.__setitem__("lhs", ["subject_op", "eq_fixed", [["var", "a"], ["var", "b"]], [1]]),
            lambda s: s["lhs"][2].__setitem__(0, ["var", "b"]),
            lambda s: s["binders"][0][1].__setitem__("domain", "bytes_n(1)"),
            lambda s: s.__setitem__("admit", ["lit", False]),
            lambda s: s.__setitem__("ops", ["eq"]),
            lambda s: s.__setitem__("rhs", ["lit", False]),
        )
        for mutate in mutations:
            doc = valid_document(); mutate(doc["suites"][0])
            with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
            self.assertEqual(caught.exception.code, "suite-source")
        doc = valid_document(); doc["suites"][0]["expected_calls"] = 0
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "suite-calls")
        doc = valid_document(); doc["ops"]["eq_fixed"]["width_params"] = []
        with self.assertRaises(oracle.CorpusError) as caught: self.validate(doc)
        self.assertEqual(caught.exception.code, "spec-op")

    def test_malformed_operation_declaration_is_not_silently_ignored(self):
        doc = valid_document()
        source = SPEC_TEXT + "  op unsupported malformed declaration\n"
        doc["spec_sha256"] = hashlib.sha256(source.encode()).hexdigest()
        with self.assertRaises(oracle.CorpusError) as caught:
            oracle.validate_document(doc, source)
        self.assertEqual(caught.exception.code, "spec-op")

    def test_exact_warrant_v0_header_rejects_v1_source(self):
        doc = valid_document()
        source = SPEC_TEXT.replace("warrant ct_compare", "warrant v1 ct_compare", 1)
        doc["spec_sha256"] = hashlib.sha256(source.encode()).hexdigest()
        with self.assertRaises(oracle.CorpusError) as caught:
            oracle.validate_document(doc, source)
        self.assertEqual(caught.exception.code, "spec")
        source = SPEC_TEXT.replace("  claim eq_fixed_agrees_w0:", "  property future_form:\n    forall a: Bool. true\n  claim eq_fixed_agrees_w0:")
        doc = valid_document(); doc["spec_sha256"] = hashlib.sha256(source.encode()).hexdigest()
        with self.assertRaises(oracle.CorpusError) as caught:
            oracle.validate_document(doc, source)
        self.assertEqual(caught.exception.code, "spec")

    def test_rehashed_source_with_unparsed_operation_is_rejected(self):
        source = SPEC_TEXT + "  op unsupported malformed declaration\n"
        doc = valid_document(); doc["spec_sha256"] = hashlib.sha256(source.encode()).hexdigest()
        with self.assertRaises(oracle.CorpusError) as caught:
            oracle.validate_document(doc, source)
        self.assertEqual(caught.exception.code, "spec-op")

    def test_limited_reader_rejects_symlinks_and_oversized_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            target = root / "target"; target.write_bytes(b"{}"); link = root / "link"
            link.symlink_to(target)
            with self.assertRaises(oracle.CorpusError) as caught: oracle._read_limited(link, 16)
            self.assertEqual(caught.exception.code, "io")
            with self.assertRaises(oracle.CorpusError) as caught: oracle._read_limited(target, 1)
            self.assertEqual(caught.exception.code, "input-limit")

    def test_malformed_duplicate_and_truncated_json_fail(self):
        for text in ('{"x":1,"x":2}', '{"schema_version":1', ''):
            with self.subTest(text=text), self.assertRaises(oracle.CorpusError) as caught:
                oracle.parse_json(text)
            self.assertEqual(caught.exception.code, "malformed-json")

    def test_tfv_expected_value_mismatch_is_rejected(self):
        example = {"id": "ex", "args": [], "op": "eq", "expected": True, "result_type": "Bool", "width": None}
        with self.assertRaises(oracle.CorpusError):
            oracle._check_tfv("tfv 1\nex R2 Kop Seq Kexpect F\n", [example])

    def test_tfv_result_error_uses_the_tfv_map_and_text_encodings(self):
        example = {"id": "ex", "args": [], "op": "select_bytes", "expected": {"type": "Result<Bytes,SelectError>", "value": {"err": "CT_SELECT_LENGTH_MISMATCH"}}, "result_type": "Result<Bytes,SelectError>", "width": None}
        oracle._check_tfv("tfv 1\nex R2 Kop S73656c6563745f6279746573 Kexpect M1 S657272 S43545f53454c4543545f4c454e4754485f4d49534d41544348\n", [example])


if __name__ == "__main__":
    unittest.main()
