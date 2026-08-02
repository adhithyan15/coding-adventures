# Optimization Learning Fixtures V1

This corpus accompanies NN04. It pins analytical gradients, independent
central finite differences, and deterministic optimizer trajectories for a
small linear problem.

Validate it from the repository root:

```text
python code/scripts/validate_optimization_learning_labs.py
```

The first lab uses four points on `y = 2x + 1`, starts away from the minimum,
and compares one-row stochastic updates, two-row mini-batches, and full-batch
updates.
