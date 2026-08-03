from __future__ import annotations

import importlib
import json
import multiprocessing
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

provenance = importlib.import_module("adj_stdlib_provenance")
ratio_builder = importlib.import_module("build_adj_ratio_provenance")


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
    def build_repository(self, root: Path) -> tuple[Path, Path, dict[str, str]]:
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
            b'formula sum(a, b) = a + b\n  locator "https://example.test/sum"\n'
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
                "bundle": bundle_hash,
                "transform": transform_hash,
                "snapshot": rendered_hash,
            },
        )

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
            shutil.copytree(provenance_source, provenance_copy)
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
                    transaction.commit(ratio_builder.build(transaction.cas, None))
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
            self.assertEqual(
                ratio_bundle["dependencies"], [ratio_builder.ARITHMETIC_BUNDLE_HASH]
            )
            self.assertEqual(
                ratio_bundle["clauses"][0]["resolution"],
                {
                    "bundle_sha256": ratio_builder.ARITHMETIC_BUNDLE_HASH,
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
            arithmetic_roots = []
            for digest in manifest["bundle_hashes"]:
                bundle_id = provenance._json_object(cas, digest, "provenance_bundle")[
                    "bundle_id"
                ]
                if bundle_id.startswith("adj.math.arithmetic.primitives"):
                    arithmetic_roots.append(digest)
            reachable = provenance._reachable(cas, arithmetic_roots)
            for digest in sorted(set(cas.index) - reachable):
                cas.object_path(digest).unlink()
            cas.index = {
                digest: record
                for digest, record in cas.index.items()
                if digest in reachable
            }
            cas.write_index()
            manifest["bundle_hashes"] = sorted(arithmetic_roots)
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
            arithmetic_only = provenance.validate_repository(
                cas_root,
                manifest_path,
                workspace / provenance.DEFAULT_SCHEMA,
                workspace_root=workspace,
            )
            self.assertEqual(
                (arithmetic_only["bundles"], arithmetic_only["objects"]), (2, 35)
            )

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
                        ratio_builder.build(transaction.cas, captured_source)
                    )
            finally:
                ratio_builder.REPO_ROOT = original_root
            self.assertEqual((cas_root / "index.json").read_bytes(), baseline_index)
            self.assertEqual(manifest_path.read_bytes(), baseline_manifest)

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
