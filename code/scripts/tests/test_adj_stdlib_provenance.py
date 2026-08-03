from __future__ import annotations

import ctypes
import importlib
import io
import json
import multiprocessing
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from contextlib import redirect_stdout
from copy import deepcopy
from dataclasses import FrozenInstanceError
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

provenance = importlib.import_module("adj_stdlib_provenance")
guardian = importlib.import_module("adj_process_guardian")
arithmetic_builder = importlib.import_module("build_adj_arithmetic_provenance")
ratio_builder = importlib.import_module("build_adj_ratio_provenance")
percent_of_builder = importlib.import_module("build_adj_percent_of_provenance")
formula_inventory_migration = importlib.import_module("migrate_adj_formula_inventories")


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
    class FakePosixGuardian:
        CONTRACT = "adj-stdlib/process-guardian/v1"

        def __init__(
            self,
            command: object,
            cleanup_timeout_seconds: float,
            *,
            status: dict[str, object] | None = None,
        ) -> None:
            self.command = ["guardian", *command]
            self.cleanup_timeout_seconds = cleanup_timeout_seconds
            self.popen_options = {"start_new_session": True}
            self.status = status or {
                "cleanup_confirmed": True,
                "contract": self.CONTRACT,
                "returncode": 0,
                "verifier_gone": False,
            }
            self.cleanup_requests = 0
            self.parent_started_calls = 0
            self.launch_failed_calls = 0
            self.close_calls = 0

        def parent_started(self) -> None:
            self.parent_started_calls += 1

        def launch_failed(self) -> None:
            self.launch_failed_calls += 1

        def request_cleanup(self) -> bool:
            if self.cleanup_requests:
                return False
            self.cleanup_requests = 1
            return True

        def read_status(self, timeout_seconds: float) -> dict[str, object]:
            self.assert_positive_timeout = timeout_seconds > 0
            self.request_cleanup()
            return self.status

        def close(self) -> None:
            self.close_calls += 1

    def test_lifecycle_failure_is_immutable_and_canonical(self) -> None:
        cleanup = provenance.LifecycleFailure(
            stage="job.close",
            api="CloseHandle",
            error_code=6,
            message="close failed",
        )
        failure = provenance.LifecycleFailure(
            stage="job.configure",
            api="SetInformationJobObject",
            error_code=87,
            message="configuration failed",
            cleanup_causes=(cleanup,),
        )
        expected = {
            "api": "SetInformationJobObject",
            "cleanup_causes": [
                {
                    "api": "CloseHandle",
                    "cleanup_causes": [],
                    "error_code": 6,
                    "message": "close failed",
                    "stage": "job.close",
                    "status_code": None,
                }
            ],
            "contract": "adj-stdlib/process-lifecycle-failure/v2",
            "error_code": 87,
            "message": "configuration failed",
            "stage": "job.configure",
            "status_code": None,
        }

        self.assertEqual(failure.to_dict(), expected)
        self.assertEqual(
            provenance.canonical_json_bytes(failure.to_dict()),
            provenance.canonical_json_bytes(expected),
        )
        extended = failure.with_cleanup(
            provenance.LifecycleFailure(
                stage="process.terminate", message="fallback failed"
            )
        )
        self.assertEqual(len(failure.cleanup_causes), 1)
        self.assertEqual(len(extended.cleanup_causes), 2)
        projected = failure.to_dict()
        projected["stage"] = "changed"
        projected["cleanup_causes"][0]["error_code"] = 999
        self.assertEqual(failure.stage, "job.configure")
        self.assertEqual(failure.cleanup_causes[0].error_code, 6)
        with self.assertRaises(FrozenInstanceError):
            failure.stage = "changed"  # type: ignore[misc]
        with self.assertRaisesRegex(ValueError, "unknown lifecycle failure stage"):
            provenance.LifecycleFailure(
                stage="free-form.stage", message="not allowed"
            )
        for field, value, expected_message in (
            ("message", "", "message must be a non-empty string"),
            ("api", "", "API must be null or a non-empty string"),
            ("error_code", True, "error code must be an integer or null"),
            (
                "status_code",
                "",
                "status code must be null or a non-empty string",
            ),
            (
                "cleanup_causes",
                [cleanup],
                "cleanup causes must be a tuple",
            ),
            (
                "cleanup_causes",
                ("not-a-failure",),
                "cleanup causes must be a tuple",
            ),
        ):
            with self.subTest(field=field, value=value):
                arguments = {
                    "stage": "job.configure",
                    "message": "valid message",
                    field: value,
                }
                with self.assertRaisesRegex(ValueError, expected_message):
                    provenance.LifecycleFailure(**arguments)

    def test_provenance_error_keeps_legacy_text_without_lifecycle(self) -> None:
        error = provenance.ProvenanceError("legacy validation failure")

        self.assertEqual(str(error), "legacy validation failure")
        self.assertEqual(error.args, ("legacy validation failure",))
        self.assertIsNone(error.lifecycle)

        lifecycle = provenance.LifecycleFailure(
            stage="job.create", message="native display text"
        )
        structured = provenance._LifecycleError(lifecycle)
        wrapped = provenance.ProvenanceError(
            "legacy wrapper text", lifecycle=lifecycle
        )
        self.assertEqual(str(structured), "native display text")
        self.assertEqual(structured.args, ("native display text",))
        self.assertEqual(str(wrapped), "legacy wrapper text")
        self.assertEqual(wrapped.args, ("legacy wrapper text",))
        self.assertIs(wrapped.lifecycle, lifecycle)

    def test_lifecycle_identity_does_not_depend_on_display_message(self) -> None:
        failures = [
            provenance._failure_from_error(
                OSError(message),
                stage="job.assign",
                api="AssignProcessToJobObject",
                error_code=5,
            )
            for message in ("WrongApi code 999", "locale-B unrelated")
        ]

        self.assertEqual(
            [
                (failure.stage, failure.api, failure.error_code)
                for failure in failures
            ],
            [
                ("job.assign", "AssignProcessToJobObject", 5),
                ("job.assign", "AssignProcessToJobObject", 5),
            ],
        )
        self.assertNotEqual(failures[0].message, failures[1].message)

    def test_cli_projects_lifecycle_failure_without_replacing_legacy_fields(self) -> None:
        lifecycle = provenance.LifecycleFailure(
            stage="command.timeout",
            api="Popen.wait",
            message="formula audit timed out",
        )
        error = provenance.ProvenanceError(
            "legacy CLI error", lifecycle=lifecycle
        )
        output = io.StringIO()

        with (
            mock.patch.object(sys, "argv", ["adj_stdlib_provenance.py", "verify"]),
            mock.patch.object(
                provenance, "validate_repository", side_effect=error
            ),
            redirect_stdout(output),
        ):
            returncode = provenance.main()

        self.assertEqual(returncode, 1)
        self.assertEqual(
            json.loads(output.getvalue()),
            {
                "error": "legacy CLI error",
                "lifecycle_failure": lifecycle.to_dict(),
                "valid": False,
            },
        )

    def test_cli_plain_provenance_error_keeps_original_key_set(self) -> None:
        output = io.StringIO()

        with (
            mock.patch.object(sys, "argv", ["adj_stdlib_provenance.py", "verify"]),
            mock.patch.object(
                provenance,
                "validate_repository",
                side_effect=provenance.ProvenanceError("plain failure"),
            ),
            redirect_stdout(output),
        ):
            returncode = provenance.main()

        self.assertEqual(returncode, 1)
        self.assertEqual(
            json.loads(output.getvalue()),
            {"error": "plain failure", "valid": False},
        )

    def test_cli_projects_unwrapped_native_lifecycle_error(self) -> None:
        lifecycle = provenance.LifecycleFailure(
            stage="job.create",
            api="CreateJobObjectW",
            error_code=5,
            message="native failure",
        )
        output = io.StringIO()

        with (
            mock.patch.object(sys, "argv", ["adj_stdlib_provenance.py", "verify"]),
            mock.patch.object(
                provenance,
                "validate_repository",
                side_effect=provenance._LifecycleError(lifecycle),
            ),
            redirect_stdout(output),
        ):
            returncode = provenance.main()

        self.assertEqual(returncode, 1)
        self.assertEqual(
            json.loads(output.getvalue())["lifecycle_failure"],
            lifecycle.to_dict(),
        )

    def test_formula_replay_failure_reaches_cli_with_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, _ = self.build_repository(
                root, with_formula_inventory=True
            )
            schema_path = (
                Path(__file__).resolve().parents[3] / provenance.DEFAULT_SCHEMA
            )
            output = io.StringIO()
            launch_error = OSError(13, "translated access denied")
            launch_error.winerror = 5  # type: ignore[attr-defined]

            with (
                mock.patch.object(
                    sys,
                    "argv",
                    [
                        "adj_stdlib_provenance.py",
                        "--repo-root",
                        str(root),
                        "verify",
                        "--cas",
                        str(cas_root),
                        "--manifest",
                        str(manifest_path),
                        "--schema",
                        str(schema_path),
                        "--formula-inventory-binary",
                        "missing-parser",
                    ],
                ),
                mock.patch.object(
                    provenance.subprocess, "Popen", side_effect=launch_error
                ),
                redirect_stdout(output),
            ):
                returncode = provenance.main()

        self.assertEqual(returncode, 1)
        payload = json.loads(output.getvalue())
        lifecycle = payload["lifecycle_failure"]
        self.assertEqual(
            (lifecycle["stage"], lifecycle["api"], lifecycle["error_code"]),
            ("process.launch", "Popen", 5),
        )
        self.assertEqual(payload["error"], lifecycle["message"])
        self.assertFalse(payload["valid"])

    def fake_windows_kernel32(self) -> mock.Mock:
        kernel32 = mock.Mock()
        kernel32.CreateJobObjectW.return_value = 101
        kernel32.SetInformationJobObject.return_value = 1
        kernel32.AssignProcessToJobObject.return_value = 1
        kernel32.TerminateJobObject.return_value = 1
        kernel32.CloseHandle.return_value = 1
        kernel32.CreateToolhelp32Snapshot.return_value = 202
        kernel32.Thread32First.return_value = 0
        kernel32.Thread32Next.return_value = 0
        kernel32.OpenThread.return_value = 303
        kernel32.ResumeThread.return_value = 1
        return kernel32

    def windows_job(
        self, kernel32: mock.Mock, error_code: list[int] | None = None
    ) -> object:
        errors = error_code or [5]
        return provenance._WindowsKillJob(
            kernel32, lambda: errors[0], mock.Mock()
        )

    def windows_process(self, *, pid: int = 404) -> mock.Mock:
        process = mock.Mock(pid=pid)
        process._handle = 505
        return process

    def process_is_alive(self, pid: int) -> bool:
        if os.name == "nt":
            kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
            kernel32.OpenProcess.argtypes = [
                ctypes.c_ulong,
                ctypes.c_int,
                ctypes.c_ulong,
            ]
            kernel32.OpenProcess.restype = ctypes.c_void_p
            kernel32.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
            kernel32.WaitForSingleObject.restype = ctypes.c_ulong
            kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
            handle = kernel32.OpenProcess(0x00100000, False, pid)
            if not handle:
                return False
            try:
                return kernel32.WaitForSingleObject(handle, 0) == 0x00000102
            finally:
                kernel32.CloseHandle(handle)
        try:
            os.kill(pid, 0)
        except OSError:
            return False
        return True

    def assert_process_exits(self, pid: int, *, timeout: float = 3) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if not self.process_is_alive(pid):
                return
            time.sleep(0.05)
        self.fail(f"process {pid} remained alive")

    def linux_process_starttime(self, pid: int) -> str | None:
        try:
            raw = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
        except (OSError, UnicodeDecodeError):
            return None
        tail = raw.rsplit(")", 1)
        if len(tail) != 2:
            return None
        fields = tail[1].split()
        return fields[19] if len(fields) > 19 else None

    def assert_linux_process_identity_exits(
        self, pid: int, starttime: str, *, timeout: float = 5
    ) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.linux_process_starttime(pid) != starttime:
                return
            time.sleep(0.05)
        self.fail(f"Linux process identity {pid}:{starttime} remained alive")

    def rust_binary_command(self, name: str) -> list[str]:
        suffix = ".exe" if os.name == "nt" else ""
        binary = Path(__file__).resolve().parents[2] / f"packages/rust/target/debug/{name}{suffix}"
        self.assertTrue(binary.is_file(), f"missing {name} binary: {binary}")
        return [os.fspath(binary)]

    def formula_inventory_command(self) -> list[str]:
        return self.rust_binary_command("adj-formula-inventory")

    def formula_audit_command(self) -> list[str]:
        return self.rust_binary_command("adj-formula-audit")

    def migrate_formula_closure(self, workspace: Path) -> None:
        original_roots = (
            arithmetic_builder.REPO_ROOT,
            ratio_builder.REPO_ROOT,
            percent_of_builder.REPO_ROOT,
        )
        arithmetic_builder.REPO_ROOT = workspace
        ratio_builder.REPO_ROOT = workspace
        percent_of_builder.REPO_ROOT = workspace
        try:
            formula_inventory_migration.migrate(
                workspace / provenance.DEFAULT_ROOT,
                workspace / provenance.DEFAULT_MANIFEST,
                workspace / provenance.DEFAULT_SCHEMA,
                workspace,
                formula_inventory_command=self.formula_inventory_command(),
                formula_audit_command=self.formula_audit_command(),
            )
        finally:
            (
                arithmetic_builder.REPO_ROOT,
                ratio_builder.REPO_ROOT,
                percent_of_builder.REPO_ROOT,
            ) = original_roots

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

    def remove_formula_inventories_from_closure(
        self, cas_root: Path, manifest_path: Path
    ) -> dict[str, str]:
        cas = provenance.Cas(cas_root)
        cas.load()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        current = formula_inventory_migration._registered_roots(cas, manifest_path)

        def store(bundle: dict[str, object], label: str) -> str:
            return cas.put_json(
                bundle,
                kind="provenance_bundle",
                label=label,
                links=provenance._bundle_declared_links(bundle),
            )

        arithmetic_id = "adj.math.arithmetic.primitives.v1"
        arithmetic = provenance._json_object(
            cas, current[arithmetic_id], "provenance_bundle"
        )
        arithmetic.pop("formula_inventory_sha256")
        old_roots = {
            arithmetic_id: store(arithmetic, "pre-inventory arithmetic bundle")
        }

        for bundle_id in (
            "adj.math.arithmetic.ratio.v1",
            "adj.math.arithmetic.percent_of.v1",
        ):
            bundle = provenance._json_object(
                cas, current[bundle_id], "provenance_bundle"
            )
            bundle.pop("formula_inventory_sha256")
            bundle["dependencies"] = [old_roots[arithmetic_id]]
            if bundle_id == "adj.math.arithmetic.ratio.v1":
                bundle["clauses"][0]["resolution"]["bundle_sha256"] = old_roots[
                    arithmetic_id
                ]
            old_roots[bundle_id] = store(bundle, f"pre-inventory {bundle_id} bundle")

        query_dependencies = {
            "adj.math.arithmetic.percent_of.query.v1": (
                "adj.math.arithmetic.percent_of.v1"
            ),
            "adj.math.arithmetic.primitives.query.v1": arithmetic_id,
            "adj.math.arithmetic.ratio.query.v1": "adj.math.arithmetic.ratio.v1",
        }
        for bundle_id, dependency_id in query_dependencies.items():
            bundle = provenance._json_object(
                cas, current[bundle_id], "provenance_bundle"
            )
            bundle.pop("formula_derivation_sha256s", None)
            bundle.pop("execution_witness_sha256s", None)
            bundle["dependencies"] = [old_roots[dependency_id]]
            old_roots[bundle_id] = store(bundle, f"pre-inventory {bundle_id} bundle")

        final_roots = {
            bundle_id: old_roots.get(bundle_id, digest)
            for bundle_id, digest in current.items()
        }
        manifest["bundle_hashes"] = sorted(final_roots.values())
        reachable = provenance._reachable(cas, manifest["bundle_hashes"])
        for digest in sorted(set(cas.index) - reachable):
            cas.object_path(digest).unlink()
        cas.index = {
            digest: record
            for digest, record in cas.index.items()
            if digest in reachable
        }
        cas.write_index()
        manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
        return old_roots

    def downgrade_formula_inputs_to_legacy(
        self, workspace: Path
    ) -> dict[str, str]:
        cas_root = workspace / provenance.DEFAULT_ROOT
        manifest_path = workspace / provenance.DEFAULT_MANIFEST
        cas = provenance.Cas(cas_root)
        cas.load()
        roots = formula_inventory_migration._registered_roots(cas, manifest_path)
        legacy_roots = dict(roots)
        for bundle_id, digest in roots.items():
            query = provenance._json_object(cas, digest, "provenance_bundle")
            if "execution_witness_sha256s" not in query:
                continue
            audit = provenance._materialize_formula_audit(
                cas, query, self.formula_audit_command()
            )
            legacy = provenance._normalized_formula_evidence(
                cas, query, audit, legacy_input_references=True
            )
            derivation_hashes = []
            witness_hashes = []
            for derivation, derivation_links, witness, witness_links in legacy:
                derivation_hash = cas.put_json(
                    derivation,
                    kind="formula_derivation",
                    label=f"{bundle_id} legacy derivation",
                    links=derivation_links,
                )
                witness["formula_derivation_sha256"] = derivation_hash
                witness_hash = cas.put_json(
                    witness,
                    kind="execution_witness",
                    label=f"{bundle_id} legacy witness",
                    links=witness_links | {derivation_hash},
                )
                derivation_hashes.append(derivation_hash)
                witness_hashes.append(witness_hash)
            query["formula_derivation_sha256s"] = sorted(derivation_hashes)
            query["execution_witness_sha256s"] = sorted(witness_hashes)
            legacy_roots[bundle_id] = cas.put_json(
                query,
                kind="provenance_bundle",
                label=f"{bundle_id} legacy query root",
                links=provenance._bundle_declared_links(query),
            )

        reachable = provenance._reachable(cas, legacy_roots.values())
        for digest in sorted(set(cas.index) - reachable):
            cas.object_path(digest).unlink()
        cas.index = {
            digest: record
            for digest, record in cas.index.items()
            if digest in reachable
        }
        cas.write_index()
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        manifest["bundle_hashes"] = sorted(legacy_roots.values())
        manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))
        return legacy_roots

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

            result = provenance._validate_repository_unlocked(
                cas_root,
                manifest_path,
                schema,
                workspace_root=root,
                formula_inventory_command=self.formula_inventory_command(),
                _allow_unwitnessed=True,
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
                ) as raised,
            ):
                provenance._run_formula_inventory(command, source)

            self.assertEqual(
                raised.exception.lifecycle.stage, "command.output_limit"
            )

    def test_formula_inventory_execution_exposes_lifecycle_failures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.adj"
            source.write_bytes(b"")
            launch_error = OSError(13, "translated access denied")
            launch_error.winerror = 5  # type: ignore[attr-defined]

            with (
                mock.patch.object(
                    provenance.subprocess, "Popen", side_effect=launch_error
                ),
                self.assertRaisesRegex(
                    provenance.ProvenanceError, "failed to run"
                ) as raised,
            ):
                provenance._run_formula_inventory(["parser"], source)

            self.assertEqual(
                (
                    raised.exception.lifecycle.stage,
                    raised.exception.lifecycle.api,
                    raised.exception.lifecycle.error_code,
                ),
                ("process.launch", "Popen", 5),
            )

            for payload, expected_stage in (
                ("import sys; sys.stdout.buffer.write(b'\\xff')", "command.decode"),
                ("print('not-json')", "command.parse"),
            ):
                with (
                    self.subTest(expected_stage=expected_stage),
                    self.assertRaisesRegex(
                        provenance.ProvenanceError, "did not emit UTF-8 JSON"
                    ) as raised,
                ):
                    provenance._run_formula_inventory(
                        [sys.executable, "-c", payload], source
                    )

                self.assertEqual(
                    raised.exception.lifecycle.stage, expected_stage
                )

    def test_windows_job_creation_failure_is_named_and_fail_closed(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        kernel32.CreateJobObjectW.return_value = 0

        with self.assertRaisesRegex(OSError, "CreateJobObjectW failed"):
            self.windows_job(kernel32)

        kernel32.CloseHandle.assert_not_called()

    def test_windows_job_setup_failure_closes_created_handle(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        kernel32.SetInformationJobObject.return_value = 0

        with self.assertRaisesRegex(OSError, "SetInformationJobObject failed"):
            self.windows_job(kernel32)

        kernel32.CloseHandle.assert_called_once_with(101)

    def test_windows_job_setup_installs_kill_on_close(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        handle, information_class, pointer, size = (
            kernel32.SetInformationJobObject.call_args.args
        )
        self.assertEqual(handle, 101)
        self.assertEqual(information_class, 9)
        self.assertEqual(
            pointer._obj.basic_limit_information.limit_flags,  # type: ignore[attr-defined]
            0x00002000,
        )
        self.assertEqual(size, ctypes.sizeof(pointer._obj))  # type: ignore[attr-defined]
        job.close()

    def test_windows_job_setup_preserves_close_failure(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        kernel32.SetInformationJobObject.return_value = 0
        kernel32.CloseHandle.return_value = 0

        with self.assertRaisesRegex(
            OSError,
            "SetInformationJobObject failed.*cleanup also failed.*CloseHandle\\(job\\)",
        ):
            self.windows_job(kernel32)

        self.assertEqual(kernel32.CloseHandle.call_count, 2)

    def test_windows_job_setup_retries_transient_close_failure(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        kernel32.SetInformationJobObject.return_value = 0
        kernel32.CloseHandle.side_effect = [0, 1]

        with self.assertRaisesRegex(
            OSError, "SetInformationJobObject failed"
        ) as raised:
            self.windows_job(kernel32)

        detail = (
            str(ctypes.WinError(5))
            if hasattr(ctypes, "WinError")
            else "Windows error 5"
        )
        legacy = OSError(5, f"SetInformationJobObject failed: {detail}")
        self.assertEqual(raised.exception.args, legacy.args)
        self.assertEqual(str(raised.exception), str(legacy))
        self.assertEqual(raised.exception.failure.cleanup_causes, ())
        self.assertEqual(kernel32.CloseHandle.call_count, 2)

    def test_windows_job_preserves_distinct_native_error_codes(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        error_register = [0]
        close_codes = iter((6, 32))

        def fail_setup(*_arguments: object) -> int:
            error_register[0] = 87
            return 0

        def fail_close(_handle: int) -> int:
            error_register[0] = next(close_codes)
            return 0

        kernel32.SetInformationJobObject.side_effect = fail_setup
        kernel32.CloseHandle.side_effect = fail_close

        with self.assertRaisesRegex(
            OSError,
            "SetInformationJobObject failed.*87.*CloseHandle\\(job\\) failed.*6.*CloseHandle\\(job\\) failed.*32",
        ) as raised:
            provenance._WindowsKillJob(
                kernel32,
                lambda: error_register[0],
                lambda value: error_register.__setitem__(0, value),
            )

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("job.configure", "SetInformationJobObject", 87),
        )
        self.assertEqual(len(failure.cleanup_causes), 1)
        close = failure.cleanup_causes[0]
        self.assertEqual(
            (close.stage, close.api, close.error_code),
            ("job.close", "CloseHandle", 6),
        )
        self.assertEqual(len(close.cleanup_causes), 1)
        self.assertEqual(close.cleanup_causes[0].error_code, 32)

    def test_windows_job_assignment_failure_is_named(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)
        kernel32.AssignProcessToJobObject.return_value = 0

        with self.assertRaisesRegex(
            OSError, "AssignProcessToJobObject failed"
        ) as raised:
            job.assign(self.windows_process())

        if hasattr(ctypes, "WinError"):
            detail = str(ctypes.WinError(5))
        else:
            detail = "Windows error 5"
        legacy = OSError(5, f"AssignProcessToJobObject failed: {detail}")
        self.assertEqual(raised.exception.args, legacy.args)
        self.assertEqual(str(raised.exception), str(legacy))
        job.close()

    def test_windows_job_closed_assignment_uses_local_state_error(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)
        job.close()

        with self.assertRaisesRegex(OSError, "closed Windows Job"):
            job.assign(self.windows_process())

        kernel32.AssignProcessToJobObject.assert_not_called()

    def test_windows_job_snapshot_failure_is_named(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)
        kernel32.CreateToolhelp32Snapshot.return_value = ctypes.c_void_p(-1).value

        with self.assertRaisesRegex(OSError, "CreateToolhelp32Snapshot failed"):
            job.resume(self.windows_process())

        job.close()

    def test_windows_job_thread_first_failure_is_not_treated_as_end(self) -> None:
        for error_code in (0, 5):
            with self.subTest(error_code=error_code):
                kernel32 = self.fake_windows_kernel32()
                job = self.windows_job(kernel32, [error_code])

                with self.assertRaisesRegex(OSError, "Thread32First failed"):
                    job.resume(self.windows_process())

                job._set_last_error.assert_called_once_with(0)
                kernel32.CloseHandle.assert_called_once_with(202)
                job.close()

    def test_windows_job_thread_error_is_captured_before_cleanup(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        error_register = [99]

        def fail_first(_snapshot: int, _pointer: object) -> int:
            error_register[0] = 5
            return 0

        def close_with_different_error(_handle: int) -> int:
            error_register[0] = 6
            return 1

        kernel32.Thread32First.side_effect = fail_first
        kernel32.CloseHandle.side_effect = close_with_different_error
        job = provenance._WindowsKillJob(
            kernel32,
            lambda: error_register[0],
            lambda value: error_register.__setitem__(0, value),
        )

        with self.assertRaisesRegex(OSError, "Thread32First failed.*5") as raised:
            job.resume(self.windows_process())

        self.assertEqual(
            (
                raised.exception.failure.stage,
                raised.exception.failure.api,
                raised.exception.failure.error_code,
            ),
            ("thread.enumerate", "Thread32First", 5),
        )
        self.assertEqual(error_register, [6])
        job.close()

    def test_windows_job_thread_first_no_more_files_means_no_thread(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32, [provenance._WindowsKillJob.ERROR_NO_MORE_FILES])

        with self.assertRaisesRegex(OSError, "has no primary thread"):
            job.resume(self.windows_process(pid=404))

        kernel32.CloseHandle.assert_called_once_with(202)
        job.close()

    def test_windows_job_thread_next_failure_is_not_treated_as_end(self) -> None:
        for error_code in (0, 5):
            with self.subTest(error_code=error_code):
                kernel32 = self.fake_windows_kernel32()
                job = self.windows_job(kernel32, [error_code])

                def unrelated_thread(_snapshot: int, pointer: object) -> int:
                    pointer._obj.owner_process_id = 999  # type: ignore[attr-defined]
                    pointer._obj.thread_id = 606  # type: ignore[attr-defined]
                    return 1

                kernel32.Thread32First.side_effect = unrelated_thread

                with self.assertRaisesRegex(OSError, "Thread32Next failed"):
                    job.resume(self.windows_process(pid=404))

                self.assertEqual(
                    job._set_last_error.call_args_list,
                    [mock.call(0), mock.call(0)],
                )
                job.close()

    def test_windows_job_rejects_truncated_thread_record(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def truncated_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.size = job._thread_entry_owner_end - 1  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = truncated_thread

        with self.assertRaisesRegex(OSError, "truncated THREADENTRY32"):
            job.resume(self.windows_process())

        job.close()

    def test_windows_job_resets_thread_record_size_before_next(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(
            kernel32, [provenance._WindowsKillJob.ERROR_NO_MORE_FILES]
        )
        observed_sizes: list[int] = []

        def unrelated_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 999  # type: ignore[attr-defined]
            pointer._obj.size = job._thread_entry_owner_end  # type: ignore[attr-defined]
            return 1

        def no_next_thread(_snapshot: int, pointer: object) -> int:
            observed_sizes.append(pointer._obj.size)  # type: ignore[attr-defined]
            return 0

        kernel32.Thread32First.side_effect = unrelated_thread
        kernel32.Thread32Next.side_effect = no_next_thread

        with self.assertRaisesRegex(OSError, "has no primary thread"):
            job.resume(self.windows_process())

        self.assertEqual(observed_sizes, [ctypes.sizeof(job._thread_entry_type)])
        job.close()

    def test_windows_job_traverses_to_primary_thread(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def unrelated_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 999  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 707  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = unrelated_thread
        kernel32.Thread32Next.side_effect = primary_thread

        job.resume(self.windows_process(pid=404))

        kernel32.OpenThread.assert_called_once_with(0x0002, False, 707)
        job.close()

    def test_windows_job_open_thread_failure_is_named(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread
        kernel32.OpenThread.return_value = 0

        with self.assertRaisesRegex(OSError, "OpenThread failed"):
            job.resume(self.windows_process(pid=404))

        job.close()

    def test_windows_job_resume_failure_closes_thread_and_snapshot(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread
        kernel32.ResumeThread.return_value = 0xFFFFFFFF

        with self.assertRaisesRegex(OSError, "ResumeThread failed"):
            job.resume(self.windows_process(pid=404))

        self.assertEqual(
            kernel32.CloseHandle.call_args_list,
            [mock.call(303), mock.call(202)],
        )
        job.close()

    def test_windows_job_requires_exact_suspended_resume_count(self) -> None:
        for previous_suspend_count in (0, 2):
            with self.subTest(previous_suspend_count=previous_suspend_count):
                kernel32 = self.fake_windows_kernel32()
                job = self.windows_job(kernel32)

                def primary_thread(_snapshot: int, pointer: object) -> int:
                    pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
                    pointer._obj.thread_id = 606  # type: ignore[attr-defined]
                    return 1

                kernel32.Thread32First.side_effect = primary_thread
                kernel32.ResumeThread.return_value = previous_suspend_count

                with self.assertRaisesRegex(
                    OSError, f"previous suspend count {previous_suspend_count}"
                ):
                    job.resume(self.windows_process(pid=404))

                job.close()

    def test_windows_job_exact_resume_success_closes_both_handles(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread

        job.resume(self.windows_process(pid=404))

        self.assertEqual(
            kernel32.CloseHandle.call_args_list,
            [mock.call(303), mock.call(202)],
        )
        job.close()

    def test_windows_job_thread_close_failure_still_closes_snapshot(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread
        kernel32.CloseHandle.side_effect = lambda handle: int(handle != 303)

        with self.assertRaisesRegex(OSError, "CloseHandle\\(thread\\) failed"):
            job.resume(self.windows_process(pid=404))

        self.assertEqual(
            kernel32.CloseHandle.call_args_list,
            [mock.call(303), mock.call(202)],
        )
        job.close()

    def test_windows_job_resume_preserves_thread_close_failure(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread
        kernel32.ResumeThread.return_value = 0xFFFFFFFF
        kernel32.CloseHandle.side_effect = lambda handle: int(handle != 303)

        with self.assertRaisesRegex(
            OSError, "ResumeThread failed.*cleanup also failed.*CloseHandle\\(thread\\)"
        ) as raised:
            job.resume(self.windows_process(pid=404))

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("thread.resume", "ResumeThread", 5),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [("thread.close", "CloseHandle", 5)],
        )
        job.close()

    def test_windows_job_resume_preserves_snapshot_close_failure(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)

        def primary_thread(_snapshot: int, pointer: object) -> int:
            pointer._obj.owner_process_id = 404  # type: ignore[attr-defined]
            pointer._obj.thread_id = 606  # type: ignore[attr-defined]
            return 1

        kernel32.Thread32First.side_effect = primary_thread
        kernel32.ResumeThread.return_value = 0xFFFFFFFF
        kernel32.CloseHandle.side_effect = lambda handle: int(handle != 202)

        with self.assertRaisesRegex(
            OSError,
            "ResumeThread failed.*cleanup also failed.*CloseHandle\\(thread snapshot\\)",
        ) as raised:
            job.resume(self.windows_process(pid=404))

        failure = raised.exception.failure
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [("thread_snapshot.close", "CloseHandle", 5)],
        )
        job.close()

    def test_windows_job_snapshot_close_failure_is_named(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(
            kernel32, [provenance._WindowsKillJob.ERROR_NO_MORE_FILES]
        )
        kernel32.CloseHandle.side_effect = lambda handle: int(handle != 202)

        with self.assertRaisesRegex(OSError, "CloseHandle\\(thread snapshot\\) failed"):
            job.resume(self.windows_process())

        job.close()

    def test_windows_job_termination_failure_is_named(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)
        kernel32.TerminateJobObject.return_value = 0

        with self.assertRaisesRegex(OSError, "TerminateJobObject failed"):
            job.terminate()

        job.close()

    def test_windows_job_close_failure_keeps_handle_for_retry(self) -> None:
        kernel32 = self.fake_windows_kernel32()
        job = self.windows_job(kernel32)
        kernel32.CloseHandle.return_value = 0

        with self.assertRaisesRegex(OSError, "CloseHandle\\(job\\) failed"):
            job.close()

        kernel32.CloseHandle.return_value = 1
        job.close()
        job.close()
        self.assertEqual(kernel32.CloseHandle.call_count, 2)

    def test_windows_job_termination_failure_still_kills_parent(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        job = mock.Mock()
        job.terminate.side_effect = OSError("TerminateJobObject failed")
        taskkill = mock.Mock(
            return_value=subprocess.CompletedProcess(["taskkill"], 0)
        )

        with (
            mock.patch.object(provenance.os, "name", "nt"),
            mock.patch.object(provenance.subprocess, "run", taskkill),
            self.assertRaisesRegex(OSError, "TerminateJobObject failed"),
        ):
            provenance._terminate_process_tree(process, job)

        self.assertEqual(taskkill.call_args.args[0][:3], ["taskkill", "/PID", "404"])
        self.assertIn("/T", taskkill.call_args.args[0])
        self.assertIn("/F", taskkill.call_args.args[0])
        self.assertEqual(taskkill.call_args.kwargs["timeout"], 5)
        process.kill.assert_called_once_with()

    def test_windows_job_taskkill_failures_remain_observable(self) -> None:
        translated_error = OSError(13, "translated access denied")
        translated_error.winerror = 5  # type: ignore[attr-defined]
        for taskkill_failure, expected, taskkill_code in (
            (
                subprocess.CompletedProcess(["taskkill"], 7),
                "taskkill /T exited 7",
                7,
            ),
            (
                subprocess.TimeoutExpired(["taskkill"], 5),
                "taskkill /T failed",
                None,
            ),
            (OSError(2, "taskkill missing"), "taskkill /T failed", 2),
            (translated_error, "taskkill /T failed", 5),
        ):
            with self.subTest(expected=expected):
                process = self.windows_process()
                process.poll.return_value = None
                job = mock.Mock()
                job.terminate.side_effect = provenance._lifecycle_error(
                    "job.terminate",
                    "TerminateJobObject failed",
                    api="TerminateJobObject",
                    error_code=5,
                )
                if isinstance(taskkill_failure, BaseException):
                    taskkill = mock.Mock(side_effect=taskkill_failure)
                else:
                    taskkill = mock.Mock(return_value=taskkill_failure)

                with (
                    mock.patch.object(provenance.os, "name", "nt"),
                    mock.patch.object(provenance.subprocess, "run", taskkill),
                    self.assertRaisesRegex(OSError, expected) as raised,
                ):
                    provenance._terminate_process_tree(process, job)

                failure = raised.exception.failure
                self.assertEqual(
                    (failure.stage, failure.api, failure.error_code),
                    ("job.terminate", "TerminateJobObject", 5),
                )
                self.assertEqual(len(failure.cleanup_causes), 1)
                self.assertEqual(
                    (
                        failure.cleanup_causes[0].stage,
                        failure.cleanup_causes[0].api,
                        failure.cleanup_causes[0].error_code,
                    ),
                    ("process_tree.terminate", "taskkill", taskkill_code),
                )
                self.assertIn("/T", taskkill.call_args.args[0])
                self.assertIn("/F", taskkill.call_args.args[0])
                self.assertEqual(taskkill.call_args.kwargs["timeout"], 5)
                process.kill.assert_called_once_with()

    def test_windows_job_root_kill_failure_is_ordered_after_taskkill(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        process.kill.side_effect = OSError(6, "TerminateProcess failed")
        job = mock.Mock()
        job.terminate.side_effect = provenance._lifecycle_error(
            "job.terminate",
            "TerminateJobObject failed",
            api="TerminateJobObject",
            error_code=5,
        )
        taskkill = mock.Mock(
            return_value=subprocess.CompletedProcess(["taskkill"], 7)
        )

        with (
            mock.patch.object(provenance.os, "name", "nt"),
            mock.patch.object(provenance.subprocess, "run", taskkill),
            self.assertRaisesRegex(OSError, "TerminateProcess failed") as raised,
        ):
            provenance._terminate_process_tree(process, job)

        failure = raised.exception.failure
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("process_tree.terminate", "taskkill", 7),
                ("process.terminate", "Popen.kill", 6),
            ],
        )

    def test_process_tree_poll_failure_is_structured_before_root_kill(self) -> None:
        process = self.windows_process()
        process.poll.side_effect = OSError(6, "injected poll failure")
        job = mock.Mock()

        with (
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(OSError, "injected poll failure") as raised,
        ):
            provenance._terminate_process_tree(process, job)

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process.poll", "Popen.poll", 6),
        )
        process.kill.assert_called_once_with()

    def test_posix_process_group_lookup_miss_uses_root_fallback(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        killpg = mock.Mock(side_effect=ProcessLookupError(3, "group absent"))

        provenance._terminate_process_tree(
            process,
            None,
            platform_name="posix",
            kill_process_group=killpg,
        )

        killpg.assert_called_once_with(process.pid, provenance.POSIX_SIGKILL)
        process.kill.assert_called_once_with()

    def test_posix_process_guardian_configuration_fails_before_launch(self) -> None:
        failure = provenance._lifecycle_error(
            "containment.configure",
            "delegated cgroup unavailable",
            api="cgroup.kill",
        )
        factory = mock.Mock(side_effect=failure)

        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(provenance.subprocess, "Popen") as popen,
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "failed to configure strict process containment",
            ) as raised,
        ):
            provenance._run_json_command(
                ["fixture"],
                [],
                label="guardian setup fixture",
                posix_guardian_factory=factory,
            )

        self.assertEqual(raised.exception.lifecycle.stage, "containment.configure")
        popen.assert_not_called()

    def test_posix_process_strict_containment_rejects_unsupported_platform(
        self,
    ) -> None:
        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(provenance.sys, "platform", "darwin"),
            mock.patch.object(provenance.subprocess, "Popen") as popen,
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "strict process containment is unavailable",
            ) as raised,
        ):
            provenance._run_json_command(
                ["fixture"], [], label="unsupported containment fixture"
            )

        self.assertEqual(raised.exception.lifecycle.stage, "containment.configure")
        popen.assert_not_called()

    def test_posix_guardian_partial_pipe_setup_closes_owned_descriptors(self) -> None:
        with (
            mock.patch.object(provenance.sys, "platform", "linux"),
            mock.patch.dict(
                provenance.os.environ,
                {provenance._PosixGuardian.CGROUP_ENV: "/delegated"},
            ),
            mock.patch.object(
                provenance.os,
                "pipe",
                side_effect=[(10, 11), OSError(24, "too many open files")],
            ),
            mock.patch.object(provenance.os, "close") as close,
            self.assertRaisesRegex(OSError, "too many open files"),
        ):
            provenance._PosixGuardian(["fixture"], 1)

        self.assertEqual(
            close.call_args_list,
            [mock.call(10), mock.call(11)],
        )

    def test_posix_guardian_inheritability_failure_closes_all_pipes(self) -> None:
        with (
            mock.patch.object(provenance.sys, "platform", "linux"),
            mock.patch.dict(
                provenance.os.environ,
                {provenance._PosixGuardian.CGROUP_ENV: "/delegated"},
            ),
            mock.patch.object(
                provenance.os,
                "pipe",
                side_effect=[(10, 11), (12, 13)],
            ),
            mock.patch.object(
                provenance.os,
                "set_inheritable",
                side_effect=[None, None, OSError(5, "injected inherit failure")],
            ),
            mock.patch.object(provenance.os, "close") as close,
            self.assertRaisesRegex(OSError, "injected inherit failure"),
        ):
            provenance._PosixGuardian(["fixture"], 1)

        self.assertEqual(
            close.call_args_list,
            [mock.call(10), mock.call(11), mock.call(12), mock.call(13)],
        )

    def test_posix_guardian_handoff_failure_requests_cleanup(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = 125
        fake = self.FakePosixGuardian(
            [],
            0.1,
            status={
                "cleanup_confirmed": False,
                "contract": self.FakePosixGuardian.CONTRACT,
                "cleanup_causes": [
                    {
                        "cleanup_causes": [],
                        "error": "child release failed",
                        "error_code": "CHILD_RELEASE_FAILED",
                    }
                ],
                "error": "cgroup remained populated",
                "error_code": "CGROUP_CLEANUP_TIMEOUT",
            },
        )
        fake.parent_started = mock.Mock(
            side_effect=OSError(5, "injected parent handoff failure")
        )
        close_attempt = mock.Mock()
        close_attempt.observe.return_value = None

        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "failed to complete process guardian handoff",
            ) as raised,
        ):
            provenance._run_json_command(
                ["fixture"],
                [],
                label="guardian handoff fixture",
                drain_timeout_seconds=0.1,
                posix_guardian_factory=lambda *_args, **_kwargs: fake,
                raw_close_attempt_factory=mock.Mock(return_value=close_attempt),
            )

        self.assertEqual(raised.exception.lifecycle.stage, "containment.configure")
        self.assertEqual(raised.exception.lifecycle.api, "guardian.parent_handoff")
        self.assertEqual(fake.cleanup_requests, 1)
        self.assertEqual(fake.close_calls, 1)
        process.wait.assert_called_once_with(timeout=0.2)
        containment = raised.exception.lifecycle.cleanup_causes[0]
        self.assertEqual(containment.status_code, "CGROUP_CLEANUP_TIMEOUT")
        self.assertEqual(
            containment.cleanup_causes[0].status_code, "CHILD_RELEASE_FAILED"
        )

    def test_posix_process_guardian_attests_child_returncode(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = 0
        process.stdout.read.side_effect = [b"{}\n", b""]
        process.stderr.read.return_value = b""
        created: list[AdjStdlibProvenanceTests.FakePosixGuardian] = []

        def factory(command: object, cleanup_timeout_seconds: float) -> object:
            result = self.FakePosixGuardian(command, cleanup_timeout_seconds)
            created.append(result)
            return result

        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(
                provenance.subprocess, "Popen", return_value=process
            ) as popen,
        ):
            result = provenance._run_json_command(
                ["fixture"],
                ["argument"],
                label="guardian success fixture",
                posix_guardian_factory=factory,
            )

        self.assertEqual(result, {})
        self.assertEqual(created[0].parent_started_calls, 1)
        self.assertEqual(created[0].cleanup_requests, 1)
        self.assertEqual(created[0].close_calls, 1)
        self.assertTrue(created[0].assert_positive_timeout)
        self.assertEqual(popen.call_args.args[0], ["guardian", "fixture", "argument"])

    def test_posix_process_guardian_failure_rejects_valid_json(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = 125
        process.stdout.read.side_effect = [b"{}\n", b""]
        process.stderr.read.return_value = b""
        status = {
            "cleanup_confirmed": False,
            "contract": self.FakePosixGuardian.CONTRACT,
            "cleanup_causes": [
                {
                    "cleanup_causes": [],
                    "error": "child release failed",
                    "error_code": "CHILD_RELEASE_FAILED",
                }
            ],
            "error": "cgroup remained populated",
            "error_code": "CGROUP_CLEANUP_TIMEOUT",
        }
        fake = self.FakePosixGuardian([], 1, status=status)

        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "CGROUP_CLEANUP_TIMEOUT",
            ) as raised,
        ):
            provenance._run_json_command(
                ["fixture"],
                [],
                label="guardian failure fixture",
                posix_guardian_factory=lambda *_args, **_kwargs: fake,
            )

        self.assertEqual(raised.exception.lifecycle.stage, "containment.confirm")
        self.assertEqual(raised.exception.lifecycle.api, "guardian.status")
        self.assertEqual(
            raised.exception.lifecycle.status_code,
            "CGROUP_CLEANUP_TIMEOUT",
        )
        self.assertEqual(
            raised.exception.lifecycle.cleanup_causes[0].status_code,
            "CHILD_RELEASE_FAILED",
        )

    def test_posix_process_timeout_escalates_after_guardian_request(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.2),
            subprocess.TimeoutExpired(["fixture"], 0.1),
            0,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""
        fake = self.FakePosixGuardian(
            [],
            0.1,
            status={
                "cleanup_confirmed": True,
                "contract": self.FakePosixGuardian.CONTRACT,
                "returncode": -9,
                "verifier_gone": True,
            },
        )
        terminator = mock.Mock()

        with (
            mock.patch.object(provenance.os, "name", "posix"),
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(provenance.ProvenanceError, "timed out"),
        ):
            provenance._run_json_command(
                ["fixture"],
                [],
                label="guardian timeout fixture",
                timeout_seconds=0.2,
                drain_timeout_seconds=0.1,
                posix_guardian_factory=lambda *_args, **_kwargs: fake,
                process_tree_terminator=terminator,
            )

        self.assertGreaterEqual(fake.cleanup_requests, 1)
        terminator.assert_not_called()

    def test_guardian_cgroup_events_require_one_exact_populated_field(self) -> None:
        self.assertTrue(guardian.cgroup_is_empty(b"populated 0\nfrozen 0\n"))
        self.assertFalse(guardian.cgroup_is_empty(b"populated 1\n"))
        for raw in (
            b"",
            b"populated 0\npopulated 1\n",
            b"populated maybe\n",
            b"populated 0 extra\n",
            b"populated \xff\n",
        ):
            with self.subTest(raw=raw), self.assertRaises(guardian.GuardianError):
                guardian.cgroup_is_empty(raw)

    def test_guardian_error_status_preserves_cleanup_causality(self) -> None:
        primary = guardian.GuardianError(
            "CHILD_RELEASE_FAILED", "child release failed"
        )
        cleanup = guardian.GuardianError(
            "CGROUP_CLEANUP_TIMEOUT", "cgroup remained populated"
        ).with_cleanup(primary)

        self.assertEqual(
            cleanup.to_dict(),
            {
                "cleanup_causes": [
                    {
                        "cleanup_causes": [],
                        "error": "child release failed",
                        "error_code": "CHILD_RELEASE_FAILED",
                    }
                ],
                "error": "cgroup remained populated",
                "error_code": "CGROUP_CLEANUP_TIMEOUT",
            },
        )

    def test_guardian_main_serializes_compound_cleanup_failure(self) -> None:
        control_read, control_write = os.pipe()
        status_read, status_write = os.pipe()
        primary = guardian.GuardianError(
            "CHILD_RELEASE_FAILED", "child release failed"
        )
        cleanup = guardian.GuardianError(
            "CGROUP_CLEANUP_TIMEOUT", "cgroup remained populated"
        ).with_cleanup(primary)
        try:
            with mock.patch.object(guardian, "supervise", side_effect=cleanup):
                returncode = guardian.main(
                    [
                        "--control-fd",
                        str(control_read),
                        "--status-fd",
                        str(status_write),
                        "--cgroup-root",
                        "/delegated",
                        "--cleanup-timeout-seconds",
                        "1",
                        "--",
                        "fixture",
                    ]
                )
            control_read = -1
            status_write = -1
            raw = os.read(status_read, guardian.MAX_CONTROL_BYTES + 1)
            status = json.loads(raw.decode("utf-8"))
            self.assertEqual(returncode, 125)
            self.assertEqual(status["error_code"], "CGROUP_CLEANUP_TIMEOUT")
            self.assertEqual(
                status["cleanup_causes"][0]["error_code"],
                "CHILD_RELEASE_FAILED",
            )
        finally:
            for descriptor in (
                control_read,
                control_write,
                status_read,
                status_write,
            ):
                if descriptor >= 0:
                    os.close(descriptor)

    def test_guardian_reaping_respects_cleanup_deadline(self) -> None:
        with (
            mock.patch.object(guardian.time, "monotonic", return_value=1.0),
            mock.patch.object(guardian.os, "waitpid") as waitpid,
        ):
            self.assertFalse(guardian._reap_children(1.0))

        waitpid.assert_not_called()

    def test_guardian_status_bytes_round_trip_through_verifier_contract(self) -> None:
        status = {
            "cleanup_confirmed": True,
            "contract": provenance._PosixGuardian.CONTRACT,
            "returncode": -9,
            "verifier_gone": True,
        }
        control_read, control_write = os.pipe()
        status_read, status_write = os.pipe()
        verifier = provenance._PosixGuardian.__new__(provenance._PosixGuardian)
        verifier._control_read = -1
        verifier._control_write = control_write
        verifier._status_read = status_read
        verifier._status_write = -1
        verifier._cleanup_requested = False

        selector = mock.Mock()
        selector.select.return_value = [(mock.Mock(), provenance.selectors.EVENT_READ)]
        try:
            raw = guardian.canonical_json_bytes(status)
            self.assertEqual(raw, provenance.canonical_json_bytes(status))
            os.write(status_write, raw)
            os.close(status_write)
            status_write = -1
            with (
                mock.patch.object(provenance.os, "set_blocking", create=True),
                mock.patch.object(
                    provenance.selectors, "DefaultSelector", return_value=selector
                ),
            ):
                self.assertEqual(verifier.read_status(1), status)
            self.assertEqual(os.read(control_read, 1), b"")
        finally:
            verifier.close()
            for descriptor in (control_read, status_write):
                if descriptor >= 0:
                    os.close(descriptor)

    def test_guardian_cleanup_requires_empty_cgroup_before_removal(self) -> None:
        cgroup = guardian.CgroupHandle(root_fd=10, child_fd=11, name="known")
        with (
            mock.patch.object(guardian, "_write_small_at") as write,
            mock.patch.object(
                guardian, "_read_small_at", return_value=b"populated 0\n"
            ) as read,
            mock.patch.object(guardian.os, "killpg", create=True) as kill_group,
            mock.patch.object(guardian.os, "close") as close,
            mock.patch.object(guardian.os, "rmdir") as remove,
        ):
            guardian.cleanup_command_cgroup(
                cgroup, time.monotonic() + 1, process_group=123
            )

        write.assert_called_once_with(11, "cgroup.kill", b"1\n")
        kill_group.assert_called_once_with(123, guardian.POSIX_SIGKILL)
        read.assert_called_once_with(11, "cgroup.events")
        close.assert_called_once_with(11)
        remove.assert_called_once_with("known", dir_fd=10)

    def test_guardian_cgroup_open_failure_preserves_rollback_failure(self) -> None:
        with (
            mock.patch.object(guardian, "validate_cgroup2_descriptor"),
            mock.patch.object(guardian.os, "mkdir"),
            mock.patch.object(
                guardian, "_open_at", side_effect=OSError(5, "open failed")
            ),
            mock.patch.object(
                guardian.os, "rmdir", side_effect=OSError(16, "remove failed")
            ),
            self.assertRaisesRegex(
                guardian.GuardianError, "fresh command cgroup could not be created"
            ) as raised,
        ):
            guardian.create_command_cgroup(10)

        self.assertEqual(raised.exception.code, "CGROUP_CREATE_FAILED")
        self.assertEqual(
            [cause.code for cause in raised.exception.cleanup_causes],
            ["CGROUP_ROLLBACK_REMOVE_FAILED"],
        )

    def test_guardian_cgroup_validation_preserves_all_rollback_failures(self) -> None:
        validation_error = guardian.GuardianError(
            "CONTROL_OPEN_FAILED", "control open failed"
        )
        with (
            mock.patch.object(guardian, "validate_cgroup2_descriptor"),
            mock.patch.object(guardian.os, "mkdir"),
            mock.patch.object(guardian, "_open_at", side_effect=[11, validation_error]),
            mock.patch.object(
                guardian.os, "close", side_effect=OSError(5, "close failed")
            ),
            mock.patch.object(
                guardian.os, "rmdir", side_effect=OSError(16, "remove failed")
            ),
            self.assertRaisesRegex(
                guardian.GuardianError, "control open failed"
            ) as raised,
        ):
            guardian.create_command_cgroup(10)

        self.assertEqual(raised.exception.code, "CONTROL_OPEN_FAILED")
        self.assertEqual(
            [cause.code for cause in raised.exception.cleanup_causes],
            ["CGROUP_ROLLBACK_CLOSE_FAILED", "CGROUP_ROLLBACK_REMOVE_FAILED"],
        )

    def test_guardian_cleanup_failure_never_removes_unproven_cgroup(self) -> None:
        cgroup = guardian.CgroupHandle(root_fd=10, child_fd=11, name="known")
        with (
            mock.patch.object(
                guardian,
                "_write_small_at",
                side_effect=guardian.GuardianError("KILL_FAILED", "kill failed"),
            ),
            mock.patch.object(
                guardian, "_read_small_at", return_value=b"populated 1\n"
            ),
            mock.patch.object(
                guardian.time,
                "monotonic",
                side_effect=[0.0, 0.0, 1.0],
            ),
            mock.patch.object(guardian.time, "sleep"),
            mock.patch.object(guardian.os, "close"),
            mock.patch.object(guardian.os, "rmdir") as remove,
            self.assertRaisesRegex(
                guardian.GuardianError, "kill failed"
            ) as raised,
        ):
            guardian.cleanup_command_cgroup(cgroup, 0.5)

        remove.assert_not_called()
        self.assertEqual(raised.exception.code, "KILL_FAILED")
        self.assertEqual(
            [cause.code for cause in raised.exception.cleanup_causes],
            ["CGROUP_CLEANUP_TIMEOUT"],
        )

    def test_guardian_reap_timeout_appends_to_cleanup_failure(self) -> None:
        cgroup = guardian.CgroupHandle(root_fd=10, child_fd=11, name="known")
        cleanup_error = guardian.GuardianError(
            "CGROUP_CLEANUP_TIMEOUT", "cgroup remained populated"
        )
        with (
            mock.patch.object(guardian.sys, "platform", "linux"),
            mock.patch.object(guardian.os, "O_CLOEXEC", 0, create=True),
            mock.patch.object(guardian.os, "WNOHANG", 1, create=True),
            mock.patch.object(guardian.os, "open", return_value=10),
            mock.patch.object(guardian, "_enable_subreaper"),
            mock.patch.object(
                guardian, "create_command_cgroup", return_value=cgroup
            ),
            mock.patch.object(
                guardian.os, "pipe2", return_value=(20, 21), create=True
            ),
            mock.patch.object(guardian.os, "fork", return_value=321, create=True),
            mock.patch.object(guardian.os, "close"),
            mock.patch.object(guardian, "_write_small_at"),
            mock.patch.object(guardian.os, "write", return_value=1),
            mock.patch.object(guardian, "_monitor", return_value=(0, False)),
            mock.patch.object(
                guardian, "cleanup_command_cgroup", side_effect=cleanup_error
            ),
            mock.patch.object(
                guardian.os, "waitpid", side_effect=ChildProcessError
            ),
            mock.patch.object(guardian, "_reap_children", return_value=False),
            self.assertRaisesRegex(
                guardian.GuardianError, "cgroup remained populated"
            ) as raised,
        ):
            guardian.supervise(
                ["fixture"],
                control_fd=5,
                cgroup_root="/delegated",
                cleanup_timeout_seconds=1,
            )

        self.assertEqual(raised.exception.code, "CGROUP_CLEANUP_TIMEOUT")
        self.assertEqual(
            [cause.code for cause in raised.exception.cleanup_causes],
            ["ADOPTED_REAP_TIMEOUT"],
        )

    def test_guardian_assigns_cgroup_before_releasing_child(self) -> None:
        cgroup = guardian.CgroupHandle(root_fd=10, child_fd=11, name="known")
        events: list[str] = []

        def write_control(root_fd: int, name: str, value: bytes) -> None:
            self.assertEqual((root_fd, name, value), (11, "cgroup.procs", b"321\n"))
            events.append("assign")

        def release(descriptor: int, value: bytes) -> int:
            self.assertEqual((descriptor, value), (21, b"1"))
            events.append("release")
            return 1

        with (
            mock.patch.object(guardian.sys, "platform", "linux"),
            mock.patch.object(guardian.os, "O_CLOEXEC", 0, create=True),
            mock.patch.object(guardian.os, "WNOHANG", 1, create=True),
            mock.patch.object(guardian.os, "open", return_value=10),
            mock.patch.object(guardian, "_enable_subreaper"),
            mock.patch.object(
                guardian, "create_command_cgroup", return_value=cgroup
            ),
            mock.patch.object(
                guardian.os, "pipe2", return_value=(20, 21), create=True
            ),
            mock.patch.object(guardian.os, "fork", return_value=321, create=True),
            mock.patch.object(guardian.os, "close"),
            mock.patch.object(guardian, "_write_small_at", side_effect=write_control),
            mock.patch.object(guardian.os, "write", side_effect=release),
            mock.patch.object(guardian, "_monitor", return_value=(0, False)),
            mock.patch.object(guardian, "cleanup_command_cgroup") as cleanup,
            mock.patch.object(
                guardian.os, "waitpid", side_effect=ChildProcessError
            ),
            mock.patch.object(guardian, "_reap_children") as reap,
        ):
            status = guardian.supervise(
                ["fixture"],
                control_fd=5,
                cgroup_root="/delegated",
                cleanup_timeout_seconds=1,
            )

        self.assertEqual(events, ["assign", "release"])
        self.assertEqual(status["returncode"], 0)
        self.assertTrue(status["cleanup_confirmed"])
        cleanup.assert_called_once()
        reap.assert_called_once()

    def test_guardian_cgroup_setup_failure_precedes_fork(self) -> None:
        setup_error = guardian.GuardianError(
            "CGROUP_DELEGATION_INVALID", "delegation invalid"
        )
        with (
            mock.patch.object(guardian.sys, "platform", "linux"),
            mock.patch.object(guardian.os, "O_CLOEXEC", 0, create=True),
            mock.patch.object(guardian.os, "open", return_value=10),
            mock.patch.object(guardian, "_enable_subreaper"),
            mock.patch.object(
                guardian, "create_command_cgroup", side_effect=setup_error
            ),
            mock.patch.object(guardian.os, "fork", create=True) as fork,
            mock.patch.object(guardian.os, "close"),
            mock.patch.object(guardian, "_reap_children"),
            self.assertRaisesRegex(guardian.GuardianError, "delegation invalid"),
        ):
            guardian.supervise(
                ["fixture"],
                control_fd=5,
                cgroup_root="/delegated",
                cleanup_timeout_seconds=1,
            )

        fork.assert_not_called()

    def test_posix_process_group_permission_failure_is_structured(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        killpg = mock.Mock(side_effect=PermissionError(13, "permission denied"))

        with self.assertRaisesRegex(OSError, "permission denied") as raised:
            provenance._terminate_process_tree(
                process,
                None,
                platform_name="posix",
                kill_process_group=killpg,
            )

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process_tree.terminate", "os.killpg", 13),
        )
        process.kill.assert_called_once_with()

    def test_posix_process_group_fallback_failures_are_ordered(self) -> None:
        process = self.windows_process()
        process.poll.side_effect = OSError(6, "poll failed")
        process.kill.side_effect = OSError(1, "root kill failed")
        killpg = mock.Mock(side_effect=PermissionError(13, "killpg failed"))

        with self.assertRaisesRegex(OSError, "root kill failed") as raised:
            provenance._terminate_process_tree(
                process,
                None,
                platform_name="posix",
                kill_process_group=killpg,
            )

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process_tree.terminate", "os.killpg", 13),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("process.poll", "Popen.poll", 6),
                ("process.terminate", "Popen.kill", 1),
            ],
        )

    def test_posix_process_root_fallback_failure_is_structured(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        process.kill.side_effect = OSError(1, "root kill failed")
        killpg = mock.Mock()

        with self.assertRaisesRegex(OSError, "root kill failed") as raised:
            provenance._terminate_process_tree(
                process,
                None,
                platform_name="posix",
                kill_process_group=killpg,
            )

        failure = raised.exception.failure
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process.terminate", "Popen.kill", 1),
        )

    def test_posix_process_root_lookup_miss_is_benign(self) -> None:
        process = self.windows_process()
        process.poll.return_value = None
        process.kill.side_effect = ProcessLookupError(3, "root absent")

        provenance._terminate_process_tree(
            process,
            None,
            platform_name="posix",
            kill_process_group=mock.Mock(),
        )

        process.kill.assert_called_once_with()

    @mock.patch.object(provenance.os, "name", "nt")
    def test_posix_process_repeated_termination_failures_are_ordered(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.2),
            subprocess.TimeoutExpired(["fixture"], 0.1),
            -9,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""
        terminator = mock.Mock(
            side_effect=[
                provenance._lifecycle_error(
                    "process_tree.terminate",
                    "killpg failed",
                    api="os.killpg",
                    error_code=13,
                ),
                provenance._lifecycle_error(
                    "process.terminate",
                    "root kill failed",
                    api="Popen.kill",
                    error_code=1,
                ),
            ]
        )

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(provenance.ProvenanceError, "timed out") as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="repeated termination fixture",
                timeout_seconds=0.2,
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=terminator,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("process_tree.terminate", "os.killpg", 13),
                ("process.wait", "Popen.wait", None),
                ("process.terminate", "Popen.kill", 1),
            ],
        )
        self.assertEqual(process.wait.call_count, 3)

    def test_posix_process_unreaped_timeout_does_not_claim_exit(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.2),
            subprocess.TimeoutExpired(["fixture"], 0.1),
            subprocess.TimeoutExpired(["fixture"], 0.1),
        ]
        process.poll.return_value = None
        process.stdout.read.side_effect = OSError(5, "stdout read failed")
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(provenance.ProvenanceError, "timed out") as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="unreaped timeout fixture",
                timeout_seconds=0.2,
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=mock.Mock(),
            )

        failure = raised.exception.lifecycle
        self.assertNotIn(
            "command.exit",
            [cause.stage for cause in failure.cleanup_causes],
        )
        self.assertFalse(any("exited None" in cause.message for cause in failure.cleanup_causes))
        self.assertEqual(process.wait.call_count, 3)

    @mock.patch.object(provenance.os, "name", "nt")
    def test_posix_process_failure_preserves_both_pipe_close_causes(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.1),
            OSError(6, "recovery wait failed"),
            -9,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""
        process.stdout.close.side_effect = OSError(5, "stdout close failed")
        process.stderr.close.side_effect = OSError(6, "stderr close failed")
        terminator = mock.Mock(
            side_effect=[
                provenance._lifecycle_error(
                    "process_tree.terminate",
                    "killpg failed",
                    api="os.killpg",
                    error_code=13,
                ),
                None,
            ]
        )

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "timed out.*killpg failed.*stdout close failed.*stderr close failed",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="POSIX compound cleanup fixture",
                timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=terminator,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "command.timeout")
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("process_tree.terminate", "os.killpg", 13),
                ("process.wait", "Popen.wait", 6),
                ("pipe.close", "Popen.stdout.close", 5),
                ("pipe.close", "Popen.stderr.close", 6),
            ],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_factory_failure_precedes_process_launch(self) -> None:
        factory = mock.Mock(side_effect=OSError("injected CreateJobObjectW failure"))

        with (
            mock.patch.object(provenance.subprocess, "Popen") as popen,
            self.assertRaisesRegex(
                provenance.ProvenanceError, "failed to create process job"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable, "-c", "print('{}')"],
                [],
                label="job factory fixture",
                windows_job_factory=factory,
            )

        self.assertEqual(raised.exception.lifecycle.stage, "job.create")
        popen.assert_not_called()

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_popen_failure_preserves_close_failure(self) -> None:
        job = mock.Mock()
        job.close.side_effect = OSError("injected CloseHandle(job) failure")

        with (
            mock.patch.object(
                provenance.subprocess,
                "Popen",
                side_effect=OSError("injected process launch failure"),
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "failed to run.*process launch failure.*failed to close process job",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="launch cleanup fixture",
                windows_job_factory=lambda: job,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api),
            ("process.launch", "Popen"),
        )
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(len(failure.cleanup_causes), 1)
        self.assertEqual(failure.cleanup_causes[0].stage, "job.close")
        self.assertEqual(len(failure.cleanup_causes[0].cleanup_causes), 1)
        self.assertEqual(job.close.call_count, 2)

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_containment_failure_cleans_process_and_pipes(self) -> None:
        original_popen = subprocess.Popen
        for failing_stage in ("assign", "resume"):
            with self.subTest(failing_stage=failing_stage):
                processes: list[subprocess.Popen[bytes]] = []
                job = mock.Mock()
                getattr(job, failing_stage).side_effect = OSError(
                    f"injected {failing_stage} failure"
                )
                job.close.side_effect = [
                    OSError("injected transient CloseHandle(job) failure"),
                    None,
                ]

                def capture_process(
                    *args: object,
                    _processes: list[subprocess.Popen[bytes]] = processes,
                    **kwargs: object,
                ) -> object:
                    process = original_popen(*args, **kwargs)
                    _processes.append(process)
                    return process

                with (
                    mock.patch.object(
                        provenance.subprocess, "Popen", capture_process
                    ),
                    self.assertRaisesRegex(
                        provenance.ProvenanceError,
                        f"failed to contain process tree.*{failing_stage} failure",
                    ) as raised,
                ):
                    provenance._run_json_command(
                        [sys.executable, "-c", "print('{}')"],
                        [],
                        label="containment cleanup fixture",
                        drain_timeout_seconds=0.2,
                        windows_job_factory=lambda job=job: job,
                    )

                failure = raised.exception.lifecycle
                self.assertEqual(
                    failure.stage,
                    "job.assign" if failing_stage == "assign" else "thread.resume",
                )
                self.assertEqual(failure.message, str(raised.exception))
                self.assertEqual(
                    [cause.stage for cause in failure.cleanup_causes],
                    ["job.close"],
                )
                self.assertEqual(len(processes), 1)
                self.assertIsNotNone(processes[0].poll())
                self.assertTrue(processes[0].stdout.closed)
                self.assertTrue(processes[0].stderr.closed)
                job.terminate.assert_called_once_with()
                self.assertEqual(job.close.call_count, 2)
                if failing_stage == "assign":
                    job.resume.assert_not_called()
                else:
                    job.assign.assert_called_once_with(processes[0])

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_containment_preserves_final_kill_failure(self) -> None:
        process = mock.Mock()
        process.stdout = mock.Mock()
        process.stderr = mock.Mock()
        process.wait.side_effect = subprocess.TimeoutExpired(["fixture"], 0.1)
        process.kill.side_effect = OSError("injected TerminateProcess failure")
        job = mock.Mock()
        job.assign.side_effect = OSError("injected assignment failure")

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(
                provenance,
                "_terminate_process_tree",
                side_effect=OSError("injected tree termination failure"),
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "assignment failure.*tree termination failure.*root process kill failed.*TerminateProcess failure",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="final kill fixture",
                drain_timeout_seconds=0.1,
                windows_job_factory=lambda: job,
            )

        failure = raised.exception.lifecycle
        self.assertIsNotNone(failure)
        self.assertEqual(failure.stage, "job.assign")
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["process_tree.terminate", "process.terminate"],
        )
        job.close.assert_called_once_with()
        process.stdout.close.assert_called_once_with()
        process.stderr.close.assert_called_once_with()

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_timeout_preserves_termination_and_close_failures(self) -> None:
        job = mock.Mock()
        job.terminate.side_effect = OSError("injected TerminateJobObject failure")
        job.close.side_effect = OSError("injected CloseHandle(job) failure")

        with self.assertRaisesRegex(
            provenance.ProvenanceError,
            "timed out.*failed to terminate process tree.*failed to close process job",
        ) as raised:
            provenance._run_json_command(
                [sys.executable, "-c", "print('{}')"],
                [],
                label="timeout cleanup fixture",
                timeout_seconds=0.1,
                drain_timeout_seconds=0.2,
                windows_job_factory=lambda: job,
            )

        failure = raised.exception.lifecycle
        self.assertIsNotNone(failure)
        self.assertEqual(failure.stage, "command.timeout")
        self.assertEqual(failure.api, "Popen.wait")
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["job.terminate", "job.close"],
        )
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes[1].cleanup_causes],
            ["job.terminate", "job.close"],
        )
        self.assertGreaterEqual(job.terminate.call_count, 1)
        self.assertEqual(job.close.call_count, 2)

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_overflow_preserves_termination_and_close_failures(self) -> None:
        class FaultingJob:
            def __init__(self) -> None:
                self.inner = provenance._WindowsKillJob()

            def assign(self, process: subprocess.Popen[bytes]) -> None:
                self.inner.assign(process)

            def resume(self, process: subprocess.Popen[bytes]) -> None:
                self.inner.resume(process)

            def terminate(self) -> None:
                self.inner.terminate()
                raise OSError("injected TerminateJobObject failure")

            def close(self) -> None:
                self.inner.close()
                raise OSError("injected CloseHandle(job) failure")

        with (
            mock.patch.object(provenance, "MAX_OBJECT_BYTES", 1024),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "output exceeds byte limit.*failed to terminate process tree.*failed to close process job",
            ) as raised,
        ):
            provenance._run_json_command(
                [
                    sys.executable,
                    "-c",
                    "import sys; sys.stdout.buffer.write(b'x' * 2048)",
                ],
                [],
                label="overflow cleanup fixture",
                timeout_seconds=5,
                drain_timeout_seconds=0.2,
                windows_job_factory=FaultingJob,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "command.output_limit")
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["job.terminate", "job.close"],
        )
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes[1].cleanup_causes],
            ["job.terminate", "job.close"],
        )

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_timeout_preserves_stuck_drain_failure(self) -> None:
        process = mock.Mock()
        process.stdout = mock.Mock()
        process.stderr = mock.Mock()
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.1),
            -9,
        ]
        threads = [mock.Mock(), mock.Mock()]
        for thread in threads:
            thread.is_alive.return_value = True
        raw_attempt = mock.Mock()
        raw_attempt.observe.return_value = None

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(
                provenance.threading, "Thread", side_effect=threads
            ),
            mock.patch.object(provenance, "_terminate_process_tree"),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "timed out after 0.1 seconds.*output pipes did not close within bounds",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="stuck drain fixture",
                timeout_seconds=0.1,
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                raw_close_attempt_factory=mock.Mock(return_value=raw_attempt),
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "command.timeout")
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["pipe.drain"],
        )
        process.stdout.close.assert_not_called()
        process.stderr.close.assert_not_called()

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_primary_stuck_drain_is_not_self_causal(self) -> None:
        process = mock.Mock(pid=404)
        process.stdout = mock.Mock()
        process.stderr = mock.Mock()
        process.wait.return_value = 0
        threads = [mock.Mock(), mock.Mock()]
        for thread in threads:
            thread.is_alive.return_value = True
        raw_attempt = mock.Mock()
        raw_attempt.observe.return_value = None

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(
                provenance.threading, "Thread", side_effect=threads
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "output pipes did not close within bounds",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="primary stuck drain fixture",
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=mock.Mock(),
                raw_close_attempt_factory=mock.Mock(return_value=raw_attempt),
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "pipe.drain")
        self.assertEqual(failure.cleanup_causes, ())
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_stuck_stdout_raw_close_does_not_skip_healthy_stderr(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = 0
        stdout_thread = mock.Mock()
        stdout_thread.is_alive.return_value = True
        stderr_thread = mock.Mock()
        stderr_thread.is_alive.return_value = False
        raw_failure = provenance.LifecycleFailure(
            stage="pipe.close",
            api="Popen.stdout.raw.close",
            error_code=None,
            message="stdout raw pipe close timed out after 0.1 seconds",
        )
        raw_attempt = mock.Mock()
        raw_attempt.observe.return_value = raw_failure
        raw_factory = mock.Mock(return_value=raw_attempt)

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(
                provenance.threading,
                "Thread",
                side_effect=[stdout_thread, stderr_thread],
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "output pipes did not close within bounds.*raw pipe close timed out",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="one stuck drain fixture",
                drain_timeout_seconds=0.1,
                raw_close_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=mock.Mock(),
                raw_close_attempt_factory=raw_factory,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "pipe.drain")
        self.assertEqual(
            [(cause.stage, cause.api) for cause in failure.cleanup_causes],
            [("pipe.close", "Popen.stdout.raw.close")],
        )
        self.assertEqual(raw_factory.call_args.args[0].name, "stdout")
        process.stdout.close.assert_not_called()
        process.stderr.close.assert_called_once_with()

    def test_raw_pipe_close_attempt_has_independent_observation_bound(self) -> None:
        release = threading.Event()
        raw = mock.Mock()

        def blocked_close() -> None:
            release.wait(5)
            raise OSError(5, "late close failure")

        raw.close.side_effect = blocked_close
        stream = mock.Mock(raw=raw)
        endpoint = provenance._DrainEndpoint(
            "stdout", stream, bytearray(), 1, 0
        )
        attempt = provenance._RawPipeCloseAttempt(endpoint)
        started = time.monotonic()
        failure = attempt.observe(0.05)

        self.assertLess(time.monotonic() - started, 0.5)
        self.assertIsNotNone(failure)
        assert failure is not None
        self.assertEqual(failure.stage, "pipe.close")
        self.assertIn("timed out", failure.message)
        release.set()
        self.assertTrue(attempt._done.wait(1))
        self.assertIn("timed out", failure.message)

    @mock.patch.object(provenance.os, "name", "nt")
    def test_raw_pipe_close_start_failure_is_ordered_after_drain(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = 0
        threads = [mock.Mock(), mock.Mock()]
        for thread in threads:
            thread.is_alive.return_value = True

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.threading, "Thread", side_effect=threads),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "output pipes did not close within bounds.*could not start",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="raw close start fixture",
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
                process_tree_terminator=mock.Mock(),
                raw_close_attempt_factory=mock.Mock(
                    side_effect=OSError(5, "injected raw close start failure")
                ),
            )

        self.assertEqual(raised.exception.lifecycle.stage, "pipe.drain")
        self.assertEqual(
            [cause.stage for cause in raised.exception.lifecycle.cleanup_causes],
            ["pipe.close", "pipe.close"],
        )

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_pipe_read_failure_fails_closed(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = -9
        process.poll.return_value = None
        process.stdout.read.side_effect = [
            b"{}\n",
            OSError(5, "injected stdout read failure"),
        ]
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "stdout pipe read failed"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="pipe read fixture",
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("pipe.read", "Popen.stdout.read", 5),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [("command.exit", "Popen.wait", -9)],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_child_exit_precedes_later_pipe_read_failure(self) -> None:
        process = mock.Mock()
        process.wait.return_value = 7
        process.poll.return_value = 7
        process.stdout.read.side_effect = OSError(5, "injected stdout read failure")
        process.stderr.read.return_value = b""
        process.stdout.close.side_effect = OSError(6, "injected stdout close failure")

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "exited 7.*stdout pipe read failed"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="prior exit fixture",
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("command.exit", "Popen.wait", 7),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("pipe.read", "Popen.stdout.read", 5),
                ("pipe.close", "Popen.stdout.close", 6),
            ],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_reader_primary_follows_event_order(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = -9
        process.poll.side_effect = [None, -9]
        stderr_failed = threading.Event()

        def stdout_read(_size: int) -> bytes:
            stderr_failed.wait(1)
            raise OSError(5, "stdout read failed")

        def stderr_read(_size: int) -> bytes:
            stderr_failed.set()
            raise OSError(6, "stderr read failed")

        process.stdout.read.side_effect = stdout_read
        process.stderr.read.side_effect = stderr_read

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "stderr pipe read failed"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="reader order fixture",
                windows_job_factory=mock.Mock,
                process_tree_terminator=mock.Mock(),
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("pipe.read", "Popen.stderr.read", 6),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("pipe.read", "Popen.stdout.read", 5),
                ("command.exit", "Popen.wait", -9),
            ],
        )

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_pipe_read_preserves_poll_failure(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.return_value = -9
        process.poll.side_effect = [OSError(6, "injected poll failure"), None]
        process.stdout.read.side_effect = OSError(5, "injected stdout read failure")
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "stdout pipe read failed.*process exited -9",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="poll failure fixture",
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "pipe.read")
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [
                ("process.poll", "Popen.poll", 6),
                ("command.exit", "Popen.wait", -9),
            ],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_wait_failure_and_retry_are_structured(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            OSError(5, "injected wait failure"),
            OSError(6, "injected wait retry failure"),
            -9,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "process wait failed"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="wait fixture",
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process.wait", "Popen.wait", 5),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code)
                for cause in failure.cleanup_causes
            ],
            [("process.wait", "Popen.wait", 6)],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_wait_failure_preserves_retry_timeout(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            OSError(5, "injected wait failure"),
            subprocess.TimeoutExpired(["fixture"], 0.1),
            -9,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "process wait failed"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="wait timeout fixture",
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("process.wait", "Popen.wait", 5),
        )
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code, cause.message)
                for cause in failure.cleanup_causes
            ],
            [
                (
                    "process.wait",
                    "Popen.wait",
                    None,
                    "process wait retry timed out after 0.1 seconds",
                )
            ],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_timeout_preserves_recovery_wait_timeout(self) -> None:
        process = mock.Mock(pid=404)
        process.wait.side_effect = [
            subprocess.TimeoutExpired(["fixture"], 0.2),
            subprocess.TimeoutExpired(["fixture"], 0.1),
            -9,
        ]
        process.stdout.read.return_value = b""
        process.stderr.read.return_value = b""

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            mock.patch.object(provenance.os, "killpg", create=True),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "timed out after 0.2 seconds"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="double timeout fixture",
                timeout_seconds=0.2,
                drain_timeout_seconds=0.1,
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "command.timeout")
        self.assertEqual(
            [
                (cause.stage, cause.api, cause.error_code, cause.message)
                for cause in failure.cleanup_causes
            ],
            [
                (
                    "process.wait",
                    "Popen.wait",
                    None,
                    "process wait after command timeout timed out after 0.1 seconds",
                )
            ],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @mock.patch.object(provenance.os, "name", "nt")
    def test_json_command_pipe_close_failure_is_structured(self) -> None:
        process = mock.Mock()
        process.wait.return_value = 0
        process.stdout.read.side_effect = [b"{}\n", b""]
        process.stderr.read.return_value = b""
        process.stdout.close.side_effect = OSError(5, "injected stdout close failure")

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "failed to close output pipe"
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="pipe close fixture",
                windows_job_factory=mock.Mock,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(
            (failure.stage, failure.api, failure.error_code),
            ("pipe.close", "Popen.stdout.close", 5),
        )
        self.assertEqual(failure.message, str(raised.exception))

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_job_close_root_renders_pipe_close_as_cleanup(self) -> None:
        process = mock.Mock()
        process.wait.return_value = 0
        process.stdout.read.side_effect = [b"{}\n", b""]
        process.stderr.read.return_value = b""
        process.stdout.close.side_effect = OSError(5, "injected stdout close failure")
        job = mock.Mock()
        job.close.side_effect = [OSError(6, "injected job close failure"), None]

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "failed to close process job.*cleanup also failed: stdout pipe close failed",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="combined close fixture",
                windows_job_factory=lambda: job,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "job.close")
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["pipe.close"],
        )
        self.assertNotIn("recovery termination failed", str(raised.exception))
        self.assertEqual(failure.message, str(raised.exception))

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_containment_preserves_pipe_close_failure(self) -> None:
        process = mock.Mock()
        process.wait.return_value = 0
        process.stdout.close.side_effect = OSError(5, "injected stdout close failure")
        job = mock.Mock()
        job.assign.side_effect = OSError(87, "injected assignment failure")

        with (
            mock.patch.object(provenance.subprocess, "Popen", return_value=process),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "assignment failure.*stdout pipe close failed",
            ) as raised,
        ):
            provenance._run_json_command(
                [sys.executable],
                [],
                label="containment pipe close fixture",
                windows_job_factory=lambda: job,
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "job.assign")
        self.assertEqual(
            [cause.stage for cause in failure.cleanup_causes],
            ["pipe.close"],
        )
        self.assertEqual(failure.message, str(raised.exception))

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_close_failure_terminates_and_retries(self) -> None:
        original_close = provenance._WindowsKillJob.close
        original_terminate = provenance._WindowsKillJob.terminate
        close_calls = 0
        terminate_calls = 0

        def fail_first_close(job: object) -> None:
            nonlocal close_calls
            close_calls += 1
            if close_calls == 1:
                raise OSError("injected CloseHandle(job) failure")
            original_close(job)

        def tracked_terminate(job: object) -> None:
            nonlocal terminate_calls
            terminate_calls += 1
            original_terminate(job)

        with (
            mock.patch.object(
                provenance._WindowsKillJob, "close", fail_first_close
            ),
            mock.patch.object(
                provenance._WindowsKillJob, "terminate", tracked_terminate
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError,
                "failed to close process job.*injected CloseHandle",
            ) as raised,
        ):
            provenance._run_json_command(
                [
                    sys.executable,
                    "-c",
                    "import sys; sys.stdout.buffer.write(b'{}\\n')",
                ],
                [],
                label="close retry fixture",
            )

        failure = raised.exception.lifecycle
        self.assertEqual(failure.stage, "job.close")
        self.assertEqual(failure.message, str(raised.exception))
        self.assertEqual(failure.cleanup_causes, ())
        self.assertEqual(close_calls, 2)
        self.assertEqual(terminate_calls, 1)

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_cleanup_failure_preserves_command_error(self) -> None:
        for command, expected, stage, error_code in (
            ("import sys; sys.exit(7)", "exited 7", "command.exit", 7),
            ("print('not-json')", "did not emit UTF-8 JSON", "command.parse", None),
        ):
            with self.subTest(expected=expected):
                original_close = provenance._WindowsKillJob.close
                close_calls = 0

                def fail_first_close(
                    job: object,
                    _original_close=original_close,
                ) -> None:
                    nonlocal close_calls
                    close_calls += 1
                    if close_calls == 1:
                        raise OSError("injected CloseHandle(job) failure")
                    _original_close(job)

                with (
                    mock.patch.object(
                        provenance._WindowsKillJob, "close", fail_first_close
                    ),
                    self.assertRaisesRegex(
                        provenance.ProvenanceError,
                        expected + ".*failed to close process job",
                    ) as raised,
                ):
                    provenance._run_json_command(
                        [sys.executable, "-c", command],
                        [],
                        label="causal cleanup fixture",
                    )

                failure = raised.exception.lifecycle
                self.assertIsNotNone(failure)
                self.assertEqual(
                    (failure.stage, failure.error_code),
                    (stage, error_code),
                )
                self.assertEqual(
                    [cause.stage for cause in failure.cleanup_causes],
                    ["job.close"],
                )
                self.assertEqual(close_calls, 2)

    @unittest.skipUnless(
        os.name == "nt" or sys.platform.startswith("linux"),
        "strict process containment backend",
    )
    def test_process_tree_timeout_kills_descendant_pipe_holders(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pid_path = Path(directory) / "child.pid"
            child = (
                "import os, pathlib, time; "
                f"pathlib.Path({str(pid_path)!r}).write_text(str(os.getpid())); "
                "time.sleep(30)"
            )
            parent = (
                "import pathlib, subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {child!r}]); "
                f"p = pathlib.Path({str(pid_path)!r}); "
                "deadline = time.monotonic() + 5; "
                "exec(\"while not p.exists() and time.monotonic() < deadline:\\n time.sleep(0.01)\"); "
                "time.sleep(30)"
            )
            started = time.monotonic()

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "timed out after 1 second"
            ):
                provenance._run_json_command(
                    [sys.executable, "-c", parent],
                    [],
                    label="descendant timeout fixture",
                    timeout_seconds=1,
                    drain_timeout_seconds=2,
                )

            self.assertTrue(pid_path.exists(), "descendant never reached readiness")
            self.assertLess(time.monotonic() - started, 8)
            self.assert_process_exits(int(pid_path.read_text()))

    @unittest.skipUnless(
        os.name == "nt" or sys.platform.startswith("linux"),
        "strict process containment backend",
    )
    def test_process_tree_parent_exit_kills_descendant_pipe_holders(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            pid_path = Path(directory) / "child.pid"
            child = (
                "import os, pathlib, time; "
                f"pathlib.Path({str(pid_path)!r}).write_text(str(os.getpid())); "
                "time.sleep(30)"
            )
            parent = (
                "import pathlib, subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {child!r}]); "
                f"p = pathlib.Path({str(pid_path)!r}); "
                "deadline = time.monotonic() + 5; "
                "exec(\"while not p.exists() and time.monotonic() < deadline:\\n time.sleep(0.01)\")"
            )
            started = time.monotonic()

            with self.assertRaisesRegex(
                provenance.ProvenanceError, "did not emit UTF-8 JSON"
            ):
                provenance._run_json_command(
                    [sys.executable, "-c", parent],
                    [],
                    label="parent exit fixture",
                    timeout_seconds=5,
                    drain_timeout_seconds=2,
                )

            self.assertTrue(pid_path.exists(), "descendant never reached readiness")
            self.assertLess(time.monotonic() - started, 4)
            self.assert_process_exits(int(pid_path.read_text()))

    @unittest.skipUnless(
        sys.platform.startswith("linux")
        and bool(os.environ.get("ADJ_PROVENANCE_CGROUP_ROOT")),
        "delegated Linux cgroup v2 fixture",
    )
    def test_posix_guardian_contains_new_session_descendant(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            identity_path = Path(directory) / "child.identity"
            child = (
                "import os, pathlib, time; "
                "raw=pathlib.Path(f'/proc/{os.getpid()}/stat').read_text(); "
                "start=raw.rsplit(')',1)[1].split()[19]; "
                f"pathlib.Path({str(identity_path)!r}).write_text(f'{{os.getpid()}} {{start}}'); "
                "time.sleep(30)"
            )
            parent = (
                "import pathlib, subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {child!r}], start_new_session=True); "
                f"p=pathlib.Path({str(identity_path)!r}); "
                "deadline=time.monotonic()+5; "
                "exec(\"while not p.exists() and time.monotonic() < deadline:\\n time.sleep(0.01)\"); "
                "sys.stdout.buffer.write(b'{}\\n'); sys.stdout.buffer.flush()"
            )

            result = provenance._run_json_command(
                [sys.executable, "-c", parent],
                [],
                label="new-session descendant fixture",
                timeout_seconds=5,
                drain_timeout_seconds=2,
            )

            self.assertEqual(result, {})
            pid_text, starttime = identity_path.read_text().split()
            self.assert_linux_process_identity_exits(int(pid_text), starttime)

    @unittest.skipUnless(
        sys.platform.startswith("linux")
        and bool(os.environ.get("ADJ_PROVENANCE_CGROUP_ROOT")),
        "delegated Linux cgroup v2 fixture",
    )
    def test_posix_guardian_timeout_contains_new_session_descendant(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            identity_path = Path(directory) / "child.identity"
            child = (
                "import os, pathlib, time; "
                "raw=pathlib.Path(f'/proc/{os.getpid()}/stat').read_text(); "
                "start=raw.rsplit(')',1)[1].split()[19]; "
                f"pathlib.Path({str(identity_path)!r}).write_text(f'{{os.getpid()}} {{start}}'); "
                "time.sleep(30)"
            )
            parent = (
                "import pathlib, subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {child!r}], start_new_session=True); "
                f"p=pathlib.Path({str(identity_path)!r}); "
                "deadline=time.monotonic()+5; "
                "exec(\"while not p.exists() and time.monotonic() < deadline:\\n time.sleep(0.01)\"); "
                "time.sleep(30)"
            )

            cgroup_root = Path(os.environ["ADJ_PROVENANCE_CGROUP_ROOT"])
            baseline = {path.name for path in cgroup_root.glob("adj-provenance-*")}
            with self.assertRaisesRegex(
                provenance.ProvenanceError, "timed out"
            ) as raised:
                provenance._run_json_command(
                    [sys.executable, "-c", parent],
                    [],
                    label="new-session timeout fixture",
                    timeout_seconds=1,
                    drain_timeout_seconds=2,
                )

            pid_text, starttime = identity_path.read_text().split()
            self.assert_linux_process_identity_exits(int(pid_text), starttime)
            self.assertEqual(raised.exception.lifecycle.stage, "command.timeout")

            def lifecycle_stages(failure: object) -> list[str]:
                if failure is None:
                    return []
                result = [failure.stage]
                for cause in failure.cleanup_causes:
                    result.extend(lifecycle_stages(cause))
                return result

            self.assertNotIn(
                "containment.confirm", lifecycle_stages(raised.exception.lifecycle)
            )
            current = {path.name for path in cgroup_root.glob("adj-provenance-*")}
            deadline = time.monotonic() + 5
            while current != baseline and time.monotonic() < deadline:
                time.sleep(0.05)
                current = {
                    path.name for path in cgroup_root.glob("adj-provenance-*")
                }
            self.assertEqual(current, baseline)

    @unittest.skipUnless(
        sys.platform.startswith("linux")
        and bool(os.environ.get("ADJ_PROVENANCE_CGROUP_ROOT")),
        "delegated Linux cgroup v2 fixture",
    )
    def test_posix_guardian_closes_protocol_descriptors_before_exec(self) -> None:
        command = (
            "import json, os; "
            "scan=os.scandir('/proc/self/fd'); "
            "candidates=[int(entry.name) for entry in scan if int(entry.name)>2]; "
            "scan.close(); fds=[]; "
            "exec(\"for fd in candidates:\\n"
            " try:\\n  os.fstat(fd); fds.append(fd)\\n"
            " except OSError:\\n  pass\"); "
            "print(json.dumps({'fds':fds}, indent=2, sort_keys=True))"
        )

        result = provenance._run_json_command(
            [sys.executable, "-c", command],
            [],
            label="guardian descriptor fixture",
            timeout_seconds=5,
            drain_timeout_seconds=2,
        )

        self.assertEqual(result, {"fds": []})

    @unittest.skipUnless(
        sys.platform.startswith("linux")
        and bool(os.environ.get("ADJ_PROVENANCE_CGROUP_ROOT")),
        "delegated Linux cgroup v2 fixture",
    )
    def test_posix_guardian_cleans_after_verifier_sigkill(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            identity_path = root / "child.identity"
            child = (
                "import os, pathlib, time; "
                "raw=pathlib.Path(f'/proc/{os.getpid()}/stat').read_text(); "
                "start=raw.rsplit(')',1)[1].split()[19]; "
                f"pathlib.Path({str(identity_path)!r}).write_text(f'{{os.getpid()}} {{start}}'); "
                "time.sleep(30)"
            )
            guarded = (
                "import pathlib, subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {child!r}], start_new_session=True); "
                f"p=pathlib.Path({str(identity_path)!r}); "
                "deadline=time.monotonic()+5; "
                "exec(\"while not p.exists() and time.monotonic() < deadline:\\n time.sleep(0.01)\"); "
                "time.sleep(30)"
            )
            harness = (
                "import sys; "
                f"sys.path.insert(0, {str(SCRIPTS_DIR)!r}); "
                "import adj_stdlib_provenance as p; "
                f"p._run_json_command([sys.executable, '-c', {guarded!r}], [], "
                "label='crash fixture', timeout_seconds=30, drain_timeout_seconds=2)"
            )
            cgroup_root = Path(os.environ["ADJ_PROVENANCE_CGROUP_ROOT"])
            baseline = {path.name for path in cgroup_root.glob("adj-provenance-*")}
            verifier = subprocess.Popen(
                [sys.executable, "-c", harness],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            current = baseline
            try:
                deadline = time.monotonic() + 8
                while not identity_path.exists() and time.monotonic() < deadline:
                    time.sleep(0.05)
                self.assertTrue(identity_path.exists(), "guarded child never started")
                pid_text, starttime = identity_path.read_text().split()
                os.kill(verifier.pid, signal.SIGKILL)
                verifier.wait(timeout=5)
                self.assert_linux_process_identity_exits(int(pid_text), starttime)
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    current = {
                        path.name for path in cgroup_root.glob("adj-provenance-*")
                    }
                    if current == baseline:
                        break
                    time.sleep(0.05)
                self.assertEqual(current, baseline)
            finally:
                if verifier.poll() is None:
                    verifier.kill()
                    verifier.wait(timeout=5)
                assert verifier.stdout is not None
                assert verifier.stderr is not None
                verifier.stdout.close()
                verifier.stderr.close()

    @unittest.skipUnless(os.name == "nt", "Windows Job Object behavior")
    def test_windows_job_assigns_before_process_execution(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "process-started"
            command = (
                "import pathlib, sys; "
                f"pathlib.Path({str(marker)!r}).write_text('started'); "
                "sys.stdout.buffer.write(b'{}\\n')"
            )
            original_assign = provenance._WindowsKillJob.assign
            execution_before_assignment: list[bool] = []

            def delayed_assign(job: object, process: subprocess.Popen[bytes]) -> None:
                time.sleep(0.2)
                execution_before_assignment.append(marker.exists())
                original_assign(job, process)

            with mock.patch.object(
                provenance._WindowsKillJob, "assign", delayed_assign
            ):
                result = provenance._run_json_command(
                    [sys.executable, "-c", command], [], label="suspended fixture"
                )

            self.assertEqual(result, {})
            self.assertEqual(execution_before_assignment, [False])
            self.assertTrue(marker.exists())

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

    def test_formula_inventory_rejects_duplicate_export_names(self) -> None:
        source = (
            b"formulabook first {\n"
            b'  formula repeated(x) = x + 1 source "s" locator "cas://s" trust authoritative\n'
            b"}\n"
            b"formulabook second {\n"
            b'  formula repeated(x) = x + 2 source "s" locator "cas://s" trust authoritative\n'
            b"}\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.adj"
            path.write_bytes(source)
            inventory = provenance._run_formula_inventory(
                self.formula_inventory_command(), path
            )
            with self.assertRaisesRegex(
                provenance.ProvenanceError, "repeats formula name repeated"
            ):
                provenance._validate_formula_inventory_value(
                    inventory, provenance.sha256_bytes(source), source
                )

    def test_formula_execution_replay_requires_the_audit_binary(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        schema_path = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_SCHEMA

        with self.assertRaisesRegex(
            provenance.ProvenanceError,
            "formula evidence replay requires --formula-audit-binary",
        ):
            provenance.validate_repository(
                cas_root,
                manifest_path,
                schema_path,
                workspace_root=formula_inventory_migration.REPO_ROOT,
                formula_inventory_command=self.formula_inventory_command(),
            )

    def test_formula_input_reference_resolves_dependency_owner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, _manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            bundle = provenance._json_object(
                cas, hashes["bundle"], "provenance_bundle"
            )
            clause = bundle["clauses"][0]
            identity = {
                "provenance": {
                    "corroborations": [],
                    "locator": clause["locator"],
                    "quote": {
                        "byte_len": clause["end"] - clause["start"],
                        "byte_offset": clause["start"],
                        "snapshot_sha256": clause["snapshot_sha256"],
                        "text_sha256": clause["quote_sha256"],
                    },
                    "source": "fixture input",
                    "trust": "authoritative",
                },
                "term": "fixture_value(1)",
            }

            reference, links = provenance._input_reference(
                [(hashes["bundle"], bundle)], identity
            )

            self.assertEqual(
                reference["owner"],
                {
                    "bundle_id": bundle["bundle_id"],
                    "bundle_sha256": hashes["bundle"],
                    "kind": "dependency",
                },
            )
            self.assertEqual(
                reference["owner_source_sha256"],
                bundle["input"]["raw_source_sha256"],
            )
            self.assertEqual(
                reference["owner_source_ir_sha256"],
                bundle["input"]["source_ir_sha256"],
            )
            self.assertEqual(reference["snapshot_sha256"], hashes["snapshot"])
            self.assertEqual(reference["source_ir_sha256"], hashes["rendered_ir"])
            self.assertEqual(reference["schema_version"], 2)
            self.assertEqual(
                links,
                {
                    hashes["bundle"],
                    hashes["input"],
                    hashes["input_ir"],
                    hashes["snapshot"],
                    hashes["rendered_ir"],
                },
            )

            query_reference, _query_links = provenance._input_reference(
                [(None, bundle)], identity
            )
            self.assertEqual(
                query_reference["owner"],
                {"bundle_id": bundle["bundle_id"], "kind": "query"},
            )

    def test_formula_input_reference_rejects_ambiguous_closure_quote(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, _manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            bundle = provenance._json_object(
                cas, hashes["bundle"], "provenance_bundle"
            )
            duplicate = deepcopy(bundle)
            duplicate["bundle_id"] = "test.duplicate.v1"
            clause = bundle["clauses"][0]
            identity = {
                "provenance": {
                    "corroborations": [],
                    "locator": clause["locator"],
                    "quote": {
                        "byte_len": clause["end"] - clause["start"],
                        "byte_offset": clause["start"],
                        "snapshot_sha256": clause["snapshot_sha256"],
                        "text_sha256": clause["quote_sha256"],
                    },
                    "source": "fixture input",
                    "trust": "authoritative",
                },
                "term": "fixture_value(1)",
            }

            with self.assertRaisesRegex(
                provenance.ProvenanceError,
                "must resolve to exactly one closure claim",
            ):
                provenance._input_reference(
                    [(hashes["bundle"], bundle), ("0" * 64, duplicate)], identity
                )

    def test_formula_input_reference_rejects_a_forged_locator(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = (
            formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        )
        cas = provenance.Cas(cas_root)
        cas.load()
        query_hash = self.registered_bundle_hash(
            cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
        )
        query = provenance._json_object(cas, query_hash, "provenance_bundle")
        audit = provenance._materialize_formula_audit(
            cas, query, self.formula_audit_command()
        )
        audit["derivations"][0]["inputs"][0]["provenance"]["locator"] = (
            "repo://forged/input.txt"
        )

        with self.assertRaisesRegex(
            provenance.ProvenanceError, "input locator disagrees"
        ):
            provenance._normalized_formula_evidence(cas, query, audit)

    def test_formula_input_reference_rejects_a_forged_quote_status(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = (
            formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        )
        cas = provenance.Cas(cas_root)
        cas.load()
        query_hash = self.registered_bundle_hash(
            cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
        )
        query = provenance._json_object(cas, query_hash, "provenance_bundle")
        audit = provenance._materialize_formula_audit(
            cas, query, self.formula_audit_command()
        )
        status = audit["derivations"][0]["verification"]["input_quotes"][0][
            "quote"
        ]
        status["byte_len"] += 1

        with self.assertRaisesRegex(
            provenance.ProvenanceError, "quote status disagrees"
        ):
            provenance._normalized_formula_evidence(cas, query, audit)

    def test_formula_audit_import_requires_a_direct_cas_edge(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = (
            formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        )
        cas = provenance.Cas(cas_root)
        cas.load()
        query_hash = self.registered_bundle_hash(
            cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
        )
        query = provenance._json_object(cas, query_hash, "provenance_bundle")
        audit = provenance._materialize_formula_audit(
            cas, query, self.formula_audit_command()
        )
        by_source, formula_bundles = provenance._execution_graph(cas, query)
        nested_import = next(
            item
            for item in audit["imports"]
            if item["importer_source_sha256"]
            != query["input"]["raw_source_sha256"]
        )
        importer_source = nested_import["importer_source_sha256"]
        importer_hash, importer = by_source[importer_source]
        importer_without_edge = deepcopy(importer)
        importer_without_edge["dependencies"] = []
        by_source[importer_source] = (importer_hash, importer_without_edge)

        with (
            mock.patch.object(
                provenance,
                "_execution_graph",
                return_value=(by_source, formula_bundles),
            ),
            self.assertRaisesRegex(
                provenance.ProvenanceError, "import is not a direct CAS dependency"
            ),
        ):
            provenance._normalized_formula_evidence(cas, query, audit)

    def test_direct_formula_dependency_ignores_fact_only_siblings(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = (
            formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        )
        cas = provenance.Cas(cas_root)
        cas.load()
        roots = formula_inventory_migration._registered_roots(cas, manifest_path)
        ratio_query = provenance._json_object(
            cas,
            roots["adj.math.arithmetic.ratio.query.v1"],
            "provenance_bundle",
        )
        ratio_formula = roots["adj.math.arithmetic.ratio.v1"]
        fact_only = roots["adj.math.arithmetic.percent_of.query.v1"]
        query_with_sibling = deepcopy(ratio_query)
        query_with_sibling["dependencies"] = sorted([ratio_formula, fact_only])

        selected, _inventory_hash, _inventory = provenance._direct_formula_inventory(
            cas, query_with_sibling
        )
        self.assertEqual(selected, ratio_formula)

        query_with_sibling["dependencies"].append(
            roots["adj.math.arithmetic.primitives.v1"]
        )
        with self.assertRaisesRegex(
            provenance.ProvenanceError, "exactly one direct formula dependency"
        ):
            provenance._direct_formula_inventory(cas, query_with_sibling)

    def test_transitive_imported_fact_witness_passes_strict_replay(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            cas_root = workspace / provenance.DEFAULT_ROOT
            manifest_path = workspace / provenance.DEFAULT_MANIFEST
            cas = provenance.Cas(cas_root)
            facts_path = "code/example/imported-facts.adj"
            formula_path = "code/example/imported-formula.adj"
            query_path = "code/example/imported-query.adj"
            fact_fixture_path = "code/example/imported-fact.txt"
            formula_fixture_path = "code/example/imported-formula.txt"
            unused_fixture_path = "code/example/unused-query-fact.txt"
            fact_fixture = b"imported value is 3."
            formula_fixture = b"Double a value by multiplying it by 2."
            unused_fixture = b"unused value is 1."
            fact_snapshot = provenance.sha256_bytes(fact_fixture)
            formula_snapshot = provenance.sha256_bytes(formula_fixture)
            unused_snapshot = provenance.sha256_bytes(unused_fixture)
            facts_source = (
                b"dictionary imported_vocab {\n"
                b"  define imported : finding\n"
                b"  define unused : finding\n"
                b"}\n"
                b"observe imported(3)\n"
                b'  quote "imported value is 3." at 0 snapshot "'
                + fact_snapshot.encode()
                + b'"\n  source "imported value fixture"\n'
                b'  locator "repo://code/example/imported-fact.txt"\n'
                b"  trust authoritative\n"
            )
            formula_source = (
                b'import "imported-facts.adj"\n'
                b"formulabook imported_math {\n"
                b"  formula double(value) = value * 2\n"
                b'    quote "Double a value by multiplying it by 2." at 0 snapshot "'
                + formula_snapshot.encode()
                + b'"\n    source "Double a value by multiplying it by 2."\n'
                b'    locator "repo://code/example/imported-formula.txt"\n'
                b"    trust authoritative\n"
                b"}\n"
            )
            query_source = (
                b'import "imported-formula.adj"\n'
                b"observe unused(1)\n"
                b'  quote "unused value is 1." at 0 snapshot "'
                + unused_snapshot.encode()
                + b'"\n  source "unused query fixture"\n'
                b'  locator "repo://code/example/unused-query-fact.txt"\n'
                b"  trust authoritative\n"
                b"? double(imported)\n"
            )
            for repo_path, data in (
                (facts_path, facts_source),
                (formula_path, formula_source),
                (query_path, query_source),
                (fact_fixture_path, fact_fixture),
                (formula_fixture_path, formula_fixture),
                (unused_fixture_path, unused_fixture),
            ):
                destination = workspace / repo_path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)

            def line_range(data: bytes, marker: bytes, final: bytes) -> tuple[int, int]:
                start = data.index(marker)
                final_start = data.index(final, start)
                return start, data.index(b"\n", final_start) + 1

            original_root = ratio_builder.REPO_ROOT
            ratio_builder.REPO_ROOT = workspace
            try:
                fact_start, fact_end = line_range(
                    facts_source, b"observe imported", b"trust authoritative"
                )
                facts_input, facts_claims = ratio_builder.local_source(
                    cas,
                    facts_path,
                    [("test.imported.fact", fact_start, fact_end)],
                    "imported facts input",
                    discarded_reason="dictionary syntax outside the selected fact",
                )
                fact_external, fact_external_claims = ratio_builder.local_source(
                    cas,
                    fact_fixture_path,
                    [("test.imported.fact", 0, len(fact_fixture))],
                    "imported fact fixture",
                    discarded_reason="no discarded fixture bytes",
                )
                formula_import_start, formula_import_end = line_range(
                    formula_source, b'import "', b'import "'
                )
                formula_start, formula_end = line_range(
                    formula_source, b"  formula double", b"trust authoritative"
                )
                formula_input, formula_claims = ratio_builder.local_source(
                    cas,
                    formula_path,
                    [
                        (
                            "test.imported.formula.import",
                            formula_import_start,
                            formula_import_end,
                        ),
                        ("test.imported.formula", formula_start, formula_end),
                    ],
                    "imported formula input",
                    discarded_reason="formulabook syntax outside selected rules",
                )
                formula_external, formula_external_claims = ratio_builder.local_source(
                    cas,
                    formula_fixture_path,
                    [("test.imported.formula", 0, len(formula_fixture))],
                    "imported formula fixture",
                    discarded_reason="no discarded fixture bytes",
                )
                query_import_start, query_import_end = line_range(
                    query_source, b'import "', b'import "'
                )
                unused_start, unused_end = line_range(
                    query_source, b"observe unused", b"trust authoritative"
                )
                question_start = query_source.index(b"? double")
                query_input, query_claims = ratio_builder.local_source(
                    cas,
                    query_path,
                    [
                        (
                            "test.imported.query.import",
                            query_import_start,
                            query_import_end,
                        ),
                        ("test.imported.query.unused", unused_start, unused_end),
                        (
                            "test.imported.query.question",
                            question_start,
                            len(query_source),
                        ),
                    ],
                    "imported fact query input",
                    discarded_reason="no discarded executable query bytes",
                )
                unused_external, unused_external_claims = ratio_builder.local_source(
                    cas,
                    unused_fixture_path,
                    [("test.imported.query.unused", 0, len(unused_fixture))],
                    "unused query fixture",
                    discarded_reason="no discarded fixture bytes",
                )
            finally:
                ratio_builder.REPO_ROOT = original_root

            def accepted_clause(
                claim_id: str,
                input_claim: dict[str, object],
                external: dict[str, object],
                external_claim: dict[str, object],
                repo_path: str,
                classification: str,
            ) -> dict[str, object]:
                return {
                    **external_claim,
                    "input_claim": ratio_builder.input_claim_payload(input_claim),
                    "locator": f"repo://{repo_path}",
                    "resolution": {
                        "authority_receipt_sha256": external["receipt_sha256"],
                        "authority_source_sha256": external["raw_source_sha256"],
                        "classification": classification,
                        "kind": "accepted_root",
                        "reason": "deterministic imported-input replay fixture",
                    },
                    "snapshot_sha256": external["raw_source_sha256"],
                    "source_ir_sha256": external["source_ir_sha256"],
                }

            facts_bundle = {
                "bundle_id": "test.imported.facts.v1",
                "clauses": [
                    accepted_clause(
                        "test.imported.fact",
                        facts_claims["test.imported.fact"],
                        fact_external,
                        fact_external_claims["test.imported.fact"],
                        fact_fixture_path,
                        "accepted_fact",
                    )
                ],
                "dependencies": [],
                "input": {
                    key: facts_input[key]
                    for key in (
                        "raw_source_sha256",
                        "receipt_sha256",
                        "source_ir_sha256",
                    )
                },
                "kind": "provenance_bundle",
                "library": facts_path,
                "sources": [facts_input, fact_external],
            }
            facts_hash = cas.put_json(
                facts_bundle,
                kind="provenance_bundle",
                label="imported facts bundle",
                links=provenance._bundle_declared_links(facts_bundle),
            )
            formula_inventory_hash = provenance.put_formula_parser_inventory(
                cas,
                formula_input["raw_source_sha256"],
                self.formula_inventory_command(),
                label="imported formula inventory",
            )
            formula_bundle = {
                "bundle_id": "test.imported.formula.v1",
                "clauses": [
                    accepted_clause(
                        "test.imported.formula",
                        formula_claims["test.imported.formula"],
                        formula_external,
                        formula_external_claims["test.imported.formula"],
                        formula_fixture_path,
                        "primary_definition",
                    )
                ],
                "dependencies": [facts_hash],
                "formula_inventory_sha256": formula_inventory_hash,
                "input": {
                    key: formula_input[key]
                    for key in (
                        "raw_source_sha256",
                        "receipt_sha256",
                        "source_ir_sha256",
                    )
                },
                "kind": "provenance_bundle",
                "library": formula_path,
                "sources": [formula_input, formula_external],
            }
            formula_hash = cas.put_json(
                formula_bundle,
                kind="provenance_bundle",
                label="imported formula bundle",
                links=provenance._bundle_declared_links(formula_bundle),
            )
            query_bundle = {
                "bundle_id": "test.imported.query.v1",
                "clauses": [
                    accepted_clause(
                        "test.imported.query.unused",
                        query_claims["test.imported.query.unused"],
                        unused_external,
                        unused_external_claims["test.imported.query.unused"],
                        unused_fixture_path,
                        "accepted_fact",
                    )
                ],
                "dependencies": [formula_hash],
                "input": {
                    key: query_input[key]
                    for key in (
                        "raw_source_sha256",
                        "receipt_sha256",
                        "source_ir_sha256",
                    )
                },
                "kind": "provenance_bundle",
                "library": query_path,
                "sources": [query_input, unused_external],
            }
            derivations, witnesses = provenance.put_formula_execution_evidence(
                cas,
                query_bundle,
                self.formula_audit_command(),
                label="transitive imported fact",
            )
            query_bundle["formula_derivation_sha256s"] = derivations
            query_bundle["execution_witness_sha256s"] = witnesses
            query_hash = cas.put_json(
                query_bundle,
                kind="provenance_bundle",
                label="imported fact query bundle",
                links=provenance._bundle_declared_links(query_bundle),
            )
            cas.write_index()
            manifest_path.parent.mkdir(parents=True, exist_ok=True)
            manifest_path.write_bytes(
                provenance.canonical_json_bytes(
                    {
                        "algorithm": "sha256",
                        "bundle_hashes": sorted(
                            [facts_hash, formula_hash, query_hash]
                        ),
                        "manifest_id": "test.imported.fact.v1",
                        "schema_version": 1,
                    }
                )
            )

            result = provenance.validate_repository(
                cas_root,
                manifest_path,
                formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_SCHEMA,
                workspace_root=workspace,
                formula_inventory_command=self.formula_inventory_command(),
                formula_audit_command=self.formula_audit_command(),
            )
            self.assertTrue(result["valid"])
            witness = provenance._json_object(cas, witnesses[0], "execution_witness")
            self.assertEqual(len(witness["inputs"]), 1)
            reference = witness["inputs"][0]
            self.assertEqual(
                reference["owner"],
                {
                    "bundle_id": facts_bundle["bundle_id"],
                    "bundle_sha256": facts_hash,
                    "kind": "dependency",
                },
            )
            self.assertEqual(
                reference["owner_source_sha256"],
                facts_input["raw_source_sha256"],
            )
            self.assertTrue(
                {
                    facts_hash,
                    facts_input["raw_source_sha256"],
                    facts_input["source_ir_sha256"],
                    fact_external["raw_source_sha256"],
                    fact_external["source_ir_sha256"],
                }.issubset(cas.index[witnesses[0]]["links"])
            )

    def test_formula_execution_replay_rejects_a_substituted_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = formula_inventory_migration.REPO_ROOT
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
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            query_hash = self.registered_bundle_hash(
                cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
            )
            query = provenance._json_object(cas, query_hash, "provenance_bundle")
            witness_hash = query["execution_witness_sha256s"][0]
            witness = provenance._json_object(cas, witness_hash, "execution_witness")
            witness["result"]["f64_bits"] = "0000000000000000"
            substituted_hash = cas.put_json(
                witness,
                kind="execution_witness",
                label="substituted execution result",
                links=cas.index[witness_hash]["links"],
            )
            query["execution_witness_sha256s"] = [substituted_hash]
            substituted_query_hash = cas.put_json(
                query,
                kind="provenance_bundle",
                label="query with substituted execution result",
                links=provenance._bundle_declared_links(query),
            )
            manifest["bundle_hashes"] = sorted(
                substituted_query_hash if digest == query_hash else digest
                for digest in manifest["bundle_hashes"]
            )
            cas.write_index()
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))

            with self.assertRaisesRegex(
                provenance.ProvenanceError,
                "stored formula evidence set disagrees with replay",
            ):
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    workspace / provenance.DEFAULT_SCHEMA,
                    workspace_root=workspace,
                    formula_inventory_command=self.formula_inventory_command(),
                    formula_audit_command=self.formula_audit_command(),
                )

    def test_formula_execution_replay_rejects_a_reachable_legacy_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = formula_inventory_migration.REPO_ROOT
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
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            query_hash = self.registered_bundle_hash(
                cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
            )
            query = provenance._json_object(cas, query_hash, "provenance_bundle")
            witness_hash = query["execution_witness_sha256s"][0]
            witness = provenance._json_object(cas, witness_hash, "execution_witness")

            def legacy(reference: dict[str, object]) -> dict[str, object]:
                return {
                    "claim_id": reference["claim_id"],
                    "identity": reference["identity"],
                    "source_ir_sha256": reference["source_ir_sha256"],
                }

            witness["inputs"] = [legacy(item) for item in witness["inputs"]]
            for check in witness["verification"]["input_quotes"]:
                check["identity"] = legacy(check["identity"])
            legacy_hash = cas.put_json(
                witness,
                kind="execution_witness",
                label="reachable legacy execution input",
                links=cas.index[witness_hash]["links"],
            )
            query["execution_witness_sha256s"] = [legacy_hash]
            legacy_query_hash = cas.put_json(
                query,
                kind="provenance_bundle",
                label="query with reachable legacy execution input",
                links=provenance._bundle_declared_links(query),
            )
            manifest["bundle_hashes"] = sorted(
                legacy_query_hash if digest == query_hash else digest
                for digest in manifest["bundle_hashes"]
            )
            cas.write_index()
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))

            with self.assertRaisesRegex(
                provenance.ProvenanceError,
                "stored formula evidence set disagrees with replay",
            ):
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    workspace / provenance.DEFAULT_SCHEMA,
                    workspace_root=workspace,
                    formula_inventory_command=self.formula_inventory_command(),
                    formula_audit_command=self.formula_audit_command(),
                )

    def test_formula_inventory_migration_accepts_only_a_legacy_baseline(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = formula_inventory_migration.REPO_ROOT
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
            schema_path = workspace / provenance.DEFAULT_SCHEMA
            legacy_roots = self.downgrade_formula_inputs_to_legacy(workspace)
            with self.assertRaisesRegex(
                provenance.ProvenanceError,
                "stored formula evidence set disagrees with replay",
            ):
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace_root=workspace,
                    formula_inventory_command=self.formula_inventory_command(),
                    formula_audit_command=self.formula_audit_command(),
                )

            original_roots = (
                arithmetic_builder.REPO_ROOT,
                ratio_builder.REPO_ROOT,
                percent_of_builder.REPO_ROOT,
            )
            arithmetic_builder.REPO_ROOT = workspace
            ratio_builder.REPO_ROOT = workspace
            percent_of_builder.REPO_ROOT = workspace
            try:
                result = formula_inventory_migration.migrate(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace,
                    formula_inventory_command=self.formula_inventory_command(),
                    formula_audit_command=self.formula_audit_command(),
                )
            finally:
                (
                    arithmetic_builder.REPO_ROOT,
                    ratio_builder.REPO_ROOT,
                    percent_of_builder.REPO_ROOT,
                ) = original_roots

            self.assertTrue(
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace_root=workspace,
                    formula_inventory_command=self.formula_inventory_command(),
                    formula_audit_command=self.formula_audit_command(),
                )["valid"]
            )
            cas = provenance.Cas(cas_root)
            cas.load()
            current = formula_inventory_migration._registered_roots(
                cas, manifest_path
            )
            for bundle_id, replacement in result["replacements"].items():
                self.assertEqual(
                    replacement["expected_old_sha256"], legacy_roots[bundle_id]
                )
                self.assertEqual(replacement["new_sha256"], current[bundle_id])
                bundle = provenance._json_object(
                    cas, current[bundle_id], "provenance_bundle"
                )
                for witness_hash in bundle.get("execution_witness_sha256s", []):
                    witness = provenance._json_object(
                        cas, witness_hash, "execution_witness"
                    )
                    self.assertTrue(
                        all(item["schema_version"] == 2 for item in witness["inputs"])
                    )

    def test_formula_execution_rejects_swapped_verified_provenance(self) -> None:
        cas_root = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_ROOT
        manifest_path = formula_inventory_migration.REPO_ROOT / provenance.DEFAULT_MANIFEST
        cas = provenance.Cas(cas_root)
        cas.load()
        query_hash = self.registered_bundle_hash(
            cas_root, manifest_path, "adj.math.arithmetic.ratio.query.v1"
        )
        query = provenance._json_object(cas, query_hash, "provenance_bundle")
        audit = provenance._materialize_formula_audit(
            cas, query, self.formula_audit_command()
        )
        formula_quotes = audit["derivations"][0]["verification"]["formula_quotes"]
        self.assertGreaterEqual(len(formula_quotes), 2)
        formula_quotes[0]["provenance"], formula_quotes[1]["provenance"] = (
            formula_quotes[1]["provenance"],
            formula_quotes[0]["provenance"],
        )

        with self.assertRaisesRegex(
            provenance.ProvenanceError,
            "formula audit provenance does not match its CAS clause",
        ):
            provenance._normalized_formula_evidence(cas, query, audit)

    def test_formula_inventory_migration_replaces_the_complete_closure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            repo_root = formula_inventory_migration.REPO_ROOT
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
            schema_path = workspace / provenance.DEFAULT_SCHEMA
            old_roots = self.remove_formula_inventories_from_closure(
                cas_root, manifest_path
            )
            cas = provenance.Cas(cas_root)
            cas.load()
            formula_command = self.formula_inventory_command()
            formula_audit_command = self.formula_audit_command()
            original_roots = (
                arithmetic_builder.REPO_ROOT,
                ratio_builder.REPO_ROOT,
                percent_of_builder.REPO_ROOT,
            )
            arithmetic_builder.REPO_ROOT = workspace
            ratio_builder.REPO_ROOT = workspace
            percent_of_builder.REPO_ROOT = workspace
            try:
                result = formula_inventory_migration.migrate(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace,
                    formula_inventory_command=formula_command,
                    formula_audit_command=formula_audit_command,
                )
                rerun = formula_inventory_migration.migrate(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace,
                    formula_inventory_command=formula_command,
                    formula_audit_command=formula_audit_command,
                )
            finally:
                (
                    arithmetic_builder.REPO_ROOT,
                    ratio_builder.REPO_ROOT,
                    percent_of_builder.REPO_ROOT,
                ) = original_roots

            cas.load()
            new_roots = formula_inventory_migration._registered_roots(
                cas, manifest_path
            )
            self.assertEqual(
                set(result["replacements"]), formula_inventory_migration.ROOT_IDS
            )
            self.assertTrue(
                {
                    old_roots[bundle_id]
                    for bundle_id in formula_inventory_migration.ROOT_IDS
                }.issubset(result["pruned_sha256s"])
            )
            self.assertTrue(
                all(not cas.object_path(digest).exists() for digest in result["pruned_sha256s"])
            )
            self.assertTrue(
                all(
                    old_roots[key] != new_roots[key]
                    for key in formula_inventory_migration.ROOT_IDS
                )
            )
            self.assertEqual(rerun["pruned_sha256s"], [])
            self.assertTrue(
                all(
                    replacement["expected_old_sha256"] == replacement["new_sha256"]
                    for replacement in rerun["replacements"].values()
                )
            )

            formula_names = {
                "adj.math.arithmetic.percent_of.v1": ["percent_of"],
                "adj.math.arithmetic.primitives.v1": [
                    "sum",
                    "difference",
                    "product",
                    "quotient",
                ],
                "adj.math.arithmetic.ratio.v1": ["ratio"],
            }
            for bundle_id, digest in new_roots.items():
                bundle = provenance._json_object(cas, digest, "provenance_bundle")
                if bundle_id in formula_names:
                    inventory = provenance._json_object(
                        cas,
                        bundle["formula_inventory_sha256"],
                        "formula_parser_inventory",
                    )
                    self.assertEqual(
                        [item["formula"] for item in inventory["formulas"]],
                        formula_names[bundle_id],
                    )
                else:
                    self.assertNotIn("formula_inventory_sha256", bundle)

            arithmetic_hash = new_roots["adj.math.arithmetic.primitives.v1"]
            for bundle_id in (
                "adj.math.arithmetic.percent_of.v1",
                "adj.math.arithmetic.ratio.v1",
            ):
                bundle = provenance._json_object(
                    cas, new_roots[bundle_id], "provenance_bundle"
                )
                self.assertEqual(bundle["dependencies"], [arithmetic_hash])
            ratio = provenance._json_object(
                cas,
                new_roots["adj.math.arithmetic.ratio.v1"],
                "provenance_bundle",
            )
            self.assertEqual(
                ratio["clauses"][0]["resolution"]["bundle_sha256"],
                arithmetic_hash,
            )
            query_dependencies = {
                "adj.math.arithmetic.percent_of.query.v1": (
                    "adj.math.arithmetic.percent_of.v1"
                ),
                "adj.math.arithmetic.primitives.query.v1": (
                    "adj.math.arithmetic.primitives.v1"
                ),
                "adj.math.arithmetic.ratio.query.v1": ("adj.math.arithmetic.ratio.v1"),
            }
            for query_id, dependency_id in query_dependencies.items():
                query = provenance._json_object(
                    cas, new_roots[query_id], "provenance_bundle"
                )
                self.assertEqual(query["dependencies"], [new_roots[dependency_id]])
                query_ir = provenance._json_object(
                    cas, query["input"]["source_ir_sha256"], "source_ir"
                )
                claims = provenance._claims_by_id(query_ir)
                question_claims = [
                    claim
                    for claim_id, claim in claims.items()
                    if claim_id.startswith("adj.question.")
                ]
                self.assertTrue(question_claims)
                self.assertTrue(
                    all("% expect" in claim["quote"] for claim in question_claims)
                )
            self.assertTrue(
                provenance.validate_repository(
                    cas_root,
                    manifest_path,
                    schema_path,
                    workspace_root=workspace,
                    formula_inventory_command=formula_command,
                    formula_audit_command=formula_audit_command,
                )["valid"]
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

    def test_root_replacement_rejects_an_unmigrated_same_id_dependency(self) -> None:
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

            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            with (
                self.assertRaisesRegex(provenance.ProvenanceError, "resolves to both"),
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
            self.migrate_formula_closure(workspace)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            arithmetic_hash = self.registered_bundle_hash(
                cas_root,
                manifest_path,
                "adj.math.arithmetic.primitives.v1",
            )
            formula_command = self.formula_inventory_command()
            audit_command = self.formula_audit_command()
            original_root = ratio_builder.REPO_ROOT
            ratio_builder.REPO_ROOT = workspace
            try:
                with provenance.BundleRegistrationTransaction(
                    cas_root,
                    manifest_path,
                    expected_manifest_id="adj.stdlib.provenance.v1",
                    schema_path=workspace / provenance.DEFAULT_SCHEMA,
                    workspace_root=workspace,
                    formula_inventory_command=formula_command,
                    formula_audit_command=audit_command,
                ) as transaction:
                    transaction.commit(
                        ratio_builder.build(
                            transaction.cas,
                            None,
                            arithmetic_bundle_sha256=arithmetic_hash,
                            formula_inventory_command=formula_command,
                            formula_audit_command=audit_command,
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
                formula_inventory_command=formula_command,
                formula_audit_command=audit_command,
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
                    formula_inventory_command=formula_command,
                    formula_audit_command=audit_command,
                ) as transaction:
                    transaction.commit(
                        ratio_builder.build(
                            transaction.cas,
                            captured_source,
                            arithmetic_bundle_sha256=arithmetic_hash,
                            formula_inventory_command=formula_command,
                            formula_audit_command=audit_command,
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
            self.migrate_formula_closure(workspace)
            baseline_index = (cas_root / "index.json").read_bytes()
            baseline_manifest = manifest_path.read_bytes()
            arithmetic_hash = self.registered_bundle_hash(
                cas_root,
                manifest_path,
                "adj.math.arithmetic.primitives.v1",
            )
            formula_command = self.formula_inventory_command()
            audit_command = self.formula_audit_command()
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
                        formula_inventory_command=formula_command,
                        formula_audit_command=audit_command,
                    ) as transaction:
                        transaction.commit(
                            percent_of_builder.build(
                                transaction.cas,
                                captured_source,
                                arithmetic_bundle_sha256=arithmetic_hash,
                                formula_inventory_command=formula_command,
                                formula_audit_command=audit_command,
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
                formula_inventory_command=formula_command,
                formula_audit_command=audit_command,
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
        formula_command = self.formula_inventory_command()
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
                builder.build(
                    cas,
                    None,
                    formula_inventory_command=formula_command,
                )
            with (
                self.subTest(builder=builder.__name__, failure="malformed hash"),
                self.assertRaisesRegex(provenance.ProvenanceError, "lowercase SHA-256"),
            ):
                builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256="not-a-sha256",
                    formula_inventory_command=formula_command,
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
                    formula_inventory_command=formula_command,
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
                    formula_inventory_command=formula_command,
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
                    formula_inventory_command=formula_command,
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
                formula_command = self.formula_inventory_command()
                audit_command = self.formula_audit_command()
                ratio_roots = ratio_builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=alternate_hash,
                    formula_inventory_command=formula_command,
                    formula_audit_command=audit_command,
                )
                percent_roots = percent_of_builder.build(
                    cas,
                    None,
                    arithmetic_bundle_sha256=alternate_hash,
                    formula_inventory_command=formula_command,
                    formula_audit_command=audit_command,
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

    def test_transitive_graph_rejects_two_digests_for_one_bundle_id(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)
            cas = provenance.Cas(cas_root)
            cas.load()
            original = provenance._json_object(
                cas, hashes["bundle"], "provenance_bundle"
            )
            alternate = deepcopy(original)
            alternate["clauses"][0]["resolution"]["reason"] = (
                "alternate digest with the same claimed bundle identity"
            )
            alternate_hash = cas.put_json(
                alternate,
                kind="provenance_bundle",
                label="alternate same-ID root",
                links=provenance._bundle_declared_links(alternate),
            )
            consumer = deepcopy(original)
            consumer["bundle_id"] = "test.consumer.v1"
            consumer["dependencies"] = [hashes["bundle"]]
            consumer_hash = cas.put_json(
                consumer,
                kind="provenance_bundle",
                label="consumer retaining historical root",
                links=provenance._bundle_declared_links(consumer),
            )
            cas.write_index()
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["bundle_hashes"] = sorted([alternate_hash, consumer_hash])
            manifest_path.write_bytes(provenance.canonical_json_bytes(manifest))

            with self.assertRaisesRegex(provenance.ProvenanceError, "resolves to both"):
                provenance.validate_repository(cas_root, manifest_path)

    def test_dependency_resolution_names_an_exported_claim(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cas_root, manifest_path, hashes = self.build_repository(root)

            def mutate(bundle: dict[str, object], _cas: provenance.Cas) -> None:
                bundle["bundle_id"] = "test.consumer.v1"
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
