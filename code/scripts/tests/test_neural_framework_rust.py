"""Fail-closed native evidence receipt checks, independent of Cargo availability."""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "validate_neural_framework_rust.py"
SPEC = importlib.util.spec_from_file_location("native_rust_evidence", SCRIPT)
evidence = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evidence)


def receipt(step=3000):
    return {
        "checkpoint": {"step": step},
        "predictions": [[0.01], [0.99], [0.99], [0.01]],
        "initial_loss": 0.25 if step == 3000 else 0.001,
        "final_loss": 0.001,
    }


class NativeRustEvidenceTests(unittest.TestCase):
    def run_receipts(self, documents):
        results = [
            subprocess.CompletedProcess([], 0, json.dumps(d), "") for d in documents
        ]
        with (
            patch.object(evidence.shutil, "which", return_value="cargo"),
            patch.object(
                evidence.subprocess,
                "run",
                side_effect=results,
            ) as run,
        ):
            result = evidence.validate(Path("trusted source with spaces/Cargo.toml"))
        return result, run.call_args_list

    def test_executes_fixed_bounded_argv_and_actual_resume_input(self):
        result, calls = self.run_receipts([receipt(), receipt(3001)])
        self.assertEqual(result["native_lane"], "rust")
        self.assertEqual(result["resumed_step"], 3001)
        self.assertEqual(
            result["cross_language_interchange"], "pending-separate-tranche"
        )
        self.assertEqual(len(calls), 2)
        self.assertIn("trusted source with spaces", str(calls[0].args[0]))
        self.assertEqual(calls[1].args[0][-2:], ["--", "--resume"])
        self.assertEqual(json.loads(calls[1].kwargs["input"]), {"step": 3000})
        for call in calls:
            self.assertNotIn("shell", call.kwargs)
            self.assertTrue(call.kwargs["check"])
            self.assertEqual(call.kwargs["timeout"], 120)

    def test_missing_cargo_and_process_errors_are_not_success(self):
        with (
            patch.object(evidence.shutil, "which", return_value=None),
            self.assertRaises(ValueError),
        ):
            evidence.validate(Path("Cargo.toml"))
        for error in [
            subprocess.TimeoutExpired("cargo", 120),
            subprocess.CalledProcessError(1, "cargo"),
        ]:
            with (
                patch.object(evidence.shutil, "which", return_value="cargo"),
                patch.object(evidence.subprocess, "run", side_effect=error),
                self.assertRaises(type(error)),
            ):
                evidence.validate(Path("Cargo.toml"))

    def test_malformed_nonfinite_and_untrained_receipts_fail(self):
        bad = []
        for field, value in [
            ("initial_loss", True),
            ("final_loss", float("nan")),
            ("final_loss", "0.001"),
            ("final_loss", -1),
            ("final_loss", 0.5),
            ("checkpoint", {"step": 2}),
            ("predictions", []),
            ("predictions", [[0, 1]] * 4),
            ("predictions", [[True]] * 4),
            ("predictions", [[float("inf")]] * 4),
            ("predictions", [[0]] * 4),
        ]:
            document = receipt()
            document[field] = value
            bad.append(document)
        bad.extend([[], {}, {**receipt(), "extra": 0}])
        for document in bad:
            with self.subTest(document=document), self.assertRaises(ValueError):
                self.run_receipts([document, receipt(3001)])

    def test_resumed_receipt_must_retain_loss_step_and_training(self):
        for field, value in [
            ("initial_loss", 0.2),
            ("checkpoint", {"step": 3000}),
            ("final_loss", -1),
            ("final_loss", 0.5),
            ("predictions", [[1]] * 4),
        ]:
            resumed = copy.deepcopy(receipt(3001))
            resumed[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.run_receipts([receipt(), resumed])

    def test_build_fronts_execute_this_contract_and_native_evidence(self):
        package = SCRIPT.parents[2] / "code/packages/rust/neural-framework"
        for filename in ("BUILD", "BUILD_windows"):
            text = (package / filename).read_text(encoding="utf-8")
            self.assertIn("test_neural_framework_rust.py", text)
            self.assertIn("validate_neural_framework_rust.py", text)

    def test_coverage_filter_is_relative_to_the_real_workspace(self):
        # Tarpaulin 0.37.2 strips cargo metadata.workspace_root, not the
        # repository root. A leading */ requires an absent parent directory.
        import fnmatch
        import shlex

        workspace = SCRIPT.parents[2] / "code/packages/rust"
        source = workspace / "neural-framework/src/lib.rs"
        build = (workspace / "neural-framework/BUILD").read_text(encoding="utf-8")
        command = shlex.split(build.split("then ", 1)[1].split("; fi", 1)[0])
        pattern = command[command.index("--include-files") + 1]
        relative = source.relative_to(workspace).as_posix()
        self.assertEqual(pattern, "neural-framework/src/*")
        self.assertTrue(fnmatch.fnmatchcase(relative, pattern))
        self.assertFalse(fnmatch.fnmatchcase(relative, "*/neural-framework/src/*"))
        for excluded in ("matrix/src/lib.rs", "neural-framework/tests/dense.rs"):
            self.assertFalse(fnmatch.fnmatchcase(excluded, pattern))
        self.assertEqual(command[command.index("--fail-under") + 1], "95")
        self.assertIn("--tests", command)


if __name__ == "__main__":
    unittest.main()
