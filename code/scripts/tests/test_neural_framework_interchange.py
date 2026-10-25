"""NN42 orchestration regressions; real runtime evidence is a separate command."""

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = (
    Path(__file__).resolve().parents[1] / "validate_neural_framework_interchange.py"
)
SPEC = importlib.util.spec_from_file_location("nn39", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def receipt(step=3000):
    return {
        "checkpoint": {
            "format": "coding-adventures-dense",
            "version": 1,
            "step": step,
            "layers": [{"weights": [[0.5]], "biases": [0.0], "activation": "linear"}],
        },
        "predictions": [[0.01], [0.99], [0.99], [0.01]],
        "initial_loss": 0.0001,
        "final_loss": 0.0001,
    }


class InterchangeTests(unittest.TestCase):
    def test_all_twelve_directed_pairs_and_native_baselines_execute(self):
        calls = []

        def execute(lane, checkpoint=None, **kwargs):
            calls.append((lane, None if checkpoint is None else checkpoint["step"]))
            return receipt(3000 if checkpoint is None else checkpoint["step"] + 1)

        with patch.object(MODULE, "execute", side_effect=execute):
            result = MODULE.validate()
        self.assertEqual(len(calls), 36)  # 4 train + 8 native + 24 cross-runtime.
        self.assertEqual(result["contract"], "NN42")
        self.assertEqual(result["checkpoint_exchange"], "passed-all-twelve-directions")
        self.assertEqual(result["binding_lanes"], [])
        self.assertEqual(result["native_lanes"], ["python", "typescript", "rust", "go"])
        self.assertEqual(len(result["exchanges"]), 12)
        self.assertEqual([step for lane, step in calls[:4]], [None] * 4)
        self.assertEqual(
            set(result["exchanges"]),
            {f"{a}->{b}" for a in MODULE.LANES for b in MODULE.LANES if a != b},
        )
        self.assertEqual(result["resumed_step"], 3002)

    def test_fixed_argv_stdin_timeout_and_cargo_separator(self):
        for lane in MODULE.LANES:
            for resume in (False, True):
                with self.subTest(lane=lane, resume=resume):
                    checkpoint = receipt()["checkpoint"] if resume else None
                    child = subprocess.CompletedProcess(
                        [], 0, json.dumps(receipt(3001 if resume else 3000))
                    )
                    with (
                        patch.object(
                            MODULE.shutil, "which", return_value="trusted tool"
                        ),
                        patch.object(
                            MODULE.subprocess, "run", return_value=child
                        ) as run,
                    ):
                        MODULE.execute(
                            lane, checkpoint, manifest=Path("trusted path/Cargo.toml")
                        )
                    argv = run.call_args.args[0]
                    options = run.call_args.kwargs
                    self.assertNotIn("shell", options)
                    self.assertTrue(options["check"])
                    self.assertEqual(options["timeout"], 120)
                    self.assertEqual(options["encoding"], "utf-8")
                    self.assertEqual(
                        options["cwd"], MODULE.GO if lane == "go" else MODULE.ROOT
                    )
                    self.assertEqual(
                        options["input"], json.dumps(checkpoint) if resume else None
                    )
                    if lane == "rust":
                        self.assertIn(str(Path("trusted path/Cargo.toml")), argv)
                        self.assertEqual(
                            argv[-2:] if resume else argv[-1:],
                            ["--", "--resume"] if resume else ["xor"],
                        )
                    if lane == "go":
                        self.assertEqual(
                            argv,
                            ["trusted tool", "run", "./cmd/xor"]
                            + (["--resume"] if resume else []),
                        )

    def test_unknown_lane_missing_tools_and_process_errors_fail(self):
        with self.assertRaises(ValueError):
            MODULE.execute("binding")
        for lane in ("rust", "typescript", "go"):
            with (
                patch.object(MODULE.shutil, "which", return_value=None),
                self.assertRaises(ValueError),
            ):
                MODULE.execute(lane)
        for error in (
            subprocess.TimeoutExpired("tool", 120),
            subprocess.CalledProcessError(1, "tool"),
        ):
            for lane in MODULE.LANES:
                with (
                    patch.object(MODULE.shutil, "which", return_value="trusted tool"),
                    patch.object(MODULE.subprocess, "run", side_effect=error),
                    self.assertRaises(type(error)),
                ):
                    MODULE.execute(lane)

    def test_malformed_nonfinite_boolean_shapes_and_counts_fail(self):
        bad = [[], {}, {**receipt(), "extra": 1}]
        for field, value in (
            ("initial_loss", True),
            ("final_loss", float("nan")),
            ("final_loss", -1),
            ("final_loss", 0.1),
            ("checkpoint", []),
            ("checkpoint", {"step": 3001}),
            ("checkpoint", {"step": True}),
            ("predictions", []),
            ("predictions", [[True]] * 4),
            ("predictions", [[None]] * 4),
            ("predictions", [[1, 2]] * 4),
            ("predictions", [[0]] * 4),
        ):
            document = receipt()
            document[field] = value
            bad.append(document)
        for document in bad:
            with (
                self.subTest(document=document),
                patch.object(
                    MODULE.subprocess,
                    "run",
                    return_value=subprocess.CompletedProcess(
                        [], 0, json.dumps(document)
                    ),
                ),
                self.assertRaises(ValueError),
            ):
                MODULE.execute("python")
        with (
            patch.object(
                MODULE.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], 0, "{"),
            ),
            self.assertRaises(ValueError),
        ):
            MODULE.execute("python")

    def test_full_parameter_and_resume_drift_fail(self):
        for failure in ("initial", "baseline", "exchange"):
            count = 0

            def execute(lane, checkpoint=None, failure=failure, **kwargs):
                nonlocal count
                count += 1
                value = copy.deepcopy(
                    receipt(3000 if checkpoint is None else checkpoint["step"] + 1)
                )
                if count == {"initial": 2, "baseline": 5, "exchange": 10}[failure]:
                    value["checkpoint"]["layers"][0]["weights"][0][0] += 0.01
                return value

            with (
                self.subTest(failure=failure),
                patch.object(MODULE, "execute", side_effect=execute),
                self.assertRaises(ValueError),
            ):
                MODULE.validate()
        for left, right in (
            (True, 1),
            (1, True),
            ("x", "y"),
            ([1], [1, 2]),
            ({"x": 1}, {}),
            (float("inf"), 1),
        ):
            with self.assertRaises(ValueError):
                MODULE.equivalent(left, right)

    def test_go_fresh_baseline_destination_and_return_drift_fail(self):
        # Call positions pin each boundary independently of the lane registry:
        # Go fresh=4; Go native resumes=11/12; Python->Go=17; Go->Python=18.
        for damaged in (4, 11, 12, 17, 18):
            calls = 0

            def execute(lane, checkpoint=None, damaged=damaged, **kwargs):
                nonlocal calls
                calls += 1
                value = receipt(3000 if checkpoint is None else checkpoint["step"] + 1)
                if calls == damaged:
                    value["checkpoint"]["layers"][0]["biases"][0] += 0.01
                return value

            with (
                self.subTest(damaged=damaged),
                patch.object(MODULE, "execute", side_effect=execute),
                self.assertRaises(ValueError),
            ):
                MODULE.validate()
