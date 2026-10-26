"""Train XOR, or resume a portable checkpoint supplied on stdin with --resume."""

import json
import sys

from neural_framework import DenseNetwork

inputs = [[0, 0], [0, 1], [1, 0], [1, 1]]
targets = [[0], [1], [1], [0]]
resume = "--resume" in sys.argv
model = (
    DenseNetwork.from_checkpoint(json.load(sys.stdin))
    if resume
    else DenseNetwork.seeded([2, 4, 1], ["tanh", "sigmoid"], 7)
)
losses = model.fit(inputs, targets, 1 if resume else 3000, 1)
print(
    json.dumps(
        {
            "checkpoint": model.to_checkpoint(),
            "predictions": model.predict(inputs),
            "initial_loss": losses[0],
            "final_loss": losses[-1],
        },
        allow_nan=False,
    )
)
