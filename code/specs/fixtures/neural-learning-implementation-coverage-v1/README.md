# Neural-learning implementation coverage v1 fixture

This catalog tracks who owns the arithmetic for the first portable neural
fixture. Go, Ruby, and Rust each compute the NN03 neuron natively. Python uses
`ctypes` to cross the NN35 C ABI, so the shared Rust core owns that lane's
arithmetic.

Run the closed coverage gate from the repository root:

```bash
python code/scripts/validate_neural_learning_implementation_coverage.py
```

The validator checks the catalog, cross-checks the NN34 and NN35 contracts,
executes all three native consumers, and calls the compiled Rust shared library
through Python. A lane is counted only when its real execution gate passes.

The `3 native + 1 binding = 4 verified lanes` calculation is an inventory, not
a quality score, benchmark, or claim of curriculum mastery.
