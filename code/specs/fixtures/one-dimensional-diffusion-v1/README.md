# One-Dimensional Diffusion V1 Fixtures

This corpus pins one complete scalar diffusion learning loop: two closed-form
forward noise levels using a saved noise value, a timestep-aware noise
predictor, per-level gradient contributions, a three-parameter finite-
difference audit, one loss-reducing SGD step, and a deterministic two-step
reverse mean.

```text
python code/scripts/validate_one_dimensional_diffusion_labs.py
```

See
[`NN19-one-dimensional-diffusion-labs.md`](../../NN19-one-dimensional-diffusion-labs.md).
