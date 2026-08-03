from __future__ import annotations

import importlib
import shutil
import socket
import sys
import tempfile
import unittest
from contextlib import ExitStack, contextmanager
from itertools import pairwise
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

provenance = importlib.import_module("adj_stdlib_provenance")
builder = importlib.import_module("build_adj_proportion_provenance")
migration = importlib.import_module("migrate_adj_formula_inventories")


def math_fraction() -> bytes:
    return (
        b"<math><mrow><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>=</mo>"
        b"<mfrac><mi>c</mi><mi>d</mi></mfrac><mo>,</mo></mrow></math>"
    )


def math_domain(punctuation: str) -> bytes:
    return (
        "<math><mrow><mi>b</mi><mo>\u2260</mo><mn>0</mn><mo>,</mo>"
        f"<mi>d</mi><mo>\u2260</mo><mn>0</mn><mo>{punctuation}</mo></mrow></math>"
    ).encode()


def synthetic_capture() -> bytes:
    definition = (
        b'<p id="fs-id1166492018126">A proportion is an equation of the form '
        b"<span>"
        + math_fraction()
        + b"</span> where <span>"
        + math_domain(".")
        + b"</span></p>"
    )
    cross_products = (
        b'<p id="fs-id1517408">For any proportion of the form '
        b"<span>"
        + math_fraction()
        + b"</span> where <span>"
        + math_domain(",")
        + b"</span> its cross products are equal.</p>"
    )
    return (
        b"reviewed header\n"
        + definition
        + b"\nintervening material\n"
        + cross_products
        + b"\nfooter"
    )


def receipt_hash(body: bytes) -> str:
    receipt = provenance.build_fetch_receipt(
        locator=builder.LOCATOR,
        final_locator=builder.LOCATOR,
        retrieved_at=builder.SOURCE_RETRIEVED_AT,
        status=200,
        media_type="text/html",
        body_sha256=provenance.sha256_bytes(body),
        body_size=len(body),
        headers={
            "content-length": "536280",
            "content-type": "text/html",
            "etag": '"c940edde2f4478001fa42e29db51cea2"',
            "last-modified": "Mon, 15 Jun 2026 17:25:34 GMT",
        },
    )
    return provenance.sha256_bytes(provenance.canonical_json_bytes(receipt))


def claim_specs(body: bytes) -> tuple[tuple[str, bytes, int, int, str], ...]:
    values = []
    for claim_id, marker in (
        (builder.DEFINITION_CLAIM, b'id="fs-id1166492018126"'),
        (builder.PROPORTION_CLAIM, b'id="fs-id1517408"'),
    ):
        start = body.index(b"<p " + marker)
        end = body.index(b"</p>", start) + 4
        values.append(
            (claim_id, marker, start, end, provenance.sha256_bytes(body[start:end]))
        )
    return tuple(values)


@contextmanager
def reviewed_source(body: bytes):
    with ExitStack() as stack:
        stack.enter_context(
            mock.patch.object(builder, "RAW_HASH", provenance.sha256_bytes(body))
        )
        stack.enter_context(mock.patch.object(builder, "RAW_SIZE", len(body)))
        stack.enter_context(
            mock.patch.object(builder, "RECEIPT_HASH", receipt_hash(body))
        )
        stack.enter_context(
            mock.patch.object(builder, "CLAIM_SPECS", claim_specs(body))
        )
        yield


def source_ir(cas: provenance.Cas, digest: str) -> dict:
    return provenance._json_object(cas, digest, "source_ir")


def claim_ids(ir: dict) -> set[str]:
    return {
        item["claim_id"]
        for segment in ir["segments"]
        for item in segment.get("claims", [])
    }


