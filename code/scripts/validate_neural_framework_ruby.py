"""NN45 executes independent Ruby training and two bounded checkpoint resumes."""

from __future__ import annotations

import json
import math
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "code/packages/ruby/neural-framework"


def validate() -> dict:
    ruby = shutil.which("ruby")
    if ruby is None:
        raise ValueError("Ruby is required for native training evidence")
    command = [
        ruby,
        "-I",
        str(ROOT / "code/packages/ruby/matrix/lib"),
        "-I",
        str(MODULE / "lib"),
        str(MODULE / "examples/xor.rb"),
    ]

    def execute(checkpoint=None):
        result = subprocess.run(  # nosec B603: fixed source, argv, and cwd
            command + (["--resume"] if checkpoint is not None else []),
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
        receipt = json.loads(result.stdout)
        if not isinstance(receipt, dict) or set(receipt) != {
            "checkpoint",
            "predictions",
            "initial_loss",
            "final_loss",
        }:
            raise ValueError("native Ruby receipt fields differ")
        for key in ("initial_loss", "final_loss"):
            value = receipt[key]
            if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
                raise ValueError("native Ruby loss is not finite and nonnegative")
        cp = receipt["checkpoint"]
        expected = 3000 if checkpoint is None else checkpoint["step"] + 1
        if (
            not isinstance(cp, dict)
            or type(cp.get("step")) is not int
            or cp["step"] != expected
        ):
            raise ValueError("native Ruby step differs")
        if receipt["final_loss"] >= 0.01:
            raise ValueError("native Ruby did not learn XOR")
        predictions = receipt["predictions"]
        if not isinstance(predictions, list) or len(predictions) != 4:
            raise ValueError("native Ruby prediction rows differ")
        for row, expected in zip(predictions, (0, 1, 1, 0), strict=True):
            if not isinstance(row, list) or len(row) != 1:
                raise ValueError("native Ruby prediction columns differ")
            value = row[0]
            if (
                type(value) not in (int, float)
                or not math.isfinite(value)
                or abs(value - expected) > 0.1
            ):
                raise ValueError("native Ruby prediction does not represent XOR")
        return receipt

    first = execute()
    previous = first
    for _ in range(2):
        resumed = execute(previous["checkpoint"])
        if abs(previous["final_loss"] - resumed["initial_loss"]) > 1e-9:
            raise ValueError("native Ruby resume changed the loss")
        previous = resumed
    return {
        "contract": "NN45",
        "native_lane": "ruby",
        "xor_loss": first["final_loss"],
        "resumed_step": previous["checkpoint"]["step"],
        "native_checkpoint_resume": "passed",
        "five_language_interchange": "pending-separate-tranche",
    }


if __name__ == "__main__":
    print(json.dumps(validate()))
