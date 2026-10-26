"""Execute the NN38 native XOR example and a real checkpoint/stdin resume.

This is deliberately native-only. A successful receipt does not claim Rust
interchange with Python/TypeScript; that belongs to the next delivery tranche.
"""

from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def validate(manifest: Path) -> dict:
    """Use the requested trusted source manifest, fixed argv, and bounded runs."""
    cargo = shutil.which("cargo")
    if cargo is None:
        raise ValueError("Cargo is required for native Rust evidence")
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

    def execute(checkpoint: dict | None = None) -> dict:
        result = subprocess.run(  # nosec B603: fixed argv; manifest is trusted source
            command + (["--", "--resume"] if checkpoint is not None else []),
            input=json.dumps(checkpoint, allow_nan=False)
            if checkpoint is not None
            else "",
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
            timeout=120,
        )
        document = json.loads(result.stdout)
        if not isinstance(document, dict) or set(document) != {
            "checkpoint",
            "predictions",
            "initial_loss",
            "final_loss",
        }:
            raise ValueError("native Rust receipt fields differ")
        for name in ("initial_loss", "final_loss"):
            if type(document[name]) not in (int, float) or not math.isfinite(
                document[name]
            ):
                raise ValueError("native Rust loss is not finite")
        return document

    initial = execute()
    resumed = execute(initial["checkpoint"])
    final = initial["final_loss"]
    if not 0 <= final < 0.01:
        raise ValueError("native seeded Rust training did not learn XOR")
    if initial["checkpoint"]["step"] != 3000 or resumed["checkpoint"]["step"] != 3001:
        raise ValueError("native Rust optimizer step count differs")
    if abs(final - resumed["initial_loss"]) > 1e-9:
        raise ValueError("native checkpoint resume did not preserve the loss")
    if not 0 <= resumed["final_loss"] < 0.01:
        raise ValueError("native resumed Rust model lost XOR training")
    for receipt in (initial, resumed):
        predictions = receipt["predictions"]
        if not isinstance(predictions, list) or len(predictions) != 4:
            raise ValueError("native Rust prediction rows differ")
        for row, expected in zip(predictions, (0, 1, 1, 0), strict=True):
            if not isinstance(row, list) or len(row) != 1:
                raise ValueError("native Rust prediction columns differ")
            value = row[0]
            if type(value) not in (int, float) or not math.isfinite(value):
                raise ValueError("native Rust prediction is not finite")
            if abs(value - expected) > 0.1:
                raise ValueError("native Rust predictions do not represent XOR")
    return {
        "contract": "NN38",
        "native_lane": "rust",
        "xor_loss": final,
        "resumed_step": 3001,
        "native_checkpoint_resume": "passed",
        "cross_language_interchange": "pending-separate-tranche",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest-path", type=Path, default=ROOT / "code/packages/rust/Cargo.toml"
    )
    arguments = parser.parse_args()
    print(json.dumps(validate(arguments.manifest_path), allow_nan=False))