class ProportionProvenanceTests(unittest.TestCase):
    def test_source_projection_is_complete_and_retained_replay_is_offline(self) -> None:
        body = synthetic_capture()
        with tempfile.TemporaryDirectory() as directory, reviewed_source(body):
            root = Path(directory)
            capture = root / "proportion.html"
            capture.write_bytes(body)
            cas = provenance.Cas(root / "cas")
            first_source, first_claims = builder.retained_external_source(cas, capture)

            with mock.patch.object(
                socket,
                "create_connection",
                side_effect=AssertionError("offline replay attempted network access"),
            ):
                replayed_source, replayed_claims = builder.retained_external_source(
                    cas, None
                )

            self.assertEqual(replayed_source, first_source)
            self.assertEqual(replayed_claims, first_claims)
            raw_ir = source_ir(cas, first_source["source_ir_sha256"])
            self.assertEqual(raw_ir["segments"][0]["start"], 0)
            self.assertEqual(raw_ir["segments"][-1]["end"], len(body))
            self.assertTrue(
                all(
                    left["end"] == right["start"]
                    for left, right in zip(raw_ir["segments"], raw_ir["segments"][1:])
                )
            )
            self.assertEqual(
                claim_ids(raw_ir),
                {builder.DEFINITION_CLAIM, builder.PROPORTION_CLAIM},
            )
            self.assertEqual(len(first_source["representations"]), 2)
            representation = first_source["representations"][1]
            rendered_ir = source_ir(cas, representation["source_ir_sha256"])
            self.assertEqual(claim_ids(rendered_ir), {builder.PROPORTION_CLAIM})
            transform = provenance._json_object(
                cas, representation["transform_sha256"], "text_transform"
            )
            self.assertEqual(transform["result_sha256"], builder.RENDERED_HASH)
            self.assertEqual(transform["result_size"], len(builder.SELECTED_TEXT))
            self.assertEqual(
                [operation["operation"] for operation in transform["operations"]],
                [
                    "discard",
                    "copy",
                    "discard",
                    "mathml_to_infix",
                    "discard",
                    "copy",
                    "discard",
                    "mathml_to_infix",
                    "discard",
                    "copy",
                    "discard",
                ],
            )

    def test_projection_explicitly_accounts_for_every_represented_byte(self) -> None:
        body = synthetic_capture()
        for spec in claim_specs(body):
            with self.subTest(claim_id=spec[0]):
                operations = builder._projection_operations(body, spec)
                self.assertEqual(operations[0]["source_start"], spec[2])
                self.assertEqual(operations[-1]["source_end"], spec[3])
                self.assertTrue(
                    all(
                        left["source_end"] == right["source_start"]
                        for left, right in pairwise(operations)
                    )
                )
                discards = [
                    operation
                    for operation in operations
                    if operation["operation"] == "discard"
                ]
                self.assertGreater(len(discards), 0)
                self.assertTrue(
                    all(
                        operation["claim_id"] == spec[0]
                        and operation["reason"].strip()
                        and operation["result_start"] == operation["result_end"]
                        for operation in discards
                    )
                )

    def test_capture_hash_and_reviewed_claim_spans_fail_closed(self) -> None:
        body = synthetic_capture()
        changed = body.replace(b"cross products", b"cross-product", 1)
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory) / "changed.html"
            capture.write_bytes(changed)
            cas = provenance.Cas(Path(directory) / "cas")
            with self.assertRaisesRegex(provenance.ProvenanceError, "reviewed SHA-256"):
                builder.retained_external_source(cas, capture)

        with tempfile.TemporaryDirectory() as directory, reviewed_source(changed):
            capture = Path(directory) / "changed.html"
            capture.write_bytes(changed)
            cas = provenance.Cas(Path(directory) / "cas")
            original_specs = claim_specs(body)
            with (
                mock.patch.object(builder, "CLAIM_SPECS", original_specs),
                self.assertRaisesRegex(provenance.ProvenanceError, "byte span drifted"),
            ):
                builder.retained_external_source(cas, capture)

    def test_build_materializes_five_roots_with_input_and_source_ir(self) -> None:
        body = synthetic_capture()
        with tempfile.TemporaryDirectory() as directory, reviewed_source(body):
            workspace = Path(directory)
            for repo_path in (
                builder.LIBRARY,
                builder.FIXTURE,
                *(query_path for _bundle_id, query_path, _facts in builder.QUERY_SPECS),
            ):
                destination = workspace / repo_path
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(builder.REPO_ROOT / repo_path, destination)
            capture = workspace / "proportion.html"
            capture.write_bytes(body)
            cas = provenance.Cas(workspace / "cas")
            arithmetic_hash = cas.put_json(
                {
                    "bundle_id": "adj.math.arithmetic.primitives.v1",
                    "kind": "provenance_bundle",
                },
                kind="provenance_bundle",
                label="test arithmetic dependency",
            )
            audit_calls = []

            def put_inventory(
                selected_cas: provenance.Cas,
                source_hash: str,
                _command: list[str],
                *,
                label: str,
            ) -> str:
                return selected_cas.put_json(
                    {
                        "contract": "test/formula_inventory/v2",
                        "kind": "formula_parser_inventory",
                        "source_sha256": source_hash,
                    },
                    kind="formula_parser_inventory",
                    label=label,
                    links=[source_hash],
                )

            def put_execution(
                selected_cas: provenance.Cas,
                query_bundle: dict,
                command: list[str],
                *,
                label: str,
            ) -> tuple[list[str], list[str]]:
                audit_calls.append((query_bundle["bundle_id"], command, label))
                derivation = selected_cas.put_json(
                    {
                        "contract": "adj-lang/formula_derivation/v2",
                        "query": query_bundle["library"],
                        "schema_version": 2,
                    },
                    kind="formula_derivation",
                    label=f"{label} derivation",
                )
                witness = selected_cas.put_json(
                    {
                        "contract": "adj-lang/formula_execution/v2",
                        "formula_derivation_sha256": derivation,
                        "schema_version": 2,
                    },
                    kind="execution_witness",
                    label=f"{label} witness",
                    links=[derivation],
                )
                return [derivation], [witness]

            original_root = builder.REPO_ROOT
            builder.REPO_ROOT = workspace
            try:
                with (
                    mock.patch.object(
                        provenance, "put_formula_parser_inventory", put_inventory
                    ),
                    mock.patch.object(
                        provenance, "put_formula_execution_evidence", put_execution
                    ),
                ):
                    roots = builder.build(
                        cas,
                        capture,
                        arithmetic_bundle_sha256=arithmetic_hash,
                        formula_inventory_command=["inventory"],
                        formula_audit_command=["audit", "--contract", "v2"],
                    )
            finally:
                builder.REPO_ROOT = original_root

            expected_ids = {"adj.math.arithmetic.proportion.v1"} | {
                bundle_id for bundle_id, _query_path, _facts in builder.QUERY_SPECS
            }
            self.assertEqual(set(roots), expected_ids)
            self.assertEqual(len(audit_calls), 4)
            self.assertTrue(
                all(call[1] == ["audit", "--contract", "v2"] for call in audit_calls)
            )
            library = provenance._json_object(
                cas, roots["adj.math.arithmetic.proportion.v1"], "provenance_bundle"
            )
            self.assertEqual(library["dependencies"], [arithmetic_hash])
            self.assertEqual(
                claim_ids(source_ir(cas, library["input"]["source_ir_sha256"])),
                {
                    "adj.code.arithmetic.proportion.import.arithmetic",
                    "adj.code.arithmetic.proportion.use.proportion_vocab",
                    "adj.code.arithmetic.proportion.vocabulary",
                    builder.PROPORTION_CLAIM,
                },
            )
            fixture_ir_hashes = set()
            for bundle_id in expected_ids - {library["bundle_id"]}:
                query = provenance._json_object(
                    cas, roots[bundle_id], "provenance_bundle"
                )
                self.assertEqual(
                    query["dependencies"], [roots["adj.math.arithmetic.proportion.v1"]]
                )
                self.assertEqual(len(query["formula_derivation_sha256s"]), 1)
                self.assertEqual(len(query["execution_witness_sha256s"]), 1)
                fixture_ir_hashes.add(query["sources"][1]["source_ir_sha256"])
                query_ir = source_ir(cas, query["input"]["source_ir_sha256"])
                self.assertIn(builder.QUESTION_CLAIM, claim_ids(query_ir))
                self.assertEqual(
                    query_ir["segments"][-1]["end"], query_ir["source_size"]
                )
            self.assertEqual(len(fixture_ir_hashes), 1)
            fixture_ir = source_ir(cas, fixture_ir_hashes.pop())
            self.assertEqual(len(claim_ids(fixture_ir)), 6)
            self.assertEqual(
                fixture_ir["segments"][-1]["end"], fixture_ir["source_size"]
            )

    def test_dependency_validation_precedes_source_materialization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cas = provenance.Cas(Path(directory) / "cas")
            wrong_hash = cas.put_json(
                {"bundle_id": "adj.math.not-arithmetic.v1"},
                kind="provenance_bundle",
                label="wrong dependency",
            )
            with (
                mock.patch.object(
                    builder,
                    "retained_external_source",
                    side_effect=AssertionError("source should not be read"),
                ),
                self.assertRaisesRegex(
                    provenance.ProvenanceError,
                    "arithmetic dependency bundle ID drifted",
                ),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=wrong_hash,
                    formula_inventory_command=["inventory"],
                    formula_audit_command=["audit"],
                )

    def test_atomic_migration_names_the_complete_eleven_root_closure(self) -> None:
        expected = {
            "adj.math.arithmetic.primitives.v1",
            "adj.math.arithmetic.primitives.query.v1",
            "adj.math.arithmetic.ratio.v1",
            "adj.math.arithmetic.ratio.query.v1",
            "adj.math.arithmetic.percent_of.v1",
            "adj.math.arithmetic.percent_of.query.v1",
            "adj.math.arithmetic.proportion.v1",
            "adj.math.arithmetic.proportion.query.v1",
            "adj.math.arithmetic.proportion.zero_first.query.v1",
            "adj.math.arithmetic.proportion.zero_second.query.v1",
            "adj.math.arithmetic.proportion.zero_third.query.v1",
        }
        self.assertEqual(migration.ROOT_IDS, expected)
        self.assertEqual(len(migration.ROOT_IDS), 11)


if __name__ == "__main__":
    unittest.main()
