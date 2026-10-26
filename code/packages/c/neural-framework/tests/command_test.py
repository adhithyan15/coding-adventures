"""Execute the real native command, including adversarial resume dimensions.

Usage: python tests/command_test.py TRUSTED_BUILT_EXECUTABLE
No checkpoint data controls argv or cwd. This is not a mocked subprocess suite.
"""

import json
import subprocess
import sys
from pathlib import Path


def validate(executable):
    executable = str(Path(executable).resolve())
    cwd = Path(__file__).resolve().parents[1]

    def run(flags, text=""):
        return subprocess.run(
            [executable] + flags,
            cwd=cwd,
            input=text,
            encoding="utf-8",
            text=True,
            capture_output=True,
            timeout=120,
            check=False,
        )

    for flags, text in (
        (["--unknown"], ""),
        (["--resume", "--resume"], ""),
        (["--resume"], ""),
        (["--resume"], "{"),
        (["--resume"], "null"),
        (["--resume"], " " * (4 * 1024 * 1024 + 1)),
    ):
        result = run(flags, text)
        assert result.returncode != 0 and not result.stdout, (flags, result)
    for inputs, outputs in ((3, 1), (2, 2)):
        # Schema-valid, but incompatible with the fixed XOR buffers. ASan must
        # observe rejection BEFORE a shape-derived read from those buffers.
        cp = {
            "format": "coding-adventures-dense",
            "version": 1,
            "step": 0,
            "layers": [
                {
                    "activation": "linear",
                    "weights": [[0] * outputs for _ in range(inputs)],
                    "biases": [0] * outputs,
                }
            ],
        }
        result = run(["--resume"], json.dumps(cp))
        assert result.returncode != 0 and not result.stdout, result
    first = run([])
    assert first.returncode == 0, first.stderr
    receipt = json.loads(first.stdout)
    assert receipt["checkpoint"]["step"] == 3000
    assert receipt["final_loss"] < 0.01
    for step in (3001, 3002):
        result = run(["--resume"], json.dumps(receipt["checkpoint"]))
        assert result.returncode == 0, result.stderr
        receipt = json.loads(result.stdout)
        assert receipt["checkpoint"]["step"] == step
        assert receipt["final_loss"] < 0.01
        baseline = run([f"--baseline-{step}"])
        assert baseline.returncode == 0, baseline.stderr
        assert receipt["checkpoint"] == json.loads(baseline.stdout)["checkpoint"]
    print("native C command: bounded/error/shape/fresh/baseline/resume PASS")


if __name__ == "__main__":
    validate(sys.argv[1])
