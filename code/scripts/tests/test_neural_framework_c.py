"""Pin actual fixed child boundaries and reject full native state drift."""

import copy
import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "native_c", Path(__file__).resolve().parents[1] / "validate_neural_framework_c.py"
)
native_c = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(native_c)


def receipt(step):
    return {
        "checkpoint": {
            "format": "coding-adventures-dense",
            "version": 1,
            "step": step,
            "layers": [
                {
                    "activation": "tanh",
                    "weights": [[0.1] * 4 for _ in range(2)],
                    "biases": [0.1] * 4,
                },
                {
                    "activation": "sigmoid",
                    "weights": [[0.1] for _ in range(4)],
                    "biases": [0.1],
                },
            ],
        },
        "predictions": [[0.01], [0.99], [0.99], [0.01]],
        "initial_loss": 0.001,
        "final_loss": 0.001,
    }


def results():
    return [receipt(step) for step in (3000, 3001, 3001, 3002, 3002)]


class NativeC(unittest.TestCase):
    def invoke(self, receipts):
        with patch.object(
            native_c.subprocess,
            "run",
            side_effect=[
                subprocess.CompletedProcess([], 0, json.dumps(r)) for r in receipts
            ],
        ) as run:
            result = native_c.validate()
        return result, run

    def test_success_and_exact_fixed_process_contract(self):
        result, run = self.invoke(results())
        self.assertEqual(result["child_processes"], 5)
        self.assertEqual(result["binding_lanes"], 0)
        self.assertEqual(result["resumed_step"], 3002)
        for index, call in enumerate(run.call_args_list):
            flags = [
                [],
                ["--baseline-3001"],
                ["--resume"],
                ["--baseline-3002"],
                ["--resume"],
            ]
            self.assertEqual(
                call.args[0],
                [str(native_c.DEFAULT_EXECUTABLE.resolve())] + flags[index],
            )
            self.assertEqual(call.kwargs["cwd"], native_c.MODULE)
            self.assertTrue(call.kwargs["check"])
            self.assertEqual(call.kwargs["timeout"], 120)
            self.assertNotIn("shell", call.kwargs)
            if index in (2, 4):
                self.assertEqual(
                    json.loads(call.kwargs["input"])["step"],
                    3000 if index == 2 else 3001,
                )
            else:
                self.assertEqual(call.kwargs["input"], "")

    def test_child_failures_are_not_training_evidence(self):
        for error in (
            FileNotFoundError(),
            subprocess.CalledProcessError(1, []),
            subprocess.TimeoutExpired([], 120),
        ):
            with (
                self.subTest(error=type(error).__name__),
                patch.object(native_c.subprocess, "run", side_effect=error),
                self.assertRaises(type(error)),
            ):
                native_c.validate()

    def test_receipt_shapes_types_finiteness(self):
        mutations = [
            lambda r: r.update(extra=0),
            lambda r: r.update(initial_loss=True),
            lambda r: r.update(final_loss=float("nan")),
            lambda r: r.update(final_loss=-0.1),
            lambda r: r.update(final_loss=0.1),
            lambda r: r["checkpoint"].update(step=3000.0),
            lambda r: r["checkpoint"].update(version=True),
            lambda r: r["checkpoint"].update(format="wrong"),
            lambda r: r["checkpoint"].update(extra=1),
            lambda r: r["checkpoint"].update(layers=[]),
            lambda r: r["checkpoint"]["layers"][0].update(extra=1),
            lambda r: r["checkpoint"]["layers"][0].update(activation="linear"),
            lambda r: r["checkpoint"]["layers"][0].update(weights=[]),
            lambda r: r["checkpoint"]["layers"][0]["weights"][0].__setitem__(0, None),
            lambda r: r["checkpoint"]["layers"][0].update(biases=[True] * 4),
            lambda r: r.update(predictions=[]),
            lambda r: r["predictions"].__setitem__(0, []),
            lambda r: r["predictions"].__setitem__(0, [True]),
            lambda r: r["predictions"].__setitem__(0, [0.5]),
        ]
        for mutate in mutations:
            r = receipt(3000)
            mutate(r)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                native_c._receipt(json.dumps(r), 3000)
        for text in (
            "null",
            "[]",
            "{",
            '{"x":1,"x":2}',
            " " * (native_c.MAX_RECEIPT + 1),
        ):
            with self.assertRaises(ValueError):
                native_c._receipt(text, 3000)

    def test_every_resume_and_baseline_drift_is_rejected(self):
        for index in (1, 2, 3, 4):
            for field in ("weight", "bias", "prediction", "loss"):
                rs = results()
                r = rs[index]
                if field == "weight":
                    r["checkpoint"]["layers"][1]["weights"][3][0] += 0.01
                elif field == "bias":
                    r["checkpoint"]["layers"][0]["biases"][2] += 0.01
                elif field == "prediction":
                    r["predictions"][0][0] += 0.01
                else:
                    r["final_loss"] += 0.001
                with (
                    self.subTest(index=index, field=field),
                    self.assertRaises(ValueError),
                ):
                    self.invoke(rs)
        rs = results()
        rs[2]["initial_loss"] += 0.001
        with self.assertRaisesRegex(ValueError, "changed the loss"):
            self.invoke(rs)

    def test_recursive_comparison_is_closed(self):
        self.assertFalse(native_c._same([1], [1, 2]))
        self.assertFalse(native_c._same({"a": 1}, {"b": 1}))
        self.assertFalse(native_c._same(1, True))
        self.assertFalse(native_c._same(False, False))
        self.assertFalse(native_c._same("a", "b"))
        r = receipt(3000)
        self.assertTrue(native_c._same(r, copy.deepcopy(r)))


if __name__ == "__main__":
    unittest.main()
