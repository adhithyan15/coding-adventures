# Neural-learning Rust C ABI v1 fixture

This deterministic catalog describes the first stable C boundary for the
neural-learning roadmap. It pins the exported C signatures, version number,
closed status table, ownership rules, NN03 hand calculation, and representative
failure probes.

Validate the document and execute the compiled Rust library from the repository
root:

```bash
python code/scripts/validate_neural_learning_rust_cabi.py
```

The validator builds the local `neural-learning-capi` crate without a shell,
loads only that known artifact, checks the exported ABI version and status
messages, executes the paper example, and verifies that rejected calls leave
caller-owned outputs unchanged.
