from __future__ import annotations

import importlib
import json
import multiprocessing
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

provenance = importlib.import_module("adj_stdlib_provenance")
ratio_builder = importlib.import_module("build_adj_ratio_provenance")
percent_of_builder = importlib.import_module("build_adj_percent_of_provenance")


def acquire_cas_lock_and_exit(cas_root: str, ready: object) -> None:
    with provenance.CasRootLock(Path(cas_root), blocking=False):
        ready.set()
        os._exit(0)


def acquire_cas_lock_with_alternate_temp(
    cas_root: str, temp_root: str, ready: object, release: object
) -> None:
    os.environ["TEMP"] = temp_root
    os.environ["TMP"] = temp_root
    os.environ["TMPDIR"] = temp_root
    with provenance.CasRootLock(Path(cas_root), blocking=False):
        ready.set()
        release.wait(10)


class AdjStdlibProvenanceTests(unittest.TestCase):
    def formula_inventory_command(self) -> list[str]:
        suffix = ".exe" if os.name == "nt" else ""
        binary = (
            Path(__file__).resolve().parents[2]
            / f"packages/rust/target/debug/adj-formula-inventory{suffix}"
        )
        self.assertTrue(binary.is_file(), f"missing formula inventory binary: {binary}")
        return [os.fspath(binary)]

    def build_repository(
        self, root: Path, *, with_formula_inventory: bool = False
    ) -> tuple[Path, Path, dict[str, str]]:
        cas_root = root / "cas"
        manifest_path = root / "manifest.json"
        body = b"Header\nSum is the result of addition.\nFooter\n"
        represented_start = body.index(b"Sum")
        represented_end = body.index(b"\n", represented_start)
        raw_quote = body[represented_start:represented_end]
        claim_id = "adj.math.arithmetic.sum"
        segments = [
            {
                "disposition": "discarded",
                "end": represented_start,
                "reason": "navigation outside the mathematical definition",
                "start": 0,
            },
            {
                "claims": [
                    {
                        "claim_id": claim_id,
                        "end": represented_end,
                        "quote": raw_quote.decode("utf-8"),
                        "quote_sha256": provenance.sha256_bytes(raw_quote),
                        "start": represented_start,
                    }
                ],
                "disposition": "represented",
                "end": represented_end,
                "start": represented_start,
            },
            {
                "disposition": "discarded",
                "end": len(body),
                "reason": "footer outside the mathematical definition",
                "start": represented_end,
            },
        ]
        cas = provenance.Cas(cas_root)
        raw_hash = cas.put(body, kind="raw_source", label="fixture source")
        rendered = b"Sum is the result of addition."
        rendered_hash = cas.put(
            rendered,
            kind="rendered_text",
            label="fixture rendered definition",
            links=[raw_hash],
        )
        receipt = provenance.build_fetch_receipt(
            locator="https://example.test/sum",
            final_locator="https://example.test/sum",
            retrieved_at="2026-08-02T12:00:00Z",
            status=200,
            media_type="text/plain; charset=utf-8",
            body_sha256=raw_hash,
            body_size=len(body),
            headers={"Content-Type": "text/plain; charset=utf-8"},
        )
        receipt_hash = cas.put_json(
            receipt,
            kind="fetch_receipt",
            label="fixture receipt",
            links=[raw_hash],
        )
        source_ir = provenance.build_source_ir(
            source_sha256=raw_hash, source=body, segments=segments
        )
        ir_hash = cas.put_json(
            source_ir,
            kind="source_ir",
            label="fixture source IR",
            links=[raw_hash],
        )
        rendered_ir = provenance.build_source_ir(
            source_sha256=rendered_hash,
            source=rendered,
            segments=[
                {
                    "claims": [
                        {
                            "claim_id": claim_id,
                            "end": len(rendered),
                            "quote": rendered.decode("utf-8"),
                            "quote_sha256": provenance.sha256_bytes(rendered),
                            "start": 0,
                        }
                    ],
                    "disposition": "represented",
                    "end": len(rendered),
                    "start": 0,
                }
            ],
        )
        rendered_ir_hash = cas.put_json(
            rendered_ir,
            kind="source_ir",
            label="fixture rendered source IR",
            links=[rendered_hash],
        )
        transform = provenance.build_text_transform(
            source_sha256=raw_hash,
            source=body,
            result_sha256=rendered_hash,
            result=rendered,
            operations=[
                {
                    "operation": "copy",
                    "result_end": len(rendered),
                    "result_start": 0,
                    "source_end": represented_end,
                    "source_start": represented_start,
                }
            ],
        )
        transform_hash = cas.put_json(
            transform,
            kind="text_transform",
            label="fixture text transform",
            links=[raw_hash, rendered_hash],
        )
        input_body = (
            b"formulabook arithmetic {\n"
            b"  formula sum(a, b) = a + b\n"
            b'    source "fixture arithmetic definition"\n'
            b'    locator "https://example.test/sum"\n'
            b"    trust authoritative\n"
            b"}\n"
        )
        input_path = root / "code/example/arithmetic.adj"
        input_path.parent.mkdir(parents=True)
        input_path.write_bytes(input_body)
        input_hash = cas.put(input_body, kind="raw_source", label="fixture ADJ input")
        input_receipt = provenance.build_input_receipt(
            repo_path="code/example/arithmetic.adj",
            captured_at="2026-08-02T12:00:00Z",
            body_sha256=input_hash,
            body_size=len(input_body),
            body_git_sha1=provenance.git_blob_sha1(input_body),
        )
        input_receipt_hash = cas.put_json(
            input_receipt,
            kind="input_receipt",
            label="fixture input receipt",
            links=[input_hash],
        )
        input_ir = provenance.build_source_ir(
            source_sha256=input_hash,
            source=input_body,
            segments=[
                {
                    "claims": [
                        {
                            "claim_id": claim_id,
                            "end": len(input_body),
                            "quote": input_body.decode("utf-8"),
                            "quote_sha256": provenance.sha256_bytes(input_body),
                            "start": 0,
                        }
                    ],
                    "disposition": "represented",
                    "end": len(input_body),
                    "start": 0,
                }
            ],
        )
        input_ir_hash = cas.put_json(
            input_ir,
            kind="source_ir",
            label="fixture input IR",
            links=[input_hash],
        )
        input_source_entry = {
            "raw_source_sha256": input_hash,
            "receipt_sha256": input_receipt_hash,
            "representations": [],
            "source_ir_sha256": input_ir_hash,
        }
        source_entry = {
            "raw_source_sha256": raw_hash,
            "receipt_sha256": receipt_hash,
            "representations": [
                {
                    "rendered_text_sha256": rendered_hash,
                    "source_ir_sha256": rendered_ir_hash,
                    "transform_sha256": transform_hash,
                }
            ],
            "source_ir_sha256": ir_hash,
        }
        clause = {
            "claim_id": claim_id,
            "end": len(rendered),
            "input_claim": {
                "end": len(input_body),
                "quote": input_body.decode("utf-8"),
                "quote_sha256": provenance.sha256_bytes(input_body),
                "start": 0,
            },
            "locator": "https://example.test/sum",
            "quote": rendered.decode("utf-8"),
            "quote_sha256": provenance.sha256_bytes(rendered),
            "resolution": {
                "authority_receipt_sha256": receipt_hash,
                "authority_source_sha256": raw_hash,
                "classification": "primary_definition",
                "kind": "accepted_root",
                "reason": "fixture definition accepted as a primitive root",
            },
            "snapshot_sha256": rendered_hash,
            "source_ir_sha256": rendered_ir_hash,
            "start": 0,
        }
        bundle = {
            "bundle_id": "test.arithmetic.v1",
            "clauses": [clause],
            "dependencies": [],
            "input": {
                "raw_source_sha256": input_hash,
                "receipt_sha256": input_receipt_hash,
                "source_ir_sha256": input_ir_hash,
            },
            "kind": "provenance_bundle",
            "library": "code/example/arithmetic.adj",
            "sources": [source_entry, input_source_entry],
        }
        inventory_hash = ""
        if with_formula_inventory:
            inventory = provenance._run_formula_inventory(
                self.formula_inventory_command(), cas.object_path(input_hash)
            )
            provenance._validate_formula_inventory_value(
                inventory, input_hash, input_body
            )
            inventory_hash = cas.put_json(
                inventory,
                kind="formula_parser_inventory",
                label="fixture formula parser inventory",
                links=[input_hash],
            )
            bundle["formula_inventory_sha256"] = inventory_hash
        bundle_hash = cas.put_json(
            bundle,
            kind="provenance_bundle",
            label="fixture bundle",
            links=provenance._bundle_declared_links(bundle),
        )
        cas.write_index()
        manifest = {
            "algorithm": "sha256",
            "bundle_hashes": [bundle_hash],
            "manifest_id": "test.provenance.v1",
            "schema_version": 1,
        }
        manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
        return (
            cas_root,
            manifest_path,
            {
                "raw": raw_hash,
                "rendered": rendered_hash,
                "rendered_ir": rendered_ir_hash,
                "receipt": receipt_hash,
                "ir": ir_hash,
                "input": input_hash,
                "input_ir": input_ir_hash,
                "input_receipt": input_receipt_hash,
                "formula_inventory": inventory_hash,
                "bundle": bundle_hash,
                "transform": transform_hash,
                "snapshot": rendered_hash,
            },
        )

    def registered_bundle_hash(
        self, cas_root: Path, manifest_path: Path, bundle_id: str
    ) -> str:
        cas = provenance.Cas(cas_root)
        cas.load()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        matches = [
            digest
            for digest in manifest["bundle_hashes"]
            if provenance._json_object(cas, digest, "provenance_bundle").get(
                "bundle_id"
            )
            == bundle_id
        ]
        self.assertEqual(len(matches), 1, f"expected one registered {bundle_id} root")
        return matches[0]

    def replace_bundle(
        self, cas_root: Path, manifest_path: Path, mutate: object
    ) -> None:
        cas = provenance.Cas(cas_root)
        cas.load()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        old_hash = manifest["bundle_hashes"][0]
        bundle = json.loads(cas.object_path(old_hash).read_text(encoding="utf-8"))
        original_links = provenance._bundle_declared_links(bundle)
        mutate(bundle, cas)
        try:
            links = provenance._bundle_declared_links(bundle)
        except provenance.ProvenanceError:
            links = original_links
        new_hash = cas.put_json(
            bundle,
            kind="provenance_bundle",
            label="mutated bundle",
            links=links,
        )
        manifest["bundle_hashes"] = [new_hash]
        cas.write_index()
        manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))

    def test_valid_repository_projects_exact_adj_verify_snapshot_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)

            result = provenance.validate_repository(cas_root, manifest_path)
            projection = root / "projection"
            projected = provenance.project_snapshots(
                cas_root, manifest_path, projection
            )

            self.assertEqual(result["bundles"], 1)
            self.assertEqual(result["objects"], 10)
            self.assertEqual(result["snapshots"], 1)
            self.assertTrue(result["valid"])
            self.assertEqual(projected["projected"], 1)
            projected_bytes = (projection / hashes["snapshot"]).read_bytes()
            self.assertEqual(
                provenance.sha256_bytes(projected_bytes), hashes["snapshot"]
            )

    def test_formula_inventory_is_replayed_from_exact_cas_input_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(
                root, with_formula_inventory=True
            )
            schema = (
                Path(__file__).resolve().parents[2]
                / "specs/data/adj-stdlib-provenance/manifest.schema.json"
            )

            result = provenance.validate_repository(
                cas_root,
                manifest_path,
                schema,
                workspace_root=root,
                formula_inventory_command=self.formula_inventory_command(),
            )

            self.assertTrue(result["valid"])
            self.assertEqual(result["objects"], 11)
            cas = provenance.Cas(cas_root)
            cas.load()
            inventory = provenance._json_object(
                cas, hashes["formula_inventory"], "formula_parser_inventory"
            )
            self.assertEqual(inventory["source_sha256"], hashes["input"])
            self.assertEqual(
                [item["formula"] for item in inventory["formulas"]], ["sum"]
            )

    def test_formula_inventory_fails_closed_without_replay_binary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(
                root, with_formula_inventory=True
            )

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "requires --formula-inventory-binary"
            ):
                provenance.validate_repository(
                    cas_root, manifest_path, workspace_root=root
                )

    def test_formula_inventory_rejects_parser_replay_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(
                root, with_formula_inventory=True
            )
            command = self.formula_inventory_command()
            original = provenance._run_formula_inventory

            def drifted(parser_command: object, source_path: Path) -> dict[str, object]:
                value = original(parser_command, source_path)
                value["formulas"][0]["step_count"] = 1
                return value

            with (
                mock.patch.object(provenance, "_run_formula_inventory", drifted),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "disagrees with parser replay"
                ),
            ):
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    workspace_root=root,
                    formula_inventory_command=command,
                )

    def test_formula_inventory_parser_output_is_bounded_while_reading(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.adj"
            source.write_bytes(b"")
            command = [
                sys.executable,
                "-c",
                "import sys; sys.stdout.buffer.write(b'x' * 2048)",
            ]

            with (
                mock.patch.object(provenance, "MAX_OBJECT_BYTES", 1024),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "output exceeds byte limit"
                ),
            ):
                provenance._run_formula_inventory(command, source)

    def test_one_broad_input_claim_cannot_ground_two_formulas(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = (
                b"formulabook demo {\n"
                b'  formula first(x) = x + 1 source "s" locator "cas://s" trust authoritative\n'
                b'  formula second(x) = x + 2 source "s" locator "cas://s" trust authoritative\n'
                b"}\n"
            )
            cas = provenance.Cas(root / "cas")
            source_hash = cas.put(source, kind="raw_source", label="two formulas")
            inventory = provenance._run_formula_inventory(
                self.formula_inventory_command(), cas.object_path(source_hash)
            )
            inventory_hash = cas.put_json(
                inventory,
                kind="formula_parser_inventory",
                label="two-formula inventory",
                links=[source_hash],
            )
            broad_claim = {
                "broad": {
                    "claim_id": "broad",
                    "end": len(source),
                    "quote": source.decode("utf-8"),
                    "quote_sha256": provenance.sha256_bytes(source),
                    "start": 0,
                }
            }

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "cannot ground more than one formula"
            ):
                provenance._validate_formula_inventory(
                    cas,
                    inventory_hash,
                    source_hash,
                    broad_claim,
                    self.formula_inventory_command(),
                )

    def test_committed_json_schema_accepts_a_complete_bundle_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)
            schema = (
                Path(__file__).resolve().parents[2]
                / "specs/data/adj-stdlib-provenance/manifest.schema.json"
            )

            result = provenance.validate_repository(cas_root, manifest_path, schema)

            self.assertTrue(result["valid"])

    def test_put_is_content_addressed_and_deduplicated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cas = provenance.Cas(Path(directory))
            first = cas.put(b"same bytes", kind="raw_source", label="same")
            second = cas.put(b"same bytes", kind="raw_source", label="different fetch")
            self.assertEqual(first, second)
            self.assertEqual(len(cas.index), 1)

    def test_manifest_registration_preserves_unowned_bundle_roots(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            with provenance.BundleRegistrationTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                original = json.loads(
                    transaction.cas.object_path(hashes["bundle"]).read_text(
                        encoding="utf-8"
                    )
                )
                ratio = json.loads(json.dumps(original))
                ratio["bundle_id"] = "test.ratio.v1"
                ratio_hash = transaction.cas.put_json(
                    ratio,
                    kind="provenance_bundle",
                    label="ratio fixture bundle",
                    links=provenance._bundle_declared_links(ratio),
                )
                registered = transaction.commit({"test.ratio.v1": ratio_hash})
            with provenance.BundleRegistrationTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                rerun = transaction.commit({"test.arithmetic.v1": hashes["bundle"]})

            self.assertEqual(registered, sorted([hashes["bundle"], ratio_hash]))
            self.assertEqual(rerun, registered)
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(manifest["bundle_hashes"], registered)

    def test_manifest_registration_refuses_implicit_root_replacement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            before = manifest_path.read_bytes()

            with (
                self.assertRaisesRegex(
                    provenance.ProvenanceError,
                    "explicit root-replacement migration",
                ),
                provenance.BundleRegistrationTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = json.loads(
                    transaction.cas.object_path(hashes["bundle"]).read_text(
                        encoding="utf-8"
                    )
                )
                changed["library"] = "code/example/corrected-arithmetic.adj"
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="changed arithmetic fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                changed_path = transaction.cas.object_path(changed_hash)
                transaction.commit({"test.arithmetic.v1": changed_hash})

            self.assertEqual(manifest_path.read_bytes(), before)
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertFalse(changed_path.exists())

    def test_explicit_root_replacement_prunes_only_superseded_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)

            with provenance.BundleRootReplacementTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = (
                    "corrected fixture definition accepted as a primitive root"
                )
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="corrected arithmetic fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                result = transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            self.assertEqual(result["bundle_hashes"], [changed_hash])
            self.assertEqual(result["pruned_sha256s"], [hashes["bundle"]])
            self.assertFalse(
                provenance.Cas(cas_root).object_path(hashes["bundle"]).exists()
            )
            self.assertTrue(provenance.Cas(cas_root).object_path(changed_hash).exists())
            self.assertTrue(
                provenance.validate_repository(
                    cas_root, manifest_path, workspace_root=root
                )["valid"]
            )

    def test_root_replacement_preserves_old_root_reachable_as_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            with provenance.BundleRegistrationTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                consumer = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                consumer["bundle_id"] = "test.consumer.v1"
                consumer["dependencies"] = [hashes["bundle"]]
                consumer_hash = transaction.cas.put_json(
                    consumer,
                    kind="provenance_bundle",
                    label="consumer fixture bundle",
                    links=provenance._bundle_declared_links(consumer),
                )
                transaction.commit({"test.consumer.v1": consumer_hash})

            with provenance.BundleRootReplacementTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = "replacement root"
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="replacement fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                result = transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            self.assertEqual(
                result["bundle_hashes"], sorted([changed_hash, consumer_hash])
            )
            self.assertEqual(result["pruned_sha256s"], [])
            self.assertTrue(
                provenance.Cas(cas_root).object_path(hashes["bundle"]).exists()
            )

    def test_root_replacement_updates_multiple_roots_in_one_compare_and_swap(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            with provenance.BundleRegistrationTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                second = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                second["bundle_id"] = "test.second.v1"
                second_hash = transaction.cas.put_json(
                    second,
                    kind="provenance_bundle",
                    label="second fixture bundle",
                    links=provenance._bundle_declared_links(second),
                )
                transaction.commit({"test.second.v1": second_hash})

            with provenance.BundleRootReplacementTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                first_new = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                first_new["clauses"][0]["resolution"]["reason"] = "first replacement"
                first_new_hash = transaction.cas.put_json(
                    first_new,
                    kind="provenance_bundle",
                    label="first replacement bundle",
                    links=provenance._bundle_declared_links(first_new),
                )
                second_new = provenance._json_object(
                    transaction.cas, second_hash, "provenance_bundle"
                )
                second_new["clauses"][0]["resolution"]["reason"] = "second replacement"
                second_new_hash = transaction.cas.put_json(
                    second_new,
                    kind="provenance_bundle",
                    label="second replacement bundle",
                    links=provenance._bundle_declared_links(second_new),
                )
                result = transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": first_new_hash,
                        },
                        "test.second.v1": {
                            "expected_old_sha256": second_hash,
                            "new_sha256": second_new_hash,
                        },
                    }
                )

            self.assertEqual(
                result["bundle_hashes"], sorted([first_new_hash, second_new_hash])
            )
            self.assertEqual(
                result["pruned_sha256s"], sorted([hashes["bundle"], second_hash])
            )

    def test_root_replacement_rejects_new_strays_and_restores_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            with (
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "staged unreferenced new objects"
                ),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = "replacement root"
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="replacement fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                stray_hash = transaction.cas.put(
                    b"unreachable replacement bytes",
                    kind="raw_source",
                    label="replacement stray",
                )
                transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            cas = provenance.Cas(cas_root)
            self.assertFalse(cas.object_path(changed_hash).exists())
            self.assertFalse(cas.object_path(stray_hash).exists())

    def test_root_replacement_cannot_add_an_unregistered_bundle_id(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()

            with (
                self.assertRaisesRegex(
                    provenance.ProvenanceError,
                    "stale root replacement for test.new-root.v1",
                ),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                added = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                added["bundle_id"] = "test.new-root.v1"
                added_hash = transaction.cas.put_json(
                    added,
                    kind="provenance_bundle",
                    label="invalid replacement addition",
                    links=provenance._bundle_declared_links(added),
                )
                transaction.replace_roots(
                    {
                        "test.new-root.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": added_hash,
                        }
                    }
                )

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertFalse(provenance.Cas(cas_root).object_path(added_hash).exists())

    def test_root_replacement_rejects_a_stale_expected_hash(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()

            with (
                self.assertRaisesRegex(
                    provenance.ProvenanceError,
                    "stale root replacement for test.arithmetic.v1",
                ),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = "stale replacement"
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="stale replacement fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": "0" * 64,
                            "new_sha256": changed_hash,
                        }
                    }
                )

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertFalse(
                provenance.Cas(cas_root).object_path(changed_hash).exists()
            )

    def test_root_replacement_restores_pruned_objects_on_final_validation_failure(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            validate = provenance._validate_repository_unlocked
            calls = 0

            def fail_final_validation(
                *args: object, **kwargs: object
            ) -> dict[str, object]:
                nonlocal calls
                calls += 1
                if calls == 3:
                    raise provenance.ProvenanceError(
                        "injected final validation failure"
                    )
                return validate(*args, **kwargs)

            with (
                mock.patch.object(
                    provenance,
                    "_validate_repository_unlocked",
                    fail_final_validation,
                ),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "injected final validation failure"
                ),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = "replacement root"
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="replacement fixture bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertTrue(
                provenance.Cas(cas_root).object_path(hashes["bundle"]).exists()
            )
            self.assertFalse(
                provenance.Cas(cas_root).object_path(changed_hash).exists()
            )

    def assert_root_replacement_publication_failure(self, failure_stage: str) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            baseline_objects = {
                path.relative_to(cas_root): path.read_bytes()
                for path in (cas_root / "objects").rglob("*")
                if path.is_file()
            }
            write_index = provenance.Cas.write_index
            write_atomic = provenance._write_atomic
            index_writes = 0
            manifest_failed = False

            def fail_index_publication(cas: provenance.Cas) -> None:
                nonlocal index_writes
                index_writes += 1
                failing_write = 1 if failure_stage == "candidate index" else 2
                if failure_stage != "manifest" and index_writes == failing_write:
                    raise OSError(f"injected {failure_stage} publication failure")
                write_index(cas)

            def fail_manifest_publication(path: Path, data: bytes) -> None:
                nonlocal manifest_failed
                if (
                    failure_stage == "manifest"
                    and path == manifest_path
                    and not manifest_failed
                ):
                    manifest_failed = True
                    raise OSError("injected manifest publication failure")
                write_atomic(path, data)

            with (
                mock.patch.object(
                    provenance.Cas,
                    "write_index",
                    fail_index_publication,
                ),
                mock.patch.object(
                    provenance,
                    "_write_atomic",
                    fail_manifest_publication,
                ),
                self.assertRaisesRegex(
                    OSError, f"injected {failure_stage} publication failure"
                ),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = (
                    f"replacement before {failure_stage} failure"
                )
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="publication failure replacement bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            restored_objects = {
                path.relative_to(cas_root): path.read_bytes()
                for path in (cas_root / "objects").rglob("*")
                if path.is_file()
            }
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertEqual(restored_objects, baseline_objects)

    def test_root_replacement_restores_bytes_after_publication_failures(self) -> None:
        for failure_stage in ("candidate index", "filtered index", "manifest"):
            with self.subTest(failure_stage=failure_stage):
                self.assert_root_replacement_publication_failure(failure_stage)

    def test_root_replacement_restores_bytes_after_partial_prune_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            baseline_objects = {
                path.relative_to(cas_root): path.read_bytes()
                for path in (cas_root / "objects").rglob("*")
                if path.is_file()
            }
            old_root_path = provenance.Cas(cas_root).object_path(hashes["bundle"])
            unlink = Path.unlink
            prune_failed = False

            def fail_after_prune(path: Path, *args: object, **kwargs: object) -> None:
                nonlocal prune_failed
                unlink(path, *args, **kwargs)
                if path == old_root_path and not prune_failed:
                    prune_failed = True
                    raise OSError("injected partial prune failure")

            with (
                mock.patch.object(Path, "unlink", fail_after_prune),
                self.assertRaisesRegex(OSError, "injected partial prune failure"),
                provenance.BundleRootReplacementTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                changed = provenance._json_object(
                    transaction.cas, hashes["bundle"], "provenance_bundle"
                )
                changed["clauses"][0]["resolution"]["reason"] = (
                    "replacement before partial prune failure"
                )
                changed_hash = transaction.cas.put_json(
                    changed,
                    kind="provenance_bundle",
                    label="partial prune failure replacement bundle",
                    links=provenance._bundle_declared_links(changed),
                )
                transaction.replace_roots(
                    {
                        "test.arithmetic.v1": {
                            "expected_old_sha256": hashes["bundle"],
                            "new_sha256": changed_hash,
                        }
                    }
                )

            restored_objects = {
                path.relative_to(cas_root): path.read_bytes()
                for path in (cas_root / "objects").rglob("*")
                if path.is_file()
            }
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertEqual(restored_objects, baseline_objects)

    def test_manifest_registration_rejects_malformed_contracts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _hashes = self.build_repository(root)
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            cases = [
                {**manifest, "schema_version": True},
                {**manifest, "bundle_hashes": [{}]},
            ]
            for malformed in cases:
                with self.subTest(manifest=malformed):
                    manifest_path.write_bytes(
                        provenance.canonical_json_bytes(malformed)
                    )
                    with (
                        self.assertRaises(provenance.ProvenanceError),
                        provenance.BundleRegistrationTransaction(
                            cas_root,
                            manifest_path,
                            expected_manifest_id="test.provenance.v1",
                            workspace_root=root,
                        ),
                    ):
                        pass

    def test_manifest_registration_lock_rejects_a_concurrent_writer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            with provenance.BundleRegistrationTransaction(
                cas_root,
                manifest_path,
                expected_manifest_id="test.provenance.v1",
                workspace_root=root,
            ) as transaction:
                with (
                    self.assertRaisesRegex(
                        provenance.ProvenanceError,
                        "another provenance operation",
                    ),
                    provenance.CasMutationTransaction(cas_root, blocking=False),
                ):
                    pass
                transaction.commit({"test.arithmetic.v1": hashes["bundle"]})

    def test_cas_root_lock_is_released_when_its_process_exits(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cas_root = Path(directory) / "cas"
            context = multiprocessing.get_context("spawn")
            ready = context.Event()
            process = context.Process(
                target=acquire_cas_lock_and_exit,
                args=(str(cas_root), ready),
            )
            process.start()
            self.assertTrue(ready.wait(10), "child did not acquire the CAS lock")
            process.join(10)
            self.assertEqual(process.exitcode, 0)

            with provenance.CasRootLock(cas_root, blocking=False):
                pass

    def test_cas_root_lock_does_not_depend_on_process_temp_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root = root / "cas"
            alternate_temp = root / "alternate-temp"
            alternate_temp.mkdir()
            context = multiprocessing.get_context("spawn")
            ready = context.Event()
            release = context.Event()
            process = context.Process(
                target=acquire_cas_lock_with_alternate_temp,
                args=(str(cas_root), str(alternate_temp), ready, release),
            )
            process.start()
            try:
                self.assertTrue(ready.wait(10), "child did not acquire the CAS lock")
                with (
                    self.assertRaisesRegex(
                        provenance.ProvenanceError,
                        "another provenance operation",
                    ),
                    provenance.CasRootLock(cas_root, blocking=False),
                ):
                    pass
            finally:
                release.set()
                process.join(10)
                if process.is_alive():
                    process.terminate()
                    process.join(10)
            self.assertEqual(process.exitcode, 0)

    def test_manifest_registration_rolls_back_failed_graph_validation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            with (
                self.assertRaisesRegex(provenance.ProvenanceError, "unreferenced"),
                provenance.BundleRegistrationTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="test.provenance.v1",
                    workspace_root=root,
                ) as transaction,
            ):
                stray_hash = transaction.cas.put(
                    b"unreachable staged bytes",
                    kind="raw_source",
                    label="rollback fixture",
                )
                stray_path = transaction.cas.object_path(stray_hash)
                transaction.commit({"test.arithmetic.v1": hashes["bundle"]})

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            self.assertFalse(stray_path.exists())

    def test_ratio_generator_is_offline_idempotent_and_reuses_quotient(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = ratio_builder.REPO_ROOT
            provenance_source = repo_root / provenance.DEFAULT_ROOT.parent
            provenance_copy = workspace / provenance.DEFAULT_ROOT.parent
            provenance_copy.parent.mkdir(parents=True)
            shutil.copytree(
                provenance_source,
                provenance_copy,
                ignore=shutil.ignore_patterns("lock"),
            )
            arithmetic_source = (
                repo_root / "code/specs/data/adj-formula-stdlib/arithmetic"
            )
            arithmetic_copy = (
                workspace / "code/specs/data/adj-formula-stdlib/arithmetic"
            )
            arithmetic_copy.parent.mkdir(parents=True)
            shutil.copytree(arithmetic_source, arithmetic_copy)

            cas_root = workspace / provenance.DEFAULT_ROOT
            manifest_path = workspace / provenance.DEFAULT_MANIFEST
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            arithmetic_hash = self.registered_bundle_hash(
                cas_root,
                manifest_path,
                "adj.math.arithmetic.primitives.v1",
            )
            original_root = ratio_builder.REPO_ROOT
            ratio_builder.REPO_ROOT = workspace
            try:
                with provenance.BundleRegistrationTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="adj.stdlib.provenance.v1",
                    schema_path=workspace / provenance.DEFAULT_SCHEMA,
                    workspace_root=workspace,
                ) as transaction:
                    transaction.commit(
                        ratio_builder.build(
                            transaction.cas,
                            None,
                            arithmetic_bundle_sha256=arithmetic_hash,
                        )
                    )
            finally:
                ratio_builder.REPO_ROOT = original_root

            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)
            cas = provenance.Cas(cas_root)
            cas.load()
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            bundles = {
                provenance._json_object(cas, digest, "provenance_bundle")[
                    "bundle_id"
                ]: (digest, provenance._json_object(cas, digest, "provenance_bundle"))
                for digest in manifest["bundle_hashes"]
            }
            ratio_hash, ratio_bundle = bundles["adj.math.arithmetic.ratio.v1"]
            query_hash, query_bundle = bundles["adj.math.arithmetic.ratio.query.v1"]
            self.assertEqual(ratio_bundle["dependencies"], [arithmetic_hash])
            self.assertEqual(
                ratio_bundle["clauses"][0]["resolution"],
                {
                    "bundle_sha256": arithmetic_hash,
                    "claim_id": "adj.math.arithmetic.quotient",
                    "kind": "dependency",
                },
            )
            self.assertNotIn(
                "0be79e8dfa46675a74374e37ba59bda163388617c3d728324ce8c7bb3d2f6f86",
                [source["raw_source_sha256"] for source in ratio_bundle["sources"]],
            )
            self.assertEqual(query_bundle["dependencies"], [ratio_hash])
            self.assertIn(query_hash, manifest["bundle_hashes"])

            ratio_ir = provenance._json_object(
                cas, ratio_bundle["input"]["source_ir_sha256"], "source_ir"
            )
            ratio_claims = {
                item["claim_id"]
                for segment in ratio_ir["segments"]
                for item in segment.get("claims", [])
            }
            self.assertEqual(
                ratio_claims,
                {
                    "adj.code.arithmetic.ratio.import.arithmetic",
                    "adj.code.arithmetic.ratio.use.ratio_vocab",
                    "adj.code.arithmetic.ratio.vocabulary",
                    "adj.math.arithmetic.ratio",
                },
            )
            query_ir = provenance._json_object(
                cas, query_bundle["input"]["source_ir_sha256"], "source_ir"
            )
            query_claims = {
                item["claim_id"]
                for segment in query_ir["segments"]
                for item in segment.get("claims", [])
            }
            self.assertEqual(
                query_claims,
                {
                    "adj.code.arithmetic.ratio.query.import",
                    "adj.input.arithmetic.ratio.denominator",
                    "adj.input.arithmetic.ratio.numerator",
                    "adj.question.arithmetic.ratio.compute",
                },
            )

            captured_source = workspace / "captured-ratio.html"
            captured_source.write_bytes(
                provenance._read_regular_file(cas.object_path(ratio_builder.RAW_HASH))
            )
            non_ratio_roots = []
            for digest in manifest["bundle_hashes"]:
                bundle_id = provenance._json_object(cas, digest, "provenance_bundle")[
                    "bundle_id"
                ]
                if not bundle_id.startswith("adj.math.arithmetic.ratio"):
                    non_ratio_roots.append(digest)
            reachable = provenance._reachable(cas, non_ratio_roots)
            for digest in sorted(set(cas.index) - reachable):
                cas.object_path(digest).unlink()
            cas.index = {
                digest: record
                for digest, record in cas.index.items()
                if digest in reachable
            }
            cas.write_index()
            manifest["bundle_hashes"] = sorted(non_ratio_roots)
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
            without_ratio = provenance.validate_repository(
                cas_root,
                manifest_path,
                workspace / provenance.DEFAULT_SCHEMA,
                workspace_root=workspace,
            )
            self.assertEqual(without_ratio["bundles"], len(non_ratio_roots))

            ratio_builder.REPO_ROOT = workspace
            try:
                with provenance.BundleRegistrationTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="adj.stdlib.provenance.v1",
                    schema_path=workspace / provenance.DEFAULT_SCHEMA,
                    workspace_root=workspace,
                ) as transaction:
                    transaction.commit(
                        ratio_builder.build(
                            transaction.cas,
                            captured_source,
                            arithmetic_bundle_sha256=arithmetic_hash,
                        )
                    )
            finally:
                ratio_builder.REPO_ROOT = original_root
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)

    def test_percent_of_generator_is_offline_idempotent_and_bootstrappable(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = percent_of_builder.REPO_ROOT
            provenance_copy = workspace / provenance.DEFAULT_ROOT.parent
            provenance_copy.parent.mkdir(parents=True)
            shutil.copytree(
                repo_root / provenance.DEFAULT_ROOT.parent,
                provenance_copy,
                ignore=shutil.ignore_patterns("lock"),
            )
            arithmetic_copy = (
                workspace / "code/specs/data/adj-formula-stdlib/arithmetic"
            )
            arithmetic_copy.parent.mkdir(parents=True)
            shutil.copytree(
                repo_root / "code/specs/data/adj-formula-stdlib/arithmetic",
                arithmetic_copy,
            )

            cas_root = workspace / provenance.DEFAULT_ROOT
            manifest_path = workspace / provenance.DEFAULT_MANIFEST
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            arithmetic_hash = self.registered_bundle_hash(
                cas_root,
                manifest_path,
                "adj.math.arithmetic.primitives.v1",
            )
            original_root = percent_of_builder.REPO_ROOT

            def register(captured_source: Path | None) -> None:
                percent_of_builder.REPO_ROOT = workspace
                try:
                    with provenance.BundleRegistrationTransaction(
                        cas_root,
                        manifest_path,
                        expected_manifest_id="adj.stdlib.provenance.v1",
                        schema_path=workspace / provenance.DEFAULT_SCHEMA,
                        workspace_root=workspace,
                    ) as transaction:
                        transaction.commit(
                            percent_of_builder.build(
                                transaction.cas,
                                captured_source,
                                arithmetic_bundle_sha256=arithmetic_hash,
                            )
                        )
                finally:
                    percent_of_builder.REPO_ROOT = original_root

            register(None)
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)

            cas = provenance.Cas(cas_root)
            cas.load()
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            bundles = {
                provenance._json_object(cas, digest, "provenance_bundle")[
                    "bundle_id"
                ]: (
                    digest,
                    provenance._json_object(cas, digest, "provenance_bundle"),
                )
                for digest in manifest["bundle_hashes"]
            }
            formula_hash, formula_bundle = bundles["adj.math.arithmetic.percent_of.v1"]
            query_hash, query_bundle = bundles[
                "adj.math.arithmetic.percent_of.query.v1"
            ]
            self.assertEqual(
                formula_bundle["dependencies"],
                [arithmetic_hash],
            )
            self.assertEqual(
                formula_bundle["clauses"][0]["resolution"]["kind"],
                "accepted_root",
            )
            arithmetic = provenance._json_object(
                cas,
                arithmetic_hash,
                "provenance_bundle",
            )
            arithmetic_raw = {
                source["raw_source_sha256"] for source in arithmetic["sources"]
            }
            self.assertTrue(
                arithmetic_raw.isdisjoint(
                    source["raw_source_sha256"] for source in formula_bundle["sources"]
                )
            )
            self.assertEqual(query_bundle["dependencies"], [formula_hash])
            self.assertIn(query_hash, manifest["bundle_hashes"])

            def claim_ids(bundle: dict[str, object]) -> set[str]:
                ir = provenance._json_object(
                    cas, bundle["input"]["source_ir_sha256"], "source_ir"
                )
                return {
                    item["claim_id"]
                    for segment in ir["segments"]
                    for item in segment.get("claims", [])
                }

            self.assertEqual(
                claim_ids(formula_bundle),
                {
                    "adj.code.arithmetic.percent_of.import.arithmetic",
                    "adj.code.arithmetic.percent_of.use.percent_of_vocab",
                    "adj.code.arithmetic.percent_of.vocabulary",
                    "adj.math.arithmetic.percent_of",
                },
            )
            self.assertEqual(
                claim_ids(query_bundle),
                {
                    "adj.code.arithmetic.percent_of.query.import",
                    "adj.input.arithmetic.percent_of.rate",
                    "adj.input.arithmetic.percent_of.whole",
                    "adj.question.arithmetic.percent_of.compute",
                },
            )
            query_ir = provenance._json_object(
                cas, query_bundle["input"]["source_ir_sha256"], "source_ir"
            )
            self.assertIn(
                "disabled edge-case example deliberately excluded from the "
                "executable worked query",
                {
                    segment.get("reason")
                    for segment in query_ir["segments"]
                    if segment["disposition"] == "discarded"
                },
            )

            captured_source = workspace / "captured-percent-of.html"
            captured_source.write_bytes(
                provenance._read_regular_file(
                    cas.object_path(percent_of_builder.RAW_HASH)
                )
            )
            other_roots = [
                digest
                for digest in manifest["bundle_hashes"]
                if not provenance._json_object(cas, digest, "provenance_bundle")[
                    "bundle_id"
                ].startswith("adj.math.arithmetic.percent_of")
            ]
            reachable = provenance._reachable(cas, other_roots)
            for digest in sorted(set(cas.index) - reachable):
                cas.object_path(digest).unlink()
            cas.index = {
                digest: record
                for digest, record in cas.index.items()
                if digest in reachable
            }
            cas.write_index()
            manifest["bundle_hashes"] = sorted(other_roots)
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
            provenance.validate_repository(
                cas_root,
                manifest_path,
                workspace / provenance.DEFAULT_SCHEMA,
                workspace_root=workspace,
            )

            register(captured_source)
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)

    def test_dependent_generators_reject_unverified_dependency_hashes(self) -> None:
        cas_root = ratio_builder.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = ratio_builder.REPO_ROOT / provenance.DEFAULT_MANIFEST
        cas = provenance.Cas(cas_root)
        cas.load()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        arithmetic_hash = self.registered_bundle_hash(
            cas_root,
            manifest_path,
            "adj.math.arithmetic.primitives.v1",
        )
        wrong_hash = next(
            digest for digest in manifest["bundle_hashes"] if digest != arithmetic_hash
        )
        raw_source_hash = next(
            digest
            for digest, record in cas.index.items()
            if "raw_source" in record["kinds"]
            and "provenance_bundle" not in record["kinds"]
        )

        for builder in (ratio_builder, percent_of_builder):
            with (
                self.subTest(builder=builder.__name__, failure="absent hash"),
                self.assertRaises(TypeError),
            ):
                builder.build(cas, None)
            with (
                self.subTest(builder=builder.__name__, failure="malformed hash"),
                self.assertRaisesRegex(provenance.ProvenanceError, "lowercase SHA-256"),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256="not-a-sha256",
                )
            with (
                self.subTest(builder=builder.__name__, failure="wrong bundle"),
                self.assertRaisesRegex(
                    provenance.ProvenanceError,
                    "arithmetic dependency bundle ID drifted",
                ),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=wrong_hash,
                )
            with (
                self.subTest(builder=builder.__name__, failure="missing object"),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "missing CAS object"
                ),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256="0" * 64,
                )
            with (
                self.subTest(builder=builder.__name__, failure="non-bundle object"),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "must be a provenance_bundle object"
                ),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=raw_source_hash,
                )

    def test_dependent_generator_clis_require_the_dependency_hash(self) -> None:
        for builder in (ratio_builder, percent_of_builder):
            with self.subTest(builder=builder.__name__):
                process = subprocess.run(
                    [sys.executable, os.fspath(Path(builder.__file__))],
                    capture_output=True,
                    check=False,
                    text=True,
                )
                self.assertEqual(process.returncode, 2)
                self.assertIn(
                    "the following arguments are required: --arithmetic-bundle-sha256",
                    process.stderr,
                )

    def test_dependent_generators_honor_alternate_same_id_dependency(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = ratio_builder.REPO_ROOT
            provenance_copy = workspace / provenance.DEFAULT_ROOT.parent
            provenance_copy.parent.mkdir(parents=True)
            shutil.copytree(
                repo_root / provenance.DEFAULT_ROOT.parent,
                provenance_copy,
                ignore=shutil.ignore_patterns("lock"),
            )
            arithmetic_copy = (
                workspace / "code/specs/data/adj-formula-stdlib/arithmetic"
            )
            arithmetic_copy.parent.mkdir(parents=True)
            shutil.copytree(
                repo_root / "code/specs/data/adj-formula-stdlib/arithmetic",
                arithmetic_copy,
            )
            cas_root = workspace / provenance.DEFAULT_ROOT
            manifest_path = workspace / provenance.DEFAULT_MANIFEST
            cas = provenance.Cas(cas_root)
            cas.load()
            arithmetic_hash = self.registered_bundle_hash(
                cas_root,
                manifest_path,
                "adj.math.arithmetic.primitives.v1",
            )
            alternate = provenance._json_object(
                cas, arithmetic_hash, "provenance_bundle"
            )
            alternate["clauses"][0]["resolution"]["reason"] = (
                "alternate same-ID dependency selected explicitly by the caller"
            )
            alternate_hash = cas.put_json(
                alternate,
                kind="provenance_bundle",
                label="alternate primitive arithmetic dependency",
                links=provenance._bundle_declared_links(alternate),
            )
            self.assertNotEqual(alternate_hash, arithmetic_hash)

            original_ratio_root = ratio_builder.REPO_ROOT
            original_percent_root = percent_of_builder.REPO_ROOT
            ratio_builder.REPO_ROOT = workspace
            percent_of_builder.REPO_ROOT = workspace
            try:
                ratio_roots = ratio_builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=alternate_hash,
                )
                percent_roots = percent_of_builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=alternate_hash,
                )
            finally:
                ratio_builder.REPO_ROOT = original_ratio_root
                percent_of_builder.REPO_ROOT = original_percent_root

            ratio = provenance._json_object(
                cas,
                ratio_roots["adj.math.arithmetic.ratio.v1"],
                "provenance_bundle",
            )
            percent_of = provenance._json_object(
                cas,
                percent_roots["adj.math.arithmetic.percent_of.v1"],
                "provenance_bundle",
            )
            self.assertEqual(ratio["dependencies"], [alternate_hash])
            self.assertEqual(
                ratio["clauses"][0]["resolution"]["bundle_sha256"],
                alternate_hash,
            )
            self.assertEqual(percent_of["dependencies"], [alternate_hash])

    def test_partition_rejects_gaps_overlaps_and_unreasoned_discards(self) -> None:
        cases = [
            [
                {"start": 0, "end": 2, "disposition": "discarded", "reason": "x"},
                {"start": 3, "end": 4, "disposition": "represented", "claims": ["c"]},
            ],
            [
                {"start": 0, "end": 3, "disposition": "discarded", "reason": "x"},
                {"start": 2, "end": 4, "disposition": "represented", "claims": ["c"]},
            ],
            [{"start": 0, "end": 4, "disposition": "discarded", "reason": ""}],
            [
                {
                    "claims": [
                        {
                            "claim_id": "partial",
                            "end": 1,
                            "quote": "a",
                            "quote_sha256": provenance.sha256_bytes(b"a"),
                            "start": 0,
                        }
                    ],
                    "disposition": "represented",
                    "end": 4,
                    "start": 0,
                }
            ],
        ]
        for segments in cases:
            with (
                self.subTest(segments=segments),
                self.assertRaises(provenance.ProvenanceError),
            ):
                provenance.validate_segments(segments, b"abcd")

    def test_claim_quote_and_hash_must_match_exact_bytes(self) -> None:
        segment = {
            "claims": [
                {
                    "claim_id": "claim",
                    "end": 4,
                    "quote": "abce",
                    "quote_sha256": provenance.sha256_bytes(b"abcd"),
                    "start": 0,
                }
            ],
            "disposition": "represented",
            "end": 4,
            "start": 0,
        }
        with self.assertRaisesRegex(provenance.ProvenanceError, "disagrees"):
            provenance.validate_segments([segment], b"abcd")

    def test_json_booleans_are_not_byte_offsets(self) -> None:
        segment = {
            "claims": [],
            "disposition": "represented",
            "end": True,
            "start": False,
        }
        with self.assertRaisesRegex(provenance.ProvenanceError, "integers"):
            provenance.validate_segments([segment], b"x")

    def test_html_entity_transform_reproduces_every_result_byte(self) -> None:
        source = b"prefix: The &quot;sum&quot;. suffix"
        result = b'The "sum".'
        start = source.index(b"The")
        end = source.index(b" suffix")
        transform = provenance.build_text_transform(
            source_sha256=provenance.sha256_bytes(source),
            source=source,
            result_sha256=provenance.sha256_bytes(result),
            result=result,
            operations=[
                {
                    "operation": "html_entity_decode",
                    "result_end": len(result),
                    "result_start": 0,
                    "source_end": end,
                    "source_start": start,
                }
            ],
        )
        self.assertEqual(transform["result_size"], len(result))

    def test_discard_transform_accounts_for_stripped_html_tags(self) -> None:
        source = b"<p>A &amp; B</p>"
        result = b"A & B"
        transform = provenance.build_text_transform(
            source_sha256=provenance.sha256_bytes(source),
            source=source,
            result_sha256=provenance.sha256_bytes(result),
            result=result,
            operations=[
                {
                    "claim_id": "claim",
                    "operation": "discard",
                    "reason": "opening paragraph tag is markup, not text",
                    "result_end": 0,
                    "result_start": 0,
                    "source_end": 3,
                    "source_start": 0,
                },
                {
                    "operation": "copy",
                    "result_end": 2,
                    "result_start": 0,
                    "source_end": 5,
                    "source_start": 3,
                },
                {
                    "operation": "html_entity_decode",
                    "result_end": 3,
                    "result_start": 2,
                    "source_end": 10,
                    "source_start": 5,
                },
                {
                    "operation": "copy",
                    "result_end": 5,
                    "result_start": 3,
                    "source_end": 12,
                    "source_start": 10,
                },
                {
                    "claim_id": "claim",
                    "operation": "discard",
                    "reason": "closing paragraph tag is markup, not text",
                    "result_end": 5,
                    "result_start": 5,
                    "source_end": len(source),
                    "source_start": 12,
                },
            ],
        )

        self.assertEqual(transform["result_size"], len(result))
        self.assertEqual(
            [operation["operation"] for operation in transform["operations"]],
            ["discard", "copy", "html_entity_decode", "copy", "discard"],
        )

    def test_discard_transform_requires_nonempty_reason(self) -> None:
        source = b"<p>"
        base_operation = {
            "claim_id": "claim",
            "operation": "discard",
            "result_end": 0,
            "result_start": 0,
            "source_end": len(source),
            "source_start": 0,
        }
        cases = [
            (base_operation, "exact operation schema"),
            ({**base_operation, "reason": ""}, "must be a non-empty string"),
        ]

        for operation, message in cases:
            with (
                self.subTest(operation=operation),
                self.assertRaisesRegex(provenance.ProvenanceError, message),
            ):
                provenance.build_text_transform(
                    source_sha256=provenance.sha256_bytes(source),
                    source=source,
                    result_sha256=provenance.sha256_bytes(b""),
                    result=b"",
                    operations=[operation],
                )

    def test_discard_transform_rejects_nonzero_result_range(self) -> None:
        source = b"<p>"
        with self.assertRaisesRegex(
            provenance.ProvenanceError, "non-canonical byte mapping"
        ):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"<"),
                result=b"<",
                operations=[
                    {
                        "claim_id": "claim",
                        "operation": "discard",
                        "reason": "opening paragraph tag is markup, not text",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": len(source),
                        "source_start": 0,
                    }
                ],
            )

    def test_discard_transform_rejects_noncanonical_source_order(self) -> None:
        source = b"<p>A</p>"
        result = b"A"
        with self.assertRaisesRegex(
            provenance.ProvenanceError, "non-canonical byte mapping"
        ):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(result),
                result=result,
                operations=[
                    {
                        "operation": "copy",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": 4,
                        "source_start": 3,
                    },
                    {
                        "claim_id": "claim",
                        "operation": "discard",
                        "reason": "opening paragraph tag is markup, not text",
                        "result_end": 1,
                        "result_start": 1,
                        "source_end": 3,
                        "source_start": 0,
                    },
                ],
            )

    def test_discard_transform_rejects_an_unaccounted_source_gap(self) -> None:
        source = b"<p> A</p>"
        with self.assertRaisesRegex(
            provenance.ProvenanceError, "non-canonical byte mapping"
        ):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"A"),
                result=b"A",
                operations=[
                    {
                        "claim_id": "claim",
                        "operation": "discard",
                        "reason": "opening paragraph tag is markup, not text",
                        "result_end": 0,
                        "result_start": 0,
                        "source_end": 3,
                        "source_start": 0,
                    },
                    {
                        "operation": "copy",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": 5,
                        "source_start": 4,
                    },
                    {
                        "claim_id": "claim",
                        "operation": "discard",
                        "reason": "closing paragraph tag is markup, not text",
                        "result_end": 1,
                        "result_start": 1,
                        "source_end": len(source),
                        "source_start": 5,
                    },
                ],
            )

    def test_discard_at_a_claim_boundary_has_one_explicit_owner(self) -> None:
        operations = [
            {
                "operation": "copy",
                "result_end": 1,
                "result_start": 0,
                "source_end": 1,
                "source_start": 0,
            },
            {
                "claim_id": "claim.a",
                "operation": "discard",
                "reason": "separator belongs to the first claim",
                "result_end": 1,
                "result_start": 1,
                "source_end": 2,
                "source_start": 1,
            },
            {
                "operation": "copy",
                "result_end": 2,
                "result_start": 1,
                "source_end": 3,
                "source_start": 2,
            },
        ]

        first = provenance._transform_operations_for_claim(
            operations, "claim.a", {"start": 0, "end": 1}
        )
        second = provenance._transform_operations_for_claim(
            operations, "claim.b", {"start": 1, "end": 2}
        )

        self.assertEqual(
            [operation["operation"] for operation in first], ["copy", "discard"]
        )
        self.assertEqual([operation["operation"] for operation in second], ["copy"])

    def test_schema_backed_adjacent_claim_discards_have_exact_owners(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cas = provenance.Cas(Path(directory) / "cas")
            raw = b"<p>A</p><p>B</p>"
            rendered = b"AB"
            raw_hash = cas.put(raw, kind="raw_source", label="two HTML claims")
            rendered_hash = cas.put(
                rendered,
                kind="rendered_text",
                label="two rendered claims",
                links=[raw_hash],
            )
            receipt = provenance.build_fetch_receipt(
                locator="https://example.test/two-claims",
                final_locator="https://example.test/two-claims",
                retrieved_at="2026-08-02T12:00:00Z",
                status=200,
                media_type="text/html",
                body_sha256=raw_hash,
                body_size=len(raw),
            )
            receipt_hash = cas.put_json(
                receipt,
                kind="fetch_receipt",
                label="two-claim receipt",
                links=[raw_hash],
            )

            def claim(claim_id: str, data: bytes, start: int, end: int) -> dict:
                quote = data[start:end]
                return {
                    "claim_id": claim_id,
                    "end": end,
                    "quote": quote.decode("utf-8"),
                    "quote_sha256": provenance.sha256_bytes(quote),
                    "start": start,
                }

            raw_ir = provenance.build_source_ir(
                source_sha256=raw_hash,
                source=raw,
                segments=[
                    {
                        "claims": [claim("claim.a", raw, 0, 8)],
                        "disposition": "represented",
                        "end": 8,
                        "start": 0,
                    },
                    {
                        "claims": [claim("claim.b", raw, 8, 16)],
                        "disposition": "represented",
                        "end": 16,
                        "start": 8,
                    },
                ],
            )
            raw_ir_hash = cas.put_json(
                raw_ir, kind="source_ir", label="two-claim raw IR", links=[raw_hash]
            )
            rendered_ir = provenance.build_source_ir(
                source_sha256=rendered_hash,
                source=rendered,
                segments=[
                    {
                        "claims": [claim("claim.a", rendered, 0, 1)],
                        "disposition": "represented",
                        "end": 1,
                        "start": 0,
                    },
                    {
                        "claims": [claim("claim.b", rendered, 1, 2)],
                        "disposition": "represented",
                        "end": 2,
                        "start": 1,
                    },
                ],
            )
            rendered_ir_hash = cas.put_json(
                rendered_ir,
                kind="source_ir",
                label="two-claim rendered IR",
                links=[rendered_hash],
            )
            transform = provenance.build_text_transform(
                source_sha256=raw_hash,
                source=raw,
                result_sha256=rendered_hash,
                result=rendered,
                operations=[
                    {
                        "claim_id": "claim.a",
                        "operation": "discard",
                        "reason": "first opening tag",
                        "result_end": 0,
                        "result_start": 0,
                        "source_end": 3,
                        "source_start": 0,
                    },
                    {
                        "operation": "copy",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": 4,
                        "source_start": 3,
                    },
                    {
                        "claim_id": "claim.a",
                        "operation": "discard",
                        "reason": "first closing tag",
                        "result_end": 1,
                        "result_start": 1,
                        "source_end": 8,
                        "source_start": 4,
                    },
                    {
                        "claim_id": "claim.b",
                        "operation": "discard",
                        "reason": "second opening tag",
                        "result_end": 1,
                        "result_start": 1,
                        "source_end": 11,
                        "source_start": 8,
                    },
                    {
                        "operation": "copy",
                        "result_end": 2,
                        "result_start": 1,
                        "source_end": 12,
                        "source_start": 11,
                    },
                    {
                        "claim_id": "claim.b",
                        "operation": "discard",
                        "reason": "second closing tag",
                        "result_end": 2,
                        "result_start": 2,
                        "source_end": 16,
                        "source_start": 12,
                    },
                ],
            )
            transform_hash = cas.put_json(
                transform,
                kind="text_transform",
                label="two-claim transform",
                links=[raw_hash, rendered_hash],
            )
            cas.write_index()
            schema = (
                Path(__file__).resolve().parents[2]
                / "specs/data/adj-stdlib-provenance/manifest.schema.json"
            )
            provenance._validate_cas_schemas(schema, cas)
            links, claims, _authorities = provenance._validate_source_entry(
                cas,
                {
                    "raw_source_sha256": raw_hash,
                    "receipt_sha256": receipt_hash,
                    "representations": [
                        {
                            "rendered_text_sha256": rendered_hash,
                            "source_ir_sha256": rendered_ir_hash,
                            "transform_sha256": transform_hash,
                        }
                    ],
                    "source_ir_sha256": raw_ir_hash,
                },
                "two_claim_source",
            )

            self.assertIn(transform_hash, links)
            self.assertEqual(set(claims[rendered_ir_hash]), {"claim.a", "claim.b"})

    def test_mathml_transform_reproduces_canonical_infix_bytes(self) -> None:
        source = (
            b"<math><semantics><mrow><mfrac><mi>n</mi><mn>100</mn></mfrac>"
            b'<mo>\xc3\x97</mo><mi>x</mi></mrow><annotation-xml encoding="MathML-Content">'
            b"<mrow><mfrac><mi>n</mi><mn>100</mn></mfrac><mo>\xc3\x97</mo>"
            b"<mi>x</mi></mrow></annotation-xml></semantics></math>"
        )
        result = b"(n/100)*x"
        transform = provenance.build_text_transform(
            source_sha256=provenance.sha256_bytes(source),
            source=source,
            result_sha256=provenance.sha256_bytes(result),
            result=result,
            operations=[
                {
                    "operation": "mathml_to_infix",
                    "result_end": len(result),
                    "result_start": 0,
                    "source_end": len(source),
                    "source_start": 0,
                }
            ],
        )
        self.assertEqual(transform["result_size"], len(result))

    def test_mathml_transform_rejects_unsupported_semantics(self) -> None:
        source = b"<math><msqrt><mi>x</mi></msqrt></math>"
        with self.assertRaisesRegex(provenance.ProvenanceError, "unsupported"):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"x"),
                result=b"x",
                operations=[
                    {
                        "operation": "mathml_to_infix",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": len(source),
                        "source_start": 0,
                    }
                ],
            )

    def test_mathml_transform_rejects_entity_declarations(self) -> None:
        source = b'<!DOCTYPE math [<!ENTITY x "expanded">]><math><mi>&x;</mi></math>'
        with self.assertRaisesRegex(provenance.ProvenanceError, "forbidden"):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"expanded"),
                result=b"expanded",
                operations=[
                    {
                        "operation": "mathml_to_infix",
                        "result_end": 8,
                        "result_start": 0,
                        "source_end": len(source),
                        "source_start": 0,
                    }
                ],
            )

    def test_mathml_transform_rejects_disagreeing_semantic_branch(self) -> None:
        source = (
            b"<math><semantics><mi>x</mi><annotation-xml><mi>y</mi>"
            b"</annotation-xml></semantics></math>"
        )
        with self.assertRaisesRegex(provenance.ProvenanceError, "disagree"):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"x"),
                result=b"x",
                operations=[
                    {
                        "operation": "mathml_to_infix",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": len(source),
                        "source_start": 0,
                    }
                ],
            )

    def test_mathml_transform_rejects_unaccounted_mixed_text(self) -> None:
        source = b"<math><mrow><mi>x</mi>contradiction</mrow></math>"
        with self.assertRaisesRegex(provenance.ProvenanceError, "mixed tail text"):
            provenance.build_text_transform(
                source_sha256=provenance.sha256_bytes(source),
                source=source,
                result_sha256=provenance.sha256_bytes(b"x"),
                result=b"x",
                operations=[
                    {
                        "operation": "mathml_to_infix",
                        "result_end": 1,
                        "result_start": 0,
                        "source_end": len(source),
                        "source_start": 0,
                    }
                ],
            )

    def test_unsuccessful_receipt_cannot_ground_a_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)

            def mutate(bundle: dict[str, object], cas: provenance.Cas) -> None:
                source = bundle["sources"][0]
                raw_hash = source["raw_source_sha256"]
                receipt = provenance.build_fetch_receipt(
                    locator="https://example.test/missing",
                    final_locator="https://example.test/missing",
                    retrieved_at="2026-08-02T12:00:00Z",
                    status=404,
                    media_type="text/plain",
                    body_sha256=raw_hash,
                    body_size=cas.index[raw_hash]["size"],
                )
                source["receipt_sha256"] = cas.put_json(
                    receipt,
                    kind="fetch_receipt",
                    label="404 receipt",
                    links=[raw_hash],
                )

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "unsuccessful"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_bundle_source_and_representation_schemas_fail_closed(self) -> None:
        mutations = (
            lambda bundle, _cas: bundle["sources"][0].update({"unexpected": True}),
            lambda bundle, _cas: bundle["sources"][0]["representations"][0].pop(
                "transform_sha256"
            ),
        )
        for mutate in mutations:
            with (
                self.subTest(mutate=mutate),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                cas_root, manifest_path, _ = self.build_repository(root)
                self.replace_bundle(cas_root, manifest_path, mutate)
                with self.assertRaisesRegex(
                    provenance.ProvenanceError, "exact .* schema"
                ):
                    provenance.validate_repository(cas_root, manifest_path)

    def test_clause_must_equal_its_byte_verified_ir_claim(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)

            def mutate(bundle: dict[str, object], _cas: provenance.Cas) -> None:
                bundle["clauses"][0]["quote"] = "different"

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "disagrees"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_transform_cannot_consume_raw_bytes_marked_discarded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)

            def mutate(bundle: dict[str, object], cas: provenance.Cas) -> None:
                source = bundle["sources"][0]
                raw_hash = source["raw_source_sha256"]
                raw = provenance._read_regular_file(cas.object_path(raw_hash))
                discarded_ir = provenance.build_source_ir(
                    source_sha256=raw_hash,
                    source=raw,
                    segments=[
                        {
                            "disposition": "discarded",
                            "end": len(raw),
                            "reason": "incorrectly discarded fixture",
                            "start": 0,
                        }
                    ],
                )
                source["source_ir_sha256"] = cas.put_json(
                    discarded_ir,
                    kind="source_ir",
                    label="discarded raw IR",
                    links=[raw_hash],
                )

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "not represented"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_discard_transform_partition_must_cover_its_raw_claim(self) -> None:
        schema = (
            Path(__file__).resolve().parents[2]
            / "specs/data/adj-stdlib-provenance/manifest.schema.json"
        )
        for include_leading_discard in (True, False):
            with (
                self.subTest(include_leading_discard=include_leading_discard),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                cas_root, manifest_path, _ = self.build_repository(root)

                def mutate(
                    bundle: dict[str, object],
                    cas: provenance.Cas,
                    include_leading_discard: bool = include_leading_discard,
                ) -> None:
                    source = bundle["sources"][0]
                    raw_hash = source["raw_source_sha256"]
                    raw = provenance._read_regular_file(cas.object_path(raw_hash))
                    claim_id = bundle["clauses"][0]["claim_id"]
                    raw_ir = provenance.build_source_ir(
                        source_sha256=raw_hash,
                        source=raw,
                        segments=[
                            {
                                "claims": [
                                    {
                                        "claim_id": claim_id,
                                        "end": len(raw),
                                        "quote": raw.decode("utf-8"),
                                        "quote_sha256": provenance.sha256_bytes(raw),
                                        "start": 0,
                                    }
                                ],
                                "disposition": "represented",
                                "end": len(raw),
                                "start": 0,
                            }
                        ],
                    )
                    source["source_ir_sha256"] = cas.put_json(
                        raw_ir,
                        kind="source_ir",
                        label="whole raw claim IR",
                        links=[raw_hash],
                    )
                    rendered_hash = bundle["clauses"][0]["snapshot_sha256"]
                    rendered = provenance._read_regular_file(
                        cas.object_path(rendered_hash)
                    )
                    start = raw.index(rendered)
                    end = start + len(rendered)
                    operations = []
                    if include_leading_discard:
                        operations.append(
                            {
                                "claim_id": claim_id,
                                "operation": "discard",
                                "reason": "header is outside the rendered definition",
                                "result_end": 0,
                                "result_start": 0,
                                "source_end": start,
                                "source_start": 0,
                            }
                        )
                    operations.extend(
                        [
                            {
                                "operation": "copy",
                                "result_end": len(rendered),
                                "result_start": 0,
                                "source_end": end,
                                "source_start": start,
                            },
                            {
                                "claim_id": claim_id,
                                "operation": "discard",
                                "reason": "footer is outside the rendered definition",
                                "result_end": len(rendered),
                                "result_start": len(rendered),
                                "source_end": len(raw),
                                "source_start": end,
                            },
                        ]
                    )
                    transform = provenance.build_text_transform(
                        source_sha256=raw_hash,
                        source=raw,
                        result_sha256=rendered_hash,
                        result=rendered,
                        operations=operations,
                    )
                    source["representations"][0]["transform_sha256"] = cas.put_json(
                        transform,
                        kind="text_transform",
                        label="explicit source partition transform",
                        links=[raw_hash, rendered_hash],
                    )

                self.replace_bundle(cas_root, manifest_path, mutate)
                if include_leading_discard:
                    cas = provenance.Cas(cas_root)
                    cas.load()
                    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                    reachable = provenance._reachable(cas, manifest["bundle_hashes"])
                    for digest in sorted(set(cas.index) - reachable):
                        cas.object_path(digest).unlink()
                    cas.index = {
                        digest: record
                        for digest, record in cas.index.items()
                        if digest in reachable
                    }
                    cas.write_index()
                    self.assertTrue(
                        provenance.validate_repository(cas_root, manifest_path, schema)[
                            "valid"
                        ]
                    )
                else:
                    with self.assertRaisesRegex(
                        provenance.ProvenanceError,
                        "explicit transform partition does not account",
                    ):
                        provenance.validate_repository(cas_root, manifest_path, schema)

    def test_dependency_resolution_names_an_exported_claim(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)

            def mutate(bundle: dict[str, object], _cas: provenance.Cas) -> None:
                bundle["dependencies"] = [hashes["bundle"]]
                bundle["clauses"][0]["resolution"] = {
                    "bundle_sha256": hashes["bundle"],
                    "claim_id": "missing.claim",
                    "kind": "dependency",
                }

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "does not export"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_accepted_root_names_its_typed_source_authority(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)

            def mutate(bundle: dict[str, object], _cas: provenance.Cas) -> None:
                bundle["clauses"][0]["resolution"]["authority_source_sha256"] = hashes[
                    "rendered"
                ]

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "authority"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_tampered_object_bytes_fail_rehash(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            cas.object_path(hashes["raw"]).write_bytes(b"changed")

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "wrong size|rehash"
            ):
                provenance.validate_repository(cas_root, manifest_path)

    @unittest.skipIf(
        os.name == "nt", "creating symlinks is not reliably permitted on Windows"
    )
    def test_symlinked_object_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            object_path = cas.object_path(hashes["raw"])
            target = root / "elsewhere"
            target.write_bytes(object_path.read_bytes())
            object_path.unlink()
            object_path.symlink_to(target)

            with self.assertRaisesRegex(provenance.ProvenanceError, "link or reparse"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_receipt_rejects_credentials_and_sensitive_headers(self) -> None:
        with self.assertRaises(provenance.ProvenanceError):
            provenance.build_fetch_receipt(
                locator="https://user:secret@example.test/source",
                final_locator="https://example.test/source",
                retrieved_at="2026-08-02T12:00:00Z",
                status=200,
                media_type="text/plain",
                body_sha256="a" * 64,
                body_size=1,
            )
        with self.assertRaisesRegex(provenance.ProvenanceError, "not allow-listed"):
            provenance.build_fetch_receipt(
                locator="https://example.test/source",
                final_locator="https://example.test/source",
                retrieved_at="2026-08-02T12:00:00Z",
                status=200,
                media_type="text/plain",
                body_sha256="a" * 64,
                body_size=1,
                headers={"Set-Cookie": "secret=1"},
            )

    def test_input_receipt_pins_repo_path_and_git_blob(self) -> None:
        body = b"formula sum(a, b) = a + b\n"
        receipt = provenance.build_input_receipt(
            repo_path="code/example.adj",
            captured_at="2026-08-02T12:00:00Z",
            body_sha256=provenance.sha256_bytes(body),
            body_size=len(body),
            body_git_sha1=provenance.git_blob_sha1(body),
        )
        self.assertEqual(receipt["body_git_sha1"], provenance.git_blob_sha1(body))
        with self.assertRaisesRegex(provenance.ProvenanceError, "repository-relative"):
            provenance.build_input_receipt(
                repo_path="../outside.adj",
                captured_at="2026-08-02T12:00:00Z",
                body_sha256=provenance.sha256_bytes(body),
                body_size=len(body),
                body_git_sha1=provenance.git_blob_sha1(body),
            )

    def test_receipt_and_ir_must_link_the_same_raw_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            cas.index[hashes["receipt"]]["links"] = [hashes["rendered"]]
            cas.write_index()

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "receipt must link"
            ):
                provenance.validate_repository(cas_root, manifest_path)

    def test_bundle_requires_matching_decomposed_input_claim(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)

            def mutate(bundle: dict[str, object], cas: provenance.Cas) -> None:
                source = bundle["sources"][1]
                input_hash = source["raw_source_sha256"]
                body = provenance._read_regular_file(cas.object_path(input_hash))
                empty_ir = provenance.build_source_ir(
                    source_sha256=input_hash,
                    source=body,
                    segments=[
                        {
                            "disposition": "discarded",
                            "end": len(body),
                            "reason": "incorrectly omitted code claim",
                            "start": 0,
                        }
                    ],
                )
                empty_ir_hash = cas.put_json(
                    empty_ir,
                    kind="source_ir",
                    label="empty input IR",
                    links=[input_hash],
                )
                source["source_ir_sha256"] = empty_ir_hash
                bundle["input"]["source_ir_sha256"] = empty_ir_hash

            self.replace_bundle(cas_root, manifest_path, mutate)

            with self.assertRaisesRegex(provenance.ProvenanceError, "input claim"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_unreferenced_objects_fail_complete_graph_accounting(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            cas.put(b"orphan", kind="rendered_text", label="orphan")
            cas.write_index()

            with self.assertRaisesRegex(provenance.ProvenanceError, "unreferenced CAS"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_raw_source_index_links_cannot_hide_an_orphan(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            orphan = cas.put(b"orphan", kind="rendered_text", label="orphan")
            cas.index[hashes["raw"]]["links"] = [orphan]
            cas.write_index()

            with self.assertRaisesRegex(provenance.ProvenanceError, "raw source"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_alternate_fanout_path_cannot_hide_unindexed_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            alternate = cas_root / "objects" / hashes["raw"][:1] / hashes["raw"][1:]
            alternate.parent.mkdir()
            alternate.write_bytes(b"extra")

            with self.assertRaisesRegex(provenance.ProvenanceError, "non-canonical"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_projection_refuses_unaccounted_output_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(root)
            output = root / "projection"
            output.mkdir()
            (output / "unexpected").write_text("x", encoding="utf-8")

            with self.assertRaisesRegex(provenance.ProvenanceError, "unexpected entry"):
                provenance.project_snapshots(cas_root, manifest_path, output)

    def test_projection_refuses_existing_snapshot_with_wrong_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            output = root / "projection"
            output.mkdir()
            (output / hashes["snapshot"]).write_bytes(b"wrong")

            with self.assertRaises(provenance.ProvenanceError):
                provenance.project_snapshots(cas_root, manifest_path, output)


if __name__ == "__main__":
    unittest.main()
