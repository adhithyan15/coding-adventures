"""Execute native Python/TypeScript/Rust/Go training and exchange NN37 checkpoints.

This is runtime evidence, not a registry count: all four languages must actually
train, read the other one's parameters, and produce the same resumed result.
Run after installing the TypeScript neural-framework package's dependencies.
"""

from __future__ import annotations

import argparse
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
LANES = ("python", "typescript", "rust", "go")
GO = ROOT / "code/packages/go/neural-framework"
RUST_MANIFEST = ROOT / "code/packages/rust/Cargo.toml"


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
    elif type(expected) is not str or type(actual) is not str or actual != expected:
        raise ValueError("receipt metadata disagree")


def execute(
    language: str, checkpoint: dict | None = None, *, manifest: Path = RUST_MANIFEST
) -> dict:
    env = os.environ.copy()
    cwd = ROOT
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
    elif language == "rust":
        cargo = shutil.which("cargo")
        if cargo is None:
            raise ValueError("Cargo is required for the native Rust lane")
        command = [
            cargo,
            "run",
            "--quiet",
            "--manifest-path",
            str(manifest),
            "-p",
            "neural-framework",
            "--example",
            "xor",
        ]
        if checkpoint is not None:
            command.append("--")  # Cargo options end before example arguments.
    elif language == "go":
        go = shutil.which("go")
        if go is None:
            raise ValueError("Go is required for the native Go lane")
        # Module resolution belongs to this trusted source directory, not the
        # caller's cwd or any checkpoint field. Go passes following arguments
        # directly to the example; unlike Cargo it needs no extra separator.
        command = [go, "run", "./cmd/xor"]
        cwd = GO
    else:
        raise ValueError("unknown native framework lane")
    if checkpoint is not None:
        command.append("--resume")
    result = subprocess.run(
        command,
        input=None if checkpoint is None else json.dumps(checkpoint, allow_nan=False),
        text=True,
        capture_output=True,
        timeout=120,
        check=True,
        cwd=cwd,
        env=env,
        encoding="utf-8",
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
    step = 3000 if checkpoint is None else checkpoint["step"] + 1
    if (
        type(receipt["checkpoint"]) is not dict
        or type(receipt["checkpoint"].get("step")) is not int
        or receipt["checkpoint"]["step"] != step
    ):
        raise ValueError("native receipt has the wrong exact step count")
    for name in ("initial_loss", "final_loss"):
        if type(receipt[name]) not in (float, int) or receipt[name] < 0:
            raise ValueError("native loss must be a nonnegative number")
    if not receipt["final_loss"] < 0.01:
        raise ValueError("native model did not learn XOR")
    predictions = receipt["predictions"]
    if type(predictions) is not list or len(predictions) != 4:
        raise ValueError("native prediction rows differ")
    for row, target in zip(predictions, (0, 1, 1, 0), strict=True):
        if (
            type(row) is not list
            or len(row) != 1
            or type(row[0]) not in (float, int)
            or abs(row[0] - target) > 0.1
        ):
            raise ValueError("native predictions do not represent XOR")
    return receipt


def validate(manifest: Path = RUST_MANIFEST) -> dict:
    """Run four independent models, then twelve actual directed exchanges."""
    starts = {lane: execute(lane, manifest=manifest) for lane in LANES}
    reference = starts["python"]
    baselines = {}
    for lane in LANES:
        equivalent(starts[lane], reference)
        one = execute(lane, starts[lane]["checkpoint"], manifest=manifest)
        two = execute(lane, one["checkpoint"], manifest=manifest)
        equivalent(one["initial_loss"], starts[lane]["final_loss"])
        equivalent(two["initial_loss"], one["final_loss"])
        baselines[lane] = (one, two)
    for lane in LANES:
        equivalent(baselines[lane][0], baselines["python"][0])
        equivalent(baselines[lane][1], baselines["python"][1])
    exchanges = []
    for source in LANES:
        for destination in LANES:
            if source == destination:
                continue
            resumed = execute(
                destination, starts[source]["checkpoint"], manifest=manifest
            )
            equivalent(resumed, baselines[source][0])
            returned = execute(source, resumed["checkpoint"], manifest=manifest)
            equivalent(returned, baselines[source][1])
            exchanges.append(f"{source}->{destination}")
    return {
        "contract": "NN42",
        "native_lanes": list(LANES),
        "binding_lanes": [],
        "xor_loss": reference["final_loss"],
        "resumed_step": 3002,
        "checkpoint_exchange": "passed-all-twelve-directions",
        "exchanges": exchanges,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-manifest-path", type=Path, default=RUST_MANIFEST)
    print(json.dumps(validate(parser.parse_args().rust_manifest_path), allow_nan=False))
