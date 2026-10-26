"""NN48: five actual C children compare fresh baselines and native resumes.

The executable path is trusted build configuration, never checkpoint content.
No shell, compiler discovery or compilation belongs here: the shared ISO
harness supplies the binary. Six-native interchange is a separate tranche.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "code/packages/c/neural-framework"
DEFAULT_EXECUTABLE = (
    MODULE / "_build" / ("native-xor.exe" if os.name == "nt" else "native-xor")
)
MAX_RECEIPT = 4 * 1024 * 1024


def _number(value):
    return type(value) in (int, float) and math.isfinite(value)


def _closed(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate native C receipt field")
        result[key] = value
    return result


def _same(a, b):
    """Full shape/type/value comparison; booleans cannot earn numeric credit."""
    if isinstance(a, dict):
        return (
            isinstance(b, dict)
            and a.keys() == b.keys()
            and all(_same(a[key], b[key]) for key in a)
        )
    if isinstance(a, list):
        return (
            isinstance(b, list)
            and len(a) == len(b)
            and all(_same(x, y) for x, y in zip(a, b))
        )
    if _number(a):
        return _number(b) and abs(a - b) <= 1e-9
    return type(a) is str and type(b) is str and a == b


def _receipt(text, expected):
    if len(text.encode("utf-8")) > MAX_RECEIPT:
        raise ValueError("native C receipt exceeds byte envelope")
    receipt = json.loads(text, object_pairs_hook=_closed)
    if not isinstance(receipt, dict) or set(receipt) != {
        "checkpoint",
        "predictions",
        "initial_loss",
        "final_loss",
    }:
        raise ValueError("native C receipt fields differ")
    for name in ("initial_loss", "final_loss"):
        if not _number(receipt[name]) or receipt[name] < 0:
            raise ValueError("native C loss is not finite and nonnegative")
    if receipt["final_loss"] >= 0.01:
        raise ValueError("native C did not learn XOR")
    cp = receipt["checkpoint"]
    if not isinstance(cp, dict) or set(cp) != {"format", "version", "step", "layers"}:
        raise ValueError("native C checkpoint fields differ")
    if (
        cp["format"] != "coding-adventures-dense"
        or type(cp["version"]) is not int
        or cp["version"] != 1
        or type(cp["step"]) is not int
        or cp["step"] != expected
    ):
        raise ValueError("native C checkpoint metadata differs")
    if not isinstance(cp["layers"], list) or len(cp["layers"]) != 2:
        raise ValueError("native C layer count differs")
    for layer, rows, cols, activation in zip(
        cp["layers"], (2, 4), (4, 1), ("tanh", "sigmoid")
    ):
        if not isinstance(layer, dict) or set(layer) != {
            "activation",
            "weights",
            "biases",
        }:
            raise ValueError("native C layer fields differ")
        if layer["activation"] != activation:
            raise ValueError("native C activation differs")
        weights, biases = layer["weights"], layer["biases"]
        if (
            not isinstance(weights, list)
            or len(weights) != rows
            or not isinstance(biases, list)
            or len(biases) != cols
            or not all(_number(v) for v in biases)
        ):
            raise ValueError("native C parameter shapes/numbers differ")
        for row in weights:
            if (
                not isinstance(row, list)
                or len(row) != cols
                or not all(_number(v) for v in row)
            ):
                raise ValueError("native C weight row differs")
    predictions = receipt["predictions"]
    if not isinstance(predictions, list) or len(predictions) != 4:
        raise ValueError("native C prediction rows differ")
    for row, target in zip(predictions, (0, 1, 1, 0)):
        if (
            not isinstance(row, list)
            or len(row) != 1
            or not _number(row[0])
            or abs(row[0] - target) > 0.1
        ):
            raise ValueError("native C prediction does not represent XOR")
    return receipt


def validate(executable: Path = DEFAULT_EXECUTABLE) -> dict:
    command = [str(executable.resolve())]

    def execute(flag, step, checkpoint=None):
        result = subprocess.run(  # nosec B603: trusted build path and fixed argv
            command + ([flag] if flag else []),
            cwd=MODULE,
            input=json.dumps(checkpoint, allow_nan=False)
            if checkpoint is not None
            else "",
            text=True,
            encoding="utf-8",
            capture_output=True,
            check=True,
            timeout=120,
        )
        return _receipt(result.stdout, step)

    first = execute(None, 3000)
    previous = first
    for step in (3001, 3002):
        baseline = execute(f"--baseline-{step}", step)
        resumed = execute("--resume", step, previous["checkpoint"])
        if abs(previous["final_loss"] - resumed["initial_loss"]) > 1e-9:
            raise ValueError("native C resume changed the loss")
        for field in ("checkpoint", "predictions", "final_loss"):
            if not _same(resumed[field], baseline[field]):
                raise ValueError(f"native C full baseline/resume drift: {field}")
        previous = resumed
    return {
        "contract": "NN48",
        "native_lane": "c",
        "binding_lanes": 0,
        "child_processes": 5,
        "xor_loss": first["final_loss"],
        "resumed_step": previous["checkpoint"]["step"],
        "native_checkpoint_resume": "passed",
        "six_language_interchange": "pending-separate-tranche",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--executable",
        type=Path,
        default=DEFAULT_EXECUTABLE,
        help="trusted harness-built native C binary, not checkpoint data",
    )
    print(json.dumps(validate(parser.parse_args().executable), allow_nan=False))


if __name__ == "__main__":
    main()
