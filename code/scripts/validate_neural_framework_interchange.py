"""Execute native Python/TypeScript training and exchange NN37 checkpoints.

This is runtime evidence, not a registry count: both languages must actually
train, read the other one's parameters, and produce the same resumed result.
Run after installing the TypeScript neural-framework package's dependencies.
"""

from __future__ import annotations

import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYTHON = ROOT / "code/packages/python/neural-framework"
TYPESCRIPT = ROOT / "code/packages/typescript/neural-framework"
TOLERANCE = 1e-9


def equivalent(actual: object, expected: object) -> None:
    """Compare data structurally; numeric tolerance is fixed by NN37."""
    if type(expected) in (float, int):
        if type(actual) not in (float, int) or not math.isfinite(actual):
            raise ValueError("receipt contains a non-finite or nonnumeric value")
        if abs(actual - expected) > TOLERANCE:
            raise ValueError(f"native results disagree: {actual!r} versus {expected!r}")
    elif isinstance(expected, dict):
        if not isinstance(actual, dict) or actual.keys() != expected.keys():
            raise ValueError("receipt object fields disagree")
        for key in expected:
            equivalent(actual[key], expected[key])
    elif isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            raise ValueError("receipt array lengths disagree")
        for left, right in zip(actual, expected, strict=True):
            equivalent(left, right)
    elif actual != expected:
        raise ValueError("receipt metadata disagree")


def execute(language: str, checkpoint: dict | None = None) -> dict:
    env = os.environ.copy()
    env["PYTHONPATH"] = os.pathsep.join(
        [
            str(PYTHON / "src"),
            str(ROOT / "code/packages/python/matrix/src"),
        ]
    )
    if language == "python":
        command = [sys.executable, str(PYTHON / "examples/xor.py")]
    elif language == "typescript":
        node = shutil.which("node")
        if node is None:
            raise ValueError("Node.js is required for the native TypeScript lane")
        command = [
            node,
            str(TYPESCRIPT / "node_modules/tsx/dist/cli.mjs"),
            str(TYPESCRIPT / "examples/xor.ts"),
        ]
    else:
        raise ValueError("unknown native framework lane")
    if checkpoint is not None:
        command.append("--resume")
    result = subprocess.run(
        command,
        input=None if checkpoint is None else json.dumps(checkpoint, allow_nan=False),
        text=True,
        capture_output=True,
        timeout=60,
        check=True,
        cwd=ROOT,
        env=env,
    )
    receipt = json.loads(result.stdout)
    if type(receipt) is not dict or set(receipt) != {
        "checkpoint",
        "predictions",
        "initial_loss",
        "final_loss",
    }:
        raise ValueError("invalid native execution receipt")
    equivalent(
        receipt, receipt
    )  # Every numeric leaf must be finite, even before comparison.
    return receipt


def main() -> None:
    python = execute("python")
    typescript = execute("typescript")
    equivalent(typescript, python)
    if python["checkpoint"]["step"] != 3000 or not python["final_loss"] < 0.01:
        raise ValueError("seeded XOR training did not satisfy the contract")
    baseline_one = execute("python", python["checkpoint"])
    baseline_two = execute("python", baseline_one["checkpoint"])
    for start, first, second in (
        (python, "typescript", "python"),
        (typescript, "python", "typescript"),
    ):
        resumed = execute(first, start["checkpoint"])
        equivalent(resumed, baseline_one)
        completed = execute(second, resumed["checkpoint"])
        equivalent(completed, baseline_two)
        if completed["checkpoint"]["step"] != 3002:
            raise ValueError(
                "checkpoint resume did not preserve exact optimizer step count"
            )
    print(
        json.dumps(
            {
                "contract": "NN37",
                "native_lanes": ["python", "typescript"],
                "xor_loss": python["final_loss"],
                "resumed_step": 3002,
                "checkpoint_exchange": "passed-both-directions",
            },
            allow_nan=False,
        )
    )


if __name__ == "__main__":
    main()
