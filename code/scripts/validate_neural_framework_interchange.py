"""Execute native Python/TypeScript/Rust/Go/Ruby/C training and exchange checkpoints.

This is runtime evidence, not a registry count: all six languages must actually
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
LANES = ("python", "typescript", "rust", "go", "ruby", "c")
GO = ROOT / "code/packages/go/neural-framework"
RUBY = ROOT / "code/packages/ruby/neural-framework"
RUST_MANIFEST = ROOT / "code/packages/rust/Cargo.toml"
C = ROOT / "code/packages/c/neural-framework"
C_EXECUTABLE = (
    C / "_build" / ("gcc-native-xor.exe" if os.name == "nt" else "gcc-native-xor")
)


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
    language: str,
    checkpoint: dict | None = None,
    *,
    manifest: Path = RUST_MANIFEST,
    c_executable: Path = C_EXECUTABLE,
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
    elif language == "ruby":
        ruby = shutil.which("ruby")
        if ruby is None:
            raise ValueError("Ruby is required for the native Ruby lane")
        # Fixed native library load paths; checkpoints supply JSON values only.
        command = [
            ruby,
            "-I",
            str(ROOT / "code/packages/ruby/matrix/lib"),
            "-I",
            str(RUBY / "lib"),
            str(RUBY / "examples/xor.rb"),
        ]
        cwd = RUBY
    elif language == "c":
        # Compiler scheduling belongs to the existing ISO harness. This path
        # comes from trusted build configuration, never the transported model.
        executable = c_executable.resolve()
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ValueError("a harness-built native C executable is required")
        command = [str(executable)]
        cwd = C
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


def validate(manifest: Path = RUST_MANIFEST, c_executable: Path = C_EXECUTABLE) -> dict:
    """Run six models, twelve baselines and thirty directed return trips."""
    configuration = {"manifest": manifest, "c_executable": c_executable}
    starts = {lane: execute(lane, **configuration) for lane in LANES}
    reference = starts["python"]
    baselines = {}
    for lane in LANES:
        equivalent(starts[lane], reference)
        one = execute(lane, starts[lane]["checkpoint"], **configuration)
        two = execute(lane, one["checkpoint"], **configuration)
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
                destination, starts[source]["checkpoint"], **configuration
            )
            equivalent(resumed, baselines[source][0])
            returned = execute(source, resumed["checkpoint"], **configuration)
            equivalent(returned, baselines[source][1])
            exchanges.append(f"{source}->{destination}")
    return {
        "contract": "NN49",
        "child_processes": 3 * len(LANES) + 2 * len(exchanges),
        "native_lanes": list(LANES),
        "binding_lanes": [],
        "xor_loss": reference["final_loss"],
        "resumed_step": 3002,
        "checkpoint_exchange": "passed-all-thirty-directions",
        "exchanges": exchanges,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust-manifest-path", type=Path, default=RUST_MANIFEST)
    parser.add_argument(
        "--c-executable",
        type=Path,
        default=C_EXECUTABLE,
        help="trusted ISO-harness binary, never checkpoint content",
    )
    options = parser.parse_args()
    print(
        json.dumps(
            validate(options.rust_manifest_path, options.c_executable), allow_nan=False
        )
    )
