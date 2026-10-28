"""Exercise each real native command, including full-state resume equality.

The executable path is supplied by the trusted build front. Checkpoint bytes
are stdin data. They never participate in executable/argument/cwd selection.
"""

import copy
import json
import math
import subprocess
import sys
from pathlib import Path


def validate_receipt(receipt):
    assert type(receipt) is dict
    assert set(receipt) == {
        "contract",
        "implementation",
        "bindings",
        "seven_native_exchange",
        "checkpoint",
        "predictions",
        "initial_loss",
        "final_loss",
        "losses",
    }
    assert receipt["contract"] == "NN51"
    assert receipt["implementation"] == "native-cpp"
    assert type(receipt["bindings"]) is int and receipt["bindings"] == 0
    assert receipt["seven_native_exchange"] == "pending"
    checkpoint = receipt["checkpoint"]
    assert type(checkpoint) is dict and set(checkpoint) == {
        "format",
        "version",
        "step",
        "layers",
    }
    assert checkpoint["format"] == "coding-adventures-dense"
    assert type(checkpoint["version"]) is int and checkpoint["version"] == 1
    assert (
        type(checkpoint["step"]) is int and 0 <= checkpoint["step"] <= 9007199254740991
    )
    predictions = receipt["predictions"]
    assert type(predictions) is list and len(predictions) == 4
    assert all(type(row) is list and len(row) == 1 for row in predictions)
    assert all(
        type(row[0]) in (int, float) and math.isfinite(row[0]) for row in predictions
    )
    loss = (
        sum((row[0] - target) ** 2 for row, target in zip(predictions, [0, 1, 1, 0]))
        / 4
    )
    assert all(
        type(receipt[k]) in (int, float) and math.isfinite(receipt[k])
        for k in ("initial_loss", "final_loss")
    )
    assert abs(loss - receipt["final_loss"]) < 1e-12
    losses = receipt["losses"]
    assert type(losses) is list and losses
    assert all(type(v) in (int, float) and math.isfinite(v) for v in losses)
    assert losses[0] == receipt["initial_loss"] and losses[-1] == receipt["final_loss"]
    return receipt


def run(executable, args=(), checkpoint=None):
    result = subprocess.run(
        [str(executable), *args],
        input=None if checkpoint is None else json.dumps(checkpoint),
        text=True,
        encoding="utf-8",
        capture_output=True,
        timeout=120,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    return validate_receipt(json.loads(result.stdout))


def reject_mutated_receipts(receipt):
    # Falsify the validator itself: a successful child exit cannot replace
    # closed provenance, finite data, shape and independently recomputed loss.
    mutations = [
        ("contract", "NN49"),
        ("implementation", "binding"),
        ("bindings", False),
        ("seven_native_exchange", "complete"),
        ("extra", 1),
        ("predictions", [[0]]),
        ("predictions", [[True]] * 4),
        ("predictions", [[float("nan")]] * 4),
        ("predictions", [None] * 4),
        ("final_loss", float("inf")),
        ("final_loss", 42),
        ("initial_loss", True),
        ("losses", []),
        ("losses", [float("nan")]),
        ("checkpoint", dict(receipt["checkpoint"], version=True)),
        ("checkpoint", dict(receipt["checkpoint"], step=1.0)),
        ("checkpoint", dict(receipt["checkpoint"], step=9007199254740992)),
    ]
    for key, value in mutations:
        bad = copy.deepcopy(receipt)
        bad[key] = value
        try:
            validate_receipt(bad)
        except AssertionError:
            continue
        raise AssertionError(f"validator admitted mutated {key}")


def main():
    executable = Path(sys.argv[1]).resolve(strict=True)
    fresh = run(executable)
    reject_mutated_receipts(fresh)
    assert fresh["checkpoint"]["step"] == 3000 and len(fresh["losses"]) == 3001
    assert fresh["final_loss"] < 0.01
    current = fresh
    for step in (3001, 3002):
        resumed = run(executable, ("--resume",), current["checkpoint"])
        baseline = run(executable, (f"--baseline-{step}",))
        assert resumed["checkpoint"] == baseline["checkpoint"]
        assert resumed["predictions"] == baseline["predictions"]
        assert resumed["final_loss"] == baseline["final_loss"]
        assert resumed["initial_loss"] == current["final_loss"]
        assert len(resumed["losses"]) == 2 and resumed["checkpoint"]["step"] == step
        current = resumed
    # Every invalid command must produce a nonzero exit and no success receipt.
    bad_model = dict(fresh["checkpoint"])
    bad_model["layers"] = [{"activation": "linear", "weights": [[1]], "biases": [0]}]
    exhausted = dict(fresh["checkpoint"], step=9007199254740991)
    wrong_output = dict(
        fresh["checkpoint"],
        layers=[
            {"activation": "linear", "weights": [[1, 2], [3, 4]], "biases": [0, 0]}
        ],
    )
    overflow = dict(
        fresh["checkpoint"],
        layers=[{"activation": "linear", "weights": [[1e308], [1e308]], "biases": [0]}],
    )
    for args, text in [
        (("extra",), ""),
        (("--resume", "extra"), "{}"),
        (("--resume",), "["),
        (("--resume",), "null"),
        (("--resume",), json.dumps(bad_model)),
        (("--resume",), json.dumps(exhausted)),
        (("--resume",), json.dumps(wrong_output)),
        (("--resume",), json.dumps(overflow)),
        (("--resume",), " " * (4194304 + 1)),
    ]:
        result = subprocess.run(
            [str(executable), *args],
            input=text,
            text=True,
            encoding="utf-8",
            capture_output=True,
            timeout=30,
            check=False,
        )
        assert result.returncode != 0 and not result.stdout and result.stderr
    print(
        f"NN51 native command: fresh/resume/full states through step 3002, loss={current['final_loss']:.17g}; seven-native exchange pending"
    )


if __name__ == "__main__":
    main()
