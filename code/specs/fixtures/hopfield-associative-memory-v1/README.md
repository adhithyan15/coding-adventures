# Hopfield Associative Memory V1 Fixtures

This corpus pins one deterministic four-neuron associative recall round. The
fixture stores a bipolar pattern with the normalized Hebbian outer product,
zeros the diagonal, damages one bit, and performs one asynchronous in-place
sweep in the saved order.

Every consumer must reproduce the weight matrix, every incoming contribution,
local field, state transition, energy, overlap, and final fixed point within the
document tolerance. A zero local field preserves the previous state.

Validate the corpus from the repository root:

```text
python code/scripts/validate_hopfield_associative_memory_labs.py
```
