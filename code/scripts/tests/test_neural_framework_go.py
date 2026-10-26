"""Pin shell-free native Go receipt admission; mocks are not runtime evidence."""

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "native_go", Path(__file__).resolve().parents[1] / "validate_neural_framework_go.py"
)
GO = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GO)


def receipt(step=3000):
    return {
        "checkpoint": {"step": step},
        "predictions": [[0], [1], [1], [0]],
        "initial_loss": 0.001,
        "final_loss": 0.001,
    }


class NativeGoTests(unittest.TestCase):
    def test_actual_argv_and_two_resumes(self):
        def run(command, **kwargs):
            self.assertEqual(command[:3], ["trusted go", "run", "./cmd/xor"])
            self.assertEqual(kwargs["cwd"], GO.MODULE)
            self.assertEqual(kwargs["timeout"], 120)
            self.assertTrue(kwargs["check"])
            self.assertNotIn("shell", kwargs)
            self.assertEqual(kwargs["encoding"], "utf-8")
            if kwargs["input"]:
                self.assertEqual(command[3:], ["--resume"])
                step = json.loads(kwargs["input"])["step"] + 1
            else:
                self.assertEqual(command[3:], [])
                step = 3000
            return subprocess.CompletedProcess(command, 0, json.dumps(receipt(step)))

        with (
            patch.object(GO.shutil, "which", return_value="trusted go"),
            patch.object(GO.subprocess, "run", side_effect=run) as child,
        ):
            result = GO.validate()
        self.assertEqual(child.call_count, 3)
        self.assertEqual(result["resumed_step"], 3002)
        self.assertEqual(result["native_lane"], "go")
        self.assertEqual(
            result["four_language_interchange"], "pending-separate-tranche"
        )

    def test_tools_and_child_failures(self):
        with (
            patch.object(GO.shutil, "which", return_value=None),
            self.assertRaises(ValueError),
        ):
            GO.validate()
        for error in (
            subprocess.TimeoutExpired("go", 120),
            subprocess.CalledProcessError(1, "go"),
        ):
            with (
                patch.object(GO.shutil, "which", return_value="go"),
                patch.object(GO.subprocess, "run", side_effect=error),
                self.assertRaises(type(error)),
            ):
                GO.validate()

    def test_malformed_receipts_fail_closed(self):
        bad = [None, [], {}, {**receipt(), "extra": 0}]
        for key in ("initial_loss", "final_loss"):
            for value in (True, None, "0", -1, float("nan"), float("inf")):
                bad.append({**receipt(), key: value})
        bad.append({**receipt(), "final_loss": 1})
        for cp in (None, [], {}, {"step": True}, {"step": 3001}, {"step": 3000.0}):
            bad.append({**receipt(), "checkpoint": cp})
        for predictions in (
            None,
            [],
            [[0]],
            [[0, 0], [1], [1], [0]],
            [[True], [1], [1], [0]],
            [["0"], [1], [1], [0]],
            [[float("nan")], [1], [1], [0]],
            [[1], [1], [1], [0]],
        ):
            bad.append({**receipt(), "predictions": predictions})
        for document in bad:
            with (
                self.subTest(document=document),
                patch.object(GO.shutil, "which", return_value="go"),
                patch.object(
                    GO.subprocess,
                    "run",
                    return_value=subprocess.CompletedProcess(
                        [], 0, json.dumps(document)
                    ),
                ),
                self.assertRaises(ValueError),
            ):
                GO.validate()
        with (
            patch.object(GO.shutil, "which", return_value="go"),
            patch.object(
                GO.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0, "{"),
            ),
            self.assertRaises(ValueError),
        ):
            GO.validate()

    def test_loss_continuity_is_required(self):
        changed = copy.deepcopy(receipt(3001))
        changed["initial_loss"] = 0.002
        outputs = [receipt(), changed]
        with (
            patch.object(GO.shutil, "which", return_value="go"),
            patch.object(
                GO.subprocess,
                "run",
                side_effect=[
                    subprocess.CompletedProcess([], 0, json.dumps(o)) for o in outputs
                ],
            ),
            self.assertRaisesRegex(ValueError, "resume changed"),
        ):
            GO.validate()


if __name__ == "__main__":
    unittest.main()
