# Gradient Accumulation V1 Fixtures

This language-neutral NN28 corpus pins one persistent scalar parameter-gradient
buffer. It shows that repeated backward calls add, optimizer steps read without
clearing, explicit zeroing resets the buffer, and mean micro-batch scaling must
be chosen deliberately.

Validate it from the repository root:

```text
python code/scripts/validate_gradient_accumulation_labs.py
```

See [NN28](../../NN28-gradient-accumulation-zeroing-labs.md) for the contract.
