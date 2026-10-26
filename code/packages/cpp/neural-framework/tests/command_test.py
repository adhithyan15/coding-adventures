"""Exercise each real native command, including full-state resume equality.

The executable path is supplied by the trusted build front. Checkpoint bytes
are stdin data. They never participate in executable/argument/cwd selection.
"""

import json
import math
import subprocess
import sys
from pathlib import Path


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
    receipt = json.loads(result.stdout)
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
    predictions = receipt["predictions"]
    assert len(predictions) == 4 and all(len(row) == 1 for row in predictions)
    assert all(
        type(row[0]) in (int, float) and math.isfinite(row[0]) for row in predictions
    )
    loss = (
        sum((row[0] - target) ** 2 for row, target in zip(predictions, [0, 1, 1, 0]))
        / 4
    )
    assert abs(loss - receipt["final_loss"]) < 1e-12
    losses = receipt["losses"]
    assert losses and all(type(v) in (int, float) and math.isfinite(v) for v in losses)
    assert losses[0] == receipt["initial_loss"] and losses[-1] == receipt["final_loss"]
    return receipt


def main():
    executable = Path(sys.argv[1]).resolve(strict=True)
    fresh = run(executable)
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
    for args, text in [
        (("extra",), ""),
        (("--resume", "extra"), "{}"),
        (("--resume",), "["),
        (("--resume",), "null"),
        (("--resume",), json.dumps(bad_model)),
        (("--resume",), json.dumps(exhausted)),
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
