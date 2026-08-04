# Residual and Receptive-Field Fixtures V1

This corpus pins the NN08 forward path through two same-padded convolutions,
an identity residual addition, ReLU, and a complete receptive-field trace for
every output position.

Validate it from the repository root:

```text
python code/scripts/validate_residual_receptive_labs.py
```

See [`../../NN08-residual-receptive-labs.md`](../../NN08-residual-receptive-labs.md)
for formulas, conformance levels, and the Rust-core direction.
