"""Command trust boundaries supplement, not replace, actual native execution."""

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "ruby_validator",
    Path(__file__).resolve().parents[1] / "validate_neural_framework_ruby.py",
)
V = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(V)


def receipt(step=3000):
    return {
        "checkpoint": {"step": step},
        "predictions": [[0], [1], [1], [0]],
        "initial_loss": 0.001,
        "final_loss": 0.001,
    }


class RubyValidatorTest(unittest.TestCase):
    def execute(self, receipts):
        with (
            patch.object(V.shutil, "which", return_value="trusted-ruby"),
            patch.object(
                V.subprocess,
                "run",
                side_effect=[
                    subprocess.CompletedProcess([], 0, json.dumps(r)) for r in receipts
                ],
            ) as run,
        ):
            result = V.validate()
            return result, run.call_args_list

    def test_fixed_commands_and_resume(self):
        result, calls = self.execute([receipt(3000), receipt(3001), receipt(3002)])
        self.assertEqual(result["resumed_step"], 3002)
        self.assertEqual(result["native_lane"], "ruby")
        for index, call in enumerate(calls):
            expected = [
                "trusted-ruby",
                "-I",
                str(V.ROOT / "code/packages/ruby/matrix/lib"),
                "-I",
                str(V.MODULE / "lib"),
                str(V.MODULE / "examples/xor.rb"),
            ]
            self.assertEqual(call.args[0], expected + (["--resume"] if index else []))
            self.assertEqual(call.kwargs["cwd"], V.MODULE)
            self.assertEqual(call.kwargs["timeout"], 120)
            self.assertTrue(call.kwargs["check"])
            self.assertNotIn("shell", call.kwargs)
            self.assertEqual(
                call.kwargs["input"],
                json.dumps({"step": 2999 + index}) if index else "",
            )

    def test_missing_tool_and_child_failures(self):
        with (
            patch.object(V.shutil, "which", return_value=None),
            self.assertRaises(ValueError),
        ):
            V.validate()
        for error in (
            subprocess.CalledProcessError(1, []),
            subprocess.TimeoutExpired([], 120),
        ):
            with (
                patch.object(V.shutil, "which", return_value="ruby"),
                patch.object(V.subprocess, "run", side_effect=error),
                self.assertRaises(type(error)),
            ):
                V.validate()
        with (
            patch.object(V.shutil, "which", return_value="ruby"),
            patch.object(
                V.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0, "{"),
            ),
            self.assertRaises(ValueError),
        ):
            V.validate()

    def test_receipt_rejection(self):
        bad = [[], {"extra": 1}]
        for key, values in {
            "initial_loss": [True, float("inf"), -1],
            "final_loss": [True, float("nan"), 0.1],
            "checkpoint": [[], {"step": True}, {"step": 3001}],
            "predictions": [
                [],
                [[0, 0]] * 4,
                [[True], [1], [1], [0]],
                [[float("inf")]] * 4,
                [[1]] * 4,
            ],
        }.items():
            for value in values:
                modified = copy.deepcopy(receipt())
                modified[key] = value
                bad.append(modified)
        for value in bad:
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.execute([value])

    def test_resume_loss_drift(self):
        second = receipt(3001)
        second["initial_loss"] = 0.002
        with self.assertRaises(ValueError):
            self.execute([receipt(), second])


if __name__ == "__main__":
    unittest.main()
